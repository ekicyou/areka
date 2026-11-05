//! 着せ替え（interval が `bind` 種の animation）の描く手順を、読み込みのときに 1 度だけ作る
//! （seriko-pattern-calls 要件 5.5・6.5・6.7・6.8）。
//!
//! 純 `bind` だけが手順を持ち、全部の pattern を番号の昇順にたどる。`bind+random` は手順を持たない
//! （コマがあるときだけ描く・animated-surface-log-flood 件 26。手順を作ると入れ子の表が描かない先を
//! 見える部品に数える）。描く語で番号が 0 以上のコマは [`BindStep::Draw`]、`insert` で相手が同じ面の
//! 純 `bind` なら [`BindStep::Insert`]、それ以外は手順に入れない。記録は出さない（描かない語は `shape_issues`、8 語は seriko が 1 度出す）。
//!
//! 合成はコマの無い有効な着せ替えの寄与としてこの手順を積み、`insert,B` の位置に B の寄与を
//! B 自身の座標で積む（[`push_bind_contribution`]・要件 5.7・6.1〜6.4・SSP の測り M6）。

use std::collections::{BTreeMap, HashSet};

use areka_parsers::shell::Interval;
use bevy_ecs::prelude::Resource;

use crate::method::is_implemented_name;
use crate::pattern_word::{PatternWord, call_targets, pattern_word};
use crate::plan::{Level, PlanScratch, push_direct};
use crate::world::EmoWorld;

/// 着せ替えの手順の 1 段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindStep {
    /// 面 `surface` を pattern の (x, y) に描く。
    Draw { surface: u32, x: i64, y: i64 },
    /// 同じ面の純 `bind` の animation `target` をこの位置に差し込む。
    Insert { target: u32 },
}

/// 面の番号 × animation の番号 → 手順の列（読み込みのときに 1 度作る・以後は読むだけ）。
#[derive(Debug, Default, Resource)]
pub(crate) struct BindSteps(BTreeMap<(u32, u32), Box<[BindStep]>>);

impl BindSteps {
    /// 面の表の全部の着せ替えから手順を作る。
    pub(crate) fn build(world: &EmoWorld) -> BindSteps {
        let mut steps = BTreeMap::new();
        for surface in world.surface_ids() {
            let Some(master) = world.surface(surface) else {
                continue;
            };
            // 同じ面の純 `bind` の番号（`insert` の相手を 1 度で引く）。
            let pure: HashSet<u32> = master
                .animations
                .iter()
                .filter(|a| matches!(a.interval, Interval::Bind))
                .map(|a| a.id)
                .collect();
            for anim in &master.animations {
                if !matches!(anim.interval, Interval::Bind) {
                    continue;
                }
                let mut patterns: Vec<_> = anim.patterns.iter().collect();
                patterns.sort_by_key(|p| p.index);
                let list = patterns
                    .into_iter()
                    .filter_map(|p| {
                        let method = p.method.as_str();
                        match pattern_word(method) {
                            PatternWord::Draw if is_implemented_name(method) => {
                                let surface = u32::try_from(p.surface_id).ok()?;
                                Some(BindStep::Draw {
                                    surface,
                                    x: p.x,
                                    y: p.y,
                                })
                            }
                            PatternWord::Insert => {
                                let target =
                                    *call_targets(PatternWord::Insert, &p.target).ok()?.first()?;
                                pure.contains(&target)
                                    .then_some(BindStep::Insert { target })
                            }
                            _ => None,
                        }
                    })
                    .collect();
                steps.insert((surface, anim.id), list);
            }
        }
        BindSteps(steps)
    }

    /// 面 `surface` の animation `animation` の手順（純 `bind` でなければ空）。
    pub(crate) fn get(&self, surface: u32, animation: u32) -> &[BindStep] {
        self.0.get(&(surface, animation)).map_or(&[], |s| s)
    }
}

