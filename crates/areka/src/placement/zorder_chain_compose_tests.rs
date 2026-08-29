//! 望む鎖の合成の決定論的テスト（要件 1.1／1.2／1.4／2.5／3.6／6.1／6.2／8.4／
//! 10.1／14.1／14.4／15.1／15.2）。
//!
//! 実機も実ディスプレイも Win32 も要らない（要件 10.1）。`Entity` は生の索引から組んだ
//! 値であり World へは 1 度も渡さない。窓の在庫は `Inventory` という表で与える
//! ——本番で `GhostWindows` が果たす役目（スコープ×種別 → 窓の有無）だけを写した
//! ものであり、判断の分岐は在庫表の中身だけで決まる。
//!
//! # 檻の入力は両側から挟む
//!
//! 「取り除かない」「動かさない」を主張する検査は、動くはずの側が本当に動く対照を
//! 併置しないと、道具が壊れていても緑になる。よって不在要素・後方参加・登記順の各分岐は
//! 「在る側」と「無い側」を対にして置いてある。
//!
//! # 摂動は「経路から外す」形で当てる
//!
//! 本モジュールが張る繋ぎの規則は 2 つ——⑴連続対のうち**同一スコープの
//! （バルーン, キャラ窓）対は張らない**（それは既存のペア機構の担当）、
//! ⑵鎖は**1 本**であり、繋ぎは連続対だけから生まれる（グループ境界を跨いで余分に
//! 繋がない）。この 2 つを経路から外した出力を檻の中で組み立て、
//! `chain_invariant_violations` と `pair_edges_written` がそれを赤で掴むことを
//! 対照として固定する。値をずらす（平行移動する）摂動では、連続対を舐める形が
//! ずれた値も吸収してしまい、檻が強く見えるだけになる。

use std::collections::{HashMap, HashSet};

use bevy_ecs::entity::Entity;
use wintf::ecs::window::{ChainPlan, ChainSegment, CrossEdge, log_chain_absent};

use super::compose_chain;
use crate::placement::zorder_group_ledger::{
    GroupElement, GroupSource, GroupWindowKind, ZOrderGroup, ZOrderGroupLedger, parse_zorder_tokens,
};

// ===========================================================================
// 道具立て
// ===========================================================================

/// バルーン窓の要素。
fn b(scope: u32) -> GroupElement {
    GroupElement {
        scope,
        kind: GroupWindowKind::Balloon,
    }
}

/// キャラ窓の要素。
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

/// 要素列を `Entity` 列へ。
fn ents(elements: &[GroupElement]) -> Vec<Entity> {
    elements.iter().copied().map(ent).collect()
}

/// 手前側 `front` が奥側 `back` に所有される繋ぎ 1 本（区間つき）。
///
/// 区間は**手前側の枠が属していたもの**である（合成の規則そのもの）。ここで書き下すのは
/// 「どのグループの繋ぎか」が記録から復元できることの主張であり、値を取り違えると赤くなる。
fn edge(front: GroupElement, back: GroupElement, segment: ChainSegment) -> CrossEdge {
    CrossEdge {
        owned: ent(front),
        owner: ent(back),
        segment,
    }
}

/// グループ `id` に属する繋ぎ 1 本。
fn g(id: u32) -> ChainSegment {
    ChainSegment::Group(id)
}

/// 窓の在庫（本番の `GhostWindows` が果たす「スコープ×種別 → 窓の有無」だけを写した表）。
#[derive(Debug, Clone, Default)]
struct Inventory {
    windows: HashSet<GroupElement>,
}

impl Inventory {
    /// 名指しした窓だけが在る在庫。
    fn of(elements: &[GroupElement]) -> Self {
        Self {
            windows: elements.iter().copied().collect(),
        }
    }

    /// 各スコープの 2 窓がそろって在る在庫。
    fn full(scopes: &[u32]) -> Self {
        let mut windows = HashSet::new();
        for &scope in scopes {
            windows.insert(b(scope));
            windows.insert(s(scope));
        }
        Self { windows }
    }

