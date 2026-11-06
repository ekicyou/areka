//! vblank のスレッドの 1 周（2）・待ち合わせ（8）・クリック透過の中継（11）。
//!
//! 時刻・`DwmFlush`・眠り方を差し込んで、`DwmFlush` の結果と待った長さの 3 通りで
//! notify がちょうど 1 回届くこと、補いの境、panic のスレッドの記録、`run` の後の中継の
//! 終わり方を確かめる。実 DWM は使わない（実機の確かめはタスク 8.2）。

use super::*;
use event_listener::Listener;
use log_capture_kit::count_levels;
use windows::Win32::Foundation::E_FAIL;

#[test]
fn compensation_boundaries() {
    let us = Duration::from_micros;
    let ms = Duration::from_millis;
    assert_eq!(
        [
            compensation(us(0)),
            compensation(us(999)),
            compensation(ms(1)),
            compensation(ms(16)),
        ],
        [Some(ms(16)), Some(us(15_001)), None, None]
    );
}

/// 偽の時計（呼ぶ前と呼んだ後の 2 回だけ読む）で 1 周を回し、
/// （届いた notify の有無, 眠った長さ, ログの件数）を返す。
fn run_once(
    waited: Duration,
    flushed: windows::core::Result<()>,
) -> (bool, Option<Duration>, log_capture_kit::LevelCounts) {
    let base = Instant::now();
    let mut reads = [base, base + waited].into_iter();
    let mut now = move || reads.next().expect("時計は 2 回だけ読むはず");
    let event = Event::new();
    let listener = event.listen();
    let mut slept = None;
    let ((), counts) =
        count_levels(|| vsync_once(&mut now, || flushed, |d| slept = Some(d), &event));
    (
        listener.wait_timeout(Duration::ZERO).is_some(),
        slept,
        counts,
    )
}

#[test]
fn vsync_once_waited_flush_notifies_without_sleep() {
    let (notified, slept, counts) = run_once(Duration::from_millis(8), Ok(()));
    assert_eq!(
        (notified, slept, counts.debug, counts.trace),
        (true, None, 0, 0)
    );
}

#[test]
fn vsync_once_unwaited_flush_sleeps_the_rest_then_notifies() {
    let (notified, slept, counts) = run_once(Duration::from_micros(200), Ok(()));
    assert_eq!(
        (notified, slept, counts.debug, counts.trace),
        (true, Some(Duration::from_micros(15_800)), 0, 1)
    );
}

#[test]
fn vsync_once_failed_flush_logs_sleeps_and_still_notifies() {
    let (notified, slept, counts) = run_once(Duration::ZERO, Err(E_FAIL.into()));
    assert_eq!(
        (notified, slept, counts.debug, counts.trace),
        (true, Some(Duration::from_millis(16)), 1, 1)
    );
}

#[test]
fn join_vsync_leaves_one_error_for_a_panicked_thread() {
    let panicked = thread::spawn(|| panic!("意図した panic（join_vsync の検査）"));
    let ((), counts) = count_levels(|| join_vsync(panicked));
    assert_eq!(counts.error, 1);

    let finished = thread::spawn(|| {});
    let ((), counts) = count_levels(|| join_vsync(finished));
    assert_eq!(counts.error, 0);
}

/// `run()` が戻った後（`running = false`）の vblank では、中継は `wake` を起こさずに
/// 終わる（`run_async_tick` の同じ検査の写し）。
#[test]
fn relay_stops_without_waking_once_the_loop_has_returned() {
    let world = Rc::new(RefCell::new(crate::ecs::world::EcsWorld::new()));
    let vblank = Arc::new(Event::new());
    let wake = Arc::new(Event::new());
    let running = Rc::new(Cell::new(true));

    let _relay = crate::executor::spawn_local(relay_vblank(
        Arc::clone(&vblank),
        Rc::downgrade(&world),
        Arc::clone(&wake),
        Rc::clone(&running),
    ));
    crate::executor::block_on(async {}); // 待ちを立てさせる

    // 対照: ループが回っている間の vblank は中継される。
    let woke = wake.listen();
    vblank.notify(usize::MAX);
    crate::executor::block_on(async {});
    assert!(
        woke.wait_timeout(Duration::ZERO).is_some(),
        "ループが回っている間は中継するはず"
    );

    // ループが戻った後の vblank。
    running.set(false);
    let after_return = wake.listen();
    vblank.notify(usize::MAX);
    crate::executor::block_on(async {});

    // 終わったことの確かめ: 旗を戻しても、もう中継しない。
    running.set(true);
    let after_end = wake.listen();
    vblank.notify(usize::MAX);
    crate::executor::block_on(async {});

    assert_eq!(
        (
            after_return.wait_timeout(Duration::ZERO).is_some(),
            after_end.wait_timeout(Duration::ZERO).is_some(),
        ),
        (false, false),
        "(戻った後の vblank で起こしたか, 終わった後に起こしたか) は両方 false のはず"
    );
}
