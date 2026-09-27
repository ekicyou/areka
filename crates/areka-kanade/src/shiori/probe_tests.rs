//! 見張り部品（[`ShioriProbe`]）のテスト。
//!
//! 記録（`warn!`・`error!`・`debug!`）は発火の決め手 [`try_fire`] をテストのスレッドで直に呼んで
//! スレッドローカルの捕捉窓で見る（見張りのスレッドの記録はその窓に入らない）。見張りの
//! スレッドを通す回は、結果（[`CutGuard::finish`] の戻り値）と解く手の呼ばれた回数で判定する。
//! どの回も眠らない: 見張りが解く手を呼んだことは解く手が送る知らせを待って確かめる。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::*;

/// 見張りの知らせを待つ上限（時刻で判定はしない・止まったときに赤にするためだけの値）。
const NOTICE_WAIT: Duration = Duration::from_secs(10);

/// 呼ばれた回数を数え、呼ばれるたびに知らせを送る解く手。
fn counting_unblock(result: Result<(), String>) -> (ShioriUnblock, Arc<AtomicUsize>, Receiver<()>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let (tx, rx) = mpsc::channel();
    let tx = std::sync::Mutex::new(tx);
    let counter = Arc::clone(&calls);
    let unblock: ShioriUnblock = Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
        let _ = tx.lock().expect("知らせの送信端").send(());
        result.clone()
    });
    (unblock, calls, rx)
}

/// 期限に達しない予算（見張りは手動の口か `finish` でしか起きない）。
fn far_budget() -> WaitBudget {
    WaitBudget {
        started: Instant::now(),
        limit: Duration::from_secs(3600),
    }
}

fn busy_probe(busy: ShioriBusy, unblock: Option<ShioriUnblock>) -> ShioriProbe {
    let probe = ShioriProbe::default();
    probe.set_busy(busy);
    probe.install_unblock(unblock);
    probe
}

fn events_named<'a>(events: &'a [CapturedEvent], name: &str) -> Vec<&'a CapturedEvent> {
    events
        .iter()
        .filter(|e| e.target == "shiori-actor" && e.field_str("event") == Some(name))
        .collect()
}

/// 張って `finish` すれば発火しない（解く手 0 回・結果なし）。見張りのスレッドの記録は窓に
/// 入らないので、`warn!` 0 件は「`finish` が先に勝った後の決め手」を同じ窓で見て判定する。
/// 対照: 同じ窓で、勝てる決め手（別の見張り部品）が `warn!` を 1 件出す。
#[test]
fn finishing_within_the_limit_does_not_cut_and_logs_no_warning() {
    let (unblock, calls, _rx) = counting_unblock(Ok(()));
    let probe = busy_probe(ShioriBusy::Request("OnBoot".into()), Some(unblock));
    let budget = far_budget();

    let guard = probe.arm(budget);
    assert_eq!(guard.finish(), None, "期限の前に終えたら切らない");
    assert_eq!(calls.load(Ordering::SeqCst), 0, "解く手は呼ばれない");

    let (control_unblock, control_calls, _crx) = counting_unblock(Ok(()));
    let control = busy_probe(ShioriBusy::Idle, Some(control_unblock));
    let ((lost, won), events) =
        capture(|| (try_fire(&probe.0, budget), try_fire(&control.0, budget)));

    assert_eq!(lost, None, "`finish` が勝った後の決め手は切らない");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let cuts = events_named(&events, "shiori_wait_cut");
    assert_eq!(cuts.len(), 1, "対照の 1 件だけ: {events:?}");
    assert_eq!(
        cuts[0].field_str("stage"),
        Some("idle"),
        "対照の決め手の記録"
    );
    assert_eq!(
        won,
        Some(ShioriCut {
            stage: "idle",
            unblocked: true
        })
    );
    assert_eq!(control_calls.load(Ordering::SeqCst), 1);
    assert!(
        events
            .iter()
            .filter(|e| e.level == Level::WARN)
            .all(|e| e.field_str("stage") == Some("idle")),
        "負けた決め手の warn! は 0 件: {events:?}"
    );
}

