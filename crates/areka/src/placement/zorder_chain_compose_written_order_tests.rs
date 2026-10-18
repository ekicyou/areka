//! 書かれた順の並びから組む鎖の決定論的テスト（`areka-P0-zorder-scope-inversion`
//! 要件 1.1／1.2／1.6／2.1／2.2／3.3／4.3／8.2）。
//!
//! 合成の繋ぎの規則（連続する 2 枚のうち同一スコープの（バルーン, キャラ窓）の順だけを
//! 除く）は変えていない。ここで固定するのは、その規則が逆転・非隣接・補いの並びにも
//! 向きを問わず効き、**最後の持ち主の関係**——鎖の繋ぎ＋連続する組の結び目——が
//! `members` の連続する 2 枚の全部と一致して一直線になることである。連続しない組の
//! 結び目は wintf の鎖の適用系が外す約束なので、持ち主の関係には数えない。
//!
//! # 摂動
//!
//! 同一スコープ対の除外を経路から外すと、`s0,b0,b1,s1` の繋ぎが 3 本になり、全数の檻では
//! 組の結び目と鎖の繋ぎが同じ窓の持ち主を 2 度書くので赤くなる。

use std::collections::HashMap;

use bevy_ecs::entity::Entity;
use wintf::ecs::window::{ChainPlan, ChainSegment, CrossEdge};

use super::compose_chain;
use crate::placement::zorder_group_ledger::{
    GroupElement, GroupSource, GroupWindowKind, ZOrderGroup, parse_zorder_tokens,
};

// ===========================================================================
// 道具立て
// ===========================================================================

fn b(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Balloon,
    }
}

fn s(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Char,
    }
}

/// 要素 1 つに割り当てる決定論的な `Entity`（スコープ×種別で一意）。
fn ent(element: GroupElement) -> Entity {
    let kind = match element.kind {
        GroupWindowKind::Balloon => 0,
        GroupWindowKind::Char => 1,
    };
    Entity::from_raw_u32(element.scope * 2 + kind + 1).expect("テスト用 entity 索引は有効")
}

fn ents(elements: &[GroupElement]) -> Vec<Entity> {
    elements.iter().copied().map(ent).collect()
}

/// 手前側 `front` が奥側 `back` に所有される繋ぎ 1 本（`front ← back`）。
fn edge(front: GroupElement, back: GroupElement, segment: ChainSegment) -> CrossEdge {
    CrossEdge {
        owned: ent(front),
        owner: ent(back),
        segment,
    }
}

/// タグ由来のグループ 1 本を、本番と同じ解釈器（補い込み）を通して組む。
fn tag_group(id: u32, tokens: &[&str]) -> ZOrderGroup {
    let (members, _) = parse_zorder_tokens(tokens).expect("テストの指定は受理されるもの");
    ZOrderGroup {
        id,
        members,
        source: GroupSource::Tag,
    }
}

/// 各スコープの 2 窓がそろって在る在庫で合成する。
fn compose_full(groups: &[ZOrderGroup], scopes: &[u32]) -> ChainPlan {
    let resolve = |element: &GroupElement| scopes.contains(&element.scope).then(|| ent(*element));
    compose_chain(groups, scopes, &resolve).expect("グループがあるので計画は組まれる")
}

/// 組の結び目（`bN ← sN`）のうち、`members` で連続しているもの。連続しない組は
/// wintf の鎖の適用系が結び目を外すので数えない。
fn consecutive_pair_links(members: &[Entity], scopes: &[u32]) -> Vec<(Entity, Entity)> {
    members
        .windows(2)
        .filter(|w| {
            scopes
                .iter()
                .any(|&scope| w[0] == ent(b(scope)) && w[1] == ent(s(scope)))
        })
        .map(|w| (w[0], w[1]))
        .collect()
}

