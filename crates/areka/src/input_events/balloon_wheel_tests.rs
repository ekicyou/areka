//! 普通のバルーンの窓のホイールの系の檻（balloon-scroll-arrows 要件 6.1・6.2・6.3・6.5・6.7・11.1）。
//!
//! 手動の位置（自動の先頭行の 1 行上）にいる文字の層を実 GPU の提示で作り（`manual_balloon_runtime`）、
//! 偽のバルーン窓の子に `PointerState` を載せて系を回す。OS の行数は数える偽で差し替える。

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::mpsc;

use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::place::PlaceKey;
use areka_emo_text::scroll_marker::{ScrollChange, ScrollDir, WheelLines};
use areka_sakura::ActorKey;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use log_capture_kit::{LineFormat, capture_lines};
use wintf::ecs::Window;
use wintf::ecs::pointer::{PointerState, WheelDelta};

use super::super::balloon::test_support::headless_emo2_wiring;
use super::super::char_watch::char_watch;
use super::super::{MouseWiring, RegionSource};
use super::balloon_wheel_with;
use crate::emo2_boot::frame::test_support::{balloon_is_manual, manual_balloon_runtime};
use crate::emo2_boot::hit_region::HitRegion;
use crate::placement::spawn::BalloonWindowMarker;

/// バルーン窓（scope 0）の子を持ち主にし、回転を載せる。
fn spawn_rotation(world: &mut World, vertical: i16, horizontal: i16) {
    let win = world
        .spawn((Window::default(), BalloonWindowMarker { scope: 0 }))
        .id();
    world.spawn((
        ChildOf(win),
        PointerState {
            wheel: WheelDelta {
                vertical,
                horizontal,
            },
            ..Default::default()
        },
    ));
}

/// 系を 1 回まわし、OS の行数を読んだ回数を返す（1 目盛り 1 行）。
fn run(world: &mut World) -> u32 {
    let reads = Cell::new(0);
    balloon_wheel_with(world, || {
        reads.set(reads.get() + 1);
        WheelLines::Lines(1)
    });
    reads.get()
}

/// 結線（Emo2Wiring）つきの World と、手動の位置（自動の 1 行上）にいる文字の層。
fn rig(vertical: i16, horizontal: i16) -> (World, Rc<RefCell<TextLayerRuntime>>) {
    let (mut world, runtime) = manual_balloon_runtime("0");
    world.insert_non_send(headless_emo2_wiring(Rc::clone(&runtime)));
    spawn_rotation(&mut world, vertical, horizontal);
    (world, runtime)
}

/// 新しい側へ 1 行送ったときの変化（自動の 1 行上なら自動へ戻る＝`Stopped`、2 行上なら `Moved`）。
fn newer_by_one(runtime: &RefCell<TextLayerRuntime>) -> ScrollChange {
    runtime.borrow_mut().scroll_manual_at(
        &PlaceKey::balloon(&ActorKey::from("0")),
        ScrollDir::Newer,
        1,
    )
}

/// 上へ（向こう側・正の回転）回すと古い側へ送る（要件 6.1・6.3）。
#[test]
fn wheel_up_scrolls_to_older_lines() {
    let (mut world, runtime) = rig(120, 0);
    assert_eq!(
        run(&mut world),
        1,
        "回転を受けた tick に 1 回だけ OS の行数を読む"
    );
    assert_eq!(
        newer_by_one(&runtime),
        ScrollChange::Moved,
        "古い側へ 1 行送られ、自動の 2 行上にいる"
    );
}

/// 下へ（手前・負の回転）回すと新しい側へ送る（要件 6.2）。自動の 1 行上から 1 行で自動へ戻る。
#[test]
fn wheel_down_scrolls_to_newer_lines() {
    let (mut world, runtime) = rig(-120, 0);
    assert_eq!(run(&mut world), 1);
    assert!(
        !balloon_is_manual(&runtime, "0"),
        "新しい側へ送られて自動へ戻った"
    );
}

/// 横の回転と回転なしは読まない（OS の行数も読まない・要件 6.7）。
#[test]
fn horizontal_or_no_rotation_is_ignored() {
    for (vertical, horizontal) in [(0, 120), (0, 0)] {
        let (mut world, runtime) = rig(vertical, horizontal);
        assert_eq!(run(&mut world), 0, "OS の行数を読まない");
        assert!(
            matches!(newer_by_one(&runtime), ScrollChange::Stopped(_)),
            "位置は動かない（自動の 1 行上のまま）"
        );
    }
}

/// 120 に満たない回転は持ち越し、合わせて 1 目盛りで送る（要件 6.3）。
#[test]
fn partial_rotations_carry_over() {
    let (mut world, runtime) = rig(60, 0);
    run(&mut world);
    run(&mut world);
    assert_eq!(newer_by_one(&runtime), ScrollChange::Moved);
}

/// キャラ窓の見張りと同じ段で回しても、バルーンの窓の回転は `OnMouseWheel` にならない（要件 6.7）。
#[test]
fn balloon_wheel_does_not_queue_on_mouse_wheel() {
    fn mock(scope: u32, x: i64, y: i64) -> HitRegion {
        HitRegion {
            scope,
            region: Some("Head".into()),
            surface_point: (x, y),
        }
    }
    let (mut world, runtime) = rig(120, 0);
    let (tx, rx) = mpsc::channel();
    world.insert_non_send(MouseWiring::with_clock(
        tx,
        RegionSource::Mock(mock),
        Box::new(|| 0),
    ));
    char_watch(&mut world);
    run(&mut world);
    // 溜めの期限を越えても吐き出すものが無い。
    world.get_non_send_mut::<MouseWiring>().unwrap().now_ms = Box::new(|| 10_000);
    char_watch(&mut world);
    assert!(rx.try_iter().next().is_none(), "kanade へ何も送らない");
    assert!(
        world
            .get_non_send::<MouseWiring>()
            .unwrap()
            .wheel
            .is_empty(),
        "ホイールの溜めに積まれない"
    );
    assert_eq!(
        newer_by_one(&runtime),
        ScrollChange::Moved,
        "送りは効いている"
    );
}

/// 結線（Emo2Wiring）が無いときは debug を出して何もしない（OS の行数も読まない）。
#[test]
fn without_emo2_wiring_does_nothing() {
    let mut world = World::new();
    spawn_rotation(&mut world, 120, 0);
    let (reads, lines) = capture_lines(LineFormat::LevelFields, || run(&mut world));
    assert_eq!(reads, 0);
    assert!(
        lines
            .iter()
            .any(|l| l.contains("DEBUG") && l.contains("balloon_wheel_no_emo2")),
        "{lines:?}"
    );
}
