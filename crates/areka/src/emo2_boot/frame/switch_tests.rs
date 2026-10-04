//! 差し替えの相の待ちの段の決定論テスト（spec: areka-P0-shell-balloon-switch task 9.3・
//! 要件 1.12・1.14・1.16・5.1・5.4・5.5・5.7・5.8・12.2・12.7・12.9・design「SwitchPhase」）。
//!
//! 切れ目の返事と資産づくりの結果は、テストが持つ送り手から偽の値を流す（kanade も背景の
//! スレッドも起こさない）。置き場のゴーストは kanade の送出端の代わりに観測用の線を持ち、
//! seriko の送り手は観測用の器（`MockSurfaceOutput`）へ合図を出す本物のアクターのもの。
//! 差し替えの依頼の数は、アクターを閉じて join した後に器へ並んだ合図 `Rebased` の数で数える。
//! 判定は集めてから 1 回。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use areka_actor::{ActorHandle, ReplySender, reply_channel};
use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_kanade::{GapLeft, KanadeMsg, MarkedEnd, RaiseOutcome, TalkGap};
use areka_seriko::{
    AnimationTable, BindResolver, DisplayCommand, MockSurfaceOutput, RebaseKind, SerikoLoopConfig,
    SurfaceResolver, spawn_seriko,
};
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::run_switch_phase;
use crate::emo2_boot::assets::{BalloonAssets, BalloonScopeAssets, ScopeAssets, ShellAssets};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::frame::test_support::{headless_wiring_with, zero_clock};
use crate::emo2_boot::ghost_switch::{PrevGhost, SwitchInFlight, SwitchStage, SwitchTarget};
use crate::emo2_boot::shell_balloon_resolve::SkinCandidate;
use crate::emo2_boot::shell_balloon_switch::{SkinKind, SkinSwitchInFlight, SkinSwitchStage};
use crate::emo2_boot::switch_assets::{SwapBuilt, SwapSlot, SwitchBuildError};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::placement::reseed::BalloonPlacementInputs;
use crate::placement::source::{DescriptSource, GhostTitles};

// ---------------------------------------------------------------- 土台

type BuildResult = Result<SwapBuilt, SwitchBuildError>;

struct Rig {
    world: World,
    wiring: Emo2Wiring,
    /// kanade の代わり（置き場のゴーストの送出端の受信端）。
    kanade: Receiver<KanadeMsg>,
    /// 1 度目の切れ目の返事の送り手（落とせば「送り手が落ちた」）。
    gap: Option<ReplySender<TalkGap>>,
    /// 資産づくりの結果の送り手（落とせば「スレッドが倒れた」）。
    build: Option<Sender<BuildResult>>,
    seriko: ActorHandle,
    seriko_out: Arc<Mutex<Vec<DisplayCommand>>>,
    /// 表示の橋渡しの代わりに置き場を強く持つ（結線状態は弱い参照だけを持つ・起動の結線と同じ形）。
    swap_slot: SwapSlot,
}

/// 進行中の印（待ちの段）と、kanade・seriko の代わりを持つ置き場のゴーストをそろえた World。
fn rig(kind: SkinKind) -> Rig {
    let out = MockSurfaceOutput::new();
    let seriko_out = out.records();
    let (sink, seriko) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::default(),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out,
    );
    let (kanade_tx, kanade) = mpsc::channel();
    let mut world = World::new();
    world.insert_non_send(GhostSlot(Some(
        GhostSession::for_test(Some(kanade_tx), PathBuf::from("ghost/A")).with_seriko_sink(sink),
    )));
    let (gap_tx, gap_rx) = reply_channel();
    let (build_tx, build_rx) = mpsc::channel();
    world.insert_non_send(SkinSwitchInFlight {
        kind,
        target: SkinCandidate {
            dir: PathBuf::from("target/B"),
            folder: "B".to_owned(),
            name: None,
            hidden: false,
        },
        stage: SkinSwitchStage::Waiting {
            gap: Some(gap_rx),
            gap_result: None,
            build: build_rx,
            built: None,
        },
    });
    let swap_slot = SwapSlot::default();
    let mut wiring = headless_wiring_with(mpsc::channel().1, zero_clock());
    wiring.swap_slot = Arc::downgrade(&swap_slot);
    Rig {
        world,
        wiring,
        kanade,
        gap: Some(gap_tx),
        build: Some(build_tx),
        seriko,
        seriko_out,
        swap_slot,
    }
}

