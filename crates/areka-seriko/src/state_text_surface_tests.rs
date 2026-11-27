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

/// シェルの切替の戻り先: 在る番号は保ち、無い番号は候補の最初に在る番号へ、候補が無ければ非表示へ。
/// 非表示の控えは替えない。
#[test]
fn shell_fallback_rewrites_only_missing_kept_numbers() {
    let mut states = fresh_states();
    let keys = ["0", "1", "2", "3"].map(ActorKey::from);
    for (key, outcome) in keys.iter().zip([
        SurfaceKeyOutcome::Show(5),
        SurfaceKeyOutcome::Show(7),
        SurfaceKeyOutcome::Show(7),
        SurfaceKeyOutcome::Hide,
    ]) {
        states.note_text_surface(key, outcome);
    }
    states.fall_back_text_surfaces(&ShellSurfaceFallback {
        surface_ids: [0, 5].into(),
        candidates: [(keys[1].clone(), vec![9, 0])].into(),
    });
    let kept: Vec<_> = keys.iter().map(|k| states.text_surface(k)).collect();
    assert_eq!(
        kept,
        [
            Some(SurfaceKeyOutcome::Show(5)),
            Some(SurfaceKeyOutcome::Show(0)),
            Some(SurfaceKeyOutcome::Hide),
            Some(SurfaceKeyOutcome::Hide),
        ]
    );
}
