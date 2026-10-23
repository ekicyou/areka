//! 毎 tick の窓の見張りの系の檻（mouse-click-wheel-events 要件 3.3・6・7・11.2・11.3・design「CharWatch」）。
//!
//! 偽の窓（`Window`＋`CharWindowMarker`）の子に `PointerState` を載せ、注入の時計で系を回す。

use super::super::{MouseWiring, RegionSource, on_char_pointer_moved};
use super::char_watch;
use crate::emo2_boot::hit_region::HitRegion;
use crate::placement::spawn::CharWindowMarker;
use areka_kanade::{ClickButton, HoverEdge, KanadeMsg, MouseEventKind, MouseInput};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::mpsc;
use wintf::ecs::pointer::{Phase, PointerState, WheelDelta};
use wintf::ecs::{PhysicalPoint, Window};

thread_local! {
    /// 真なら、どの位置も当たり判定「Swap」（止まったまま当たり判定が替わった、を作る）。
    static SWAPPED: Cell<bool> = const { Cell::new(false) };
}

/// x < 50 → Head、50 ≤ x < 100 → Face、それ以外 → 無し。座標は 2 倍（縮約後と分かるように）。
fn mock(scope: u32, x: i64, y: i64) -> HitRegion {
    let region = if SWAPPED.with(Cell::get) {
        Some("Swap")
    } else if x < 50 {
        Some("Head")
    } else if x < 100 {
        Some("Face")
    } else {
        None
    };
    HitRegion {
        scope,
        region: region.map(str::to_owned),
        surface_point: (x * 2, y * 2),
    }
}

struct Rig {
    world: World,
    rx: mpsc::Receiver<KanadeMsg>,
    clock: Rc<Cell<u64>>,
}

fn rig() -> Rig {
    SWAPPED.with(|s| s.set(false));
    let (tx, rx) = mpsc::channel();
    let clock = Rc::new(Cell::new(0));
    let c = clock.clone();
    let mut world = World::new();
    world.insert_non_send(MouseWiring::with_clock(
        tx,
        RegionSource::Mock(mock),
        Box::new(move || c.get()),
    ));
    Rig { world, rx, clock }
}

impl Rig {
    /// 偽のキャラ窓と、その子（絵の当たった所）。
    fn window(&mut self, scope: usize) -> (Entity, Entity) {
        let win = self
            .world
            .spawn((Window::default(), CharWindowMarker { scope }))
            .id();
        let child = self.world.spawn(ChildOf(win)).id();
        (win, child)
    }

    /// `e` を持ち主にする（他の entity の `PointerState` は外す・ホイールは 0）。
    fn point(&mut self, e: Entity, x: i32, y: i32) {
        self.point_wheel(e, x, y, 0, 0);
    }

    fn point_wheel(&mut self, e: Entity, x: i32, y: i32, vertical: i16, horizontal: i16) {
        self.world.entity_mut(e).insert(PointerState {
            client_point: PhysicalPoint::new(x, y),
            wheel: WheelDelta {
                vertical,
                horizontal,
            },
            ..Default::default()
        });
    }

    fn away(&mut self, e: Entity) {
        self.world.entity_mut(e).remove::<PointerState>();
    }

    fn run(&mut self) -> Vec<MouseInput> {
        char_watch(&mut self.world);
        // 本番では FrameFinalize がホイールを 0 に戻す（1 tick だけ有効）。
        let owners: Vec<Entity> = self
            .world
            .query_filtered::<Entity, With<PointerState>>()
            .iter(&self.world)
            .collect();
        for e in owners {
            self.world.get_mut::<PointerState>(e).unwrap().wheel = WheelDelta::default();
        }
        self.drain()
    }

    fn drain(&mut self) -> Vec<MouseInput> {
        self.rx
            .try_iter()
            .map(|m| match m {
                KanadeMsg::Mouse(i) => i,
                _ => panic!("マウス以外の知らせ"),
            })
            .collect()
    }

    fn wiring(&self) -> &MouseWiring {
        self.world.get_non_send::<MouseWiring>().unwrap()
    }
}

fn hover(scope: u32, edge: HoverEdge, at: (i64, i64), region: Option<&str>) -> MouseInput {
    MouseInput {
        scope,
        x: at.0,
        y: at.1,
        region: region.map(str::to_owned),
        kind: MouseEventKind::Hover(edge),
    }
}