/// 1 フレームぶん差し替えの相を回し、出た記録を返す。
fn frame(rig: &mut Rig) -> Vec<CapturedEvent> {
    capture(|| run_switch_phase(&mut rig.wiring, &mut rig.world)).1
}

fn reply(rig: &mut Rig, gap: TalkGap) {
    rig.gap.take().unwrap().send(gap).unwrap();
}

fn deliver(rig: &mut Rig, built: BuildResult) {
    rig.build.as_ref().unwrap().send(built).unwrap();
}

fn empty_world() -> EmoWorld {
    EmoWorld::build(&areka_parsers::shell::parse(""))
}

fn empty_atlas() -> AtlasTable {
    AtlasTable::new(Vec::new(), Vec::new(), Vec::new())
}

/// scope 0・1 の新しいシェルの資産（作者の DPI は 120）。
fn shell_built() -> SwapBuilt {
    SwapBuilt::Shell {
        assets: ShellAssets {
            shells: [0, 1]
                .map(|scope| ScopeAssets {
                    scope,
                    emo_world: empty_world(),
                    atlas: empty_atlas(),
                    initial_surface_id: 0,
                })
                .into(),
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds: BindSet::default(),
            bind_resolver: BindResolver::empty(),
            loop_table: AnimationTable::empty(),
            author_dpi: 120,
            bake_failures: Vec::new(),
            boxes: crate::emo2_boot::shell_box_assets::ShellBoxAssets::default(),
        },
        source: DescriptSource {
            ghost_kv: BTreeMap::new(),
            shell_kv: BTreeMap::new(),
            shell_dir: PathBuf::from("shell/B"),
            titles: GhostTitles::from_scope_titles([(0, "s".to_string())]),
        },
        balloon: BalloonPlacementInputs {
            author_dpi: 96,
            windowpositions: BTreeMap::new(),
        },
        restored: Vec::new(),
    }
}

/// scope 0・1 の新しいバルーンの資産（作者の DPI は 144）。
fn balloon_built() -> SwapBuilt {
    SwapBuilt::Balloon {
        assets: BalloonAssets {
            balloons: [0, 1]
                .map(|scope| BalloonScopeAssets {
                    scope,
                    emo_world: empty_world(),
                    atlas: empty_atlas(),
                    model: areka_parsers::balloon::parse_str("", None),
                    background_color: (255, 255, 255),
                    name: String::new(),
                })
                .into(),
            loop_tables: [0, 1].map(|s| (s, AnimationTable::empty())).into(),
            author_dpi: 144,
        },
    }
}

fn build_failure() -> BuildResult {
    Err(SwitchBuildError::ShellUndecodable {
        shell_dir: PathBuf::from("shell/B"),
        failures: vec!["surface0.png".to_owned()],
    })
}

fn plant_ghost_switch(world: &mut World) {
    world.insert_non_send(SwitchInFlight {
        target: SwitchTarget {
            dir: PathBuf::from("X"),
            folder: "X".to_owned(),
            name: "X".to_owned(),
            sakura_name: None,
        },
        prev: PrevGhost {
            dir: PathBuf::from("A"),
            name: None,
            sakura_name: None,
        },
        stage: SwitchStage::SendOff,
        boot_event: None,
    });
}

/// 進行中の印の段（`None`＝印が無い）。
fn stage(world: &World) -> Option<&'static str> {
    world
        .get_non_send::<SkinSwitchInFlight>()
        .map(|m| match m.stage {
            SkinSwitchStage::Waiting { .. } => "waiting",
            SkinSwitchStage::Committed { .. } => "committed",
        })
}

/// kanade へ届いた便り（待ちの依頼は `Some(印)`・それ以外＝汎用の入口などは `None`）と、
/// 待ちの依頼の返信の送り手。
fn kanade_msgs(rig: &Rig) -> (Vec<Option<bool>>, Vec<ReplySender<TalkGap>>) {
    let mut seen = Vec::new();
    let mut replies = Vec::new();
    for msg in rig.kanade.try_iter() {
        match msg {
            KanadeMsg::AwaitTalkGap { raise, reply } => {
                seen.push(Some(raise.is_some()));
                replies.push(reply);
            }
            _ => seen.push(None),
        }
    }
    (seen, replies)
}

