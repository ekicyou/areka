//! 一番上の `always`・出来事の直後の評価・表の差し替えの檻（spec: areka-P0-animated-image-playback
//! 要件 3.6・4.1・4.3・4.7・7.2・tasks.md 3.4）。
//!
//! 面の切り替えはアクターと同じ順（`apply` → `on_surface_changed` → `refresh`）で踏む。刻みは
//! `on_tick(now_ms)`、乱数は注入列で、どちらも決定論。出来事の時刻は時計の無い起動と同じく
//! 直前の刻みの時刻（`refresh` の `at_ms` に `None`）。

use std::collections::BTreeMap;

use areka_emo_compose::{BindSet, Cell, EmoWorld, PartKey, PatternState};

use super::tests::{cfg, counting_rng};
use super::*;
use crate::parts::PartAnim;
use crate::parts::film_tests::{ENDLESS, ONCE, table as film_table};
use crate::resolve::SurfaceTarget;

fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
}

fn scope() -> ActorKey {
    ActorKey::from("0")
}

/// アクターの面の切り替えと同じ順で踏み、出す指令（無ければ `None`）を返す。
fn switch(
    rt: &mut LoopRuntime,
    states: &mut ScopeStates,
    target: SurfaceTarget,
) -> Option<DisplayCommand> {
    let crate::state::ApplyOutcome::Changed(cmd) = states.apply(&scope(), target) else {
        return None;
    };
    rt.on_surface_changed(&scope(), Slot::Shell);
    Some(
        rt.refresh(&scope(), Slot::Shell, None, states)
            .unwrap_or(cmd),
    )
}

/// 一番上の animation 0 の欄の読み（`None`＝載っていない＝経過 0）。
fn top_cell(states: &ScopeStates) -> Option<u32> {
    states
        .current_pattern(&scope(), Slot::Shell)
        .get(0)
        .map(|f| f.surface_id)
}

/// 子 `film` の欄の絵（`None`＝載っていない＝経過 0）。
fn film_cell(pattern: &PatternState, film: areka_emo_compose::FilmId) -> Option<u32> {
    match pattern.cell(Some(PartKey::Film(film)), 0) {
        Cell::Picture(id) => Some(id),
        Cell::Rest => None,
        other => panic!("子の欄に {other:?}"),
    }
}

/// 一番上 0 の `always`: 101（待ち 0＝経過 0）→ 100ms で 102 → 100ms で 103（周期 200）。1 は何も持たない。
const TOP_ALWAYS: &str = "surface0\n{\nanimation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation0.pattern2,overlay,103,100,0,0\n}\n\
    surface1\n{\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\n";

