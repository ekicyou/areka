use super::{FixedMetrics, GlyphMetrics, LayoutEngine, LineRect, VisibleWindow, WrapPlan};
use crate::region::TextRegion;
use crate::state::TextItem;
use crate::writing::WritingMode;
use super::test_support::{
    IMAGE, broken_lines, glyphs, inline_positions, model, model_rect, window_for,
};

// ── R4.5: FixedMetrics の決定論仮想値（全角=height・半角=height/2・pitch=ceil(×1.25)） ──

/// 全角（非 ASCII）は font_height・半角（ASCII）は font_height/2。
#[test]
fn fixed_metrics_advance_full_width_for_nonascii_half_for_ascii() {
    let m = FixedMetrics;
    assert_eq!(m.advance('あ', 12.0), 12.0);
    assert_eq!(m.advance('a', 12.0), 6.0);
    assert_eq!(m.advance(' ', 10.0), 5.0);
    assert_eq!(m.advance('漢', 10.0), 10.0);
}

/// 行送りピッチは ceil(font_height × 1.25)——端数ケースで檻化
/// （tasks.md Implementation Notes: floor/丸めなし変異を殺す値を選ぶ）。
#[test]
fn fixed_metrics_line_pitch_ceils_fractional_values() {
    let m = FixedMetrics;
    assert_eq!(m.line_pitch(12.0), 15.0); // 15.0（割り切れ）
    assert_eq!(m.line_pitch(10.0), 13.0); // 12.5 → 13（ceil でなければ fail）
}

// ── R6.1: 横書き——行内 +x・行送り +y・折返し閾値 wordwrappoint.x ──

/// 横書きの折返し: 行内位置＋次グリフ幅 > 閾値で折返し、
/// ちょうど閾値で終わるグリフは折返さない（> 判定・境界檻）。
#[test]
fn horizontal_wraps_before_glyph_exceeding_threshold() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(50), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // 全角 6 グリフ・font 10: 5 個目は送り終端 50＝閾値ちょうど→残留、
    // 6 個目は 50+10=60 > 50 → 折返し。
    let lines = LayoutEngine::layout(
        &glyphs(6),
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2);
    assert_eq!(
        inline_positions(&lines[0]),
        vec![0.0, 10.0, 20.0, 30.0, 40.0]
    );
    assert_eq!(inline_positions(&lines[1]), vec![0.0]);
    // 行矩形: 行内範囲＝開始〜送り終端・行送り軸は +y へ pitch(13) 進む。
    assert_eq!(
        lines[0].rect,
        LineRect {
            left: 0.0,
            top: 0.0,
            right: 50.0,
            bottom: 10.0
        }
    );
    assert_eq!(
        lines[1].rect,
        LineRect {
            left: 0.0,
            top: 13.0,
            right: 10.0,
            bottom: 23.0
        }
    );
}

/// 半角/全角混在の送り幅が累積する（グリフ別 advance の出力整合・R9.4）。
#[test]
fn horizontal_mixed_width_advances_accumulate() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'a' },
        TextItem::Glyph { ch: 'あ' },
        TextItem::Glyph { ch: 'b' },
    ];
    let lines = LayoutEngine::layout(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 1);
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 6.0, 18.0]);
    let advances: Vec<f32> = lines[0].glyphs.iter().map(|g| g.advance).collect();
    assert_eq!(advances, vec![6.0, 12.0, 6.0]);
    assert_eq!(lines[0].rect.right, 24.0);
}

// ── R6.2: 日本語縦書き（vertical_rl）——行内 +y・行送り −x・閾値 wordwrappoint.y ──

