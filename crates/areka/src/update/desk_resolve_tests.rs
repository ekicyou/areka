//! 対象の解決の兄弟テスト（design「窓口と入口」の対象の解決・要件 1.4・1.7）。
//!
//! 一時の根に emo2 風のフォルダ（`ghost/<名>/ghost/master`・`ghost/<名>/shell/<名>`・
//! `balloon/<名>`）を置き、今の 3 つの `dir`・名前・倒れ先の `homeurl` と、`updateother` の
//! 名前引き（目録の `name` と完全一致）を判定する。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use areka_actor::ReplySender;
use areka_ghost::BasewareRoot;
use areka_kanade::resources::ResourceOutcome;
use areka_kanade::{KanadeMsg, KanadeNotice};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;

use super::{
    Here, Stage, UpdateDesk, can_update, drain, on_steady, resolve, resolve_targets, update_current,
};
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::ghost_switch::on_notice;
use crate::exit_wait::begin_close;
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::menu::captions::QueryReply;
use crate::update::{
    RawUpdateRequest, SummaryKind, TargetKind, TargetSpec, UpdateReason, register,
};

const GHOST_URL: &str = "https://ghost.example/emo/";
const SHELL_URL: &str = "https://shell.example/master/";
const BALLOON_URL: &str = "https://balloon.example/kaku/";

/// 一時の根（捨てると木が消える）。
struct Root {
    tmp: TempPath,
}

impl Root {
    /// `ghost/emo`（名前・homeurl あり）・`ghost/bare`（名前・homeurl なし）・
    /// `ghost/emo/shell/{master,Hidden}`・`balloon/{kaku,plain}` を置く。
    fn new(label: &str) -> Self {
        let tmp = TempPath::new(label);
        let r = tmp.path();
        write(
            &r.join("ghost/emo/ghost/master"),
            &format!("charset,UTF-8\r\nname,Emo\r\nhomeurl,{GHOST_URL}\r\n"),
        );
        write(&r.join("ghost/bare/ghost/master"), "charset,UTF-8\r\n");
        write(
            &r.join("ghost/emo/shell/master"),
            &format!("charset,UTF-8\r\nname,MasterShell\r\nhomeurl,{SHELL_URL}\r\n"),
        );
        write(
            &r.join("ghost/emo/shell/Hidden"),
            "charset,UTF-8\r\nname,HiddenShell\r\nmenu,hidden\r\nhomeurl,https://hidden.example/\r\n",
        );
        write(
            &r.join("balloon/kaku"),
            &format!("charset,UTF-8\r\nname,Kaku\r\nhomeurl,{BALLOON_URL}\r\n"),
        );
        write(&r.join("balloon/plain"), "charset,UTF-8\r\n");
        Root { tmp }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.tmp.path().join(rel)
    }

    fn baseware(&self) -> BasewareRoot {
        BasewareRoot::new(self.tmp.path().to_path_buf())
    }

    /// 今のゴースト `emo`・シェル `master`・バルーン `kaku`（SHIORI の名前は無し）。
    fn here(&self) -> Here {
        Here {
            root: self.baseware(),
            ghost_dir: self.path("ghost/emo"),
            ghost_name: None,
            shell_dir: Some(self.path("ghost/emo/shell/master")),
            balloon_dir: self.path("balloon/kaku"),
        }
    }
}

fn write(dir: &Path, descript: &str) {
    std::fs::create_dir_all(dir).expect("検体のフォルダを作る");
    std::fs::write(dir.join("descript.txt"), descript).expect("descript.txt を書く");
}

fn spec(kind: TargetKind, dir: PathBuf, name: &str, homeurl: Option<&str>) -> TargetSpec {
    TargetSpec {
        kind,
        dir,
        name: name.to_owned(),
        descript_homeurl: homeurl.map(str::to_owned),
    }
}

fn all_three() -> RawUpdateRequest {
    RawUpdateRequest::Current(vec![
        TargetKind::Ghost,
        TargetKind::Shell,
        TargetKind::Balloon,
    ])
}

fn skipped(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some("update_target_skipped"))
        .collect()
}

