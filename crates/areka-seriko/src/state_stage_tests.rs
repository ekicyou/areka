//! 窓の知らせとバルーンの面の番号の檻（spec: areka-P0-animated-image-playback 要件 6.1・6.3・
//! tasks.md 3.6）。
//!
//! バルーンの面の番号は、`\b[番号]` を受けていればその番号（`\b[-1]` なら面なし）、受けていなければ
//! 知らせの面。`stage_slots` と `commit_pattern` のバルーンの腕は同じ番号を使う。

use super::test_support::*;
use super::*;

fn note(scope: &ActorKey, open: bool, face: u32) -> StageNote {
    StageNote::Balloon {
        scope: scope.clone(),
        open,
        face,
        generation: 0,
    }
}

fn pat(id: u32, sid: u32) -> PatternState {
    let mut p = PatternState::default();
    p.set(
        id,
        areka_emo_compose::PatternFrame {
            surface_id: sid,
            method: areka_emo_compose::ComposeMethod::Overlay,
            x: 0,
            y: 0,
        },
    );
    p
}

fn balloon_slots(states: &ScopeStates) -> Vec<(ActorKey, u32, bool)> {
    states
        .stage_slots()
        .into_iter()
        .filter(|(_, slot, ..)| *slot == Slot::Balloon)
        .map(|(scope, _, id, open)| (scope, id, open))
        .collect()
}

/// `\b` を受けていないスコープは知らせの面を使う（進行の対象にも、`ShowBalloon` の面にも）。
#[test]
fn scope_without_balloon_cue_uses_the_noted_face() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    assert!(
        balloon_slots(&states).is_empty(),
        "知らせの前は面が分からない"
    );

    states.note_stage(&note(&scope, false, 3));
    assert_eq!(balloon_slots(&states), vec![(scope.clone(), 3, false)]);

    states.note_stage(&note(&scope, true, 4));
    assert_eq!(balloon_slots(&states), vec![(scope.clone(), 4, true)]);
    assert_eq!(
        states.commit_pattern(&scope, Slot::Balloon, pat(0, 9)),
        PatternApplyOutcome::Changed(DisplayCommand::ShowBalloon {
            scope: scope.clone(),
            surface_id: 4,
            pattern: pat(0, 9),
        }),
        "`commit_pattern` のバルーンの腕も知らせの面"
    );
}

/// `\b[番号]` を受けた後は、知らせの面を使わない（古い知らせで上書きしない）。窓の開け閉めは
/// 知らせに従う。
#[test]
fn balloon_cue_wins_over_the_noted_face() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.note_stage(&note(&scope, true, 3));
    states.apply_balloon(&scope, SurfaceTarget::Show(5));
    assert_eq!(balloon_slots(&states), vec![(scope.clone(), 5, true)]);

    states.note_stage(&note(&scope, false, 3));
    assert_eq!(
        balloon_slots(&states),
        vec![(scope.clone(), 5, false)],
        "面は `\\b` のまま・窓は知らせ"
    );
    assert_eq!(
        states.commit_pattern(&scope, Slot::Balloon, pat(0, 9)),
        PatternApplyOutcome::Changed(DisplayCommand::ShowBalloon {
            scope: scope.clone(),
            surface_id: 5,
            pattern: pat(0, 9),
        }),
        "窓が閉じていても指令は出す（出すか隠すかは表示層・要件 6.3）"
    );
}

/// `\b[-1]` を受けたら面なし（知らせが在っても進行の対象にならず、指令も出さない）。
#[test]
fn balloon_hide_cue_leaves_no_face() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.note_stage(&note(&scope, true, 3));
    states.apply_balloon(&scope, SurfaceTarget::Hide);
    assert!(balloon_slots(&states).is_empty());
    assert_eq!(
        states.commit_pattern(&scope, Slot::Balloon, pat(0, 9)),
        PatternApplyOutcome::Unchanged
    );
}

/// 知らせの無い流れ（`emo2`・今のテスト）では、`\b` で表示中の面を開いた窓として扱い、シェルは
/// `shown_slots` と同じ（見えている＝真）。並びはスコープの昇順 → シェル → バルーン。
#[test]
fn without_notes_stage_slots_match_shown_slots() {
    let mut states = empty_states();
    let (s0, s1) = (ActorKey::from("0"), ActorKey::from("1"));
    states.apply(&s1, SurfaceTarget::Show(10));
    states.apply_balloon(&s1, SurfaceTarget::Show(0));
    states.apply(&s0, SurfaceTarget::Show(0));
    states.apply_balloon(&s0, SurfaceTarget::Show(2));
    states.apply(&ActorKey::from("2"), SurfaceTarget::Hide);

    assert_eq!(
        states.stage_slots(),
        vec![
            (s0.clone(), Slot::Shell, 0, true),
            (s0, Slot::Balloon, 2, true),
            (s1.clone(), Slot::Shell, 10, true),
            (s1, Slot::Balloon, 0, true),
        ]
    );
}
