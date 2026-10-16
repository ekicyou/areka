//! 選択肢の「済み」の境目の檻（areka-P0-balloon-text-bugs 件 1・要件 1.10〜1.13）。
//!
//! タスク 4.3 の部分は GPU なしで、場所の状態（`ActorTextState`）の境目と層の状態
//! （`TextLayerState`）の口だけを固定する: 境目の範囲・消去（`\c`・台詞の頭）で 0 に戻ること・
//! 済みにした後に足された選択肢は済んでいないこと・アンカーは済みにならないこと・
//! 場所が無ければ何もしないこと・空回しの写しが境目も写すこと。

use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

use crate::place::PlaceKey;
use crate::state::{ActorTextState, ChoiceSpan, SpanKind, TextLayerState};

fn cue(actor: &str, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

fn choice(actor: &str, id: &str) -> TalkCue {
    cue(
        actor,
        CueCommand::Choice {
            id: id.to_owned(),
            text: id.to_owned(),
            references: Vec::new(),
        },
    )
}

/// アンカーの範囲を 1 つ記録する cue の列（開き → 文字 → 閉じ）。
fn anchor(actor: &str, id: &str) -> [TalkCue; 3] {
    [
        cue(
            actor,
            CueCommand::AnchorBegin {
                id: id.to_owned(),
                references: Vec::new(),
            },
        ),
        cue(actor, CueCommand::Text(id.to_owned())),
        cue(actor, CueCommand::AnchorEnd),
    ]
}

fn balloon(actor: &str) -> PlaceKey {
    PlaceKey::balloon(&ActorKey::from(actor))
}

fn place<'a>(state: &'a TextLayerState, actor: &str) -> &'a ActorTextState {
    state
        .place_state(&balloon(actor))
        .expect("場所の状態がある")
}

/// 場所の範囲ごとに「済んだ選択肢か」を並べる（序数順）。
fn settled_flags(state: &TextLayerState, actor: &str) -> Vec<(SpanKind, bool)> {
    let s = place(state, actor);
    s.choices()
        .iter()
        .map(|span: &ChoiceSpan| (span.kind, s.is_settled_choice(span)))
        .collect()
}

fn applied(cues: &[TalkCue]) -> TextLayerState {
    let mut state = TextLayerState::default();
    for c in cues {
        state.apply_cue(c);
    }
    state
}

/// 境目の範囲: 済ませる前は全部が済んでおらず、済ませると今ある選択肢の全部が済む
/// （境目は範囲の数で止まる＝2 回済ませても同じ）。
#[test]
fn settle_moves_the_boundary_to_the_span_count_and_stops_there() {
    let mut state = applied(&[choice("0", "a"), choice("0", "b")]);
    assert_eq!(
        settled_flags(&state, "0"),
        vec![(SpanKind::Choice, false), (SpanKind::Choice, false)]
    );
    assert!(place(&state, "0").has_live_choice());

    assert!(state.settle_choices_at(&balloon("0")));
    assert_eq!(
        settled_flags(&state, "0"),
        vec![(SpanKind::Choice, true), (SpanKind::Choice, true)]
    );
    assert!(!place(&state, "0").has_live_choice());

    let once = state.clone();
    assert!(state.settle_choices_at(&balloon("0")));
    assert_eq!(state, once, "2 回目は何も変えない");
}

/// 範囲の無い場所: 済ませても何も起きず、済んでいない選択肢も無い。
#[test]
fn settle_on_a_place_without_spans_is_harmless() {
    let mut state = applied(&[cue("0", CueCommand::Text("字".into()))]);
    assert!(!place(&state, "0").has_live_choice());
    assert!(state.settle_choices_at(&balloon("0")));
    assert!(place(&state, "0").choices().is_empty());
    assert!(!place(&state, "0").has_live_choice());
}

