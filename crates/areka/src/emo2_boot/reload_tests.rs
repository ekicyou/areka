//! 読み直しの窓口の判定・断り・始めの記録・終了の門の決定論テスト（areka-P0-mcp-reload 要件 1.1・1.6・
//! 2.1・2.2・2.4・3.1・3.2・3.7・7.4・7.5・8.4・9.1・9.2・9.3・12.1 ⑶⑹）。
//!
//! 置き場のゴーストは kanade の送出端だけを持つテスト用の組み立て（名前が無いので、記録のゴースト名は
//! ルートフォルダの絶対パス）。判定は集めてから 1 回。

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use areka_actor::ReplySender;
use areka_actor::reply_channel;
use areka_ghost::BasewareRoot;
use areka_kanade::{
    BootOrigin, GapLeft, GapRaise, KanadeMsg, RaiseOutcome, ReloadScope, ShioriMethod, TalkGap,
};
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::ghost_switch::{
    PrevGhost, SwitchEnd, SwitchStage, SwitchTarget, WelcomeAttempt,
};
use crate::emo2_boot::reload_args::DescriptParts;
use crate::emo2_boot::shell_balloon_resolve::SkinCandidate;
use crate::emo2_boot::shell_balloon_switch::{SkinKind, SkinSwitchStage};
use crate::ghost_session::GhostSession;
use crate::install::desk::{InstallDesk, insert_busy_desk_for_test};
use crate::update::desk::{UpdateDesk, insert_running_desk_for_test};

struct Rig {
    world: World,
    _kanade: Receiver<KanadeMsg>,
    ghost: String,
    _temp: TempPath,
}

/// 起動の文脈・置き場のゴースト（kanade の送出端だけ）・窓口の登録をそろえた World。
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
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        root.ghost_dir("A"),
    ))));
    register(&mut world);
    let ghost = std::path::absolute(root.ghost_dir("A"))
        .unwrap()
        .display()
        .to_string();
    Rig {
        world,
        _kanade: kanade,
        ghost,
        _temp: temp,
    }
}

fn mcp(target: ReloadTarget) -> ReloadRequest {
    ReloadRequest {
        target,
        origin: ReloadOrigin::Mcp,
        notes: DescriptNotes::default(),
    }
}

fn plant_reload(world: &mut World) {
    world.insert_non_send(ReloadInFlight {
        target: ReloadTarget::Shell,
        origin: ReloadOrigin::Script,
        ghost: "planted".to_owned(),
        stage: ReloadStage::Switching {
            reserved: PathBuf::from("planted"),
        },
    });
}

fn plant_ghost_switch(world: &mut World) {
    world.insert_non_send(SwitchInFlight {
        target: SwitchTarget {
            dir: PathBuf::from("B"),
            folder: "B".to_owned(),
            name: "B".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: PathBuf::from("A"),
            name: None,
            sakura_name: None,
        },
        stage: SwitchStage::SendOff,
        boot_event: None,
        copy: crate::emo2_boot::switch_copy::SwitchCopy::None,
    });
}

fn plant_skin_switch(world: &mut World) {
    world.insert_non_send(SkinSwitchInFlight {
        kind: SkinKind::Shell,
        target: SkinCandidate {
            dir: PathBuf::from("planted"),
            folder: "planted".to_owned(),
            name: None,
            hidden: false,
        },
        stage: SkinSwitchStage::Waiting {
            gap: Some(reply_channel().1),
            gap_result: None,
            build: mpsc::channel().1,
            built: None,
        },
    });
}

fn named<'a>(events: &'a [CapturedEvent], event: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

/// 1 件の要求を記録つきで出し、(判定, `reload_refused` の (水準, 理由, ゴースト) の並び, warn 以上の総数)。
fn refuse(
    rig: &mut Rig,
    request: ReloadRequest,
) -> (ReloadVerdict, Vec<(Level, String, String)>, usize) {
    let (verdict, events) = capture(|| request_reload(&mut rig.world, request));
    let refused = named(&events, "reload_refused")
        .into_iter()
        .map(|e| {
            (
                e.level,
                e.field_str("reason").unwrap_or_default().to_owned(),
                e.field("ghost").unwrap_or_default().to_owned(),
            )
        })
        .collect();
    let warns = events.iter().filter(|e| e.level <= Level::WARN).count();
    (verdict, refused, warns)
}

