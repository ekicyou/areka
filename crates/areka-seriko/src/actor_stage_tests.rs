//! 窓の知らせ `SerikoMsg::Stage` と合図 `StageAck`（spec: areka-P0-animated-image-playback
//! 要件 2.3・6.1・6.3・6.7・tasks.md 5.1・design「出し直しの前に出た指令を見分ける」）。
//!
//! バルーンの表はスコープ 0 に `parts_film_tests` の子の表（面 1 は終わりなしの子 `ENDLESS`＝コマ
//! 10・11・12 各 100ms と、合計 1 回の子 `ONCE`＝コマ 20・21 各 100ms を置く）。刻みと知らせの時刻は
//! 注入の絶対時刻、乱数は使わない（子の表に抽選は無い）。

use super::test_support::*;
use super::*;
use crate::looper::parts_emo2_tests::emo2_shell_table;
use crate::looper::tests::{always_fire, cfg};
use crate::output::{DisplayCommand, MockSurfaceOutput};
use crate::parts::film_tests::{ENDLESS, ONCE, table as film_table};
use areka_emo_compose::{BindSet, Cell, FilmId, PartKey, PatternState};
use areka_sakura::{ActorKey, CueCommand, TalkCue};
use std::sync::{Arc, Mutex};

fn scope() -> ActorKey {
    ActorKey::from("0")
}

fn stage(open: bool, generation: u64, at_ms: u64) -> SerikoMsg {
    SerikoMsg::Stage {
        note: StageNote::Balloon {
            scope: scope(),
            open,
            face: 1,
            generation,
        },
        at_ms: Some(at_ms),
    }
}

fn ack(generation: u64) -> DisplayCommand {
    DisplayCommand::StageAck {
        scope: scope(),
        generation,
    }
}

/// 同期 `handle_message` に知らせ・cue・刻みを流す足場。
struct Rig {
    resolver: SurfaceResolver,
    states: ScopeStates,
    rt: LoopRuntime,
    out: MockSurfaceOutput,
    records: Arc<Mutex<Vec<DisplayCommand>>>,
}

impl Rig {
    fn new(config: SerikoLoopConfig) -> Self {
        let out = MockSurfaceOutput::new();
        let records = out.records();
        Self {
            resolver: SurfaceResolver::new(BTreeMap::new()),
            states: ScopeStates::new(BindSet::from_ids([])),
            rt: LoopRuntime::new(config),
            out,
            records,
        }
    }

    /// バルーンのスコープ 0 だけに子の表。
    fn films() -> Self {
        Self::new(SerikoLoopConfig {
            shell_table: AnimationTable::empty(),
            balloon_tables: BTreeMap::from([(scope(), film_table())]),
            rng: always_fire(),
        })
    }

    fn send(&mut self, msg: SerikoMsg) -> Vec<DisplayCommand> {
        let before = self.records.lock().unwrap().len();
        let flow = handle_message(
            &self.resolver,
            &BindResolver::empty(),
            &mut self.states,
            &mut self.rt,
            &mut self.out,
            msg,
        );
        assert_eq!(flow, ControlFlow::Continue(()));
        self.records.lock().unwrap()[before..].to_vec()
    }

    fn tick(&mut self, now_ms: u64) -> Vec<DisplayCommand> {
        self.send(SerikoMsg::Tick { now_ms })
    }

    fn cue(&mut self, command: CueCommand) -> Vec<DisplayCommand> {
        self.send(SerikoMsg::Cue(TalkCue {
            at: 0.0,
            actor: scope(),
            command,
            duration: 0.0,
        }))
    }

    fn all(&self) -> Vec<DisplayCommand> {
        self.records.lock().unwrap().clone()
    }
}

/// `ShowBalloon` の欄の子 `film` の絵（`None`＝載っていない＝経過 0）。
fn film(cmd: &DisplayCommand, film: FilmId) -> Option<u32> {
    let DisplayCommand::ShowBalloon {
        surface_id: 1,
        pattern,
        ..
    } = cmd
    else {
        panic!("面 1 の ShowBalloon を期待: {cmd:?}");
    };
    cell(pattern, film)
}

