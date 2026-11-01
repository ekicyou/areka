use std::cell::{Cell, RefCell};
use std::sync::mpsc;

use super::wait::{
    DENSE_SPIN, GPU_PERMIT_IDLE, GpuSlots, PermitVerdict, WAIT_CAP, join_bounded,
    join_thread_bounded, judge_permit_wait, run_bounded_watching, wait_recv, wait_thread_end_with,
    wait_until_idle_with,
};
use super::{
    BACKOFF_SLEEP, Duration, Instant, Progress, SPIN_WAIT, WaitFailure, run_bounded,
    wait_until_with,
};

// ===========================================================================
// 待ちの芯の檻（areka-P0-ghost-session-test-load-flake タスク 2.2・要件 2.1・2.3・2.5・3.1〜3.3）
//
// [`wait_until_with`] へ偽の時計と偽の休みを渡し、打ち切りの決め方を実時間を待たずに確かめる。
// 時刻は **条件の関数が呼ばれた回数だけ**で決まる（1 回ごとに `step` 進む）。芯が 1 回の反復で
// 時計を何回読むかに期待値が依存しないので、実装の内側の都合で檻が動かない。
// 休みは記録するだけで眠らない。空回しの `yield_now` だけは本物だが、1 回は一瞬で終わる。
// ===========================================================================

/// 偽の時計で [`wait_until_with`] を回した結果。
struct Run {
    result: Result<(), WaitFailure>,
    /// 条件の関数が呼ばれた回数。
    conds: u32,
    /// 休みが呼ばれたときの「条件の呼ばれた回数」と、休みの長さ。
    pauses: Vec<(u32, Duration)>,
}

/// `step` ずつ進む偽の時計で待つ。`cond(k)` と `count(k)` は、k 回目（1 始まり）の条件の呼び出しと、
/// その時点の進みの目印を返す。`count` が `None` なら目印なし（[`Progress::Unknown`]）。
fn drive(
    step: Duration,
    mut cond: impl FnMut(u32) -> bool,
    count: Option<&dyn Fn(u32) -> u64>,
) -> Run {
    let base = Instant::now();
    let conds = Cell::new(0u32);
    let pauses = RefCell::new(Vec::new());
    let probe = || count.map_or(0, |f| f(conds.get()));
    let progress = match count {
        Some(_) => Progress::Count(&probe),
        None => Progress::Unknown,
    };
    let result = wait_until_with(
        || base + step * conds.get(),
        |d| pauses.borrow_mut().push((conds.get(), d)),
        "檻の待ち",
        progress,
        || {
            let k = conds.get() + 1;
            conds.set(k);
            cond(k)
        },
    );
    Run {
        result,
        conds: conds.get(),
        pauses: pauses.into_inner(),
    }
}

/// 檻 1: 届いていれば成功。最初から真なら時計も休みも使わずに `Ok`。
/// 時計が上限を大きく越えた後でも、その回に条件が真なら `Ok`（条件の確かめが打ち切りの判定より先）。
#[test]
fn a_reached_condition_succeeds_however_late() {
    let result = wait_until_with(
        || panic!("条件が最初から真なのに時計を読んだ"),
        |_| panic!("条件が最初から真なのに休んだ"),
        "最初から真",
        Progress::Unknown,
        || true,
    );
    assert_eq!(result, Ok(()));

    // 読むたびに上限の 10 倍進む時計。2 回目の条件で届く。判定が条件の確かめより先だと、
    // 2 回目の条件の前に読んだ時計で打ち切りになる。
    let zero = || 0u64;
    for count in [None, Some(&zero as &dyn Fn() -> u64)] {
        let base = Instant::now();
        let reads = Cell::new(0u32);
        let conds = Cell::new(0u32);
        let progress = match count {
            Some(f) => Progress::Count(f),
            None => Progress::Unknown,
        };
        let result = wait_until_with(
            || {
                reads.set(reads.get() + 1);
                base + WAIT_CAP * 10 * reads.get()
            },
            |_| {},
            "遅れて届く",
            progress,
            || {
                conds.set(conds.get() + 1);
                conds.get() == 2
            },
        );
        assert_eq!(result, Ok(()), "届いた回は、時計がどれだけ進んでいても成功");
    }
}

