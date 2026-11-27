//! 統括ハブの器と指令ディスパッチ（`EmoPresenter` 本体・target 登録・`Hide`／`InvalidateCache`）。
//!
//! `EmoPresenter` の定義と、指令の入口 [`EmoPresenter::apply`] からの分岐を持つ。`ShowSurface` の適用は
//! `super::show`、`ReplaceTarget` は `super::replace`、DPI 再スケールは `super::refresh`、照会系は
//! `super::read`、当たり判定は `super::hit` に分かれる（いずれも同一型の別 `impl` ブロック）。

use crate::hit_shapes::HitMasks;

use super::budget::FrameBudget;
use super::hit_picture::HitNotes;
use super::scale_change::ScaleChange;
use super::visibility::strip_finite_films;
use super::{
    AtlasTable, BindSet, ComposeCache, Composer, EmoWorld, Entity, HashMap, NeverShown,
    PatternState, PhantomData, PresentCommand, PresentError, PresentOutcome, PresentTarget,
    ReplySender, ScalePolicy, ScaleRatio, TargetId, VisibilityOwnership, World,
};

/// 指令適用の統括ハブ（合成・キャッシュ・表示・当たり判定の一点結線・UI スレッド専有）。
///
/// target を [`Self::attach_target`] で登録し、[`Self::apply`] で [`PresentCommand`] を適用する。
/// COM/GPU 資源を内包するため `!Send`（`PhantomData<*const ()>` で型強制）。`unsafe impl Send`
/// は置かない。
pub struct EmoPresenter {
    /// target 識別子 → 表示コンテキスト。
    pub(super) targets: HashMap<TargetId, PresentTarget>,
    /// 表示成立点で積んだ倍率の記録（取り出すまで積んだ順に持つ・[`Self::take_scale_changes`]）。
    pub(super) scale_changes: Vec<ScaleChange>,
    /// `!Send`/`!Sync` を型で担保するマーカー（UI スレッドアフィニティの構造的強制）。
    _not_send: PhantomData<*const ()>,
}

