//! 定義だけで決まる形の誤り（[`ShapeIssue`]）を面の表から集める（areka-P0-offline-surface-dump
//! 要件 7.5・design ShapeIssuesOnce）。
//!
//! 合成は面を組むたびに同じ誤りに出会うので、その知らせは `debug!` に留め、ここで集めた誤りを
//! 読み込みの側がシェルごとに 1 度だけ warn で出す。判定は合成と読み込みの報告が使う関数
//! （[`pattern_edge`]・[`nest_edges`]・[`reaches`]・[`is_implemented_name`]）から引き、写さない。
//! element定義の辺の循環は [`crate::NestReport`] が既に持つので、ここは pattern定義の辺だけを見る。
//! 記録は出さない（広がりの予算が尽きたときの `error!` だけ出す）。
//!
//! `animation-sort` の知らない値は転記層が原文のまま [`SortOrder::Other`] で運ぶので、その 1 つを
//! シェル全体の誤りとして集める（合成は既定 `descend` として組む）。

use std::collections::HashMap;

use areka_parsers::shell::SortOrder;

use crate::budget::VISIT_BUDGET;
use crate::method::is_implemented_name;
use crate::nesting::{nest_edges, pattern_edge, reaches};
use crate::world::EmoWorld;

/// 定義だけで決まる形の誤り 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapeIssue {
    /// `animation-sort` の値（原文）が `ascend`／`descend` でない。シェル全体に 1 つの欄なので面の番号は
    /// 持たない。合成は既定 `descend` として組む。
    UnknownAnimationSort { value: String },
    /// 面 `surface` の animation `animation` の pattern `pattern` が指す `target` から、入れ子の辺を
    /// たどると `surface` へ戻る（自分自身を指す場合を含む）。合成はその枝を打ち切る。
    Cycle {
        surface: u32,
        animation: u32,
        pattern: u32,
        target: u32,
    },
    /// 面を指す pattern の描画メソッドが描かないもの（`method` は欄の原文）。合成はそのコマを描かない。
    /// 面の番号を指さない語（`start` 等の 7 語・`move`）と負の番号は数えない。
    UndrawnPattern {
        surface: u32,
        animation: u32,
        pattern: u32,
        method: String,
    },
}

/// 面の表から形の誤りを集める。並びは知らない `animation-sort` が先頭、続いて面の番号の昇順 → animation の書いた順 → pattern の書いた順で、
/// 1 つの pattern が両方に当たれば `Cycle` が先。
pub fn shape_issues(world: &EmoWorld) -> Vec<ShapeIssue> {
    shape_issues_limited(world, VISIT_BUDGET)
}

/// [`shape_issues`] の中身。`limit` は [`reaches`] 1 回の予算（本番は [`VISIT_BUDGET`]・テストは小さな数）。
/// 予算が尽きた `error!` は呼び出し 1 回につき 1 度だけ（指摘 2）。
pub(crate) fn shape_issues_limited(world: &EmoWorld, limit: usize) -> Vec<ShapeIssue> {
    let edges = nest_edges(world);
    let mut reported = false;
    // 同じ（先, 面）の組は答えが同じなので 1 度だけたどる（指摘 132 と同じ型）。
    let mut cycles: HashMap<(u32, u32), bool> = HashMap::new();
    let mut issues = Vec::new();
    if let SortOrder::Other(value) = world.animation_sort() {
        issues.push(ShapeIssue::UnknownAnimationSort {
            value: value.to_string(),
        });
    }
    for surface in world.surface_ids() {
        let Some(master) = world.surface(surface) else {
            continue;
        };
        for anim in &master.animations {
            for p in &anim.patterns {
                let method = p.method.as_str();
                let Some(target) = pattern_edge(p) else {
                    continue;
                };
                let cycle = *cycles
                    .entry((target, surface))
                    .or_insert_with(|| reaches(&edges, target, surface, limit, &mut reported));
                if cycle {
                    issues.push(ShapeIssue::Cycle {
                        surface,
                        animation: anim.id,
                        pattern: p.index,
                        target,
                    });
                }
                if !is_implemented_name(method) {
                    issues.push(ShapeIssue::UndrawnPattern {
                        surface,
                        animation: anim.id,
                        pattern: p.index,
                        method: method.to_string(),
                    });
                }
            }
        }
    }
    issues
}

#[cfg(test)]
#[path = "plan_issues_tests.rs"]
mod tests;
