//! 働きのコマを時刻の順に当てる係の決定論テスト（spec: seriko-pattern-calls 要件 3.1–3.5・3.9・
//! 4.1–4.4・5.2・7.2・7.3・8.1 の M1・M2・9.1）。再生の表は偽の表、選びは決め打った乱数。

use super::*;
use crate::table::{LoopFrame, LoopTrigger};
use areka_emo_compose::ComposeMethod;
use std::collections::BTreeMap;
use std::num::{NonZeroU32, NonZeroU64};

/// 偽の再生の表（番号 → (開始, 当て終えた時刻)）。
#[derive(Default)]
struct FakePlays(BTreeMap<u32, (u64, Option<u64>)>);

impl CallPlays for FakePlays {
    fn playing(&self, animation: u32) -> Option<(u64, Option<u64>)> {
        self.0.get(&animation).copied()
    }
    fn set_acts_through(&mut self, animation: u32, through_ms: u64) {
        if let Some(p) = self.0.get_mut(&animation) {
            p.1 = Some(through_ms);
        }
    }
    fn start(&mut self, animation: u32, at_ms: u64) {
        self.0.insert(animation, (at_ms, None));
    }
    fn stop(&mut self, animation: u32) {
        self.0.remove(&animation);
    }
    fn playing_ids(&self) -> Vec<u32> {
        self.0.keys().copied().collect()
    }
}

fn plays(list: &[(u32, u64)]) -> FakePlays {
    FakePlays(list.iter().map(|&(id, s)| (id, (s, None))).collect())
}

fn frame(act: FrameAct, wait_ms: u32, x: i64, y: i64) -> LoopFrame {
    LoopFrame {
        surface_id: 10,
        picture: None,
        method: ComposeMethod::Overlay,
        act,
        wait_ms,
        x,
        y,
    }
}

fn draw(surface_id: i64, wait_ms: u32) -> LoopFrame {
    LoopFrame {
        surface_id,
        ..frame(FrameAct::Draw, wait_ms, 0, 0)
    }
}

fn start(targets: &[u32], pick_one: bool) -> LoopFrame {
    let act = FrameAct::Start {
        targets: targets.into(),
        pick_one,
    };
    frame(act, 0, 0, 0)
}

fn stop(targets: &[u32], pick_one: bool) -> LoopFrame {
    let act = FrameAct::Stop {
        targets: targets.into(),
        pick_one,
    };
    frame(act, 0, 0, 0)
}

fn mv(wait_ms: u32, x: i64, y: i64) -> LoopFrame {
    frame(FrameAct::Move, wait_ms, x, y)
}

fn anim(id: u32, trigger: LoopTrigger, frames: Vec<LoopFrame>) -> LoopAnimation {
    LoopAnimation {
        id,
        trigger,
        frames,
    }
}

const RANDOM: LoopTrigger = LoopTrigger::Random { k: 1 };

/// 選びは常に `i` を返す。
fn fixed_pick(i: u32) -> LoopRng {
    Box::new(move |_| i)
}

fn run(table: &[LoopAnimation], p: &mut FakePlays, now_ms: u64) -> CallOutcome {
    run_calls(
        p,
        |id| table.iter().find(|a| a.id == id),
        now_ms,
        &mut fixed_pick(0),
    )
}

/// 遅れた刻みでも、呼ばれた側は呼び出しの予定の時刻で始まり、頭の働きのコマも同じ刻みで当たる（3.1・3.5）。
#[test]
fn late_tick_starts_target_at_scheduled_time() {
    let table = [
        anim(0, RANDOM, vec![draw(10, 100), start(&[1], false)]),
        anim(1, LoopTrigger::CallOnly, vec![draw(20, 0), mv(50, 3, 4)]),
    ];
    let mut p = plays(&[(0, 0)]);
    let out = run(&table, &mut p, 350);
    assert_eq!(p.playing(1), Some((100, Some(350))));
    assert_eq!(
        p.playing(0),
        Some((0, Some(350))),
        "呼んだ側は止めない（3.4）"
    );
    assert_eq!(out.shift, Some((150, 3, 4)));
}

/// 再生中の相手を呼ぶと、頭からやり直す（3.9・M1）。
#[test]
fn restarting_a_playing_target_restarts_from_head() {
    let table = [
        anim(0, RANDOM, vec![draw(10, 100), start(&[1], false)]),
        anim(1, RANDOM, vec![mv(0, 7, 7), draw(20, 1000)]),
    ];
    let mut p = plays(&[(0, 0)]);
    p.0.insert(1, (0, Some(50)));
    let out = run(&table, &mut p, 120);
    assert_eq!(p.playing(1), Some((100, Some(120))));
    assert_eq!(out.shift, Some((100, 7, 7)), "やり直した頭の move も当たる");
}

