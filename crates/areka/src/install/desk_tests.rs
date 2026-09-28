//! UI 側の窓口の決定論テスト（design「Testing Strategy / 窓口と入口」・要件 1.1・1.9・1.10・2.10・2.13・9.3）。
//!
//! 確かめること: 受付の 4 つの判定と記録・手続きの最中に届いた依頼が届いた順に 1 件ずつ背景の
//! スレッドへ渡ること・イベントの頼みは切替の予約が在る間は送らず、予約が下りた tick に送る
//! 時点のゴーストへ送ること・送り直しの頼みは前に送った後に定常到達が届いてから送ること・
//! 素性の答え・8.1 までの上書きの頼みは書庫を返すこと。
//!
//! 背景のスレッドは起こさない: 窓口の依頼の送出端を受信端に差し替え、頼みは窓口の頼みの送出端へ
//! 直接入れる。kanade の代わりは置き場の中身に持たせた送出端の受信端。取り出しの系は `drain` を
//! 直接呼ぶ（フレームは回さない）。どの受け取りも待たない（`try_recv`）。

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use areka_actor::{ReplyReceiver, reply_channel};
use areka_ghost::BasewareRoot;
use areka_kanade::{KanadeMsg, KanadeNotice, RaiseOutcome, ShioriMethod};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use sample_ghost_kit::{NarBuilder, install_txt};
use temp_path_kit::TempPath;
use tracing::Level;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::ghost_switch::{
    PrevGhost, SwitchInFlight, SwitchStage, SwitchTarget, on_notice,
};
use crate::exit_wait::begin_close;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::install::procedure::InstalledRecord;
use crate::install::{InstallOrigin, SubmitVerdict, register, submit};

// ---------------------------------------------------------------- 道具立て

/// 窓口を据えた World と、背景のスレッドの代わり（依頼の受信端）と、頼みの送出端。
struct Rig {
    world: World,
    orders: Receiver<InstallOrder>,
    asks: Sender<DeskAsk>,
}

impl Rig {
    fn new() -> Self {
        let mut world = World::new();
        world.init_resource::<Schedules>();
        register(&mut world);
        let (worker, orders) = mpsc::channel();
        let asks = {
            let mut desk = world.non_send_mut::<InstallDesk>();
            desk.worker = Some(worker);
            desk.asks_tx.clone()
        };
        Rig {
            world,
            orders,
            asks,
        }
    }

    /// 置き場に kanade への送出端だけを持つゴーストを据え、その受信端を返す。
    fn put_ghost(&mut self, dir: PathBuf) -> Receiver<KanadeMsg> {
        let (tx, rx) = mpsc::channel();
        self.world
            .insert_non_send(GhostSlot(Some(GhostSession::for_test(Some(tx), dir))));
        rx
    }

    /// イベントの頼みを 1 件入れ、答えの受信端を返す。
    fn ask_raise(&self, id: &'static str, resend: bool) -> ReplyReceiver<RaiseOutcome> {
        let (reply, answer) = reply_channel();
        self.asks
            .send(DeskAsk::Raise {
                id,
                references: vec!["r0".to_owned(), "r1".to_owned()],
                resend,
                reply,
            })
            .expect("窓口の頼みの受信端は生きている");
        answer
    }

    fn drain(&mut self) {
        drain(&mut self.world);
    }
}

fn order(path: &str) -> InstallOrder {
    InstallOrder {
        archives: vec![PathBuf::from(path)],
        origin: InstallOrigin::Menu,
    }
}

fn reservation() -> SwitchInFlight {
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
        stage: SwitchStage::SendOff,
    }
}