fn wheel(scope: u32, delta: i32, at: (i64, i64), region: Option<&str>) -> MouseInput {
    MouseInput {
        scope,
        x: at.0,
        y: at.1,
        region: region.map(str::to_owned),
        kind: MouseEventKind::Wheel { delta },
    }
}

/// 入る順: EnterAll → Enter。A→B は Leave(A) → Enter(B)。出る順: Leave → LeaveAll（要件 7.1–7.5）。
#[test]
fn enter_move_across_and_leave_in_order() {
    let mut r = rig();
    let (_win, child) = r.window(0);
    r.point(child, 10, 5);
    assert_eq!(
        r.run(),
        vec![
            hover(0, HoverEdge::EnterAll, (20, 10), Some("Head")),
            hover(0, HoverEdge::Enter, (20, 10), Some("Head")),
        ]
    );
    r.point(child, 60, 5);
    assert_eq!(
        r.run(),
        vec![
            hover(0, HoverEdge::Leave, (120, 10), Some("Head")),
            hover(0, HoverEdge::Enter, (120, 10), Some("Face")),
        ]
    );
    r.away(child);
    assert_eq!(
        r.run(),
        vec![
            hover(0, HoverEdge::Leave, (120, 10), Some("Face")),
            hover(0, HoverEdge::LeaveAll, (120, 10), Some("Face")),
        ]
    );
    assert!(r.wiring().hover.is_empty());
}

/// 位置が同じなら何も出ない。止まったまま当たり判定が替わっても、動くまで出ない（要件 7.9）。
#[test]
fn no_events_without_movement_even_if_region_changes() {
    let mut r = rig();
    let (_win, child) = r.window(0);
    r.point(child, 10, 5);
    r.run();
    assert_eq!(r.run(), vec![]);
    SWAPPED.with(|s| s.set(true));
    assert_eq!(r.run(), vec![]);
    r.point(child, 11, 5);
    assert_eq!(
        r.run(),
        vec![
            hover(0, HoverEdge::Leave, (22, 10), Some("Head")),
            hover(0, HoverEdge::Enter, (22, 10), Some("Swap")),
        ]
    );
}

/// 窓 2 つは別々: 本体から相方へ移ると、出る窓の出入りが先（要件 7.14）。
#[test]
fn two_windows_are_separate_and_leaving_window_goes_first() {
    let mut r = rig();
    let (_w0, c0) = r.window(0);
    let (_w1, c1) = r.window(1);
    r.point(c0, 10, 5);
    r.run();
    r.away(c0);
    r.point(c1, 120, 5);
    assert_eq!(
        r.run(),
        vec![
            hover(0, HoverEdge::Leave, (20, 10), Some("Head")),
            hover(0, HoverEdge::LeaveAll, (20, 10), Some("Head")),
            hover(1, HoverEdge::EnterAll, (240, 10), None),
        ]
    );
}

/// 窓を消すと何も送らず、3 つの表の覚えが消える（要件 7.10）。
#[test]
fn despawned_window_is_forgotten_silently() {
    let mut r = rig();
    let (win, child) = r.window(0);
    r.point_wheel(child, 10, 5, 120, 0);
    r.run();
    r.clock.set(10);
    r.point_wheel(child, 10, 5, 120, 0);
    r.run(); // 溜めが残る
    r.world.non_send_mut::<MouseWiring>().presses.insert(
        (win, ClickButton::Left),
        super::super::click::PressNote {
            double: false,
            by_box: false,
        },
    );
    r.world.despawn(child);
    r.world.despawn(win);
    r.clock.set(200);
    assert_eq!(r.run(), vec![]);
    let w = r.wiring();
    assert!(w.hover.is_empty());
    assert!(w.wheel.is_empty());
    assert!(w.presses.is_empty());
}

/// 窓の外へ出たら右・中・拡張の控えを捨てる（左は残す・要件 3.3）。
#[test]
fn leaving_window_discards_side_button_notes() {
    let mut r = rig();
    let (win, child) = r.window(0);
    r.point(child, 10, 5);
    r.run();
    let note = super::super::click::PressNote {
        double: false,
        by_box: false,
    };
    {
        let mut w = r.world.non_send_mut::<MouseWiring>();
        for b in [
            ClickButton::Left,
            ClickButton::Right,
            ClickButton::Middle,
            ClickButton::XButton1,
            ClickButton::XButton2,
        ] {
            w.presses.insert((win, b), note);
        }
    }
    r.away(child);
    r.run();
    let keys: Vec<_> = r.wiring().presses.keys().copied().collect();
    assert_eq!(keys, vec![(win, ClickButton::Left)]);
}

