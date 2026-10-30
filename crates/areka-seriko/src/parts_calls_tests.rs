//! 入れ子の部品の刻みと合図で働きのコマを当てる結合のテスト（spec: seriko-pattern-calls 要件 3.8・
//! 11.2・9.1）。部品の中の `start` と `stop` が一番上と同じ規則で働く。
//!
//! 一番上 0 が element定義で部品 100 を置き、1 は何も置かない。刻みは `on_tick(now_ms)`、乱数は
//! 注入列で、どちらも決定論。

use areka_emo_compose::{BindSet, EmoWorld};
use areka_sakura::ActorKey;

use crate::looper::LoopRuntime;
use crate::looper::tests::{cfg, counting_rng};
use crate::resolve::SurfaceTarget;
use crate::state::{ScopeStates, Slot};
use crate::table::AnimationTable;
use crate::timeline::LoopRng;

/// 部品の中の絵の面（中身は空）。
const SURFACES: &str = "surface1\n{\n}\nsurface101\n{\n}\nsurface102\n{\n}\n\
    surface110\n{\n}\nsurface111\n{\n}\nsurface120\n{\n}\n";

/// 面 0 が部品 100 を置く。`anims` は部品 100 の animation の行。
fn shell(anims: &str) -> AnimationTable {
    let text = format!(
        "surface0\n{{\nelement0,overlay,100,0,0\n}}\nsurface100\n{{\n{anims}}}\n{SURFACES}"
    );
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(&text)))
}

fn scope() -> ActorKey {
    ActorKey::from("0")
}

/// シェルの面 0 を出した状態のランタイム（選びの乱数は `pick` を返し続ける）。
fn shown(anims: &str, lottery: LoopRng, pick: u32) -> (LoopRuntime, ScopeStates) {
    let mut config = cfg(shell(anims), lottery);
    config.pick_rng = Box::new(move |_bound| pick);
    let mut states = ScopeStates::new(crate::DefaultBinds::uniform(BindSet::default()));
    states.apply(&scope(), SurfaceTarget::Show(0));
    (LoopRuntime::new(config), states)
}

/// 部品 100 の animation `id` の欄の絵（載っていなければ `None`）。
fn part_cell(states: &ScopeStates, id: u32) -> Option<u32> {
    states
        .current_pattern(&scope(), Slot::Shell)
        .part(100)
        .find(|&(a, _)| a == id)
        .map(|(_, f)| f.surface_id)
}

/// 面を `sid` へ切り替える（アクターと同じ順・出来事の時刻は `at_ms`）。
fn switch(rt: &mut LoopRuntime, states: &mut ScopeStates, sid: u32, at_ms: u64) {
    states.apply(&scope(), SurfaceTarget::Show(sid));
    rt.on_surface_changed(&scope(), Slot::Shell);
    rt.refresh(&scope(), Slot::Shell, Some(at_ms), states);
}

/// 「．さくら」の形を部品の中に: 抽選で動く animation2 が `alternativestart,(0,1)` で呼ぶ。
const ALTERNATIVE: &str = "animation0.interval,never\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,200,0,0\n\
    animation0.pattern2,overlay,101,1000,0,0\n\
    animation1.interval,never\n\
    animation1.pattern0,overlay,110,0,0,0\n\
    animation1.pattern1,overlay,111,200,0,0\n\
    animation1.pattern2,overlay,110,1000,0,0\n\
    animation2.interval,random,2\n\
    animation2.pattern0,overlay,120,200,0,0\n\
    animation2.pattern1,alternativestart,(0,1)\n";

/// 部品の中の `alternativestart` も、選んだ相手を呼び出しの予定の時刻に頭から始め、相手の今のコマを
/// 同じ刻みに載せる。呼んだ側は働きのコマが今のコマでも前の描くコマを残す（3.1・3.3・3.8・11.2）。
#[test]
fn alternativestart_inside_a_part_starts_the_pick_at_call_time() {
    for (pick, target, other) in [(0, 0, 1), (1, 1, 0)] {
        let (lottery, probe) = counting_rng(&[0]);
        let (mut rt, mut states) = shown(ALTERNATIVE, lottery, pick);
        rt.on_tick(0, &mut states);
        rt.on_tick(1000, &mut states); // animation2 が 1000 に始まる。呼び出しは 1200。
        assert_eq!(part_cell(&states, target), None);

        rt.on_tick(1500, &mut states); // 遅れた刻み: 相手は 1200 に始まり、経過 300 で 2 枚目。
        assert_eq!(part_cell(&states, target), Some(102 + 9 * target));
        assert_eq!(part_cell(&states, other), None);
        assert_eq!(part_cell(&states, 2), Some(120), "呼んだ側は前の絵を残す");
        assert_eq!(probe.lock().unwrap().calls, 1, "選びは抽選の乱数を引かない");

        rt.on_tick(2500, &mut states); // 相手の経過 1300: 3 枚目。
        assert_eq!(part_cell(&states, target), Some(101 + 9 * target));
    }
}