    /// 在庫にあるスコープを昇順で（本番の `GhostWindows::scopes` に相当）。
    fn scopes(&self) -> Vec<u32> {
        let mut scopes: Vec<u32> = self.windows.iter().map(|e| e.scope).collect();
        scopes.sort_unstable();
        scopes.dedup();
        scopes
    }

    /// 要素 1 つを実在する窓へ解決する（実在しなければ `None`）。
    fn resolve(&self, element: &GroupElement) -> Option<Entity> {
        self.windows.contains(element).then(|| ent(*element))
    }
}

/// 在庫から合成する（在庫のスコープをそのまま `all_scopes` として渡す形）。
fn compose(groups: &[ZOrderGroup], inventory: &Inventory) -> Option<ChainPlan> {
    let scopes = inventory.scopes();
    compose_chain(groups, &scopes, &|element| inventory.resolve(element))
}

/// タグ由来のグループ 1 本を、本番と同じ解釈器を通して組む。
fn tag_group(id: u32, tokens: &[&str]) -> ZOrderGroup {
    let (members, _normalizations) =
        parse_zorder_tokens(tokens).expect("テストの指定は受理されるもの");
    ZOrderGroup {
        id,
        members,
        source: GroupSource::Tag,
    }
}

/// 台帳を通さずに要素列を直接持たせたグループ（解釈器が作れない形＝要素 1 個以下の検査用）。
fn raw_group(id: u32, members: &[GroupElement]) -> ZOrderGroup {
    ZOrderGroup {
        id,
        members: members.to_vec(),
        source: GroupSource::Tag,
    }
}

// ===========================================================================
// 不変条件の検査器（要件 14.4）
// ===========================================================================

/// 鎖の 4 つの不変条件を検査し、破れているものを人が読める形で並べて返す。
///
/// ⑴ある窓を所有する窓は高々 1 つ（`owner` が 2 度現れない＝星形にならない）
/// ⑵ある窓が所有される回数も高々 1 回（`owned` が 2 度現れない＝輪にも分岐にもならない）
/// ⑶繋ぎの両端は必ず鎖の要素
/// ⑷同じ窓は鎖に 2 度現れない
///
/// 「無い」を主張する検査なので、下の `the_invariant_checker_catches_every_broken_shape`
/// が 4 つとも実際に掴むことを既知の壊れた形で較正している。
fn chain_invariant_violations(plan: &ChainPlan) -> Vec<String> {
    let mut violations = Vec::new();

    let mut owners: HashMap<Entity, usize> = HashMap::new();
    let mut owneds: HashMap<Entity, usize> = HashMap::new();
    for e in &plan.cross_edges {
        *owners.entry(e.owner).or_default() += 1;
        *owneds.entry(e.owned).or_default() += 1;
    }
    for (entity, count) in &owners {
        if *count > 1 {
            violations.push(format!("①星形: {entity:?} が {count} 枚を所有している"));
        }
    }
    for (entity, count) in &owneds {
        if *count > 1 {
            violations.push(format!(
                "②多重被所有: {entity:?} が {count} 回所有されている"
            ));
        }
    }

    let members: HashSet<Entity> = plan.members.iter().copied().collect();
    for e in &plan.cross_edges {
        if !members.contains(&e.owned) || !members.contains(&e.owner) {
            violations.push(format!("③鎖の外の窓を繋いでいる: {e:?}"));
        }
    }

    if members.len() != plan.members.len() {
        violations.push(format!(
            "④同じ窓が鎖に 2 度以上現れている: {:?}",
            plan.members
        ));
    }

    violations.sort();
    violations
}

/// 繋ぎの中に「同一スコープの（バルーン, キャラ窓）対」が混ざっているスコープを拾う。
///
/// この対はスコープ内ペア機構の担当であり、本 spec は 1 本も張らない（境界）。
fn pair_edges_written(plan: &ChainPlan, scopes: &[u32]) -> Vec<u32> {
    scopes
        .iter()
        .copied()
        .filter(|&scope| {
            // 端点だけで照らす——区間の値に依らせると、区間が変わっただけで
            // 「ペアの繋ぎを張っていない」が偽の緑になる。
            plan.cross_edges
                .iter()
                .any(|e| e.owned == ent(b(scope)) && e.owner == ent(s(scope)))
        })
        .collect()
}

