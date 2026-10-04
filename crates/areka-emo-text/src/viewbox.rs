//! # viewbox — スクロール位置の内部表現・軸写像・量子化（純粋層）
//!
//! 可視窓（[`crate::layout::VisibleWindow`]）の `block_offset` を「真位置（f32 連続量・
//! 物理 px）」と「確定位置（whole-pixel 整数）」へ分離して保持し（α 後の補間シームの土台・
//! R8.2）、ブロック軸のスカラを writing_mode 追随の 2D ベクトルへ写す [`ScrollState`]／
//! [`ScrollPlanner`]／[`block_axis_vector`] を担い、ダーティ導出（[`ScrollPlanner::derive_dirty`]）
//! と状態遷移（[`ScrollPlanner::plan`]／[`ScrollPlanner::commit`]・[`FramePlan`]）を提供する
//! （plan は状態不変・純粋／commit は COM 実行成功後にのみ確定を反映＝失敗フレーム再試行安全）。
//!
//! **層規律**: 純粋層——`windows` 系 crate への依存を一切持たない（決定論檻）。失敗経路の
//! ない純関数中心ゆえログ/panic を用いない。
//!
//! ## 軸割当の正準（draw.rs `render`・layout.rs `VisibleWindow` と 1:1）
//!
//! ブロック軸（行送り軸）の 2D 割当は既存描画実行と一致させ、独自規則を発明しない（R5.3）:
//!
//! | writing_mode | ブロック軸 | `block_offset` の符号（layout.rs:130–137） |
//! |---|---|---|
//! | horizontal_tb | y（横書き＝縦スクロール） | 負（内容が上へ） |
//! | vertical_rl | x（縦書き＝横スクロール） | 正（内容が右へ） |
//! | vertical_lr | x | 負（内容が左へ） |
//!
//! 符号は可視窓決定側が確定した `block_offset` を**素通し**する（独自の軸規則を作らない）。
//! draw.rs の `render`（横書き＝`origin.Y` に加算・縦書き＝`origin.X` に加算）と同一の軸割当。
//!
//! ## 真位置と量子化（DD11・小数アキュムレータ）
//!
//! - 真位置: `pos = block_offset × k`（× k は [`ScaleContract::to_physical`] 経由の一点適用
//!   ——k を独自に `× scale` せず契約点を通す）。
//! - 量子化: `committed = round(pos)`——**真位置からの直接丸め**（増分丸めの累積をしない＝
//!   構造的にドリフトなし）。丸めは `f32::round`（round half away from zero）→ `as i32`。
//! - 不変条件 `|committed − pos| ≤ 0.5`（k≠1.0 の R6.4 檻）。k=1.0 では行 pitch が整数
//!   （`ceil` 由来）ゆえ行単位 `block_offset` が整数＝`pos` が整数＝`committed == pos`
//!   （byte 一致の構造前提）。
//!
//! ## choice-render 座標契約点（R9.3）／補間シーム（α 後）（R8）
//!
//! canvas（image px・validrect-local）→描画面（物理 px）の写像は
//! `p_surface_block = (p_canvas_block + block_offset) × k`（行内軸は `× k` のみ）。量子化状態
//! （committed）は [`ScrollPlanner::scroll_state`] で読める（クリック範囲の実導出は
//! choice-render の責務）。α 後の補間は `pos` の生成器（補間過程）だけを差し替える——`plan`/`commit`・
//! 量子化・ダーティ導出は再設計不要（R8.3）。

use crate::canvas::ContentCanvas;
use crate::layout::VisibleWindow;
use crate::region::{ImagePx, ScaleContract};
use crate::writing::WritingMode;

