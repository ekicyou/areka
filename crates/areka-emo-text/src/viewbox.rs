//! # viewbox — スクロール位置の内部表現・軸写像・量子化（純粋層）
//!
//! 可視窓（[`crate::layout::VisibleWindow`]）の `block_offset` を、真位置（f32 連続量・物理 px）と
//! 確定位置（whole-pixel 整数）に分けて保持する [`ScrollState`]／[`ScrollPlanner`]／
//! [`block_axis_vector`]、ダーティ導出（[`ScrollPlanner::derive_dirty_with_overhangs`]）、
//! `plan`／`commit` の二相（[`ScrollPlanner::plan_with_overhangs`]／[`ScrollPlanner::commit`]・
//! [`FramePlan`]）を提供する。plan は状態不変、commit は COM 実行成功後にのみ確定を反映する
//! （失敗フレーム再試行安全）。
//!
//! 純粋層: `windows` 系 crate に依存しない。失敗経路のない純関数中心ゆえログ/panic を用いない。
//!
//! ## 軸割当（draw.rs の `DrawExecutor::render`・layout.rs `VisibleWindow` と 1:1）
//!
//! ブロック軸（行送り軸）の 2D 割当は既存描画実行と一致させ、独自規則を発明しない:
//!
//! | writing_mode | ブロック軸 | `block_offset` の符号（[`VisibleWindow::block_offset`](crate::layout::VisibleWindow::block_offset)） |
//! |---|---|---|
//! | horizontal_tb | y（横書き＝縦スクロール） | 負（内容が上へ） |
//! | vertical_rl | x（縦書き＝横スクロール） | 正（内容が右へ） |
//! | vertical_lr | x | 負（内容が左へ） |
//!
//! 符号は可視窓決定側が確定した `block_offset` を素通しする。
//!
//! ## 真位置と量子化
//!
//! - 真位置: `pos = block_offset × k`（[`ScaleContract::to_physical`] 経由の一点適用）。
//! - 量子化: `committed = round(pos)`——真位置からの直接丸め（増分丸めを累積しないのでドリフトしない）。
//!   丸めは `f32::round`（round half away from zero）→ `as i32`。
//! - 不変条件 `|committed − pos| ≤ 0.5`（`|pos| ≤ i32::MAX` の間・越えると `as i32` が飽和する）。
//!   k=1.0 では行 pitch が整数（`ceil` 由来）ゆえ行単位 `block_offset` が整数で `committed == pos`
//!   （byte 一致の構造前提）。
//!
//! ## choice-render 座標契約点／補間シーム
//!
//! canvas（image px・validrect-local）→描画面（物理 px）の写像は
//! `p_surface_block = (p_canvas_block + block_offset) × k`（行内軸は `× k` のみ）。量子化状態
//! （committed）は [`ScrollPlanner::scroll_state`] で読める（クリック範囲の導出は choice-render の責務）。
//! 後で補間を足すときは `pos` の生成器だけを差し替える——`plan`/`commit`・量子化・ダーティ導出は
//! そのまま使える。

use crate::canvas::ContentCanvas;
use crate::layout::VisibleWindow;
use crate::region::{ImagePx, ScaleContract};
use crate::writing::WritingMode;

/// スクロール位置の内部表現（choice-render と後で足す補間が読む契約点）。
///
/// 真位置（f32 連続量）と確定位置（whole-pixel 整数）を分けて保持する。`committed = round(pos)`
/// ゆえ `|committed − pos| ≤ 0.5`（`|pos| ≤ i32::MAX` の間・越えると飽和する）。補間は `pos` の
/// 生成器だけを差し替える。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollState {
    /// 真位置（物理 px・f32 連続量）＝ `block_offset × k`（ブロック軸スカラ・符号は素通し）。
    pub pos: f32,
    /// 面に反映済みの whole-pixel 位置（`round(pos)`）。
    pub committed: i32,
}

