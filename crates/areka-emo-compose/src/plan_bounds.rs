//! シェル全体の外形（[`ShellBounds`]）の算出（areka-P0-offline-surface-dump 要件 4.3）。
//!
//! 面ごとの外形（`plan_extent.rs` の `compute_extent`）の辿り方（入れ子・祖先スタックで循環を
//! 避ける）を写し、座標を符号つきで持つ。違いは、⑴ すべての面を数える、⑵ 負の座標を原点で
//! 切らない、⑶ すべての animation（interval を問わない・`never`・`runonce` を含む）のすべての
//! pattern を数える、の 3 点。記録は出さない（読み込みと合成が既に出している）。
//! 右下は面ごとの `compute_extent`（合成の絵の大きさ）も重ねて、どの面の絵も外形に収める。

use areka_emo_atlas::AtlasTable;

use crate::method::is_implemented_name;
use crate::nesting::ElementKind;
use crate::plan::{compute_extent, surface_and_binding};
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
    // [left, top, right, bottom]。(0,0) を必ず含むよう 0 から始める。
    let mut acc = [0i64; 4];
    let mut visited = Vec::new();
    for id in world.surface_ids() {
        walk(&mut acc, &mut visited, world, atlas, id, 0, 0);
        // 合成の絵の大きさ（`compute_extent`）は描画メソッドを問わず bind の pattern0・`always` の
        // pattern を数えるので、右下はそれも収める（Invariants）。bind・`always` の循環の知らせは
        // `compute_extent` の debug だけ。
        let extent = compute_extent(&mut visited, world, atlas, id);
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

/// [`shell_bounds`] の再帰: 面 `surface_id` を `(ox, oy)` へずらして `acc` に重ねる。
fn walk(
    acc: &mut [i64; 4],
    visited: &mut Vec<u32>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    surface_id: u32,
    ox: i64,
    oy: i64,
) {
    // 循環は黙って打ち切る（warn は読み込みが出す）。
    if visited.contains(&surface_id) {
        return;
    }
    let Some((master, binding)) = surface_and_binding(world, surface_id) else {
        return;
    };
    visited.push(surface_id);

    for (i, element) in master.elements.iter().enumerate() {
        let (ex, ey) = element.transform.offset();
        let original = match element.kind {
            ElementKind::Surface(child) => {
                walk(acc, visited, world, atlas, child, ox + ex, oy + ey);
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
        let (x, y) = (ox + ex, oy + ey);
        acc[0] = acc[0].min(x);
        acc[1] = acc[1].min(y);
        acc[2] = acc[2].max(x + w as i64);
        acc[3] = acc[3].max(y + h as i64);
    }

    for pattern in master.animations.iter().flat_map(|a| &a.patterns) {
        // 負の番号（描かない）・u32 に収まらない番号・描かないメソッド（`start` 等・`move` を含む）は数えない。
        let Ok(child) = u32::try_from(pattern.surface_id) else {
            continue;
        };
        if !is_implemented_name(pattern.method.as_str()) {
            continue;
        }
        walk(
            acc,
            visited,
            world,
            atlas,
            child,
            ox + pattern.x,
            oy + pattern.y,
        );
    }

    visited.pop();
}

#[cfg(test)]
#[path = "plan_bounds_tests.rs"]
mod tests;
