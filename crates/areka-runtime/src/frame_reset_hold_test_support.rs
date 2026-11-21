//! 位置の預かりの檻へ、スコープごとの表示の決まりを答える偽の読み口。

use std::collections::{BTreeMap, BTreeSet};

use areka_emo_compose::ScaleRatio;
use areka_emo_present::NeverShown;

use super::move_hold::ShownRatioSource;

/// 表示したスコープ（拡大率 1）・表示しないまま決まったスコープ・実表示寸を答える読み口。
/// どれにも入れないスコープは「まだ決まっていない」。
#[derive(Debug, Default, Clone)]
pub struct SettledSource {
    shown: BTreeSet<u32>,
    unshown: BTreeMap<u32, NeverShown>,
    sizes: BTreeMap<u32, (u32, u32)>,
}

impl SettledSource {
    pub fn new() -> Self {
        Self::default()
    }

    /// `scope` が実表示寸 `size` で最初の面を表示した。
    pub fn shown(mut self, scope: u32, size: (u32, u32)) -> Self {
        self.shown.insert(scope);
        self.sizes.insert(scope, size);
        self
    }

    /// `scope` が `reason` で最初の面を表示しないまま決まった。
    pub fn unshown(mut self, scope: u32, reason: NeverShown) -> Self {
        self.unshown.insert(scope, reason);
        self
    }
}

impl ShownRatioSource for SettledSource {
    fn shown_ratio(&self, scope: u32) -> Option<ScaleRatio> {
        self.shown.contains(&scope).then_some(ScaleRatio::ONE)
    }

    fn never_shown(&self, scope: u32) -> Option<NeverShown> {
        self.unshown.get(&scope).copied()
    }

    fn physical_size(&self, scope: u32) -> Option<(u32, u32)> {
        self.sizes.get(&scope).copied()
    }
}