/// 1 フレームの描画計画（純粋・決定論の値オブジェクト）。
///
/// [`ScrollPlanner::plan_with_overhangs`] が状態を変えずに返す。COM 層の実行が成功したときだけ
/// [`ScrollPlanner::commit`] で確定を反映する（失敗フレームは未 commit のまま次フレームで再計画）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FramePlan {
    /// 変化なし——blit も描画も present も行わない（描画呼び出し 0 の檻の対象）。
    NoChange,
    /// Clear cue 適用で見える行が無い——back を全域透明 Clear（描画 0 件）して flip。
    /// 見える行があれば消去は全域の `Update` が担う。
    FullClear,
    /// blit ＋ ダーティ描画（露出帯 ∪ 変化行）。
    Update {
        /// 面内 blit ベクトル（物理 px 整数・軸は writing_mode 追随・スクロールなしは 0）。
        blit: (i32, i32),
        /// ダーティ矩形と、その矩形を復元するために描く行の対（露出帯 ∪ 変化行 ∪ 残滓）。
        dirty: Vec<DirtyRect>,
        /// 全ダーティ矩形の交差行の和集合（昇順・重複なし）＝このフレームで資源を組む行。
        /// 実際に描くのは矩形ごとの [`DirtyRect::lines`] だけで、各矩形の行はこの集合の部分集合。
        draw_lines: Vec<usize>,
    },
}

/// ダーティ矩形 1 枚と、その矩形を復元するために描く行の対。
///
/// `lines` は矩形と行送り軸で交差する GlyphRun/Choice 住人の index（昇順・可視窓の先頭行以降）。
/// 矩形と交差しない行のインクはその矩形の内側に 1 画素も無い（行の物理矩形は実測はみ出し＋ガード
/// 1 image px を含む整数格子で、交差判定は半開区間）ので、交差行だけを描けば足りる。行送りの軸で
/// 重なる矩形はまとめてあり、どの行も高々 1 枚にしか入らないので、1 フレームの描画行数は可視の
/// 行の数を超えない。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirtyRect {
    /// 復元する領域（物理 px 整数・面寸クランプ済み・ガード余白込み）。
    pub rect: PhysicalRect,
    /// この矩形を復元するために描く住人 index（昇順・重複なし）。
    pub lines: Vec<usize>,
}

/// 矩形だけを比べる補助（テスト専用）。行の割当を見ない assert が
/// `Vec<DirtyRect> == Vec<PhysicalRect>` をそのまま書けるようにする。
#[cfg(test)]
impl PartialEq<PhysicalRect> for DirtyRect {
    fn eq(&self, other: &PhysicalRect) -> bool {
        self.rect == *other
    }
}

/// ブロック軸（行送り軸）スカラ `v` を writing_mode 追随の 2D ベクトル `(x, y)` へ写す。
///
/// 横書き＝`(0, v)`（y 軸）・縦書き（vertical_rl／vertical_lr）＝`(v, 0)`（x 軸）。符号は `v` を
/// 素通しする。blit ベクトル・ダーティ帯の軸切替の共通口。
pub fn block_axis_vector(mode: WritingMode, v: i32) -> (i32, i32) {
    match mode {
        WritingMode::HorizontalTb => (0, v),
        WritingMode::VerticalRl | WritingMode::VerticalLr => (v, 0),
    }
}

