//! `EmoPresenter`（presenter.rs）: 提示段の統括ハブ（合成・キャッシュ・表示・マスクの一点結線）。
//!
//! 上流が組んだ部品（[`ComposeCache`]・[`SwapChainPresenter`]・[`VisualMount`]・emo-compose の
//! [`Composer`]）を target ごとに束ね、指令 [`PresentCommand`] を UI スレッド上で適用する。合成そのもの
//! は emo-compose、供給面バイト転送は `SwapChainPresenter`、当たり判定マスクは wintf hit-test が担い、
//! 本型は「指令を受けて、キャッシュ引き当て or 合成 → 供給面アップロード → AlphaMask 同期 → 可視制御」を
//! **一続きの UI スレッド呼び出し**として結線する（design §EmoPresenter・§System Flows 指令適用）。
//!
//! # UI スレッドアフィニティ（型で強制・R7.1）
//!
//! `EmoPresenter` は COM/GPU 資源（`SwapChainPresenter` 内の DXGI/D3D11・`VisualMount` の WUC visual）を
//! 内包するため **`!Send`**（NonSend）である。`unsafe impl Send` は置かず、`PhantomData<*const ()>` を
//! 併せ持つことで「他スレッドへ移動できない」ことを**構造（型）で**担保する。wintf World へ NonSend
//! 資源として登録するか example が直接所有し、`apply`/`attach_target`/`read_back` は必ず UI スレッド
//! （NonSend 到達可能スレッド）から呼ばれる（design §Responsibilities & Constraints）。
//!
//! # 原子入替（R2.4）
//!
//! 表示バッファ（`chain.upload`）と当たり判定マスク（`AlphaMaskResource::set`）の更新は**同一 `apply`
//! 呼び出し内**で連続して起き、hit-test も同一 UI スレッドで走るため中間状態は観測不能である。ゆえに
//! surface 切替に伴う「表示とマスクの対入替」は構造的に原子化される（別途ロック不要）。
//!
//! # 失敗経路のログ規律（silent failure 禁止）
//!
//! 全失敗分岐は返す前に `tracing::error!`/`warn!` を出す。`ComposeError::SurfaceNotFound`（解決不能 id）は
//! **error! ＋ 表示不変 ＋ reply `Err`**（R3.4）、`ComposeError::EmptyComposition`（全透明退化）は
//! **warn! ＋ Hide 縮退 ＋ reply `Ok`**（設計ディスカッション #1: 許容される正常退化・skip 解釈は採らない）、
//! デバイス層失敗は `PresentError::Device`（HRESULT ＋文脈）で `Err`。panic は用いない。
//!
//! # 表示スケール k の適用漏斗（emo-dpi-scaling・要件 1.1/1.2/1.5・2.1-2.4）
//!
//! DPI 追従表示の係数 k は **`ShowSurface` の適用ごと**に導出する（design Flow 1「k 導出は show 適用ごと
//! に行う」）——target へ焼き付けず、`attach` でも決めない。これにより「照会値＝実適用 k」の不変条件を
//! 維持する点が経路上の 1 箇所（表示成立点）に閉じる。
//!
//! 経路は `world.get::<DPI>(target.window)` → [`derive_scale`]（政策＝[`ScalePolicy`]・縮退は log-first）
//! → `cache.get(.., k)` → ミス時のみ `compose`（**native 原寸**）→ [`resample`]（native → k 適用）→
//! `cache.insert(.., k, ..)` である。以降の供給面アップロード・`AlphaMaskResource` 同期・`set_bounds`・
//! 可視制御は**既存コードのまま**で、流れる合成結果が k 適用済みになるだけで自動追従する
//! （design 「Strategy A2＝composed 外形従属の連鎖を k 追従へ転用」）。
//!
//! k=1/1（窓 DPI ＝ author_dpi）は [`resample`] を**呼ばずに** native をそのまま表示資源とする——
//! [`resample`] 自体も恒等をバイトコピーで保証するが、素通しなら「k 導入前と同一のオブジェクトが同一経路を
//! 流れる」ことが構造で言えるため、既存 golden の不変（要件 7.2）が最も強く担保される。

use std::collections::HashMap;
use std::marker::PhantomData;

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

use areka_actor::ReplySender;
use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{
    BindSet, ComposeError, ComposedSurface, Composer, EmoWorld, PatternState, RegionPriority,
    ScaleRatio, resample,
};

use wintf::ecs::{AlphaMaskResource, DPI, GraphicsCore, WucGraphicsResource};

use crate::cache::ComposeCache;
use crate::chain::SwapChainPresenter;
use crate::command::{PresentCommand, PresentError, PresentOutcome, TargetId};
use crate::mount::VisualMount;
use crate::scale::{ScalePolicy, derive_scale};

/// target ごとの表示コンテキスト（シェル・バルーンで同一機構・R5.1 の統一原則）。
///
/// `chain`／`mount` は **初回 `ShowSurface` で原寸が確定してから遅延生成**する（0×0 の供給面は作れない・
/// 全透明退化では生成しない）。`emo_world`／`atlas`（構築時 `bind_atlas` 済み）と `composer`／`cache` は
/// 合成・引き当ての入力側、`window` は装着先の窓ハンドル（R1.3）である。
struct PresentTarget {
    /// 合成入力（構築時 `bind_atlas` 済み・不変）。
    emo_world: EmoWorld,
    /// アトラス正本（不変・`InvalidateCache` の再合成源）。
    atlas: AtlasTable,
    /// 合成器（状態非保持・スクラッチのみ再利用）。
    composer: Composer,
    /// 合成入力（surface id＋bind 集合）→ (composed, mask) 対の容量 1 メモ化スロット。
    cache: ComposeCache,
    /// 装着先の窓 Entity（R1.3・遅延装着の対象）。
    window: Entity,
    /// 窓装着ハンドル（初回表示で生成）。
    mount: Option<VisualMount>,
    /// 自前供給面（初回表示で原寸確定後に生成）。
    chain: Option<SwapChainPresenter>,
    /// 現在可視か（`Hide`／全透明退化で false・`ShowSurface` 成功で true）。
    visible: bool,
    /// 現在表示中のサーフェス id ＝「**最後に表示が成立した id**」（CurrentSurfaceRead・R3.1-3.3）。
    ///
    /// 画面の絵ではなく表示成立の結果を刻む（全透明合成でも表示成立＝その id が正・α 非依存で collision
    /// 解決の単一真実源）。`Hide`／`EmptyComposition` 縮退で `None`。書き込みは既存 `visible` 更新点と
    /// 同一の3箇所のみ（表示成立＝`Some(surface_id)`／縮退・Hide＝`None`）で、失敗経路は表示成立点より
    /// 手前で early return するため前値を保持する（`ComposeKey` からは導出しない＝`invalidate_all` で
    /// キーが消えても表示は残るため画面と乖離する）。
    current_surface_id: Option<u32>,
    /// 拡大政策（`attach_target` で確定・以後不変・要件 1.5）。
    ///
    /// k は target（＝窓）ごとの `policy` と**その窓の** `DPI` component から導出されるため、DPI の
    /// 異なる複数モニタに窓が同時に存在しても各窓が自窓の k で表示される。政策自体（author_dpi・
    /// アプリ管理拡大率）は時間で変わらない——変わるのは窓 DPI の側である。
    policy: ScalePolicy,
    /// **実際に表示へ適用中の** k（照会契約の単一真実源・要件 1.2）。
    ///
    /// 更新は**表示成立点のみ**（失敗経路は手前で early return ＝前値保持・要件 4.4）。表示が一度も
    /// 成立していない間は `None` で、[`EmoPresenter::text_slot_view`] もその間は `None` を返す
    /// （「まだ何も適用していない」を 1.0 で塗り潰さない）。
    applied: Option<ScaleRatio>,
    /// 表示中サーフェスの **native 原寸**（k 適用**前**の合成外形・照会契約 `surface_size()` の供給源）。
    ///
    /// 物理寸との関係は `物理寸 == applied.scaled_extent(native_size)`（丸め権威は
    /// [`ScaleRatio::scaled_extent`] 1 本）。供給面 `chain.size()` は k 適用**後**の物理寸を持つため、
    /// 照会契約の native 原寸をここで別に保持する。
    ///
    /// **更新規則**: 更新点は `applied` と同じ表示成立点 1 箇所だが、書き込む値は「今回合成したか」に
    /// 依らず常に [`PresentTarget::cached_native`]（＝いま表示に使ったキャッシュエントリ由来の原寸）
    /// である。今回合成した回だけ書く実装は、`insert` 済みのまま失敗して後から**ヒットで**表示が成立した
    /// 場合に「画面の絵と別サーフェスの原寸」あるいは `None` が残り、照会契約が壊れる。
    native_size: Option<(u32, u32)>,
    /// **cache スロットの現エントリに対応する native 原寸**（k 適用前の合成外形）。
    ///
    /// `ComposeCache` は容量 1 スロットで、挿入者は本 presenter ただ 1 箇所（`apply_show` のミス経路）
    /// である。ゆえに `cache.insert` と同じ場所で本フィールドを書けば、**スロットの中身と本フィールドは
    /// 常に対**になる——引き当てがヒットした回は「そのエントリを入れたときの原寸」がここに在る。
    /// `invalidate_all`（スロット破棄）では `None` へ戻す（対を崩さない）。表示成立点はこの値を
    /// [`PresentTarget::native_size`] へ写すだけでよく、合成の有無で分岐しない。
    cached_native: Option<(u32, u32)>,
    /// 最後に表示が成立した show 入力（再表示＝k 再適用のための入力保持）。
    ///
    /// DPI 変化時に「同じ絵を新しい k で描き直す」ための唯一の入力源であり、読み手は
    /// [`EmoPresenter::refresh_scale`] である。記録点は `applied`/`native_size` と同一（表示成立点）で、
    /// 失敗経路では前値が保たれる——ゆえに再表示は常に「最後に**実際に画面へ出た**入力」を描き直す。
    ///
    /// `Hide` では**消さない**（キャッシュ・供給面と同じく保持する）。再表示するか否かは可視ゲートが
    /// 決めるのであって、入力を捨てて決めるのではない（`Hide` → 再 show の復帰経路を壊さない）。
    last_show: Option<(u32, BindSet, PatternState)>,
    /// **未消費の窓寸 reconcile 要求**（表示成立点の状態照合が積む・design Flow 1 キー決定／議題 #2 裁定）。
    ///
    /// 表示成立点で今回の物理寸（k 適用後の scaled 寸）を**前回適用の物理寸**と照合し、異なるときだけ
    /// `Some(新物理寸)` を置く。呼び手（emo2_boot の frame drain フェーズ）は
    /// [`EmoPresenter::take_pending_resize`] で取り出し、同一フレーム内で窓 client を合わせる。
    ///
    /// # なぜ「エッジ」ではなく「状態」なのか（議題 #2 裁定）
    ///
    /// 窓寸 reconcile は **表示が成立したという状態**に紐づく。`Changed<DPI>` エッジに紐づけると、
    /// エッジが初回 show より前に消費された場合に不整合が残置する。ゆえに報告は表示成立点で積まれ、
    /// **取り出されるまで保持される**（呼び手が或るフレームで取り出さなくても要求は失われない）。
    ///
    /// # 初回表示も必ず報告する（Flow 3 手順 5）
    ///
    /// 前回適用寸が無い（`applied`／`native_size` が `None`）初回表示は「差分あり」として扱う。窓は
    /// 起動時の k₀ 見積もり寸で生成されており、実窓 DPI 由来の k と一致する保証がない——初回を黙らせると
    /// k₀ と実 DPI の差分を補正する経路が永久に走らない。
    ///
    /// # べき等（churn を作らない）
    ///
    /// 同寸の再表示では**何も置かない**。`None` を書き戻して既存の未消費要求を消すこともしない
    /// （同寸でも「まだ窓へ反映していない要求」は生きているため）。
    pending_resize: Option<(u32, u32)>,
}

/// 予約 text 層スロットへの読み取り専用の到達手段（emo-text-layer が消費する additive 公開増分・R9.1/9.2）。
///
/// [`EmoPresenter::text_slot_view`] が返すスナップショット値。フィールドは非公開（`#[non_exhaustive]`
/// 相当）で accessor のみを公開し、スロット状態の変更手段を一切持たない（読み取り専用 view）。
/// 装着 API 形（emo-present が描画物を受け取る）は emo-text の描画型が本 crate へ逆流し依存方向
/// （emo-present → emo-text 禁止）と衝突するため採らない（design §TextSlotView）。
///
/// mount は初回 `ShowSurface` で遅延生成されるため、それ以前は取得できない（`text_slot_view` が
/// `None`）。呼び手は表示確立後に取得するか再取得を試みる（runtime 前提条件）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSlotView {
    /// 予約済み text 層スロット entity（`Name("emo-text-layer-slot")`・内容なしの seam）。
    slot: Entity,
    /// スロットが属する装着先の窓 Entity。
    window: Entity,
    /// バルーン/シェル surface の **native 原寸**（k 適用前・取得時点のスナップショット）。
    surface_size: (u32, u32),
    /// バルーン surface と同一の合成スケール k（**実適用値**・要件 1.2）。
    scale: f32,
}

impl TextSlotView {
    /// 予約済み text 層スロット entity（emo-text-layer が描画を装着する先）。
    pub fn slot(&self) -> Entity {
        self.slot
    }

    /// スロットが属する装着先の窓 Entity。
    pub fn window(&self) -> Entity {
        self.window
    }

    /// バルーン surface の **native 原寸**（k 適用**前**の合成外形・要件 1.2 の照会契約）。
    ///
    /// 表示中の**物理寸ではない**。物理寸は丸め権威 [`ScaleRatio::scaled_extent`] を通した
    /// `scaled_extent(scale(), surface_size())` である（下流の照合式
    /// `GetClientRect ≒ surface_size × scale`・design §State Management）。k=1.0 の窓では両者が
    /// 一致するため、k 導入前の観測値（＝供給面寸）とも等しい。
    pub fn surface_size(&self) -> (u32, u32) {
        self.surface_size
    }

    /// バルーン surface と同一の合成スケール k（**実際に表示へ適用中の値**・要件 1.2）。
    ///
    /// かつてはコンパイル時定数 1.0（`CURRENT_COMPOSE_SCALE`）を恒常で返していたが、DPI 追従表示の
    /// 導入で **`PresentTarget.applied`（表示成立点でのみ更新される単一真実源）の写し**へ変わった。
    /// 窓 DPI ＝ author_dpi なら 1.0、192dpi／author 96 なら 2.0 を返す。下流（
    /// `collision-dpi-hittest` の ÷k・`emo-text-layer` の行寸）はこの値を実適用 k として参照してよい。
    pub fn scale(&self) -> f32 {
        self.scale
    }
}

/// 指令適用の統括ハブ（合成・キャッシュ・表示・マスクの一点結線・UI スレッド専有）。
///
/// target を [`Self::attach_target`] で登録し、[`Self::apply`] で [`PresentCommand`] を適用する。
/// COM/GPU 資源を内包するため `!Send`（`PhantomData<*const ()>` で型強制・R7.1）。`unsafe impl Send`
/// は置かない。
pub struct EmoPresenter {
    /// target 識別子 → 表示コンテキスト。
    targets: HashMap<TargetId, PresentTarget>,
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
                cache: ComposeCache::new(),
                window,
                mount: None,
                chain: None,
                visible: false,
                current_surface_id: None,
                // アプリ管理拡大率は本仕様では ONE 固定の縮退シーム（要件 1.6）。
                policy: ScalePolicy::new(author_dpi, ScaleRatio::ONE),
                applied: None,
                native_size: None,
                cached_native: None,
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

    /// `ShowSurface` の適用（キャッシュ引き当て or 合成 → 供給面アップロード → マスク同期 → 可視化）。
    ///
    /// 手順（design §System Flows・Flow 1）: (1) 未装着なら error! ＋ `Err(TargetNotAttached)`。
    /// (1.5) 窓の `DPI` component と target 政策から**この適用に使う k**を導出する（[`derive_scale`]・
    /// component 不在は `None` のまま渡して要件 1.4 の縮退へ落とす）。以降 k は合成入力と同格のキー
    /// 要素であり、ミス時は合成（native）→ [`resample`]（k 適用）を経て挿入される。(2) 合成入力
    /// （surface id＋bind 集合）が直前と完全一致するヒットなら再合成しない（R4.2）——bind 集合が
    /// 1 要素でも異なれば必ずミス＝再合成する（着せ替え・まばたきの正しさの担保）。(3) ミスなら合成し、
    /// `SurfaceNotFound` は error! ＋表示不変＋
    /// `Err`（R3.4）、`EmptyComposition` は warn! ＋ Hide 縮退＋`Ok`（設計ディスカッション #1）、`Ok` なら
    /// `cache.insert`（マスクを 1 回だけ生成）。(4) 使えるエントリで、`chain`/`mount` 未生成なら原寸確定
    /// 後に遅延生成し、`chain.upload` ＋ `AlphaMaskResource::set` ＋ 可視化を同一呼び出し内で行う（R2.4）。
    fn apply_show(
        &mut self,
        world: &mut World,
        target_id: TargetId,
        surface_id: u32,
        binds: BindSet,
        pattern: PatternState,
        reply: Option<ReplySender<PresentOutcome>>,
    ) {
        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(?target_id, surface_id, "apply(ShowSurface): 未装着ターゲット");
            Self::reply(reply, Err(PresentError::TargetNotAttached(target_id)));
            return;
        };

        // (0) k 導出（show 適用ごと・design Flow 1）。窓 DPI は wintf の `DPI` component から読む
        // （consume のみ・新規依存なし）。**component 不在は `None` のまま [`derive_scale`] へ渡す**——
        // ここで 96 を捏造すると要件 1.4 の縮退（error! ＋ k=1.0）が「正常系のふり」で通ってしまう。
        let window = target.window;
        let window_dpi = world.get::<DPI>(window).map(|d| (d.dpi_x, d.dpi_y));
        let scale = derive_scale(target.policy, window_dpi);

