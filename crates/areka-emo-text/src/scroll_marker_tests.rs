//! `PlaceScroll` の送りの規則の檻（design.md「純粋な判断（`scroll_marker_tests.rs`）」）。
//!
//! 自動の先頭行 `auto_first` は本来 `window_for` が毎フレーム覚え直す。ここでは私有の欄を
//! 直接置いて、送りの規則だけを固定する。

use super::*;

/// 自動の先頭行が `auto_first` の、手動に入っていない状態。
fn auto_at(auto_first: usize) -> PlaceScroll {
    PlaceScroll {
        auto_first,
        ..PlaceScroll::default()
    }
}

// ---- 矢印の送り（要件 5.1〜5.4） ----

#[test]
fn arrow_step_is_two_lines() {
    assert_eq!(ARROW_STEP_LINES, 2);
}

#[test]
fn older_from_auto_starts_manual_two_lines_back() {
    let mut s = auto_at(5);
    assert_eq!(
        s.step(ScrollDir::Older, ARROW_STEP_LINES),
        ScrollChange::Started
    );
    assert_eq!(s.manual, Some(3));
    assert_eq!(
        s.step(ScrollDir::Older, ARROW_STEP_LINES),
        ScrollChange::Moved
    );
    assert_eq!(s.manual, Some(1));
}

#[test]
fn older_overshoot_is_clamped_to_line_zero_then_stops() {
    let mut s = auto_at(3);
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Started);
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Moved);
    assert_eq!(s.manual, Some(0), "行き過ぎは行 0 に詰める");
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Unchanged);
    assert_eq!(s.manual, Some(0));
}

#[test]
fn older_from_auto_with_one_hidden_line_clamps_to_zero() {
    let mut s = auto_at(1);
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Started);
    assert_eq!(s.manual, Some(0));
}

#[test]
fn newer_reaching_auto_drops_manual() {
    let mut s = auto_at(5);
    s.step(ScrollDir::Older, 2); // 3
    s.step(ScrollDir::Older, 2); // 1
    assert_eq!(s.step(ScrollDir::Newer, 2), ScrollChange::Moved);
    assert_eq!(s.manual, Some(3));
    assert_eq!(
        s.step(ScrollDir::Newer, 2),
        ScrollChange::Stopped(StopReason::ReachedAuto),
        "3+2=5 は自動の先頭行に達する＝自動へ戻る"
    );
    assert_eq!(s.manual, None);
}

#[test]
fn newer_past_auto_also_drops_manual() {
    let mut s = auto_at(4);
    s.step(ScrollDir::Older, 1); // 3
    assert_eq!(
        s.step(ScrollDir::Newer, 2),
        ScrollChange::Stopped(StopReason::ReachedAuto)
    );
    assert_eq!(s.manual, None);
}

#[test]
fn newer_without_manual_does_nothing() {
    let mut s = auto_at(5);
    assert_eq!(s.step(ScrollDir::Newer, 2), ScrollChange::Unchanged);
    assert_eq!(s, auto_at(5));
}

#[test]
fn nothing_hidden_means_no_step() {
    let mut s = auto_at(0);
    assert!(!s.has_hidden_lines());
    assert_eq!(s.step(ScrollDir::Older, 2), ScrollChange::Unchanged);
    assert_eq!(s.step(ScrollDir::Newer, 2), ScrollChange::Unchanged);
    assert_eq!(s, auto_at(0));
}

#[test]
fn has_hidden_lines_when_pushed_out_or_manual() {
    assert!(auto_at(1).has_hidden_lines());
    let mut s = auto_at(2);
    s.step(ScrollDir::Older, 2);
    s.auto_first = 0; // 手動が在れば新しい側に隠れた行が在る
    assert!(s.has_hidden_lines());
}

// ---- ホイール（要件 6.3〜6.6） ----

#[test]
fn wheel_one_notch_with_lines_3_sends_three_lines() {
    let mut s = auto_at(10);
    assert_eq!(s.wheel(120, WheelLines::Lines(3), 4), ScrollChange::Started);
    assert_eq!(s.manual, Some(7));
    assert_eq!(
        s.wheel(-120, WheelLines::Lines(3), 4),
        ScrollChange::Stopped(StopReason::ReachedAuto)
    );
    assert_eq!(s.manual, None);
}

