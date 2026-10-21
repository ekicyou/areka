//! shell・balloon・descript の読み直しの列の決定論テスト（areka-P0-mcp-reload 要件 2.5・5.1・6.1・7.1・
//! 7.2・7.3・7.6・8.4・9.1・9.2・9.3）。
//!
//! 置き場のゴーストは kanade の送出端と seriko の送り手だけを持つテスト用の組み立て（実行系が無いので
//! 今のシェルは分からず、シェルの段は始められない）。差し替えの相の合図（`skin_applied`・
//! `skin_phase_ended`）はここから直に呼ぶ（本番の呼び手は `frame/switch.rs`）。名前と readme と
//! シェルの段の実物は、偽の SHIORI で起こしたゴースト（`ghost_switch_test_support`）で突き合わせる。
//! 判定は集めてから 1 回。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use areka_emo_compose::BindSet;
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use areka_ghost::BasewareRoot;
use areka_kanade::KanadeMsg;
use areka_seriko::{
    AnimationTable, BindResolver, MockSurfaceOutput, SerikoLoopConfig, SurfaceResolver,
    spawn_seriko,
};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::assets::{BootAssets, LoopTables};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::reload_args::DescriptParts;
use crate::emo2_boot::talk_clock::TalkClock;
use crate::ghost_session::GhostSession;
use crate::readme::ReadmeWiring;

struct Rig {
    world: World,
    kanade: Receiver<KanadeMsg>,
    root: BasewareRoot,
    ghost: String,
    _temp: TempPath,
}

/// 窓も GPU も持たない結線（入口は有無だけを見る）。
fn headless_wiring() -> Emo2Wiring {
    Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel().1,
        mpsc::channel().1,
        mpsc::channel().1,
        mpsc::channel().1,
        Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        ))),
        TalkClock::new(Arc::new(|| 0.0)),
        BootAssets {
            shells: Vec::new(),
            balloons: Vec::new(),
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds: BindSet::default(),
            bind_resolver: BindResolver::empty(),
            loop_tables: LoopTables {
                shell: AnimationTable::empty(),
                balloon: BTreeMap::new(),
            },
            shell_author_dpi: 96,
            balloon_author_dpi: 96,
            boxes: crate::emo2_boot::shell_box_assets::ShellBoxAssets::default(),
        },
    )
}

/// 起動の文脈（今のゴースト `A`・今のバルーン `kaku`）・置き場・結線・窓口の登録をそろえた World。
fn rig(label: &str) -> Rig {
    let temp = TempPath::new(label);
    let root = BasewareRoot::new(temp.path().to_path_buf());
    let mut world = World::new();
    world.init_resource::<Schedules>();
    world.insert_resource(BootContext {
        roots: areka_ghost::BasewareRoots::single(root.dir().to_path_buf()),
        roots_origin: crate::roots::RootsOrigin::EnvVar,
        app_profile_dir: temp.path().join("profile"),
        helper_exe: temp.path().join("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir("A"),
                balloon_root: root.balloon_dir("kaku"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.ghost_dir("A"),
                folder: Some("A".to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("kaku"),
                folder: Some("kaku".to_owned()),
            },
        },
    });
    let (tx, kanade) = mpsc::channel();
    let seriko = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::default(),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        MockSurfaceOutput::new(),
    )
    .0;
    world.insert_non_send(GhostSlot(Some(
        GhostSession::for_test(Some(tx), root.ghost_dir("A")).with_seriko_sink(seriko),
    )));
    world.insert_non_send(headless_wiring());
    register(&mut world);
    let ghost = std::path::absolute(root.ghost_dir("A"))
        .unwrap()
        .display()
        .to_string();
    Rig {
        world,
        kanade,
        root,
        ghost,
        _temp: temp,
    }
}

fn req(target: ReloadTarget) -> ReloadRequest {
    ReloadRequest {
        target,
        origin: ReloadOrigin::Script,
        notes: DescriptNotes::default(),
    }
}

fn parts(ghost: bool, shell: bool, balloon: bool) -> ReloadTarget {
    ReloadTarget::Descript(DescriptParts {
        ghost,
        shell,
        balloon,
    })
}

/// 印の段の写し（列・applied・予約の鍵）。印が無い・別の段なら `None`。
fn skins(world: &World) -> Option<(Vec<SkinStep>, bool, Option<(SkinKind, PathBuf)>)> {
    match &world.get_non_send::<ReloadInFlight>()?.stage {
        ReloadStage::Skins {
            queue,
            applied,
            reserved,
        } => Some((queue.iter().copied().collect(), *applied, reserved.clone())),
        _ => None,
    }
}

