//! 切替の写しの決定論テスト（spec: areka-P0-user-data-root 要件 4.3・4.7・10.4・task 4.2）。
//!
//! 写しの答えは受信端を自分で組んで送り、届く順をテストが決める（時刻・眠りに頼らない）。
//! 背景のスレッドを走らせる [`start`] の確かめは、受信端の `recv` で答えを待つ。切替先は
//! 起動入力の作り口の無い World で起こすので、起こす試みは `ghost_switch_boot_failed` で倒れ、
//! その `dir` 欄が「どのフォルダから起こしたか」を答える。根は `target\` の下に掘る。

use std::fs;
use std::sync::mpsc;

use areka_ghost::{BasewareRoot, BasewareRoots};
use areka_kanade::{ChangeHandoff, ChangeOrigin};
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use wintf::AppExit;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::ghost_switch::{
    GhostSpec, PrevGhost, SwitchRequest, SwitchTarget, SwitchVerdict, request_ghost_switch,
};
use crate::ghost_session::{GhostSession, GhostSlot};

/// 書ける根 `user` と実行ファイルの隣の根 `exe`（同梱のゴースト `A`・`B`）。
struct Fixture {
    _tmp: TempPath,
    user: PathBuf,
    exe: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let tmp = TempPath::under_target(label);
        let user = tmp.path().join("user");
        let exe = tmp.path().join("exe");
        fs::create_dir_all(&user).unwrap();
        for folder in ["A", "B"] {
            let master = exe.join("ghost").join(folder).join("ghost").join("master");
            fs::create_dir_all(&master).unwrap();
            fs::write(
                master.join("descript.txt"),
                format!("charset,UTF-8\nname,{folder}\n"),
            )
            .unwrap();
        }
        Self {
            _tmp: tmp,
            user,
            exe,
        }
    }

    fn roots(&self) -> BasewareRoots {
        BasewareRoots::new(vec![
            BasewareRoot::user_root(self.user.clone(), false),
            BasewareRoot::new(self.exe.clone()),
        ])
    }

    fn bundled(&self, folder: &str) -> PathBuf {
        self.exe.join("ghost").join(folder)
    }

    fn copied(&self, folder: &str) -> PathBuf {
        self.user.join("ghost").join(folder)
    }

    /// 今のゴースト `A` から `B` への切替の予約（送り出しの段・見込み `copy`）を据えた World。
    /// 起動入力の作り口は据えない（起こす試みは倒れて `dir` 欄を残す）。
    fn world(&self, copy: SwitchCopy) -> World {
        let mut world = World::new();
        world.insert_non_send(AppExit::new());
        let a = self.bundled("A");
        world.insert_resource(BootContext {
            roots: self.roots(),
            roots_origin: crate::roots::RootsOrigin::Default,
            app_profile_dir: self.user.join("profile"),
            helper_exe: self.exe.join("helper.exe"),
            argv_session: false,
            current: CurrentGhost {
                cfg: ConfigInputs {
                    ghost_root: a.clone(),
                    balloon_root: self.exe.join("balloon").join("X"),
                },
                ghost: GhostDecision {
                    route: GhostRoute::Memory,
                    dir: a.clone(),
                    folder: Some("A".to_owned()),
                },
                balloon: BalloonDecision {
                    route: BalloonRoute::Default,
                    dir: self.exe.join("balloon").join("X"),
                    folder: Some("X".to_owned()),
                },
            },
        });
        world.insert_non_send(GhostSlot(Some(GhostSession::for_test(None, a.clone()))));
        world.insert_non_send(SwitchInFlight {
            target: SwitchTarget {
                dir: self.bundled("B"),
                folder: "B".to_owned(),
                name: "B".to_owned(),
                sakura_name: None,
            },
            prev: PrevGhost {
                dir: a,
                name: Some("A".to_owned()),
                sakura_name: None,
            },
            stage: SwitchStage::SendOff,
            boot_event: None,
            copy,
        });
        world
    }
}

fn handoff() -> ChangeHandoff {
    ChangeHandoff { script: None }
}

