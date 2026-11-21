//! `GetOutcome` / `CorrelationToken`（ABI 非公開の Rust 内部表現）。
//!
//! - [`GetOutcome`][] — `Get` の成功結果。即時応答 [`GetOutcome::Immediate`] と
//!   遅延結果 [`GetOutcome::Deferred`] の 2 値分岐。COM 面には出さない内部表現。
//! - [`CorrelationToken`][] — 遅延リクエストと後続の完了応答（`IShioriHost::Complete`）を突き合わせる
//!   `u64` トークン。
//! - [`CorrelationTokenAllocator`][] — 単調増加採番アロケータ。

use std::sync::atomic::{AtomicU64, Ordering};

use windows_core::HSTRING;

/// `Get` の成功結果（ABI 非公開の Rust 内部表現）。
///
/// ergonomic 層が raw `Get` の HRESULT を判別して構築する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetOutcome {
    /// 即時応答。
    Immediate(HSTRING),
    /// 遅延結果。後続の完了応答と突き合わせる相関トークンを内包する。
    Deferred(CorrelationToken),
}

/// 遅延リクエストと後続完了応答を突き合わせる相関トークン。
///
/// in-proc 単一脳前提の最小実装。寿命は対応する遅延 request の完了まで。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CorrelationToken(pub u64);

/// [`CorrelationToken`] の単調増加採番アロケータ。過去に発行した値を再発行しない。
#[derive(Debug, Default)]
pub struct CorrelationTokenAllocator {
    next: AtomicU64,
}

impl CorrelationTokenAllocator {
    /// 0 起点のアロケータを生成する。
    pub const fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
        }
    }

    /// 次の相関トークンを採番する。連続呼び出しは厳密に単調増加し、重複しない。
    ///
    /// `Relaxed` で十分（要るのは値の単調性だけで、他のメモリ操作との順序付けは不要）。
    pub fn next(&self) -> CorrelationToken {
        CorrelationToken(self.next.fetch_add(1, Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_core::HSTRING;

    // --- GetOutcome 構築 ---

    #[test]
    fn immediate_holds_response_hstring() {
        let response = HSTRING::from("pong");
        let outcome = GetOutcome::Immediate(response.clone());
        match outcome {
            GetOutcome::Immediate(got) => assert_eq!(got, response),
            other => panic!("expected Immediate, got {other:?}"),
        }
    }

    #[test]
    fn deferred_holds_correlation_token() {
        let token = CorrelationToken(42);
        let outcome = GetOutcome::Deferred(token);
        match outcome {
            GetOutcome::Deferred(got) => assert_eq!(got, CorrelationToken(42)),
            other => panic!("expected Deferred, got {other:?}"),
        }
    }

    #[test]
    fn immediate_and_deferred_are_distinct_variants() {
        let immediate = GetOutcome::Immediate(HSTRING::from("x"));
        let deferred = GetOutcome::Deferred(CorrelationToken(1));
        assert!(matches!(immediate, GetOutcome::Immediate(_)));
        assert!(matches!(deferred, GetOutcome::Deferred(_)));
    }

    // --- CorrelationToken の素朴な性質 ---

    #[test]
    fn correlation_token_wraps_u64() {
        let token = CorrelationToken(7);
        assert_eq!(token.0, 7u64);
    }

    // --- 採番アロケータ: 単調増加・一意 ---

    #[test]
    fn allocator_is_strictly_monotonic() {
        let alloc = CorrelationTokenAllocator::new();
        let mut prev = alloc.next();
        for _ in 0..1000 {
            let cur = alloc.next();
            assert!(
                cur.0 > prev.0,
                "トークンは厳密に単調増加であること: prev={}, cur={}",
                prev.0,
                cur.0
            );
            prev = cur;
        }
    }

    #[test]
    fn allocator_yields_unique_tokens() {
        let alloc = CorrelationTokenAllocator::new();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            let token = alloc.next();
            assert!(
                seen.insert(token.0),
                "トークンは一意であること: {}",
                token.0
            );
        }
    }

    /// 採番は完了に依存せず単調増加し続ける（過去値を再発行しない）。
    #[test]
    fn allocator_never_reissues_after_completion() {
        let alloc = CorrelationTokenAllocator::new();
        let issued: Vec<u64> = (0..16).map(|_| alloc.next().0).collect();
        let max_issued = *issued.iter().max().unwrap();
        let after = alloc.next();
        assert!(
            after.0 > max_issued,
            "完了後の新規採番は過去採番の最大値より大きいこと: after={}, max_issued={}",
            after.0,
            max_issued
        );
    }

    #[test]
    fn allocator_default_starts_from_zero_base() {
        let alloc = CorrelationTokenAllocator::default();
        assert_eq!(alloc.next(), CorrelationToken(0));
        assert_eq!(alloc.next(), CorrelationToken(1));
    }
}
