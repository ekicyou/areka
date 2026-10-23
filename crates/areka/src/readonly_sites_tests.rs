//! 13 種の書き込みを読み取りだけで踏む行列（readonly-mode 要件 2.1・2.3・6.1・6.2・10.1）。
//!
//! 書けないときの告知の行列（`write_failure_sites_tests.rs`）と同じ表 [`KINDS`] を読み、検体の根に
//! 試験の取っ手（[`WithholdGuard::under`]）を立てて、13 種の書き込みの場所を 1 つずつ呼ぶ。
//! どの種類も、次の 3 つを判定する。
//! - 記憶のファイルの中身と置き場の木が、呼ぶ前と同じ（要件 2.1）
//! - 保存の口（`load_scope`・`FsPersistIo`）から期待の値が読み返せる（要件 2.3）
//! - 致命の机に何も届かない（最初の出所が無い・要件 6.1・6.2）
//!
//! 判定した種類の集まりが表と等しいことも判定する（1 種でも抜けたら赤）。ふつうの起動で書くことは、
//! 同じ表を使う `every_write_kind_reaches_the_desk` が判定する。プロセスに 1 つの旗は立てない
//! （取っ手だけで控える・構造の試験が固める）。根はワークツリーの `target\` の下だけ。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_ghost::prop_sink::PropSetCueSink;
use areka_kanade::{ChangeHandoff, KanadeNotice};
use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::persist::withhold::WithholdGuard;
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, SylphyaPublisher, load_scope};
use bevy_ecs::prelude::*;
use temp_path_kit::TempPath;

use crate::app_exit::{ExitOrigin, FirstExit};
use crate::boot_config::BootContext;
use crate::boot_resolve::{
    BalloonRoute, DEFAULT_GHOST_FOLDER, GhostRoute, LastUsed, record_last_balloon,
    record_last_shell, running_name,
};
use crate::emo2_boot::ghost_switch::{
    PrevGhost, SwitchInFlight, SwitchStage, SwitchTarget, WelcomeAttempt, on_notice, switch_to,
};
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::ghost_session::{GhostSlot, StartupDescriptValues};
use crate::placement::AuthorDpi;
use crate::placement::persist::{char_pos_entries, persist_entries};
use crate::placement::resolver::PointPx;
use crate::write_failure::sites_tests::{KINDS, drain_desk};
use crate::{MarkInputs, Teardown, boot_first_ghost, insert_persist_wiring, settle_session_mark};

