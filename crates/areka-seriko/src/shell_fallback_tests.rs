//! シェルの差し替えの戻り先の決定論テスト（spec: descript-window-keys 要件 3.2〜3.4・7.2）。
//!
//! 1 スコープ分の決め方 [`decide`] の 4 通りと、[`ScopeStates::fall_back_shell_surfaces`] が表示中の
//! スコープだけを書き換えること（非表示・未表示は替えない）を確かめる。

use super::*;
use crate::ScopeStates;
use crate::resolve::SurfaceTarget;
use areka_emo_compose::BindSet;

/// 新しいシェルの面の表（0・5）。descript の既定の面 5・組み込みの既定の面 0 を想定する。
fn ids(list: &[u32]) -> BTreeSet<u32> {
    list.iter().copied().collect()
}

fn states() -> ScopeStates {
    ScopeStates::new(crate::DefaultBinds::uniform(BindSet::default()))
}

#[test]
fn decide_keeps_current_surface_when_new_shell_has_it() {
    assert_eq!(decide(3, &[5, 0], &ids(&[0, 3, 5])), SurfaceKeep::Keep);
}

#[test]
fn decide_falls_back_to_descript_surface_first() {
    assert_eq!(decide(3, &[5, 0], &ids(&[0, 5])), SurfaceKeep::FallBack(5));
}

#[test]
fn decide_falls_back_to_builtin_surface_when_descript_surface_missing() {
    assert_eq!(decide(3, &[5, 0], &ids(&[0])), SurfaceKeep::FallBack(0));
}

#[test]
fn decide_hides_when_no_candidate_exists() {
    assert_eq!(decide(3, &[5, 0], &ids(&[7])), SurfaceKeep::Hide);
    assert_eq!(decide(3, &[], &ids(&[7])), SurfaceKeep::Hide, "候補なし");
}

/// 表示中のスコープは 4 通りの答えどおりに書き換わり、非表示・未表示のスコープは替わらない。
/// 結果は書き換えたスコープだけ（昇順）。
#[test]
fn fall_back_rewrites_only_shown_scopes() {
    let mut st = states();
    let (s0, s1, s2, s3, s4) = (
        ActorKey::from("0"),
        ActorKey::from("1"),
        ActorKey::from("2"),
        ActorKey::from("3"),
        ActorKey::from("4"),
    );
    st.apply(&s0, SurfaceTarget::Show(3)); // 在る → 替えない
    st.apply(&s1, SurfaceTarget::Show(13)); // descript の面 15 へ
    st.apply(&s2, SurfaceTarget::Show(23)); // descript の面 25 が無い → 組み込み 10 へ
    st.apply(&s3, SurfaceTarget::Show(33)); // どれも無い → 出さない
    st.apply(&s4, SurfaceTarget::Hide); // 非表示 → 替えない
    let fallback = ShellSurfaceFallback {
        surface_ids: ids(&[3, 10, 15]),
        candidates: BTreeMap::from([
            (s0.clone(), vec![0]),
            (s1.clone(), vec![15, 10]),
            (s2.clone(), vec![25, 10]),
            (s3.clone(), vec![35, 30]),
            (s4.clone(), vec![15]),
            // 未表示のスコープ（状態が無い）は候補があっても替えない。
            (ActorKey::from("5"), vec![15]),
        ]),
    };

    let outcomes = st.fall_back_shell_surfaces(&fallback);

    assert_eq!(
        outcomes,
        vec![
            ShellFallbackOutcome::FellBack {
                scope: s1.clone(),
                missing: 13,
                to: 15
            },
            ShellFallbackOutcome::FellBack {
                scope: s2.clone(),
                missing: 23,
                to: 10
            },
            ShellFallbackOutcome::Hidden {
                scope: s3.clone(),
                missing: 33,
                tried: vec![35, 30]
            },
        ]
    );
    assert_eq!(
        st.current_surfaces(crate::Slot::Shell),
        vec![
            (s0, Some(3)),
            (s1, Some(15)),
            (s2, Some(10)),
            (s3.clone(), None),
            (s4, None),
        ]
    );
    // 出さないスコープは `Hidden`（次の `\s[-1]` は冪等に扱われる）。
    assert_eq!(
        st.apply(&s3, SurfaceTarget::Hide),
        crate::ApplyOutcome::Unchanged
    );
}

/// 候補の表に載っていないスコープで今の面が無ければ、候補なしとして出さない。
#[test]
fn fall_back_hides_scope_without_candidate_list() {
    let mut st = states();
    let s0 = ActorKey::from("0");
    st.apply(&s0, SurfaceTarget::Show(3));
    let outcomes = st.fall_back_shell_surfaces(&ShellSurfaceFallback {
        surface_ids: ids(&[0]),
        candidates: BTreeMap::new(),
    });
    assert_eq!(
        outcomes,
        vec![ShellFallbackOutcome::Hidden {
            scope: s0,
            missing: 3,
            tried: vec![]
        }]
    );
}

/// 面の番号の集合が空の戻り先（材料なし・既定値）は何も替えない。
#[test]
fn empty_fallback_rewrites_nothing() {
    let mut st = states();
    let s0 = ActorKey::from("0");
    st.apply(&s0, SurfaceTarget::Show(3));
    assert!(
        st.fall_back_shell_surfaces(&ShellSurfaceFallback::default())
            .is_empty()
    );
    assert_eq!(st.current_surfaces(crate::Slot::Shell), vec![(s0, Some(3))]);
}
