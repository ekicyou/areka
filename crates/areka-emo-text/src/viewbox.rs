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

    /// 1 フレームの描画計画を返す（**状態不変・純粋**・`&self`・R2.3/4.3）。
    ///
    /// - `clear_requested` が立っていれば [`FramePlan::FullClear`]（描画 0 件・back 全域 Clear）。
    /// - それ以外は現状 `committed` から目標（`window.block_offset` の量子化）への blit と、
    ///   前回確定 `prev_lines` に対する [`Self::derive_dirty`] の結果（露出帯 ∪ 変化行）を組む。
    ///   blit が 0 かつ dirty が空なら [`FramePlan::NoChange`]（blit・描画・present とも 0）、
    ///   さもなくば [`FramePlan::Update`]。
    ///
    /// `self` を一切変えないため、同一入力の反復 `plan` は同一計画を返す（デバイス失敗フレームは
    /// 未 commit のまま次フレームで再計画＝再試行安全）。確定は [`Self::commit`] の役目。
    pub fn plan(
        &self,
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        mode: WritingMode,
        contract: &ScaleContract,
        surface_size: (u32, u32),
    ) -> FramePlan {
        if self.clear_requested {
            return FramePlan::FullClear;
        }
        let target = self.resolve_position(window.block_offset, contract);
        let blit = self.blit_vector(&target, mode);
        let (dirty, draw_lines) = Self::derive_dirty(
            canvas,
            window,
            mode,
            contract,
            blit,
            surface_size,
            &self.prev_lines,
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
    pub(crate) fn derive_dirty(
        canvas: &ContentCanvas,
        window: &VisibleWindow,
        mode: WritingMode,
        contract: &ScaleContract,
        blit: (i32, i32),
        surface_size: (u32, u32),
        prev_lines: &[CommittedLine],
    ) -> (Vec<PhysicalRect>, Vec<usize>) {
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
                if let Some(rect) =
                    resident_rect(resident, block_offset, mode, contract, surface_size)
                {
                    dirty.push(rect);
                }
            }
        }

        // 描画対象行: dirty とブロック軸で交差する全 GlyphRun 住人（first_visible_line で切らない）。
        let mut draw_lines = Vec::new();
        for i in glyph_run_indices(canvas) {
            if let Some(rect) =
                resident_rect(&canvas.residents[i], block_offset, mode, contract, surface_size)
            {
                if dirty.iter().any(|d| d.intersects_block_axis(&rect, mode)) {
                    draw_lines.push(i);
                }
            }
        }

        (dirty, draw_lines)
    }
}

/// GlyphRun 住人の index を昇順で返す（描画対象・全域ダーティの draw_lines 共通口）。
fn glyph_run_indices(canvas: &ContentCanvas) -> impl Iterator<Item = usize> + '_ {
    canvas
        .residents
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r.content, ResidentContent::GlyphRun(_)))
        .map(|(i, _)| i)
}

/// 住人 1 行の行指紋（内容文字列・ブロック軸位置・行寸）を作る（canvas-local・DD4）。
fn line_fingerprint(resident: &Resident, mode: WritingMode) -> CommittedLine {
    let (text, extent) = match &resident.content {
        ResidentContent::GlyphRun(run) => {
            (run.glyphs.iter().map(|g| g.ch).collect::<String>(), run.size)
        }
        // 画像/サーフェスのシーム住人は空 text・零寸（M1 は from_layout で生成されない）。
        ResidentContent::Image(_) | ResidentContent::Surface(_) => (String::new(), (0.0, 0.0)),
    };
    let (dx, dy) = resident.transform.offset();
    // ブロック軸位置: 横書き＝dy・縦書き＝dx（canvas-local・スクロール非依存）。
    let block_pos = match mode {
        WritingMode::HorizontalTb => dy,
        WritingMode::VerticalRl | WritingMode::VerticalLr => dx,
    };
    CommittedLine {
        text,
        block_pos_bits: block_pos.to_bits(),
        extent_bits: (extent.0.to_bits(), extent.1.to_bits()),
    }
}

