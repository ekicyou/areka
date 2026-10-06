//! 時計つきの起動（spec: areka-P0-animated-image-playback 要件 1.6・2.9・3.1・tasks.md 3.5・
//! design「時計の開始の時刻（`SerikoClock`）」）。
//!
//! 台本の合図（`\s`・`\b`・着せ替え）を処理したときに注入の時計を読み、その時刻で `always` の時計が
//! 始まることを、刻みの時刻と区別できる値で確かめる。刻みは注入の絶対時刻・乱数は使わない。

use super::test_support::*;
use super::*;
use crate::bind::BindOptionDecls;
use crate::looper::tests::{always_fire, cfg, pattern_of};
use crate::output::{DisplayCommand, MockSurfaceOutput};
use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_sakura::{ActorKey, CueCommand, TalkCue};
use dola::cue::CueSink;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// 一番上 0 の `always`: 101（待ち 0＝経過 0）→ 100ms で 102 → 100ms で 103（周期 200）。1 は何も持たない。
const TOP_ALWAYS: &str = "surface0\n{\nanimation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation0.pattern2,overlay,103,100,0,0\n}\n\
    surface1\n{\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\n";

/// 一番上 0 は着せ替え 5 で子 100 を出す。100 は [`TOP_ALWAYS`] と同じ `always` を持つ。
const BIND_ALWAYS: &str = "surface0\n{\nanimation5.interval,bind\n\
    animation5.pattern0,overlay,100,0,0,0\n}\n\
    surface100\n{\nanimation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation0.pattern2,overlay,103,100,0,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\n";

fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
}

/// テストが値を書き替えられる時計。
fn manual_clock(start: u64) -> (SerikoClock, Arc<AtomicU64>) {
    let now = Arc::new(AtomicU64::new(start));
    let read = Arc::clone(&now);
    (Arc::new(move || read.load(Ordering::SeqCst)), now)
}

/// 同期 `handle_message` に cue と刻みを流す足場（統括器に時計を注入する）。
struct Rig {
    resolver: SurfaceResolver,
    bind_resolver: BindResolver,
    states: ScopeStates,
    rt: LoopRuntime,
    out: MockSurfaceOutput,
    records: Arc<Mutex<Vec<DisplayCommand>>>,
}

impl Rig {
    fn new(config: SerikoLoopConfig, clock: Option<SerikoClock>) -> Self {
        let sakura = BTreeMap::from([(("目".to_string(), "閉".to_string()), 5)]);
        let out = MockSurfaceOutput::new();
        let records = out.records();
        Self {
            resolver: SurfaceResolver::new(BTreeMap::new()),
            bind_resolver: BindResolver::new(sakura, BTreeMap::new(), BindOptionDecls::default()),
            states: ScopeStates::new(BindSet::from_ids([])),
            rt: LoopRuntime::new(config).with_clock(clock),
            out,
            records,
        }
    }

    fn send(&mut self, msg: SerikoMsg) -> Vec<DisplayCommand> {
        let before = self.records.lock().unwrap().len();
        let flow = handle_message(
            &self.resolver,
            &self.bind_resolver,
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
            actor: ActorKey::from("0"),
            command,
            duration: 0.0,
        }))
    }
}

/// ただ 1 件の指令の欄。
fn single_pattern(cmds: &[DisplayCommand]) -> &PatternState {
    assert_eq!(cmds.len(), 1, "発行は 1 件だけ: {cmds:?}");
    pattern_of(&cmds[0])
}

/// 一番上の animation 0 の欄の絵（`None`＝載っていない＝経過 0）。
fn top(pattern: &PatternState) -> Option<u32> {
    pattern.get(0).map(|f| f.surface_id)
}

/// `\s` の時計の開始は合図を処理した時刻（時計 1030）で、直前の刻み（0）でも合図の後に時計が
/// 進んだ値（5000）でもない（要件 1.6・3.1）。
#[test]
fn surface_cue_starts_always_at_the_clock_time_of_processing() {
    let (clock, now) = manual_clock(0);
    let mut rig = Rig::new(cfg(table_of(TOP_ALWAYS), always_fire()), Some(clock));
    rig.tick(0);
    now.store(1030, Ordering::SeqCst);
    let cmds = rig.cue(CueCommand::Emote { key: "0".into() });
    assert_eq!(top(single_pattern(&cmds)), None, "最初の指令は経過 0");
    now.store(5000, Ordering::SeqCst);

    assert!(
        rig.tick(1100).is_empty(),
        "経過 70＝経過 0 のコマ（刻み 0 から数えれば 102）"
    );
    assert_eq!(
        top(single_pattern(&rig.tick(1130))),
        Some(102),
        "1030 から 100ms"
    );
}

/// 刻みの時刻が開始の時刻より僅かに前でも経過は負にならず 0 で止まる（design「時計の開始の時刻」）。
#[test]
fn tick_slightly_before_the_start_clamps_elapsed_to_zero() {
    let (clock, _now) = manual_clock(1030);
    let mut rig = Rig::new(cfg(table_of(TOP_ALWAYS), always_fire()), Some(clock));
    rig.tick(0);
    rig.cue(CueCommand::Emote { key: "0".into() });

    assert!(rig.tick(1020).is_empty(), "開始の 10ms 前の刻みは経過 0");
    assert_eq!(
        top(rig
            .states
            .current_pattern(&ActorKey::from("0"), Slot::Shell)),
        None
    );
    assert_eq!(
        top(single_pattern(&rig.tick(1130))),
        Some(102),
        "開始は 1030 のまま"
    );
}