/// 檻 2: 目印が増え続ける間は、`SPIN_WAIT` を何倍越えても打ち切らない。`WAIT_CAP` に届いた回で
/// `CapReached`。`moves` は目印の増えた分、`since_last_move` は最後に増えてからの時間。
#[test]
fn a_moving_partner_is_never_cut_off_before_the_cap() {
    let step = Duration::from_secs(1);
    // 目印は 10 回に 1 つ増える（進みの無い時間は最大 9 秒＜ SPIN_WAIT）。
    let run = drive(step, |_| false, Some(&|k| u64::from(k / 10)));

    let caps = (WAIT_CAP.as_secs() / step.as_secs()) as u32;
    // 待ち始めは 1 回目の条件の後（時計 1 秒）。上限に届くのは時計が 1 + 300 秒の回。
    assert_eq!(run.conds, caps + 1);
    assert_eq!(
        run.result,
        Err(WaitFailure::CapReached {
            what: "檻の待ち".into(),
            waited: WAIT_CAP,
            moves: u64::from(caps + 1) / 10,
            since_last_move: Duration::from_secs(1),
            last_step: step,
        })
    );
}

/// 檻 3: 目印が増えないまま `SPIN_WAIT` に届いたら `Stalled`。途中で 1 度増えると、
/// 進みの無い時間は 0 から数え直しになる。
#[test]
fn a_partner_that_stops_moving_is_reported_as_stalled() {
    let step = Duration::from_secs(1);
    let idle_steps = (SPIN_WAIT.as_secs() / step.as_secs()) as u32;

    let run = drive(step, |_| false, Some(&|_| 5));
    assert_eq!(run.conds, 1 + idle_steps);
    assert_eq!(
        run.result,
        Err(WaitFailure::Stalled {
            what: "檻の待ち".into(),
            waited: SPIN_WAIT,
            idle: SPIN_WAIT,
            moves: 0,
            last_step: step,
        })
    );

    // 20 回目の条件の時点で 1 度だけ増える。止まったと判じるのは、そこから SPIN_WAIT 後。
    let run = drive(step, |_| false, Some(&|k| u64::from(k >= 20)));
    assert_eq!(
        run.conds,
        20 + idle_steps,
        "増えたら進みの無い時間を 0 に戻す"
    );
    assert_eq!(
        run.result,
        Err(WaitFailure::Stalled {
            what: "檻の待ち".into(),
            waited: step * (19 + idle_steps),
            idle: SPIN_WAIT,
            moves: 1,
            last_step: step,
        })
    );
}

/// 檻 4: 目印なしの待ちは、今と同じ総時間 `SPIN_WAIT` で `TimedOut`。
#[test]
fn an_unmarked_wait_times_out_at_the_same_total_as_before() {
    let step = Duration::from_secs(1);
    let run = drive(step, |_| false, None);
    assert_eq!(run.conds, 1 + (SPIN_WAIT.as_secs() / step.as_secs()) as u32);
    assert_eq!(
        run.result,
        Err(WaitFailure::TimedOut {
            what: "檻の待ち".into(),
            waited: SPIN_WAIT,
        })
    );
}

/// 檻 5: 待ち始めから `DENSE_SPIN` に届くまでは休まない（空回し）。届いた後は反復ごとに
/// 1 度ずつ `BACKOFF_SLEEP` 休んで CPU を返す。
#[test]
fn the_wait_starts_returning_the_cpu_after_the_dense_spin_window() {
    let step = Duration::from_millis(1);
    let reach = 100u32;
    let run = drive(step, |k| k == reach, Some(&|_| 0));
    assert_eq!(run.result, Ok(()));

    // k 回目の条件の後の時計は待ち始めから (k - 1) ms。DENSE_SPIN に届くのは k = 1 + 60。
    let first = 1 + (DENSE_SPIN.as_millis() / step.as_millis()) as u32;
    let expected: Vec<(u32, Duration)> = (first..reach).map(|k| (k, BACKOFF_SLEEP)).collect();
    assert_eq!(run.pauses, expected);
}

