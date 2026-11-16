//! `shell_carry_tests.rs` の檻が `areka-runtime` と本体の両方で使う集合の組み立て（写しを作らない・spec: runtime-crate-split 設計「実装の中の討議の結果」1）。

use areka_emo_compose::BindSet;

pub fn set(ids: &[u32]) -> BindSet {
    BindSet::from_ids(ids.iter().copied())
}