/// 部品の中の `stop` の見え方は `-1` と同じ（相手の欄と時計が外れる・呼んだ側は残る・4.1・3.8）。
#[test]
fn stop_inside_a_part_looks_the_same_as_minus_one() {
    const STOP: &str = "animation0.interval,random,2\n\
        animation0.pattern0,overlay,101,0,0,0\n\
        animation0.pattern1,overlay,102,5000,0,0\n\
        animation1.interval,random,2\n\
        animation1.pattern0,overlay,120,300,0,0\n\
        animation1.pattern1,stop,0\n";
    const MINUS_ONE: &str = "animation0.interval,random,2\n\
        animation0.pattern0,overlay,101,0,0,0\n\
        animation0.pattern1,overlay,-1,300,0,0\n\
        animation1.interval,random,2\n\
        animation1.pattern0,overlay,120,300,0,0\n";
    let run = |anims: &str| {
        let (lottery, _probe) = counting_rng(&[0, 0]);
        let (mut rt, mut states) = shown(anims, lottery, 0);
        [0, 1000, 1200, 1300, 1400]
            .map(|t| {
                let cmds = rt.on_tick(t, &mut states);
                (cmds, states.current_pattern(&scope(), Slot::Shell).clone())
            })
            .to_vec()
    };
    let stop = run(STOP);
    assert_eq!(part_cell_of(&stop[2].1, 0), Some(101), "1200 は止める前");
    assert_eq!(part_cell_of(&stop[3].1, 0), None, "1300 で止まる");
    assert_eq!(part_cell_of(&stop[3].1, 1), Some(120), "呼んだ側は残る");
    assert_eq!(stop, run(MINUS_ONE));
}

fn part_cell_of(pattern: &areka_emo_compose::PatternState, id: u32) -> Option<u32> {
    pattern
        .part(100)
        .find(|&(a, _)| a == id)
        .map(|(_, f)| f.surface_id)
}

/// 部品の中で止めた `always` も、面を出すまで始め直さない。面を出したら印が消え、次の刻みの時刻で
/// 頭から始まる（4.5・3.8・M2）。
#[test]
fn stopped_always_inside_a_part_stays_stopped_until_the_surface_is_shown() {
    const STOP_ALWAYS: &str = "animation0.interval,always\n\
        animation0.pattern0,overlay,101,100,0,0\n\
        animation0.pattern1,overlay,102,100,0,0\n\
        animation1.interval,random,2\n\
        animation1.pattern0,overlay,120,100,0,0\n\
        animation1.pattern1,stop,0\n";
    let (lottery, _probe) = counting_rng(&[0]);
    let (mut rt, mut states) = shown(STOP_ALWAYS, lottery, 0);
    rt.on_tick(0, &mut states);
    rt.on_tick(950, &mut states);
    assert_eq!(part_cell(&states, 0), Some(101), "always は 0 から回る");
    rt.on_tick(1000, &mut states); // animation1 が始まる。止めは 1100。
    rt.on_tick(1100, &mut states);
    assert_eq!(part_cell(&states, 0), None);
    for t in [1150, 1250, 2500, 3000] {
        rt.on_tick(t, &mut states);
        assert_eq!(part_cell(&states, 0), None, "{t} でも止まったまま");
    }
    assert!(
        rt.on_surface_shown(&scope(), Slot::Shell),
        "部品の印を消した"
    );
    assert!(
        !rt.on_surface_shown(&scope(), Slot::Shell),
        "消す印はもう無い"
    );
    rt.on_tick(3050, &mut states); // 3050 に頭から: 最初の待ちの前は何も出さない。
    assert_eq!(part_cell(&states, 0), None);
    rt.on_tick(3160, &mut states);
    assert_eq!(part_cell(&states, 0), Some(101), "3050 から 110 ms");
}

/// 合図の時刻の更新でも、頭のコマを置く前に部品の働きのコマを当てる（呼ばれた相手の頭のコマが
/// その評価の絵に載る・3.8）。
#[test]
fn refresh_applies_part_calls_at_the_event_time() {
    const CALLER: &str = "animation0.interval,always\n\
        animation0.pattern0,start,1\n\
        animation0.pattern1,overlay,120,100,0,0\n\
        animation1.interval,never\n\
        animation1.pattern0,overlay,110,0,0,0\n\
        animation1.pattern1,overlay,111,500,0,0\n";
    let (lottery, _probe) = counting_rng(&[]);
    let (mut rt, mut states) = shown(CALLER, lottery, 0);
    rt.refresh(&scope(), Slot::Shell, Some(700), &mut states);
    assert_eq!(part_cell(&states, 1), Some(110), "呼ばれた相手の頭のコマ");
    assert_eq!(
        part_cell(&states, 0),
        None,
        "1 周目の頭が働きのコマなら欄を外す"
    );
    // 周ごとの呼び出しで相手は 800・…・1200 に頭からやり直す。呼んだ側は周の頭（前の周の 120）。
    rt.on_tick(1250, &mut states);
    assert_eq!(part_cell(&states, 1), Some(110));
    assert_eq!(part_cell(&states, 0), Some(120));
}

/// 呼ばれるだけ（`never`）の部品の時計は、部品が見えない間も捨てず（抽選で始まった再生と同じ・
/// 見えなかった間も進んでいた扱い）、また見えた刻みに開始からの経過のコマを出す（11.2・3.5）。
#[test]
fn called_never_clock_survives_while_the_part_is_hidden() {
    const CALLED: &str = "animation0.interval,never\n\
        animation0.pattern0,overlay,101,0,0,0\n\
        animation0.pattern1,overlay,102,1000,0,0\n\
        animation0.pattern2,overlay,101,5000,0,0\n\
        animation2.interval,random,2\n\
        animation2.pattern0,overlay,120,200,0,0\n\
        animation2.pattern1,start,0\n";
    let (lottery, _probe) = counting_rng(&[0]);
    let (mut rt, mut states) = shown(CALLED, lottery, 0);
    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states);
    rt.on_tick(1300, &mut states); // 1200 に animation0 が始まる。
    assert_eq!(part_cell(&states, 0), Some(101));
    switch(&mut rt, &mut states, 1, 1400); // 部品 100 は見えない。
    rt.on_tick(1500, &mut states);
    assert_eq!(part_cell(&states, 0), None);
    switch(&mut rt, &mut states, 0, 2500); // また見える: 1200 からの経過 1300 で 2 枚目。
    assert_eq!(part_cell(&states, 0), Some(102));
}