/// 檻 6: 4 つの失敗の文言は、何を・何秒・進みの回数を含み、先頭の `［…］` が互いに違う。
#[test]
fn the_four_failures_read_differently() {
    let cases = [
        (
            WaitFailure::Stalled {
                what: "止まる待ち".into(),
                waited: Duration::from_millis(45_500),
                idle: Duration::from_secs(30),
                moves: 7,
                last_step: Duration::from_millis(14_900),
            },
            "待ちの打ち切り［止まった］",
            vec![
                "「止まる待ち」",
                "30.0 秒",
                "45.5 秒",
                "7 回",
                "最後の 1 回の確かめに 14.9 秒",
            ],
        ),
        (
            WaitFailure::CapReached {
                what: "長い待ち".into(),
                waited: Duration::from_secs(300),
                moves: 1234,
                since_last_move: Duration::from_millis(2_500),
                last_step: Duration::from_millis(234_700),
            },
            "待ちの打ち切り［進んではいた］",
            vec![
                "「長い待ち」",
                "300.0 秒",
                "1234 回",
                "2.5 秒",
                "最後の 1 回の確かめに 234.7 秒",
            ],
        ),
        (
            WaitFailure::TimedOut {
                what: "目印なし".into(),
                waited: Duration::from_secs(30),
            },
            "待ちの打ち切り［進みは不明］",
            vec!["「目印なし」", "30.0 秒"],
        ),
        (
            WaitFailure::Disconnected {
                what: "受け口".into(),
                waited: Duration::from_millis(1_200),
            },
            "待ちの打ち切り［相手が居ない］",
            vec!["「受け口」", "1.2 秒"],
        ),
    ];
    let mut heads = Vec::new();
    for (failure, head, parts) in &cases {
        let text = failure.to_string();
        assert!(text.starts_with(head), "先頭が違う: {text}");
        for part in parts {
            assert!(text.contains(part), "{part} が無い: {text}");
        }
        heads.push(text[..text.find('］').expect("［…］がある")].to_owned());
    }
    heads.sort();
    heads.dedup();
    assert_eq!(heads.len(), cases.len(), "先頭の［…］が重なっている");
}

/// 檻 7: 受け口の待ち。届けばその値。送り手が何も送らずに居なくなれば `Disconnected`（何秒も待たない）。
#[test]
fn a_receiver_returns_the_value_or_reports_a_vanished_sender() {
    let (tx, rx) = mpsc::channel();
    tx.send(7u32).expect("受け口は生きている");
    assert_eq!(wait_recv("値が届く", Progress::Unknown, &rx), Ok(7));

    drop(tx);
    match wait_recv("送り手が居なくなる", Progress::Unknown, &rx) {
        Err(WaitFailure::Disconnected { what, waited }) => {
            assert_eq!(what, "送り手が居なくなる");
            assert!(
                waited < SPIN_WAIT,
                "居なくなったのに打ち切りまで待った: {waited:?}"
            );
        }
        other => panic!("Disconnected のはず: {other:?}"),
    }
}

/// 別スレッドの処理を待つ包みは、相手が panic して居なくなったら、すぐに `［相手が居ない］` の文言で
/// panic する。古い呼び名 `run_bounded` は今の文を先頭に保ち、後ろに同じ文言を足す。
#[test]
fn a_vanished_worker_panics_with_the_partner_gone_text() {
    let text = |r: std::thread::Result<()>| -> String {
        let payload = r.expect_err("相手が居なくなったら panic する");
        payload
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_else(|| "（文字列でない panic）".into())
    };

    run_bounded_watching("終わる処理", Progress::Unknown, || {});

    let watched = text(std::panic::catch_unwind(|| {
        run_bounded_watching("落ちる処理", Progress::Unknown, || {
            panic!("わざと落とす")
        })
    }));
    assert!(
        watched.starts_with("待ちの打ち切り［相手が居ない］: 「落ちる処理」"),
        "{watched}"
    );

    let old = text(std::panic::catch_unwind(|| {
        run_bounded("落ちる処理", SPIN_WAIT, || panic!("わざと落とす"))
    }));
    assert!(
        old.starts_with("'落ちる処理' did not complete within 30s (possible hang)"),
        "今の文が先頭に無い: {old}"
    );
    assert!(old.contains("待ちの打ち切り［相手が居ない］"), "{old}");
}

/// アクターの終わりを待つ `join_bounded` は、body が終われば `Ok`、body が panic していれば
/// `ActorError::Panicked` を返す（握り潰さない）。
#[test]
fn join_bounded_returns_the_actor_result() {
    let (_tx, ends) = areka_actor::spawn_actor::<(), _>("ends", |_rx| {});
    assert!(join_bounded("終わるアクター", ends).is_ok());

    let (_tx, falls) = areka_actor::spawn_actor::<(), _>("falls", |_rx| panic!("わざと落とす"));
    match join_bounded("落ちるアクター", falls) {
        Err(areka_actor::ActorError::Panicked { actor, message }) => {
            assert_eq!(actor, "falls");
            assert!(message.contains("わざと落とす"), "{message}");
        }
        other => panic!("panic を ActorError::Panicked で返す: {other:?}"),
    }
}

/// スレッドの終わりの待ちの芯を、読むたびに `step` 進む偽の時計で回す（休みは眠らない）。
fn drive_thread_end(
    step: Duration,
    what: &str,
    inner: Option<&dyn Fn() -> u64>,
    is_finished: impl Fn() -> bool,
) {
    let base = Instant::now();
    let reads = Cell::new(0u32);
    wait_thread_end_with(
        || {
            reads.set(reads.get() + 1);
            base + step * reads.get()
        },
        |_| {},
        what,
        inner,
        is_finished,
    );
}

