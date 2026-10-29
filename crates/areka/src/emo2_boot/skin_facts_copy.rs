//! 表示の層から今のシェルとバルーンの事実（[`SkinFacts`]）を写す（spec: mcp-strict-errors）。
//!
//! `check_script` と strict の記録が同じ写しを使う。量はシェルの大きさで決まり台本の長さに依らない。
//! シェル・バルーンの差し替えでは、差し替えの荷物から新しい事実を組む（[`swapped`]）。

use std::collections::BTreeMap;

use areka_emo_present::EmoPresenter;
use areka_seriko::SkinFacts;

use crate::emo2_boot::switch_assets::SwapBuilt;
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

/// 差し替えの荷物から差し替えの後の事実を組む。シェルは別名の表とシェルを、バルーンはバルーンだけを
/// 替え、もう片方は表示の層の今の事実のまま。窓を持つスコープだけを入れる（[`copy`] と同じ）。
pub(crate) fn swapped(presenter: &EmoPresenter, scopes: &[u32], built: &SwapBuilt) -> SkinFacts {
    let now = copy(presenter, scopes.iter().copied());
    match built {
        SwapBuilt::Shell { assets, .. } => {
            // 別名の表はスコープに依らない（`assets.rs` の解決器と同じく先頭の World から採る）。
            let aliases = assets
                .shells
                .first()
                .map(|s| s.emo_world.alias_snapshot())
                .unwrap_or_default();
            let shells = assets
                .shells
                .iter()
                .filter(|s| scopes.contains(&s.scope))
                .map(|s| (s.scope, s.emo_world.surface_ids().collect()))
                .collect();
            now.with_shells(aliases, shells)
        }
        SwapBuilt::Balloon { assets } => now.with_balloons(
            assets
                .balloons
                .iter()
                .filter(|b| scopes.contains(&b.scope))
                .map(|b| (b.scope, b.emo_world.surface_ids().collect()))
                .collect(),
        ),
    }
}

#[cfg(test)]
#[path = "skin_facts_copy_tests.rs"]
mod tests;