/// スクロール位置の内部表現（R8.2/9.3 の契約点・choice-render と α 後の補間が読む）。
///
/// スクロール位置を**真位置**（f32 連続量）と**確定位置**（whole-pixel 整数）へ分離して
/// 保持する値オブジェクト。不変条件 `|committed − pos| ≤ 0.5`（`committed = round(pos)`
/// ゆえ恒真）。α 後の補間は `pos` の生成器（補間過程）だけを差し替える——`committed`／写像
/// 規約は不変（R8.3）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollState {
    /// 真位置（物理 px・f32 連続量）＝ `block_offset × k`（ブロック軸スカラ・符号は素通し）。
    /// α 後の補間はこの値の生成を差し替える。
    pub pos: f32,
    /// 面に反映済みの whole-pixel 位置（真位置格子吸着・`round(pos)`・`|committed − pos| ≤ 0.5`）。
    pub committed: i32,
}

/// 1 フレームの描画計画（純粋・決定論の値オブジェクト・DD1/DD4）。
///
/// [`ScrollPlanner::plan`] が状態を変えずに返す 3 種の計画結果。COM 層はこの enum を受けて
/// blit 指示・ダーティ D2D 描画・FullClear を実行し、成功時にだけ [`ScrollPlanner::commit`] で
/// 確定を反映する（失敗フレームは未 commit のまま次フレームで再計画＝再試行安全）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FramePlan {
    /// 変化なし——blit も描画も present も行わない（描画呼び出し 0 の檻の対象・R3.2/3.6）。
    NoChange,
    /// Clear cue 適用——back を全域透明 Clear（描画 0 件）して flip（R4.3）。
    FullClear,
    /// blit ＋ ダーティ描画（露出帯 ∪ 変化行・R2.3/3.2/3.3）。
    Update {
        /// 面内 blit ベクトル（物理 px 整数・軸は writing_mode 追随・スクロールなしは 0）。
        blit: (i32, i32),
        /// ダーティ矩形と、その矩形を復元するために描く行の対（露出帯 ∪ 変化行 ∪ 残滓）。
        dirty: Vec<DirtyRect>,
        /// 全ダーティ矩形の交差行の**和集合**（昇順・重複なし）＝このフレームで資源を組む行。
        /// 実際に描くのは矩形ごとの [`DirtyRect::lines`] だけで、和集合は資源準備と不整合検査の
        /// 対象を 1 語で指すために持つ（不変条件: 各矩形の行はこの集合の部分集合）。
        draw_lines: Vec<usize>,
    },
}

/// ダーティ矩形 1 枚と、**その矩形を復元するために描く行**の対（R11.1/11.3）。
///
/// `lines` は矩形と行送り軸で交差する GlyphRun/Choice 住人の index（昇順・可視窓の先頭行以降）。
/// 矩形と交差しない行のインクはその矩形の内側に 1 画素も無い（行の物理矩形は実測はみ出し＋ガード
/// 1 image px を含む整数格子で、交差判定は半開区間）ので、交差行だけを描いても画素は変わらない。
/// 1 枚ごとに交差行だけを描くことで、1 フレームの描画呼び出しは「矩形の枚数 × 描画対象行数」の
/// 積ではなく**各矩形の交差行数の和**になる。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirtyRect {
    /// 復元する領域（物理 px 整数・面寸クランプ済み・ガード余白込み）。
    pub rect: PhysicalRect,
    /// この矩形を復元するために描く住人 index（昇順・重複なし）。
    pub lines: Vec<usize>,
}

/// 矩形だけを比べる補助（テスト専用）。既存の檻は矩形の並びだけを固定しており、行の割当を
/// 見ないものが多い——`Vec<DirtyRect> == Vec<PhysicalRect>` をそのまま書けるようにして、
/// 行の割当を追加した後もそれらの assert を書き換えずに保つ。
#[cfg(test)]
impl PartialEq<PhysicalRect> for DirtyRect {
    fn eq(&self, other: &PhysicalRect) -> bool {
        self.rect == *other
    }
}

