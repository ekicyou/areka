use super::test_support::{IMAGE, broken_lines, model, model_rect, window_for};
use super::{FixedMetrics, LayoutEngine, VisibleWindow, WrapPlan};
use crate::region::TextRegion;
use crate::state::TextItem;
use crate::writing::WritingMode;

/// 領域内に収まる間はあふれ非発火（先頭可視行 0・オフセット 0）。
/// 最新行の下端が validrect.bottom とちょうど一致する境界は「超えていない」
/// （判定は `>`・境界檻）。
#[test]
fn horizontal_within_region_does_not_scroll() {
    // validrect top0/bottom34: 3 行の下端 10/22/34——3 行目はちょうど 34。
    // 境界は「3 行ちょうどが収まる」意図を保つため 36（旧 pitch 13 の 3 行目下端）から
    // 34（新 pitch 12 の 3 行目下端）へ導き直した。
    let region = TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(34), Some(0), Some(400))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let window = window_for(&broken_lines(3), &region, WritingMode::HorizontalTb, 10.0);
    assert_eq!(
        window,
        VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0
        }
    );
}

/// 横書きのあふれは縦スクロール（R7.2）: 1 行超過で先頭可視行が 1 行進み、
/// 内容は上（−y）へ pitch 分オフセットする。行単位＝オフセットは行位置差そのもの。
#[test]
fn horizontal_overflow_scrolls_vertically_by_whole_lines() {
    // 境界 34 は「3 行ちょうどが収まる」新格子の値（旧格子の 36 のままだと
    // 最小スキップ後の下端が境界ちょうどにならず、最小性の意図が消える）。
    let region = TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(34), Some(0), Some(400))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // 4 行目の下端 46 > 34 → 1 行スキップで 46-12=34 ≤ 34（最小スキップ檻）。
    let one_over = window_for(&broken_lines(4), &region, WritingMode::HorizontalTb, 10.0);
    assert_eq!(
        one_over,
        VisibleWindow {
            first_visible_line: 1,
            block_offset: -12.0
        }
    );
    // 6 行（最新行下端 70）→ 3 行スキップ（70-36=34）・オフセット −36。
    let three_over = window_for(&broken_lines(6), &region, WritingMode::HorizontalTb, 10.0);
    assert_eq!(
        three_over,
        VisibleWindow {
            first_visible_line: 3,
            block_offset: -36.0
        }
    );
}

/// vertical_rl のあふれは横スクロール（R7.2）: 最新列の左端 < validrect.left で発火し、
/// 内容は右（+x）へオフセットする（古い列が右端から消える——正準表）。
#[test]
fn vertical_rl_overflow_scrolls_content_rightward() {
    // validrect left360/right400。列の左端 390/378/366/354——4 列目 354 < 360。
    let region = TextRegion::resolve(
        &model_rect((None, None), (Some(0), Some(224), Some(360), Some(400))),
        IMAGE,
        WritingMode::VerticalRl,
    );
    assert_eq!(region.start(), (400.0, 0.0));
    let fits = window_for(&broken_lines(3), &region, WritingMode::VerticalRl, 10.0);
    assert_eq!(fits.first_visible_line, 0);
    assert_eq!(fits.block_offset, 0.0);
    let over = window_for(&broken_lines(4), &region, WritingMode::VerticalRl, 10.0);
    assert_eq!(
        over,
        VisibleWindow {
            first_visible_line: 1,
            block_offset: 12.0
        }
    );
}

/// vertical_lr のあふれは横スクロール: 最新列の右端 > validrect.right で発火し、
/// 内容は左（−x）へオフセットする（正準表）。
#[test]
fn vertical_lr_overflow_scrolls_content_leftward() {
    // validrect left0/right40。列の右端 10/22/34/46——4 列目 46 > 40。
    let region = TextRegion::resolve(
        &model_rect((None, None), (Some(0), Some(224), Some(0), Some(40))),
        IMAGE,
        WritingMode::VerticalLr,
    );
    let fits = window_for(&broken_lines(3), &region, WritingMode::VerticalLr, 10.0);
    assert_eq!(fits.first_visible_line, 0);
    assert_eq!(fits.block_offset, 0.0);
    let over = window_for(&broken_lines(4), &region, WritingMode::VerticalLr, 10.0);
    assert_eq!(
        over,
        VisibleWindow {
            first_visible_line: 1,
            block_offset: -12.0
        }
    );
}

