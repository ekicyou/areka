//! 一番上の面の `runonce`・`periodic` の檻（spec: areka-P0-seriko-trigger-intervals 要件 2・3・5.1〜5.3・
//! 5.5・5.7・5.8・6.1・6.2・7.5・8.1・8.2・9.2・9.3・9.6・tasks.md 4.1）。
//!
//! 面の切り替えはアクターと同じ順（`apply` → `on_surface_changed` → `refresh`）で踏む。刻みは
//! `on_tick(now_ms)`、出来事の時刻は `refresh` の `at_ms`、乱数は注入列で、どれも決定論。時刻は
//! 観測の後にだけ進める。

use std::collections::BTreeMap;

use areka_emo_compose::BindSet;

use super::test_support::{
    capture_logs, cell, count, scope, shell_runtime, started, switch, table_of,
};
use super::tests::{always_fire, cfg, counting_rng, pattern_of};
use super::*;
use crate::resolve::SurfaceTarget;
use crate::state::{BindApplyOutcome, StageNote};

/// 面 0 の一番上: `runonce`（animation 0＝301 → 100ms で 302 → 100ms で `-1`・全部で 200ms）と
/// `periodic,1`（animation 1＝401 → 100ms で 402 → 300ms で `-1`・全部で 400ms）。面 1 は何も持たない。
const TRIGGERS: &str = "surface0\n{\n\
    animation0.interval,runonce\n\
    animation0.pattern0,overlay,301,0,0,0\n\
    animation0.pattern1,overlay,302,100,0,0\n\
    animation0.pattern2,overlay,-1,100,0,0\n\
    animation1.interval,periodic,1\n\
    animation1.pattern0,overlay,401,0,0,0\n\
    animation1.pattern1,overlay,402,100,0,0\n\
    animation1.pattern2,overlay,-1,300,0,0\n}\n\
    surface1\n{\n}\n\
    surface301\n{\n}\nsurface302\n{\n}\nsurface401\n{\n}\nsurface402\n{\n}\n";

/// 起動後の最初の表示で `runonce` が 1 回だけ鳴る。開始は出来事の時刻そのもので、切り替えの指令が
/// 頭のコマを載せ、遅れた刻みは過ぎた分だけ進み、`-1` で消して終え、その後は鳴らない（要件 2.1・
/// 2.2・5.1・5.2・6.1・6.2）。境界を何度跨いでも乱数を引かない（要件 5.7）。
#[test]
fn runonce_plays_once_from_the_event_time_on_first_show() {
    let (rng, probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(cfg(table_of(TRIGGERS), rng));
    let mut states = ScopeStates::new(BindSet::default());
    assert!(rt.on_tick(1000, &mut states).is_empty());

    let cmd = switch(&mut rt, &mut states, SurfaceTarget::Show(0), 1030).expect("Show");
    assert_eq!(
        pattern_of(&cmd).get(0).map(|f| f.surface_id),
        Some(301),
        "切り替えの指令が頭のコマを載せる（次の刻みを待たない）"
    );
    assert_eq!(
        started(&rt, Slot::Shell, 0),
        Some(1030),
        "開始は出来事の時刻（刻みの境目に丸めない）"
    );

    assert!(
        rt.on_tick(1100, &mut states).is_empty(),
        "経過 70＝頭のコマのまま"
    );
    assert_eq!(rt.on_tick(1140, &mut states).len(), 1);
    assert_eq!(
        cell(&states, Slot::Shell, 0),
        Some(302),
        "経過 110 で 2 枚目"
    );
    rt.on_tick(1230, &mut states);
    assert_eq!(cell(&states, Slot::Shell, 0), None, "`-1` で消す");
    assert_eq!(started(&rt, Slot::Shell, 0), None, "`-1` で終える");

    let mut now = 1250;
    while now <= 4000 {
        rt.on_tick(now, &mut states);
        assert_eq!(
            started(&rt, Slot::Shell, 0),
            None,
            "{now}: 2 回目は鳴らない"
        );
        assert_eq!(cell(&states, Slot::Shell, 0), None, "{now}");
        now += 50;
    }
    assert_eq!(probe.lock().unwrap().calls, 0, "3 語は乱数を引かない");
}

/// 同じ面の再指定と、着せ替えだけの変化では `runonce` は鳴らない（要件 2.3・2.4）。
#[test]
fn runonce_stays_silent_on_same_surface_and_dress_change() {
    let (mut rt, mut states) = shell_runtime(TRIGGERS);
    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 10).expect("Show");
    assert_eq!(started(&rt, Slot::Shell, 0), Some(10), "前提: 最初は鳴る");
    rt.on_tick(300, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 0), None, "前提: 鳴り終えた");

    assert!(
        switch(&mut rt, &mut states, SurfaceTarget::Show(0), 320).is_none(),
        "同じ面の再指定は切り替わりでない"
    );
    rt.on_tick(330, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 0), None, "再指定では鳴らない");

    // 着せ替えだけ: アクターは面の切り替えの知らせ無しで評価だけを呼ぶ。
    assert!(matches!(
        states.apply_bind(&scope(), 7, true),
        BindApplyOutcome::Changed(_)
    ));
    rt.refresh(&scope(), Slot::Shell, Some(340), &mut states);
    assert_eq!(started(&rt, Slot::Shell, 0), None, "着せ替えでは鳴らない");
    rt.on_tick(350, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 0), None);
    assert_eq!(cell(&states, Slot::Shell, 0), None);
}

