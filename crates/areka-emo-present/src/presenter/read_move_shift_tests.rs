//! ずれの読み口 `move_shift` と合図 `MoveShift` の決定論テスト（spec `seriko-pattern-calls`
//! タスク 4.2・要件 7.1・7.6）。ずれが対象ごとに覚えられ、合成の覚えに触らないことを固定する。

use bevy_ecs::world::World;

use crate::command::{PresentCommand, TargetId};
use crate::presenter::EmoPresenter;
use crate::presenter::test_support::{
    build_target_assets, make_world_with_gpu, show_ok, spawn_window_with_dpi,
};

fn shift(presenter: &mut EmoPresenter, world: &mut World, target: u32, x: i64, y: i64) {
    presenter.apply(
        world,
        PresentCommand::MoveShift {
            target: TargetId(target),
            x,
            y,
        },
    );
}

/// 装着だけした対象 2 つの presenter（GPU なし・表示はしない）。
fn two_targets(world: &mut World) -> EmoPresenter {
    let mut presenter = EmoPresenter::new();
    for (id, salt) in [(0u32, 0x41u8), (1, 0x42)] {
        let window = spawn_window_with_dpi(world, 96);
        let (emo_world, atlas, _golden) = build_target_assets(3, 2, salt);
        presenter
            .attach_target(world, TargetId(id), window, emo_world, atlas, 96)
            .expect("attach_target 失敗");
    }
    presenter
}

/// 未登録の対象と、まだ合図を受けていない対象は (0, 0)。
#[test]
fn move_shift_is_zero_before_any_shift() {
    let mut world = World::new();
    let presenter = two_targets(&mut world);
    assert_eq!(presenter.move_shift(TargetId(0)), (0, 0));
    assert_eq!(presenter.move_shift(TargetId(1)), (0, 0));
    assert_eq!(presenter.move_shift(TargetId(9)), (0, 0), "未登録は (0, 0)");
}

/// ずれは対象ごとに覚えられ、同じ対象への後の合図が勝つ。0 で元へ戻る。
#[test]
fn move_shift_is_remembered_per_target() {
    let mut world = World::new();
    let mut presenter = two_targets(&mut world);

    shift(&mut presenter, &mut world, 0, 3, -4);
    shift(&mut presenter, &mut world, 1, 0, 9999);
    assert_eq!(presenter.move_shift(TargetId(0)), (3, -4));
    assert_eq!(presenter.move_shift(TargetId(1)), (0, 9999));

    shift(&mut presenter, &mut world, 0, -7, 2);
    assert_eq!(presenter.move_shift(TargetId(0)), (-7, 2), "後の合図が勝つ");
    assert_eq!(
        presenter.move_shift(TargetId(1)),
        (0, 9999),
        "他の対象は変わらない"
    );

    shift(&mut presenter, &mut world, 1, 0, 0);
    assert_eq!(presenter.move_shift(TargetId(1)), (0, 0), "0 で元へ戻る");
}

/// 未装着の対象への合図は捨てる（登録を生やさない・他の対象に書かない）。
#[test]
fn move_shift_to_unattached_target_is_dropped() {
    let mut world = World::new();
    let mut presenter = two_targets(&mut world);

    shift(&mut presenter, &mut world, 9, 5, 5);
    assert_eq!(presenter.move_shift(TargetId(9)), (0, 0));
    assert_eq!(
        presenter.target_visible(TargetId(9)),
        None,
        "登録を生やさない"
    );
    assert_eq!(presenter.move_shift(TargetId(0)), (0, 0));
    assert_eq!(presenter.move_shift(TargetId(1)), (0, 0));
}

/// 表示中の対象へ合図を送っても、合成の覚え・表示中の面・可視・適用済みの k は変わらない。
#[test]
fn move_shift_leaves_composition_memory_untouched() {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x43);
    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let before = (
        presenter
            .read_back(TargetId(0))
            .expect("表示直後の読み戻し"),
        presenter.current_surface_id(TargetId(0)),
        presenter.target_visible(TargetId(0)),
        presenter.applied_ratio(TargetId(0)),
        presenter.target_physical_size(TargetId(0)),
    );

    shift(&mut presenter, &mut world, 0, 12, -34);

    assert_eq!(presenter.move_shift(TargetId(0)), (12, -34));
    let after = (
        presenter
            .read_back(TargetId(0))
            .expect("合図の後も覚えに残る"),
        presenter.current_surface_id(TargetId(0)),
        presenter.target_visible(TargetId(0)),
        presenter.applied_ratio(TargetId(0)),
        presenter.target_physical_size(TargetId(0)),
    );
    assert_eq!(before, after, "合成の覚え・面・可視・k は変わらない");
}