/// 連続対を 1 つも除かずに繋いだ形＝「同一スコープ対の除外」を経路から外した出力。
fn all_consecutive_edges(members: &[Entity]) -> Vec<CrossEdge> {
    members
        .windows(2)
        .map(|w| CrossEdge {
            owned: w[0],
            owner: w[1],
            segment: ChainSegment::Tail,
        })
        .collect()
}

/// 合成が成立し、4 つの不変条件を満たし、ペア対を 1 本も張っていないことを確かめて返す。
fn composed(groups: &[ZOrderGroup], inventory: &Inventory) -> ChainPlan {
    let plan = compose(groups, inventory).expect("グループがあるので計画は組まれる");
    assert!(
        chain_invariant_violations(&plan).is_empty(),
        "不変条件が破れている: {:?}",
        chain_invariant_violations(&plan)
    );
    let scopes = inventory.scopes();
    assert!(
        pair_edges_written(&plan, &scopes).is_empty(),
        "スコープ内ペアの繋ぎを張っている（ペア機構の担当・境界違反）: {:?}",
        pair_edges_written(&plan, &scopes)
    );
    plan
}

// ===========================================================================
// 分岐: 数値モード・明示モード・畳み込み
// ===========================================================================

/// 数値モードは、並びの左のスコープほど手前で、各スコープが 2 枚のかたまりになる
/// （要件 1.1／1.2）。張る繋ぎはスコープの継ぎ目 1 本だけである。
#[test]
fn t_zcc01_numeric_mode_puts_left_scope_in_front_as_a_block() {
    let groups = [tag_group(1, &["1", "0"])];
    let inventory = Inventory::full(&[0, 1]);

    let plan = composed(&groups, &inventory);

    assert_eq!(plan.members, ents(&[b(1), s(1), b(0), s(0)]));
    assert_eq!(plan.cross_edges, vec![edge(s(1), b(0), g(1))]);
    assert!(plan.absent.is_empty(), "不在要素は無い: {:?}", plan.absent);
    assert_eq!(
        plan.members.last().copied(),
        Some(ent(s(0))),
        "鎖の根は末尾の窓"
    );
}

/// 明示モードで 4 枚を書いた指定は、数値モードの同じ並びと同一の鎖になる（要件 2.1）。
#[test]
fn t_zcc02_explicit_mode_matches_the_numeric_shape() {
    let inventory = Inventory::full(&[0, 1]);

    let explicit = composed(&[tag_group(1, &["b1", "s1", "b0", "s0"])], &inventory);
    let numeric = composed(&[tag_group(1, &["1", "0"])], &inventory);

    assert_eq!(explicit.members, numeric.members);
    assert_eq!(explicit.cross_edges, numeric.cross_edges);
}

/// 片方だけ指名された明示モード（畳み込み・要件 2.6）でも、鎖はスコープ 2 枚の
/// かたまりで並ぶ。相棒窓は台帳が補っているので、合成は特別扱いを持たない。
#[test]
fn t_zcc03_folded_partial_specification_yields_full_scope_blocks() {
    let groups = [tag_group(1, &["b1", "s0"])];
    let inventory = Inventory::full(&[0, 1]);

    let plan = composed(&groups, &inventory);

    assert_eq!(plan.members, ents(&[b(1), s(1), b(0), s(0)]));
    assert_eq!(plan.cross_edges, vec![edge(s(1), b(0), g(1))]);
}

// ===========================================================================
// 分岐: 不在要素（要件 1.4／8.4）
// ===========================================================================

