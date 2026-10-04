//! 箱の文字の面（シェルの窓の直接の子）の装着と片付けの檻（task 7.2・要件 1.6・3.6・3.7・9.4）。
//!
//! GPU/WUC は headless 実資源（`GraphicsCore::new()`・WARP 可・`surface.rs` の既存テストと同じ方針）。

use super::*;

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::name::Name;

use windows::UI::Composition::Compositor;
use windows::Win32::System::WinRT::{DQTAT_COM_ASTA, DQTAT_COM_NONE};

use wintf::com::wuc::create_dispatcher_queue_controller;
use wintf::ecs::{GraphicsCommandList, HitTest, Visual};

/// テスト用 WUC dispatcher と Compositor（`surface.rs` の既存テストと同じ作り方）。
fn make_dispatcher_and_compositor() -> (windows::System::DispatcherQueueController, Compositor) {
    let dq = create_dispatcher_queue_controller(DQTAT_COM_ASTA)
        .or_else(|e_asta| create_dispatcher_queue_controller(DQTAT_COM_NONE).map_err(|_| e_asta))
        .expect("DispatcherQueueController 生成失敗（ASTA/NONE いずれも不可）");
    let compositor = Compositor::new().expect("Compositor::new 失敗");
    (dq, compositor)
}

/// 窓と、その直接の子 2 つ（差し込み口・絵）を組む。返り値は (window, slot, picture)。
fn spawn_window_with_slot_and_picture(world: &mut World) -> (Entity, Entity, Entity) {
    let window = world.spawn_empty().id();
    let slot = world
        .spawn((
            Name::new("emo-text-layer-slot"),
            Visual::default(),
            ChildOf(window),
        ))
        .id();
    let picture = world
        .spawn((Name::new("picture"), Visual::default(), ChildOf(window)))
        .id();
    world.flush();
    (window, slot, picture)
}

fn children_of(world: &World, window: Entity) -> Vec<Entity> {
    world
        .get::<Children>(window)
        .map(|c| c.iter().collect())
        .unwrap_or_default()
}

/// 窓の子の面は、指定の並び位置（差し込み口の直後＝1）に挿さり、字の矩形 0 個のマスクの当たり判定を持ち、
/// 物理 px の位置と大きさを持ち、透明で始まる。片付けると entity が消え、窓の子の並びから外れる。
#[test]
fn window_child_is_inserted_at_index_with_empty_hit_mask_and_despawned_on_cleanup() {
    let (_dq, compositor) = make_dispatcher_and_compositor();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");

    let mut world = World::new();
    let (window, slot, picture) = spawn_window_with_slot_and_picture(&mut world);

    let surface = TextSurface::attach_window_child(
        &mut world,
        window,
        1,
        &compositor,
        &core,
        (5, 4),
        (12.0, 7.0),
    )
    .expect("attach_window_child 失敗");
    let child = surface
        .window_child()
        .expect("窓の子の面は自分の entity を持つ");

    // --- 並び: 差し込み口 → 箱 → 絵（箱は絵より手前・要件 3.7） ---
    assert_eq!(
        children_of(&world, window),
        vec![slot, child, picture],
        "窓の子の面は Children の指定位置（1）へ挿さる"
    );
    assert_eq!(
        world.get::<ChildOf>(child).map(|c| c.parent()),
        Some(window),
        "窓の直接の子（差し込み口の下の子ではない・要件 3.6）"
    );

    // --- 当たり判定は字の矩形のマスクで、装着の時は 0 個（何も受けない・要件 9.4・task 13） ---
    assert_eq!(
        world.get::<HitTest>(child).copied(),
        Some(HitTest::alpha_mask()),
        "箱の面はマスク（字の矩形の集まり）で当たりを決める"
    );
    let mask = world
        .get::<wintf::ecs::AlphaMaskResource>(child)
        .and_then(wintf::ecs::AlphaMaskResource::mask)
        .expect("空のマスクを持つ（無いと wintf は矩形全体の判定へ縮退する）");
    assert_eq!(
        (mask.width(), mask.height()),
        (5, 4),
        "マスクは面と同じ物理寸"
    );
    assert!(
        (0..4).all(|y| (0..5).all(|x| !mask.is_hit(x, y))),
        "装着の時は字が無いので何も受けない"
    );

    // --- 物理 px の位置と大きさ・自前 brush・wintf の描画経路は使わない ---
    let arr = world.get::<Arrangement>(child).expect("Arrangement がある");
    assert_eq!((arr.size.width, arr.size.height), (5.0, 4.0));
    assert_eq!((arr.offset.x, arr.offset.y), (12.0, 7.0));
    assert!(
        world
            .get::<VisualGraphics>(child)
            .is_some_and(|vg| vg.is_valid()),
        "自前 brush を装着した VisualGraphics を持つ"
    );
    assert!(
        world.get::<GraphicsCommandList>(child).is_none(),
        "GraphicsCommandList は持たない（背景の絵を描かない）"
    );

    // --- 透明で始まる（背景の絵を描かない・要件 1.6） ---
    let bytes = surface.read_back().expect("read_back 失敗");
    assert_eq!(bytes.len(), 5 * 4 * 4);
    assert!(bytes.iter().all(|&b| b == 0), "窓の子の面は透明で始まる");

    // --- 片付け: entity を消し、窓の子の並びから外れる ---
    surface.despawn_window_child(&mut world);
    assert!(
        world.get_entity(child).is_err(),
        "片付けで窓の子の entity が消える"
    );
    assert_eq!(
        children_of(&world, window),
        vec![slot, picture],
        "片付けた後の窓の子は元の並びに戻る"
    );
}