/// ブロック軸（行送り軸）スカラ `v` を writing_mode 追随の 2D ベクトル `(x, y)` へ写す
/// （R5.1–5.3）。
///
/// 軸割当は draw.rs `render`（横書き＝`origin.Y`／縦書き＝`origin.X` に加算）と 1:1:
/// 横書き＝`(0, v)`（y 軸）・縦書き（vertical_rl／vertical_lr）＝`(v, 0)`（x 軸）。符号は
/// `v` を**素通し**する——可視窓 `block_offset` の符号規約（layout.rs:130–137）をそのまま
/// 使い、独自の軸規則を発明しない（R5.3）。blit ベクトル・ダーティ帯の軸切替の共通口。
pub fn block_axis_vector(mode: WritingMode, v: i32) -> (i32, i32) {
    match mode {
        WritingMode::HorizontalTb => (0, v),
        WritingMode::VerticalRl | WritingMode::VerticalLr => (v, 0),
    }
}

/// スクロール位置の計画者（純粋・決定論）。
///
/// 真位置／確定位置の**表現**・軸写像・量子化（3.1）、ダーティ導出（3.2）、`plan`/`commit`
/// 二相と `FramePlan`（3.3）を担う。確定位置 `committed`・直近真位置 `pos`・前回確定時の
/// 行指紋 `prev_lines`・Clear 要求フラグ `clear_requested` を内部状態として保持する
/// （初期: committed=0・pos=0・prev_lines 空・clear_requested=false）。`plan` は状態不変
/// （`&self`）で、確定は `commit`（`&mut self`）でのみ反映する（失敗フレーム再試行安全）。
///
/// 純粋層規律: `windows` 非依存（lib.rs 構造檻へ追加）。同一入力→同一出力。
#[derive(Clone, Debug, Default)]
pub struct ScrollPlanner {
    /// 面に反映済みの whole-pixel 位置（`commit` で更新・初期 0）。
    committed: i32,
    /// 直近の真位置（f32 連続量・α 後に補間過程が更新元になる・初期 0）。
    pos: f32,
    /// 前回 `commit` 時の canvas 行指紋（変化行検出の唯一の根拠・初期空＝全域ダーティ）。
    prev_lines: Vec<CommittedLine>,
    /// Clear cue 受領フラグ——true の間 `plan` は `FramePlan::FullClear` を返す。
    /// `commit(FullClear)` で false へ戻す（未 commit の失敗フレームは保持＝再試行安全）。
    clear_requested: bool,
}

impl ScrollPlanner {
    /// 初期状態（真位置・確定位置ともに 0）の計画者を作る。
    pub fn new() -> ScrollPlanner {
        ScrollPlanner::default()
    }

    /// スクロール位置契約点（R9.3/R8.3——choice-render／α 後の補間が読む）。
    ///
    /// canvas（image px・validrect-local）→描画面（物理 px）の写像は
    /// `p_surface_block = (p_canvas_block + block_offset) × k`（行内軸は `× k` のみ）で、
    /// 量子化状態は返り値の `committed`。現在保持している真位置／確定位置を返すのみ
    /// （純粋・状態不変）。
    pub fn scroll_state(&self) -> ScrollState {
        ScrollState {
            pos: self.pos,
            committed: self.committed,
        }
    }

    /// 可視窓の `block_offset` から真位置／確定位置を算出する（純粋・状態不変・R8.2）。
    ///
    /// 真位置 `pos = block_offset × k`（× k は [`ScaleContract::to_physical`] 経由の一点
    /// 適用）・確定位置 `committed = round(pos)`——**真位置からの直接丸め**（増分丸めの累積を
    /// しない＝構造的にドリフトなし）。pos/committed はブロック軸スカラゆえ writing_mode
    /// 非依存（軸割当は [`block_axis_vector`] の領分・符号は `block_offset` を素通し）。
    pub fn resolve_position(&self, block_offset: f32, contract: &ScaleContract) -> ScrollState {
        let pos = contract.to_physical(ImagePx(block_offset)).0;
        ScrollState {
            pos,
            committed: pos.round() as i32,
        }
    }

    /// 現在の確定位置から目標確定位置 `target` への面内 blit ベクトルを軸写像して返す
    /// （blit ＝ `target.committed − committed` をブロック軸へ・[`block_axis_vector`] 委譲）。
    ///
    /// 状態遷移（`commit`）は後続 3.2 の領分——本メソッドは軸写像の純粋補助のみ
    /// （初期状態では `committed = 0` ゆえ blit ＝ `target.committed` の軸写像）。
    pub fn blit_vector(&self, target: &ScrollState, mode: WritingMode) -> (i32, i32) {
        block_axis_vector(mode, target.committed - self.committed)
    }