/// vertical_rl: 折返しは y 閾値・列は右→左へ pitch 分進む・列矩形は左方向へ厚み。
#[test]
fn vertical_rl_wraps_on_y_threshold_and_feeds_leftward() {
    let region = TextRegion::resolve(
        &model((None, None), (None, Some(30))),
        IMAGE,
        WritingMode::VerticalRl,
    );
    // 書字開始角＝validrect 右上 (400, 0)。全角 5 グリフ・font 10:
    // 3 個目は送り終端 30＝閾値ちょうど→残留、4 個目で折返し。
    assert_eq!(region.start(), (400.0, 0.0));
    let lines = LayoutEngine::layout(
        &glyphs(5),
        5,
        &region,
        WritingMode::VerticalRl,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2);
    assert_eq!(inline_positions(&lines[0]), vec![0.0, 10.0, 20.0]);
    assert_eq!(inline_positions(&lines[1]), vec![0.0, 10.0]);
    // 列 0: x 帯 [390,400]・y 0..30。列 1: block 400−13=387 → x 帯 [377,387]。
    assert_eq!(
        lines[0].rect,
        LineRect {
            left: 390.0,
            top: 0.0,
            right: 400.0,
            bottom: 30.0
        }
    );
    assert_eq!(
        lines[1].rect,
        LineRect {
            left: 377.0,
            top: 0.0,
            right: 387.0,
            bottom: 20.0
        }
    );
}

// ── R6.3: vertical_lr——行内軸は rl と同一・行送りだけ +x（単一読み替え規則） ──

/// vertical_lr: 折返し・行内位置は vertical_rl と完全一致し、
/// 列送りの向きだけが +x へ反転する（軸読み替えの単一規則の檻）。
#[test]
fn vertical_lr_mirrors_rl_inline_layout_and_feeds_rightward() {
    let model = model((None, None), (None, Some(30)));
    let region_lr = TextRegion::resolve(&model, IMAGE, WritingMode::VerticalLr);
    // 書字開始角＝validrect 左上 (0, 0)。
    assert_eq!(region_lr.start(), (0.0, 0.0));
    let lines_lr = LayoutEngine::layout(
        &glyphs(5),
        5,
        &region_lr,
        WritingMode::VerticalLr,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines_lr.len(), 2);
    assert_eq!(
        lines_lr[0].rect,
        LineRect {
            left: 0.0,
            top: 0.0,
            right: 10.0,
            bottom: 30.0
        }
    );
    assert_eq!(
        lines_lr[1].rect,
        LineRect {
            left: 13.0,
            top: 0.0,
            right: 23.0,
            bottom: 20.0
        }
    );
    // 行内軸の配置は vertical_rl と同一（読み替え規則は 1 つ・分岐なし）。
    let region_rl = TextRegion::resolve(&model, IMAGE, WritingMode::VerticalRl);
    let lines_rl = LayoutEngine::layout(
        &glyphs(5),
        5,
        &region_rl,
        WritingMode::VerticalRl,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    for (lr, rl) in lines_lr.iter().zip(&lines_rl) {
        assert_eq!(inline_positions(lr), inline_positions(rl));
    }
}

// ── 改行マーカー（NewLine{ratio}）: 行送り量 = line_pitch × ratio（正準表） ──

/// 明示改行の ratio（1.0／0.5）が行送り量へ反映される
/// （font 12 → pitch 15: 行送り軸位置 0 → 15 → 22.5・端数檻）。
#[test]
fn explicit_line_break_ratio_scales_line_feed() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 0.5 },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 3);
    let tops: Vec<f32> = lines.iter().map(|l| l.rect.top).collect();
    assert_eq!(tops, vec![0.0, 15.0, 22.5]);
    assert!(lines.iter().all(|l| l.glyphs.len() == 1));
}

/// 縦書きでも改行マーカーは列送り（−x）として同じ規則で効く（正準表「同（列送り）」）。
#[test]
fn vertical_line_break_feeds_column_axis() {
    let region = TextRegion::resolve(
        &model((None, None), (None, None)),
        IMAGE,
        WritingMode::VerticalRl,
    );
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 0.5 },
        TextItem::Glyph { ch: 'あ' },
    ];
    let lines = LayoutEngine::layout(
        &items,
        2,
        &region,
        WritingMode::VerticalRl,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2);
    // pitch 15 × 0.5 = 7.5 だけ左へ: 列 1 の右辺 = 400 − 7.5。
    assert_eq!(lines[0].rect.right, 400.0);
    assert_eq!(lines[1].rect.right, 392.5);
}

// ── 縮退・境界 ──

