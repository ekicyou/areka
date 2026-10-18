//! 読み直しの窓口の判定・断り・始めの記録・終了の門の決定論テスト（areka-P0-mcp-reload 要件 1.1・1.6・
//! 2.1・2.2・7.4・7.5・9.1・9.2・12.1 ⑶）。
//!
//! 置き場のゴーストは kanade の送出端だけを持つテスト用の組み立て（名前が無いので、記録のゴースト名は
//! ルートフォルダの絶対パス）。判定は集めてから 1 回。

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use areka_actor::reply_channel;
use areka_ghost::BasewareRoot;
use areka_kanade::KanadeMsg;
use bevy_ecs::schedule::Schedules;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;

use super::*;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::ghost_switch::{PrevGhost, SwitchStage, SwitchTarget};
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

/// 受理: `reload_started` が対象・出どころ・ゴースト名つきで 1 件。始め方の無い対象は印を置かずに
/// `reload_failed`（not_implemented）を `TARGET_ERROR` で残す（要件 1.6・9.3）。
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
                .map(|e| (e.level, e.target.clone()))
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
            vec![(Level::ERROR, TARGET_ERROR.to_owned())],
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