/// 最後の持ち主の関係（鎖の繋ぎ＋連続する組の結び目）が `members` の連続する 2 枚と
/// 過不足なく一致し、一直線になっているかを確かめ、破れを並べて返す。
fn final_owner_violations(plan: &ChainPlan, scopes: &[u32]) -> Vec<String> {
    let mut violations = Vec::new();

    // 書き手ごとに並べたまま連ねる——集合にすると 2 つの機構が同じ持ち主を書く破れが消える。
    let mut relation: Vec<(Entity, Entity)> = plan
        .cross_edges
        .iter()
        .map(|e| (e.owned, e.owner))
        .collect();
    relation.extend(consecutive_pair_links(&plan.members, scopes));

    let mut expected: Vec<(Entity, Entity)> =
        plan.members.windows(2).map(|w| (w[0], w[1])).collect();
    let mut actual = relation.clone();
    expected.sort();
    actual.sort();
    if actual != expected {
        violations.push(format!(
            "持ち主の関係が連続する 2 枚と一致しない: actual={actual:?} expected={expected:?}"
        ));
    }

    // 一直線: 各窓の持ち主は高々 1 つ・各窓が持つ窓も高々 1 つ・手前の端から辿ると
    // 全窓を 1 度ずつ通って奥の端で止まる（輪も分岐も無い）。
    let mut owner_of: HashMap<Entity, Entity> = HashMap::new();
    let mut owns: HashMap<Entity, usize> = HashMap::new();
    for &(owned, owner) in &relation {
        if owner_of.insert(owned, owner).is_some() {
            violations.push(format!("{owned:?} の持ち主が 2 度書かれている"));
        }
        *owns.entry(owner).or_default() += 1;
    }
    for (owner, count) in &owns {
        if *count > 1 {
            violations.push(format!("分岐: {owner:?} が {count} 枚を持つ"));
        }
    }
    if let Some(&head) = plan.members.first() {
        let mut walked = vec![head];
        let mut cursor = head;
        while let Some(&next) = owner_of.get(&cursor) {
            if walked.contains(&next) {
                violations.push(format!("輪: {next:?} へ戻った"));
                break;
            }
            walked.push(next);
            cursor = next;
        }
        if walked != plan.members {
            violations.push(format!(
                "手前の端から辿った列が members と違う: walked={walked:?} members={:?}",
                plan.members
            ));
        }
    }

    violations
}

// ===========================================================================
// 代表の並び
// ===========================================================================

/// `s0,b0,b1,s1`（ぱすたの `s0,b0,s1` の補い後）の繋ぎは `s0←b0`・`b0←b1` の 2 本だけ。
/// `b1,s1` は連続する組なので組の結び目が受け持つ（要件 1.1／1.2／1.6／3.3）。
#[test]
fn t_zcw01_inverted_scope_zero_joins_only_two_edges() {
    let groups = [tag_group(1, &["s0", "b0", "b1", "s1"])];
    let plan = compose_full(&groups, &[0, 1]);

    assert_eq!(plan.members, ents(&[s(0), b(0), b(1), s(1)]));
    assert_eq!(
        plan.cross_edges,
        vec![
            edge(s(0), b(0), ChainSegment::Group(1)),
            edge(b(0), b(1), ChainSegment::Group(1))
        ]
    );
    assert!(final_owner_violations(&plan, &[0, 1]).is_empty());

    // 補いの経路でも同じ鎖になる（`s1` だけの名指しに `b1` をすぐ手前へ補う・要件 3.3）。
    let filled = compose_full(&[tag_group(1, &["s0", "b0", "s1"])], &[0, 1]);
    assert_eq!(filled.members, plan.members);
    assert_eq!(filled.cross_edges, plan.cross_edges);
}

/// `b1,s0,s1,b0`（両スコープとも非隣接・スコープ 0 は逆転も）の繋ぎは 3 本
/// （`b1←s0`・`s0←s1`・`s1←b0`）。連続する組が無いので除く連続も無い（要件 2.1／2.2）。
#[test]
fn t_zcw02_split_scopes_join_every_consecutive_pair() {
    let groups = [tag_group(1, &["b1", "s0", "s1", "b0"])];
    let plan = compose_full(&groups, &[0, 1]);

    assert_eq!(plan.members, ents(&[b(1), s(0), s(1), b(0)]));
    let g = ChainSegment::Group(1);
    assert_eq!(
        plan.cross_edges,
        vec![
            edge(b(1), s(0), g),
            edge(s(0), s(1), g),
            edge(s(1), b(0), g)
        ]
    );
    assert!(final_owner_violations(&plan, &[0, 1]).is_empty());
}