/// 行頭の 1 グリフが閾値超過でも配置される（無限折返しなし・グリフを落とさない）。
#[test]
fn single_glyph_exceeding_threshold_is_placed_per_line() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (Some(3), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let lines = LayoutEngine::layout(
        &glyphs(2),
        2,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(
        lines.len(),
        2,
        "1 行 1 グリフで前進する（無限ループしない）"
    );
    assert_eq!(inline_positions(&lines[0]), vec![0.0]);
    assert_eq!(inline_positions(&lines[1]), vec![0.0]);
}

/// 空入力は空の行列（行なし）。可視 0 のグリフ列も行を生まない。
#[test]
fn empty_input_and_zero_visible_yield_no_lines() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let empty = LayoutEngine::layout(
        &[],
        0,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert!(empty.is_empty());
    let unrevealed = LayoutEngine::layout(
        &glyphs(3),
        0,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert!(unrevealed.is_empty());
}

/// 可視 prefix: visible_count 個のグリフだけが配置され、超過指定は全量へ飽和する。
#[test]
fn visible_count_gates_placed_glyphs() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items: Vec<TextItem> = "abcde".chars().map(|ch| TextItem::Glyph { ch }).collect();
    let partial = LayoutEngine::layout(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(partial.len(), 1);
    assert_eq!(inline_positions(&partial[0]), vec![0.0, 6.0, 12.0]);
    let saturated = LayoutEngine::layout(
        &items,
        99,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(saturated[0].glyphs.len(), 5);
}

/// 可視 prefix 内の改行マーカーは遅延（deferred newline）: その後ろに可視グリフが
/// 現れるまで行を開かず保留する（R4.2）。次の可視グリフが reveal された時点でのみ
/// 一括実体化する（R4.1）——保留中は空行を出さない（R1.2）。
#[test]
fn line_break_defers_until_next_visible_glyph() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'a' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::Glyph { ch: 'b' },
    ];
    // visible=1: b が未リビール——改行は保留のまま行を開かない（R4.2）。
    let held = LayoutEngine::layout(
        &items,
        1,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(held.len(), 1, "保留改行は行を開かない（空行を出さない）");
    assert_eq!(held[0].glyphs.len(), 1);
    assert_eq!(held[0].glyphs[0].ch, 'a');
    // visible=2: b がリビール——保留改行が実体化して 2 行になる（R4.1）。
    let materialized = LayoutEngine::layout(
        &items,
        2,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(materialized.len(), 2, "次可視グリフ配置で保留改行が実体化");
    assert_eq!(materialized[0].glyphs[0].ch, 'a');
    assert_eq!(materialized[1].glyphs[0].ch, 'b');
    // 実体化後の 2 行目: 行内 0 起点（b は ASCII で advance 6）・行送り軸位置は
    // pitch(15) 分進む・中間空行は生じない。
    assert_eq!(
        materialized[1].rect,
        LineRect {
            left: 0.0,
            top: 15.0,
            right: 6.0,
            bottom: 27.0
        }
    );
}

/// 末尾改行（後続の可視グリフを持たない）は保留のまま蒸発する——空行を開かない
/// （R1.1/1.2/5.2）。A→B 切替で A の末尾段落区切りが痕跡を残さない核心。
#[test]
fn trailing_line_break_defers_and_evaporates() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 1.0 },
    ];
    let lines = LayoutEngine::layout(
        &items,
        1,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 1, "末尾保留改行は蒸発＝空行を開かない");
    assert_eq!(lines[0].glyphs.len(), 1);
}

// ── 遅延意味論（deferred newline・newline-defer）の判断分岐（FixedMetrics 全網羅） ──

/// 3.1: 連続する複数の改行マーカーは単一の累算保留として実体化され、中間に空行が生じず
/// 行間が ratio 合計（`pitch × Σratio`）になる。実体化後の後続グリフは通常配置（pending
/// 消費済み）。`[a, \n, \n(0.5), b, c]` → 2 行（a／b c・間隔 pitch×1.5）。
#[test]
fn consecutive_newlines_accumulate_into_single_flush() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'a' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::LineBreak { ratio: 0.5 },
        TextItem::Glyph { ch: 'b' },
        TextItem::Glyph { ch: 'c' },
    ];
    // font 12 → pitch 15。累算 Σratio=1.5 → 行送り 22.5。
    let lines = LayoutEngine::layout(
        &items,
        3,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2, "連続改行は単一累算＝中間空行なし");
    let tops: Vec<f32> = lines.iter().map(|l| l.rect.top).collect();
    assert_eq!(tops, vec![0.0, 22.5], "行間 = pitch(15) × Σratio(1.5)");
    assert_eq!(lines[0].glyphs.len(), 1, "行 0 は a のみ");
    assert_eq!(
        lines[1].glyphs.len(),
        2,
        "行 1 は b c（pending 消費済みで通常配置）"
    );
}

