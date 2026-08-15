//! 統括ハブの器と指令ディスパッチ（`EmoPresenter` 本体・target 登録・`Hide`／`InvalidateCache`）。
//!
//! `EmoPresenter` の定義と、指令の入口 [`EmoPresenter::apply`] からの分岐を持つ。`ShowSurface` の適用は
//! `super::show`、DPI 再スケールは `super::refresh`、照会系は `super::read`、当たり判定は `super::hit` に
//! 分かれる（いずれも同一型 `EmoPresenter` の別 `impl` ブロック）。

use super::budget::FrameBudget;
use super::{
    AtlasTable, ComposeCache, Composer, EmoWorld, Entity, HashMap, PhantomData, PresentCommand,
    PresentError, PresentOutcome, PresentTarget, ReplySender, ScalePolicy, ScaleRatio, TargetId,
    VisibilityOwnership, World,
};

/// 指令適用の統括ハブ（合成・キャッシュ・表示・マスクの一点結線・UI スレッド専有）。
///
/// target を [`Self::attach_target`] で登録し、[`Self::apply`] で [`PresentCommand`] を適用する。
/// COM/GPU 資源を内包するため `!Send`（`PhantomData<*const ()>` で型強制・R7.1）。`unsafe impl Send`
/// は置かない。
pub struct EmoPresenter {
    /// target 識別子 → 表示コンテキスト。
    pub(super) targets: HashMap<TargetId, PresentTarget>,
    /// `!Send`/`!Sync` を型で担保するマーカー（UI スレッドアフィニティの構造的強制・R7.1）。
    _not_send: PhantomData<*const ()>,
}

