//! 鎖の規則——キャラ窓の横の連なり（鎖）を、どの道も同じ 1 つの規則で読む
//! （audit-placement-core research 18.10「連鎖の決まりの総括」）。
//!
//! # 規則
//!
//! - ⑴ **鎖に入るスコープと鎖の前の窓**: スコープは、揃え方が左右の辺でなければ鎖に入る
//!   （[`joins_chain`]）。左右の辺のスコープは辺に置かれ、横には動かさず、鎖の組の基準にもしない
//!   （resolver の P1'）。鎖の前の窓は、n より下の番号で鎖に入る一番近いスコープ
//!   （[`ChainRule::prev`]）。無ければ n は鎖の頭で、組を持たない（動かさない・印を書かない）。
//!   どの揃え方で見るかは道で決まる（[`ChainView`]）:
//!   - 既定の並び（resolver）と、次の起動が読む隣り合いの印（保存）は descript の揃え方。台本の
//!     吸着はその回かぎりで、次の起動には残らない。
//!   - 起動の確定は、descript の鎖から、窓が今左右の辺に揃っているスコープを外す（既定の並びと
//!     覚えた印を今の画面へ当てる道。今辺にある窓は次の窓の基準にしない）。
//!   - 拡大率の後の詰め直しは、窓の今の揃え方だけ（変わる直前の画面を読む道）。
//! - ⑵ **隙間と隣り合い**: 組（鎖の前の窓, n）の決まった隙間は n の descript の `defaultx`
//!   （下端・上端だけ。[`ChainTerms::gap`]＝[`chain_gap`]・DPI で換算しない）で、resolver は
//!   `x(n) = x(鎖の前の窓) − w(n) − 隙間` で置く。組が隣り合うのは、向かい合う端の隙間が 0 か、
//!   左の隙間がこの隙間のとき（どちらも ±2px・`chain_finalize::judge_chain_adjacency`）。右は
//!   隙間 0 だけ。
//! - ⑶ **印**: n の隣り合いの印は、descript の鎖の前の窓との組の ⑵ の結果（`left`・`right`・
//!   `none`）。n が辺のスコープか窓の組に入っていれば `none`。覚えた `left` は ⑵ の隙間で付け直す
//!   （利用者がドラッグで隙間 0 に付けた組も、次の起動では `defaultx` だけ離れる）。
//! - ⑷ **辺のスコープの隣に付けた窓**（詰め直しだけ）: 鎖の前の窓へ付け直さない窓が、変わる直前に
//!   飛ばした辺のスコープの隣に隙間 0 で付いていたら、その辺のスコープを動かさない基準にして
//!   同じ左右の側へ付け直す（[`edge_partner`]）。
//!
//! # 道ごとの当て方
//!
//! | 道 | 鎖の見方 | すること |
//! |---|---|---|
//! | 既定の並び（`resolver::resolve_placement`） | descript | ⑵ の式で置く |
//! | 起動の確定（[`boot_chain`]） | descript かつ今 | 記憶に位置の無い既定のままの窓は resolver と同じ決め、記憶の窓は覚えた印（[`boot_chain_decision`]） |
//! | 隣り合いの印の保存（`persist::persist_entries`） | descript | 画面の並びから ⑶ の印を書く |
//! | 拡大率の後の詰め直し（`chain_realign`） | 今 | 変わる直前に ⑵ で隣り合っていた組を付け直す。⑷ |
//! | シェルの切り替え（`reseed`） | — | 鎖は解き直さない。descript の値（材料）と窓の揃え方を入れ替えるだけ |
//!
//! 残す限り（research 18.10「この仕様で行く」）: 辺のスコープの隣に付けた窓の並びは記憶に残らない
//! （印は descript の鎖の前の窓との組だけ）。台本が揃え方を変えた回の保存は、descript の鎖で印を
//! 決める。台本が揃え方を変えた窓の扱いの限りは [`ChainView::joins`] に書く。辺のスコープの隣へ
//! 番号の小さい側の窓を付けた組は、詰め直しで付け直さない（番号の小さい側は動かさない）。
//!
//! 本体が使うのは [`boot_chain`] だけで、ほかは配置の crate の中の 4 つの道が使う（記憶から戻した
//! 窓の組の寄せ `persist::restore_and_judge` は [`ChainRule::from_terms`] で `ChainView::Saved` の鎖を組む）。

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use tracing::{debug, warn};

