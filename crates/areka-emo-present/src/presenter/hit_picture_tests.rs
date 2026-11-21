use std::num::NonZeroU32;

use areka_parsers::shell::{
    AppendTarget, Collision, CollisionBounds, CollisionName, CollisionShape, Surface,
};
use wintf::ecs::{AlphaMaskResource, GraphicsCore, IdRead};

use super::super::test_support::{
    CapturedEvent, bake_world, capture, elem, gradient, gradient_with_alpha, make_world_with_gpu,
    mount_entities, set_window_dpi, shell_of, show_ok, spawn_window_with_dpi,
};
use super::super::{EmoPresenter, TargetId, World};

const T: TargetId = TargetId(0);

fn face(id: u32, image: &str, rect: Option<(i64, i64, i64, i64)>) -> Surface {
    Surface {
        targets: vec![AppendTarget::Single(id)],
        elements: vec![elem(image, 0, 0)],
        collisions: rect
            .map(|(left, top, right, bottom)| Collision {
                index: 0,
                shape: CollisionShape::Rect(CollisionBounds {
                    left,
                    top,
                    right,
                    bottom,
                }),
                name: CollisionName::new("Head".to_string()),
            })
            .into_iter()
            .collect(),
        animations: Vec::new(),
    }
}

/// 面 1000・1001 は 20×10 の不透明な絵に矩形 1 つ、面 2000 は当たり判定を持たない。
fn shown_presenter(world: &mut World) -> (EmoPresenter, bevy_ecs::entity::Entity) {
    let window = spawn_window_with_dpi(world, 96);
    let (emo_world, atlas) = bake_world(
        shell_of(vec![
            face(1000, "a.png", Some((2, 2, 8, 6))),
            face(1001, "b.png", Some((10, 2, 18, 6))),
            face(2000, "a.png", None),
        ]),
        vec![
            ("a.png", 20, 10, gradient(20, 10, 0x11)),
            ("b.png", 20, 10, gradient(20, 10, 0x77)),
        ],
    );
    let mut presenter = EmoPresenter::new();
    presenter.attach_target(T, window, emo_world, atlas, 96);
    show_ok(&mut presenter, world, T, 1000);
    (presenter, window)
}

fn painted(presenter: &EmoPresenter) -> Option<(u32, usize, (u32, u32), u64)> {
    let p = presenter.targets[&T].hit_picture.as_ref()?;
    Some((
        p.surface_id,
        p.first,
        p.target.size(),
        p.target.generation(),
    ))
}

fn read(presenter: &EmoPresenter, x: u32, y: u32) -> IdRead {
    presenter.targets[&T]
        .hit_picture
        .as_ref()
        .expect("描き先がある")
        .target
        .read(x, y)
}

const ID1: IdRead = IdRead::Id(NonZeroU32::MIN);

#[test]
fn display_commit_paints_the_shown_surface() {
    let mut world = make_world_with_gpu();
    let (presenter, _) = shown_presenter(&mut world);

    assert_eq!(painted(&presenter), Some((1000, 0, (20, 10), 1)));
    assert_eq!(read(&presenter, 4, 3), ID1);
    assert_eq!(read(&presenter, 12, 3), IdRead::Empty);

    let (surface_entity, _) = mount_entities(&presenter, T);
    let handed = world
        .get::<AlphaMaskResource>(surface_entity)
        .and_then(|r| r.mask())
        .expect("クリックの透過へ渡したマスク");
    let shown = presenter.targets[&T]
        .shown_mask
        .as_deref()
        .expect("表示中の α マスクの写し");
    assert!(std::ptr::eq(handed, shown), "同じ実体を写していない");
}

#[test]
fn same_surface_and_scale_only_changes_do_not_repaint() {
    let mut world = make_world_with_gpu();
    let (mut presenter, window) = shown_presenter(&mut world);

    show_ok(&mut presenter, &mut world, T, 1000);
    set_window_dpi(&mut world, window, 192);
    show_ok(&mut presenter, &mut world, T, 1000);
    assert_eq!(painted(&presenter), Some((1000, 0, (20, 10), 1)));

    // 同じ大きさの描き先は使い回して塗り直す。
    show_ok(&mut presenter, &mut world, T, 1001);
    assert_eq!(painted(&presenter), Some((1001, 0, (20, 10), 2)));
    assert_eq!(read(&presenter, 4, 3), IdRead::Empty);
    assert_eq!(read(&presenter, 12, 3), ID1);
}

#[test]
fn surface_without_hit_regions_has_no_picture() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _) = shown_presenter(&mut world);

    show_ok(&mut presenter, &mut world, T, 2000);
    assert_eq!(painted(&presenter), None);
    show_ok(&mut presenter, &mut world, T, 1000);
    assert_eq!(painted(&presenter), Some((1000, 0, (20, 10), 1)));
}

