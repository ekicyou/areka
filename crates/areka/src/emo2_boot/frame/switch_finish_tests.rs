//! 差し替えの相の完了の段と後始末の決定論テスト（spec: areka-P0-shell-balloon-switch task 9.4・
//! 要件 2.4・2.7・3.2・3.3・3.4・4.4・6.1・6.5・6.7・design「SwitchPhase」）。
//!
//! 置き換えの返信は、テストが持つ送り手から偽の値を流す（drain も GPU も回さない）。置き場の
//! ゴーストは kanade の送出端の代わりに観測用の線と、偽の置き場（`FakePersistIo`）へ書く本物の
//! 記憶の書き手を持つ。実行系と窓は無いので、実行系の今のシェルの書き換えと配置の入れ直しは
//! 縮退の記録（`skin_shell_dir_not_set`・`reseed_skipped`）になる（実物は統合テスト
//! `shell_balloon_switch_session_tests.rs` が偽の SHIORI で突き合わせる）。判定は集めてから 1 回。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use areka_actor::{ReplySender, reply_channel};
use areka_emo_present::{PresentError, PresentOutcome, TargetId};
use areka_ghost::BasewareRoot;
use areka_kanade::{KanadeMsg, ShioriMethod};
use areka_sylphya::persist::{FakePersistIo, PersistIo};
use areka_sylphya::{
    PersistKey, PersistScope, ScopeRoots, SylphyaInit, SylphyaParts, load_scope, save_scope,
    spawn_sylphya,
};
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::run_switch_phase;
use crate::boot_config::{BootContext, ConfigInputs, CurrentGhost};
use crate::boot_resolve::{BalloonDecision, BalloonRoute, GhostDecision, GhostRoute};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::frame::test_support::{headless_wiring_with, zero_clock};
use crate::emo2_boot::shell_balloon_resolve::SkinCandidate;
use crate::emo2_boot::shell_balloon_switch::{
    SkinKind, SkinSwitchInFlight, SkinSwitchStage, SwapFinish,
};
use crate::emo2_boot::switch_assets::{SwapPayload, SwapSlot};
use crate::ghost_session::{GhostSession, GhostSlot};
use crate::placement::reseed::BalloonPlacementInputs;
use crate::placement::reseed::tests as reseed_tests;
use crate::placement::source::{DescriptSource, GhostTitles};
use crate::placement::zorder_group_ledger::GroupSource;

// ---------------------------------------------------------------- 土台

/// 共有の偽の置き場（アクターへ移しつつ、同じ置き場を別の口の `load_scope` で読む）。
struct SharedFakeIo(Arc<FakePersistIo>);
impl PersistIo for SharedFakeIo {
    fn read(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.0.read(path)
    }
    fn commit(&self, path: &Path, content: &str) -> std::io::Result<()> {
        self.0.commit(path, content)
    }
}

struct Rig {
    world: World,
    wiring: Emo2Wiring,
    /// kanade の代わり（置き場のゴーストの送出端の受信端）。
    kanade: Receiver<KanadeMsg>,
    /// scope ごとの置き換えの返信の送り手（drain の代わり）。
    replies: Vec<(u32, Option<ReplySender<PresentOutcome>>)>,
    store: Arc<FakePersistIo>,
    roots: ScopeRoots,
    /// 記憶の書き手（`None`＝書き手が無いゴースト）。
    sylphya: Option<SylphyaParts>,
}

fn entry(key: PersistKey, value: &str) -> (PersistKey, String) {
    (key, value.to_owned())
}

/// 前から在る Ghost スコープの記憶（どちらの鍵も古い値）。
fn old_memory() -> Vec<(PersistKey, String)> {
    vec![
        entry(PersistKey::LastBalloon, "old-balloon"),
        entry(PersistKey::LastShell, "old-shell"),
    ]
}