#[test]
fn current_three_resolve_dirs_names_and_fallback_homeurls() {
    let root = Root::new("areka-update-resolve-current");
    let (specs, events) = capture(|| resolve(&root.here(), &all_three()));
    assert_eq!(
        specs,
        vec![
            spec(
                TargetKind::Ghost,
                root.path("ghost/emo"),
                "Emo",
                Some(GHOST_URL)
            ),
            spec(
                TargetKind::Shell,
                root.path("ghost/emo/shell/master"),
                "MasterShell",
                Some(SHELL_URL),
            ),
            spec(
                TargetKind::Balloon,
                root.path("balloon/kaku"),
                "Kaku",
                Some(BALLOON_URL)
            ),
        ]
    );
    assert!(skipped(&events).is_empty(), "{events:?}");
}

#[test]
fn ghost_name_prefers_the_shiori_name() {
    let root = Root::new("areka-update-resolve-shiori-name");
    let here = Here {
        ghost_name: Some("FromShiori".to_owned()),
        ..root.here()
    };
    let specs = resolve(&here, &RawUpdateRequest::Current(vec![TargetKind::Ghost]));
    assert_eq!(
        specs,
        vec![spec(
            TargetKind::Ghost,
            root.path("ghost/emo"),
            "FromShiori",
            Some(GHOST_URL)
        )]
    );
}

#[test]
fn names_fall_back_to_the_folder_and_homeurl_to_none() {
    let root = Root::new("areka-update-resolve-folder-name");
    let here = Here {
        ghost_dir: root.path("ghost/bare"),
        balloon_dir: root.path("balloon/plain"),
        ..root.here()
    };
    let specs = resolve(
        &here,
        &RawUpdateRequest::Current(vec![TargetKind::Ghost, TargetKind::Balloon]),
    );
    assert_eq!(
        specs,
        vec![
            spec(TargetKind::Ghost, root.path("ghost/bare"), "bare", None),
            spec(
                TargetKind::Balloon,
                root.path("balloon/plain"),
                "plain",
                None
            ),
        ]
    );
}

#[test]
fn a_missing_folder_or_unknown_shell_is_skipped_with_one_warning_each() {
    let root = Root::new("areka-update-resolve-missing");
    let here = Here {
        shell_dir: None,
        balloon_dir: root.path("balloon/gone"),
        ..root.here()
    };
    let (specs, events) = capture(|| resolve(&here, &all_three()));
    assert_eq!(
        specs,
        vec![spec(
            TargetKind::Ghost,
            root.path("ghost/emo"),
            "Emo",
            Some(GHOST_URL)
        )]
    );
    let skipped = skipped(&events);
    assert_eq!(skipped.len(), 2, "{events:?}");
    assert!(skipped.iter().all(|e| e.level == Level::WARN), "{events:?}");
}

#[test]
fn updateother_looks_names_up_exactly_in_the_order_given() {
    let root = Root::new("areka-update-resolve-other");
    let raw = RawUpdateRequest::Other(vec![
        (TargetKind::Balloon, "Kaku".to_owned()),
        (TargetKind::Shell, "MasterShell".to_owned()),
    ]);
    let (specs, events) = capture(|| resolve(&root.here(), &raw));
    assert_eq!(
        specs,
        vec![
            spec(
                TargetKind::Balloon,
                root.path("balloon/kaku"),
                "Kaku",
                Some(BALLOON_URL)
            ),
            spec(
                TargetKind::Shell,
                root.path("ghost/emo/shell/master"),
                "MasterShell",
                Some(SHELL_URL),
            ),
        ]
    );
    assert!(skipped(&events).is_empty(), "{events:?}");
}

#[test]
fn updateother_does_not_find_a_case_mismatched_name() {
    let root = Root::new("areka-update-resolve-case");
    let raw = RawUpdateRequest::Other(vec![
        (TargetKind::Shell, "mastershell".to_owned()),
        (TargetKind::Balloon, "KAKU".to_owned()),
    ]);
    let (specs, events) = capture(|| resolve(&root.here(), &raw));
    assert!(specs.is_empty(), "{specs:?}");
    let skipped = skipped(&events);
    assert_eq!(skipped.len(), 2, "{events:?}");
    assert!(skipped.iter().all(|e| e.level == Level::WARN), "{events:?}");
}

