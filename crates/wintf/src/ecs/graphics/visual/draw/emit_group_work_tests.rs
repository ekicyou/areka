//! まとめの作業用の絵と DC の使い回し（WARP）。節・装置・大きさが同じ間は 1 度だけ作り、絵は毎回透明から描く。

use super::super::*;
use super::emit_test_support::*;
use bevy_ecs::hierarchy::ChildOf;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::core::Interface;
use windows_numerics::Matrix3x2;

fn group(mix: D2D1_COMPOSITE_MODE) -> VisualGroup {
    VisualGroup {
        mix: MixMode::Composite(mix),
    }
}

/// 16×16 の根。
fn root16(world: &mut World) -> Entity {
    world
        .spawn((Visual::default(), at(0.0, 0.0, 16.0, 16.0)))
        .id()
}

/// 親 `parent` の下に、`(l, t)-(r, b)` を色 `bgra` で塗った節を置く。
fn paint(world: &mut World, parent: Entity, (l, t, r, b): (f32, f32, f32, f32), bgra: [u8; 4]) {
    world.spawn((
        Visual::default(),
        at(l, t, r - l, b - t),
        solid(rect(0.0, 0.0, r - l, b - t), bgra),
        ChildOf(parent),
    ));
}

/// 2 倍の根（作業用の絵の道）で、`work` を使い回して `commands` を 32×32 へ描く。
fn replay_x2(commands: &[DrawCommand], work: &mut GroupWork) -> Pixels {
    let root = Matrix3x2::scale(2.0, 2.0);
    draw_to_pixels(32, 32, root, |dc| {
        let dc6 = dc.cast().ok();
        super::emit(dc, dc6.as_ref(), root, commands, work);
    })
}

/// 根の下に、赤い角 (0,0)-(2,2)・(14,14)-(16,16)（まとめの範囲を 16×16 に保つ）と青 `blue` を持つまとめ。
fn anchored_group(world: &mut World, blue: (f32, f32, f32, f32)) -> (Entity, Entity, Entity) {
    let root = root16(world);
    let g = world
        .spawn((
            Visual::default(),
            at(0.0, 0.0, 16.0, 16.0),
            group(D2D1_COMPOSITE_MODE_SOURCE_OVER),
            ChildOf(root),
        ))
        .id();
    paint(world, g, (0.0, 0.0, 2.0, 2.0), RED);
    paint(world, g, (14.0, 14.0, 16.0, 16.0), RED);
    let b = world
        .spawn((
            Visual::default(),
            at(blue.0, blue.1, blue.2 - blue.0, blue.3 - blue.1),
            solid(rect(0.0, 0.0, blue.2 - blue.0, blue.3 - blue.1), BLUE),
            ChildOf(g),
        ))
        .id();
    (root, g, b)
}

#[test]
fn work_bitmap_is_made_once_while_node_device_and_size_stay() {
    let mut world = World::new();
    let (root, _, _) = anchored_group(&mut world, (4.0, 4.0, 8.0, 8.0));
    let commands = tree_commands(&mut world, root, None);
    let mut work = GroupWork::default();

    let first = replay_x2(&commands, &mut work);
    for _ in 0..3 {
        assert_eq!(replay_x2(&commands, &mut work).bytes, first.bytes);
    }

    assert_eq!(work.created(), 1);
    assert_eq!(work.len(), 1);
    pixel_near(&first, BLUE, 12, 12);
}

#[test]
fn reused_work_bitmap_starts_transparent_each_frame() {
    let mut world = World::new();
    let (root, _, blue) = anchored_group(&mut world, (4.0, 4.0, 8.0, 8.0));
    let mut work = GroupWork::default();
    replay_x2(&tree_commands(&mut world, root, None), &mut work);

    world.entity_mut(blue).insert(at(8.0, 8.0, 4.0, 4.0));
    let commands = tree_commands(&mut world, root, None);
    let reused = replay_x2(&commands, &mut work);
    let fresh = replay_x2(&commands, &mut GroupWork::default());

    assert_eq!(work.created(), 1, "大きさが同じなので作り直さない");
    assert_eq!(reused.bytes, fresh.bytes);
    // 前の回の青 (4,4)-(8,8) が残らない。
    pixel_near(&reused, CLEAR, 12, 12);
    pixel_near(&reused, BLUE, 20, 20);
}

#[test]
fn work_bitmap_is_remade_on_size_change_and_released_when_the_group_is_gone() {
    let mut world = World::new();
    let (root, g, _) = anchored_group(&mut world, (4.0, 4.0, 8.0, 8.0));
    let mut work = GroupWork::default();
    replay_x2(&tree_commands(&mut world, root, None), &mut work);

    // まとめの範囲は根の視野で切られるので、半分を視野の外へ出すと 8×8 になる。
    world.entity_mut(g).insert(at(8.0, 8.0, 16.0, 16.0));
    replay_x2(&tree_commands(&mut world, root, None), &mut work);
    assert_eq!(work.created(), 2);
    assert_eq!(work.len(), 1);

    world.entity_mut(g).remove::<VisualGroup>();
    replay_x2(&tree_commands(&mut world, root, None), &mut work);
    assert_eq!(work.len(), 0, "描かなかったまとめの絵と DC は放す");
}

#[test]
fn work_bitmap_is_remade_on_device_change() {
    let e = Entity::from_raw_u32(7).unwrap();
    let bounds = crate::numerics::Aabb::new(0.0, 0.0, 8.0, 8.0);
    let commands = [
        DrawCommand::PushGroup(GroupLayer {
            entity: e,
            version: 0,
            mix: MixMode::default(),
            world_mat: Matrix3x2::identity(),
            local_aabb: bounds,
            world_aabb: bounds,
        }),
        DrawCommand::PopGroup,
    ];
    let root = Matrix3x2::scale(2.0, 2.0);
    let mut work = GroupWork::default();
    for dc in [warp_dc(), other_device_dc(), other_device_dc()] {
        let target = crate::ecs::graphics::composite::create_bitmap(
            &dc,
            (16, 16),
            D2D1_BITMAP_OPTIONS_TARGET,
        )
        .unwrap();
        // SAFETY: Windows API 境界。`BeginDraw` と `EndDraw` の対の中で描く。
        let ended = unsafe {
            dc.SetTarget(&target);
            dc.BeginDraw();
            super::emit(&dc, None, root, &commands, &mut work);
            let ended = dc.EndDraw(None, None);
            dc.SetTarget(None);
            ended
        };
        ended.expect("前の装置の絵を描かない");
    }
    // 別の装置の DC は呼ぶたびに別の装置（3 回とも違う）。
    assert_eq!(work.created(), 3);
    assert_eq!(work.len(), 1);
}
