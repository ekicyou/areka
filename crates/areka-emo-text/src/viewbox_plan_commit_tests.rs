use super::test_support::{broken_lines, canvas_for, commit_initial, expect_update, phys, window};
use super::{FramePlan, PhysicalRect, ScrollPlanner, ScrollState};
use crate::canvas::ContentCanvas;
use crate::region::ScaleContract;
use crate::writing::WritingMode;

// ── 3.3 R2.3/4.3: plan/commit 二相（純粋計画・確定・Clear・失敗フレーム再試行） ──
//
// 幾何は 3.2 と同一前提（font 10 → 行送り 12＝10 + 行間 2・全角 1 グリフ/行）。面寸 (400,100)。

/// 3 行 canvas（横書き・validrect 100×400）——plan/commit 二相の共通母体。
fn plan_canvas() -> ContentCanvas {
    canvas_for(
        &broken_lines(3),
        WritingMode::HorizontalTb,
        (Some(0), Some(100), Some(0), Some(400)),
        10.0,
    )
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

    // スクロール（内容不変・block_offset=-12＝内容が上へ）→ 下端露出帯付き Update。
    let w1 = window(1, -12.0);
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
                (0, -12),
                "横書きスクロールの blit は y 軸・符号素通し"
            );
            assert!(!dirty.is_empty(), "露出帯がダーティ");
        }
        other => panic!("スクロールは Update のはず: {other:?}"),
    }
    planner.commit(&canvas, &w1, WritingMode::HorizontalTb, &contract, &scroll);
    assert_eq!(
        planner.scroll_state().committed,
        -12,
        "commit で committed が目標へ追従"
    );
}

/// plan は 3 種の計画結果を返し分ける——変化なし＝NoChange・見える行の無い request_clear 後＝
/// FullClear・スクロール/伸長＝Update。
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

    // (FullClear) request_clear 後で見える行が無い。
    planner.request_clear();
    assert_eq!(
        planner.plan(
            &empty_canvas(),
            &w,
            WritingMode::HorizontalTb,
            &contract,
            (400, 100)
        ),
        FramePlan::FullClear,
        "Clear 要求後で見える行が無ければ FullClear"
    );
}

/// 行の無い canvas（消去の後で字がまだ来ていないフレーム）。
fn empty_canvas() -> ContentCanvas {
    canvas_for(
        &[],
        WritingMode::HorizontalTb,
        (Some(0), Some(100), Some(0), Some(400)),
        10.0,
    )
}

