use super::test_support::{broken_lines, canvas_for, commit_initial, expect_update, phys, window};
use super::{DIRTY_GUARD_IMG_PX, FramePlan, LineOverhang, ScrollPlanner};
use crate::region::ScaleContract;
use crate::state::TextItem;
use crate::writing::WritingMode;

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
    // 内容を動かさず block_offset だけ −12（内容が上へ・行送り 10 + 2）・blit も同量。
    let (dirty, _draw) = ScrollPlanner::derive_dirty(
        &canvas,
        &window(1, -12.0),
        WritingMode::HorizontalTb,
        &contract,
        (0, -12),
        (400, 100),
        &prev,
    );
    // 下端の露出帯 {0,88,400,12} をガード 1px で拡張＋クランプ → {0,87,400,13}。
    assert_eq!(dirty, vec![phys(0, 87, 400, 13)], "露出帯のみ・変化行ゼロ");
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
    // vertical_rl＝内容が右（正）・blit も +12（左端が露出）。
    let (dirty, _draw) = ScrollPlanner::derive_dirty(
        &canvas,
        &window(1, 12.0),
        WritingMode::VerticalRl,
        &contract,
        (12, 0),
        (100, 200),
        &prev,
    );
    // 左端の露出帯 {0,0,12,200} をガード拡張＋クランプ → {0,0,13,200}。
    assert_eq!(dirty, vec![phys(0, 0, 13, 200)], "左端露出帯のみ");
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
    // vertical_lr＝内容が左（負）・blit も −12（右端が露出）。
    let (dirty, _draw) = ScrollPlanner::derive_dirty(
        &canvas,
        &window(1, -12.0),
        WritingMode::VerticalLr,
        &contract,
        (-12, 0),
        (100, 200),
        &prev,
    );
    // 右端の露出帯 {88,0,12,200} をガード拡張＋クランプ → {87,0,13,200}。
    assert_eq!(dirty, vec![phys(87, 0, 13, 200)], "右端露出帯のみ");
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
    // 現在行 {0,0,20,10} をガード拡張＋クランプ → {0,0,21,11}（overhang 無し＝em ボックス丈・
    // 実測はみ出しは COM 層 render が付与。本 pure 層檻は既定 0 の幾何を檻化）。露出帯なし（blit=0）。
    assert_eq!(dirty, vec![phys(0, 0, 21, 11)], "現在行のみ");
    assert_eq!(draw, vec![0], "描画対象は現在行のみ");
}

// ── DD-9: 行内縮小の後方縮退（遅延化で到達可能・`plan` の guard 経路）──

/// DD-9: 同一 index 行の extent が縮む（行内縮小・改行なし）と `plan` は全域ダーティ
/// （blit 0・面全域・全 GlyphRun 住人）へ縮退する——差分描画では退避インク（縮小の外側）を
/// 取りこぼすため（後方時刻ジャンプ・un-reveal で確定行が縮む任意アクセスへの防御）。
#[test]
fn within_line_shrink_falls_back_to_full_dirty() {
    let contract = ScaleContract::new(1.0, None);
    let mode = WritingMode::HorizontalTb;
    let vr = (Some(0), Some(100), Some(0), Some(400));
    let surface = (400u32, 100u32);
    // 前回確定＝広い行（全角 4「ああああ」・幅 40）。
    let wide = canvas_for(
        &[
            TextItem::Glyph { ch: 'あ' },
            TextItem::Glyph { ch: 'あ' },
            TextItem::Glyph { ch: 'あ' },
            TextItem::Glyph { ch: 'あ' },
        ],
        mode,
        vr,
        10.0,
    );
    let mut planner = ScrollPlanner::new();
    commit_initial(&mut planner, &wide, mode, &contract, surface);
    // 新 canvas＝同 index 行が縮む（全角 2「ああ」・幅 20・block 不動）。
    let narrow = canvas_for(
        &[TextItem::Glyph { ch: 'あ' }, TextItem::Glyph { ch: 'あ' }],
        mode,
        vr,
        10.0,
    );
    let plan = planner.plan(&narrow, &window(0, 0.0), mode, &contract, surface);
    match plan {
        FramePlan::Update {
            blit,
            dirty,
            draw_lines,
        } => {
            assert_eq!(blit, (0, 0), "縮退は blit 0");
            assert_eq!(
                dirty,
                vec![phys(0, 0, 400, 100)],
                "面全域 1 枚（退避インク一掃）"
            );
            assert_eq!(draw_lines, vec![0], "全 GlyphRun 住人を再描画");
        }
        other => panic!("行内縮小は全域ダーティ Update を期待: {other:?}"),
    }
}

