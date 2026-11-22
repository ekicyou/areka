//! シェル・バルーンの切替の入口の断りと、受理の送りの順の決定論テスト（GPU なし）。

use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;

use areka_emo_compose::BindSet;
use areka_seriko::{
    BindResolver, MockSurfaceOutput, SerikoLoopConfig, SurfaceResolver, spawn_seriko,
};
use log_capture_kit::{LineFormat, capture_lines};

use super::*;
use crate::body_systems::NO_BODY;
use crate::ghost_session::GhostSession;

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
        request_skin_switch(&mut world, menu_or_script(SkinOrigin::Menu))
    });
    let (script, script_logs) = capture_lines(LineFormat::LevelTargetFields, || {
        request_skin_switch(
            &mut world,
            menu_or_script(SkinOrigin::Script { raise_event: false }),
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

/// 起動の文脈・置き場（kanade の送出端と seriko の送り手）・結線をそろえた World と、kanade の受信端。
fn world_with_context() -> (World, mpsc::Receiver<KanadeMsg>) {
    let root = areka_ghost::BasewareRoot::new(PathBuf::from("root"));
    let (tx, kanade) = mpsc::channel();
    let seriko = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        areka_seriko::DefaultBinds::uniform(BindSet::default()),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        MockSurfaceOutput::new(),
    )
    .0;
    let mut world = World::new();
    world.insert_resource(crate::ghost_switch_test_support::boot_context(&root, "A"));
    world.insert_non_send(GhostSlot(Some(
        GhostSession::for_test(Some(tx), root.ghost_dir("A")).with_seriko_sink(seriko),
    )));
    world.insert_non_send(areka_runtime::frame::shared_test_support::wiring().1);
    (world, kanade)
}

fn balloon_named(name: &str) -> SkinRequest {
    SkinRequest {
        kind: SkinKind::Balloon,
        target: SkinSpec::Name(name.to_owned()),
        origin: SkinOrigin::Script { raise_event: false },
    }
}

/// 1 回の取り出しで同じ解けない名 3 件・解ける名・解けない名を渡すと、該当なしは 3 行・進行中は 1 行で、
/// 候補は 1 度だけ読む。受理の後は埋めた候補を捨て、次の要求で 1 度だけ読み直す。
#[test]
fn unresolved_names_in_one_drain_read_the_catalog_once() {
    let (mut world, _kanade) = world_with_context();
    let (tx, rx) = mpsc::channel();
    wire_switch_rx(&mut world, rx);
    for name in ["Nobody", "Nobody", "Nobody", "balloon2", "Nobody"] {
        tx.send(SkinRequestRaw {
            kind: SkinKind::Balloon,
            name: name.to_owned(),
            raise_event: false,
        })
        .expect("送れる");
    }
    let reads = Cell::new(0);
    let mut read = |_: SkinKind, _: &SwitchContext| {
        reads.set(reads.get() + 1);
        vec![candidate()]
    };

    let ((), logs) = capture_lines(LineFormat::LevelTargetFields, || {
        drain_switch_requests_with(&mut world, &mut read)
    });
    let drained = (
        lines_with(&logs, "skin_switch_unknown"),
        lines_with(&logs, "skin_switch_busy"),
        reads.get(),
    );
    world.remove_non_send::<SkinSwitchInFlight>();
    let mut catalog = [None, None];
    let accepted = request_skin_switch_with(
        &mut world,
        balloon_named("balloon2"),
        |_| 0,
        &mut catalog,
        &mut read,
    );
    world.remove_non_send::<SkinSwitchInFlight>();
    let after = request_skin_switch_with(
        &mut world,
        balloon_named("Nobody"),
        |_| 0,
        &mut catalog,
        &mut read,
    );

    assert_eq!(
        (drained, accepted, after, reads.get()),
        ((3, 1, 1), SkinVerdict::Accepted, SkinVerdict::NotFound, 3),
        "((取り出しの該当なし, 進行中, 読んだ回数), 受理, 受理の後, 読んだ回数の合計): {logs:?}"
    );
}
