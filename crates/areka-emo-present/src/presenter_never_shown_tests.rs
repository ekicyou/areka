use super::*;

use std::time::Duration;

use super::test_support::{
    build_target_assets, force_applied, hide_ok, show_with, spawn_window_with_dpi,
};

/// `build_target_assets` の唯一の面。
const FACE: u32 = 1000;
/// `build_target_assets` に無い面。
const MISSING_FACE: u32 = 4242;

fn attached(world: &mut World, presenter: &mut EmoPresenter) -> TargetId {
    let window = spawn_window_with_dpi(world, 96);
    let (emo_world, atlas, _) = build_target_assets(3, 2, 0x10);
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
    TargetId(0)
}

fn field(presenter: &EmoPresenter, target: TargetId) -> Option<NeverShown> {
    presenter.targets[&target].never_shown
}

/// 表示の成立は GPU 無しでは作れないので、実適用 k を直に書いて「表示した」に代える。
fn mark_shown(presenter: &mut EmoPresenter, target: TargetId) {
    force_applied(
        presenter,
        target,
        Some(ScaleRatio::new(96, 96).expect("1/1")),
    );
}

fn replace_ok(
    presenter: &mut EmoPresenter,
    world: &mut World,
    target: TargetId,
    show: Option<u32>,
) {
    let (emo_world, atlas, _) = build_target_assets(3, 2, 0x20);
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ReplaceTarget {
            target,
            emo_world: Box::new(emo_world),
            atlas,
            author_dpi: 96,
            show: show.map(|id| (id, BindSet::default())),
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "ReplaceTarget が Ok でない"
    );
}

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

#[test]
fn hide_before_first_show_is_hidden() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    let target = attached(&mut world, &mut presenter);

    hide_ok(&mut presenter, &mut world, target);

    assert_eq!(presenter.never_shown(target), Some(NeverShown::Hidden));
}

#[test]
fn compose_failure_before_first_show_is_failed() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    let target = attached(&mut world, &mut presenter);

    let outcome = show_with(
        &mut presenter,
        &mut world,
        target,
        MISSING_FACE,
        BindSet::default(),
        PatternState::default(),
    );

    assert!(outcome.is_err(), "較正: 無い面の合成は失敗: {outcome:?}");
    assert_eq!(presenter.never_shown(target), Some(NeverShown::Failed));
}

#[test]
fn hide_after_show_is_none() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    let target = attached(&mut world, &mut presenter);
    mark_shown(&mut presenter, target);

    hide_ok(&mut presenter, &mut world, target);

    assert_eq!(field(&presenter, target), None);
    assert_eq!(presenter.never_shown(target), None);
}

#[test]
fn show_after_hidden_is_none() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    let target = attached(&mut world, &mut presenter);
    hide_ok(&mut presenter, &mut world, target);
    assert_eq!(field(&presenter, target), Some(NeverShown::Hidden), "較正");
    mark_shown(&mut presenter, target);

    let _ = show_with(
        &mut presenter,
        &mut world,
        target,
        FACE,
        BindSet::default(),
        PatternState::default(),
    );

    assert_eq!(field(&presenter, target), None);
    assert_eq!(presenter.never_shown(target), None);
}

#[test]
fn replace_without_show_before_surface_command_is_not_requested() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    let target = attached(&mut world, &mut presenter);

    replace_ok(&mut presenter, &mut world, target, None);

    assert_eq!(
        presenter.never_shown(target),
        Some(NeverShown::NotRequested)
    );
}

#[test]
fn replace_carries_the_reason() {
    let mut world = World::new();
    let mut presenter = EmoPresenter::new();
    let target = attached(&mut world, &mut presenter);
    hide_ok(&mut presenter, &mut world, target);

    replace_ok(&mut presenter, &mut world, target, None);
    assert_eq!(presenter.never_shown(target), Some(NeverShown::Hidden));

    replace_ok(&mut presenter, &mut world, target, Some(MISSING_FACE));
    assert_eq!(presenter.never_shown(target), Some(NeverShown::Failed));
}