/// parallelstart は並びの全部を同じ時刻に始める（3.2）。parallelstop は全部を止める（4.2）。
#[test]
fn parallel_start_and_stop_act_on_all_at_the_same_time() {
    let table = [
        anim(0, RANDOM, vec![draw(10, 100), start(&[1, 2], false)]),
        anim(1, LoopTrigger::CallOnly, vec![draw(20, 1000)]),
        anim(2, LoopTrigger::CallOnly, vec![draw(30, 1000)]),
        anim(3, RANDOM, vec![draw(10, 200), stop(&[1, 2], false)]),
    ];
    let mut p = plays(&[(0, 0)]);
    run(&table, &mut p, 100);
    assert_eq!(p.playing(1).map(|x| x.0), Some(100));
    assert_eq!(p.playing(2).map(|x| x.0), Some(100));
    p.0.insert(3, (100, None));
    run(&table, &mut p, 300);
    assert_eq!(p.playing_ids(), vec![0, 3]);
}

/// alternativestart は選びの乱数で並びの全部から 1 つを選ぶ（3.3）。
#[test]
fn alternative_start_picks_one_with_the_pick_rng() {
    let table = [
        anim(0, RANDOM, vec![start(&[1, 2, 3], true)]),
        anim(1, LoopTrigger::CallOnly, vec![draw(20, 1000)]),
        anim(2, LoopTrigger::CallOnly, vec![draw(20, 1000)]),
        anim(3, LoopTrigger::CallOnly, vec![draw(20, 1000)]),
    ];
    let mut p = plays(&[(0, 0)]);
    let mut pick: LoopRng = Box::new(|_| 2);
    run_calls(&mut p, |id| table.iter().find(|a| a.id == id), 0, &mut pick);
    assert_eq!(p.playing_ids(), vec![0, 3]);
    // 並びの長さを渡す（重さは等しい）。
    let mut seen: LoopRng = Box::new(|b| {
        assert_eq!(b, 3);
        0
    });
    let mut q = plays(&[(0, 0)]);
    run_calls(&mut q, |id| table.iter().find(|a| a.id == id), 0, &mut seen);
    assert_eq!(q.playing_ids(), vec![0, 1]);
}

/// alternativestop は再生中で絞らず並びの全部から選び、再生していない相手を選べば何もしない（4.3・4.4・M2）。
#[test]
fn alternative_stop_choosing_a_not_playing_target_does_nothing() {
    let table = [
        anim(0, RANDOM, vec![stop(&[1, 2], true)]),
        anim(1, RANDOM, vec![draw(20, 1000)]),
        anim(2, RANDOM, vec![draw(20, 1000)]),
    ];
    let mut p = plays(&[(0, 0), (1, 0)]);
    let mut pick = fixed_pick(1);
    run_calls(&mut p, |id| table.iter().find(|a| a.id == id), 0, &mut pick);
    assert_eq!(p.playing_ids(), vec![0, 1], "2 を選んだので 1 は止まらない");
    let mut pick = fixed_pick(0);
    let mut p = plays(&[(0, 0), (1, 0)]);
    run_calls(&mut p, |id| table.iter().find(|a| a.id == id), 0, &mut pick);
    assert_eq!(p.playing_ids(), vec![0]);
}

/// 自分を呼ぶ・呼び合う巡回は、待ちが 0 でも同じ時刻の 2 度目の開始で止まる（5.2）。
#[test]
fn self_and_mutual_calls_terminate() {
    let table = [
        anim(0, RANDOM, vec![start(&[0], false)]),
        anim(1, RANDOM, vec![start(&[2], false)]),
        anim(2, LoopTrigger::CallOnly, vec![start(&[1], false)]),
    ];
    let mut p = plays(&[(0, 0), (1, 0)]);
    run(&table, &mut p, 0);
    assert_eq!(p.playing_ids(), vec![0, 1, 2]);
    // 待ちが正の自己呼び出しは時刻が進み、今まででやり直しが止まる。
    let table = [anim(0, RANDOM, vec![draw(10, 100), start(&[0], false)])];
    let mut p = plays(&[(0, 0)]);
    run(&table, &mut p, 1050);
    assert_eq!(p.playing(0), Some((1000, Some(1050))));
}

