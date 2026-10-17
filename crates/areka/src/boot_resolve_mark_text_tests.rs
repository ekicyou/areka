//! 記録の文の両面（areka-P0-write-failure-notice task 6.3・要件 4.1〜4.3・7.3・7.6）。
//!
//! - 投函の 4 行（`last_used_recorded`・`last_shell_recorded`・`last_balloon_recorded`・
//!   `install_balloon_remembered`）は、書けたときも書けないときも「書きました」「書き換えました」を含まない。
//!   書けないときだけ、致命の机の汲み出しが `memory_write_failed` を残す（書けたときは 0 件）。
//! - 印を消せない文（`session_mark_clear_degraded`）は、記憶に印が残るときだけ「前回落ちたとして」を含む。
//! - 印を残す文（`session_mark_kept`）は、記憶に印が在るときだけ「Ref6/7 付き」を含む。
//!
//! 記録は呼び手のスレッドで捕まえる（4 行は投函の直後に呼び手のスレッドで出る。`memory_write_failed`
//! は UI スレッドの汲み出しの系が出すので、捕まえる閉包の中で `Update` の段を回す）。
//! 根・検体はワークツリーの `target\` の下だけ。

use std::path::Path;

use areka_sylphya::SylphyaPublisher;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use wintf::ecs::{FrameTime, Update};

use super::*;
use crate::app_exit::FirstExit;
use crate::boot_config::BootContext;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::ghost_session::GhostSlot;

/// `event` の記録（`event` 欄で引く）。
fn events_named<'a>(events: &'a [CapturedEvent], event: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

/// `event` の記録がちょうど 1 件あり、その文を返す。
fn one_message(events: &[CapturedEvent], event: &str) -> String {
    let hits = events_named(events, event);
    assert_eq!(hits.len(), 1, "{event} はちょうど 1 件: {events:?}");
    hits[0].message().to_owned()
}

/// 置き場を普通のファイルに替える（書けない）。
fn unwritable(dir: &Path) {
    if dir.is_dir() {
        std::fs::remove_dir_all(dir).expect("置き場のフォルダを外す");
    }
    std::fs::write(dir, "x").expect("置き場の位置に普通のファイルを置く");
}

/// 置き場をフォルダへ戻す（書ける）。
fn writable(dir: &Path) {
    if dir.is_file() {
        std::fs::remove_file(dir).expect("置き場の位置のファイルを外す");
    }
    std::fs::create_dir_all(dir).expect("置き場のフォルダを作る");
}

/// 書き込みの場所を呼び、反映の柵の後に本番の `Update` の段（致命の机の汲み出し）を 1 回回す。
/// 呼び手のスレッドで出た記録を返す。
fn post_and_drain(
    world: &mut World,
    publisher: &SylphyaPublisher,
    write: &dyn Fn(&mut World),
) -> Vec<CapturedEvent> {
    world.remove_resource::<FirstExit>();
    let ((), events) = capture(|| {
        write(world);
        publisher.barrier().expect("記憶の書き手は生きている");
        world.insert_resource(FrameTime(dola::runtime::clock::now()));
        world.run_schedule(Update);
    });
    events
}

/// 投函の 4 行は書けた・書けないの両面で「書きました」「書き換えました」を含まず、書けないときだけ
/// `memory_write_failed` が出る（要件 4.1・7.3）。
#[test]
fn post_lines_never_claim_written_and_failure_is_logged_only_when_unwritable() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.wire_app_memory();
    rig.boot("A");
    let app = rig.app_dir();
    let ghost = rig
        .root
        .ghost_dir("A")
        .join("ghost")
        .join("master")
        .join("profile")
        .join("areka");
    let publisher = rig
        .world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|s| s.runtime())
        .expect("A の実行系が起きている")
        .sylphya_publisher()
        .clone();
    publisher.barrier().expect("起動の書き込みが済む");
    {
        // argv でない決め方（最後に使ったものを 3 つとも書く）。
        let mut ctx = rig.world.resource_mut::<BootContext>();
        ctx.current.ghost.route = GhostRoute::Memory;
        ctx.current.balloon.route = BalloonRoute::Memory;
    }

    let record_all = |w: &mut World| {
        let ctx = w.resource::<BootContext>();
        LastUsed {
            ghost: &ctx.current.ghost,
            balloon: &ctx.current.balloon,
            shell_folder: "master",
        }
        .record(&publisher);
    };
    let shell_only = |_: &mut World| record_last_shell(&publisher, "master");
    let balloon_only = |_: &mut World| record_last_balloon(&publisher, "b");
    let installed = |w: &mut World| crate::install::desk::remember_balloon(w, "b".to_owned());
    let lines: [(&str, &dyn Fn(&mut World)); 4] = [
        ("last_used_recorded", &record_all),
        ("last_shell_recorded", &shell_only),
        ("last_balloon_recorded", &balloon_only),
        ("install_balloon_remembered", &installed),
    ];

    let mut wrong: Vec<String> = Vec::new();
    for can_write in [true, false] {
        for (event, write) in lines {
            if can_write {
                writable(&app);
                writable(&ghost);
            } else {
                unwritable(&app);
                unwritable(&ghost);
            }
            let events = post_and_drain(&mut rig.world, &publisher, write);
            let message = one_message(&events, event);
            let failed = events_named(&events, "memory_write_failed").len();
            if message.contains("書きました") || message.contains("書き換えました") {
                wrong.push(format!(
                    "{event}（書ける={can_write}）が「書きました」か「書き換えました」: {message}"
                ));
            }
            if can_write == (failed > 0) {
                wrong.push(format!(
                    "{event}（書ける={can_write}）の memory_write_failed が {failed} 件"
                ));
            }
        }
    }
    writable(&app);
    writable(&ghost);
    assert!(
        wrong.is_empty(),
        "記録の文が起きたことと合わない: {wrong:#?}"
    );
    assert!(rig.shutdown(), "A を降ろせる");
}

