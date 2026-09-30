//! 装着の置き換え（`presenter/replace.rs`）の檻（GPU・要件 4.3・4.6・11.9）。
//!
//! | 檻 | 固定する性質 |
//! |---|---|
//! | 登録を消す口 | 表から外れ・装着の子 2 つが消え・引き継ぐ値（窓・可視性の持ち主・適用済みの k・素の大きさ・保留中の窓寸の要求）がそのまま返る |
//! | 未登録 | `error!` 1 件と `Err(TargetNotAttached)`・他の登録と窓の子は不変 |

use super::*;

use bevy_ecs::hierarchy::Children;

use super::test_support::{
    build_target_assets, capture, make_world_with_gpu, mount_entities, show_ok,
    spawn_window_with_dpi,
};

/// 表示を確立する面（`build_target_assets` の唯一の面）。
const FACE: u32 = 1000;
/// 面の素の大きさ。
const FACE_SIZE: (u32, u32) = (4, 3);

fn child_count(world: &World, window: Entity) -> usize {
    world.get::<Children>(window).map_or(0, |c| c.len())
}

/// 窓 DPI `dpi`・作者 DPI 96 で target を登録する（表示はまだ確立しない）。
fn attached_target(
    world: &mut World,
    presenter: &mut EmoPresenter,
    target: TargetId,
    dpi: u16,
) -> Entity {
    let window = spawn_window_with_dpi(world, dpi);
    let (emo_world, atlas, _) = build_target_assets(FACE_SIZE.0, FACE_SIZE.1, 0);
    presenter
        .attach_target(world, target, window, emo_world, atlas, 96)
        .expect("attach_target");
    window
}

/// 要件 4.6: 登録を消す口は表から外し、装着の子 2 つを消し、引き継ぐ値をそのまま返す。
///
/// 外部所有・k=2 の表示で、`pending_resize` は取り出さずに残す（引き継ぐ値がどれも既定値と
/// 違う形にして、「既定値を返しているだけ」で緑にならないようにする）。
#[test]
fn detach_target_unregisters_despawns_children_and_returns_the_carried_values() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let target = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, target, 192);
    presenter
        .set_visibility_ownership(target, VisibilityOwnership::External)
        .expect("ownership");
    show_ok(&mut presenter, &mut world, target, FACE);
    let (surface, slot) = mount_entities(&presenter, target);
    assert_eq!(child_count(&world, window), 2, "較正: 表示の後は窓の子が 2");

    let state = presenter
        .detach_target(&mut world, target)
        .expect("登録済みの target は外せる");

    assert_eq!(state.window, window);
    assert_eq!(state.ownership, VisibilityOwnership::External);
    assert_eq!(state.applied, Some(ScaleRatio::new(2, 1).unwrap()));
    assert_eq!(state.native_size, Some(FACE_SIZE));
    assert_eq!(
        state.pending_resize,
        Some((FACE_SIZE.0 * 2, FACE_SIZE.1 * 2)),
        "取り出していない窓寸の要求は引き継ぐ"
    );

    assert_eq!(presenter.target_visible(target), None, "表から外れていない");
    assert_eq!(child_count(&world, window), 0, "窓の子が残っている");
    assert!(world.get_entity(surface).is_err());
    assert!(world.get_entity(slot).is_err());
}

/// 要件 4.6: 一度も表示していない（装着の子が無い）target も外せる。子を消す対象は無い。
#[test]
fn detach_target_without_a_mount_returns_the_registration_defaults() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let target = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, target, 96);

    let state = presenter.detach_target(&mut world, target).expect("外せる");

    assert_eq!(state.window, window);
    assert_eq!(state.ownership, VisibilityOwnership::CommandDriven);
    assert_eq!(state.applied, None);
    assert_eq!(state.native_size, None);
    assert_eq!(state.pending_resize, None);
    assert_eq!(presenter.target_visible(target), None);
}

/// 要件 4.6: 未登録の target は `error!` 1 件と `Err(TargetNotAttached)`。表は変わらない
/// （他の登録の表示・窓の子はそのまま）。同じ target を 2 度外すのも未登録として扱う。
#[test]
fn detach_target_of_an_unregistered_target_fails_and_leaves_the_table_unchanged() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let kept = TargetId(0);
    let window = attached_target(&mut world, &mut presenter, kept, 96);
    show_ok(&mut presenter, &mut world, kept, FACE);
    let (surface, slot) = mount_entities(&presenter, kept);

    let missing = TargetId(9);
    let (result, events) = capture(|| presenter.detach_target(&mut world, missing));

    assert!(
        matches!(result, Err(PresentError::TargetNotAttached(t)) if t == missing),
        "未登録は TargetNotAttached: {result:?}"
    );
    let errors = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .count();
    assert_eq!(errors, 1, "未登録の error! はちょうど 1 件: {events:?}");

    assert_eq!(presenter.target_visible(kept), Some(true));
    assert_eq!(presenter.current_surface_id(kept), Some(FACE));
    assert_eq!(mount_entities(&presenter, kept), (surface, slot));
    assert_eq!(child_count(&world, window), 2, "他の登録の窓の子は不変");

    // 2 度目は未登録。
    presenter
        .detach_target(&mut world, kept)
        .expect("1 度目は外せる");
    assert!(matches!(
        presenter.detach_target(&mut world, kept),
        Err(PresentError::TargetNotAttached(t)) if t == kept
    ));
}
