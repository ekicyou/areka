use super::*;

use super::test_support::{build_target_assets, spawn_window_with_dpi};

#[test]
fn unregistered_target_is_not_requested() {
    let presenter = EmoPresenter::new();
    assert_eq!(
        presenter.never_shown(TargetId(7)),
        Some(NeverShown::NotRequested)
    );
}

#[test]
fn attached_target_without_surface_command_is_not_requested() {
    let mut world = World::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x10);
    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);

    assert_eq!(
        presenter.never_shown(TargetId(0)),
        Some(NeverShown::NotRequested)
    );
}

#[test]
fn requested_target_with_no_result_yet_is_none() {
    let mut world = World::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x10);
    let mut presenter = EmoPresenter::new();
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    presenter
        .targets
        .get_mut(&TargetId(0))
        .unwrap()
        .surface_requested = true;

    assert_eq!(presenter.never_shown(TargetId(0)), None);
}