#[test]
fn updateother_does_not_find_a_hidden_shell() {
    let root = Root::new("areka-update-resolve-hidden");
    let raw = RawUpdateRequest::Other(vec![(TargetKind::Shell, "HiddenShell".to_owned())]);
    let (specs, events) = capture(|| resolve(&root.here(), &raw));
    assert!(specs.is_empty(), "{specs:?}");
    let skipped = skipped(&events);
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].level, Level::WARN);
}

/// 窓口を据え、起動の文脈（バルーン `balloon`）と置き場のゴースト `ghost`（kanade の送出端だけ・
/// 実行系なし＝シェルは解けない）を置いた World と、kanade の代わりの受信端。
fn world(root: &Root, ghost: &str, balloon: &str) -> (World, Receiver<KanadeMsg>) {
    let base = root.baseware();
    let mut world = World::new();
    world.init_resource::<Schedules>();
    register(&mut world);
    world.insert_resource(BootContext {
        root: base.clone(),
        app_profile_dir: root.path("profile"),
        helper_exe: root.path("helper.exe"),
        argv_session: false,
        current: CurrentGhost {
            cfg: ConfigInputs {
                ghost_root: base.ghost_dir(ghost),
                balloon_root: base.balloon_dir(balloon),
            },
            ghost: GhostDecision {
                route: GhostRoute::Default,
                dir: base.ghost_dir(ghost),
                folder: Some(ghost.to_owned()),
            },
            balloon: BalloonDecision {
                route: BalloonRoute::Default,
                dir: base.balloon_dir(balloon),
                folder: Some(balloon.to_owned()),
            },
        },
    });
    let (tx, kanade) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        base.ghost_dir(ghost),
    ))));
    (world, kanade)
}

/// World から写す口: 置き場のゴーストの根と起動の文脈のバルーンを読む。実行系の無い置き場
/// （テスト用の組み立て）はシェルを解けないので、シェルだけ `warn!` で飛ばす。
#[test]
fn resolve_targets_reads_the_slot_and_the_boot_context() {
    let root = Root::new("areka-update-resolve-world");
    let (world, _kanade) = world(&root, "emo", "kaku");
    let (specs, events) = capture(|| resolve_targets(&world, &all_three()));
    assert_eq!(
        specs,
        vec![
            spec(
                TargetKind::Ghost,
                root.path("ghost/emo"),
                "Emo",
                Some(GHOST_URL)
            ),
            spec(
                TargetKind::Balloon,
                root.path("balloon/kaku"),
                "Kaku",
                Some(BALLOON_URL)
            ),
        ]
    );
    let skipped = skipped(&events);
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].field_str("kind"), Some("shell"));
}

/// 起動の文脈か置き場のゴーストが無ければ何も解かない。
#[test]
fn resolve_targets_without_a_context_resolves_nothing() {
    let world = World::new();
    assert!(resolve_targets(&world, &all_three()).is_empty());
}

// ---------------------------------------------------------------- homeurl の写しと「選べるか」

const COPY_URL: &str = "https://shiori.example/emo/";

/// kanade に届いた照会（名前の列と返信端）。照会でない送出は無い。
fn queries(kanade: &Receiver<KanadeMsg>) -> Vec<(Vec<&'static str>, ReplySender<QueryReply>)> {
    kanade
        .try_iter()
        .map(|msg| match msg {
            KanadeMsg::ResourceQuery { ids, reply } => (ids, reply),
            _ => panic!("照会だけが届く"),
        })
        .collect()
}

fn copy(world: &World) -> Option<String> {
    world.non_send::<UpdateDesk>().ghost_homeurl.clone()
}

/// 3 つとも更新先が無ければ選べない（ゴースト `bare`・バルーン `plain`・シェルは解けない）。
#[test]
fn not_selectable_when_none_of_the_three_has_a_source() {
    let root = Root::new("areka-update-can-none");
    let (world, _kanade) = world(&root, "bare", "plain");
    assert!(!can_update(&world));
}