/// `f` の panic の文（panic しなければ赤）。
fn panic_text(f: impl FnOnce()) -> String {
    let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .expect_err("終わらないスレッドは打ち切りで panic する");
    payload
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| "（文字列でない panic）".into())
}

/// 相手のスレッドの中の数えが増え続ける間は、`SPIN_WAIT` を何倍越えても `［止まった］` で打ち切らない
/// （要件 4.1）。中の数えを見ないと、終わったかの目印だけでは 30 秒で打ち切られる。
#[test]
fn a_thread_whose_inner_count_keeps_moving_is_not_cut_off() {
    let inner = Cell::new(0u64);
    let tick = || {
        inner.set(inner.get() + 1);
        inner.get()
    };
    // 1 秒ずつ進む時計で 120 回確かめた後に終わる＝待ち始めから約 120 秒（SPIN_WAIT の 4 倍）。
    let checks = Cell::new(0u32);
    drive_thread_end(
        Duration::from_secs(1),
        "進み続けるスレッド",
        Some(&tick),
        || {
            checks.set(checks.get() + 1);
            checks.get() > 120
        },
    );
    assert!(checks.get() > 120, "終わるまで待った");
}

/// 終わらないスレッドの打ち切りの文は、std のスレッドでもアクターでも同じ頭で始まる（要件 4.4）。
/// 本物のスレッドを止めておき、読むたびに `SPIN_WAIT` 進む偽の時計で芯を回す（実時間は待たない）。
#[test]
fn a_thread_that_never_ends_reads_the_same_head_for_both_handles() {
    let head = |what: &str| {
        format!("'{what}' join did not complete (possible hang) — 待ちの打ち切り［止まった］")
    };

    let (std_tx, std_rx) = mpsc::channel::<()>();
    let std_handle = std::thread::spawn(move || {
        let _ = std_rx.recv();
    });
    let std_text = panic_text(|| {
        drive_thread_end(SPIN_WAIT, "std のスレッド", None, || {
            std_handle.is_finished()
        })
    });

    let (actor_tx, actor) = areka_actor::spawn_actor::<(), _>("never-ends", |rx| {
        let _ = rx.recv();
    });
    let actor_text =
        panic_text(|| drive_thread_end(SPIN_WAIT, "アクター", None, || actor.is_finished()));

    assert!(std_text.starts_with(&head("std のスレッド")), "{std_text}");
    assert!(actor_text.starts_with(&head("アクター")), "{actor_text}");

    // 止めておいた相手を放し、本物の待ちで終わりを受け取る（返り値も握り潰さない）。
    drop((std_tx, actor_tx));
    assert!(join_thread_bounded("std のスレッド", None, std_handle).is_ok());
    assert!(join_bounded("アクター", actor).is_ok());
}

// ===========================================================================
// GPU の装置の許可の檻（areka-P0-ghost-session-test-load-flake タスク 5.2・檻 16・要件 2.7）
//
// プロセスに 1 つの数え（足場が使う）とは別の数えを置き、待たずに取る口だけで確かめる（実時間は待たない）。
// ===========================================================================

/// 2 つ取った後は 3 つ目が取れず、1 つ返すと取れる。
#[test]
fn gpu_slots_hold_at_most_the_capacity_and_a_returned_permit_frees_one() {
    static SLOTS: GpuSlots = GpuSlots::new(2);
    let first = SLOTS.try_take().expect("1 つ目は取れる");
    let _second = SLOTS.try_take().expect("2 つ目は取れる");
    assert!(
        SLOTS.try_take().is_none(),
        "同時に 2 つまで（3 つ目は取れない）"
    );
    drop(first);
    let _third = SLOTS.try_take().expect("1 つ返すと取れる");
    assert!(
        SLOTS.try_take().is_none(),
        "返した分だけ空く（2 つ持っている）"
    );
}

// ===========================================================================
// 許可の待ちの上限の檻（areka-test-binary-stability タスク 5.2・要件 6.1〜6.3）
//
// 本物の `take` と同じ決め手 `judge_permit_wait` と同じ文の作り `GpuSlots::wait_failure` を、偽の時刻で
// 呼ぶ（実時間は待たない）。上限 2 の数えで 2 つを取った形＝2 つ取りのテストが 1 つずつ持って待ち合った形。
// ===========================================================================

