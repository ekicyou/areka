//! 投げ込みの装着と受け手のテスト（design「Testing Strategy / areka `file_drop_wiring_tests.rs`」
//! テスト 13・16・18・要件 1.1〜1.4・1.6・1.8・3.1・3.3・8.1・9.2）。
//!
//! 受け手は本物の `on_ghost_files_dropped` を印つきの entity へ直に呼ぶ（wintf の腕から受け手までは
//! `drop_files_tests.rs` が固定済み）。依頼は `install::queued_orders` で待ち行列の中身と順を読む。

use std::sync::mpsc;

use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use wintf::ecs::window::OnFilesDropped;

use super::{attach_file_drop_receivers, on_ghost_files_dropped};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::install::{self, InstallOrder, InstallOrigin};
use crate::placement::spawn::{BalloonWindowMarker, CharWindowMarker, GhostWindowMarker};

type Receiver = fn(&mut World, bevy_ecs::entity::Entity, Vec<std::path::PathBuf>);

/// 受付の窓口を据えた World（依頼を待ち行列に積める）。
fn world_with_desk() -> World {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    install::register(&mut world);
    world
}

fn with_event<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn has_receiver(world: &World, e: bevy_ecs::entity::Entity) -> bool {
    world
        .get::<OnFilesDropped>(e)
        .is_some_and(|c| std::ptr::fn_addr_eq(c.0, on_ghost_files_dropped as Receiver))
}

/// テスト 13（要件 1.1〜1.3・9.2）: 印つきのキャラクター窓とバルーン窓に受け手が差さり、印なしの窓には
/// 差さらない。窓を足してもう 1 度呼ぶ（起こし直しの形）と新しい窓にも差さる。
#[test]
fn receivers_go_on_ghost_windows_only_and_again_on_reopened_ones() {
    let mut world = World::new();
    let chara = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();
    let balloon = world
        .spawn((GhostWindowMarker, BalloonWindowMarker { scope: 0 }))
        .id();
    let plain = world.spawn_empty().id();
    attach_file_drop_receivers(&mut world);
    assert_eq!(
        [chara, balloon, plain].map(|e| has_receiver(&world, e)),
        [true, true, false],
        "キャラクター窓・バルーン窓に差さり、印なしの窓には差さらない"
    );

    let chara2 = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();
    let balloon2 = world
        .spawn((GhostWindowMarker, BalloonWindowMarker { scope: 0 }))
        .id();
    attach_file_drop_receivers(&mut world);
    assert_eq!(
        [chara, balloon, chara2, balloon2, plain].map(|e| has_receiver(&world, e)),
        [true, true, true, true, false],
        "起こし直した窓にも差さる"
    );
}

/// テスト 16（要件 1.6・1.8）: 0 件の投げ込みは受け取りの記録 `count=0` の 1 行だけで、kanade へ
/// 送らず依頼も 0 件。
#[test]
fn empty_drop_writes_one_reception_line_and_nothing_else() {
    let mut world = world_with_desk();
    let (tx, rx) = mpsc::channel();
    let dir = TempPath::new("file-drop-empty");
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        dir.path().to_path_buf(),
    ))));
    let chara = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();

    let ((), events) = capture(|| on_ghost_files_dropped(&mut world, chara, Vec::new()));

    let received = with_event(&events, "file_drop_received");
    assert_eq!(received.len(), 1, "受け取りの記録はちょうど 1 行");
    let fields = received[0].fields_map();
    assert_eq!(
        ["scope", "count", "files", "dirs", "installs"].map(|k| fields.get(k).copied()),
        [Some("0"), Some("0"), Some("0"), Some("0"), Some("0")],
        "0 も書く"
    );
    assert!(fields.contains_key("elapsed_ms"), "所要の欄がある");
    assert_eq!(
        events
            .iter()
            .filter(|e| e.level <= tracing::Level::INFO)
            .count(),
        1,
        "受け取りの記録のほかに info 以上の記録が無い: {events:?}"
    );
    assert!(rx.try_recv().is_err(), "kanade へは何も送らない");
    assert_eq!(install::queued_orders(&world), Vec::<InstallOrder>::new());
}

/// テスト 18（要件 1.4・8.1）: 印の無い窓へ届いたら `file_drop_unknown_window` 1 件で戻り、受け取りの
/// 記録も送出も依頼も無い。
#[test]
fn window_without_marker_warns_once_and_does_nothing() {
    let mut world = world_with_desk();
    let (tx, rx) = mpsc::channel();
    let dir = TempPath::new("file-drop-unknown");
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        dir.path().to_path_buf(),
    ))));
    let plain = world.spawn(GhostWindowMarker).id();

    let ((), events) = capture(|| {
        on_ghost_files_dropped(&mut world, plain, vec![dir.path().join("a.txt")]);
    });

    let unknown = with_event(&events, "file_drop_unknown_window");
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].level, tracing::Level::WARN);
    assert_eq!(unknown[0].field("count"), Some("1"));
    assert!(with_event(&events, "file_drop_received").is_empty());
    assert!(rx.try_recv().is_err(), "kanade へは何も送らない");
    assert_eq!(install::queued_orders(&world), Vec::<InstallOrder>::new());
}

/// 要件 3.1・3.3: `install.txt` を持つ書庫をバルーン窓へ落とすと、出どころ「窓への投げ込み」の依頼が
/// ちょうど 1 つ積まれ、受け取りの記録は `installs=1`・スコープはバルーンの印の番号。
#[test]
fn installable_archive_becomes_one_window_drop_order() {
    let mut world = world_with_desk();
    let dir = TempPath::new("file-drop-install");
    let nar = dir.path().join("ghost.nar");
    sample_ghost_kit::NarBuilder::new()
        .file(
            "install.txt",
            b"charset,UTF-8\r\ntype,ghost\r\ndirectory,x\r\n",
        )
        .done()
        .write_to(&nar)
        .expect("書庫を置ける");
    let balloon = world
        .spawn((GhostWindowMarker, BalloonWindowMarker { scope: 1 }))
        .id();

    let ((), events) = capture(|| on_ghost_files_dropped(&mut world, balloon, vec![nar.clone()]));

    assert_eq!(
        install::queued_orders(&world),
        vec![InstallOrder {
            archives: vec![nar],
            origin: InstallOrigin::WindowDrop,
        }]
    );
    let received = with_event(&events, "file_drop_received");
    assert_eq!(received.len(), 1);
    let fields = received[0].fields_map();
    assert_eq!(
        ["scope", "count", "files", "dirs", "installs"].map(|k| fields.get(k).copied()),
        [Some("1"), Some("1"), Some("0"), Some("0"), Some("1")]
    );
    let queued = with_event(&events, "install_order_queued");
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].field("origin"), Some("WindowDrop"));
}
