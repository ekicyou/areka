//! scope ごとの消去の回数（`TextLayerState::clear_count`）の数え方を確かめる
//! （要件 1.1・1.5・3.4・3.5）。
//!
//! `Clear` は名指しの scope だけを 1 進め、`ClearAll` は既にある全 scope を 1 ずつ進める。
//! 他の cue では動かない。時刻は cue に焼き込んだ値だけで、実時間を待たない。

use areka_sakura::contract::FONT_TAG_CARRIER;

use super::test_support::cue;
use super::*;

fn count(state: &TextLayerState, actor: &str) -> u64 {
    state.clear_count(&ActorKey::from(actor))
}

/// `Clear` は名指しの scope の回数だけを進める。状態の無い scope への `Clear` は 1 になる。
#[test]
fn clear_advances_only_the_named_scope() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
    state.apply_cue(&cue("1", 0.0, CueCommand::Text("い".into())));
    assert_eq!(count(&state, "0"), 0);

    state.apply_cue(&cue("0", 0.5, CueCommand::Clear));
    assert_eq!(count(&state, "0"), 1);
    assert_eq!(count(&state, "1"), 0);

    state.apply_cue(&cue("0", 0.6, CueCommand::Clear));
    assert_eq!(count(&state, "0"), 2);
    assert_eq!(count(&state, "1"), 0);

    // 状態の無い scope への `Clear` は、その scope の状態を作って回数を 1 にする。
    assert!(state.actor_state(&ActorKey::from("2")).is_none());
    state.apply_cue(&cue("2", 0.7, CueCommand::Clear));
    assert_eq!(count(&state, "2"), 1);
    assert_eq!(count(&state, "0"), 2);
    assert_eq!(count(&state, "1"), 0);
}

/// `ClearAll` は既にある全 scope を 1 ずつ進め、状態の無い scope は 0 のまま。
#[test]
fn clear_all_advances_every_existing_scope_and_leaves_stateless_at_zero() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
    state.apply_cue(&cue("1", 0.0, CueCommand::Text("い".into())));
    state.apply_cue(&cue("1", 0.1, CueCommand::Clear));

    state.apply_cue(&cue("0", 0.5, CueCommand::ClearAll));
    assert_eq!(count(&state, "0"), 1);
    assert_eq!(count(&state, "1"), 2);
    assert_eq!(count(&state, "2"), 0);
    assert!(state.actor_state(&ActorKey::from("2")).is_none());

    // 状態の無いところへの全消去でも、回数の無い scope は 0 のまま。
    let mut empty = TextLayerState::default();
    empty.apply_cue(&cue("0", 0.0, CueCommand::ClearAll));
    assert_eq!(count(&empty, "0"), 0);
}

/// `Text`・`NewLine`・`Choice`・`Cursor`・`Custom`・表示系の cue では回数が動かない。
#[test]
fn other_cues_do_not_move_the_count() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Clear));
    assert_eq!(count(&state, "0"), 1);

    let others = [
        CueCommand::Text("あいう".into()),
        CueCommand::NewLine { ratio: 1.0 },
        CueCommand::Choice {
            id: "yes".into(),
            text: "はい".into(),
            references: vec![],
        },
        CueCommand::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        CueCommand::command_carrier(FONT_TAG_CARRIER, vec!["bold".into(), "true".into()]),
        CueCommand::command_carrier("not-a-font-tag", vec![]),
        CueCommand::Wait,
    ];
    for (i, command) in others.into_iter().enumerate() {
        state.apply_cue(&cue("0", 0.1 * (i + 1) as f64, command));
        assert_eq!(count(&state, "0"), 1, "cue {i} が回数を動かした");
        assert_eq!(
            count(&state, "1"),
            0,
            "cue {i} が別の scope の回数を動かした"
        );
    }
}

/// 同じ cue の列からは同じ回数になる（決定論）。
#[test]
fn same_cue_sequence_yields_same_counts() {
    let sequence = vec![
        cue("0", 0.0, CueCommand::Text("あ".into())),
        cue("1", 0.1, CueCommand::Text("い".into())),
        cue("0", 0.2, CueCommand::Clear),
        cue("1", 0.3, CueCommand::ClearAll),
        cue("2", 0.4, CueCommand::Clear),
        cue("0", 0.5, CueCommand::Text("う".into())),
        cue("0", 0.6, CueCommand::ClearAll),
    ];
    let mut a = TextLayerState::default();
    let mut b = TextLayerState::default();
    for c in &sequence {
        a.apply_cue(c);
    }
    for c in &sequence {
        b.apply_cue(c);
    }
    for actor in ["0", "1", "2", "3"] {
        assert_eq!(count(&a, actor), count(&b, actor), "scope {actor}");
    }
    assert_eq!(
        [
            count(&a, "0"),
            count(&a, "1"),
            count(&a, "2"),
            count(&a, "3")
        ],
        [3, 2, 2, 0]
    );
}
