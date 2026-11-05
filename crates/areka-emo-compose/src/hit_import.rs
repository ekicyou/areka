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
//! 記録は出さない（無い番号と循環は [`crate::nesting::NestReport`] が報告する・広がりの予算が尽きたときと、
//! 根をまたいで持ち込む領域の数が [`IMPORT_BUDGET`] を越えたときの `error!` だけ出す）。

use std::collections::{HashMap, HashSet};

use areka_parsers::shell::Collision;
use bevy_ecs::prelude::Component;

use crate::budget::{Budget, VISIT_BUDGET, surface_rows};
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
/// 広がりの予算は根の面 1 つにつき 1 つ（`records` が `Quiet` なら尽きても記録を出さない）。きれいに
/// 終わった面の覚え（[`HitMemo`]）は根の間で 1 つを共有する。根をまたいで持ち込む領域の数は
/// [`IMPORT_BUDGET`] 1 つで数える（[`attach_hit_regions_limited`]）。
pub(crate) fn attach_hit_regions(emo: &mut EmoWorld, records: Records) {
    attach_hit_regions_limited(emo, records, IMPORT_BUDGET)
}

/// シェル全体で面に持ち込んで持ち続ける領域の数の上限（指摘 79）。根ごとの広がりの予算は 1 つの根の
/// 仕事を抑えるだけで、面の数だけ同じ長さの列を持ち続ける。仮の値で、測り直しの候補。
pub(crate) const IMPORT_BUDGET: usize = 1 << 20;