/// 後ろ側の参加は今のまま: 名前の挙がらないスコープは昇順で `[バルーン, キャラ窓]` の
/// かたまりとして後ろへ連なり、その組は繋がない（要件 4.3）。グループの末尾から後ろへの
/// 渡りは手前側（グループ）の区間で記録される。
#[test]
fn t_zcw03_tail_participation_is_unchanged_behind_an_inverted_group() {
    let groups = [tag_group(1, &["s0", "b0", "b1", "s1"])];
    let scopes = [3, 0, 2, 1];
    let plan = compose_full(&groups, &scopes);

    assert_eq!(
        plan.members,
        ents(&[s(0), b(0), b(1), s(1), b(2), s(2), b(3), s(3)])
    );
    let g = ChainSegment::Group(1);
    assert_eq!(
        plan.cross_edges,
        vec![
            edge(s(0), b(0), g),
            edge(b(0), b(1), g),
            edge(s(1), b(2), g),
            edge(s(2), b(3), ChainSegment::Tail),
        ]
    );
    assert!(final_owner_violations(&plan, &scopes).is_empty());
}

// ===========================================================================
// 全数: 逆転・非隣接・補いの組み合わせ（要件 8.2「枝分かれの無い一直線」）
// ===========================================================================

/// 要素列の全順列を `out` へ。
fn permutations(items: &mut Vec<GroupElement>, k: usize, out: &mut Vec<Vec<GroupElement>>) {
    if k == items.len() {
        out.push(items.clone());
        return;
    }
    for i in k..items.len() {
        items.swap(k, i);
        permutations(items, k + 1, out);
        items.swap(k, i);
    }
}

fn token(element: GroupElement) -> String {
    match element.kind {
        GroupWindowKind::Balloon => format!("b{}", element.scope),
        GroupWindowKind::Char => format!("s{}", element.scope),
    }
}

/// スコープ 0〜2 の各々を「書かない・`bN` だけ・`sN` だけ・2 枚とも」から選び、書いた窓の
/// 全順列を指定にする。後ろ側の参加も混ぜるため在庫には常にスコープ 3 を足す。
/// どの指定でも最後の持ち主の関係は連続する 2 枚の全部と一致し、一直線になる。
#[test]
fn t_zcw04_every_written_order_yields_one_straight_line() {
    let scopes = [0, 1, 2, 3];
    let mut checked = 0usize;
    let mut seen_inverted = false;
    let mut seen_split = false;
    let mut seen_filled = false;

    for pattern in 0..4u32.pow(3) {
        let mut written = Vec::new();
        for scope in 0..3u32 {
            match (pattern / 4u32.pow(scope)) % 4 {
                1 => written.push(b(scope)),
                2 => written.push(s(scope)),
                3 => {
                    written.push(b(scope));
                    written.push(s(scope));
                }
                _ => {}
            }
        }
        if written.len() < 2 {
            continue;
        }
        let mut orders = Vec::new();
        permutations(&mut written, 0, &mut orders);
        for order in orders {
            let tokens: Vec<String> = order.iter().copied().map(token).collect();
            let tokens: Vec<&str> = tokens.iter().map(String::as_str).collect();
            let groups = [tag_group(1, &tokens)];
            let plan = compose_full(&groups, &scopes);

            let violations = final_owner_violations(&plan, &scopes);
            assert!(violations.is_empty(), "{tokens:?}: {violations:?}");

            // 刺激が届いたことの対照: 全数が 3 つの形を実際に含む。
            let pos = |e: GroupElement| plan.members.iter().position(|m| *m == ent(e));
            for scope in 0..3 {
                if let (Some(bp), Some(sp)) = (pos(b(scope)), pos(s(scope))) {
                    seen_inverted |= sp < bp;
                    seen_split |= bp.abs_diff(sp) > 1;
                }
            }
            seen_filled |= groups[0].members.len() > tokens.len();
            checked += 1;
        }
    }

    assert!(seen_inverted && seen_split && seen_filled);
    assert!(checked > 1000, "全数の件数が少なすぎる: {checked}");
}

/// 檻の較正: 同一スコープ対の除外を外した繋ぎ（連続対の全部）を当てると、全数の検査器は
/// 組の結び目との二重書きを赤で掴む。
#[test]
fn t_zcw05_the_checker_catches_a_pair_edge_written_by_the_chain() {
    let groups = [tag_group(1, &["s0", "b0", "b1", "s1"])];
    let mut plan = compose_full(&groups, &[0, 1]);
    plan.cross_edges = plan
        .members
        .windows(2)
        .map(|w| CrossEdge {
            owned: w[0],
            owner: w[1],
            segment: ChainSegment::Group(1),
        })
        .collect();

    assert!(!final_owner_violations(&plan, &[0, 1]).is_empty());
}