use super::chain_finalize::{
    Adjacency, AdjacencyMark, ChainRect, ChainSlot, PairDecision, ScopeChainState, ScriptPlaced,
    boot_default_decision, boot_pair_decision, chain_gap, join_left, joins_chain, judge_adjacency,
};
use super::config::{ScopeConfig, build_placement_config};
use super::follow::Anchored;
use super::reset_pos::DefaultPlacementBasis;
use super::resolver::{Anchor, PointPx};
use super::spawn::GhostWindows;
use super::sticky_group_ledger::StickyGroupLedger;

/// 1 スコープの鎖の見方（規則の入力）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChainTerms {
    /// descript の揃え方（次の起動でも効く）。
    pub descript: Anchor,
    /// descript の `defaultx`（生の値）。
    pub default_x: Option<i32>,
    /// 窓の今の揃え方（`follow::Anchored`・台本の吸着を含む。無ければ descript と同じ）。
    pub live: Anchor,
}

impl ChainTerms {
    /// 鎖の前の窓の左に空ける隙間（規則 ⑵）。
    pub(crate) fn gap(&self) -> i32 {
        chain_gap(self.descript, self.default_x)
    }
}

/// どの道の鎖か（規則 ⑴）。
#[derive(Debug, Clone, Copy)]
pub(crate) enum ChainView {
    /// 既定の並びと、隣り合いの印の保存: descript の揃え方。
    Saved,
    /// 起動の確定: descript の鎖から、今左右の辺に揃っている窓を外す。
    Boot,
    /// 拡大率の後の詰め直し: 窓の今の揃え方。
    Realign,
}

impl ChainView {
    /// この道で、スコープが鎖に入るか。
    pub(crate) fn joins(self, terms: &ChainTerms) -> bool {
        match self {
            Self::Saved => joins_chain(terms.descript),
            // shortcut: 台本が最初の確定より前に辺へ揃えた窓は、本体（鎖の頭）でも外すので、次の
            // スコープは鎖の頭になり今の幅で付け直さない（descript で辺に揃えた本体と同じく、辺の
            // 窓とは鎖の組にしない）。右の辺の本体の隣へ付け直すなら、鎖の頭だけは今の揃え方を見ない。
            Self::Boot => joins_chain(terms.descript) && joins_chain(terms.live),
            // shortcut: descript が辺で台本が辺から外した窓も鎖に入るので、その次のスコープの組の
            // 相手はその窓になる（本体の隣に付いていても付け直さない。詰め直しは画面の今だけを
            // 読む）。塞ぐなら、組の相手を変わる直前に隣り合っていた下の番号の窓から選ぶ。
            Self::Realign => joins_chain(terms.live),
        }
    }
}

/// 窓の一式の鎖の見方（スコープ昇順）。起動の確定・隣り合いの印の保存・詰め直しが同じこの型で
/// 鎖に入るか・鎖の前の窓・隙間を引く。
#[derive(Debug)]
pub(crate) struct ChainRule {
    scopes: Vec<(usize, ChainTerms)>,
    has_basis: bool,
}

impl ChainRule {
    /// World から組む。descript の揃え方と `defaultx` は並べ直しの材料（[`DefaultPlacementBasis`]）
    /// から、今の揃え方は `windows`（スコープ, キャラ窓）の [`Anchored`] から読む。材料が無ければ
    /// 全スコープを下端・`defaultx` 無しと見る（[`ChainRule::has_basis`] が偽。記録は呼び手が残す）。
    pub(crate) fn of(world: &World, windows: impl IntoIterator<Item = (usize, Entity)>) -> Self {
        let config = world
            .get_resource::<DefaultPlacementBasis>()
            .map(|b| build_placement_config(&b.source.ghost_kv, &b.source.shell_kv));
        let fallback = ScopeConfig::default();
        let scopes = windows
            .into_iter()
            .map(|(scope, window)| {
                let sc = config
                    .as_ref()
                    .and_then(|c| c.scopes.get(&scope))
                    .unwrap_or(&fallback);
                let descript = Anchor::from_alignment(&sc.alignment);
                let terms = ChainTerms {
                    descript,
                    default_x: sc.default_x,
                    live: world.get::<Anchored>(window).map_or(descript, |a| a.0),
                };
                (scope, terms)
            })
            .collect();
        Self::sorted(scopes, config.is_some())
    }

