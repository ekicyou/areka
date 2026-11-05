//! 子のサーフェスの当たり判定の領域を親へ持ち込む（surface-element-nesting 要件 4.1〜4.10）。
//!
//! サーフェス S の持ち込み済みの列＝［子から持ち込む分（S の element定義の番号の昇順）］→
//! ［S に直接書かれた領域（転記の並び）］。当たり判定は列を後ろから見るので、S に直接書いた領域が
//! 最前・番号の大きい element定義の子が次になる（要件 4.3・4.5）。
//!
//! 子 C から持ち込む分は、C の持ち込み済みの列のうち S に直接書かれた領域と同じ名前でないものを、
//! element定義の X,Y だけずらした写し（要件 4.1・4.8）。C の列の並びは変えない（要件 4.4）。C の列は
//! C 自身の名前で孫の分を落とした結果なので、落とし方は段ごとに当てはまる（要件 4.9）。
//!
//! たどるのは element定義の辺だけで、合成（`plan.rs`）が飛ばす辺——面の表に無い番号・範囲を超える数・
//! 先祖へ戻る辺——からは持ち込まない（要件 4.6）。pattern定義の先からも持ち込まない（要件 4.7）。
//! 記録は出さない（無い番号と循環は [`crate::nesting::NestReport`] が報告する・広がりの予算が尽きたときの
//! `error!` だけ出す）。

use areka_parsers::shell::Collision;
use bevy_ecs::prelude::Component;

use crate::budget::{Budget, VISIT_BUDGET};
use crate::nesting::ElementKind;
use crate::normalized::SurfaceMaster;
use crate::world::{EmoWorld, Records, SurfaceIndex};

/// 持ち込み済みの当たり判定の列（持ち込みが在るサーフェスにだけ付くコンポーネント）。
///
/// 引くのは [`EmoWorld::hit_regions`]（付いていなければ `SurfaceMaster.collisions`）。
#[derive(Debug, Clone, PartialEq, Component)]
pub struct HitRegions(pub Vec<Collision>);

/// 面の表の全サーフェスを根に 1 回ずつ列を求め、持ち込みが在るサーフェスにだけ [`HitRegions`] を付ける
/// （[`EmoWorld::build_with_images`] の最後で呼ぶ）。持ち込みが 1 つも無いサーフェスには何も付けない。
/// 広がりの予算は根の面 1 つにつき 1 つ（`records` が `Quiet` なら尽きても記録を出さない）。
pub(crate) fn attach_hit_regions(emo: &mut EmoWorld, records: Records) {
    let mut found = Vec::new();
    for id in emo.surface_ids() {
        let Some(master) = emo.surface(id) else {
            continue;
        };
        let list = regions_of(
            emo,
            master,
            &mut Budget::with_records(VISIT_BUDGET, records),
        );
        // 列は［持ち込み］→［直接］なので、長さが違えば持ち込みが在る。
        if list.len() != master.collisions.len() {
            found.push((id, list));
        }
    }
    let world = emo.world_mut();
    for (id, list) in found {
        let Some(&entity) = world.resource::<SurfaceIndex>().0.get(&id) else {
            continue;
        };
        world.entity_mut(entity).insert(HitRegions(list));
    }
}

/// 持ち込みの走査の段: 面と、element の読み位置と、その段で集めた列と、親の element定義の X,Y。
struct Level<'a> {
    master: &'a SurfaceMaster,
    next: usize,
    out: Vec<Collision>,
    dx: i64,
    dy: i64,
}

/// `master` を根にした持ち込み済みの列。段のスタック（先祖の列を兼ねる）で回し、子の段が終わったら
/// 親に直接書かれた領域と同じ名前を除き、element定義の X,Y を飽和で足して親の列へ足す。
// ponytail: 根ごとに子をたどり直す（同じ子を何度も求める）。読み込み 1 回だけなので、菱形が深く
// 重なって遅ければ「先祖に左右されない子」の列を覚えておく。
fn regions_of(world: &EmoWorld, master: &SurfaceMaster, budget: &mut Budget) -> Vec<Collision> {
    let root = master.id;
    let mut levels = vec![Level {
        master,
        next: 0,
        out: Vec::new(),
        dx: 0,
        dy: 0,
    }];
    while let Some(level) = levels.last_mut() {
        let parent = level.master;
        // elements は element定義の番号の昇順・同じ番号は書いた順（SurfaceMaster の不変条件）。
        if let Some(element) = parent.elements.get(level.next) {
            level.next += 1;
            let ElementKind::Surface(c) = element.kind else {
                continue;
            };
            if levels.iter().any(|l| l.master.id == c) {
                continue;
            }
            let Some(child) = world.surface(c) else {
                continue;
            };
            if !budget.enter("hit_import", root) {
                continue;
            }
            let (dx, dy) = element.transform.offset();
            levels.push(Level {
                master: child,
                next: 0,
                out: Vec::new(),
                dx,
                dy,
            });
            continue;
        }
        // 段が尽きた: ［持ち込み］→［直接］の列にして親へ渡す（根なら答え）。
        let Some(mut done) = levels.pop() else {
            break;
        };
        done.out.extend(done.master.collisions.iter().cloned());
        let Some(up) = levels.last_mut() else {
            return done.out;
        };
        let own = &up.master.collisions;
        let (dx, dy) = (done.dx, done.dy);
        up.out.extend(
            done.out
                .into_iter()
                .filter(|r| own.iter().all(|p| p.name != r.name))
                .map(|r| Collision {
                    left: r.left.saturating_add(dx),
                    top: r.top.saturating_add(dy),
                    right: r.right.saturating_add(dx),
                    bottom: r.bottom.saturating_add(dy),
                    ..r
                }),
        );
    }
    Vec::new()
}

#[cfg(test)]
#[path = "hit_import_tests.rs"]
mod tests;