/// DD-9: 前方伸長（extent 増加・block 不動）は縮退せず増分（変化行のみ dirty）を維持する
/// ——typewriter ホットパスが guard の過剰発火で全域再描画へ落ちないことの固定。
#[test]
fn forward_line_growth_stays_incremental() {
    let contract = ScaleContract::new(1.0, None);
    let mode = WritingMode::HorizontalTb;
    let vr = (Some(0), Some(100), Some(0), Some(400));
    let surface = (400u32, 100u32);
    let narrow = canvas_for(
        &[TextItem::Glyph { ch: 'あ' }, TextItem::Glyph { ch: 'あ' }],
        mode,
        vr,
        10.0,
    );
    let mut planner = ScrollPlanner::new();
    commit_initial(&mut planner, &narrow, mode, &contract, surface);
    // 伸長（全角 2→4・同一行・block 不動）。
    let grown = canvas_for(
        &[
            TextItem::Glyph { ch: 'あ' },
            TextItem::Glyph { ch: 'あ' },
            TextItem::Glyph { ch: 'あ' },
            TextItem::Glyph { ch: 'あ' },
        ],
        mode,
        vr,
        10.0,
    );
    let (blit, dirty) =
        expect_update(&planner.plan(&grown, &window(0, 0.0), mode, &contract, surface));
    assert_eq!(blit, (0, 0));
    assert_ne!(
        dirty,
        vec![phys(0, 0, 400, 100)],
        "全域縮退しない（増分を維持）"
    );
    assert_eq!(dirty.len(), 1, "変化行 0 の矩形のみ（露出帯なし・blit 0）");
}

