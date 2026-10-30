//! 一番上とバルーンの面の刻みと合図で働きのコマを当てる結合のテスト（spec: seriko-pattern-calls
//! 要件 3.1・3.4・3.5・3.6・3.8・3.9・4.1・4.5・11.2・11.3・9.1）。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use areka_emo_compose::BindSet;

use super::test_support::{cell, scope, started, switch, table_of};
use super::tests::counting_rng;
use super::*;
use crate::resolve::SurfaceTarget;

/// 絵の面（中身は空）。
const SURFACES: &str = "surface100\n{\n}\nsurface101\n{\n}\nsurface110\n{\n}\n\
    surface111\n{\n}\nsurface120\n{\n}\nsurface130\n{\n}\n";

/// 「．さくら」の形: 抽選で動く animation2 が `alternativestart,(0,1)` で 0 か 1 を呼ぶ
/// （呼ぶ前に 200 ms の描くコマ・相手は呼ばれるだけで 1200 ms 続く）。
const SAKURA: &str = "surface0\n{\n\
    animation0.interval,never\n\
    animation0.pattern0,overlay,100,0,0,0\n\
    animation0.pattern1,overlay,101,200,0,0\n\
    animation0.pattern2,overlay,100,1000,0,0\n\
    animation1.interval,never\n\
    animation1.pattern0,overlay,110,0,0,0\n\
    animation1.pattern1,overlay,111,200,0,0\n\
    animation1.pattern2,overlay,110,1000,0,0\n\
    animation2.interval,random,2\n\
    animation2.pattern0,overlay,120,200,0,0\n\
    animation2.pattern1,alternativestart,(0,1)\n}\n";

/// 選びの乱数: 渡された並びの長さを覚え、いつも `pick` を返す。
fn pick_rng(pick: u32) -> (LoopRng, Arc<Mutex<Vec<u32>>>) {
    let bounds = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&bounds);
    let rng: LoopRng = Box::new(move |bound| {
        seen.lock().unwrap().push(bound);
        pick
    });
    (rng, bounds)
}

fn config(text: &str, rng: LoopRng, pick_rng: LoopRng) -> SerikoLoopConfig {
    SerikoLoopConfig {
        shell_table: table_of(&format!("{text}{SURFACES}")),
        balloon_tables: BTreeMap::new(),
        rng,
        pick_rng,
    }
}

fn no_binds() -> ScopeStates {
    ScopeStates::new(crate::DefaultBinds::uniform(BindSet::default()))
}

/// シェルの面 0 を出した状態のランタイム。
fn shell_shown(text: &str, rng: LoopRng, pick_rng: LoopRng) -> (LoopRuntime, ScopeStates) {
    let mut states = no_binds();
    states.apply(&scope(), SurfaceTarget::Show(0));
    (LoopRuntime::new(config(text, rng, pick_rng)), states)
}

/// `alternativestart` は並びの全部から等しく 1 つを選び、選んだ相手を呼び出しの予定の時刻に頭から
/// 始める。遅れた刻みでも開始は予定の時刻で、相手の今のコマは同じ刻みの `Show` に載る。呼んだ側は
/// 働きのコマが今のコマでも前の描くコマを残す（3.1・3.3・3.9・1.3）。
#[test]
fn alternativestart_starts_the_pick_from_head_at_call_time_in_the_same_show() {
    for (pick, target, other) in [(0, 0, 1), (1, 1, 0)] {
        let (lottery, probe) = counting_rng(&[0]);
        let (picker, bounds) = pick_rng(pick);
        let (mut rt, mut states) = shell_shown(SAKURA, lottery, picker);
        assert!(rt.on_tick(0, &mut states).is_empty());
        rt.on_tick(1000, &mut states); // animation2 が 1000 に始まる。呼び出しは 1200。
        assert_eq!(started(&rt, Slot::Shell, target), None);

        let cmds = rt.on_tick(1500, &mut states); // 遅れた刻み
        assert_eq!(
            *bounds.lock().unwrap(),
            vec![2],
            "並び (0,1) の全部から選ぶ"
        );
        assert_eq!(started(&rt, Slot::Shell, target), Some(1200));
        assert_eq!(started(&rt, Slot::Shell, other), None);
        // 経過 300 ≥ 200 で相手の 2 枚目が同じ刻みに載る。
        assert_eq!(cell(&states, Slot::Shell, target), Some(101 + 10 * target));
        assert_eq!(
            cell(&states, Slot::Shell, 2),
            Some(120),
            "呼んだ側は前の絵を残す"
        );
        assert_eq!(cmds.len(), 1, "同じ刻みの Show 1 件");
        assert_eq!(probe.lock().unwrap().calls, 1, "選びは抽選の乱数を引かない");
    }
}