fn cell(pattern: &PatternState, film: FilmId) -> Option<u32> {
    match pattern.cell(Some(PartKey::Film(film)), 0) {
        Cell::Picture(id) => Some(id),
        Cell::Rest => None,
        other => panic!("子の欄に {other:?}"),
    }
}

fn count_acks(cmds: &[DisplayCommand]) -> usize {
    cmds.iter()
        .filter(|c| matches!(c, DisplayCommand::StageAck { .. }))
        .count()
}

/// 開いた窓（世代 1）で回数つきがコマ 21 まで進んだ状態を作る（刻み 1100）。
fn rig_running() -> Rig {
    let mut rig = Rig::films();
    assert_eq!(
        rig.send(stage(true, 1, 1000)),
        vec![ack(1)],
        "最初の出番: 合図だけ（経過 0 は欄に載らない）"
    );
    let cmds = rig.tick(1100);
    assert_eq!(cmds.len(), 1, "{cmds:?}");
    assert_eq!(film(&cmds[0], ENDLESS), Some(11));
    assert_eq!(film(&cmds[0], ONCE), Some(21), "前提: 回数つきが進んでいる");
    rig
}

/// 世代が進んだ知らせ（閉じた知らせを挟まない）: 出力は「合図 → `ShowBalloon`」の順で、回数つきは
/// 捨てられて知らせの時刻 1150 で作り直され、終わりなしは続きから（要件 2.3・6.1）。
#[test]
fn newer_generation_acks_first_then_restarts_finite_clocks_at_the_note_time() {
    let mut rig = rig_running();

    let cmds = rig.send(stage(true, 2, 1150));
    assert_eq!(cmds.len(), 2, "合図と指令の 2 件: {cmds:?}");
    assert_eq!(cmds[0], ack(2), "合図が先");
    assert_eq!(film(&cmds[1], ONCE), None, "回数つきは経過 0 へ戻る");
    assert_eq!(film(&cmds[1], ENDLESS), Some(11), "終わりなしは続き");

    let cmds = rig.tick(1240);
    assert_eq!(film(&cmds[0], ENDLESS), Some(12), "終わりなしは 1000 から");
    assert_eq!(film(&cmds[0], ONCE), None, "1150 から 90ms＝まだ経過 0");
    let cmds = rig.tick(1260);
    assert_eq!(cmds.len(), 1, "{cmds:?}");
    assert_eq!(film(&cmds[0], ONCE), Some(21), "1150 から 110ms");
}

/// 同じ世代・古い世代の知らせでは合図が出ず、回数つきの時計も捨てない（design「世代が同じか古い
/// 知らせでは合図を出さない」）。
#[test]
fn same_or_older_generation_sends_no_ack_and_keeps_finite_clocks() {
    let mut rig = rig_running();

    assert!(rig.send(stage(true, 1, 1150)).is_empty(), "同じ世代");
    assert!(rig.send(stage(true, 0, 1160)).is_empty(), "古い世代");
    let cmds = rig.tick(1200);
    assert_eq!(cmds.len(), 1, "{cmds:?}");
    assert_eq!(film(&cmds[0], ENDLESS), Some(12));
    assert_eq!(film(&cmds[0], ONCE), Some(21), "回数つきは 1000 からの続き");

    // 古い知らせで覚えた世代が戻らない: 1 を飛ばして 2 が来たら合図が出る。
    assert_eq!(rig.send(stage(true, 2, 1210))[0], ack(2));
    assert!(rig.send(stage(true, 1, 1220)).is_empty());
    assert!(rig.send(stage(true, 2, 1230)).is_empty());
}

