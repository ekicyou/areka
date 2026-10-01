//! 受付と窓口の段の兄弟テスト（design「窓口と入口」の二重起動・預かり・定常でない・要件 1.1・1.14・
//! 1.15・1.17・9.6・9.7）。
//!
//! 背景スレッドは起こさない: 窓口の仕事の送出端を受信端に差し替え、背景スレッドの頼みは窓口の頼みの
//! 送出端へ直接入れる。kanade の代わりは置き場の中身に持たせた送出端の受信端。取り出しの系は `drain`
//! を直接呼ぶ（フレームは回さない）。どの受け取りも待たない（`try_recv`）。

use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};

use areka_ghost::BasewareRoot;
use areka_kanade::{KanadeMsg, ShioriMethod};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::ghost_switch::{PrevGhost, SwitchInFlight, SwitchStage, SwitchTarget};
use crate::exit_wait::begin_close;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::update::refs::executing_refs;
use crate::update::{SubmitVerdict, SummaryKind, UpdateOrder, UpdateReason, register, submit};

// ---------------------------------------------------------------- 道具立て

/// 窓口を据え、起動の文脈と置き場のゴースト（kanade の送出端だけ）を置いた World。
struct Rig {
    world: World,
    /// 背景スレッドの代わり（仕事の受信端）。
    jobs: Receiver<UpdateJob>,
    /// 背景スレッドの頼みの送出端。
    asks: Sender<DeskAsk>,
    /// kanade の代わり（置き場のゴーストの送出端の受信端）。
    kanade: Receiver<KanadeMsg>,
    _root: TempPath,
}

impl Rig {
    fn new(label: &str) -> Self {
        let root = TempPath::new(label);
        let base = BasewareRoot::new(root.path().to_path_buf());
        write(&base.ghost_dir("emo").join("ghost/master"), "name,Emo");
        let mut world = World::new();
        world.init_resource::<Schedules>();
        register(&mut world);
        world.insert_resource(BootContext {
            root: base.clone(),
            app_profile_dir: root.path().join("profile"),
            helper_exe: root.path().join("helper.exe"),
            argv_session: false,
            current: CurrentGhost {
                cfg: ConfigInputs {
                    ghost_root: base.ghost_dir("emo"),
                    balloon_root: base.balloon_dir("kaku"),
                },
                ghost: GhostDecision {
                    route: GhostRoute::Default,
                    dir: base.ghost_dir("emo"),
                    folder: Some("emo".to_owned()),
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
            base.ghost_dir("emo"),
        ))));
        let (worker, jobs) = mpsc::channel();
        let asks = {
            let mut desk = world.non_send_mut::<UpdateDesk>();
            desk.worker = Some(worker);
            desk.asks_tx.clone()
        };
        Rig {
            world,
            jobs,
            asks,
            kanade,
            _root: root,
        }
    }

    fn submit(&mut self, reason: UpdateReason) -> SubmitVerdict {
        submit(&mut self.world, ghost_only(), reason)
    }

    fn drain(&mut self) {
        drain(&mut self.world);
    }

    fn stage(&self) -> Stage {
        self.world.non_send::<UpdateDesk>().stage
    }

    fn held(&self) -> Option<(RawUpdateRequest, UpdateReason)> {
        self.world.non_send::<UpdateDesk>().held.clone()
    }

    /// 台本の受け口と同じ道で、生の要求を 1 件入れる（取り出しで理由 `Script` の受付に掛かる）。
    fn send_raw(&self) {
        raw_sender(&self.world)
            .send(ghost_only())
            .expect("窓口の生の要求の受信端は生きている");
    }

    fn ask(&self, ask: DeskAsk) {
        self.asks.send(ask).expect("窓口の頼みの受信端は生きている");
    }

