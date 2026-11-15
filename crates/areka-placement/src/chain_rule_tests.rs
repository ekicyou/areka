// =============================================================================
// 鎖の規則の檻（audit-placement-core research 18.10 の場合の表の、World を持たない行）。
//
// 表の列（道）ごとの答えを 1 つの規則（`chain_rule`）から引けることを、表の形のまま確かめる。
// World の要る行（起動の確定・詰め直し・保存の実際の窓）は本体の `frame_chain_rule_tests.rs` ほか。
// =============================================================================

use std::collections::BTreeMap;

use wintf::ecs::DPI;

use super::super::chain_finalize::{
    AdjacencyMark, ChainRect, ChainSlot, KeepReason, PairDecision, ScopeChainState,
    adjacency_decision, judge_chain_adjacency, resolve_chain,
};
use super::super::config::build_placement_config;
use super::super::resolver::{Anchor, RectPx, ScopeInput, SizePx, resolve_placement};
use super::*;

use Anchor::{Bottom, Free, Left, Right, Top};

fn terms(descript: Anchor, default_x: Option<i32>, live: Anchor) -> ChainTerms {
    ChainTerms {
        descript,
        default_x,
        live,
    }
}

/// 規則 ⑴: 鎖に入るかは道ごとの揃え方で決まる。行は (descript, 今, 保存, 起動, 詰め直し)。
#[test]
fn chain_membership_by_view() {
    let rows = [
        // descript と今が同じ（台本が揃え方を変えていない）。
        (Bottom, Bottom, true, true, true),
        (Top, Top, true, true, true),
        (Free, Free, true, true, true),
        (Right, Right, false, false, false),
        (Left, Left, false, false, false),
        // 台本が辺へ揃えた（descript は鎖・今は辺）: 保存は descript、動かす道は外す。
        (Bottom, Left, true, false, false),
        (Bottom, Right, true, false, false),
        (Free, Right, true, false, false),
        // 台本が辺から外した（descript は辺・今は鎖）: 起動は descript の鎖に無いので外し、
        // 詰め直しは今だけを見る。
        (Right, Bottom, false, false, true),
        (Right, Free, false, false, true),
        (Left, Top, false, false, true),
    ];
    for (descript, live, saved, boot, realign) in rows {
        let t = terms(descript, None, live);
        assert_eq!(
            (
                ChainView::Saved.joins(&t),
                ChainView::Boot.joins(&t),
                ChainView::Realign.joins(&t)
            ),
            (saved, boot, realign),
            "descript {descript:?}・今 {live:?}"
        );
    }
}

/// 規則 ⑴: 鎖の前の窓は、下の番号で鎖に入る一番近いスコープ。無ければ鎖の頭。
#[test]
fn previous_chain_window_skips_scopes_outside_the_chain() {
    // 0 下端・1 右の辺・2 下端・3 台本が左の辺へ（descript は下端）・4 下端・（5 は無い）・6 下端。
    let rule = ChainRule::from_terms(vec![
        (6, terms(Bottom, None, Bottom)),
        (0, terms(Bottom, None, Bottom)),
        (1, terms(Right, None, Right)),
        (2, terms(Bottom, None, Bottom)),
        (3, terms(Bottom, None, Left)),
        (4, terms(Bottom, None, Bottom)),
    ]);
    // (見方, n, 鎖の前の窓)
    let rows = [
        (ChainView::Saved, 0, None),
        (ChainView::Saved, 2, Some(0)),
        (ChainView::Saved, 3, Some(2)),
        (ChainView::Saved, 4, Some(3)),
        (ChainView::Saved, 6, Some(4)),
        (ChainView::Boot, 2, Some(0)),
        (ChainView::Boot, 4, Some(2)),
        (ChainView::Realign, 4, Some(2)),
        (ChainView::Realign, 6, Some(4)),
    ];
    for (view, n, prev) in rows {
        assert_eq!(rule.prev(view, n), prev, "{view:?} の n={n}");
    }
    assert_eq!(rule.next(ChainView::Saved, 0), Some(2), "辺の 1 を飛ばす");
    assert_eq!(rule.next(ChainView::Saved, 2), Some(3));
    assert_eq!(rule.next(ChainView::Boot, 2), Some(4));
    assert_eq!(rule.next(ChainView::Saved, 4), Some(6), "無い 5 を飛ばす");
    assert_eq!(rule.next(ChainView::Saved, 6), None);
    assert_eq!(
        rule.skipped(ChainView::Realign, Some(0), 4)
            .collect::<Vec<_>>(),
        [3, 1],
        "飛ばしたスコープは n に近い順"
    );
    assert_eq!(
        rule.skipped(ChainView::Realign, None, 2)
            .collect::<Vec<_>>(),
        [1]
    );
    assert!(
        rule.skipped(ChainView::Saved, Some(0), 4).eq([1]),
        "保存の見方では 3 は鎖に入る"
    );
}