    /// 1 フレームの描画計画を返す（**状態不変・純粋**・`&self`・R2.3/4.3）——実測はみ出し無し
    /// （em ボックス丈）の従来経路。測定値（[`LineOverhang`]）を持たない呼び手（pure 層 unit・
    /// mirror planner 等）向けの薄いラッパで、[`Self::plan_with_overhangs`] に空スライスを渡す。
    /// COM 層 `ViewboxExecutor::render` は実測はみ出しを渡す [`Self::plan_with_overhangs`] を使う。
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
    /// （**状態不変・純粋**・`&self`・R2.3/4.3）。
    ///
    /// - `clear_requested` が立っていれば [`FramePlan::FullClear`]（描画 0 件・back 全域 Clear）。
    /// - それ以外は現状 `committed` から目標（`window.block_offset` の量子化）への blit と、
    ///   前回確定 `prev_lines` に対する [`Self::derive_dirty`] の結果（露出帯 ∪ 変化行）を組む。
    ///   変化行のダーティは `overhangs` の実測分だけ em ボックスを外側へ広げてはみ出しインクを含める
    ///   （byte 等価の前提・D2）。blit が 0 かつ dirty が空なら [`FramePlan::NoChange`]、さもなくば
    ///   [`FramePlan::Update`]。
    ///
    /// `self` を一切変えないため、同一入力の反復は同一計画を返す（デバイス失敗フレームは未 commit の
    /// まま次フレームで再計画＝再試行安全）。確定は [`Self::commit`] の役目。
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
            return FramePlan::FullClear;
        }
        // 後方（un-reveal/un-scroll）縮退: 内容が前回確定より減った＝スクロールアウトした行の
        // 再露出、または確定行の行内縮小（同一 index 行の block 位置移動・extent 縮小）を面内
        // blit＋差分描画で保持できない（退避インクを取りこぼす）ため、全域ダーティ（blit=0・
        // 面全域・全住人）へ縮退して正しさを優先する（既存の format 変更/不整合縮退と同型・
        // design Error Handling「最悪でもレガシー全域再描画と等価な 1 フレーム」・DD-9）。
        // 前方 typewriter（住人単調増加・prefix 伸長で extent 増加・block 不動）では不発ゆえ
        // 増分ホットパスは維持——注入時刻の後方ジャンプ・un-reveal 等（確定 content を縮める
        // 任意アクセスパターン）に対する防御で byte 等価（oracle 全域再描画）を保つ。
        // 遅延化（newline-defer）で trailing 空行が消え、旧・即時意味論では行数減少で偶然
        // マスクされていた行内縮小欠陥が露出したため、判定を行数減少から被覆不能変化へ拡張した。
        let new_lines = Self::committed_lines(canvas, mode);
        if Self::is_backward_shrink(&self.prev_lines, &new_lines) {
            let (dirty, draw_lines) = Self::derive_dirty_with_overhangs(
                canvas,
                window,
                mode,
                contract,
                (0, 0),
                surface_size,
                &[],
                overhangs,
            );
            return FramePlan::Update {
                blit: (0, 0),
                dirty,
                draw_lines,
            };
        }
        let target = self.resolve_position(window.block_offset, contract);
        let blit = self.blit_vector(&target, mode);
        let (dirty, draw_lines) = Self::derive_dirty_with_overhangs(
            canvas,
            window,
            mode,
            contract,
            blit,
            surface_size,
            &self.prev_lines,
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

    /// COM 実行が成功した後にだけ確定を反映する（`&mut self`・R2.3/4.3）。
    ///
    /// `plan` と**同一の window/canvas/contract**で呼ばれる前提（呼び手が plan→COM→commit を
    /// 同一入力で回す・design System Flows）:
    /// - [`FramePlan::NoChange`]: no-op（状態を変えない）。
    /// - [`FramePlan::FullClear`]: 全域リセットの確定（committed=0・pos=0・prev_lines 空・
    ///   `clear_requested` を落とす）。
    /// - [`FramePlan::Update`]: 目標位置を確定し（`committed`/`pos`）、新 canvas から行指紋
    ///   `prev_lines` を張り直す（次フレームの変化行検出の根拠）。
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
                self.prev_lines.clear();
                self.clear_requested = false;
            }
            FramePlan::Update { .. } => {
                let target = self.resolve_position(window.block_offset, contract);
                self.committed = target.committed;
                self.pos = target.pos;
                self.prev_lines = Self::committed_lines(canvas, mode);
            }
        }
    }

    /// Clear cue の適用点（破棄・リセットの唯一の口・`&mut self`・R4.3）。
    ///
    /// `clear_requested` を立て、確定位置／行指紋をその場で初期化する（committed=0・pos=0・
    /// prev_lines 空）。次 `plan` が [`FramePlan::FullClear`] を返し、COM 成功後の
    /// `commit(FullClear)` がフラグを落とす（未 commit の失敗フレームはフラグ保持＝再試行安全）。
    pub fn request_clear(&mut self) {
        self.clear_requested = true;
        self.committed = 0;
        self.pos = 0.0;
        self.prev_lines.clear();
    }
}

