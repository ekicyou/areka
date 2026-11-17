use std::cell::Cell;

use super::{
    Duration, Instant, SETTLE_MIN, SETTLE_QUIET_ROUNDS, SPIN_WAIT, settle_bounded,
    settle_bounded_with,
};

// ===========================================================================
// task 4.1 — 「尽きるのが正常」の回収待ちの有界化（要件 4.2・4.4／design Flow 4・C3）
//
// [`settle_bounded`] は 2 つの条件の**両方**が満たされるまで返らない（最小持続 [`SETTLE_MIN`]
// かつ 連続空観測 [`SETTLE_QUIET_ROUNDS`] 回）。本ファイルはヘルパを**単独で**駆動し、
// 片方だけでは返らないこと（＝どちらの条件も効いていること）と、どちらも満たされなくても
// 上限 [`SPIN_WAIT`] で必ず返ることを固定する。
//
// # 壁時計に依存しない檻の作り方
//
// 「時間が経つまで返らない」を実時間の計測で確かめると、並列負荷で合否が動く（本仕様が
// まさに直そうとしている病）。そこで内側の継ぎ目 [`settle_bounded_with`] へ**注入時計**を渡し、
// 時刻を `step` の呼出回数だけの関数にする。期待値は整数の呼出回数になり、実時間は一切
// 判定に入らない（実行に要する実時間＝反復数 × [`BACKOFF_SLEEP`] は変わるが、合否は変わらない）。
// ===========================================================================

/// 注入時計で [`settle_bounded_with`] を駆動し、`step` の呼出回数を返す。
///
/// 時刻は **`step` の呼出回数だけ**で決まる（1 反復あたり `per_step` 進む）。ヘルパが 1 反復で
/// 時計を何回読むかに期待値が依存しないので、実装内部の都合で檻が動かない。
/// `outcome` は 1 始まりの反復番号を受け取り、その反復の回収件数を返す。
fn drive(per_step: Duration, mut outcome: impl FnMut(u32) -> usize) -> u32 {
    let base = Instant::now();
    let steps = Cell::new(0u32);
    settle_bounded_with(
        || base + per_step * steps.get(),
        || {
            let n = steps.get() + 1;
            steps.set(n);
            outcome(n)
        },
    );
    steps.get()
}

/// 連続空観測が先に満たされても、最小持続 [`SETTLE_MIN`] に達するまで返らない（要件 4.2）。
///
/// 毎反復 0 件（＝`SETTLE_QUIET_ROUNDS` は早々に満たされる）・1 反復 1ms の注入時計。
/// 返るのは `SETTLE_MIN` を満たす反復。最小持続の条件を外すと呼出回数が
/// `SETTLE_QUIET_ROUNDS` まで縮むので、この等値検査が赤になる。
#[test]
fn settle_bounded_does_not_return_before_the_minimum_duration() {
    let per_step = Duration::from_millis(1);
    let need = SETTLE_MIN.as_millis() as u32;
    assert!(
        need > SETTLE_QUIET_ROUNDS,
        "檻の前提: 最小持続の反復数 {need} は連続空回数 {SETTLE_QUIET_ROUNDS} より大きいこと（そうでないと最小持続が効いているかを分離できない）"
    );

    let calls = drive(per_step, |_| 0);

    assert_eq!(
        calls, need,
        "毎反復 0 件でも最小持続 {SETTLE_MIN:?} に達するまで返ってはならない（実際の反復数 {calls}・連続空回数の条件だけで打ち切っているなら {SETTLE_QUIET_ROUNDS} 前後で返る）"
    );
}

/// 最小持続が先に満たされても、連続 [`SETTLE_QUIET_ROUNDS`] 回の空観測に達するまで返らない（要件 4.2）。
///
/// 1 反復 100ms の注入時計＝2 反復で `SETTLE_MIN` を満たす。最初の 1 反復だけ 1 件回収して
/// 連続空回数を 0 へ戻すので、返るのは `1 + SETTLE_QUIET_ROUNDS` 反復目。連続空回数の条件を
/// 外すと最小持続を満たした直後（2 反復目）で返るので、この等値検査が赤になる。
#[test]
fn settle_bounded_does_not_return_before_the_consecutive_quiet_rounds() {
    let per_step = Duration::from_millis(100);
    let expected = SETTLE_QUIET_ROUNDS + 1;
    assert!(
        per_step * expected <= SPIN_WAIT,
        "檻の前提: 想定反復 {expected} × {per_step:?} が上限 {SPIN_WAIT:?} を超えないこと（超えると上限側で返って別物を測る）"
    );
    assert!(
        per_step * 2 >= SETTLE_MIN,
        "檻の前提: 2 反復で最小持続 {SETTLE_MIN:?} を満たすこと（満たさないと最小持続側で返って別物を測る）"
    );

    // 反復 1 だけ 1 件（＝連続空回数が 0 へ戻る）・以後は 0 件。
    let calls = drive(per_step, |n| usize::from(n == 1));

    assert_eq!(
        calls, expected,
        "最小持続を満たした後も連続 {SETTLE_QUIET_ROUNDS} 回の空観測に達するまで返ってはならない（実際の反復数 {calls}・最小持続だけで打ち切っているなら 2 反復で返る）"
    );
}

/// 一度も空にならなくても上限 [`SPIN_WAIT`] で必ず返る（hang しない・要件 4.2）。
#[test]
fn settle_bounded_returns_at_the_upper_bound_even_when_never_quiet() {
    let per_step = Duration::from_secs(1);
    let expected = SPIN_WAIT.as_secs() as u32 + 1;

    // 毎反復 1 件＝連続空回数は永久に 0。打ち切れるのは上限だけ。
    let calls = drive(per_step, |_| 1);

    assert_eq!(
        calls, expected,
        "毎反復 1 件でも上限 {SPIN_WAIT:?} 超過で返ること（実際の反復数 {calls}）"
    );
}

/// 既定の [`settle_bounded`] は実時計に繋がっており、panic せずに返る（要件 4.4・design C3）。
///
/// 実時間で判定するのは**下限だけ**（単調時計では下回りようがない）。上限は測らないので
/// 並列負荷で赤にならない。
#[test]
fn settle_bounded_drives_the_real_clock_without_panicking() {
    let calls = Cell::new(0u32);
    let started = Instant::now();

    settle_bounded(|| {
        calls.set(calls.get() + 1);
        0
    });

    let elapsed = started.elapsed();
    assert!(
        elapsed >= SETTLE_MIN,
        "実時計でも最小持続 {SETTLE_MIN:?} に達する前に返ってはならない（実測 {elapsed:?}）"
    );
    assert!(
        calls.get() >= SETTLE_QUIET_ROUNDS,
        "回収機会が連続空回数 {SETTLE_QUIET_ROUNDS} を下回ってはならない（実測 {} 回）",
        calls.get()
    );
}