    /// 窓の一式（[`GhostWindows`]）の全スコープで組む（無ければ空）。
    pub(crate) fn of_ghost_windows(world: &World) -> Self {
        let windows = world.get_resource::<GhostWindows>();
        Self::of(
            world,
            windows
                .into_iter()
                .flat_map(|gw| gw.scopes().filter_map(|s| Some((s, gw.char_window(s)?)))),
        )
    }

    /// 見方の列から組む（World を持たない記憶の戻し `persist::restore_and_judge` と檻の口）。
    pub(crate) fn from_terms(scopes: Vec<(usize, ChainTerms)>) -> Self {
        Self::sorted(scopes, true)
    }

    fn sorted(mut scopes: Vec<(usize, ChainTerms)>, has_basis: bool) -> Self {
        scopes.sort_by_key(|&(scope, _)| scope);
        Self { scopes, has_basis }
    }

    /// 並べ直しの材料から組めたか。
    pub(crate) fn has_basis(&self) -> bool {
        self.has_basis
    }

    /// スコープの見方。列に無いスコープは下端・`defaultx` 無し。
    pub(crate) fn terms(&self, scope: usize) -> ChainTerms {
        self.scopes.iter().find(|&&(s, _)| s == scope).map_or(
            ChainTerms {
                descript: Anchor::Bottom,
                default_x: None,
                live: Anchor::Bottom,
            },
            |&(_, terms)| terms,
        )
    }

    /// この道で、スコープが鎖に入るか（規則 ⑴）。
    pub(crate) fn joins(&self, view: ChainView, scope: usize) -> bool {
        view.joins(&self.terms(scope))
    }

    /// 鎖の前の窓の左に空ける隙間（規則 ⑵）。
    pub(crate) fn gap(&self, scope: usize) -> i32 {
        self.terms(scope).gap()
    }

    /// 鎖の前の窓: `scope` より下の番号で鎖に入る一番近いスコープ（規則 ⑴）。
    pub(crate) fn prev(&self, view: ChainView, scope: usize) -> Option<usize> {
        self.scopes
            .iter()
            .rev()
            .find(|(s, terms)| *s < scope && view.joins(terms))
            .map(|&(s, _)| s)
    }

    /// `scope` より上の番号で鎖に入る一番近いスコープ（`scope` を鎖の前の窓とする組の後ろ側）。
    pub(crate) fn next(&self, view: ChainView, scope: usize) -> Option<usize> {
        self.scopes
            .iter()
            .find(|(s, terms)| *s > scope && view.joins(terms))
            .map(|&(s, _)| s)
    }

    /// `after`（無ければ一番下）より上・`before` より下の番号で、鎖に入らないスコープ
    /// （`before` に近い順）。
    pub(crate) fn skipped(
        &self,
        view: ChainView,
        after: Option<usize>,
        before: usize,
    ) -> impl Iterator<Item = usize> + '_ {
        self.scopes
            .iter()
            .rev()
            .filter(move |(s, terms)| {
                *s < before && after.is_none_or(|a| *s > a) && !view.joins(terms)
            })
            .map(|&(s, _)| s)
    }
}

/// 起動の確定で組（鎖の前の窓, n）に当てる決め。[`boot_pair_decision`] の結果のうち、記憶に位置の
/// 無い既定のままのスコープは resolver と同じ揃え方の決め（[`boot_default_decision`]）に、覚えた印
/// `left` は descript の `defaultx` の隙間で付ける決め（規則 ⑶）に改める。
pub(crate) fn boot_chain_decision(
    state: &ScopeChainState,
    terms: &ChainTerms,
    script_placed: bool,
    in_sticky_group: bool,
    mark: Option<AdjacencyMark>,
) -> PairDecision {
    match boot_pair_decision(state, script_placed, in_sticky_group, mark) {
        PairDecision::JoinLeft if state.default_x.is_some() => {
            boot_default_decision(terms.descript, terms.default_x)
        }
        PairDecision::JoinLeft => join_left(terms.gap()),
        decision => decision,
    }
}

