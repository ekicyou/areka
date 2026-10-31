//! 窓の中で起こしたアクターの記録の引き継ぎ（[`super::spawn_actor`]）の対照の試験。
//!
//! 捕捉はすべて `log_capture_kit::capture` を通し、試験の中で受け手を直に差さない。
//! 宛先は本ファイル専用の 1 つで、各主張は「宛先＋項目 `probe` の値」で絞った記録だけを
//! 数える（フックの試験のフックの記録や、並んで回るほかの試験の記録と混ざらない）。
//! 待ちはすべて受信（上限 10 秒）か join で、眠りの時間に頼らない。

use super::*;
use log_capture_kit::{CapturedEvent, capture};
use std::time::Duration;

/// 本ファイル専用の宛先。
const TARGET: &str = "areka_actor::spawn_log_capture_tests";

/// 済みの受信の上限（超えたら赤・ハングしない）。
const WAIT: Duration = Duration::from_secs(10);

/// 本ファイルの宛先で、項目 `probe` が `probe` の記録だけを取り出す。
fn probed<'a>(events: &'a [CapturedEvent], probe: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.target == TARGET && e.field_str("probe") == Some(probe))
        .collect()
}

// ⑴ 窓の中で起こしたアクターの warn と trace が、出した形のまま窓に入る。
#[test]
fn records_of_actor_spawned_inside_window_are_captured() {
    let (handle, events) = capture(|| {
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let (_tx, handle) = spawn_actor::<(), _>("capture-inherit-1", move |_rx| {
            tracing::warn!(target: TARGET, probe = "inherit-1", code = 7, "窓の中の警告");
            tracing::trace!(target: TARGET, probe = "inherit-1", code = 8, "窓の中の追跡");
            done_tx.send(()).expect("済みを返す");
        });
        done_rx
            .recv_timeout(WAIT)
            .expect("アクターは済みを返すはず");
        handle
    });
    handle.join().expect("body は正常終了するはず");

    let hits = probed(&events, "inherit-1");
    assert_eq!(
        hits.len(),
        2,
        "warn と trace の 2 件が窓に入るはず: {hits:?}"
    );
    let (warn, trace) = (hits[0], hits[1]);
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_eq!(warn.message(), "窓の中の警告");
    assert_eq!(warn.field("code"), Some("7"));
    assert_eq!(trace.level, tracing::Level::TRACE);
    assert_eq!(trace.message(), "窓の中の追跡");
    assert_eq!(trace.field("code"), Some("8"));
}

// ⑵ 窓の中で起こしたアクターが本体の中で起こした子の記録も窓に入る。
#[test]
fn records_of_child_spawned_by_inherited_actor_are_captured() {
    let (handle, events) = capture(|| {
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let (_tx, handle) = spawn_actor::<(), _>("capture-inherit-parent", move |_rx| {
            let (_child_tx, child) = spawn_actor::<(), _>("capture-inherit-child", |_rx| {
                tracing::warn!(target: TARGET, probe = "inherit-child", "子の警告");
            });
            child.join().expect("子は正常終了するはず");
            done_tx.send(()).expect("済みを返す");
        });
        done_rx.recv_timeout(WAIT).expect("親は済みを返すはず");
        handle
    });
    handle.join().expect("親の body は正常終了するはず");

    let hits = probed(&events, "inherit-child");
    assert_eq!(hits.len(), 1, "子の警告が窓に入るはず: {hits:?}");
    assert_eq!(hits[0].level, tracing::Level::WARN);
    assert_eq!(hits[0].message(), "子の警告");
}