fn shell_target() -> SkinCandidate {
    SkinCandidate {
        dir: PathBuf::from("skins/ghost/A/shell/summer"),
        folder: "summer".to_owned(),
        name: Some("夏服".to_owned()),
        hidden: false,
    }
}

fn balloon_target() -> SkinCandidate {
    SkinCandidate {
        dir: PathBuf::from("skins/balloon/fluffy"),
        folder: "fluffy".to_owned(),
        name: Some("ふわふわ".to_owned()),
        hidden: false,
    }
}

fn abs(dir: &Path) -> String {
    std::path::absolute(dir).unwrap().display().to_string()
}

/// 新しいシェルの後始末の残り（`seriko.zorder` は `1,0`）。
fn shell_finish() -> SwapFinish {
    SwapFinish::Shell {
        source: DescriptSource {
            ghost_kv: BTreeMap::new(),
            shell_kv: [("seriko.zorder".to_owned(), "1,0".to_owned())].into(),
            shell_dir: shell_target().dir,
            titles: GhostTitles::from_scope_titles([(0, "s".to_string())]),
        },
        balloon: BalloonPlacementInputs {
            author_dpi: 96,
            windowpositions: BTreeMap::new(),
        },
        restored: Vec::new(),
    }
}

/// 新しいバルーンの後始末の残り（scope 0・1 の文字の模型と背景色）。
fn balloon_finish() -> SwapFinish {
    SwapFinish::Balloon {
        scopes: [0, 1]
            .map(|scope| {
                (
                    scope,
                    areka_parsers::balloon::parse_str("", None),
                    (10, 20, 30),
                )
            })
            .into(),
    }
}

/// 起動の文脈（今のバルーンは argv で決まった `kaku`）。
fn boot_context() -> BootContext {
    let root = BasewareRoot::new(PathBuf::from("skins"));
    BootContext {
        root: root.clone(),
        app_profile_dir: PathBuf::from("skins/profile"),
        helper_exe: PathBuf::from("skins/helper.exe"),
        argv_session: true,
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
                route: BalloonRoute::Argv,
                dir: root.balloon_dir("kaku"),
                folder: None,
            },
        },
    }
}

/// 頼んだ段（scope 0・1 の返信待ち）の印と、kanade・記憶の書き手の代わりを持つ置き場のゴースト。
fn rig(kind: SkinKind, with_memory: bool) -> Rig {
    let finish = match kind {
        SkinKind::Shell => shell_finish(),
        SkinKind::Balloon => balloon_finish(),
    };
    rig_on(World::new(), kind, with_memory, finish)
}

/// [`rig`] の土台を `world`（窓を組んだ World など）の上に、後始末の残り `finish` で組む。
fn rig_on(mut world: World, kind: SkinKind, with_memory: bool, finish: SwapFinish) -> Rig {
    let store = Arc::new(FakePersistIo::new());
    let roots = ScopeRoots {
        app: Some(PathBuf::from("/app")),
        ghost: Some(PathBuf::from("/g")),
        ..ScopeRoots::default()
    };
    save_scope(
        PersistScope::Ghost,
        &roots,
        &SharedFakeIo(store.clone()),
        old_memory(),
    );
    let sylphya = with_memory.then(|| {
        spawn_sylphya(SylphyaInit {
            roots: roots.clone(),
            io: Box::new(SharedFakeIo(store.clone())),
            runtime_sink: None,
        })
    });
    let (kanade_tx, kanade) = mpsc::channel();
    let mut session = GhostSession::for_test(Some(kanade_tx), PathBuf::from("skins/ghost/A"));
    if let Some(parts) = &sylphya {
        session = session.with_memory_publisher(parts.publisher.clone());
    }
    world.insert_non_send(GhostSlot(Some(session)));
    world.insert_resource(boot_context());

    let target = match kind {
        SkinKind::Shell => shell_target(),
        SkinKind::Balloon => balloon_target(),
    };
    let mut replies = Vec::new();
    let mut receivers = Vec::new();
    for scope in [0, 1] {
        let (tx, rx) = reply_channel::<PresentOutcome>();
        replies.push((scope, Some(tx)));
        receivers.push((scope, rx));
    }
    world.insert_non_send(SkinSwitchInFlight {
        kind,
        target,
        stage: SkinSwitchStage::Committed {
            epoch: 7,
            replies: receivers,
            finish,
            marked: None,
            committed_at: Instant::now(),
        },
    });
    Rig {
        world,
        wiring: headless_wiring_with(mpsc::channel().1, zero_clock()),
        kanade,
        replies,
        store,
        roots,
        sylphya,
    }
}

