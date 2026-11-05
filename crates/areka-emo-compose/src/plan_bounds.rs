//! シェル全体の外形（[`ShellBounds`]）の算出（areka-P0-offline-surface-dump 要件 4.3）。
//!
//! 面ごとの外形（`plan_extent.rs` の `compute_extent_limited`）の辿り方（入れ子・祖先スタックで循環を
//! 避ける・明示のスタック）を写し、座標を符号つきで持つ。違いは、⑴ すべての面を数える、⑵ 負の座標を原点で
//! 切らない、⑶ すべての animation（interval を問わない・`never`・`runonce` を含む）のすべての
//! pattern を数える、の 3 点。記録は出さない（読み込みと合成が既に出している・広がりの予算が尽きたときの `error!` だけ出す）。
//! 右下は面ごとの外形（合成の絵の大きさ・束縛のときに求めた表）も重ねて、どの面の絵も外形に収める。

use std::collections::{HashMap, HashSet};

use areka_emo_atlas::AtlasTable;

use crate::budget::{Budget, VISIT_BUDGET};
use crate::method::is_implemented_name;
use crate::nesting::ElementKind;
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
        // 合成の絵の大きさ（外形の表）は描画メソッドを問わず bind の pattern0・`always` の
        // pattern を数えるので、右下はそれも収める（Invariants）。bind・`always` の循環の知らせは
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

/// シェルの外形の走査の積み荷: 面 `surface` へ親の原点から `(x, y)` ずらして入るか、面から出る。
enum Step {
    Enter { surface: u32, x: i64, y: i64 },
    Exit,
}

/// 段の外形（その面の原点からの相対・[left, top, right, bottom]）。何も数えていなければ `None`。
type Rect = Option<[i64; 4]>;

/// シェルの外形の走査の段: 面と、親の原点からの位置と、段の下で数えた外形と、きれいに終わるかと、
/// 段の下で予算に数えた数。
struct Level {
    surface: u32,
    x: i64,
    y: i64,
    rect: Rect,
    clean: bool,
    spent: usize,
}

/// きれいに終わった面（その下で先祖へ戻る辺を捨てず、予算にも断られなかった面）の、下で予算に数えた数と
/// 外形（その面の原点からの相対）。きれいに終わった面の下に循環は無いので、どの経路から入っても答えは同じ。
pub(crate) type BoundsMemo = HashMap<u32, (usize, Rect)>;

/// シェルの外形の走査の作業場（経路にある面の集合・積み荷の列・段の列）。どれも走査が終わると空に戻る。
#[derive(Default)]
pub(crate) struct BoundsScratch {
    on_path: HashSet<u32>,
    stack: Vec<Step>,
    levels: Vec<Level>,
}

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

/// 根の面 `root` から明示のスタックでたどり、根の原点からの外形を返す（面の表に無ければ `None`）。
/// 子は今までの再帰と同じ順（element → pattern）に積み、逆にして上から取り出す。段ごとの外形はその面の
/// 原点からの相対で持ち、段が終わったら親の原点へずらして重ねる。
///
/// `memo` が在れば、きれいに終わった面の外形を覚え、後で同じ面に入るとき予算の残りが「入る 1 ＋下で数えた
/// 数」以上ならそれだけ一度に引いて覚えた外形を使う（足りなければ今どおり入る＝答えと記録は覚えが無いときと同じ）。
pub(crate) fn walk(
    s: &mut BoundsScratch,
    mut memo: Option<&mut BoundsMemo>,
    world: &EmoWorld,
    atlas: &AtlasTable,
    root: u32,
    limit: usize,
) -> Rect {
    let mut budget = Budget::new(limit);
    let mut answer = None;
    s.stack.push(Step::Enter {
        surface: root,
        x: 0,
        y: 0,
    });
    while let Some(step) = s.stack.pop() {
        let Step::Enter { surface, x, y } = step else {
            // 段が尽きた: 経路から外し、きれいなら覚え、外形を親の原点へずらして重ねる（根なら答え）。
            let Some(done) = s.levels.pop() else {
                break;
            };
            s.on_path.remove(&done.surface);
            if done.clean
                && let Some(m) = memo.as_deref_mut()
            {
                m.insert(done.surface, (done.spent, done.rect));
            }
            match s.levels.last_mut() {
                Some(up) => {
                    up.spent += done.spent + 1;
                    up.clean &= done.clean;
                    merge(&mut up.rect, done.rect, done.x, done.y);
                }
                None => answer = done.rect,
            }
            continue;
        };
        // 循環は黙って打ち切る（warn は読み込みが出す）。
        if s.on_path.contains(&surface) {
            if let Some(up) = s.levels.last_mut() {
                up.clean = false;
            }
            continue;
        }
        // 根の面（段が無い）は予算に数えない。
        if let Some(up) = s.levels.last_mut() {
            if let Some(&(n, rect)) = memo.as_deref().and_then(|m| m.get(&surface))
                && budget.take(n + 1)
            {
                up.spent += n + 1;
                merge(&mut up.rect, rect, x, y);
                continue;
            }
            if !budget.enter("bounds", root) {
                up.clean = false;
                continue;
            }
        }
        let Some((master, binding)) = surface_and_binding(world, surface) else {
            if let Some(up) = s.levels.last_mut() {
                up.spent += 1;
            }
            continue;
        };
        s.on_path.insert(surface);
        s.stack.push(Step::Exit);
        let children = s.stack.len();
        let mut rect = None;

        for (i, element) in master.elements.iter().enumerate() {
            let (ex, ey) = element.transform.offset();
            let original = match element.kind {
                ElementKind::Surface(child) => {
                    s.stack.push(Step::Enter {
                        surface: child,
                        x: ex,
                        y: ey,
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
            merge(&mut rect, Some([0, 0, w as i64, h as i64]), ex, ey);
        }

        for pattern in master.animations.iter().flat_map(|a| &a.patterns) {
            // 負の番号（描かない）・u32 に収まらない番号・描かないメソッド（`start` 等・`move` を含む）は数えない。
            let Ok(child) = u32::try_from(pattern.surface_id) else {
                continue;
            };
            if !is_implemented_name(pattern.method.as_str()) {
                continue;
            }
            s.stack.push(Step::Enter {
                surface: child,
                x: pattern.x,
                y: pattern.y,
            });
        }
        s.levels.push(Level {
            surface,
            x,
            y,
            rect,
            clean: true,
            spent: 0,
        });
        s.stack[children..].reverse();
    }
    answer
}

#[cfg(test)]
#[path = "plan_bounds_tests.rs"]
mod tests;