/// 置き場の荷物の (世代, scope ごとの (scope, 作者の DPI), 返信の送り手の scope)。
fn slot(rig: &Rig) -> Option<(u64, Vec<(u32, u16)>, Vec<u32>)> {
    rig.swap_slot.lock().unwrap().as_ref().map(|p| {
        (
            p.epoch,
            p.targets.iter().map(|t| (t.0, t.3)).collect(),
            p.replies.iter().map(|r| r.0).collect(),
        )
    })
}

/// 頼んだ段の世代と返信の受け手の scope（頼んだ段でなければ `None`）。
fn committed(world: &World) -> Option<(u64, Vec<u32>, Option<MarkedEnd>)> {
    world
        .get_non_send::<SkinSwitchInFlight>()
        .and_then(|m| match &m.stage {
            SkinSwitchStage::Committed {
                epoch,
                replies,
                marked,
                ..
            } => Some((*epoch, replies.iter().map(|r| r.0).collect(), *marked)),
            SkinSwitchStage::Waiting { .. } => None,
        })
}

/// seriko を閉じて join し、届いた差し替えの依頼（合図 `Rebased` の (世代, 種別)）を返す。
fn replaces(rig: Rig) -> Vec<(u64, RebaseKind)> {
    let Rig {
        world,
        seriko,
        seriko_out,
        ..
    } = rig;
    // 置き場のゴーストが持つ送り手を落とすとアクターの受信端が切れて終わる（閉鎖の便りと同じ）。
    drop(world);
    seriko.join().unwrap();
    seriko_out
        .lock()
        .unwrap()
        .iter()
        .filter_map(|c| match c {
            DisplayCommand::Rebased { epoch, kind, .. } => Some((*epoch, *kind)),
            _ => None,
        })
        .collect()
}

fn count(events: &[CapturedEvent], event: &str, level: Level) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event) && e.level == level)
        .count()
}

/// `skin_switch_*` の記録の (event, 水準, reason, stage)。
fn records(events: &[CapturedEvent]) -> Vec<(String, Level, Option<String>, Option<String>)> {
    events
        .iter()
        .filter_map(|e| {
            let event = e.field_str("event")?;
            event.starts_with("skin_switch_").then(|| {
                (
                    event.to_owned(),
                    e.level,
                    e.field_str("reason").map(str::to_owned),
                    e.field_str("stage").map(str::to_owned),
                )
            })
        })
        .collect()
}

fn rec(
    event: &str,
    level: Level,
    reason: Option<&str>,
    stage: Option<&str>,
) -> Vec<(String, Level, Option<String>, Option<String>)> {
    vec![(
        event.to_owned(),
        level,
        reason.map(str::to_owned),
        stage.map(str::to_owned),
    )]
}

// ---------------------------------------------------------------- 待ちの段の着地