/// 一番上の `always` は抽選を待たず（乱数を引かず、境界の前から）始まり、切り替えで捨てられ、
/// 戻ると戻った出来事の時刻から頭で始め直す（要件 4.1・4.3）。
#[test]
fn top_always_starts_without_lottery_and_restarts_from_head_on_switch() {
    let (rng, probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(cfg(table_of(TOP_ALWAYS), rng));
    let mut states = ScopeStates::new(BindSet::default());

    assert!(rt.on_tick(0, &mut states).is_empty());
    let cmd = switch(&mut rt, &mut states, SurfaceTarget::Show(0)).expect("切り替えの Show");
    assert!(
        tests::pattern_of(&cmd).is_empty(),
        "最初の指令は経過 0（欄に載らない）"
    );

    assert!(rt.on_tick(50, &mut states).is_empty(), "まだ経過 0 のコマ");
    let cmds = rt.on_tick(100, &mut states);
    assert_eq!(cmds.len(), 1, "境界（1000）の前に 2 枚目へ進む");
    assert_eq!(
        tests::pattern_of(&cmds[0]).get(0).map(|f| f.surface_id),
        Some(102)
    );
    assert!(
        rt.on_tick(150, &mut states).is_empty(),
        "コマが替わらなければ出さない"
    );

    // 刻みの間に 1 → 0 と戻る: 戻った出来事の時刻（直前の刻み 150）から頭で始め直す。
    switch(&mut rt, &mut states, SurfaceTarget::Show(1)).expect("1 へ");
    let cmd = switch(&mut rt, &mut states, SurfaceTarget::Show(0)).expect("0 へ");
    assert!(tests::pattern_of(&cmd).is_empty());
    assert!(
        rt.on_tick(200, &mut states).is_empty(),
        "経過 50＝経過 0 のコマ"
    );
    assert_eq!(top_cell(&states), None);
    assert_eq!(rt.on_tick(250, &mut states).len(), 1);
    assert_eq!(top_cell(&states), Some(102), "経過 100 で 2 枚目＝頭から");

    // 境界を何度跨いでも乱数を引かない。
    let mut now = 300;
    while now <= 3000 {
        rt.on_tick(now, &mut states);
        now += 50;
    }
    assert_eq!(probe.lock().unwrap().calls, 0, "always は乱数を引かない");
}

/// 一番上 0 の `random` だけ（201 を出して 300ms 後に `-1`）。
const RANDOM_ONLY: &str = "surface0
{
animation1.interval,random,2
    animation1.pattern0,overlay,201,0,0,0
    animation1.pattern1,overlay,-1,300,0,0
}
    surface201
{
}
";

/// [`RANDOM_ONLY`] の面 0 に [`TOP_ALWAYS`] と同じ `always`（animation 0）を足したもの。
const RANDOM_WITH_ALWAYS: &str = "surface0
{
animation0.interval,always
    animation0.pattern0,overlay,101,0,0,0
    animation0.pattern1,overlay,102,100,0,0
    animation0.pattern2,overlay,103,100,0,0
    animation1.interval,random,2
    animation1.pattern0,overlay,201,0,0,0
    animation1.pattern1,overlay,-1,300,0,0
}
    surface101
{
}
surface102
{
}
surface103
{
}
surface201
{
}
";

/// `random` だけの表と、同じ面に `always` を足した表で、同じ乱数の列を与えると `random` の発火の
/// 時刻（欄の移り変わり）と乱数を引いた回数が同じ（要件 4.7）。
#[test]
fn adding_always_keeps_random_fire_times() {
    let draws = [1, 0, 1, 1, 0, 0, 1, 0];

    let run = |text: &str| {
        let (rng, probe) = counting_rng(&draws);
        let mut rt = LoopRuntime::new(cfg(table_of(text), rng));
        let mut states = ScopeStates::new(BindSet::default());
        rt.on_tick(0, &mut states);
        switch(&mut rt, &mut states, SurfaceTarget::Show(0));
        let mut seen = Vec::new();
        let mut now = 0;
        while now <= 9000 {
            rt.on_tick(now, &mut states);
            let cell = states
                .current_pattern(&scope(), Slot::Shell)
                .get(1)
                .map(|f| f.surface_id);
            if seen.last().map(|(_, c)| *c) != Some(cell) {
                seen.push((now, cell));
            }
            now += 50;
        }
        let calls = probe.lock().unwrap().calls;
        (seen, calls)
    };

    let (plain, plain_calls) = run(RANDOM_ONLY);
    let (mixed, mixed_calls) = run(RANDOM_WITH_ALWAYS);
    assert!(
        plain.iter().any(|(_, c)| *c == Some(201)),
        "前提: random が発火している: {plain:?}"
    );
    assert_eq!(mixed, plain, "random の発火の時刻が変わらない");
    assert_eq!(mixed_calls, plain_calls, "引いた乱数の数が同じ");
}

/// 回数つきの子を置く 3 から、置かない 2 へ、刻みの間に 3 へ戻ると、2 で見えなくなった時計は
/// 捨てられ、3 に戻った出来事の時刻から頭で始め直す（要件 2.3・2.7・design「PartClocks」）。
#[test]
fn finite_film_restarts_from_head_after_away_and_back_within_one_tick() {
    let (rng, _probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(cfg(film_table(), rng));
    let mut states = ScopeStates::new(BindSet::default());

    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(3));
    let once = |rt: &LoopRuntime| {
        rt.parts
            .clock_at(&scope(), Slot::Shell, PartKey::Film(ONCE), 0)
    };
    assert_eq!(
        once(&rt),
        Some(PartAnim::Playing { started_at_ms: 0 }),
        "見えた出来事の時刻（直前の刻み 0）で生まれる"
    );
    rt.on_tick(150, &mut states);
    assert_eq!(
        film_cell(states.current_pattern(&scope(), Slot::Shell), ONCE),
        Some(21)
    );

    switch(&mut rt, &mut states, SurfaceTarget::Show(2));
    assert_eq!(once(&rt), None, "見えなくなった回数つきの時計は捨てる");
    let cmd = switch(&mut rt, &mut states, SurfaceTarget::Show(3)).expect("3 へ");
    assert_eq!(film_cell(tests::pattern_of(&cmd), ONCE), None, "経過 0");
    assert_eq!(once(&rt), Some(PartAnim::Playing { started_at_ms: 150 }));

    rt.on_tick(200, &mut states);
    assert_eq!(
        film_cell(states.current_pattern(&scope(), Slot::Shell), ONCE),
        None,
        "経過 50＝頭のコマ（続きなら合計 1 回を終えて 21）"
    );
}

/// 一番上が隠れた（`\s[-1]`）ら、シェルの面の回数つきの時計を捨てる。終わりなしは残す（要件 2.3）。
#[test]
fn hiding_top_drops_finite_shell_clocks_only() {
    let (rng, _probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(cfg(film_table(), rng));
    let mut states = ScopeStates::new(BindSet::default());

    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(1));
    rt.on_tick(50, &mut states);
    let clock = |rt: &LoopRuntime, film| {
        rt.parts
            .clock_at(&scope(), Slot::Shell, PartKey::Film(film), 0)
    };
    assert!(clock(&rt, ONCE).is_some() && clock(&rt, ENDLESS).is_some());

    switch(&mut rt, &mut states, SurfaceTarget::Hide).expect("Hide");
    assert_eq!(clock(&rt, ONCE), None, "回数つきは捨てる");
    assert_eq!(
        clock(&rt, ENDLESS),
        Some(PartAnim::Playing { started_at_ms: 0 }),
        "終わりなしは残す"
    );
}

/// 表の差し替えは、その面の種類の時計だけを捨てる（バルーンの表ならバルーンの面・シェルの表なら
/// シェルの面・要件 3.6）。
#[test]
fn table_replacement_drops_clocks_of_that_face_kind_only() {
    let (mut rng, _probe) = counting_rng(&[]);
    let (loop_rng, _p) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: film_table(),
        balloon_tables: BTreeMap::from([(scope(), film_table())]),
        rng: loop_rng,
    });
    let mut states = ScopeStates::new(BindSet::default());

    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(1));
    // バルーンの面の部品の時計（進行は task 3.6・ここでは入れ物を直に満たす）。
    rt.parts.advance(
        &scope(),
        Slot::Balloon,
        1,
        &BindSet::default(),
        &film_table(),
        0,
        false,
        &mut rng,
        &mut PatternState::default(),
    );
    let clock =
        |rt: &LoopRuntime, slot| rt.parts.clock_at(&scope(), slot, PartKey::Film(ENDLESS), 0);
    assert!(clock(&rt, Slot::Shell).is_some() && clock(&rt, Slot::Balloon).is_some());

    rt.replace_balloon_tables(BTreeMap::from([(scope(), film_table())]));
    assert_eq!(
        clock(&rt, Slot::Balloon),
        None,
        "バルーンの面の時計を捨てる"
    );
    assert!(clock(&rt, Slot::Shell).is_some(), "シェルの面の時計は残す");

    rt.replace_shell_table(film_table());
    assert_eq!(clock(&rt, Slot::Shell), None, "シェルの面の時計を捨てる");
}