/// スクロール位置の計画者（純粋・決定論）。
///
/// 真位置／確定位置の量子化、ダーティ導出、`plan`/`commit` の二相を担う。`plan` は状態不変
/// （`&self`）で、確定は `commit`（`&mut self`）でのみ反映する（失敗フレーム再試行安全）。
/// 初期状態は committed=0・pos=0・prev_lines は「分からない」・clear_requested=false。
/// `windows` 非依存は `lib.rs` の `pure_layer_modules_have_no_windows_imports` が見張る。
#[derive(Clone, Debug, Default)]
pub struct ScrollPlanner {
    /// 面に反映済みの whole-pixel 位置（`commit` で更新・初期 0）。
    committed: i32,
    /// 直近の真位置（f32 連続量・初期 0）。
    pos: f32,
    /// 前回 `commit` 時の canvas 行指紋（変化行検出の唯一の根拠）。`None`＝前に描いた中身が
    /// 分からない（初め・消去の要求の後）＝全域を描き直す。`Some(空)`＝何も描いていないと分かっている。
    prev_lines: Option<Vec<CommittedLine>>,
    /// Clear cue 受領フラグ——true の間 `plan` は見える行が無ければ `FramePlan::FullClear`、
    /// あれば全域の `FramePlan::Update` を返す。どちらの commit でも false へ戻す
    /// （未 commit の失敗フレームは保持＝再試行安全）。
    clear_requested: bool,
}

impl ScrollPlanner {
    /// 初期状態（真位置・確定位置ともに 0）の計画者を作る。
    pub fn new() -> ScrollPlanner {
        ScrollPlanner::default()
    }

    /// スクロール位置契約点（choice-render／後で足す補間が読む）。
    ///
    /// 現在保持している真位置／確定位置を返す（状態不変）。
    pub fn scroll_state(&self) -> ScrollState {
        ScrollState {
            pos: self.pos,
            committed: self.committed,
        }
    }

    /// 可視窓の `block_offset` から真位置／確定位置を算出する（状態不変）。
    ///
    /// `pos = block_offset × k`（[`ScaleContract::to_physical`] 経由）・`committed = round(pos)`。
    /// どちらもブロック軸スカラゆえ writing_mode 非依存（軸割当は [`block_axis_vector`]）。
    pub fn resolve_position(&self, block_offset: f32, contract: &ScaleContract) -> ScrollState {
        let pos = contract.to_physical(ImagePx(block_offset)).0;
        ScrollState {
            pos,
            committed: pos.round() as i32,
        }
    }

    /// 現在の確定位置から目標確定位置 `target` への面内 blit ベクトル
    /// （`target.committed − committed`（飽和）をブロック軸へ写す）。状態は変えない。
    pub fn blit_vector(&self, target: &ScrollState, mode: WritingMode) -> (i32, i32) {
        block_axis_vector(mode, target.committed.saturating_sub(self.committed))
    }

    /// ずらし量の各成分を ±面の寸（x は幅・y は高さ）で頭打ちにする（向きは保つ）。
    /// |ずらし| ≥ 面の寸は「全部送り出された」と同じ意味で、露出の帯と写しの結果は変わらない。
    fn clamp_blit_to_surface(blit: (i32, i32), surface_size: (u32, u32)) -> (i32, i32) {
        let lim = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
        let (w, h) = (lim(surface_size.0), lim(surface_size.1));
        (blit.0.clamp(-w, w), blit.1.clamp(-h, h))
    }

    /// [`Self::plan_with_overhangs`] に実測はみ出し無し（空スライス）を渡す、テスト用の薄いラッパ。
    /// 本番の呼び手（`ViewboxExecutor::render_styled`）は実測を渡すので `#[cfg(test)]` で塞いである。
    #[cfg(test)]
    pub fn plan(
        &self,
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        mode: WritingMode,
        contract: &ScaleContract,
        surface_size: (u32, u32),
    ) -> FramePlan {
        self.plan_with_overhangs(canvas, window, mode, contract, surface_size, &[])
    }

