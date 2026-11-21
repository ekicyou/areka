//! 面の状態（`apply`・`apply_balloon`）の決定論テスト。

use super::test_support::*;
use super::*;

/// スコープ分離: あるスコープへの適用が他スコープへ波及しない。
#[test]
fn scope_isolation_does_not_touch_other_scope() {
    let mut states = fresh_states();
    let scope0 = ActorKey::from("0");
    let scope1 = ActorKey::from("1");

    let outcome = states.apply(&scope0, SurfaceTarget::Show(2100));
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope0.clone(),
            surface_id: 2100,
            binds: binds_1100_1207(),
            pattern: PatternState::default(),
        })
    );
    assert_eq!(states.scopes.get(&scope0), Some(&ScopeState::Shown(2100)));
    assert_eq!(states.scopes.get(&scope1), None, "他スコープは未設定のまま");

    let outcome1 = states.apply(&scope1, SurfaceTarget::Show(2200));
    assert_eq!(
        outcome1,
        ApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope1.clone(),
            surface_id: 2200,
            binds: binds_1100_1207(),
            pattern: PatternState::default(),
        })
    );
    assert_eq!(
        states.scopes.get(&scope0),
        Some(&ScopeState::Shown(2100)),
        "scope 0 は scope 1 の更新で変わらない"
    );
    assert_eq!(states.scopes.get(&scope1), Some(&ScopeState::Shown(2200)));
}

/// 非表示遷移: 表示中から Hide で Hidden へ遷移し Hide 指令を発行する。
#[test]
fn hide_transition_from_shown() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    assert!(matches!(
        states.apply(&scope, SurfaceTarget::Show(2100)),
        ApplyOutcome::Changed(_)
    ));

    let outcome = states.apply(&scope, SurfaceTarget::Hide);
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::Hide {
            scope: scope.clone(),
        })
    );
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));
}

/// 非表示保持・冪等: 既に Hidden な状態への Hide は Unchanged（再発行しない）。
#[test]
fn hide_when_already_hidden_is_unchanged() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    states.apply(&scope, SurfaceTarget::Show(2100));
    assert!(matches!(
        states.apply(&scope, SurfaceTarget::Hide),
        ApplyOutcome::Changed(_)
    ));

    let outcome = states.apply(&scope, SurfaceTarget::Hide);
    assert_eq!(outcome, ApplyOutcome::Unchanged);
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));
}

/// 冪等 Show: 同一 surface の再指定は 2 回目 Unchanged。
#[test]
fn show_same_surface_twice_second_is_unchanged() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    let first = states.apply(&scope, SurfaceTarget::Show(2100));
    assert_eq!(
        first,
        ApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2100,
            binds: binds_1100_1207(),
            pattern: PatternState::default(),
        })
    );

    let second = states.apply(&scope, SurfaceTarget::Show(2100));
    assert_eq!(second, ApplyOutcome::Unchanged);
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Shown(2100)));
}

/// 別 surface への切替は Changed（冪等ガードは同一 id 限定であることの反証）。
#[test]
fn show_different_surface_is_changed() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    states.apply(&scope, SurfaceTarget::Show(2100));
    let outcome = states.apply(&scope, SurfaceTarget::Show(2106));
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2106,
            binds: binds_1100_1207(),
            pattern: PatternState::default(),
        })
    );
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Shown(2106)));
}

/// 非表示→表示の復帰: Hidden から Show(id) で Shown(id) へ遷移し Show 指令。
#[test]
fn recovery_from_hidden_to_shown() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    states.apply(&scope, SurfaceTarget::Show(2100));
    states.apply(&scope, SurfaceTarget::Hide);
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));

    let outcome = states.apply(&scope, SurfaceTarget::Show(2106));
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2106,
            binds: binds_1100_1207(),
            pattern: PatternState::default(),
        })
    );
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Shown(2106)));
}

/// binds() アクセサ: 静的集合を返す（差し替えは `rebase_shell` だけ）。
#[test]
fn binds_accessor_returns_static_set() {
    let states = ScopeStates::new(crate::DefaultBinds::uniform(BindSet::from_ids([
        1100, 1207,
    ])));
    assert_eq!(
        states.binds(),
        &crate::DefaultBinds::uniform(BindSet::from_ids([1100, 1207]))
    );

    let empty = crate::test_support::empty_states();
    assert_eq!(
        empty.binds(),
        &crate::DefaultBinds::uniform(BindSet::from_ids([]))
    );
}