/// 別の面から戻ったとき・非表示から戻ったときは、戻った出来事の時刻でもう 1 回鳴る。3 語の無い面と
/// 非表示の間は引き金の状態を持たない（要件 2.5・2.6・8.1）。
#[test]
fn runonce_replays_on_return_and_after_hidden() {
    let (mut rt, mut states) = shell_runtime(TRIGGERS);
    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 10).expect("Show");
    assert_eq!(started(&rt, Slot::Shell, 0), Some(10));
    rt.on_tick(300, &mut states);

    switch(&mut rt, &mut states, SurfaceTarget::Show(1), 400).expect("1 へ");
    rt.on_tick(420, &mut states);
    assert!(rt.armed.is_empty(), "3 語の無い面では構えない");
    let cmd = switch(&mut rt, &mut states, SurfaceTarget::Show(0), 450).expect("0 へ");
    assert_eq!(pattern_of(&cmd).get(0).map(|f| f.surface_id), Some(301));
    assert_eq!(started(&rt, Slot::Shell, 0), Some(450), "戻ったら鳴る");
    rt.on_tick(700, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 0), None);

    switch(&mut rt, &mut states, SurfaceTarget::Hide, 800).expect("Hide");
    rt.on_tick(820, &mut states);
    assert!(rt.armed.is_empty(), "非表示の間は構えない");
    let cmd = switch(&mut rt, &mut states, SurfaceTarget::Show(0), 900).expect("0 へ");
    assert_eq!(pattern_of(&cmd).get(0).map(|f| f.surface_id), Some(301));
    assert_eq!(
        started(&rt, Slot::Shell, 0),
        Some(900),
        "非表示から戻ったら鳴る"
    );
}

/// 出来事の時刻が分からない（時計も刻みもまだ無い）ときは構えず、次の刻みの時刻で鳴る。
#[test]
fn runonce_waits_for_the_first_tick_when_no_time_is_known() {
    let (mut rt, mut states) = shell_runtime(TRIGGERS);
    states.apply(&scope(), SurfaceTarget::Show(0));
    rt.on_surface_changed(&scope(), Slot::Shell);
    rt.refresh(&scope(), Slot::Shell, None, &mut states);
    assert!(rt.armed.is_empty(), "時刻が無ければ構えない");

    let cmds = rt.on_tick(1000, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 0), Some(1000), "刻みの時刻で鳴る");
    assert_eq!(cmds.len(), 1, "同じ刻みで頭のコマを出す");
    assert_eq!(cell(&states, Slot::Shell, 0), Some(301));
}