    /// kanade に届いた送出（id・Reference・返信端の有無）。
    fn raised(&self) -> Vec<(String, Vec<String>, bool)> {
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
                    (id, references, reply.is_some())
                }
                _ => panic!("汎用の通知の入口の送出だけが届く"),
            })
            .collect()
    }

    fn orders(&self) -> Vec<UpdateOrder> {
        self.jobs.try_iter().map(|job| job.order).collect()
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

fn ghost_only() -> RawUpdateRequest {
    RawUpdateRequest::Current(vec![TargetKind::Ghost])
}

fn executing(reason: UpdateReason) -> (String, Vec<String>, bool) {
    (
        "OnUpdateFailure".to_owned(),
        executing_refs(TargetKind::Ghost, reason),
        false,
    )
}

fn named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

fn warns(events: &[CapturedEvent]) -> usize {
    events.iter().filter(|e| e.level == Level::WARN).count()
}

// ---------------------------------------------------------------- 受付

/// 受けた依頼は対象を解いて背景スレッドへ渡り、メニューは答え待ち・台本は走っているの段へ。
#[test]
fn accepted_orders_reach_the_worker_and_set_the_stage_by_reason() {
    let mut rig = Rig::new("areka-update-desk-accept");
    let (verdict, events) = capture(|| rig.submit(UpdateReason::Manual));
    assert_eq!(verdict, SubmitVerdict::Started);
    assert_eq!(rig.stage(), Stage::AwaitingExec);
    let orders = rig.orders();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].reason, UpdateReason::Manual);
    assert_eq!(orders[0].summary, SummaryKind::Result);
    assert_eq!(orders[0].targets.len(), 1);
    assert_eq!(orders[0].targets[0].name, "Emo");
    assert_eq!(orders[0].ghost_folder.as_deref(), Some("emo"));
    let started = named(&events, "update_order_started");
    assert_eq!(started.len(), 1, "{events:?}");
    assert_eq!(started[0].level, Level::INFO);
    assert_eq!(warns(&events), 0, "{events:?}");
    assert!(rig.raised().is_empty());

    let mut rig = Rig::new("areka-update-desk-accept-script");
    let other = RawUpdateRequest::Other(vec![(TargetKind::Shell, "none".to_owned())]);
    // 引けない名前だけなら対象 0 で断る（`updateother` の総括の形は解けた後に決まる）。
    assert_eq!(
        submit(&mut rig.world, other, UpdateReason::Script),
        SubmitVerdict::NoTargets
    );
    assert_eq!(rig.submit(UpdateReason::Script), SubmitVerdict::Started);
    assert_eq!(rig.stage(), Stage::Running);
}

/// 断りの順（窓口が無い → 終了 → 送出端・文脈が無い → 対象 0）と、判定ごとに `warn!` 1 件。
#[test]
fn refusals_leave_one_warning_each_and_send_nothing() {
    let mut bare = World::new();
    let (verdict, events) = capture(|| submit(&mut bare, ghost_only(), UpdateReason::Manual));
    assert_eq!(verdict, SubmitVerdict::NoDesk);
    assert_eq!(warns(&events), 1, "{events:?}");

    let mut rig = Rig::new("areka-update-desk-refuse");
    rig.world.remove_resource::<BootContext>();
    let (verdict, events) = capture(|| rig.submit(UpdateReason::Manual));
    assert_eq!(verdict, SubmitVerdict::NoContext);
    assert_eq!(warns(&events), 1, "{events:?}");

    let mut rig = Rig::new("areka-update-desk-refuse-targets");
    let (verdict, events) = capture(|| {
        submit(
            &mut rig.world,
            RawUpdateRequest::Current(Vec::new()),
            UpdateReason::Manual,
        )
    });
    assert_eq!(verdict, SubmitVerdict::NoTargets);
    assert_eq!(warns(&events), 1, "{events:?}");

    let _ = begin_close(&mut rig.world);
    let (verdict, events) = capture(|| rig.submit(UpdateReason::Manual));
    assert_eq!(verdict, SubmitVerdict::Closing);
    assert_eq!(warns(&events), 1, "{events:?}");
    assert!(rig.orders().is_empty());
    assert!(rig.raised().is_empty());
    assert_eq!(rig.stage(), Stage::Idle);
}

/// 二重起動: 走っている間の要求は `Executing`。kanade へ `OnUpdateFailure(executing)` が返事なしで
/// 1 件だけ届き、総括は無く、背景スレッドへは渡らない（要件 1.15・9.7）。
#[test]
fn a_request_while_running_sends_one_executing_and_no_summary() {
    let mut rig = Rig::new("areka-update-desk-running");
    assert_eq!(rig.submit(UpdateReason::Script), SubmitVerdict::Started);
    assert_eq!(rig.orders().len(), 1);

    let (verdict, events) = capture(|| rig.submit(UpdateReason::Manual));
    assert_eq!(verdict, SubmitVerdict::Executing);
    let raised = rig.raised();
    assert_eq!(raised, vec![executing(UpdateReason::Manual)]);
    assert!(
        raised
            .iter()
            .all(|(id, ..)| !id.starts_with("OnUpdateResult")),
        "総括は無い"
    );
    assert!(rig.orders().is_empty());
    let refused = named(&events, "update_refused");
    assert_eq!(refused.len(), 1, "{events:?}");
    assert_eq!(warns(&events), 1, "{events:?}");
    assert_eq!(rig.stage(), Stage::Running);
}

