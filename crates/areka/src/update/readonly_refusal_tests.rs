//! 読み取りだけの走りの更新の断りの決定論テスト（readonly-mode・design「UpdateRefusal」・
//! 要件 7.3・7.4・7.5・7.6・7.7・10.4・11.3）。
//!
//! 確かめること: World に [`ReadOnlyRun`] が在れば、バルーン「にゃんばるーん」の `updateother` も
//! 自分のゴーストの更新も `ReadOnly` で断られ、段は `Idle`・預かりは空・窓口は忙しくない・背景の
//! スレッドは起こさない。置き場のゴーストへは成否の欄が空の `OnUpdateResultEx` → `OnUpdateResult` の
//! 2 件だけを返事なしで送り、`readonly_refused`（`what = update`）を 1 回につき 1 件残す
//! （`update_refused` の warn は出さない）。
//!
//! 印は試験の World に資源を直に足す（`readonly::engage` は呼ばない）。

use std::path::Path;
use std::sync::mpsc::{self, Receiver};

use areka_ghost::BasewareRoot;
use areka_kanade::{KanadeMsg, ShioriMethod};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::capture;
use temp_path_kit::TempPath;
use tracing::Level;

use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::exit_wait::begin_close;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::readonly::ReadOnlyRun;
use crate::update::desk::{self, Stage, UpdateDesk};
use crate::update::{RawUpdateRequest, SubmitVerdict, TargetKind, UpdateReason, register, submit};

const GHOST_NAME: &str = "窓際のぱすたさんDRESSTEST";
const BALLOON_NAME: &str = "にゃんばるーん";

/// 窓口を据え、起動の文脈・置き場のゴースト（kanade の送出端だけ）・読み取りだけの印を置いた World。
struct Rig {
    world: World,
    kanade: Receiver<KanadeMsg>,
    _root: TempPath,
}

impl Rig {
    fn new(label: &str) -> Self {
        let root = TempPath::under_target(label);
        let base = BasewareRoot::new(root.path().to_path_buf());
        write(
            &base.ghost_dir("pasta").join("ghost/master"),
            &format!("name,{GHOST_NAME}"),
        );
        write(&base.balloon_dir("kaku"), "name,Kaku");
        write(
            &base.balloon_dir("nyan"),
            &format!("name,{BALLOON_NAME}\r\nhomeurl,https://balloon.example/nyan/"),
        );
        let mut world = World::new();
        world.init_resource::<Schedules>();
        register(&mut world);
        world.insert_resource(BootContext {
            roots: areka_ghost::BasewareRoots::single(base.dir().to_path_buf()),
            roots_origin: crate::roots::RootsOrigin::EnvVar,
            app_profile_dir: root.path().join("profile"),
            helper_exe: root.path().join("helper.exe"),
            argv_session: false,
            current: CurrentGhost {
                cfg: ConfigInputs {
                    ghost_root: base.ghost_dir("pasta"),
                    balloon_root: base.balloon_dir("kaku"),
                },
                ghost: GhostDecision {
                    route: GhostRoute::Default,
                    dir: base.ghost_dir("pasta"),
                    folder: Some("pasta".to_owned()),
                },
                balloon: BalloonDecision {
                    route: BalloonRoute::Default,
                    dir: base.balloon_dir("kaku"),
                    folder: Some("kaku".to_owned()),
                },
            },
        });
        let (tx, kanade) = mpsc::channel();
        world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
            Some(tx),
            base.ghost_dir("pasta"),
        ))));
        world.insert_resource(ReadOnlyRun);
        Rig {
            world,
            kanade,
            _root: root,
        }
    }

    /// kanade に届いた送出（id・Reference）。どれも返事なしの GET。
    fn raised(&self) -> Vec<(String, Vec<String>)> {
        self.kanade
            .try_iter()
            .map(|msg| match msg {
                KanadeMsg::RaiseEvent {
                    id,
                    references,
                    method,
                    reply,
                } => {
                    assert_eq!(method, ShioriMethod::Get);
                    assert!(reply.is_none(), "返事は待たない");
                    (id, references)
                }
                _ => panic!("汎用の通知の入口の送出だけが届く"),
            })
            .collect()
    }

    /// 段は待ち・預かりは空・窓口は忙しくない・背景のスレッドは起こさない。
    fn assert_untouched(&self) {
        let desk = self.world.non_send::<UpdateDesk>();
        assert_eq!(desk.stage, Stage::Idle);
        assert!(desk.held.is_none(), "預からない");
        assert!(desk.worker.is_none(), "背景のスレッドを起こさない");
        assert!(!desk::is_busy(&self.world), "窓口を忙しいに残さない");
    }
}

