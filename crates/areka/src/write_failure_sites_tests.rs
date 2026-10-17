//! 13 種の書き込みが致命の机へ届く行列（areka-P0-write-failure-notice task 6.1・要件 1.1・1.2・7.1・7.6）。
//!
//! 要件 1 の表の 13 種を 1 つずつ書けなくし、失敗が机へ届くことを判定する。
//! - 投函の物（動いているゴーストの記憶の書き手を通る）: 本物の実行系を 1 体起こし（その書き手は
//!   致命の机の送り手つきで起きる）、書く置き場を普通のファイルに替えてから書き込みの場所を呼ぶ。
//!   反映の柵の後、机に届いていない（最初の出所が無い）ことを見てから汲み出しの系を 1 回回し、
//!   最初の出所が `MemoryWriteFailed` になることを判定する。
//! - 直の物（UI スレッドが置き場へ直に書く）: 置き場を普通のファイルにして書き込みの場所を呼び、
//!   途中の物は最初の出所が `MemoryWriteFailed`、起動の時点は起動を止める口、後始末は告知を選ぶ口が
//!   失敗を返すことを判定する。
//!
//! 置き場の並びは、ゴーストが居る（投函）と居ない（直）の両方を踏む。判定した種類の並びが
//! 要件 1 の表の 13 種と等しいことも判定する。根・検体はワークツリーの `target\` の下だけ。

use std::path::Path;

use areka_ghost::prop_sink::PropSetCueSink;
use areka_kanade::{ChangeHandoff, KanadeNotice};
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use areka_sylphya::{PersistKey, PersistScope, SylphyaPublisher};
use temp_path_kit::TempPath;

use super::*;
use crate::app_exit::FirstExit;
use crate::boot_config::BootContext;
use crate::boot_resolve::{
    BalloonRoute, GhostRoute, LastUsed, record_last_balloon, record_last_shell,
};
use crate::emo2_boot::ghost_switch::{
    PrevGhost, SwitchInFlight, SwitchStage, SwitchTarget, WelcomeAttempt, on_notice, switch_to,
};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::ghost_session::{GhostSlot, StartupDescriptValues};
use crate::placement::AuthorDpi;
use crate::placement::persist::{char_pos_entries, persist_entries};
use crate::placement::resolver::PointPx;
use crate::{MarkInputs, Teardown, boot_first_ghost, insert_persist_wiring, settle_session_mark};

/// 要件 1 の表の 13 種（1.1 のアプリの記憶 6 種＋1.2 のゴーストごとの記憶 7 種）。
const KINDS: [&str; 13] = [
    "起動中の印を書く",
    "降ろした直後",
    "定常の印",
    "印を消す",
    "最後に使ったゴースト",
    "置き場の並び",
    "最後に使ったバルーン",
    "最後に使ったシェル",
    "最後のシェルだけ",
    "最後のバルーンだけ",
    "入れたバルーン",
    "窓の位置",
    "ゴーストのプロパティ",
];

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

/// 最初の出所の失敗の中身（`MemoryWriteFailed` でなければ `None`）。
fn first_failure(world: &World) -> Option<PersistFailure> {
    match world.get_resource::<FirstExit>().map(|f| &f.0) {
        Some(ExitOrigin::MemoryWriteFailed(failure)) => Some(failure.clone()),
        _ => None,
    }
}

/// 投函の物: 書き込みの場所を呼び、反映の柵の後に机の汲み出しを 1 回回して最初の出所を読む。
/// 汲み出しの前に最初の出所が在れば、場所が机を経ずに終わらせた（投函の道を通っていない）ので `None`。
fn posted(
    world: &mut World,
    publisher: &SylphyaPublisher,
    write: impl FnOnce(&mut World),
) -> Option<PersistFailure> {
    world.remove_resource::<FirstExit>();
    write(world);
    publisher.barrier().expect("記憶の書き手は生きている");
    if world.contains_resource::<FirstExit>() {
        return None;
    }
    drain_system(world);
    first_failure(world)
}

/// 直の物（途中）: 書き込みの場所を呼び、最初の出所を読む。
fn direct(world: &mut World, write: impl FnOnce(&mut World)) -> Option<PersistFailure> {
    world.remove_resource::<FirstExit>();
    write(world);
    first_failure(world)
}

/// 今のゴースト・バルーンの決め方を書き換える（argv で決まった側は最後に使ったものを書かない）。
fn routes(world: &mut World, ghost: GhostRoute, balloon: BalloonRoute) {
    let mut ctx = world.resource_mut::<BootContext>();
    ctx.current.ghost.route = ghost;
    ctx.current.balloon.route = balloon;
}

