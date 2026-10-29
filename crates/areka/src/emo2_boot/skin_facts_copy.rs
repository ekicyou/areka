//! 表示の層から今のシェルとバルーンの事実（[`SkinFacts`]）を写す（spec: mcp-strict-errors）。
//!
//! `check_script` と strict の記録が同じ写しを使う。量はシェルの大きさで決まり台本の長さに依らない。

use std::collections::BTreeMap;

use areka_emo_present::EmoPresenter;
use areka_seriko::SkinFacts;

use crate::emo2_boot::target_map::{balloon_target, shell_target};

/// 窓を持つスコープごとに、表示の層から写す。登録の無い target は入れない（その scope は診ない）。
pub(crate) fn copy(presenter: &EmoPresenter, scopes: impl Iterator<Item = u32>) -> SkinFacts {
    let mut shells = BTreeMap::new();
    let mut balloons = BTreeMap::new();
    for scope in scopes {
        if let Some(ids) = presenter.surface_ids(shell_target(scope)) {
            shells.insert(scope, ids);
        }
        if let Some(ids) = presenter.surface_ids(balloon_target(scope)) {
            balloons.insert(scope, ids);
        }
    }
    let aliases = presenter
        .alias_snapshot(shell_target(0))
        .unwrap_or_default();
    SkinFacts::new(aliases, shells, balloons)
}
