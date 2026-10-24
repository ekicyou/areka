//! 読み取りだけの走りのインストールの断りの決定論テスト（readonly-mode・design「InstallRefusal」・
//! 要件 7.1・7.4・7.5・7.6・7.7・10.4・11.3）。
//!
//! 確かめること: World に [`ReadOnlyRun`] が在れば、窓への投げ込み・台本の `path`（窓口の取り出し
//! → 受付）・メニューの出どころのどの依頼も `ReadOnly` で断られ、待ち行列は空・窓口は忙しくない・
//! `readonly_refused`（`what = install`）が依頼ごとに 1 件・`install_order_refused` は出ない・
//! ゴーストへの送り物は 0 件。
//!
//! 背景のスレッドは起こさない（積まないので取り出しは渡す依頼を持たない）。印は試験の World に
//! 資源を直に足す（`readonly::engage` は呼ばない）。

use std::path::PathBuf;
use std::sync::mpsc;

use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::capture;
use tracing::Level;

use super::*;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::readonly::ReadOnlyRun;

/// どの出どころの依頼も断られ、積まれず、窓口は忙しくなく、記録は依頼ごとに info が 1 件で
/// 断りの warn は無く、ゴーストへは何も送らない。
#[test]
fn readonly_refuses_every_origin_before_queueing() {
    let mut world = World::new();
    world.init_resource::<Schedules>();
    register(&mut world);
    world.insert_resource(ReadOnlyRun);
    let (kanade_tx, kanade) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(kanade_tx),
        PathBuf::from("ghost/A"),
    ))));

    let ((drop_verdict, menu_verdict), events) = capture(|| {
        let drop_verdict = submit(
            &mut world,
            InstallOrder {
                archives: vec![PathBuf::from("C:/drop.nar")],
                origin: InstallOrigin::WindowDrop,
            },
        );
        // 台本の `path` は生の要求として窓口へ届き、取り出しが受付へ渡す。
        desk::raw_sender(&world)
            .send(RawInstallRequest {
                path: PathBuf::from("C:/script.nar"),
                origin: InstallOrigin::Script,
            })
            .expect("窓口の生の要求の受信端は生きている");
        desk::drain(&mut world);
        let menu_verdict = submit(
            &mut world,
            InstallOrder {
                archives: vec![PathBuf::from("C:/menu.nar")],
                origin: InstallOrigin::Menu,
            },
        );
        (drop_verdict, menu_verdict)
    });

    assert_eq!(drop_verdict, SubmitVerdict::ReadOnly);
    assert_eq!(menu_verdict, SubmitVerdict::ReadOnly);
    assert!(queued_orders(&world).is_empty(), "待ち行列に積まない");
    assert!(!desk::is_busy(&world), "窓口を忙しいに残さない");
    assert_eq!(kanade.try_iter().count(), 0, "ゴーストへは何も送らない");

    let refused: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("readonly_refused"))
        .collect();
    assert_eq!(refused.len(), 3, "依頼ごとに 1 件");
    let origins: Vec<_> = refused.iter().map(|e| e.field("origin")).collect();
    assert_eq!(
        origins,
        vec![Some("WindowDrop"), Some("Script"), Some("Menu")]
    );
    for e in &refused {
        assert_eq!(e.level, Level::INFO);
        assert_eq!(e.field_str("what"), Some("install"));
        assert_eq!(e.field("count"), Some("1"));
    }
    assert!(
        events
            .iter()
            .all(|e| e.field_str("event") != Some("install_order_refused")
                && e.field_str("event") != Some("install_order_queued")),
        "断りの warn も受け付けの記録も出さない"
    );
    assert_eq!(events.iter().filter(|e| e.level == Level::WARN).count(), 0);
}