/// 出来事の時刻は刻みの単調性の番人に入れない: 刻み（480）より後の出来事の時刻（500）で時計を
/// 作っても、次の刻み 480 は捨てられず、経過は 0 で止まる（design「LoopRuntime」）。
#[test]
fn event_time_is_not_fed_into_tick_guard() {
    let (rng, _probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(cfg(table_of(TOP_ALWAYS), rng));
    let mut states = ScopeStates::new(BindSet::default());

    rt.on_tick(400, &mut states);
    states.apply(&scope(), SurfaceTarget::Show(0));
    rt.on_surface_changed(&scope(), Slot::Shell);
    rt.refresh(&scope(), Slot::Shell, Some(500), &mut states);
    assert_eq!(rt.last_seen, Some(400), "出来事の時刻は番人に入れない");

    assert!(rt.on_tick(480, &mut states).is_empty());
    assert_eq!(rt.last_seen, Some(480), "刻み 480 は捨てられない");
    assert_eq!(top_cell(&states), None, "経過は 0 で止まる");
    assert_eq!(rt.on_tick(600, &mut states).len(), 1);
    assert_eq!(top_cell(&states), Some(102), "出来事の時刻 500 から 100ms");
}

/// 一番上の `always` の再生は、出来事の時刻が分からない（刻みの前に表示した・表を差し替えた）ときは
/// 次の刻みの時刻で生まれる（要件 4.1）。
#[test]
fn top_always_is_born_on_the_tick_when_no_event_time_is_known() {
    let (rng, _probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(cfg(table_of(TOP_ALWAYS), rng));
    let mut states = ScopeStates::new(BindSet::default());

    // 刻みの前に表示: 出来事の時刻が無いので再生は生まれない。
    switch(&mut rt, &mut states, SurfaceTarget::Show(0));
    assert!(rt.playback.is_empty(), "刻みの前は再生が無い");
    rt.on_tick(1000, &mut states);
    assert_eq!(top_cell(&states), None, "刻み 1000 で生まれる（経過 0）");
    rt.on_tick(1100, &mut states);
    assert_eq!(top_cell(&states), Some(102), "1000 から 100ms");

    // 表の差し替え: 再生を捨て、次の刻み 1150 で生まれ直す。
    states.rebase_shell(BindSet::default());
    rt.replace_shell_table(table_of(TOP_ALWAYS));
    assert!(rt.playback.is_empty(), "差し替えで再生を捨てる");
    rt.on_tick(1150, &mut states);
    assert_eq!(top_cell(&states), None, "刻み 1150 で生まれる（経過 0）");
    rt.on_tick(1250, &mut states);
    assert_eq!(top_cell(&states), Some(102), "1150 から 100ms");
}

/// 境界の抽選の時点で `always` の再生がまだ無くても（境界を跨ぐ刻みの直前に表を差し替えた）、
/// `always` は乱数を引かない: `random` だけの表と、引いた乱数の数と `random` の発火の時刻が同じ
/// （要件 4.7）。
#[test]
fn always_draws_no_rng_even_when_unborn_at_the_boundary() {
    let draws = [1, 0, 1, 0, 0, 1, 0, 1];
    let run = |text: &str| {
        let (rng, probe) = counting_rng(&draws);
        let mut rt = LoopRuntime::new(cfg(table_of(text), rng));
        let mut states = ScopeStates::new(BindSet::default());
        rt.on_tick(0, &mut states);
        switch(&mut rt, &mut states, SurfaceTarget::Show(0));
        let mut seen = Vec::new();
        let mut now = 50;
        while now <= 9000 {
            if now % 1000 == 0 {
                // 境界を跨ぐ刻みの直前に差し替える＝抽選の時点で `always` の再生が無い。
                states.rebase_shell(BindSet::default());
                rt.replace_shell_table(table_of(text));
            }
            rt.on_tick(now, &mut states);
            let cell = states
                .current_pattern(&scope(), Slot::Shell)
                .get(1)
                .map(|f| f.surface_id);
            if seen.last().map(|(_, c)| *c) != Some(cell) {
                seen.push((now, cell));
            }
            now += 50;
        }
        let calls = probe.lock().unwrap().calls;
        (seen, calls)
    };

    let (plain, plain_calls) = run(RANDOM_ONLY);
    let (mixed, mixed_calls) = run(RANDOM_WITH_ALWAYS);
    assert!(
        plain.iter().any(|(_, c)| *c == Some(201)),
        "前提: random が発火している: {plain:?}"
    );
    assert_eq!(plain_calls, 9, "前提: 境界 1000..9000 で 1 回ずつ引く");
    assert_eq!(mixed_calls, plain_calls, "always は乱数を引かない");
    assert_eq!(mixed, plain, "random の発火の時刻が変わらない");
}

/// 一番上の `always` の途中の `-1` は「消えている」になり、再生は捨てずに周の頭へ戻って繰り返す
/// （要件 4.2・4.5）。101（待ち 0＝経過 0）→ 100ms で `-1` → 100ms で 103（周期 200・103 は
/// 0 ミリ秒しか出ない）。
#[test]
fn top_always_blanks_on_negative_frame_and_keeps_looping() {
    let table = table_of(
        "surface0
{
animation0.interval,always
         animation0.pattern0,overlay,101,0,0,0
         animation0.pattern1,overlay,-1,100,0,0
         animation0.pattern2,overlay,103,100,0,0
}
         surface101
{
}
surface103
{
}
",
    );
    let (rng, _probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let mut states = ScopeStates::new(BindSet::default());
    let cell =
        |states: &ScopeStates| match states.current_pattern(&scope(), Slot::Shell).cell(None, 0) {
            Cell::Rest => "rest",
            Cell::Blank => "blank",
            other => panic!("一番上の欄に {other:?}"),
        };
    let playing = |rt: &LoopRuntime| {
        rt.playback
            .get(&(scope(), Slot::Shell))
            .is_some_and(|pb| pb.contains_key(&0))
    };

    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(0));
    for (now, expect) in [
        (50, "rest"),
        (100, "blank"),
        (150, "blank"),
        (200, "rest"),
        (250, "rest"),
        (300, "blank"),
        (400, "rest"),
    ] {
        rt.on_tick(now, &mut states);
        assert_eq!(cell(&states), expect, "{now}");
        assert!(playing(&rt), "{now}: 末尾でも負の番号でも再生は捨てない");
        assert_eq!(
            rt.playback[&(scope(), Slot::Shell)][&0].started_at_ms,
            0,
            "{now}: 始め直さない"
        );
    }
}
