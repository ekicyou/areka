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
    ScaleRatio, hit_region_scaled, resample,
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
    /// 表示中の**物理寸**＝`scaled_extent(applied, surface_size)`（丸め権威を通した値・要件 3.1）。
    ///
    /// 構築時に [`ScaleRatio::scaled_extent`] で確定させる（`scale` からの再計算を呼び手に許さない）。
    physical_size: (u32, u32),
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

    /// 表示中の**物理寸**（窓 client がこの値と一致すべき唯一の寸法・要件 3.1/4.2）。
    ///
    /// `scaled_extent(applied, surface_size())` を**丸め権威**
    /// [`ScaleRatio::scaled_extent`] 経由で構築時に確定させた値である。呼び手が
    /// [`scale`] と [`surface_size`] から掛け算で復元することを想定していない——
    /// [`ScaleRatio::as_f32`] は照会用の出口ビューであり、その doc が明記するとおり
    /// **寸法・画素演算に使ってはならない**。既約有理 `num/den` が f32 で非厳密になる k
    /// （例 112dpi／author 96 ＝ 7/6）では積が端数ちょうど 0.5 の直下へ落ち、
    /// round half away from zero が切り下がって権威と 1px 食い違う（27px → 権威 32・f32 経由 31）。
    /// 窓 client を 1px 小さく書くとべき等 skip がその誤差を恒久化するため、消費点は必ず本値を使う。
    ///
    /// k=1.0 の窓では [`surface_size`] と一致する（恒等）。
    ///
    /// [`scale`]: Self::scale
    /// [`surface_size`]: Self::surface_size
    pub fn physical_size(&self) -> (u32, u32) {
        self.physical_size
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

/// 窓 client 物理 px の点に対する当たり判定結果（[`EmoPresenter::hit_region_client`] の戻り値）。
///
/// 所有権を持たない借用ビューであり、寿命は presenter の不変借用に従う（マウス移動ごとの割当を
/// 生まない）。フィールドは 2 つとも「同一の判定 1 回」から生まれた対であり、呼び手は
/// **両者を分離して再計算してはならない**——[`surface_point`] は縮約の結果そのもの（唯一の生成点は
/// [`areka_emo_compose::hit_region_scaled`]、未表示縮退時のみ `hit_region_client` 内の直接呼出）で
/// あり、下流は横流しするのみである（二重縮約の構造的排除・design §Data Models 不変条件 (1)）。
///
/// [`surface_point`]: Self::surface_point
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientHit<'a> {
    /// 当たった領域名（無ければ `None`）。k=1.0 では [`EmoPresenter::hit_region`] と完全一致する。
    pub region: Option<&'a str>,
    /// 縮約後のサーフェス px 座標（作者定義空間）。SHIORI へ配信する「ローカル座標」の正準値（要件 1.8）。
    pub surface_point: (i64, i64),
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
            // 物理寸は**丸め権威**で確定させる（`as_f32` 経由の掛け算は 1px 食い違う・D4）。
            physical_size: applied.scaled_extent(surface_size.0, surface_size.1),
            scale: applied.as_f32(),
        })
    }

    /// target の**窓 client 物理寸**（＝実適用 k を丸め単一権威に通した値・要件 3.1/4.2）。
    ///
    /// `scaled_extent(applied, native_size)` を [`ScaleRatio::scaled_extent`] 経由で計算する。
    /// 窓 client を合わせる消費点（`emo2_boot` の resnap／DPI 追従フェーズ）はこの照会だけを見れば
    /// よく、native 原寸と物理寸を**取り違えようがない**——[`TextSlotView`] 経由だと
    /// [`TextSlotView::surface_size`]（native）と [`TextSlotView::physical_size`]（物理）が
    /// 隣り合って生えており、消費点での 1 トークンの取り違えが「窓が原寸へ引き戻される」
    /// 静かな欠陥になる。本照会はその選択肢を消費点から取り除く。
    ///
    /// **[`applied_scale`] から掛け算で復元してはならない**——`as_f32` は照会用の出口ビューであり、
    /// 既約分母が 2 冪でない k（例 112dpi／author 96 ＝ 7/6）では f32 の積が端数ちょうど 0.5 の
    /// 直下へ落ち、権威と 1px 食い違う（native 27px → 権威 32・f32 経由 31）。
    ///
    /// 表示が一度も成立していない target・未登録 target は `None`（[`applied_scale`] と同じ規律で、
    /// 未確定を原寸や 1.0 倍で塗り潰さない）。値は [`TextSlotView::physical_size`] と常に一致する
    /// （同一の `applied`／`native_size` から同一の権威で導くため）。
    ///
    /// [`applied_scale`]: Self::applied_scale
    pub fn target_physical_size(&self, target: TargetId) -> Option<(u32, u32)> {
        let t = self.targets.get(&target)?;
        let applied = t.applied?;
        let native = t.native_size?;
        Some(applied.scaled_extent(native.0, native.1))
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
    /// スナップショット経路）。
    ///
    /// # 当たり判定の ÷k はこの f32 を経由しない
    ///
    /// `as_f32` は照会用の**出口ビュー**であり、寸法・画素演算に使ってはならない（[`target_physical_size`]
    /// の doc が述べる 1px 食い違いと同じ理由）。`areka-P0-collision-dpi-hittest` の点÷k は f32 を一切
    /// 経由せず、有理値のまま [`Self::hit_region_client`] が内部で厳密に消費する。厳密値そのものが要る
    /// 呼び手（実機 probe の期待ゲート等）は [`Self::applied_ratio`] を使う。
    ///
    /// [`target_physical_size`]: Self::target_physical_size
    pub fn applied_scale(&self, target: TargetId) -> Option<f32> {
        Some(self.targets.get(&target)?.applied?.as_f32())
    }

    /// 実適用 k の**厳密照会**（既約有理のまま返す・f32 版 [`Self::applied_scale`] と併存）。
    ///
    /// 真実源・確定点・`None` の意味はすべて [`Self::applied_scale`] と同一（同じ `applied` を読む）で、
    /// 違いは表現だけである——本照会は丸めも近似も挟まない [`ScaleRatio`] を返す。実機サインオフの
    /// **期待ゲート**（「この水準では k がちょうど 5/4 であること」を hard assert する用途・要件 4.1）は
    /// f32 比較では 1 ulp の揺れを議論する羽目になるため、こちらを使う。
    ///
    /// 判定経路（[`Self::hit_region_client`]）は本照会を**経由しない**——判定は私有 `applied` を直読する
    /// （公開面を判定の依存に据えると、照会と判定で別の k を見る余地が生まれる）。
    pub fn applied_ratio(&self, target: TargetId) -> Option<ScaleRatio> {
        self.targets.get(&target)?.applied
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
    /// ではない**。÷k を吸収する**正準の呼び手**は姉妹メソッド [`Self::hit_region_client`] であり、
    /// `areka-P0-collision-dpi-hittest`（W5）で**実装済み**である。production の判定入口はそちらであって
    /// 本メソッドではない——本メソッドを窓 client 物理 px で直接呼ぶと k≠1.0 で当たり判定がずれる。
    /// k=1.0 の窓では両座標系が一致するため、本メソッドの挙動は k 導入の前後で完全に不変である
    /// （[`Self::hit_region_client`] の `region` とも k=1.0 で完全一致する）。現サーフェス無し（未表示／
    /// `Hide`／空合成
    /// 縮退／未登録 target）は `None`（R4.4）。重なりは画家のアルゴリズム（後定義が手前・[`RegionPriority::Painter`]）で
    /// 解決する。`EmoWorld` を presenter 外へ露出しない（`&SurfaceMaster` を外へ出さない）ため純関数
    /// [`areka_emo_compose::hit_region`] の呼出は本メソッド内で閉じ、戻り値の寿命は `&self` に従う
    /// （マウス移動ごとの割当を生まない・design §CurrentSurfaceRead Service Interface）。
    pub fn hit_region(&self, target: TargetId, x: i64, y: i64) -> Option<&str> {
        let t = self.targets.get(&target)?;
        let master = t.emo_world.surface(t.current_surface_id?)?;
        areka_emo_compose::hit_region(master, x, y, RegionPriority::Painter)
    }

    /// 窓 client 物理 px の点を**実適用 k で縮約**して当たり判定を解決する（DPI 追従の正準判定入口・
    /// 要件 1.1/1.4-1.7/4.5）。
    ///
    /// [`Self::hit_region`] が native サーフェス px を受けるのに対し、本メソッドは **k 適用後の窓 client
    /// 物理 px**（`WM_MOUSEMOVE` 等が運ぶ生座標）をそのまま受ける。÷k は本メソッドが吸収するため、
    /// 呼び手が座標を前処理してはならない（前処理すると二重縮約になる）。戻り値の
    /// [`ClientHit::surface_point`] が SHIORI へ配信する「ローカル座標」の正準値である（要件 1.8）。
    ///
    /// # k の真実源（要件 1.4/1.7）
    ///
    /// k は私有 [`PresentTarget::applied`] の**直読のみ**で得る。f32 の出口ビュー [`Self::applied_scale`]
    /// を経由せず（丸めを持ち込まない）、[`derive_scale`] を再呼出もしない（モニタ DPI からの再導出は
    /// 「表示に実際に掛かった k」と食い違い得る）。判定のたびに読むためスナップショットを保持せず、
    /// 窓 DPI 変化で `applied` が更新されれば以後の判定は自動的に新しい k で行われる——旧 k による
    /// 判定は構造的に残らない（要件 1.7）。
    ///
    /// # 縮退（いずれも panic せず定義された結果を返す）
    ///
    /// - **現サーフェス無し**（未表示／`Hide`／空合成縮退／未登録 target）: `region` は `None`
    ///   （[`Self::hit_region`] の縮退と同一）。`surface_point` は有効 k（`applied` 不在なら
    ///   [`ScaleRatio::ONE`]）で縮約した値を返す——判定が無くても座標空間の契約は保つ。これは
    ///   **正常な縮退**であり `warn!` を出さない（未表示 scope 上のマウス移動ごとに鳴らさない）。
    /// - **面はあるのに `applied` が不在**（k 取得不能）: `warn!` を 1 行記録したうえで
    ///   [`ScaleRatio::ONE`]（＝縮約なし）で照合を続行し、当たり判定そのものを失わせない
    ///   （要件 1.6・ログ無し失敗経路の禁止）。これは**現行の公開 API 経由では到達不能な防御分岐**
    ///   である——`applied` と現サーフェス（`current_surface_id`・`emo_world` の面）は同じ表示成立点
    ///   1 箇所で確定するため、「面はあるのに k が無い」状態を外から作れない。到達し得るのは presenter の
    ///   内部不変条件が破れた場合のみであり、その事実こそが `warn!` の伝える情報である。ゆえに警告は
    ///   上の正常縮退とは**明確に別の事象**であり、両者を同じ分岐にまとめてはならない。
    ///
    /// # 観測（要件 4.5）
    ///
    /// k・縮約前座標・縮約後座標・解決 region を `debug!` 1 行の構造化出力で残す。実機サインオフは
    /// `RUST_LOG=areka_emo_present=debug` でこの 1 行を grep して決定論的に判定する。
    ///
    /// 縮約の丸め権威は [`ScaleRatio::unscale_coord`] ただ 1 本であり、本メソッドはその式を持たない
    /// （正常経路は [`areka_emo_compose::hit_region_scaled`] へ委譲・未表示縮退時のみ座標を得るために
    /// 直接呼ぶ）。`&self` のみを取り World・GPU に依存しないため、判定はマウス移動ごとに安全に呼べる。
    pub fn hit_region_client(&self, target: TargetId, x: i64, y: i64) -> ClientHit<'_> {
        // k の真実源は私有 `applied` の直読ただ 1 つ（f32 非経由・`derive_scale` 再呼出なし）。
        // 判定ごとに読むため k 更新へ自動追従する（スナップショットを持たない＝要件 1.7）。
        // 現サーフェスも同じ不変借用から引く（`region` が引けない縮退でも座標契約は保つ）。
        let (applied, master) = match self.targets.get(&target) {
            Some(t) => (
                t.applied,
                t.current_surface_id.and_then(|id| t.emo_world.surface(id)),
            ),
            // 未登録 target は正常縮退（判定対象が存在しない＝異常ではない）。
            None => (None, None),
        };

        let k = match (applied, master.is_some()) {
            (Some(k), _) => k,
            // 正常縮退（未登録／未表示）: k が無いのは当然ゆえ鳴らさない（マウス移動ごとの警告を作らない）。
            (None, false) => ScaleRatio::ONE,
            // 要件 1.6: 面はあるのに k が無い＝内部不変条件の破れ。黙って 1.0 へ倒さず必ず鳴らす。
            (None, true) => {
                tracing::warn!(
                    ?target,
                    client_x = x,
                    client_y = y,
                    "[hit_region_client] 表示中サーフェスがあるのに適用スケール未確定（applied 不在）——k=1.0 相当で照合を続行"
                );
                ScaleRatio::ONE
            }
        };

        let (region, surface_point) = match master {
            // 正常経路: 縮約＋照合を合成純関数へ完全委譲（÷k の式を本層に持たない）。
            Some(master) => {
                let hit = hit_region_scaled(master, x, y, k, RegionPriority::Painter);
                (hit.region, hit.surface_point)
            }
            // 未表示縮退: 照合先が無いので座標だけ丸め権威で縮約する（式は持たず権威を呼ぶ）。
            None => (None, (k.unscale_coord(x), k.unscale_coord(y))),
        };

        tracing::debug!(
            ?target,
            // k の有理表現（既約 num/den）。`ScaleRatio` の num/den は非公開ゆえ `Debug` で出す。
            k_ratio = ?k,
            client_x = x,
            client_y = y,
            surface_x = surface_point.0,
            surface_y = surface_point.1,
            region = ?region,
            "[hit_region_client] client 物理 px を ÷k して当たり判定を解決"
        );

        ClientHit {
            region,
            surface_point,
        }
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
#[path = "presenter_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "presenter_display_tests.rs"]
mod display_tests;
#[cfg(test)]
#[path = "presenter_compose_input_tests.rs"]
mod compose_input_tests;
#[cfg(test)]
#[path = "presenter_read_accessor_tests.rs"]
mod read_accessor_tests;
#[cfg(test)]
#[path = "presenter_dpi_scale_tests.rs"]
mod dpi_scale_tests;
#[cfg(test)]
#[path = "presenter_resize_report_tests.rs"]
mod resize_report_tests;
#[cfg(test)]
#[path = "presenter_refresh_and_log_tests.rs"]
mod refresh_and_log_tests;
#[cfg(test)]
#[path = "presenter_fractional_scale_tests.rs"]
mod fractional_scale_tests;