/// 写しに在れば選べる（`descript.txt` は 3 つとも無い）。
#[test]
fn selectable_when_the_copy_has_one() {
    let root = Root::new("areka-update-can-copy");
    let (mut world, _kanade) = world(&root, "bare", "plain");
    world.non_send_mut::<UpdateDesk>().ghost_homeurl = Some(COPY_URL.to_owned());
    assert!(can_update(&world));
}

/// `descript.txt` の 1 つ（バルーン）に在れば選べる。
#[test]
fn selectable_when_one_descript_has_one() {
    let root = Root::new("areka-update-can-descript");
    let (world, _kanade) = world(&root, "bare", "kaku");
    assert!(can_update(&world));
}

/// 走っている間と答え待ちの間は選べない（段が `Idle` に戻れば選べる）。終了が始まった後・窓口が
/// 無いときも選べない。
#[test]
fn not_selectable_while_an_update_runs_or_after_exit_or_without_a_desk() {
    let root = Root::new("areka-update-can-running");
    let (mut world, _kanade) = world(&root, "emo", "kaku");
    assert!(can_update(&world), "対照: 走っていなければ選べる");
    for stage in [Stage::Running, Stage::AwaitingExec] {
        world.non_send_mut::<UpdateDesk>().stage = stage;
        assert!(!can_update(&world), "{stage:?}");
    }
    world.non_send_mut::<UpdateDesk>().stage = Stage::Idle;
    assert!(can_update(&world), "対照: 段が戻れば選べる");
    let _ = begin_close(&mut world);
    assert!(!can_update(&world), "終了が始まった後");
    world.remove_non_send::<UpdateDesk>();
    assert!(!can_update(&world), "窓口が無い");
}

/// 定常到達のたびに写しを消して `homeurl` の照会が 1 件飛び、空でない返事だけが写しに載る
/// （返事が来るまでは覗いても待たない）。
#[test]
fn reaching_steady_sends_one_query_and_the_reply_lands_in_the_copy() {
    let root = Root::new("areka-update-steady-query");
    let (mut world, kanade) = world(&root, "bare", "plain");
    world.non_send_mut::<UpdateDesk>().ghost_homeurl = Some("https://old.example/".to_owned());

    on_steady(&mut world);
    assert_eq!(copy(&world), None, "定常到達で写しは消える");
    let mut sent = queries(&kanade);
    assert_eq!(sent.len(), 1);
    let (ids, reply) = sent.remove(0);
    assert_eq!(ids, vec!["homeurl"]);

    drain(&mut world);
    assert_eq!(copy(&world), None, "返事の前");
    reply
        .send(vec![(
            "homeurl",
            ResourceOutcome::Value(COPY_URL.to_owned()),
        )])
        .expect("返事待ちは生きている");
    drain(&mut world);
    assert_eq!(copy(&world).as_deref(), Some(COPY_URL));
    assert!(can_update(&world));

    // 次の定常到達: 空の返事は写しに載らない。
    on_steady(&mut world);
    let mut sent = queries(&kanade);
    assert_eq!(sent.len(), 1);
    let (_, reply) = sent.remove(0);
    reply
        .send(vec![("homeurl", ResourceOutcome::Value(String::new()))])
        .expect("返事待ちは生きている");
    drain(&mut world);
    assert_eq!(copy(&world), None);
    assert!(!can_update(&world));
}

/// 終了が始まったら照会の返事待ちは捨てられ、後から返事が来ても写しに載らない。
#[test]
fn exit_discards_the_pending_query_reply() {
    let root = Root::new("areka-update-steady-exit");
    let (mut world, kanade) = world(&root, "bare", "plain");
    on_steady(&mut world);
    let mut sent = queries(&kanade);
    assert_eq!(sent.len(), 1);
    let (_, reply) = sent.remove(0);

    let (_, events) = capture(|| begin_close(&mut world));
    assert_eq!(
        events
            .iter()
            .filter(|e| e.field_str("event") == Some("update_query_discarded"))
            .count(),
        1,
        "{events:?}"
    );
    assert!(world.non_send::<UpdateDesk>().homeurl_query.is_none());
    assert!(
        reply
            .send(vec![(
                "homeurl",
                ResourceOutcome::Value(COPY_URL.to_owned())
            )])
            .is_err(),
        "返事待ちは捨てられている"
    );
    drain(&mut world);
    assert_eq!(copy(&world), None);
}

