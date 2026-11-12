use super::test_support::{
    RenderPair, Rig, build, diff_rows, geo_model, glyph_items, opaque_count, table_with,
};
use super::{DrawStats, ViewboxExecutor};
use crate::canvas::ResidentContent;
use crate::draw::{ResolvedFont, create_text_format};
use crate::layout::VisibleWindow;
use crate::look::TextLook;
use crate::region::{ScaleContract, TextRegion};
use crate::state::TextItem;
use crate::viewbox::{FramePlan, ScrollPlanner};
use crate::writing::WritingMode;

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
    assert_eq!(
        stats.draw_text_layout_calls, 1,
        "初回は 1 行の住人をちょうど 1 度描く"
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

    // 6 行（font 10 → pitch 10+2=12・canvas-local y=0,12,…,60）。各行を相異なるグリフにし、
    // 12px の縦スクロールで供給面の内容が実際に移動する（同一グリフだと行が同型で readback
    // 不変になる）ことを観測可能にする。canvas は validrect-local に全行保持。
    let items = multiline_items(&["あ", "い", "う", "え", "お", "か"]);
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

    // frame B: 同一 canvas・block_offset -12（1 行ぶん上へスクロール・content 不変）。
    let window_b = VisibleWindow {
        first_visible_line: 1,
        block_offset: -12.0,
    };

    // mirror planner（純粋層）で期待計画を独立に算出する。
    let mut mirror = ScrollPlanner::new();
    let plan_a = mirror.plan(&canvas, &window_a, mode, &contract, size);
    mirror.commit(&canvas, &window_a, mode, &contract, &plan_a);
    let plan_b = mirror.plan(&canvas, &window_b, mode, &contract, size);
    let (exp_blit, exp_draw_lines, exp_dirty_len, exp_line_sum) = match plan_b {
        FramePlan::Update {
            blit,
            ref draw_lines,
            ref dirty,
        } => (
            blit,
            draw_lines.len(),
            dirty.len(),
            dirty.iter().map(|d| d.lines.len()).sum::<usize>(),
        ),
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
        draw_delta, exp_line_sum,
        "描画は矩形ごとの交差行数の**和**に限られる（R11.1）"
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

/// R11.1/11.3: ダーティ矩形が 2 枚以上あるフレーム（追いつき＝新しい行が一挙に入りつつ
/// スクロールする）で、`DrawTextLayout` の増分が **矩形ごとの交差行数の和** に一致し、
/// 「枚数 × 描画対象行数」の積にはならないこと。
///
/// 直前の `visible_window_move_blits_and_redraws_only_exposure_band` は dirty が 1 枚
/// （露出帯のみ）なので和と積が一致し、この差を弁別できない——本テストが弁別の担い手。
/// 期待値は独立な mirror planner（純粋層）から採り、和 < 積 が成り立つ入力であること自体も
/// 検査する（幾何が変わって弁別力を失ったら赤で気づける）。
#[test]
fn catch_up_scroll_draws_sum_of_per_rect_lines_not_product() {
    let mut rig = Rig::new();
    let image = (60u32, 84u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let size = surface.size();
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    // font 10 → pitch 12。行 i の下端 = 12i + 10 ゆえ 84px には 7 行（i=0..6・82 ≤ 84）が収まり、
    // 8 行目（94）からあふれる。frame A は 7 行（スクロールなし）。
    let chars_a = ["あ", "い", "う", "え", "お", "か", "き"];
    let (canvas_a, window_a) = build(&multiline_items(&chars_a), &region, mode, 10.0);
    exec.render(&canvas_a, &window_a, &font, mode, &contract, &mut surface)
        .expect("frame A render 失敗");
    let stats_a: DrawStats = exec.stats();

    // frame B: 2 行が一挙に入り（追いつき）、途中の行 3 の字も変わる＝下端の組（露出帯と新規行 2 本は
    // 重なるので 1 枚にまとまる・再監査 R2-C1）＋ 行 3 の 1 枚のダーティ。
    let chars_b = ["あ", "い", "う", "ゑ", "お", "か", "き", "く", "け"];
    let (canvas_b, window_b) = build(&multiline_items(&chars_b), &region, mode, 10.0);

    // mirror planner（純粋層）で期待計画を独立に算出する。
    let mut mirror = ScrollPlanner::new();
    let plan_a = mirror.plan(&canvas_a, &window_a, mode, &contract, size);
    mirror.commit(&canvas_a, &window_a, mode, &contract, &plan_a);
    let plan_b = mirror.plan(&canvas_b, &window_b, mode, &contract, size);
    let (exp_dirty_len, exp_draw_lines, exp_line_sum) = match plan_b {
        FramePlan::Update {
            ref dirty,
            ref draw_lines,
            ..
        } => (
            dirty.len(),
            draw_lines.len(),
            dirty.iter().map(|d| d.lines.len()).sum::<usize>(),
        ),
        other => panic!("frame B は Update を期待したが {other:?}"),
    };
    assert!(
        exp_dirty_len >= 2,
        "弁別の前提: ダーティ矩形が 2 枚以上（実際 {exp_dirty_len} 枚）"
    );
    assert!(
        exp_line_sum < exp_dirty_len * exp_draw_lines,
        "弁別の前提: 和 {exp_line_sum} < 積 {}（矩形ごとに交差行が異なる入力）",
        exp_dirty_len * exp_draw_lines
    );

    let changed = exec
        .render(&canvas_b, &window_b, &font, mode, &contract, &mut surface)
        .expect("frame B render 失敗");
    assert!(changed, "追いつきフレームは変化あり");
    let stats_b: DrawStats = exec.stats();

    let draw_delta = (stats_b.draw_text_layout_calls - stats_a.draw_text_layout_calls) as usize;
    assert_eq!(
        draw_delta,
        exp_line_sum,
        "描画呼び出しは矩形ごとの交差行数の和（積 {} ではない）",
        exp_dirty_len * exp_draw_lines
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
    items.push(TextItem::glyph("■"));
    let (canvas_a, window_a) = build(&items, &region, mode, 10.0);
    exec.render(&canvas_a, &window_a, &font, mode, &contract, &mut surface)
        .expect("frame A render 失敗");
    let stats_a: DrawStats = exec.stats();

    // 行1 が「■」→「■■」へ 1 グリフ進行（行0 は確定・不変）。
    items.push(TextItem::glyph("■"));
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
fn multiline_items(chars: &[&str]) -> Vec<TextItem> {
    let mut items = Vec::new();
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        items.push(TextItem::glyph(ch));
    }
    items
}

/// 観測可能な完了状態（R4.3・指摘 17）: content 描画済み → `request_clear()` → 次フレームに
/// 別の字があれば、そのフレームで全面を消して字を描く（透明のフラッシュを挟まない）＝描いた面が
/// 全域再描画の見本と画素で一致し、`full_clears` はちょうど 1 増える。次の同じ入力は何も数えない。
/// 見える行の無い消去は FullClear（全域透明・`full_clears` +1）。
#[test]
fn request_clear_draws_same_frame_glyphs_and_counts_one_clear() {
    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mut pair = RenderPair::attach(&mut rig, image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);

    // frame 1: content A を描く（初回＝全域）。
    let (canvas_a, window_a) = build(&glyph_items("■■■■"), &region, mode, 10.0);
    let (_, painted) = pair.render("A", &canvas_a, &window_a, &font, mode, &contract);
    assert!(
        opaque_count(&painted) > 0,
        "content A の非透明ピクセルが現れる"
    );
    let full_before = pair.viewbox.stats().full_clears;

    // frame 2: 消去と同じフレームに content B（A より短い＝消し残りがあれば見本と食い違う）。
    pair.viewbox.request_clear();
    let (canvas_b, window_b) = build(&glyph_items("あ"), &region, mode, 10.0);
    let (oracle, drawn) = pair.render("clear+B", &canvas_b, &window_b, &font, mode, &contract);
    assert!(opaque_count(&drawn) > 0, "消去のフレームで字 B が描かれる");
    assert!(
        oracle == drawn,
        "消去と同じフレームの面が見本と画素で一致する（食い違う行: {:?}）",
        diff_rows(&oracle, &drawn, image.0, image.1)
    );
    assert_eq!(
        pair.viewbox.stats().full_clears,
        full_before + 1,
        "消去の要求が立っていたフレームを 1 と数える"
    );

    // frame 3: 同じ入力は要求が下りているので何も数えない。
    let (_, again) = pair.render("B again", &canvas_b, &window_b, &font, mode, &contract);
    assert_eq!(again, drawn, "同じ入力の再フレームは面を変えない");
    assert_eq!(
        pair.viewbox.stats().full_clears,
        full_before + 1,
        "要求の下りた後のフレームは full_clears を増やさない"
    );

    // frame 4: 見える行の無い消去は FullClear（全域透明）。
    pair.viewbox.request_clear();
    let (canvas_e, window_e) = build(&[], &region, mode, 10.0);
    let (_, cleared) = pair.render("clear+empty", &canvas_e, &window_e, &font, mode, &contract);
    assert_eq!(opaque_count(&cleared), 0, "行の無い消去は全域透明");
    assert_eq!(
        pair.viewbox.stats().full_clears,
        full_before + 2,
        "FullClear も 1 と数える"
    );
}

/// 台詞の切れ目（指摘 11）: `request_clear` がブラシの記憶を空にし、書体の台帳の台詞を進める。
#[test]
fn request_clear_forgets_brushes_and_advances_the_talk() {
    let mut rig = Rig::new();
    let image = (40u32, 20u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let font = ResolvedFont::resolve(&geo_model(Some(10)));
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let red = TextLook {
        color: (255, 0, 0),
        ..font.looks.default.clone()
    };
    let (styles, id) = table_with(&red, &font.looks.default);
    let (mut canvas, window) = build(&glyph_items("あ"), &region, mode, 10.0);
    for r in &mut canvas.residents {
        if let ResidentContent::GlyphRun(run) = &mut r.content {
            run.glyphs.iter_mut().for_each(|g| g.style = id);
        }
    }
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec.render_styled(
        &canvas,
        &window,
        &font,
        mode,
        &contract,
        &mut surface,
        &styles,
    )
    .expect("render_styled 失敗");
    assert_eq!(exec.brushes.len(), 1, "赤のブラシを 1 本覚える");
    let epoch = exec.fonts.talk_epoch();

    exec.request_clear();
    assert_eq!(exec.brushes.len(), 0, "消去の要求でブラシの記憶が空になる");
    assert_eq!(exec.fonts.talk_epoch(), epoch + 1, "台帳の台詞が 1 進む");
}

/// 観測可能な完了状態（縮退の主檻・R3.5/Error Handling）: フォント（高さ）変更を注入すると
/// format と行キャッシュが組み直され、当該フレームが全域ダーティへ縮退して全住人が再描画される
///（`draw_text_layout_calls` 増分＝全 GlyphRun 住人数・`line_layout_creations` 増分＝全住人数
/// ＝キャッシュ破棄で全再生成・`full_clears` 不変＝透明フラッシュを起こす FullClear と区別・
/// read_back は font B の content で非透明）。
///
/// canvas と可視窓は font A のまま変えない＝ふつうの差分なら何も描かない入力（対照で 0 を確かめる）
/// なので、全住人の再描画は縮退だけから来る。
#[test]
fn font_change_degrades_to_full_domain_redraw() {
    let mut rig = Rig::new();
    let image = (120u32, 120u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");

    // 3 行（相異なるグリフ・全行が 120px に収まる＝block_offset 0）。
    let chars = ["あ", "い", "う"];
    let items = multiline_items(&chars);

    // font A（高さ 10）で描画（初回＝全域ダーティ）。
    let font_a = ResolvedFont::resolve(&geo_model(Some(10)));
    let region_a = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let (canvas_a, window_a) = build(&items, &region_a, mode, 10.0);
    let total = canvas_a.residents.len();
    assert_eq!(total, 3, "3 行の GlyphRun 住人（全住人再描画を数える台）");
    exec.render(&canvas_a, &window_a, &font_a, mode, &contract, &mut surface)
        .expect("font A render 失敗");
    // 対照: 同じ canvas・窓・font A をもう一度描くと、ふつうの差分は何も描かない。
    let before_control = exec.stats().draw_text_layout_calls;
    exec.render(&canvas_a, &window_a, &font_a, mode, &contract, &mut surface)
        .expect("font A 再 render 失敗");
    let stats_a = exec.stats();
    assert_eq!(
        stats_a.draw_text_layout_calls, before_control,
        "前提: 同じ入力のふつうの差分は 0 行を描く"
    );
    let full_before = stats_a.full_clears;

    // font B（高さ 14）へ変更（FormatKey の高さビットが変わり format/行キャッシュ組み直し）。
    let font_b = ResolvedFont::resolve(&geo_model(Some(14)));
    let total_b = total;
    let changed = exec
        .render(&canvas_a, &window_a, &font_b, mode, &contract, &mut surface)
        .expect("font B render 失敗");
    assert!(changed, "font 変更フレームは変化あり（全域ダーティ縮退）");
    let stats_b = exec.stats();

    let draw_delta = stats_b.draw_text_layout_calls - stats_a.draw_text_layout_calls;
    assert_eq!(
        draw_delta, total_b as u64,
        "縮退フレームは全 GlyphRun 住人を再描画する（全域ダーティは矩形 1 枚に全住人が交差＝和と積が一致する唯一の形）"
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

/// K1（要件 4.8・4.9）: 束縛書式の記憶の鍵は書式を決める値すべてを見る。同じ実行部で、先頭の
/// 候補・大きさ・書字方向が同じで既定の見た目の太字・斜体だけが違うフォントを続けて描くと、
/// 書式が組み直され、2 度目の束縛書式の太さ・斜体が新しい値（同じ経路で作った書式の値）になる。
#[test]
fn bold_or_italic_only_change_rebuilds_the_bound_format() {
    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mut surface = rig.attach(image, 1.0);
    let mode = WritingMode::HorizontalTb;
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    let (canvas, window) = build(&glyph_items("■■"), &region, mode, 10.0);

    let font_a = ResolvedFont::resolve(&geo_model(Some(10)));
    let mut font_b = font_a.clone();
    font_b.looks.default.bold = true;
    font_b.looks.default.italic = true;

    // 束縛書式の太さ・斜体（実行部が今持っている書式）。
    let bound = |exec: &ViewboxExecutor| {
        let (_, format) = exec.format.as_ref().expect("描いた後は書式を持つ");
        unsafe { (format.GetFontWeight(), format.GetFontStyle()) }
    };

    exec.render(&canvas, &window, &font_a, mode, &contract, &mut surface)
        .expect("font A render 失敗");
    let first = bound(&exec);
    let creations_a = exec.stats().line_layout_creations;
    let draws_a = exec.stats().draw_text_layout_calls;

    let changed = exec
        .render(&canvas, &window, &font_b, mode, &contract, &mut surface)
        .expect("font B render 失敗");
    let second = bound(&exec);
    // 配置は同じなので、組み直しが前の行を忘れさせて初めて全域の道で描き直す。
    assert!(changed, "太字・斜体だけの変更フレームは変化あり");
    assert_eq!(
        exec.stats().draw_text_layout_calls - draws_a,
        canvas.residents.len() as u64,
        "太字・斜体だけの変更でも全住人を描き直す（古い画素を残さない）"
    );

    let expected =
        create_text_format(&exec.dwrite, &exec.fonts.pick(&font_b), mode).expect("期待値の書式");
    let expected = unsafe { (expected.GetFontWeight(), expected.GetFontStyle()) };
    assert_ne!(first, expected, "弁別の前提: 太字・斜体で書式の値が変わる");
    assert_eq!(
        second, expected,
        "太字・斜体だけの変更でも束縛書式が組み直され新しい値になる"
    );
    assert!(
        exec.stats().line_layout_creations > creations_a,
        "組み直しで行キャッシュも捨てられ行レイアウトを作り直す"
    );
}

/// 想定外不整合の検査関数（render の縮退経路が呼ぶ述語）が矛盾入力で理由を返し、整合入力で
/// `None` を返す（範囲外 draw_lines・面寸超過 dirty・矩形の行が描画対象行の部分集合でない、の
/// 3 トリガ・R Error Handling／R11.1）。
/// 縮退の判定（`degrade_if_needed`）からこの述語への結線は
/// `degrade_full_domain_draws_only_rows_overlapping_surface` が確かめる。
#[test]
fn plan_inconsistency_detects_out_of_range_and_oversize() {
    use super::plan::plan_inconsistency;
    use crate::viewbox::{DirtyRect, PhysicalRect};

    let size = (100u32, 50u32);
    let full = PhysicalRect {
        x: 0,
        y: 0,
        w: 100,
        h: 50,
    };
    /// 矩形＋交差行の対を組む短縮構築。
    fn dr(rect: PhysicalRect, lines: &[usize]) -> DirtyRect {
        DirtyRect {
            rect,
            lines: lines.to_vec(),
        }
    }

    // 整合: 範囲内 index・面寸ちょうどの dirty・矩形の行 ⊆ draw_lines → None。
    assert!(
        plan_inconsistency(&[dr(full, &[0, 1])], &[0, 1], 2, size).is_none(),
        "範囲内・面寸内・部分集合は整合（縮退しない）"
    );

    // 範囲外 draw_lines（住人数 2 に対し index 5）→ Some。
    assert!(
        plan_inconsistency(&[], &[5], 2, size).is_some(),
        "draw_lines の範囲外 index を検知する"
    );

    // dirty が面幅を超える（x+w=101 > 100）→ Some。
    assert!(
        plan_inconsistency(
            &[dr(
                PhysicalRect {
                    x: 0,
                    y: 0,
                    w: 101,
                    h: 50
                },
                &[]
            )],
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
            &[dr(
                PhysicalRect {
                    x: 0,
                    y: 40,
                    w: 100,
                    h: 20
                },
                &[]
            )],
            &[],
            2,
            size
        )
        .is_some(),
        "面高を超える dirty を検知する"
    );

    // 矩形の行が draw_lines に無い（Phase 1 が資源を組まない行を矩形が要求している）→ Some。
    assert!(
        plan_inconsistency(&[dr(full, &[0, 1])], &[0], 2, size).is_some(),
        "矩形の行が描画対象行の部分集合でないことを検知する"
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

/// 再監査 R2-D1: 書式を組み直したフレームが失敗しても、次のフレームは全域を描き直す（組み直しは
/// 済んでいて鍵が一致するので、計画が NoChange になって古い見た目の画素が残る、にしない）。
/// 再試行の絵は、新しい書式で 1 度だけ描いた新しい描き手の絵とバイトで一致する。
#[test]
fn a_failed_frame_after_a_format_rebuild_redraws_the_whole_surface_next() {
    let mut rig = Rig::new();
    let image = (80u32, 40u32);
    let mode = WritingMode::HorizontalTb;
    let region = TextRegion::resolve(&geo_model(Some(10)), image, mode);
    let contract = ScaleContract::new(1.0, None);
    let (canvas, window) = build(&glyph_items("■■"), &region, mode, 10.0);
    let font_a = ResolvedFont::resolve(&geo_model(Some(10)));
    let mut font_b = font_a.clone();
    font_b.looks.default.bold = true;

    let mut surface = rig.attach(image, 1.0);
    let mut exec = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    exec.render(&canvas, &window, &font_a, mode, &contract, &mut surface)
        .expect("font A render 失敗");
    exec.inject_render_failure();
    assert!(
        exec.render(&canvas, &window, &font_b, mode, &contract, &mut surface)
            .is_err(),
        "組み直したフレームに失敗を注入する"
    );
    let draws = exec.stats().draw_text_layout_calls;
    let changed = exec
        .render(&canvas, &window, &font_b, mode, &contract, &mut surface)
        .expect("再試行 render 失敗");
    assert!(changed, "失敗の次のフレームは描き直す");
    assert_eq!(
        exec.stats().draw_text_layout_calls - draws,
        canvas.residents.len() as u64,
        "全域を描き直す"
    );
    let retried = surface.read_back().expect("read_back(再試行)");

    let mut fresh_surface = rig.attach(image, 1.0);
    let mut fresh = ViewboxExecutor::new(&rig.core).expect("ViewboxExecutor::new 失敗");
    fresh
        .render(
            &canvas,
            &window,
            &font_b,
            mode,
            &contract,
            &mut fresh_surface,
        )
        .expect("新しい描き手の render 失敗");
    assert!(
        retried == fresh_surface.read_back().expect("read_back(新しい描き手)"),
        "再試行の絵は新しい描き手の絵と一致する"
    );
}

/// 指摘 8: 選択肢の帯の超過分は既定の字の高さでなく、その行の丈（`run.size` の行送りの軸＝
/// `resident_rect` と同じ）と比べる。既定 28・行の em 14・帯 17（寄せ −1）なら超過は 2。
#[test]
fn choice_band_overflow_uses_row_extent_not_default_height() {
    use super::render::expand_overhang_for_band;
    use crate::viewbox::LineOverhang;
    let none = LineOverhang::default();
    let h = expand_overhang_for_band(none, 17.0, -1.0, (40.0, 14.0), WritingMode::HorizontalTb);
    assert_eq!(h.bottom, 2.0, "横書き: 帯の下端 16 − 行の丈 14");
    let v = expand_overhang_for_band(none, 17.0, -1.0, (14.0, 40.0), WritingMode::VerticalRl);
    assert_eq!(v.right, 2.0, "縦書き: 帯の右端 16 − 列の丈 14");
    assert_eq!(v.left, 1.0, "縦書き: 負の寄せの分だけ左へ");
}

/// 指摘 18・23: 描き手の全域の縮退（GPU 無し）。3 行（y 0・12・24・丈 10）を丈 20 の面で縮退させると、
/// 面の下の行 2 は描かず、上のはみ出し 5 で面へ入れば描く。整合した計画・消去・変化なしはそのまま。
#[test]
fn degrade_full_domain_draws_only_rows_overlapping_surface() {
    use super::plan::degrade_if_needed;
    use crate::viewbox::{DirtyRect, LineOverhang, PhysicalRect};
    let mode = WritingMode::HorizontalTb;
    let contract = ScaleContract::new(1.0, None);
    let region = TextRegion::resolve(&geo_model(Some(10)), (120, 120), mode);
    let (canvas, window) = build(&multiline_items(&["あ", "い", "う"]), &region, mode, 10.0);
    let degrade = |plan, over: &[LineOverhang]| {
        degrade_if_needed(plan, &canvas, &window, mode, &contract, (120, 20), over)
    };
    let draw_of = |plan| match plan {
        FramePlan::Update {
            blit, draw_lines, ..
        } => {
            assert_eq!(blit, (0, 0), "縮退は送らない");
            draw_lines
        }
        other => panic!("全域の Update を期待: {other:?}"),
    };
    let update = |draw_lines: Vec<usize>| FramePlan::Update {
        blit: (0, 0),
        dirty: vec![DirtyRect {
            rect: PhysicalRect {
                x: 0,
                y: 0,
                w: 10,
                h: 10,
            },
            lines: draw_lines.clone(),
        }],
        draw_lines,
    };
    assert_eq!(
        draw_of(degrade(update(vec![99]), &[])),
        vec![0, 1],
        "範囲外の行を描く計画は全域へ縮退する（面の下の行 2 は描かない）"
    );
    let mut over = vec![LineOverhang::default(); 3];
    over[2].top = 5.0;
    assert_eq!(draw_of(degrade(update(vec![99]), &over)), vec![0, 1, 2]);
    assert_eq!(
        degrade(update(vec![0]), &[]),
        update(vec![0]),
        "整合した計画はそのまま"
    );
    assert_eq!(degrade(FramePlan::FullClear, &over), FramePlan::FullClear);
    assert_eq!(degrade(FramePlan::NoChange, &[]), FramePlan::NoChange);
}
