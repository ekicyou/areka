//! # viewbox — スクロール位置の内部表現・軸写像・量子化（純粋層）
//!
//! 可視窓（[`crate::layout::VisibleWindow`]）の `block_offset` を「真位置（f32 連続量・
//! 物理 px）」と「確定位置（whole-pixel 整数）」へ分離して保持し（M2 補間シームの土台・
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
//! ## choice-render 座標契約点（R9.3）／M2 補間シーム（R8）
//!
//! canvas（image px・validrect-local）→描画面（物理 px）の写像は
//! `p_surface_block = (p_canvas_block + block_offset) × k`（行内軸は `× k` のみ）。量子化状態
//! （committed）は [`ScrollPlanner::scroll_state`] で読める（クリック範囲の実導出は
//! choice-render の責務）。M2 は `pos` の生成器（補間過程）だけを差し替える——`plan`/`commit`・
//! 量子化・ダーティ導出は再設計不要（R8.3）。

use crate::canvas::{ContentCanvas, Resident, ResidentContent};
use crate::layout::VisibleWindow;
use crate::region::{ImagePx, ScaleContract};
use crate::writing::WritingMode;

/// スクロール位置の内部表現（R8.2/9.3 の契約点・choice-render と M2 が読む）。
///
/// スクロール位置を**真位置**（f32 連続量）と**確定位置**（whole-pixel 整数）へ分離して
/// 保持する値オブジェクト。不変条件 `|committed − pos| ≤ 0.5`（`committed = round(pos)`
/// ゆえ恒真）。M2 補間は `pos` の生成器（補間過程）だけを差し替える——`committed`／写像
/// 規約は不変（R8.3）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollState {
    /// 真位置（物理 px・f32 連続量）＝ `block_offset × k`（ブロック軸スカラ・符号は素通し）。
    /// M2 補間はこの値の生成を差し替える。
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
        /// ダーティ矩形（物理 px 整数・面寸クランプ済み・露出帯 ∪ 変化行）。
        dirty: Vec<PhysicalRect>,
        /// dirty と交差する canvas 住人 index（描画対象・クリップで dirty 限定）。
        draw_lines: Vec<usize>,
    },
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
    /// 直近の真位置（f32 連続量・M2 で補間過程が更新元になる・初期 0）。
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

    /// スクロール位置契約点（R9.3/R8.3——choice-render／M2 が読む）。
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
/// **経験則の推定でなく実測**ゆえ「はみ出し < 行 pitch のギャップ」がフォント設計上必ず成立し、
/// 隣接行の em ボックスへ届かない（＝確定行の余計な再描画を構造的に起こさない）。
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
/// 前回確定時の canvas 行のスナップショット。内容文字列・ブロック軸位置・行寸を保持し、
/// 新 canvas の同 index 行と比較して差分（typewriter の現在行・catch-up の複数行・新規行）を
/// 一様に検出する。位置/寸は **canvas-local**（validrect-local image px・スクロール非依存）
/// ゆえ、可視窓のみ移動（内容不変）では全行の指紋が一致＝変化行ゼロになる。float は
/// ビット表現で同値比較する（既存 `FormatKey` の `to_bits()` 規律に同じ——全順序比較を避け
/// 同値判定のみ）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommittedLine {
    /// 行の内容文字列（グリフ ch の連結・非グリフ住人は空）。
    text: String,
    /// ブロック軸位置のビット表現（横書き＝dy・縦書き＝dx——canvas-local image px）。
    block_pos_bits: u32,
    /// 行寸 `(幅, 高さ)` のビット表現（image px）。
    extent_bits: (u32, u32),
    /// hover 印（Choice 行のみ非 0・非 Choice 行は常に 0）。非 hover の Choice 行は 0・
    /// hover 中は `ordinal + 1`（0 と衝突させない）。hover の付与/切替/解除で当該 Choice 行の
    /// 指紋だけが変わり、既存 `derive_dirty` が当該行のみをダーティ化する（R4.4）。
    choice_marker: u32,
}