/// 段の語 4 通り: 今の呼び出しから段を決め、解く手を 1 回呼び、`warn!` を 1 件残す。
#[test]
fn firing_names_the_stage_calls_the_unblock_once_and_warns_once() {
    let cases = [
        (
            ShioriBusy::Request("OnClose".into()),
            "on_close_notify",
            Some("OnClose"),
        ),
        (
            ShioriBusy::Request("OnBoot".into()),
            "in_flight_request",
            Some("OnBoot"),
        ),
        (ShioriBusy::Unload, "unload", None),
        (ShioriBusy::Idle, "idle", None),
    ];
    for (busy, stage, id) in cases {
        let (unblock, calls, _rx) = counting_unblock(Ok(()));
        let probe = busy_probe(busy.clone(), Some(unblock));
        let budget = WaitBudget {
            started: Instant::now(),
            limit: Duration::from_millis(3000),
        };
        let (cut, events) = capture(|| try_fire(&probe.0, budget));

        assert_eq!(
            cut,
            Some(ShioriCut {
                stage,
                unblocked: true
            }),
            "{busy:?}"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1, "{busy:?}: 解く手は 1 回");
        let warns: Vec<_> = events.iter().filter(|e| e.level == Level::WARN).collect();
        assert_eq!(warns.len(), 1, "{busy:?}: {events:?}");
        let w = warns[0];
        assert_eq!(w.target, "shiori-actor");
        assert_eq!(w.field_str("event"), Some("shiori_wait_cut"));
        assert_eq!(w.field_str("stage"), Some(stage));
        assert_eq!(w.field("limit_ms"), Some("3000"));
        assert!(w.field("elapsed_ms").is_some(), "経過を残す");
        assert_eq!(w.field("unblocked"), Some("true"));
        let expected_id = format!("{:?}", id.map(str::to_string));
        assert_eq!(w.field("id"), Some(expected_id.as_str()), "{busy:?}");
        assert!(
            events.iter().all(|e| e.level != Level::ERROR),
            "解けた回に error! は無い: {events:?}"
        );

        // 2 度目の決め手は切らない（発火は高々 1 回）。
        assert_eq!(try_fire(&probe.0, budget), None);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

/// 手動の口: 張られた見張りが今の呼び出しの段で解き、`finish` がその結果を返す。
#[test]
fn manual_cut_through_the_watchdog_thread_returns_the_cut() {
    let (unblock, calls, rx) = counting_unblock(Ok(()));
    let probe = busy_probe(ShioriBusy::Request("OnClose".into()), Some(unblock));

    let guard = probe.arm(far_budget());
    probe.cut_now();
    rx.recv_timeout(NOTICE_WAIT).expect("見張りが解く手を呼ぶ");

    assert_eq!(
        guard.finish(),
        Some(ShioriCut {
            stage: "on_close_notify",
            unblocked: true
        })
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// 期限の口: 手動を使わず、短い期限で見張りが自分で起きる（結果は時刻に依らない）。
#[test]
fn the_deadline_fires_without_the_manual_trigger() {
    let (unblock, calls, rx) = counting_unblock(Ok(()));
    let probe = busy_probe(ShioriBusy::Unload, Some(unblock));

    let guard = probe.arm(WaitBudget {
        started: Instant::now(),
        limit: Duration::from_millis(20),
    });
    rx.recv_timeout(NOTICE_WAIT)
        .expect("期限で見張りが解く手を呼ぶ");

    assert_eq!(
        guard.finish(),
        Some(ShioriCut {
            stage: "unload",
            unblocked: true
        })
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// 後始末に入る前に費やした時間も上限の内に数える: 出発点が既に上限を過ぎていれば即発火。
#[test]
fn time_spent_before_arming_counts_against_the_limit() {
    let (unblock, _calls, rx) = counting_unblock(Ok(()));
    let probe = busy_probe(ShioriBusy::Idle, Some(unblock));
    // 上限は知らせを待つ上限（NOTICE_WAIT）より十分長く、出発点はそれより前に置く。経過を
    // 引かずに上限そのものを待てば NOTICE_WAIT の内に発火せず赤になる。
    let limit = Duration::from_secs(60);
    let started = Instant::now()
        .checked_sub(Duration::from_secs(120))
        .expect("単調時計は 2 分以上進んでいる");

    let guard = probe.arm(WaitBudget { started, limit });
    rx.recv_timeout(NOTICE_WAIT).expect("残りが 0 なので即発火");
    assert_eq!(guard.finish().map(|c| c.stage), Some("idle"));
}

/// 解く手が無い（据えていない・`None` を据えた）ときは `error!` を残し「解けた」は偽。
#[test]
fn missing_unblock_logs_error_and_reports_not_unblocked() {
    for installed in [false, true] {
        let probe = ShioriProbe::default();
        probe.set_busy(ShioriBusy::Unload);
        if installed {
            probe.install_unblock(None);
        }
        let (cut, events) = capture(|| try_fire(&probe.0, far_budget()));

        assert_eq!(
            cut,
            Some(ShioriCut {
                stage: "unload",
                unblocked: false
            })
        );
        let errors = events_named(&events, "shiori_unblock_unavailable");
        assert_eq!(errors.len(), 1, "installed={installed}: {events:?}");
        assert_eq!(errors[0].level, Level::ERROR);
        assert_eq!(errors[0].field_str("stage"), Some("unload"));
        let warns = events_named(&events, "shiori_wait_cut");
        assert_eq!(warns.len(), 1);
        assert_eq!(warns[0].field("unblocked"), Some("false"));
    }
}

/// 解く手が失敗したときは `error!` にその理由を残し「解けた」は偽。
#[test]
fn failing_unblock_logs_error_and_reports_not_unblocked() {
    let (unblock, calls, _rx) = counting_unblock(Err("access denied".into()));
    let probe = busy_probe(ShioriBusy::Request("OnClose".into()), Some(unblock));
    let (cut, events) = capture(|| try_fire(&probe.0, far_budget()));

    assert_eq!(
        cut,
        Some(ShioriCut {
            stage: "on_close_notify",
            unblocked: false
        })
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let errors = events_named(&events, "shiori_unblock_failed");
    assert_eq!(errors.len(), 1, "{events:?}");
    assert_eq!(errors[0].level, Level::ERROR);
    assert_eq!(errors[0].field_str("stage"), Some("on_close_notify"));
    assert!(
        errors[0]
            .field("error")
            .is_some_and(|e| e.contains("access denied")),
        "{events:?}"
    );
    assert_eq!(
        events_named(&events, "shiori_wait_cut")[0].field("unblocked"),
        Some("false")
    );
}

/// 張る前に手動の口を呼んでも空振りしない: 予約が残り、次に張った時点で即発火する。
/// 予約は 1 度で消費され、その次に張った見張りは発火しない。
#[test]
fn manual_cut_before_arming_is_kept_and_fires_on_the_next_arm() {
    let (unblock, calls, rx) = counting_unblock(Ok(()));
    let probe = busy_probe(ShioriBusy::Request("OnBoot".into()), Some(unblock));

    probe.cut_now();
    assert_eq!(calls.load(Ordering::SeqCst), 0, "張る前は何もしない");

    let guard = probe.arm(far_budget());
    rx.recv_timeout(NOTICE_WAIT)
        .expect("張った時点で予約が発火する");
    assert_eq!(guard.finish().map(|c| c.stage), Some("in_flight_request"));

    let again = probe.arm(far_budget());
    assert_eq!(again.finish(), None, "予約は消費済み");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

/// 同時の回: 後始末の終わりが先に勝っていれば、あとから起きた見張りは切らない。
/// 「終わりが勝った」状態を手で作ってから手動の口で見張りを起こす（眠らずに順序を固定）。
#[test]
fn when_finish_wins_the_race_the_watchdog_does_not_cut() {
    let (unblock, calls, _rx) = counting_unblock(Ok(()));
    let probe = busy_probe(ShioriBusy::Request("OnClose".into()), Some(unblock));

    let guard = probe.arm(far_budget());
    probe
        .0
        .outcome
        .compare_exchange(ARMED, FINISHED, Ordering::AcqRel, Ordering::Acquire)
        .expect("終わりが先に勝つ");
    probe.cut_now();

    assert_eq!(guard.finish(), None, "負けた見張りは切らない");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

/// 降り済みの段では切らず、`debug!` を 1 件だけ残す（`warn!` 0 件・解く手 0 回）。
/// 対照は同じ窓に出る `debug!` そのもの。見張りのスレッドを通す回も結果なし。
#[test]
fn after_unload_the_limit_does_not_cut_and_logs_debug_once() {
    let (unblock, calls, rx) = counting_unblock(Ok(()));
    let probe = busy_probe(ShioriBusy::Unloaded, Some(unblock));
    let budget = WaitBudget {
        started: Instant::now(),
        limit: Duration::from_millis(3000),
    };

    let (cut, events) = capture(|| try_fire(&probe.0, budget));
    assert_eq!(cut, None);
    let debugs = events_named(&events, "shiori_wait_limit_after_unload");
    assert_eq!(debugs.len(), 1, "{events:?}");
    assert_eq!(debugs[0].level, Level::DEBUG);
    assert_eq!(debugs[0].field("limit_ms"), Some("3000"));
    assert!(debugs[0].field("elapsed_ms").is_some());
    assert_eq!(
        events
            .iter()
            .filter(|e| e.level == Level::WARN || e.level == Level::ERROR)
            .count(),
        0,
        "{events:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    // 見張りのスレッドを通す回: `finish` と手動の口のどちらが先に勝つかは決めない（どちらの
    // 枝でも結果なし・解く手 0 回）。降り済みの段で切らないことの判定は上の直呼びで固定済み。
    let guard = probe.arm(far_budget());
    probe.cut_now();
    assert_eq!(guard.finish(), None, "見張りのスレッドを通しても切らない");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(rx.try_recv().is_err(), "解く手の知らせは来ていない");
}

/// 解く手は一度だけ据わる（2 度目は無視）。
#[test]
fn the_unblock_is_installed_only_once() {
    let (first, first_calls, _r1) = counting_unblock(Ok(()));
    let (second, second_calls, _r2) = counting_unblock(Ok(()));
    let probe = busy_probe(ShioriBusy::Idle, Some(first));
    probe.install_unblock(Some(second));

    assert_eq!(
        try_fire(&probe.0, far_budget()).map(|c| c.unblocked),
        Some(true)
    );
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 0);
}

/// `finish` せずに持ち手を落としても見張りは畳まれる: 送信端は残らず、勝者は終わり側に決まり、
/// その後の手動の口は古い見張りへ届かず予約になる（次に張った新しい回でだけ発火する）。
#[test]
fn dropping_the_guard_without_finish_stops_the_watchdog() {
    let (unblock, calls, rx) = counting_unblock(Ok(()));
    let probe = busy_probe(ShioriBusy::Request("OnClose".into()), Some(unblock));

    drop(probe.arm(far_budget()));
    assert!(lock(&probe.0.armed).tx.is_none(), "送信端は残らない");
    assert_eq!(probe.0.outcome.load(Ordering::Acquire), FINISHED);

    probe.cut_now();
    assert!(
        lock(&probe.0.armed).pending_cut,
        "古い見張りへは届かず予約になる"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let guard = probe.arm(far_budget());
    rx.recv_timeout(NOTICE_WAIT)
        .expect("新しい回の見張りが予約で発火する");
    assert_eq!(guard.finish().map(|c| c.stage), Some("on_close_notify"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