/// 窓が 1 枚も無いスコープは、グループから取り除かれず「窓が無かった要素」として返る。
/// 残った窓だけで指定順の相対順が成立する（要件 1.4／8.4）。
#[test]
fn t_zcc04_absent_scope_is_reported_and_never_dropped_from_the_group() {
    let groups = [tag_group(1, &["0", "1"])];
    let present = Inventory::full(&[0]);

    let plan = composed(&groups, &present);

    assert_eq!(plan.members, ents(&[b(0), s(0)]));
    assert!(
        plan.cross_edges.is_empty(),
        "同一スコープの対しか無いので張る繋ぎは 0 本: {:?}",
        plan.cross_edges
    );
    assert_eq!(
        plan.absent,
        vec![(1, "b1".to_string()), (1, "s1".to_string())],
        "不在要素は宣言したグループの ID を伴う（要件 8.4）"
    );

    // 対照: 窓がそろえば同じ指定が繋ぎを 1 本生み、不在要素は空になる。
    let both = composed(&groups, &Inventory::full(&[0, 1]));
    assert_eq!(both.cross_edges, vec![edge(s(0), b(1), g(1))]);
    assert!(both.absent.is_empty());
}

/// スコープの片割れだけが実在するときは、その 1 枚が鎖に入り、
/// 欠けた 1 枚だけが不在要素として並ぶ（宣言順のまま）。
#[test]
fn t_zcc05_partially_present_scope_keeps_the_surviving_window_in_the_chain() {
    let groups = [tag_group(1, &["0", "1"])];
    let inventory = Inventory::of(&[b(0), s(0), b(1)]);

    let plan = composed(&groups, &inventory);

    assert_eq!(plan.members, ents(&[b(0), s(0), b(1)]));
    assert_eq!(plan.cross_edges, vec![edge(s(0), b(1), g(1))]);
    assert_eq!(plan.absent, vec![(1, "s1".to_string())]);
}

/// 1 枚も実在しないときも計画そのものは組まれる（指定は生きている）。
/// 鎖は空で、繋ぎも 0 本、不在要素だけが宣言順に並ぶ。
#[test]
fn t_zcc06_group_with_no_existing_window_still_yields_a_plan_with_absent_elements() {
    let groups = [tag_group(1, &["0", "1"])];
    let inventory = Inventory::default();

    let plan = compose(&groups, &inventory).expect("グループがある以上、計画は組まれる");

    assert!(plan.members.is_empty());
    assert!(plan.cross_edges.is_empty());
    assert_eq!(
        plan.absent,
        vec![
            (1, "b0".to_string()),
            (1, "s0".to_string()),
            (1, "b1".to_string()),
            (1, "s1".to_string())
        ]
    );
}

// ===========================================================================
// 分岐: 指定ゼロ＝既定状態（要件 6.1／6.2／6.4）
// ===========================================================================

/// グループが 1 つも無ければ計画そのものを作らない——窓が何枚在っても、
/// どのスコープの前後も規定しない（要件 6.1／6.2／6.4）。
#[test]
fn t_zcc07_no_group_means_no_plan_at_all() {
    let inventory = Inventory::full(&[0, 1, 2]);

    assert!(
        compose(&[], &inventory).is_none(),
        "既定状態では計画を作らない"
    );

    // 対照: 同じ在庫でもグループが 1 本あれば計画は組まれる（検査器の空振りではない）。
    assert!(compose(&[tag_group(1, &["0", "1"])], &inventory).is_some());
}

// ===========================================================================
// 分岐: 複数グループの登記順連結（要件 3.6）
// ===========================================================================

/// グループどうしは登記の順で 1 本に連なる。shell 設定由来の基底が最前で、
/// 以降はタグの登記順（先に登記されたほど手前）である（要件 3.6）。
#[test]
fn t_zcc08_groups_are_concatenated_in_registration_order_with_the_descript_base_in_front() {
    let mut ledger = ZOrderGroupLedger::default();
    ledger.set_descript_base(parse_zorder_tokens(&["3", "2"]).expect("受理される").0);
    ledger
        .try_add_tag_group(parse_zorder_tokens(&["0", "1"]).expect("受理される").0)
        .expect("重複しないので受理される");
    let inventory = Inventory::full(&[0, 1, 2, 3]);

    let plan = composed(ledger.groups(), &inventory);

    assert_eq!(
        plan.members,
        ents(&[b(3), s(3), b(2), s(2), b(0), s(0), b(1), s(1)]),
        "基底が最前・以降は登記順"
    );
    assert_eq!(
        plan.cross_edges,
        vec![
            edge(s(3), b(2), g(0)),
            edge(s(2), b(0), g(0)),
            edge(s(0), b(1), g(1))
        ],
        "繋ぎはスコープの継ぎ目だけ（グループの境目も継ぎ目の 1 つ）。         区間は手前側の枠のもの——基底（g0）の末尾からタグ由来（g1）へ渡る繋ぎは g0 が名乗る"
    );
}