/// kanade の受信端に届いた汎用の通知の入口の送出（id と Reference と GET か）を取り出し、
/// 同梱の返信端へ `outcome` を返す。届いていなければ空。
fn raised(kanade: &Receiver<KanadeMsg>, outcome: RaiseOutcome) -> Vec<(String, Vec<String>)> {
    kanade
        .try_iter()
        .map(|msg| match msg {
            KanadeMsg::RaiseEvent {
                id,
                references,
                method,
                reply,
            } => {
                assert_eq!(
                    method,
                    ShioriMethod::Get,
                    "インストールのイベントは GET で送る"
                );
                let reply = reply.expect("返信端を同梱して送る");
                let _ = reply.send(outcome);
                (id, references)
            }
            _ => panic!("汎用の通知の入口の送出だけが届く"),
        })
        .collect()
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

// ---------------------------------------------------------------- 受付

/// 受けた依頼は待ち行列に並び、`info!(install_order_queued)` が出どころと本数つきで 1 件。
#[test]
fn submit_queues_and_logs_origin_and_count() {
    let mut rig = Rig::new();
    let two = InstallOrder {
        archives: vec![PathBuf::from("C:/a.nar"), PathBuf::from("C:/b.nar")],
        origin: InstallOrigin::Script,
    };
    let (verdict, events) = capture(|| submit(&mut rig.world, two.clone()));
    assert_eq!(verdict, SubmitVerdict::Queued);
    assert_eq!(
        rig.world
            .non_send::<InstallDesk>()
            .queue
            .iter()
            .collect::<Vec<_>>(),
        vec![&two]
    );
    let queued = events_named(&events, "install_order_queued");
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].level, Level::INFO);
    assert_eq!(queued[0].field("origin"), Some("Script"));
    assert_eq!(queued[0].field("count"), Some("2"));
}

/// 書庫 0 本・窓口なし・終了中は断って `warn!` を 1 件残し、待ち行列に積まない。
#[test]
fn submit_refuses_empty_no_desk_and_closing() {
    let mut rig = Rig::new();
    let empty = InstallOrder {
        archives: Vec::new(),
        origin: InstallOrigin::Menu,
    };
    let (verdict, events) = capture(|| submit(&mut rig.world, empty));
    assert_eq!(verdict, SubmitVerdict::Empty);
    assert_eq!(events.iter().filter(|e| e.level == Level::WARN).count(), 1);

    let mut bare = World::new();
    let (verdict, events) = capture(|| submit(&mut bare, order("C:/a.nar")));
    assert_eq!(verdict, SubmitVerdict::NoDesk);
    assert_eq!(events.iter().filter(|e| e.level == Level::WARN).count(), 1);

    let _ = begin_close(&mut rig.world);
    let (verdict, events) = capture(|| submit(&mut rig.world, order("C:/a.nar")));
    assert_eq!(verdict, SubmitVerdict::Closing);
    assert_eq!(events.iter().filter(|e| e.level == Level::WARN).count(), 1);
    assert!(rig.world.non_send::<InstallDesk>().queue.is_empty());
    assert!(events_named(&events, "install_order_queued").is_empty());
}

// ---------------------------------------------------------------- 待ち行列

/// 生の要求は取り出しで受付を通って依頼になり、手続きの最中に届いた 2 件は、前の依頼の
/// 終わりの知らせのたびに 1 件ずつ、届いた順に背景のスレッドへ渡る（要件 9.3）。
#[test]
fn orders_arriving_mid_procedure_reach_the_worker_in_order() {
    let mut rig = Rig::new();
    let raw = raw_sender(&rig.world);
    let send = |path: &str| {
        raw.send(RawInstallRequest {
            path: PathBuf::from(path),
            origin: InstallOrigin::Script,
        })
        .expect("窓口の生の要求の受信端は生きている");
    };

    send("C:/1.nar");
    rig.drain();
    let first: Vec<InstallOrder> = rig.orders.try_iter().collect();
    assert_eq!(
        first,
        vec![InstallOrder {
            archives: vec![PathBuf::from("C:/1.nar")],
            origin: InstallOrigin::Script,
        }]
    );

    // 1 件目の手続きの最中に 2 件。
    send("C:/2.nar");
    send("C:/3.nar");
    rig.drain();
    rig.drain();
    assert_eq!(rig.orders.try_iter().count(), 0, "手続きの最中は渡さない");

    rig.asks.send(DeskAsk::OrderDone).unwrap();
    rig.drain();
    let second: Vec<Vec<PathBuf>> = rig.orders.try_iter().map(|o| o.archives).collect();
    assert_eq!(second, vec![vec![PathBuf::from("C:/2.nar")]]);

    rig.drain();
    assert_eq!(
        rig.orders.try_iter().count(),
        0,
        "終わりの知らせまで次は渡さない"
    );

    rig.asks.send(DeskAsk::OrderDone).unwrap();
    rig.drain();
    let third: Vec<Vec<PathBuf>> = rig.orders.try_iter().map(|o| o.archives).collect();
    assert_eq!(third, vec![vec![PathBuf::from("C:/3.nar")]]);
}

