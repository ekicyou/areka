//! スコープごとに控える文字の層向けの `\s` の答えの決定論テスト。

use super::test_support::*;
use super::*;

#[test]
fn none_until_a_surface_arrives() {
    let states = fresh_states();
    assert_eq!(states.text_surface(&ActorKey::from("0")), None);
}

#[test]
fn unresolved_leaves_kept_value() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");
    states.note_text_surface(&scope, SurfaceKeyOutcome::Unresolved);
    assert_eq!(states.text_surface(&scope), None);
    states.note_text_surface(&scope, SurfaceKeyOutcome::Show(5));
    states.note_text_surface(&scope, SurfaceKeyOutcome::Unresolved);
    assert_eq!(
        states.text_surface(&scope),
        Some(SurfaceKeyOutcome::Show(5))
    );
}

#[test]
fn later_answer_overwrites_per_scope() {
    let mut states = fresh_states();
    let (s0, s1) = (ActorKey::from("0"), ActorKey::from("1"));
    states.note_text_surface(&s0, SurfaceKeyOutcome::Show(5));
    states.note_text_surface(&s0, SurfaceKeyOutcome::Hide);
    states.note_text_surface(&s1, SurfaceKeyOutcome::Show(7));
    assert_eq!(states.text_surface(&s0), Some(SurfaceKeyOutcome::Hide));
    assert_eq!(states.text_surface(&s1), Some(SurfaceKeyOutcome::Show(7)));
}