// ===========================================================================
// 分岐: 未指定スコープの後方参加（要件 15.1／15.2）
// ===========================================================================

/// どのグループにも属さないスコープは、全グループの後ろへ、スコープ ID の昇順で、
/// 2 枚のかたまりとして連なる（要件 15.1／15.2）。渡された在庫の並びには依らない。
#[test]
fn t_zcc09_unassigned_scopes_join_behind_every_group_in_ascending_scope_order() {
    let groups = [tag_group(1, &["b1", "s1"])];
    let inventory = Inventory::full(&[0, 1, 2, 3]);

    // 在庫の並びを降順で渡しても、後方の並びは昇順に決まる。
    let plan = compose_chain(&groups, &[3, 0, 2, 1], &|e| inventory.resolve(e))
        .expect("グループがあるので計画は組まれる");

    assert!(chain_invariant_violations(&plan).is_empty());
    assert_eq!(
        plan.members,
        ents(&[b(1), s(1), b(0), s(0), b(2), s(2), b(3), s(3)])
    );
    assert_eq!(
        plan.cross_edges,
        vec![
            edge(s(1), b(0), g(1)),
            edge(s(0), b(2), ChainSegment::Tail),
            edge(s(2), b(3), ChainSegment::Tail)
        ],
        "グループの繋ぎと後方配置の繋ぎが、同じ 1 本の鎖の中で別々の区間を名乗る"
    );
    assert!(plan.absent.is_empty(), "後方参加は宣言要素ではない");
}

/// 後方参加のスコープも、片割れしか実在しなければその 1 枚だけが鎖に入る。
/// 未指定スコープは宣言要素ではないので、欠けた窓は不在要素に載らない（要件 8.4 の射程）。
#[test]
fn t_zcc10_partially_present_tail_scope_contributes_only_the_existing_window() {
    let groups = [tag_group(1, &["b0", "s0"])];
    let inventory = Inventory::of(&[b(0), s(0), s(2)]);

    let plan = composed(&groups, &inventory);

    assert_eq!(plan.members, ents(&[b(0), s(0), s(2)]));
    assert_eq!(plan.cross_edges, vec![edge(s(0), s(2), g(1))]);
    assert!(
        plan.absent.is_empty(),
        "未指定スコープの欠けは記録しない: {:?}",
        plan.absent
    );
}

/// グループに名前の挙がったスコープは、在庫にも居るからといって後方へ二重参加しない
/// （不変条件④の要）。
#[test]
fn t_zcc11_a_scope_named_by_a_group_never_joins_the_tail_as_well() {
    let groups = [tag_group(1, &["0", "1"])];
    let inventory = Inventory::full(&[0, 1, 2]);

    let plan = composed(&groups, &inventory);

    assert_eq!(
        plan.members,
        ents(&[b(0), s(0), b(1), s(1), b(2), s(2)]),
        "0・1 はグループの位置に 1 度だけ現れ、後方には 2 だけが来る"
    );
    assert_eq!(plan.members.len(), 6, "重複参加があれば長さが増える");
}

/// 在庫のスコープが重複して渡されても、鎖には 1 度しか現れない（不変条件④）。
#[test]
fn t_zcc12_duplicated_inventory_scopes_do_not_duplicate_chain_members() {
    let groups = [tag_group(1, &["b0", "s0"])];
    let inventory = Inventory::full(&[0, 1]);

    let plan = compose_chain(&groups, &[1, 1, 1], &|e| inventory.resolve(e))
        .expect("グループがあるので計画は組まれる");

    assert!(chain_invariant_violations(&plan).is_empty());
    assert_eq!(plan.members, ents(&[b(0), s(0), b(1), s(1)]));
}