/// D2 の核: 実測インクはみ出し（[`LineOverhang`]）を渡すと変化行のダーティが em ボックスから
/// はみ出し分だけ外側へ広がる（横書き＝top/bottom で Y・縦書き＝left/right で X）。overhang 無し
/// （既定 0）は em ボックス丈——`GetOverhangMetrics` 実測を渡す COM 層経路（`ViewboxExecutor`）が
/// em 下端はみ出しインクを取りこぼさない機構を pure 層で檻化する（実 fixture 被覆は viewbox_draw の
/// `yugothic_real_fixture_matches_oracle_byte_for_byte`）。
#[test]
fn overhang_extends_changed_line_dirty_beyond_em_box() {
    let contract = ScaleContract::new(1.0, None);
    let vr = (Some(0), Some(224), Some(0), Some(400));
    // 現在行が 1→2 グリフへ伸長（横書き・1 行）。
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

    // overhang 無し → em ボックス丈 {0,0,21,11}（typewriter_single_glyph と同一の基準）。
    let (dirty_em, _) = ScrollPlanner::derive_dirty(
        &canvas,
        &window(0, 0.0),
        WritingMode::HorizontalTb,
        &contract,
        (0, 0),
        (400, 224),
        &prev,
    );
    assert_eq!(
        dirty_em,
        vec![phys(0, 0, 21, 11)],
        "overhang 無し＝em ボックス丈"
    );

    // 実測 overhang top=1・bottom=3（横書き＝Y 方向）。em 行 {0,0,20,10} を上へ 1・下へ 3 →
    // {0,-1,20,13}・ガード 1 → {-1,-2,21,14}・クランプ → {0,0,21,14}。
    let over = vec![LineOverhang {
        top: 1.0,
        bottom: 3.0,
        left: 0.0,
        right: 0.0,
    }];
    let (dirty_over, draw_over) = ScrollPlanner::derive_dirty_with_overhangs(
        &canvas,
        &window(0, 0.0),
        WritingMode::HorizontalTb,
        &contract,
        (0, 0),
        (400, 224),
        &prev,
        &over,
    );
    assert_eq!(
        dirty_over,
        vec![phys(0, 0, 21, 14)],
        "実測はみ出し分だけ Y 方向へ拡張（下 3・上 1）——em 丈 11 より高い"
    );
    assert_eq!(
        draw_over,
        vec![0],
        "描画対象は現在行のみ（overhang は幾何を変えるだけ）"
    );

    // 縦書き（vertical_rl）: overhang は right（X 正方向）へ効く——横書きの top/bottom（Y）は
    // 無視され、left/right（X）で列のブロック軸が広がる（軸読み替えの檻）。
    let vprev = canvas_for(
        &[TextItem::Glyph { ch: 'あ' }],
        WritingMode::VerticalRl,
        vr,
        10.0,
    );
    let vprev_lines = ScrollPlanner::committed_lines(&vprev, WritingMode::VerticalRl);
    let vcanvas = canvas_for(
        &[TextItem::Glyph { ch: 'あ' }, TextItem::Glyph { ch: 'あ' }],
        WritingMode::VerticalRl,
        vr,
        10.0,
    );
    let (v_em, _) = ScrollPlanner::derive_dirty(
        &vcanvas,
        &window(0, 0.0),
        WritingMode::VerticalRl,
        &contract,
        (0, 0),
        (400, 224),
        &vprev_lines,
    );
    // 縦書きで X 方向へ right=4 拡張すると em 丈より横幅が広がる（top/bottom は Y ゆえ無視される）。
    let vover = vec![LineOverhang {
        top: 9.0, // Y 方向（縦書きの行内軸）は無視される検証用のダミー大値。
        bottom: 9.0,
        left: 0.0,
        right: 4.0,
    }];
    let (v_over, _) = ScrollPlanner::derive_dirty_with_overhangs(
        &vcanvas,
        &window(0, 0.0),
        WritingMode::VerticalRl,
        &contract,
        (0, 0),
        (400, 224),
        &vprev_lines,
        &vover,
    );
    assert_eq!(v_em.len(), 1);
    assert_eq!(v_over.len(), 1);
    assert_eq!(
        v_over[0].h, v_em[0].h,
        "縦書きは top/bottom（Y）を無視——高さ（行内軸）は overhang で変わらない"
    );
    assert_eq!(
        v_over[0].w,
        v_em[0].w + 4,
        "縦書きは right（X）でブロック軸（列幅）が実測はみ出し分だけ広がる"
    );
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
    // 行1「いろ」{0,12,20,10} をガード拡張 → {0,11,21,12}（overhang 無し＝em 箱丈）。行0 は現れない。
    assert_eq!(dirty, vec![phys(0, 11, 21, 12)], "変化行（末尾）のみ");
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
    // 行2 {0,24,10,10}→{0,23,11,12}・行3 {0,36,10,10}→{0,35,11,12}（overhang 無し）。行0/1 は不変。
    assert_eq!(dirty, vec![phys(0, 23, 11, 12), phys(0, 35, 11, 12)]);
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
    assert_eq!(
        dirty,
        vec![phys(0, 0, 27, 15)],
        "y 負側はガード後 0 へクランプ"
    );
    assert!(
        dirty
            .iter()
            .all(|r| r.x + r.w <= surface.0 && r.y + r.h <= surface.1),
        "全ダーティ矩形は面寸を越えない"
    );

    // (c) 端の露出帯もクランプされる（下端 by=-12）。
    let scroll_canvas = canvas_for(&broken_lines(4), WritingMode::HorizontalTb, vr, 10.0);
    let scroll_prev = ScrollPlanner::committed_lines(&scroll_canvas, WritingMode::HorizontalTb);
    let (band, _) = ScrollPlanner::derive_dirty(
        &scroll_canvas,
        &window(1, -12.0),
        WritingMode::HorizontalTb,
        &contract,
        (0, -12),
        surface,
        &scroll_prev,
    );
    // 下端露出帯 {0,113,500,12}・ガード 2（x/y を 2 減らし w/h を 4 増やす＝下端 127）・
    // 面寸 125 でクランプ → {0,111,500,14}。
    assert_eq!(
        band,
        vec![phys(0, 111, 500, 14)],
        "端の露出帯はクランプされる"
    );
    assert!(
        band.iter()
            .all(|r| r.x + r.w <= surface.0 && r.y + r.h <= surface.1),
        "露出帯も面寸を越えない"
    );
}

// ── back 全被覆の素朴檻: blit 写域 ∪ dirty ＝ 面全域（ブロック軸） ──

/// 横書きスクロール（by=-12＝行送り 1 行分）で blit 写域（保持ピクセルの移動先）と dirty（露出帯）の
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
    let blit = (0i32, -12i32);
    let (dirty, _) = ScrollPlanner::derive_dirty(
        &canvas,
        &window(1, -12.0),
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