        // (1) 引き当て: 合成入力（id＋binds＋pattern）＋表示スケール k の完全一致のみヒット＝再合成
        // しない（R4.2/R5.2・要件 2.4）。ミスのみ合成する。pattern は指令が運ぶ現在コマ集合をそのまま
        // 透過する（presenter は新しい判断を持たず輸送のみ）。空 PatternState なら拡張前と観測等価
        // （R5.4）。k が変われば必ずミスするため、旧 k の絵とマスクを表示に載せることはない（設計 D6）。
        let cache_hit = target
            .cache
            .get(surface_id, &binds, &pattern, scale)
            .is_some();
        if !cache_hit {
            match target
                .composer
                // pattern を合成入力の第一級要素として合成器へ透過する（R5.1）。
                .compose(&target.emo_world, &target.atlas, surface_id, &binds, &pattern)
            {
                Ok(composed) => {
                    // 合成は常に native 原寸（emo-compose の合成経路は k を知らない・設計 D3 の A2）。
                    let native_extent = (composed.width(), composed.height());
                    // k 適用（要件 2.1/2.3）: 合成済みの 1 枚（element 入れ子・SERIKO パターン・mayuna
                    // 着せ替えが畳み込まれた結果）へ**単一の k** を掛けるため、要素間の相対配置・重なりは
                    // 等倍時と同一の見た目関係を保つ。恒等 k は resample を呼ばず native を素通しする
                    // （要件 7.2: 既存 golden がバイト単位で不変であることの構造保証・割り当ても増えない）。
                    let display = if scale.is_identity() {
                        composed
                    } else {
                        let mut scaled = ComposedSurface::new(0, 0);
                        resample(&composed, scale, &mut scaled);
                        scaled
                    };
                    // 挿入時にマスクを 1 回だけ生成し、表示バッファと対で束ねる（R2.1/R2.4）。
                    // pattern は binds と同格のキー要素として挿入キーへ透過する（R5.2）。マスクは
                    // k 適用済み bytes 由来ゆえ物理 px 契約が無修正で整合する（設計 D6）。
                    target
                        .cache
                        .insert(surface_id, binds.clone(), pattern.clone(), scale, display);
                    // スロットの中身と対で原寸を控える（`insert` と同じ場所＝対が崩れない唯一の書き方）。
                    // 以降この回が失敗して early return しても、後からヒットで表示が成立した時点で
                    // 正しい原寸が照会契約へ渡る。
                    target.cached_native = Some(native_extent);
                }
                Err(ComposeError::EmptyComposition(id)) => {
                    // 全透明退化（外形 0×0）: 許容される正常退化として Hide 縮退＋reply Ok（skip ではない）。
                    tracing::warn!(
                        ?target_id,
                        surface_id = id,
                        was_visible = target.visible,
                        "apply(ShowSurface): 全透明退化（EmptyComposition）→ Hide 縮退（reply Ok）"
                    );
                    if let Some(mount) = target.mount.as_ref() {
                        mount.set_visible(world, false);
                    }
                    target.visible = false;
                    // EmptyComposition 縮退は Hide と同じ表示結果ゆえ現サーフェス無し（R3.2・Key decisions (b)）。
                    target.current_surface_id = None;
                    Self::reply(reply, Ok(()));
                    return;
                }
                Err(e) => {
                    // 解決不能 id（SurfaceNotFound 等）: error! ＋ 表示不変 ＋ reply Err（R3.4）。
                    tracing::error!(
                        ?target_id,
                        surface_id,
                        error = %e,
                        "apply(ShowSurface): 合成失敗 → 表示は適用前のまま（reply Err）"
                    );
                    Self::reply(reply, Err(PresentError::Compose(e)));
                    return;
                }
            }
        }

        // (2) 供給面・装着の遅延生成（初回表示・原寸確定後）。
        if target.chain.is_none() {
            let (w, h) = {
                let entry = target
                    .cache
                    .get(surface_id, &binds, &pattern, scale)
                    .expect("直前に引き当て済み");
                (entry.composed.width(), entry.composed.height())
            };

            // Compositor は所有クローンで取り出し、以後の &mut World 装着と借用衝突しないようにする。
            let Some(compositor) = world
                .get_resource::<WucGraphicsResource>()
                .and_then(|r| r.compositor().cloned())
            else {
                tracing::error!(
                    ?target_id,
                    "apply(ShowSurface): WucGraphicsResource/Compositor 不在（供給面を生成できない）"
                );
                Self::reply(
                    reply,
                    Err(PresentError::Device {
                        hresult: 0,
                        context: "WucGraphicsResource::compositor",
                    }),
                );
                return;
            };

            // GraphicsCore は生成呼び出しの間だけ借用する（surface は所有で返るため借用は閉じる）。
            let new_chain = {
                let Some(gfx) = world.get_resource::<GraphicsCore>() else {
                    tracing::error!(
                        ?target_id,
                        "apply(ShowSurface): GraphicsCore 不在（供給面を生成できない）"
                    );
                    Self::reply(
                        reply,
                        Err(PresentError::Device {
                            hresult: 0,
                            context: "GraphicsCore resource",
                        }),
                    );
                    return;
                };
                SwapChainPresenter::new(gfx, &compositor, w, h)
            };
            let (chain, surface) = match new_chain {
                Ok(pair) => pair,
                // SwapChainPresenter::new は内部で error! 済み（chain.rs device_err）。ここは reply のみ。
                Err(e) => {
                    Self::reply(reply, Err(e));
                    return;
                }
            };

            let mount = match VisualMount::attach(world, window, &surface, &compositor, (w, h)) {
                Ok(m) => m,
                // VisualMount::attach も内部で error! 済み（mount.rs device_err）。
                Err(e) => {
                    Self::reply(reply, Err(e));
                    return;
                }
            };

            target.chain = Some(chain);
            target.mount = Some(mount);
        }

        // (3) 供給面アップロード ＋ マスク同期 ＋ 可視化（同一呼び出し内＝原子入替・R2.4）。
        let entry = target
            .cache
            .get(surface_id, &binds, &pattern, scale)
            .expect("直前に引き当て済み");
        let chain = target.chain.as_mut().expect("直上で生成済み");
        if let Err(e) = chain.upload(&entry.composed) {
            // upload は内部で error! 済み（chain.rs）。表示は前状態を保つ（成功まで旧状態不変）。
            Self::reply(reply, Err(e));
            return;
        }
        // 表示物理寸は**供給面の実寸**を単一真実源とする（upload が外形変化を検知して合わせ込んだ後の
        // 値＝k 適用済み composed の外形）。エントリ外形から別途組み立てないことで、供給面・visual
        // 境界・マスクが同一の物理寸に揃うことを構造で担保する（R3.2・k 追従は A2 の自動追従）。
        let size = chain.size();

        let mount = target.mount.as_ref().expect("直上で生成済み");
        if let Some(mut mask_res) = world.get_mut::<AlphaMaskResource>(mount.surface_entity()) {
            // 表示バッファと同一 bytes 由来のマスクを hit-test へ供給する（R2.2/R2.5）。
            mask_res.set(entry.mask.clone());
        } else {
            tracing::warn!(
                ?target_id,
                entity = ?mount.surface_entity(),
                "apply(ShowSurface): surface entity に AlphaMaskResource が無い（当たり判定は矩形/前状態）"
            );
        }
        mount.set_visible(world, true);
        mount.set_bounds(world, size);
        target.visible = true;
        // 表示成立＝この id が現サーフェス（全透明でも成立・α 非依存の単一真実源・R3.1/3.3・Key decisions）。
        target.current_surface_id = Some(surface_id);
        // ここが**表示成立点**＝ k・native 原寸・再表示入力の唯一の更新点（design Flow 1 キー決定）。
        // 手前の失敗経路はすべて early return 済みゆえ、失敗時は前 k・前表示が保たれる（要件 4.4）。

        // (3.5) 状態照合＝窓寸 reconcile 要求の生成（design Flow 1 キー決定・議題 #2 裁定）。
        //
        // **前値を上書きする前に**前回適用の物理寸を組み立てる。組み立ては契約式
        // `物理寸 == applied.scaled_extent(native_size)`（design §State Management）に従う——別フィールドで
        // 物理寸を二重に持つと更新点が 2 つになり、片方だけ書かれる欠陥（本 spec で既出）を招く。両者は
        // 表示成立点で必ず揃って更新されるため、この導出は常に「前回この経路が表示へ載せた物理寸」に一致する
        // （`resample` の事後条件が `出力外形 == scaled_extent(入力外形)` ゆえ `chain.size()` と厳密に等しい）。
        //
        // 前値なし（初回表示）は `None` ≠ `Some(size)` ゆえ**必ず差分扱い**になる。これは意図した設計である
        // ——窓は起動時 k₀ 見積もり寸で生成されており実窓 DPI 由来の k と一致する保証がないため、初回を
        // 黙らせると Flow 3 手順 5 の補正が永久に走らない。
        let prev_physical = target
            .applied
            .zip(target.native_size)
            .map(|(k, (nw, nh))| k.scaled_extent(nw, nh));
        let size_changed = prev_physical != Some(size);
        if size_changed {
            // 差分あり＝呼び手（frame drain フェーズ）へ新物理寸を報告する。同寸のときは**何も触らない**
            // ——`None` を書き戻すと未消費の要求を殺してしまう（取りこぼしを作らない・べき等）。
            target.pending_resize = Some(size);
        }

        target.applied = Some(scale);
        // いま表示に使ったエントリ由来の原寸をそのまま写す（合成した回か否かで分岐しない——分岐させると
        // 「insert 済みのまま失敗 → 後からヒットで成立」の経路で照会値が画面と乖離する）。
        target.native_size = target.cached_native;
        target.last_show = Some((surface_id, binds, pattern));

        // 表示成立点の観測ログ（設計 D10・要件 6.1/6.3 の判定素材）。実機サインオフは有界 auto-exit で
        // 起動し `RUST_LOG` を grep してここを読むため、**`info!` レベル**であることが契約である
        // （`debug!` へ落とすと既定の観測条件で消える）。k 導出値（`k`・`k_ratio`）と適用寸（`native_*`・
        // `scaled_*`）が揃うことで、2 水準（125%/200%）の実行が「異なる物理寸で描かれた」ことを
        // ログだけで決定論的に判定できる。
        //
        // `native_*` の供給源 `native_size` は直上で `cached_native` から写しており、スロットと対の
        // 不変条件によりこの経路では必ず `Some` である（引き当てが成立した＝スロットに中身がある）。
        // 万一崩れた場合の `0×0` は**実在し得ない外形**（0 外形は上流 `EmptyComposition` が先行遮断する）
        // ゆえ、値を捏造せず「対が壊れた」ことを示す診断番兵として機能する。
        let (native_w, native_h) = target.native_size.unwrap_or((0, 0));
        tracing::info!(
            ?target_id,
            surface_id,
            cache_hit,
            // k の有理表現（既約 num/den）。`ScaleRatio` の num/den は非公開ゆえ `Debug` で出す。
            k_ratio = ?scale,
            k = scale.as_f32(),
            author_dpi = target.policy.author_dpi,
            // `None` は要件 1.4 の縮退（DPI component 不在 → k=1.0）そのものゆえ潰さずに出す。
            window_dpi = ?window_dpi,
            native_w,
            native_h,
            scaled_w = size.0,
            scaled_h = size.1,
            // 今回の表示成立が窓寸 reconcile 要求を積んだか（議題 #2 裁定の状態照合の観測点）。
            size_changed,
            "apply(ShowSurface): 表示・マスクを更新"
        );
        Self::reply(reply, Ok(()));
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