/// 1 フレームぶん差し替えの相を回し、出た記録を返す。
fn frame(rig: &mut Rig) -> Vec<CapturedEvent> {
    capture(|| run_switch_phase(&mut rig.wiring, &mut rig.world)).1
}

/// scope の置き換えの返信を送る（drain が `ReplaceTarget` を適用した結果の代わり）。
fn answer(rig: &mut Rig, scope: u32, outcome: PresentOutcome) {
    let slot = rig.replies.iter_mut().find(|r| r.0 == scope).unwrap();
    slot.1.take().unwrap().send(outcome).unwrap();
}

/// scope の置き換えの返信の送り手を落とす（drain が返さずに捨てた形）。
fn drop_reply(rig: &mut Rig, scope: u32) {
    let slot = rig.replies.iter_mut().find(|r| r.0 == scope).unwrap();
    drop(slot.1.take());
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

/// kanade へ届いた汎用の入口の便り (id, Reference, GET か, 返信端なしか)。それ以外は印だけ。
fn raised(rig: &Rig) -> Vec<(String, Vec<String>, bool, bool)> {
    rig.kanade
        .try_iter()
        .map(|msg| match msg {
            KanadeMsg::RaiseEvent {
                id,
                references,
                method,
                reply,
            } => (id, references, method == ShioriMethod::Get, reply.is_none()),
            _ => (
                "（汎用の入口でない便り）".to_owned(),
                Vec::new(),
                false,
                false,
            ),
        })
        .collect()
}

/// 記憶の書き手を閉じ、Ghost スコープの記憶を読み戻す（書き手が無ければ置き場をそのまま読む）。
fn memory(rig: Rig) -> Vec<(PersistKey, String)> {
    if let Some(parts) = rig.sylphya {
        drop(rig.world);
        parts.publisher.barrier().expect("barrier");
        parts.publisher.close();
        let _ = parts.handle.join();
    }
    load_scope(
        PersistScope::Ghost,
        &rig.roots,
        &SharedFakeIo(rig.store.clone()),
    )
}

fn count(events: &[CapturedEvent], event: &str, level: Level) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event) && e.level == level)
        .count()
}

/// `skin_*`・`last_*` の記録の (event, 水準, stage)。
fn records(events: &[CapturedEvent]) -> Vec<(String, Level, Option<String>)> {
    events
        .iter()
        .filter_map(|e| {
            let event = e.field_str("event")?;
            (event.starts_with("skin_") || event.starts_with("last_")).then(|| {
                (
                    event.to_owned(),
                    e.level,
                    e.field_str("stage").map(str::to_owned),
                )
            })
        })
        .collect()
}

fn rec(event: &str, level: Level, stage: Option<&str>) -> (String, Level, Option<String>) {
    (event.to_owned(), level, stage.map(str::to_owned))
}

fn attach_failure() -> PresentOutcome {
    Err(PresentError::TargetNotAttached(TargetId(2)))
}

// ---------------------------------------------------------------- 失敗と待ち

