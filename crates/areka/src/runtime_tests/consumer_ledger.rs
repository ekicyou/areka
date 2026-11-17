//! `areka_runtime::consumer_ledger` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::consumer_ledger::*;

#[cfg(test)]
#[path = "consumer_ledger_agreement_tests.rs"]
mod agreement_tests;
