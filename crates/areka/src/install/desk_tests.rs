//! UI 側の窓口の決定論テスト（design「Testing Strategy / 窓口と入口」・要件 1.1・1.9・1.10・2.10・2.13・9.3）。
//!
//! 確かめること: 受付の 4 つの判定と記録・手続きの最中に届いた依頼が届いた順に 1 件ずつ背景の
//! スレッドへ渡ること・イベントの頼みは切替の予約が在る間は送らず、予約が下りた tick に送る
//! 時点のゴーストへ送ること・送り直しの頼みは前に送った後に定常到達が届いてから送ること・
//! 素性の答え・宛先が起動中のゴーストでない上書きの頼みは書庫を返すこと・起動中のゴーストへの
//! 上書きの切替が中止されたら「預かった」へ戻して次の定常到達で頼み直すこと・切替の入口が
//! `NotFound`／`NoContext` なら `warn!` を 1 件残して書庫を返すこと。
//!
//! 背景のスレッドは起こさない: 窓口の依頼の送出端を受信端に差し替え、頼みは窓口の頼みの送出端へ
//! 直接入れる。kanade の代わりは置き場の中身に持たせた送出端の受信端。取り出しの系は `drain` を
//! 直接呼ぶ（フレームは回さない）。どの受け取りも待たない（`try_recv`）。

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use areka_actor::{ReplyReceiver, reply_channel};
use areka_ghost::BasewareRoot;
use areka_kanade::{CancelReason, KanadeMsg, KanadeNotice, RaiseOutcome, ShioriMethod};
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
use crate::install::procedure::{InstalledRecord, Overwritten};
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

    /// 起動中のゴーストへの上書きの頼みを 1 件入れ、答えの受信端を返す。
    fn ask_overwrite(&self, archive: areka_nar::NarArchive) -> ReplyReceiver<Overwritten> {
        let (reply, answer) = reply_channel();
        self.asks
            .send(DeskAsk::Overwrite {
                archive,
                target_ghost: None,
                reply,
            })
            .expect("窓口の頼みの受信端は生きている");
        answer
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
        boot_event: None,
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

/// 起動の文脈もゴーストも無いときの上書きの頼みは、切替の入口が断る（`NoContext`）ので書庫を返す。
/// 起動の文脈もゴーストも無くても、入れた後の記録の頼みには答える（中身は `desk_record_tests.rs`）。
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

/// 目録に載るゴースト（`descript.txt` の `name` はフォルダ名）を 1 体置いた根。
fn root_with_ghost(tmp: &TempPath, folder: &str) -> BasewareRoot {
    let root = BasewareRoot::new(tmp.path().to_path_buf());
    let master = root.ghost_dir(folder).join("ghost").join("master");
    std::fs::create_dir_all(&master).unwrap();
    std::fs::write(
        master.join("descript.txt"),
        format!(
            "charset,UTF-8
name,{folder}
"
        ),
    )
    .unwrap();
    root
}

/// `directory,<folder>` の `ghost` の書庫。
fn ghost_archive(tmp: &TempPath, folder: &str) -> areka_nar::NarArchive {
    let path = tmp.child(&format!("{folder}.nar"));
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "charset,UTF-8",
                "type,ghost",
                &format!("name,{folder}"),
                &format!("directory,{folder}"),
            ]),
        )
        .done()
        .file("ghost/master/overwritten.txt", b"new")
        .done()
        .write_to(&path)
        .unwrap();
    areka_nar::NarArchive::open(&path).unwrap()
}

/// kanade の受信端に届いた切替の要求（切替先の名前・知らせを送るか・出どころの綴り）を取り出す。
fn change_requests(kanade: &Receiver<KanadeMsg>) -> Vec<(String, bool, &'static str)> {
    kanade
        .try_iter()
        .map(|msg| match msg {
            KanadeMsg::ChangeGhost(req) => {
                (req.target.name, req.raise_event, req.origin.as_ref_str())
            }
            _ => panic!("切替の要求だけが届く"),
        })
        .collect()
}