/// 起動の確定の鎖（[`boot_chain`] の結果）: 鎖に入るスコープだけの解きの列と、同じ順の走査の列。
#[derive(Debug, Default)]
pub struct BootChain {
    /// 解きの列（先頭が鎖の頭）。
    pub slots: Vec<ChainSlot>,
    /// `slots` と同じ順の状態。
    pub states: Vec<ScopeChainState>,
    /// `slots` と同じ順の (スコープ, キャラ窓, 今の位置)。
    pub targets: Vec<(usize, Entity, PointPx)>,
}

/// 起動の確定の走査（`states` と `targets` は同じ順・スコープ昇順）から、鎖に入るスコープの解きの
/// 列を組む。鎖に入らないスコープ（descript か今の揃え方が左右の辺）は列から外し、`debug!` を 1 行
/// 残す。材料（[`DefaultPlacementBasis`]）が無ければ `warn!` を 1 行出し、下端・`defaultx` 0 で組む。
pub fn boot_chain(
    world: &World,
    states: &[ScopeChainState],
    targets: &[(usize, Entity, PointPx)],
) -> BootChain {
    let rule = ChainRule::of(
        world,
        targets.iter().map(|&(scope, window, _)| (scope, window)),
    );
    if !rule.has_basis() {
        warn!(
            "chain_finalize: 配置の材料（DefaultPlacementBasis）が無いので、記憶に位置の無いスコープを下端の揃え方・defaultx 0 として解く"
        );
    }
    let ledger = world.get_resource::<StickyGroupLedger>();
    let ghost_windows = world.get_resource::<GhostWindows>();
    let mut chain = BootChain::default();
    // 前のスロットの後に飛ばしたスコープがあるか。
    let mut skipped = false;
    for (s, t) in states.iter().zip(targets) {
        if !rule.joins(ChainView::Boot, s.scope) {
            debug!(
                scope = s.scope,
                x = s.current_x,
                "chain_finalize: 左右の辺のスコープは動かさず、横の連なりの基準にもしない（resolver と同じ）"
            );
            skipped = true;
            continue;
        }
        let decision = boot_chain_decision(
            s,
            &rule.terms(s.scope),
            world.get::<ScriptPlaced>(t.1).is_some(),
            ledger.is_some_and(|l| l.group_of(s.scope as u32).is_some()),
            ghost_windows.and_then(|gw| gw.remembered_adjacency(s.scope)),
        );
        // 辺のスコープを飛ばした組で覚えた印を読む（辺のスコープの次の印は、鎖の規則の前は
        // 辺のスコープとの組として書かれていた・まれなので受け入れて記録に残す）。
        if let Some(prev) = chain.slots.last()
            && skipped
            && s.default_x.is_none()
            && matches!(
                decision,
                PairDecision::JoinLeft
                    | PairDecision::JoinLeftOffset { .. }
                    | PairDecision::JoinRight
            )
        {
            debug!(
                prev_scope = prev.scope,
                scope = s.scope,
                "chain_finalize: 覚えた印を、左右の辺のスコープを飛ばした鎖の前の窓との組として読む"
            );
        }
        skipped = false;
        chain.slots.push(ChainSlot {
            scope: s.scope,
            current_x: s.current_x,
            width: s.width,
            decision,
        });
        chain.states.push(*s);
        chain.targets.push(*t);
    }
    chain
}

/// 詰め直しで、鎖の前の窓と隣り合っていなかった窓 `cur` の、辺のスコープの相手を選ぶ。`edges` は
/// 飛ばした辺のスコープの変わる直前の矩形（`cur` に近い順）。隙間 0 で左右に隣り合っていた最初の
/// スコープと、その隣り合いを返す（辺のスコープとの組に `defaultx` の隙間は無い）。
pub(crate) fn edge_partner(
    cur: ChainRect,
    edges: impl IntoIterator<Item = (usize, ChainRect)>,
) -> Option<(usize, Adjacency)> {
    edges.into_iter().find_map(|(scope, edge)| {
        let adjacency = judge_adjacency(edge, cur);
        matches!(adjacency, Adjacency::Left { .. } | Adjacency::Right { .. })
            .then_some((scope, adjacency))
    })
}

#[cfg(test)]
#[path = "chain_rule_tests.rs"]
mod chain_rule_tests;