/// メニューの動作は今の 3 つを理由 `manual`・総括 `Result` で受付へ掛ける（実行系の無い置き場は
/// シェルを解けないので、渡るのはゴーストとバルーン）。
#[test]
fn update_current_submits_the_current_three_as_manual() {
    let root = Root::new("areka-update-current-manual");
    let (mut world, _kanade) = world(&root, "emo", "kaku");
    let (worker, jobs) = mpsc::channel();
    world.non_send_mut::<UpdateDesk>().worker = Some(worker);

    let ((), events) = capture(|| update_current(&mut world));
    // シェルも頼んだ（解けずに飛ばされた）ことを記録で確かめる。
    let skipped = skipped(&events);
    assert_eq!(skipped.len(), 1, "{events:?}");
    assert_eq!(skipped[0].field_str("reason"), Some("no_shell"));
    let orders: Vec<_> = jobs.try_iter().map(|job| job.order).collect();
    assert_eq!(orders.len(), 1);
    let order = &orders[0];
    assert_eq!(order.reason, UpdateReason::Manual);
    assert_eq!(order.summary, SummaryKind::Result);
    let kinds: Vec<_> = order.targets.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, vec![TargetKind::Ghost, TargetKind::Balloon]);
    assert_eq!(world.non_send::<UpdateDesk>().stage, Stage::AwaitingExec);
}

/// 本番の道筋: `ghost_switch::on_notice` の定常到達の腕が窓口の照会を呼ぶ（届くたびに 1 件）。
#[test]
fn steady_notice_sends_one_homeurl_query_each_time() {
    let root = Root::new("areka-update-steady-notice");
    let (mut world, kanade) = world(&root, "bare", "plain");
    on_notice(&mut world, KanadeNotice::Steady);
    let first = queries(&kanade);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].0, vec!["homeurl"]);
    on_notice(&mut world, KanadeNotice::Steady);
    let second = queries(&kanade);
    assert_eq!(second.len(), 1, "2 度目の定常到達で 1 件（計 2 件）");
    assert_eq!(second[0].0, vec!["homeurl"]);
}

/// 照会の失敗と返信端の断は、どちらも `warn!(update_homeurl_query_failed)` を 1 件残して写しを空のまま
/// にする（返事待ちも下ろす）。
#[test]
fn a_failed_or_dropped_reply_warns_once_and_leaves_the_copy_empty() {
    let root = Root::new("areka-update-steady-failed");
    let (mut world, kanade) = world(&root, "bare", "plain");
    let failed_warnings = |events: &[CapturedEvent]| {
        events
            .iter()
            .filter(|e| e.field_str("event") == Some("update_homeurl_query_failed"))
            .map(|e| e.level)
            .collect::<Vec<_>>()
    };

    on_steady(&mut world);
    let (_, reply) = queries(&kanade).remove(0);
    reply
        .send(vec![(
            "homeurl",
            ResourceOutcome::Failed("timeout".to_owned()),
        )])
        .expect("返事待ちは生きている");
    let ((), events) = capture(|| drain(&mut world));
    assert_eq!(failed_warnings(&events), vec![Level::WARN], "{events:?}");
    assert_eq!(copy(&world), None);
    assert!(world.non_send::<UpdateDesk>().homeurl_query.is_none());

    on_steady(&mut world);
    let (_, reply) = queries(&kanade).remove(0);
    drop(reply);
    let ((), events) = capture(|| drain(&mut world));
    assert_eq!(failed_warnings(&events), vec![Level::WARN], "{events:?}");
    assert_eq!(copy(&world), None);
    assert!(world.non_send::<UpdateDesk>().homeurl_query.is_none());
}

/// 終了が始まった後の定常到達は照会を掛け直さない（kanade へ 0 件・返事待ちも持たない）。
#[test]
fn steady_after_exit_does_not_query_again() {
    let root = Root::new("areka-update-steady-after-exit");
    let (mut world, kanade) = world(&root, "bare", "plain");
    let _ = begin_close(&mut world);
    on_steady(&mut world);
    assert!(queries(&kanade).is_empty());
    assert!(world.non_send::<UpdateDesk>().homeurl_query.is_none());
}