/// 根の下の木（相対パス → ファイルの中身。フォルダは `None`）。
fn tree(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(dir).expect("木を歩く") {
            let path = entry.expect("ディレクトリ項目").path();
            let rel = path.strip_prefix(root).expect("根の下").to_path_buf();
            if path.is_dir() {
                out.insert(rel, None);
                walk(root, &path, out);
            } else {
                out.insert(rel, Some(std::fs::read(&path).expect("ファイルを読む")));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    assert!(!out.is_empty(), "走査が空振りした: {}", root.display());
    out
}

/// 保存の口から 1 つの key を読み返す。
fn read_back(app: &Path, ghost: &Path, scope: PersistScope, key: PersistKey) -> Option<String> {
    let roots = ScopeRoots {
        app: Some(app.to_path_buf()),
        ghost: Some(ghost.to_path_buf()),
        ..ScopeRoots::default()
    };
    load_scope(scope, &roots, &FsPersistIo)
        .into_iter()
        .find_map(|(k, v)| (k == key).then_some(v))
}

/// 書き込みの場所を呼び、記憶の書き手の反映の柵と机の汲み出しを 1 回回して、机に届いたか
/// （最初の出所が在るか）を返す。
fn reached_desk(
    world: &mut World,
    publisher: Option<&SylphyaPublisher>,
    write: impl FnOnce(&mut World),
) -> bool {
    write(world);
    if let Some(publisher) = publisher {
        publisher.barrier().expect("記憶の書き手は生きている");
    }
    drain_desk(world);
    world.contains_resource::<FirstExit>()
}

fn routes(world: &mut World, ghost: GhostRoute, balloon: BalloonRoute) {
    let mut ctx = world.resource_mut::<BootContext>();
    ctx.current.ghost.route = ghost;
    ctx.current.balloon.route = balloon;
}

/// 切替の予約（切替先 B は実際に起こすので、根の下の本物のフォルダを指す）。
fn reservation(rig: &SwitchRig, stage: SwitchStage) -> SwitchInFlight {
    SwitchInFlight {
        target: SwitchTarget {
            dir: rig.root.ghost_dir("B"),
            folder: "B".to_owned(),
            name: "B".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: rig.root.ghost_dir("A"),
            name: Some("A".to_owned()),
            sakura_name: None,
        },
        stage,
        boot_event: None,
        copy: crate::emo2_boot::switch_copy::SwitchCopy::None,
    }
}

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

fn publisher_of(world: &World) -> SylphyaPublisher {
    world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|s| s.runtime())
        .expect("ゴーストの実行系が起きている")
        .sylphya_publisher()
        .clone()
}

/// 1 種の判定の記録（種類, 期待のスコープ, 期待の key, 期待の値）。
type Expect = (&'static str, PersistScope, PersistKey, String);

#[test]
fn every_write_kind_is_withheld_under_the_guard() {
    let script = || FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e")));
    let mut rig = SwitchRig::new(vec![("A", script()), ("B", script())]);
    rig.wire_app_memory();
    // 切替先 B の窓の準備が閉包を投函する先（取り出しの系は回さないので走らない）。
    rig.world
        .insert_resource(wintf::ecs::widget::bitmap_source::WintfTaskPool::with_threads(1));
    rig.boot("A");
    let root = rig.root.dir().to_path_buf();
    let app = rig.app_dir();
    let ghost = rig
        .root
        .ghost_dir("A")
        .join("ghost")
        .join("master")
        .join("profile")
        .join("areka");
    let publisher = publisher_of(&rig.world);
    publisher.barrier().expect("起動の書き込みが済む");
    let picked = TempPath::under_target("readonly-sites-picked");
    let (present_root, absent_root) = (picked.child("present"), picked.child("absent"));
    std::fs::create_dir_all(&present_root).unwrap();
    std::fs::create_dir_all(&absent_root).unwrap();

    let _guard = WithholdGuard::under(&root);
    let before = tree(&root);

    // 1 種ずつ: 呼ぶ → 机に届かない・木が同じ・期待の値が読み返せる。
    let mut judged: Vec<&str> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    let mut judge = |kind: &'static str, desk: bool, expect: Expect| {
        let (_, scope, key, value) = &expect;
        // 印を消すは空の値を書き、読み返しでは鍵が無い形になる（無い＝空として比べる）。
        let got = read_back(&app, &ghost, *scope, *key).unwrap_or_default();
        if desk {
            failed.push(format!("{kind}: 致命の机に届いた"));
        }
        if tree(&root) != before {
            failed.push(format!("{kind}: 根の木が変わった"));
        }
        if got != *value {
            failed.push(format!(
                "{kind}（{scope:?}・{key:?}）: 読み返し {got:?} ≠ 期待 {value:?}"
            ));
        }
        judged.push(kind);
    };
    let welcoming = reservation(
        &rig,
        SwitchStage::Welcoming {
            attempt: WelcomeAttempt::Target,
        },
    );
    let send_off = reservation(&rig, SwitchStage::SendOff);
    let world = &mut rig.world;
    let ctx_ghost_folder = |w: &World| {
        w.resource::<BootContext>()
            .current
            .ghost
            .folder
            .clone()
            .expect("記憶で決まったゴーストのフォルダ")
    };
    let last_used = |w: &mut World| {
        let ctx = w.resource::<BootContext>();
        LastUsed {
            ghost: &ctx.current.ghost,
            balloon: &ctx.current.balloon,
            shell_folder: "master",
        }
        .record(&publisher);
    };

    // 最後に使ったもの（ゴースト・バルーン・シェル）は起動が書いた値と同じ値を書くので、先に別の値で
    // 塗ってから書き戻させる（塗らないと、書かなかったときも読み返しが合ってしまう）。
    let paint = |w: &mut World, scope: PersistScope, key: PersistKey| {
        let desk = reached_desk(w, Some(&publisher), |_| {
            publisher.persist_put(scope, vec![(key, "painted".to_owned())]);
        });
        assert!(!desk, "塗りの書き込みが机に届いた: {key:?}");
        assert_eq!(
            read_back(&app, &ghost, scope, key).as_deref(),
            Some("painted")
        );
    };

    // --- 投函（ゴーストの記憶の書き手を通る）: アプリの記憶 ---
    routes(world, GhostRoute::Memory, BalloonRoute::Argv);
    paint(world, PersistScope::App, PersistKey::LastGhost);
    let desk = reached_desk(world, Some(&publisher), last_used);
    let kind = "最後に使ったゴースト";
    judge(
        kind,
        desk,
        (
            kind,
            PersistScope::App,
            PersistKey::LastGhost,
            ctx_ghost_folder(world),
        ),
    );

    routes(world, GhostRoute::Argv, BalloonRoute::Argv);
    world.insert_non_send(welcoming);
    let desk = reached_desk(world, Some(&publisher), |w| {
        on_notice(w, KanadeNotice::Steady)
    });
    let running = running_name(&world.resource::<BootContext>().current.ghost);
    let kind = "定常の印";
    judge(
        kind,
        desk,
        (kind, PersistScope::App, PersistKey::LastRunning, running),
    );

    world.resource_mut::<BootContext>().roots_origin = crate::roots::RootsOrigin::Default;
    let desk = reached_desk(world, Some(&publisher), |w| {
        crate::roots::add_root(w, present_root.clone());
    });
    let encoded = |w: &World| {
        crate::roots::encode_memory(
            &w.resource::<BootContext>().roots,
            crate::roots::user_root::exe_dir().as_deref(),
        )
    };
    let kind = "置き場の並び";
    judge(
        kind,
        desk,
        (
            kind,
            PersistScope::App,
            PersistKey::BasewareRoots,
            encoded(world),
        ),
    );

    // --- 投函: ゴーストごとの記憶 ---
    routes(world, GhostRoute::Argv, BalloonRoute::Memory);
    let balloon = world
        .resource::<BootContext>()
        .current
        .balloon
        .folder
        .clone()
        .expect("記憶で決まったバルーンのフォルダ");
    paint(world, PersistScope::Ghost, PersistKey::LastBalloon);
    let desk = reached_desk(world, Some(&publisher), last_used);
    let kind = "最後に使ったバルーン";
    judge(
        kind,
        desk,
        (kind, PersistScope::Ghost, PersistKey::LastBalloon, balloon),
    );

    paint(world, PersistScope::Ghost, PersistKey::LastShell);
    routes(world, GhostRoute::Argv, BalloonRoute::Argv);
    let desk = reached_desk(world, Some(&publisher), last_used);
    let kind = "最後に使ったシェル";
    judge(
        kind,
        desk,
        (
            kind,
            PersistScope::Ghost,
            PersistKey::LastShell,
            "master".to_owned(),
        ),
    );

    let desk = reached_desk(world, Some(&publisher), |_| {
        record_last_shell(&publisher, "shell-only")
    });
    let kind = "最後のシェルだけ";
    judge(
        kind,
        desk,
        (
            kind,
            PersistScope::Ghost,
            PersistKey::LastShell,
            "shell-only".to_owned(),
        ),
    );

    let desk = reached_desk(world, Some(&publisher), |_| {
        record_last_balloon(&publisher, "balloon-only")
    });
    let kind = "最後のバルーンだけ";
    judge(
        kind,
        desk,
        (
            kind,
            PersistScope::Ghost,
            PersistKey::LastBalloon,
            "balloon-only".to_owned(),
        ),
    );

    let desk = reached_desk(world, Some(&publisher), |w| {
        crate::install::desk::remember_balloon(w, "installed".to_owned());
    });
    let kind = "入れたバルーン";
    judge(
        kind,
        desk,
        (
            kind,
            PersistScope::Ghost,
            PersistKey::LastBalloon,
            "installed".to_owned(),
        ),
    );

    insert_persist_wiring(world, publisher.clone());
    let desk = reached_desk(world, Some(&publisher), |w| {
        persist_entries(w, char_pos_entries(0, PointPx { x: 10, y: 20 }));
    });
    let window_x = PersistKey::WindowPos {
        scope: 0,
        axis: areka_sylphya::Axis::X,
    };
    let kind = "窓の位置";
    judge(
        kind,
        desk,
        (kind, PersistScope::Ghost, window_x, "10".to_owned()),
    );

    let desk = reached_desk(world, Some(&publisher), |_| {
        PropSetCueSink::new(publisher.clone()).emit(prop_set_cue(PersistKey::BootCount, "7"));
    });
    let kind = "ゴーストのプロパティ";
    judge(
        kind,
        desk,
        (
            kind,
            PersistScope::Ghost,
            PersistKey::BootCount,
            "7".to_owned(),
        ),
    );

    // 降ろす（取っ手の下のまま）。
    assert!(rig.shutdown(), "A を降ろせる");
    let world = &mut rig.world;
    drain_desk(world);

    // --- 直（UI スレッドが置き場へ直に書く）: アプリの記憶 ---
    let desk = reached_desk(world, None, |w| {
        crate::roots::add_root(w, absent_root.clone())
    });
    let kind = "置き場の並び";
    judge(
        kind,
        desk,
        (
            kind,
            PersistScope::App,
            PersistKey::BasewareRoots,
            encoded(world),
        ),
    );

    // 降ろした直後: 最後のゴースト＝既定・印＝切替先を書き、切替先 B を起こす。
    world.insert_non_send(send_off);
    let desk = reached_desk(world, None, |w| {
        switch_to(w, ChangeHandoff { script: None })
    });
    let kind = "降ろした直後";
    judge(
        kind,
        desk,
        (
            kind,
            PersistScope::App,
            PersistKey::LastGhost,
            DEFAULT_GHOST_FOLDER.to_owned(),
        ),
    );
    let ghost_after_drop = read_back(&app, &ghost, PersistScope::App, PersistKey::LastRunning);
    assert_eq!(ghost_after_drop.as_deref(), Some("B"), "印は切替先");
    assert!(rig.shutdown(), "B を降ろせる");
    let world = &mut rig.world;
    drain_desk(world);

    // 印を消す: 後始末が失敗を返さない（告知を選ぶ口も Ok）。
    let (verdict, clear_failed) = settle_session_mark(
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
    let told = crate::write_failure::tell_at_close(None, clear_failed.clone(), None, true);
    let kind = "印を消す";
    judge(
        kind,
        clear_failed.is_some() || told.is_err(),
        (
            kind,
            PersistScope::App,
            PersistKey::LastRunning,
            String::new(),
        ),
    );

    // 起動中の印を書く: 起動を止めずにゴーストを起こす。
    let (cfg, ghost_decision, balloon_decision) = {
        let c = &world.resource::<BootContext>().current;
        (c.cfg.clone(), c.ghost.clone(), c.balloon.clone())
    };
    let descript = StartupDescriptValues {
        author_dpi: AuthorDpi::DEFAULT,
        zorder_raw: None,
    };
    let mut boot = Ok(());
    let desk = reached_desk(world, None, |w| {
        boot = boot_first_ghost(
            w,
            &cfg,
            areka_kanade::BootOrigin::Plain,
            &descript,
            &ghost_decision,
            &balloon_decision,
        );
    });
    let kind = "起動中の印を書く";
    judge(
        kind,
        desk || boot.is_err(),
        (
            kind,
            PersistScope::App,
            PersistKey::LastRunning,
            running_name(&ghost_decision),
        ),
    );
    assert!(rig.shutdown(), "起こしたゴーストを降ろせる");

    assert!(
        failed.is_empty(),
        "読み取りだけで守れなかった書き込み: {failed:#?}"
    );
    // 判定した種類の集まりが要件 1 の表の 13 種と等しい（置き場の並びは居る・居ないの 2 回で 1 種）。
    judged.sort_unstable();
    judged.dedup();
    let mut table = KINDS.to_vec();
    table.sort_unstable();
    assert_eq!(judged, table, "判定した種類が要件 1 の表と合わない");
}