/// `periodic` は切り替わった瞬間には鳴らず、出来事の時刻を起点に数値秒ちょうどの境目で鳴る。開始は
/// 境目そのもので、遅れた刻みは過ぎた分だけ進んだコマを同じ刻みで出す（要件 3.1・3.5・6.1・6.2）。
#[test]
fn periodic_fires_at_exact_laps_from_the_event_time() {
    let (mut rt, mut states) = shell_runtime(TRIGGERS);
    rt.on_tick(0, &mut states);
    let cmd = switch(&mut rt, &mut states, SurfaceTarget::Show(0), 30).expect("Show");
    assert!(
        pattern_of(&cmd).get(1).is_none(),
        "切り替わった瞬間には鳴らない"
    );
    rt.on_tick(1000, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 1), None, "起点 30 から 970ms");

    rt.on_tick(1150, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        Some(1030),
        "開始は起点＋1000 そのもの（刻みの時刻ではない）"
    );
    assert_eq!(
        cell(&states, Slot::Shell, 1),
        Some(402),
        "遅れた 120ms の分だけ進んだコマ"
    );
    rt.on_tick(1500, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 1), None, "1 回流して終える");
    assert_eq!(cell(&states, Slot::Shell, 1), None);

    rt.on_tick(2040, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 1), Some(2030), "2 周目の境目");
    assert_eq!(cell(&states, Slot::Shell, 1), Some(401));
}

/// 面を離れたら定期の再生は止まり、戻った出来事の時刻が新しい起点になる（要件 3.2）。
#[test]
fn periodic_stops_on_leaving_and_restarts_from_the_return_time() {
    let (mut rt, mut states) = shell_runtime(TRIGGERS);
    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 30).expect("Show");
    switch(&mut rt, &mut states, SurfaceTarget::Show(1), 500).expect("1 へ");
    assert!(
        rt.on_tick(1100, &mut states).is_empty(),
        "離れた面の境目（1030）では鳴らない"
    );
    assert!(rt.playback.is_empty());

    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 1200).expect("0 へ");
    rt.on_tick(2100, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        None,
        "前の起点 30 の境目（2030）では鳴らない"
    );
    rt.on_tick(2210, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        Some(2200),
        "戻った時刻 1200 が新しい起点"
    );
}

/// 面 0 の一番上: `periodic,1` で、周期より長い再生（401 → 1500ms で 402＝末尾に残る）。
const LONG_PERIODIC: &str = "surface0\n{\n\
    animation1.interval,periodic,1\n\
    animation1.pattern0,overlay,401,0,0,0\n\
    animation1.pattern1,overlay,402,1500,0,0\n}\n\
    surface401\n{\n}\nsurface402\n{\n}\n";

/// 境目が来たときに同じ animation がまだ再生中なら、その回は始めず（コマを乱さず）、飛ばした回が
/// 後から鳴ることも無い。次の境目で鳴る（要件 3.3・5.3）。
#[test]
fn periodic_skips_a_lap_while_still_playing() {
    let (mut rt, mut states) = shell_runtime(LONG_PERIODIC);
    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 30).expect("Show");
    rt.on_tick(1040, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 1), Some(1030));

    rt.on_tick(2040, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        Some(1030),
        "再生中の境目（2030）では始め直さない"
    );
    assert_eq!(cell(&states, Slot::Shell, 1), Some(401), "コマを乱さない");

    rt.on_tick(2540, &mut states);
    assert_eq!(cell(&states, Slot::Shell, 1), Some(402), "末尾のコマ");
    assert_eq!(started(&rt, Slot::Shell, 1), None, "末尾で終える");
    rt.on_tick(2600, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        None,
        "飛ばした回は後から鳴らない"
    );

    rt.on_tick(3040, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 1), Some(3030), "次の境目で鳴る");
}

/// 面 0 の一番上: `periodic,1` で、長さが周期ちょうどの再生（401 → 400ms で 402 → 600ms で `-1`・
/// 全部で 1000ms）。
const EXACT_PERIODIC_STOP: &str = "surface0\n{\n\
    animation1.interval,periodic,1\n\
    animation1.pattern0,overlay,401,0,0,0\n\
    animation1.pattern1,overlay,402,400,0,0\n\
    animation1.pattern2,overlay,-1,600,0,0\n}\n\
    surface401\n{\n}\nsurface402\n{\n}\n";