/// `never` を足しても、同じ種なら抽選の結果は今と同じ（3.6・11.3）。
#[test]
fn adding_a_never_animation_keeps_the_lottery() {
    const PLAIN: &str = "surface0\n{\n\
        animation2.interval,random,2\n\
        animation2.pattern0,overlay,120,0,0,0\n\
        animation2.pattern1,overlay,111,100,0,0\n";
    const NEVER: &str = "animation5.interval,never\n\
        animation5.pattern0,overlay,130,0,0,0\n";
    let trace = |text: String| {
        let (mut rt, mut states) = shell_shown(&text, seeded_rng(7), seeded_rng(9));
        (0..=20_000)
            .step_by(250)
            .map(|t| {
                rt.on_tick(t, &mut states);
                started(&rt, Slot::Shell, 2)
            })
            .collect::<Vec<_>>()
    };
    let plain = trace(format!("{PLAIN}}}\n"));
    assert!(plain.iter().any(Option::is_some), "抽選で 1 度は始まる");
    assert_eq!(trace(format!("{PLAIN}{NEVER}}}\n")), plain);
}

/// `stop` の見え方は `-1` と同じ（相手の欄と再生が外れる・呼んだ側は残る・4.1）。
#[test]
fn stop_looks_the_same_as_minus_one() {
    const STOP: &str = "surface0\n{\n\
        animation0.interval,random,2\n\
        animation0.pattern0,overlay,100,0,0,0\n\
        animation0.pattern1,overlay,101,5000,0,0\n\
        animation1.interval,random,2\n\
        animation1.pattern0,overlay,120,300,0,0\n\
        animation1.pattern1,stop,0\n}\n";
    const MINUS_ONE: &str = "surface0\n{\n\
        animation0.interval,random,2\n\
        animation0.pattern0,overlay,100,0,0,0\n\
        animation0.pattern1,overlay,-1,300,0,0\n\
        animation1.interval,random,2\n\
        animation1.pattern0,overlay,120,300,0,0\n}\n";
    let run = |text: &str| {
        let (lottery, _probe) = counting_rng(&[0, 0]);
        let (mut rt, mut states) = shell_shown(text, lottery, seeded_rng(0));
        [0, 1000, 1200, 1300, 1400]
            .map(|t| {
                let cmds = rt.on_tick(t, &mut states);
                (
                    cmds,
                    states.current_pattern(&scope(), Slot::Shell).clone(),
                    started(&rt, Slot::Shell, 0),
                )
            })
            .to_vec()
    };
    let stop = run(STOP);
    assert_eq!(stop[3].2, None, "1300 で止まる");
    assert_eq!(stop, run(MINUS_ONE));
}

/// 止めた `always` は、面を出すまで始め直さない。面を出したら印が消え、次の評価で始まる（M2）。
#[test]
fn stopped_always_stays_stopped_until_the_surface_is_shown() {
    const STOP_ALWAYS: &str = "surface0\n{\n\
        animation0.interval,always\n\
        animation0.pattern0,overlay,100,100,0,0\n\
        animation0.pattern1,overlay,101,100,0,0\n\
        animation1.interval,random,2\n\
        animation1.pattern0,overlay,120,100,0,0\n\
        animation1.pattern1,stop,0\n}\n";
    let (lottery, _probe) = counting_rng(&[0]);
    let (mut rt, mut states) = shell_shown(STOP_ALWAYS, lottery, seeded_rng(0));
    rt.on_tick(0, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 0), Some(0));
    rt.on_tick(1000, &mut states); // animation1 が始まる。止めは 1100。
    rt.on_tick(1100, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 0), None);
    assert_eq!(cell(&states, Slot::Shell, 0), None);
    for t in [1200, 2500, 3000] {
        rt.on_tick(t, &mut states);
        assert_eq!(started(&rt, Slot::Shell, 0), None, "{t} でも止まったまま");
    }
    assert!(rt.on_surface_shown(&scope(), Slot::Shell), "印を消した");
    assert!(
        !rt.on_surface_shown(&scope(), Slot::Shell),
        "消す印はもう無い"
    );
    rt.on_tick(3050, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 0), Some(3050));
}