/// 列の途中の印を直に置く（相の合図の判定用）。
fn plant(world: &mut World, queue: &[SkinStep], applied: bool, reserved: (SkinKind, PathBuf)) {
    world.insert_non_send(ReloadInFlight {
        target: parts(false, true, true),
        origin: ReloadOrigin::Script,
        ghost: "planted".to_owned(),
        stage: ReloadStage::Skins {
            queue: queue.iter().copied().collect(),
            applied,
            reserved: Some(reserved),
        },
    });
}

/// kanade へ届いた待ちの依頼（`raise` の有無）の並び。
fn waits(rx: &Receiver<KanadeMsg>) -> Vec<bool> {
    rx.try_iter()
        .filter_map(|m| match m {
            KanadeMsg::AwaitTalkGap { raise, .. } => Some(raise.is_some()),
            _ => None,
        })
        .collect()
}

fn skin_in_flight(world: &World) -> Option<(SkinKind, PathBuf)> {
    world
        .get_non_send::<SkinSwitchInFlight>()
        .map(|f| (f.kind, f.target.dir.clone()))
}

/// `reload_failed`／`reload_done` の (event, 水準, target, reason, ghost) の並び。
fn ends(events: &[CapturedEvent]) -> Vec<(String, Level, String, Option<String>, Option<String>)> {
    events
        .iter()
        .filter(|e| matches!(e.field_str("event"), Some("reload_failed" | "reload_done")))
        .map(|e| {
            (
                e.field_str("event").unwrap_or_default().to_owned(),
                e.level,
                e.target.clone(),
                e.field_str("reason").map(str::to_owned),
                e.field("ghost").map(str::to_owned),
            )
        })
        .collect()
}

fn failed_end(
    reason: &str,
    ghost: &str,
) -> (String, Level, String, Option<String>, Option<String>) {
    (
        "reload_failed".to_owned(),
        Level::ERROR,
        TARGET_ERROR.to_owned(),
        Some(reason.to_owned()),
        Some(ghost.to_owned()),
    )
}

fn write_descript(dir: &Path, body: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\r\n{body}\r\n"),
    )
    .unwrap();
}

/// 段の列は対象から組む（`ghost` は名前とシェル・重なった段は 1 度・この順）。
#[test]
fn steps_are_built_from_the_target() {
    use SkinStep::{Balloon, Names, Shell};
    let got: Vec<Vec<SkinStep>> = [
        ReloadTarget::Shell,
        ReloadTarget::Balloon,
        ReloadTarget::Descript(DescriptParts::ALL),
        parts(true, false, false),
        parts(true, true, false),
        parts(false, true, false),
        parts(false, false, true),
        parts(false, true, true),
        ReloadTarget::Ghost,
    ]
    .iter()
    .map(|t| skin_steps(t).into_iter().collect())
    .collect();
    assert_eq!(
        got,
        vec![
            vec![Shell],
            vec![Balloon],
            vec![Names, Shell, Balloon],
            vec![Names, Shell],
            vec![Names, Shell],
            vec![Shell],
            vec![Balloon],
            vec![Shell, Balloon],
            vec![],
        ]
    );
}

/// balloon の読み直し: 今のバルーンのフォルダを切替先に `raise` 無しの待ちを 1 件頼み、予約の鍵を印に
/// 控える。述語は鍵が合うときだけ真（要件 2.5・6.1）。
#[test]
fn balloon_reload_reserves_the_current_balloon() {
    let mut rig = rig("areka-reload-skin-balloon");
    let kaku = rig.root.balloon_dir("kaku");
    let verdict = request_reload(&mut rig.world, req(ReloadTarget::Balloon));
    let preds = (
        skin_is_reload(&rig.world, SkinKind::Balloon, &kaku),
        skin_is_reload(&rig.world, SkinKind::Balloon, Path::new("other")),
        skin_is_reload(&rig.world, SkinKind::Shell, &kaku),
    );
    assert_eq!(
        (
            verdict,
            waits(&rig.kanade),
            skins(&rig.world),
            skin_in_flight(&rig.world),
            rig.world
                .get_non_send::<SkinSwitchInFlight>()
                .map(|f| f.target.folder.clone()),
            preds,
        ),
        (
            ReloadVerdict::Accepted,
            vec![false],
            Some((vec![], false, Some((SkinKind::Balloon, kaku.clone())))),
            Some((SkinKind::Balloon, kaku)),
            Some("kaku".to_owned()),
            (true, false, false),
        )
    );
}

