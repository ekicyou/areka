//! `shell_balloon_resolve_tests.rs` の檻が `areka-runtime` と本体の両方で使う候補の組み立て（写しを作らない・spec: runtime-crate-split 設計「実装の中の討議の結果」1）。

use super::*;

pub fn by_name(name: &str) -> SkinSpec {
    SkinSpec::Name(name.to_owned())
}

pub fn never(_: usize) -> usize {
    panic!("乱数は引かない")
}

pub const NO_MEMO: Result<&str, NotFoundReason> = Err(NotFoundReason::LastInstalledNone);

/// 解いた先のフォルダ名（失敗は理由のまま）。
pub fn folder(result: Result<SkinCandidate, NotFoundReason>) -> Result<String, NotFoundReason> {
    result.map(|c| c.folder)
}
