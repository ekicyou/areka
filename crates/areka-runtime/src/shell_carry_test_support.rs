//! `shell_carry_tests.rs` の檻が `areka-runtime` と本体の両方で使う集合の組み立て（写しを作らない）。

use areka_emo_compose::BindSet;

pub fn set(ids: &[u32]) -> BindSet {
    BindSet::from_ids(ids.iter().copied())
}
