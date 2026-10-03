//! 通信中の数（`OnlineCounter`）の兄弟テスト（要件 2.3・2.4・2.5）。
//!
//! 各テストは関数内の `static` で自分の数を持ち、プロセスに 1 つの [`PROCESS`] には触れない
//! （同じ実行ファイルの他のテストの `Status` を揺らさないため）。

use super::*;

#[test]
fn begin_increments_and_drop_restores() {
    static COUNTER: OnlineCounter = OnlineCounter::new();
    assert!(!COUNTER.is_online(), "始まる前は通信していない（要件 2.5）");
    let guard = COUNTER.begin("update");
    assert!(COUNTER.is_online(), "守り手を持つ間は通信中");
    drop(guard);
    assert!(!COUNTER.is_online(), "守り手を落とすと戻る（要件 2.3）");
}

#[test]
fn nested_guards_yield_one_truth_until_the_last_drops() {
    static COUNTER: OnlineCounter = OnlineCounter::new();
    let update = COUNTER.begin("update");
    let fetch = COUNTER.begin("install-fetch");
    assert!(COUNTER.is_online(), "重なっても真偽は 1 つ（要件 2.4）");
    drop(update);
    assert!(
        COUNTER.is_online(),
        "ほかの通信が続く間は通信中のまま（要件 2.3）"
    );
    drop(fetch);
    assert!(!COUNTER.is_online(), "最後の守り手を落とすと戻る");
}

#[test]
fn early_return_restores_the_count() {
    static COUNTER: OnlineCounter = OnlineCounter::new();
    let run = |stop_early: bool| -> u32 {
        let _online = COUNTER.begin("update");
        assert!(COUNTER.is_online());
        if stop_early {
            return 1;
        }
        2
    };
    assert_eq!(run(true), 1);
    assert!(!COUNTER.is_online(), "早期 return でも戻る");
    assert_eq!(run(false), 2);
    assert!(!COUNTER.is_online(), "通常の終わりでも戻る");
}

#[test]
fn question_mark_error_restores_the_count() {
    static COUNTER: OnlineCounter = OnlineCounter::new();
    let fetch = || -> Result<(), String> {
        let _online = COUNTER.begin("install-fetch");
        assert!(COUNTER.is_online());
        Err::<(), String>("取得に失敗".to_owned())?;
        Ok(())
    };
    assert!(fetch().is_err());
    assert!(!COUNTER.is_online(), "`?` で抜けても戻る");
}

#[test]
fn panic_restores_the_count() {
    static COUNTER: OnlineCounter = OnlineCounter::new();
    // 閉包の中の観測は panic と区別できるよう外の旗へ写す（中の assert の panic で緑に化けない）。
    static SEEN_ONLINE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let result = std::panic::catch_unwind(|| {
        let _online = COUNTER.begin("update");
        SEEN_ONLINE.store(COUNTER.is_online(), std::sync::atomic::Ordering::SeqCst);
        panic!("通信の途中で落ちる");
    });
    assert!(result.is_err());
    assert!(
        SEEN_ONLINE.load(std::sync::atomic::Ordering::SeqCst),
        "落ちる前は通信中"
    );
    assert!(!COUNTER.is_online(), "panic で抜けても戻る");
}
