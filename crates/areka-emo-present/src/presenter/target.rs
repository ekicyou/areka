//! target ごとの表示コンテキスト（`PresentTarget`）——presenter が target 単位で持つ私有状態。
//!
//! 合成入力（`emo_world`／`atlas`／`composer`／`cache`）・装着資源（`window`／`mount`）・
//! 表示成立点で更新される状態（`applied`／`native_size`／`last_show`／`pending_resize` ほか）を 1 つの
//! 構造体に束ねる。フィールドの更新規律は各 doc が正本であり、書き込み点は `presenter` サブツリー内に閉じる。

use std::cell::RefCell;

use super::budget::FrameBudget;
use super::hit_picture::{HitNotes, HitPicture};
use super::scale_change::AppliedScale;
use crate::draw_nodes::DrawNodes;
use crate::gpu_atlas::GpuAtlas;
use crate::hit_shapes::HitMasks;

use super::{
    AtlasTable, BindSet, ComposeCache, Composer, EmoWorld, Entity, NeverShown, PatternState,
    ScalePolicy, ScaleRatio, VisualMount,
};

/// target の可視性を**誰が確定するか**。既定は [`Self::CommandDriven`]。
///
/// # なぜ「指令にフラグを載せる」ではなく target の所有権なのか
///
/// 表示指令の送信側（seriko／adapter／talk スレッド）は可視性の判断材料（バルーン内に可視コンテンツが
/// 置かれたか・会話が終わって何秒経ったか）を持たないので、「この指令は可視化してよいか」を指令へ載せる
/// 形は送信側に答えられない問いを押し付ける。可視性の所有者は target（＝窓）ごとに静的に決まる
/// （シェル窓＝指令駆動・バルーン窓＝外部所有）ので、target の属性として持つ。
///
/// # 非対称性
///
/// ゲートされるのは可視化側だけ。`Hide`（`\b[-1]` 相当）・全透明退化の Hide 縮退は所有権に依らず常に
/// 即時で不可視化する。「消す」指令が所有権で遅延・抑止されると、明示指令の即時性という契約が壊れる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisibilityOwnership {
    /// 指令駆動: `ShowSurface` の表示成立＝可視。シェル窓（キャラクター窓）はこちら。
    #[default]
    CommandDriven,
    /// 外部所有: `ShowSurface` は**表示状態の確立のみ**を行い可視化しない。可視化は
    /// [`EmoPresenter::show_target`] のみ、不可視化は `Hide` 系のみが行う。
    ///
    /// [`EmoPresenter::show_target`]: super::EmoPresenter::show_target
    External,
}