impl ScrollPlanner {
    /// canvas の全住人から行指紋列を作る（純粋・決定論・DD4）。
    ///
    /// 住人 index と 1:1（layout 行 index と一致）。M1 の実装住人はグリフのみだが、画像/
    /// サーフェスのシーム住人も一様に扱える（空 text・零寸の指紋）。位置/寸は canvas-local
    /// ゆえスクロール非依存——`derive_dirty` の `prev_lines` として次フレームへ渡す。
    pub(crate) fn committed_lines(canvas: &ContentCanvas, mode: WritingMode) -> Vec<CommittedLine> {
        canvas
            .residents
            .iter()
            .map(|resident| line_fingerprint(resident, mode))
            .collect()
    }

    /// 後方縮退（面内 blit＋差分描画で保持できない content 減少）の判定（純粋・決定論・DD-9）。
    ///
    /// 次のいずれかで true——(a) 行数減少（`new_lines.len() < prev_lines.len()`＝行がスクロール
    /// アウト/消滅）、(b) 共通 prefix の同一 index 行で **block 軸位置が動いた**（旧位置のインクを
    /// 新変化行矩形が覆えない）、(c) 同一 index 行で **extent が縮んだ**（旧インクの外側＝退避分を
    /// 新矩形が覆えない）。前方 typewriter（prefix 伸長で extent 増加・block 不動）では false ゆえ
    /// 増分描画のホットパスを維持する。行内開始位置は layout の不変則（全行同一の行内開始）ゆえ、
    /// block 位置＋extent の指紋だけで被覆可否は健全に判定できる（text 内容は覆えれば無関係）。
    fn is_backward_shrink(prev_lines: &[CommittedLine], new_lines: &[CommittedLine]) -> bool {
        if new_lines.len() < prev_lines.len() {
            return true;
        }
        prev_lines.iter().zip(new_lines).any(|(prev, new)| {
            // (b) block 軸位置が動いた行は、旧位置のインクを新変化行矩形が覆えない。
            if prev.block_pos_bits != new.block_pos_bits {
                return true;
            }
            // (c) extent が縮んだ行は、旧インクの退避分（縮小の外側）を新矩形が覆えない。
            let (pw, ph) = (
                f32::from_bits(prev.extent_bits.0),
                f32::from_bits(prev.extent_bits.1),
            );
            let (nw, nh) = (
                f32::from_bits(new.extent_bits.0),
                f32::from_bits(new.extent_bits.1),
            );
            nw < pw || nh < ph
        })
    }

    /// ダーティ矩形と描画対象行を導出する（純粋・状態不変・DD4）。
    ///
    /// dirty ＝（a）露出帯（`blit` の逆側に生じる未保持領域・blit≠0 のとき 1 枚）∪（b）変化行
    /// （`prev_lines` と新 canvas の指紋差分行の物理矩形——**新スクロール位置**）。各矩形は物理 px
    /// 整数格子へ拡張（min を floor・max を ceil）し、ガード余白 `ceil(DIRTY_GUARD_IMG_PX × k)` を
    /// 全辺へ加えて面寸 `surface_size` へクランプ（退化矩形は除外）。
    ///
    /// `prev_lines` が空（初回・Clear 後・format 再構築）のときは**全域ダーティ**（面全域 1 枚・
    /// 描画対象＝全 GlyphRun 住人）を返す。返り値 `draw_lines` は dirty とブロック軸で交差する
    /// 全 GlyphRun 住人 index（クリップにより描画結果は dirty 内へ限定される前提ゆえ交差住人を
    /// 全て含む・`first_visible_line` で切らない）。
    ///
    /// 状態遷移（plan/commit・`FramePlan` の enum 化）は後続タスクの領分——本メソッドは
    /// 「変化なし＝空 dirty・空 draw_lines」「全域＝面全域」の導出結果を返すに留める。
    ///
    /// 実測はみ出し無し（em ボックス丈）の従来経路——測定値を持たない pure 層 unit 向けの薄い
    /// ラッパで、[`Self::derive_dirty_with_overhangs`] に空スライスを渡す。
    pub(crate) fn derive_dirty(
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        mode: WritingMode,
        contract: &ScaleContract,
        blit: (i32, i32),
        surface_size: (u32, u32),
        prev_lines: &[CommittedLine],
    ) -> (Vec<PhysicalRect>, Vec<usize>) {
        Self::derive_dirty_with_overhangs(
            canvas,
            window,
            mode,
            contract,
            blit,
            surface_size,
            prev_lines,
            &[],
        )
    }

