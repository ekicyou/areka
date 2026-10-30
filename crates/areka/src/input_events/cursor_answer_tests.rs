//! `cursor_answer.rs` の兄弟テスト（shell-mouse-cursor tasks.md 4.2・要件 2.1・2.3・2.5・2.7・2.8・
//! 6.1・6.2・7.1）。
//!
//! 当たり判定は `RegionSource::Mock`、シェルのカーソルの資源は手で組む（ファイルは読まない）。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;
use windows::Win32::UI::WindowsAndMessaging::{HCURSOR, IDC_ARROW, IDC_HAND, IDC_HELP};
use wintf::ecs::cursor::{
    CursorAsk, CursorQuery, CursorReply, CursorTrigger, OwnedCursor, read_cur_directory,
    stock_cursor,
};
use wintf::ecs::pointer::PointerState;
use wintf::ecs::{PhysicalPoint, Window};

use super::super::super::{MouseWiring, RegionSource};
use super::super::char_watch;
use super::{BUNDLED, CharCursors, register_char_cursor};
use crate::emo2_boot::hit_region::HitRegion;
use crate::emo2_boot::shell_cursor_assets::ShellCursorAssets;
use crate::emo2_boot::shell_cursor_table::{CursorScene, Shape, SystemCursor};
use crate::input_events::hover_region::cursor_pick::Spot;
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker};

const TARGET: &str = "areka::input_events::char_watch::cursor_answer";

/// x < 50 → Bust、50 ≤ x < 100 → Head、それ以外 → 無し。
fn mock(scope: u32, x: i64, y: i64) -> HitRegion {
    let region = match x {
        ..50 => Some("Bust"),
        50..100 => Some("Head"),
        _ => None,
    };
    HitRegion {
        scope,
        region: region.map(str::to_owned),
        surface_point: (x, y),
    }
}

fn wired_world(assets: Option<ShellCursorAssets>) -> World {
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut world = World::new();
    world.insert_non_send(MouseWiring::with_clock(
        tx,
        RegionSource::Mock(mock),
        Box::new(|| 0),
    ));
    if let Some(a) = assets {
        world.insert_non_send(a);
    }
    register_char_cursor(&mut world);
    world
}

fn char_window(world: &mut World, scope: usize) -> Entity {
    world
        .spawn((Window::default(), CharWindowMarker { scope }))
        .id()
}

fn ask(trigger: CursorTrigger, x: i32) -> CursorAsk {
    CursorAsk {
        trigger,
        client: (x, 5),
        see_through: false,
        buttons_down: false,
        dpi: 96,
    }
}

fn answer(world: &World, window: Entity, ask: &CursorAsk) -> Option<CursorReply> {
    (world.resource::<CursorQuery>().0)(world, window, ask)
}

fn show(world: &World, window: Entity, x: i32) -> HCURSOR {
    match answer(world, window, &ask(CursorTrigger::Move, x)) {
        Some(CursorReply::Show(h)) => h,
        other => panic!("Show でない: {other:?}"),
    }
}

fn debugs(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.target == TARGET && e.level == Level::DEBUG)
        .collect()
}

#[test]
fn char_window_gets_the_expected_stock_shapes() {
    let mut assets = ShellCursorAssets::default();
    // スコープ 1 の Head だけ乗る形を help にする（スコープ 0 は既定のまま＝要件 2.7）。
    assets.rows.rows.insert(
        (1, "Head".into(), CursorScene::Up),
        Shape::System(SystemCursor::Help),
    );
    let mut world = wired_world(Some(assets));
    let w0 = char_window(&mut world, 0);
    let w1 = char_window(&mut world, 1);
    let finger = stock_cursor(IDC_HAND).unwrap();

    assert_eq!(show(&world, w0, 60), finger, "Head の既定は指差し");
    assert_eq!(show(&world, w0, 200), stock_cursor(IDC_ARROW).unwrap());
    // 当たり判定から当たり判定へ直に移ったその場で替わる（要件 2.3）。
    let hand = show(&world, w0, 10);
    assert_ne!(hand, finger, "Bust は同梱の手を開いた形");
    assert_eq!(show(&world, w0, 60), finger);
    assert_eq!(show(&world, w1, 60), stock_cursor(IDC_HELP).unwrap());
    assert_eq!(show(&world, w0, 61), finger, "別の窓の形に引きずられない");
}

#[test]
fn see_through_keeps_the_shape() {
    let mut world = wired_world(Some(ShellCursorAssets::default()));
    let w = char_window(&mut world, 0);
    let mut a = ask(CursorTrigger::Move, 60);
    a.see_through = true;
    assert_eq!(answer(&world, w, &a), Some(CursorReply::Keep));
}