/// 窓口が無ければ、配る送出端は受信端の無い送出端（送ると失敗する）。
#[test]
fn raw_sender_without_desk_has_no_receiver() {
    let world = World::new();
    let raw = raw_sender(&world);
    assert!(
        raw.send(RawInstallRequest {
            path: PathBuf::from("C:/a.nar"),
            origin: InstallOrigin::Menu,
        })
        .is_err()
    );
}

// ---------------------------------------------------------------- 送出の保留

/// 切替の予約が在る間は送らず、予約が下りた tick に送る。送り先は送る時点の置き場のゴースト
/// （予約の間に置き場が替わったら、替わった後のゴーストへ）。返事は kanade から直接届く。
#[test]
fn raise_waits_for_the_switch_and_goes_to_the_ghost_at_send_time() {
    let mut rig = Rig::new();
    let old_kanade = rig.put_ghost("ghost/A".into());
    rig.world.insert_non_send(reservation());

    let answer = rig.ask_raise("OnInstallBegin", false);
    rig.drain();
    rig.drain();
    assert!(
        raised(&old_kanade, RaiseOutcome::Script).is_empty(),
        "予約の間は送らない"
    );
    assert!(
        matches!(answer.try_recv(), Ok(None)),
        "頼みは手元に置く（返信端は生きている）"
    );

    // 切替で置き場のゴーストが替わり、予約が下りた。
    let new_kanade = rig.put_ghost("ghost/B".into());
    rig.world.remove_non_send::<SwitchInFlight>();
    rig.drain();
    assert!(raised(&old_kanade, RaiseOutcome::Script).is_empty());
    assert_eq!(
        raised(&new_kanade, RaiseOutcome::Script),
        vec![(
            "OnInstallBegin".to_owned(),
            vec!["r0".to_owned(), "r1".to_owned()]
        )]
    );
    assert!(matches!(answer.try_recv(), Ok(Some(RaiseOutcome::Script))));

    rig.drain();
    assert!(
        raised(&new_kanade, RaiseOutcome::Script).is_empty(),
        "送るのは 1 回だけ"
    );
}

/// 置き場にゴーストが居ない間は送らず、居るようになった tick に送る。
#[test]
fn raise_waits_for_a_ghost_in_the_slot() {
    let mut rig = Rig::new();
    let _answer = rig.ask_raise("OnInstallBegin", false);
    rig.drain();
    let kanade = rig.put_ghost("ghost/A".into());
    rig.drain();
    assert_eq!(raised(&kanade, RaiseOutcome::NoReply).len(), 1);
}

/// 送り直しの頼みは、前に送った後に定常到達が届くまで送らない（定常到達の回数で比べる）。
#[test]
fn resend_waits_for_a_steady_arrival_after_the_last_send() {
    let mut rig = Rig::new();
    let kanade = rig.put_ghost("ghost/A".into());

    let _first = rig.ask_raise("OnInstallBegin", false);
    rig.drain();
    assert_eq!(raised(&kanade, RaiseOutcome::NotSteady).len(), 1);

    let again = rig.ask_raise("OnInstallBegin", true);
    rig.drain();
    rig.drain();
    assert!(
        raised(&kanade, RaiseOutcome::Script).is_empty(),
        "定常到達の前は送らない"
    );

    on_steady(&mut rig.world);
    rig.drain();
    assert_eq!(raised(&kanade, RaiseOutcome::Script).len(), 1);
    assert!(matches!(again.try_recv(), Ok(Some(RaiseOutcome::Script))));
}