/// [`EXACT_PERIODIC_STOP`] と同じ長さで、`-1` の代わりに末尾のコマ（403）が残る再生。
const EXACT_PERIODIC_RESIDUAL: &str = "surface0\n{\n\
    animation1.interval,periodic,1\n\
    animation1.pattern0,overlay,401,0,0,0\n\
    animation1.pattern1,overlay,402,400,0,0\n\
    animation1.pattern2,overlay,403,600,0,0\n}\n\
    surface401\n{\n}\nsurface402\n{\n}\nsurface403\n{\n}\n";

/// 長さが周期ちょうどの再生は、終わりと次の境目が同じ時刻になる。境目の時刻にはもう再生中でないので
/// 毎周鳴る（1 周おきにならない）。終えたのにまだ片付けていなかった前の再生は、終わりの記録を 1 件
/// 残して入れ替わり、絵は新しい再生の経過のコマになる（要件 3.1・3.3・3.5・6.1・6.2）。
#[test]
fn periodic_as_long_as_its_period_fires_at_every_lap() {
    for (text, end) in [
        (EXACT_PERIODIC_STOP, "seriko: loop 停止"),
        (EXACT_PERIODIC_RESIDUAL, "seriko: loop 末尾残留"),
    ] {
        let (mut rt, mut states) = shell_runtime(text);
        let logs = capture_logs(|| {
            rt.on_tick(0, &mut states);
            switch(&mut rt, &mut states, SurfaceTarget::Show(0), 30);
            // （刻みの時刻, 開始の時刻, 欄の絵）: 境目を過ぎた刻み・周の途中の刻み・境目ちょうどの刻み
            // （3030）・前の再生の終わり＝境目をまたいだ遅い刻み（4500）。
            for (now, start, shown) in [
                (1040, 1030, 401),
                (1500, 1030, 402),
                (2040, 2030, 401),
                (2500, 2030, 402),
                (3030, 3030, 401),
                (4500, 4030, 402),
            ] {
                rt.on_tick(now, &mut states);
                assert_eq!(started(&rt, Slot::Shell, 1), Some(start), "{now}");
                assert_eq!(cell(&states, Slot::Shell, 1), Some(shown), "{now}");
            }
        });
        assert_eq!(
            count(&logs, "seriko: trigger periodic を鳴らした"),
            4,
            "{logs:?}"
        );
        assert_eq!(
            count(&logs, end),
            3,
            "入れ替わった再生 1 本につき終わりの記録 1 件: {logs:?}"
        );
    }
}

/// 境目の時刻にまだ再生中だった周は、その境目を見る刻みが再生の終わりより後に来ても飛ばす
/// （「再生中か」は刻みの時刻でなく境目の時刻で測る・要件 3.3・3.5）。
#[test]
fn periodic_skips_a_lap_playing_at_the_boundary_even_when_seen_by_a_late_tick() {
    let (mut rt, mut states) = shell_runtime(LONG_PERIODIC);
    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 30).expect("Show");
    rt.on_tick(1040, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 1), Some(1030));

    // 再生は 2530 で終わる。境目 2030 を見る刻みは、その後の 2600。
    rt.on_tick(2600, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        None,
        "境目 2030 では再生中だった＝始めない"
    );
    assert_eq!(cell(&states, Slot::Shell, 1), Some(402), "末尾のコマが残る");
    rt.on_tick(2700, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        None,
        "飛ばした周は後から鳴らない"
    );

    rt.on_tick(3040, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 1), Some(3030), "次の境目で鳴る");
}

/// 前の再生の終わりと次の境目の両方をまたいだ刻み（終わり < 境目 < 刻み）は、境目で 1 回だけ鳴らす。
/// 前の再生の終わりの記録は 1 件（要件 3.4・3.5・6.2）。
#[test]
fn periodic_fires_at_the_boundary_when_a_late_tick_spans_the_previous_end() {
    let (mut rt, mut states) = shell_runtime(TRIGGERS);
    let logs = capture_logs(|| {
        rt.on_tick(0, &mut states);
        switch(&mut rt, &mut states, SurfaceTarget::Show(0), 30);
        rt.on_tick(1040, &mut states);
        assert_eq!(started(&rt, Slot::Shell, 1), Some(1030));

        // 再生は 1430 で終わり、次の境目は 2030。その間に刻みは来ない。
        rt.on_tick(2100, &mut states);
        assert_eq!(started(&rt, Slot::Shell, 1), Some(2030), "境目から");
        assert_eq!(cell(&states, Slot::Shell, 1), Some(401), "経過 70");
        rt.on_tick(2140, &mut states);
        assert_eq!(cell(&states, Slot::Shell, 1), Some(402), "経過 110");
    });
    let periodic: Vec<String> = logs
        .into_iter()
        .filter(|l| l.contains("animation_id=1"))
        .collect();
    assert_eq!(
        count(&periodic, "seriko: trigger periodic を鳴らした"),
        2,
        "{periodic:?}"
    );
    assert_eq!(count(&periodic, "seriko: loop 停止"), 1, "{periodic:?}");
}

