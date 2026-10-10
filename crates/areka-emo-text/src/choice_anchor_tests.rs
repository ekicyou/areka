//! 選択肢とアンカーが同じ列に並ぶときの当たりの檻（要件 2.1〜2.3・2.8・8.4）。
//!
//! 行の注釈（[`annotate_lines`]）と当たりの行（[`derive_hit_rows`]）は範囲の位置と通し番号しか
//! 読まないので、種類がアンカーでも選択肢と同じに働く。ここではそれを、合図（`AnchorBegin`／
//! `Text`／`Choice`／`AnchorEnd`）から組んだ範囲の列と、決まった字幅の配置（`FixedMetrics`）で
//! 固定する。範囲の記録そのもの（位置・文字・通し番号）の檻は `state_anchor_tests.rs`。
//!
//! 共通の配置: 横書き・字の丈 10（全角 1 字の送り幅も 10）・描画範囲は原点 (0,0)・幅 40
//! （全角 4 字で折り返す）。行送りは 12（丈 10＋行間 2）。当たりの帯は行ごとに丈 12・寄せ 1
//! （行ボックス 13.3 を行送り 12 で頭打ちにして中央へ寄せる）なので、ブロック軸は
//! 1 行目 1..13・2 行目 13..25・3 行目 25..37。

use super::*;
use crate::layout::{FixedMetrics, LayoutEngine, WrapPlan};
use crate::state::{SpanKind, TextLayerState};
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

const ACTOR: &str = "0";

/// 時刻 `at` に届き、`duration` 秒かけて表示される合図。
fn cue_at(at: f64, duration: f64, command: CueCommand) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(ACTOR),
        command,
        duration,
    }
}

fn open(id: &str) -> TalkCue {
    cue_at(
        0.0,
        0.0,
        CueCommand::AnchorBegin {
            id: id.to_owned(),
            references: Vec::new(),
        },
    )
}

fn close() -> TalkCue {
    cue_at(0.0, 0.0, CueCommand::AnchorEnd)
}

/// 届いた時点で全部の字が出る文字。
fn text(s: &str) -> TalkCue {
    cue_at(0.0, 0.0, CueCommand::Text(s.into()))
}

/// 届いた時点で全部の字が出る選択肢。
fn choice(id: &str, s: &str) -> TalkCue {
    cue_at(
        0.0,
        0.0,
        CueCommand::Choice {
            id: id.to_owned(),
            text: s.to_owned(),
            references: Vec::new(),
        },
    )
}

fn applied(cues: &[TalkCue]) -> TextLayerState {
    let mut state = TextLayerState::default();
    for c in cues {
        state.apply_cue(c);
    }
    state
}

/// 普通のバルーンの場所の範囲の列（種類と位置だけ）。
fn kinds_and_ranges(state: &TextLayerState) -> Vec<(SpanKind, core::ops::Range<usize>)> {
    state
        .actor_state(&ActorKey::from(ACTOR))
        .expect("場所の状態")
        .choices()
        .iter()
        .map(|s| (s.kind, s.glyph_range.clone()))
        .collect()
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> LineRect {
    LineRect {
        left,
        top,
        right,
        bottom,
    }
}

/// 時刻 `t` に出ている字までを配置し、行の注釈と当たりの行を出す（提示と同じ順: 配置 → 注釈 →
/// 帯 → 当たりの行）。当たりの行には、通し番号で引いた元の範囲の種類を添える。
fn hit(
    state: &TextLayerState,
    t: f64,
) -> (Vec<LineChoiceSegment>, Vec<(SpanKind, usize, LineRect)>) {
    let actor = ActorKey::from(ACTOR);
    let place = state.actor_state(&actor).expect("場所の状態");
    let mode = WritingMode::HorizontalTb;
    let model = BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(Some(0), Some(200), Some(0), Some(40)),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    );
    let region = TextRegion::resolve(&model, (400, 224), mode);
    let lines = LayoutEngine::layout(
        place.items(),
        state.visible_glyphs(&actor, t),
        &region,
        mode,
        10.0,
        &FixedMetrics,
        WrapPlan::CharByChar,
    );
    let spans = place.choices();
    let segments = annotate_lines(&lines, spans);
    let bands = line_bands(&lines, mode, &FixedMetrics);
    let rows = derive_hit_rows(&lines, &segments, mode, &region, &bands)
        .iter()
        // 通し番号は列の添字に等しい（崩れていればここで落ちる）。
        .map(|row| (spans[row.ordinal].kind, row.ordinal, row.rect))
        .collect();
    (segments, rows)
}

/// 折り返しをまたぐアンカーの範囲は、またいだ行ごとに当たりの区間を 1 つずつ持つ（要件 2.1・2.3）。
/// 範囲の外の字（前の「あい」・後ろの「き」）は当たりに入らない。
#[test]
fn anchor_across_a_wrap_has_a_hit_segment_on_each_line() {
    // 「あい」＋アンカー「うえおか」＋「き」: 1 行目＝あいうえ・2 行目＝おかき。
    let state = applied(&[
        text("あい"),
        open("x"),
        text("うえおか"),
        close(),
        text("き"),
    ]);
    let (segments, rows) = hit(&state, 0.0);
    assert_eq!(
        segments,
        vec![
            LineChoiceSegment {
                line_index: 0,
                ordinal: 0,
                inline_range: (20.0, 40.0), // 1 行目の「うえ」
            },
            LineChoiceSegment {
                line_index: 1,
                ordinal: 0,
                inline_range: (0.0, 20.0), // 2 行目の「おか」
            },
        ]
    );
    assert_eq!(
        rows,
        vec![
            (SpanKind::Anchor, 0, rect(20.0, 1.0, 40.0, 13.0)),
            (SpanKind::Anchor, 0, rect(0.0, 13.0, 20.0, 25.0)),
        ]
    );
}

