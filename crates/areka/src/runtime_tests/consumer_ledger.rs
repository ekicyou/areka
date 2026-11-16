//! `areka_runtime::consumer_ledger` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::consumer_ledger::*;
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "consumer_ledger_agreement_tests.rs"]
mod agreement_tests;
