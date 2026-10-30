//! 着せ替え（interval が `bind` 種の animation）の描く手順を、読み込みのときに 1 度だけ作る
//! （seriko-pattern-calls 要件 5.5・6.5・6.7・6.8）。
//!
//! 純 `bind` は全部の pattern を番号の昇順に、`bind+random` は pattern0 だけをたどる
//! （2 コマ目以降は seriko が再生するコマで、静的に描くと閉じ目が目を覆う）。描く語で番号が 0 以上の
//! コマは [`BindStep::Draw`]、純 `bind` の `insert` で相手が同じ面の純 `bind` なら [`BindStep::Insert`]、
//! それ以外は手順に入れない。記録は出さない（描かない語は `shape_issues`、8 語は seriko が 1 度出す）。
//!
//! 合成はコマの無い有効な着せ替えの寄与としてこの手順を積み、`insert,B` の位置に B の寄与を
//! B 自身の座標で積む（[`push_bind_contribution`]・要件 5.7・6.1〜6.4・SSP の測り M6）。

use std::collections::BTreeMap;

use areka_parsers::shell::{Animation, Interval};
use bevy_ecs::prelude::Resource;

use crate::method::is_implemented_name;
use crate::pattern_word::{PatternWord, call_targets, pattern_word};
use crate::plan::{BlitOp, Level, push_contribution};
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
            for anim in &master.animations {
                let pure = match anim.interval {
                    Interval::Bind => true,
                    Interval::BindRandom { .. } => false,
                    _ => continue,
                };
                let mut patterns: Vec<_> = anim
                    .patterns
                    .iter()
                    .filter(|p| pure || p.index == 0)
                    .collect();
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
                            PatternWord::Insert if pure => {
                                let target =
                                    *call_targets(PatternWord::Insert, &p.target).ok()?.first()?;
                                is_pure_bind(&master.animations, target)
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

    /// 面 `surface` の animation `animation` の手順（`bind` 種でなければ空）。
    pub(crate) fn get(&self, surface: u32, animation: u32) -> &[BindStep] {
        self.0.get(&(surface, animation)).map_or(&[], |s| s)
    }
}

/// 1 つの段で、どの着せ替え B をどの着せ替え A が差し込むかを決める（(B, A) の列・B は 1 度だけ）。
///
/// 描く順（`order`）に有効でコマの無い着せ替えの手順をたどり、`insert,B` の B が有効で、まだ誰にも
/// 差し込まれておらず、展開中の祖先でもなければ A のものにする（巡回は打ち切る・2 つの差し込み手は
/// 描く順で先の方だけ・6.4）。差し込まれた B は本来の位置では積まない（6.1）。差し込む A が無効か
/// コマを持つ（展開されない）なら B は誰のものにもならず、本来の位置に積む（設計議題 4）。
pub(crate) fn claim_inserts(level: &Level<'_>, order: &[u32]) -> Vec<(u32, u32)> {
    let mut claims = Vec::new();
    let mut expanded = Vec::new();
    let mut stack = Vec::new();
    for &id in order {
        claim_from(level, id, &mut claims, &mut expanded, &mut stack);
    }
    claims
}

fn claim_from(
    level: &Level<'_>,
    a: u32,
    claims: &mut Vec<(u32, u32)>,
    expanded: &mut Vec<u32>,
    stack: &mut Vec<u32>,
) {
    if expanded.contains(&a) || !expands(level, a) {
        return;
    }
    expanded.push(a);
    stack.push(a);
    for step in level.world.bind_steps(level.surface_id, a) {
        let BindStep::Insert { target: b } = *step else {
            continue;
        };
        if level.binds.contains(b) && !stack.contains(&b) && !claims.iter().any(|&(c, _)| c == b) {
            claims.push((b, a));
            claim_from(level, b, claims, expanded, stack);
        }
    }
    stack.pop();
}

/// 着せ替え `id` の手順を展開するか（有効で、手順を置き換える今のコマが無い）。
fn expands(level: &Level<'_>, id: u32) -> bool {
    level.binds.contains(id) && level.frame(id).is_none()
}

/// コマの無い有効な着せ替え `a` の手順を積む。`Draw` は段の位置＋コマの座標に、`Insert { B }` は
/// [`claim_inserts`] が B を `a` のものにしたときだけ、その位置に B の寄与（B の今のコマか B の手順）を
/// B 自身の座標で積む（6.1・6.3・6.7）。`done` は段の中で積んだ B（同じ B を 2 度積まない）。
pub(crate) fn push_bind_contribution(
    out_ops: &mut Vec<BlitOp>,
    visited: &mut Vec<u32>,
    level: &Level<'_>,
    claims: &[(u32, u32)],
    done: &mut Vec<u32>,
    a: u32,
) {
    let steps = level.world.bind_steps(level.surface_id, a);
    if steps.is_empty() {
        // まばたき等の `bind+random` で pattern0 が無い・描くコマが無い: 静的な寄与は無い（再生は seriko の領分）。
        tracing::debug!(
            target: "areka_emo_compose",
            surface_id = level.surface_id,
            animation_id = a,
            "有効な着せ替えに描く手順が無い: 静的合成へ寄与なし"
        );
        return;
    }
    for step in steps {
        match *step {
            BindStep::Draw { surface, x, y } => level.flatten(out_ops, visited, surface, x, y),
            BindStep::Insert { target: b } => {
                if claims.contains(&(b, a)) && !done.contains(&b) {
                    done.push(b);
                    push_contribution(out_ops, visited, level, claims, done, b);
                }
            }
        }
    }
}

/// 同じ面に、番号 `id` の純 `bind` の animation が在るか。
fn is_pure_bind(animations: &[Animation], id: u32) -> bool {
    animations
        .iter()
        .any(|a| a.id == id && matches!(a.interval, Interval::Bind))
}

#[cfg(test)]
#[path = "bind_steps_tests.rs"]
mod tests;