// ===========================================================================
// 分岐: 要素 1 個以下のグループ
// ===========================================================================

/// 要素 1 個のグループも、要素 0 個のグループも、鎖の連結を壊さない。
/// （台帳は要素 2 個未満のタグを受理しないので正典の経路では現れないが、
/// 合成は台帳の受理規則を前提にせず、渡された要素列をそのまま連ねる。）
#[test]
fn t_zcc13_groups_with_one_or_zero_elements_still_concatenate() {
    let groups = [
        raw_group(1, &[b(0)]),
        raw_group(2, &[]),
        tag_group(3, &["b1", "s1"]),
    ];
    let inventory = Inventory::full(&[0, 1]);

    let plan = composed(&groups, &inventory);

    assert_eq!(
        plan.members,
        ents(&[b(0), b(1), s(1)]),
        "s0 はどのグループにも書かれておらず、スコープ 0 は既にグループに属するので後方にも来ない"
    );
    assert_eq!(
        plan.cross_edges,
        vec![edge(b(0), b(1), g(1))],
        "相手の居ないバルーンは奥側のバルーンに所有される"
    );
    assert!(plan.absent.is_empty());
}

/// 要素 0 個のグループしか無く、在庫も空なら、鎖は空だが計画は `Some` である
/// ——「グループが 1 つも無い」（`None`）とは別の状態である。
#[test]
fn t_zcc14_an_empty_group_is_not_the_same_state_as_having_no_group() {
    let empty_inventory = Inventory::default();

    let plan = compose(&[raw_group(1, &[])], &empty_inventory).expect("グループは 1 本ある");
    assert!(plan.members.is_empty());
    assert!(plan.cross_edges.is_empty());
    assert!(plan.absent.is_empty());

    assert!(compose(&[], &empty_inventory).is_none(), "こちらは既定状態");
}

// ===========================================================================
// 不変条件の全網羅（要件 14.4）
// ===========================================================================

/// 分岐表のすべての形について、4 つの不変条件が同時に成り立つ。
#[test]
fn t_zcc15_every_branch_satisfies_all_four_chain_invariants() {
    let cases: Vec<(&str, Vec<ZOrderGroup>, Inventory)> = vec![
        (
            "数値モード 1 グループ",
            vec![tag_group(1, &["1", "0"])],
            Inventory::full(&[0, 1]),
        ),
        (
            "明示モード 1 グループ",
            vec![tag_group(1, &["b1", "s1", "b0", "s0"])],
            Inventory::full(&[0, 1]),
        ),
        (
            "畳み込み",
            vec![tag_group(1, &["b1", "s0"])],
            Inventory::full(&[0, 1]),
        ),
        (
            "不在要素あり",
            vec![tag_group(1, &["0", "1"])],
            Inventory::of(&[b(0), s(0), b(1)]),
        ),
        (
            "在庫が空",
            vec![tag_group(1, &["0", "1"])],
            Inventory::default(),
        ),
        (
            "複数グループ",
            vec![tag_group(1, &["3", "2"]), tag_group(2, &["0", "1"])],
            Inventory::full(&[0, 1, 2, 3]),
        ),
        (
            "未指定スコープの後方参加",
            vec![tag_group(1, &["b1", "s1"])],
            Inventory::full(&[0, 1, 2, 3]),
        ),
        (
            "後方参加が片割れだけ",
            vec![tag_group(1, &["b0", "s0"])],
            Inventory::of(&[b(0), s(0), s(2), b(3)]),
        ),
        (
            "要素 1 個以下のグループ",
            vec![
                raw_group(1, &[b(0)]),
                raw_group(2, &[]),
                tag_group(3, &["b1", "s1"]),
            ],
            Inventory::full(&[0, 1]),
        ),
        (
            "グループの窓だけが在庫に無い",
            vec![tag_group(1, &["5", "6"])],
            Inventory::full(&[0, 1]),
        ),
    ];

    for (name, groups, inventory) in cases {
        let plan = compose(&groups, &inventory).expect("グループがあるので計画は組まれる");
        assert!(
            chain_invariant_violations(&plan).is_empty(),
            "{name}: 不変条件が破れている: {:?}",
            chain_invariant_violations(&plan)
        );
        assert!(
            pair_edges_written(&plan, &inventory.scopes()).is_empty(),
            "{name}: スコープ内ペアの繋ぎを張っている"
        );
        // 繋ぎは必ず「連続対の部分集合」であり、順序は手前から奥のまま。
        // 照らすのは端点だけである——区間は記録の欄であって並びの主張ではない。
        let consecutive = all_consecutive_edges(&plan.members);
        for e in &plan.cross_edges {
            assert!(
                consecutive
                    .iter()
                    .any(|c| c.owned == e.owned && c.owner == e.owner),
                "{name}: 連続対でない繋ぎを張っている: {e:?}"
            );
        }
        // 区間は必ず埋まる（どの分岐でも「どのグループの繋ぎか」が記録から読める）。
        for e in &plan.cross_edges {
            let named = groups
                .iter()
                .any(|g| ChainSegment::Group(g.id) == e.segment);
            assert!(
                named || e.segment == ChainSegment::Tail,
                "{name}: 実在しない区間を名乗っている: {e:?}"
            );
        }
    }
}