    /// 実測インクはみ出し（[`LineOverhang`]・住人 index と 1:1）付きでダーティ矩形と描画対象行を
    /// 導出する（純粋・状態不変・DD4／D2）。変化行の em ボックスを `overhangs` の実測分だけ外側へ
    /// 広げてはみ出しインクを含める（`overhangs` が index を欠く／空なら既定 0＝em ボックス丈）。
    pub(crate) fn derive_dirty_with_overhangs(
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        mode: WritingMode,
        contract: &ScaleContract,
        blit: (i32, i32),
        surface_size: (u32, u32),
        prev_lines: &[CommittedLine],
        overhangs: &[LineOverhang],
    ) -> (Vec<PhysicalRect>, Vec<usize>) {
        // 住人 index の実測はみ出し（無ければ既定 0＝em ボックス丈・テスト等の非測定経路）。
        let overhang_of = |i: usize| overhangs.get(i).copied().unwrap_or_default();
        // ── 全域ダーティ（初回・Clear 後・format 再構築）: 面全域 1 枚・全 GlyphRun 住人 ──
        if prev_lines.is_empty() {
            let (w, h) = surface_size;
            let full = PhysicalRect { x: 0, y: 0, w, h };
            let dirty = if full.is_empty() {
                Vec::new()
            } else {
                vec![full]
            };
            let draw_lines = glyph_run_indices(canvas).collect();
            return (dirty, draw_lines);
        }

        let mut dirty: Vec<PhysicalRect> = Vec::new();

        // (a) 露出帯: blit の逆側の未保持領域（1 枚）。
        if let Some(band) = exposure_band(blit, surface_size) {
            if let Some(rect) = expand_guard_clamp(
                band.x as f32,
                band.y as f32,
                (band.x + band.w) as f32,
                (band.y + band.h) as f32,
                contract,
                surface_size,
            ) {
                dirty.push(rect);
            }
        }

        // (b) 変化行: 指紋差分行の物理矩形（新スクロール位置）。
        let block_offset = window.block_offset;
        for (i, resident) in canvas.residents.iter().enumerate() {
            let fingerprint = line_fingerprint(resident, mode);
            let changed = match prev_lines.get(i) {
                Some(prev) => *prev != fingerprint,
                None => true, // 新規行（prev.len() を超える index）。
            };
            if changed {
                if let Some(rect) = resident_rect(
                    resident,
                    block_offset,
                    mode,
                    contract,
                    surface_size,
                    overhang_of(i),
                ) {
                    dirty.push(rect);
                }
            }
        }

        // 描画対象行: dirty とブロック軸で交差する全 GlyphRun 住人（first_visible_line で切らない）。
        let mut draw_lines = Vec::new();
        for i in glyph_run_indices(canvas) {
            if let Some(rect) = resident_rect(
                &canvas.residents[i],
                block_offset,
                mode,
                contract,
                surface_size,
                overhang_of(i),
            ) {
                if dirty.iter().any(|d| d.intersects_block_axis(&rect, mode)) {
                    draw_lines.push(i);
                }
            }
        }

        (dirty, draw_lines)
    }
}

/// グリフ描画対象住人の index を昇順で返す（描画対象・全域ダーティの draw_lines 共通口）。
/// Choice 住人は内包 `run` を GlyphRun と同格に素描画するため同じく対象に含める（R9.5）。
fn glyph_run_indices(canvas: &ContentCanvas) -> impl Iterator<Item = usize> + '_ {
    canvas
        .residents
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            matches!(
                r.content,
                ResidentContent::GlyphRun(_) | ResidentContent::Choice(_)
            )
        })
        .map(|(i, _)| i)
}

