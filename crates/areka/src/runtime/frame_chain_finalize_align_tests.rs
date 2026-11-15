//! 起動の並べ直しが、記憶に位置の無いスコープを resolver と同じ揃え方の規則で解き直す檻
//! （audit-placement-core 指摘 485）。
//!
//! 窓は本番と同じく resolver（`resolve_placement`）の出力で生やし、並べ直しの材料
//! （`DefaultPlacementBasis`）も起動の準備と同じ形で置く。並べ直しは resolver の決めを崩さず、
//! 出ている絵の幅が準備と違うときだけ、その幅で resolver の式を当て直す: 下端・上端と x の無い
//! Free は「前の窓の x − 自分の幅 − `defaultx`」、左右の辺のスコープは動かさず横の連なりの基準にも
//! しない、x を書いた Free は動かさない。

use std::collections::BTreeMap;

use super::chain_finalize_persist_test_support::spawn_shown;
use super::test_support::{PerTargetSizes, placement_basis, pos_of};
use super::*;

use areka_placement::chain_finalize::AdjacencyMark;
use areka_placement::config::build_placement_config;
use areka_placement::follow::MonitorSnapshot;
use areka_placement::resolver::{RectPx, ScopeInput, SizePx, resolve_placement};
use wintf::ecs::DPI;

/// 主の画面の作業領域。
const WA: RectPx = RectPx {
    left: 0,
    top: 0,
    right: 2880,
    bottom: 1704,
};
/// 準備の幅（resolver が解いた幅）。
const SPAWN: [(u32, u32); 3] = [(434, 687), (278, 357), (300, 400)];

/// shell の descript の値で resolver が解いた位置に窓を生やし、`shown` の絵が出たとして
/// 置き直しを 1 回回す（並べ直しはまだ回さない）。
fn spawn_resolved(shell: &[(&str, &str)], shown: &PerTargetSizes, scopes: usize) -> World {
    let shell_kv: BTreeMap<String, String> = shell
        .iter()
        .map(|&(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    let cfg = build_placement_config(&BTreeMap::new(), &shell_kv);
    let inputs: Vec<ScopeInput> = SPAWN[..scopes]
        .iter()
        .enumerate()
        .map(|(scope, &(w, h))| ScopeInput {
            scope,
            char_size: SizePx {
                w: w as i32,
                h: h as i32,
            },
            balloon_size: SizePx { w: 223, h: 158 },
        })
        .collect();
    let placements = resolve_placement(
        &cfg,
        WA,
        &inputs,
        DPI {
            dpi_x: 96,
            dpi_y: 96,
        },
    );
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![WA],
    });
    world.insert_resource(placement_basis(shell_kv, scopes));
    spawn_shown(&mut world, &placements, shown);
    world
}

fn sizes(shown: &[(u32, u32)]) -> PerTargetSizes {
    PerTargetSizes::new(shown.iter().enumerate().map(|(s, &wh)| (s, Some(wh))))
}

fn x_of(world: &World, scope: usize) -> i32 {
    let gw = world.resource::<GhostWindows>().clone();
    pos_of(world, gw.char_window(scope).expect("char 窓がある"))
        .expect("位置がある")
        .x
}

/// ⑴ `kero.defaultx,36` の隙間は並べ直しの後も残る（areka-P0-scope-chain-gap 要件 2.8）。
#[test]
fn boot_finalize_keeps_the_defaultx_gap() {
    let shown = sizes(&SPAWN[..2]);
    let mut world = spawn_resolved(&[("kero.defaultx", "36")], &shown, 2);
    let before = x_of(&world, 1);
    assert_eq!(
        before,
        2446 - 278 - 36,
        "前提: resolver は 36 の隙間を空ける"
    );

    finalize_chain_once_with(&shown, &mut world);

    assert_eq!(
        x_of(&world, 1),
        before,
        "幅が変わらなければ resolver の位置のまま（36 の隙間を消さない）"
    );
}

