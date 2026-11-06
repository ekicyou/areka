//! 窓の生成の位置・失敗・記録（`audit-wintf-runtime` 要件 4.1・4.2・5.2・7.1、設計 R2 の 3・6・9・10）。
//!
//! 本物の窓を作るテストは、作った直後に `ShowWindow(SW_HIDE)` で隠してから確かめる。

use super::*;
use crate::ecs::types::Point;
use crate::ecs::window::window_system::create_windows;
use crate::ecs::world::EcsWorldSelfRef;
use log_capture_kit::count_levels;
use std::rc::Rc;
use windows::Win32::Foundation::RECT;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetWindowRect, SW_HIDE, WINDOW_EX_STYLE, WS_POPUP,
};
use windows::core::w;

fn shared_world() -> Rc<RefCell<EcsWorld>> {
    Rc::new(RefCell::new(EcsWorld::new()))
}

fn popup(title: &str, parent: Option<HWND>, pos: WindowPos) -> (Window, WindowStyle, WindowPos) {
    (
        Window {
            title: title.to_string(),
            parent,
        },
        WindowStyle {
            style: WS_POPUP,
            ex_style: WS_EX_LAYERED,
        },
        pos,
    )
}

fn hide(world: &World, entity: Entity) -> HWND {
    let hwnd = world
        .get::<WindowHandle>(entity)
        .expect("WindowHandle が付くべき")
        .hwnd;
    let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
    hwnd
}

/// 3: 両方既定なら呼ばない・位置だけなら大きさを変えない・大きさだけなら動かさない・両方なら両方。
#[test]
fn initial_pos_flags_four_cases() {
    let base = SWP_NOZORDER | SWP_NOACTIVATE;
    assert_eq!(initial_pos_flags(CW_USEDEFAULT, CW_USEDEFAULT), None);
    assert_eq!(
        initial_pos_flags(100, CW_USEDEFAULT),
        Some(base | SWP_NOSIZE)
    );
    assert_eq!(
        initial_pos_flags(CW_USEDEFAULT, 300),
        Some(base | SWP_NOMOVE)
    );
    assert_eq!(initial_pos_flags(100, 300), Some(base));
}

/// 3: 位置だけを与えた本物の窓は、作った時点で左上がその位置にある。
#[test]
fn position_only_window_is_created_at_the_given_position() {
    let shared = shared_world();
    let mut world = World::new();
    world.insert_non_send(WindowRegistry::<LibWindow<WndState>>::new());
    let pos = WindowPos {
        position: Some(Point { x: 123, y: 77 }),
        ..Default::default()
    };
    let entity = world.spawn(popup("PosOnly", None, pos)).id();

    EcsWindowFactory::create_window(&mut world, entity, Rc::downgrade(&shared));
    let hwnd = hide(&world, entity);

    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.expect("GetWindowRect");
    assert_eq!((rect.left, rect.top), (123, 77));
}

/// 6: 失敗の枝は印を入れ、`error` を 1 行だけ残す。
#[test]
fn creation_failure_marks_entity_and_logs_one_error() {
    let mut world = World::new();
    let entity = world.spawn(Window::default()).id();
    let ((), counts) = count_levels(|| mark_creation_failed(&mut world, entity, "boom"));
    assert!(world.get::<WindowCreationFailed>(entity).is_some());
    assert_eq!(counts.error, 1);
}

/// 6: 印の付いた実体には `create_windows` が窓を作らない。印を外すと次の刻みで作る。
#[test]
fn create_windows_skips_marked_entity_until_the_mark_is_removed() {
    let shared = shared_world();
    let mut world = World::new();
    world.insert_non_send(EcsWorldSelfRef(Rc::downgrade(&shared)));
    world.insert_non_send(WindowRegistry::<LibWindow<WndState>>::new());
    let entity = world
        .spawn((
            popup("Marked", None, WindowPos::default()),
            WindowCreationFailed,
        ))
        .id();

    create_windows(&mut world);
    assert!(
        world.get::<WindowHandle>(entity).is_none(),
        "印の付いた実体は作らない"
    );

    world.entity_mut(entity).remove::<WindowCreationFailed>();
    create_windows(&mut world);
    hide(&world, entity);
}

/// 9: 壊した親を渡すと、付け替えの失敗を `error` で残し、窓は作られる。
#[test]
fn broken_parent_logs_error_and_still_creates_the_window() {
    // SAFETY: Win32 境界。作ってすぐ壊した窓の HWND を「壊れた親」として使う。
    let dead = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("dead-parent"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            None,
            None,
        )
    }
    .expect("STATIC の窓を作る");
    unsafe { DestroyWindow(dead) }.expect("DestroyWindow");

    let shared = shared_world();
    let mut world = World::new();
    world.insert_non_send(WindowRegistry::<LibWindow<WndState>>::new());
    let entity = world
        .spawn(popup("BrokenParent", Some(dead), WindowPos::default()))
        .id();

    let ((), counts) = count_levels(|| {
        EcsWindowFactory::create_window(&mut world, entity, Rc::downgrade(&shared))
    });
    hide(&world, entity);
    assert!(counts.error >= 1, "親の付け替えの失敗は error で残す");
}

/// 10: 登録簿の無い World で作ると、その場で作って入れ、`error` を 1 行残す。
#[test]
fn missing_registry_logs_one_error() {
    let shared = shared_world();
    let mut world = World::new();
    let entity = world
        .spawn(popup("NoRegistry", None, WindowPos::default()))
        .id();

    let ((), counts) = count_levels(|| {
        EcsWindowFactory::create_window(&mut world, entity, Rc::downgrade(&shared))
    });
    hide(&world, entity);
    assert!(
        world
            .get_non_send::<WindowRegistry<LibWindow<WndState>>>()
            .is_some_and(|r| !r.is_empty()),
        "その場で作った登録簿に入る"
    );
    assert_eq!(counts.error, 1);
}