fn write(dir: &Path, descript: &str) {
    std::fs::create_dir_all(dir).expect("検体のフォルダを作る");
    std::fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\r\n{descript}\r\n"),
    )
    .expect("descript.txt を書く");
}

/// 1 回の依頼を断らせ、送り物と `readonly_refused` の記録を確かめる（2 回目も同じ）。
fn assert_refused_twice(
    raw: RawUpdateRequest,
    reason: UpdateReason,
    label: &str,
    expected: &[(&str, Vec<String>)],
) {
    let mut rig = Rig::new(label);
    for _ in 0..2 {
        let (verdict, events) = capture(|| submit(&mut rig.world, raw.clone(), reason));
        assert_eq!(verdict, SubmitVerdict::ReadOnly);
        rig.assert_untouched();
        let raised = rig.raised();
        let expected: Vec<(String, Vec<String>)> = expected
            .iter()
            .map(|(id, refs)| ((*id).to_owned(), refs.clone()))
            .collect();
        assert_eq!(raised, expected, "Ex → Result の 2 件だけ");

        let refused: Vec<_> = events
            .iter()
            .filter(|e| e.field_str("event") == Some("readonly_refused"))
            .collect();
        assert_eq!(refused.len(), 1, "1 回につき 1 件: {events:?}");
        let e = refused[0];
        assert_eq!(e.level, Level::INFO);
        assert_eq!(e.field_str("what"), Some("update"));
        assert_eq!(e.field_str("reason"), Some(reason.as_ref_str()));
        assert_eq!(e.field("targets"), Some("1"));
        assert_eq!(e.field("notified"), Some("true"));
        assert!(
            events
                .iter()
                .all(|e| e.field_str("event") != Some("update_refused")),
            "断りの warn を出さない"
        );
        assert_eq!(events.iter().filter(|e| e.level == Level::WARN).count(), 0);
    }
}

/// バルーン「にゃんばるーん」の `updateother` は断られ、成否の欄が空の総括 2 件だけが届く。
#[test]
fn readonly_refuses_updateother_balloon_with_ex_then_result() {
    assert_refused_twice(
        RawUpdateRequest::Other(vec![(TargetKind::Balloon, BALLOON_NAME.to_owned())]),
        UpdateReason::Script,
        "areka-update-readonly-balloon",
        &[
            (
                "OnUpdateResultEx",
                vec![format!("{BALLOON_NAME}\u{1}balloon\u{1}")],
            ),
            ("OnUpdateResult", vec!["balloon\u{1}".to_owned()]),
        ],
    );
}

/// 自分のゴーストの更新（`Current`）でも、SSP の測りどおり Ex と Result の 2 件を送る。
#[test]
fn readonly_refuses_own_ghost_with_ex_then_result() {
    assert_refused_twice(
        RawUpdateRequest::Current(vec![TargetKind::Ghost]),
        UpdateReason::Manual,
        "areka-update-readonly-ghost",
        &[
            (
                "OnUpdateResultEx",
                vec![format!("{GHOST_NAME}\u{1}ghost\u{1}")],
            ),
            ("OnUpdateResult", vec!["ghost\u{1}".to_owned()]),
        ],
    );
}

/// 対象が 0 なら何も送らず、`notified = false` の記録だけを 1 件残す。
#[test]
fn readonly_with_no_targets_sends_nothing() {
    let mut rig = Rig::new("areka-update-readonly-none");
    let raw = RawUpdateRequest::Other(vec![(TargetKind::Balloon, "居ない".to_owned())]);
    let (verdict, events) = capture(|| submit(&mut rig.world, raw, UpdateReason::Script));
    assert_eq!(verdict, SubmitVerdict::ReadOnly);
    rig.assert_untouched();
    assert!(rig.raised().is_empty(), "対象が 0 なら送らない");
    let refused: Vec<_> = events
        .iter()
        .filter(|e| e.field_str("event") == Some("readonly_refused"))
        .collect();
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].field("targets"), Some("0"));
    assert_eq!(refused[0].field("notified"), Some("false"));
}

/// 判定の順: 終わりが始まっていれば、印より先に `Closing` で断る（総括は送らない）。
#[test]
fn closing_is_judged_before_the_readonly_mark() {
    let mut rig = Rig::new("areka-update-readonly-closing");
    let _ = begin_close(&mut rig.world);
    let verdict = submit(
        &mut rig.world,
        RawUpdateRequest::Current(vec![TargetKind::Ghost]),
        UpdateReason::Manual,
    );
    assert_eq!(verdict, SubmitVerdict::Closing);
    assert!(rig.raised().is_empty());
}
