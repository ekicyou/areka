//! バルーンの面で部品と `always` を回す檻（spec: areka-P0-animated-image-playback 要件 6.1・6.3・
//! 6.6・6.7・tasks.md 3.6）。
//!
//! バルーンの表もシェルと同じ `parts_film_tests` の表（0 と 1 は終わりなしの子 `ENDLESS` を置き、
//! 1 は合計 1 回の子 `ONCE` も置く）。窓の知らせは `note_stage` で直に覚えさせる（運ぶ口は task 5.1）。
//! 刻みは `on_tick(now_ms)`、乱数は注入列（子の表に抽選は無い。抽選の檻は部品に `random` を置いた表で回す）。

use std::collections::BTreeMap;

use areka_emo_compose::{BindSet, Cell, FilmId, PartKey};

use super::tests::counting_rng;
use super::*;
use crate::parts::PartAnim;
use crate::parts::film_tests::{ENDLESS, ONCE, table as film_table};
use crate::resolve::SurfaceTarget;
use crate::state::StageNote;

fn scope() -> ActorKey {
    ActorKey::from("0")
}

fn note(states: &mut ScopeStates, open: bool, face: u32) {
    states.note_stage(&StageNote::Balloon {
        scope: scope(),
        open,
        face,
        generation: 0,
    });
}

/// シェルの表は `shell`、バルーンの表はスコープ 0 に子の表。
fn runtime(shell: AnimationTable) -> LoopRuntime {
    let (rng, _probe) = counting_rng(&[]);
    LoopRuntime::new(SerikoLoopConfig {
        shell_table: shell,
        balloon_tables: BTreeMap::from([(scope(), film_table())]),
        rng,
    })
}

fn clock(rt: &LoopRuntime, slot: Slot, film: FilmId) -> Option<PartAnim> {
    rt.parts.clock_at(&scope(), slot, PartKey::Film(film), 0)
}

fn playing(at: u64) -> Option<PartAnim> {
    Some(PartAnim::Playing { started_at_ms: at })
}

/// `slot` の面の子 `film` の欄の絵（`None`＝載っていない＝経過 0）。
fn film_cell(states: &ScopeStates, slot: Slot, film: FilmId) -> Option<u32> {
    match states
        .current_pattern(&scope(), slot)
        .cell(Some(PartKey::Film(film)), 0)
    {
        Cell::Picture(id) => Some(id),
        Cell::Rest => None,
        other => panic!("子の欄に {other:?}"),
    }
}

/// 窓が開いている面（`\b` を受けていないので知らせの面 0）で、子の時計が刻みの時刻で生まれ、
/// コマが替わると知らせの面の `ShowBalloon` が出る。出来事の直後の `refresh` でも同じ面で生まれる。
#[test]
fn open_window_births_child_clock_on_noted_face() {
    let mut rt = runtime(AnimationTable::empty());
    let mut states = ScopeStates::new(BindSet::default());
    rt.on_tick(500, &mut states);

    note(&mut states, true, 0);
    assert_eq!(
        rt.refresh(&scope(), Slot::Balloon, Some(600), &mut states),
        None
    );
    assert_eq!(
        clock(&rt, Slot::Balloon, ENDLESS),
        playing(600),
        "知らせの直後の評価の時刻で生まれる"
    );

    let cmds = rt.on_tick(700, &mut states);
    assert_eq!(cmds.len(), 1, "コマが替わったので 1 件: {cmds:?}");
    let DisplayCommand::ShowBalloon { surface_id, .. } = &cmds[0] else {
        panic!("ShowBalloon を期待: {cmds:?}");
    };
    assert_eq!(*surface_id, 0, "知らせの面");
    assert_eq!(film_cell(&states, Slot::Balloon, ENDLESS), Some(11));
    assert_eq!(
        clock(&rt, Slot::Shell, ENDLESS),
        None,
        "シェルの面には生まれない"
    );
}

