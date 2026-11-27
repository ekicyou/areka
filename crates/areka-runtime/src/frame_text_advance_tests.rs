//! 文字の層を進める相と、文字の層に触る相の借りられない時の扱いと、次のフレームを起こす時刻。

use std::sync::mpsc;
use std::time::Duration;

use areka_emo_text::actor::spawn_emo_text;
use areka_sakura::{CueCommand, TalkCue};
use dola::cue::CueSink;
use dola::time::testing::{at_ms, ms};
use dola::{TimePoint, TimeSpan};
use wintf::ecs::FrameTime;

use super::super::source_text_test_support::{code_only, index_of};
use super::scale_text::wake_after;
use super::test_support::{capture_logs, count_level, headless_wiring_with, zero_clock};
use super::*;

fn headless_wiring() -> RuntimeWiring {
    headless_wiring_with(mpsc::channel::<PresentCommand>().1, zero_clock())
}

fn actor() -> ActorKey {
    ActorKey::from("0")
}

/// 文字の層の受け口から、発火時刻 `fire_time` の 2 字の合図（1 字 50 ms）を UI の受け取りまで通す。
fn queue_text(wiring: &RuntimeWiring, fire_time: TimePoint) {
    let (mut sink, _join) = spawn_emo_text(Rc::clone(wiring.runtime()));
    sink.emit(TalkCue::scheduled(
        fire_time,
        TimeSpan::ZERO,
        actor(),
        CueCommand::Text("あい".into()),
        ms(100),
    ));
    crate::spine::pump_until_idle();
}

fn visible(wiring: &RuntimeWiring, at: TimePoint) -> usize {
    wiring
        .runtime()
        .borrow()
        .state()
        .visible_glyphs(&actor(), at)
}

#[test]
fn resolve_frame_now_takes_the_override_then_the_frame_time() {
    let mut world = World::new();
    assert_eq!(resolve_frame_now(None, &world), None, "どちらも無い");
    assert_eq!(
        resolve_frame_now(Some(at_ms(5_000)), &world),
        Some(at_ms(5_000))
    );
    world.insert_resource(FrameTime(at_ms(9_000)));
    assert_eq!(resolve_frame_now(None, &world), Some(at_ms(9_000)));
    assert_eq!(
        resolve_frame_now(Some(at_ms(5_000)), &world),
        Some(at_ms(5_000)),
        "テストの値が勝つ"
    );
}

#[test]
fn advance_phase_applies_a_queued_cue_once_the_frame_time_reaches_its_fire_time() {
    let mut wiring = headless_wiring();
    let mut world = World::new();
    queue_text(&wiring, at_ms(1_000));
    assert_eq!(visible(&wiring, TimePoint::MAX), 0, "受け取りでは当てない");

    run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(999)));
    assert_eq!(
        visible(&wiring, TimePoint::MAX),
        0,
        "発火時刻の前には当てない"
    );

    run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(1_000)));
    assert_eq!(visible(&wiring, at_ms(1_000)), 1);
    assert_eq!(visible(&wiring, at_ms(1_050)), 2);
}

#[test]
fn advance_phase_without_a_frame_time_leaves_the_queue_for_the_next_frame() {
    let mut wiring = headless_wiring();
    let mut world = World::new();
    queue_text(&wiring, at_ms(0));

    run_text_advance_phase(&mut wiring, &mut world, None);
    assert_eq!(visible(&wiring, TimePoint::MAX), 0);

    world.insert_resource(FrameTime(at_ms(10)));
    run_text_advance_phase(&mut wiring, &mut world, None);
    assert_eq!(visible(&wiring, TimePoint::MAX), 2, "FrameTime で進める");
}

/// 進める相・文字の拡大率の相・文字の相は、文字の層を借りられないフレームを見送り、縮みの始まりだけ
/// `error!` を 1 行出す。借りられたフレームを挟むと、次の縮みでまた 1 行出す。
#[test]
fn text_layer_phases_skip_and_log_once_per_streak_while_the_layer_is_borrowed() {
    type Phase = fn(&mut RuntimeWiring, &mut World);
    let phases: [(&str, Phase); 3] = [
        ("advance", |w, world| {
            run_text_advance_phase(w, world, Some(at_ms(1_000)))
        }),
        ("text-scale", |w, world| {
            let _ = run_text_scale_phase(w, world);
        }),
        ("text", |w, world| {
            run_text_phase(w, world, Some(at_ms(1_000)))
        }),
    ];
    for (name, phase) in phases {
        let mut wiring = headless_wiring();
        let mut world = World::new();
        let runtime = Rc::clone(wiring.runtime());

        let held = runtime.borrow();
        let first = capture_logs(|| {
            phase(&mut wiring, &mut world);
            phase(&mut wiring, &mut world);
        });
        drop(held);
        assert_eq!(count_level(&first, "ERROR"), 1, "{name}: {first:?}");
        assert!(
            first.iter().any(|l| l.contains(name)),
            "{name}: 相の名が載る: {first:?}"
        );

        let freed = capture_logs(|| phase(&mut wiring, &mut world));
        assert_eq!(count_level(&freed, "ERROR"), 0, "{name}: {freed:?}");

        let held = runtime.borrow();
        let again = capture_logs(|| phase(&mut wiring, &mut world));
        drop(held);
        assert_eq!(
            count_level(&again, "ERROR"),
            1,
            "{name}: 次の縮み: {again:?}"
        );
    }
}

#[test]
fn a_skipped_advance_keeps_the_queue_and_applies_it_on_the_next_frame() {
    let mut wiring = headless_wiring();
    let mut world = World::new();
    queue_text(&wiring, at_ms(0));
    let runtime = Rc::clone(wiring.runtime());

    let held = runtime.borrow();
    let _ = capture_logs(|| run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(10))));
    drop(held);
    assert_eq!(visible(&wiring, TimePoint::MAX), 0);

    run_text_advance_phase(&mut wiring, &mut world, Some(at_ms(20)));
    assert_eq!(visible(&wiring, TimePoint::MAX), 2);
}

#[test]
fn wake_after_is_the_remaining_time_capped_at_one_hour() {
    assert_eq!(
        wake_after(at_ms(1_250), at_ms(1_000)),
        Some(Duration::from_millis(250))
    );
    assert_eq!(wake_after(at_ms(1_000), at_ms(1_000)), None, "もう来ている");
    assert_eq!(wake_after(at_ms(900), at_ms(1_000)), None);
    assert_eq!(
        wake_after(at_ms(10_000_000), at_ms(0)),
        Some(Duration::from_secs(3600)),
        "遠い時刻は 1 時間で頭打ち"
    );
}

/// 本番の `frame_system` で、進める相は結線を取り出した直後・作業領域の同期より前に 1 つだけ在る。
#[test]
fn the_frame_calls_the_advance_phase_first() {
    let code = code_only(include_str!("frame.rs"));
    let call = "run_text_advance_phase(&mut wiring, world, None);";
    assert_eq!(code.matches(call).count(), 1, "呼出がちょうど 1 つではない");
    let (remove, advance, sync) = (
        index_of(&code, "world.remove_non_send::<RuntimeWiring>()"),
        index_of(&code, call),
        index_of(&code, "work_area_sync::sync_monitor_snapshot(world)"),
    );
    assert!(
        remove < advance && advance < sync,
        "結線を取り出した直後・最初の相の前に無い（remove={remove}・advance={advance}・sync={sync}）"
    );
}