    /// 実測インクはみ出し（[`LineOverhang`]・住人 index と 1:1）付きで 1 フレームの描画計画を返す
    /// （状態不変・`&self`）。
    ///
    /// - `clear_requested` が立っていれば、見える行が無ければ [`FramePlan::FullClear`]、あれば
    ///   全域の [`FramePlan::Update`]（消去と同じフレームで字も描く）。
    /// - `prev_lines` が `None` なら全域の道（面全域 1 枚）。
    /// - それ以外は `committed` から目標への blit と、[`Self::derive_dirty_with_overhangs`] の結果
    ///   （露出帯 ∪ 変化行）を組む。変化行のダーティは `overhangs` の実測分だけ em ボックスを外側へ
    ///   広げる。blit が 0 かつ dirty が空なら [`FramePlan::NoChange`]、さもなくば [`FramePlan::Update`]。
    ///
    /// 同一入力の反復は同一計画を返す（未 commit の失敗フレームは次フレームで再計画）。
    pub fn plan_with_overhangs(
        &self,
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        mode: WritingMode,
        contract: &ScaleContract,
        surface_size: (u32, u32),
        overhangs: &[LineOverhang],
    ) -> FramePlan {
        if self.clear_requested {
            // 消去と同じフレームで見える字も描く（全面を消してから描く）。
            let plan =
                Self::full_domain_plan(canvas, window, mode, contract, surface_size, overhangs);
            return match plan {
                FramePlan::Update { draw_lines, .. } if draw_lines.is_empty() => {
                    FramePlan::FullClear
                }
                plan => plan,
            };
        }
        let Some(prev_lines) = self.prev_lines.as_deref() else {
            return Self::full_domain_plan(canvas, window, mode, contract, surface_size, overhangs);
        };
        // 全域ダーティ（blit=0・面全域）へ縮退する条件: 可視窓が古い側へ戻った・内容が減った
        // （スクロールアウトした行の再露出）・確定行が行内で縮んだ（block 位置の移動・extent の縮小）。
        // blit＋差分描画では退避インクを取りこぼすので、正しさを優先する。前方 typewriter
        // （住人単調増加・block 不動）では不発で、増分ホットパスは維持される。
        let new_lines = Self::committed_lines(canvas, mode);
        let target = self.resolve_position(window.block_offset, contract);
        let blit = Self::clamp_blit_to_surface(self.blit_vector(&target, mode), surface_size);
        if Self::is_backward_shrink(prev_lines, &new_lines)
            || Self::is_scroll_toward_older(blit, mode)
        {
            return Self::full_domain_plan(canvas, window, mode, contract, surface_size, overhangs);
        }
        let (dirty, draw_lines) = Self::derive_dirty_with_overhangs(
            canvas,
            window,
            mode,
            contract,
            blit,
            surface_size,
            prev_lines,
            &new_lines,
            overhangs,
        );
        if blit == (0, 0) && dirty.is_empty() {
            FramePlan::NoChange
        } else {
            FramePlan::Update {
                blit,
                dirty,
                draw_lines,
            }
        }
    }

    /// COM 実行が成功した後にだけ確定を反映する（`&mut self`）。
    ///
    /// `plan` と同一の window/canvas/contract で呼ぶ前提:
    /// - [`FramePlan::NoChange`]: no-op。
    /// - [`FramePlan::FullClear`]: 全域リセット（committed=0・pos=0・prev_lines=`Some(空)`・
    ///   `clear_requested` を落とす）。
    /// - [`FramePlan::Update`]: 目標位置を確定し、新 canvas から `prev_lines` を張り直す。
    ///   `clear_requested` も落とす。
    pub fn commit(
        &mut self,
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        mode: WritingMode,
        contract: &ScaleContract,
        plan: &FramePlan,
    ) {
        match plan {
            FramePlan::NoChange => {}
            FramePlan::FullClear => {
                self.committed = 0;
                self.pos = 0.0;
                self.prev_lines = Some(Vec::new());
                self.clear_requested = false;
            }
            FramePlan::Update { .. } => {
                let target = self.resolve_position(window.block_offset, contract);
                self.committed = target.committed;
                self.pos = target.pos;
                self.prev_lines = Some(Self::committed_lines(canvas, mode));
                self.clear_requested = false;
            }
        }
    }

    /// 消去の要求が立っているか（描き手が `full_clears` を数えるため・純粋）。
    pub(crate) fn clear_pending(&self) -> bool {
        self.clear_requested
    }