/// [`attach_hit_regions`] の中身。`limit` は根をまたいで持ち込む領域の数の予算（本番は [`IMPORT_BUDGET`]・
/// テストは小さな数）。尽きたら `error!` を 1 行出し（`records` が `Quiet` なら出さない）、その面から先には
/// 持ち込みを付けない。
pub(crate) fn attach_hit_regions_limited(emo: &mut EmoWorld, records: Records, limit: usize) {
    let mut left = limit;
    let mut found = Vec::new();
    let mut memo = HitMemo::new();
    for id in emo.surface_ids() {
        let Some(master) = emo.surface(id) else {
            continue;
        };
        let list = regions_of(
            emo,
            master,
            &mut Budget::with_records(VISIT_BUDGET, records),
            Some(&mut memo),
        );
        // 列は［持ち込み］→［直接］なので、長さが違えば持ち込みが在る（根は領域を切り詰めない）。
        let imported = list.len() - master.collisions.len();
        if imported > left {
            if records.emits() {
                tracing::error!(
                    target: "areka_emo_compose",
                    surface_id = id,
                    imported,
                    budget = limit,
                    "持ち込む領域の数がシェル全体の予算を越えた: この面から先へ持ち込みを付けない"
                );
            }
            break;
        }
        left -= imported;
        if imported > 0 {
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

/// きれいに終わった面（その下で先祖へ戻る辺を捨てず、予算にも断られなかった面）の、入る分と下で予算に数えた数と
/// 持ち込み済みの列。きれいに終わった面の下に循環は無いので、どの経路から入っても答えは同じ。
pub(crate) type HitMemo = HashMap<u32, (usize, Vec<Collision>)>;

/// 持ち込みの走査の段: 面と、element の読み位置と、その段で集めた列と、親の element定義の X,Y と、
/// きれいに終わるかと、この面に入る分（入る 1 ＋面の行）と段の下で予算に数えた数。
struct Level<'a> {
    master: &'a SurfaceMaster,
    /// `master` に直接書かれた領域の名前（段ごとに 1 度だけ作り、持ち込む 1 本ごとに引く・指摘 80）。
    names: HashSet<&'a str>,
    next: usize,
    out: Vec<Collision>,
    dx: i64,
    dy: i64,
    clean: bool,
    spent: usize,
}

/// `master` を根にした持ち込み済みの列。段のスタックで回し（先祖の確かめは経路の集合）、子の段が終わったら
/// 親に直接書かれた領域と同じ名前を除き、element定義の X,Y を飽和で足して親の列へ足す。予算は入る面 1 つに
/// 「入る 1 ＋面の行の全部（[`surface_rows`]）」（足りなければ入らない・指摘 130）と、子の段から親へ渡す列の
/// 長さ（名前で除く前）を数える。渡す列を予算で切るときは奥（列の前＝持ち込みの奥）から落とし、直接の領域
/// （いちばん手前）を残す（指摘 137）。
///
/// `memo` が在れば、きれいに終わった面の答えを覚え、後で同じ面に入るとき予算の残りが「入る分＋下で数えた
/// 数＋覚えた列の長さ」以上ならそれだけ一度に引いて覚えた答えを使う（足りなければ今どおり入る＝答えと記録は
/// 覚えが無いときと同じ）。
pub(crate) fn regions_of(
    world: &EmoWorld,
    master: &SurfaceMaster,
    budget: &mut Budget,
    mut memo: Option<&mut HitMemo>,
) -> Vec<Collision> {
    let root = master.id;
    let mut on_path = HashSet::from([root]);
    let mut levels = vec![Level {
        master,
        names: names_of(master),
        next: 0,
        out: Vec::new(),
        dx: 0,
        dy: 0,
        clean: true,
        spent: surface_rows(master).saturating_add(1),
    }];
    while let Some(level) = levels.last_mut() {
        let parent = level.master;
        // elements は element定義の番号の昇順・同じ番号は書いた順（SurfaceMaster の不変条件）。
        if let Some(element) = parent.elements.get(level.next) {
            level.next += 1;
            let ElementKind::Surface(c) = element.kind else {
                continue;
            };
            if on_path.contains(&c) {
                level.clean = false;
                continue;
            }
            let Some(child) = world.surface(c) else {
                continue;
            };
            let (dx, dy) = element.transform.offset();
            if let Some((n, list)) = memo.as_deref().and_then(|m| m.get(&c))
                && budget.take(n + list.len())
            {
                level.spent = level.spent.saturating_add(n + list.len());
                import(&mut level.out, &level.names, list.iter().cloned(), dx, dy);
                continue;
            }
            let rows = surface_rows(child);
            if !budget.enter(rows, "hit_import", root) {
                level.clean = false;
                continue;
            }
            on_path.insert(c);
            levels.push(Level {
                master: child,
                names: names_of(child),
                next: 0,
                out: Vec::new(),
                dx,
                dy,
                clean: true,
                spent: rows + 1,
            });
            continue;
        }
        // 段が尽きた: ［持ち込み］→［直接］の列にして親へ渡す（根なら答え）。
        let Some(mut done) = levels.pop() else {
            break;
        };
        on_path.remove(&done.master.id);
        // 根でなければ親へ渡す領域の数も予算から引き、足りない分は渡さない（指摘 22）。
        let own = &done.master.collisions;
        let total = done.out.len() + own.len();
        let carried = if levels.is_empty() {
            total
        } else {
            budget.take_up_to(total, "hit_import", root)
        };
        done.clean &= carried == total;
        // 列は［持ち込み（奥）］→［直接（手前）］。切るときは奥から落とす（指摘 137）。
        done.out.extend(own.iter().cloned());
        done.out.drain(..total - carried);
        if done.clean
            && let Some(m) = memo.as_deref_mut()
        {
            m.insert(done.master.id, (done.spent, done.out.clone()));
        }
        let Some(up) = levels.last_mut() else {
            return done.out;
        };
        up.spent = up.spent.saturating_add(done.spent).saturating_add(carried);
        up.clean &= done.clean;
        import(
            &mut up.out,
            &up.names,
            done.out.into_iter(),
            done.dx,
            done.dy,
        );
    }
    Vec::new()
}

/// 面に直接書かれた領域の名前の集合。
fn names_of(master: &SurfaceMaster) -> HashSet<&str> {
    master.collisions.iter().map(|c| c.name.as_str()).collect()
}

/// 子の列 `list` のうち親に直接書かれた領域の名前（`own`）と同じ名前でないものを、element定義の X,Y を
/// 飽和で足して親の列 `out` へ足す。
fn import(
    out: &mut Vec<Collision>,
    own: &HashSet<&str>,
    list: impl Iterator<Item = Collision>,
    dx: i64,
    dy: i64,
) {
    out.extend(
        list.filter(|r| !own.contains(r.name.as_str()))
            .map(|r| Collision {
                left: r.left.saturating_add(dx),
                top: r.top.saturating_add(dy),
                right: r.right.saturating_add(dx),
                bottom: r.bottom.saturating_add(dy),
                ..r
            }),
    );
}

#[cfg(test)]
#[path = "hit_import_tests.rs"]
mod tests;