/// 置き換えの返信のどれかが失敗か送り手の脱落なら `error!(skin_switch_failed, stage=attach)` 1 件で
/// 印を消し、記憶・通知（kanade への便り）は 0（要件 6.1 の「差し替えが済む」が成り立たない）。
#[test]
fn reply_failure_records_one_error_and_writes_nothing() {
    type Poke = fn(&mut Rig);
    let pokes: [(&str, Poke); 2] = [
        ("失敗の返信", |r| {
            answer(r, 0, Ok(()));
            answer(r, 1, attach_failure());
        }),
        ("送り手が落ちた", |r| {
            drop_reply(r, 0);
            answer(r, 1, Ok(()));
        }),
    ];
    let mut got = Vec::new();
    let mut want = Vec::new();
    for kind in [SkinKind::Shell, SkinKind::Balloon] {
        for (label, poke) in pokes {
            let mut rig = rig(kind, true);
            poke(&mut rig);
            let events = frame(&mut rig);
            let seen = (records(&events), stage(&rig.world), raised(&rig));
            got.push((kind, label, seen, memory(rig)));
            want.push((
                kind,
                label,
                (
                    vec![rec("skin_switch_failed", Level::ERROR, Some("attach"))],
                    None,
                    Vec::new(),
                ),
                old_memory(),
            ));
        }
    }
    assert_eq!(
        got, want,
        "(種別, 場面, (skin_*／last_* の記録, 印, kanade への便り), 記憶)"
    );
}

/// 表示の橋渡しの代わりに置き場を持ち、起動の結線（`wire_emo2_boot`）と同じ形で結線状態へ渡す。
/// 返り値を落とすと、seriko のアクターと一緒に橋渡しが消えた形になる。
fn wire_bridge_slot(wiring: &mut Emo2Wiring) -> SwapSlot {
    let bridge = SwapSlot::default();
    wiring.swap_slot = Arc::downgrade(&bridge);
    bridge
}

/// 頼んだ段で seriko（表示の橋渡し）が合図を出す前に倒れたら、荷物と返信の送り手は橋渡しと一緒に
/// 消え、`error!(skin_switch_failed, stage=attach)` 1 件で印を消す（記憶・通知 0）。結線状態が
/// 荷物を生かし続けると印が残り、以後の切替がずっと断られる（design「SwitchPhase」）。
#[test]
fn bridge_gone_while_committed_fails_attach_and_clears_the_marker() {
    let mut got = Vec::new();
    let mut want = Vec::new();
    for kind in [SkinKind::Shell, SkinKind::Balloon] {
        let mut rig = rig(kind, true);
        let bridge = wire_bridge_slot(&mut rig.wiring);
        let replies = rig
            .replies
            .iter_mut()
            .map(|(scope, tx)| (*scope, tx.take().unwrap()))
            .collect();
        *bridge.lock().unwrap() = Some(SwapPayload {
            epoch: 7,
            targets: Vec::new(),
            replies,
        });
        drop(bridge);
        let events = frame(&mut rig);
        let seen = (records(&events), stage(&rig.world), raised(&rig));
        got.push((kind, seen, memory(rig)));
        want.push((
            kind,
            (
                vec![rec("skin_switch_failed", Level::ERROR, Some("attach"))],
                None,
                Vec::new(),
            ),
            old_memory(),
        ));
    }
    assert_eq!(
        got, want,
        "(種別, (skin_*／last_* の記録, 印, kanade への便り), 記憶)"
    );
}

/// 返信が一部しか届かないフレームは待ち続ける（記録・便り 0）。残りが届いたフレームで後始末する。
#[test]
fn partial_replies_keep_waiting_until_all_arrive() {
    let mut rig = rig(SkinKind::Balloon, true);
    answer(&mut rig, 0, Ok(()));
    let first = frame(&mut rig);
    let first_seen = (records(&first), stage(&rig.world), raised(&rig).len());
    answer(&mut rig, 1, Ok(()));
    let second = frame(&mut rig);
    let second_seen = (
        count(&second, "skin_switch_done", Level::INFO),
        stage(&rig.world),
        raised(&rig).len(),
    );
    assert_eq!(
        (first_seen, second_seen),
        ((Vec::new(), Some("committed"), 0), (1, None, 1)),
        "((記録, 印, 便り), (完了の記録, 印, 便り))"
    );
}