/// 並び位置が子の数を超えるときは末尾へ挿さる（bevy の `insert_children` の切り詰め）。
#[test]
fn window_child_index_past_end_is_clamped_to_last() {
    let (_dq, compositor) = make_dispatcher_and_compositor();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");

    let mut world = World::new();
    let (window, slot, picture) = spawn_window_with_slot_and_picture(&mut world);

    let surface = TextSurface::attach_window_child(
        &mut world,
        window,
        99,
        &compositor,
        &core,
        (2, 2),
        (0.0, 0.0),
    )
    .expect("attach_window_child 失敗");
    let child = surface.window_child().expect("entity を持つ");
    assert_eq!(children_of(&world, window), vec![slot, picture, child]);
}

/// 窓の entity が居ないときは panic せず `Device` の誤りを返し、子を作らない（log-first）。
#[test]
fn window_child_attach_fails_without_panic_when_window_missing() {
    let (_dq, compositor) = make_dispatcher_and_compositor();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");

    let mut world = World::new();
    let window = world.spawn_empty().id();
    world.despawn(window);
    let before = world.query::<Entity>().iter(&world).count();

    let err = TextSurface::attach_window_child(
        &mut world,
        window,
        0,
        &compositor,
        &core,
        (2, 2),
        (0.0, 0.0),
    )
    .err()
    .expect("窓が居なければ Err");
    assert!(matches!(err, TextLayerError::Device { .. }), "{err:?}");
    assert_eq!(
        world.query::<Entity>().iter(&world).count(),
        before,
        "子の entity を残さない"
    );
}

/// 差し込み口へ装着した面は窓の子を持たない（片付けの対象は窓の子の面だけ）。
#[test]
fn slot_attached_surface_has_no_window_child() {
    let (_dq, compositor) = make_dispatcher_and_compositor();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");

    let mut world = World::new();
    let (window, slot, _picture) = spawn_window_with_slot_and_picture(&mut world);
    let binding = TextSlotBinding::new(slot, window, 1.0, (10, 8), (10, 8));
    let surface = TextSurface::attach(&mut world, &binding, &compositor, &core, (2, 2), (0.0, 0.0))
        .expect("attach 失敗");
    assert_eq!(surface.window_child(), None);
}