/// 閉じた知らせは合図を出さず回数つきを捨てる。知らせだけでは `HideBalloon`・`Hide` を出さない
/// （窓の開け閉めは表示層の持ち物）。次の出番で合図が先に出る（要件 2.3・6.1）。
#[test]
fn closed_note_drops_finite_clocks_and_notes_never_hide() {
    let mut rig = rig_running();

    let cmds = rig.send(stage(false, 1, 1150));
    assert_eq!(count_acks(&cmds), 0, "閉じた知らせに合図は無い: {cmds:?}");
    assert_eq!(cmds.len(), 1, "回数つきの欄が経過 0 へ戻る 1 件: {cmds:?}");
    assert_eq!(film(&cmds[0], ONCE), None);
    assert_eq!(film(&cmds[0], ENDLESS), Some(11), "終わりなしは残る");

    let cmds = rig.send(stage(true, 2, 1500));
    assert_eq!(cmds.first(), Some(&ack(2)), "次の出番は合図が先: {cmds:?}");
    let cmds = rig.tick(1650);
    assert_eq!(film(&cmds[0], ONCE), Some(21), "回数つきは 1500 から");

    rig.send(stage(false, 2, 1700));
    rig.send(stage(true, 3, 1800));
    rig.send(stage(true, 3, 1900));
    assert!(
        rig.all().iter().all(|c| !matches!(
            c,
            DisplayCommand::Hide { .. } | DisplayCommand::HideBalloon { .. }
        )),
        "知らせから Hide・HideBalloon は出ない: {:?}",
        rig.all()
    );
}

/// 知らせの無い流れ（emo2 のシェルの表・子の表のバルーンを `\b` で出して刻む）では合図が 1 件も
/// 出ない（要件 6.7）。
#[test]
fn flows_without_notes_emit_no_stage_ack() {
    let mut rig = Rig::new(SerikoLoopConfig {
        shell_table: emo2_shell_table(),
        balloon_tables: BTreeMap::from([(scope(), film_table())]),
        rng: always_fire(),
    });
    rig.tick(0);
    rig.cue(CueCommand::Emote { key: "0".into() });
    rig.cue(CueCommand::BalloonSurface { key: "1".into() });
    for i in 1..=100 {
        rig.tick(i * 40);
    }
    let all = rig.all();
    assert!(
        all.iter()
            .any(|c| matches!(c, DisplayCommand::ShowBalloon { .. })),
        "前提: バルーンのコマが進んでいる"
    );
    assert!(
        all.iter().any(|c| matches!(c, DisplayCommand::Show { .. })),
        "前提: シェルの指令が出ている"
    );
    assert_eq!(count_acks(&all), 0, "知らせが無ければ合図は 0 件");

    // emo2 のシェルだけの流れも同じ。
    let mut rig = Rig::new(cfg(emo2_shell_table(), always_fire()));
    rig.tick(0);
    rig.cue(CueCommand::Emote { key: "0".into() });
    for i in 1..=100 {
        rig.tick(i * 40);
    }
    assert!(!rig.all().is_empty(), "前提: 指令が出ている");
    assert_eq!(count_acks(&rig.all()), 0);
}

/// 時計つきの起動の `send_stage` は送るときの時計の値を載せ、実のアクターが合図を先に出す。
/// 受け手が消えた後の `send_stage` は `debug!` で捨てる（design「受け手が消えた後は `debug!`」）。
#[test]
fn send_stage_carries_the_clock_and_logs_debug_after_the_actor_is_gone() {
    let clock: SerikoClock = Arc::new(|| 1000);
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (sink, handle) = spawn_seriko_clocked(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::from_ids([]),
        BindResolver::empty(),
        SerikoLoopConfig {
            shell_table: AnimationTable::empty(),
            balloon_tables: BTreeMap::from([(scope(), film_table())]),
            rng: always_fire(),
        },
        out,
        Some(clock),
    );
    sink.send_tick(900);
    sink.send_stage(StageNote::Balloon {
        scope: scope(),
        open: true,
        face: 1,
        generation: 1,
    });
    sink.send_tick(1050);
    sink.send_tick(1100);
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");
    let records = records.lock().unwrap().clone();
    assert_eq!(records.first(), Some(&ack(1)), "合図が先: {records:?}");
    assert_eq!(records.len(), 2, "刻み 1100 で 1 件だけ: {records:?}");
    assert_eq!(
        film(&records[1], ONCE),
        Some(21),
        "時計は知らせの時刻 1000 で始まる（直前の刻み 900 なら 1050 で出ている）"
    );

    let logs = capture_logs(|| {
        sink.send_stage(StageNote::Balloon {
            scope: scope(),
            open: false,
            face: 1,
            generation: 1,
        })
    });
    assert!(
        logs.contains("level=DEBUG") && logs.contains("窓の知らせを配送できず破棄した"),
        "{logs}"
    );
}
