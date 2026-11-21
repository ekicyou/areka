//! 正準台帳の見本の組が表の行とちょうど同じ組であることの檻。
//!
//! 見本の一覧は表の行と一致すること自体を判定する（行を足して見本を足し忘れれば赤）。
//! `runtime` の受け口へ見本を実際に配って一致を見る檻は、`areka` の `runtime_tests/` の同じ名前の
//! ファイルにある。

use std::collections::BTreeSet;

use super::test_support::{ledger_keys, samples};
use super::{ConsumerLedger, LedgerKey};

// ---------------------------------------------------------------- 檻

/// 見本の一覧は表の行とちょうど同じ組を持つ（行を足して見本を足し忘れても、見本だけ残っても赤）。
#[test]
fn samples_cover_exactly_the_ledger_rows() {
    let ledger = ConsumerLedger::canonical();
    let sampled: BTreeSet<LedgerKey> = samples()
        .into_iter()
        .map(|(name, selector, _)| (name.to_string(), selector.map(str::to_string)))
        .collect();
    assert_eq!(
        sampled,
        ledger_keys(&ledger),
        "見本の組と正準台帳の行の組は同じであること"
    );
    for (name, selector, args) in samples() {
        if let Some(selector) = selector {
            assert_eq!(
                args.first(),
                Some(&selector),
                "{name}: 選別子つきの行の見本は、第 1 引数がその選別子であること"
            );
        }
    }
}