/// 不変条件の検査器そのものの較正——4 つの形をそれぞれ既知の壊れた計画で赤にする。
///
/// 「破れていない」を主張する検査は、検査器が壊れていても緑になる。ここで毎回赤を作る。
#[test]
fn the_invariant_checker_catches_every_broken_shape() {
    let members = ents(&[b(0), s(0), b(1), s(1)]);

    // ①星形: 1 枚が 2 枚を所有する。
    let star = ChainPlan {
        members: members.clone(),
        cross_edges: vec![
            edge(b(0), s(1), ChainSegment::Tail),
            edge(s(0), s(1), ChainSegment::Tail),
        ],
        absent: Vec::new(),
    };
    assert!(
        chain_invariant_violations(&star)
            .iter()
            .any(|v| v.starts_with('①')),
        "星形を掴めていない: {:?}",
        chain_invariant_violations(&star)
    );

    // ②多重被所有: 1 枚が 2 枚に所有される。
    let forked = ChainPlan {
        members: members.clone(),
        cross_edges: vec![
            edge(b(0), s(0), ChainSegment::Tail),
            edge(b(0), b(1), ChainSegment::Tail),
        ],
        absent: Vec::new(),
    };
    assert!(
        chain_invariant_violations(&forked)
            .iter()
            .any(|v| v.starts_with('②')),
        "多重被所有を掴めていない: {:?}",
        chain_invariant_violations(&forked)
    );

    // ③鎖の外の窓を繋いでいる。
    let outsider = ChainPlan {
        members: members.clone(),
        cross_edges: vec![edge(s(1), b(9), ChainSegment::Tail)],
        absent: Vec::new(),
    };
    assert!(
        chain_invariant_violations(&outsider)
            .iter()
            .any(|v| v.starts_with('③')),
        "鎖の外の窓を掴めていない: {:?}",
        chain_invariant_violations(&outsider)
    );

    // ④同じ窓が 2 度現れる。
    let mut doubled = members.clone();
    doubled.push(ent(b(0)));
    let repeated = ChainPlan {
        members: doubled,
        cross_edges: Vec::new(),
        absent: Vec::new(),
    };
    assert!(
        chain_invariant_violations(&repeated)
            .iter()
            .any(|v| v.starts_with('④')),
        "重複参加を掴めていない: {:?}",
        chain_invariant_violations(&repeated)
    );

    // 較正の対照: 正しい形は 1 件も挙がらない。
    let sound = ChainPlan {
        members,
        cross_edges: vec![edge(s(0), b(1), ChainSegment::Tail)],
        absent: Vec::new(),
    };
    assert!(chain_invariant_violations(&sound).is_empty());
}