/// ダーティ矩形へ加えるガード余白（**image px**・DD4）。
///
/// spike 実測では 0（透明背景への premultiplied 描画ゆえ AA こぼれは blit 位相不変）だが、
/// 保守既定として 1 image px を全辺に加え、フォント差による AA こぼれを吸収する。物理 px 換算は
/// `ceil(DIRTY_GUARD_IMG_PX × k)`（[`ScaleContract::scale`] 適用）。
pub const DIRTY_GUARD_IMG_PX: f32 = 1.0;

/// 行の **インクはみ出し量**（em ボックス各辺から外側へ何 image px はみ出すか・全成分 ≥ 0）。
///
/// **なぜ必要か**: レイアウトの行矩形は em ボックス（横書き＝行内長×`font_height`）だが、
/// DirectWrite の実描画は行ボックス（ascent＋descent）で行い、フォントによっては em ボックス
/// 各辺よりインクが外へはみ出す（Yu Gothic UI 28px は descent 側へ実測 3px・アクセント/合字/
/// イタリック右張り出し/装飾スワッシュも同様）。ダーティ矩形が em ボックス丈だと、この
/// **はみ出しインクがクリップで切り落とされて行の下端等が欠ける**（全域再描画のオラクルは
/// クリップしないため byte 等価が破れる＝実機「文字列の下が描画されない」不具合 D2 の真因）。
///
/// 値は各行の `IDWriteTextLayout` を [`GetOverhangMetrics`] で**実測**したもの（COM 層
/// `LineLayoutStore` が測定・キャッシュし、pure 層へ数値として手渡す＝pure 層は windows 非依存の
/// まま）。行ボックスのブロック軸寸が `font_height`（`max_height`／縦は `max_width`）に設定済み
/// ゆえ、その軸の overhang（横書き＝`top`/`bottom`・縦書き＝`left`/`right`）が em ボックスからの
/// はみ出しを直接与える（行内軸は巨大 `PROBE_MAX_EXTENT` 箱ゆえ overhang 無意味＝0 に丸める）。
/// **経験則の推定でなく実測**である。ただし行送りが「字の丈 ＋ 行間 2px」に確定した後は
/// 「はみ出し < 行と行の隙間」は成り立たない——隙間は 2px しかなく、Yu Gothic UI 28px の
/// 下端はみ出しは実測 3px で隣の行の em ボックスへ届く。届いた先が変化行なら
/// [`ScrollPlanner::derive_dirty_with_overhangs`] の交差判定が隣の行も描画対象に含めるため
/// 見えは保たれ、可視窓の外へ出た行の残り分は同メソッドの (c) が消す。
///
/// [`GetOverhangMetrics`]: https://learn.microsoft.com/windows/win32/api/dwrite/nf-dwrite-idwritetextlayout-getoverhangmetrics
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineOverhang {
    /// 上辺より上（image px・横書きの ascent 側はみ出し・アクセント等）。
    pub top: f32,
    /// 下辺より下（image px・横書きの descent 側はみ出し＝D2 の主因）。
    pub bottom: f32,
    /// 左辺より左（image px・縦書き列のブロック軸はみ出し・横書きの行頭側）。
    pub left: f32,
    /// 右辺より右（image px・縦書き列のブロック軸はみ出し・イタリック右張り出し等）。
    pub right: f32,
}

