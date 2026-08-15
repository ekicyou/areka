//! target ごとの表示コンテキスト（`PresentTarget`）——presenter が target 単位で持つ私有状態。
//!
//! 合成入力（`emo_world`／`atlas`／`composer`／`cache`）・装着資源（`window`／`mount`／`chain`）・
//! 表示成立点で更新される状態（`applied`／`native_size`／`last_show`／`pending_resize` ほか）を 1 つの
//! 構造体に束ねる。フィールドの更新規律は各 doc が正本であり、書き込み点は `presenter` サブツリー内に
//! 閉じる（`pub(super)` はその範囲を表す＝分割前の「`presenter` 私有」と可視集合が同一）。

use super::budget::FrameBudget;
use super::{
    AtlasTable, BindSet, ComposeCache, Composer, EmoWorld, Entity, PatternState, ScalePolicy,
    ScaleRatio, SwapChainPresenter, VisualMount,
};

/// target の可視性を**誰が確定するか**（`areka-P0-balloon-visibility` Requirement 6.8 の所有一元化）。
///
/// 既定は [`Self::CommandDriven`]＝従来挙動そのもので、本 enum の導入は additive である
/// （既存 target は 1 つも挙動が変わらない）。
///
/// # なぜ「指令にフラグを載せる」ではなく target の所有権なのか（design D2）
///
/// 表示指令の送信側（seriko／adapter／talk スレッド）は可視性の判断材料（バルーン内に可視コンテンツが
/// 置かれたか・会話が終わって何秒経ったか）を一切持たない。ゆえに「この指令は可視化してよいか」を
/// 指令へ載せる形は送信側に答えられない問いを押し付ける。可視性の所有者は target（＝窓）ごとに
/// 静的に決まる（シェル窓＝指令駆動・バルーン窓＝外部所有）ため、**target の属性**として持つ。
///
/// # 非対称性（本設計の要）
///
/// ゲートされるのは**可視化側だけ**である。`Hide`（`\b[-1]` 相当）・全透明退化の Hide 縮退は所有権に
/// 依らず常に即時で不可視化する（Requirement 6.1）——「消す」指令が所有権で遅延・抑止されると、明示
/// 指令の即時性という完成済みの契約が壊れるためである。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisibilityOwnership {
    /// 指令駆動（従来互換）: `ShowSurface` の表示成立＝可視。シェル窓（キャラクター窓）はこちら。
    #[default]
    CommandDriven,
    /// 外部所有: `ShowSurface` は**表示状態の確立のみ**を行い可視化しない。可視化は
    /// [`EmoPresenter::show_target`] のみ、不可視化は従来どおり `Hide` 系のみが行う。
    ///
    /// [`EmoPresenter::show_target`]: super::EmoPresenter::show_target
    External,
}