// ⑶ 窓の外で起こしたアクター・ほかの窓の記録は、この窓に入らない。
// ⒜ 窓を開く前に起こしたアクター ⒝ 窓の無い別のスレッドが起こしたアクター
// ⒞ 別のスレッドが自分の窓の中で起こしたアクター、のどれもこの窓が開いているあいだに出す。
#[test]
fn records_of_actors_spawned_outside_this_window_are_not_captured() {
    // ⒜ 窓を開く前に起こす。出すのは窓の中から済みの送り口を受けたとき。
    let (before_tx, before) = spawn_actor::<mpsc::Sender<()>, _>("capture-outside-before", |rx| {
        for done in rx {
            tracing::warn!(target: TARGET, probe = "outside-before", "窓の前に起こしたアクター");
            done.send(()).expect("済みを返す");
        }
    });

    // ⒝ 窓の無いスレッド。起こすのは窓の中から送り口を受けた後（＝窓が開いているあいだ）。
    let (plain_go, plain_go_rx) = mpsc::channel::<mpsc::Sender<()>>();
    let plain = thread::spawn(move || {
        let done = plain_go_rx
            .recv_timeout(WAIT)
            .expect("窓から合図が来るはず");
        let (_tx, h) = spawn_actor::<(), _>("capture-outside-plain", move |_rx| {
            tracing::warn!(target: TARGET, probe = "outside-plain", "窓の無いスレッドのアクター");
            done.send(()).expect("済みを返す");
        });
        h.join().expect("body は正常終了するはず");
    });

    // ⒞ 自分の窓を開いたスレッド。その窓の中で、合図を受けてから起こす。
    let (other_go, other_go_rx) = mpsc::channel::<mpsc::Sender<()>>();
    let other = thread::spawn(move || {
        let (h, events) = capture(|| {
            let done = other_go_rx
                .recv_timeout(WAIT)
                .expect("窓から合図が来るはず");
            let (inner_tx, inner_rx) = mpsc::channel::<()>();
            let (_tx, h) = spawn_actor::<(), _>("capture-outside-other", move |_rx| {
                tracing::warn!(target: TARGET, probe = "outside-other", "ほかの窓のアクター");
                inner_tx.send(()).expect("済みを返す");
            });
            // 自分の窓を閉じる前に済みを受け、この窓の側へも済みを返す。
            inner_rx
                .recv_timeout(WAIT)
                .expect("アクターは済みを返すはず");
            done.send(()).expect("済みを返す");
            h
        });
        h.join().expect("body は正常終了するはず");
        events
    });

    let (control, events) = capture(|| {
        let (done_tx, done_rx) = mpsc::channel::<()>();
        before_tx.send(done_tx.clone()).expect("⒜ へ送る");
        plain_go.send(done_tx.clone()).expect("⒝ へ送る");
        other_go.send(done_tx.clone()).expect("⒞ へ送る");
        // 対照: 同じ窓の中で起こしたアクター。
        let (_tx, h) = spawn_actor::<(), _>("capture-outside-control", move |_rx| {
            tracing::warn!(target: TARGET, probe = "outside-control", "この窓のアクター");
            done_tx.send(()).expect("済みを返す");
        });
        for _ in 0..4 {
            done_rx.recv_timeout(WAIT).expect("4 つとも済みを返すはず");
        }
        h
    });
    control.join().expect("body は正常終了するはず");
    drop(before_tx);
    before
        .join()
        .expect("⒜ は全 Sender を落とすと正常に終わるはず");
    plain.join().expect("⒝ のスレッドは正常終了するはず");
    let other_events = other.join().expect("⒞ のスレッドは正常終了するはず");

    let control_hits = probed(&events, "outside-control");
    assert_eq!(
        control_hits.len(),
        1,
        "対照: この窓のアクターは入るはず: {control_hits:?}"
    );
    for probe in ["outside-before", "outside-plain", "outside-other"] {
        let hits = probed(&events, probe);
        assert!(hits.is_empty(), "{probe} はこの窓に入らないはず: {hits:?}");
    }
    let other_hits = probed(&other_events, "outside-other");
    assert_eq!(
        other_hits.len(),
        1,
        "対照: ⒞ は自分の窓に入るはず（捕捉が素通りしていない）: {other_hits:?}"
    );
}

// ⑷ 窓 A の中で起こしたアクターが A を閉じた後に出した記録は、後から開いた窓 B に入らない。
#[test]
fn records_after_window_closes_do_not_reach_later_window() {
    let ((x_tx, x), events_a) = capture(|| {
        let (x_tx, x) = spawn_actor::<mpsc::Sender<()>, _>("capture-late-x", |rx| {
            for done in rx {
                tracing::warn!(target: TARGET, probe = "late-x", "X の警告");
                done.send(()).expect("済みを返す");
            }
        });
        let (done_tx, done_rx) = mpsc::channel::<()>();
        x_tx.send(done_tx).expect("X へ送る");
        done_rx.recv_timeout(WAIT).expect("X は済みを返すはず");
        (x_tx, x)
    });
    let a_hits = probed(&events_a, "late-x");
    assert_eq!(
        a_hits.len(),
        1,
        "対照: A の中の X の記録は A に入るはず: {a_hits:?}"
    );

    let (y, events_b) = capture(|| {
        let (done_tx, done_rx) = mpsc::channel::<()>();
        x_tx.send(done_tx.clone()).expect("X へ送る");
        let (_tx, y) = spawn_actor::<(), _>("capture-late-y", move |_rx| {
            tracing::warn!(target: TARGET, probe = "late-y", "Y の警告");
            done_tx.send(()).expect("済みを返す");
        });
        for _ in 0..2 {
            done_rx.recv_timeout(WAIT).expect("X と Y は済みを返すはず");
        }
        y
    });
    y.join().expect("Y の body は正常終了するはず");

    let y_hits = probed(&events_b, "late-y");
    assert_eq!(
        y_hits.len(),
        1,
        "対照: B の中で起こした Y は B に入るはず: {y_hits:?}"
    );
    let x_hits = probed(&events_b, "late-x");
    assert!(
        x_hits.is_empty(),
        "A を閉じた後の X の記録は B に入らないはず: {x_hits:?}"
    );

    drop(x_tx);
    x.join().expect("X は全 Sender を落とすと正常に終わるはず");
}