#[test]
fn balloon_window_and_unwired_worlds_are_not_handled() {
    let mut world = wired_world(Some(ShellCursorAssets::default()));
    let balloon = world
        .spawn((Window::default(), BalloonWindowMarker { scope: 0 }))
        .id();
    assert_eq!(answer(&world, balloon, &ask(CursorTrigger::Move, 60)), None);

    // マウスの結線が無い世界。
    let mut world = World::new();
    world.insert_non_send(ShellCursorAssets::default());
    register_char_cursor(&mut world);
    let w = char_window(&mut world, 0);
    assert_eq!(answer(&world, w, &ask(CursorTrigger::Move, 60)), None);

    // シェルのカーソルの資源が無い世界。
    let mut world = wired_world(None);
    let w = char_window(&mut world, 0);
    assert_eq!(answer(&world, w, &ask(CursorTrigger::Move, 60)), None);
}

#[test]
fn debug_line_only_when_the_shape_changes() {
    let mut world = wired_world(Some(ShellCursorAssets::default()));
    let w = char_window(&mut world, 0);
    let (_, events) = capture(|| {
        show(&world, w, 60);
        // 同じ当たり判定の中で動いた: 形は変わらない。
        show(&world, w, 70);
        show(&world, w, 80);
        // 対照: 当たり判定の無い所へ出た。
        show(&world, w, 200);
    });
    let lines = debugs(&events);
    assert_eq!(lines.len(), 2, "{events:#?}");
    assert_eq!(lines[0].field_str("region"), Some("Head"));
    assert_eq!(lines[0].field_str("scene"), Some("Up"));
    assert_eq!(lines[0].field("origin"), Some("Some(Default)"));
    assert_eq!(lines[1].field_str("region"), None);
    assert_eq!(lines[1].field_str("scene"), Some("Arrow"));
}

#[test]
fn vanished_windows_are_pruned_on_every_answer() {
    let mut world = wired_world(Some(ShellCursorAssets::default()));
    let gone = char_window(&mut world, 0);
    let stay = char_window(&mut world, 1);
    show(&world, gone, 60);
    world.despawn(gone);
    show(&world, stay, 60);
    let cursors = world.non_send::<CharCursors>();
    let windows = cursors.windows.borrow();
    assert!(!windows.contains_key(&gone));
    assert!(windows.contains_key(&stay));
}

#[test]
fn watcher_and_answer_resolve_the_same_region_at_the_same_point() {
    let mut world = wired_world(Some(ShellCursorAssets::default()));
    let w = char_window(&mut world, 0);
    let child = world.spawn(ChildOf(w)).id();
    for x in [10, 60, 200] {
        world.entity_mut(child).insert(PointerState {
            client_point: PhysicalPoint::new(x, 5),
            ..Default::default()
        });
        char_watch(&mut world);
        let watched = world
            .non_send::<MouseWiring>()
            .hover
            .get(&w)
            .and_then(|h| h.region.clone());
        answer(&world, w, &ask(CursorTrigger::Move, x));
        let answered = match &world.non_send::<CharCursors>().windows.borrow()[&w].0.spot {
            Spot::Region(r) => Some(r.clone()),
            _ => None,
        };
        assert_eq!(watched, answered, "x = {x}");
    }
}

#[test]
fn bundled_art_that_cannot_be_made_falls_back_to_finger_once_per_size() {
    let world = wired_world(Some(ShellCursorAssets::default()));
    let cursors = world.non_send::<CharCursors>();
    let (got, events) = capture(|| {
        (
            cursors.bundled(0, b"broken", 32),
            cursors.bundled(0, b"broken", 32),
            cursors.bundled(0, b"broken", 64),
        )
    });
    let finger = Some(stock_cursor(IDC_HAND).unwrap());
    assert_eq!(got, (finger, finger, finger));
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.target == TARGET && e.level == Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 2, "寸法ごとに 1 度だけ");
}

#[test]
fn bundled_cur_files_have_ten_sizes_with_hotspots_inside() {
    for (shape, bytes) in BUNDLED {
        let entries = read_cur_directory(bytes).unwrap();
        let sizes: Vec<u32> = entries.iter().map(|e| e.width).collect();
        assert_eq!(
            sizes,
            [32, 40, 48, 56, 64, 72, 80, 88, 96, 128],
            "{shape:?}"
        );
        for e in &entries {
            assert_eq!(e.width, e.height);
            assert!(u32::from(e.hotspot.0) < e.width && u32::from(e.hotspot.1) < e.height);
        }
        for size in [32, 64] {
            OwnedCursor::from_cur_bytes(bytes, size).unwrap();
        }
    }
}

#[test]
fn shell_file_that_cannot_be_made_falls_back_to_the_scene_default() {
    use crate::emo2_boot::shell_cursor_assets::CursorFile;
    let mut assets = ShellCursorAssets::default();
    assets.files.push(CursorFile::Cur(b"broken".to_vec()));
    assets
        .rows
        .rows
        .insert((0, "Head".into(), CursorScene::Up), Shape::File(0));
    let mut world = wired_world(Some(assets));
    let w = char_window(&mut world, 0);
    assert_eq!(show(&world, w, 60), stock_cursor(IDC_HAND).unwrap());
}