/// 1 字ずつ出ている途中の当たりは、出た字までで打ち切られる（要件 2.2）。
/// 範囲の記録は最初から全文ぶんで、切るのは配置の側。
#[test]
fn hit_area_stops_at_what_has_been_revealed() {
    // アンカーを開いて 6 字を 1 秒に 1 字（字 i は時刻 i に出る）。話している最中なので、
    // 閉じはまだ届いていない。
    let mut state = applied(&[
        open("x"),
        cue_at(0.0, 6.0, CueCommand::Text("あいうえおか".into())),
    ]);
    assert_eq!(kinds_and_ranges(&state), vec![(SpanKind::Anchor, 0..6)]);
    // 出た字の数 → 当たりの行（1 行目は 4 字まで・5 字目から 2 行目）。
    let line0 = |n: f32| (SpanKind::Anchor, 0, rect(0.0, 1.0, 10.0 * n, 13.0));
    let line1 = |n: f32| (SpanKind::Anchor, 0, rect(0.0, 13.0, 10.0 * n, 25.0));
    let expected = [
        vec![],
        vec![line0(1.0)],
        vec![line0(2.0)],
        vec![line0(3.0)],
        vec![line0(4.0)],
        vec![line0(4.0), line1(1.0)],
        vec![line0(4.0), line1(2.0)],
    ];
    for (shown, want) in expected.iter().enumerate() {
        // 字 i は時刻 i に出るので、時刻 shown − 0.5 には shown 字が出ている。
        let (_, rows) = hit(&state, shown as f64 - 0.5);
        assert_eq!(&rows, want, "出た字 {shown}");
    }
    // 全部出た後に閉じが届いても、当たりは変わらない。
    state.apply_cue(&close());
    assert_eq!(hit(&state, 6.0).1, expected[6]);
}

/// 選択肢とアンカーが混ざる列でも、当たりの矩形と種類は範囲ごとに合う（要件 2.8・8.4）。
#[test]
fn mixed_choices_and_anchors_keep_their_rects_and_kinds() {
    // 選択肢「はい」＋「か」＋アンカー「あい」＋選択肢「いいえ」:
    // 1 行目＝はいかあ・2 行目＝いいいえ。アンカーだけが折り返しをまたぐ。
    let state = applied(&[
        choice("yes", "はい"),
        text("か"),
        open("x"),
        text("あい"),
        close(),
        choice("no", "いいえ"),
    ]);
    assert_eq!(
        hit(&state, 0.0).1,
        vec![
            (SpanKind::Choice, 0, rect(0.0, 1.0, 20.0, 13.0)),
            (SpanKind::Anchor, 1, rect(30.0, 1.0, 40.0, 13.0)),
            (SpanKind::Anchor, 1, rect(0.0, 13.0, 10.0, 25.0)),
            (SpanKind::Choice, 2, rect(10.0, 13.0, 40.0, 25.0)),
        ]
    );
}

/// 同じ種類の範囲は重ならない。アンカーは選択肢を包んでよく、そのときは両方が当たりの行を持つ
/// （どちらを採るかは押下の側が決める）。
#[test]
fn same_kind_ranges_are_disjoint_and_an_anchor_may_wrap_a_choice() {
    // アンカー a「あ＋選択肢『はい』＋い」＋選択肢「いいえ」＋アンカー b「うえ」:
    // 1 行目＝あはいい・2 行目＝いいえう・3 行目＝え。
    let state = applied(&[
        open("a"),
        text("あ"),
        choice("yes", "はい"),
        text("い"),
        close(),
        choice("no", "いいえ"),
        open("b"),
        text("うえ"),
        close(),
    ]);
    let spans = kinds_and_ranges(&state);
    // アンカー a（0..4）が選択肢「はい」（1..3）を包む。
    assert_eq!(
        spans,
        vec![
            (SpanKind::Anchor, 0..4),
            (SpanKind::Choice, 1..3),
            (SpanKind::Choice, 4..7),
            (SpanKind::Anchor, 7..9),
        ]
    );
    for (i, (kind, range)) in spans.iter().enumerate() {
        for (later_kind, later) in &spans[i + 1..] {
            assert!(
                kind != later_kind || range.end <= later.start,
                "同じ種類の範囲が重なっている: {range:?} と {later:?}"
            );
        }
    }
    assert_eq!(
        hit(&state, 0.0).1,
        vec![
            (SpanKind::Anchor, 0, rect(0.0, 1.0, 40.0, 13.0)),
            (SpanKind::Choice, 1, rect(10.0, 1.0, 30.0, 13.0)), // アンカー a の行の中
            (SpanKind::Choice, 2, rect(0.0, 13.0, 30.0, 25.0)),
            (SpanKind::Anchor, 3, rect(30.0, 13.0, 40.0, 25.0)),
            (SpanKind::Anchor, 3, rect(0.0, 25.0, 10.0, 37.0)),
        ]
    );
}
