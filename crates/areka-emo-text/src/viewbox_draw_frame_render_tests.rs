use super::{DrawStats, ViewboxExecutor};
use crate::draw::ResolvedFont;
use crate::layout::VisibleWindow;
use crate::region::{ScaleContract, TextRegion};
use crate::state::TextItem;
use crate::viewbox::{FramePlan, ScrollPlanner};
use crate::writing::WritingMode;
use super::test_support::{Rig, build, geo_model, glyph_items, opaque_count};

/// 観測可能な完了状態 1: content ありの初回フレーム → render `Ok(true)`・read_back に
/// content の非透明ピクセルが現れる（初回は全域ダーティ＝全行描画・blit=(0,0) ゆえ blits 増なし）。
#[test]
fn initial_frame_renders_content() {
    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    let items = glyph_items("■■");
    let (canvas, window) = build(&items, &region, mode, 10.0);
    let changed = exec
        .render(&canvas, &window, &font, mode, &contract, &mut surface)
        .expect("render 失敗");
    assert!(
        changed,
        "content ありの初回フレームは変化あり（present 要）"
    );

    let bytes = surface.read_back().expect("read_back 失敗");
    assert!(
        opaque_count(&bytes) > 0,
        "初回描画で content の非透明ピクセルが現れる"
    );

    let stats: DrawStats = exec.stats();
    assert!(
        stats.draw_text_layout_calls >= 1,
        "初回は少なくとも 1 行描画する"
    );
    assert_eq!(
        stats.blits, 0,
        "初回 blit=(0,0) は blits を増やさない（全面 CopyResource）"
    );
}

/// 観測可能な完了状態 2: 同一 content・同一 window の再フレーム → plan=NoChange →
/// render `Ok(false)`・blit も描画も行レイアウト生成も発生しない（全カウンタ増分 0）。
#[test]
fn unchanged_frame_is_nochange_no_blit_no_draw() {
    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    let items = glyph_items("■■");
    let (canvas, window) = build(&items, &region, mode, 10.0);
    exec.render(&canvas, &window, &font, mode, &contract, &mut surface)
        .expect("初回 render 失敗");
    let after_first: DrawStats = exec.stats();

    let changed = exec
        .render(&canvas, &window, &font, mode, &contract, &mut surface)
        .expect("再 render 失敗");
    assert!(
        !changed,
        "可視窓不変・content 不変のフレームは変化なし（present 不要）"
    );

    let after_second: DrawStats = exec.stats();
    assert_eq!(
        after_second.blits, after_first.blits,
        "NoChange は blit を発生させない"
    );
    assert_eq!(
        after_second.draw_text_layout_calls, after_first.draw_text_layout_calls,
        "NoChange は描画を発生させない"
    );
    assert_eq!(
        after_second.line_layout_creations, after_first.line_layout_creations,
        "NoChange は行レイアウト生成を発生させない"
    );
}

