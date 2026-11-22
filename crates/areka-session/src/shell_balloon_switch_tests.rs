//! シェル・バルーンの切替の入口の断りと、受理の送りの順の決定論テスト（GPU なし）。

use std::cell::Cell;
use std::path::PathBuf;
use std::sync::mpsc;

use log_capture_kit::{LineFormat, capture_lines};

use super::*;
use crate::body_systems::NO_BODY;

fn menu_or_script(origin: SkinOrigin) -> SkinRequest {
    SkinRequest {
        kind: SkinKind::Balloon,
        target: SkinSpec::Folder("planted".to_owned()),
        origin,
    }
}

fn lines_with(logs: &[String], event: &str) -> usize {
    logs.iter().filter(|l| l.contains(event)).count()
}

/// インストール中はメニューの要求だけを `Installing` で断り、台本の要求は断らない
/// （文脈の無い World なので台本は `NoContext` まで進む）。
#[test]
fn installing_refuses_menu_requests_only() {
    let mut world = World::new();
    world.insert_resource(BodySystems {
        install_busy: |_| true,
        ..NO_BODY
    });

    let (menu, menu_logs) = capture_lines(LineFormat::LevelTargetFields, || {
        request_skin_switch_with(&mut world, menu_or_script(SkinOrigin::Menu), |_| 0)
    });
    let (script, script_logs) = capture_lines(LineFormat::LevelTargetFields, || {
        request_skin_switch_with(
            &mut world,
            menu_or_script(SkinOrigin::Script { raise_event: false }),
            |_| 0,
        )
    });

    assert_eq!(
        (
            menu,
            lines_with(&menu_logs, "skin_switch_installing"),
            script,
            lines_with(&script_logs, "skin_switch_installing"),
        ),
        (SkinVerdict::Installing, 1, SkinVerdict::NoContext, 0),
        "(メニューの判定, 断りの行, 台本の判定, 断りの行) が違う"
    );
}

fn context(kanade: mpsc::Sender<KanadeMsg>) -> SwitchContext {
    SwitchContext {
        roots: areka_ghost::BasewareRoots::single(PathBuf::from("root")),
        ghost_dir: PathBuf::from("ghost"),
        kanade,
        current_shell: None,
        current_balloon: None,
        balloon_dir: PathBuf::from("balloon"),
    }
}

fn candidate() -> SkinCandidate {
    SkinCandidate {
        dir: PathBuf::from("balloon2"),
        folder: "balloon2".to_owned(),
        name: None,
        hidden: false,
    }
}

/// 受理の後半を、kanade の受信端が生きている／落ちている送出端で 1 回ずつ回し、
/// (判定, 送れなかった行, 組み立ての回数, 印の有無) を返す。
fn accept_once(receiver_alive: bool) -> (SkinVerdict, usize, usize, bool) {
    let (tx, rx) = mpsc::channel::<KanadeMsg>();
    let _rx = receiver_alive.then_some(rx);
    let ctx = context(tx);
    let mut world = World::new();
    let spawned = Cell::new(0);
    let (verdict, logs) = capture_lines(LineFormat::LevelTargetFields, || {
        accept_skin_with(
            &mut world,
            &ctx,
            SkinKind::Balloon,
            candidate(),
            None,
            |_| {
                spawned.set(spawned.get() + 1);
                mpsc::channel().1
            },
        )
    });
    (
        verdict,
        lines_with(&logs, "skin_switch_send_failed"),
        spawned.get(),
        world.get_non_send::<SkinSwitchInFlight>().is_some(),
    )
}

/// kanade へ待ちを頼めなかった回は組み立てを起こさず印も置かない。頼めた回は 1 回だけ起こす。
#[test]
fn a_failed_gap_request_starts_no_build() {
    assert_eq!(
        (accept_once(false), accept_once(true)),
        (
            (SkinVerdict::NoContext, 1, 0, false),
            (SkinVerdict::Accepted, 0, 1, true),
        ),
        "(判定, 送れなかった行, 組み立ての回数, 印) が 送れない・送れる の順で違う"
    );
}
