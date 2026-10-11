//! `wait` のテストの支え: 時刻の起点と、待つもの（[`WaitSpec`]）の組み立て。
//! 判定のテストとループのテストの両方が使う。

use super::WaitSpec;

/// テストの時刻の起点（2026-10-03T04:00:00Z）。
pub(super) const T0: u64 = 1_791_000_000;

pub(super) fn watch(id: &str) -> WaitSpec {
    WaitSpec::Watch {
        id: id.to_owned(),
        repo: "areka".to_owned(),
        name: None,
    }
}

pub(super) fn merge(id: &str, repo: &str) -> WaitSpec {
    WaitSpec::Merge {
        id: id.to_owned(),
        repo: repo.to_owned(),
    }
}

pub(super) fn load(id: &str) -> WaitSpec {
    WaitSpec::Load { id: id.to_owned() }
}

pub(super) fn resume(id: &str) -> WaitSpec {
    WaitSpec::Resume { id: id.to_owned() }
}