/// 1 回の刻みで境目を 2 つまたいでも、始めるのは 1 回だけ（開始は最新の境目）で、またいだ数を
/// 積み上げない（要件 3.4）。
#[test]
fn periodic_fires_once_when_a_tick_spans_two_laps() {
    let (mut rt, mut states) = shell_runtime(TRIGGERS);
    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 30).expect("Show");

    rt.on_tick(2100, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        Some(2030),
        "境目 1030・2030 をまたいだ刻みは最新の境目から"
    );
    assert_eq!(cell(&states, Slot::Shell, 1), Some(401), "経過 70");
    rt.on_tick(2500, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 1), None, "1 回流して終える");
    rt.on_tick(2600, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        None,
        "またいだ数を積み上げない"
    );
    rt.on_tick(3040, &mut states);
    assert_eq!(started(&rt, Slot::Shell, 1), Some(3030));
}

/// 表の差し替えで引き金の状態を捨て、次の刻みの時刻で構え直す。
#[test]
fn table_replacement_drops_trigger_state_and_rearms_on_the_next_tick() {
    let (mut rt, mut states) = shell_runtime(TRIGGERS);
    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 10).expect("Show");
    rt.on_tick(300, &mut states);
    assert!(rt.armed.contains_key(&(scope(), Slot::Shell)), "前提");

    states.rebase_shell(BindSet::default());
    rt.replace_shell_table(table_of(TRIGGERS));
    assert!(rt.armed.is_empty(), "差し替えで捨てる");

    rt.on_tick(350, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 0),
        Some(350),
        "次の刻みで構え直す"
    );
    rt.on_tick(1360, &mut states);
    assert_eq!(
        started(&rt, Slot::Shell, 1),
        Some(1350),
        "構え直した時刻が起点"
    );
}

/// 面 0 の一番上に `always`（animation 0）と `random`（animation 1）だけ（3 語なし）。
const NO_TRIGGER_WORDS: &str = "surface0\n{\n\
    animation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation1.interval,random,2\n\
    animation1.pattern0,overlay,201,0,0,0\n\
    animation1.pattern1,overlay,-1,300,0,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface201\n{\n}\n";

/// 3 語の無い表では、`random` が再生中でも `always` が回っていても、引き金の状態も記録も生まれない
/// （要件 8.1・8.2）。
#[test]
fn tables_without_trigger_words_keep_no_trigger_state() {
    let mut rt = LoopRuntime::new(cfg(table_of(NO_TRIGGER_WORDS), always_fire()));
    let mut states = ScopeStates::new(BindSet::default());

    let logs = capture_logs(|| {
        rt.on_tick(0, &mut states);
        switch(&mut rt, &mut states, SurfaceTarget::Show(0), 10).expect("Show");
        let mut now = 50;
        while now <= 3000 {
            rt.on_tick(now, &mut states);
            assert!(rt.armed.is_empty(), "{now}: 引き金の状態は空のまま");
            now += 50;
        }
    });
    assert!(
        count(&logs, "seriko: loop 抽選発火") >= 1,
        "陽性対照: random は再生している: {logs:?}"
    );
    assert_eq!(count(&logs, "seriko: trigger"), 0, "{logs:?}");
}

/// 面 0 の一番上に `random`（animation 1）だけ。
const RANDOM_ONLY: &str = "surface0\n{\n\
    animation1.interval,random,2\n\
    animation1.pattern0,overlay,201,0,0,0\n\
    animation1.pattern1,overlay,-1,300,0,0\n}\n\
    surface201\n{\n}\n";