impl EmoPresenter {
    /// 空の統括ハブを構築する（target は未登録）。
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
            scale_changes: Vec::new(),
            _not_send: PhantomData,
        }
    }

    /// target を登録し、窓 Entity を装着先として記録する（窓生成は呼び手の責務）。
    ///
    /// 装着（`VisualMount`）は初回 `ShowSurface` で原寸が確定してから遅延生成するので、本メソッドは
    /// skeleton（`mount=None`/`visible=false`）を登録するのみで World には触れない。既存 id への
    /// 再登録は値の差し替えだけで、前の装着（窓の子の Visual）は窓に残る。差し替えには
    /// `ReplaceTarget` を使う。
    ///
    /// # `author_dpi`（作者基準 DPI）
    ///
    /// k の分母となる作者宣言値（shell `seriko.dpi`／balloon `dpi`・既定 [`DEFAULT_AUTHOR_DPI`]）を
    /// target の拡大政策 [`ScalePolicy`] として確定する。k そのものはここで導出しない: 窓 DPI は時間で
    /// 変わる（モニタ跨ぎ移動・表示スケール変更）ので、導出は `ShowSurface` 適用ごとに行う。政策は
    /// target（窓）ごとに持つので、DPI の異なるモニタ上の複数窓がそれぞれ自窓の k で表示される。
    /// `0` は [`ScalePolicy::new`] が既定 96 へ正規化する（分母ゼロで表示を失わない）。
    ///
    /// [`DEFAULT_AUTHOR_DPI`]: crate::scale::DEFAULT_AUTHOR_DPI
    pub fn attach_target(
        &mut self,
        target: TargetId,
        window: Entity,
        emo_world: EmoWorld,
        atlas: AtlasTable,
        author_dpi: u16,
    ) {
        self.targets.insert(
            target,
            PresentTarget {
                emo_world,
                atlas,
                composer: Composer::new(),
                budget: FrameBudget::new(),
                cache: ComposeCache::new(),
                window,
                mount: None,
                visible: false,
                // 可視性の所有者は既定で登録する。バルーン窓のような外部所有は結線側が
                // `set_visibility_ownership` で明示する。
                ownership: VisibilityOwnership::default(),
                current_surface_id: None,
                // アプリ管理拡大率は ONE 固定の縮退シーム。
                policy: ScalePolicy::new(author_dpi, ScaleRatio::ONE),
                applied: None,
                never_shown: None,
                surface_requested: false,
                // 起点は装着ごと（差し替えもここを通るので空から始まる）。
                scale_baseline: None,
                native_size: None,
                last_show: None,
                held_pattern: None,
                stage_generation: 0,
                acked_generation: 0,
                move_shift: (0, 0),
                pending_resize: None,
                gpu: None,
                nodes: None,
                failure_note: None,
                hit_picture: None,
                hit_masks: HitMasks::default(),
                shown_base: None,
                hit_notes: HitNotes::default(),
            },
        );
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
            } => self.show_or_hold(world, target, surface_id, binds, pattern, reply),
            PresentCommand::Hide { target, reply } => self.apply_hide(world, target, reply),
            PresentCommand::InvalidateCache { target, reply } => {
                self.apply_invalidate(target, reply)
            }
            PresentCommand::ReplaceTarget {
                target,
                emo_world,
                atlas,
                author_dpi,
                show,
                reply,
            } => self.apply_replace(world, target, *emo_world, atlas, author_dpi, show, reply),
            PresentCommand::StageAck { target, generation } => {
                self.apply_stage_ack(target, generation)
            }
            PresentCommand::MoveShift { target, x, y } => self.apply_move_shift(target, x, y),
        }
    }

    /// 合図 `MoveShift` の適用: 対象の欄にずれを書くだけ（合成・可視性・合成の覚えには触らない・
    /// 応答なし）。未装着の対象（ゴーストの入れ替えの継ぎ目）は `StageAck` と同じく `debug!` を出して捨てる。
    fn apply_move_shift(&mut self, target_id: TargetId, x: i64, y: i64) {
        let Some(t) = self.targets.get_mut(&target_id) else {
            tracing::debug!(
                ?target_id,
                x,
                y,
                attached = false,
                "apply(MoveShift): 捨てた"
            );
            return;
        };
        t.move_shift = (x, y);
    }

    /// 合図 `StageAck` の適用: 追い付いた世代を「今の値と受けた番号の大きい方」にするだけ（合成・
    /// 可視性には触らない・応答なし）。出番の世代より大きい番号・未装着の対象（ゴーストの入れ替えの
    /// 継ぎ目）は `debug!` を出して捨てる。
    fn apply_stage_ack(&mut self, target_id: TargetId, generation: u64) {
        let Some(t) = self.targets.get_mut(&target_id) else {
            tracing::debug!(
                ?target_id,
                generation,
                attached = false,
                "apply(StageAck): 捨てた"
            );
            return;
        };
        if generation > t.stage_generation {
            tracing::debug!(
                ?target_id,
                generation,
                stage_generation = t.stage_generation,
                attached = true,
                "apply(StageAck): 捨てた"
            );
            return;
        }
        t.acked_generation = t.acked_generation.max(generation);
    }

    /// 外からの `ShowSurface` の入口: 隠れている外から所有される対象へのコマだけが違う指令は預かる
    /// だけにし（合成 0 回）、それ以外は `apply_show` へ通す。
    ///
    /// 預かるのは、外から所有され・見えておらず・表示が一度成立していて（`last_show` が在る）・指令の
    /// 面の番号と着せ替えが `last_show` と同じで・コマが `last_show` と違う、の 5 つが揃うときだけ。
    /// 面の比べ先は `current_surface_id` でなく `last_show` なので、`Hide` の直後のコマだけの指令も
    /// 預かる。応答は成功を 1 回返す。コマまで同じ指令は通す（引き当てが当たるので合成しない・隠れて
    /// いる間の DPI の変化も拾う）。通した指令で表示が成立したら預かったコマは捨て（最後の指令が勝つ）、
    /// 失敗したら残す。
    ///
    /// その判定の前に、外から所有され・合図が出番の世代に追い付いていない対象なら、指令のコマから回数
    /// つきの子の欄を外す（`strip_finite_films`・`show_target` と同じ関数）。`show_target`・
    /// `refresh_scale`・差し替えは `apply_show` を直接呼ぶので、ここを通らない。
    fn show_or_hold(
        &mut self,
        world: &mut World,
        target_id: TargetId,
        surface_id: u32,
        binds: BindSet,
        mut pattern: PatternState,
        reply: Option<ReplySender<PresentOutcome>>,
    ) {
        // 合図が今の出番の世代に追い付くまでは、出し直しの前に決めたコマかもしれないので回数つきの
        // 子の欄を外す（指令そのものは捨てない・預かるかの判定より前）。
        if let Some(t) = self.targets.get(&target_id)
            && t.ownership == VisibilityOwnership::External
            && t.acked_generation < t.stage_generation
        {
            strip_finite_films(&t.emo_world, &mut pattern);
        }
        if let Some(t) = self.targets.get_mut(&target_id)
            && t.ownership == VisibilityOwnership::External
            && !t.visible
            && t.last_show
                .as_ref()
                .is_some_and(|(shown_id, shown_binds, shown_frame)| {
                    *shown_id == surface_id && *shown_binds == binds && *shown_frame != pattern
                })
        {
            t.held_pattern = Some(pattern);
            self.record_show_outcome(target_id, &Ok(()));
            Self::reply(reply, Ok(()));
            return;
        }

        let outcome = self.apply_show(world, target_id, surface_id, binds, pattern, false);
        self.record_show_outcome(target_id, &outcome);
        if outcome.is_ok()
            && let Some(t) = self.targets.get_mut(&target_id)
        {
            t.held_pattern = None;
        }
        Self::reply(reply, outcome);
    }

    /// 面の表示の結果から、最初の面を表示しないまま決まった理由を書く（面の指令を受けた印も立てる）。
    pub(super) fn record_show_outcome(&mut self, target_id: TargetId, outcome: &PresentOutcome) {
        let Some(t) = self.targets.get_mut(&target_id) else {
            return;
        };
        t.surface_requested = true;
        t.never_shown = match (t.applied, outcome) {
            (Some(_), _) => None,
            (None, Ok(())) => Some(NeverShown::Hidden),
            (None, Err(_)) => Some(NeverShown::Failed),
        };
    }

    /// `Hide`（`\s[-1]` 相当）の適用: visual 非表示＋当たり判定停止。装着・キャッシュは保持する。
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
        tracing::debug!(
            ?target_id,
            was_visible = target.visible,
            "apply(Hide): 非表示へ"
        );
        target.visible = false;
        // Hide（`\s[-1]` 相当）は表示していない＝現サーフェス無し。
        target.current_surface_id = None;
        target.surface_requested = true;
        if target.applied.is_none() {
            target.never_shown = Some(NeverShown::Hidden);
        }
        Self::reply(reply, Ok(()));
    }

    /// `InvalidateCache` の適用: 合成キャッシュ全破棄。画面の節は写し済みゆえ表示は継続。
    fn apply_invalidate(
        &mut self,
        target_id: TargetId,
        reply: Option<ReplySender<PresentOutcome>>,
    ) {
        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(?target_id, "apply(InvalidateCache): 未装着ターゲット");
            Self::reply(reply, Err(PresentError::TargetNotAttached(target_id)));
            return;
        };

        // 原寸はエントリの中に在るので、対を崩さないために別途落とすフィールドは無い。`native_size` は
        // 触らない: 表が空でも画面には前回の絵が残っており、照会契約はその絵の原寸を返し続けるのが
        // 正しい。以後は必ずミス＝再合成が走り、対が再構築される。
        target.cache.invalidate_all();
        tracing::debug!(
            ?target_id,
            "apply(InvalidateCache): キャッシュ全破棄（表示は継続）"
        );
        Self::reply(reply, Ok(()));
    }

    /// reply 同梱時に結果を高々 1 回送る。受信端が既に drop 済みなら撃ちっぱなし扱い（debug ログ）。
    pub(super) fn reply(reply: Option<ReplySender<PresentOutcome>>, outcome: PresentOutcome) {
        if let Some(tx) = reply
            && tx.send(outcome).is_err()
        {
            tracing::debug!("reply: 受信端が既に drop 済み（撃ちっぱなし扱い・無視）");
        }
    }
}

impl Default for EmoPresenter {
    fn default() -> Self {
        Self::new()
    }
}