/// 消去と字が同じフレーム（指摘 17）——request_clear で位置/指紋が 0 化し、見える行のある次 plan は
/// 全域の Update（全面を消してから描く＝0 フレーム）。未 commit の反復は同じ計画（再試行安全）。
/// その commit で要求が下り、次 plan は通常の差分（同じ入力なら NoChange）へ戻る。
#[test]
fn request_clear_with_glyphs_plans_full_domain_update_and_commit_drops_request() {
    let contract = ScaleContract::new(1.0, None);
    let canvas = plan_canvas();
    let mut planner = ScrollPlanner::new();
    // 全域を確定 → スクロールで committed を -12 まで進める。
    let w0 = window(0, 0.0);
    let f0 = planner.plan(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    planner.commit(&canvas, &w0, WritingMode::HorizontalTb, &contract, &f0);
    let w1 = window(1, -12.0);
    let f1 = planner.plan(
        &canvas,
        &w1,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    planner.commit(&canvas, &w1, WritingMode::HorizontalTb, &contract, &f1);
    assert_eq!(planner.scroll_state().committed, -12);

    // request_clear: scroll_state 0 化・要求が立つ。
    planner.request_clear();
    assert_eq!(
        planner.scroll_state(),
        ScrollState {
            pos: 0.0,
            committed: 0
        },
        "位置/指紋が初期化される"
    );
    assert!(planner.clear_pending(), "request_clear で要求が立つ");
    let clear_plan = planner.plan(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    let expected = ScrollPlanner::full_domain_plan(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
        &[],
    );
    assert_eq!(
        clear_plan, expected,
        "消去と字が同じフレームは全域の Update（字をそのフレームで描く）"
    );
    let FramePlan::Update {
        blit, draw_lines, ..
    } = &clear_plan
    else {
        panic!("全域の Update のはず: {clear_plan:?}");
    };
    assert_eq!(*blit, (0, 0), "全域の道は写さない");
    assert_eq!(draw_lines, &vec![0, 1, 2], "見える 3 行を描く");

    // 未 commit の反復は同じ計画のまま（失敗フレーム再試行安全）。
    assert_eq!(
        planner.plan(
            &canvas,
            &w0,
            WritingMode::HorizontalTb,
            &contract,
            (400, 100)
        ),
        clear_plan,
        "commit するまで同じ計画を保持"
    );
    assert!(planner.clear_pending(), "未 commit の間は要求が残る");

    // commit(Update) → 要求が下り、同じ入力の次 plan は NoChange。
    planner.commit(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        &clear_plan,
    );
    assert!(
        !planner.clear_pending(),
        "全域の Update の commit で要求が下りる"
    );
    assert_eq!(
        planner.plan(
            &canvas,
            &w0,
            WritingMode::HorizontalTb,
            &contract,
            (400, 100)
        ),
        FramePlan::NoChange,
        "消去のフレームの確定後は通常の差分へ戻る"
    );
}

/// 見える行の無い消去の確定サイクル——request_clear 後の行の無い plan は FullClear、
/// commit(FullClear) で要求が下り、次に字が来た plan は通常導出（全行が新しい行の Update）へ戻る。
#[test]
fn request_clear_then_commit_full_clear_returns_to_normal() {
    let contract = ScaleContract::new(1.0, None);
    let canvas = plan_canvas();
    let empty = empty_canvas();
    let mut planner = ScrollPlanner::new();
    let w0 = window(0, 0.0);
    let f0 = planner.plan(
        &canvas,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    planner.commit(&canvas, &w0, WritingMode::HorizontalTb, &contract, &f0);

    planner.request_clear();
    let clear_plan = planner.plan(
        &empty,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        (400, 100),
    );
    assert_eq!(clear_plan, FramePlan::FullClear);
    assert!(
        planner.clear_pending(),
        "FullClear の commit 前は要求が残る"
    );

    planner.commit(
        &empty,
        &w0,
        WritingMode::HorizontalTb,
        &contract,
        &clear_plan,
    );
    assert!(!planner.clear_pending(), "commit(FullClear) で要求が下りる");
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
            -12.0f32,
            -12,
            (0, -12),
            Edge::Bottom,
        ),
        (WritingMode::VerticalRl, 12.0, 12, (12, 0), Edge::Left),
        (WritingMode::VerticalLr, -12.0, -12, (-12, 0), Edge::Right),
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
    // 行単位スクロールの刻み＝文字の大きさ + 行間 2。他の檻と同じ文字の大きさ 10
    // （刻み 12）にすると k=1.25 で真位置が 12 × 1.25 = 15 と常に整数になり、増分丸めが
    // 直接丸めと一致してしまって末尾の負の対照（増分はドリフトする）が空振りする。
    // 刻みが 4 の倍数でなければ真位置に端数が出るので、文字の大きさを 13（刻み 15・
    // 15 × 1.25 = 18.75）へ導き直す。刻みが再び 4 の倍数になれば末尾の対照が赤で知らせる。
    const FONT_H: f32 = 13.0;
    const PITCH: f32 = 15.0; // 13 + 行間 2。
    let contract = ScaleContract::new(k, None);
    let canvas = canvas_for(
        &broken_lines(4),
        mode,
        (Some(0), Some(200), Some(0), Some(400)),
        FONT_H,
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
        let plan = planner.plan(&canvas, &window(1, -12.0), mode, &contract, surf_h);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (0, -12));
        assert_back_fully_covered(blit, &dirty, surf_h, "横書きスクロール");
    }

    // (2) vertical_rl スクロール（bx>0＝内容が右へ・左端露出）。
    {
        let mode = WritingMode::VerticalRl;
        let canvas = canvas_for(&broken_lines(3), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &canvas, mode, &contract, surf_v);
        let plan = planner.plan(&canvas, &window(1, 12.0), mode, &contract, surf_v);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (12, 0));
        assert_back_fully_covered(blit, &dirty, surf_v, "vertical_rl スクロール");
    }

    // (3) vertical_lr スクロール（bx<0＝内容が左へ・右端露出）。
    {
        let mode = WritingMode::VerticalLr;
        let canvas = canvas_for(&broken_lines(3), mode, vr, 10.0);
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &canvas, mode, &contract, surf_v);
        let plan = planner.plan(&canvas, &window(1, -12.0), mode, &contract, surf_v);
        let (blit, dirty) = expect_update(&plan);
        assert_eq!(blit, (-12, 0));
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

    // (5) 初回全域（prev が分からない＝面全域 1 枚のダーティ）。
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
/// 挟むと同一スクロール window の再計画が NoChange へ変わる（確定が効いた差分）——初回
/// window の二相（`plan_returns_three_variants`）とは別に、スクロール window での二相を commit
/// 境界を跨いで確認する。
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

    let w = window(1, -12.0);
    let before = planner.scroll_state();
    let a = planner.plan(&canvas, &w, mode, &contract, surface);
    let b = planner.plan(&canvas, &w, mode, &contract, surface);
    assert_eq!(a, b, "未 commit の反復 plan は同一（決定論・再試行安全）");
    assert_eq!(planner.scroll_state(), before, "plan は状態不変（純粋）");
    assert!(matches!(a, FramePlan::Update { .. }), "スクロールは Update");

    planner.commit(&canvas, &w, mode, &contract, &a);
    assert_eq!(planner.scroll_state().committed, -12, "commit で確定が進む");
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

/// 古い側への送り（内容は同じ・可視窓だけを戻す）は全域ダーティ Update へ縮退する（実機 8.2）。
/// 前回の面は可視窓より前の行を描かず原点の上の余白を空けたままなので、blit するとその余白が
/// 新しく見える行の上へ写る。3 つの書字方向とも、前へ送った位置を確定した後に戻す計画は
/// blit=0・面全域 1 枚。
#[test]
fn scroll_back_toward_older_degrades_to_full_domain_across_modes() {
    let contract = ScaleContract::new(1.0, None);
    let surface = (48u32, 36u32);
    // (mode, 前へ送った block_offset)——axis_mapping_end_to_end と同じ向き。
    let cases = [
        (WritingMode::HorizontalTb, -12.0f32),
        (WritingMode::VerticalRl, 12.0),
        (WritingMode::VerticalLr, -12.0),
    ];
    for (mode, forward) in cases {
        let canvas = canvas_for(
            &broken_lines(3),
            mode,
            (Some(0), Some(200), Some(0), Some(400)),
            10.0,
        );
        let mut planner = ScrollPlanner::new();
        commit_initial(&mut planner, &canvas, mode, &contract, surface);
        let fw = window(1, forward);
        let ahead = planner.plan(&canvas, &fw, mode, &contract, surface);
        planner.commit(&canvas, &fw, mode, &contract, &ahead);

        let back = planner.plan(&canvas, &window(0, 0.0), mode, &contract, surface);
        let (blit, dirty) = expect_update(&back);
        assert_eq!(blit, (0, 0), "{mode:?}: 古い側への送りは blit しない");
        assert_eq!(
            dirty,
            vec![phys(0, 0, surface.0, surface.1)],
            "{mode:?}: ダーティは面全域 1 枚"
        );
    }
}

// ── 指摘 3: 飽和した確定位置でも plan → commit → plan があふれない ──

/// ±3e9 の位置（`i32` に収まらない＝確定位置が飽和する）で plan → commit → plan が panic せず、
/// どちらのフレームも面全域を描き直す（1 つ目は面の寸で頭打ちにした送りの露出帯、2 つ目は古い側への
/// 送りの全域の縮退）。
#[test]
fn saturated_position_plan_commit_plan_redraws_full_surface() {
    let contract = ScaleContract::new(1.0, None);
    let canvas = plan_canvas();
    let mode = WritingMode::HorizontalTb;
    let surface = (400u32, 100u32);
    let full = phys(0, 0, surface.0, surface.1);
    let mut planner = ScrollPlanner::new();
    commit_initial(&mut planner, &canvas, mode, &contract, surface);

    let far = window(0, -3e9);
    let first = planner.plan(&canvas, &far, mode, &contract, surface);
    let (blit, dirty) = expect_update(&first);
    assert_eq!(blit, (0, -100), "送りは面の高さで頭打ち・向きは保つ");
    assert!(
        dirty.contains(&full),
        "全部送り出された＝面全域を描き直す: {dirty:?}"
    );
    planner.commit(&canvas, &far, mode, &contract, &first);
    assert_eq!(planner.scroll_state().committed, i32::MIN, "確定位置は飽和");

    let back = window(0, 3e9);
    let second = planner.plan(&canvas, &back, mode, &contract, surface);
    let (blit, dirty) = expect_update(&second);
    assert_eq!(blit, (0, 0), "古い側への送りは全域の縮退");
    assert!(dirty.contains(&full), "面全域を描き直す: {dirty:?}");
}

// ── 指摘 16: 「前の行が分からない」と「何も描いていない」を分ける ──

/// 空の canvas を commit した次の plan は `NoChange`（空の列＝何も描いていないと分かっている）。
/// 分からない（初め・消去の要求の後）ときだけ面全域 1 枚を描き直し、`FullClear` の確定も
/// 「何も描いていない」と分かっている状態へ移る（設計の状態図）。
#[test]
fn committing_empty_canvas_makes_next_plan_no_change() {
    let contract = ScaleContract::new(1.0, None);
    let mode = WritingMode::HorizontalTb;
    let surface = (400u32, 100u32);
    let empty = canvas_for(
        &broken_lines(0),
        mode,
        (Some(0), Some(100), Some(0), Some(400)),
        10.0,
    );
    let w = window(0, 0.0);
    let mut planner = ScrollPlanner::new();

    let first = planner.plan(&empty, &w, mode, &contract, surface);
    assert_eq!(
        expect_update(&first),
        ((0, 0), vec![phys(0, 0, 400, 100)]),
        "分からない＝面全域 1 枚"
    );
    planner.commit(&empty, &w, mode, &contract, &first);
    assert_eq!(
        planner.plan(&empty, &w, mode, &contract, surface),
        FramePlan::NoChange,
        "空の canvas を確定した次は NoChange"
    );

    planner.request_clear();
    let clear = planner.plan(&empty, &w, mode, &contract, surface);
    assert_eq!(clear, FramePlan::FullClear);
    planner.commit(&empty, &w, mode, &contract, &clear);
    assert_eq!(
        planner.plan(&empty, &w, mode, &contract, surface),
        FramePlan::NoChange,
        "FullClear の確定の後も空なら NoChange"
    );
}

/// 再監査 R1-C2: 送りの前に上端が面の下端のすぐ内側（ε＝4）にあった最下行は、線を面の内側へ
/// 寄せて `[h − t, h]` に描いていた。1 行ぶん送ると、その線は `[h − t + by, h + by]` へ写り、
/// 線の太さ t が ε とガードより大きいと今の行の矩形より上へ出る。描き直しの範囲は、太さが行の
/// 丈（em 10）までの線の写り先を覆う（場面は font 10・pitch 12・面 400×100・9 行・by −12）。
#[test]
fn pulled_bottom_row_dirty_covers_where_its_stroke_was_shifted() {
    let contract = ScaleContract::new(1.0, None);
    let mode = WritingMode::HorizontalTb;
    let canvas = canvas_for(
        &broken_lines(9),
        mode,
        (Some(0), Some(100), Some(0), Some(400)),
        10.0,
    );
    let lines = ScrollPlanner::committed_lines(&canvas, mode);
    let (dirty, _) = ScrollPlanner::derive_dirty_with_overhangs(
        &canvas,
        &window(1, -12.0),
        mode,
        &contract,
        (0, -12),
        (400, 100),
        &lines,
        &lines,
        &[],
    );
    // 写り先は y 78..88（h + by − em .. h + by）・行内は字の幅 0..10。
    let uncovered: Vec<u32> = (78..88)
        .filter(|&y| {
            !dirty.iter().any(|d| {
                let r = &d.rect;
                r.y <= y && y < r.y + r.h && r.x == 0 && r.w >= 10
            })
        })
        .collect();
    assert!(
        uncovered.is_empty(),
        "寄せた線の写り先のうち描き直さない行がある: {uncovered:?}"
    );
}