/// 前に送った後・送り直しの頼みが届く前に定常到達が届いていれば、直ちに送り直す。定常到達は
/// 切替の道筋の通知の受け手（`ghost_switch::on_notice`）から窓口へ届く。
#[test]
fn resend_goes_at_once_when_steady_arrived_since_the_last_send() {
    let mut rig = Rig::new();
    let kanade = rig.put_ghost("ghost/A".into());

    let _first = rig.ask_raise("OnInstallBegin", false);
    rig.drain();
    assert_eq!(raised(&kanade, RaiseOutcome::NotSteady).len(), 1);

    on_notice(&mut rig.world, KanadeNotice::Steady);
    let _again = rig.ask_raise("OnInstallBegin", true);
    rig.drain();
    assert_eq!(raised(&kanade, RaiseOutcome::Script).len(), 1);
}

// ---------------------------------------------------------------- 素性・上書き・記録

/// 今のゴーストの素性: 根・フォルダ名・名前（実行系が無ければフォルダ名）・`sakura.name`・
/// `install.accept` を答える。起動の文脈か置き場のゴーストが無ければ None。
#[test]
fn facts_describe_the_running_ghost() {
    let tmp = TempPath::new("desk-facts");
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let master = root.ghost_dir("A").join("ghost").join("master");
    std::fs::create_dir_all(&master).unwrap();
    std::fs::write(
        master.join("descript.txt"),
        "charset,UTF-8\r\nname,Alice\r\nsakura.name,さくら\r\ninstall.accept,甲, 乙\r\n",
    )
    .unwrap();

    let mut rig = Rig::new();
    let ask_facts = |rig: &mut Rig| {
        let (reply, answer) = reply_channel();
        rig.asks.send(DeskAsk::Facts { reply }).unwrap();
        rig.drain();
        answer.try_recv().expect("直ちに答える")
    };
    assert_eq!(ask_facts(&mut rig), Some(None), "文脈もゴーストも無い");

    rig.world.insert_resource(boot_context(&root, "A"));
    rig.put_ghost(root.ghost_dir("A"));
    assert_eq!(
        ask_facts(&mut rig),
        Some(Some(GhostFacts {
            root: tmp.path().to_path_buf(),
            folder: Some("A".to_owned()),
            name: "A".to_owned(),
            sakura_name: Some("さくら".to_owned()),
            install_accept: vec!["甲".to_owned(), "乙".to_owned()],
        }))
    );
}

/// 起動中のゴーストへの上書きの頼みは、8.1 まで書庫を返す（もう起動中のゴーストではない）。
/// 入れた後の記録の頼みは直ちに答える（中身は 6.3）。
#[test]
fn overwrite_returns_the_archive_and_record_answers() {
    let tmp = TempPath::new("desk-overwrite");
    let path = tmp.child("newbie.nar");
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                "name,あたらしい",
                "directory,newbie",
            ]),
        )
        .done()
        .file(
            "ghost/master/descript.txt",
            b"charset,UTF-8\r\nname,Newbie\r\n",
        )
        .done()
        .write_to(&path)
        .unwrap();
    let archive = areka_nar::NarArchive::open(&path).unwrap();

    let mut rig = Rig::new();
    let (reply, answer) = reply_channel();
    rig.asks
        .send(DeskAsk::Overwrite {
            archive,
            target_ghost: None,
            reply,
        })
        .unwrap();
    let (record_reply, recorded) = reply_channel();
    rig.asks
        .send(DeskAsk::Record {
            record: InstalledRecord {
                kind: areka_nar::InstallKind::Ghost,
                object_name: "あたらしい".to_owned(),
                folder: "newbie".to_owned(),
                ghost_folder: Some("newbie".to_owned()),
            },
            reply: record_reply,
        })
        .unwrap();
    rig.drain();
    match answer.try_recv() {
        Ok(Some(Overwritten::NotRunning(back))) => {
            assert_eq!(back.manifest().directory, "newbie")
        }
        _ => panic!("書庫が返る"),
    }
    assert!(matches!(recorded.try_recv(), Ok(Some(()))));
}

fn boot_context(root: &BasewareRoot, current: &str) -> BootContext {
    BootContext {
        root: root.clone(),
        app_profile_dir: root.dir().join("profile"),
        helper_exe: root.dir().join("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: root.ghost_dir(current),
                balloon_root: root.balloon_dir("B"),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: root.ghost_dir(current),
                folder: Some(current.to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: root.balloon_dir("B"),
                folder: Some("B".to_owned()),
            },
        },
    }
}