/// 差し替えまで進まない結果: 記録 1 件・印が消える・差し替え（置き場と seriko）・イベント
/// （kanade への便り）・記憶の記録はどれも 0（要件 1.12・1.14・1.16・5.1・5.4・5.5・5.7・12.7）。
#[test]
fn each_non_reached_outcome_drops_with_one_record_and_nothing_else() {
    type Poke = fn(&mut Rig);
    let cases: Vec<(
        &str,
        Poke,
        Vec<(String, Level, Option<String>, Option<String>)>,
    )> = vec![
        (
            "利用者の中断",
            |r| reply(r, TalkGap::CancelledByUser),
            rec("skin_switch_cancelled", Level::INFO, None, None),
        ),
        (
            "定常でない",
            |r| {
                reply(
                    r,
                    TalkGap::Left {
                        reason: GapLeft::NotSteady,
                    },
                )
            },
            rec("skin_switch_not_steady", Level::WARN, None, None),
        ),
        (
            "終了",
            |r| {
                reply(
                    r,
                    TalkGap::Left {
                        reason: GapLeft::Closing,
                    },
                )
            },
            rec("skin_switch_dropped", Level::INFO, Some("closing"), None),
        ),
        (
            "ゴースト切替",
            |r| {
                reply(
                    r,
                    TalkGap::Left {
                        reason: GapLeft::GhostChange,
                    },
                )
            },
            rec(
                "skin_switch_dropped",
                Level::INFO,
                Some("ghost_change"),
                None,
            ),
        ),
        (
            "送らなかった",
            |r| {
                reply(
                    r,
                    TalkGap::NotSent {
                        outcome: RaiseOutcome::NotAllowed,
                    },
                )
            },
            rec("skin_switch_not_sent", Level::WARN, None, None),
        ),
        (
            "送り手が落ちた",
            |r| drop(r.gap.take()),
            rec(
                "skin_switch_dropped",
                Level::INFO,
                Some("kanade_stopped"),
                None,
            ),
        ),
        (
            "資産の失敗",
            |r| deliver(r, build_failure()),
            vec![(
                "skin_switch_failed".to_owned(),
                Level::ERROR,
                build_failure().err().map(|e| e.to_string()),
                Some("build".to_owned()),
            )],
        ),
        (
            "資産のスレッドが倒れた",
            |r| drop(r.build.take()),
            rec(
                "skin_switch_failed",
                Level::ERROR,
                Some("worker_gone"),
                Some("build"),
            ),
        ),
        (
            "資産がそろった後の終了",
            |r| {
                deliver(r, Ok(balloon_built()));
                reply(
                    r,
                    TalkGap::Left {
                        reason: GapLeft::Closing,
                    },
                )
            },
            rec("skin_switch_dropped", Level::INFO, Some("closing"), None),
        ),
    ];
    let mut got = Vec::new();
    let mut want = Vec::new();
    for (label, poke, record) in cases {
        let mut rig = rig(SkinKind::Balloon);
        poke(&mut rig);
        let events = frame(&mut rig);
        let after = stage(&rig.world);
        let (kanade, _) = kanade_msgs(&rig);
        let placed = slot(&rig);
        let memory = events
            .iter()
            .filter(|e| {
                e.field_str("event")
                    .is_some_and(|ev| ev.starts_with("last_"))
            })
            .count();
        got.push((
            label,
            records(&events),
            after,
            kanade,
            placed,
            memory,
            replaces(rig),
        ));
        want.push((label, record, None, Vec::new(), None, 0, Vec::new()));
    }
    assert_eq!(
        got, want,
        "(場面, skin_switch_* の記録, 印, kanade への便り, 置き場, 記憶の記録, seriko への差し替え)"
    );
}

/// どちらもまだ届かないフレームは何もしない（待ち続ける・記録 0）。
#[test]
fn nothing_arrived_keeps_waiting_silently() {
    let mut rig = rig(SkinKind::Shell);
    let events = frame(&mut rig);
    let (kanade, _) = kanade_msgs(&rig);
    let after = (records(&events), stage(&rig.world), kanade, slot(&rig));
    assert_eq!(
        (after, replaces(rig)),
        ((Vec::new(), Some("waiting"), Vec::new(), None), Vec::new()),
        "((記録, 印, 便り, 置き場), 差し替え)"
    );
}

// ---------------------------------------------------------------- 頼む

/// 資産がそろった後に `Reached` を受けると、世代を進めて荷物を置き場へ置き、返信の受け手を
/// 控え、seriko へ差し替えを 1 件頼む。荷物は scope ごとに作者の DPI を運び、イベント（kanade
/// への便り）・警告と失敗の記録は 0（要件 12.2・12.7）。
#[test]
fn reached_after_assets_requests_one_replace_per_kind() {
    let mut got = Vec::new();
    let mut want = Vec::new();
    for (kind, built, dpi, rebase) in [
        (SkinKind::Shell, shell_built(), 120, RebaseKind::Shell),
        (SkinKind::Balloon, balloon_built(), 144, RebaseKind::Balloon),
    ] {
        let mut rig = rig(kind);
        deliver(&mut rig, Ok(built));
        let first = frame(&mut rig);
        let waiting = stage(&rig.world);
        reply(
            &mut rig,
            TalkGap::Reached {
                marked: Some(MarkedEnd::Completed),
            },
        );
        let second = frame(&mut rig);
        let (kanade, _) = kanade_msgs(&rig);
        let placed = slot(&rig);
        let held = committed(&rig.world);
        let epoch = placed.as_ref().map_or(0, |p| p.0);
        let warned = count(&second, "skin_switch_failed", Level::ERROR)
            + second.iter().filter(|e| e.level == Level::WARN).count();
        got.push((
            records(&first),
            waiting,
            warned,
            kanade,
            placed,
            held,
            replaces(rig),
        ));
        want.push((
            Vec::new(),
            Some("waiting"),
            0,
            Vec::new(),
            Some((epoch, vec![(0, dpi), (1, dpi)], vec![0, 1])),
            Some((epoch, vec![0, 1], Some(MarkedEnd::Completed))),
            vec![(epoch, rebase)],
        ));
    }
    assert_ne!(
        got[0].4.as_ref().map(|p| p.0),
        got[1].4.as_ref().map(|p| p.0),
        "世代は進む"
    );
    assert_eq!(
        got, want,
        "(1 フレーム目の記録, 1 フレーム目の印, 警告と失敗, 便り, 置き場, 頼んだ段, seriko への差し替え)"
    );
}