impl EmoPresenter {
    /// 空の統括ハブを構築する（target は未登録）。
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
            _not_send: PhantomData,
        }
    }

    /// target を登録し、窓 Entity を装着先として記録する（窓生成は呼び手＝placement/example の責務）。
    ///
    /// 供給面（`SwapChainPresenter`）と装着（`VisualMount`）は**初回 `ShowSurface` で原寸が確定してから
    /// 遅延生成**するため、本メソッドは skeleton（`chain=None`/`mount=None`/`visible=false`）を組んで登録
    /// するのみで World には触れない。既存 id への再登録は表示コンテキストごと置換する。
    ///
    /// `world` は将来の system 化（`&mut World` を要する装着タイミング）へ向けた API 一貫性のために受ける
    /// が、遅延生成方針ゆえ本メソッドでは参照しない。
    ///
    /// # `author_dpi`（作者基準 DPI・要件 1.1/1.5）
    ///
    /// k の分母となる作者宣言値（shell `seriko.dpi`／balloon `dpi`・既定 [`DEFAULT_AUTHOR_DPI`]）を
    /// target の拡大政策 [`ScalePolicy`] として確定する。**k そのものはここで導出しない**——k は窓 DPI に
    /// 依存し、窓 DPI は時間で変わる（モニタ跨ぎ移動・表示スケール変更）ため、導出は `ShowSurface` 適用
    /// ごとに行う（design Flow 1）。政策は target（＝窓）ごとに保持されるため、DPI の異なるモニタ上の
    /// 複数窓がそれぞれ自窓の k で表示される（要件 1.5）。`0` は [`ScalePolicy::new`] が既定 96 へ
    /// 正規化する（分母ゼロで表示を失わない・log-first）。
    ///
    /// [`DEFAULT_AUTHOR_DPI`]: crate::scale::DEFAULT_AUTHOR_DPI
    pub fn attach_target(
        &mut self,
        _world: &mut World,
        target: TargetId,
        window: Entity,
        emo_world: EmoWorld,
        atlas: AtlasTable,
        author_dpi: u16,
    ) -> Result<(), PresentError> {
        self.targets.insert(
            target,
            PresentTarget {
                emo_world,
                atlas,
                composer: Composer::new(),
                // 確保計数は target と同じ寿命で始まる（累積の起点＝この target の登録時）。
                budget: FrameBudget::new(),
                cache: ComposeCache::new(),
                window,
                mount: None,
                chain: None,
                visible: false,
                // 可視性の所有者は常に既定（従来挙動）で登録する。バルーン窓のような外部所有は
                // 結線側が `set_visibility_ownership` で明示する（attach に判断を持ち込まない）。
                ownership: VisibilityOwnership::default(),
                current_surface_id: None,
                // アプリ管理拡大率は本仕様では ONE 固定の縮退シーム（要件 1.6）。
                policy: ScalePolicy::new(author_dpi, ScaleRatio::ONE),
                applied: None,
                native_size: None,
                last_show: None,
                pending_resize: None,
            },
        );
        Ok(())
    }

    /// 指令を適用する（UI スレッド上で呼ぶ）。reply 同梱時は完了/失敗を高々 1 回返信する。
    ///
    /// 戻り値は持たず、結果は各 variant の `reply`（`Some` のとき）へ送る。失敗経路も含め、全分岐が
    /// ログを出したうえで reply する（silent failure 禁止）。
    pub fn apply(&mut self, world: &mut World, cmd: PresentCommand) {
        match cmd {
            PresentCommand::ShowSurface {
                target,
                surface_id,
                binds,
                pattern,
                reply,
            } => self.apply_show(world, target, surface_id, binds, pattern, reply),
            PresentCommand::Hide { target, reply } => self.apply_hide(world, target, reply),
            PresentCommand::InvalidateCache { target, reply } => {
                self.apply_invalidate(target, reply)
            }
        }
    }

    /// `Hide`（`\s[-1]` 相当）の適用: visual 非表示＋当たり判定停止。swap chain・キャッシュは保持する（R3.3）。
    fn apply_hide(
        &mut self,
        world: &mut World,
        target_id: TargetId,
        reply: Option<ReplySender<PresentOutcome>>,
    ) {
        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(?target_id, "apply(Hide): 未装着ターゲット");
            Self::reply(reply, Err(PresentError::TargetNotAttached(target_id)));
            return;
        };

        if let Some(mount) = target.mount.as_ref() {
            mount.set_visible(world, false);
        }
        tracing::debug!(?target_id, was_visible = target.visible, "apply(Hide): 非表示へ");
        target.visible = false;
        // Hide（`\s[-1]` 相当）は表示していない＝現サーフェス無し（R3.2/4.4・Key decisions (a)）。
        target.current_surface_id = None;
        Self::reply(reply, Ok(()));
    }

    /// `InvalidateCache` の適用: 合成キャッシュ全破棄（R4.3）。表示中バッファ/マスクは反映済みゆえ表示は継続。
    fn apply_invalidate(&mut self, target_id: TargetId, reply: Option<ReplySender<PresentOutcome>>) {
        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(?target_id, "apply(InvalidateCache): 未装着ターゲット");
            Self::reply(reply, Err(PresentError::TargetNotAttached(target_id)));
            return;
        };

        // エントリを全て捨てる。原寸はエントリの中に在るので、対を崩さないために別途落とす
        // フィールドは無い（要件 7.1・容量 3 で `cached_native` はエントリへ移った）。表示中の
        // `native_size` は触らない——表が空でも画面には前回の絵が残っており、照会契約はその絵の
        // 原寸を返し続けるのが正しい（R4.3: キャッシュ無効化は表示を変えない）。以後は必ずミス＝
        // 再合成が走り、対が再構築される。
        target.cache.invalidate_all();
        tracing::debug!(?target_id, "apply(InvalidateCache): キャッシュ全破棄（表示は継続）");
        Self::reply(reply, Ok(()));
    }

    /// reply 同梱時に結果を高々 1 回送る。受信端が既に drop 済みなら撃ちっぱなし扱い（debug ログ）。
    pub(super) fn reply(reply: Option<ReplySender<PresentOutcome>>, outcome: PresentOutcome) {
        if let Some(tx) = reply {
            if tx.send(outcome).is_err() {
                tracing::debug!("reply: 受信端が既に drop 済み（撃ちっぱなし扱い・無視）");
            }
        }
    }
}

impl Default for EmoPresenter {
    fn default() -> Self {
        Self::new()
    }
}
