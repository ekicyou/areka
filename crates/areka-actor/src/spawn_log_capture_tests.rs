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
