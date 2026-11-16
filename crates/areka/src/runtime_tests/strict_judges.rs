//! `areka_runtime::strict_judges` の代わりの親（子のテストの宣言だけ）。本体に残る子（`doc_tables`）は
//! 本物の親の項目を道で引くので、取り込みの行を持たない。

#[cfg(test)]
#[path = "strict_judges_tests.rs"]
mod strict_judges_tests;