/// 窓が閉じている間は評価しても時計が生まれない。開くと生まれ、閉じると回数つきが捨てられて
/// 終わりなしだけが進む。もう 1 度開くと回数つきは頭から（要件 2.3・6.1）。
#[test]
fn closed_window_births_no_clock_and_only_endless_clocks_advance() {
    let mut rt = runtime(AnimationTable::empty());
    let mut states = ScopeStates::new(BindSet::default());

    note(&mut states, false, 1);
    assert!(rt.on_tick(1000, &mut states).is_empty());
    assert_eq!(
        rt.refresh(&scope(), Slot::Balloon, Some(1000), &mut states),
        None
    );
    assert_eq!(
        clock(&rt, Slot::Balloon, ENDLESS),
        None,
        "閉じている間は生まれない"
    );
    assert_eq!(clock(&rt, Slot::Balloon, ONCE), None);

    note(&mut states, true, 1);
    rt.on_tick(2000, &mut states);
    assert_eq!(clock(&rt, Slot::Balloon, ENDLESS), playing(2000));
    assert_eq!(clock(&rt, Slot::Balloon, ONCE), playing(2000));
    assert_eq!(rt.on_tick(2100, &mut states).len(), 1);
    assert_eq!(film_cell(&states, Slot::Balloon, ENDLESS), Some(11));
    assert_eq!(film_cell(&states, Slot::Balloon, ONCE), Some(21));

    note(&mut states, false, 1);
    let cmds = rt.on_tick(2200, &mut states);
    assert_eq!(clock(&rt, Slot::Balloon, ONCE), None, "回数つきは捨てる");
    assert_eq!(
        clock(&rt, Slot::Balloon, ENDLESS),
        playing(2000),
        "終わりなしは残る"
    );
    assert_eq!(
        film_cell(&states, Slot::Balloon, ENDLESS),
        Some(12),
        "終わりなしは進む"
    );
    assert_eq!(
        film_cell(&states, Slot::Balloon, ONCE),
        None,
        "回数つきは経過 0"
    );
    assert!(
        matches!(
            cmds.as_slice(),
            [DisplayCommand::ShowBalloon { surface_id: 1, .. }]
        ),
        "隠れていてもコマは出す（出すか隠すかは表示層・要件 6.3）: {cmds:?}"
    );
    rt.on_tick(2300, &mut states);
    assert_eq!(
        clock(&rt, Slot::Balloon, ONCE),
        None,
        "閉じている間は生まれない"
    );

    note(&mut states, true, 1);
    rt.on_tick(2400, &mut states);
    assert_eq!(
        clock(&rt, Slot::Balloon, ONCE),
        playing(2400),
        "開くと頭から"
    );
    assert_eq!(clock(&rt, Slot::Balloon, ENDLESS), playing(2000));
}

/// `\b[-1]` の後の評価で、バルーンの面の回数つきの時計を捨てる（終わりなしは残す・3.4 の道）。
#[test]
fn balloon_hide_cue_drops_finite_clocks() {
    let mut rt = runtime(AnimationTable::empty());
    let mut states = ScopeStates::new(BindSet::default());
    rt.on_tick(0, &mut states);
    states.apply_balloon(&scope(), SurfaceTarget::Show(1));
    rt.on_surface_changed(&scope(), Slot::Balloon);
    rt.refresh(&scope(), Slot::Balloon, Some(100), &mut states);
    assert_eq!(clock(&rt, Slot::Balloon, ONCE), playing(100));

    states.apply_balloon(&scope(), SurfaceTarget::Hide);
    rt.on_surface_changed(&scope(), Slot::Balloon);
    assert_eq!(
        rt.refresh(&scope(), Slot::Balloon, Some(150), &mut states),
        None
    );
    assert_eq!(clock(&rt, Slot::Balloon, ONCE), None);
    assert_eq!(clock(&rt, Slot::Balloon, ENDLESS), playing(100));
}

/// シェルとバルーンが同じ子を置いても時計は混ざらない（要件 6.6）。
#[test]
fn shell_and_balloon_clocks_do_not_mix() {
    let mut rt = runtime(film_table());
    let mut states = ScopeStates::new(BindSet::default());
    states.apply(&scope(), SurfaceTarget::Show(0));
    rt.on_tick(1000, &mut states);
    assert_eq!(clock(&rt, Slot::Shell, ENDLESS), playing(1000));
    assert_eq!(clock(&rt, Slot::Balloon, ENDLESS), None);

    note(&mut states, true, 0);
    rt.on_tick(1500, &mut states);
    assert_eq!(clock(&rt, Slot::Balloon, ENDLESS), playing(1500));
    assert_eq!(
        clock(&rt, Slot::Shell, ENDLESS),
        playing(1000),
        "シェルの時計はそのまま"
    );

    rt.on_tick(1600, &mut states);
    assert_eq!(
        film_cell(&states, Slot::Shell, ENDLESS),
        None,
        "シェルは 2 周目の頭"
    );
    assert_eq!(film_cell(&states, Slot::Balloon, ENDLESS), Some(11));
}