/// 住人 1 行の行指紋（内容文字列・ブロック軸位置・行寸）を作る（canvas-local・DD4）。
fn line_fingerprint(resident: &Resident, mode: WritingMode) -> CommittedLine {
    let (text, extent) = match &resident.content {
        ResidentContent::GlyphRun(run) => (
            run.glyphs.iter().map(|g| g.ch).collect::<String>(),
            run.size,
        ),
        // Choice 住人は内包 run から GlyphRun と同一の指紋を作る（非 hover の素描画が
        // GlyphRun と同一ゆえ指紋も同一・R9.5。hover セグメントの指紋反映は task 6.1）。
        ResidentContent::Choice(choice) => (
            choice.run.glyphs.iter().map(|g| g.ch).collect::<String>(),
            choice.run.size,
        ),
        // 画像/サーフェスのシーム住人は空 text・零寸（M1 は from_layout で生成されない）。
        ResidentContent::Image(_) | ResidentContent::Surface(_) => (String::new(), (0.0, 0.0)),
    };
    let (dx, dy) = resident.transform.offset();
    // ブロック軸位置: 横書き＝dy・縦書き＝dx（canvas-local・スクロール非依存）。
    let block_pos = match mode {
        WritingMode::HorizontalTb => dy,
        WritingMode::VerticalRl | WritingMode::VerticalLr => dx,
    };
    // hover 印: Choice 行のみ非 0（非 hover=0・hover ordinal o=o+1 で 0 と衝突させない）。
    // 非 Choice 住人は常に 0。hover の付与/切替/解除で当該 Choice 行の指紋だけが変わり、
    // 既存 line_fingerprint/derive_dirty のアルゴリズムを無改変のまま当該行のみをダーティ化する
    // （R4.4）。
    let choice_marker = match &resident.content {
        ResidentContent::Choice(choice) => choice.hovered.map_or(0, |o| o as u32 + 1),
        ResidentContent::GlyphRun(_)
        | ResidentContent::Image(_)
        | ResidentContent::Surface(_) => 0,
    };
    CommittedLine {
        text,
        block_pos_bits: block_pos.to_bits(),
        extent_bits: (extent.0.to_bits(), extent.1.to_bits()),
        choice_marker,
    }
}

/// 住人の描画面矩形（新スクロール位置・物理 px 整数・ガード＋クランプ済み）を返す。
///
/// canvas-local 位置 `transform.offset()` にブロック軸の `block_offset` を加算し × k した float
/// 矩形を [`expand_guard_clamp`] へ通す。GlyphRun／Choice 住人は内包 `run` から同一のインク矩形を
/// 導く（R9.5）。非グリフ住人（シーム）は描画実体を持たないため `None`（描画対象にもダーティ
/// にも寄与しない——COM 層 draw の warn!＋skip と整合）。
fn resident_rect(
    resident: &Resident,
    block_offset: f32,
    mode: WritingMode,
    contract: &ScaleContract,
    surface_size: (u32, u32),
    overhang: LineOverhang,
) -> Option<PhysicalRect> {
    let run = match &resident.content {
        ResidentContent::GlyphRun(run) => run,
        // Choice 住人は内包 run から GlyphRun と同一のインク矩形を導く（R9.5）。
        ResidentContent::Choice(choice) => &choice.run,
        // 非グリフ住人（シーム）は描画実体を持たない＝ダーティにも描画対象にも寄与しない。
        ResidentContent::Image(_) | ResidentContent::Surface(_) => return None,
    };
    let (dx, dy) = resident.transform.offset();
    let (w, h) = run.size;
    let k = contract.scale;
    // em ボックス（image px・validrect-local + block_offset）をブロック軸の**実測はみ出し**
    // （[`LineOverhang`]・`GetOverhangMetrics` 由来）だけ外側へ広げ、はみ出しインクをダーティに
    // 含める（byte 等価の前提・D2）。行内軸は em 寸（run.size）のまま——行内はみ出し（イタリック
    // 右張り出し等）は tight box が要るため後続（現状 overhang.left/right は縦書きのブロック軸用）。
    // ブロック軸: horizontal＝Y（上へ top・下へ bottom）／vertical＝X（左へ left・右へ right）。
    let (ix0, iy0, ix1, iy1) = match mode {
        WritingMode::HorizontalTb => {
            let (x0, y0) = (dx, dy + block_offset);
            (x0, y0 - overhang.top, x0 + w, y0 + h + overhang.bottom)
        }
        WritingMode::VerticalRl | WritingMode::VerticalLr => {
            let (x0, y0) = (dx + block_offset, dy);
            (x0 - overhang.left, y0, x0 + w + overhang.right, y0 + h)
        }
    };
    expand_guard_clamp(ix0 * k, iy0 * k, ix1 * k, iy1 * k, contract, surface_size)
}

