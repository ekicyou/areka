use super::test_support::{broken_lines, canvas_for, commit_initial, expect_update, phys, window};
use super::{FramePlan, PhysicalRect, ScrollPlanner, ScrollState};
use crate::canvas::ContentCanvas;
use crate::region::ScaleContract;
use crate::state::TextItem;
use crate::writing::WritingMode;

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
    let first = planner.plan(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    planner.commit(&canvas, &w0, WritingMode::HorizontalTb, &contract, &first);

    // スクロールを未 commit で 2 回 plan → 同一計画・scroll_state 不変。
    let w1 = window(1, -13.0);
    let before = planner.scroll_state();
    let a = planner.plan(
        &canvas,
        &w1,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    let b = planner.plan(
        &canvas,
        &w1,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
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
    let first = planner.plan(
        &canvas,
        &w,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    assert!(
        matches!(first, FramePlan::Update { .. }),
        "初回は全域 Update"
    );
    planner.commit(&canvas, &w, WritingMode::HorizontalTb, &contract, &first);
    // 同一 window の次 plan は NoChange（指紋一致・blit 0）。
    assert_eq!(
        planner.plan(
            &canvas,
            &w,
            WritingMode::HorizontalTb,
            &contract,
            (400, 100)
        ),
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
    let first = planner.plan(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    planner.commit(&canvas, &w0, WritingMode::HorizontalTb, &contract, &first);
    assert_eq!(planner.scroll_state().committed, 0);

    // スクロール（内容不変・block_offset=-13＝内容が上へ）→ 下端露出帯付き Update。
    let w1 = window(1, -13.0);
    let scroll = planner.plan(
        &canvas,
        &w1,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    match &scroll {
        FramePlan::Update { blit, dirty, .. } => {
            assert_eq!(
                *blit,
                (0, -13),
                "横書きスクロールの blit は y 軸・符号素通し"
            );
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
    let first = planner.plan(
        &canvas,
        &w,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    assert!(matches!(first, FramePlan::Update { .. }), "初回は Update");
    planner.commit(&canvas, &w, WritingMode::HorizontalTb, &contract, &first);

    // (NoChange) 変化なし。
    assert_eq!(
        planner.plan(
            &canvas,
            &w,
            WritingMode::HorizontalTb,
            &contract,
            (400, 100)
        ),
        FramePlan::NoChange,
        "変化なしは NoChange"
    );

    // (FullClear) request_clear 後。
    planner.request_clear();
    assert_eq!(
        planner.plan(
            &canvas,
            &w,
            WritingMode::HorizontalTb,
            &contract,
            (400, 100)
        ),
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
    let f0 = planner.plan(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    planner.commit(&canvas, &w0, WritingMode::HorizontalTb, &contract, &f0);
    let w1 = window(1, -13.0);
    let f1 = planner.plan(
        &canvas,
        &w1,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
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
    let clear_plan = planner.plan(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    assert_eq!(clear_plan, FramePlan::FullClear);

    // 未 commit の反復は FullClear のまま（失敗フレーム再試行安全）。
    assert_eq!(
        planner.plan(
            &canvas,
            &w0,
            WritingMode::HorizontalTb,
            &contract,
            (400, 100)
        ),
        FramePlan::FullClear,
        "commit するまで FullClear を保持"
    );

    // commit(FullClear) → フラグが落ち・prev 空ゆえ次 plan は通常導出（全域 Update）へ戻る。
    planner.commit(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        &clear_plan,
    );
    let after = planner.plan(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    assert!(
        matches!(after, FramePlan::Update { .. }),
        "FullClear 確定後は通常導出へ戻る"
    );
}

// ── (a) 軸写像 3 方向の end-to-end 総括（resolve_position→blit_vector→露出帯の辺） ──

/// 3 方向で resolve_position の確定位置・blit_vector の軸/符号・plan が返す露出帯の辺
/// （横=下端・vertical_rl=左端・vertical_lr=右端）が正準表と 1:1 で一致する。3.1 の
/// 軸写像単体檻を「量子化→軸写像→ダーティ帯」の一本の経路として束ねて総括する。
#[test]
fn axis_mapping_end_to_end_across_three_writing_modes() {
    #[derive(Clone, Copy)]
    enum Edge {
        Bottom,
        Left,
        Right,
    }
    let contract = ScaleContract::new(1.0, None);
    let surface = (48u32, 36u32);
    // (mode, block_offset, 期待 committed, 期待 blit, 露出帯の辺)。
    let cases = [
        (
            WritingMode::HorizontalTb,
            -13.0f32,
            -13,
            (0, -13),
            Edge::Bottom,
        ),
        (WritingMode::VerticalRl, 13.0, 13, (13, 0), Edge::Left),
        (WritingMode::VerticalLr, -13.0, -13, (-13, 0), Edge::Right),
    ];
    for (mode, offset, exp_committed, exp_blit, edge) in cases {
        // 量子化＋軸写像（状態不変の純関数経路）。
        let planner = ScrollPlanner::new();
        let target = planner.resolve_position(offset, &contract);
        assert_eq!(target.committed, exp_committed, "{mode:?}: 確定位置");
        assert_eq!(
            planner.blit_vector(&target, mode),
            exp_blit,
            "{mode:?}: blit の軸/符号は block_offset 素通し"
        );

        // end-to-end: 可視窓のみ移動（content 不変）→ plan は blit＋露出帯 1 枚を返す。
        let canvas = canvas_for(
            &broken_lines(3),
            mode,
            (Some(0), Some(200), Some(0), Some(400)),
            10.0,
        );
        let mut driven = ScrollPlanner::new();
        commit_initial(&mut driven, &canvas, mode, &contract, surface);
        let plan = driven.plan(&canvas, &window(1, offset), mode, &contract, surface);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, exp_blit, "{mode:?}: plan の blit も同一");
        assert_eq!(
            dirty.len(),
            1,
            "{mode:?}: 可視窓のみ移動は露出帯 1 枚（変化行ゼロ）"
        );
        let band = dirty[0];
        let (w, h) = surface;
        let on_expected_edge = match edge {
            // 横書き: 下端いっぱい（内容が上へ流れ下端が露出）・全幅。
            Edge::Bottom => band.y + band.h == h && band.y > 0 && band.x == 0 && band.w == w,
            // vertical_rl: 左端（内容が右へ流れ左端が露出）・全高。
            Edge::Left => band.x == 0 && band.x + band.w < w && band.y == 0 && band.h == h,
            // vertical_lr: 右端（内容が左へ流れ右端が露出）・全高。
            Edge::Right => band.x + band.w == w && band.x > 0 && band.y == 0 && band.h == h,
        };
        assert!(
            on_expected_edge,
            "{mode:?}: 露出帯が期待辺にある（band={band:?}）"
        );
    }
}

// ── (b) 長スクロール列のドリフトなし（主眼）: plan→commit を数百回回す ──

/// `plan`/`commit` を数百ステップ回すステートフルな長スクロール列で、確定位置が真位置
/// からの**直接丸め**（`committed == round(block_offset × k)`）に保たれ、`|committed − pos|`
/// が常に ≤ 0.5 で累積ドリフトしないことを実証する（単発 resolve でなく commit で committed を
/// 更新する列を回す）。`k=1.0` は `committed == pos`。増分丸めの累積シミュレータを対照として
/// 併走させ、本メトリクスが「直接丸め」と「増分丸め（ドリフトする）」を弁別できることを裏取る。
fn assert_long_scroll_is_drift_free(mode: WritingMode, k: f32, step_sign: f32, steps: usize) {
    const PITCH: f32 = 13.0; // font 10 → ceil(12.5)（行単位スクロールの刻み）。
    let contract = ScaleContract::new(k, None);
    let canvas = canvas_for(
        &broken_lines(4),
        mode,
        (Some(0), Some(200), Some(0), Some(400)),
        10.0,
    );
    let surface = (400u32, 400u32); // ブロック軸に十分・committed 追従は面寸非依存。
    let mut planner = ScrollPlanner::new();
    commit_initial(&mut planner, &canvas, mode, &contract, surface);

    // 対照: 増分（前ステップ真位置との差）を毎回丸めて足し込む素朴実装＝累積ドリフトの見本。
    let mut incremental: i64 = 0;
    let mut prev_pos = 0.0f32;
    let mut last_pos = 0.0f32;

    for n in 1..=steps {
        let block_offset = step_sign * n as f32 * PITCH;
        let w = window(0, block_offset);
        let plan = planner.plan(&canvas, &w, mode, &contract, surface);
        assert!(
            matches!(plan, FramePlan::Update { .. }),
            "step {n}: スクロール（blit≠0）は Update"
        );
        planner.commit(&canvas, &w, mode, &contract, &plan);

        let state = planner.scroll_state();
        let pos = block_offset * k; // resolve_position と同一の f32 演算＝厳密一致。
        assert_eq!(state.pos, pos, "step {n}: 真位置 = block_offset × k");
        assert_eq!(
            state.committed,
            pos.round() as i32,
            "step {n}: 確定位置 = round(真位置)（増分でなく直接丸め）"
        );
        assert!(
            (state.committed as f32 - state.pos).abs() <= 0.5,
            "step {n}: |committed − pos| ≤ 0.5（累積ドリフトなし）"
        );
        if k == 1.0 {
            assert_eq!(
                state.committed as f32, state.pos,
                "step {n}: k=1.0 では committed == pos（byte 一致の構造前提）"
            );
        }

        incremental += (pos - prev_pos).round() as i64;
        prev_pos = pos;
        last_pos = pos;
    }

    // 直接丸めの最終確定は真位置と ≤ 0.5（数百ステップ後も誤差が積み上がらない）。
    let final_committed = planner.scroll_state().committed as f32;
    assert!(
        (final_committed - last_pos).abs() <= 0.5,
        "直接丸めは最終真位置と ≤ 0.5（committed={final_committed} pos={last_pos}）"
    );
    // 非整数スケールでは増分丸め対照が 0.5px を大きく超えて漂う＝本檻の識別力の裏取り。
    if k != 1.0 {
        assert!(
            (incremental as f32 - last_pos).abs() > 1.0,
            "増分丸めは累積ドリフトする（対照 {incremental} vs 真位置 {last_pos}）——\
             本檻はそのドリフトを許さないことを検証している"
        );
    }
}

/// 横書き・非整数スケール（k=1.25）の長スクロール列でドリフトが起きない（主眼）。
#[test]
fn long_scroll_horizontal_nonunit_scale_is_drift_free() {
    assert_long_scroll_is_drift_free(WritingMode::HorizontalTb, 1.25, -1.0, 400);
}

/// 横書き・k=1.0 の長スクロール列では全ステップで committed == pos（byte 一致前提）。
#[test]
fn long_scroll_horizontal_unit_scale_keeps_committed_equal_to_pos() {
    assert_long_scroll_is_drift_free(WritingMode::HorizontalTb, 1.0, -1.0, 400);
}

/// 縦書き（vertical_rl・正方向）・k=1.25 でも軸違いでドリフトなしが成立する。
#[test]
fn long_scroll_vertical_rl_nonunit_scale_is_drift_free() {
    assert_long_scroll_is_drift_free(WritingMode::VerticalRl, 1.25, 1.0, 400);
}

// ── (c) ダーティ導出 5 ケースの一式化（plan/commit 二相の分類を束ねて総括） ──

/// plan/commit 二相の分類が 5 ケースを一式で満たす: 可視窓のみ移動＝露出帯のみ／
/// typewriter 1 グリフ＝現在行のみ／catch-up 複数行＝変化行の和／Clear＝FullClear／
/// 変化なし＝NoChange。3.2/3.3 の derive_dirty 単体檻とは別に、plan の variant 選択
/// （commit で張った prev_lines を根拠にした end-to-end 分類）を束ねて確認する。
#[test]
fn plan_dirty_derivation_suite_covers_five_cases() {
    let contract = ScaleContract::new(1.0, None);
    let mode = WritingMode::HorizontalTb;
    let vr = (Some(0), Some(100), Some(0), Some(400));
    let surface = (400u32, 100u32);
    let glyph = |ch| TextItem::Glyph { ch };

    // (1) 可視窓のみ移動（content 不変）→ dirty＝露出帯のみ（変化行ゼロ・下端の帯）。
    {
        let canvas = canvas_for(&broken_lines(4), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &canvas, mode, &contract, surface);
        let plan = planner.plan(&canvas, &window(1, -13.0), mode, &contract, surface);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (0, -13), "(1) スクロールの blit は y 軸・符号素通し");
        assert_eq!(dirty.len(), 1, "(1) 露出帯 1 枚のみ（変化行ゼロ）");
        assert_eq!(
            dirty[0].y + dirty[0].h,
            surface.1,
            "(1) 露出帯は下端に接する"
        );
    }

    // (2) typewriter 1 グリフ進行（現在行が伸長）→ dirty＝現在行のみ・draw＝[0]。
    {
        let prev_canvas = canvas_for(&[glyph('あ')], mode, vr, 10.0);
        let curr_canvas = canvas_for(&[glyph('あ'), glyph('あ')], mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &prev_canvas, mode, &contract, surface);
        let plan = planner.plan(&curr_canvas, &window(0, 0.0), mode, &contract, surface);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (0, 0), "(2) 非スクロール＝blit 0");
        assert_eq!(dirty.len(), 1, "(2) 現在行のみがダーティ");
        if let FramePlan::Update { draw_lines, .. } = &plan {
            assert_eq!(draw_lines, &vec![0], "(2) 描画対象は現在行のみ");
        }
    }

    // (3) catch-up 複数行（2→4 行へ一挙）→ dirty＝新規 2 行の和・draw＝[2,3]。
    {
        let prev_canvas = canvas_for(&broken_lines(2), mode, vr, 10.0);
        let curr_canvas = canvas_for(&broken_lines(4), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &prev_canvas, mode, &contract, surface);
        let plan = planner.plan(&curr_canvas, &window(0, 0.0), mode, &contract, surface);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (0, 0), "(3) 非スクロール＝blit 0");
        assert_eq!(dirty.len(), 2, "(3) 変化行 2 行の和");
        if let FramePlan::Update { draw_lines, .. } = &plan {
            assert_eq!(draw_lines, &vec![2, 3], "(3) 新規 2 行のみ描画対象");
        }
    }

    // (4) Clear cue → FullClear（描画 0 件の全域リセット）。
    {
        let canvas = canvas_for(&broken_lines(3), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &canvas, mode, &contract, surface);
        planner.request_clear();
        assert_eq!(
            planner.plan(&canvas, &window(0, 0.0), mode, &contract, surface),
            FramePlan::FullClear,
            "(4) Clear 要求後は FullClear"
        );
    }

    // (5) 変化なし（同一 canvas・同一 window）→ NoChange（blit 0・dirty 空）。
    {
        let canvas = canvas_for(&broken_lines(3), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &canvas, mode, &contract, surface);
        assert_eq!(
            planner.plan(&canvas, &window(0, 0.0), mode, &contract, surface),
            FramePlan::NoChange,
            "(5) 変化なしは NoChange"
        );
    }
}

// ── (d) back 全被覆（3 方向・複数シナリオ・2D ピクセル格子で未被覆 0 を総当り） ──

/// 面全域の各物理ピクセルが「blit の写域」または「いずれかの dirty 矩形」で被覆される
/// ことを 2D 格子で総当り確認する。blit 写域は front を blit ぶんずらした自己重なり
/// `[max(0,bx),min(w,w+bx)) × [max(0,by),min(h,h+by))`（production の exposure_band に
/// 依存せず独立導出）・blit=0 は全面。露出帯が dirty に含まれることで union＝全域になる。
fn assert_back_fully_covered(
    blit: (i32, i32),
    dirty: &[PhysicalRect],
    surface: (u32, u32),
    label: &str,
) {
    let (w, h) = (surface.0 as i32, surface.1 as i32);
    let (bx, by) = blit;
    // 保持ピクセルの写域（blit=0 は全面コピー＝面全域）。
    let rx0 = bx.max(0);
    let rx1 = (w + bx).min(w);
    let ry0 = by.max(0);
    let ry1 = (h + by).min(h);
    for y in 0..h {
        for x in 0..w {
            let in_retained = x >= rx0 && x < rx1 && y >= ry0 && y < ry1;
            let in_dirty = dirty.iter().any(|r| {
                let (dx0, dy0) = (r.x as i32, r.y as i32);
                let (dx1, dy1) = ((r.x + r.w) as i32, (r.y + r.h) as i32);
                x >= dx0 && x < dx1 && y >= dy0 && y < dy1
            });
            assert!(
                in_retained || in_dirty,
                "{label}: 物理ピクセル ({x},{y}) が blit 写域にも dirty にも属さない（残像漏れ）"
            );
        }
    }
}

/// 3 方向のスクロール（露出帯付き）・content 伸長（blit=0）・初回全域のいずれでも、
/// blit 写域 ∪ dirty が面全域を隙間なく被覆する（残像が 2 フレーム越しに漏れる経路が
/// 構造的にないことの総当り檻）。
#[test]
fn back_is_fully_covered_across_modes_and_scenarios() {
    let contract = ScaleContract::new(1.0, None);
    let vr = (Some(0), Some(200), Some(0), Some(400));
    let surf_h = (48u32, 36u32); // 横書き（ブロック軸＝y）。
    let surf_v = (36u32, 48u32); // 縦書き（ブロック軸＝x）。

    // (1) 横書きスクロール（by<0＝内容が上へ・下端露出）。
    {
        let mode = WritingMode::HorizontalTb;
        let canvas = canvas_for(&broken_lines(3), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &canvas, mode, &contract, surf_h);
        let plan = planner.plan(&canvas, &window(1, -13.0), mode, &contract, surf_h);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (0, -13));
        assert_back_fully_covered(blit, &dirty, surf_h, "横書きスクロール");
    }

    // (2) vertical_rl スクロール（bx>0＝内容が右へ・左端露出）。
    {
        let mode = WritingMode::VerticalRl;
        let canvas = canvas_for(&broken_lines(3), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &canvas, mode, &contract, surf_v);
        let plan = planner.plan(&canvas, &window(1, 13.0), mode, &contract, surf_v);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (13, 0));
        assert_back_fully_covered(blit, &dirty, surf_v, "vertical_rl スクロール");
    }

    // (3) vertical_lr スクロール（bx<0＝内容が左へ・右端露出）。
    {
        let mode = WritingMode::VerticalLr;
        let canvas = canvas_for(&broken_lines(3), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &canvas, mode, &contract, surf_v);
        let plan = planner.plan(&canvas, &window(1, -13.0), mode, &contract, surf_v);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (-13, 0));
        assert_back_fully_covered(blit, &dirty, surf_v, "vertical_lr スクロール");
    }

    // (4) content 伸長で blit=0（全面コピー＝写域が面全域）。
    {
        let mode = WritingMode::HorizontalTb;
        let prev_canvas = canvas_for(&broken_lines(2), mode, vr, 10.0);
        let grown = canvas_for(&broken_lines(4), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &prev_canvas, mode, &contract, surf_h);
        let plan = planner.plan(&grown, &window(0, 0.0), mode, &contract, surf_h);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (0, 0), "伸長のみ＝スクロールなし＝blit 0");
        assert_back_fully_covered(blit, &dirty, surf_h, "content 伸長 blit=0");
    }

    // (5) 初回全域（prev 空＝面全域 1 枚のダーティ）。
    {
        let mode = WritingMode::HorizontalTb;
        let canvas = canvas_for(&broken_lines(3), mode, vr, 10.0);
        let planner = ScrollPlanner::new();
        let plan = planner.plan(&canvas, &window(0, 0.0), mode, &contract, surf_h);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (0, 0), "初回は blit 0");
        assert_back_fully_covered(blit, &dirty, surf_h, "初回全域");
    }
}

// ── (e) plan/commit 二相の反復同一性（commit 境界を跨ぐ補強） ──

/// 未 commit の反復 plan は同一計画を返し scroll_state を動かさない（再試行安全）。commit を
/// 挟むと同一スクロール window の再計画が NoChange へ変わる（確定が効いた差分）——3.3 の初回
/// window 檻とは別に、スクロール window での二相を commit 境界を跨いで確認する。
#[test]
fn plan_commit_two_phase_idempotent_across_commit_boundary() {
    let contract = ScaleContract::new(1.0, None);
    let mode = WritingMode::HorizontalTb;
    let canvas = canvas_for(
        &broken_lines(4),
        mode,
        (Some(0), Some(100), Some(0), Some(400)),
        10.0,
    );
    let surface = (400u32, 100u32);
    let mut planner = ScrollPlanner::new();
    commit_initial(&mut planner, &canvas, mode, &contract, surface);

    let w = window(1, -13.0);
    let before = planner.scroll_state();
    let a = planner.plan(&canvas, &w, mode, &contract, surface);
    let b = planner.plan(&canvas, &w, mode, &contract, surface);
    assert_eq!(a, b, "未 commit の反復 plan は同一（決定論・再試行安全）");
    assert_eq!(planner.scroll_state(), before, "plan は状態不変（純粋）");
    assert!(matches!(a, FramePlan::Update { .. }), "スクロールは Update");

    planner.commit(&canvas, &w, mode, &contract, &a);
    assert_eq!(planner.scroll_state().committed, -13, "commit で確定が進む");
    assert_eq!(
        planner.plan(&canvas, &w, mode, &contract, surface),
        FramePlan::NoChange,
        "commit 後の同一 window は NoChange（確定が効いた差分）"
    );
}

/// 後方（un-reveal/un-scroll）縮退（D1 修正）: 内容が前回確定より減った（住人数が減った）とき、
/// plan は差分（露出帯 ∪ 変化行）でなく**全域ダーティ Update**（blit=0・面全域・全住人）を返す。
/// スクロールアウトした確定行の再露出は面内 blit で保持できないため（保持していない）、正しさ優先で
/// 全域再描画へ縮退する（注入時刻の後方ジャンプ等の異常アクセスに対する堅牢化・byte 等価維持）。
/// 前方 typewriter では住人は単調増加ゆえ通常不発。
#[test]
fn plan_shrunk_content_degrades_to_full_domain() {
    let contract = ScaleContract::new(1.0, None);
    let mode = WritingMode::HorizontalTb;
    let vr = (Some(0), Some(100), Some(0), Some(400));
    let surface = (400u32, 100u32);
    let mut planner = ScrollPlanner::new();

    // 前方: 3 行を確定（prev_lines＝3 住人）。
    let canvas3 = canvas_for(&broken_lines(3), mode, vr, 10.0);
    let w3 = window(0, 0.0);
    let plan3 = planner.plan(&canvas3, &w3, mode, &contract, surface);
    planner.commit(&canvas3, &w3, mode, &contract, &plan3);

    // 後方: 内容が 1 行へ減った（住人 1 < prev 3）→ 全域ダーティ縮退。
    let canvas1 = canvas_for(&broken_lines(1), mode, vr, 10.0);
    let w1 = window(0, 0.0);
    match planner.plan(&canvas1, &w1, mode, &contract, surface) {
        FramePlan::Update {
            blit,
            dirty,
            draw_lines,
        } => {
            assert_eq!(blit, (0, 0), "後方縮退は blit=0（全域再描画・保持しない）");
            assert_eq!(dirty, vec![phys(0, 0, 400, 100)], "ダーティは面全域 1 枚");
            assert_eq!(draw_lines, vec![0], "描画対象は全 GlyphRun 住人（1 行）");
        }
        other => panic!("後方縮退は全域 Update を期待したが {other:?}"),
    }
}
