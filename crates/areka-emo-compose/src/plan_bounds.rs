//! シェル全体の外形（[`ShellBounds`]）の算出（areka-P0-offline-surface-dump 要件 4.3）。
//!
//! 面ごとの外形（`plan_extent.rs` の `compute_extent_limited`）と同じ辿り方（入れ子・経路にある面の集合で循環を
//! 避ける・明示のスタック・`outline_walk.rs` の 1 本）を使い、座標を符号つきで持つ。違いは、⑴ すべての面を数える、⑵ 負の座標を原点で
//! 切らない、⑶ すべての animation（interval を問わない・`never`・`runonce` を含む）のすべての
//! pattern を数える、の 3 点。記録は出さない（読み込みと合成が既に出している・広がりの予算が尽きたときの `error!` だけ出す）。
//! 右下は面ごとの外形（合成の絵の大きさ・束縛のときに求めた表）も重ねて、どの面の絵も外形に収める。

use areka_emo_atlas::AtlasTable;

use crate::budget::VISIT_BUDGET;
use crate::method::is_implemented_name;
use crate::nesting::ElementKind;
use crate::outline_walk::{Memo, Outline, Scratch, Step};
use crate::plan::surface_and_binding;
use crate::world::EmoWorld;

/// 土台の原点を (0,0) とする外形。(0,0) を必ず含む（`left, top <= 0`・`right, bottom >= 0`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellBounds {
    /// 左端（px・0 以下）。
    pub left: i32,
    /// 上端（px・0 以下）。
    pub top: i32,
    /// 右端（px・0 以上）。
    pub right: i32,
    /// 下端（px・0 以上）。
    pub bottom: i32,
}

/// シェルのすべての面の土台・element・element定義が置くサーフェス・すべての animation の描く
/// pattern を土台の原点に重ねた外形を返す（要件 4.3）。
///
/// 着せ替えにも `PatternState` にもよらない（1 つのシェルで 1 つ）。負の番号の pattern と描かない
/// メソッドの pattern は数えない。未束縛の element（原寸不明）は数えない。
pub fn shell_bounds(world: &EmoWorld, atlas: &AtlasTable) -> ShellBounds {
    shell_bounds_limited(world, atlas, VISIT_BUDGET)
}

/// [`shell_bounds`] の中身。`limit` は根の面 1 つの広がりの予算（本番は [`VISIT_BUDGET`]・テストは
/// 小さな数）。きれいに終わった面の覚えは根の間で 1 つを共有する。根ごとの外形は表から引く（予算は表を
/// 作ったときの [`VISIT_BUDGET`]）。
pub(crate) fn shell_bounds_limited(
    world: &EmoWorld,
    atlas: &AtlasTable,
    limit: usize,
) -> ShellBounds {
    // [left, top, right, bottom]。(0,0) を必ず含むよう 0 から始める。
    let mut acc = [0i64; 4];
    let mut scratch = BoundsScratch::default();
    let mut memo = BoundsMemo::new();
    for id in world.surface_ids() {
        if let Some(r) = walk(&mut scratch, Some(&mut memo), world, atlas, id, limit) {
            acc = [
                acc[0].min(r[0]),
                acc[1].min(r[1]),
                acc[2].max(r[2]),
                acc[3].max(r[3]),
            ];
        }
        // 合成の絵の大きさ（外形の表）は描画メソッドを問わず着せ替えの描くコマ（純 bind は全部・
        // `bind+random` は番号の最も小さい 1 つ）と `always` の pattern を数えるので、右下はそれも
        // 収める（Invariants）。bind・`always` の循環の知らせは
        // 表を作るときの debug だけ。
        let extent = world.extent(id);
        acc[2] = acc[2].max(extent.w as i64);
        acc[3] = acc[3].max(extent.h as i64);
    }
    let to_i32 = |v: i64| v.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    ShellBounds {
        left: to_i32(acc[0]),
        top: to_i32(acc[1]),
        right: to_i32(acc[2]),
        bottom: to_i32(acc[3]),
    }
}

/// 段の外形（その面の原点からの相対・[left, top, right, bottom]）。何も数えていなければ `None`。
type Rect = Option<[i64; 4]>;

/// きれいに終わった面の、下で予算に数えた数と外形（その面の原点からの相対・[`crate::outline_walk::Memo`]）。
pub(crate) type BoundsMemo = Memo<[i64; 4]>;

/// シェルの外形の走査の作業場（走査が終わると空に戻る）。
pub(crate) type BoundsScratch = Scratch<[i64; 4]>;

/// 外形（[left, top, right, bottom]・その面の原点からの相対）。
impl Outline for [i64; 4] {
    /// 外形 `acc` に、`(x, y)` ずらした外形 `r` を飽和で重ねる。
    fn merge(acc: &mut Rect, r: Rect, x: i64, y: i64) {
        let Some(r) = r else {
            return;
        };
        let r = [
            r[0].saturating_add(x),
            r[1].saturating_add(y),
            r[2].saturating_add(x),
            r[3].saturating_add(y),
        ];
        *acc = Some(acc.map_or(r, |a| {
            [
                a[0].min(r[0]),
                a[1].min(r[1]),
                a[2].max(r[2]),
                a[3].max(r[3]),
            ]
        }));
    }
}

/// 根の面 `root` からたどり、根の原点からの外形を返す（面の表に無ければ `None`）。たどり方（明示のスタック・
/// 循環は黙って打ち切る・予算・覚え）は面ごとの外形と共有する [`crate::outline_walk::walk`] で、ここは面ごとの
/// 寄与（element → pattern の順の子と、自分の層の外形）だけを渡す（指摘 84）。
pub(crate) fn walk(
    s: &mut BoundsScratch,
    memo: Option<&mut BoundsMemo>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    root: u32,
    limit: usize,
) -> Rect {
    crate::outline_walk::walk(s, world, memo, root, limit, "bounds", |surface, stack| {
        let (master, binding) = surface_and_binding(world, surface)?;
        let mut rect = None;
        for (i, element) in master.elements.iter().enumerate() {
            let (ex, ey) = element.transform.offset();
            let original = match element.kind {
                ElementKind::Surface(child) => {
                    stack.push(Step::Enter {
                        surface: child,
                        x: ex,
                        y: ey,
                        quiet: true,
                    });
                    continue;
                }
                ElementKind::Film(film) => world.film_sheet(film).map(|sheet| sheet.original),
                _ => binding.0.get(i).copied().flatten().map(|element_id| {
                    let original = atlas.entry(element_id).original;
                    (original.w, original.h)
                }),
            };
            let Some((w, h)) = original else {
                continue;
            };
            Outline::merge(&mut rect, Some([0, 0, w as i64, h as i64]), ex, ey);
        }

        for pattern in master.animations.iter().flat_map(|a| &a.patterns) {
            // 負の番号（描かない）・u32 に収まらない番号・描かないメソッド（`start` 等・`move` を含む）は数えない。
            let Ok(child) = u32::try_from(pattern.surface_id) else {
                continue;
            };
            if !is_implemented_name(pattern.method.as_str()) {
                continue;
            }
            // 循環は黙って打ち切る（warn は読み込みが出す）。
            stack.push(Step::Enter {
                surface: child,
                x: pattern.x,
                y: pattern.y,
                quiet: true,
            });
        }
        rect
    })
}

#[cfg(test)]
#[path = "plan_bounds_tests.rs"]
mod tests;