fn reservation(stage: SwitchStage) -> SwitchInFlight {
    SwitchInFlight {
        target: SwitchTarget {
            dir: "ghost/B".into(),
            folder: "B".to_owned(),
            name: "B".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: "ghost/A".into(),
            name: Some("A".to_owned()),
            sakura_name: None,
        },
        stage,
        boot_event: None,
        copy: crate::emo2_boot::switch_copy::SwitchCopy::None,
    }
}

/// `\![areka.prop.set,<key>,<値>]` の汎用キャリア cue。
fn prop_set_cue(key: PersistKey, value: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("sakura"),
        command: CueCommand::command_carrier(
            areka_ghost::prop_sink::PROP_SET_CUE_NAME,
            vec![key.to_canonical_key(), value.to_owned()],
        ),
        duration: 0.0,
    }
}

#[test]
fn every_write_kind_reaches_the_desk() {
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
    writable(&app);
    writable(&ghost);
    let picked = TempPath::under_target("write-failure-sites-picked");
    let (present_root, absent_root) = (picked.child("present"), picked.child("absent"));
    std::fs::create_dir_all(&present_root).unwrap();
    std::fs::create_dir_all(&absent_root).unwrap();

    // (種類, 期待のスコープ, 期待の key, 机に届いた失敗)
    let mut judged: Vec<(&str, PersistScope, PersistKey, Option<PersistFailure>)> = Vec::new();
    let world = &mut rig.world;

    // --- 投函（ゴーストの記憶の書き手を通る）: アプリの記憶 ---
    unwritable(&app);
    routes(world, GhostRoute::Memory, BalloonRoute::Argv);
    let got = posted(world, &publisher, |w| {
        let ctx = w.resource::<BootContext>();
        LastUsed {
            ghost: &ctx.current.ghost,
            balloon: &ctx.current.balloon,
            shell_folder: "master",
        }
        .record(&publisher);
    });
    judged.push((
        "最後に使ったゴースト",
        PersistScope::App,
        PersistKey::LastGhost,
        got,
    ));

    routes(world, GhostRoute::Argv, BalloonRoute::Argv);
    world.insert_non_send(reservation(SwitchStage::Welcoming {
        attempt: WelcomeAttempt::Target,
    }));
    let got = posted(world, &publisher, |w| on_notice(w, KanadeNotice::Steady));
    judged.push(("定常の印", PersistScope::App, PersistKey::LastRunning, got));

    world.resource_mut::<BootContext>().roots_origin = crate::roots::RootsOrigin::Default;
    let got = posted(world, &publisher, |w| {
        crate::roots::add_root(w, present_root.clone());
    });
    judged.push((
        "置き場の並び",
        PersistScope::App,
        PersistKey::BasewareRoots,
        got,
    ));
    writable(&app);

    // --- 投函: ゴーストごとの記憶 ---
    unwritable(&ghost);
    routes(world, GhostRoute::Argv, BalloonRoute::Memory);
    let got = posted(world, &publisher, |w| {
        let ctx = w.resource::<BootContext>();
        LastUsed {
            ghost: &ctx.current.ghost,
            balloon: &ctx.current.balloon,
            shell_folder: "master",
        }
        .record(&publisher);
    });
    judged.push((
        "最後に使ったバルーン",
        PersistScope::Ghost,
        PersistKey::LastBalloon,
        got,
    ));

    routes(world, GhostRoute::Argv, BalloonRoute::Argv);
    let got = posted(world, &publisher, |w| {
        let ctx = w.resource::<BootContext>();
        LastUsed {
            ghost: &ctx.current.ghost,
            balloon: &ctx.current.balloon,
            shell_folder: "master",
        }
        .record(&publisher);
    });
    judged.push((
        "最後に使ったシェル",
        PersistScope::Ghost,
        PersistKey::LastShell,
        got,
    ));

    let got = posted(world, &publisher, |_| {
        record_last_shell(&publisher, "master")
    });
    judged.push((
        "最後のシェルだけ",
        PersistScope::Ghost,
        PersistKey::LastShell,
        got,
    ));

    let got = posted(world, &publisher, |_| record_last_balloon(&publisher, "b"));
    judged.push((
        "最後のバルーンだけ",
        PersistScope::Ghost,
        PersistKey::LastBalloon,
        got,
    ));

    let got = posted(world, &publisher, |w| {
        crate::install::desk::remember_balloon(w, "b".to_owned());
    });
    judged.push((
        "入れたバルーン",
        PersistScope::Ghost,
        PersistKey::LastBalloon,
        got,
    ));

    insert_persist_wiring(world, publisher.clone());
    let got = posted(world, &publisher, |w| {
        persist_entries(w, char_pos_entries(0, PointPx { x: 10, y: 20 }));
    });
    let window_x = PersistKey::WindowPos {
        scope: 0,
        axis: areka_sylphya::Axis::X,
    };
    judged.push(("窓の位置", PersistScope::Ghost, window_x, got));

    let got = posted(world, &publisher, |_| {
        PropSetCueSink::new(publisher.clone()).emit(prop_set_cue(PersistKey::BootCount, "7"));
    });
    judged.push((
        "ゴーストのプロパティ",
        PersistScope::Ghost,
        PersistKey::BootCount,
        got,
    ));
    writable(&ghost);

    // 降ろす（書ける置き場で降ろし、降ろす途中の書き込みを机に残さない）。
    assert!(rig.shutdown(), "A を降ろせる");
    let world = &mut rig.world;
    drain_system(world);

    // --- 直（UI スレッドが置き場へ直に書く）: アプリの記憶 ---
    unwritable(&app);
    let got = direct(world, |w| crate::roots::add_root(w, absent_root.clone()));
    judged.push((
        "置き場の並び",
        PersistScope::App,
        PersistKey::BasewareRoots,
        got,
    ));

    world.insert_non_send(reservation(SwitchStage::SendOff));
    let got = direct(world, |w| switch_to(w, ChangeHandoff { script: None }));
    judged.push((
        "降ろした直後",
        PersistScope::App,
        PersistKey::LastGhost,
        got,
    ));

    // 印を消す: 後始末が失敗を受け、告知を選ぶ口が失敗を返す（終了コード 1）。
    let (verdict, failed) = settle_session_mark(
        &MarkInputs {
            app_profile_dir: app.clone(),
            argv_session: false,
            first: Some(ExitOrigin::Escape),
            logsink_fallback: false,
        },
        Teardown {
            run_ok: true,
            down_ok: true,
            shiori_cut: false,
        },
    );
    assert_eq!(verdict, crate::MarkVerdict::Clear);
    let told = tell_at_close(None, failed.clone(), None, true);
    judged.push((
        "印を消す",
        PersistScope::App,
        PersistKey::LastRunning,
        failed.filter(|_| told.is_err()),
    ));

    // 起動中の印を書く: 起こす前に書けず、起動を止める口が失敗を返す。ゴーストは起こさない。
    let (cfg, ghost_decision, balloon_decision) = {
        let c = &world.resource::<BootContext>().current;
        (c.cfg.clone(), c.ghost.clone(), c.balloon.clone())
    };
    let descript = StartupDescriptValues {
        author_dpi: AuthorDpi::DEFAULT,
        zorder_raw: None,
    };
    let boot = boot_first_ghost(
        world,
        &cfg,
        areka_kanade::BootOrigin::Plain,
        &descript,
        &ghost_decision,
        &balloon_decision,
    );
    let slot_empty = world
        .get_non_send::<GhostSlot>()
        .is_none_or(|slot| slot.0.is_none());
    let got = boot.err().filter(|failure| {
        stop_boot_with(failure.clone(), true).code() == windows::Win32::Foundation::E_FAIL
            && slot_empty
    });
    judged.push((
        "起動中の印を書く",
        PersistScope::App,
        PersistKey::LastRunning,
        got,
    ));
    writable(&app);

    // 判定: どの種類も、期待のスコープと key の失敗が机に届いた。
    let missed: Vec<String> = judged
        .iter()
        .filter(|(_, scope, key, got)| {
            !got.as_ref()
                .is_some_and(|f| f.scope == *scope && f.keys.contains(key))
        })
        .map(|(kind, scope, key, got)| format!("{kind}（{scope:?}・{key:?}）: {got:?}"))
        .collect();
    assert!(missed.is_empty(), "机に届かなかった書き込み: {missed:#?}");

    // 判定した種類の数が要件 1 の表の 13 種と等しい（置き場の並びは居る・居ないの 2 回で 1 種）。
    let mut kinds: Vec<&str> = judged.iter().map(|(kind, ..)| *kind).collect();
    kinds.sort_unstable();
    kinds.dedup();
    let mut table = KINDS.to_vec();
    table.sort_unstable();
    assert_eq!(kinds, table, "判定した種類が要件 1 の表と合わない");
    assert_eq!(kinds.len(), 13);
}