/// 不在要素の記録材料が、areka 側から**そのまま鎖の語彙へ渡せる**形で届く。
///
/// `[zorder-chain] absent` を出すのは指令消化の相（出口）であって本合成ではない。
/// ここで固定するのは「出口を立てる task が、計画から `group_id` と要素の字面を取り出して
/// [`log_chain_absent`] へ渡せる」ことである——欄の型か可視性のどちらかが欠けていれば、
/// このテストはそもそもコンパイルできない（引受先が居ない欄を残さないための対照）。
#[test]
fn t_zcc18_absent_elements_can_be_handed_straight_to_the_chain_record_vocabulary() {
    let groups = [tag_group(1, &["0", "1"]), tag_group(2, &["4", "5"])];
    let plan = composed(&groups, &Inventory::full(&[0, 4]));

    // どのグループの宣言が空振りしたかが、要素ごとに読める。
    assert_eq!(
        plan.absent,
        vec![
            (1, "b1".to_string()),
            (1, "s1".to_string()),
            (2, "b5".to_string()),
            (2, "s5".to_string())
        ],
        "不在要素が宣言したグループを名乗っていない"
    );

    // 出口が行うことをそのまま写した呼び出し（型と可視性の実証）。
    for (group_id, element) in &plan.absent {
        log_chain_absent(*group_id, element);
    }
}

// ===========================================================================
// 摂動（経路から外す形）
// ===========================================================================

/// 摂動⑴——「同一スコープ対の除外」を経路から外すと、出力は連続対を 1 つも除かない形
/// になる。その形は本番の出力と一致せず、`pair_edges_written` が赤で掴む。
#[test]
fn t_zcc16_dropping_the_same_scope_pair_exclusion_is_caught() {
    let groups = [tag_group(1, &["0", "1"])];
    let inventory = Inventory::full(&[0, 1]);
    let scopes = inventory.scopes();

    let plan = composed(&groups, &inventory);

    // 除外を落とした出力（＝連続対を全部張る形）。
    let mutant = ChainPlan {
        members: plan.members.clone(),
        cross_edges: all_consecutive_edges(&plan.members),
        absent: plan.absent.clone(),
    };

    assert_ne!(
        plan.cross_edges, mutant.cross_edges,
        "除外が効いていない（本番の出力が既に連続対を全部張っている）"
    );
    assert_eq!(
        pair_edges_written(&mutant, &scopes),
        vec![0, 1],
        "摂動体ではスコープ内ペアの繋ぎが現れる（検査器が赤を出せる）"
    );
    assert!(
        pair_edges_written(&plan, &scopes).is_empty(),
        "本番の出力にはペアの繋ぎが 1 本も無い"
    );
    // 摂動体は不変条件だけでは掴めない——だからペアの検査器が要る。
    assert!(chain_invariant_violations(&mutant).is_empty());
}

/// 摂動⑵——グループ境界を跨いで余分に繋ぐと（後のグループの先頭が 2 枚を所有する形）、
/// 不変条件①が赤で止める。
#[test]
fn t_zcc17_linking_across_a_group_boundary_breaks_the_single_owner_invariant() {
    let groups = [tag_group(1, &["b0", "s0"]), tag_group(2, &["b1", "s1"])];
    let inventory = Inventory::full(&[0, 1]);

    let plan = composed(&groups, &inventory);
    assert_eq!(plan.cross_edges, vec![edge(s(0), b(1), g(1))]);

    // グループ境界を跨ぐ繋ぎを 1 本足す（後のグループの先頭が 2 枚を所有する）。
    let mut cross_edges = plan.cross_edges.clone();
    cross_edges.push(edge(b(0), b(1), ChainSegment::Tail));
    let mutant = ChainPlan {
        members: plan.members.clone(),
        cross_edges,
        absent: plan.absent.clone(),
    };

    let violations = chain_invariant_violations(&mutant);
    assert!(
        violations.iter().any(|v| v.starts_with('①')),
        "境界跨ぎの余分な繋ぎを掴めていない: {violations:?}"
    );
}
