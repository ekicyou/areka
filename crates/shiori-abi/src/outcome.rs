//! `GetOutcome` / `CorrelationToken`（ABI 非公開の Rust 内部表現）。
//!
//! ergonomic 層が raw COM 呼び出しの結果を Rust 風に型化するためのデータ型を定義する
//! （design.md §Components and Interfaces → Ergonomic Layer / §Data Models → CorrelationToken (D2·R3-3),
//! 要件 3.2/3.3/3.5）。
//!
//! - [`GetOutcome`][] — `Get` の成功結果。即時応答 [`GetOutcome::Immediate`]（要件 3.2）と
//!   遅延結果 [`GetOutcome::Deferred`]（要件 3.3）の 2 値分岐。COM 面には出さない内部表現（D4）。
//! - [`CorrelationToken`][] — 遅延リクエストと後続の完了応答（`IShioriHost::Complete`）を突き合わせる
//!   `u64` トークン（design.md §Data Models D2）。
//! - [`CorrelationTokenAllocator`][] — 単調増加採番アロケータ。完了後再利用は単調増加で衝突回避する
//!   （design.md §Data Models「再利用は完了後に許容（…単調増加採番で衝突回避）」）。

use std::sync::atomic::{AtomicU64, Ordering};

use windows_core::HSTRING;

/// `Get` の成功結果（ABI 非公開の Rust 内部表現・D4）。
///
/// ergonomic 層が raw `Get` の HRESULT を判別して構築する（COM 面には出さない）。
/// HRESULT → variant のルーティング（`S_OK`→`Immediate` / `SHIORI_S_PENDING`→`Deferred`）は
/// ergonomic 層の責務であり、本型はその受け皿となるデータ表現を提供する（要件 3.2/3.3/3.5）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GetOutcome {
    /// 即時応答。応答文字列を HSTRING で内包する（要件 3.2）。
    Immediate(HSTRING),
    /// 遅延結果。後続の完了応答と突き合わせる相関トークンを内包する（要件 3.3）。
    Deferred(CorrelationToken),
}

/// 遅延リクエストと後続完了応答を突き合わせる相関トークン（design.md §Data Models D2・R3-3）。
///
/// `u64` ベースの ABI 非公開な Rust 内部表現。in-proc 単一脳前提の最小実装。
/// 寿命は対応する遅延 request の完了まで。完了後の再利用は
/// [`CorrelationTokenAllocator`] の単調増加採番で衝突回避する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CorrelationToken(pub u64);

/// [`CorrelationToken`] の単調増加採番アロケータ（design.md §Data Models「単調増加採番で衝突回避」）。
///
/// 内包する [`AtomicU64`] を fetch-add することで、過去の全採番より厳密に大きい値を発行する。
/// これにより「完了後の再利用」を新規採番では決して過去値を再発行しない形で担保する。
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
    /// `Relaxed` で十分（同期は値の単調性のみが要件で、他メモリ操作との順序付けは不要）。
    pub fn next(&self) -> CorrelationToken {
        CorrelationToken(self.next.fetch_add(1, Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_core::HSTRING;

    // --- GetOutcome 構築（要件 3.2/3.5・design.md §Testing Strategy） ---

    /// 即時応答は応答文字列付きで `Immediate` として構築できること（要件 3.2）。
    #[test]
    fn immediate_holds_response_hstring() {
        let response = HSTRING::from("pong");
        let outcome = GetOutcome::Immediate(response.clone());
        match outcome {
            GetOutcome::Immediate(got) => assert_eq!(got, response),
            other => panic!("expected Immediate, got {other:?}"),
        }
    }

    /// 遅延結果は相関トークン付きで `Deferred` として構築できること（要件 3.3）。
    #[test]
    fn deferred_holds_correlation_token() {
        let token = CorrelationToken(42);
        let outcome = GetOutcome::Deferred(token);
        match outcome {
            GetOutcome::Deferred(got) => assert_eq!(got, CorrelationToken(42)),
            other => panic!("expected Deferred, got {other:?}"),
        }
    }

    /// 即時/遅延は判別可能な別 variant であること（要件 3.5）。
    #[test]
    fn immediate_and_deferred_are_distinct_variants() {
        let immediate = GetOutcome::Immediate(HSTRING::from("x"));
        let deferred = GetOutcome::Deferred(CorrelationToken(1));
        assert!(matches!(immediate, GetOutcome::Immediate(_)));
        assert!(matches!(deferred, GetOutcome::Deferred(_)));
    }

    // --- CorrelationToken の素朴な性質 ---

    /// `CorrelationToken` は内包する `u64` を保持する（design.md §Data Models D2）。
    #[test]
    fn correlation_token_wraps_u64() {
        let token = CorrelationToken(7);
        assert_eq!(token.0, 7u64);
    }

    // --- 採番アロケータ: 単調増加・一意（要件 3.3・design.md §Data Models / §Testing Strategy） ---

    /// 連続 `next()` が厳密に単調増加すること。
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

    /// 連続採番されたトークンに重複が無いこと（一意性）。
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

    /// 完了後再利用ポリシー: 採番は完了に依存せず単調増加し続ける（=過去値を再発行しない）。
    /// design.md §Data Models「再利用は完了後に許容（…単調増加採番で衝突回避）」を
    /// 「新規採番は常に過去の全採番より大きい」という単調性で担保する。
    #[test]
    fn allocator_never_reissues_after_completion() {
        let alloc = CorrelationTokenAllocator::new();
        // 一連の request を採番→完了したと見なして破棄する。
        let issued: Vec<u64> = (0..16).map(|_| alloc.next().0).collect();
        let max_issued = *issued.iter().max().unwrap();
        // 完了後に新規採番しても、過去の最大値より厳密に大きいこと。
        let after = alloc.next();
        assert!(
            after.0 > max_issued,
            "完了後の新規採番は過去採番の最大値より大きいこと: after={}, max_issued={}",
            after.0,
            max_issued
        );
    }

    /// 既定値（`Default`）でも `new()` と同等に 0 起点から採番すること。
    #[test]
    fn allocator_default_starts_from_zero_base() {
        let alloc = CorrelationTokenAllocator::default();
        assert_eq!(alloc.next(), CorrelationToken(0));
        assert_eq!(alloc.next(), CorrelationToken(1));
    }
}