/// 判定は 別の読み直し → ゴースト切替 → シェル／バルーン切替 → 更新 → インストール → 文脈 の順。
/// 断りの全部を立ててから 1 つずつ外し、各段で `reload_refused` が理由とゴースト名つきで 1 件だけ残り、
/// 印は置かれない（要件 2.1・9.1・9.2・12.1 ⑶）。
#[test]
fn refusals_in_order_leave_one_record_each() {
    let mut rig = rig("areka-reload-refusals");
    plant_reload(&mut rig.world);
    plant_ghost_switch(&mut rig.world);
    plant_skin_switch(&mut rig.world);
    insert_running_desk_for_test(&mut rig.world);
    insert_busy_desk_for_test(&mut rig.world);
    rig.world.remove_resource::<BootContext>();
    let ghost = rig.ghost.clone();
    let one = |reason: &str| vec![(Level::WARN, reason.to_owned(), ghost.clone())];

    let mut got = Vec::new();
    got.push(refuse(&mut rig, mcp(ReloadTarget::Ghost)));
    rig.world.remove_non_send::<ReloadInFlight>();
    got.push(refuse(&mut rig, mcp(ReloadTarget::Ghost)));
    rig.world.remove_non_send::<SwitchInFlight>();
    got.push(refuse(&mut rig, mcp(ReloadTarget::Ghost)));
    rig.world.remove_non_send::<SkinSwitchInFlight>();
    got.push(refuse(&mut rig, mcp(ReloadTarget::Ghost)));
    rig.world.remove_non_send::<UpdateDesk>();
    got.push(refuse(&mut rig, mcp(ReloadTarget::Ghost)));
    rig.world.remove_non_send::<InstallDesk>();
    got.push(refuse(&mut rig, mcp(ReloadTarget::Ghost)));

    use RefusedReason as R;
    assert_eq!(
        (got, rig.world.get_non_send::<ReloadInFlight>().is_none()),
        (
            vec![
                (
                    ReloadVerdict::Refused(R::ReloadInFlight),
                    one("reload_in_flight"),
                    1
                ),
                (
                    ReloadVerdict::Refused(R::GhostSwitching),
                    one("ghost_switching"),
                    1
                ),
                (
                    ReloadVerdict::Refused(R::SkinSwitching),
                    one("skin_switching"),
                    1
                ),
                (ReloadVerdict::Refused(R::Updating), one("updating"), 1),
                (ReloadVerdict::Refused(R::Installing), one("installing"), 1),
                (ReloadVerdict::Refused(R::NoContext), one("no_context"), 1),
            ],
            true,
        ),
        "(各段の (判定, reload_refused, warn 以上の総数), 印が無い)"
    );
}

/// 文脈なし: 置き場のゴーストが無い・kanade の送出端が無いときも `no_context` で断る。
#[test]
fn missing_ghost_or_kanade_is_no_context() {
    let mut rig = rig("areka-reload-no-context");
    rig.world
        .insert_non_send(GhostSlot(Some(GhostSession::for_test(
            None,
            PathBuf::from("A"),
        ))));
    let no_kanade = request_reload(&mut rig.world, mcp(ReloadTarget::Shell));
    rig.world.insert_non_send(GhostSlot(None));
    let no_slot = refuse(&mut rig, mcp(ReloadTarget::Shell));
    assert_eq!(
        (no_kanade, no_slot),
        (
            ReloadVerdict::Refused(RefusedReason::NoContext),
            (
                ReloadVerdict::Refused(RefusedReason::NoContext),
                vec![(Level::WARN, "no_context".to_owned(), String::new())],
                1
            ),
        )
    );
}

