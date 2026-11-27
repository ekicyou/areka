//! 文字の層の受け手が次のフレームを起こす所（列に残った時・寿命の旗で回った後の受け取り・借りられない
//! フレーム・まだ現れていない字）。
//!
//! 起こしの旗と期限はプロセスで 1 つなので、ほかのテストが立てた物が混ざる。だから共有の旗の上では
//! 「立てた物が含まれる」側だけを確かめ、「置かない」側は相が使う純判断の値で確かめる。

use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use areka_emo_text::actor::spawn_emo_text;
use areka_sakura::{CueCommand, TalkCue};
use dola::cue::CueSink;
use dola::time::testing::{at_ms, ms};
use dola::{TimePoint, TimeSpan};
use wintf::ecs::world::tick_wake::{self, WakeBits};

use super::scale_text::{next_reveal, wake_after};
use super::test_support::{capture_logs, count_level, headless_wiring_with, zero_clock};
use super::*;
use crate::talk_clock::TalkClock;
use crate::talk_lifecycle::{BalloonLifecycleSink, TalkLifecycleSignal};

/// この組のテストどうしが、共有の旗を読み倒し合わないための錠。
static WAKE: Mutex<()> = Mutex::new(());

fn headless_wiring() -> RuntimeWiring {
    headless_wiring_with(mpsc::channel::<PresentCommand>().1, zero_clock())
}

fn actor() -> ActorKey {
    ActorKey::from("0")
}

/// 発火時刻 `fire_time` の 3 字の合図（1 字 50 ms）。
fn three_glyphs(fire_time: TimePoint) -> TalkCue {
    TalkCue::scheduled(
        fire_time,
        TimeSpan::ZERO,
        actor(),
        CueCommand::Text("あいう".into()),
        ms(150),
    )
}

fn visible(wiring: &RuntimeWiring, at: TimePoint) -> usize {
    wiring
        .runtime()
        .borrow()
        .state()
        .visible_glyphs(&actor(), at)
}

/// `after` だけ先の時刻で旗と期限を読む（読み倒す）。
fn woken(after: Duration) -> tick_wake::WakeSnapshot {
    tick_wake::take(Instant::now() + after)
}

/// 時刻の来ていない合図が列に残ったフレームは、その時刻に次のフレームを起こす予約を置き、その時刻の
/// フレームで当たる。
#[test]
fn a_frame_that_leaves_a_cue_waiting_arms_a_wake_at_its_fire_time() {
    let _wake = WAKE.lock().unwrap_or_else(|p| p.into_inner());
    let mut wiring = headless_wiring();
    let mut world = World::new();
    let (mut sink, _drain) = spawn_emo_text(Rc::clone(wiring.runtime()));
    sink.emit(three_glyphs(at_ms(1_250)));
    crate::spine::pump_until_idle();
    let _ = woken(Duration::ZERO);

    run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(1_000)));
    assert_eq!(visible(&wiring, TimePoint::MAX), 0, "列に残る");
    assert_eq!(
        wake_after(at_ms(1_250), at_ms(1_000)),
        Some(Duration::from_millis(250)),
        "相が預ける待ち"
    );
    assert!(
        woken(Duration::from_millis(300)).deadline_due,
        "発火時刻を過ぎた時に期限が来ている"
    );

    run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(1_250)));
    assert_eq!(
        visible(&wiring, at_ms(1_250)),
        1,
        "発火時刻のフレームで当たる"
    );
}