#[test]
fn no_device_keeps_the_display_and_retries_at_the_next_commit() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _) = shown_presenter(&mut world);
    show_ok(&mut presenter, &mut world, T, 2000);
    presenter.refresh_after_device_change(&mut world);

    // 面 1000 は控えに在るので、装置が無くても表示は成立する。
    world.resource_mut::<GraphicsCore>().invalidate();
    show_ok(&mut presenter, &mut world, T, 1000);
    assert_eq!(presenter.current_surface_id(T), Some(1000));
    assert_eq!(painted(&presenter), None);

    // 装置の作り直しの追従が表示を出し直し、その成立でまた塗る。
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore::new 失敗"));
    presenter.refresh_after_device_change(&mut world);
    assert_eq!(painted(&presenter), Some((1000, 0, (20, 10), 1)));
}

#[test]
fn device_change_discards_the_picture_and_repaints() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _) = shown_presenter(&mut world);
    show_ok(&mut presenter, &mut world, T, 1001);
    show_ok(&mut presenter, &mut world, T, 1000);
    assert_eq!(painted(&presenter), Some((1000, 0, (20, 10), 3)));

    presenter.refresh_after_device_change(&mut world);
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore::new 失敗"));
    presenter.refresh_after_device_change(&mut world);
    assert_eq!(painted(&presenter), Some((1000, 0, (20, 10), 1)));
}

/// 40×20 の絵に楕円 `Eye` と三角形 `Fan`。x が 34 以上の列だけ透明。
fn ellipse_and_polygon_presenter(world: &mut World) -> EmoPresenter {
    let window = spawn_window_with_dpi(world, 96);
    let named = |shape, name: &str| Collision {
        index: 0,
        shape,
        name: CollisionName::new(name.to_string()),
    };
    let (emo_world, atlas) = bake_world(
        shell_of(vec![Surface {
            targets: vec![AppendTarget::Single(1000)],
            elements: vec![elem("a.png", 0, 0)],
            collisions: vec![
                named(
                    CollisionShape::Ellipse(CollisionBounds {
                        left: 2,
                        top: 2,
                        right: 18,
                        bottom: 18,
                    }),
                    "Eye",
                ),
                named(
                    CollisionShape::Polygon(vec![(22, 2), (38, 2), (38, 18)]),
                    "Fan",
                ),
            ],
            animations: Vec::new(),
        }]),
        vec![(
            "a.png",
            40,
            20,
            gradient_with_alpha(40, 20, 0x31, |x, _| if x >= 34 { 0 } else { 0xFF }),
        )],
    );
    let mut presenter = EmoPresenter::new();
    presenter.attach_target(T, window, emo_world, atlas, 96);
    show_ok(&mut presenter, world, T, 1000);
    presenter
}

#[test]
fn ellipse_and_polygon_resolve_to_names_and_transparent_pixels_miss() {
    let mut world = make_world_with_gpu();
    let presenter = ellipse_and_polygon_presenter(&mut world);
    for (x, y, want) in [
        (10, 10, Some("Eye")),
        (3, 3, None), // 楕円の囲みの隅
        (30, 6, Some("Fan")),
        (25, 15, None), // 三角形の外
        (36, 6, None),  // 三角形の中だが絵が透明
        (20, 10, None),
        (-1, 5, None),
        (40, 5, None),
    ] {
        let hit = presenter.hit_region_client(T, x, y);
        assert_eq!(hit.region, want, "({x},{y})");
        assert_eq!(hit.surface_point, (x, y));
    }
}

fn unreadable_warns(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::WARN
                && e.field("message")
                    .is_some_and(|m| m.contains("描き先が読めない"))
        })
        .count()
}

#[test]
fn missing_picture_misses_and_warns_once_until_a_repaint() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _) = shown_presenter(&mut world);
    let drop_picture = |p: &mut EmoPresenter| p.targets.get_mut(&T).unwrap().hit_picture = None;

    drop_picture(&mut presenter);
    let (hits, first) = capture(|| {
        [
            presenter.hit_region_client(T, 4, 3).region,
            presenter.hit_region_client(T, 5, 3).region,
        ]
    });
    assert_eq!(hits, [None, None]);
    assert_eq!(unreadable_warns(&first), 1, "{first:?}");

    show_ok(&mut presenter, &mut world, T, 1001);
    assert_eq!(presenter.hit_region_client(T, 12, 3).region, Some("Head"));
    drop_picture(&mut presenter);
    let (_, again) = capture(|| presenter.hit_region_client(T, 12, 3));
    assert_eq!(
        unreadable_warns(&again),
        1,
        "塗り直しの後はまた 1 件: {again:?}"
    );
}

#[test]
fn picture_of_another_surface_is_unreadable() {
    let mut world = make_world_with_gpu();
    let (mut presenter, _) = shown_presenter(&mut world);
    presenter.targets.get_mut(&T).unwrap().current_surface_id = Some(1001);
    let (hit, events) = capture(|| presenter.hit_region_client(T, 12, 3));
    assert_eq!(hit.region, None);
    assert_eq!(unreadable_warns(&events), 1, "{events:?}");
}
