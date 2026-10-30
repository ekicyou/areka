//! 着せ替え（interval が `bind` 種の animation）の描く手順を、読み込みのときに 1 度だけ作る
//! （seriko-pattern-calls 要件 5.5・6.5・6.7・6.8）。
//!
//! 純 `bind` は全部の pattern を番号の昇順に、`bind+random` は pattern0 だけをたどる
//! （2 コマ目以降は seriko が再生するコマで、静的に描くと閉じ目が目を覆う）。描く語で番号が 0 以上の
//! コマは [`BindStep::Draw`]、純 `bind` の `insert` で相手が同じ面の純 `bind` なら [`BindStep::Insert`]、
//! それ以外は手順に入れない。記録は出さない（描かない語は `shape_issues`、8 語は seriko が 1 度出す）。

use std::collections::BTreeMap;

use areka_parsers::shell::{Animation, Interval};
use bevy_ecs::prelude::Resource;

use crate::method::is_implemented_name;
use crate::pattern_word::{PatternWord, call_targets, pattern_word};
use crate::world::EmoWorld;

/// 着せ替えの手順の 1 段。
// shortcut: 合成（task 2.2）が使うまで lib のビルドでは読まれない。2.2 で allow を外す。
#[allow(dead_code)]
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

/// 同じ面に、番号 `id` の純 `bind` の animation が在るか。
fn is_pure_bind(animations: &[Animation], id: u32) -> bool {
    animations
        .iter()
        .any(|a| a.id == id && matches!(a.interval, Interval::Bind))
}

#[cfg(test)]
#[path = "bind_steps_tests.rs"]
mod tests;