/// 空の行列は既定窓（先頭 0・オフセット 0）——失敗経路なしの純関数。
#[test]
fn empty_lines_yield_default_window() {
    let region = TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(34), Some(0), Some(400))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let window = LayoutEngine::visible_window(&[], &region, WritingMode::HorizontalTb);
    assert_eq!(
        window,
        VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0
        }
    );
}

/// 全行超過（どこまでスキップしても最新行が収まらない）は最新行へ飽和する
/// （最新行は常に可視・行を失わない縮退規則）。
#[test]
fn all_lines_overflowing_saturates_to_newest_line() {
    // font 50 → pitch 52・行下端 50/102/154 は全て validrect.bottom 40 超過。
    let region = TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(40), Some(0), Some(400))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let window = window_for(&broken_lines(3), &region, WritingMode::HorizontalTb, 50.0);
    assert_eq!(
        window,
        VisibleWindow {
            first_visible_line: 2,
            block_offset: -104.0
        }
    );
    // 1 行だけで領域より厚い場合も先頭 0・オフセット 0（それ以上戻せない）。
    let single = window_for(&broken_lines(1), &region, WritingMode::HorizontalTb, 50.0);
    assert_eq!(
        single,
        VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0
        }
    );
}

/// ratio 付き改行の端数行送り（pitch 15 × 0.5 = 7.5）でもオフセットは
/// 実際の行位置差＝端数そのもの（整数量子化しない・端数檻）。
/// 新式 `pitch = font + 行間 2` では ratio 0.5 が端数を生むのは pitch が奇数のとき
/// ＝font が奇数のときなので、font 12（pitch 14・送り 7.0 で端数が消える）から
/// font 13（pitch 15・送り 7.5）へ導き直した——「端数」という意図を残すため。
#[test]
fn fractional_ratio_feed_scrolls_by_fractional_line_distance() {
    let region = TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(30), Some(0), Some(400))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // font 13 → pitch 15（13 + 行間 2）。ratio 0.5 区切り 4 行:
    // 上端 0/7.5/15/22.5・下端 13/20.5/28/35.5——最新行 35.5 > 30。
    let items = [
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 0.5 },
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 0.5 },
        TextItem::Glyph { ch: 'あ' },
        TextItem::LineBreak { ratio: 0.5 },
        TextItem::Glyph { ch: 'あ' },
    ];
    let window = window_for(&items, &region, WritingMode::HorizontalTb, 13.0);
    assert_eq!(
        window,
        VisibleWindow {
            first_visible_line: 1,
            block_offset: -7.5
        }
    );
}

/// 末尾の保留改行はあふれ判定に参加しない（内容ビューボックスを増やさない・
/// R3.1/5.3/7.3 前段）。満杯 3 行（ちょうど収まる）＋trailing `\n` → 保留のまま
/// 蒸発しあふれ不発火（`first_visible_line=0`）。新規檻 5（実体化後発火）と対を成す。
#[test]
fn trailing_pending_newline_does_not_trigger_overflow() {
    let region = TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(34), Some(0), Some(400))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    // 3 行（下端 10/22/34——ちょうど収まる）＋末尾改行は保留のまま蒸発＝空 4 行目を
    // 開かないためあふれ入力に現れない。
    let mut items = broken_lines(3);
    items.push(TextItem::LineBreak { ratio: 1.0 });
    let window = window_for(&items, &region, WritingMode::HorizontalTb, 10.0);
    assert_eq!(
        window,
        VisibleWindow {
            first_visible_line: 0,
            block_offset: 0.0
        },
        "保留改行はあふれ判定に不参加（スクロール不発火）"
    );
}

/// 同一入力に対する visible_window 出力は完全一致する（純関数・決定論檻・R7.5）。
#[test]
fn visible_window_same_input_yields_identical_output() {
    let region = TextRegion::resolve(
        &model_rect((Some(0), Some(0)), (Some(0), Some(34), Some(0), Some(400))),
        IMAGE,
        WritingMode::HorizontalTb,
    );
    let lines = LayoutEngine::layout(
        &broken_lines(5),
        5,
        &region,
        WritingMode::HorizontalTb,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    let first = LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb);
    let second = LayoutEngine::visible_window(&lines, &region, WritingMode::HorizontalTb);
    assert_eq!(first, second);
}

/// 同一入力に対する layout 出力は完全一致する（純関数・決定論檻）。
#[test]
fn same_input_yields_identical_output() {
    let model = model((Some(0), Some(0)), (Some(50), None));
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        let region = TextRegion::resolve(&model, IMAGE, mode);
        let items = [
            TextItem::Glyph { ch: 'あ' },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::Glyph { ch: 'a' },
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
        assert_eq!(first, second, "mode {mode:?} で決定論が崩れている");
    }
}