// ---------------------------------------------------------------- 後始末

/// シェル: `OnShellChanged`（Ref0＝新しいシェルの名前・Ref1＝ゴーストの名前・Ref2＝新しいシェルの
/// 絶対パス）を GET・返信なしで 1 件、`LastShell` だけを書き（`LastBalloon` は不変）、重なりの基底を
/// 新しいシェルの `seriko.zorder` へ置き直し、`info!(skin_switch_done)` 1 件で印を消す。今の
/// バルーンは触らない（要件 2.4・2.7・6.1・6.5）。
#[test]
fn shell_success_raises_on_shell_changed_and_records_only_last_shell() {
    let mut rig = rig(SkinKind::Shell, true);
    answer(&mut rig, 0, Ok(()));
    answer(&mut rig, 1, Ok(()));
    let events = frame(&mut rig);
    let base: Vec<GroupSource> = rig
        .wiring
        .zorder_ledger
        .groups()
        .iter()
        .map(|g| g.source)
        .collect();
    let balloon = rig.world.resource::<BootContext>().current.balloon.clone();
    let seen = (
        raised(&rig),
        count(&events, "skin_switch_done", Level::INFO),
        count(&events, "last_shell_recorded", Level::INFO),
        count(&events, "skin_switch_failed", Level::ERROR),
        stage(&rig.world),
        base,
        balloon,
    );
    assert_eq!(
        (seen, memory(rig)),
        (
            (
                vec![(
                    "OnShellChanged".to_owned(),
                    vec!["夏服".to_owned(), "A".to_owned(), abs(&shell_target().dir)],
                    true,
                    true,
                )],
                1,
                1,
                0,
                None,
                vec![GroupSource::Descript],
                boot_context().current.balloon,
            ),
            vec![
                entry(PersistKey::LastBalloon, "old-balloon"),
                entry(PersistKey::LastShell, "summer"),
            ],
        ),
        "((便り, 完了, LastShell の記録, 失敗, 印, 重なりの基底, 今のバルーン), 記憶)"
    );
}

/// バルーン: `OnBalloonChange`（Ref0＝名前・Ref1＝絶対パス）を GET・返信なしで 1 件、`LastBalloon`
/// だけを書き（argv で起きたプロセスでも・`LastShell` は不変）、今のバルーンを記憶の経路の新しい
/// バルーンにし、scope ごとの文字の模型を替えて `info!(skin_switch_done)` 1 件で印を消す
/// （要件 3.2・3.3・3.4・6.1・6.7）。
#[test]
fn balloon_success_raises_on_balloon_change_and_records_only_last_balloon() {
    let mut rig = rig(SkinKind::Balloon, true);
    answer(&mut rig, 0, Ok(()));
    answer(&mut rig, 1, Ok(()));
    let events = frame(&mut rig);
    let seen = (
        raised(&rig),
        count(&events, "skin_switch_done", Level::INFO),
        count(&events, "last_balloon_recorded", Level::INFO),
        count(&events, "skin_switch_failed", Level::ERROR),
        stage(&rig.world),
        rig.wiring.balloon_model_scopes(),
        rig.world.resource::<BootContext>().current.balloon.clone(),
    );
    assert_eq!(
        (seen, memory(rig)),
        (
            (
                vec![(
                    "OnBalloonChange".to_owned(),
                    vec!["ふわふわ".to_owned(), abs(&balloon_target().dir)],
                    true,
                    true,
                )],
                1,
                1,
                0,
                None,
                vec![0, 1],
                BalloonDecision {
                    route: BalloonRoute::Memory,
                    dir: balloon_target().dir,
                    folder: Some("fluffy".to_owned()),
                },
            ),
            vec![
                entry(PersistKey::LastBalloon, "fluffy"),
                entry(PersistKey::LastShell, "old-shell"),
            ],
        ),
        "((便り, 完了, LastBalloon の記録, 失敗, 印, 文字の模型の scope, 今のバルーン), 記憶)"
    );
}