/// 寿命の受け口の旗でフレームが回り（文字の層の列は空で、何も予約されない）、その後で文字のメッセージが
/// 取り出されて列に入った時に、旗が立ち、次のフレームでその合図が当たる。
#[test]
fn a_text_message_taken_after_a_lifecycle_driven_frame_wakes_the_next_frame() {
    let _wake = WAKE.lock().unwrap_or_else(|p| p.into_inner());
    let mut wiring = headless_wiring();
    let mut world = World::new();
    let (tx, _rx) = mpsc::channel::<TalkLifecycleSignal>();
    let mut life = BalloonLifecycleSink::new(tx, TalkClock::new(Arc::new(|| at_ms(0))));
    let (mut text, _drain) = spawn_emo_text(Rc::clone(wiring.runtime()));
    let cue = three_glyphs(at_ms(1_000));
    let _ = woken(Duration::ZERO);

    life.emit(cue.clone());
    text.emit(cue);
    assert!(
        WakeBits(woken(Duration::ZERO).bits).contains(tick_wake::PRESENT),
        "寿命の受け口の旗"
    );
    run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(1_000)));
    assert_eq!(
        visible(&wiring, TimePoint::MAX),
        0,
        "文字のメッセージはまだ"
    );
    assert_eq!(
        wiring.runtime().borrow_mut().advance(at_ms(1_000)),
        None,
        "列は空で、待つ時刻が無い"
    );

    crate::spine::pump_until_idle();
    assert!(
        WakeBits(woken(Duration::ZERO).bits).contains(tick_wake::PRESENT),
        "取り出して列に入れた時の旗"
    );
    run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(1_016)));
    assert_eq!(visible(&wiring, at_ms(1_016)), 1, "次のフレームで当たる");
}

/// 進める相が文字の層を借りられないフレームは、落ちずに `error!` を出して次の画面の更新を予約し、次の
/// フレームで列の物が当たる。
#[test]
fn a_frame_that_cannot_borrow_the_layer_rearms_and_the_next_frame_applies_the_queue() {
    let _wake = WAKE.lock().unwrap_or_else(|p| p.into_inner());
    let mut wiring = headless_wiring();
    let mut world = World::new();
    let (mut sink, _drain) = spawn_emo_text(Rc::clone(wiring.runtime()));
    sink.emit(three_glyphs(at_ms(1_000)));
    crate::spine::pump_until_idle();
    let _ = woken(Duration::ZERO);

    let runtime = Rc::clone(wiring.runtime());
    let held = runtime.borrow();
    let logs = capture_logs(|| run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(1_000))));
    drop(held);
    assert_eq!(count_level(&logs, "ERROR"), 1, "{logs:?}");
    assert!(
        WakeBits(woken(Duration::ZERO).bits).contains(tick_wake::REARM),
        "次の画面の更新を予約する"
    );
    assert_eq!(visible(&wiring, TimePoint::MAX), 0, "見送ったフレーム");

    run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(1_016)));
    assert_eq!(visible(&wiring, at_ms(1_016)), 1, "次のフレームで当たる");
}

/// まだ現れていない字が在る間、文字の相は次に現れる字の時刻への予約を置き、字が現れ切ったら置かない。
#[test]
fn the_text_phase_arms_the_next_glyph_until_every_glyph_has_appeared() {
    let _wake = WAKE.lock().unwrap_or_else(|p| p.into_inner());
    let mut wiring = headless_wiring();
    let mut world = World::new();
    let (mut sink, _drain) = spawn_emo_text(Rc::clone(wiring.runtime()));
    sink.emit(three_glyphs(at_ms(1_000)));
    crate::spine::pump_until_idle();
    run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(1_000)));
    let times = wiring
        .runtime()
        .borrow()
        .state()
        .actor_state(&actor())
        .expect("当たった")
        .reveal()
        .times()
        .to_vec();
    assert_eq!(times.len(), 3, "前提: 3 字");
    let next = |frame: TimePoint| next_reveal([&times[..]].into_iter(), frame);

    for (&shown, &upcoming) in times.iter().zip(&times[1..]) {
        let _ = woken(Duration::ZERO);
        run_text_phase(&mut wiring, &mut world, Some(shown));
        assert_eq!(next(shown), Some(upcoming), "{shown} の次の字");
        let wait = upcoming.span_since(shown).expect("後の字").to_std();
        assert!(
            woken(wait + Duration::from_millis(5)).deadline_due,
            "{shown} のフレームは {upcoming} に起こす"
        );
    }
    let last = *times.last().expect("3 字");
    run_text_phase(&mut wiring, &mut world, Some(last));
    assert_eq!(next(last), None, "字が現れ切ったら予約しない");
    assert_eq!(visible(&wiring, last), 3);
}