/// 切替の予約が在れば `NotSteady` で断り、イベント 0 件・`warn!` 1 件（要件 1.17・9.7）。
#[test]
fn a_switch_reservation_refuses_with_no_event_and_one_warning() {
    let mut rig = Rig::new("areka-update-desk-switching");
    rig.world.insert_non_send(SwitchInFlight {
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
        stage: SwitchStage::SendOff,
        boot_event: None,
    });
    let (verdict, events) = capture(|| rig.submit(UpdateReason::Manual));
    assert_eq!(verdict, SubmitVerdict::NotSteady);
    assert!(rig.raised().is_empty());
    assert!(rig.orders().is_empty());
    assert_eq!(warns(&events), 1, "{events:?}");
    assert_eq!(named(&events, "update_refused").len(), 1, "{events:?}");
    assert_eq!(rig.stage(), Stage::Idle);
}

// ---------------------------------------------------------------- 預かり

/// 頼みと台本の要求の届き方。
#[derive(Debug, Clone, Copy)]
enum Arrival {
    /// 同じ `drain` に届く。
    SameTick,
    /// 要求が先の tick、頼みが後の tick。
    RawFirst,
    /// 頼みが先の tick、要求が後の tick。
    AskFirst,
}

const ARRIVALS: [Arrival; 3] = [Arrival::SameTick, Arrival::RawFirst, Arrival::AskFirst];

/// メニューの要求を受けて答え待ちにした上で、台本の要求と頼みを `arrival` の形で届ける。
/// 戻り値は（背景スレッドへ渡った依頼・kanade へ届いた送出・終わりの段・終わりの預かり）。
fn held_run(
    ask: DeskAsk,
    arrival: Arrival,
) -> (
    Vec<UpdateOrder>,
    Vec<(String, Vec<String>, bool)>,
    Stage,
    Option<(RawUpdateRequest, UpdateReason)>,
) {
    let mut rig = Rig::new("areka-update-desk-held");
    assert_eq!(rig.submit(UpdateReason::Manual), SubmitVerdict::Started);
    assert_eq!(rig.orders().len(), 1);
    assert_eq!(rig.stage(), Stage::AwaitingExec);
    match arrival {
        Arrival::SameTick => {
            rig.send_raw();
            rig.ask(ask);
            rig.drain();
        }
        Arrival::RawFirst => {
            rig.send_raw();
            rig.drain();
            assert_eq!(
                rig.held(),
                Some((ghost_only(), UpdateReason::Script)),
                "答え待ちの間の要求は預かる"
            );
            assert!(rig.raised().is_empty());
            rig.ask(ask);
            rig.drain();
        }
        Arrival::AskFirst => {
            rig.ask(ask);
            rig.drain();
            rig.send_raw();
            rig.drain();
        }
    }
    (rig.orders(), rig.raised(), rig.stage(), rig.held())
}

/// ⑴ ゴーストが応えた（`OrderDone`）: 預かりは受付に掛け直されて背景スレッドへ渡り、
/// `OnUpdateFailure` は 0 件。届き方に依らない。
#[test]
fn a_held_request_starts_after_the_ghost_answered() {
    for arrival in ARRIVALS {
        let (orders, raised, stage, held) = held_run(DeskAsk::OrderDone, arrival);
        assert_eq!(orders.len(), 1, "{arrival:?}");
        assert_eq!(orders[0].reason, UpdateReason::Script, "{arrival:?}");
        assert!(raised.is_empty(), "{arrival:?}: {raised:?}");
        assert_eq!(stage, Stage::Running, "{arrival:?}");
        assert_eq!(held, None, "{arrival:?}");
    }
}

/// ⑵ 応えが無かった（`Started`）: 預かりは `OnUpdateFailure(executing)` 1 件で断られ、空になる。
/// 届き方に依らない。
#[test]
fn a_held_request_is_refused_when_the_standard_procedure_starts() {
    for arrival in ARRIVALS {
        let (orders, raised, stage, held) = held_run(DeskAsk::Started, arrival);
        assert!(orders.is_empty(), "{arrival:?}");
        assert_eq!(raised, vec![executing(UpdateReason::Script)], "{arrival:?}");
        assert_eq!(stage, Stage::Running, "{arrival:?}");
        assert_eq!(held, None, "{arrival:?}");
    }
}

