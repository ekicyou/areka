//! 印の合図（`\![*]`・`%*` の汎用の `\!` の合図・名前 `*`）を目印の字として積む檻。
//!
//! 読み手が `\![*]` を汎用の合図にする（タスク 7.2）までは台本を組み立てても合図が出ないので、
//! 合図の列を直接組む（7.2 が出す形＝`CueCommand::command_carrier("*", [])`）。

use areka_sakura::contract::TalkCue;

use super::test_support::{cue, items_of};
use super::*;
use crate::inline_marker::MARKER_GLYPH;
use crate::layout::{FixedMetrics, LayoutEngine, PositionedLine, VisibleWindow, WrapPlan};
use crate::region::TextRegion;
use crate::writing::WritingMode;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};

fn marker(actor: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::command_carrier("*", vec![]))
}

fn text(actor: &str, s: &str) -> TalkCue {
    cue(actor, 0.0, CueCommand::Text(s.into()))
}

fn applied(cues: &[TalkCue]) -> TextLayerState {
    let mut state = TextLayerState::default();
    for c in cues {
        state.apply_cue(c);
    }
    state
}

fn glyph_texts(state: &TextLayerState, actor: &str) -> Vec<String> {
    items_of(state, actor)
        .iter()
        .filter_map(|it| match it {
            TextItem::Glyph { text } => Some(text.to_string()),
            _ => None,
        })
        .collect()
}

fn spans(state: &TextLayerState, actor: &str) -> Vec<ChoiceSpan> {
    state
        .actor_state(&ActorKey::from(actor))
        .map(|s| s.choices().to_vec())
        .unwrap_or_default()
}

/// `\![*]\q[晴れてる,X1]`: 目印の字が 1 つ、選択肢の範囲はその後ろ、選択肢は 1 つ（要件 4.1・5.7）。
#[test]
fn marker_before_choice_is_one_glyph_and_the_choice_starts_after_it() {
    let state = applied(&[
        marker("0"),
        cue(
            "0",
            0.0,
            CueCommand::Choice {
                id: "X1".into(),
                text: "晴れてる".into(),
                references: Vec::new(),
            },
        ),
    ]);
    assert_eq!(
        glyph_texts(&state, "0"),
        [MARKER_GLYPH, "晴", "れ", "て", "る"]
    );
    let spans = spans(&state, "0");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].kind, SpanKind::Choice);
    assert_eq!(spans[0].glyph_range, 1..5);
    assert_eq!(spans[0].label, "晴れてる");
}

/// 続けた 2 つの印は 2 字（要件 4.5）。目印の字も本文の字と同じくリビールと見た目の番号を持つ。
#[test]
fn two_markers_are_two_glyphs_with_reveal_and_style() {
    let state = applied(&[marker("0"), marker("0"), text("0", "あ")]);
    assert_eq!(glyph_texts(&state, "0"), [MARKER_GLYPH, MARKER_GLYPH, "あ"]);
    let actor = state.actor_state(&ActorKey::from("0")).unwrap();
    assert_eq!(actor.reveal().times().len(), 3);
    assert_eq!(actor.glyph_styles().len(), 3);
}

/// `\c` で印も消える（要件 5.6）。
#[test]
fn clear_removes_the_marker() {
    let state = applied(&[marker("0"), cue("0", 0.0, CueCommand::Clear)]);
    assert!(glyph_texts(&state, "0").is_empty());
}

/// `\_a[ID]\![*]本文\_a`: 範囲は印を含む 3 字、題は「本文」だけ（U+FFFC を含まない）。
#[test]
fn anchor_range_covers_the_marker_but_the_title_does_not() {
    let state = applied(&[
        cue(
            "0",
            0.0,
            CueCommand::AnchorBegin {
                id: "ID".into(),
                references: Vec::new(),
            },
        ),
        marker("0"),
        text("0", "本文"),
        cue("0", 0.0, CueCommand::AnchorEnd),
    ]);
    let spans = spans(&state, "0");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].kind, SpanKind::Anchor);
    assert_eq!(spans[0].glyph_range, 0..3);
    assert_eq!(spans[0].label, "本文");
    assert!(!spans[0].label.contains(MARKER_GLYPH));
}

fn region(validrect: (i32, i32, i32, i32), mode: WritingMode) -> TextRegion {
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(
            Some(validrect.0),
            Some(validrect.1),
            Some(validrect.2),
            Some(validrect.3),
        ),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    TextRegion::resolve(&model, (400, 224), mode)
}

fn layout(state: &TextLayerState, region: &TextRegion, mode: WritingMode) -> Vec<PositionedLine> {
    let items = items_of(state, "0");
    let visible = glyph_texts(state, "0").len();
    LayoutEngine::layout(
        items,
        visible,
        region,
        mode,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    )
}

/// 印だけの 4 行目がスクロールの窓に入る（要件 5.6）。3 行ちょうどの領域（下端 34・
/// 行の送り 12）に「あ」3 行と印だけの行を足すと、1 行ぶん送られる。
#[test]
fn a_marker_only_line_scrolls_into_the_window() {
    let newline = || cue("0", 0.0, CueCommand::NewLine { ratio: 1.0 });
    let state = applied(&[
        text("0", "あ"),
        newline(),
        text("0", "あ"),
        newline(),
        text("0", "あ"),
        newline(),
        marker("0"),
    ]);
    let mode = WritingMode::HorizontalTb;
    let region = region((0, 34, 0, 400), mode);
    let lines = layout(&state, &region, mode);
    assert_eq!(lines.len(), 4);
    assert_eq!(&*lines[3].glyphs[0].text, MARKER_GLYPH);
    assert_eq!(
        LayoutEngine::visible_window(&lines, &region, mode),
        VisibleWindow {
            first_visible_line: 1,
            block_offset: -12.0
        }
    );
}

/// 縦書きでは印は列の向きに 1 字分（要件 5.5）。
#[test]
fn vertical_marker_takes_one_glyph_along_the_column() {
    let state = applied(&[marker("0"), text("0", "あ")]);
    let mode = WritingMode::VerticalRl;
    let region = region((0, 224, 0, 400), mode);
    let lines = layout(&state, &region, mode);
    assert_eq!(lines.len(), 1);
    let g = &lines[0].glyphs;
    assert_eq!(g.len(), 2);
    assert_eq!(g[1].inline_pos - g[0].inline_pos, 10.0);
}
