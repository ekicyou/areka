//! 台本のバルーンの基準点の指令の預かりの檻。
//!
//! 表示の着地は待たず、同じスコープの先の指令が預かりに居るときだけその後ろに並ぶ。

use areka_emo_compose::ScaleRatio;
use areka_placement::script_balloon::{OffsetAxis, ScriptAlign, ScriptOffset};

use crate::move_cue::{PositionCue, parse_move_directive};

use super::run_move_drain_with;
use super::shared_test_support::{steady_harness, wiring};
use super::test_support::all_shown;

fn offset(scope: u32) -> PositionCue {
    PositionCue::SetBalloonOffset {
        scope,
        offset: ScriptOffset {
            x: OffsetAxis::Absolute(10),
            y: OffsetAxis::Relative(0),
        },
    }
}

fn align(scope: u32) -> PositionCue {
    PositionCue::SetBalloonAlign {
        scope,
        align: ScriptAlign::Left,
    }
}

/// スコープ 0 を、まだ表示していないスコープ 1 を基準に動かす（スコープ 0 の指令として預かられる）。
fn move_0_based_on_1() -> PositionCue {
    let tokens: Vec<String> = ["--X=0", "--base=1"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    PositionCue::Move(parse_move_directive(0, &tokens).directive)
}

fn move_1_to_global() -> PositionCue {
    let tokens: Vec<String> = ["--X=0", "--base=global"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    PositionCue::Move(parse_move_directive(1, &tokens).directive)
}

fn shown_only(scopes: &'static [u32]) -> impl Fn(u32) -> Option<ScaleRatio> {
    move |s| scopes.contains(&s).then_some(ScaleRatio::ONE)
}

#[test]
fn script_values_are_released_before_any_window_is_shown() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring();
    for cue in [offset(0), align(1)] {
        tx.send(cue).expect("送出は成功する");
    }

    run_move_drain_with(&wiring, &shown_only(&[]), &mut harness.world);

    assert!(wiring.move_held.borrow().is_empty(), "表示を待って預かった");
}

#[test]
fn a_script_value_is_not_held_behind_a_move_of_another_scope() {
    let mut harness = steady_harness();
    let (tx, wiring) = wiring();
    tx.send(move_1_to_global()).expect("送出は成功する");
    tx.send(offset(0)).expect("送出は成功する");

    run_move_drain_with(&wiring, &shown_only(&[]), &mut harness.world);

    assert_eq!(wiring.move_held.borrow().len(), 1, "移動だけが預かりに残る");
}

#[test]
fn a_script_value_does_not_overtake_a_held_move_of_the_same_scope() {
    for cue in [offset(0), align(0)] {
        let mut harness = steady_harness();
        let (tx, wiring) = wiring();
        tx.send(move_0_based_on_1()).expect("送出は成功する");
        tx.send(cue.clone()).expect("送出は成功する");

        run_move_drain_with(&wiring, &shown_only(&[0]), &mut harness.world);
        assert_eq!(
            wiring.move_held.borrow().len(),
            2,
            "{cue:?} が預かった移動を追い越した"
        );

        run_move_drain_with(&wiring, &all_shown, &mut harness.world);
        assert!(wiring.move_held.borrow().is_empty(), "預かりが残った");
    }
}

#[test]
fn a_balloon_reset_does_not_overtake_a_held_script_value() {
    for cue in [offset(0), align(0)] {
        let mut harness = steady_harness();
        let (tx, wiring) = wiring();
        tx.send(move_0_based_on_1()).expect("送出は成功する");
        tx.send(cue.clone()).expect("送出は成功する");
        tx.send(PositionCue::ResetBalloonPos)
            .expect("送出は成功する");

        run_move_drain_with(&wiring, &shown_only(&[0]), &mut harness.world);
        assert_eq!(
            wiring.move_held.borrow().len(),
            3,
            "初期化が預かった {cue:?} を追い越した"
        );

        run_move_drain_with(&wiring, &all_shown, &mut harness.world);
        assert!(wiring.move_held.borrow().is_empty(), "預かりが残った");
    }
}