/// 差し替えが済んだ相の終わりで列が空なら `reload_done` を 1 件残して印を下ろす（要件 8.4）。
#[test]
fn applied_phase_end_with_empty_queue_is_done() {
    let mut rig = rig("areka-reload-skin-done");
    let kaku = rig.root.balloon_dir("kaku");
    request_reload(&mut rig.world, req(ReloadTarget::Balloon));
    let (_, events) = capture(|| {
        skin_applied(&mut rig.world);
        rig.world.remove_non_send::<SkinSwitchInFlight>();
        skin_phase_ended(&mut rig.world, SkinKind::Balloon, &kaku);
    });
    assert_eq!(
        (
            ends(&events),
            events
                .iter()
                .find(|e| e.field_str("event") == Some("reload_done"))
                .and_then(|e| e.field("target").map(str::to_owned)),
            events.iter().filter(|e| e.level <= Level::WARN).count(),
            rig.world.get_non_send::<ReloadInFlight>().is_none(),
        ),
        (
            vec![(
                "reload_done".to_owned(),
                Level::INFO,
                "areka::emo2_boot::reload".to_owned(),
                None,
                Some(rig.ghost.clone()),
            )],
            Some("Balloon".to_owned()),
            0,
            true,
        )
    );
}

/// 差し替えが済んだ相の終わりで列が残っていれば次の段を始める（終わりの記録はまだ無い）。
#[test]
fn applied_phase_end_starts_the_next_step() {
    let mut rig = rig("areka-reload-skin-next");
    let shell = rig.root.ghost_dir("A").join("shell").join("master");
    let kaku = rig.root.balloon_dir("kaku");
    plant(
        &mut rig.world,
        &[SkinStep::Balloon],
        false,
        (SkinKind::Shell, shell.clone()),
    );
    let (_, events) = capture(|| {
        skin_applied(&mut rig.world);
        skin_phase_ended(&mut rig.world, SkinKind::Shell, &shell);
    });
    assert_eq!(
        (
            ends(&events),
            waits(&rig.kanade),
            skins(&rig.world),
            skin_in_flight(&rig.world),
        ),
        (
            vec![],
            vec![false],
            Some((vec![], false, Some((SkinKind::Balloon, kaku.clone())))),
            Some((SkinKind::Balloon, kaku)),
        )
    );
}

/// 差し替えが済まずに相が終わったら `reload_failed(skin_failed)` を残して止まる（残りの段は始めない・
/// 要件 9.2・9.3）。
#[test]
fn phase_end_without_applied_fails_and_stops() {
    let mut rig = rig("areka-reload-skin-failed");
    let shell = rig.root.ghost_dir("A").join("shell").join("master");
    plant(
        &mut rig.world,
        &[SkinStep::Balloon],
        false,
        (SkinKind::Shell, shell.clone()),
    );
    let (_, events) = capture(|| skin_phase_ended(&mut rig.world, SkinKind::Shell, &shell));
    assert_eq!(
        (
            ends(&events),
            waits(&rig.kanade),
            rig.world.get_non_send::<ReloadInFlight>().is_none(),
            skin_in_flight(&rig.world),
        ),
        (
            vec![failed_end("skin_failed", "planted")],
            vec![],
            true,
            None
        )
    );
}

/// 他人の予約（鍵が合わない相）では述語は偽で、その相の終わりで `reload_failed(overtaken)` を残して
/// 印を下ろす。段の間に他人の予約が在れば上書きせずに同じく `overtaken`。
#[test]
fn foreign_reservation_is_false_and_ends_as_overtaken() {
    let mut rig = rig("areka-reload-skin-foreign");
    let kaku = rig.root.balloon_dir("kaku");
    let other = rig.root.balloon_dir("other");
    plant(
        &mut rig.world,
        &[],
        false,
        (SkinKind::Balloon, kaku.clone()),
    );
    let is_reload = skin_is_reload(&rig.world, SkinKind::Balloon, &other);
    let (_, foreign) = capture(|| skin_phase_ended(&mut rig.world, SkinKind::Balloon, &other));
    let first_mark_gone = rig.world.get_non_send::<ReloadInFlight>().is_none();

    // 段の間: 次の段を始めようとしたら他人の予約が既に在る。
    plant(
        &mut rig.world,
        &[SkinStep::Balloon],
        true,
        (SkinKind::Shell, kaku.clone()),
    );
    request_skin_switch_planted(&mut rig.world, &other);
    let (_, between) = capture(|| skin_phase_ended(&mut rig.world, SkinKind::Shell, &kaku));

    assert_eq!(
        (
            is_reload,
            ends(&foreign),
            first_mark_gone,
            ends(&between),
            rig.world.get_non_send::<ReloadInFlight>().is_none(),
            skin_in_flight(&rig.world),
            waits(&rig.kanade),
        ),
        (
            false,
            vec![failed_end("overtaken", "planted")],
            true,
            vec![failed_end("overtaken", "planted")],
            true,
            Some((SkinKind::Balloon, other)),
            vec![],
        )
    );
}

