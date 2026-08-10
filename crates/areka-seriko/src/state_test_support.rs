use super::*;

/// テスト用 bind 集合（非空・emo2 実測相当の任意 id）。
pub(super) fn binds_1100_1207() -> BindSet {
    BindSet::from_ids([1100, 1207])
}

pub(super) fn empty_states() -> ScopeStates {
    ScopeStates::new(binds_1100_1207())
}