/// [`RANDOM_ONLY`] の面 0 に `runonce`（animation 0）と `periodic,1`（animation 2）を足したもの。
const RANDOM_WITH_TRIGGERS: &str = "surface0\n{\n\
    animation0.interval,runonce\n\
    animation0.pattern0,overlay,301,0,0,0\n\
    animation0.pattern1,overlay,-1,200,0,0\n\
    animation1.interval,random,2\n\
    animation1.pattern0,overlay,201,0,0,0\n\
    animation1.pattern1,overlay,-1,300,0,0\n\
    animation2.interval,periodic,1\n\
    animation2.pattern0,overlay,401,0,0,0\n\
    animation2.pattern1,overlay,-1,400,0,0\n}\n\
    surface201\n{\n}\nsurface301\n{\n}\nsurface401\n{\n}\n";

/// `random` だけの表と、同じ面に `runonce`・`periodic` を足した表で、同じ乱数の列を与えると `random` の
/// 発火の時刻と乱数を引いた回数が同じで、足した 2 本もそれぞれの決まりで鳴る（要件 5.7・5.8・8.2）。
#[test]
fn trigger_words_beside_random_do_not_interfere() {
    let draws = [1, 0, 1, 1, 0, 0, 1, 0];
    let run = |text: &str| {
        let (rng, probe) = counting_rng(&draws);
        let mut rt = LoopRuntime::new(cfg(table_of(text), rng));
        let mut states = ScopeStates::new(BindSet::default());
        rt.on_tick(0, &mut states);
        switch(&mut rt, &mut states, SurfaceTarget::Show(0), 0);
        let runonce = started(&rt, Slot::Shell, 0);
        let (mut random, mut periodic) = (Vec::new(), Vec::new());
        let mut now = 0;
        while now <= 9000 {
            rt.on_tick(now, &mut states);
            let seen = cell(&states, Slot::Shell, 1);
            if random.last().map(|(_, c)| *c) != Some(seen) {
                random.push((now, seen));
            }
            if let Some(at) = started(&rt, Slot::Shell, 2) {
                if periodic.last() != Some(&at) {
                    periodic.push(at);
                }
            }
            now += 50;
        }
        let calls = probe.lock().unwrap().calls;
        (random, calls, runonce, periodic)
    };

    let (plain, plain_calls, _, _) = run(RANDOM_ONLY);
    let (mixed, mixed_calls, runonce, periodic) = run(RANDOM_WITH_TRIGGERS);
    assert!(
        plain.iter().any(|(_, c)| *c == Some(201)),
        "前提: random が発火している: {plain:?}"
    );
    assert_eq!(mixed, plain, "random の発火の時刻が変わらない");
    assert_eq!(mixed_calls, plain_calls, "引いた乱数の数が同じ");
    assert_eq!(runonce, Some(0), "runonce は切り替えで鳴る");
    assert_eq!(
        periodic,
        (1..=9).map(|lap| lap * 1000).collect::<Vec<u64>>(),
        "periodic は 1 秒ごとに鳴る"
    );
}

/// 面 0 の一番上には何も無く、別の面 5 に `talk,3` が在る表。
const TALK_ELSEWHERE: &str = "surface0\n{\n}\n\
    surface5\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,501,0,0,0\n\
    animation0.pattern1,overlay,-1,100,0,0\n}\n\
    surface501\n{\n}\n";

/// 表に `talk` が在れば、一番上に 3 語の無い面でも構える（文字の数えの置き場）。その面では再生も
/// 刻みごとの一番上の判定も生まれない。
#[test]
fn talk_elsewhere_in_the_table_arms_without_top_judgement() {
    let (mut rt, mut states) = shell_runtime(TALK_ELSEWHERE);
    rt.on_tick(0, &mut states);
    switch(&mut rt, &mut states, SurfaceTarget::Show(0), 10).expect("Show");
    assert!(
        rt.armed.contains_key(&(scope(), Slot::Shell)),
        "表に talk が在るので構える"
    );
    let mut now = 50;
    while now <= 3000 {
        assert!(rt.on_tick(now, &mut states).is_empty(), "{now}");
        assert!(rt.playback.is_empty(), "{now}");
        now += 50;
    }
}

