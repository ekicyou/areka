//! 位置初期化の近道の檻（areka-P0-window-link-directives 要件 8.1〜8.4・9.5・design「shortcut」）。
//!
//! 受け手は本物の `on_ghost_key_down` を World へ直に呼ぶ（wintf の窓の手続きから受け手までは
//! wintf の `keyboard_tests.rs` が固定済み）。「シェル位置初期化」はゴーストへの往復の送りと答えの
//! 受け手の有無で、「バルーン位置初期化」は `reset_balloon_positions` の記録のきっかけで見る。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;

use areka_kanade::KanadeMsg;
use bevy_ecs::prelude::*;
use wintf::ecs::window::{KeyDown, OnKeyDown};

use super::{PositionShortcut, attach_shortcut_receivers, on_ghost_key_down, shortcut_of};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::menu::reset_pos_frame::PendingResetRaise;
use crate::placement::follow::OffsetBase;
use crate::placement::reseed::BalloonPlacementInputs;
use crate::placement::reset_pos::DefaultPlacementBasis;
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::{DescriptSource, GhostTitles};
use crate::placement::spawn::{GhostWindowMarker, GhostWindows, spawn_ghost_windows};
use crate::placement::test_support::{LogEvent, capture_logs};

const F10: u16 = 0x79;
const F12: u16 = 0x7B;

fn key(vk: u16, ctrl: bool, shift: bool, alt: bool, repeat: bool) -> KeyDown {
    KeyDown {
        vk,
        ctrl,
        shift,
        alt,
        repeat,
    }
}

fn ctrl_shift(vk: u16) -> KeyDown {
    key(vk, true, true, false, false)
}

fn placement() -> ScopePlacement {
    ScopePlacement {
        scope: 0,
        char_pos: PointPx { x: 1483, y: 733 },
        char_size: SizePx { w: 434, h: 687 },
        balloon_pos: PointPx { x: 1071, y: 708 },
        balloon_size: SizePx { w: 223, h: 158 },
        balloon_offset: PointPx { x: -412, y: -25 },
        balloon_offset_base: OffsetBase::unpinned(PointPx { x: -412, y: -25 }),
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}

fn spawn_one_scope(world: &mut World) -> GhostWindows {
    spawn_ghost_windows(
        world,
        &[placement()],
        &GhostTitles::from_scope_titles([(0, "s".to_string())]),
    )
}

fn basis() -> DefaultPlacementBasis {
    DefaultPlacementBasis {
        source: DescriptSource {
            ghost_kv: BTreeMap::new(),
            shell_kv: BTreeMap::new(),
            shell_dir: PathBuf::from("shell/master"),
            titles: GhostTitles::from_scope_titles([(0, "s".to_string())]),
        },
        balloon: BalloonPlacementInputs {
            author_dpi: 96,
            windowpositions: BTreeMap::new(),
        },
    }
}

/// ゴーストへの送り口を置いた World（受信端を返す）。
fn world_with_ghost() -> (World, mpsc::Receiver<KanadeMsg>) {
    let (tx, rx) = mpsc::channel();
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        PathBuf::from("ghost/A"),
    ))));
    (world, rx)
}

fn named<'a>(events: &'a [LogEvent], name: &str) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// バルーン位置初期化の記録（成功・外し）のきっかけの欄。
fn balloon_reset_triggers(events: &[LogEvent]) -> Vec<Option<String>> {
    events
        .iter()
        .filter(|e| {
            matches!(
                e.field_str("event"),
                Some("balloon_pos_reset" | "balloon_pos_reset_skipped")
            )
        })
        .map(|e| e.field_str("trigger").map(str::to_owned))
        .collect()
}

/// 組み合わせの判定（8.1・8.2）: Ctrl＋Shift の F12／F10 だけを拾う。
#[test]
fn only_ctrl_shift_f12_and_f10_without_alt_or_repeat_are_picked() {
    assert_eq!(
        shortcut_of(ctrl_shift(F12)),
        Some(PositionShortcut::ResetShellPosition)
    );
    assert_eq!(
        shortcut_of(ctrl_shift(F10)),
        Some(PositionShortcut::ResetBalloonPosition)
    );
    for vk in [F12, F10] {
        assert_eq!(
            shortcut_of(key(vk, true, true, true, false)),
            None,
            "Alt つき"
        );
        assert_eq!(
            shortcut_of(key(vk, true, false, false, false)),
            None,
            "Ctrl だけ"
        );
        assert_eq!(
            shortcut_of(key(vk, false, true, false, false)),
            None,
            "Shift だけ"
        );
        assert_eq!(
            shortcut_of(key(vk, false, false, false, false)),
            None,
            "修飾なし"
        );
        assert_eq!(
            shortcut_of(key(vk, true, true, false, true)),
            None,
            "繰り返し"
        );
    }
    assert_eq!(shortcut_of(ctrl_shift(0x7A)), None, "F11 は拾わない");
}

/// 組み合わせが合わない押下は消費せず、何もしない。
#[test]
fn other_keys_are_not_consumed() {
    let (mut world, rx) = world_with_ghost();
    world.insert_resource(basis());
    spawn_one_scope(&mut world);
    let e = world.spawn_empty().id();
    let (consumed, events) =
        capture_logs(|| on_ghost_key_down(&mut world, e, key(F12, true, true, true, false)));
    assert!(!consumed);
    assert!(rx.try_recv().is_err());
    assert!(events.is_empty(), "{events:?}");
}