/// スクロール blit の逆側に生じる露出帯（未保持領域・物理 px 整数・ガード前）を返す。
///
/// 写像正準表（DD1）: 横書き（blit=y）＝内容が上（by<0）で下端露出・内容が下（by>0）で上端露出。
/// 縦書き（blit=x）＝vertical_lr の内容が左（bx<0）で右端露出・vertical_rl の内容が右（bx>0）で
/// 左端露出。blit=0（露出なし）は `None`。帯幅は面寸でクランプ（`|blit| ≥ 面寸`は全域）。
fn exposure_band(blit: (i32, i32), surface_size: (u32, u32)) -> Option<PhysicalRect> {
    let (w, h) = surface_size;
    let (bx, by) = blit;
    if by != 0 {
        let mag = by.unsigned_abs().min(h);
        if mag == 0 {
            return None;
        }
        return Some(if by < 0 {
            PhysicalRect {
                x: 0,
                y: h - mag,
                w,
                h: mag,
            } // 内容が上へ → 下端露出
        } else {
            PhysicalRect {
                x: 0,
                y: 0,
                w,
                h: mag,
            } // 内容が下へ → 上端露出
        });
    }
    if bx != 0 {
        let mag = bx.unsigned_abs().min(w);
        if mag == 0 {
            return None;
        }
        return Some(if bx < 0 {
            PhysicalRect {
                x: w - mag,
                y: 0,
                w: mag,
                h,
            } // 内容が左へ → 右端露出
        } else {
            PhysicalRect {
                x: 0,
                y: 0,
                w: mag,
                h,
            } // 内容が右へ → 左端露出
        });
    }
    None
}

/// float 矩形を物理 px 整数格子へ拡張（min を floor・max を ceil）し、ガード余白
/// `ceil(DIRTY_GUARD_IMG_PX × k)` を全辺へ加えて面寸へクランプする（退化は `None`）。
fn expand_guard_clamp(
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    contract: &ScaleContract,
    surface_size: (u32, u32),
) -> Option<PhysicalRect> {
    let (w, h) = (surface_size.0 as i64, surface_size.1 as i64);
    // ガード余白（物理 px・全辺）。負や面外は i64 で扱ってから 0..面寸へクランプする。
    let guard = (DIRTY_GUARD_IMG_PX * contract.scale).ceil() as i64;
    let x0 = (min_x.floor() as i64 - guard).clamp(0, w);
    let y0 = (min_y.floor() as i64 - guard).clamp(0, h);
    let x1 = (max_x.ceil() as i64 + guard).clamp(0, w);
    let y1 = (max_y.ceil() as i64 + guard).clamp(0, h);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some(PhysicalRect {
        x: x0 as u32,
        y: y0 as u32,
        w: (x1 - x0) as u32,
        h: (y1 - y0) as u32,
    })
}

#[cfg(test)]
#[path = "viewbox_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "viewbox_axis_tests.rs"]
mod axis_tests;
#[cfg(test)]
#[path = "viewbox_dirty_tests.rs"]
mod dirty_tests;
#[cfg(test)]
#[path = "viewbox_plan_commit_tests.rs"]
mod plan_commit_tests;
#[cfg(test)]
#[path = "viewbox_choice_marker_tests.rs"]
mod choice_marker_tests;