/// 終了の途中: 門が閉じたら、ほかの断りより先に `Closing` を返し、warn 以上を残さず（`debug!` の
/// `reload_refused`・reason=closing だけ）始めない。終了の片付けは印を `reload_dropped` で下ろす
/// （要件 2.2）。
#[test]
fn closing_returns_closing_and_exit_drops_the_mark() {
    let mut rig = rig("areka-reload-closing");
    plant_reload(&mut rig.world);
    let (_, events) = capture(|| crate::exit_wait::begin_close(&mut rig.world));
    let dropped = named(&events, "reload_dropped")
        .into_iter()
        .map(|e| {
            (
                e.level,
                e.field_str("reason").map(str::to_owned),
                e.field("ghost").map(str::to_owned),
            )
        })
        .collect::<Vec<_>>();
    let mark_after_exit = rig.world.get_non_send::<ReloadInFlight>().is_some();

    plant_ghost_switch(&mut rig.world);
    let (verdict, events) = capture(|| request_reload(&mut rig.world, mcp(ReloadTarget::Ghost)));
    let closing = named(&events, "reload_refused")
        .into_iter()
        .map(|e| (e.level, e.field_str("reason").map(str::to_owned)))
        .collect::<Vec<_>>();
    let warns = events.iter().filter(|e| e.level <= Level::WARN).count();

    assert_eq!(
        (dropped, mark_after_exit, verdict, closing, warns),
        (
            vec![(
                Level::INFO,
                Some("exit".to_owned()),
                Some("planted".to_owned())
            )],
            false,
            ReloadVerdict::Closing,
            vec![(Level::DEBUG, Some("closing".to_owned()))],
            0,
        )
    );
}