/// target ごとの表示コンテキスト（シェル・バルーンで同一機構）。
///
/// `mount` は初回 `ShowSurface` で原寸が確定してから遅延生成する（原寸の配置が要る・
/// 全透明退化では生成しない）。`emo_world`／`atlas`（構築時 `bind_atlas` 済み）と `composer`／`cache` は
/// 合成・引き当ての入力側、`window` は装着先の窓ハンドルである。
pub(super) struct PresentTarget {
    /// 合成入力（構築時 `bind_atlas` 済み・不変）。
    pub(super) emo_world: EmoWorld,
    /// アトラス正本（不変・`InvalidateCache` の再合成源）。
    pub(super) atlas: AtlasTable,
    /// 合成器（状態非保持・スクラッチのみ再利用）。
    pub(super) composer: Composer,
    /// 毎フレーム経路の確保計数の器。累積カウンタの所有者はここ。target と同じ寿命で適用をまたいで
    /// 存続するので、「ウォームアップ後の N 反復で 1 件も確保が増えていない」を run 全体の累積として
    /// 主張できる。適用単位の増分は表示成立点で `take_delta` により取り出され、perf サマリ行の `alloc_*`
    /// へ載る。再利用席（合成先と計画の常設席）も本フィールドが持つ。
    pub(super) budget: FrameBudget,
    /// 合成入力（surface id＋bind 集合＋pattern）→ (計画, composed 原寸) の組の**容量 3・LRU** メモ化表。
    pub(super) cache: ComposeCache,
    /// 装着先の窓 Entity（遅延装着の対象）。
    pub(super) window: Entity,
    /// 窓装着ハンドル（初回表示で生成）。
    pub(super) mount: Option<VisualMount>,
    /// 現在可視か（`Hide`／全透明退化で false・`ShowSurface` 成功で true）。
    ///
    /// `ownership` が [`VisibilityOwnership::External`] のときは `ShowSurface` で true にならない（表示状態の
    /// 確立と可視化が分離される）。その target を可視にできるのは [`EmoPresenter::show_target`] だけで、
    /// 不可視化は所有権に依らず `Hide` 系が常に即時に行う。
    ///
    /// [`EmoPresenter::show_target`]: super::EmoPresenter::show_target
    pub(super) visible: bool,
    /// 可視性の所有者（既定 [`VisibilityOwnership::CommandDriven`]）。
    ///
    /// `attach_target` は常に既定で登録し、変更は [`EmoPresenter::set_visibility_ownership`] のみが行う
    /// （結線側が target の役割——シェル窓かバルーン窓か——を知っている唯一の層である）。
    ///
    /// [`EmoPresenter::set_visibility_ownership`]: super::EmoPresenter::set_visibility_ownership
    pub(super) ownership: VisibilityOwnership,
    /// **最後に表示が確立したサーフェス id**。
    ///
    /// 画面の絵ではなく確立の結果を刻む（全透明合成でも表示成立＝その id が正・α 非依存で当たり判定
    /// 解決の単一真実源）。`Hide`／`EmptyComposition` 縮退で `None`。書き込みは `show.rs` の縮退と確立
    /// （External を含む・確立＝`Some(surface_id)`／縮退＝`None`）と `hub.rs` の Hide（`None`）の 3 箇所のみ。
    /// 失敗経路は表示成立点より手前で early return するので前値を保持する。`ComposeKey` からは導出しない
    /// （`invalidate_all` でキーが消えても表示は残るため画面と乖離する）。
    ///
    /// # 「確立」と「可視」は別軸である
    ///
    /// [`VisibilityOwnership::External`] の target では確立しても可視にならないため、本フィールドは
    /// 「表示中か」ではなく「最後に確立した面 id」を意味する。可視か否かは [`EmoPresenter::target_visible`]
    /// が別軸で答える。面 ID の所有（本フィールド）と可視性の所有（`visible`）を混同しないこと。
    ///
    /// [`EmoPresenter::target_visible`]: super::EmoPresenter::target_visible
    pub(super) current_surface_id: Option<u32>,
    /// 拡大政策（`attach_target` で確定・以後不変）。
    ///
    /// k は target（＝窓）ごとの `policy` と**その窓の** `DPI` component から導出されるため、DPI の異なる
    /// 複数モニタに窓が同時に存在しても各窓が自窓の k で表示される。政策自体は時間で変わらず、変わるのは
    /// 窓 DPI の側である。
    pub(super) policy: ScalePolicy,
    /// **実際に表示へ適用中の** k（照会契約の単一真実源）。
    ///
    /// 更新は**表示成立点のみ**（失敗経路は手前で early return ＝前値保持）。表示が一度も
    /// 成立していない間は `None` で、[`EmoPresenter::text_slot_view`] もその間は `None` を返す
    /// （「まだ何も適用していない」を 1.0 で塗り潰さない）。
    pub(super) applied: Option<ScaleRatio>,
    /// 最初の面を表示しないまま決まった理由（表示が一度成り立てば `None`）。
    pub(super) never_shown: Option<NeverShown>,
    /// 面の指令（`ShowSurface`・表示を伴う置き換え）を一度でも受けた。保留・失敗でも立つ。
    pub(super) surface_requested: bool,
    /// **装着ごとの起点**（最後に記録した（k・DPI））。
    ///
    /// 装着（`attach_target`）で `None` から始まり、差し替えでは引き継がない（`applied` は寸法の照合の
    /// ために引き継ぐ）。更新は `applied` と同じ表示成立点だけで、そこで違えば記録を積む。
    pub(super) scale_baseline: Option<AppliedScale>,
    /// 表示中サーフェスの **native 原寸**（k 適用**前**の合成外形・照会契約 `surface_size()` の供給源）。
    ///
    /// 物理寸との関係は `物理寸 == applied.scaled_extent(native_size)`（丸め権威は
    /// [`ScaleRatio::scaled_extent`] 1 本）。表示中エントリが追い出された後（`InvalidateCache`・LRU）も
    /// 照会契約が画面の絵の原寸を返し続けるため、エントリとは別にここへ写しを持つ。
    ///
    /// 更新点は `applied` と同じ表示成立点 1 箇所で、書き込む値は「今回合成したか」に依らず常に、いま
    /// 表示に使ったキャッシュエントリの `composed` の外形（[`CacheEntry::composed`]・原寸）。今回合成した
    /// 回だけ書く実装は、`insert` 済みのまま失敗して後から**ヒットで**表示が成立した場合に「画面の絵と
    /// 別サーフェスの原寸」あるいは `None` が残り、照会契約が壊れる。原寸がエントリの中に在る（挿入時の
    /// 別フィールドではない）のは、容量が 3 で「保持しているエントリ＝直前に挿入したエントリ」が
    /// 成り立たないため。
    ///
    /// [`CacheEntry::composed`]: crate::cache::CacheEntry::composed
    pub(super) native_size: Option<(u32, u32)>,
    /// 最後に表示が成立した show 入力（再表示＝k 再適用のための入力保持）。
    ///
    /// DPI 変化時に「同じ絵を新しい k で描き直す」ための唯一の入力源であり、読み手は
    /// [`EmoPresenter::refresh_scale`] である。記録点は `applied`/`native_size` と同一（表示成立点）で、
    /// 失敗経路では前値が保たれる——ゆえに再表示は常に「最後に**実際に画面へ出た**入力」を描き直す。
    ///
    /// `Hide` では**消さない**（キャッシュ・装着と同じく保持する）。再表示するか否かは可視ゲートが
    /// 決めるのであって、入力を捨てて決めるのではない（`Hide` → 再 show の復帰経路を壊さない）。
    pub(super) last_show: Option<(u32, BindSet, PatternState)>,
    /// **出すときに使うコマ**（隠れている間に預かった指令のコマ）。
    ///
    /// 置くのは指令の入口（`hub.rs` の `ShowSurface` の腕）だけで、外から所有され・見えておらず・
    /// 面が確立済みで番号が同じ・着せ替えが `last_show` と同じ指令のとき（合成しない）。
    /// [`EmoPresenter::show_target`] は在ればこれで通し直し、成立したら空にする。入口で合成が
    /// 成立したときも空にする。対象の差し替えは target を作り直すので空から始まる。
    /// `last_show` は合成した入力のままで、これとは混ぜない。
    ///
    /// [`EmoPresenter::show_target`]: super::EmoPresenter::show_target
    pub(super) held_pattern: Option<PatternState>,
    /// **出番の世代**。外から所有される対象を、見えていない状態
    /// から [`EmoPresenter::show_target`] で見えるようにするたびに 1 つ進める。0 始まり・対象の
    /// 差し替えで引き継ぐ（窓が同じ）。命令で見える対象（シェル）は 0 のまま。
    ///
    /// [`EmoPresenter::show_target`]: super::EmoPresenter::show_target
    pub(super) stage_generation: u64,
    /// **追い付いた世代**（合図 `StageAck` で受けた番号の最大・常に `stage_generation` 以下）。
    /// これが出番の世代より小さい間、外から所有される対象へ着く `ShowSurface` は回数つきの子の欄を
    /// 外される。0 始まり・対象の差し替えで引き継ぐ。
    pub(super) acked_generation: u64,
    /// **ずれ**（合図 `MoveShift` で最後に受けた値・作者の座標）。
    /// 0 始まり。置き方の層が [`EmoPresenter::move_shift`] で読んで窓の位置へ当てる。
    ///
    /// [`EmoPresenter::move_shift`]: super::EmoPresenter::move_shift
    pub(super) move_shift: (i64, i64),
    /// **未消費の窓寸 reconcile 要求**（表示成立点の状態照合が積む）。
    ///
    /// 表示成立点で今回の物理寸（k 適用後の scaled 寸）を前回適用の物理寸と照合し、異なるときだけ
    /// `Some(新物理寸)` を置く。呼び手（`areka` の `runtime::frame` の drain フェーズ）は
    /// [`EmoPresenter::take_pending_resize`] で取り出し、同一フレーム内で窓 client を合わせる。
    ///
    /// # なぜ「エッジ」ではなく「状態」なのか
    ///
    /// 窓寸 reconcile は表示が成立したという状態に紐づく。`Changed<DPI>` エッジに紐づけると、エッジが
    /// 初回 show より前に消費された場合に不整合が残る。ゆえに報告は表示成立点で積まれ、取り出される
    /// まで保持される（呼び手が或るフレームで取り出さなくても要求は失われない）。
    ///
    /// # 初回表示も必ず報告する
    ///
    /// 前回適用寸が無い（`applied`／`native_size` が `None`）初回表示は「差分あり」として扱う。窓は
    /// 起動時の k₀ 見積もり寸で生成されており、実窓 DPI 由来の k と一致する保証がない。初回を黙らせると
    /// k₀ と実 DPI の差分を補正する経路が永久に走らない。
    ///
    /// # べき等（churn を作らない）
    ///
    /// 同寸の再表示では何も置かない。`None` を書き戻して既存の未消費要求を消すこともしない（同寸でも
    /// 「まだ窓へ反映していない要求」は生きているため）。
    pub(super) pending_resize: Option<(u32, u32)>,
    /// 装置ごとの頁の絵と葉の控え（最初の表示か書き出しで作り、装置が替わったら捨てる）。
    ///
    /// `&self` の書き出しの口も画面の外の描画で葉を記録するので `RefCell` に入れる。
    pub(super) gpu: RefCell<Option<GpuAtlas>>,
    /// 計画を `mount` のまとめの節の下へ写す部品（`mount` と一緒に作る）。
    pub(super) nodes: Option<DrawNodes>,
    /// **最後に記録した装置の失敗の原因**（`context`・`hresult`）。
    ///
    /// 成立をはさまずに同じ原因が続くあいだだけ値を持ち、その間の失敗の行は `debug!` になる。対象を
    /// 作るとき空・装置の失敗で `Some`・表示の成立点と装置以外の失敗（合成の失敗・全透明の縮退）で空。
    pub(super) failure_note: Option<(&'static str, i32)>,
    /// 塗ってある ID の描き先。空・失敗・装置の変化で `None`。`Hide` では残す。
    pub(super) hit_picture: Option<HitPicture>,
    /// `region` のマスクの控え（対象の寿命で持つ）。
    pub(super) hit_masks: HitMasks,
    /// 最後に表示が確立した計画の土台の面（`DrawPlan::base_surface`・当たりの領域を引く面）。
    pub(super) shown_base: Option<u32>,
    pub(super) hit_notes: HitNotes,
}
