//! 投げ込みの装着と受け手のテスト（design「Testing Strategy / areka `file_drop_wiring_tests.rs`」
//! テスト 13〜20・要件 1.1〜1.4・1.6・1.8・2.4・2.5・3.1〜3.3・4.1〜4.6・5.1・6.1〜6.5・7.2・8.1・
//! 9.2・9.4〜9.6）。
//!
//! 受け手は本物の `on_ghost_files_dropped` を印つきの entity へ直に呼ぶ（wintf の腕から受け手までは
//! `drop_files_tests.rs` が固定済み）。依頼は `install::queued_orders` で待ち行列の中身と順を読む。

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use areka_kanade::{KanadeMsg, ShioriMethod};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use wintf::ecs::window::OnFilesDropped;

use super::{ON_DIRECTORY_DROP, ON_FILE_DROP2, attach_file_drop_receivers, on_ghost_files_dropped};
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
    let plain = world.spawn(wintf::ecs::Window::default()).id();
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

/// 受信端に届いた `RaiseEvent` を届いた順に (id, references) で読む。`method = Get`・`reply = None`
/// でない物が来たら落とす（要件 4.1・5.1）。
fn raised(rx: &mpsc::Receiver<KanadeMsg>) -> Vec<(String, Vec<String>)> {
    rx.try_iter()
        .map(|msg| match msg {
            KanadeMsg::RaiseEvent {
                id,
                references,
                method,
                reply,
            } => {
                assert_eq!(method, ShioriMethod::Get, "{id} は GET で送る");
                assert!(reply.is_none(), "{id} は応えを待たない");
                (id, references)
            }
            _ => panic!("RaiseEvent 以外が届いた"),
        })
        .collect()
}

/// 置き場に受信端つきのゴーストを据えた World と、その受信端。
fn world_with_ghost(dir: &Path) -> (World, mpsc::Receiver<KanadeMsg>) {
    let mut world = world_with_desk();
    let (tx, rx) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        dir.to_path_buf(),
    ))));
    (world, rx)
}

/// 一時フォルダに実物のファイルを置く。
fn put_file(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, b"x").expect("ファイルを置ける");
    p
}

/// 一時フォルダに実物のフォルダを置く。
fn put_dir(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::create_dir(&p).expect("フォルダを置ける");
    p
}

/// 一時フォルダに実物の書庫を置く（`install` が真なら最上位に `install.txt` を持つ）。
fn put_archive(dir: &Path, name: &str, install: bool) -> PathBuf {
    let p = dir.join(name);
    let entry = if install { "install.txt" } else { "readme.txt" };
    sample_ghost_kit::NarBuilder::new()
        .file(entry, b"charset,UTF-8\r\ntype,ghost\r\ndirectory,x\r\n")
        .done()
        .write_to(&p)
        .expect("書庫を置ける");
    p
}

/// テスト 14（要件 1.4・4.3）: バルーン窓へ落とすと、`OnFileDrop2` の Reference1 はバルーンの印の番号。
#[test]
fn drop_on_balloon_window_carries_its_scope() {
    let dir = TempPath::new("file-drop-scope");
    let (mut world, rx) = world_with_ghost(dir.path());
    let balloon = world
        .spawn((GhostWindowMarker, BalloonWindowMarker { scope: 1 }))
        .id();
    let a = put_file(dir.path(), "a.txt");

    on_ghost_files_dropped(&mut world, balloon, vec![a]);

    let sent = raised(&rx);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, ON_FILE_DROP2);
    assert_eq!(sent[0].1[1], "1", "Reference1 はバルーンの印のスコープ番号");
}