/// 済みにした後に足された選択肢（序数が境目以上）は済んでいない。
#[test]
fn choices_added_after_settling_are_not_settled() {
    let mut state = applied(&[choice("0", "a")]);
    state.settle_choices_at(&balloon("0"));
    state.apply_cue(&choice("0", "b"));
    assert_eq!(
        settled_flags(&state, "0"),
        vec![(SpanKind::Choice, true), (SpanKind::Choice, false)]
    );
    assert!(place(&state, "0").has_live_choice());
}

/// アンカーは境目より前でも済みにならない。アンカーしか無い場所には済んでいない選択肢が無い。
#[test]
fn anchors_are_never_settled() {
    let mut cues = vec![choice("0", "a")];
    cues.extend(anchor("0", "link"));
    let mut state = applied(&cues);
    state.settle_choices_at(&balloon("0"));
    assert_eq!(
        settled_flags(&state, "0"),
        vec![(SpanKind::Choice, true), (SpanKind::Anchor, false)]
    );

    let only_anchor = applied(&anchor("1", "link"));
    assert!(!place(&only_anchor, "1").has_live_choice());
}

/// `\c`（`Clear`）で境目が 0 に戻る: 消した後の新しい選択肢は済んでいない。
#[test]
fn clear_resets_the_boundary() {
    let mut state = applied(&[choice("0", "a"), choice("0", "b")]);
    state.settle_choices_at(&balloon("0"));
    state.apply_cue(&cue("0", CueCommand::Clear));
    state.apply_cue(&choice("0", "c"));
    assert_eq!(settled_flags(&state, "0"), vec![(SpanKind::Choice, false)]);
    assert!(place(&state, "0").has_live_choice());
}

/// 台詞の頭（`ClearAll`）でも全部の場所の境目が 0 に戻る。
#[test]
fn clear_all_resets_the_boundary_of_every_place() {
    let mut state = applied(&[choice("0", "a"), choice("1", "b")]);
    state.settle_all_choices();
    state.apply_cue(&cue("0", CueCommand::ClearAll));
    state.apply_cue(&choice("0", "c"));
    state.apply_cue(&choice("1", "d"));
    assert_eq!(settled_flags(&state, "0"), vec![(SpanKind::Choice, false)]);
    assert_eq!(settled_flags(&state, "1"), vec![(SpanKind::Choice, false)]);
}

/// 場所を指す口はその場所だけを済ませ、場所が無ければ何もしない（偽を返し、場所も作らない）。
#[test]
fn settle_at_touches_only_that_place_and_ignores_missing_places() {
    let mut state = applied(&[choice("0", "a"), choice("1", "b")]);
    assert!(state.settle_choices_at(&balloon("0")));
    assert!(!place(&state, "0").has_live_choice());
    assert!(place(&state, "1").has_live_choice(), "ほかの場所は済まない");

    let before = state.clone();
    assert!(!state.settle_choices_at(&balloon("9")));
    assert_eq!(state, before, "無い場所は作らず何も変えない");
}

/// 全部の場所を済ませる口: どの場所にも済んでいない選択肢が残らない（アンカーは残る）。
#[test]
fn settle_all_settles_every_place() {
    let mut cues = vec![choice("0", "a"), choice("1", "b")];
    cues.extend(anchor("1", "link"));
    let mut state = applied(&cues);
    state.settle_all_choices();
    assert!(state.places().all(|(_, s)| !s.has_live_choice()));
    assert_eq!(
        settled_flags(&state, "1"),
        vec![(SpanKind::Choice, true), (SpanKind::Anchor, false)]
    );
}

/// 空回しの写しは境目も写す。
#[test]
fn rehearsal_copy_carries_the_boundary() {
    let mut state = applied(&[choice("0", "a")]);
    state.settle_choices_at(&balloon("0"));
    let copy = state.rehearsal_copy();
    assert!(!place(&copy, "0").has_live_choice());
    assert_eq!(settled_flags(&copy, "0"), vec![(SpanKind::Choice, true)]);
}