/// バルーンの窓が閉じている間は非表示（`periodic` は止まる）。開き直しで `runonce` は鳴らず、
/// `periodic` は開いた出来事の時刻が新しい起点になる（要件 3.2・5.5）。
#[test]
fn balloon_window_reopen_keeps_runonce_silent_and_restarts_periodic() {
    let (rng, _probe) = counting_rng(&[]);
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(scope(), table_of(TRIGGERS))]),
        rng,
    });
    let mut states = ScopeStates::new(BindSet::default());
    let note = |states: &mut ScopeStates, open: bool| {
        states.note_stage(&StageNote::Balloon {
            scope: scope(),
            open,
            face: 0,
            generation: 0,
        });
    };

    note(&mut states, true);
    rt.on_tick(1000, &mut states);
    assert_eq!(
        started(&rt, Slot::Balloon, 0),
        Some(1000),
        "前提: 開いている面で runonce が鳴る"
    );
    rt.on_tick(1300, &mut states);
    assert_eq!(started(&rt, Slot::Balloon, 0), None);

    // 閉じる（出来事 1500）: 起点 1000 の境目 2000・3000 では鳴らない。
    note(&mut states, false);
    rt.refresh(&scope(), Slot::Balloon, Some(1500), &mut states);
    rt.on_tick(2100, &mut states);
    rt.on_tick(3100, &mut states);
    assert_eq!(
        started(&rt, Slot::Balloon, 1),
        None,
        "閉じている間は鳴らない"
    );

    // 開き直す（出来事 3200）。
    note(&mut states, true);
    rt.refresh(&scope(), Slot::Balloon, Some(3200), &mut states);
    assert_eq!(
        started(&rt, Slot::Balloon, 0),
        None,
        "開き直しで runonce は鳴らない"
    );
    rt.on_tick(4100, &mut states);
    assert_eq!(
        started(&rt, Slot::Balloon, 1),
        None,
        "前の起点 1000 の境目（4000）では鳴らない"
    );
    rt.on_tick(4250, &mut states);
    assert_eq!(
        started(&rt, Slot::Balloon, 1),
        Some(4200),
        "開いた時刻 3200 が新しい起点"
    );
    assert_eq!(started(&rt, Slot::Balloon, 0), None);
}

/// 記録: 面に入ったら `debug!` を 1 件、`runonce`・`periodic` の開始は `info!`。始めない経路（境目の前・
/// 面を離れた・3 語の無い面）は何も記録しない（要件 7.5）。
#[test]
fn entering_and_starts_are_logged_and_discards_are_silent() {
    let (mut rt, mut states) = shell_runtime(TRIGGERS);
    let logs = capture_logs(|| {
        rt.on_tick(0, &mut states);
        switch(&mut rt, &mut states, SurfaceTarget::Show(0), 30);
        for now in [500, 1040, 1500, 2040, 2500] {
            rt.on_tick(now, &mut states);
        }
        switch(&mut rt, &mut states, SurfaceTarget::Show(1), 2600);
        rt.on_tick(2700, &mut states);
        rt.on_tick(3100, &mut states);
    });

    let line = |needle: &str| {
        let found: Vec<&String> = logs.iter().filter(|l| l.contains(needle)).collect();
        assert_eq!(found.len(), 1, "{needle}: {logs:?}");
        found[0].clone()
    };
    let entered = line("seriko: trigger 面に入った");
    assert!(
        entered.contains("level=DEBUG") && entered.contains("surface_id=0"),
        "{entered}"
    );
    let runonce = line("seriko: trigger runonce を鳴らした");
    assert!(
        runonce.contains("level=INFO") && runonce.contains("animation_id=0"),
        "{runonce}"
    );
    assert_eq!(
        count(&logs, "seriko: trigger periodic を鳴らした"),
        2,
        "{logs:?}"
    );
    assert!(
        logs.iter()
            .filter(|l| l.contains("seriko: trigger periodic を鳴らした"))
            .all(|l| l.contains("level=INFO") && l.contains("animation_id=1")),
        "{logs:?}"
    );
    assert_eq!(
        count(&logs, "seriko: trigger"),
        4,
        "始めない経路は記録しない: {logs:?}"
    );
    assert_eq!(count(&logs, "level=WARN"), 0, "{logs:?}");
}