/// 鎖の頭が辺のスコープのとき、次のスコープが鎖の頭になる（どの道でも組を持たない）。
#[test]
fn a_scope_after_a_side_edge_head_is_the_chain_head() {
    let rule = ChainRule::from_terms(vec![
        (0, terms(Right, None, Right)),
        (1, terms(Bottom, Some(36), Bottom)),
    ]);
    for view in [ChainView::Saved, ChainView::Boot, ChainView::Realign] {
        assert_eq!(rule.prev(view, 1), None, "{view:?}");
        assert!(rule.skipped(view, None, 1).eq([0]), "{view:?}");
    }
    assert_eq!(
        rule.terms(9),
        terms(Bottom, None, Bottom),
        "列に無いスコープ"
    );
}

/// 規則 ⑵: 隙間は下端・上端の `defaultx` だけ（今の揃え方は読まない）。
#[test]
fn the_gap_is_the_descript_defaultx_of_bottom_and_top() {
    let rows = [
        (Bottom, Some(36), 36),
        (Top, Some(-5), -5),
        (Bottom, None, 0),
        (Free, Some(36), 0),
        (Right, Some(36), 0),
        (Left, Some(36), 0),
    ];
    for (descript, default_x, gap) in rows {
        assert_eq!(
            terms(descript, default_x, Free).gap(),
            gap,
            "{descript:?} {default_x:?}"
        );
    }
}

/// 起動の確定の決めの表の行: (descript, defaultx, 既定の位置, 今の x, 台本の印, 窓の組, 覚えた印, 決め)。
type BootRow = (
    Anchor,
    Option<i32>,
    Option<i32>,
    i32,
    bool,
    bool,
    Option<AdjacencyMark>,
    PairDecision,
);

/// 記憶に位置が無く、既定のまま（既定の位置と今の x が同じ・台本の印も窓の組も無い）。
fn at_default(descript: Anchor, default_x: Option<i32>, want: PairDecision) -> BootRow {
    (
        descript,
        default_x,
        Some(100),
        100,
        false,
        false,
        None,
        want,
    )
}

/// 記憶から戻した（既定の位置が無い・台本の印も窓の組も無い）。
fn remembered(
    descript: Anchor,
    default_x: Option<i32>,
    mark: Option<AdjacencyMark>,
    want: PairDecision,
) -> BootRow {
    (descript, default_x, None, 100, false, false, mark, want)
}

/// 起動の確定の決めの表。
#[test]
fn boot_decision_table() {
    use AdjacencyMark as M;
    use KeepReason as K;
    use PairDecision as P;
    let moved = P::Keep(K::ExplicitlyPlaced);
    let sticky = P::Keep(K::StickyGroup { scope: 1 });
    let rows = [
        // 既定のまま: resolver と同じ決め。
        at_default(Bottom, None, P::JoinLeft),
        at_default(Bottom, Some(36), P::JoinLeftOffset { offset: 36 }),
        at_default(Top, Some(-200), P::JoinLeftOffset { offset: -200 }),
        at_default(Free, None, P::JoinLeft),
        at_default(Free, Some(50), P::Keep(K::FreePosition)),
        at_default(Right, Some(36), P::Keep(K::DesktopEdge)),
        // 記憶に位置が無く、動かされた・台本が置いた。
        (Bottom, Some(36), Some(100), 90, false, false, None, moved),
        (Bottom, None, Some(100), 100, true, false, None, moved),
        // 記憶から戻した: 覚えた印。left は descript の隙間で付ける。
        remembered(
            Bottom,
            Some(36),
            Some(M::Left),
            P::JoinLeftOffset { offset: 36 },
        ),
        remembered(Bottom, None, Some(M::Left), P::JoinLeft),
        remembered(Free, Some(50), Some(M::Left), P::JoinLeft),
        remembered(Bottom, Some(36), Some(M::Right), P::JoinRight),
        remembered(
            Bottom,
            Some(36),
            Some(M::Separate),
            P::Keep(K::RememberedSeparate),
        ),
        remembered(Bottom, Some(36), None, P::Keep(K::NoRememberedMark)),
        // 記憶から戻した窓の組・台本の印（台本の印が先）。
        (Bottom, None, None, 100, false, true, Some(M::Left), sticky),
        (Bottom, None, None, 100, true, true, Some(M::Left), moved),
    ];
    for (i, (descript, default_x, default_pos, x, script, sticky, mark, want)) in
        rows.into_iter().enumerate()
    {
        let state = ScopeChainState {
            scope: 1,
            current_x: x,
            width: 300,
            default_x: default_pos,
        };
        let got = boot_chain_decision(
            &state,
            &terms(descript, default_x, descript),
            script,
            sticky,
            mark,
        );
        assert_eq!(got, want, "行 {i}: {descript:?} {default_x:?}");
    }
}

const WA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 4000,
    bottom: 2000,
};