/// 合図の時刻の更新でも頭のコマを置く前に当てる。`always` の周の頭が働きのコマなら、前の周の最後の
/// 描くコマを残す（1 周目は欄を外す・1.3・7.7）。
#[test]
fn refresh_applies_calls_at_the_event_time_and_always_lap_head_keeps_last_draw() {
    const CALLER: &str = "surface0\n{\n\
        animation0.interval,always\n\
        animation0.pattern0,start,1\n\
        animation0.pattern1,overlay,120,100,0,0\n\
        animation1.interval,never\n\
        animation1.pattern0,overlay,110,0,0,0\n\
        animation1.pattern1,overlay,111,500,0,0\n}\n";
    let (lottery, _probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(config(CALLER, lottery, seeded_rng(0)));
    let mut states = no_binds();
    let shown = switch(&mut rt, &mut states, SurfaceTarget::Show(0), 40).expect("Show");
    assert_eq!(started(&rt, Slot::Shell, 1), Some(40));
    let DisplayCommand::Show { pattern, .. } = shown else {
        panic!("Show を期待: {shown:?}");
    };
    assert_eq!(pattern.get(1).map(|f| f.surface_id), Some(110));
    assert_eq!(pattern.get(0), None, "1 周目の頭の働きのコマは絵を重ねない");

    rt.on_tick(150, &mut states); // 2 周目の頭（140）の start がもう 1 度当たる。
    assert_eq!(started(&rt, Slot::Shell, 1), Some(140));
    assert_eq!(
        cell(&states, Slot::Shell, 0),
        Some(120),
        "前の周の描くコマを残す"
    );
}

/// バルーンの面でも同じに動く（3.8）。
#[test]
fn balloon_surface_applies_calls_the_same_way() {
    let (lottery, _probe) = counting_rng(&[0]);
    let (picker, _bounds) = pick_rng(1);
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(scope(), table_of(&format!("{SAKURA}{SURFACES}")))]),
        rng: lottery,
        pick_rng: picker,
    });
    let mut states = no_binds();
    states.apply_balloon(&scope(), SurfaceTarget::Show(0));
    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states);
    rt.on_tick(1500, &mut states);
    assert_eq!(started(&rt, Slot::Balloon, 1), Some(1200));
    assert_eq!(cell(&states, Slot::Balloon, 1), Some(111));
    assert_eq!(cell(&states, Slot::Balloon, 2), Some(120));
}

/// 抽選で動く animation0 が 100 の絵の後に `move` を 4 度当て、最後に `-1` で止まる（spec:
/// seriko-pattern-calls 7.1・7.2・7.6）。1000 に始まれば: 1100 (3,5)・1200 絵 101 と (7,5)・
/// 1300 同じ (7,5)・1400 (0,9999)・1500 `-1`。
const MOVES: &str = "surface0\n{\n\
    animation0.interval,random,2\n\
    animation0.pattern0,overlay,100,0,0,0\n\
    animation0.pattern1,move,0,100,3,5\n\
    animation0.pattern2,overlay,101,100,0,0\n\
    animation0.pattern3,move,0,0,7,5\n\
    animation0.pattern4,move,0,100,7,5\n\
    animation0.pattern5,move,0,100,0,9999\n\
    animation0.pattern6,overlay,-1,100,0,0\n}\n";

/// 指令の並びのうち、ずれの命令の値（ずれの命令が `Show` の後に並ぶことも確かめる）。
fn shifts_of(cmds: &[DisplayCommand]) -> Vec<(i64, i64)> {
    if let Some(i) = cmds
        .iter()
        .position(|c| matches!(c, DisplayCommand::MoveShift { .. }))
    {
        assert!(
            cmds[i..]
                .iter()
                .all(|c| matches!(c, DisplayCommand::MoveShift { .. })),
            "ずれの命令は Show の後: {cmds:?}"
        );
    }
    cmds.iter()
        .filter_map(|c| match c {
            DisplayCommand::MoveShift { scope: s, x, y } => {
                assert_eq!(*s, scope());
                Some((*x, *y))
            }
            _ => None,
        })
        .collect()
}