/// 3.2: 先頭改行（可視グリフ配置前の保留）は空行を作らず block 前進のみで実体化する
/// （DD-2）・`ratio 0` の改行は「送りゼロで行を替える」縮退挙動を保つ（DD-5）・改行
/// マーカーのみの入力（可視グリフなし）は 0 行を返し内容ビューボックスを変化させない（1.5）。
#[test]
fn leading_newline_zero_ratio_and_newline_only_input() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // (a) 先頭改行 `[\n, a]` → 1 行・block 位置 start + pitch(15)・空行なし。
    let leading = [
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::Glyph { ch: 'a' },
    ];
    let lines = LayoutEngine::layout(
        &leading,
        1,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 1, "先頭改行は空行を作らない");
    assert_eq!(lines[0].rect.top, 15.0, "block 位置は start + pitch へ前進");
    assert_eq!(lines[0].glyphs.len(), 1);

    // (b) ratio 0 `[a, \n(0), b]` → 2 行・同一 block 位置（行替えのみ・送りゼロ）。
    let zero = [
        TextItem::Glyph { ch: 'a' },
        TextItem::LineBreak { ratio: 0.0 },
        TextItem::Glyph { ch: 'b' },
    ];
    let zlines = LayoutEngine::layout(
        &zero,
        2,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(zlines.len(), 2, "ratio 0 でも行を替える");
    let ztops: Vec<f32> = zlines.iter().map(|l| l.rect.top).collect();
    assert_eq!(ztops, vec![0.0, 0.0], "送りゼロ＝同一 block 位置");

    // (c) 改行のみ `[\n, \n]`（可視グリフなし）→ 0 行（ビューボックス不変の構造的証明）。
    let only = [
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::LineBreak { ratio: 1.0 },
    ];
    let empty = LayoutEngine::layout(
        &only,
        0,
        &region,
        WritingMode::HorizontalTb,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert!(
        empty.is_empty(),
        "改行のみの入力は 0 行（占有範囲を作らない）"
    );
}

/// 3.3: typewriter リビール進行との整合。同一 items で可視グリフ数を段階的に増やすと、
/// 改行マーカーの実体化はその改行より後ろの可視グリフが現れた時点でのみ起きる（R4.1/4.2）。
/// `[a, \n, b, \n, c]` の可視 1→2→3 で行数 1→2→3。
#[test]
fn reveal_progression_materializes_only_when_next_glyph_appears() {
    let region = TextRegion::resolve(
        &model((Some(0), Some(0)), (None, None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let items = [
        TextItem::Glyph { ch: 'a' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::Glyph { ch: 'b' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::Glyph { ch: 'c' },
    ];
    // 可視 v のとき行数 v（改行はその後ろのグリフが reveal された時点でのみ実体化）。
    for visible in 1..=3 {
        let lines = LayoutEngine::layout(
            &items,
            visible,
            &region,
            WritingMode::HorizontalTb,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(
            lines.len(),
            visible,
            "可視 {visible}: 改行の実体化は後続グリフの reveal 時のみ（保留は行を開かない）"
        );
        // 末尾行は必ず可視グリフを含む（空行を出さない Postcondition 1）。
        assert!(lines.iter().all(|l| !l.glyphs.is_empty()));
    }
}

/// 3.4: 満杯付近で保留改行が実体化された直後にあふれ判定が従来どおり発火する（3.2/7.3 後段）。
/// 満杯 3 行＋`\n`＋**次の可視グリフ**（実体化トリガあり）→ 4 行目が実体化しあふれ発火。
/// 保留のみ（次グリフなし）で不発火である対の片側は `trailing_pending_newline_does_not_
/// trigger_overflow`（既存更新檻）が担保する。
#[test]
fn materialized_newline_near_full_triggers_overflow() {
    let region = TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(36), Some(0), Some(400))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // 満杯 3 行（下端 10/23/36）＋改行＋次グリフ → 改行が実体化し 4 行目（下端 49 > 36）。
    let mut items = broken_lines(3);
    items.push(TextItem::LineBreak { ratio: 1.0 });
    items.push(TextItem::Glyph { ch: 'あ' });
    let window = window_for(&items, &region, WritingMode::HorizontalTb, 10.0);
    assert_eq!(
        window,
        VisibleWindow {
            first_visible_line: 1,
            block_offset: -13.0
        },
        "実体化後は従来どおりあふれ発火（次グリフが保留改行を実体化）"
    );
}

/// 3.5: 縦書き（vertical_rl／vertical_lr）でも横書きと同一の遅延・累算・実体化・蒸発規則が
/// 成立し、行送りが軸読み替え正準表（`block_dir × pitch × Σratio`）に従う（R6.1/6.2）。
#[test]
fn deferred_rules_hold_in_vertical_modes() {
    // (a) vertical_rl 累算実体化: `[あ, \n, \n(0.5), あ]` → 2 列・列送り −x で Σratio 1.5。
    let region_rl = TextRegion::resolve(
        &model((None, None), (None, None)),
        IMAGE,
        WritingMode::VerticalRl,
    );
    let acc = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 1.0 },
        TextItem::LineBreak { ratio: 0.5 },
        TextItem::Glyph { ch: 'あ' },
    ];
    // font 12 → pitch 15。書字開始角 x=400。列 1 の右辺 = 400 − 15×1.5 = 377.5。
    let lines = LayoutEngine::layout(
        &acc,
        2,
        &region_rl,
        WritingMode::VerticalRl,
        12.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2, "縦書きでも連続改行は単一累算（中間列なし）");
    assert_eq!(lines[0].rect.right, 400.0);
    assert_eq!(
        lines[1].rect.right, 377.5,
        "列送り = block_dir × pitch × Σratio"
    );

    // (b) 縦書き 2 方向で trailing 改行は蒸発する（`[あ, \n]` → 1 列）。
    for mode in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
        let region = TextRegion::resolve(&model((None, None), (None, None)), IMAGE, mode);
        let trailing = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 1.0 },
        ];
        let t = LayoutEngine::layout(
            &trailing,
            1,
            &region,
            mode,
            12.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(t.len(), 1, "{mode:?}: 末尾保留改行は蒸発（空列なし）");
    }
}

/// 3.6: 決定論（同一入力→同一出力）に、連続改行・末尾改行を含む新しい入力パターンを
/// 追加し、遅延意味論の全分岐が 3 方向で同一入力→同一出力を返すことを確認する（7.1）。
#[test]
fn deferred_semantics_same_input_yields_identical_output() {
    let model = model((Some(0), Some(0)), (Some(50), None));
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let region = TextRegion::resolve(&model, IMAGE, mode);
        // 連続改行＋末尾改行を含む列（遅延・累算・実体化・蒸発の全分岐を通す）。
        let items = [
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'a' },
            TextItem::LineBreak { ratio: 1.0 },
        ];
        let first = LayoutEngine::layout(
            &items,
            2,
            &region,
            mode,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        let second = LayoutEngine::layout(
            &items,
            2,
            &region,
            mode,
            10.0,
            &FixedMetrics,
            WrapPlan::CharByChar,
        );
        assert_eq!(
            first, second,
            "mode {mode:?} で遅延意味論の決定論が崩れている"
        );
    }
}

/// 折返し・改行後の行は描画開始点の行内成分へ戻る（origin が validrect 内部の場合も
/// 同一規則・行送り軸だけが進む）。
#[test]
fn wrapped_lines_return_to_start_inline_component() {
    let region = TextRegion::resolve(
        &model((Some(100), Some(50)), (Some(150), None)),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    assert_eq!(region.start(), (100.0, 50.0));
    // font 10: 100,110,120,130,140（5 個目終端 150＝閾値ちょうど）→ 6 個目で折返し。
    let lines = LayoutEngine::layout(
        &glyphs(6),
        6,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    assert_eq!(lines.len(), 2);
    assert_eq!(inline_positions(&lines[1]), vec![100.0]);
    assert_eq!(lines[1].rect.top, 63.0); // 50 + pitch 13
}