/// 住人の描画面矩形（新スクロール位置・物理 px 整数・ガード＋クランプ済み）を返す。
///
/// canvas-local 位置 `transform.offset()` にブロック軸の `block_offset` を加算し × k した float
/// 矩形を [`expand_guard_clamp`] へ通す。非 GlyphRun 住人（シーム）は描画実体を持たないため
/// `None`（描画対象にもダーティにも寄与しない——COM 層 draw の warn!＋skip と整合）。
fn resident_rect(
    resident: &Resident,
    block_offset: f32,
    mode: WritingMode,
    contract: &ScaleContract,
    surface_size: (u32, u32),
) -> Option<PhysicalRect> {
    let ResidentContent::GlyphRun(run) = &resident.content else {
        return None;
    };
    let (dx, dy) = resident.transform.offset();
    let (w, h) = run.size;
    let k = contract.scale;
    // 新スクロール位置: ブロック軸へ block_offset を加算してから × k（行内軸は × k のみ）。
    let (min_x, min_y) = match mode {
        WritingMode::HorizontalTb => (dx * k, (dy + block_offset) * k),
        WritingMode::VerticalRl | WritingMode::VerticalLr => ((dx + block_offset) * k, dy * k),
    };
    expand_guard_clamp(
        min_x,
        min_y,
        min_x + w * k,
        min_y + h * k,
        contract,
        surface_size,
    )
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
            PhysicalRect { x: 0, y: h - mag, w, h: mag } // 内容が上へ → 下端露出
        } else {
            PhysicalRect { x: 0, y: 0, w, h: mag } // 内容が下へ → 上端露出
        });
    }
    if bx != 0 {
        let mag = bx.unsigned_abs().min(w);
        if mag == 0 {
            return None;
        }
        return Some(if bx < 0 {
            PhysicalRect { x: w - mag, y: 0, w: mag, h } // 内容が左へ → 右端露出
        } else {
            PhysicalRect { x: 0, y: 0, w: mag, h } // 内容が右へ → 左端露出
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
mod tests {
    use areka_parsers::balloon::{
        BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
    };

    use super::{
        DIRTY_GUARD_IMG_PX, FramePlan, PhysicalRect, ScrollPlanner, ScrollState, block_axis_vector,
    };
    use crate::canvas::ContentCanvas;
    use crate::layout::{FixedMetrics, LayoutEngine, VisibleWindow};
    use crate::region::{ScaleContract, TextRegion};
    use crate::state::TextItem;
    use crate::writing::WritingMode;

    /// 3 方向の代表 `block_offset`（符号は layout.rs 正準規約）:
    /// 横書き＝負（内容が上）・vertical_rl＝正（内容が右）・vertical_lr＝負（内容が左）。
    const HORIZONTAL_OFFSET: f32 = -13.0;
    const VERTICAL_RL_OFFSET: f32 = 13.0;
    const VERTICAL_LR_OFFSET: f32 = -13.0;

    // ── R5.1–5.3: 軸写像（横=y・縦=x）と符号素通し（独自の軸規則を発明しない） ──

    /// 横書きはブロック軸＝y（`(0, v)`）・縦書きは x（`(v, 0)`）へ写り、符号は
    /// `block_offset` をそのまま素通しする（layout.rs/draw.rs の軸割当正準と 1:1）。
    #[test]
    fn block_offset_maps_to_expected_axis_and_sign_per_writing_mode() {
        let contract = ScaleContract::new(1.0, None);
        let planner = ScrollPlanner::new();

        // 横書き: committed=-13 → y 軸のみ（x=0）・符号（負）素通し。
        let h = planner.resolve_position(HORIZONTAL_OFFSET, &contract);
        assert_eq!(h.committed, -13);
        assert_eq!(
            block_axis_vector(WritingMode::HorizontalTb, h.committed),
            (0, -13)
        );

        // vertical_rl: committed=13 → x 軸のみ（y=0）・符号（正）素通し。
        let rl = planner.resolve_position(VERTICAL_RL_OFFSET, &contract);
        assert_eq!(rl.committed, 13);
        assert_eq!(
            block_axis_vector(WritingMode::VerticalRl, rl.committed),
            (13, 0)
        );

        // vertical_lr: committed=-13 → x 軸のみ（y=0）・符号（負）素通し（rl と同軸・符号のみ逆）。
        let lr = planner.resolve_position(VERTICAL_LR_OFFSET, &contract);
        assert_eq!(lr.committed, -13);
        assert_eq!(
            block_axis_vector(WritingMode::VerticalLr, lr.committed),
            (-13, 0)
        );
    }

    /// 軸写像は符号を発明しない——正負ゼロいずれのスカラも選択軸へそのまま透過する
    /// （横書き＝y・縦書き 2 方向＝x の全網羅）。
    #[test]
    fn block_axis_vector_passes_sign_through_on_selected_axis() {
        for v in [-7, 0, 5] {
            assert_eq!(block_axis_vector(WritingMode::HorizontalTb, v), (0, v));
            assert_eq!(block_axis_vector(WritingMode::VerticalRl, v), (v, 0));
            assert_eq!(block_axis_vector(WritingMode::VerticalLr, v), (v, 0));
        }
    }

    // ── DD11/R6.4/R8.2: 真位置と量子化（真位置からの直接丸め・round half away from zero） ──

    /// 量子化は真位置からの直接丸め（`round(pos)`）で、半整数は 0 から遠い側へ丸める
    /// （trunc/floor 変異を殺す境界値・k=1.0 の端数 `block_offset`＝ratio 改行由来）。
    #[test]
    fn quantization_is_direct_round_half_away_from_zero() {
        let contract = ScaleContract::new(1.0, None);
        let planner = ScrollPlanner::new();
        // -7.5 → -8（trunc なら -7）。
        assert_eq!(planner.resolve_position(-7.5, &contract).committed, -8);
        // 7.5 → 8（trunc/floor なら 7）。
        assert_eq!(planner.resolve_position(7.5, &contract).committed, 8);
    }

    /// k=1.0 かつ行単位 `block_offset`（整数）では pos が整数＝`committed == pos`
    /// （byte 一致の構造前提）。
    #[test]
    fn scale_one_integer_offset_yields_committed_equal_to_pos() {
        let contract = ScaleContract::new(1.0, None);
        let state = ScrollPlanner::new().resolve_position(-39.0, &contract);
        assert_eq!(state.pos, -39.0);
        assert_eq!(state.committed as f32, state.pos);
    }

    /// k≠1.0（1.25）でも真位置からの直接丸めで `|committed − pos| ≤ 0.5` を保つ
    /// （小数アキュムレータの単発檻・長スクロール列の累積檻は後続 3.4 の担当）。
    #[test]
    fn nonunit_scale_keeps_committed_within_half_pixel() {
        let contract = ScaleContract::new(1.25, None);
        let planner = ScrollPlanner::new();
        for block_offset in [-39.0f32, -13.0, -10.0, 10.0, 13.0, 39.0] {
            let state = planner.resolve_position(block_offset, &contract);
            let pos = block_offset * 1.25;
            assert_eq!(state.pos, pos, "真位置は block_offset × k");
            assert_eq!(state.committed, pos.round() as i32, "確定位置は round(pos)");
            assert!(
                (state.committed as f32 - state.pos).abs() <= 0.5,
                "block_offset {block_offset}: |committed − pos| ≤ 0.5"
            );
        }
        // 端数 0.5 ちょうどの境界（10 × 1.25 = 12.5 → 13・round half away）。
        let boundary = planner.resolve_position(10.0, &contract);
        assert_eq!(boundary.committed, 13);
        assert!((boundary.committed as f32 - boundary.pos).abs() <= 0.5);
    }

    // ── R7.5 系: 純関数（同一入力→同一出力）・契約点の初期状態・blit 軸写像委譲 ──

    /// 同一 `block_offset` の反復算出は同一 `ScrollState` を返す（決定論・純関数）。
    #[test]
    fn resolve_position_is_deterministic() {
        let contract = ScaleContract::new(1.25, None);
        let planner = ScrollPlanner::new();
        let first = planner.resolve_position(-27.0, &contract);
        let second = planner.resolve_position(-27.0, &contract);
        assert_eq!(first, second);
    }

    /// 初期計画者の契約点は真位置 0・確定位置 0（commit 前・後続 3.2 の状態遷移なし）。
    #[test]
    fn new_planner_scroll_state_is_zero() {
        assert_eq!(
            ScrollPlanner::new().scroll_state(),
            ScrollState {
                pos: 0.0,
                committed: 0
            }
        );
    }

    /// blit ベクトルは確定位置差をブロック軸へ写す（初期 committed=0 ゆえ target の軸写像）。
    #[test]
    fn blit_vector_maps_target_committed_to_block_axis() {
        let contract = ScaleContract::new(1.0, None);
        let planner = ScrollPlanner::new();
        let target = planner.resolve_position(-13.0, &contract); // committed=-13
        assert_eq!(
            planner.blit_vector(&target, WritingMode::HorizontalTb),
            (0, -13)
        );
        assert_eq!(
            planner.blit_vector(&target, WritingMode::VerticalRl),
            (-13, 0)
        );
        // block_axis_vector への委譲の檻（同一結果）。
        assert_eq!(
            planner.blit_vector(&target, WritingMode::VerticalLr),
            block_axis_vector(WritingMode::VerticalLr, target.committed)
        );
    }

    // ── 3.2 R2.2/3.2/3.3/4.2: ダーティ導出（露出帯 ∪ 変化行 ∪ 全域）の檻 ──
    //
    // 幾何の共通前提: FixedMetrics・font 10 → pitch 13（ceil(12.5)）・全角 1 グリフ/行。
    // 露出帯の辺は写像正準表（横書き＝下端・vertical_rl＝左端・vertical_lr＝右端）。

    /// テスト画像原寸（image px・他モジュール檻と同一値）。
    const IMAGE: (u32, u32) = (400, 224);

    /// PhysicalRect 短縮構築。
    fn phys(x: u32, y: u32, w: u32, h: u32) -> PhysicalRect {
        PhysicalRect { x, y, w, h }
    }

    /// テスト用 BalloonModel（origin (0,0)・折返し既定・validrect 指定）。
    fn model_rect(
        validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
    ) -> BalloonModel {
        BalloonModel::new(
            WindowPosition::new(None, None),
            Origin::new(Some(0), Some(0)),
            WordWrapPoint::new(None, None),
            ValidRect::new(validrect.0, validrect.1, validrect.2, validrect.3),
            Font::new(None, None, FontColor::new(None, None, None)),
            None,
        )
    }

    /// n 行（各行 全角 1 グリフ・明示改行 ratio 1.0 区切り）の item 列。
    fn broken_lines(n: usize) -> Vec<TextItem> {
        let mut items = Vec::new();
        for i in 0..n {
            if i > 0 {
                items.push(TextItem::LineBreak { ratio: 1.0 });
            }
            items.push(TextItem::Glyph { ch: 'あ' });
        }
        items
    }

    /// items→canvas の通し（validrect-local・visible は全量）。
    fn canvas_for(
        items: &[TextItem],
        mode: WritingMode,
        validrect: (Option<i32>, Option<i32>, Option<i32>, Option<i32>),
        font_height: f32,
    ) -> ContentCanvas {
        let region = TextRegion::resolve(&model_rect(validrect), IMAGE, mode);
        let visible = items
            .iter()
            .filter(|i| matches!(i, TextItem::Glyph { .. }))
            .count();
        let lines = LayoutEngine::layout(items, visible, &region, mode, font_height, &FixedMetrics);
        ContentCanvas::from_layout(&lines, &region, mode)
    }

    /// 可視窓のみ移動（content 不変・block_offset だけ変化・blit≠0）。
    fn window(first_visible_line: usize, block_offset: f32) -> VisibleWindow {
        VisibleWindow {
            first_visible_line,
            block_offset,
        }
    }

    // ── R2.2/3.2/5.1–5.3: 可視窓のみ移動 → dirty＝露出帯のみ（変化行ゼロ）・軸別の辺 ──

    /// 横書きで可視窓のみ移動（content 不変）→ dirty は下端露出帯のみ・変化行ゼロ。
    /// 露出帯はガード余白（1 image px × k=1）で 1px 全辺拡張して面寸クランプされる。
    #[test]
    fn visible_window_only_move_horizontal_dirties_bottom_band_only() {
        let contract = ScaleContract::new(1.0, None);
        let canvas = canvas_for(
            &broken_lines(4),
            WritingMode::HorizontalTb,
            (Some(0), Some(100), Some(0), Some(400)),
            10.0,
        );
        // 前回確定＝同一 canvas の指紋（canvas-local ゆえスクロール非依存）。
        let prev = ScrollPlanner::committed_lines(&canvas, WritingMode::HorizontalTb);
        // 内容を動かさず block_offset だけ −13（内容が上へ）・blit も同量。
        let (dirty, _draw) = ScrollPlanner::derive_dirty(
            &canvas,
            &window(1, -13.0),
            WritingMode::HorizontalTb,
            &contract,
            (0, -13),
            (400, 100),
            &prev,
        );
        // 下端の露出帯 {0,87,400,13} をガード 1px で拡張＋クランプ → {0,86,400,14}。
        assert_eq!(dirty, vec![phys(0, 86, 400, 14)], "露出帯のみ・変化行ゼロ");
    }

    /// vertical_rl で可視窓のみ移動 → dirty は左端露出帯のみ（行が左へ流れる）。
    #[test]
    fn visible_window_only_move_vertical_rl_dirties_left_band_only() {
        let contract = ScaleContract::new(1.0, None);
        let canvas = canvas_for(
            &broken_lines(2),
            WritingMode::VerticalRl,
            (Some(0), Some(200), Some(0), Some(100)),
            10.0,
        );
        let prev = ScrollPlanner::committed_lines(&canvas, WritingMode::VerticalRl);
        // vertical_rl＝内容が右（正）・blit も +13（左端が露出）。
        let (dirty, _draw) = ScrollPlanner::derive_dirty(
            &canvas,
            &window(1, 13.0),
            WritingMode::VerticalRl,
            &contract,
            (13, 0),
            (100, 200),
            &prev,
        );
        // 左端の露出帯 {0,0,13,200} をガード拡張＋クランプ → {0,0,14,200}。
        assert_eq!(dirty, vec![phys(0, 0, 14, 200)], "左端露出帯のみ");
    }

    /// vertical_lr で可視窓のみ移動 → dirty は右端露出帯のみ（行が右へ流れる）。
    #[test]
    fn visible_window_only_move_vertical_lr_dirties_right_band_only() {
        let contract = ScaleContract::new(1.0, None);
        let canvas = canvas_for(
            &broken_lines(2),
            WritingMode::VerticalLr,
            (Some(0), Some(200), Some(0), Some(100)),
            10.0,
        );
        let prev = ScrollPlanner::committed_lines(&canvas, WritingMode::VerticalLr);
        // vertical_lr＝内容が左（負）・blit も −13（右端が露出）。
        let (dirty, _draw) = ScrollPlanner::derive_dirty(
            &canvas,
            &window(1, -13.0),
            WritingMode::VerticalLr,
            &contract,
            (-13, 0),
            (100, 200),
            &prev,
        );
        // 右端の露出帯 {87,0,13,200} をガード拡張＋クランプ → {86,0,14,200}。
        assert_eq!(dirty, vec![phys(86, 0, 14, 200)], "右端露出帯のみ");
    }

    // ── R3.3: typewriter 進行（現在行の text 伸長）→ dirty＝現在行のみ ──

    /// typewriter で現在行が 1 グリフ伸びる（blit なし）→ dirty は現在行の矩形のみ。
    #[test]
    fn typewriter_single_glyph_dirties_current_line_only() {
        let contract = ScaleContract::new(1.0, None);
        let vr = (Some(0), Some(224), Some(0), Some(400));
        // 前回: 1 グリフ「あ」・今回: 2 グリフ「ああ」（同一行・折返しなし）。
        let prev_canvas = canvas_for(
            &[TextItem::Glyph { ch: 'あ' }],
            WritingMode::HorizontalTb,
            vr,
            10.0,
        );
        let prev = ScrollPlanner::committed_lines(&prev_canvas, WritingMode::HorizontalTb);
        let canvas = canvas_for(
            &[TextItem::Glyph { ch: 'あ' }, TextItem::Glyph { ch: 'あ' }],
            WritingMode::HorizontalTb,
            vr,
            10.0,
        );
        let (dirty, draw) = ScrollPlanner::derive_dirty(
            &canvas,
            &window(0, 0.0),
            WritingMode::HorizontalTb,
            &contract,
            (0, 0),
            (400, 224),
            &prev,
        );
        // 現在行 {0,0,20,10} をガード拡張＋クランプ → {0,0,21,11}。露出帯なし（blit=0）。
        assert_eq!(dirty, vec![phys(0, 0, 21, 11)], "現在行のみ");
        assert_eq!(draw, vec![0], "描画対象は現在行のみ");
    }

    /// 確定行は変化行に含まれない: 2 行で末尾行だけ伸長 → dirty/draw に確定行 0 が現れない。
    #[test]
    fn confirmed_line_is_excluded_from_dirty_and_draw() {
        let contract = ScaleContract::new(1.0, None);
        let vr = (Some(0), Some(100), Some(0), Some(400));
        // 前回: 行0「ああ」・行1「い」。今回: 行0 不変・行1「いろ」へ伸長。
        let prev_canvas = canvas_for(
            &[
                TextItem::Glyph { ch: 'あ' },
                TextItem::Glyph { ch: 'あ' },
                TextItem::LineBreak { ratio: 1.0 },
                TextItem::Glyph { ch: 'い' },
            ],
            WritingMode::HorizontalTb,
            vr,
            10.0,
        );
        let prev = ScrollPlanner::committed_lines(&prev_canvas, WritingMode::HorizontalTb);
        let canvas = canvas_for(
            &[
                TextItem::Glyph { ch: 'あ' },
                TextItem::Glyph { ch: 'あ' },
                TextItem::LineBreak { ratio: 1.0 },
                TextItem::Glyph { ch: 'い' },
                TextItem::Glyph { ch: 'ろ' },
            ],
            WritingMode::HorizontalTb,
            vr,
            10.0,
        );
        let (dirty, draw) = ScrollPlanner::derive_dirty(
            &canvas,
            &window(0, 0.0),
            WritingMode::HorizontalTb,
            &contract,
            (0, 0),
            (400, 100),
            &prev,
        );
        // 行1「いろ」{0,13,20,10} をガード拡張 → {0,12,21,12}。行0 は現れない。
        assert_eq!(dirty, vec![phys(0, 12, 21, 12)], "変化行（末尾）のみ");
        assert_eq!(draw, vec![1], "確定行 0 は draw から除外される");
    }

    // ── R3.3/4.2: catch-up 複数行・新規行追加 → 変化行の和 ──

    /// 2 行から 4 行へ一挙に伸びる（catch-up＋新規行）→ dirty は新規 2 行の和。
    #[test]
    fn catchup_and_new_lines_union_of_changed_lines() {
        let contract = ScaleContract::new(1.0, None);
        let vr = (Some(0), Some(100), Some(0), Some(400));
        let prev_canvas = canvas_for(&broken_lines(2), WritingMode::HorizontalTb, vr, 10.0);
        let prev = ScrollPlanner::committed_lines(&prev_canvas, WritingMode::HorizontalTb);
        let canvas = canvas_for(&broken_lines(4), WritingMode::HorizontalTb, vr, 10.0);
        let (dirty, draw) = ScrollPlanner::derive_dirty(
            &canvas,
            &window(0, 0.0),
            WritingMode::HorizontalTb,
            &contract,
            (0, 0),
            (400, 100),
            &prev,
        );
        // 行2 {0,26,10,10}→{0,25,11,12}・行3 {0,39,10,10}→{0,38,11,12}。行0/1 は不変。
        assert_eq!(dirty, vec![phys(0, 25, 11, 12), phys(0, 38, 11, 12)]);
        assert_eq!(draw, vec![2, 3], "新規 2 行のみ描画対象");
    }

    // ── R4.2/3.3: 初回（前回指紋なし）→ 全域ダーティ・全 GlyphRun 住人 ──

    /// prev_lines が空（初回・Clear 後・format 再構築）→ dirty＝面全域 1 枚・draw＝全住人。
    #[test]
    fn empty_prev_is_full_domain() {
        let contract = ScaleContract::new(1.0, None);
        let canvas = canvas_for(
            &broken_lines(3),
            WritingMode::HorizontalTb,
            (Some(0), Some(100), Some(0), Some(400)),
            10.0,
        );
        let (dirty, draw) = ScrollPlanner::derive_dirty(
            &canvas,
            &window(0, 0.0),
            WritingMode::HorizontalTb,
            &contract,
            (0, 0),
            (400, 100),
            &[],
        );
        assert_eq!(dirty, vec![phys(0, 0, 400, 100)], "全域 1 枚");
        assert_eq!(draw, vec![0, 1, 2], "全 GlyphRun 住人");
    }

    // ── 整数格子拡張＋ガード＋クランプ（k=1.0／k=1.25 で面寸を越えない） ──

    /// ガード定数は 1 image px（保守既定・AA こぼれ吸収）。
    #[test]
    fn dirty_guard_is_one_image_px() {
        assert_eq!(DIRTY_GUARD_IMG_PX, 1.0);
    }

    /// k=1.25: 全域は面寸ちょうど・端の変化行/露出帯はクランプされ面寸を越えない。
    /// ガード余白は物理 px `ceil(1.0 × 1.25)=2`。
    #[test]
    fn nonunit_scale_expands_and_clamps_within_surface() {
        let contract = ScaleContract::new(1.25, None);
        let vr = (Some(0), Some(100), Some(0), Some(400));
        let surface = (500u32, 125u32); // ceil(400×1.25), ceil(100×1.25)

        // (a) 全域＝面寸ちょうど（クランプが面寸を根拠にする）。
        let full_canvas = canvas_for(&broken_lines(4), WritingMode::HorizontalTb, vr, 10.0);
        let (full_dirty, _) = ScrollPlanner::derive_dirty(
            &full_canvas,
            &window(0, 0.0),
            WritingMode::HorizontalTb,
            &contract,
            (0, 0),
            surface,
            &[],
        );
        assert_eq!(full_dirty, vec![phys(0, 0, 500, 125)], "全域は面寸ちょうど");

        // (b) 先頭行伸長 → 変化行が y=0 側でガードにより負へ出るが 0 へクランプ。
        let prev_canvas = canvas_for(
            &[TextItem::Glyph { ch: 'あ' }],
            WritingMode::HorizontalTb,
            vr,
            10.0,
        );
        let prev = ScrollPlanner::committed_lines(&prev_canvas, WritingMode::HorizontalTb);
        let grown = canvas_for(
            &[TextItem::Glyph { ch: 'あ' }, TextItem::Glyph { ch: 'あ' }],
            WritingMode::HorizontalTb,
            vr,
            10.0,
        );
        let (dirty, _) = ScrollPlanner::derive_dirty(
            &grown,
            &window(0, 0.0),
            WritingMode::HorizontalTb,
            &contract,
            (0, 0),
            surface,
            &prev,
        );
        // 行0 {0,0,20,10}×1.25 → floor/ceil {0,0,25,13}・ガード 2 → {-2,-2,27,15}・クランプ → {0,0,27,15}。
        assert_eq!(dirty, vec![phys(0, 0, 27, 15)], "y 負側はガード後 0 へクランプ");
        assert!(
            dirty.iter().all(|r| r.x + r.w <= surface.0 && r.y + r.h <= surface.1),
            "全ダーティ矩形は面寸を越えない"
        );

        // (c) 端の露出帯もクランプされる（下端 by=-13）。
        let scroll_canvas = canvas_for(&broken_lines(4), WritingMode::HorizontalTb, vr, 10.0);
        let scroll_prev =
            ScrollPlanner::committed_lines(&scroll_canvas, WritingMode::HorizontalTb);
        let (band, _) = ScrollPlanner::derive_dirty(
            &scroll_canvas,
            &window(1, -13.0),
            WritingMode::HorizontalTb,
            &contract,
            (0, -13),
            surface,
            &scroll_prev,
        );
        // 下端露出帯 {0,112,500,13}・ガード 2 → {-2,110,502,127}・クランプ → {0,110,500,15}。
        assert_eq!(band, vec![phys(0, 110, 500, 15)], "端の露出帯はクランプされる");
        assert!(
            band.iter().all(|r| r.x + r.w <= surface.0 && r.y + r.h <= surface.1),
            "露出帯も面寸を越えない"
        );
    }

    // ── back 全被覆の素朴檻: blit 写域 ∪ dirty ＝ 面全域（ブロック軸） ──

    /// 横書きスクロール（by=-13）で blit 写域（保持ピクセルの移動先）と dirty（露出帯）の
    /// 和がブロック軸（y）全域を隙間なく覆う（残像漏れの構造檻・簡易ケース）。
    #[test]
    fn back_is_fully_covered_by_blit_and_dirty_horizontal() {
        let contract = ScaleContract::new(1.0, None);
        let surface = (400u32, 100u32);
        let canvas = canvas_for(
            &broken_lines(4),
            WritingMode::HorizontalTb,
            (Some(0), Some(100), Some(0), Some(400)),
            10.0,
        );
        let prev = ScrollPlanner::committed_lines(&canvas, WritingMode::HorizontalTb);
        let blit = (0i32, -13i32);
        let (dirty, _) = ScrollPlanner::derive_dirty(
            &canvas,
            &window(1, -13.0),
            WritingMode::HorizontalTb,
            &contract,
            blit,
            surface,
            &prev,
        );
        // blit 写域（by<0＝内容上へ）: front 行 [|by|,H) を back 行 [0,H-|by|) へ写す。
        let by = blit.1.unsigned_abs();
        let mut spans = vec![(0u32, surface.1 - by)];
        for r in &dirty {
            spans.push((r.y, r.y + r.h)); // 横書きのブロック軸＝y
        }
        assert!(
            covers_block_axis_fully(spans, surface.1),
            "blit 写域 ∪ dirty が y 軸全域を覆う"
        );
    }

    /// [start,end) 区間列がブロック軸 `[0,dim)` を隙間なく覆うか（先頭 0・末尾 dim 到達）。
    fn covers_block_axis_fully(mut spans: Vec<(u32, u32)>, dim: u32) -> bool {
        spans.sort_unstable();
        let mut cursor = 0u32;
        for (s, e) in spans {
            if s > cursor {
                return false; // 隙間
            }
            if e > cursor {
                cursor = e;
            }
        }
        cursor >= dim
    }

    /// 行指紋は canvas-local（スクロール非依存）——同一 canvas は block_offset に依らず一致。
    #[test]
    fn committed_lines_are_scroll_independent() {
        let canvas = canvas_for(
            &broken_lines(3),
            WritingMode::HorizontalTb,
            (Some(0), Some(100), Some(0), Some(400)),
            10.0,
        );
        let a = ScrollPlanner::committed_lines(&canvas, WritingMode::HorizontalTb);
        let b = ScrollPlanner::committed_lines(&canvas, WritingMode::HorizontalTb);
        assert_eq!(a, b, "同一 canvas の指紋は決定論的に一致");
    }

    // ── 3.3 R2.3/4.3: plan/commit 二相（純粋計画・確定・Clear・失敗フレーム再試行） ──
    //
    // 幾何は 3.2 と同一前提（font 10 → pitch 13・全角 1 グリフ/行）。面寸 (400,100)。

    /// 3 行 canvas（横書き・validrect 100×400）——plan/commit 二相の共通母体。
    fn plan_canvas() -> ContentCanvas {
        canvas_for(
            &broken_lines(3),
            WritingMode::HorizontalTb,
            (Some(0), Some(100), Some(0), Some(400)),
            10.0,
        )
    }

    /// plan は状態不変・純粋——未 commit の反復 plan は同一計画を返し `scroll_state` も動かない
    /// （デバイス失敗フレームの再試行安全＝現行の「skip して次フレーム再計画」規律）。
    #[test]
    fn plan_is_pure_and_repeatable_without_commit() {
        let contract = ScaleContract::new(1.0, None);
        let canvas = plan_canvas();
        let mut planner = ScrollPlanner::new();
        // 初回フレームを確定して prev_lines を張る（以後のスクロールは Update）。
        let w0 = window(0, 0.0);
        let first = planner.plan(&canvas, &w0, WritingMode::HorizontalTb, &contract, (400, 100));
        planner.commit(&canvas, &w0, WritingMode::HorizontalTb, &contract, &first);

        // スクロールを未 commit で 2 回 plan → 同一計画・scroll_state 不変。
        let w1 = window(1, -13.0);
        let before = planner.scroll_state();
        let a = planner.plan(&canvas, &w1, WritingMode::HorizontalTb, &contract, (400, 100));
        let b = planner.plan(&canvas, &w1, WritingMode::HorizontalTb, &contract, (400, 100));
        assert_eq!(a, b, "未 commit の反復 plan は同一計画（再試行安全）");
        assert_eq!(
            planner.scroll_state(),
            before,
            "plan は self を一切変えない（純粋）"
        );
        assert!(matches!(a, FramePlan::Update { .. }), "スクロールは Update");
    }

    /// commit を挟むと確定が次回計画へ反映される——同一 window の初回全域 Update を commit
    /// すると、指紋一致・blit 0 で次の同一 plan は NoChange になる。
    #[test]
    fn commit_makes_next_identical_plan_no_change() {
        let contract = ScaleContract::new(1.0, None);
        let canvas = plan_canvas();
        let mut planner = ScrollPlanner::new();
        let w = window(0, 0.0);
        // 初回 plan は全域 Update（prev 空）。
        let first = planner.plan(&canvas, &w, WritingMode::HorizontalTb, &contract, (400, 100));
        assert!(matches!(first, FramePlan::Update { .. }), "初回は全域 Update");
        planner.commit(&canvas, &w, WritingMode::HorizontalTb, &contract, &first);
        // 同一 window の次 plan は NoChange（指紋一致・blit 0）。
        assert_eq!(
            planner.plan(&canvas, &w, WritingMode::HorizontalTb, &contract, (400, 100)),
            FramePlan::NoChange,
            "commit 後の同一 window は変化なし"
        );
    }

    /// スクロール（block_offset 変化）→ plan が露出帯付き Update → commit で
    /// `scroll_state().committed` が目標へ追従する（M1 ステップスクロールの即時追従）。
    #[test]
    fn commit_of_scroll_update_advances_committed() {
        let contract = ScaleContract::new(1.0, None);
        let canvas = plan_canvas();
        let mut planner = ScrollPlanner::new();
        let w0 = window(0, 0.0);
        let first = planner.plan(&canvas, &w0, WritingMode::HorizontalTb, &contract, (400, 100));
        planner.commit(&canvas, &w0, WritingMode::HorizontalTb, &contract, &first);
        assert_eq!(planner.scroll_state().committed, 0);

        // スクロール（内容不変・block_offset=-13＝内容が上へ）→ 下端露出帯付き Update。
        let w1 = window(1, -13.0);
        let scroll = planner.plan(&canvas, &w1, WritingMode::HorizontalTb, &contract, (400, 100));
        match &scroll {
            FramePlan::Update { blit, dirty, .. } => {
                assert_eq!(*blit, (0, -13), "横書きスクロールの blit は y 軸・符号素通し");
                assert!(!dirty.is_empty(), "露出帯がダーティ");
            }
            other => panic!("スクロールは Update のはず: {other:?}"),
        }
        planner.commit(&canvas, &w1, WritingMode::HorizontalTb, &contract, &scroll);
        assert_eq!(
            planner.scroll_state().committed,
            -13,
            "commit で committed が目標へ追従"
        );
    }

    /// plan は 3 種の計画結果を返し分ける——変化なし＝NoChange・request_clear 後＝FullClear・
    /// スクロール/伸長＝Update。
    #[test]
    fn plan_returns_three_variants() {
        let contract = ScaleContract::new(1.0, None);
        let canvas = plan_canvas();
        let mut planner = ScrollPlanner::new();
        let w = window(0, 0.0);

        // (Update) 初回＝全域。
        let first = planner.plan(&canvas, &w, WritingMode::HorizontalTb, &contract, (400, 100));
        assert!(matches!(first, FramePlan::Update { .. }), "初回は Update");
        planner.commit(&canvas, &w, WritingMode::HorizontalTb, &contract, &first);

        // (NoChange) 変化なし。
        assert_eq!(
            planner.plan(&canvas, &w, WritingMode::HorizontalTb, &contract, (400, 100)),
            FramePlan::NoChange,
            "変化なしは NoChange"
        );

        // (FullClear) request_clear 後。
        planner.request_clear();
        assert_eq!(
            planner.plan(&canvas, &w, WritingMode::HorizontalTb, &contract, (400, 100)),
            FramePlan::FullClear,
            "Clear 要求後は FullClear"
        );
    }

    /// FullClear の確定サイクル——request_clear で位置/指紋が 0 化し次 plan が FullClear、
    /// commit(FullClear) で clear_requested が落ちて次 plan は通常導出（全域 Update）へ戻る。
    /// 未 commit の失敗フレームはフラグ保持＝再試行安全。
    #[test]
    fn request_clear_then_commit_full_clear_returns_to_normal() {
        let contract = ScaleContract::new(1.0, None);
        let canvas = plan_canvas();
        let mut planner = ScrollPlanner::new();
        // 全域を確定 → スクロールで committed を -13 まで進める。
        let w0 = window(0, 0.0);
        let f0 = planner.plan(&canvas, &w0, WritingMode::HorizontalTb, &contract, (400, 100));
        planner.commit(&canvas, &w0, WritingMode::HorizontalTb, &contract, &f0);
        let w1 = window(1, -13.0);
        let f1 = planner.plan(&canvas, &w1, WritingMode::HorizontalTb, &contract, (400, 100));
        planner.commit(&canvas, &w1, WritingMode::HorizontalTb, &contract, &f1);
        assert_eq!(planner.scroll_state().committed, -13);

        // request_clear: scroll_state 0 化・次 plan は FullClear。
        planner.request_clear();
        assert_eq!(
            planner.scroll_state(),
            ScrollState {
                pos: 0.0,
                committed: 0
            },
            "位置/指紋が初期化される"
        );
        let clear_plan = planner.plan(&canvas, &w0, WritingMode::HorizontalTb, &contract, (400, 100));
        assert_eq!(clear_plan, FramePlan::FullClear);

        // 未 commit の反復は FullClear のまま（失敗フレーム再試行安全）。
        assert_eq!(
            planner.plan(&canvas, &w0, WritingMode::HorizontalTb, &contract, (400, 100)),
            FramePlan::FullClear,
            "commit するまで FullClear を保持"
        );

        // commit(FullClear) → フラグが落ち・prev 空ゆえ次 plan は通常導出（全域 Update）へ戻る。
        planner.commit(&canvas, &w0, WritingMode::HorizontalTb, &contract, &clear_plan);
        let after = planner.plan(&canvas, &w0, WritingMode::HorizontalTb, &contract, (400, 100));
        assert!(
            matches!(after, FramePlan::Update { .. }),
            "FullClear 確定後は通常導出へ戻る"
        );
    }
}