/// 資産と `Reached` が同じフレームに届いても、資産を先に見るので頼む（送り直しは無い）。
#[test]
fn assets_and_reached_in_one_frame_commit_without_recheck() {
    let mut rig = rig(SkinKind::Balloon);
    deliver(&mut rig, Ok(balloon_built()));
    reply(&mut rig, TalkGap::Reached { marked: None });
    let events = frame(&mut rig);
    let (kanade, _) = kanade_msgs(&rig);
    let got = (
        count(&events, "skin_switch_gap_recheck", Level::DEBUG),
        kanade,
        stage(&rig.world),
    );
    let sent = replaces(rig).len();
    assert_eq!(
        (got, sent),
        ((0, Vec::new(), Some("committed")), 1),
        "((送り直しの記録, 便り, 印), 差し替え)"
    );
}

/// 資産がそろって `Reached` を受けても、ゴースト切替が進んでいれば頼まずに取りやめる
/// （ゴースト切替が勝つ・要件 1.12・12.9）。
#[test]
fn ghost_switch_in_flight_wins_before_the_replace() {
    let mut rig = rig(SkinKind::Shell);
    deliver(&mut rig, Ok(shell_built()));
    frame(&mut rig);
    plant_ghost_switch(&mut rig.world);
    reply(&mut rig, TalkGap::Reached { marked: None });
    let events = frame(&mut rig);
    let (kanade, _) = kanade_msgs(&rig);
    let got = (records(&events), stage(&rig.world), kanade, slot(&rig));
    assert_eq!(
        (got, replaces(rig)),
        (
            (
                rec(
                    "skin_switch_dropped",
                    Level::INFO,
                    Some("ghost_switch"),
                    None
                ),
                None,
                Vec::new(),
                None
            ),
            Vec::new()
        ),
        "((記録, 印, 便り, 置き場), 差し替え)"
    );
}

/// 表示の橋渡しが消えていて置き場を引き上げられなければ、`error!(stage=seriko, reason=bridge_gone)`
/// 1 件で印を消す（荷物は置かずに捨てる・seriko への差し替え 0）。
#[test]
fn bridge_gone_at_commit_drops_with_one_error() {
    let mut rig = rig(SkinKind::Shell);
    rig.wiring.swap_slot = Weak::new();
    deliver(&mut rig, Ok(shell_built()));
    reply(&mut rig, TalkGap::Reached { marked: None });
    let events = frame(&mut rig);
    let got = (records(&events), stage(&rig.world), slot(&rig));
    assert_eq!(
        (got, replaces(rig)),
        (
            (
                rec(
                    "skin_switch_failed",
                    Level::ERROR,
                    Some("bridge_gone"),
                    Some("seriko")
                ),
                None,
                None
            ),
            Vec::new()
        ),
        "((記録, 印, 置き場), 差し替え)"
    );
}