/// 抽選などでこの時刻に始まった再生を、同じ時刻の呼び出しが始め直さない（5.2・3.3）。
#[test]
fn playback_started_this_instant_is_not_restarted_by_a_call() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    let table = [
        anim(0, RANDOM, vec![start(&[1, 2], true), start(&[0], false)]),
        anim(1, LoopTrigger::CallOnly, vec![draw(20, 1000)]),
        anim(2, LoopTrigger::CallOnly, vec![draw(30, 1000)]),
    ];
    let draws = Arc::new(AtomicU32::new(0));
    let seen = Arc::clone(&draws);
    // 引くたびに別の相手を選ぶ。
    let mut pick: LoopRng = Box::new(move |_| seen.fetch_add(1, Ordering::SeqCst) % 2);
    let mut p = plays(&[(0, 0)]);
    run_calls(&mut p, |id| table.iter().find(|a| a.id == id), 0, &mut pick);
    assert_eq!(draws.load(Ordering::SeqCst), 1, "選びは 1 度だけ引く");
    assert_eq!(p.playing_ids(), vec![0, 1], "始まる相手は 1 つだけ");
}

/// 止めた再生の後の出来事は捨てる（4.1）。止める前の move は残る。
#[test]
fn events_after_stop_are_dropped() {
    let table = [
        anim(0, RANDOM, vec![draw(10, 100), stop(&[1], false)]),
        anim(1, RANDOM, vec![mv(50, 1, 1), mv(150, 2, 2)]),
    ];
    let mut p = plays(&[(0, 0), (1, 0)]);
    let out = run(&table, &mut p, 300);
    assert_eq!(p.playing_ids(), vec![0]);
    assert_eq!(out.shift, Some((50, 1, 1)));
}

/// `move` は後の時刻の値が勝ち、同じ時刻なら当てる順の後ろが勝つ（7.2・7.3）。
#[test]
fn last_move_value_wins() {
    let table = [
        anim(0, RANDOM, vec![mv(100, 1, 1), mv(100, 2, 2)]),
        anim(1, RANDOM, vec![mv(150, 3, 3), mv(50, 4, 4)]),
    ];
    let mut p = plays(&[(0, 0), (1, 0)]);
    assert_eq!(run(&table, &mut p, 1000).shift, Some((200, 4, 4)));
    let table = [anim(0, RANDOM, vec![mv(100, 1, 1), mv(100, 2, 2)])];
    let mut p = plays(&[(0, 0)]);
    assert_eq!(run(&table, &mut p, 150).shift, Some((100, 1, 1)));
    assert_eq!(run(&table, &mut p, 250).shift, Some((200, 2, 2)));
    assert_eq!(
        run(&table, &mut p, 300).shift,
        None,
        "当て終えたコマは当て直さない"
    );
}

/// `always` は周ごとに働きのコマを当て、回数つきは回数で終わる。`-1` の描くコマの後ろは止まった再生ゆえ当てない。
#[test]
fn always_repeats_per_lap_and_stop_frame_cuts_others() {
    let period = NonZeroU64::new(100).unwrap();
    let frames = vec![draw(10, 50), start(&[1], false), draw(11, 50)];
    let callee = anim(1, LoopTrigger::CallOnly, vec![draw(20, 1000)]);
    let endless = LoopTrigger::Always {
        period_ms: period,
        laps: None,
    };
    let table = [anim(0, endless, frames.clone()), callee.clone()];
    let mut p = plays(&[(0, 0)]);
    run(&table, &mut p, 420);
    assert_eq!(p.playing(1).map(|x| x.0), Some(350));
    let twice = LoopTrigger::Always {
        period_ms: period,
        laps: NonZeroU32::new(2),
    };
    let table = [anim(0, twice, frames), callee.clone()];
    let mut p = plays(&[(0, 0)]);
    run(&table, &mut p, 420);
    assert_eq!(p.playing(1).map(|x| x.0), Some(150));
    let table = [
        anim(0, RANDOM, vec![draw(-1, 100), start(&[1], false)]),
        callee,
    ];
    let mut p = plays(&[(0, 0)]);
    run(&table, &mut p, 420);
    assert_eq!(p.playing(1), None);
}

/// 表に無い相手は何もしない（5.1）。
#[test]
fn targets_not_in_the_table_do_nothing() {
    let table = [anim(0, RANDOM, vec![start(&[9], false), stop(&[8], false)])];
    let mut p = plays(&[(0, 0)]);
    run(&table, &mut p, 10);
    assert_eq!(p.playing_ids(), vec![0]);
}