#[test]
fn wheel_one_notch_with_lines_1_sends_one_line() {
    let mut s = auto_at(10);
    assert_eq!(s.wheel(120, WheelLines::Lines(1), 4), ScrollChange::Started);
    assert_eq!(s.manual, Some(9));
}

#[test]
fn wheel_page_sends_visible_line_count() {
    let mut s = auto_at(10);
    assert_eq!(s.wheel(120, WheelLines::Page, 4), ScrollChange::Started);
    assert_eq!(s.manual, Some(6));
}

#[test]
fn wheel_two_notches_at_once_send_twice() {
    let mut s = auto_at(10);
    s.wheel(240, WheelLines::Lines(3), 4);
    assert_eq!(s.manual, Some(4));
}

#[test]
fn wheel_remainder_is_carried_while_sendable() {
    let mut s = auto_at(10);
    assert_eq!(
        s.wheel(60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(s.wheel_remainder, 60);
    assert_eq!(s.manual, None);
    assert_eq!(s.wheel(60, WheelLines::Lines(3), 4), ScrollChange::Started);
    assert_eq!(s.manual, Some(7));
    assert_eq!(s.wheel_remainder, 0);
    // 端数つきの回転: 180 = 1 目盛り＋60
    assert_eq!(s.wheel(180, WheelLines::Lines(3), 4), ScrollChange::Moved);
    assert_eq!(s.manual, Some(4));
    assert_eq!(s.wheel_remainder, 60);
    // 逆向きの回転は端数と打ち消し合う
    assert_eq!(
        s.wheel(-60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(s.wheel_remainder, 0);
}

#[test]
fn wheel_does_nothing_and_drops_remainder_without_hidden_lines() {
    let mut s = auto_at(0);
    assert_eq!(
        s.wheel(60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(s.wheel_remainder, 0, "送れない間は端数を持ち越さない");
    assert_eq!(
        s.wheel(120, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(s, auto_at(0));
}

#[test]
fn wheel_remainder_is_dropped_when_the_notch_cannot_send() {
    let mut s = auto_at(2);
    s.wheel(120, WheelLines::Lines(3), 4); // 行 0 に詰まる
    assert_eq!(s.manual, Some(0));
    assert_eq!(
        s.wheel(60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(
        s.wheel(60, WheelLines::Lines(3), 4),
        ScrollChange::Unchanged
    );
    assert_eq!(
        s.wheel_remainder, 0,
        "古い側の端で送れなかった目盛りの端数は捨てる"
    );
    assert_eq!(s.manual, Some(0));
}

#[test]
fn wheel_newer_clamps_to_auto_with_limits() {
    let mut s = auto_at(4);
    s.step(ScrollDir::Older, 2); // 2
    assert_eq!(
        s.wheel(-120, WheelLines::Lines(3), 4),
        ScrollChange::Stopped(StopReason::ReachedAuto),
        "行き過ぎても自動へ戻るだけ"
    );
    assert_eq!(s.manual, None);
}

// ---- 見える字の増減（要件 8.1） ----

#[test]
fn growing_visible_text_drops_manual_with_new_text() {
    let mut s = auto_at(5);
    assert_eq!(s.observe_visible(10), None, "手動でなければ理由は返さない");
    s.step(ScrollDir::Older, 2);
    assert_eq!(s.observe_visible(10), None, "同じ数なら何もしない");
    assert_eq!(s.manual, Some(3));
    assert_eq!(s.observe_visible(11), Some(StopReason::NewText));
    assert_eq!(s.manual, None);
    assert_eq!(s.last_visible, 11);
}

#[test]
fn shrinking_visible_text_only_remembers_the_count() {
    let mut s = auto_at(5);
    s.observe_visible(10);
    s.step(ScrollDir::Older, 2);
    assert_eq!(s.observe_visible(4), None);
    assert_eq!(s.manual, Some(3), "減ったときは手動を捨てない");
    assert_eq!(s.last_visible, 4);
    assert_eq!(
        s.observe_visible(5),
        Some(StopReason::NewText),
        "覚え直した数より増えたら捨てる"
    );
}

#[test]
fn forget_drops_manual_and_remainder() {
    let mut s = auto_at(10);
    s.wheel(180, WheelLines::Lines(3), 4);
    assert_eq!(
        s.forget(StopReason::Cleared),
        ScrollChange::Stopped(StopReason::Cleared)
    );
    assert_eq!(s.manual, None);
    assert_eq!(s.wheel_remainder, 0);
    assert_eq!(s.forget(StopReason::Cleared), ScrollChange::Unchanged);
}

// ---- 見える範囲・出る条件・置き場（task 3.2・要件 3.1〜3.4・3.7・4.1・4.2・7.1・7.2・7.7・8.5・8.7） ----

use crate::layout::{FixedMetrics, WrapPlan};
use crate::state::TextItem;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, MarkerCoord, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

/// layout の檻と同じ画像原寸。
const IMAGE: (u32, u32) = (400, 224);

/// 書字方向ごとの描画範囲（validrect: top,bottom,left,right）。どれも 3 行ちょうどが収まる。
fn region_of(mode: WritingMode) -> TextRegion {
    let rect = match mode {
        WritingMode::HorizontalTb => (Some(0), Some(34), Some(0), Some(400)),
        WritingMode::VerticalRl => (Some(0), Some(224), Some(360), Some(400)),
        WritingMode::VerticalLr => (Some(0), Some(224), Some(0), Some(34)),
    };
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(rect.0, rect.1, rect.2, rect.3),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    TextRegion::resolve(&model, IMAGE, mode)
}

/// n 行（各行 全角 1 字・明示改行で区切る）の item 列。`leading_break` で冒頭に保留改行を置く。
fn items_of(n: usize, leading_break: bool) -> Vec<TextItem> {
    let mut items = Vec::new();
    if leading_break {
        items.push(TextItem::LineBreak { ratio: 1.0 });
    }
    for i in 0..n {
        if i > 0 {
            items.push(TextItem::LineBreak { ratio: 1.0 });
        }
        items.push(TextItem::glyph("あ"));
    }
    items
}

/// n 行を `font_height` で並べる。
fn laid(n: usize, mode: WritingMode, region: &TextRegion, font_height: f32) -> Vec<PositionedLine> {
    LayoutEngine::layout(
        &items_of(n, false),
        n,
        region,
        mode,
        font_height,
        &FixedMetrics,
        WrapPlan::CharByChar,
    )
}

/// 先頭に見せる行の開始側（古い側の辺）が描画範囲の開始側に重なっていること。
fn assert_line_at_start(
    lines: &[PositionedLine],
    region: &TextRegion,
    mode: WritingMode,
    w: VisibleWindow,
) {
    let l = &lines[w.first_visible_line].rect;
    let start = region.start();
    let (edge, origin) = match mode {
        WritingMode::HorizontalTb => (l.top, start.1),
        WritingMode::VerticalRl => (l.right, start.0),
        WritingMode::VerticalLr => (l.left, start.0),
    };
    assert_eq!(
        edge + w.block_offset,
        origin,
        "{mode:?} の行 {} が描画範囲の始まりに来ない",
        w.first_visible_line
    );
}

const MODES: [WritingMode; 3] = [
    WritingMode::HorizontalTb,
    WritingMode::VerticalRl,
    WritingMode::VerticalLr,
];

#[test]
fn marker_kind_up_sends_older_down_sends_newer() {
    assert_eq!(MarkerKind::Up.dir(), ScrollDir::Older);
    assert_eq!(MarkerKind::Down.dir(), ScrollDir::Newer);
}

/// 出る条件は横書き・`vertical_rl`・`vertical_lr` とも同じ（要件 3.1〜3.3・4.1・7.2）。
#[test]
fn visibility_follows_hidden_lines_in_every_writing_mode() {
    for mode in MODES {
        let region = region_of(mode);
        let mut s = PlaceScroll::default();

        // 収まっている間は出さない。
        let fits = laid(3, mode, &region, 10.0);
        let (w, _) = window_for(&fits, &region, mode, &mut s);
        assert_eq!(
            marker_visibility(&w, &s),
            MarkerShown::default(),
            "{mode:?}"
        );

        // 自動で押し出されたら arrow0 だけ。
        let lines = laid(6, mode, &region, 10.0);
        s.observe_visible(6);
        let (w, _) = window_for(&lines, &region, mode, &mut s);
        assert_eq!(w.first_visible_line, 3, "{mode:?}");
        let only_up = MarkerShown {
            up: true,
            down: false,
        };
        assert_eq!(marker_visibility(&w, &s), only_up, "{mode:?}");

        // 戻している間は両方。
        s.step(ScrollDir::Older, ARROW_STEP_LINES);
        let (w, _) = window_for(&lines, &region, mode, &mut s);
        assert_eq!(w.first_visible_line, 1, "{mode:?}");
        assert_line_at_start(&lines, &region, mode, w);
        let both = MarkerShown {
            up: true,
            down: true,
        };
        assert_eq!(marker_visibility(&w, &s), both, "{mode:?}");

        // 一番古い端（行 0）では arrow1 だけ。
        s.step(ScrollDir::Older, ARROW_STEP_LINES);
        let (w, _) = window_for(&lines, &region, mode, &mut s);
        assert_eq!(w.first_visible_line, 0, "{mode:?}");
        assert_eq!(w.block_offset, 0.0, "{mode:?}");
        let only_down = MarkerShown {
            up: false,
            down: true,
        };
        assert_eq!(marker_visibility(&w, &s), only_down, "{mode:?}");
    }
}

/// 手動の見える範囲は自動と同じ式・同じ向き（横書き＝上へ負・`vertical_rl`＝右へ正・
/// `vertical_lr`＝左へ負）。行 1 を先頭に見せると 1 行ぶん（12 px）送られる。
#[test]
fn manual_window_uses_the_auto_formula_and_direction() {
    for (mode, expected) in [
        (WritingMode::HorizontalTb, -12.0),
        (WritingMode::VerticalRl, 12.0),
        (WritingMode::VerticalLr, -12.0),
    ] {
        let region = region_of(mode);
        let lines = laid(6, mode, &region, 10.0);
        let mut s = PlaceScroll::default();
        window_for(&lines, &region, mode, &mut s);
        s.step(ScrollDir::Older, ARROW_STEP_LINES);
        let (w, stop) = window_for(&lines, &region, mode, &mut s);
        assert_eq!(stop, None);
        assert_eq!(
            w,
            VisibleWindow {
                first_visible_line: 1,
                block_offset: expected
            },
            "{mode:?}"
        );
    }
}

/// 行 0 の手動の位置: 冒頭の保留改行の空きは見せず、一番古い行が描画範囲の始まりに来る。
#[test]
fn manual_line_zero_puts_oldest_line_at_region_start_even_after_leading_break() {
    let mode = WritingMode::HorizontalTb;
    let region = region_of(mode);
    let lines = LayoutEngine::layout(
        &items_of(5, true),
        5,
        &region,
        mode,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    let mut s = PlaceScroll::default();
    let (auto, _) = window_for(&lines, &region, mode, &mut s);
    assert!(auto.first_visible_line >= 1);
    s.step(ScrollDir::Older, usize::MAX);
    let (w, _) = window_for(&lines, &region, mode, &mut s);
    assert_eq!(w.first_visible_line, 0);
    assert_line_at_start(&lines, &region, mode, w);
}

/// 字が増えたそのフレームの見える範囲は自動の位置（1 フレーム遅れが無い）。
/// 順は observe_visible → window_for（逆だと 1 フレームだけ手動の位置で描く）。
#[test]
fn text_growth_frame_draws_the_auto_window_without_delay() {
    let mode = WritingMode::HorizontalTb;
    let region = region_of(mode);
    let six = laid(6, mode, &region, 10.0);
    let mut s = PlaceScroll::default();
    s.observe_visible(6);
    window_for(&six, &region, mode, &mut s);
    s.step(ScrollDir::Older, ARROW_STEP_LINES);
    assert_eq!(
        window_for(&six, &region, mode, &mut s).0.first_visible_line,
        1
    );

    // 字が増えたフレーム: 先に数を覚え、それから範囲を作る。
    let seven = laid(7, mode, &region, 10.0);
    let mut reversed = s.clone();
    assert_eq!(s.observe_visible(7), Some(StopReason::NewText));
    let (w, stop) = window_for(&seven, &region, mode, &mut s);
    assert_eq!(stop, None);
    assert_eq!(w, LayoutEngine::visible_window(&seven, &region, mode));
    let only_up = MarkerShown {
        up: true,
        down: false,
    };
    assert_eq!(marker_visibility(&w, &s), only_up);

    // 逆の順は同じフレームを手動の位置で描いてしまう（だから順を固定する）。
    let (late, _) = window_for(&seven, &region, mode, &mut reversed);
    assert_eq!(late.first_visible_line, 1);
}

/// 自動の先頭行が手動の位置まで下がったら手動を捨てる（ReachedAuto）。
#[test]
fn manual_caught_up_by_auto_is_dropped() {
    let mode = WritingMode::HorizontalTb;
    let region = region_of(mode);
    let mut s = PlaceScroll::default();
    s.observe_visible(6);
    window_for(&laid(6, mode, &region, 10.0), &region, mode, &mut s);
    s.step(ScrollDir::Older, ARROW_STEP_LINES);
    // 字が減った（例: 描画範囲の替わりで並べ直した）だけでは手動を捨てない。
    let fewer = laid(4, mode, &region, 10.0);
    assert_eq!(s.observe_visible(4), None);
    let (w, stop) = window_for(&fewer, &region, mode, &mut s);
    assert_eq!(stop, Some(StopReason::ReachedAuto));
    assert_eq!(w, LayoutEngine::visible_window(&fewer, &region, mode));
    assert_eq!(s.manual, None);
}

/// 拡大率の再追従（並べ直し）を挟んでも手動の位置は行の番号で残り、送りは並べ直した行から
/// 作る（要件 8.7）。
#[test]
fn manual_line_survives_relayout_after_scale_refollow() {
    let mode = WritingMode::HorizontalTb;
    let region = region_of(mode);
    let mut s = PlaceScroll::default();
    window_for(&laid(6, mode, &region, 10.0), &region, mode, &mut s);
    s.step(ScrollDir::Older, ARROW_STEP_LINES);
    let relaid = laid(6, mode, &region, 20.0);
    let (w, stop) = window_for(&relaid, &region, mode, &mut s);
    assert_eq!(stop, None);
    assert_eq!(w.first_visible_line, 1);
    assert_line_at_start(&relaid, &region, mode, w);
    assert_ne!(
        w.block_offset, -12.0,
        "送りは並べ直した行の位置から作り直す"
    );
}

// ---- 置き場（要件 3.4・3.7・4.2・7.1・7.7） ----

fn keys(arrow0: MarkerPoint, arrow1: MarkerPoint) -> ArrowKeys {
    ArrowKeys {
        arrow0,
        arrow1,
        ..ArrowKeys::default()
    }
}

fn pt(x: Option<MarkerCoord>, y: Option<MarkerCoord>) -> MarkerPoint {
    MarkerPoint { x, y }
}

#[test]
fn placement_with_coords_resolves_against_the_extent() {
    use MarkerCoord::{FromEnd, FromStart, Literal};
    // areka-balloon と同じ座標（arrow1.y,-20）を面の画像 326×169 で。
    let k = keys(
        pt(Some(FromStart(303)), Some(FromStart(10))),
        pt(Some(FromStart(303)), Some(FromEnd(-20))),
    );
    let expected = ResolvedMarkerPlacement {
        up: (303, 10),
        down: (303, 149),
    };
    assert_eq!(ResolvedMarkerPlacement::resolve(&k, (326, 169)), expected);
    // 箱は箱の大きさが広がり。`--数値` はそのまま負の座標。
    let k = keys(
        pt(Some(FromEnd(-12)), Some(Literal(-5))),
        pt(Some(FromEnd(-12)), Some(FromEnd(-8))),
    );
    let expected = ResolvedMarkerPlacement {
        up: (88, -5),
        down: (88, 42),
    };
    assert_eq!(ResolvedMarkerPlacement::resolve(&k, (100, 50)), expected);
}

#[test]
fn placement_without_coords_uses_the_ssp_measured_default() {
    // 座標なし（SSP 2.9.26 の 326×169 の測り）。
    let measured = ResolvedMarkerPlacement {
        up: (306, 15),
        down: (306, 154),
    };
    let none = ResolvedMarkerPlacement::resolve(&ArrowKeys::default(), (326, 169));
    assert_eq!(none, measured);
    // 片方だけの座標も「座標なし」。
    let k = keys(
        pt(Some(MarkerCoord::FromStart(1)), None),
        pt(None, Some(MarkerCoord::FromStart(1))),
    );
    assert_eq!(ResolvedMarkerPlacement::resolve(&k, (326, 169)), measured);
    // 箱も同じ決まり（箱の大きさ）。
    assert_eq!(default_marker_position(MarkerKind::Up, (100, 50)), (80, 15));
    assert_eq!(
        default_marker_position(MarkerKind::Down, (100, 50)),
        (80, 35)
    );
}