/// 印の書き手が書いた記憶のファイル（置き場にただ 1 つ在るファイル）。
fn mark_file(app_dir: &Path) -> std::path::PathBuf {
    let files: Vec<_> = std::fs::read_dir(app_dir)
        .expect("置き場を読む")
        .map(|e| e.expect("置き場の項目").path())
        .filter(|p| p.is_file())
        .collect();
    assert_eq!(files.len(), 1, "置き場の記憶のファイル: {files:?}");
    files[0].clone()
}

/// 印を消せない文は、記憶に印が残るときだけ「前回落ちたとして」を含む（要件 4.2・7.3）。
#[test]
fn clear_degraded_foretells_crash_only_when_mark_remains() {
    let tmp = TempPath::under_target("mark-text-clear");

    // 印が残る: 印を書いた後、書き換えの一時ファイルの位置にフォルダを置いて置き換えを失敗させる
    // （本体は読めるまま）。
    let kept = tmp.child("kept");
    write_session_mark(&kept, "動いている子").expect("印を書く");
    let file = mark_file(&kept);
    let mut temp = file.clone().into_os_string();
    temp.push(".tmp");
    std::fs::create_dir_all(&temp).expect("一時ファイルの位置にフォルダを置く");
    let (cleared, kept_events) = capture(|| clear_session_mark(&kept));
    assert!(cleared.is_err(), "印を消せないはず");
    assert_eq!(read_session_mark(&kept), Some("動いている子".to_owned()));
    let kept_text = one_message(&kept_events, "session_mark_clear_degraded");

    // 印が無い: 置き場が普通のファイル（読めず書けない）。
    let gone = tmp.child("gone");
    unwritable(&gone);
    let (cleared, gone_events) = capture(|| clear_session_mark(&gone));
    assert!(cleared.is_err(), "印を消せないはず");
    assert_eq!(read_session_mark(&gone), None);
    let gone_text = one_message(&gone_events, "session_mark_clear_degraded");

    assert_eq!(
        (
            kept_text.contains("前回落ちたとして"),
            gone_text.contains("前回落ちたとして"),
        ),
        (true, false),
        "印が残る・無いの文: {kept_text} / {gone_text}"
    );
}

/// 印を残す文は、記憶に印が在るときだけ「Ref6/7 付き」を含む（要件 4.3・7.3）。
#[test]
fn keep_mark_foretells_ref6_7_only_when_mark_exists() {
    let tmp = TempPath::under_target("mark-text-keep");

    let marked = tmp.child("marked");
    write_session_mark(&marked, "動いている子").expect("印を書く");
    let ((), marked_events) = capture(|| keep_session_mark(&marked, "run_failed", None));
    let marked_text = one_message(&marked_events, "session_mark_kept");

    let empty = tmp.child("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let ((), empty_events) = capture(|| keep_session_mark(&empty, "run_failed", None));
    let empty_text = one_message(&empty_events, "session_mark_kept");

    assert_eq!(
        (
            marked_text.contains("Ref6/7 付き"),
            empty_text.contains("Ref6/7 付き"),
        ),
        (true, false),
        "印が在る・無いの文: {marked_text} / {empty_text}"
    );
}