/// 縦のホイールで送り、横だけでは何も出ない（要件 6.1・6.7）。
#[test]
fn vertical_wheel_sends_horizontal_only_does_not() {
    let mut r = rig();
    let (_win, child) = r.window(0);
    r.point(child, 10, 5);
    r.run();
    r.point_wheel(child, 10, 5, 0, 120);
    assert_eq!(r.run(), vec![]);
    r.point_wheel(child, 10, 5, -120, 0);
    assert_eq!(r.run(), vec![wheel(0, -120, (20, 10), Some("Head"))]);
}

/// 持ち主が窓の entity そのもの（絵の透けた所）ならホイールを送らない（要件 6.10）。
#[test]
fn wheel_over_transparent_area_is_not_sent() {
    let mut r = rig();
    let (win, _child) = r.window(0);
    r.point(win, 10, 5);
    // 透けた所は当たり判定「無し」として入る。
    assert_eq!(r.run(), vec![hover(0, HoverEdge::EnterAll, (20, 10), None)]);
    r.point_wheel(win, 10, 5, 120, 0);
    assert_eq!(r.run(), vec![]);
    assert!(r.wiring().wheel.is_empty());
}

/// 間隔の中の回転は溜まり、時計を進めた次の回しで回転を待たずに吐き出す（要件 6.5・11.2）。
#[test]
fn pending_wheel_is_flushed_on_a_later_run_after_the_clock_advances() {
    let mut r = rig();
    let (_win, child) = r.window(0);
    r.point_wheel(child, 10, 5, 120, 0);
    let first = r.run();
    assert_eq!(first.last(), Some(&wheel(0, 120, (20, 10), Some("Head"))));
    r.clock.set(30);
    r.point_wheel(child, 10, 5, 120, 0);
    assert_eq!(r.run(), vec![]);
    r.clock.set(60);
    r.point_wheel(child, 10, 5, 240, 0);
    assert_eq!(r.run(), vec![]);
    r.clock.set(99);
    assert_eq!(r.run(), vec![]);
    r.clock.set(100);
    assert_eq!(r.run(), vec![wheel(0, 360, (20, 10), Some("Head"))]);
    assert_eq!(r.run(), vec![]);
}

/// 出入りは同じ tick の `OnMouseMove` より先に届く（要件 7.7）。
#[test]
fn hover_arrives_before_move_of_the_same_tick() {
    let mut r = rig();
    let (win, child) = r.window(0);
    r.point(child, 10, 5);
    char_watch(&mut r.world);
    let state = r.world.get::<PointerState>(child).unwrap().clone();
    on_char_pointer_moved(&mut r.world, win, win, &Phase::Bubble(state));
    let kinds: Vec<_> = r.drain().into_iter().map(|i| i.kind).collect();
    assert_eq!(
        kinds,
        vec![
            MouseEventKind::Hover(HoverEdge::EnterAll),
            MouseEventKind::Hover(HoverEdge::Enter),
            MouseEventKind::Move,
        ]
    );
}

/// 移動の間引きが閉じていても出入りは出る（系は移動の間引きの状態を読まない・要件 7.6）。
#[test]
fn hover_is_not_throttled_by_the_move_throttle() {
    let mut r = rig();
    let (win, child) = r.window(0);
    let moved = |r: &mut Rig, x: i32| {
        r.point(child, x, 5);
        let state = r.world.get::<PointerState>(child).unwrap().clone();
        on_char_pointer_moved(&mut r.world, win, win, &Phase::Bubble(state));
        r.drain().len()
    };
    assert_eq!(moved(&mut r, 10), 1);
    assert_eq!(moved(&mut r, 11), 0, "移動の間引きは閉じている");
    assert_eq!(
        r.run(),
        vec![
            hover(0, HoverEdge::EnterAll, (22, 10), Some("Head")),
            hover(0, HoverEdge::Enter, (22, 10), Some("Head")),
        ]
    );
}

/// 結線の前は何もしない（落ちない）。
#[test]
fn no_wiring_is_a_no_op() {
    let mut world = World::new();
    let win = world
        .spawn((Window::default(), CharWindowMarker { scope: 0 }))
        .id();
    world.entity_mut(win).insert(PointerState::default());
    char_watch(&mut world);
}
