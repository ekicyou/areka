//! 投函の 4 行の記録の文の両面。
//!
//! - 投函の 4 行（`last_used_recorded`・`last_shell_recorded`・`last_balloon_recorded`・
//!   `install_balloon_remembered`）は、書けたときも書けないときも「書きました」「書き換えました」を含まない。
//!   書けないときだけ、致命の机の汲み出しが `memory_write_failed` を残す（書けたときは 0 件）。
//!
//! 記録は呼び手のスレッドで捕まえる（4 行は投函の直後に呼び手のスレッドで出る。`memory_write_failed`
//! は UI スレッドの汲み出しの系が出すので、捕まえる閉包の中で `Update` の段を回す）。
//! 根・検体はワークツリーの `target\` の下だけ。

use std::path::Path;

use areka_sylphya::SylphyaPublisher;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::{FrameTime, Update};

use crate::sample_test_support::switch_rig;
use areka_session::app_exit::FirstExit;
use areka_session::boot_config::BootContext;
use areka_session::boot_resolve::*;
use areka_session::ghost_session::GhostSlot;
use areka_session::ghost_switch_test_support::{FakeShiori, standard_script};

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
/// `memory_write_failed` が出る。
#[test]
fn post_lines_never_claim_written_and_failure_is_logged_only_when_unwritable() {
    let mut rig = switch_rig(vec![(
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