/// `\b` の時計の開始も合図を処理した時刻（要件 3.1）。
#[test]
fn balloon_cue_starts_always_at_the_clock_time_of_processing() {
    let (clock, _now) = manual_clock(1030);
    let config = SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(ActorKey::from("0"), table_of(TOP_ALWAYS))]),
        rng: always_fire(),
    };
    let mut rig = Rig::new(config, Some(clock));
    rig.tick(0);
    let cmds = rig.cue(CueCommand::BalloonSurface { key: "0".into() });
    assert_eq!(top(single_pattern(&cmds)), None, "最初の指令は経過 0");

    assert!(rig.tick(1100).is_empty(), "経過 70＝経過 0 のコマ");
    let cmds = rig.tick(1130);
    assert!(
        matches!(cmds[..], [DisplayCommand::ShowBalloon { .. }]),
        "{cmds:?}"
    );
    assert_eq!(top(single_pattern(&cmds)), Some(102), "1030 から 100ms");
}

/// 着せ替えで見えた部品の `always` の時計も合図を処理した時刻で始まる（要件 3.1）。
#[test]
fn bind_cue_starts_part_always_at_the_clock_time_of_processing() {
    let (clock, _now) = manual_clock(1030);
    let table = table_of(BIND_ALWAYS);
    assert!(table.has_animated_parts(), "前提: 動く部品の在る表");
    let mut rig = Rig::new(cfg(table, always_fire()), Some(clock));
    rig.cue(CueCommand::Emote { key: "0".into() });
    rig.tick(0);
    let cmds = rig.cue(CueCommand::command_carrier(
        "bind",
        vec!["目".to_string(), "閉".to_string(), "1".to_string()],
    ));
    let part = |p: &PatternState| {
        p.part(100)
            .map(|(id, f)| (id, f.surface_id))
            .collect::<Vec<_>>()
    };
    assert!(part(single_pattern(&cmds)).is_empty(), "最初の指令は経過 0");

    rig.tick(1100);
    assert!(
        part(
            rig.states
                .current_pattern(&ActorKey::from("0"), Slot::Shell)
        )
        .is_empty(),
        "経過 70＝経過 0 のコマ（刻み 0 から数えれば 102）"
    );
    rig.tick(1130);
    assert_eq!(
        part(
            rig.states
                .current_pattern(&ActorKey::from("0"), Slot::Shell)
        ),
        vec![(0, 102)],
        "1030 から 100ms"
    );
}

/// 実のアクターへ `\s[0]` → 刻み 1100 → `\s[1]` を流し、記録を返す（刻み 0 で始める）。
fn run_actor(
    spawn: impl FnOnce(MockSurfaceOutput) -> (SerikoSink, areka_actor::ActorHandle),
) -> Vec<DisplayCommand> {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn(out);
    sink.send_tick(0);
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "0"));
    sink.send_tick(1100);
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "1"));
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");
    records.lock().unwrap().clone()
}

fn show_surfaces(records: &[DisplayCommand]) -> Vec<(u32, Option<u32>)> {
    records
        .iter()
        .map(|c| match c {
            DisplayCommand::Show {
                surface_id,
                pattern,
                ..
            } => (*surface_id, top(pattern)),
            other => panic!("Show を期待: {other:?}"),
        })
        .collect()
}

/// 時計なしの起動（`spawn_seriko`・`spawn_seriko_clocked(None)`）は今と同じ指令の列（出来事の時刻は
/// 直前の刻み 0）。時計つきは合図を処理した時刻 1030 で始まり、刻み 1100 では指令が出ない。
#[test]
fn clockless_spawn_keeps_the_command_sequence_and_clocked_spawn_starts_at_cue_time() {
    let spawn = |clock: Option<SerikoClock>| {
        move |out| {
            spawn_seriko_clocked(
                SurfaceResolver::new(BTreeMap::new()),
                BindSet::from_ids([]),
                BindResolver::empty(),
                cfg(table_of(TOP_ALWAYS), always_fire()),
                out,
                clock,
            )
        }
    };
    let legacy = run_actor(|out| {
        spawn_seriko(
            SurfaceResolver::new(BTreeMap::new()),
            BindSet::from_ids([]),
            BindResolver::empty(),
            cfg(table_of(TOP_ALWAYS), always_fire()),
            out,
        )
    });
    assert_eq!(
        show_surfaces(&legacy),
        vec![(0, None), (0, Some(102)), (1, None)],
        "時計なし: 刻み 0 から 1100ms で 102"
    );
    assert_eq!(
        run_actor(spawn(None)),
        legacy,
        "None は spawn_seriko と同じ列"
    );

    let (clock, _now) = manual_clock(1030);
    assert_eq!(
        show_surfaces(&run_actor(spawn(Some(clock)))),
        vec![(0, None), (1, None)],
        "時計つき: 1030 から 70ms はまだ経過 0"
    );
}