/// F12（8.1）: メニューと同じ往復 —— `OnResetWindowPos` を送り、答えの受け手を置き、消費する。
#[test]
fn ctrl_shift_f12_raises_the_reset_round_trip() {
    let (mut world, rx) = world_with_ghost();
    spawn_one_scope(&mut world);
    let e = world.spawn_empty().id();
    assert!(on_ghost_key_down(&mut world, e, ctrl_shift(F12)));
    match rx.try_recv() {
        Ok(KanadeMsg::RaiseEvent { id, reply, .. }) => {
            assert_eq!(id, "OnResetWindowPos");
            assert!(reply.is_some(), "答えの受け手つき");
        }
        _ => panic!("OnResetWindowPos が答えの受け手つきで送られる"),
    }
    assert!(world.contains_non_send::<PendingResetRaise>());
}

/// F10（8.2・9.5）: バルーン位置初期化をきっかけ「近道」で呼び、ゴーストへは送らず、消費する。
#[test]
fn ctrl_shift_f10_resets_balloons_with_shortcut_trigger() {
    let (mut world, rx) = world_with_ghost();
    world.insert_resource(basis());
    spawn_one_scope(&mut world);
    let e = world.spawn_empty().id();
    let (consumed, events) = capture_logs(|| on_ghost_key_down(&mut world, e, ctrl_shift(F10)));
    assert!(consumed);
    let triggers = balloon_reset_triggers(&events);
    assert!(!triggers.is_empty(), "初期化を呼んだ: {events:?}");
    assert!(
        triggers.iter().all(|t| t.as_deref() == Some("shortcut")),
        "{triggers:?}"
    );
    assert!(named(&events, "position_shortcut_ignored").is_empty());
    assert!(rx.try_recv().is_err(), "ゴーストへ何も送らない");
}

/// 選べない場面（8.3）: 何もせず `position_shortcut_ignored` を 1 行記録し、押下は消費する。
#[test]
fn not_selectable_only_logs_and_consumes() {
    for (vk, name) in [
        (F12, "reset_shell_position"),
        (F10, "reset_balloon_position"),
    ] {
        // 窓の組が無い ⇒ どちらも選べない。
        let (mut world, rx) = world_with_ghost();
        let e = world.spawn_empty().id();
        let (consumed, events) = capture_logs(|| on_ghost_key_down(&mut world, e, ctrl_shift(vk)));
        assert!(consumed, "{name}");
        let ignored = named(&events, "position_shortcut_ignored");
        assert_eq!(ignored.len(), 1, "{name}: {events:?}");
        assert_eq!(ignored[0].level, tracing::Level::INFO);
        assert_eq!(ignored[0].field_str("shortcut"), Some(name));
        assert_eq!(ignored[0].field_str("reason"), Some("not_selectable"));
        assert!(balloon_reset_triggers(&events).is_empty(), "{name}");
        assert!(rx.try_recv().is_err(), "{name}: ゴーストへ何も送らない");
        assert!(!world.contains_non_send::<PendingResetRaise>(), "{name}");
    }

    // 答え待ちの間の F12 も選べない（メニューの灰色と同じ式）。
    let (mut world, rx) = world_with_ghost();
    spawn_one_scope(&mut world);
    let e = world.spawn_empty().id();
    assert!(on_ghost_key_down(&mut world, e, ctrl_shift(F12)));
    assert!(rx.try_recv().is_ok());
    let (consumed, events) = capture_logs(|| on_ghost_key_down(&mut world, e, ctrl_shift(F12)));
    assert!(consumed);
    assert_eq!(named(&events, "position_shortcut_ignored").len(), 1);
    assert!(rx.try_recv().is_err(), "2 度目は送らない");
}

/// すべてのゴーストの窓（キャラクターの窓とバルーンの窓）に受け手が付き、印なしの窓には付かない（8.4）。
#[test]
fn receivers_go_on_every_ghost_window_only() {
    type Receiver = fn(&mut World, Entity, KeyDown) -> bool;
    let mut world = World::new();
    let windows = spawn_one_scope(&mut world);
    let plain = world.spawn(wintf::ecs::Window::default()).id();
    attach_shortcut_receivers(&mut world);
    let has = |world: &World, e: Entity| {
        world
            .get::<OnKeyDown>(e)
            .is_some_and(|c| std::ptr::fn_addr_eq(c.0, on_ghost_key_down as Receiver))
    };
    let ghost: Vec<Entity> = world
        .query_filtered::<Entity, With<GhostWindowMarker>>()
        .iter(&world)
        .collect();
    assert_eq!(ghost.len(), 2, "キャラクターの窓とバルーンの窓");
    assert!(ghost.contains(&windows.char_window(0).unwrap()));
    assert!(ghost.contains(&windows.balloon_window(0).unwrap()));
    for e in ghost {
        assert!(has(&world, e), "{e:?}");
    }
    assert!(!has(&world, plain), "印なしの窓には付かない");
}