/// resolver が解いた 2 スコープ（本体は右の辺から 1000 空けて置き、kero の負の `defaultx` が
/// 作業領域の端で切られないようにする）。
fn resolved(default_x: i32, widths: [i32; 2]) -> [ChainRect; 2] {
    let shell: BTreeMap<String, String> = [
        ("sakura.defaultx", "1000".to_owned()),
        ("kero.defaultx", default_x.to_string()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    let cfg = build_placement_config(&BTreeMap::new(), &shell);
    let inputs: Vec<ScopeInput> = widths
        .iter()
        .enumerate()
        .map(|(scope, &w)| ScopeInput {
            scope,
            char_size: SizePx { w, h: 600 },
            balloon_size: SizePx { w: 200, h: 100 },
        })
        .collect();
    let out = resolve_placement(
        &cfg,
        WA,
        &inputs,
        DPI {
            dpi_x: 96,
            dpi_y: 96,
        },
    );
    [0, 1].map(|i| ChainRect {
        x: out[i].char_pos.x,
        y: out[i].char_pos.y,
        w: out[i].char_size.w,
        h: out[i].char_size.h,
    })
}

/// 規則 ⑵⑶ が道をまたいで合う: resolver が置いた組は、保存が印 `left` を書き、幅が変わった
/// 後の起動の確定（既定のまま・覚えた印）と詰め直しが、どれも resolver が新しい幅で解く位置へ置く。
/// `defaultx` は 0・±1・±2・正・負・中心を越える負（513・514 の値を含む）。
#[test]
fn every_path_puts_a_default_pair_where_the_resolver_does() {
    for default_x in [0, 1, 2, -1, -2, 36, -36, -500] {
        let t = terms(Bottom, Some(default_x), Bottom);
        let [prev, cur] = resolved(default_x, [434, 278]);
        assert_eq!(
            prev.x - (cur.x + cur.w),
            default_x,
            "前提: resolver は defaultx だけ空ける"
        );
        // 保存: 画面の並びから印 left。
        let adjacency = judge_chain_adjacency(prev, cur, t.gap());
        assert_eq!(
            AdjacencyMark::from_adjacency(adjacency),
            AdjacencyMark::Left,
            "defaultx {default_x}: 保存の印"
        );

        // 幅が変わった後（拡大率 200%・本体は同じ位置で幅だけ変わったとする）。
        let [new_prev, want] = resolved(default_x, [868, 556]);
        let solve = |decision: PairDecision| {
            let slots = [
                ChainSlot {
                    scope: 0,
                    current_x: new_prev.x,
                    width: new_prev.w,
                    decision: PairDecision::Keep(KeepReason::ExplicitlyPlaced),
                },
                ChainSlot {
                    scope: 1,
                    current_x: cur.x,
                    width: want.w,
                    decision,
                },
            ];
            resolve_chain(&slots).first().map_or(cur.x, |m| m.new_x)
        };
        let state = |default_pos| ScopeChainState {
            scope: 1,
            current_x: cur.x,
            width: want.w,
            default_x: default_pos,
        };
        let paths = [
            (
                "起動の確定（既定のまま）",
                boot_chain_decision(&state(Some(cur.x)), &t, false, false, None),
            ),
            (
                "起動の確定（覚えた印 left）",
                boot_chain_decision(&state(None), &t, false, false, Some(AdjacencyMark::Left)),
            ),
            ("詰め直し", adjacency_decision(adjacency, t.gap())),
        ];
        for (path, decision) in paths {
            assert_eq!(
                solve(decision),
                want.x,
                "defaultx {default_x}: {path} が resolver の位置へ置く（決め {decision:?}）"
            );
        }
    }
}

/// 詰め直しの辺のスコープの相手: 隙間 0 で左右に付いていた一番近い辺のスコープ。
#[test]
fn edge_partner_is_the_nearest_side_edge_scope_attached_with_no_gap() {
    use super::super::chain_finalize::Adjacency;
    let cur = ChainRect {
        x: 1000,
        y: 500,
        w: 300,
        h: 400,
    };
    let at = |x: i32, y: i32| ChainRect {
        x,
        y,
        w: 200,
        h: 400,
    };
    // (辺のスコープの矩形の列（近い順）, 相手)
    let right_of_cur = at(1300, 500);
    let left_of_cur = at(800, 500);
    type Row = (Vec<(usize, ChainRect)>, Option<(usize, Adjacency)>);
    let rows: [Row; 6] = [
        // cur は辺のスコープの左に付いている。
        (
            vec![(1, right_of_cur)],
            Some((1, Adjacency::Left { gap: 0 })),
        ),
        // cur は辺のスコープの右に付いている。
        (
            vec![(1, left_of_cur)],
            Some((1, Adjacency::Right { gap: 0 })),
        ),
        // 離れている・重なっている・縦に外れている辺のスコープは相手にしない。
        (vec![(1, at(1400, 500))], None),
        (vec![(1, at(1200, 500))], None),
        (vec![(1, at(1300, 1000))], None),
        // 近い順に見て、最初に付いていたもの。
        (
            vec![(3, at(1400, 500)), (2, left_of_cur), (1, right_of_cur)],
            Some((2, Adjacency::Right { gap: 0 })),
        ),
    ];
    for (i, (edges, want)) in rows.into_iter().enumerate() {
        assert_eq!(edge_partner(cur, edges), want, "行 {i}");
    }
}