/// テスト 15（要件 2.6・3.1・3.3・4.1〜4.6・5.1・5.3・6.1・6.2・6.5・8.1・9.4・9.5）: ファイル 2・フォルダ 2・
/// 書庫 2 を混ぜて落とすと、`OnFileDrop2` 1 回 → `OnDirectoryDrop` を落とされた順に 1 つずつ → 依頼 1 つ。
/// `install.txt` の無い書庫は `OnFileDrop2` に `application/zip` で載る。
#[test]
fn mixed_drop_sends_files_then_dirs_then_one_order() {
    let dir = TempPath::new("file-drop-mixed");
    let (mut world, rx) = world_with_ghost(dir.path());
    let chara = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();
    let png = put_file(dir.path(), "a.png");
    let d1 = put_dir(dir.path(), "d1");
    let ghost = put_archive(dir.path(), "ghost.nar", true);
    let d2 = put_dir(dir.path(), "d2");
    let bare = put_file(dir.path(), "b");
    let plain_zip = put_archive(dir.path(), "plain.zip", false);
    let paths = vec![
        png.clone(),
        d1.clone(),
        ghost.clone(),
        d2.clone(),
        bare.clone(),
        plain_zip.clone(),
    ];

    let ((), events) = capture(|| on_ghost_files_dropped(&mut world, chara, paths));

    let sent = raised(&rx);
    let lossy = |p: &PathBuf| p.to_string_lossy().into_owned();
    assert_eq!(
        sent,
        vec![
            (
                ON_FILE_DROP2.to_owned(),
                vec![
                    [lossy(&png), lossy(&bare), lossy(&plain_zip)].join("\u{1}"),
                    "0".to_owned(),
                    "image/png\u{1}\u{1}application/zip".to_owned(),
                ],
            ),
            (
                ON_DIRECTORY_DROP.to_owned(),
                vec![lossy(&d1), "0".to_owned()]
            ),
            (
                ON_DIRECTORY_DROP.to_owned(),
                vec![lossy(&d2), "0".to_owned()]
            ),
        ],
        "OnFileDrop2 → OnDirectoryDrop（1 つ目）→ OnDirectoryDrop（2 つ目）の順"
    );
    assert_eq!(
        install::queued_orders(&world),
        vec![InstallOrder {
            archives: vec![ghost],
            origin: InstallOrigin::WindowDrop,
        }],
        "依頼はちょうど 1 つ・出どころは窓への投げ込み"
    );
    let received = with_event(&events, "file_drop_received");
    assert_eq!(received.len(), 1);
    let fields = received[0].fields_map();
    assert_eq!(
        ["count", "files", "dirs", "installs"].map(|k| fields.get(k).copied()),
        [Some("6"), Some("3"), Some("2"), Some("1")]
    );
    let sent_lines: Vec<_> = with_event(&events, "file_drop_event_sent")
        .iter()
        .map(|e| (e.level, e.field_str("id").map(str::to_owned)))
        .collect();
    assert_eq!(
        sent_lines,
        [ON_FILE_DROP2, ON_DIRECTORY_DROP, ON_DIRECTORY_DROP]
            .map(|id| (tracing::Level::INFO, Some(id.to_owned())))
            .to_vec(),
        "送ったイベント 1 件につき info 1 行"
    );
    // 依頼は知らせの後（要件 6.2）: 記録の並びで、送った最後の行より後に依頼の受付が来る。
    let last_of = |name: &str| {
        events
            .iter()
            .rposition(|e| e.field_str("event") == Some(name))
            .expect("記録がある")
    };
    assert!(last_of("file_drop_event_sent") < last_of("install_order_queued"));
}

/// テスト 17（要件 6.3・8.1・9.6）: 送り口が無い（置き場にゴーストが居ない）と、イベント 1 件につき
/// `file_drop_no_kanade` の warn 1 件で送らず、インストール対象の依頼は渡す（要件 3.8）。
#[test]
fn no_kanade_warns_once_per_event_and_still_hands_the_order() {
    let dir = TempPath::new("file-drop-no-kanade");
    let mut world = world_with_desk();
    let chara = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();
    let paths = vec![
        put_file(dir.path(), "a.txt"),
        put_dir(dir.path(), "d1"),
        put_dir(dir.path(), "d2"),
        put_archive(dir.path(), "ghost.nar", true),
    ];

    let ((), events) = capture(|| on_ghost_files_dropped(&mut world, chara, paths));

    let no_kanade: Vec<_> = with_event(&events, "file_drop_no_kanade")
        .iter()
        .map(|e| (e.level, e.field_str("id").map(str::to_owned)))
        .collect();
    assert_eq!(
        no_kanade,
        [ON_FILE_DROP2, ON_DIRECTORY_DROP, ON_DIRECTORY_DROP]
            .map(|id| (tracing::Level::WARN, Some(id.to_owned())))
            .to_vec(),
        "イベント 1 件につき 1 件"
    );
    assert!(with_event(&events, "file_drop_event_sent").is_empty());
    assert_eq!(install::queued_orders(&world).len(), 1, "依頼は渡す");
}