/// 他人の切替の予約を直に置く（利用者の `\![change,balloon,…]` の代わり）。
fn request_skin_switch_planted(world: &mut World, dir: &Path) {
    world.insert_non_send(SkinSwitchInFlight {
        kind: SkinKind::Balloon,
        target: SkinCandidate {
            dir: dir.to_path_buf(),
            folder: "other".to_owned(),
            name: None,
            hidden: false,
        },
        stage: crate::emo2_boot::shell_balloon_switch::SkinSwitchStage::Waiting {
            gap: Some(areka_actor::reply_channel().1),
            gap_result: None,
            build: mpsc::channel().1,
            built: None,
        },
    });
}

/// descript の `ghost`: descript.txt が読めなければ値を保って `reload_descript_read_failed` を
/// （ファイルとゴースト名つきで）1 件残し列を続ける（ここでは次のシェルの段が今のシェルを知らずに
/// `no_context` で止まる）。読めても実行系が無く名前を替えられなければ `names_failed`（要件 7.6・9.3）。
#[test]
fn names_step_read_failure_keeps_going_and_no_runtime_fails() {
    let mut rig = rig("areka-reload-skin-names");
    let (_, unread) = capture(|| request_reload(&mut rig.world, req(parts(true, false, false))));
    let read_failed: Vec<_> = unread
        .iter()
        .filter(|e| e.field_str("event") == Some("reload_descript_read_failed"))
        .map(|e| {
            (
                e.level,
                e.target.clone(),
                e.field("ghost").map(str::to_owned),
                e.field("file").map(str::to_owned),
            )
        })
        .collect();

    write_descript(
        &rig.root.ghost_dir("A").join("ghost").join("master"),
        "name,A",
    );
    write_descript(
        &rig.root.ghost_dir("A").join("shell").join("master"),
        "name,通常",
    );
    let (_, readable) = capture(|| request_reload(&mut rig.world, req(parts(true, false, false))));

    let file = rig
        .root
        .ghost_dir("A")
        .join("ghost")
        .join("master")
        .join("descript.txt")
        .display()
        .to_string();
    assert_eq!(
        (
            read_failed,
            ends(&unread),
            ends(&readable),
            rig.world.get_non_send::<ReloadInFlight>().is_none(),
            waits(&rig.kanade),
        ),
        (
            vec![(
                Level::ERROR,
                TARGET_ERROR.to_owned(),
                Some(rig.ghost.clone()),
                Some(file)
            )],
            vec![failed_end("no_context", &rig.ghost)],
            vec![failed_end("names_failed", &rig.ghost)],
            true,
            vec![],
        )
    );
}

/// 偽の SHIORI で起こしたゴーストで descript（`ghost`）を読み直す: 名前と readme の置き場が書き換えた
/// descript.txt の値に替わり、続くシェルの段が実行系のマウントの今のシェルを切替先に予約する
/// （要件 5.1・7.2・7.3）。
#[test]
fn descript_ghost_replaces_names_and_readme_then_reserves_the_current_shell() {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.boot("A");
    let steady = rig.wait_steady();
    let ghost_root = rig.root.ghost_dir("A");
    let descript = ghost_root.join("ghost").join("master").join("descript.txt");
    let text = std::fs::read_to_string(&descript).unwrap();
    let rewritten: Vec<String> = text
        .lines()
        .map(|line| {
            if line.starts_with("sakura.name,") {
                "sakura.name,読み直したさくら".to_owned()
            } else {
                line.to_owned()
            }
        })
        .chain(["readme,readme-reloaded.txt".to_owned()])
        .collect();
    std::fs::write(&descript, rewritten.join("\r\n")).unwrap();

    let verdict = request_reload(&mut rig.world, req(parts(true, false, false)));
    let session = rig.world.non_send::<GhostSlot>().0.as_ref().unwrap();
    let sakura = session.names().and_then(|n| n.sakura_name.clone());
    let shell_dir = session.runtime().map(|r| r.mount().shell.dir.clone());
    let readme = rig
        .world
        .get_non_send::<ReadmeWiring>()
        .map(|w| w.path().to_path_buf());
    let marked = skins(&rig.world);
    let reserved = skin_in_flight(&rig.world);
    rig.world.remove_non_send::<ReloadInFlight>();
    let shutdown_ok = rig.shutdown();

    let shell_dir = shell_dir.expect("実行系のマウントにシェルが在る");
    assert_eq!(
        (
            steady,
            verdict,
            sakura,
            readme,
            marked,
            reserved,
            shutdown_ok
        ),
        (
            true,
            ReloadVerdict::Accepted,
            Some("読み直したさくら".to_owned()),
            Some(ghost_root.join("readme-reloaded.txt")),
            Some((vec![], false, Some((SkinKind::Shell, shell_dir.clone())))),
            Some((SkinKind::Shell, shell_dir)),
            true,
        ),
        "(定常, 判定, さくらの名前, readme の置き場, 印の段, 予約, 降ろせた)"
    );
}
