//! 時計が止まっている間（眠り）に seriko のコマが進まず、動き出すと止めた所から続く事の結合テスト。

use std::collections::BTreeMap;

use areka_emo_compose::{BindSet, ComposeMethod, EmoWorld, PatternFrame, PatternState};
use areka_sakura::{ActorKey, CueSink};
use areka_seriko::{AnimationTable, DisplayCommand, SerikoLoopConfig, SurfaceResolver};
use dola::time::testing::{at_ms, ms};

mod common;
use common::*;

/// 面 10 に、毎回の境で発火する 1 本（コマは経った 0・40・120 ms で 2106・2110・`-1`）。
const SURFACES: &str = "surface10\n{\nanimation0.interval,random,4\n\
animation0.pattern0,overlay,2106,0,0,0\n\
animation0.pattern1,overlay,2110,40,0,0\n\
animation0.pattern2,overlay,-1,80,0,0\n}\n";

fn show(frame: Option<u32>) -> DisplayCommand {
    let mut pattern = PatternState::default();
    if let Some(surface_id) = frame {
        pattern.set(
            0,
            PatternFrame {
                surface_id,
                method: ComposeMethod::Overlay,
                x: 0,
                y: 0,
            },
        );
    }
    DisplayCommand::Show {
        scope: ActorKey::from("0"),
        surface_id: 10,
        binds: BindSet::from_ids([]),
        pattern,
    }
}

fn run(ticks: &[i64]) -> Vec<DisplayCommand> {
    let world = EmoWorld::build(&areka_parsers::shell::parse(SURFACES));
    let config = SerikoLoopConfig {
        shell_table: AnimationTable::from_world(&world),
        balloon_tables: BTreeMap::new(),
        rng: Box::new(|_| 0),
        pick_rng: areka_seriko::seeded_rng(0),
    };
    run_actor(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::from_ids([]),
        config,
        |sink| {
            CueSink::emit(sink, emote_cue(ms(0), "0", "10"));
            for &t in ticks {
                sink.send_tick(at_ms(t));
            }
        },
    )
}

/// 境の前（500 ms）と発火の 20 ms 後（1,020 ms）で時計が止まり、その値の刻みが何度来ても、境の
/// 抽選も次のコマ（経った 40 ms）も起きない。動き出した後は 1,040 ms ちょうどで 2110、1,120 ms で
/// `-1` の止めになり、出る指令は止まらずに刻んだ時と同じである。
#[test]
fn frames_hold_while_clock_is_stopped_and_resume_from_there() {
    let expected = vec![show(None), show(Some(2106)), show(Some(2110)), show(None)];

    let uninterrupted = run(&[0, 1_000, 1_020, 1_039, 1_040, 1_120]);
    assert_eq!(uninterrupted, expected);

    let slept = run(&[
        0, 500, 500, 500, 1_000, 1_020, 1_020, 1_020, 1_020, 1_039, 1_040, 1_120,
    ]);
    assert_eq!(
        slept, expected,
        "止まった時計の刻みではコマが進まず、動き出すと止めた所から続く"
    );

    let resumed_short = run(&[0, 1_000, 1_020, 1_020, 1_020, 1_039]);
    assert_eq!(
        resumed_short,
        vec![show(None), show(Some(2106))],
        "経った 40 ms に届くまで次のコマは出ない"
    );
}
