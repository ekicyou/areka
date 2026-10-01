//! 定義の差し替え（`rebase_shell`／`rebase_balloon`）の決定論テスト（要件 2.6・3.5）。

use super::test_support::*;
use super::*;
use crate::resolve::SurfaceTarget;
use areka_emo_compose::{ComposeMethod, PatternFrame};

/// animation `anim_id` に surface `surf` のコマ 1 枚を持つ pattern。
fn pattern_of(anim_id: u32, surf: u32) -> PatternState {
    let mut p = PatternState::default();
    p.set(
        anim_id,
        PatternFrame {
            surface_id: surf,
            method: ComposeMethod::Overlay,
            x: 0,
            y: 0,
        },
    );
    p
}

/// シェル面・バルーン面を表示し、動的な着せ替えと両 slot のパターンの進行を積んだ状態。
/// scope 0 は Shown(5)／バルーン Shown(2)、scope 1 は Hidden。
fn loaded_states() -> ScopeStates {
    let mut states = empty_states(); // static = {1100, 1207}
    let s0 = ActorKey::from("0");
    let s1 = ActorKey::from("1");
    states.apply(&s0, SurfaceTarget::Show(5));
    states.apply(&s1, SurfaceTarget::Hide);
    states.apply_balloon(&s0, SurfaceTarget::Show(2));
    states.apply_bind(&s0, 1302, true);
    states.commit_pattern(&s0, Slot::Shell, pattern_of(7, 1412));
    states.commit_pattern(&s0, Slot::Balloon, pattern_of(3, 40));
    states
}

/// シェルの差し替え: 今の面は保ち、静的な着せ替えを新しい既定へ替え、動的な着せ替えと
/// シェル側のパターンの進行を消す。バルーン側は触らない（要件 2.6）。
#[test]
fn rebase_shell_keeps_surfaces_and_resets_binds_and_shell_patterns() {
    let mut states = loaded_states();
    let s0 = ActorKey::from("0");
    let s1 = ActorKey::from("1");
    // 前提（空振りの反証）: 動的な着せ替えとシェルのパターンが積まれている。
    assert_eq!(
        states.current_binds(&s0),
        &BindSet::from_ids([1100, 1207, 1302])
    );
    assert!(!states.current_pattern(&s0, Slot::Shell).is_empty());

    let new_default = BindSet::from_ids([2001]);
    states.rebase_shell(new_default.clone());

    // 今の面は保たれる。
    assert_eq!(states.scopes.get(&s0), Some(&ScopeState::Shown(5)));
    assert_eq!(states.scopes.get(&s1), Some(&ScopeState::Hidden));
    assert_eq!(states.balloon.get(&s0), Some(&ScopeState::Shown(2)));
    // 着せ替えは新しい既定へ戻る（動的な着せ替えは消える）。
    assert_eq!(states.binds(), &new_default);
    assert_eq!(states.current_binds(&s0), &new_default);
    assert_eq!(states.current_binds(&s1), &new_default);
    // シェル側のパターンの進行は消え、バルーン側は残る。
    assert!(states.current_pattern(&s0, Slot::Shell).is_empty());
    assert_eq!(
        states.current_pattern(&s0, Slot::Balloon),
        &pattern_of(3, 40)
    );
}

/// バルーンの差し替え: バルーン側のパターンの進行だけを消す。面・着せ替え・シェル側の
/// パターンは不変（要件 3.5）。
#[test]
fn rebase_balloon_clears_only_balloon_patterns() {
    let mut states = loaded_states();
    let s0 = ActorKey::from("0");
    assert!(!states.current_pattern(&s0, Slot::Balloon).is_empty());

    states.rebase_balloon();

    assert_eq!(states.scopes.get(&s0), Some(&ScopeState::Shown(5)));
    assert_eq!(states.balloon.get(&s0), Some(&ScopeState::Shown(2)));
    assert_eq!(states.binds(), &binds_1100_1207());
    assert_eq!(
        states.current_binds(&s0),
        &BindSet::from_ids([1100, 1207, 1302])
    );
    assert_eq!(
        states.current_pattern(&s0, Slot::Shell),
        &pattern_of(7, 1412)
    );
    assert!(states.current_pattern(&s0, Slot::Balloon).is_empty());
}