/// 観測可能な完了状態 3: 可視窓のみ移動（content 不変）→ 保持ピクセルの面内 blit（1 回）と
/// 露出帯の描画だけが発生し、確定行の再描画は起きない（行レイアウト再生成 0）。read_back は
/// スクロールで内容が移動する。期待計画は独立な mirror planner（純粋層）で算出する。
#[test]
fn visible_window_move_blits_and_redraws_only_exposure_band() {
    let mut rig = Rig::new();
    let image = (60u32, 60u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let size = surface.size();
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    // 6 行（pitch 13・canvas-local y=0,13,…,65）。各行を相異なるグリフにし、13px の縦
    // スクロールで供給面の内容が実際に移動する（同一グリフだと行が同型で readback 不変になる）
    // ことを観測可能にする。canvas は validrect-local に全行保持。
    let line_chars = ['あ', 'い', 'う', 'え', 'お', 'か'];
    let mut items = Vec::new();
    for (i, &ch) in line_chars.iter().enumerate() {
        if i > 0 {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        items.push(TextItem::Glyph { ch });
    }
    let (canvas, _auto_window) = build(&items, &region, mode, 10.0);
    let total_glyph_lines = canvas.residents.len();

    // frame A: block_offset 0（スクロール前）。
    let window_a = VisibleWindow {
        first_visible_line: 0,
        block_offset: 0.0,
    };
    exec.render(&canvas, &window_a, &font, mode, &contract, &mut surface)
        .expect("frame A render 失敗");
    let before = surface.read_back().expect("read_back(A) 失敗");
    let stats_a: DrawStats = exec.stats();

    // frame B: 同一 canvas・block_offset -13（1 行ぶん上へスクロール・content 不変）。
    let window_b = VisibleWindow {
        first_visible_line: 1,
        block_offset: -13.0,
    };

    // mirror planner（純粋層）で期待計画を独立に算出する。
    let mut mirror = ScrollPlanner::new();
    let plan_a = mirror.plan(&canvas, &window_a, mode, &contract, size);
    mirror.commit(&canvas, &window_a, mode, &contract, &plan_a);
    let plan_b = mirror.plan(&canvas, &window_b, mode, &contract, size);
    let (exp_blit, exp_draw_lines, exp_dirty_len) = match plan_b {
        FramePlan::Update {
            blit,
            ref draw_lines,
            ref dirty,
        } => (blit, draw_lines.len(), dirty.len()),
        other => panic!("frame B は Update を期待したが {other:?}"),
    };
    assert_ne!(exp_blit, (0, 0), "可視窓移動は blit を生む");
    assert_eq!(exp_dirty_len, 1, "content 不変ゆえ dirty は露出帯 1 枚のみ");
    assert!(exp_draw_lines >= 1, "露出帯には流入行が交差する");
    assert!(
        exp_draw_lines < total_glyph_lines,
        "確定行は再描画対象から外れる（全行描画でない）: {exp_draw_lines} < {total_glyph_lines}"
    );

    let changed = exec
        .render(&canvas, &window_b, &font, mode, &contract, &mut surface)
        .expect("frame B render 失敗");
    assert!(changed, "可視窓移動フレームは変化あり");
    let stats_b: DrawStats = exec.stats();

    assert_eq!(
        stats_b.blits,
        stats_a.blits + 1,
        "面内 blit がちょうど 1 回発生する"
    );
    let draw_delta = (stats_b.draw_text_layout_calls - stats_a.draw_text_layout_calls) as usize;
    assert_eq!(
        draw_delta,
        exp_dirty_len * exp_draw_lines,
        "描画は露出帯（1 枚）× 交差行に限られる"
    );
    assert!(
        draw_delta <= exp_draw_lines,
        "draw_text_layout 増分 {draw_delta} ≤ 露出帯交差行数 {exp_draw_lines}"
    );
    assert_eq!(
        stats_b.line_layout_creations, stats_a.line_layout_creations,
        "content 不変ゆえ確定行レイアウトは再生成されない（保持面＋blit が保持機構）"
    );

    let after = surface.read_back().expect("read_back(B) 失敗");
    assert_ne!(
        before, after,
        "スクロールで供給面の内容が移動する（先頭行が消え内容が上へ）"
    );
}

/// 観測可能な完了状態 4: typewriter 1 グリフ進行 → 描画は現在行（変化行）1 行に限られ、
/// 確定行は再描画も行レイアウト再生成もされない（blit=0）。
#[test]
fn typewriter_progress_draws_only_current_line() {
    let mut rig = Rig::new();
    let image = (120u32, 60u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    // 行0 確定「■■」＋行1 リビール中「■」（2 行・60px に収まる＝block_offset 0）。
    let mut items = glyph_items("■■");
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.push(TextItem::Glyph { ch: '■' });
    let (canvas_a, window_a) = build(&items, &region, mode, 10.0);
    exec.render(&canvas_a, &window_a, &font, mode, &contract, &mut surface)
        .expect("frame A render 失敗");
    let stats_a: DrawStats = exec.stats();

    // 行1 が「■」→「■■」へ 1 グリフ進行（行0 は確定・不変）。
    items.push(TextItem::Glyph { ch: '■' });
    let (canvas_b, window_b) = build(&items, &region, mode, 10.0);
    let changed = exec
        .render(&canvas_b, &window_b, &font, mode, &contract, &mut surface)
        .expect("frame B render 失敗");
    assert!(changed, "typewriter 進行フレームは変化あり");
    let stats_b: DrawStats = exec.stats();

    let draw_delta = stats_b.draw_text_layout_calls - stats_a.draw_text_layout_calls;
    assert_eq!(
        draw_delta, 1,
        "描画は現在行（変化行）1 行のみ・確定行 0 は再描画されない"
    );
    let create_delta = stats_b.line_layout_creations - stats_a.line_layout_creations;
    assert_eq!(
        create_delta, 1,
        "行レイアウト再生成は現在行のみ（確定行 0 はキャッシュ）"
    );
    assert_eq!(
        stats_b.blits, stats_a.blits,
        "typewriter 進行は blit を発生させない（blit=0）"
    );
}

/// 複数行（各行相異なるグリフ）の item 列を組む（縮退檻で「全住人再描画」を数える台）。
fn multiline_items(chars: &[char]) -> Vec<TextItem> {
    let mut items = Vec::new();
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        items.push(TextItem::Glyph { ch });
    }
    items
}

/// 観測可能な完了状態（R4.3）: content 描画済み → `request_clear()` → 次フレームが
/// FullClear（`full_clears` +1・read_back 全域透明）→ さらに次フレームで content が
/// 全域ダーティ再描画で復帰する（透明フラッシュは 1 フレームのみ）。
#[test]
fn request_clear_triggers_full_clear_then_content_returns() {
    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    let items = glyph_items("■■");
    let (canvas, window) = build(&items, &region, mode, 10.0);

    // frame 1: content 描画（初回＝全域ダーティ）。
    exec.render(&canvas, &window, &font, mode, &contract, &mut surface)
        .expect("content render 失敗");
    let painted = surface.read_back().expect("read_back(content) 失敗");
    assert!(
        opaque_count(&painted) > 0,
        "content の非透明ピクセルが現れる"
    );
    let full_before = exec.stats().full_clears;

    // Clear cue 適用点（planner 初期化＋行キャッシュ破棄はこの口だけ）。
    exec.request_clear();

    // frame 2: FullClear（描画 0 件・back 全域透明 Clear → flip）。
    let changed = exec
        .render(&canvas, &window, &font, mode, &contract, &mut surface)
        .expect("FullClear render 失敗");
    assert!(changed, "FullClear は present 要（変化あり）");
    assert_eq!(
        exec.stats().full_clears,
        full_before + 1,
        "FullClear は full_clears をちょうど 1 増やす"
    );
    let cleared = surface.read_back().expect("read_back(cleared) 失敗");
    assert_eq!(opaque_count(&cleared), 0, "Clear 後は全域透明（全 α=0）");

    // frame 3: 同一 content の再フレーム → prev_lines 空ゆえ全域ダーティで content 復帰
    //（透明フラッシュは frame 2 の 1 回のみ・FullClear は増えない）。
    let changed3 = exec
        .render(&canvas, &window, &font, mode, &contract, &mut surface)
        .expect("Clear 後 content render 失敗");
    assert!(changed3, "Clear 後の再描画は全域ダーティで変化あり");
    assert_eq!(
        exec.stats().full_clears,
        full_before + 1,
        "content 復帰は FullClear を増やさない（全域ダーティ Update）"
    );
    let restored = surface.read_back().expect("read_back(restored) 失敗");
    assert!(
        opaque_count(&restored) > 0,
        "content が全域ダーティで復帰する"
    );
}

/// 観測可能な完了状態（縮退の主檻・R3.5/Error Handling）: フォント（高さ）変更を注入すると
/// format と行キャッシュが組み直され、当該フレームが全域ダーティへ縮退して全住人が再描画される
///（`draw_text_layout_calls` 増分＝全 GlyphRun 住人数・`line_layout_creations` 増分＝全住人数
/// ＝キャッシュ破棄で全再生成・`full_clears` 不変＝透明フラッシュを起こす FullClear と区別・
/// read_back は font B の content で非透明）。
#[test]
fn font_change_degrades_to_full_domain_redraw() {
    let mut rig = Rig::new();
    let image = (120u32, 120u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    // 3 行（相異なるグリフ・全行が 120px に収まる＝block_offset 0）。
    let chars = ['あ', 'い', 'う'];
    let items = multiline_items(&chars);

    // font A（高さ 10）で描画（初回＝全域ダーティ）。
    let font_a = ResolvedFont::resolve(&geo_model(Some(10)));
    let region_a = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let (canvas_a, window_a) = build(&items, &region_a, mode, 10.0);
    let total = canvas_a.residents.len();
    assert_eq!(total, 3, "3 行の GlyphRun 住人（全住人再描画を数える台）");
    exec.render(&canvas_a, &window_a, &font_a, mode, &contract, &mut surface)
        .expect("font A render 失敗");
    let stats_a = exec.stats();
    let full_before = stats_a.full_clears;

    // font B（高さ 14）へ変更（FormatKey の高さビットが変わり format/行キャッシュ組み直し）。
    let font_b = ResolvedFont::resolve(&geo_model(Some(14)));
    let region_b = TextRegion::resolve(&geo_model(Some(14)), image, mode);
    let (canvas_b, window_b) = build(&items, &region_b, mode, 14.0);
    let total_b = canvas_b.residents.len();
    assert_eq!(total_b, 3, "font B でも 3 行（全住人が縮退で再描画される）");

    let changed = exec
        .render(&canvas_b, &window_b, &font_b, mode, &contract, &mut surface)
        .expect("font B render 失敗");
    assert!(changed, "font 変更フレームは変化あり（全域ダーティ縮退）");
    let stats_b = exec.stats();

    let draw_delta = stats_b.draw_text_layout_calls - stats_a.draw_text_layout_calls;
    assert_eq!(
        draw_delta, total_b as u64,
        "縮退フレームは全 GlyphRun 住人を再描画する（全域ダーティ 1 枚 × 全住人）"
    );
    let create_delta = stats_b.line_layout_creations - stats_a.line_layout_creations;
    assert_eq!(
        create_delta, total_b as u64,
        "行キャッシュ破棄ゆえ全住人ぶん行レイアウトを再生成する"
    );
    assert_eq!(
        stats_b.full_clears, full_before,
        "縮退は全域ダーティ Update（透明フラッシュを起こす FullClear ではない）"
    );
    let bytes = surface.read_back().expect("read_back(font B) 失敗");
    assert!(
        opaque_count(&bytes) > 0,
        "font B の content が全域ダーティで正しく描かれる（透明のままでない）"
    );
}

/// 想定外不整合の検査関数（render の縮退経路が呼ぶ述語）が矛盾入力で理由を返し、整合入力で
/// `None` を返す（範囲外 draw_lines・面寸超過 dirty の両トリガ・R Error Handling）。
/// render からの結線は `font_change_degrades_to_full_domain_redraw`（縮退の実発火）が担保する。
#[test]
fn plan_inconsistency_detects_out_of_range_and_oversize() {
    use super::plan_inconsistency;
    use crate::viewbox::PhysicalRect;

    let size = (100u32, 50u32);
    let full = PhysicalRect {
        x: 0,
        y: 0,
        w: 100,
        h: 50,
    };

    // 整合: 範囲内 index・面寸ちょうどの dirty → None。
    assert!(
        plan_inconsistency(&[full], &[0, 1], 2, size).is_none(),
        "範囲内・面寸内は整合（縮退しない）"
    );

    // 範囲外 draw_lines（住人数 2 に対し index 5）→ Some。
    assert!(
        plan_inconsistency(&[], &[5], 2, size).is_some(),
        "draw_lines の範囲外 index を検知する"
    );

    // dirty が面幅を超える（x+w=101 > 100）→ Some。
    assert!(
        plan_inconsistency(
            &[PhysicalRect {
                x: 0,
                y: 0,
                w: 101,
                h: 50
            }],
            &[],
            2,
            size
        )
        .is_some(),
        "面幅を超える dirty を検知する"
    );

    // dirty が面高を超える（y+h=60 > 50）→ Some。
    assert!(
        plan_inconsistency(
            &[PhysicalRect {
                x: 0,
                y: 40,
                w: 100,
                h: 20
            }],
            &[],
            2,
            size
        )
        .is_some(),
        "面高を超える dirty を検知する"
    );
}

/// G5: 描画中デバイス失敗の再試行安全を実描画で檻化する。EndDraw 後・flip/commit 前に失敗を
/// 注入（test-only fault-injection）すると、その Update フレームは `Err` を返し、**front は不変**
/// （flip されない＝前フレームの確定面を保持）・**planner は未 commit**（次フレームで同一計画を
/// 再試行）になる。失敗解除後の再提示で正しい content が反映されることで「未 commit＝再試行安全」を
/// 証明する（もし失敗フレームで誤って commit していたら、再試行が NoChange になり content が
/// 反映されず本檻が落ちる）。純粋 ScrollPlanner の再試行檻（task 3.3）の COM 側 runtime 版。
#[test]
fn device_failure_mid_render_is_retry_safe_front_unchanged_no_commit() {
    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    // フレーム 1: content "■" を描き確定（front＝"■"）。
    let (canvas1, window1) = build(&glyph_items("■"), &region, mode, 10.0);
    assert!(
        exec.render(&canvas1, &window1, &font, mode, &contract, &mut surface)
            .expect("フレーム1 render 失敗"),
        "初回 content フレームは変化あり"
    );
    let r1 = surface.read_back().expect("read_back(frame1)");
    let ink1 = opaque_count(&r1);
    assert!(ink1 > 0, "フレーム1で content が描かれる");

    // フレーム 2: content を "■■" へ伸長（Update）。だが EndDraw 後に失敗を注入する。
    let (canvas2, window2) = build(&glyph_items("■■"), &region, mode, 10.0);
    exec.inject_render_failure();
    let err = exec
        .render(&canvas2, &window2, &font, mode, &contract, &mut surface)
        .err()
        .expect("失敗注入フレームは Err を返す（panic しない・log-first）");
    assert!(
        matches!(err, crate::TextLayerError::Device { .. }),
        "注入失敗は Device エラー: {err:?}"
    );
    // front 不変: flip していないため read_back は前フレーム "■" のまま（伸長が反映されない）。
    let r2 = surface.read_back().expect("read_back(frame2 失敗後)");
    assert_eq!(
        r2, r1,
        "失敗フレームは front を変えない（flip せず＝前フレームの確定面を保持）"
    );

    // フレーム 3: 失敗は解除済み（自動消費）。同一 content "■■" を再提示すると、planner が
    // 未 commit（prev_lines は依然 "■"）ゆえ Update を再計画し、今度は成功して front が更新される。
    let changed = exec
        .render(&canvas2, &window2, &font, mode, &contract, &mut surface)
        .expect("フレーム3（再試行）render 失敗");
    assert!(
        changed,
        "再試行フレームは変化あり（未 commit ゆえ再計画される）"
    );
    let r3 = surface.read_back().expect("read_back(frame3 再試行)");
    assert_ne!(
        r3, r1,
        "再試行で content 伸長が反映される（front が更新＝失敗フレームで commit していない証拠）"
    );
    assert!(
        opaque_count(&r3) > ink1,
        "再試行後は伸長ぶんインクが増える: {} > {ink1}",
        opaque_count(&r3)
    );
}