/// バルーンの表が `text` だけのランタイム（シェルの表は空・乱数は `rng`）。
fn runtime_of(text: &str, rng: LoopRng) -> LoopRuntime {
    let table = AnimationTable::from_world(&areka_emo_compose::EmoWorld::build(
        &areka_parsers::shell::parse(text),
    ));
    LoopRuntime::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(scope(), table)]),
        rng,
    })
}

/// バルーンの面 0 の一番上の `always`（周期 200）。
const BALLOON_TOP_ALWAYS: &str = "surface0\n{\nanimation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation0.pattern2,overlay,103,100,0,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\n";

/// バルーンの面 0 の一番上の `always` の再生の開始の時刻（無ければ `None`）。
fn top_started(rt: &LoopRuntime) -> Option<u64> {
    rt.playback
        .get(&(scope(), Slot::Balloon))
        .and_then(|pb| pb.get(&0))
        .map(|p| p.started_at_ms)
}

/// 窓が閉じている間は、刻みでも出来事の直後の評価でも一番上の `always` の再生が生まれない。
/// 開いた後は、評価の時刻（刻みの時刻・出来事の時刻）で生まれる（要件 6.1）。
#[test]
fn closed_window_births_no_top_always_playback() {
    let (rng, _probe) = counting_rng(&[]);
    let mut rt = runtime_of(BALLOON_TOP_ALWAYS, rng);
    let mut states = ScopeStates::new(BindSet::default());

    note(&mut states, false, 0);
    rt.on_tick(1000, &mut states);
    assert_eq!(top_started(&rt), None, "閉じている間の刻みでは生まれない");
    rt.refresh(&scope(), Slot::Balloon, Some(1100), &mut states);
    assert_eq!(top_started(&rt), None, "閉じている間の評価では生まれない");

    note(&mut states, true, 0);
    rt.refresh(&scope(), Slot::Balloon, Some(1200), &mut states);
    assert_eq!(
        top_started(&rt),
        Some(1200),
        "開いた後の評価の時刻で生まれる"
    );

    let (rng, _probe) = counting_rng(&[]);
    let mut rt = runtime_of(BALLOON_TOP_ALWAYS, rng);
    let mut states = ScopeStates::new(BindSet::default());
    note(&mut states, true, 0);
    rt.on_tick(1000, &mut states);
    assert_eq!(
        top_started(&rt),
        Some(1000),
        "開いている面の刻みの時刻で生まれる"
    );
}

/// バルーンの面 0 が部品 100 を置き、100 が `random` を持つ。
const BALLOON_PART_RANDOM: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nanimation0.interval,random,2\n\
    animation0.pattern0,overlay,101,100,0,0\n}\n\
    surface101\n{\n}\n";

/// 窓が閉じている間は、抽選の境を跨いでも部品の抽選をせず（乱数を引かず）時計が生まれない。
/// 開いた後の境で初めて引いて生まれる（要件 6.1）。
#[test]
fn closed_window_draws_no_part_lottery() {
    let (rng, probe) = counting_rng(&[0]);
    let mut rt = runtime_of(BALLOON_PART_RANDOM, rng);
    let mut states = ScopeStates::new(BindSet::default());
    let part_clock = |rt: &LoopRuntime| {
        rt.parts
            .clock_at(&scope(), Slot::Balloon, PartKey::Surface(100), 0)
    };

    note(&mut states, false, 0);
    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states);
    rt.on_tick(2000, &mut states);
    assert_eq!(
        probe.lock().unwrap().calls,
        0,
        "閉じている間は乱数を引かない"
    );
    assert_eq!(part_clock(&rt), None, "閉じている間は時計が生まれない");

    note(&mut states, true, 0);
    rt.on_tick(3000, &mut states);
    assert_eq!(probe.lock().unwrap().calls, 1, "開いた後の境で引く");
    assert_eq!(part_clock(&rt), playing(3000));
}