fn count(events: &[CapturedEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// 最初に起こそうとしたフォルダ（`ghost_switch_boot_failed` の 1 件目の `dir` 欄）。
fn first_boot_dir(events: &[CapturedEvent]) -> Option<String> {
    events
        .iter()
        .find(|e| e.field_str("event") == Some("ghost_switch_boot_failed"))
        .and_then(|e| e.field("dir").map(str::to_owned))
}

fn shown(dir: &Path) -> Option<String> {
    Some(dir.display().to_string())
}

fn stage(world: &World) -> Option<SwitchStage> {
    world.get_non_send::<SwitchInFlight>().map(|f| f.stage)
}

/// 写す要なし: 印の無い並び・書ける根の物・実行ファイルが並びに無い → `None`（スレッドを起こさない）。
#[test]
fn start_without_copy_need_is_none() {
    let f = Fixture::new("switch-copy-none");
    let plain = BasewareRoots::new(vec![BasewareRoot::new(f.exe.clone())]);
    let cases = [
        start(&plain, Some(&f.exe), &f.bundled("B")),
        start(&f.roots(), Some(&f.exe), &f.copied("B")),
        start(&f.roots(), None, &f.bundled("B")),
    ];
    assert!(
        cases.iter().all(|c| matches!(c, SwitchCopy::None)),
        "(印なし・書ける根の物・実行ファイルなし) はどれも None: {cases:?}"
    );
    assert!(!f.copied("B").exists(), "写していない");
}

/// 写す要あり: 背景のスレッドが写し、受信端に写しのフォルダが届く。
#[test]
fn start_copies_on_a_background_thread() {
    let f = Fixture::new("switch-copy-start");
    let SwitchCopy::Running(rx) = start(&f.roots(), Some(&f.exe), &f.bundled("B")) else {
        panic!("写し中の見込みになる");
    };
    assert_eq!(rx.recv().unwrap(), f.copied("B"));
    assert!(
        f.copied("B")
            .join("ghost")
            .join("master")
            .join("descript.txt")
            .is_file(),
        "写しの中身が在る"
    );
}

/// 写す要なし → 切替先のフォルダ（同梱のまま）から起こす。
#[test]
fn no_copy_boots_from_the_target() {
    let f = Fixture::new("switch-copy-asis");
    let mut world = f.world(SwitchCopy::None);

    let ((), events) = capture(|| switch_to(&mut world, handoff()));

    assert_eq!(
        first_boot_dir(&events),
        shown(&f.bundled("B")),
        "{events:?}"
    );
    assert_eq!(count(&events, "ghost_switch_copy_wait"), 0);
}

/// 止まる前に写し終えた: 取り出しの系の先頭の受け取りで `Done` になり、止まった時点でそのまま
/// 写しから起こす（写し待ちを経ない）。
#[test]
fn copied_before_stop_boots_from_the_copy() {
    let f = Fixture::new("switch-copy-before");
    let (tx, rx) = mpsc::channel();
    let mut world = f.world(SwitchCopy::Running(rx));
    tx.send(f.copied("B")).unwrap();

    poll(&mut world);
    let received = matches!(
        &world.non_send::<SwitchInFlight>().copy,
        SwitchCopy::Done(d) if *d == f.copied("B")
    );
    let ((), events) = capture(|| switch_to(&mut world, handoff()));

    assert!(received, "受け取りで写し終えたになる");
    assert_eq!(
        (
            count(&events, "ghost_switch_copy_wait"),
            first_boot_dir(&events)
        ),
        (0, shown(&f.copied("B"))),
        "(写し待ち・起こしたフォルダ): {events:?}"
    );
}

/// 止まった後に写し終えた: 止まった時点では段を「写し待ち」にして戻り（前のゴーストは降ろさない）、
/// その間の切替の要求は退け、受け取った時点で写しから起こす。
#[test]
fn copied_after_stop_waits_then_boots_from_the_copy() {
    let f = Fixture::new("switch-copy-after");
    let (tx, rx) = mpsc::channel();
    let mut world = f.world(SwitchCopy::Running(rx));

    let ((), stopped) = capture(|| switch_to(&mut world, handoff()));
    let waiting = (
        stage(&world),
        count(&stopped, "ghost_switch_copy_wait"),
        count(&stopped, "ghost_switch_down_ms"),
        world
            .get_non_send::<GhostSlot>()
            .is_some_and(|s| s.0.is_some()),
    );
    poll(&mut world);
    let still = stage(&world);
    let verdict = request_ghost_switch(
        &mut world,
        SwitchRequest {
            ghost: GhostSpec::Folder("A".to_owned()),
            raise_event: false,
            origin: ChangeOrigin::Automatic,
            boot_event: None,
        },
    );
    tx.send(f.copied("B")).unwrap();
    let ((), resumed) = capture(|| poll(&mut world));

    assert_eq!(
        waiting,
        (Some(SwitchStage::CopyWait), 1, 0, true),
        "(段・写し待ちの記録・降ろした・前のゴーストは置き場に居る): {stopped:?}"
    );
    assert_eq!(still, Some(SwitchStage::CopyWait), "答えが無い間は待つ");
    assert_eq!(verdict, SwitchVerdict::Busy, "写し待ちの間の要求は退ける");
    assert_eq!(
        (
            count(&resumed, "ghost_switch_down_ms"),
            first_boot_dir(&resumed)
        ),
        (1, shown(&f.copied("B"))),
        "(降ろした・起こしたフォルダ): {resumed:?}"
    );
}

/// 写しの失敗 → 元から: `copy_for_write` は失敗で元のフォルダを答え（書ける根に半端を残さない）、
/// 切替はそれを受け取って同梱のフォルダから起こす。答えずにスレッドが消えたときも元から（error）。
#[test]
fn copy_failure_boots_from_the_original() {
    let f = Fixture::new("switch-copy-fail");
    // 書ける根の `ghost` をファイルにして、写しの置き場を作れなくする。
    fs::write(f.user.join("ghost"), "not a folder").unwrap();
    let SwitchCopy::Running(rx) = start(&f.roots(), Some(&f.exe), &f.bundled("B")) else {
        panic!("写し中の見込みになる");
    };
    let answer = rx.recv().unwrap();

    let (tx, rx) = mpsc::channel();
    let mut world = f.world(SwitchCopy::Running(rx));
    tx.send(answer.clone()).unwrap();
    let ((), failed) = capture(|| switch_to(&mut world, handoff()));

    let (tx, rx) = mpsc::channel::<PathBuf>();
    drop(tx);
    let mut lost_world = f.world(SwitchCopy::Running(rx));
    let ((), lost) = capture(|| switch_to(&mut lost_world, handoff()));

    assert_eq!(answer, f.bundled("B"), "失敗は元のフォルダを答える");
    assert!(!f.copied("B").exists(), "半端な写しを残さない");
    assert_eq!(
        first_boot_dir(&failed),
        shown(&f.bundled("B")),
        "{failed:?}"
    );
    assert_eq!(
        (
            count(&lost, "ghost_switch_copy_lost"),
            first_boot_dir(&lost)
        ),
        (1, shown(&f.bundled("B"))),
        "{lost:?}"
    );
}