    /// Clear cue の適用点（破棄・リセットの唯一の口・`&mut self`）。
    ///
    /// `clear_requested` を立て、確定位置／行指紋をその場で初期化する（committed=0・pos=0・
    /// prev_lines は「分からない」＝`None`）。次 `plan` が [`FramePlan::FullClear`]（見える行が無い）
    /// か全域の [`FramePlan::Update`]（ある）を返し、COM 成功後の commit がフラグを落とす
    /// （未 commit の失敗フレームはフラグ保持＝再試行安全）。
    pub fn request_clear(&mut self) {
        self.clear_requested = true;
        self.committed = 0;
        self.pos = 0.0;
        self.prev_lines = None;
    }

    /// 前に描いた中身を「分からない」にする（`prev_lines` = `None`・位置と消去の要求はそのまま）。
    /// 描き手が書式を組み直したとき（面の画素が古い見た目のまま）に呼ぶ——次の `plan` は commit が
    /// 成功するまで全域の道を通るので、組み直したフレームが失敗しても古い画素を残さない。
    pub(crate) fn forget_prev(&mut self) {
        self.prev_lines = None;
    }
}

/// ダーティ矩形へ加えるガード余白（**image px**）。
///
/// 透明背景への premultiplied 描画では AA こぼれは blit の位相で変わらないので 0 でも足りる見込みだが、
/// 保守既定として 1 image px を全辺に加え、フォント差による AA こぼれを吸収する（byte 等価は
/// `viewbox_draw_live_diff_tests.rs` が確かめる）。物理 px 換算は
/// `ceil(DIRTY_GUARD_IMG_PX × k)`（[`ScaleContract::scale`] 適用）。
pub const DIRTY_GUARD_IMG_PX: f32 = 1.0;

/// 行の **インクはみ出し量**（em ボックス各辺から外側へ何 image px はみ出すか・全成分 ≥ 0）。
///
/// レイアウトの行矩形は em ボックスだが、DirectWrite の実描画はフォントによって em ボックス各辺より
/// 外へインクがはみ出す（Yu Gothic UI 28px は descent 側へ実測 3px・アクセント/合字/イタリック右
/// 張り出しも同様）。ダーティ矩形が em ボックス丈だと、はみ出しインクがクリップで切り落とされ、
/// 全域再描画との byte 等価が破れる。
///
/// 値は各行の `IDWriteTextLayout` を [`GetOverhangMetrics`] で実測したもの（COM 層
/// `LineLayoutStore` が測定・キャッシュして数値で渡す）。行ボックスのブロック軸寸は行の丈
/// （既定だけの行は `font_height`・装飾のある行は行内最大の em。`viewbox_draw_decoration.rs` の
/// `block_extent`）に設定済みなので、その軸の
/// overhang（横書き＝`top`/`bottom`・縦書き＝`left`/`right`）がそのまま箱からのはみ出しになる
/// （行内軸は巨大な `PROBE_MAX_EXTENT` 箱ゆえ 0 に丸める）。
///
/// 行間は 2px しかなく、はみ出しが行と行の隙間に収まるとは限らない。隣の行へ届いた分は、届いた先が
/// 変化行なら [`ScrollPlanner::derive_dirty_with_overhangs`] の交差判定が隣の行も描画対象に含め、
/// 可視窓の外へ出た行の残り分は同メソッドの (c) が消す。
/// [`GetOverhangMetrics`]: https://learn.microsoft.com/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-getoverhangmetrics
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineOverhang {
    /// 上辺より上（image px・横書きの ascent 側はみ出し・アクセント等）。
    pub top: f32,
    /// 下辺より下（image px・横書きの descent 側はみ出し）。
    pub bottom: f32,
    /// 左辺より左（image px・縦書き列のブロック軸はみ出し・横書きの行頭側）。
    pub left: f32,
    /// 右辺より右（image px・縦書き列のブロック軸はみ出し・イタリック右張り出し等）。
    pub right: f32,
}