/// 1 つの段で、どの着せ替え B をどの着せ替え A が差し込むかを決め、`s.claims` に B → A で置く（B は 1 度だけ）。
///
/// 描く順（`s.merged_ids`）に有効でコマの無い着せ替えの手順をたどり、`insert,B` の B が有効で、まだ誰にも
/// 差し込まれておらず、展開中の祖先でもなければ A のものにする（巡回は打ち切る・2 つの差し込み手は
/// 描く順で先の方だけ・6.4）。差し込まれた B は本来の位置では積まない（6.1）。差し込む A が無効か
/// コマを持つ（展開されない）なら B は誰のものにもならず、本来の位置に積む（設計議題 4）。
pub(crate) fn claim_inserts(s: &mut PlanScratch, level: &Level<'_>) {
    s.claims.clear();
    s.expanded.clear();
    for k in 0..s.merged_ids.len() {
        let id = s.merged_ids[k];
        claim_from(s, level, id);
    }
}

/// `a` から差し込みの連鎖を、作業場の小さなスタック（`s.chain`＝展開中の祖先と手順の読み位置）でたどる。
fn claim_from(s: &mut PlanScratch, level: &Level<'_>, root: u32) {
    open_claim(s, level, root);
    while let Some(top) = s.chain.last_mut() {
        let (a, i) = *top;
        let Some(&step) = level.world.bind_steps(level.surface_id, a).get(i) else {
            s.chain.pop();
            continue;
        };
        top.1 += 1;
        let BindStep::Insert { target: b } = step else {
            continue;
        };
        // 展開中の祖先（`s.chain`）は根 `root` のほかは差し込み済み（`s.claims` に在る）なので、
        // 祖先の確かめは根と比べて表を引くだけで足りる（列を順に引かない・指摘 26）。
        if level.binds.contains(b) && b != root && !s.claims.contains_key(&b) {
            s.claims.insert(b, a);
            open_claim(s, level, b);
        }
    }
}

/// 着せ替え `a` の手順をまだたどっておらず、展開するなら、たどり済みにしてスタックへ積む。
fn open_claim(s: &mut PlanScratch, level: &Level<'_>, a: u32) {
    if !s.expanded.contains(&a) && expands(level, a) {
        s.expanded.insert(a);
        s.chain.push((a, 0));
    }
}

/// 着せ替え `id` の手順を展開するか（有効で、手順を置き換える今のコマが無い）。
fn expands(level: &Level<'_>, id: u32) -> bool {
    level.binds.contains(id) && level.frame(id).is_none()
}

/// コマの無い有効な着せ替え `a` の手順を積み荷にする。`Draw` は段の位置＋コマの座標に入る積み荷、
/// `Insert { B }` は [`claim_inserts`] が B を A のものにしたときだけ、その位置に B の寄与（B の今のコマか
/// B の手順）を B 自身の座標で積む（6.1・6.3・6.7）。`s.done` は段の中で積んだ B（同じ B を 2 度積まない）。
/// `insert` の連鎖は作業場の小さなスタック（`s.chain`）でたどる。`a` の手順は空でないこと（呼び手が
/// [`push_direct`] で確かめる）。
pub(crate) fn push_bind_contribution(s: &mut PlanScratch, level: &Level<'_>, a: u32) {
    s.chain.push((a, 0));
    while let Some(top) = s.chain.last_mut() {
        let (a, i) = *top;
        let Some(&step) = level.world.bind_steps(level.surface_id, a).get(i) else {
            s.chain.pop();
            continue;
        };
        top.1 += 1;
        match step {
            BindStep::Draw { surface, x, y } => s.steps.push(level.enter(surface, x, y)),
            BindStep::Insert { target: b } => {
                if s.claims.get(&b) == Some(&a)
                    && s.done.insert(b)
                    && push_direct(&mut s.steps, &s.anims, level, b)
                {
                    s.chain.push((b, 0));
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "bind_steps_tests.rs"]
mod tests;
