//! # viewbox — スクロール位置の内部表現・軸写像・量子化（純粋層）
//!
//! 可視窓（[`crate::layout::VisibleWindow`]）の `block_offset` を「真位置（f32 連続量・
//! 物理 px）」と「確定位置（whole-pixel 整数）」へ分離して保持し（M2 補間シームの土台・
//! R8.2）、ブロック軸のスカラを writing_mode 追随の 2D ベクトルへ写す [`ScrollState`]／
//! [`ScrollPlanner`]／[`block_axis_vector`] を担う。状態遷移（plan/commit）・ダーティ導出・
//! `FramePlan` は後続タスクが本モジュールへ追加する（本タスクは表現と軸写像・量子化のみ）。
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
/// 本タスク（3.1）は真位置／確定位置の**表現**・軸写像・量子化のみを担う。状態遷移
/// （`plan`/`commit`）・ダーティ導出・`FramePlan` は後続タスク（3.2/3.3）が本型へ追加する
/// ——それらを受け入れられるよう確定位置 `committed` と直近真位置 `pos` を内部状態として
/// 保持する（初期 0）。
///
/// 純粋層規律: `windows` 非依存（lib.rs 構造檻へ追加）。同一入力→同一出力。
#[derive(Clone, Debug, Default)]
pub struct ScrollPlanner {
    /// 面に反映済みの whole-pixel 位置（`commit` で更新——後続 3.2 の領分・初期 0）。
    committed: i32,
    /// 直近の真位置（f32 連続量・M2 で補間過程が更新元になる・初期 0）。
    pos: f32,
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
}

#[cfg(test)]
mod tests {
    use super::{ScrollPlanner, ScrollState, block_axis_vector};
    use crate::region::ScaleContract;
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
}