/// descript の語: 無い種類・知らない語は語ごとに `warn!` 1 件、読み直せる語が無ければ
/// `reload_descript_no_words` 1 件で `NoWords`（`reload_started` は残らない・要件 7.4・7.5）。
#[test]
fn descript_words_warn_per_word_and_no_words_refuses() {
    let mut rig = rig("areka-reload-descript-words");
    let nothing = DescriptParts {
        ghost: false,
        shell: false,
        balloon: false,
    };
    let req = ReloadRequest {
        target: ReloadTarget::Descript(nothing),
        origin: ReloadOrigin::Script,
        notes: DescriptNotes {
            unsupported: vec!["headline".to_owned(), "calendar skin".to_owned()],
            unknown: vec!["nosuchword".to_owned()],
            empty: true,
        },
    };
    let (verdict, events) = capture(|| request_reload(&mut rig.world, req));
    let words = |event: &str| {
        named(&events, event)
            .into_iter()
            .map(|e| (e.level, e.field("word").map(str::to_owned)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        (
            verdict,
            words("reload_descript_unsupported"),
            words("reload_descript_unknown"),
            words("reload_descript_no_words"),
            named(&events, "reload_refused").len(),
            named(&events, "reload_started").len(),
        ),
        (
            ReloadVerdict::Refused(RefusedReason::NoWords),
            vec![
                (Level::WARN, Some("headline".to_owned())),
                (Level::WARN, Some("calendar skin".to_owned())),
            ],
            vec![(Level::WARN, Some("nosuchword".to_owned()))],
            vec![(Level::WARN, None)],
            0,
            0,
        )
    );
}

/// 受理: `reload_started` が対象・出どころ・ゴースト名つきで 1 件。ここの置き場（結線も seriko の送り手も
/// 無い）では descript の列のシェルの段が始められず、印を下ろして `reload_failed`（no_context）を
/// `TARGET_ERROR` で残す（要件 1.6・9.3・列の本体は `reload_skin_tests.rs`）。
#[test]
fn accepted_leaves_one_started_record() {
    let mut rig = rig("areka-reload-started");
    let req = ReloadRequest {
        target: ReloadTarget::Descript(DescriptParts::ALL),
        origin: ReloadOrigin::Script,
        notes: DescriptNotes {
            unsupported: Vec::new(),
            unknown: vec!["x".to_owned()],
            empty: false,
        },
    };
    let (verdict, events) = capture(|| request_reload(&mut rig.world, req));
    let started = named(&events, "reload_started")
        .into_iter()
        .map(|e| {
            (
                e.level,
                e.field("origin").map(str::to_owned),
                e.field("ghost").map(str::to_owned),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        (
            verdict,
            started,
            named(&events, "reload_descript_unknown").len(),
            named(&events, "reload_failed")
                .iter()
                .map(|e| (
                    e.level,
                    e.target.clone(),
                    e.field_str("reason").map(str::to_owned)
                ))
                .collect::<Vec<_>>(),
            rig.world.get_non_send::<ReloadInFlight>().is_none(),
        ),
        (
            ReloadVerdict::Accepted,
            vec![(
                Level::INFO,
                Some("Script".to_owned()),
                Some(rig.ghost.clone())
            )],
            1,
            vec![(
                Level::ERROR,
                TARGET_ERROR.to_owned(),
                Some("no_context".to_owned())
            )],
            true,
        )
    );
}

/// 取り出しの系: 台本の受信端の要求を全件、出どころ「台本」で窓口へ渡す（ここから台本の道が生きる）。
/// 受信端が無ければ無操作。
#[test]
fn drain_hands_script_requests_to_the_desk() {
    let mut rig = rig("areka-reload-drain");
    drain(&mut rig.world);
    let (tx, rx) = mpsc::channel();
    crate::emo2_boot::reload_cue::wire_reload_rx(&mut rig.world, rx);
    for target in [ReloadTarget::Shell, ReloadTarget::Balloon] {
        tx.send(ScriptReload {
            target,
            notes: DescriptNotes::default(),
        })
        .unwrap();
    }
    let ((), events) = capture(|| drain(&mut rig.world));
    let started = named(&events, "reload_started")
        .into_iter()
        .map(|e| {
            (
                e.field("target").map(str::to_owned),
                e.field("origin").map(str::to_owned),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        started,
        vec![
            (Some("Shell".to_owned()), Some("Script".to_owned())),
            (Some("Balloon".to_owned()), Some("Script".to_owned())),
        ]
    );
}

/// 印と同じ鍵か違う鍵の予約を置く（`plant_ghost_switch` の切替先の場所だけ差し替える）。
fn plant_switch_to(world: &mut World, dir: PathBuf) {
    plant_ghost_switch(world);
    world.non_send_mut::<SwitchInFlight>().target.dir = dir;
}

fn plant_switching(world: &mut World, target: ReloadTarget, reserved: PathBuf) {
    world.insert_non_send(ReloadInFlight {
        target,
        origin: ReloadOrigin::Mcp,
        ghost: "planted".to_owned(),
        stage: ReloadStage::Switching { reserved },
    });
}

/// ghost の受理: 目録を引かず今のゴーストの場所で `raise_event: false` の切替を kanade へ送り、
/// 立った予約の鍵（切替先の場所）を印に控える。自分の予約なので由来は `Reloaded{Ghost}`、実行系だけの
/// 組み直しは偽（要件 2.4・3.1・3.2）。
#[test]
fn ghost_reload_reserves_the_current_ghost() {
    let mut rig = rig("areka-reload-ghost-start");
    let dir = rig
        .world
        .resource::<BootContext>()
        .current
        .ghost
        .dir
        .clone();
    let (verdict, events) = capture(|| request_reload(&mut rig.world, mcp(ReloadTarget::Ghost)));
    let sent = rig
        ._kanade
        .try_iter()
        .map(|m| match m {
            KanadeMsg::ChangeGhost(c) => Some((c.raise_event, c.target.dir)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let reserved = rig
        .world
        .get_non_send::<ReloadInFlight>()
        .and_then(|f| match &f.stage {
            ReloadStage::Switching { reserved } => Some(reserved.clone()),
            _ => None,
        });
    assert_eq!(
        (
            verdict,
            sent,
            rig.world
                .get_non_send::<SwitchInFlight>()
                .map(|f| f.target.dir.clone()),
            reserved,
            named(&events, "reload_failed").len(),
            boot_origin_override(&rig.world),
            wants_runtime_only(&rig.world),
        ),
        (
            ReloadVerdict::Accepted,
            vec![Some((false, rig.ghost.clone()))],
            Some(dir.clone()),
            Some(dir),
            0,
            Some(BootOrigin::Reloaded {
                scope: ReloadScope::Ghost
            }),
            false,
        )
    );
}

/// 予約の各終わり方を終わりの記録 1 件へ写して印を下ろす: 定常（切替先）は `reload_done`、それ以外は
/// `reload_failed(reason)` を `TARGET_ERROR`＋ゴースト名で（要件 3.7・8.4・9.2・9.3・12.1 ⑹）。
#[test]
fn switch_end_leaves_one_end_record_and_drops_the_mark() {
    let mut rig = rig("areka-reload-switch-end");
    let ends = [
        SwitchEnd::Steady {
            attempt: WelcomeAttempt::Target,
        },
        SwitchEnd::Steady {
            attempt: WelcomeAttempt::Default,
        },
        SwitchEnd::Cancelled,
        SwitchEnd::NotAccepted,
        SwitchEnd::TargetQuit,
        SwitchEnd::DefaultFault,
        SwitchEnd::Fatal,
        SwitchEnd::WriteFailure,
    ];
    let mut got = Vec::new();
    for end in ends {
        assert_eq!(
            request_reload(&mut rig.world, mcp(ReloadTarget::Ghost)),
            ReloadVerdict::Accepted
        );
        let ((), events) = capture(|| on_switch_end(&mut rig.world, end));
        // 予約は呼び手（`end_switch`）が下ろす。
        rig.world.remove_non_send::<SwitchInFlight>();
        let ends = events
            .iter()
            .filter(|e| matches!(e.field_str("event"), Some("reload_done" | "reload_failed")))
            .map(|e| {
                (
                    e.level,
                    e.target.clone(),
                    e.field_str("event").map(str::to_owned),
                    e.field_str("reason").map(str::to_owned),
                    e.field("ghost").map(str::to_owned),
                )
            })
            .collect::<Vec<_>>();
        got.push((ends, rig.world.get_non_send::<ReloadInFlight>().is_none()));
    }
    let ghost = Some(rig.ghost.clone());
    let fail = |reason: &str| {
        (
            vec![(
                Level::ERROR,
                TARGET_ERROR.to_owned(),
                Some("reload_failed".to_owned()),
                Some(reason.to_owned()),
                ghost.clone(),
            )],
            true,
        )
    };
    assert_eq!(
        got,
        vec![
            (
                vec![(
                    Level::INFO,
                    "areka::emo2_boot::reload".to_owned(),
                    Some("reload_done".to_owned()),
                    None,
                    ghost.clone(),
                )],
                true,
            ),
            fail("fell_back_to_default"),
            fail("cancelled"),
            fail("not_accepted"),
            fail("target_quit"),
            fail("fatal"),
            fail("fatal"),
            fail("fatal"),
        ]
    );
}

/// 印があっても他人の予約（鍵が違う・予約が無い）なら述語は偽で、終わりは `reload_failed(overtaken)`。
/// 自分の予約の shiori の印なら実行系だけの組み直しが真・由来は `Reloaded{Shiori}`。印の無い終わりは
/// 何も残さない（要件 3.2・9.3・12.1 ⑹）。
#[test]
fn foreign_reservation_is_false_and_ends_as_overtaken() {
    let mut rig = rig("areka-reload-overtaken");
    let ((), none) = capture(|| on_switch_end(&mut rig.world, SwitchEnd::Cancelled));

    plant_switching(&mut rig.world, ReloadTarget::Shiori, PathBuf::from("A"));
    plant_switch_to(&mut rig.world, PathBuf::from("A"));
    let own = (
        boot_origin_override(&rig.world),
        wants_runtime_only(&rig.world),
    );
    plant_switch_to(&mut rig.world, PathBuf::from("B"));
    let foreign = (
        boot_origin_override(&rig.world),
        wants_runtime_only(&rig.world),
    );
    rig.world.remove_non_send::<SwitchInFlight>();
    let no_switch = (
        boot_origin_override(&rig.world),
        wants_runtime_only(&rig.world),
    );
    plant_switch_to(&mut rig.world, PathBuf::from("B"));
    let ((), events) = capture(|| {
        on_switch_end(
            &mut rig.world,
            SwitchEnd::Steady {
                attempt: WelcomeAttempt::Target,
            },
        )
    });
    let overtaken = events
        .iter()
        .filter(|e| {
            e.field_str("event")
                .is_some_and(|n| n.starts_with("reload_"))
        })
        .map(|e| (e.level, e.field_str("reason").map(str::to_owned)))
        .collect::<Vec<_>>();

    assert_eq!(
        (
            none.len(),
            own,
            foreign,
            no_switch,
            overtaken,
            rig.world.get_non_send::<ReloadInFlight>().is_none(),
        ),
        (
            0,
            (
                Some(BootOrigin::Reloaded {
                    scope: ReloadScope::Shiori
                }),
                true
            ),
            (None, false),
            (None, false),
            vec![(Level::ERROR, Some("overtaken".to_owned()))],
            true,
        )
    );
}

/// shiori の受理を 1 件出し、kanade へ届いた台詞の切れ目の待ち（印・返信端）を取り出す。
fn start_shiori_reload(rig: &mut Rig) -> (Vec<Option<GapRaise>>, ReplySender<TalkGap>) {
    assert_eq!(
        request_reload(&mut rig.world, mcp(ReloadTarget::Shiori)),
        ReloadVerdict::Accepted
    );
    let mut raises = Vec::new();
    let mut reply = None;
    for msg in rig._kanade.try_iter() {
        if let KanadeMsg::AwaitTalkGap { raise, reply: r } = msg {
            raises.push(raise);
            reply = Some(r);
        }
    }
    (
        raises,
        reply.expect("AwaitTalkGap が kanade へ届いていない"),
    )
}

fn awaiting(world: &World) -> bool {
    world
        .get_non_send::<ReloadInFlight>()
        .is_some_and(|f| matches!(f.stage, ReloadStage::AwaitingDestroy { .. }))
}

/// shiori の受理: 台詞の切れ目の待ちに `OnDestroy`（Ref0＝`reload`・NOTIFY）を載せて送り、印は
/// `AwaitingDestroy`（予約はまだ無い・返事が来るまで覗いても変わらない）。`Reached` が届くと `drain` が
/// 今のゴーストの場所で `raise_event: false` の切替を送り、`SwitchInFlight` が立ち、印は自分の予約の
/// `Switching` になる＝実行系だけの組み直しが真・由来は `Reloaded{Shiori}`（要件 2.4・4.1）。
#[test]
fn shiori_reload_raises_on_destroy_and_reached_reserves_the_switch() {
    let mut rig = rig("areka-reload-shiori-start");
    let dir = rig
        .world
        .resource::<BootContext>()
        .current
        .ghost
        .dir
        .clone();
    let (raises, reply) = start_shiori_reload(&mut rig);
    let before = (
        awaiting(&rig.world),
        rig.world.get_non_send::<SwitchInFlight>().is_some(),
    );
    drain(&mut rig.world);
    let pending = awaiting(&rig.world);
    reply
        .send(TalkGap::Reached { marked: None })
        .expect("受け手は印にある");
    let ((), events) = capture(|| drain(&mut rig.world));
    let sent = rig
        ._kanade
        .try_iter()
        .map(|m| match m {
            KanadeMsg::ChangeGhost(c) => Some((c.raise_event, c.target.dir)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let reserved = rig
        .world
        .get_non_send::<ReloadInFlight>()
        .and_then(|f| match &f.stage {
            ReloadStage::Switching { reserved } => Some((f.target.clone(), reserved.clone())),
            _ => None,
        });
    assert_eq!(
        (
            raises,
            before,
            pending,
            sent,
            rig.world
                .get_non_send::<SwitchInFlight>()
                .map(|f| f.target.dir.clone()),
            reserved,
            named(&events, "reload_failed").len(),
            wants_runtime_only(&rig.world),
            boot_origin_override(&rig.world),
        ),
        (
            vec![Some(GapRaise {
                id: "OnDestroy".to_owned(),
                references: vec!["reload".to_owned()],
                method: ShioriMethod::Notify,
            })],
            (true, false),
            true,
            vec![Some((false, rig.ghost.clone()))],
            Some(dir.clone()),
            Some((ReloadTarget::Shiori, dir)),
            0,
            true,
            Some(BootOrigin::Reloaded {
                scope: ReloadScope::Shiori
            }),
        )
    );
}

/// 切れ目の返事が `Reached` でない: 利用者の取りやめ・ほかの切替（`Left`）は `warn!`、許可表の漏れ
/// （`NotSent`）と kanade の脱落（返信端の drop）は `error!` で、どれも `reload_failed(gap_*)` を
/// `TARGET_ERROR`＋ゴースト名で 1 件残して印を下ろし、切替は送らない（要件 4.1・9.1・9.3）。
#[test]
fn shiori_gap_replies_other_than_reached_drop_the_mark() {
    let mut rig = rig("areka-reload-shiori-gap");
    let replies: [Option<TalkGap>; 4] = [
        Some(TalkGap::CancelledByUser),
        Some(TalkGap::Left {
            reason: GapLeft::GhostChange,
        }),
        Some(TalkGap::NotSent {
            outcome: RaiseOutcome::NotAllowed,
        }),
        None,
    ];
    let mut got = Vec::new();
    for answer in replies {
        let (_, reply) = start_shiori_reload(&mut rig);
        match answer {
            Some(gap) => reply.send(gap).expect("受け手は印にある"),
            None => drop(reply),
        }
        let ((), events) = capture(|| drain(&mut rig.world));
        let ends = named(&events, "reload_failed")
            .into_iter()
            .map(|e| {
                (
                    e.level,
                    e.target.clone(),
                    e.field_str("reason").map(str::to_owned),
                    e.field("ghost").map(str::to_owned),
                )
            })
            .collect::<Vec<_>>();
        let switched = rig
            ._kanade
            .try_iter()
            .any(|m| matches!(m, KanadeMsg::ChangeGhost(_)));
        got.push((
            ends,
            rig.world.get_non_send::<ReloadInFlight>().is_none(),
            switched || rig.world.get_non_send::<SwitchInFlight>().is_some(),
        ));
    }
    let ghost = Some(rig.ghost.clone());
    let end = |level: Level, reason: &str| {
        (
            vec![(
                level,
                TARGET_ERROR.to_owned(),
                Some(reason.to_owned()),
                ghost.clone(),
            )],
            true,
            false,
        )
    };
    assert_eq!(
        got,
        vec![
            end(Level::WARN, "gap_cancelled"),
            end(Level::WARN, "gap_left"),
            end(Level::ERROR, "gap_not_sent"),
            end(Level::ERROR, "kanade_gone"),
        ]
    );
}

/// シェルの面の持ち越し（要件 3.8）は、自分の予約の ghost の読み直しのときだけ真。shiori の読み直し・
/// 他人の予約・予約なし・印なしは偽（ふつうの切替と shiori の読み直しは今までどおり）。
#[test]
fn shell_state_is_carried_only_by_its_own_ghost_reload() {
    let mut rig = rig("areka-reload-carry");
    let no_mark = carries_shell_state(&rig.world);

    plant_switching(&mut rig.world, ReloadTarget::Ghost, PathBuf::from("A"));
    plant_switch_to(&mut rig.world, PathBuf::from("A"));
    let own_ghost = carries_shell_state(&rig.world);
    plant_switch_to(&mut rig.world, PathBuf::from("B"));
    let foreign = carries_shell_state(&rig.world);
    rig.world.remove_non_send::<SwitchInFlight>();
    let no_switch = carries_shell_state(&rig.world);

    plant_switching(&mut rig.world, ReloadTarget::Shiori, PathBuf::from("A"));
    plant_switch_to(&mut rig.world, PathBuf::from("A"));
    let own_shiori = carries_shell_state(&rig.world);

    assert_eq!(
        (no_mark, own_ghost, foreign, no_switch, own_shiori),
        (false, true, false, false, false)
    );
}