/// 物理 px 整数矩形（ダーティ矩形・露出帯・クリップの共通型）。
///
/// 原点＝左上・単位＝物理 px。全成分 `u32`（負や面外はクランプ済みが前提）。ブロック軸
/// （行送り軸）の区間は [`block_span`](Self::block_span) が返し、`viewbox_diff.rs` の `merge_and_assign` が
/// この区間で描画対象行の重なりを判定する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysicalRect {
    /// 左辺（物理 px）。
    pub x: u32,
    /// 上辺（物理 px）。
    pub y: u32,
    /// 幅（物理 px）。
    pub w: u32,
    /// 高さ（物理 px）。
    pub h: u32,
}

impl PhysicalRect {
    /// 幅または高さが 0 の退化矩形（描画対象にならない）。
    pub fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }

    /// ブロック軸（行送り軸）の `[start, end)` 区間（横書き＝y・縦書き＝x）。
    pub fn block_span(&self, mode: WritingMode) -> (u32, u32) {
        match mode {
            WritingMode::HorizontalTb => (self.y, self.y + self.h),
            WritingMode::VerticalRl | WritingMode::VerticalLr => (self.x, self.x + self.w),
        }
    }
}

/// 行指紋（planner 内部・変化行検出の唯一の根拠）。
///
/// 前回確定時の canvas 行のスナップショット。新 canvas の同 index 行と比較して差分（typewriter の
/// 現在行・catch-up の複数行・新規行）を一様に検出する。位置/寸は canvas-local（validrect-local
/// image px・スクロール非依存）ゆえ、可視窓のみ移動（内容不変）では全行の指紋が一致する。float は
/// ビット表現で同値比較する（`FontKey` の `to_bits` 規律に同じ）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommittedLine {
    /// 行の内容文字列（グリフの文字列＝クラスタの連結・非グリフ住人は空）。
    text: String,
    /// ブロック軸位置のビット表現（横書き＝dy・縦書き＝dx——canvas-local image px）。
    block_pos_bits: u32,
    /// 行寸 `(幅, 高さ)` のビット表現（image px）。
    extent_bits: (u32, u32),
    /// 行内の装飾番号列（[`crate::layout::PositionedGlyph::style`] の `StyleId.0` をグリフ順に
    /// 並べたもの・非グリフ住人は空）。同じ文字列でも装飾が違えば変化ありと判定される。番号は
    /// `Clear`／`ClearAll` で振り直されるが、そのとき `request_clear` が `prev_lines` を捨てるので
    /// 古い番号との比較は起きない。
    styles: Vec<u32>,
    /// hover 印（Choice 行のみ非 0・非 Choice 行は常に 0）。非 hover の Choice 行は 0・
    /// hover 中は `ordinal + 1`（0 と衝突させない）。hover の付与/切替/解除で当該 Choice 行の
    /// 指紋だけが変わり、既存 `derive_dirty_with_overhangs` が当該行のみをダーティ化する。
    choice_marker: u32,
}

#[path = "viewbox_diff.rs"]
mod diff;
#[cfg(test)]
use diff::line_fingerprint;

#[cfg(test)]
#[path = "viewbox_axis_tests.rs"]
mod axis_tests;
#[cfg(test)]
#[path = "viewbox_choice_marker_tests.rs"]
mod choice_marker_tests;
#[cfg(test)]
#[path = "viewbox_dirty_tests.rs"]
mod dirty_tests;
#[cfg(test)]
#[path = "viewbox_plan_commit_tests.rs"]
mod plan_commit_tests;
#[cfg(test)]
#[path = "viewbox_style_fingerprint_tests.rs"]
mod style_fingerprint_tests;
#[cfg(test)]
#[path = "viewbox_test_support.rs"]
pub(crate) mod test_support;