/// 「切替を頼んだ」のまま切替が中止された（予約が下りた）ら「預かった」へ戻し、その tick では
/// 頼み直さず、次の定常到達で頼み直す（design「desk / State Management」）。頼みはどれも同じ
/// フォルダの名指し・知らせなし・出どころ「自動」。背景のスレッドは答えを待ち続ける（捨てない）。
#[test]
fn a_cancelled_overwrite_switch_goes_back_to_held_and_retries_on_the_next_steady() {
    let tmp = TempPath::new("desk-overwrite-cancelled");
    let root = root_with_ghost(&tmp, "A");
    let mut rig = Rig::new();
    rig.world.insert_resource(boot_context(&root, "A"));
    let kanade = rig.put_ghost(root.ghost_dir("A"));
    let answer = rig.ask_overwrite(ghost_archive(&tmp, "A"));
    let same = ("A".to_owned(), false, "automatic");

    let ((), events) = capture(|| {
        rig.drain();
        assert_eq!(change_requests(&kanade), vec![same.clone()], "切替を頼む");
        on_notice(
            &mut rig.world,
            KanadeNotice::ChangeCancelled {
                reason: CancelReason::Rejected,
            },
        );
        rig.drain();
        rig.drain();
        assert!(
            change_requests(&kanade).is_empty(),
            "中止の後の tick では頼み直さない"
        );
        on_notice(&mut rig.world, KanadeNotice::Steady);
        assert_eq!(
            change_requests(&kanade),
            vec![same.clone()],
            "次の定常到達で頼み直す"
        );
        // 中止と定常到達が tick を挟まずに届いても、定常到達で戻して頼み直す。
        on_notice(
            &mut rig.world,
            KanadeNotice::ChangeCancelled {
                reason: CancelReason::Rejected,
            },
        );
        on_notice(&mut rig.world, KanadeNotice::Steady);
        assert_eq!(change_requests(&kanade), vec![same.clone()]);
    });
    assert!(
        matches!(answer.try_recv(), Ok(None)),
        "背景のスレッドは答えを待つ（書庫は窓口が持ち続ける）"
    );
    assert_eq!(
        events_named(&events, "install_overwrite_requeued").len(),
        2,
        "{events:?}"
    );
    let requested = events_named(&events, "ghost_switch_requested");
    assert_eq!(requested.len(), 3, "{events:?}");
    for event in requested {
        assert_eq!(event.field_str("origin"), Some("automatic"), "{event:?}");
        assert_eq!(event.field("raise_event"), Some("false"), "{event:?}");
    }
}

/// 別の切替の最中に預かった書庫（`Busy`）は、その予約が下りた tick に頼み直す。ただし終了が指示された
/// 後は頼み直さない（終了の指示が無くなれば頼み直す＝止めていたのは終了の指示だけ）。
#[test]
fn a_busy_overwrite_is_not_retried_after_the_exit_was_instructed() {
    let tmp = TempPath::new("desk-overwrite-busy-exit");
    let root = root_with_ghost(&tmp, "A");
    let mut rig = Rig::new();
    rig.world.insert_resource(boot_context(&root, "A"));
    let kanade = rig.put_ghost(root.ghost_dir("A"));
    rig.world.insert_non_send(reservation());
    let _answer = rig.ask_overwrite(ghost_archive(&tmp, "A"));
    rig.drain();
    assert!(change_requests(&kanade).is_empty(), "予約の間は頼まない");

    rig.world.remove_non_send::<SwitchInFlight>();
    rig.world.insert_resource(crate::app_exit::FirstExit(
        crate::app_exit::ExitOrigin::Escape,
    ));
    rig.drain();
    assert!(
        change_requests(&kanade).is_empty(),
        "終了の指示の後は頼まない"
    );

    rig.world.remove_resource::<crate::app_exit::FirstExit>();
    rig.drain();
    assert_eq!(
        change_requests(&kanade),
        vec![("A".to_owned(), false, "automatic")]
    );
}