        target.cache.invalidate_all();
        // スロットと対の原寸も落とす（対を崩さない）。表示中の `native_size` は触らない——スロットが
        // 空でも画面には前回の絵が残っており、照会契約はその絵の原寸を返し続けるのが正しい（R4.3:
        // キャッシュ無効化は表示を変えない）。以後は必ずミス＝再合成が走り、対が再構築される。
        target.cached_native = None;
        tracing::debug!(?target_id, "apply(InvalidateCache): キャッシュ全破棄（表示は継続）");
        Self::reply(reply, Ok(()));
    }

    /// target の予約 text 層スロットへの読み取り専用の到達手段（mount 未生成なら `None`・R9.1/9.2）。
    ///
    /// mount（と供給面）は初回 `ShowSurface` で原寸確定後に遅延生成されるため、未登録 target・
    /// 初回表示確立前は取得不可（`None`）である。呼び手（結線側）は表示確立後に取得するか再取得を
    /// 試みる。返る値はスナップショット（読み取り専用 view）で、スロット状態は変更できない。
    ///
    /// # 取得条件（k 導入後・要件 1.2）
    ///
    /// mount／供給面の存在に加えて **表示が一度成立していること**（`applied`／`native_size` が確定
    /// していること）を条件とする。`scale()` は実適用 k、`surface_size()` は native 原寸を返す契約で
    /// あり、いずれも表示成立点でしか確定しないためである——供給面だけ生成できて upload に失敗した
    /// ような中間状態で「k=1.0・供給面寸」という**実態のない値**を返さない（無言の縮退を作らない）。
    pub fn text_slot_view(&self, target: TargetId) -> Option<TextSlotView> {
        let t = self.targets.get(&target)?;
        let mount = t.mount.as_ref()?;
        // 供給面の遅延生成前は表示未確立（既存契約の維持）。
        t.chain.as_ref()?;
        // 実適用 k と native 原寸は表示成立点でのみ確定する（照会値＝実適用値の担保）。
        let applied = t.applied?;
        let surface_size = t.native_size?;
        Some(TextSlotView {
            slot: mount.text_slot(),
            window: t.window,
            surface_size,
            scale: applied.as_f32(),
        })
    }

    /// 下流照会契約（要件 1.2）: いま**実際に表示へ適用中**の k。
    ///
    /// 単一真実源は [`PresentTarget::applied`] で、その更新点は表示成立点ただ 1 箇所である——ゆえに本照会が
    /// 返す値は常に「いま画面に載っている絵に実際に掛かった k」であり、「導出したが適用に失敗した k」が
    /// 漏れることはない（要件 4.4 の失敗経路は `applied` を書かずに early return する）。
    ///
    /// 表示が一度も成立していない target・未登録 target は `None`。「まだ何も適用していない」を 1.0 で
    /// 塗り潰さない（1.0 は等倍という**適用結果**であって未確定の別名ではない）。
    ///
    /// 同じ値は [`TextSlotView::scale`] からも読める（あちらは text 層向けにスロット情報と束ねた
    /// スナップショット経路）。下流 `areka-P0-collision-dpi-hittest` の点÷k はこの値を参照してよい。
    pub fn applied_scale(&self, target: TargetId) -> Option<f32> {
        Some(self.targets.get(&target)?.applied?.as_f32())
    }

    /// 窓 DPI 変化に伴う再スケール（要件 4.1-4.4・design Flow 2）。
    ///
    /// 窓の現 `DPI` から k を再導出し、**前回適用 k と異なり・可視であり・再表示入力を保持している**
    /// ときだけ内部で `ShowSurface` を再実行する。表示物理寸が変われば `Some(新物理寸)` を返し、呼び手
    /// （`run_dpi_phase`）が**同一フレーム・同一 UI スレッド呼出**で窓寸 reconcile（char=`resize_window_to`
    /// ／balloon=`resize_window_keep_position`）を行う——完了後に照会値・表示寸・窓 client が揃う（要件 4.2）。
    ///
    /// # ゲート（いずれか不成立なら `None`・副作用なし）
    ///
    /// - **未登録** target。
    /// - **k 不変**: 再導出値が `applied` と等しい。`Changed<DPI>` の初回 run が全窓にマッチする仕様
    ///   （`SystemState::new`）はここで吸収される（`anchor_changed_system` と同じ流儀）。
    /// - **不可視**: `Hide`／全透明退化で消えている target を再表示で**蘇らせない**。DPI 変化は
    ///   「見えているものを描き直す」事象であって、表示を復活させる事象ではない。
    /// - **再表示入力なし**（`last_show` が `None`）: 一度も表示が成立していない。
    ///
    /// # k 導出の権威は `apply_show` 側にある
    ///
    /// ここでの [`derive_scale`] 呼出は**ゲート判定の述語**であり、実際に適用される k は `apply_show` が
    /// 自前で導出したものである（漏斗を二重化しない・design Flow 1「k 導出は show 適用ごと」）。両者は
    /// 同一の純関数へ同一入力を与える（同一 UI スレッド内・間に World 変更なし）ため必ず一致し、その一致は
    /// 再表示後の `applied` 照合で**実際に検査される**（食い違えば失敗として扱われ黙って通らない）。
    ///
    /// # [`Self::take_pending_resize`] との関係（二重 resize も取りこぼしもしない）
    ///
    /// タスク 4.2 の結線は `run_dpi_phase`（本メソッド）と drain フェーズ（`take_pending_resize`）の
    /// **両方**を毎フレーム呼ぶため、両者の責任範囲を重ねない:
    ///
    /// - **再表示して成立した**場合: その表示成立が積んだ要求を本メソッドが**取り出して**返す。ゆえに
    ///   同一フレームの drain フェーズが `take_pending_resize` を呼んでも同じ要求は二度出ない。
    /// - **ゲート不成立で再表示しなかった**場合: `pending_resize` に**一切触れない**。未消費の要求
    ///   （例: 初回表示が積んだ k₀ 補正）は drain フェーズがそのまま拾う。
    /// - **再表示が失敗した**場合: 同じく触れずに `None` を返す（前 k・前表示・未消費要求すべて維持）。
    ///
    /// # 失敗（要件 4.4）
    ///
    /// 再表示が表示成立に至らなければ `error!` を出して `None` を返し、**直前の k による表示を維持**する
    /// （`apply_show` が表示成立点より手前で early return するため前値は構造的に保たれる）。`apply_show`
    /// 自身も失敗を error! するが、それは「合成／デバイスが失敗した」ことしか語らない——どの k からどの k
    /// への再導出が落ちたのか・前表示を維持したのかは本メソッドでしか分からないため、専用のログを出す
    /// （無言の失敗経路を作らない）。全透明退化（`EmptyComposition` → Hide 縮退）は設計上許容された正常
    /// 退化ゆえ `apply_show` の `warn!` に委ね、ここでは `debug!` に留める（同一事象を二重に鳴らさない）。
    ///
    /// 進行中の talk 再生・SERIKO ループは presenter の**外**に状態を持つため、再表示はキャッシュミス 1 回の
    /// コストで済み挙動を失わない（要件 4.3）。本メソッドは target 状態を一切リセットしない。
    pub fn refresh_scale(&mut self, world: &mut World, target_id: TargetId) -> Option<(u32, u32)> {
        // ゲート判定に要る値を先に取り出して借用を閉じる（以降 `apply_show` が `&mut self` を要する）。
        let (window, policy, previous, visible, last_show) = {
            let t = self.targets.get(&target_id)?;
            (
                t.window,
                t.policy,
                t.applied,
                t.visible,
                t.last_show.clone(),
            )
        };

        // 窓 DPI は `apply_show` と同一経路で読む。**component 不在を 96 で捏造しない**——捏造すると
        // 要件 1.4 の縮退（error! ＋ k=1.0）が「正常系のふり」で通る。`None` のまま渡して縮退させる。
        let window_dpi = world.get::<DPI>(window).map(|d| (d.dpi_x, d.dpi_y));
        let scale = derive_scale(policy, window_dpi);

        if previous == Some(scale) {
            tracing::trace!(?target_id, k_ratio = ?scale, "refresh_scale: k 不変（再表示しない）");
            return None;
        }
        if !visible {
            tracing::debug!(
                ?target_id,
                "refresh_scale: 不可視ゆえ再表示しない（Hide/全透明退化を蘇らせない）"
            );
            return None;
        }
        let Some((surface_id, binds, pattern)) = last_show else {
            tracing::debug!(
                ?target_id,
                "refresh_scale: 再表示入力なし（表示が一度も成立していない）"
            );
            return None;
        };

        // 表示更新は既存の単一漏斗をそのまま通す（`reply` なし＝内部再実行・design Flow 2）。成立点の記録・
        // 失敗時の early return・D10 ログ・状態照合報告はすべて `apply_show` 側の不変条件がそのまま効く。
        self.apply_show(world, target_id, surface_id, binds, pattern, None);

        let t = self.targets.get(&target_id)?;
        if t.applied != Some(scale) {
            // 表示成立に至らなかった。前 k・前表示は `apply_show` の early return が保っている（要件 4.4）。
            if t.visible {
                tracing::error!(
                    ?target_id,
                    k_ratio_from = ?previous,
                    k_ratio_to = ?scale,
                    window_dpi = ?window_dpi,
                    "refresh_scale: 再表示が成立せず（直前の k による表示を維持）"
                );
            } else {
                // 全透明退化（`EmptyComposition` → Hide 縮退）。`apply_show` が warn! 済みゆえ重ねない。
                tracing::debug!(
                    ?target_id,
                    "refresh_scale: 再表示が全透明退化（Hide 縮退・warn は apply_show 側）"
                );
            }
            return None;
        }

        // 成立: 状態照合が積んだ要求をここで消費して返す（drain フェーズと二重に出さない）。物理寸が
        // 変わらなければ `None`＝窓寸 reconcile 不要（k だけ変わって丸め後の寸が同じ場合が実在する）。
        self.take_pending_resize(target_id)
    }

    /// 表示成立点の状態照合が積んだ**窓寸 reconcile 要求**を取り出す（取り出しで消える・drain 契約）。
    ///
    /// `Some(新物理寸)` は「直近の表示成立で物理寸が前回適用寸から変わった（初回表示を含む）」ことを
    /// 表す。呼び手（emo2_boot の frame drain フェーズ）は**同一フレーム内**で char 窓なら
    /// `resize_window_to`（アンカー保存）・balloon 窓なら `resize_window_keep_position` を呼び、窓
    /// client を新物理寸へ合わせる（design Flow 2／Flow 3 手順 5・議題 #2 裁定）。未登録 target・
    /// 要求なしは `None`。
    ///
    /// # なぜ `reply`（[`PresentOutcome`]）ではなくここに置くのか
    ///
    /// 本番の drain 経路（`run_drain_phase`）は指令へ `reply` を**同梱しない**（撃ちっぱなし）ため、
    /// [`PresentOutcome`] を太らせても報告は呼び手へ届かない。加えて報告は「表示が成立したという
    /// **状態**」であり、エッジ（`Changed<DPI>`）の消費順序に依存してはならない（議題 #2 裁定）。
    /// ゆえに target ごとの**取り出し可能な状態**として置く。
    ///
    /// # 取りこぼさない（未消費なら保持）
    ///
    /// 要求は取り出されるまで消えない。呼び手が或るフレームで取り出さなくても次に取り出した者が最新の
    /// 物理寸を受け取るため、報告が黙って失われる経路が無い。逆に取り出した後は同寸表示を何度繰り返しても
    /// `None` のままで、窓へ無用な書込（churn）を誘発しない。
    pub fn take_pending_resize(&mut self, target: TargetId) -> Option<(u32, u32)> {
        self.targets.get_mut(&target)?.pending_resize.take()
    }

    /// target がいま表示しているサーフェス id（CurrentSurfaceRead・R3.1-3.3）。
    ///
    /// 「最後に表示が成立したサーフェス id」を返す（画面の絵ではなく表示成立の結果・α 非依存）。
    /// 未表示（一度も `ShowSurface` していない）・`Hide` 済み・空合成へ縮退した場合、および未登録
    /// target は `None`。単一真実源は `PresentTarget.current_surface_id`（`ComposeKey` から導出しない・
    /// design §CurrentSurfaceRead State Management）。既存の表示ロジックへ分岐を足さない additive な
    /// 読み取りのみ（R3.4）。
    pub fn current_surface_id(&self, target: TargetId) -> Option<u32> {
        self.targets.get(&target)?.current_surface_id
    }

    /// 現サーフェスの当たり判定領域名を解決する（`current_surface_id` → `EmoWorld::surface` → 純関数・R4.1/4.4）。
    ///
    /// 座標は **native サーフェス px**（k 適用前の合成座標系）で解釈される。窓 client 物理 px は k 倍
    /// された座標系ゆえ、k≠1.0 では呼び手が渡す前に ÷k する必要がある——**その変換は本メソッドの責務
    /// ではなく**、下流 `areka-P0-collision-dpi-hittest`（W5）の領分である（要件 7.9: 本仕様は当たり
    /// 判定の点÷k・ヒット規約を変更しない）。k=1.0 の窓では両座標系が一致するため、本メソッドの挙動は
    /// k 導入の前後で完全に不変である。現サーフェス無し（未表示／`Hide`／空合成
    /// 縮退／未登録 target）は `None`（R4.4）。重なりは画家のアルゴリズム（後定義が手前・[`RegionPriority::Painter`]）で
    /// 解決する。`EmoWorld` を presenter 外へ露出しない（`&SurfaceMaster` を外へ出さない）ため純関数
    /// [`areka_emo_compose::hit_region`] の呼出は本メソッド内で閉じ、戻り値の寿命は `&self` に従う
    /// （マウス移動ごとの割当を生まない・design §CurrentSurfaceRead Service Interface）。
    pub fn hit_region(&self, target: TargetId, x: i64, y: i64) -> Option<&str> {
        let t = self.targets.get(&target)?;
        let master = t.emo_world.surface(t.current_surface_id?)?;
        areka_emo_compose::hit_region(master, x, y, RegionPriority::Painter)
    }

    /// target の表示中画素を CPU へ読み戻す（R6.2/R8.3・検証・将来の直読みヒットテスト基盤）。
    ///
    /// 未装着、または供給面未生成（未表示）なら [`PresentError::TargetNotAttached`] を返す。
    pub fn read_back(&self, target: TargetId) -> Result<Vec<u8>, PresentError> {
        let Some(t) = self.targets.get(&target) else {
            tracing::error!(?target, "read_back: 未装着ターゲット");
            return Err(PresentError::TargetNotAttached(target));
        };
        match t.chain.as_ref() {
            Some(chain) => chain.read_back(),
            None => {
                tracing::error!(?target, "read_back: 供給面が未生成（未だ表示していない）");
                Err(PresentError::TargetNotAttached(target))
            }
        }
    }

    /// reply 同梱時に結果を高々 1 回送る。受信端が既に drop 済みなら撃ちっぱなし扱い（debug ログ）。
    fn reply(reply: Option<ReplySender<PresentOutcome>>, outcome: PresentOutcome) {
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;
    use std::time::Duration;

    use areka_actor::reply_channel;
    use areka_emo_atlas::{
        AlphaParams, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
    };
    use areka_emo_compose::{BindSet, ComposeMethod, PatternFrame};
    use areka_parsers::shell::{
        Animation, AppendTarget, DefRef, DrawMethod, Element, ElementPath, Interval, Pattern, Shell,
        Surface,
    };

    use wintf::ecs::{GraphicsCore, HitTest, HitTestMode, Visual, WucGraphicsResource};
    use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};

    // ── GPU/WUC フィクスチャ（chain.rs / mount.rs / wuc_resource.rs テストと同一方針）──────────
    // 本番 UI スレッドは MTA（メモリ「areka WUC は MTA スレッドで動く」）。WucGraphicsResource::new は
    // DQTAT_COM_NONE（apartment 不変）でディスパッチャを組むため、COM を MTA 初期化してから呼ぶ。

    /// GraphicsCore ＋ WucGraphicsResource を実資源として載せた wintf World を組む。
    ///
    /// `EmoPresenter` は供給面生成時に World から両資源を読む（compositor は `WucGraphicsResource` 由来）。
    /// ゆえに本番同様、World へ両者を挿入した状態を作る。
    fn make_world_with_gpu() -> World {
        // 各テストは専用スレッドで走る。MTA を初期化（S_FALSE/RPC_E_CHANGED_MODE は無視）。
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let core = GraphicsCore::new().expect("GraphicsCore::new 失敗（HARDWARE デバイス生成）");
        let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
        let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");

        let mut world = World::new();
        world.insert_resource(core);
        world.insert_resource(wuc);
        world
    }

    /// 窓 entity を **`DPI` component 付き**で作る（design「Testing Strategy > Integration Tests」の
    /// テスト World 前提）。
    ///
    /// 本番の窓生成は必ず `DPI` を付与する（wintf が `GetDpiForWindow` の実値で補正する）。テストで
    /// component を省くと要件 1.4 の縮退（error! ＋ k=1.0）が「正常系のふり」で緑になってしまうため、
    /// **明示挿入を規律とする**。96 挿入＝恒等 k、192 挿入＝k=2/1。縮退分岐そのものは
    /// `show_surface_without_dpi_component_degrades_to_identity`（DPI 不在専用テスト）で檻に入れる。
    fn spawn_window_with_dpi(world: &mut World, dpi: u16) -> Entity {
        world.spawn(DPI::from_dpi(dpi, dpi)).id()
    }

    /// 窓 entity の `DPI` component を差し替える（モニタ跨ぎ移動・表示スケール変更の決定論的代替）。
    fn set_window_dpi(world: &mut World, window: Entity, dpi: u16) {
        world.entity_mut(window).insert(DPI::from_dpi(dpi, dpi));
    }

    /// `build_target_assets` と同一入力の **native 合成結果を `scale` 倍**した表示用サーフェスの
    /// バイト列（k≠1 表示の golden）。
    ///
    /// presenter が辿るのと同じ `Composer::compose`（native）→ `resample`（k 適用）の 2 段を、
    /// テスト側で独立に再現する。「readback が偶然それらしい寸法になった」ではなく
    /// **k 適用後のバイトそのもの**を固定する。
    fn scaled_golden(
        emo_world: &EmoWorld,
        atlas: &AtlasTable,
        surface_id: u32,
        scale: ScaleRatio,
    ) -> (Vec<u8>, (u32, u32), (u32, u32)) {
        let mut composer = Composer::new();
        let native = composer
            .compose(
                emo_world,
                atlas,
                surface_id,
                &BindSet::default(),
                &PatternState::default(),
            )
            .expect("golden 用の native 合成は Ok");
        let native_size = (native.width(), native.height());
        let mut scaled = ComposedSurface::new(0, 0);
        resample(&native, scale, &mut scaled);
        let scaled_size = (scaled.width(), scaled.height());
        (scaled.bytes().to_vec(), native_size, scaled_size)
    }

    /// 有効 `ShowSurface` を適用し、reply が `Ok(())` であることを確認する（テスト補助）。
    fn show_ok(presenter: &mut EmoPresenter, world: &mut World, target: TargetId, surface_id: u32) {
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            world,
            PresentCommand::ShowSurface {
                target,
                surface_id,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "ShowSurface（surface {surface_id}）が Ok でない"
        );
    }

    // ── ComposedSurface 生成補助（chain.rs テストと同技法）──────────────────────────────────
    // `ComposedSurface::bytes_mut` は emo-compose の pub(crate) ゆえ本クレートから画素を直接焼けない。
    // 上流公開 API（atlas bake → EmoWorld → Composer::compose）で本物を合成して得る。

    fn elem(path: &str, x: i64, y: i64) -> Element {
        Element {
            layer: 0,
            path: ElementPath::new(path.to_string()),
            x,
            y,
        }
    }

    fn surface(id: u32, elements: Vec<Element>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements,
            collisions: Vec::new(),
            animations: Vec::new(),
        }
    }

    fn shell_of(surfaces: Vec<Surface>) -> Shell {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions,
        }
    }

    /// surface 1000 = 単一 element（`w×h` 全不透明・座標由来グラデーション）の `(EmoWorld, AtlasTable)`
    /// と、同一入力を `Composer::compose` で直接合成した golden バイト列を返す。
    ///
    /// α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は正確に `w×h`。golden は presenter が
    /// 内部で辿るのと同一の world/atlas から作るため、readback とのバイト一致が二重に決定論的になる。
    fn build_target_assets(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable, Vec<u8>) {
        let base = Path::new("shell/master");
        let surfaces = vec![surface(1000, vec![elem("p.png", 0, 0)])];

        let mut dec = MemoryDecoder::new();
        let stride = w * 4;
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let a: u8 = 0xFF;
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.push(b);
                img.push(g);
                img.push(r);
                img.push(a);
            }
        }
        dec.insert(base.join("p.png"), w, h, stride, img, true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        let atlas = baked.table;

        // golden: presenter と同一入力を直接合成（move 前に計算する）。
        let mut composer = Composer::new();
        let golden = composer
            .compose(&world, &atlas, 1000, &BindSet::default(), &PatternState::default())
            .expect("静的 element 単体の合成は Ok");
        let golden_bytes = golden.bytes().to_vec();

        (world, atlas, golden_bytes)
    }

    /// R2.4/R3.2/R8.2 観測完了（golden 一致）: `attach_target` → `apply(ShowSurface 有効 id)` で reply が
    /// `Ok(())`、かつ `read_back` が同一入力の直接合成 golden と**全バイト一致**する。
    ///
    /// 供給面は D2D 非経由の純バイト転送ゆえ、readback と `ComposedSurface.bytes()` のバイト一致が
    /// 決定論的に成立する（WARP でも可＝CI 決定論）。
    #[test]
    fn golden_match_read_back_equals_direct_compose() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);

        let (emo_world, atlas, golden) = build_target_assets(3, 2, 0x11);
        assert!(golden.iter().any(|&b| b != 0), "golden は非退化（全 0 でない）");

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );

        let outcome = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("reply（ShowSurface）を受信できない");
        assert!(
            matches!(outcome, Ok(())),
            "有効 id の ShowSurface は Ok を返す: {outcome:?}"
        );

        let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
        assert_eq!(
            rb, golden,
            "readback が直接合成 golden とバイト一致しない（表示・供給面の恒等転送が壊れている）"
        );
    }

    /// R3.4 観測完了（表示不変）: 有効 id で表示を確立後、**解決不能 id** の `ShowSurface` は reply が
    /// `Err(Compose(SurfaceNotFound))` で、`read_back` バイトは**適用前と不変**（表示を乱さない）。
    #[test]
    fn invalid_surface_id_replies_err_and_leaves_display_unchanged() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);

        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x5A);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // まず有効 id で表示を確立（供給面生成＋表示バイト確定）。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );
        let before = presenter.read_back(TargetId(0)).expect("read_back（前）失敗");

        // 解決不能 id: error! ＋ 表示不変 ＋ reply Err（R3.4）。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 9999,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx1),
            },
        );
        let outcome = rx1
            .recv_timeout(Duration::from_secs(10))
            .expect("reply（無効 id）を受信できない");
        assert!(
            matches!(
                outcome,
                Err(PresentError::Compose(ComposeError::SurfaceNotFound(9999)))
            ),
            "無効 id は Err(Compose(SurfaceNotFound(9999))) を返す: {outcome:?}"
        );

        let after = presenter.read_back(TargetId(0)).expect("read_back（後）失敗");
        assert_eq!(
            before, after,
            "無効 id の適用で表示中バイトが変化した（表示不変の不変条件を破っている）"
        );
    }

    /// 有効 surface 1000（`w×h` 全不透明 element）＋ 定義層皆無で外形 0×0 に退化する surface 7000
    /// （element/animation ゼロ）を**同一 target**へ載せる `(EmoWorld, AtlasTable)` を返す。
    ///
    /// surface 7000 は composer_tests.rs の `no_layers_degenerate_propagates_empty_composition`
    /// と同型の構成（bind 済み world に存在するが合成外形 0×0）で、`Composer::compose` が
    /// `Err(ComposeError::EmptyComposition(7000))` を返す。単なる全透明 element は非ゼロ外形の
    /// `Ok`（`all_transparent_surface_is_ok_transparent_nonzero_extent`）で EmptyComposition では
    /// ないため、退化は「定義層皆無 → 0×0」の経路で作る。
    fn build_assets_with_valid_and_empty(w: u32, h: u32, salt: u8) -> (EmoWorld, AtlasTable) {
        let base = Path::new("shell/master");
        // surface 1000: 全不透明 element 1 本。surface 7000: element/animation ゼロ（定義層皆無）。
        let surfaces = vec![
            surface(1000, vec![elem("p.png", 0, 0)]),
            surface(7000, Vec::new()),
        ];

        let mut dec = MemoryDecoder::new();
        let stride = w * 4;
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let a: u8 = 0xFF;
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.push(b);
                img.push(g);
                img.push(r);
                img.push(a);
            }
        }
        dec.insert(base.join("p.png"), w, h, stride, img, true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        (world, baked.table)
    }

    /// R3.4 観測完了（skip＋表示不変の回帰檻）: 有効 id で表示・マスクを確立後、**解決不能 id** の
    /// `ShowSurface` は reply が `Err(Compose(SurfaceNotFound))`、かつ (a) `read_back` バイト、
    /// (b) surface entity の `HitTest`（`AlphaMask`）、(c) `AlphaMaskResource`（設定済みマスク）の
    /// いずれも**適用前と不変**（表示＋マスクを一切乱さない）。
    ///
    /// 4.1 の `invalid_surface_id_replies_err_and_leaves_display_unchanged` はバイト不変のみを見るが、
    /// 本テストは「skip＝表示器（visual/mask/hit-test）を触らない」を独立・自己完結に固定する。
    #[test]
    fn invalid_surface_skips_and_leaves_display_and_mask_unchanged() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);

        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x37);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 有効 id で表示・マスク・hit-test を確立。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );

        // 表示器の適用前状態を捕捉（bytes ＋ HitTest ＋ mask 寸法/有無）。
        let surface_entity = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("有効表示後は mount が生成済み")
            .surface_entity();

        let bytes_before = presenter.read_back(TargetId(0)).expect("read_back（前）失敗");
        let hit_before = world
            .get::<HitTest>(surface_entity)
            .expect("surface entity に HitTest が無い")
            .mode;
        assert_eq!(hit_before, HitTestMode::AlphaMask, "有効表示後は αマスク判定");
        let mask_dims_before = world
            .get::<AlphaMaskResource>(surface_entity)
            .and_then(|r| r.mask().map(|m| (m.width(), m.height())));
        assert!(mask_dims_before.is_some(), "有効表示後は AlphaMask が供給済み");
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "有効表示後は可視"
        );

        // 解決不能 id: error! ＋ skip（表示器不触）＋ reply Err（R3.4）。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 4242,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx1),
            },
        );
        let outcome = rx1
            .recv_timeout(Duration::from_secs(10))
            .expect("reply（無効 id）を受信できない");
        assert!(
            matches!(
                outcome,
                Err(PresentError::Compose(ComposeError::SurfaceNotFound(4242)))
            ),
            "無効 id は Err(Compose(SurfaceNotFound(4242))) を返す: {outcome:?}"
        );

        // (a) 表示バイト不変。
        let bytes_after = presenter.read_back(TargetId(0)).expect("read_back（後）失敗");
        assert_eq!(
            bytes_before, bytes_after,
            "無効 id の skip で表示中バイトが変化した（表示を乱さない不変条件違反）"
        );
        // (b) HitTest 不変（None へ落ちていない＝当たり判定が生きたまま）。
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::AlphaMask,
            "無効 id の skip で HitTest が変化した（マスク/当たり判定を乱している）"
        );
        // (c) AlphaMaskResource 不変（供給済みマスクが消えていない）。
        let mask_dims_after = world
            .get::<AlphaMaskResource>(surface_entity)
            .and_then(|r| r.mask().map(|m| (m.width(), m.height())));
        assert_eq!(
            mask_dims_before, mask_dims_after,
            "無効 id の skip で AlphaMaskResource が変化した（マスクを乱している）"
        );
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "無効 id の skip で可視状態が変化した（表示を乱している）"
        );
    }

    /// 設計ディスカッション #1 観測完了（EmptyComposition → Hide 縮退＋reply Ok）: 有効表示で mount を
    /// 確立後、**外形 0×0 に退化する既存 surface**（定義層皆無）を `ShowSurface` すると reply は
    /// **`Ok(())`**（`Err` ではない）で、target は Hidden へ縮退（`Visual` 不可視＋`HitTest::none()`）し、
    /// 0×0 供給面を作ろうとして panic しない（既存 chain は破棄されず保持）。
    ///
    /// 前段で `Composer::compose(7000)` が `EmptyComposition(7000)` を返すことを直接確認し、退化経路が
    /// 「不在 surface（SurfaceNotFound）」ではなく「存在するが 0×0」であることを固定する。
    #[test]
    fn empty_composition_degrades_to_hidden_and_replies_ok() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);

        let (emo_world, atlas) = build_assets_with_valid_and_empty(5, 4, 0x22);

        // 前提固定: 7000 は「存在するが外形 0×0」＝EmptyComposition（SurfaceNotFound ではない）。
        {
            let mut composer = Composer::new();
            let direct = composer.compose(&emo_world, &atlas, 7000, &BindSet::default(), &PatternState::default());
            assert_eq!(
                direct.err(),
                Some(ComposeError::EmptyComposition(7000)),
                "surface 7000 は定義層皆無で EmptyComposition を返す前提でなければならない"
            );
        }

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 有効 1000 で mount/chain を確立し可視化。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );
        let surface_entity = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("有効表示後は mount が生成済み")
            .surface_entity();
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "有効表示後は可視"
        );
        let bytes_len_before = presenter
            .read_back(TargetId(0))
            .expect("read_back（前）失敗")
            .len();

        // EmptyComposition 退化: warn! ＋ Hide 縮退 ＋ reply Ok（skip でも Err でもない）。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 7000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx1),
            },
        );
        let outcome = rx1
            .recv_timeout(Duration::from_secs(10))
            .expect("reply（EmptyComposition）を受信できない");
        assert!(
            matches!(outcome, Ok(())),
            "EmptyComposition は Hide 縮退＋reply Ok（Err ではない）: {outcome:?}"
        );

        // Hidden へ縮退: Visual 不可視 ＋ HitTest::none()。
        assert!(
            !world.get::<Visual>(surface_entity).unwrap().is_visible,
            "EmptyComposition は Hidden へ縮退（Visual 不可視）でなければならない"
        );
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::None,
            "EmptyComposition は当たり判定停止（HitTest::none）でなければならない"
        );
        assert!(
            !presenter.targets.get(&TargetId(0)).unwrap().visible,
            "EmptyComposition 後は target.visible=false"
        );

        // 0×0 供給面は作らない: 既存 chain は破棄されず保持（read_back は旧外形の長さのまま成立）。
        let bytes_len_after = presenter
            .read_back(TargetId(0))
            .expect("EmptyComposition 後も既存 chain は保持され read_back できる")
            .len();
        assert_eq!(
            bytes_len_before, bytes_len_after,
            "EmptyComposition で 0×0 chain へ差し替わった（既存 chain 保持の不変条件違反）"
        );
        // 7000 は非合成ゆえキャッシュへ載らない（0×0 を挿入しない）。
        assert!(
            presenter
                .targets
                .get(&TargetId(0))
                .unwrap()
                .cache
                .get(7000, &BindSet::default(), &PatternState::default(), ScaleRatio::ONE)
                .is_none(),
            "EmptyComposition は cache へ 0×0 を挿入しない"
        );
    }

    /// R3.3 観測完了（Hide → 再 ShowSurface 復帰）: 有効表示 → `Hide`（不可視＋`HitTest::none()`＋
    /// chain/cache 保持）→ 同一有効 id を再 `ShowSurface` で表示復帰（可視＋`HitTest::alpha_mask()`）。
    /// 再表示はキャッシュヒットで再合成せず、`read_back` が初回表示バイトと一致する（キャッシュからの復帰）。
    #[test]
    fn hide_then_reshow_recovers_display_from_cache() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);

        let (emo_world, atlas, _golden) = build_target_assets(6, 5, 0x4D);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 初回表示（可視・αマスク判定確立）。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "初回 ShowSurface が Ok でない"
        );
        let surface_entity = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("初回表示後は mount が生成済み")
            .surface_entity();
        let bytes_shown = presenter.read_back(TargetId(0)).expect("read_back（初回）失敗");
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::AlphaMask,
            "初回表示後は αマスク判定"
        );

        // Hide: 不可視 ＋ HitTest::none() ＋ chain/cache 保持。
        let (txh, rxh) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::Hide {
                target: TargetId(0),
                reply: Some(txh),
            },
        );
        assert!(
            matches!(rxh.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "Hide が Ok でない"
        );
        assert!(
            !world.get::<Visual>(surface_entity).unwrap().is_visible,
            "Hide 後は Visual 不可視"
        );
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::None,
            "Hide 後は当たり判定停止（HitTest::none）"
        );
        {
            let target = presenter.targets.get(&TargetId(0)).unwrap();
            assert!(target.chain.is_some(), "Hide は swap chain を保持する（R3.3）");
            assert!(
                target.cache.get(1000, &BindSet::default(), &PatternState::default(), ScaleRatio::ONE).is_some(),
                "Hide は合成キャッシュを保持する（R3.3）"
            );
            assert!(!target.visible, "Hide 後は target.visible=false");
        }

        // 再 ShowSurface（同一有効 id）: キャッシュヒットで再合成せず表示復帰。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx1),
            },
        );
        assert!(
            matches!(rx1.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "再 ShowSurface が Ok でない"
        );

        // 表示復帰: 可視 ＋ αマスク判定へ戻る。
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "再表示後は可視へ復帰"
        );
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::AlphaMask,
            "再表示後は αマスク判定へ復帰"
        );
        assert!(
            presenter.targets.get(&TargetId(0)).unwrap().visible,
            "再表示後は target.visible=true"
        );

        // 観測可能な復帰: read_back が初回表示バイトと一致（キャッシュからの復帰）。
        let bytes_reshown = presenter.read_back(TargetId(0)).expect("read_back（再表示）失敗");
        assert_eq!(
            bytes_shown, bytes_reshown,
            "再表示のバイトが初回表示と一致しない（キャッシュからの表示復帰が壊れている）"
        );
    }

    /// R9.1/9.2 観測完了（mount 未生成＝取得不可）: 未登録 target・登録済みだが初回 `ShowSurface` 前
    /// （mount 遅延生成前）のいずれも `text_slot_view` が `None` を返す（取得結果が空）。
    ///
    /// mount 未生成経路は World に GPU 資源を要しない（`attach_target` は skeleton 登録のみ）ため、
    /// 素の `World` で決定論的に固定する。
    #[test]
    fn text_slot_view_is_none_before_display_established() {
        let mut world = World::new();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x66);

        let mut presenter = EmoPresenter::new();
        // 未登録 target: 取得結果は空。
        assert!(
            presenter.text_slot_view(TargetId(0)).is_none(),
            "未登録 target の text_slot_view は None"
        );

        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
        // 登録済みでも初回 ShowSurface 前（mount 未生成）は空（design: mount は遅延生成・R9.2）。
        assert!(
            presenter.text_slot_view(TargetId(0)).is_none(),
            "初回 ShowSurface 前（mount 未生成）の text_slot_view は None"
        );
    }

    /// R9.1/9.2 観測完了（表示確立後の正値）: 有効 `ShowSurface` で表示確立後、`text_slot_view` が
    /// `Some` を返し、(a) `slot()` ＝ mount の予約スロット（`Name("emo-text-layer-slot")` を持つ）、
    /// (b) `window()` ＝ 装着先窓 Entity、(c) `surface_size()` ＝ バルーン/シェル surface の物理 px 原寸、
    /// (d) `scale()` ＝ 現行の物理 1:1 表示契約の恒常値 1.0、をすべて満たす。
    #[test]
    fn text_slot_view_returns_slot_window_size_scale_after_display() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);

        let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x77);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );

        let view = presenter
            .text_slot_view(TargetId(0))
            .expect("表示確立後の text_slot_view は Some");

        // (a) slot ＝ mount の予約スロット（Name で二重に裏取り）。
        let expected_slot = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("表示確立後は mount が生成済み")
            .text_slot();
        assert_eq!(view.slot(), expected_slot, "slot() が予約スロット entity と一致しない");
        let name = world
            .get::<bevy_ecs::name::Name>(view.slot())
            .expect("予約スロットに Name が無い");
        assert_eq!(name.as_str(), "emo-text-layer-slot");

        // (b) window ＝ attach_target で渡した装着先窓。
        assert_eq!(view.window(), window, "window() が装着先窓 entity と一致しない");

        // (c) surface_size ＝ 合成原寸（物理 px・本 fixture は 3×2）。
        assert_eq!(view.surface_size(), (3, 2), "surface_size() が物理原寸と一致しない");

        // (d) scale ＝ 現行契約の恒常値 1.0（物理 1:1・DPI 契約の共有点）。
        assert_eq!(view.scale(), 1.0, "scale() は現行契約で恒常 1.0");
    }

    /// surface 1000（`w×h` 全不透明 element ＋ bind animation 2000 が surface 5000 を (0,0) に重ねる）
    /// の `(EmoWorld, AtlasTable)` と、bind 無し／bind 有りそれぞれの直接合成 golden を返す。
    ///
    /// 5000 の part（1×1 不透明・base と異色）は base 内に収まるため、bind 有無で**外形は不変・
    /// バイトのみ変わる**（供給面リサイズ経路を踏まずに bind 差分の表示反映だけを固定できる）。
    fn build_target_assets_with_bind(
        w: u32,
        h: u32,
        salt: u8,
    ) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
        let base = Path::new("shell/master");
        let bind_part = Surface {
            id: 5000,
            targets: vec![AppendTarget::Single(5000)],
            elements: vec![elem("q.png", 0, 0)],
            collisions: Vec::new(),
            animations: Vec::new(),
        };
        let base_surface = Surface {
            id: 1000,
            targets: vec![AppendTarget::Single(1000)],
            elements: vec![elem("p.png", 0, 0)],
            collisions: Vec::new(),
            animations: vec![Animation {
                id: 2000,
                interval: Interval::Bind,
                patterns: vec![Pattern {
                    index: 0,
                    method: DrawMethod::new("overlay".to_string()),
                    surface_id: 5000,
                    wait: 0,
                    x: 0,
                    y: 0,
                }],
            }],
        };
        let surfaces = vec![base_surface, bind_part];

        let mut dec = MemoryDecoder::new();
        let stride = w * 4;
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.extend_from_slice(&[b, g, r, 0xFF]);
            }
        }
        dec.insert(base.join("p.png"), w, h, stride, img, true);
        // 1×1 の不透明 part（base 左上と必ず異なる色 → bind 有無でバイトが必ず変わる）。
        dec.insert(base.join("q.png"), 1, 1, 4, vec![0xFF, 0xFF, 0xFF, 0xFF], true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        let atlas = baked.table;

        let mut composer = Composer::new();
        let golden_plain = composer
            .compose(&world, &atlas, 1000, &BindSet::default(), &PatternState::default())
            .expect("bind 無し合成は Ok")
            .bytes()
            .to_vec();
        let golden_bound = composer
            .compose(&world, &atlas, 1000, &BindSet::from_ids([2000]), &PatternState::default())
            .expect("bind 有り合成は Ok")
            .bytes()
            .to_vec();
        assert_ne!(
            golden_plain, golden_bound,
            "fixture 前提: bind 有無で合成バイトが異ならなければ回帰檻にならない"
        );

        (world, atlas, golden_plain, golden_bound)
    }

    /// 回帰檻（キャッシュ仕様バグ・実表示レベル）: **同一 surface id で bind 集合だけ変えた**
    /// `ShowSurface` が必ず再合成され、`read_back` が各 bind 状態の直接合成 golden とバイト一致する。
    ///
    /// 旧設計（surface id のみキー）では 2 回目以降が古い合成にヒットし、着せ替え・まばたきの
    /// bind 差分が表示に反映されなかった（2026-07-09 まばたきデモで顕在化）。往復（無し→有り→無し）
    /// で両方向の再合成を固定する。
    #[test]
    fn bind_change_on_same_surface_updates_display() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);

        let (emo_world, atlas, golden_plain, golden_bound) =
            build_target_assets_with_bind(4, 3, 0x2B);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let show = |presenter: &mut EmoPresenter, world: &mut World, binds: BindSet| {
            let (tx, rx) = reply_channel::<PresentOutcome>();
            presenter.apply(
                world,
                PresentCommand::ShowSurface {
                    target: TargetId(0),
                    surface_id: 1000,
                    binds,
                    pattern: PatternState::default(),
                    reply: Some(tx),
                },
            );
            assert!(
                matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
                "ShowSurface が Ok でない"
            );
        };

        // bind 無し → golden_plain。
        show(&mut presenter, &mut world, BindSet::default());
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            golden_plain,
            "bind 無し表示が直接合成 golden と一致しない"
        );

        // 同一 surface・bind 有り → 再合成されて golden_bound（旧設計はここで古い絵を返した）。
        show(&mut presenter, &mut world, BindSet::from_ids([2000]));
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            golden_bound,
            "bind 追加が表示へ反映されない（合成入力キーの回帰＝着せ替えバグ再発）"
        );

        // bind 無しへ戻す → 再合成されて golden_plain（往復の両方向を固定）。
        show(&mut presenter, &mut world, BindSet::default());
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            golden_plain,
            "bind 除去が表示へ反映されない（合成入力キーの回帰＝着せ替えバグ再発）"
        );
    }

    /// surface 1000／3000 = 同 `w×h`・全不透明・**別バイト**（別 element・別 salt）を持つ単一
    /// world の `(EmoWorld, AtlasTable)` と、各面の直接合成 golden 2 本を返す（build_target_assets の
    /// 複面版）。
    ///
    /// 両面とも α=255（全不透明）ゆえ α=0 除外トリムは全域を残し、合成外形は両面とも正確に `w×h`
    /// （＝同寸）。ゆえに供給面（chain）リサイズ経路を踏まずに「同寸・異 id 再 Show」だけを固定できる。
    /// golden は presenter が内部で辿るのと同一 world/atlas から作るため readback とのバイト一致が
    /// 二重に決定論的。2 面の golden が別物であることを fixture 自身が assert する（R6.1 の回帰檻前提）。
    fn build_two_face_assets(w: u32, h: u32) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
        let base = Path::new("shell/master");
        let surfaces = vec![
            surface(1000, vec![elem("p.png", 0, 0)]),
            surface(3000, vec![elem("q.png", 0, 0)]),
        ];

        let stride = w * 4;
        let gradient = |salt: u8| -> Vec<u8> {
            let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
            for y in 0..h {
                for x in 0..w {
                    let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                    let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                    let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                    img.extend_from_slice(&[b, g, r, 0xFF]);
                }
            }
            img
        };

        let mut dec = MemoryDecoder::new();
        dec.insert(base.join("p.png"), w, h, stride, gradient(0x11), true);
        dec.insert(base.join("q.png"), w, h, stride, gradient(0x77), true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        let atlas = baked.table;

        let mut composer = Composer::new();
        let golden_1000 = composer
            .compose(&world, &atlas, 1000, &BindSet::default(), &PatternState::default())
            .expect("面 1000 の合成は Ok")
            .bytes()
            .to_vec();
        let golden_3000 = composer
            .compose(&world, &atlas, 3000, &BindSet::default(), &PatternState::default())
            .expect("面 3000 の合成は Ok")
            .bytes()
            .to_vec();
        assert_ne!(
            golden_1000, golden_3000,
            "fixture 前提: 同寸でも 2 面のバイトが異ならなければ再表示の回帰檻にならない"
        );

        (world, atlas, golden_1000, golden_3000)
    }

    /// R6.1 観測完了（同寸・異 id 再 Show ＝ 新面提示 ＋ 文字スロット安定）: バルーン target が既に
    /// ある面（1000）を表示中に、**同寸の異なる面 id（3000）**を `ShowSurface` すると——(a) reply Ok・
    /// (b) 可視維持・(c) `HitTest::AlphaMask` 維持・(d) `read_back` が **新面 3000 の golden** と一致
    /// （新面が実際に提示された証跡）・(e) `text_slot_view()`（slot/window/surface_size/scale）が切替の
    /// 前後で**完全一致**（文字スロットが安定＝TextSlotView が不変）——をすべて満たす。
    ///
    /// 同寸ゆえ供給面（chain）と装着（mount）は再生成されず（apply_show の `chain.is_none()` 分岐を
    /// 踏まない）、予約 text スロット entity は据え置かれる＝emo-text の描画資源を破壊しない
    /// （design §emo-present 回帰・文字層＝同寸保持）。本 crate 本体は無改変（test-only・R6.3）。
    #[test]
    fn reshow_same_size_different_face_keeps_text_slot_stable() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);

        let (emo_world, atlas, golden_1000, golden_3000) = build_two_face_assets(6, 5);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 面 1000 を表示確立（可視・αマスク判定・供給面/装着を遅延生成）。
        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "面 1000 の初回 ShowSurface が Ok でない"
        );
        // 前提: 初回表示は面 1000 の golden（切替前の基準）。
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back（面 1000）失敗"),
            golden_1000,
            "初回表示が面 1000 の golden と一致しない（前提が崩れている）"
        );

        let surface_entity = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.mount.as_ref())
            .expect("初回表示後は mount が生成済み")
            .surface_entity();

        // 切替前の文字スロット表示スナップショット（TextSlotView は Copy＝値で退避）。
        let slot_before = presenter
            .text_slot_view(TargetId(0))
            .expect("表示確立後の text_slot_view は Some");

        // 同寸・異 id 再 Show（面 3000）。
        let (tx1, rx1) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 3000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx1),
            },
        );
        // (a) reply Ok。
        assert!(
            matches!(rx1.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "同寸・異 id（面 3000）の再 ShowSurface が Ok でない"
        );

        // (b) 可視維持。
        assert!(
            world.get::<Visual>(surface_entity).unwrap().is_visible,
            "同寸・異 id 再表示後も可視のまま"
        );
        assert!(
            presenter.targets.get(&TargetId(0)).unwrap().visible,
            "同寸・異 id 再表示後も target.visible=true"
        );
        // (c) HitTest::AlphaMask 維持。
        assert_eq!(
            world.get::<HitTest>(surface_entity).unwrap().mode,
            HitTestMode::AlphaMask,
            "同寸・異 id 再表示後も αマスク判定を維持"
        );

        // (d) read_back が新面 3000 の golden と一致（新面が実際に提示された証跡・R6.1）。
        let rb = presenter.read_back(TargetId(0)).expect("read_back（面 3000）失敗");
        assert_eq!(
            rb, golden_3000,
            "再表示のバイトが新面 3000 の golden と一致しない（新面が提示されていない）"
        );
        assert_ne!(
            rb, golden_1000,
            "再表示のバイトが旧面 1000 のまま（面切替が表示へ反映されていない）"
        );

        // (e) 文字スロット表示が切替の前後で完全一致（slot/window/surface_size/scale が不変・R6.1）。
        let slot_after = presenter
            .text_slot_view(TargetId(0))
            .expect("再表示後の text_slot_view は Some");
        assert_eq!(
            slot_before, slot_after,
            "同寸・異 id 再表示で文字スロット表示（slot/window/surface_size/scale）が変化した（TextSlotView が不安定）"
        );
    }

    /// surface 1000（`w×h` 全不透明 element）＋ pattern の現在コマが参照する overlay surface 5000
    /// （1×1 不透明・base と異色・base 左上に収まる）を同一 world へ載せた `(EmoWorld, AtlasTable)` と、
    /// 空 pattern／非空 pattern それぞれの直接合成 golden を返す。
    ///
    /// surface 5000 は surface 1000 の **bind animation ではなく**（1000 に animation を定義しない）、
    /// pattern の現在コマ（`PatternFrame{ surface_id: 5000, Overlay, (0,0) }`）としてのみ top-level 合流
    /// する（plan.rs: 合流対象 = 有効 bind pattern0 の id ∪ PatternState に現在コマを持つ id）。5000 は
    /// 定義層（extent 母集合＝全 element ＋全 bind animation pattern0）に寄与しないため合成外形は base の
    /// `w×h` のまま不変で、pattern 有無で**外形は不変・バイトのみ変わる**（chain リサイズ経路を踏まず
    /// 「pattern が compose へ届いたか」だけを固定できる）。
    fn build_target_assets_with_pattern(
        w: u32,
        h: u32,
        salt: u8,
    ) -> (EmoWorld, AtlasTable, Vec<u8>, Vec<u8>) {
        let base = Path::new("shell/master");
        // surface 1000: 全不透明 element 1 本（animation は持たない＝bind 非依存）。
        // surface 5000: 1×1 不透明 part（pattern の現在コマが参照する overlay 源）。
        let surfaces = vec![
            surface(1000, vec![elem("p.png", 0, 0)]),
            surface(5000, vec![elem("q.png", 0, 0)]),
        ];

        let mut dec = MemoryDecoder::new();
        let stride = w * 4;
        let mut img: Vec<u8> = Vec::with_capacity((stride * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                img.extend_from_slice(&[b, g, r, 0xFF]);
            }
        }
        dec.insert(base.join("p.png"), w, h, stride, img, true);
        // 1×1 の不透明 part（base 左上と必ず異なる色 → pattern 有無でバイトが必ず変わる）。
        dec.insert(base.join("q.png"), 1, 1, 4, vec![0xFF, 0xFF, 0xFF, 0xFF], true);

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(baked.errors.is_empty(), "atlas bake セットアップは失敗しない");

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        let atlas = baked.table;

        let mut composer = Composer::new();
        let golden_plain = composer
            .compose(&world, &atlas, 1000, &BindSet::default(), &PatternState::default())
            .expect("空 pattern 合成は Ok")
            .bytes()
            .to_vec();
        let golden_pattern = composer
            .compose(&world, &atlas, 1000, &BindSet::default(), &pattern_overlay(2000, 5000))
            .expect("非空 pattern 合成は Ok")
            .bytes()
            .to_vec();
        assert_ne!(
            golden_plain, golden_pattern,
            "fixture 前提: pattern 有無で合成バイトが異ならなければ回帰檻にならない"
        );

        (world, atlas, golden_plain, golden_pattern)
    }

    /// animation `anim_id` に surface `surf` の `Overlay` 現在コマ 1 枚を持つ非空 `PatternState`。
    /// `PatternState::default()`（空）と等価でないことを保証する pattern 差分の実体。
    fn pattern_overlay(anim_id: u32, surf: u32) -> PatternState {
        let mut p = PatternState::default();
        p.set(
            anim_id,
            PatternFrame {
                surface_id: surf,
                method: ComposeMethod::Overlay,
                x: 0,
                y: 0,
            },
        );
        p
    }

    /// Task 8.2 完了檻（pattern が presenter → compose ＋ cache を実際に貫く・R5.1/5.2/5.4）: 同一
    /// `(target, surface_id, binds)` でも `ShowSurface` が運ぶ **pattern が変われば表示が変わる**。
    ///
    /// (1) 空 pattern の Show → `read_back` が空 pattern 直接合成 golden と一致（R5.4: 拡張前と観測等価）。
    /// (2) 同一 id・binds のまま **非空 pattern** の Show → `read_back` が非空 pattern 直接合成 golden と
    ///     一致し、かつ空 pattern の絵と**異なる**（pattern が compose へ届き ComposeKey も pattern 分だけ
    ///     ミスして再合成された証跡・R5.1/5.2）。(3) 再び空 pattern の Show → 空 golden へ戻る（pattern が
    ///     キー要素として往復両方向で効く）。presenter が pattern を既定（空）で握り潰していれば (2) が
    ///     空 golden のままとなり本テストは RED になる。
    #[test]
    fn show_surface_pattern_flows_through_to_compose_and_cache() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);

        let (emo_world, atlas, golden_plain, golden_pattern) =
            build_target_assets_with_pattern(4, 3, 0x3C);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let show = |presenter: &mut EmoPresenter, world: &mut World, pattern: PatternState| {
            let (tx, rx) = reply_channel::<PresentOutcome>();
            presenter.apply(
                world,
                PresentCommand::ShowSurface {
                    target: TargetId(0),
                    surface_id: 1000,
                    binds: BindSet::default(),
                    pattern,
                    reply: Some(tx),
                },
            );
            assert!(
                matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
                "ShowSurface が Ok でない"
            );
        };

        // (1) 空 pattern → golden_plain（拡張前と観測等価・R5.4）。
        show(&mut presenter, &mut world, PatternState::default());
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            golden_plain,
            "空 pattern の表示が空 pattern 直接合成 golden と一致しない（R5.4）"
        );

        // (2) 同一 id・binds・非空 pattern → 再合成されて golden_pattern（pattern が compose＋cache を貫く証跡）。
        show(&mut presenter, &mut world, pattern_overlay(2000, 5000));
        let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
        assert_eq!(
            rb, golden_pattern,
            "非空 pattern が表示へ反映されない（pattern が compose へ届いていない＝presenter が握り潰している）"
        );
        assert_ne!(
            rb, golden_plain,
            "非空 pattern の表示が空 pattern と同一（ComposeKey が pattern を無視＝古い絵に衝突している）"
        );

        // (3) 空 pattern へ戻す → golden_plain（pattern がキー要素として往復両方向で効く）。
        show(&mut presenter, &mut world, PatternState::default());
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            golden_plain,
            "空 pattern へ戻した表示が空 golden と一致しない（pattern キー要素の往復が壊れている）"
        );
    }

    // ── CurrentSurfaceRead: 現サーフェス id 状態のライフサイクル固定（Task 2・R3.1-3.4）───────────
    // 現サーフェス id は「最後に表示が成立したサーフェス id」（画面の絵でなく表示成立の結果・α非依存）。
    // 書き込みは既存 `visible` 更新点と同一の3箇所のみ（表示成立/EmptyComposition 縮退/Hide）＝additive。

    /// テスト 10・R3.2 観測完了（未表示→None）: `attach_target` 直後（一度も `ShowSurface` していない）は
    /// `current_surface_id` が `None`。`hit_region` も現サーフェス無しゆえ `None`（純関数へ届かない）。
    ///
    /// `attach_target` は skeleton 登録のみで World に触れないため、GPU 不要の素の `World` で決定論固定する。
    #[test]
    fn current_surface_id_is_none_before_first_show() {
        let mut world = World::new();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x10);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            None,
            "attach_target 直後（未表示）は現サーフェス無し（3.2）"
        );
        assert_eq!(
            presenter.hit_region(TargetId(0), 0, 0),
            None,
            "未表示 target の hit_region は現サーフェス無しゆえ None"
        );
    }

    /// テスト 11・R3.1 観測完了（表示後→直近 id）: 有効 `ShowSurface(1000)` 適用後、`current_surface_id`
    /// が `Some(1000)`（直近に表示が成立したサーフェス id）。
    #[test]
    fn current_surface_id_is_last_shown_after_display() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x11);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        assert!(
            matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );

        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(1000),
            "表示成立後は直近に表示した id（3.1）"
        );
    }

    /// テスト 12・R3.3 観測完了（切替→新 id）: 面 1000 表示中に同寸の別 id 3000 を `ShowSurface` すると、
    /// `current_surface_id` が `Some(3000)` へ追随する（以後の問い合わせは新 id）。
    #[test]
    fn current_surface_id_follows_surface_switch() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, _g1, _g3) = build_two_face_assets(6, 5);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let show = |presenter: &mut EmoPresenter, world: &mut World, id: u32| {
            let (tx, rx) = reply_channel::<PresentOutcome>();
            presenter.apply(
                world,
                PresentCommand::ShowSurface {
                    target: TargetId(0),
                    surface_id: id,
                    binds: BindSet::default(),
                    pattern: PatternState::default(),
                    reply: Some(tx),
                },
            );
            assert!(
                matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
                "ShowSurface が Ok でない"
            );
        };

        show(&mut presenter, &mut world, 1000);
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(1000),
            "初回表示成立後は Some(1000)"
        );

        show(&mut presenter, &mut world, 3000);
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(3000),
            "別 id へ切替後は新 id を返す（3.3）"
        );
    }

    /// テスト 13・R3.2/4.4 観測完了（Hide→None）: 有効表示後の `Hide` で `current_surface_id` が `None`
    /// （「未表示等」に Hide が含まれる＝`\s[-1]` 相当で表示していない）。
    #[test]
    fn current_surface_id_is_none_after_hide() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x13);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(1000),
            "Hide 前は Some(1000)（前提）"
        );

        let (txh, rxh) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::Hide {
                target: TargetId(0),
                reply: Some(txh),
            },
        );
        assert!(
            matches!(rxh.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "Hide が Ok でない"
        );

        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            None,
            "Hide 後は現サーフェス無し（3.2/4.4）"
        );
        assert_eq!(
            presenter.hit_region(TargetId(0), 0, 0),
            None,
            "Hide 後は hit_region も現サーフェス無しゆえ None"
        );
    }

    /// テスト 14 観測完了（InvalidateCache→不変）: 有効表示後に `InvalidateCache` を適用しても
    /// `current_surface_id` は不変（キャッシュ無効化は表示を変えない）。単一真実源が `ComposeKey` 由来では
    /// なくフィールドであることの回帰檻（`invalidate_all` でキーが消えても現サーフェス id は残る）。
    #[test]
    fn current_surface_id_unchanged_by_invalidate_cache() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x14);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let (tx0, rx0) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx0),
            },
        );
        assert!(
            matches!(rx0.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "前提の有効 ShowSurface が Ok でない"
        );

        let (txi, rxi) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::InvalidateCache {
                target: TargetId(0),
                reply: Some(txi),
            },
        );
        assert!(
            matches!(rxi.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
            "InvalidateCache が Ok でない"
        );

        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(1000),
            "InvalidateCache は表示を変えないため現サーフェス id は不変（ComposeKey 由来案の棄却根拠）"
        );
    }

    /// テスト 15・R3.2 観測完了（未登録 target→None）: 一度も `attach_target` していない target に対し
    /// `current_surface_id`／`hit_region` の両アクセサが `None`（未登録＝現サーフェス無し）。
    ///
    /// 両アクセサとも `HashMap` 引きのみで GPU/World を要さないため、`EmoPresenter::new()` 単体で固定する。
    #[test]
    fn unregistered_target_returns_none_for_both_accessors() {
        let presenter = EmoPresenter::new();
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            None,
            "未登録 target の current_surface_id は None"
        );
        assert_eq!(
            presenter.hit_region(TargetId(0), 10, 20),
            None,
            "未登録 target の hit_region は None"
        );
    }

    // ── DPI 追従（k 適用の単一漏斗）: タスク 3.2／3.3 の檻 ────────────────────────────────────
    // k は「target ごとの政策（author_dpi）× 窓ごとの実 DPI」から **show 適用ごと**に導出される。
    // 檻は (a) 政策が窓単位で保たれること、(b) 導出 k が実際に合成結果へ掛かって表示寸・表示バイトを
    // 変えること、(c) k がキャッシュキーへ届くこと、(d) DPI 不在が縮退分岐として独立に成立すること。

    /// タスク 3.2・要件 1.5 観測完了（窓ごとの k 基底）: `attach_target` は target ごとに拡大政策を
    /// 保持し、**別窓・別 author_dpi の 2 target が互いの政策を汚さない**。同一の窓 DPI（192）を与えて
    /// も政策が異なれば導出 k が異なる＝政策が k の基底として実際に効いている。
    ///
    /// `attach_target` は skeleton 登録のみで World に触れないため GPU 不要（素の `World` で決定論固定）。
    #[test]
    fn attach_target_keeps_scale_policy_per_window() {
        let mut world = World::new();
        let win_96 = spawn_window_with_dpi(&mut world, 96);
        let win_144 = spawn_window_with_dpi(&mut world, 144);
        let (w0, a0, _g) = build_target_assets(3, 2, 0x91);
        let (w1, a1, _g) = build_target_assets(3, 2, 0x92);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), win_96, w0, a0, 96)
            .expect("attach_target(0) 失敗");
        presenter
            .attach_target(&mut world, TargetId(1), win_144, w1, a1, 144)
            .expect("attach_target(1) 失敗");

        let p0 = presenter.targets.get(&TargetId(0)).unwrap().policy;
        let p1 = presenter.targets.get(&TargetId(1)).unwrap().policy;
        assert_eq!(p0.author_dpi, 96, "target 0 は自分の author_dpi を保つ");
        assert_eq!(p1.author_dpi, 144, "target 1 は自分の author_dpi を保つ");
        assert_eq!(
            p0.app_scale,
            ScaleRatio::ONE,
            "アプリ管理拡大率は ONE 固定シーム（要件 1.6）"
        );
        assert_eq!(p1.app_scale, ScaleRatio::ONE);
        assert_eq!(
            presenter.targets.get(&TargetId(0)).unwrap().window,
            win_96,
            "政策は target＝窓の対応ごとに保たれる"
        );

        // 同一の窓 DPI を与えても政策が違えば k が違う（政策が k の基底＝要件 1.5 の窓ごと k）。
        assert_eq!(
            derive_scale(p0, Some((192, 192))),
            ScaleRatio::new(2, 1).unwrap()
        );
        assert_eq!(
            derive_scale(p1, Some((192, 192))),
            ScaleRatio::new(4, 3).unwrap()
        );

        // 表示前は実適用 k・native 原寸とも未確定（照会は「まだ何も適用していない」を 1.0 で塗らない）。
        for id in [TargetId(0), TargetId(1)] {
            let t = presenter.targets.get(&id).unwrap();
            assert_eq!(t.applied, None, "表示成立前の applied は None");
            assert_eq!(t.native_size, None, "表示成立前の native_size は None");
            assert!(t.last_show.is_none(), "表示成立前の last_show は None");
            assert!(
                presenter.text_slot_view(id).is_none(),
                "表示成立前は照会不可"
            );
        }
    }

    /// タスク 3.3 の名指し受け入れ基準・要件 2.1/2.2 観測完了（k=2/1 の実拡大表示）: 窓 `DPI`=192・
    /// author_dpi=96（k=2/1）でキャッシュミスの `ShowSurface` を適用すると——(a) 供給面寸が
    /// `scaled_extent(2/1, native)` と一致し、(b) `read_back` のバイト長がその寸に一致し、
    /// (c) `read_back` バイトが **native 合成 → `resample(2/1)`** の独立再現と全バイト一致する。
    ///
    /// k=1.0 固定の途中状態なら (a) が native 寸のまま残るため RED になる（要件 2.2 の「両水準が同一
    /// 物理寸にならない」ことを、96 水準の既存 golden 檻と対で担保する）。
    #[test]
    fn show_surface_scales_display_to_scaled_extent_at_k2() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);

        let (emo_world, atlas, native_golden) = build_target_assets(3, 2, 0x81);
        // 同一入力を独立に再現して k 適用後の golden を作る（presenter の内部値の追認ではない）。
        let (probe_world, probe_atlas, _) = build_target_assets(3, 2, 0x81);
        let k2 = ScaleRatio::new(2, 1).unwrap();
        let (scaled_golden_bytes, native_size, scaled_size) =
            scaled_golden(&probe_world, &probe_atlas, 1000, k2);
        assert_eq!(native_size, (3, 2), "fixture の native 原寸");
        assert_eq!(
            scaled_size,
            k2.scaled_extent(3, 2),
            "golden の外形は丸め権威 scaled_extent に従う"
        );
        assert_eq!(scaled_size, (6, 4));

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        show_ok(&mut presenter, &mut world, TargetId(0), 1000);

        // (a) 供給面（swap chain）寸＝k 倍後の物理寸（既存の「composed 外形従属」連鎖が k 追従した証跡）。
        let chain_size = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.chain.as_ref())
            .expect("表示成立後は供給面が生成済み")
            .size();
        assert_eq!(
            chain_size, scaled_size,
            "供給面寸が scaled_extent(k, native) と一致しない（k が表示へ届いていない）"
        );

        // (b) readback の画素数が k 倍後の寸に一致（stride = width*4 の密配列）。
        let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
        assert_eq!(
            rb.len(),
            (scaled_size.0 * scaled_size.1 * 4) as usize,
            "readback の画素数が k 倍後の寸と一致しない"
        );
        assert_ne!(
            rb.len(),
            native_golden.len(),
            "k=2/1 なのに native 寸のまま（k=1.0 固定の途中状態が残っている・要件 2.2）"
        );

        // (c) バイトそのものが native→resample(k) の独立再現と一致（寸だけ合わせた偽物を弾く）。
        assert_eq!(
            rb, scaled_golden_bytes,
            "表示バイトが native 合成の k 倍リサンプル結果と一致しない"
        );

        // 実適用 k・native 原寸が表示成立点で記録される（照会契約の単一真実源）。
        let t = presenter.targets.get(&TargetId(0)).unwrap();
        assert_eq!(t.applied, Some(k2), "applied が実適用 k と一致しない");
        assert_eq!(
            t.native_size,
            Some(native_size),
            "native_size は k 適用前の原寸"
        );
        assert_eq!(
            t.last_show.as_ref().map(|(id, _, _)| *id),
            Some(1000),
            "last_show は最後に成立した show 入力を保持する"
        );
    }

    /// タスク 3.2・要件 1.2 観測完了（照会契約の更新）: k=2/1 の表示確立後、`TextSlotView::scale()` は
    /// **実適用 k（2.0）**を返し（恒常 1.0 の廃止）、`surface_size()` は **native 原寸**を返す
    /// （供給面が持つ k 適用後の物理寸ではない）。物理寸との関係は
    /// `scaled_extent(scale(), surface_size()) == chain.size()` として成立する。
    #[test]
    fn text_slot_view_reports_applied_scale_and_native_surface_size() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x82);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);

        let view = presenter
            .text_slot_view(TargetId(0))
            .expect("表示確立後の text_slot_view は Some");
        assert_eq!(
            view.scale(),
            2.0,
            "scale() が実適用 k を返さない（恒常 1.0 の定数返しが残っている）"
        );
        assert_eq!(
            view.surface_size(),
            (3, 2),
            "surface_size() は native 原寸（k 適用後の供給面寸ではない）"
        );

        // 契約式: 物理寸 == scaled_extent(k, native)。供給面の実寸で裏取りする。
        let chain_size = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.chain.as_ref())
            .unwrap()
            .size();
        let k = ScaleRatio::new(2, 1).unwrap();
        assert_eq!(
            k.scaled_extent(view.surface_size().0, view.surface_size().1),
            chain_size,
            "物理寸 = scaled_extent(scale(), surface_size()) の契約が成立しない"
        );
        assert_ne!(
            view.surface_size(),
            chain_size,
            "k≠1 では native 原寸と物理寸が一致しない（供給面寸を返していれば同値になる）"
        );
    }

    /// 要件 1.4 観測完了（DPI 取得不能の縮退・専用檻）: 窓 entity に `DPI` component が**無い**target
    /// でも表示は成立し、k は 1.0 へ縮退する（表示を失わない）。
    ///
    /// # `author_dpi` に **192**（非 96）を使う理由＝縮退の**帰属可能性**
    ///
    /// author_dpi=96 で組むと、縮退の答（`app_scale × 1/1` ＝ 1/1）と「component 不在を 96 で捏造した
    /// 場合の答」（`96/96` ＝ 1/1）が**数値として区別できない**。すなわち `world.get::<DPI>(..)` に
    /// `.or(Some((96, 96)))` を足す実装ミス——本体コメントが名指しで禁じている当のもの——を素通し
    /// させてしまい、檻が空虚になる。author_dpi=192 なら捏造時の k は `96/192 = 1/2` となり、
    /// 適用 k・readback 寸（`scaled_extent(1/2, (4,3)) = (2,2)`）・`scale()` の 3 つがすべて外れる。
    /// したがって本テストの緑は「縮退分岐を通った」ことに帰属する。
    ///
    /// 縮退時の表示は k=1.0 の等倍＝native 合成 golden と全バイト一致であり、`scale()` は 1.0 を返す。
    /// 他テストは `DPI` を明示挿入する規律ゆえ、この分岐は本テストだけが踏む（縮退が「正常系のふり」で
    /// 通らないことの保証）。`derive_scale` 側の `error!` 発火自体は同関数の in-crate テストが檻に入れる。
    #[test]
    fn show_surface_without_dpi_component_degrades_to_identity() {
        let mut world = make_world_with_gpu();
        // 意図的に DPI component 無しの窓（本番では起こらない＝取得不能の代替）。
        let window = world.spawn_empty().id();
        assert!(
            world.get::<DPI>(window).is_none(),
            "前提: DPI component 不在"
        );

        let (emo_world, atlas, native_golden) = build_target_assets(4, 3, 0x83);

        let mut presenter = EmoPresenter::new();
        // author_dpi=192（非 96）: 縮退の 1/1 と「96 捏造」の 96/192=1/2 を数値で弁別する（上記 doc）。
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 192)
            .expect("attach_target 失敗");
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);

        let t = presenter.targets.get(&TargetId(0)).unwrap();
        assert_eq!(
            t.applied,
            Some(ScaleRatio::ONE),
            "DPI 不在は author_dpi に依らず app_scale×1/1 へ縮退する（要件 1.4）"
        );
        assert_eq!(t.native_size, Some((4, 3)));
        assert!(t.visible, "縮退しても表示を失わない");
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            native_golden,
            "k=1.0 縮退の表示は等倍 native 合成と全バイト一致（96 捏造なら 1/2 縮小で 2×2 になる）"
        );
        assert_eq!(
            presenter.text_slot_view(TargetId(0)).unwrap().scale(),
            1.0,
            "縮退時の照会値も実適用 k（1.0）"
        );
    }

    /// 要件 2.4/4.1 観測完了（k のキー参加）: 同一合成入力の再 show は **キャッシュヒット**（再合成
    /// しない）が、窓 DPI が変われば k が変わって**必ずミス**し、新しい k で再サンプルされる。
    ///
    /// ヒットの判定は間接推測ではなく**改竄プローブ**で行う: 表示成立後のキャッシュスロットを同一キー
    /// のまま別の絵（面 3000 由来）で上書きし、再 show の表示がその絵になるなら presenter は確かに
    /// キャッシュを引いた（再合成していれば面 1000 の絵に戻る）。続けて窓 DPI を 192→96 へ変えると、
    /// k が 2/1→1/1 になりキー相違でミス＝再合成されて面 1000 の等倍 golden へ戻る。
    #[test]
    fn same_scale_hits_cache_and_window_dpi_change_misses_and_resamples() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);

        let (emo_world, atlas, golden_1000, _golden_3000) = build_two_face_assets(6, 5);
        // 改竄プローブ用に同一 fixture を独立生成（決定論ゆえ同一資産）。
        let (probe_world, probe_atlas, _, _) = build_two_face_assets(6, 5);
        let k2 = ScaleRatio::new(2, 1).unwrap();
        let (scaled_1000, native_size, scaled_size) =
            scaled_golden(&probe_world, &probe_atlas, 1000, k2);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 1 回目（ミス→合成→k=2/1 リサンプル）。
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            scaled_1000,
            "初回表示が k=2/1 のリサンプル結果と一致しない"
        );
        {
            let t = presenter.targets.get(&TargetId(0)).unwrap();
            assert!(
                t.cache
                    .get(1000, &BindSet::default(), &PatternState::default(), k2)
                    .is_some(),
                "導出 k がキャッシュキーへ届いていない（k=2/1 で引けない）"
            );
            assert!(
                t.cache
                    .get(
                        1000,
                        &BindSet::default(),
                        &PatternState::default(),
                        ScaleRatio::ONE
                    )
                    .is_none(),
                "k=1/1 で引けてしまう（k がキー要素になっていない）"
            );
        }

        // 改竄プローブ: 同一キーのスロットを別の絵（面 3000 の k 適用結果）で上書きする。
        let tampered = {
            let mut composer = Composer::new();
            let native = composer
                .compose(
                    &probe_world,
                    &probe_atlas,
                    3000,
                    &BindSet::default(),
                    &PatternState::default(),
                )
                .expect("面 3000 の合成は Ok");
            let mut scaled = ComposedSurface::new(0, 0);
            resample(&native, k2, &mut scaled);
            scaled
        };
        let tampered_bytes = tampered.bytes().to_vec();
        assert_ne!(
            tampered_bytes, scaled_1000,
            "プローブ前提: 別の絵であること"
        );
        presenter
            .targets
            .get_mut(&TargetId(0))
            .unwrap()
            .cache
            .insert(
                1000,
                BindSet::default(),
                PatternState::default(),
                k2,
                tampered,
            );

        // 2 回目（同一入力・同一 k）: ヒットゆえ再合成せず、改竄された絵がそのまま表示される。
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            tampered_bytes,
            "同一入力・同一 k の再 show でキャッシュを引いていない（無駄な再合成）"
        );

        // 窓 DPI 変化（192→96）: k=1/1 へ変わりキー相違＝必ずミス→再合成→等倍 golden へ戻る。
        set_window_dpi(&mut world, window, 96);
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
        assert_eq!(
            rb, golden_1000,
            "窓 DPI 変化後も旧 k の絵が出ている（k がキーに参加していない）"
        );
        assert_eq!(
            rb.len(),
            (native_size.0 * native_size.1 * 4) as usize,
            "k=1/1 の表示寸は native 原寸"
        );
        assert_ne!(
            scaled_size, native_size,
            "前提: 2 水準の物理寸は異なる（要件 2.2）"
        );

        let t = presenter.targets.get(&TargetId(0)).unwrap();
        assert_eq!(
            t.applied,
            Some(ScaleRatio::ONE),
            "照会値が新 k へ追随していない"
        );
        assert_eq!(
            t.native_size,
            Some(native_size),
            "native 原寸は k に依らず不変"
        );
        assert!(
            t.cache
                .get(1000, &BindSet::default(), &PatternState::default(), k2)
                .is_none(),
            "容量 1 スロットは新 k のエントリへ置き換わる"
        );
    }

    /// surface 1000（`w1×h1`）と surface 3000（`w2×h2`）＝**native 原寸が互いに異なる** 2 面を
    /// 同一 world へ載せた `(EmoWorld, AtlasTable)`。
    ///
    /// `build_two_face_assets` は同寸 2 面（供給面リサイズ経路を踏まない檻）だが、こちらは
    /// 「照会契約の native 原寸が**表示中の面**を指しているか」を弁別するために寸法を変えてある
    /// （同寸では取り違えが観測できない）。両面とも α=255 ゆえトリムは全域を残し、合成外形は宣言どおり。
    fn build_two_sized_face_assets(w1: u32, h1: u32, w2: u32, h2: u32) -> (EmoWorld, AtlasTable) {
        let base = Path::new("shell/master");
        let surfaces = vec![
            surface(1000, vec![elem("p.png", 0, 0)]),
            surface(3000, vec![elem("q.png", 0, 0)]),
        ];

        let gradient = |w: u32, h: u32, salt: u8| -> Vec<u8> {
            let mut img: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                for x in 0..w {
                    let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
                    let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
                    let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
                    img.extend_from_slice(&[b, g, r, 0xFF]);
                }
            }
            img
        };

        let mut dec = MemoryDecoder::new();
        dec.insert(
            base.join("p.png"),
            w1,
            h1,
            w1 * 4,
            gradient(w1, h1, 0x21),
            true,
        );
        dec.insert(
            base.join("q.png"),
            w2,
            h2,
            w2 * 4,
            gradient(w2, h2, 0x5C),
            true,
        );

        let set = SurfaceSet {
            surfaces: &surfaces,
            base_dir: base,
            alpha_params: AlphaParams {
                use_self_alpha: UseSelfAlpha::On,
            },
        };
        let baked = bake(&[set], &dec, PackConfig::default());
        assert!(
            baked.errors.is_empty(),
            "atlas bake セットアップは失敗しない"
        );

        let mut world = EmoWorld::build(&shell_of(surfaces));
        world.bind_atlas(&baked.table, SetId(0));
        (world, baked.table)
    }

    /// 要件 1.2/4.4 観測完了（**insert 済みのまま失敗 → 後からヒットで成立**した表示でも照会契約が
    /// 正しい）: 供給面生成に失敗した初回 show は `Err` を返すが、その回の合成結果は既にキャッシュへ
    /// 入っている。資源が復旧した後の再 show は**キャッシュヒット**（＝今回は合成しない）でありながら
    /// 表示が成立する——このとき native 原寸を供給できなければ、確立済みの表示に対して
    /// `text_slot_view` が永続的に `None` を返してしまう。
    ///
    /// 「合成した回だけ `native_size` を書く」実装ではここが RED になる（`native_size` が `None` のまま）。
    /// `cached_native`（cache スロットと対の原寸）を表示成立点で**無条件に**写す実装だけが緑になる。
    ///
    /// device 失敗は `WucGraphicsResource` を**一時的に外す**ことで再現する（2 個目の Compositor を
    /// 生成しない＝要件 5.3 の AV 非再導入を守る）。
    #[test]
    fn native_size_recovers_when_failed_show_is_followed_by_cache_hit() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x84);
        let k2 = ScaleRatio::new(2, 1).unwrap();

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 供給面生成の前提資源を一時退避（合成→insert の**後**で失敗する経路へ入る）。
        let wuc = world
            .remove_resource::<WucGraphicsResource>()
            .expect("前提: make_world_with_gpu が WucGraphicsResource を載せている");

        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        let outcome = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("reply（供給面生成失敗）を受信できない");
        assert!(
            matches!(
                outcome,
                Err(PresentError::Device {
                    context: "WucGraphicsResource::compositor",
                    ..
                })
            ),
            "供給面生成の前提資源が無ければ Device エラー: {outcome:?}"
        );

        {
            let t = presenter.targets.get(&TargetId(0)).unwrap();
            assert!(
                t.cache
                    .get(1000, &BindSet::default(), &PatternState::default(), k2)
                    .is_some(),
                "失敗前に insert 済み＝次回の同一入力は必ずキャッシュヒットになる（本テストの前提）"
            );
            assert_eq!(
                t.cached_native,
                Some((4, 3)),
                "スロットと対の native 原寸は insert と同時に控えられている"
            );
            assert_eq!(t.applied, None, "表示は成立していない（R4.4: 前値のまま）");
            assert_eq!(t.native_size, None, "表示未成立ゆえ照会値も未確定");
            assert!(
                presenter.text_slot_view(TargetId(0)).is_none(),
                "表示未成立の間は照会不可"
            );
        }

        // 資源を戻して同一入力を再 show（＝キャッシュヒット経由で表示が成立する）。
        world.insert_resource(wuc);
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);

        let view = presenter
            .text_slot_view(TargetId(0))
            .expect("ヒット経由で成立した表示でも照会可能でなければならない（欠陥の RED 点）");
        assert_eq!(
            view.surface_size(),
            (4, 3),
            "ヒット経由の成立でも native 原寸が正しく供給される"
        );
        assert_eq!(view.scale(), 2.0, "実適用 k は 2.0");

        let t = presenter.targets.get(&TargetId(0)).unwrap();
        assert_eq!(t.native_size, Some((4, 3)));
        assert_eq!(
            k2.scaled_extent(4, 3),
            t.chain
                .as_ref()
                .expect("表示成立後は供給面が生成済み")
                .size(),
            "物理寸 = scaled_extent(applied, native_size) の契約が回復後も成立する"
        );
    }

    /// 要件 1.2 観測完了（照会 native 原寸は**表示中の面**を指す）: native 原寸の異なる 2 面を切り替え
    /// ながら表示すると、`surface_size()` は常に**いま画面に出ている面**の原寸を返し、
    /// `scaled_extent(scale(), surface_size()) == 供給面寸` が各時点で成立する。
    ///
    /// 3 回目は 2 回目と同一入力＝**キャッシュヒット**であり、ヒット回でも照会値が前の面へ巻き戻ったり
    /// 失われたりしないことを固定する（`native_size` を「合成した回だけ書く」実装が生む取り違えの檻）。
    /// 同寸 fixture では取り違えが観測できないため、寸法の異なる 2 面を専用に用意している。
    #[test]
    fn native_size_tracks_displayed_surface_across_size_changing_switch() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas) = build_two_sized_face_assets(6, 5, 4, 3);
        let k2 = ScaleRatio::new(2, 1).unwrap();

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 3 回目は 2 回目と同一入力＝キャッシュヒット（ヒット回の照会値を固定する）。
        for (step, (surface_id, native)) in
            [(1000u32, (6u32, 5u32)), (3000, (4, 3)), (3000, (4, 3))]
                .into_iter()
                .enumerate()
        {
            show_ok(&mut presenter, &mut world, TargetId(0), surface_id);

            let view = presenter
                .text_slot_view(TargetId(0))
                .expect("表示成立後は照会可能");
            assert_eq!(
                view.surface_size(),
                native,
                "step {step}: surface_size() が表示中の面（{surface_id}）の native 原寸を指していない"
            );
            assert_eq!(view.scale(), 2.0, "step {step}: 実適用 k");

            let chain_size = presenter
                .targets
                .get(&TargetId(0))
                .and_then(|t| t.chain.as_ref())
                .expect("表示成立後は供給面が生成済み")
                .size();
            assert_eq!(
                k2.scaled_extent(native.0, native.1),
                chain_size,
                "step {step}: 物理寸 = scaled_extent(scale(), surface_size()) が成立しない"
            );
            assert_eq!(
                presenter.current_surface_id(TargetId(0)),
                Some(surface_id),
                "step {step}: 現サーフェス id"
            );
        }
    }

    // ── 表示成立点の状態照合＝窓寸 reconcile 報告（タスク 3.4・議題 #2 裁定）────────────────────
    // design Flow 1 キー決定「表示成立点で今回 scaled 寸を前回適用寸と照合し、差分があれば新物理寸を
    // 呼び手（frame drain フェーズ）へ報告する」の檻。報告は `reply` ではなく取り出し可能な状態
    // （`take_pending_resize`）に置かれる——本番 drain 経路が `reply: None`（撃ちっぱなし）ゆえ。

    /// 要件 3.1/4.1/4.2 観測完了（**寸法変化が呼び手へ報告される**）: 同一 surface を k=1/1 で表示した
    /// のち窓 `DPI` を 192 へ変えて再表示すると、表示成立点の状態照合が**新しい物理寸**を積み、
    /// `take_pending_resize` がそれを返す（呼び手＝drain フェーズが同一フレームで窓寸 reconcile に使う）。
    ///
    /// 報告値は native 原寸ではなく **k 倍後の物理寸**であり、供給面の実寸と一致する。照合を行わない
    /// 実装・native 寸を報告する実装のいずれでも RED になる。
    #[test]
    fn dpi_change_reports_new_physical_size_to_caller() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, _golden) = build_target_assets(6, 5, 0x85);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // k=1/1 の初回表示（初回報告は Flow 3 手順 5 の領分ゆえ、ここでは取り出して捨てる）。
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            Some((6, 5)),
            "初回表示は物理寸を報告する（本テストの前提・Flow 3 手順 5）"
        );
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "取り出しで要求は消える（drain 契約）"
        );

        // モニタ跨ぎ移動・表示スケール変更の決定論的代替: 窓 DPI を 96→192（k=1/1→2/1）。
        set_window_dpi(&mut world, window, 192);
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);

        let k2 = ScaleRatio::new(2, 1).unwrap();
        let expected = k2.scaled_extent(6, 5);
        assert_eq!(expected, (12, 10), "前提: k=2/1 の物理寸");
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            Some(expected),
            "物理寸が変わったのに新物理寸が呼び手へ報告されない（状態照合の欠落）"
        );

        // 報告値＝実際に表示へ載った物理寸（供給面の実寸）であることを裏取りする。
        let chain_size = presenter
            .targets
            .get(&TargetId(0))
            .and_then(|t| t.chain.as_ref())
            .expect("表示成立後は供給面が生成済み")
            .size();
        assert_eq!(chain_size, expected, "報告値と供給面寸が乖離している");
    }

    /// 要件 4.2 観測完了（**べき等・churn を作らない**）: 物理寸が変わらない再表示は何も報告しない。
    ///
    /// 3 段で檻に入れる——(1) 初回表示の報告を取り出す、(2) 同一入力の再 show（**キャッシュヒット**）は
    /// `None`、(3) 別 surface（3000・**同一 native 原寸**＝キャッシュミスで再合成）も `None`。
    /// (3) が効くのは「合成したか否か」ではなく**物理寸そのもの**で判定していることの担保である
    /// （表示成立ごとに無条件で `Some(size)` を積む実装は (2)(3) 双方で RED）。
    #[test]
    fn unchanged_physical_size_reports_nothing() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);
        // 面 1000 と 3000 は同一 native 原寸（6×5）＝合成入力は違うが物理寸は同じ。
        let (emo_world, atlas, _g1000, _g3000) = build_two_face_assets(6, 5);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let k2 = ScaleRatio::new(2, 1).unwrap();
        let physical = k2.scaled_extent(6, 5);
        assert_eq!(physical, (12, 10), "前提: k=2/1 の物理寸");

        // (1) 初回表示は報告あり（取り出して要求を空にする）。
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            Some(physical),
            "初回表示の報告（本テストの前提）"
        );

        // (2) 同一入力の再 show＝キャッシュヒット・同寸 → 報告なし。
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "同寸のヒット再表示が窓寸 reconcile 要求を捏造している（churn の源）"
        );

        // (3) 別 surface＝キャッシュミスで再合成するが物理寸は同じ → 報告なし。
        show_ok(&mut presenter, &mut world, TargetId(0), 3000);
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(3000),
            "前提: 面が切り替わっている（＝ミスして再合成した回）"
        );
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "再合成しただけで物理寸が同じなら報告してはならない（判定が寸法でなく合成有無になっている）"
        );
    }

    /// 要件 3.1 観測完了（**初回表示も必ず報告する**・design Flow 3 手順 5）: 窓は起動時 k₀ 見積もり寸で
    /// 生成されており実窓 DPI 由来の k と一致する保証がないため、**前回適用寸が無い初回表示**も差分扱いで
    /// 物理寸を報告しなければ、k₀ と実 DPI の差分を補正する経路が永久に走らない。
    ///
    /// 報告値は native 原寸（4×3）ではなく k 倍後の物理寸（8×6）である。初回を黙らせる実装
    /// （`prev.is_some() && prev != Some(size)` 条件）は本テストで RED になる。
    #[test]
    fn first_show_reports_physical_size_for_initial_reconcile() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x86);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // 表示前は要求なし（attach しただけで窓を動かさない）。
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "表示未成立の間に窓寸 reconcile 要求があってはならない"
        );

        show_ok(&mut presenter, &mut world, TargetId(0), 1000);

        let k2 = ScaleRatio::new(2, 1).unwrap();
        let physical = k2.scaled_extent(4, 3);
        assert_eq!(physical, (8, 6), "前提: k=2/1 の物理寸");
        assert_ne!(physical, (4, 3), "前提: native 原寸と物理寸が弁別可能");
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            Some(physical),
            "初回表示が物理寸を報告しない（k₀ 見積もり窓寸との差分が補正されない・Flow 3 手順 5）"
        );
    }

    /// 要件 4.4 観測完了（**失敗は何も報告しない・前値を維持する**）: 表示成立点より手前で early return
    /// する失敗経路は、窓寸 reconcile 要求を積まない。
    ///
    /// 2 種の失敗クラスで檻に入れる——(A) 表示未成立での device 失敗（`WucGraphicsResource` 一時退避・
    /// 2 個目の Compositor を作らない＝要件 5.3 の AV 非再導入を守る）、(C) 表示成立**後**の合成失敗
    /// （`SurfaceNotFound`）。(C) は直前に窓 DPI を 192→96 へ変えてから失敗させるため、報告を
    /// 表示成立点より手前（例: `derive_scale` 直後）へ置いた実装なら `Some((4,3))` が積まれて RED になる。
    #[test]
    fn failed_show_reports_no_resize_and_keeps_previous_values() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x87);
        let k2 = ScaleRatio::new(2, 1).unwrap();

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // (A) 供給面生成の前提資源を一時退避 → 合成・insert の後、表示成立の手前で失敗する。
        let wuc = world
            .remove_resource::<WucGraphicsResource>()
            .expect("前提: make_world_with_gpu が WucGraphicsResource を載せている");
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 1000,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        assert!(
            matches!(
                rx.recv_timeout(Duration::from_secs(10)),
                Ok(Err(PresentError::Device { .. }))
            ),
            "前提: 供給面生成に失敗する"
        );
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "表示が成立していない失敗が窓寸 reconcile 要求を積んでいる（要件 4.4 違反）"
        );
        {
            let t = presenter.targets.get(&TargetId(0)).unwrap();
            assert_eq!(t.applied, None, "失敗は前値（未確定）を維持する");
            assert_eq!(t.native_size, None, "失敗は前値（未確定）を維持する");
        }

        // (B) 資源を戻して表示を成立させる（以降の「前値」を作る）。
        world.insert_resource(wuc);
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            Some(k2.scaled_extent(4, 3)),
            "前提: 成立した表示は報告する"
        );

        // (C) 窓 DPI を 192→96（k=2/1→1/1・物理寸なら 8×6→4×3 相当）へ変えたうえで**合成に失敗**させる。
        //     表示は成立しないため、k も表示も前値のまま＝報告も無い。
        set_window_dpi(&mut world, window, 96);
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::ShowSurface {
                target: TargetId(0),
                surface_id: 9999,
                binds: BindSet::default(),
                pattern: PatternState::default(),
                reply: Some(tx),
            },
        );
        assert!(
            matches!(
                rx.recv_timeout(Duration::from_secs(10)),
                Ok(Err(PresentError::Compose(ComposeError::SurfaceNotFound(
                    9999
                ))))
            ),
            "前提: 解決不能 id は Err(SurfaceNotFound)"
        );
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "表示成立前に early return した失敗が新 k の物理寸を報告している（報告点が表示成立点より手前）"
        );

        // 前 k・前表示・照会契約はすべて据え置き（要件 4.4）。
        let t = presenter.targets.get(&TargetId(0)).unwrap();
        assert_eq!(t.applied, Some(k2), "失敗しても前 k を維持する");
        assert_eq!(
            t.native_size,
            Some((4, 3)),
            "失敗しても前 native 原寸を維持する"
        );
        assert_eq!(
            t.chain.as_ref().expect("供給面は生成済み").size(),
            k2.scaled_extent(4, 3),
            "失敗しても前表示（物理寸）を維持する"
        );
    }

    // ── 表示成立点 info ログ（設計 D10・要件 6.1/6.3）の檻 ──────────────────────────────────
    // 実機サインオフ（R6.3）は有界 auto-exit で起動して `RUST_LOG` を grep し、**このログのフィールド名と
    // 値**から「2 水準が異なる k・異なる物理寸で描かれた」ことを決定論的に判定する。ゆえに level が
    // `info` であることと D10 各フィールドが正しい値で在ることは観測状態と同格の契約であり、檻に入れる。
    //
    // 捕捉は **`tracing` 単体**（本 crate の既存依存）で組む——`tracing-subscriber` は dev-dependency に
    // 無く、要件 7.3（新規外部依存の禁止）ゆえ足さない。`with_default` は **スレッドローカル**の既定
    // subscriber を差すため、並列実行される他テストのイベントを取り込まない（`set_global_default` は
    // プロセス大域＝並列テストで混線するため使わない）。

    /// 捕捉した 1 イベント（level ＋ フィールド名 → Debug 表現）。
    #[derive(Debug, Clone)]
    struct CapturedEvent {
        level: tracing::Level,
        fields: std::collections::HashMap<String, String>,
    }

    /// 全フィールドを Debug 表現で拾う visitor。
    ///
    /// [`tracing::field::Visit`] の `record_u64`/`record_f64`/`record_bool` 等はすべて既定実装が
    /// `record_debug` へ転送するため、`record_debug` 1 本の実装で型を問わず全フィールドを捕捉できる。
    struct FieldGrab(std::collections::HashMap<String, String>);

    impl tracing::field::Visit for FieldGrab {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            self.0
                .insert(field.name().to_string(), format!("{value:?}"));
        }
    }

    /// イベントを溜めるだけの最小 subscriber（span は使わないので new_span は固定 id を返す）。
    #[derive(Clone, Default)]
    struct CaptureSubscriber(std::sync::Arc<std::sync::Mutex<Vec<CapturedEvent>>>);

    impl tracing::Subscriber for CaptureSubscriber {
        fn enabled(&self, _meta: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _attrs: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            let mut grab = FieldGrab(std::collections::HashMap::new());
            event.record(&mut grab);
            self.0
                .lock()
                .expect("捕捉バッファの毒化なし")
                .push(CapturedEvent {
                    level: *event.metadata().level(),
                    fields: grab.0,
                });
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// 要件 6.1/6.3 観測完了（設計 D10 の観測ログ）: 表示成立点で **`info` レベル**のログが出て、
    /// k 導出値（`k`・`k_ratio`）・`author_dpi`・`window_dpi`・native 寸・scaled 寸が揃う。
    ///
    /// k=2/1・native 4×3・物理 8×6 という**互いに弁別可能**な値で組むため、native と scaled の取り違え・
    /// k の取り違えはすべて RED になる。`info!` を `debug!` へ落とす改変も level assert が捕まえる
    /// （R6.3 の `RUST_LOG` grep は既定の観測条件で info を読むため、level 自体が契約である）。
    #[test]
    fn display_success_emits_d10_observation_log_at_info() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x88);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        let cap = CaptureSubscriber::default();
        tracing::subscriber::with_default(cap.clone(), || {
            show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        });

        let events = cap.0.lock().expect("捕捉バッファ").clone();
        let ev = events
            .iter()
            .find(|e| {
                e.fields
                    .get("message")
                    .is_some_and(|m| m.contains("表示・マスクを更新"))
            })
            .unwrap_or_else(|| panic!("表示成立点のログが出ていない: {events:?}"));

        assert_eq!(
            ev.level,
            tracing::Level::INFO,
            "表示成立点の観測ログが info レベルでない（R6.3 の RUST_LOG grep が既定条件で読めない）"
        );

        let field = |name: &str| -> String {
            ev.fields
                .get(name)
                .unwrap_or_else(|| panic!("D10 フィールド `{name}` が無い: {:?}", ev.fields))
                .clone()
        };

        // k 導出値: f32 の照会表現と、既約有理表現（num/den）の双方。
        assert_eq!(field("k"), "2.0", "k（f32）が実適用値でない");
        let k_ratio = field("k_ratio");
        assert!(
            k_ratio.contains("num: 2") && k_ratio.contains("den: 1"),
            "k_ratio に既約 num/den が出ていない: {k_ratio}"
        );

        // 導出の両入力（分母＝作者基準 DPI・分子側＝窓 DPI）。
        assert_eq!(field("author_dpi"), "96");
        assert_eq!(
            field("window_dpi"),
            "Some((192, 192))",
            "窓 DPI が出ていない（不在＝要件 1.4 縮退も None として観測できる必要がある）"
        );

        // 適用寸: native（k 適用前）と scaled（k 適用後・実際に窓へ載る物理寸）が弁別可能に揃う。
        assert_eq!(field("native_w"), "4");
        assert_eq!(field("native_h"), "3");
        assert_eq!(
            field("scaled_w"),
            "8",
            "scaled 寸が native のまま（k が届いていない）"
        );
        assert_eq!(
            field("scaled_h"),
            "6",
            "scaled 寸が native のまま（k が届いていない）"
        );

        // 状態照合の結果（初回表示ゆえ差分あり＝窓寸 reconcile 要求を積んだ）。
        assert_eq!(field("size_changed"), "true");
        assert_eq!(field("surface_id"), "1000");
        assert_eq!(field("target_id"), "TargetId(0)");
    }

    // ── applied_scale／refresh_scale（タスク 3.5・design Flow 2）───────────────────────────────

    /// 捕捉イベント列に「表示成立点のログ」が在るか（＝`apply_show` が表示を成立させたか）。
    ///
    /// `refresh_scale` が「何もしなかった」ことの証明に使う——戻り値 `None` だけでは
    /// 「再表示したが同寸だった」と区別できないため、表示成立そのものの有無を観測する。
    fn has_display_success_log(events: &[CapturedEvent]) -> bool {
        events.iter().any(|e| {
            e.fields
                .get("message")
                .is_some_and(|m| m.contains("表示・マスクを更新"))
        })
    }

    /// 要件 1.2 観測完了（照会契約 `applied_scale`）: 未登録 target と表示成立前は `None`、k≠1 の表示
    /// 成立後は**実適用 k** を返す。
    ///
    /// 恒常 1.0 を返す実装・`attach` 時点で 1.0 を確定させる実装のいずれも RED になる
    /// （表示成立前に `Some(1.0)` が出れば「まだ何も適用していない」を塗り潰している）。
    #[test]
    fn applied_scale_is_none_before_display_and_reports_applied_k_after() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x90);

        let mut presenter = EmoPresenter::new();
        assert_eq!(
            presenter.applied_scale(TargetId(7)),
            None,
            "未登録 target は None"
        );

        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
        assert_eq!(
            presenter.applied_scale(TargetId(0)),
            None,
            "attach しただけ（表示成立前）は None——1.0 で塗り潰さない"
        );

        show_ok(&mut presenter, &mut world, TargetId(0), 1000);

        assert_eq!(
            presenter.applied_scale(TargetId(0)),
            Some(2.0),
            "表示成立後は実適用 k（192/96=2.0）を返す"
        );
        // 同一の単一真実源（`applied`）から出る 2 経路が一致する。
        assert_eq!(
            presenter.applied_scale(TargetId(0)),
            presenter.text_slot_view(TargetId(0)).map(|v| v.scale()),
            "applied_scale と TextSlotView::scale() が乖離している（真実源が 2 つある）"
        );
    }

    /// タスク 3.5 の名指し受け入れ基準・要件 4.1/4.2 観測完了: k=1/1 で表示を確立したのち窓 `DPI` を
    /// 192 へ差し替えて `refresh_scale` を呼ぶと——(a) 戻り値が `scaled_extent(2/1, native)`、
    /// (b) `applied_scale` が 2.0、(c) readback が k=2/1 のリサンプル結果と全バイト一致する。
    ///
    /// さらに (d) `refresh_scale` が返した要求は**消費済み**であり、続く `take_pending_resize` は
    /// `None` を返す——タスク 4.2 が `run_dpi_phase`（`refresh_scale`）と drain フェーズ
    /// （`take_pending_resize`）の**両方**を呼ぶため、同一の reconcile が二度出ないことが結線契約である。
    #[test]
    fn refresh_scale_after_dpi_change_reapplies_new_k() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, native_golden) = build_target_assets(6, 5, 0x91);
        // 同一入力を独立に再現して k=2/1 の golden を作る（presenter の内部値の追認ではない）。
        let (probe_world, probe_atlas, _) = build_target_assets(6, 5, 0x91);
        let k2 = ScaleRatio::new(2, 1).unwrap();
        let (scaled_bytes, native_size, scaled_size) =
            scaled_golden(&probe_world, &probe_atlas, 1000, k2);
        assert_eq!(native_size, (6, 5));
        assert_eq!(scaled_size, (12, 10));

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");

        // k=1/1 の表示確立（初回表示が積む k₀ 補正要求は取り出して捨てる）。
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(presenter.applied_scale(TargetId(0)), Some(1.0));
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            native_golden,
            "前提: k=1/1 の表示は等倍 native 合成"
        );
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            Some(native_size),
            "前提: 初回表示の要求を取り出しておく"
        );

        // モニタ跨ぎ移動・表示スケール変更の決定論的代替（WM_DPICHANGED 相当）。
        set_window_dpi(&mut world, window, 192);

        // (a) 戻り値＝新物理寸。
        assert_eq!(
            presenter.refresh_scale(&mut world, TargetId(0)),
            Some(scaled_size),
            "DPI 変化後の refresh_scale が新物理寸を返さない（再導出・再表示が走っていない）"
        );
        // (b) 照会値が新 k へ追随。
        assert_eq!(
            presenter.applied_scale(TargetId(0)),
            Some(2.0),
            "refresh_scale 後も照会値が旧 k のまま（要件 4.2 の一貫更新が成立していない）"
        );
        // (c) 実際に画面へ載った画素が k=2/1 のリサンプル結果。
        let rb = presenter.read_back(TargetId(0)).expect("read_back 失敗");
        assert_eq!(
            rb, scaled_bytes,
            "表示バイトが k=2/1 のリサンプル結果と一致しない（照会値だけ更新して絵を更新していない）"
        );
        assert_ne!(rb, native_golden, "前提: 2 水準の絵は弁別可能");

        // (d) 要求は refresh_scale が消費済み＝drain フェーズと二重に resize しない（タスク 4.2 の結線契約）。
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "refresh_scale が返した要求が drain 側にも残っている（同一フレームで二重 resize になる）"
        );
    }

    /// 要件 4.1 観測完了（**k 不変なら何もしない**）: DPI を変えずに `refresh_scale` を呼んでも
    /// `None` を返し、**再表示を一切行わない**。
    ///
    /// 「何もしない」は戻り値だけでは証明できない（同寸再表示でも `None` になる）ため、2 つの独立した
    /// 観測で固定する——(1) キャッシュスロットを同一キーのまま**別の絵**で改竄しておき、readback が
    /// 改竄後の絵に**ならない**こと（再表示していればヒットして改竄画が載る）、(2) 表示成立点のログが
    /// **1 件も出ていない**こと。
    ///
    /// さらに (3) 未消費の窓寸 reconcile 要求を**握り潰さない**ことを確認する——ゲート不成立時に
    /// `pending_resize` を触る実装は、drain フェーズが拾うはずだった初回表示の要求を消してしまう。
    #[test]
    fn refresh_scale_without_dpi_change_does_nothing() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas, _g1000, _g3000) = build_two_face_assets(6, 5);
        let (probe_world, probe_atlas, _, _) = build_two_face_assets(6, 5);
        let k2 = ScaleRatio::new(2, 1).unwrap();
        let (scaled_1000, _native, scaled_size) =
            scaled_golden(&probe_world, &probe_atlas, 1000, k2);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            scaled_1000
        );
        // 初回表示の要求は**あえて取り出さない**（(3) の握り潰し検査のため）。

        // 改竄プローブ: 同一キーのスロットを別の絵（面 3000 の k 適用結果）で上書きする。
        let tampered = {
            let mut composer = Composer::new();
            let native = composer
                .compose(
                    &probe_world,
                    &probe_atlas,
                    3000,
                    &BindSet::default(),
                    &PatternState::default(),
                )
                .expect("面 3000 の合成は Ok");
            let mut scaled = ComposedSurface::new(0, 0);
            resample(&native, k2, &mut scaled);
            scaled
        };
        let tampered_bytes = tampered.bytes().to_vec();
        assert_ne!(
            tampered_bytes, scaled_1000,
            "プローブ前提: 別の絵であること"
        );
        presenter
            .targets
            .get_mut(&TargetId(0))
            .unwrap()
            .cache
            .insert(
                1000,
                BindSet::default(),
                PatternState::default(),
                k2,
                tampered,
            );

        // DPI は据え置き（k 不変）。
        let cap = CaptureSubscriber::default();
        let got = tracing::subscriber::with_default(cap.clone(), || {
            presenter.refresh_scale(&mut world, TargetId(0))
        });

        assert_eq!(got, None, "k 不変なのに新物理寸を返している");
        // (1) 改竄画が載っていない＝再表示していない。
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            scaled_1000,
            "k 不変なのに再表示している（改竄画が画面へ載った＝無駄な表示更新）"
        );
        // (2) 表示成立点のログが 1 件も出ていない。
        let events = cap.0.lock().expect("捕捉バッファ").clone();
        assert!(
            !has_display_success_log(&events),
            "k 不変なのに表示成立点のログが出ている（再表示が走った）: {events:?}"
        );
        // (3) 未消費の要求を握り潰していない（drain フェーズが拾えること）。
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            Some(scaled_size),
            "ゲート不成立の refresh_scale が未消費の窓寸 reconcile 要求を消している（取りこぼし）"
        );
    }

    /// 要件 4.1 観測完了（**再表示入力が無ければ何もしない**）: 一度も表示が成立していない target は
    /// DPI が変わっても `refresh_scale` が `None`＝副作用なしであること。
    ///
    /// 実際に閉じるのは**可視ゲート**である（`visible` と `last_show` はいずれも表示成立点でのみ
    /// 真になるため、未表示 target は `visible == false` で先に弾かれる）。`last_show` ゲートは
    /// 多層防御であり、可視ゲートを外す変異を単独で捕まえる（設計の 3 ゲート記述をそのまま保つ）。
    ///
    /// 未登録 target も同様に `None`（登録有無で panic しない）——こちらが本テストの非自明な檻。
    #[test]
    fn refresh_scale_without_last_show_input_does_nothing() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x92);

        let mut presenter = EmoPresenter::new();
        assert_eq!(
            presenter.refresh_scale(&mut world, TargetId(7)),
            None,
            "未登録 target は None（panic しない）"
        );

        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
        // 一度も show していない状態で DPI を変える。
        set_window_dpi(&mut world, window, 192);

        let cap = CaptureSubscriber::default();
        let got = tracing::subscriber::with_default(cap.clone(), || {
            presenter.refresh_scale(&mut world, TargetId(0))
        });

        assert_eq!(got, None, "再表示入力が無いのに新物理寸を返している");
        let events = cap.0.lock().expect("捕捉バッファ").clone();
        assert!(
            !has_display_success_log(&events),
            "再表示入力が無いのに表示が成立している: {events:?}"
        );
        assert_eq!(
            presenter.applied_scale(TargetId(0)),
            None,
            "表示は依然として未成立"
        );
        assert_eq!(presenter.current_surface_id(TargetId(0)), None);
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "副作用（窓寸 reconcile 要求）が生じている"
        );
        assert!(
            presenter.read_back(TargetId(0)).is_err(),
            "供給面が生成されている（表示していないのに資源を作った）"
        );
    }

    /// 要件 4.1/3.2 観測完了（**`Hide` 済み target を蘇らせない**）: 非表示の target は DPI が変わっても
    /// 再表示しない——DPI 変化は「見えているものを描き直す」事象であって表示を復活させる事象ではない。
    ///
    /// 可視ゲートを外した実装では、`Hide` した窓が DPI 変化だけで再出現する（`current_surface_id` が
    /// `Some` に戻る）ため RED になる。
    #[test]
    fn refresh_scale_does_not_resurrect_hidden_target() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, _golden) = build_target_assets(4, 3, 0x93);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        let _ = presenter.take_pending_resize(TargetId(0));

        // `\s[-1]` 相当で非表示にする（キャッシュ・供給面・last_show は保持される）。
        let (tx, rx) = reply_channel::<PresentOutcome>();
        presenter.apply(
            &mut world,
            PresentCommand::Hide {
                target: TargetId(0),
                reply: Some(tx),
            },
        );
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(10)),
            Ok(Ok(()))
        ));
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            None,
            "前提: 非表示"
        );
        assert!(
            presenter
                .targets
                .get(&TargetId(0))
                .unwrap()
                .last_show
                .is_some(),
            "前提: Hide しても再表示入力は保持される（可視ゲートだけが再表示を止める）"
        );

        set_window_dpi(&mut world, window, 192);
        let cap = CaptureSubscriber::default();
        let got = tracing::subscriber::with_default(cap.clone(), || {
            presenter.refresh_scale(&mut world, TargetId(0))
        });

        assert_eq!(got, None, "非表示 target が新物理寸を報告している");
        let events = cap.0.lock().expect("捕捉バッファ").clone();
        assert!(
            !has_display_success_log(&events),
            "非表示 target が DPI 変化だけで再表示された（蘇生）: {events:?}"
        );
        let t = presenter.targets.get(&TargetId(0)).unwrap();
        assert!(!t.visible, "非表示のままでなければならない");
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            None,
            "現サーフェスが復活している（蘇生した）"
        );
        assert_eq!(
            presenter.applied_scale(TargetId(0)),
            Some(1.0),
            "再表示していない以上、実適用 k は前値のまま"
        );
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "副作用（窓寸 reconcile 要求）が生じている"
        );
    }

    /// 要件 1.4/4.1 観測完了（**DPI 取得不能を 96 で捏造しない**・ゲート判定の帰属可能性）: 窓の `DPI`
    /// component が失われても `refresh_scale` は縮退 k（`app_scale × 1/1`）を導出し、前回適用 k と等しい
    /// ため**再表示しない**。
    ///
    /// # `author_dpi` に **192**（非 96）を使う理由
    ///
    /// `apply_show` 側の縮退テストと同じ論法である。author_dpi=96 で組むと、縮退の答（1/1）と
    /// 「component 不在を 96 で捏造した場合の答」（96/96＝1/1）が数値として区別できず、
    /// `world.get::<DPI>(..)` に `.or(Some((96, 96)))` を足す実装ミスを素通しさせる。author_dpi=192 なら
    /// 捏造時の k は `96/192 = 1/2` となり、前回適用 k（1/1）と**異なる**ためゲートを通過して再表示が走り、
    /// 戻り値が `Some((2, 2))` になる——本テストはそれを RED として捕らえる。
    #[test]
    fn refresh_scale_does_not_fabricate_dpi_when_component_is_absent() {
        let mut world = make_world_with_gpu();
        // 窓 DPI 192・author_dpi 192 ゆえ k=1/1（縮退値と一致するが、捏造値 96/192=1/2 とは一致しない）。
        let window = spawn_window_with_dpi(&mut world, 192);
        let (emo_world, atlas, native_golden) = build_target_assets(4, 3, 0x95);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 192)
            .expect("attach_target 失敗");
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(
            presenter.applied_scale(TargetId(0)),
            Some(1.0),
            "前提: 192/192 で k=1/1"
        );
        let _ = presenter.take_pending_resize(TargetId(0));

        // DPI 取得不能の決定論的代替（本番では起こらない＝component を落とす）。
        world.entity_mut(window).remove::<DPI>();
        assert!(
            world.get::<DPI>(window).is_none(),
            "前提: DPI component 不在"
        );

        let cap = CaptureSubscriber::default();
        let got = tracing::subscriber::with_default(cap.clone(), || {
            presenter.refresh_scale(&mut world, TargetId(0))
        });

        assert_eq!(
            got, None,
            "DPI 不在を 96 で捏造している（k=1/2 と誤導出して再表示が走った）"
        );
        let events = cap.0.lock().expect("捕捉バッファ").clone();
        assert!(
            !has_display_success_log(&events),
            "DPI 不在の縮退で再表示が走っている: {events:?}"
        );
        assert_eq!(
            presenter.applied_scale(TargetId(0)),
            Some(1.0),
            "縮退後も実適用 k は 1/1 のまま"
        );
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            native_golden,
            "表示が縮小されている（96 捏造で 1/2 が適用された）"
        );
    }

    /// 要件 4.4 観測完了（**再表示の失敗は前 k・前表示を維持し、黙らない**）: `refresh_scale` の内部
    /// 再 show が失敗しても、直前の k による表示がそのまま残る。
    ///
    /// 失敗は `last_show` の surface id を解決不能値へ差し替えて注入する——ゴースト再読込で
    /// `EmoWorld` から面が消えた場合に実在する状況であり、かつ 2 個目の `Compositor` を作らない
    /// （要件 5.3 の AV 非再導入）。供給面生成の失敗経路は初回表示でしか通らない（`chain` が既に在る）
    /// ため、表示確立**後**の失敗を作るにはこの注入が要る。
    ///
    /// `apply_show` 自身も失敗を error! するが、それは「合成に失敗した」ことしか語らない。DPI 追従の
    /// 文脈（どの k からどの k への再導出が落ちたか・前表示を維持したこと）は `refresh_scale` でしか
    /// 分からないため、本経路は専用の error! を出す（無言の失敗経路を作らない）。
    #[test]
    fn refresh_scale_failure_keeps_previous_display_and_k() {
        let mut world = make_world_with_gpu();
        let window = spawn_window_with_dpi(&mut world, 96);
        let (emo_world, atlas, native_golden) = build_target_assets(4, 3, 0x94);

        let mut presenter = EmoPresenter::new();
        presenter
            .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
        show_ok(&mut presenter, &mut world, TargetId(0), 1000);
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            Some((4, 3)),
            "前提: 初回表示の要求を取り出しておく"
        );

        // 失敗注入: 再表示入力の surface id を解決不能値へ差し替える。
        presenter
            .targets
            .get_mut(&TargetId(0))
            .unwrap()
            .last_show
            .as_mut()
            .expect("前提: 表示成立済みゆえ last_show は Some")
            .0 = 9999;

        set_window_dpi(&mut world, window, 192);
        let cap = CaptureSubscriber::default();
        let got = tracing::subscriber::with_default(cap.clone(), || {
            presenter.refresh_scale(&mut world, TargetId(0))
        });

        assert_eq!(
            got, None,
            "失敗したのに新物理寸を報告している（要件 4.4 違反）"
        );

        // 前 k・前表示・現サーフェスがすべて据え置き（表示を失わない）。
        assert_eq!(
            presenter.applied_scale(TargetId(0)),
            Some(1.0),
            "失敗したのに照会値が新 k へ動いている（前 k 維持の違反）"
        );
        assert_eq!(
            presenter.read_back(TargetId(0)).expect("read_back 失敗"),
            native_golden,
            "失敗したのに表示が失われた／変わった（前表示維持の違反）"
        );
        assert_eq!(
            presenter.current_surface_id(TargetId(0)),
            Some(1000),
            "失敗したのに現サーフェスが失われた"
        );
        assert!(
            presenter.targets.get(&TargetId(0)).unwrap().visible,
            "失敗で表示が消えている（表示を失わない縮退の違反）"
        );
        assert_eq!(
            presenter.take_pending_resize(TargetId(0)),
            None,
            "失敗が窓寸 reconcile 要求を積んでいる"
        );

        // 無言の失敗経路を作らない: refresh_scale 固有の error! が出ている。
        let events = cap.0.lock().expect("捕捉バッファ").clone();
        let err = events
            .iter()
            .find(|e| {
                e.fields
                    .get("message")
                    .is_some_and(|m| m.contains("refresh_scale: 再表示が成立せず"))
            })
            .unwrap_or_else(|| panic!("refresh_scale の失敗が無言（専用ログが無い）: {events:?}"));
        assert_eq!(
            err.level,
            tracing::Level::ERROR,
            "再表示失敗が error! でない（要件 4.4 のログ規律）"
        );
        assert!(
            !has_display_success_log(&events),
            "失敗したのに表示成立点のログが出ている: {events:?}"
        );
    }
}