/// 返りの無いまま `idle` を越えると `Stalled`、途中で 1 つ返れば `Continue`。打ち切りの文に「許可」
/// 「2／上限 2」「［止まった］」と待った秒数が出る。
#[test]
fn a_permit_wait_with_no_return_stalls_and_names_the_permits() {
    static SLOTS: GpuSlots = GpuSlots::new(2);
    let _first = SLOTS.try_take().expect("1 つ目は取れる");
    let _second = SLOTS.try_take().expect("2 つ目は取れる");
    assert!(SLOTS.try_take().is_none(), "上限 2 を持ち切った");

    let idle = Duration::from_secs(40);
    let started = Instant::now();
    let at = |secs: u64| started + Duration::from_secs(secs);
    assert_eq!(
        judge_permit_wait(at(39), started, idle),
        PermitVerdict::Continue,
        "idle の手前は待ち続ける"
    );
    let verdict = judge_permit_wait(at(41), started, idle);
    assert_eq!(
        verdict,
        PermitVerdict::Stalled,
        "返りの無いまま idle を越えた"
    );
    // 途中で 1 つ返った（最後の返りが 20 秒目）なら、41 秒目でも返りから 21 秒で止まっていない。
    assert_eq!(
        judge_permit_wait(at(41), at(20), idle),
        PermitVerdict::Continue,
        "返りを進みとして数える"
    );

    let failure = SLOTS
        .wait_failure(
            2,
            verdict,
            started,
            at(41),
            started,
            0,
            Duration::from_secs(1),
        )
        .expect("Stalled は打ち切りの文になる");
    let text = failure.to_string();
    for want in ["許可", "2／上限 2", "［止まった］", "41.0 秒"] {
        assert!(text.contains(want), "「{want}」が文に無い: {text}");
    }
    assert!(
        SLOTS
            .wait_failure(
                2,
                PermitVerdict::Continue,
                started,
                at(41),
                started,
                0,
                Duration::ZERO
            )
            .is_none(),
        "Continue は文にならない"
    );
}

/// 返りが続く限り、待ち始めから [`WAIT_CAP`] の 300 秒をまたいでも打ち切らない。返りが止まって
/// [`GPU_PERMIT_IDLE`] の 500 秒が過ぎたら `［止まった］`（2026-11-02 裁定「案 2」・総時間の上限は効かせない）。
#[test]
fn a_permit_wait_with_returns_is_never_cut_by_the_total_limit() {
    static SLOTS: GpuSlots = GpuSlots::new(2);
    let started = Instant::now();
    let at = |secs: u64| started + Duration::from_secs(secs);
    // 100 秒ごとに 1 つ返る待ち。1,000 秒（WAIT_CAP の 3 倍余り）まで、どの時点でも打ち切らない。
    for secs in (0..=1_000).step_by(10) {
        let last_return_at = at(secs / 100 * 100);
        assert_eq!(
            judge_permit_wait(at(secs), last_return_at, GPU_PERMIT_IDLE),
            PermitVerdict::Continue,
            "{secs} 秒目（最後の返りは {} 秒目）",
            secs / 100 * 100
        );
    }
    // 1,000 秒目で返りが止まる。1,499 秒目はまだ待ち、1,500 秒目（返りから 500 秒）で止まった。
    let last = at(1_000);
    assert_eq!(
        judge_permit_wait(at(1_499), last, GPU_PERMIT_IDLE),
        PermitVerdict::Continue
    );
    let verdict = judge_permit_wait(at(1_500), last, GPU_PERMIT_IDLE);
    assert_eq!(verdict, PermitVerdict::Stalled);
    let text = SLOTS
        .wait_failure(
            2,
            verdict,
            started,
            at(1_500),
            last,
            10,
            Duration::from_secs(1),
        )
        .expect("Stalled は打ち切りの文になる")
        .to_string();
    for want in ["2／上限 2", "［止まった］", "500.0 秒", "1500.0 秒"] {
        assert!(text.contains(want), "「{want}」が文に無い: {text}");
    }
}

/// 芯の内側の形は渡した `idle` で `［止まった］` にする（`wait_until_with` の 30 秒より短くても）。
#[test]
fn the_inner_wait_core_stalls_at_the_given_idle() {
    let base = Instant::now();
    let reads = Cell::new(0u32);
    let zero = || 0u64;
    let result = wait_until_idle_with(
        || {
            reads.set(reads.get() + 1);
            base + Duration::from_secs(1) * reads.get()
        },
        |_| {},
        "短い idle",
        Progress::Count(&zero),
        Duration::from_secs(5),
        || false,
    );
    match result {
        Err(WaitFailure::Stalled { idle, .. }) => {
            assert_eq!(idle, Duration::from_secs(5), "渡した idle で打ち切る")
        }
        other => panic!("［止まった］のはず: {other:?}"),
    }
}