/// 物理 px 整数矩形（DD1——ダーティ矩形・露出帯・クリップの共通型）。
///
/// 原点＝左上・単位＝物理 px。全成分 `u32`（負や面外はクランプ済みが前提）。ブロック軸
/// （行送り軸）の交差判定は [`intersects_block_axis`](Self::intersects_block_axis) が担う
/// （horizontal_tb＝y・vertical_rl/lr＝x——写像正準表と 1:1）。
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

    /// ブロック軸（行送り軸）の `[start, end)` 区間（横書き＝y・縦書き＝x——写像正準表）。
    pub fn block_span(&self, mode: WritingMode) -> (u32, u32) {
        match mode {
            WritingMode::HorizontalTb => (self.y, self.y + self.h),
            WritingMode::VerticalRl | WritingMode::VerticalLr => (self.x, self.x + self.w),
        }
    }

    /// ブロック軸で `other` と重なるか（半開区間の重なり・接辺は非交差）。
    ///
    /// 描画対象行の判定に用いる——行はブロック軸で分離するため、行送り軸の区間が
    /// ダーティ矩形と重なる住人だけを（クリップ下で）再描画すれば足りる（DD4）。
    pub fn intersects_block_axis(&self, other: &PhysicalRect, mode: WritingMode) -> bool {
        let (a0, a1) = self.block_span(mode);
        let (b0, b1) = other.block_span(mode);
        a0 < b1 && b0 < a1
    }
}

/// 行指紋（planner 内部・DD4——変化行検出の唯一の根拠）。
///
/// 前回確定時の canvas 行のスナップショット。内容文字列・ブロック軸位置・行寸・装飾番号列を保持し、
/// 新 canvas の同 index 行と比較して差分（typewriter の現在行・catch-up の複数行・新規行）を
/// 一様に検出する。位置/寸は **canvas-local**（validrect-local image px・スクロール非依存）
/// ゆえ、可視窓のみ移動（内容不変）では全行の指紋が一致＝変化行ゼロになる。float は
/// ビット表現で同値比較する（既存 `FormatKey` の `to_bits()` 規律に同じ——全順序比較を避け
/// 同値判定のみ）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommittedLine {
    /// 行の内容文字列（グリフの文字列＝クラスタの連結・非グリフ住人は空）。
    text: String,
    /// ブロック軸位置のビット表現（横書き＝dy・縦書き＝dx——canvas-local image px）。
    block_pos_bits: u32,
    /// 行寸 `(幅, 高さ)` のビット表現（image px）。
    extent_bits: (u32, u32),
    /// 行内の装飾番号列（[`crate::layout::PositionedGlyph::style`] の `StyleId.0` をグリフ順に
    /// 並べたもの・非グリフ住人は空）。同じ文字列でも装飾が違えば列が違う＝**変化ありと判定される**
    /// （R11.4——装飾だけが変わった行を古い見た目のまま再利用しない）。番号は `Clear`／`ClearAll`
    /// で振り直されるが、そのとき `request_clear` が `prev_lines` を捨てる（[`FramePlan::FullClear`]）
    /// ので古い番号との比較は起きない。
    styles: Vec<u32>,
    /// hover 印（Choice 行のみ非 0・非 Choice 行は常に 0）。非 hover の Choice 行は 0・
    /// hover 中は `ordinal + 1`（0 と衝突させない）。hover の付与/切替/解除で当該 Choice 行の
    /// 指紋だけが変わり、既存 `derive_dirty` が当該行のみをダーティ化する（R4.4）。
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
mod test_support;