/// seriko へ送れなければ `error!(stage=seriko)` 1 件で置き場を空にして印を消す。
#[test]
fn seriko_send_failure_empties_the_slot() {
    let mut rig = rig(SkinKind::Balloon);
    // 置き場のゴーストの seriko を先に止める（送り手は残るが受信端が消える）。
    rig.world
        .get_non_send::<GhostSlot>()
        .and_then(|s| s.0.as_ref())
        .and_then(GhostSession::seriko_sink)
        .unwrap()
        .close()
        .unwrap();
    deliver(&mut rig, Ok(balloon_built()));
    frame(&mut rig);
    // 閉鎖の便りが処理されて受信端が消えるまで待つ（アクターの出口は 1 か所）。
    let deadline = Instant::now() + Duration::from_secs(5);
    while !rig.seriko.is_finished() {
        assert!(
            Instant::now() < deadline,
            "seriko が閉鎖の便りを 5 秒以内に処理して終わらなかった"
        );
        std::thread::yield_now();
    }
    reply(&mut rig, TalkGap::Reached { marked: None });
    let events = frame(&mut rig);
    let got = (records(&events), stage(&rig.world), slot(&rig));
    assert_eq!(
        (got, replaces(rig)),
        (
            (
                rec("skin_switch_failed", Level::ERROR, None, Some("seriko")),
                None,
                None
            ),
            Vec::new()
        ),
        "((記録, 印, 置き場), 差し替え)"
    );
}

// ---------------------------------------------------------------- 切れ目が先に届いた

/// `Reached` が資産より先に届くと、資産がそろったフレームで印なしの待ちを 1 件送り直し
/// （`debug!` 1 件）、その返事が届くまで差し替えは 0 件。返事が終了なら取りやめ、`Reached` なら
/// 差し替えを 1 件頼む（印の台詞の終わり方は 1 度目の返事のもの・design Flow 1 ⑵）。
#[test]
fn reached_before_assets_rechecks_once_and_follows_the_second_reply() {
    let mut got = Vec::new();
    let mut want = Vec::new();
    for (second, record, end) in [
        (
            TalkGap::Left {
                reason: GapLeft::Closing,
            },
            rec("skin_switch_dropped", Level::INFO, Some("closing"), None),
            None,
        ),
        (
            TalkGap::Reached { marked: None },
            Vec::new(),
            Some("committed"),
        ),
    ] {
        let mut rig = rig(SkinKind::Balloon);
        reply(
            &mut rig,
            TalkGap::Reached {
                marked: Some(MarkedEnd::Replaced),
            },
        );
        let early = frame(&mut rig);
        let early_seen = (records(&early), stage(&rig.world), kanade_msgs(&rig).0);
        deliver(&mut rig, Ok(balloon_built()));
        let ready = frame(&mut rig);
        let (resent, mut waits) = kanade_msgs(&rig);
        let ready_seen = (records(&ready), resent, slot(&rig));
        // 送り直しの返事より前に、もう 1 フレーム回しても差し替えは頼まない。
        let idle = frame(&mut rig);
        let idle_seen = (records(&idle), slot(&rig), stage(&rig.world));
        waits.pop().unwrap().send(second.clone()).unwrap();
        let last = frame(&mut rig);
        let marked = committed(&rig.world).and_then(|c| c.2);
        let last_seen = (records(&last), stage(&rig.world), marked);
        let sent = replaces(rig).len();
        got.push((early_seen, ready_seen, idle_seen, last_seen, sent));
        want.push((
            (Vec::new(), Some("waiting"), Vec::new()),
            (
                rec("skin_switch_gap_recheck", Level::DEBUG, None, None),
                vec![Some(false)],
                None,
            ),
            (Vec::new(), None, Some("waiting")),
            (record, end, end.map(|_| MarkedEnd::Replaced)),
            usize::from(end.is_some()),
        ));
    }
    assert_eq!(
        got, want,
        "(先に届いたフレーム (記録, 印, 便り), 資産のフレーム (記録, 便り, 置き場), \
         返事待ち (記録, 置き場, 印), 返事のフレーム (記録, 印, 印の台詞の終わり方), 差し替え)"
    );
}

/// 送り直しを送れなければ `error!(skin_switch_send_failed)` 1 件で印を消す。
#[test]
fn recheck_send_failure_drops_with_one_error() {
    let mut rig = rig(SkinKind::Balloon);
    reply(&mut rig, TalkGap::Reached { marked: None });
    frame(&mut rig);
    let (_, dead) = mpsc::channel();
    drop(std::mem::replace(&mut rig.kanade, dead));
    deliver(&mut rig, Ok(balloon_built()));
    let events = frame(&mut rig);
    let got = (records(&events), stage(&rig.world), slot(&rig));
    assert_eq!(
        (got, replaces(rig)),
        (
            (
                rec("skin_switch_send_failed", Level::ERROR, None, None),
                None,
                None
            ),
            Vec::new()
        ),
        "((記録, 印, 置き場), 差し替え)"
    );
}
