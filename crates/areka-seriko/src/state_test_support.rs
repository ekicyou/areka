use super::*;

// テストの道具は層をまたぐ受け口の 1 か所の定義（spec: audit-seriko 指摘 6）。
pub(super) use crate::test_support::{capture_text, fresh_states};

/// テスト用 bind 集合（非空・emo2 実測相当の任意 id）。
pub(super) fn binds_1100_1207() -> BindSet {
    BindSet::from_ids([1100, 1207])
}
