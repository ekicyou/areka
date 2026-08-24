//! 自発書込カウンタ（[`SELF_INITIATED_DEPTH`](super::SELF_INITIATED_DEPTH)）が
//! **スレッドごとに独立している**ことの決定論テスト（要件 6.6・設計 C20）。
//!
//! # 何を固定するのか
//!
//! `SetWindowPos`／`EndDeferWindowPos` は `WM_WINDOWPOSCHANGED` を**呼び出したスレッドの上で
//! 同期送達**する。よって「いま自発書込の内側か」は本来スレッドごとの問いであり、プロセス全体で
//! 1 個の値を共有する理由が無い。共有していた間は、あるテストの書込経路が持ち上げた値を、
//! 並列に走る無関係なテストの [`is_self_initiated`](super::is_self_initiated) が読んでしまっていた。
//!
//! ここで固定するのは 3 点。
//!
//! - 別スレッドがガードを持ち上げている**最中**に、こちらのスレッドの判定が偽であること
//! - 持ち上げている当のスレッドでは真であること（局所化が判定そのものを殺していないこと）
//! - 同一スレッド上の入れ子は、最後の 1 枚が落ちるまで真であり続けること
//!
//! # 錠を取らないことが要点
//!
//! 本ファイルは [`lock_self_initiated_for_test`](super::lock_self_initiated_for_test) を
//! **意図的に取らない**。錠なしで、しかも自分で別スレッドを起こして並列に走らせたうえで緑になる
//! ——これがスレッド局所化の成立そのものである。錠を足すとこのテストは何も測らなくなる。
//!
//! # 待ち合わせは channel だけ
//!
//! 順序付けに `sleep` も実時間の閾値も使わない（要件 6.5）。「持ち上げた」「もう落としてよい」の
//! 2 つを channel の送受信で受け渡し、遅い機械でも速い機械でも同じ判定になるようにする。

use std::sync::mpsc;
use std::thread;

use super::{SetWindowPosGuard, is_self_initiated};

/// 別スレッドがガードを持ち上げている最中、こちらの判定は偽のままである。
///
/// 是正前（プロセス共有の `AtomicI32`）はここが真になり、このテストは赤になる。
#[test]
fn a_guard_held_on_another_thread_is_invisible_from_this_thread() {
    // 持ち上げ側 → 主スレッド: 「持ち上げた。そちらから見た値を確かめてよい」
    let (raised_tx, raised_rx) = mpsc::channel::<bool>();
    // 主スレッド → 持ち上げ側: 「確かめ終えた。落としてよい」
    let (release_tx, release_rx) = mpsc::channel::<()>();

    let worker = thread::spawn(move || {
        let guard = SetWindowPosGuard::new();
        // 持ち上げている当のスレッドでは真であること。
        raised_tx
            .send(is_self_initiated())
            .expect("主スレッドが受け取る前に落ちてはならない");
        release_rx
            .recv()
            .expect("主スレッドが解放を指示する前に channel が閉じてはならない");
        drop(guard);
        // 落とした後は当のスレッドでも偽へ戻ること。
        is_self_initiated()
    });

    let seen_by_holder = raised_rx
        .recv()
        .expect("持ち上げ側が送る前に channel が閉じてはならない");
    assert!(
        seen_by_holder,
        "ガードを持ち上げた当のスレッドでは is_self_initiated() が真であること"
    );

    // ここが本題: 別スレッドの持ち上げはこちらから見えない。
    assert!(
        !is_self_initiated(),
        "別スレッドが持ち上げたガードがこちらのスレッドの判定を汚してはならない"
    );

    release_tx.send(()).expect("持ち上げ側が生きていること");
    let seen_after_drop = worker.join().expect("持ち上げ側がパニックしていないこと");
    assert!(
        !seen_after_drop,
        "ガードを落とした後は当のスレッドでも偽へ戻ること"
    );

    // 相手が落とした後もこちらは終始偽のまま。
    assert!(
        !is_self_initiated(),
        "他スレッドの解放がこちらの判定を動かしてはならない"
    );
}