/// ⑶ 預かりが埋まっている間の 2 件目は `Executing`（1 件目は預かったまま）。同じ `drain` でも
/// 別の `drain` でも同じ。
#[test]
fn a_second_request_while_holding_is_refused() {
    for same_tick in [true, false] {
        let mut rig = Rig::new("areka-update-desk-held-second");
        assert_eq!(rig.submit(UpdateReason::Manual), SubmitVerdict::Started);
        assert_eq!(rig.orders().len(), 1);
        rig.send_raw();
        if !same_tick {
            rig.drain();
        }
        rig.send_raw();
        let ((), events) = capture(|| rig.drain());
        assert_eq!(
            rig.raised(),
            vec![executing(UpdateReason::Script)],
            "{same_tick}"
        );
        assert!(rig.orders().is_empty(), "{same_tick}");
        assert_eq!(rig.stage(), Stage::AwaitingExec, "{same_tick}");
        assert_eq!(
            rig.held(),
            Some((ghost_only(), UpdateReason::Script)),
            "{same_tick}"
        );
        assert_eq!(named(&events, "update_refused").len(), 1, "{events:?}");
    }
}

// ---------------------------------------------------------------- 残りの分かれ目

/// 背景スレッドが居ない（仕事の受信端が落ちている）: `WorkerGone`・`error!(update_worker_gone)` 1 件・
/// 仕事の送出端を捨てて次の依頼で起こし直せるようにし、段は走っていないのまま。
#[test]
fn a_gone_worker_is_reported_and_dropped_and_the_stage_stays_idle() {
    let mut rig = Rig::new("areka-update-desk-worker-gone");
    let (dead, _) = mpsc::channel::<UpdateJob>();
    rig.world.non_send_mut::<UpdateDesk>().worker = Some(dead);
    let (verdict, events) = capture(|| rig.submit(UpdateReason::Manual));
    assert_eq!(verdict, SubmitVerdict::WorkerGone);
    let gone = named(&events, "update_worker_gone");
    assert_eq!(gone.len(), 1, "{events:?}");
    assert_eq!(gone[0].level, Level::ERROR);
    assert!(rig.world.non_send::<UpdateDesk>().worker.is_none());
    assert_eq!(rig.stage(), Stage::Idle);
    assert!(rig.raised().is_empty());
}

/// 二重起動の断りの種別は要求の先頭の対象（シェルが先頭なら Reference3 が `shell`）。
#[test]
fn a_double_start_led_by_a_shell_reports_shell() {
    let mut rig = Rig::new("areka-update-desk-running-shell");
    assert_eq!(rig.submit(UpdateReason::Script), SubmitVerdict::Started);
    let shell_first = RawUpdateRequest::Current(vec![TargetKind::Shell, TargetKind::Ghost]);
    assert_eq!(
        submit(&mut rig.world, shell_first, UpdateReason::Manual),
        SubmitVerdict::Executing
    );
    let raised = rig.raised();
    assert_eq!(raised.len(), 1, "{raised:?}");
    let (id, references, with_reply) = &raised[0];
    assert_eq!(id, "OnUpdateFailure");
    assert_eq!(references[0], "executing");
    assert_eq!(references[3], "shell");
    assert!(!with_reply);
}

/// 更新の実行中か: 段が `Idle` でない間と、読み直しの切替の後送りを持つ間だけ真（窓口が無ければ偽・
/// shell-balloon-switch 要件 1.13）。
#[test]
fn is_busy_while_running_or_holding_the_after_switch() {
    assert!(!is_busy(&World::new()), "窓口が無ければ偽");
    let mut rig = Rig::new("areka-update-desk-busy");
    assert!(!is_busy(&rig.world));
    assert_eq!(rig.submit(UpdateReason::Script), SubmitVerdict::Started);
    assert!(is_busy(&rig.world));
    rig.world.non_send_mut::<UpdateDesk>().stage = Stage::Idle;
    assert!(!is_busy(&rig.world));
    rig.world.non_send_mut::<UpdateDesk>().after_switch = Some((PathBuf::from("emo"), Vec::new()));
    assert!(is_busy(&rig.world));
}