/// target ごとの表示コンテキスト（シェル・バルーンで同一機構・R5.1 の統一原則）。
///
/// `chain`／`mount` は **初回 `ShowSurface` で原寸が確定してから遅延生成**する（0×0 の供給面は作れない・
/// 全透明退化では生成しない）。`emo_world`／`atlas`（構築時 `bind_atlas` 済み）と `composer`／`cache` は
/// 合成・引き当ての入力側、`window` は装着先の窓ハンドル（R1.3）である。
pub(super) struct PresentTarget {
    /// 合成入力（構築時 `bind_atlas` 済み・不変）。
    pub(super) emo_world: EmoWorld,
    /// アトラス正本（不変・`InvalidateCache` の再合成源）。
    pub(super) atlas: AtlasTable,
    /// 合成器（状態非保持・スクラッチのみ再利用）。
    pub(super) composer: Composer,
    /// 毎フレーム経路の確保計数の器（Requirement 1.3・design.md §FrameBudget）。
    ///
    /// **累積カウンタの所有者はここ**である。target と同じ寿命で適用をまたいで存続するため、
    /// 「ウォームアップ後の N 反復で 1 件も確保が増えていない」（Requirement 3.1）を run 全体の
    /// 累積として主張できる。適用単位の増分は表示成立点で `take_delta` により取り出され、
    /// perf サマリ行の `alloc_*` フィールドへ載る。
    ///
    /// 後段（task 5.2）は本フィールドへ再利用席（合成先の常設席・リサンプル作業領域の席・
    /// マスクの輪番）を足す。席が増えても計数の API 面と所有者は動かない。
    pub(super) budget: FrameBudget,
    /// 合成入力（surface id＋bind 集合）→ (composed, mask, native) 対の**容量 3・LRU** メモ化表
    /// （容量は 2026-08-15 の開発者裁定で 1 → 3・要件 7.1）。
    pub(super) cache: ComposeCache,
    /// 装着先の窓 Entity（R1.3・遅延装着の対象）。
    pub(super) window: Entity,
    /// 窓装着ハンドル（初回表示で生成）。
    pub(super) mount: Option<VisualMount>,
    /// 自前供給面（初回表示で原寸確定後に生成）。
    pub(super) chain: Option<SwapChainPresenter>,
    /// 現在可視か（`Hide`／全透明退化で false・`ShowSurface` 成功で true）。
    ///
    /// **`ownership` が [`VisibilityOwnership::External`] のときは `ShowSurface` で true にならない**
    /// （表示状態の確立と可視化が分離される）。その target を可視にできるのは
    /// [`EmoPresenter::show_target`] だけであり、不可視化は所有権に依らず `Hide` 系が常に即時に行う。
    ///
    /// [`EmoPresenter::show_target`]: super::EmoPresenter::show_target
    pub(super) visible: bool,
    /// 可視性の所有者（既定 [`VisibilityOwnership::CommandDriven`]＝従来挙動）。
    ///
    /// `attach_target` は常に既定で登録し、変更は [`EmoPresenter::set_visibility_ownership`] のみが行う
    /// （結線側が target の役割——シェル窓かバルーン窓か——を知っている唯一の層である）。
    ///
    /// [`EmoPresenter::set_visibility_ownership`]: super::EmoPresenter::set_visibility_ownership
    pub(super) ownership: VisibilityOwnership,
    /// **最後に表示が確立したサーフェス id**（CurrentSurfaceRead・R3.1-3.3）。
    ///
    /// 画面の絵ではなく確立の結果を刻む（全透明合成でも表示成立＝その id が正・α 非依存で collision
    /// 解決の単一真実源）。`Hide`／`EmptyComposition` 縮退で `None`。書き込みは既存 `visible` 更新点と
    /// 同一の3箇所のみ（確立＝`Some(surface_id)`／縮退・Hide＝`None`）で、失敗経路は表示成立点より
    /// 手前で early return するため前値を保持する（`ComposeKey` からは導出しない＝`invalidate_all` で
    /// キーが消えても表示は残るため画面と乖離する）。
    ///
    /// # 「確立」と「可視」は別軸である（`areka-P0-balloon-visibility` Requirement 6.2/6.8）
    ///
    /// [`VisibilityOwnership::External`] の target では確立しても可視にならないため、本フィールドは
    /// 「表示中か」ではなく「最後に確立した面 id」を意味する。可視か否かは
    /// [`EmoPresenter::target_visible`] が別軸で答える——面 ID の所有（本フィールド）と可視性の所有
    /// （`visible`）を混同しないこと。
    ///
    /// [`EmoPresenter::target_visible`]: super::EmoPresenter::target_visible
    pub(super) current_surface_id: Option<u32>,
    /// 拡大政策（`attach_target` で確定・以後不変・要件 1.5）。
    ///
    /// k は target（＝窓）ごとの `policy` と**その窓の** `DPI` component から導出されるため、DPI の
    /// 異なる複数モニタに窓が同時に存在しても各窓が自窓の k で表示される。政策自体（author_dpi・
    /// アプリ管理拡大率）は時間で変わらない——変わるのは窓 DPI の側である。
    pub(super) policy: ScalePolicy,
    /// **実際に表示へ適用中の** k（照会契約の単一真実源・要件 1.2）。
    ///
    /// 更新は**表示成立点のみ**（失敗経路は手前で early return ＝前値保持・要件 4.4）。表示が一度も
    /// 成立していない間は `None` で、[`EmoPresenter::text_slot_view`] もその間は `None` を返す
    /// （「まだ何も適用していない」を 1.0 で塗り潰さない）。
    pub(super) applied: Option<ScaleRatio>,
    /// 表示中サーフェスの **native 原寸**（k 適用**前**の合成外形・照会契約 `surface_size()` の供給源）。
    ///
    /// 物理寸との関係は `物理寸 == applied.scaled_extent(native_size)`（丸め権威は
    /// [`ScaleRatio::scaled_extent`] 1 本）。供給面 `chain.size()` は k 適用**後**の物理寸を持つため、
    /// 照会契約の native 原寸をここで別に保持する。
    ///
    /// **更新規則**: 更新点は `applied` と同じ表示成立点 1 箇所だが、書き込む値は「今回合成したか」に
    /// 依らず常に [`CacheEntry::native`]（＝いま表示に使ったキャッシュエントリが束ねている原寸）
    /// である。今回合成した回だけ書く実装は、`insert` 済みのまま失敗して後から**ヒットで**表示が成立した
    /// 場合に「画面の絵と別サーフェスの原寸」あるいは `None` が残り、照会契約が壊れる。
    ///
    /// 原寸がエントリの中に在る（target 側の別フィールドではない）のは、容量が 3 になって
    /// 「保持しているエントリ＝直前に挿入したエントリ」が成り立たなくなったためである
    /// （要件 7.1・[`CacheEntry::native`] の doc）。
    ///
    /// [`CacheEntry::native`]: crate::cache::CacheEntry::native
    pub(super) native_size: Option<(u32, u32)>,
    /// 最後に表示が成立した show 入力（再表示＝k 再適用のための入力保持）。
    ///
    /// DPI 変化時に「同じ絵を新しい k で描き直す」ための唯一の入力源であり、読み手は
    /// [`EmoPresenter::refresh_scale`] である。記録点は `applied`/`native_size` と同一（表示成立点）で、
    /// 失敗経路では前値が保たれる——ゆえに再表示は常に「最後に**実際に画面へ出た**入力」を描き直す。
    ///
    /// `Hide` では**消さない**（キャッシュ・供給面と同じく保持する）。再表示するか否かは可視ゲートが
    /// 決めるのであって、入力を捨てて決めるのではない（`Hide` → 再 show の復帰経路を壊さない）。
    pub(super) last_show: Option<(u32, BindSet, PatternState)>,
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
    pub(super) pending_resize: Option<(u32, u32)>,
}