/// current_binds 既定フォールバック: 動的 bind 未適用の scope は静的既定集合を返す。
#[test]
fn current_binds_unbound_scope_returns_static_default() {
    let states = ScopeStates::new(crate::DefaultBinds::uniform(BindSet::from_ids([
        1100, 1207,
    ])));
    let scope = ActorKey::from("0");
    // 動的 bind を一切適用していない scope は `new` で渡した既定集合（初期値）を返す。
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207])
    );

    // 別の未束縛 scope も同じ既定集合（per-scope エントリ不在時のフォールバック）。
    let other = ActorKey::from("1");
    assert_eq!(
        states.current_binds(&other),
        &BindSet::from_ids([1100, 1207])
    );
}

/// Unresolved は no-op: 状態不変・発行なし・panic なし。
#[test]
fn unresolved_is_noop() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    let outcome = states.apply(&scope, SurfaceTarget::Unresolved);
    assert_eq!(outcome, ApplyOutcome::Unchanged);
    assert_eq!(states.scopes.get(&scope), None, "状態は生成されない");

    states.apply(&scope, SurfaceTarget::Show(2100));
    let outcome2 = states.apply(&scope, SurfaceTarget::Unresolved);
    assert_eq!(outcome2, ApplyOutcome::Unchanged);
    assert_eq!(
        states.scopes.get(&scope),
        Some(&ScopeState::Shown(2100)),
        "既存状態は保たれる"
    );
}

/// 未知 scope への Hide は「変化」とみなし Hide を 1 度発行、以後は保持。
#[test]
fn hide_on_unknown_scope_emits_once_then_holds() {
    let mut states = fresh_states();
    let scope = ActorKey::from("2");

    let first = states.apply(&scope, SurfaceTarget::Hide);
    assert_eq!(
        first,
        ApplyOutcome::Changed(DisplayCommand::Hide {
            scope: scope.clone(),
        })
    );
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));

    assert_eq!(
        states.apply(&scope, SurfaceTarget::Hide),
        ApplyOutcome::Unchanged
    );
}

// ---- apply_balloon（バルーン面・別 map 同居・apply() の鏡映） ----

/// 新規表示: 未知バルーン scope への Show(id) は Shown(id) へ更新し
/// `ShowBalloon { scope, surface_id, pattern }` を発行する（binds を**持たない**）。
#[test]
fn apply_balloon_new_show_emits_showballoon_without_binds() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    let outcome = states.apply_balloon(&scope, SurfaceTarget::Show(2));
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::ShowBalloon {
            scope: scope.clone(),
            surface_id: 2,
            pattern: PatternState::default(),
        })
    );
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Shown(2)));
}

/// 冪等な再表示: 同一面 id の 2 回目は Unchanged（再発行しない）。
#[test]
fn apply_balloon_show_same_surface_twice_second_is_unchanged() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    assert_eq!(
        states.apply_balloon(&scope, SurfaceTarget::Show(2)),
        ApplyOutcome::Changed(DisplayCommand::ShowBalloon {
            scope: scope.clone(),
            surface_id: 2,
            pattern: PatternState::default(),
        })
    );
    assert_eq!(
        states.apply_balloon(&scope, SurfaceTarget::Show(2)),
        ApplyOutcome::Unchanged
    );
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Shown(2)));
}

/// 別面への切替は Changed（冪等ガードが同一 id 限定であることの反証）。
#[test]
fn apply_balloon_show_different_surface_is_changed() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    states.apply_balloon(&scope, SurfaceTarget::Show(2));
    let outcome = states.apply_balloon(&scope, SurfaceTarget::Show(6));
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::ShowBalloon {
            scope: scope.clone(),
            surface_id: 6,
            pattern: PatternState::default(),
        })
    );
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Shown(6)));
}

/// 非表示: 表示中から Hide で Hidden へ遷移し `HideBalloon` を発行する。
#[test]
fn apply_balloon_hide_transition_from_shown() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    assert!(matches!(
        states.apply_balloon(&scope, SurfaceTarget::Show(2)),
        ApplyOutcome::Changed(_)
    ));

    let outcome = states.apply_balloon(&scope, SurfaceTarget::Hide);
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::HideBalloon {
            scope: scope.clone(),
        })
    );
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Hidden));
}