/// 切替の入口が `NotFound`（起動中のゴーストのフォルダが目録に無い）・`NoContext`（送り先の
/// ゴーストが居ない・起動の文脈が無い）と判定したら、`warn!(install_overwrite_unavailable)` を
/// 判定つきで 1 件残し、書庫を背景のスレッドへ返す（切替は頼まない・手続きは止まらない）。
#[test]
fn an_unavailable_switch_returns_the_archive_with_one_warning() {
    let cases = [
        ("NotFound", "not-found", false, true, true),
        ("NoContext", "no-ghost", true, false, true),
        ("NoContext", "no-context", true, true, false),
    ];
    for (verdict, label, listed, ghost, context) in cases {
        let tmp = TempPath::new(&format!("desk-overwrite-{label}"));
        let root = match listed {
            true => root_with_ghost(&tmp, "A"),
            false => BasewareRoot::new(tmp.path().to_path_buf()),
        };
        let mut rig = Rig::new();
        if context {
            rig.world.insert_resource(boot_context(&root, "A"));
        }
        let kanade = ghost.then(|| rig.put_ghost(root.ghost_dir("A")));
        let answer = rig.ask_overwrite(ghost_archive(&tmp, "A"));

        let ((), events) = capture(|| rig.drain());
        match answer.try_recv() {
            Ok(Some(Overwritten::NotRunning(back))) => {
                assert_eq!(back.manifest().directory, "A", "{verdict}")
            }
            _ => panic!("{label}: 書庫が返る"),
        }
        let warned = events_named(&events, "install_overwrite_unavailable");
        assert_eq!(warned.len(), 1, "{label}: {events:?}");
        assert_eq!(warned[0].level, Level::WARN, "{verdict}");
        assert_eq!(warned[0].field("verdict"), Some(verdict), "{events:?}");
        assert!(
            events_named(&events, "ghost_switch_requested").is_empty(),
            "{verdict}: {events:?}"
        );
        if let Some(kanade) = kanade {
            assert!(change_requests(&kanade).is_empty(), "{verdict}");
        }
    }
}

/// 「切替を頼んだ」の段でも、切替の予約の切替先が預かった宛先と違う（その間に利用者や台本が頼んだ
/// 別のゴーストへの切替）なら、全窓を閉じた直後の口は展開せず `debug!(install_overwrite_skipped)` を
/// 残す（背景のスレッドは待ったまま）。切替先が預かった宛先なら（ASCII の大文字小文字は無視）展開する。
#[test]
fn run_between_installs_only_on_the_switch_to_the_held_destination() {
    let tmp = TempPath::new("desk-overwrite-between");
    let root = root_with_ghost(&tmp, "A");
    let mut rig = Rig::new();
    rig.world.insert_resource(boot_context(&root, "A"));
    let kanade = rig.put_ghost(root.ghost_dir("A"));
    let answer = rig.ask_overwrite(ghost_archive(&tmp, "A"));
    rig.drain();
    assert_eq!(
        change_requests(&kanade),
        vec![("A".to_owned(), false, "automatic")],
        "「切替を頼んだ」へ進む"
    );
    let marker = root.ghost_dir("A").join("ghost/master/overwritten.txt");

    // 予約は残したまま、切替先だけが別のゴースト（B）。
    rig.world.insert_non_send(reservation());
    let ((), other) = capture(|| run_overwrite_between(&mut rig.world));
    assert!(
        events_named(&other, "install_overwrite_done").is_empty(),
        "{other:?}"
    );
    assert_eq!(
        events_named(&other, "install_overwrite_skipped").len(),
        1,
        "{other:?}"
    );
    assert!(!marker.exists(), "A のフォルダには展開しない");
    assert!(
        matches!(answer.try_recv(), Ok(None)),
        "背景のスレッドは待ったまま"
    );

    // 切替先が預かった宛先（綴りの大文字小文字だけが違う）。
    rig.world.non_send_mut::<SwitchInFlight>().target.folder = "a".to_owned();
    let ((), ours) = capture(|| run_overwrite_between(&mut rig.world));
    let done = events_named(&ours, "install_overwrite_done");
    assert_eq!(done.len(), 1, "{ours:?}");
    assert_eq!(done[0].field("ok"), Some("true"));
    assert!(marker.exists(), "A のフォルダへ展開する");
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

/// 終了の後始末での窓口の片付け（task 9・要件 8.2・8.5・8.6・11.8）。
#[path = "desk_exit_tests.rs"]
mod exit_tests;