/// テスト 17b（要件 6.3・8.1）: kanade が止まっている（受信端が落ちている）と、イベント 1 件につき
/// `file_drop_send_failed` の warn 1 件。送り口なしの記録は出ず、依頼は渡す。
#[test]
fn stopped_kanade_warns_send_failed_once_per_event() {
    let dir = TempPath::new("file-drop-stopped");
    let (mut world, rx) = world_with_ghost(dir.path());
    drop(rx);
    let chara = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();
    let paths = vec![
        put_file(dir.path(), "a.txt"),
        put_dir(dir.path(), "d1"),
        put_archive(dir.path(), "ghost.nar", true),
    ];

    let ((), events) = capture(|| on_ghost_files_dropped(&mut world, chara, paths));

    let failed = with_event(&events, "file_drop_send_failed");
    assert_eq!(failed.len(), 2, "イベント 1 件につき 1 件: {events:?}");
    assert!(failed.iter().all(|e| e.level == tracing::Level::WARN));
    assert!(with_event(&events, "file_drop_no_kanade").is_empty());
    assert!(with_event(&events, "file_drop_event_sent").is_empty());
    assert_eq!(install::queued_orders(&world).len(), 1, "依頼は渡す");
}

/// テスト 19（要件 2.4・2.5・8.1）: 存在しない `x.nar` はフォルダかどうかを問えず
/// （`file_drop_probe_failed`）、目次も読めず（`file_drop_archive_unreadable`・理由は `Io`）、
/// インストール対象でないファイルとして `OnFileDrop2` に載る。
#[test]
fn missing_archive_is_recorded_and_goes_to_file_drop2() {
    let dir = TempPath::new("file-drop-missing");
    let (mut world, rx) = world_with_ghost(dir.path());
    let chara = world
        .spawn((GhostWindowMarker, CharWindowMarker { scope: 0 }))
        .id();
    let missing = dir.path().join("x.nar");

    let ((), events) = capture(|| on_ghost_files_dropped(&mut world, chara, vec![missing.clone()]));

    let probe = with_event(&events, "file_drop_probe_failed");
    assert_eq!(probe.len(), 1);
    assert_eq!(probe[0].level, tracing::Level::WARN);
    let unreadable = with_event(&events, "file_drop_archive_unreadable");
    assert_eq!(unreadable.len(), 1);
    assert_eq!(unreadable[0].level, tracing::Level::WARN);
    // `reason` は `NarError` の Display。`Io` の変種は「I/O に失敗」と書く。
    assert!(
        unreadable[0]
            .field("reason")
            .is_some_and(|r| r.contains("I/O に失敗")),
        "理由は Io: {:?}",
        unreadable[0].field("reason")
    );
    let sent = raised(&rx);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, ON_FILE_DROP2);
    assert_eq!(sent[0].1[0], missing.to_string_lossy());
    assert_eq!(install::queued_orders(&world), Vec::<InstallOrder>::new());
}

/// テスト 20（要件 7.1・7.2）: 本番の 2 ファイルに、送らない正典の 15 語が引用符つきの文字列リテラルで
/// 0 件。較正として送る 2 語はちょうど 1 件ずつ在る（0 件の主張が空振りでない）。テストのファイルは
/// 読まない（2 語の字面を持つ）。
#[test]
fn production_sources_name_only_the_two_sent_events() {
    let sources = [
        include_str!("file_drop.rs"),
        include_str!("../../../wintf/src/ecs/window_proc/drop_files.rs"),
    ];
    let count = |word: &str| {
        let lit = format!("\"{word}\"");
        sources
            .iter()
            .map(|s| s.matches(lit.as_str()).count())
            .sum::<usize>()
    };
    assert_eq!(
        ["OnFileDrop2", "OnDirectoryDrop"].map(count),
        [1, 1],
        "較正: 送る 2 語はちょうど 1 件ずつ"
    );
    let not_sent = [
        "OnFileDropping",
        "OnFileDrop",
        "OnFileDropEx",
        "OnFileDropped",
        "OnArchiveViewerOpen",
        "OnMediaPlayerOpen",
        "OnPictureViewerOpen",
        "OnTextDrop",
        "OnURLDropping",
        "OnURLDropped",
        "OnURLDropFailure",
        "OnOtherObjectDropping",
        "OnOtherObjectDropped",
        "OnNarCreating",
        "OnNarCreated",
    ];
    // 送らない語は「直後が `"`」で数える（`"SSP: OnTextDrop"` のような字面も拾う）
    let tail = |word: &str| {
        let lit = format!("{word}\"");
        sources
            .iter()
            .map(|s| s.matches(lit.as_str()).count())
            .sum::<usize>()
    };
    let found: Vec<_> = not_sent.iter().filter(|w| tail(w) > 0).collect();
    assert!(found.is_empty(), "送らない語が本番に在る: {found:?}");
}
