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

/// k≠1.0（1.25）でも真位置からの直接丸めで `|committed − pos| ≤ 0.5` を保つ
/// （小数アキュムレータの単発檻）。
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

// ── R7.5 系: 契約点の初期状態・blit 軸写像委譲 ──

/// 初期計画者の契約点は真位置 0・確定位置 0（commit 前）。
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

// ── 指摘 3: 飽和した確定位置からのずらしはあふれず ±面の寸に収まる ──

/// 確定位置が両端に飽和していても、ずらしは飽和の引き算で求まり（あふれの panic なし）、
/// 面の寸で頭打ちにした結果は ±面の寸に収まって向きを保つ。
#[test]
fn saturated_committed_blit_stays_within_surface() {
    let contract = ScaleContract::new(1.0, None);
    let surface = (40u32, 30u32);
    for (from, to) in [(3e9f32, -3e9f32), (-3e9, 3e9)] {
        let mut planner = ScrollPlanner::new();
        planner.committed = planner.resolve_position(from, &contract).committed;
        let target = planner.resolve_position(to, &contract);
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            let raw = planner.blit_vector(&target, mode);
            let (bx, by) = ScrollPlanner::clamp_blit_to_surface(raw, surface);
            assert!(bx.unsigned_abs() <= surface.0 && by.unsigned_abs() <= surface.1);
            assert_eq!((bx.signum(), by.signum()), (raw.0.signum(), raw.1.signum()));
            let block_dim = if mode == WritingMode::HorizontalTb {
                30
            } else {
                40
            };
            assert_eq!(
                bx.abs() + by.abs(),
                block_dim,
                "{mode:?}: 全部送り出された＝面の寸ちょうど"
            );
        }
    }
}