/// ⑵ 左右の辺のスコープは辺に残る（指摘 1・SSP の測り 210）。
#[test]
fn boot_finalize_leaves_side_edge_scopes_on_their_edge() {
    for (word, edge_x) in [("right", WA.right - 278), ("left", WA.left)] {
        let shown = sizes(&SPAWN[..2]);
        let mut world = spawn_resolved(&[("kero.seriko.alignmenttodesktop", word)], &shown, 2);
        assert_eq!(
            x_of(&world, 1),
            edge_x,
            "前提: resolver は {word} の辺に置く"
        );

        finalize_chain_once_with(&shown, &mut world);

        assert_eq!(
            x_of(&world, 1),
            edge_x,
            "{word} の辺のスコープは辺から動かない"
        );
    }
}

/// ⑶ x を書いた Free のスコープは動かない（areka-P0-scope-chain-gap P3）。
#[test]
fn boot_finalize_leaves_a_free_scope_with_x_in_place() {
    let shown = sizes(&SPAWN[..2]);
    let mut world = spawn_resolved(
        &[
            ("kero.seriko.alignmenttodesktop", "free"),
            ("kero.defaultx", "100"),
        ],
        &shown,
        2,
    );
    assert_eq!(
        x_of(&world, 1),
        WA.left + 100,
        "前提: resolver は左上から 100"
    );

    finalize_chain_once_with(&shown, &mut world);

    assert_eq!(x_of(&world, 1), WA.left + 100, "x を書いた Free は動かない");
}

/// ⑷ 辺のスコープの次のスコープは、resolver と同じく辺のスコープの前の窓を基準にする。
/// 本体の絵の幅が準備と違うので、scope 2 はその幅で解き直されて動く。
#[test]
fn boot_finalize_skips_side_edge_scopes_as_the_chain_base() {
    let shown = sizes(&[(500, 687), SPAWN[1], SPAWN[2]]);
    let mut world = spawn_resolved(&[("kero.seriko.alignmenttodesktop", "right")], &shown, 3);
    let x2_before = x_of(&world, 2);

    finalize_chain_once_with(&shown, &mut world);

    let x0 = x_of(&world, 0);
    assert_eq!(x_of(&world, 1), WA.right - 278, "辺のスコープは辺のまま");
    assert_eq!(
        x_of(&world, 2),
        x0 - 300,
        "scope 2 の基準は本体（辺のスコープを飛ばす）"
    );
    assert_ne!(x_of(&world, 2), x2_before, "前提: 幅が変わったので動いた");
}

/// ⑷ の例外: 辺のスコープの次が記憶から戻したスコープなら、覚えた印はその辺のスコープとの
/// 隣り合いなので、辺のスコープを鎖に残し、次のスコープは辺のスコープの隣へ付く。
#[test]
fn boot_finalize_keeps_a_side_edge_scope_as_the_base_of_a_remembered_next_scope() {
    let shown = sizes(&SPAWN);
    let mut world = spawn_resolved(&[("kero.seriko.alignmenttodesktop", "right")], &shown, 3);
    {
        let mut gw = world.resource_mut::<GhostWindows>();
        assert!(gw.clear_default_char_pos(2));
        gw.set_remembered_adjacency(2, Some(AdjacencyMark::Left));
    }

    finalize_chain_once_with(&shown, &mut world);

    let x1 = x_of(&world, 1);
    assert_eq!(x1, WA.right - 278, "辺のスコープは辺のまま");
    assert_eq!(
        x_of(&world, 2),
        x1 - 300,
        "記憶の印 left は辺のスコープとの隣り合いなので、その隣へ付く"
    );
}

/// ⑸ 幅が本当に変わったときは、その幅で resolver の式（`defaultx` 込み）を当て直す。
#[test]
fn boot_finalize_corrects_a_real_width_change_with_the_gap() {
    let shown = sizes(&[(500, 687), (300, 357)]);
    let mut world = spawn_resolved(&[("kero.defaultx", "36")], &shown, 2);
    let x1_before = x_of(&world, 1);

    finalize_chain_once_with(&shown, &mut world);

    let x0 = x_of(&world, 0);
    assert_eq!(
        x_of(&world, 1),
        x0 - 300 - 36,
        "今の幅で x0 − w1 − defaultx"
    );
    assert_ne!(x_of(&world, 1), x1_before, "前提: 幅が変わったので動いた");
}
