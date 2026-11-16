//! `areka_runtime::strict_judges` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::strict_judges::*;
use std::sync::Arc;
use areka_sakura::strict::DetectionPoint;
use areka_sakura::strict::StrictJudges;
use areka_runtime::consumer_ledger::ConsumerLedger;

#[cfg(test)]
#[path = "strict_judges_tests.rs"]
mod strict_judges_tests;