/// 同一スレッド上の入れ子は、最後の 1 枚が落ちるまで真であり続ける。
///
/// 新しいスレッドを起こして走らせるのは、カウンタが 0 から始まることを局所化の側から
/// 保証するため（テストハーネスのスレッドの履歴に依存しない）。
#[test]
fn nested_guards_on_one_thread_stay_true_until_the_last_is_dropped() {
    let observed = thread::spawn(|| {
        let at_rest = is_self_initiated();

        let outer = SetWindowPosGuard::new();
        let with_one = is_self_initiated();

        let inner = SetWindowPosGuard::new();
        let with_two = is_self_initiated();

        drop(inner);
        let after_inner = is_self_initiated();

        drop(outer);
        let after_both = is_self_initiated();

        (at_rest, with_one, with_two, after_inner, after_both)
    })
    .join()
    .expect("観測スレッドがパニックしていないこと");

    let (at_rest, with_one, with_two, after_inner, after_both) = observed;
    assert!(!at_rest, "新しいスレッドのカウンタは 0 から始まること");
    assert!(with_one, "1 枚目で真になること");
    assert!(with_two, "入れ子でも真のままであること");
    assert!(
        after_inner,
        "内側 1 枚を落としても外側が残るので真のままであること"
    );
    assert!(!after_both, "最後の 1 枚を落としたら偽へ戻ること");
}

/// 2 本のスレッドが同時に持ち上げても、互いの深度を混ぜない。
///
/// 双方が「自分は真」と名乗り、どちらが先に落ちても他方の判定は動かない。主スレッドは
/// 終始偽である。
#[test]
fn two_threads_hold_their_own_depth_without_mixing() {
    let (report_tx, report_rx) = mpsc::channel::<(u8, bool)>();
    let (release_a_tx, release_a_rx) = mpsc::channel::<()>();
    let (release_b_tx, release_b_rx) = mpsc::channel::<()>();

    let tx_a = report_tx.clone();
    let a = thread::spawn(move || {
        let guard = SetWindowPosGuard::new();
        tx_a.send((0, is_self_initiated()))
            .expect("受け手が生きていること");
        release_a_rx.recv().expect("解放指示が来ること");
        drop(guard);
        is_self_initiated()
    });

    let b = thread::spawn(move || {
        let guard = SetWindowPosGuard::new();
        report_tx
            .send((1, is_self_initiated()))
            .expect("受け手が生きていること");
        release_b_rx.recv().expect("解放指示が来ること");
        drop(guard);
        is_self_initiated()
    });

    // 2 本とも持ち上げ終えるまで待つ（順不同）。
    let first = report_rx.recv().expect("1 本目の報告が来ること");
    let second = report_rx.recv().expect("2 本目の報告が来ること");
    assert!(
        first.1 && second.1,
        "各スレッドが自分の持ち上げを真と見ること"
    );
    assert_ne!(
        first.0, second.0,
        "2 本の別々のスレッドから 1 回ずつ届くこと"
    );

    // 2 本が同時に持ち上げている最中でも主スレッドは偽。
    assert!(
        !is_self_initiated(),
        "他スレッド 2 本ぶんの深度が主スレッドへ漏れてはならない"
    );

    // 片方だけ落としても、もう片方の判定は動かない。
    release_a_tx.send(()).expect("A が生きていること");
    let a_after = a.join().expect("A がパニックしていないこと");
    assert!(!a_after, "A は落とした後 偽へ戻ること");
    assert!(!is_self_initiated(), "主スレッドは終始 偽であること");

    release_b_tx.send(()).expect("B が生きていること");
    let b_after = b.join().expect("B がパニックしていないこと");
    assert!(!b_after, "B は落とした後 偽へ戻ること");
    assert!(!is_self_initiated(), "主スレッドは終始 偽であること");
}