/// 冪等な再非表示: 既に Hidden な状態への Hide は Unchanged（再発行しない）。
#[test]
fn apply_balloon_hide_when_already_hidden_is_unchanged() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    states.apply_balloon(&scope, SurfaceTarget::Show(2));
    assert!(matches!(
        states.apply_balloon(&scope, SurfaceTarget::Hide),
        ApplyOutcome::Changed(_)
    ));

    assert_eq!(
        states.apply_balloon(&scope, SurfaceTarget::Hide),
        ApplyOutcome::Unchanged
    );
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Hidden));
}

/// 非表示→表示の復帰: Hidden から Show(id) で Shown(id) へ遷移し ShowBalloon。
#[test]
fn apply_balloon_recovery_from_hidden_to_shown() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    states.apply_balloon(&scope, SurfaceTarget::Show(2));
    states.apply_balloon(&scope, SurfaceTarget::Hide);
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Hidden));

    let outcome = states.apply_balloon(&scope, SurfaceTarget::Show(6));
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::ShowBalloon {
            scope: scope.clone(),
            surface_id: 6,
            pattern: PatternState::default(),
        })
    );
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Shown(6)));
}

/// 未知 scope への Hide は「変化」とみなし HideBalloon を 1 度発行、以後は保持。
#[test]
fn apply_balloon_hide_on_unknown_scope_emits_once_then_holds() {
    let mut states = fresh_states();
    let scope = ActorKey::from("3");

    let first = states.apply_balloon(&scope, SurfaceTarget::Hide);
    assert_eq!(
        first,
        ApplyOutcome::Changed(DisplayCommand::HideBalloon {
            scope: scope.clone(),
        })
    );
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Hidden));

    assert_eq!(
        states.apply_balloon(&scope, SurfaceTarget::Hide),
        ApplyOutcome::Unchanged
    );
}

/// Unresolved は no-op（防御・正規経路は actor が手前で skip）: 状態不変・発行なし。
#[test]
fn apply_balloon_unresolved_is_noop() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    let outcome = states.apply_balloon(&scope, SurfaceTarget::Unresolved);
    assert_eq!(outcome, ApplyOutcome::Unchanged);
    assert_eq!(states.balloon.get(&scope), None, "状態は生成されない");

    states.apply_balloon(&scope, SurfaceTarget::Show(2));
    let outcome2 = states.apply_balloon(&scope, SurfaceTarget::Unresolved);
    assert_eq!(outcome2, ApplyOutcome::Unchanged);
    assert_eq!(
        states.balloon.get(&scope),
        Some(&ScopeState::Shown(2)),
        "既存状態は保たれる"
    );
}

/// 相互独立: `apply_balloon` はシェル map（`scopes`）を一切触らない。
#[test]
fn apply_balloon_does_not_touch_shell_scopes() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    states.apply(&scope, SurfaceTarget::Show(2100));
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Shown(2100)));

    states.apply_balloon(&scope, SurfaceTarget::Show(2));
    assert_eq!(
        states.scopes.get(&scope),
        Some(&ScopeState::Shown(2100)),
        "apply_balloon 後もシェル面状態は不変（R4.6）"
    );
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Shown(2)));

    states.apply_balloon(&scope, SurfaceTarget::Hide);
    assert_eq!(
        states.scopes.get(&scope),
        Some(&ScopeState::Shown(2100)),
        "apply_balloon(Hide) 後もシェル面状態は不変（R4.6）"
    );
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Hidden));
}

/// 相互独立: `apply()`（シェル面）はバルーン map（`balloon`）を一切触らない。
#[test]
fn apply_shell_does_not_touch_balloon() {
    let mut states = fresh_states();
    let scope = ActorKey::from("0");

    states.apply_balloon(&scope, SurfaceTarget::Show(2));
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Shown(2)));

    states.apply(&scope, SurfaceTarget::Show(2100));
    assert_eq!(
        states.balloon.get(&scope),
        Some(&ScopeState::Shown(2)),
        "apply(Show) 後もバルーン面状態は不変（R4.6）"
    );
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Shown(2100)));

    states.apply(&scope, SurfaceTarget::Hide);
    assert_eq!(
        states.balloon.get(&scope),
        Some(&ScopeState::Shown(2)),
        "apply(Hide) 後もバルーン面状態は不変（R4.6）"
    );
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));
}