/// ずれは値が変わった刻みにだけ、同じ刻みの `Show` の後に出る（同じ値は繰り返さない）。終わり・`-1`
/// では戻さず、面を出したときに (0, 0) を 1 度だけ出す（3.8・7.1・7.2・7.6・M3）。
#[test]
fn move_shift_is_emitted_when_it_changes_and_reset_only_when_the_surface_is_shown() {
    let (lottery, _probe) = counting_rng(&[0]);
    let (mut rt, mut states) = shell_shown(MOVES, lottery, seeded_rng(0));
    rt.on_tick(0, &mut states);
    let expect: [(u64, usize, &[(i64, i64)]); 6] = [
        (1000, 1, &[]),
        (1100, 0, &[(3, 5)]),
        (1200, 1, &[(7, 5)]),
        (1300, 0, &[]),
        (1400, 0, &[(0, 9999)]),
        (1500, 1, &[]),
    ];
    for (t, shows, want) in expect {
        let cmds = rt.on_tick(t, &mut states);
        assert_eq!(shifts_of(&cmds), want, "{t}: {cmds:?}");
        assert_eq!(cmds.len() - want.len(), shows, "{t} の Show: {cmds:?}");
    }
    assert!(
        rt.on_tick(1600, &mut states).is_empty(),
        "止まっても戻さない"
    );
    assert!(rt.take_shifts().is_empty());

    assert!(!rt.on_surface_shown(&scope(), Slot::Shell));
    assert_eq!(
        rt.take_shifts(),
        vec![DisplayCommand::MoveShift {
            scope: scope(),
            x: 0,
            y: 0
        }],
        "面を出したら (0, 0)"
    );
    rt.on_surface_shown(&scope(), Slot::Shell);
    assert!(rt.take_shifts().is_empty(), "0 のときは出さない");
}

/// バルーンの面のずれはバルーン用の命令で出る（3.8）。
#[test]
fn balloon_surface_emits_the_balloon_shift() {
    let (lottery, _probe) = counting_rng(&[0]);
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(scope(), table_of(&format!("{MOVES}{SURFACES}")))]),
        rng: lottery,
        pick_rng: seeded_rng(0),
    });
    let mut states = no_binds();
    states.apply_balloon(&scope(), SurfaceTarget::Show(0));
    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states);
    assert_eq!(
        rt.on_tick(1100, &mut states),
        vec![DisplayCommand::MoveShiftBalloon {
            scope: scope(),
            x: 3,
            y: 5
        }]
    );
    rt.on_surface_shown(&scope(), Slot::Balloon);
    assert_eq!(
        rt.take_shifts(),
        vec![DisplayCommand::MoveShiftBalloon {
            scope: scope(),
            x: 0,
            y: 0
        }]
    );
}

/// 合図の時刻の更新で鳴った `runonce` の `move` も、その時刻のずれとして積まれる（3.1・7.1）。
#[test]
fn runonce_move_applied_at_the_event_time_is_queued() {
    const RUNONCE: &str = "surface0\n{\n\
        animation0.interval,runonce\n\
        animation0.pattern0,move,0,0,2,4\n\
        animation0.pattern1,overlay,100,100,0,0\n}\n";
    let (lottery, _probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(config(RUNONCE, lottery, seeded_rng(0)));
    let mut states = no_binds();
    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 40);
    assert_eq!(
        rt.take_shifts(),
        vec![DisplayCommand::MoveShift {
            scope: scope(),
            x: 2,
            y: 4
        }]
    );
    rt.on_tick(200, &mut states);
    assert!(rt.take_shifts().is_empty(), "同じコマを 2 度当てない");
}

/// オフラインの書き出しの結果にずれは写らない（ずれは `PatternState` の外・9.6・M8）。
#[test]
fn settled_surface_carries_no_shift() {
    let settle = |text: &str| {
        let table = table_of(&format!(
            "{}{SURFACES}",
            text.replace("random,2", "runonce")
        ));
        crate::settle_surface(&table, 0, &BindSet::default(), 0)
    };
    let zeroed = MOVES
        .replace(",3,5", ",0,0")
        .replace(",7,5", ",0,0")
        .replace(",0,9999", ",0,0");
    let moved = settle(MOVES);
    assert_eq!(moved, settle(&zeroed));
    assert_eq!(moved.get(0), None, "`-1` で止まった後");
}