/// 記憶の書き手が無ければ `warn!(skin_memory_not_recorded)` 1 件で、切替は成功として扱う
/// （通知 1 件・完了 1 件・印を消す・要件 6.1）。
#[test]
fn missing_memory_writer_warns_and_still_succeeds() {
    let mut got = Vec::new();
    for kind in [SkinKind::Shell, SkinKind::Balloon] {
        let mut rig = rig(kind, false);
        answer(&mut rig, 0, Ok(()));
        answer(&mut rig, 1, Ok(()));
        let events = frame(&mut rig);
        got.push((
            count(&events, "skin_memory_not_recorded", Level::WARN),
            count(&events, "skin_switch_done", Level::INFO),
            raised(&rig).len(),
            stage(&rig.world),
            memory(rig),
        ));
    }
    assert_eq!(
        got,
        vec![(1, 1, 1, None, old_memory()); 2],
        "(記憶なしの警告, 完了, 便り, 印, 記憶)（シェル・バルーン）"
    );
}

/// `swap_ms` は、返信のそろったフレームの drain の時間（置き換えの適用を含む上限）と後始末の
/// 時間の和（要件 4.4）。drain の時間を 50 ms と置けば、記録は 50 ms 以上になる。
#[test]
fn swap_ms_includes_the_drain_of_the_completing_frame() {
    let mut got = Vec::new();
    for kind in [SkinKind::Shell, SkinKind::Balloon] {
        let mut rig = rig(kind, false);
        rig.wiring.last_drain = Duration::from_millis(50);
        answer(&mut rig, 0, Ok(()));
        answer(&mut rig, 1, Ok(()));
        let events = frame(&mut rig);
        let swap_ms = events
            .iter()
            .find(|e| e.field_str("event") == Some("skin_switch_done"))
            .and_then(|e| e.field("swap_ms"))
            .and_then(|v| v.parse::<f64>().ok());
        got.push((kind, swap_ms.is_some_and(|ms| ms >= 50.0), swap_ms));
    }
    assert!(
        got.iter().all(|g| g.1),
        "(種別, drain の時間を含むか, swap_ms): {got:?}"
    );
}

/// 揃え方は新しいシェルから読み直す（要件 2.7）: 起動のシェルが下端揃え（bottom）で、新しい
/// シェルが上端揃え（top）なら、後始末の後のキャラ窓は今の窓寸のまま作業領域の上端へ揃う
/// （窓寸が変わらないフレームでも、後段の再スナップを待たずに置き直す）。
#[test]
fn shell_finish_moves_char_windows_to_the_new_alignment() {
    let (world, windows) = reseed_tests::booted_world(96);
    let before = reseed_tests::char_positions(&world, &windows);
    let finish = SwapFinish::Shell {
        source: reseed_tests::new_shell(&[("seriko.alignmenttodesktop", "top")]),
        balloon: BalloonPlacementInputs {
            author_dpi: 96,
            windowpositions: BTreeMap::new(),
        },
        restored: Vec::new(),
    };
    let mut rig = rig_on(world, SkinKind::Shell, false, finish);
    answer(&mut rig, 0, Ok(()));
    answer(&mut rig, 1, Ok(()));
    frame(&mut rig);
    let after = reseed_tests::char_positions(&rig.world, &windows);
    let tops: Vec<i32> = after.iter().map(|p| p.y).collect();
    let xs = (
        before.iter().map(|p| p.x).collect::<Vec<_>>(),
        after.iter().map(|p| p.x).collect::<Vec<_>>(),
    );
    assert_eq!(
        (tops, xs.0 == xs.1, stage(&rig.world)),
        (vec![0, 0], true, None),
        "(キャラ窓の上端, 横位置は保つか, 印) 起動の位置={before:?}"
    );
}
