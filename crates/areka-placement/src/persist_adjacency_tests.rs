//! 位置を書く束への隣り合いの印の書き足しと、記憶からの印の読みの決定論テスト
//! （dpi-realign-remembered-chain 要件 4.6・6.2・6.4・8.6・design AdjacencyPersist）。
//!
//! 窓の一式は本番と同じ `spawn_ghost_windows` で組み、各キャラ窓の `WindowPos` を書き換えて
//! 画面の並びを作る。束に足す中身は [`with_adjacency_marks`]（[`persist_entries`] が送る前に
//! 通す所）で直接確かめる（記憶の表は同じ鍵を 1 つに潰すので、重複のまとめは束で見る）。

use std::collections::BTreeSet;

use areka_sylphya::{Axis, PersistKey};
use bevy_ecs::prelude::*;
use wintf::ecs::WindowPos;

use super::{
    adjacency_entry, balloon_offset_entries, char_pos_entries, restored_scope_marks,
    saved_adjacency, with_adjacency_marks,
};
use crate::chain_finalize::AdjacencyMark;
use crate::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::shared_test_support::placement_sized;
use crate::source::GhostTitles;
use crate::spawn::{GhostWindows, spawn_ghost_windows};
use crate::sticky_group_ledger::StickyGroupLedger;
use log_capture_kit::capture;

const SAVE_TARGET: &str = "areka::persist::save";
const H: i32 = 600;
const Y: i32 = 400;

fn placement(scope: usize, x: i32, w: i32) -> ScopePlacement {
    ScopePlacement {
        anchor: Anchor::Bottom,
        ..placement_sized(
            scope,
            PointPx { x, y: Y },
            PointPx { x: -200, y: 0 },
            false,
            SizePx { w, h: H },
            SizePx { w: 223, h: 158 },
        )
    }
}

/// `(scope, 左上 x, 幅)` の並びで窓の一式を組む（y と高さは共通）。
fn world_with(layout: &[(usize, i32, i32)]) -> World {
    let mut world = World::new();
    let placements: Vec<ScopePlacement> = layout
        .iter()
        .map(|&(scope, x, w)| placement(scope, x, w))
        .collect();
    spawn_ghost_windows(
        &mut world,
        &placements,
        &GhostTitles::from_scope_titles(layout.iter().map(|&(s, _, _)| (s, format!("s{s}")))),
    );
    world
}

fn char_window(world: &World, scope: usize) -> Entity {
    world
        .resource::<GhostWindows>()
        .char_window(scope)
        .expect("char 窓がある")
}

fn marks(entries: &[(PersistKey, String)]) -> Vec<(u32, String)> {
    entries
        .iter()
        .filter_map(|(k, v)| match k {
            PersistKey::WindowAdjacent { scope } => Some((*scope, v.clone())),
            _ => None,
        })
        .collect()
}

fn pos_batch(scope: u32) -> Vec<(PersistKey, String)> {
    char_pos_entries(scope, PointPx { x: 1234, y: 567 })
}

/// スコープ 1 の位置を書くと、組 (0,1) と (1,2) の印が足され、位置の値は送ったまま先頭に残る。
/// 左に付いた組は `left`、右に付いた組は `right`。スコープ 0 の印は作らない（前の窓が無い）。
#[test]
fn position_batch_adds_left_and_right_marks_of_touched_pairs() {
    // 0: 1000..1400、1: 600..1000（0 の左に隙間 0）、2: 1000..1300（1 の右に隙間 0）。
    let world = world_with(&[(0, 1000, 400), (1, 600, 400), (2, 1000, 300)]);
    let sent = pos_batch(1);
    let out = with_adjacency_marks(&world, sent.clone());
    assert_eq!(&out[..sent.len()], &sent[..], "位置の鍵と値は変えない");
    assert_eq!(
        marks(&out),
        vec![(1, "left".to_string()), (2, "right".to_string())]
    );
}

/// 隙間・重なりが許す幅（2 px）に収まれば隣り合い、越えれば `none`（要件 6.4）。
#[test]
fn apart_and_overlapped_pairs_are_marked_none_with_positive_pair() {
    for (x1, expected) in [
        (602, "right"),
        (598, "right"),
        (650, "none"), // 離れ（隙間 50）
        (550, "none"), // 重なり（−50）
    ] {
        // 0: 200..600、1: x1..x1+300。
        let world = world_with(&[(0, 200, 400), (1, x1, 300)]);
        let out = with_adjacency_marks(&world, pos_batch(0));
        assert_eq!(marks(&out), vec![(1, expected.to_string())], "x1={x1}");
    }
}

/// 組（StickyGroupLedger）に入った窓は、隣り合っていても `none`（要件 6.4）。組の外は判定どおり。
#[test]
fn sticky_group_member_is_marked_none() {
    let mut world = world_with(&[(0, 1000, 400), (1, 600, 400), (2, 1000, 300)]);
    let mut ledger = StickyGroupLedger::default();
    ledger
        .try_add_script_group(BTreeSet::from([1]))
        .expect("組を作れる");
    world.insert_resource(ledger);
    let out = with_adjacency_marks(&world, pos_batch(1));
    assert_eq!(
        marks(&out),
        vec![(1, "none".to_string()), (2, "right".to_string())]
    );
}

/// バルーンのオフセットだけの束（とほかの鍵）には印を足さない。
#[test]
fn balloon_offset_only_batch_gets_no_marks() {
    let world = world_with(&[(0, 1000, 400), (1, 600, 400)]);
    let mut sent = balloon_offset_entries(1, PointPx { x: -300, y: 7 });
    sent.push((PersistKey::BootCount, "5".to_string()));
    let out = with_adjacency_marks(&world, sent.clone());
    assert_eq!(out, sent);
}

/// 矩形の読めない窓に触れる組は印を書かず `warn!` を出し、位置とほかの組の印は書く。
#[test]
fn unreadable_rect_skips_that_pair_with_warn_and_keeps_position() {
    let mut world = world_with(&[(0, 1000, 400), (1, 600, 400), (2, 1000, 300)]);
    let w2 = char_window(&world, 2);
    world.get_mut::<WindowPos>(w2).unwrap().size = None;
    let sent = pos_batch(1);
    let (out, events) = capture(|| with_adjacency_marks(&world, sent.clone()));
    assert_eq!(&out[..sent.len()], &sent[..]);
    assert_eq!(marks(&out), vec![(1, "left".to_string())]);
    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.target == SAVE_TARGET && e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(warns.len(), 1, "{events:?}");
    assert_eq!(warns[0].field("scope"), Some("2"));
}

/// 束に 2 つのスコープの位置があり同じ組に触れても、印の鍵は 1 つずつ。
#[test]
fn duplicate_mark_keys_are_merged_into_one() {
    let world = world_with(&[(0, 1000, 400), (1, 600, 400), (2, 1000, 300)]);
    let mut sent = pos_batch(0);
    sent.extend(pos_batch(1));
    sent.extend(pos_batch(2));
    let out = with_adjacency_marks(&world, sent.clone());
    assert_eq!(&out[..sent.len()], &sent[..]);
    assert_eq!(
        marks(&out),
        vec![(1, "left".to_string()), (2, "right".to_string())]
    );
    // 束にもう印があれば、それを残して足さない。
    let mut with_mark = pos_batch(1);
    with_mark.push(adjacency_entry(1, AdjacencyMark::Left));
    let out = with_adjacency_marks(&world, with_mark);
    assert_eq!(
        marks(&out),
        vec![(1, "left".to_string()), (2, "right".to_string())]
    );
}

/// 組の相手が窓の一式に無い（1 体だけのゴースト・最後のスコープの右隣）ときはログなしで飛ばす。
/// 並べ直しの材料は置く（材料が無いときの `debug!` は別のテストが見る）。
#[test]
fn missing_partner_scope_is_skipped_silently() {
    let mut single = world_with(&[(0, 1000, 400)]);
    with_basis(&mut single, &[]);
    let (out, events) = capture(|| with_adjacency_marks(&single, pos_batch(0)));
    assert!(marks(&out).is_empty());
    assert!(events.iter().all(|e| e.target != SAVE_TARGET), "{events:?}");

    let mut pair = world_with(&[(0, 1000, 400), (1, 600, 400)]);
    with_basis(&mut pair, &[]);
    let (out, events) = capture(|| with_adjacency_marks(&pair, pos_batch(1)));
    assert_eq!(marks(&out), vec![(1, "left".to_string())]);
    assert!(events.iter().all(|e| e.target != SAVE_TARGET), "{events:?}");
}

/// 窓の一式（`GhostWindows`）がまだ無いときは印を足さず、飛ばしたことを `debug!` に残す。
#[test]
fn missing_ghost_windows_adds_no_mark_and_logs_debug() {
    let world = World::new();
    let (out, events) = capture(|| with_adjacency_marks(&world, pos_batch(1)));
    assert_eq!(out, pos_batch(1));
    assert!(
        events
            .iter()
            .any(|e| e.target == SAVE_TARGET && e.level == tracing::Level::DEBUG),
        "{events:?}"
    );
}

/// 並べ直しの材料（`DefaultPlacementBasis`）が無いときは、全スコープを下端・`defaultx` 0 として
/// 印を決め、その決めを `debug!` 1 行に残す（黙って既定へ落とさない・audit-placement-core 507）。
#[test]
fn missing_basis_marks_with_the_bottom_rule_and_logs_debug() {
    // 0: 1000..1400、1: 600..1000（0 の左に隙間 0）。
    let world = world_with(&[(0, 1000, 400), (1, 600, 400)]);
    let (out, events) = capture(|| with_adjacency_marks(&world, pos_batch(0)));
    assert_eq!(marks(&out), vec![(1, "left".to_string())]);
    let lines = events
        .iter()
        .filter(|e| {
            e.target == SAVE_TARGET
                && e.level == tracing::Level::DEBUG
                && e.message().contains("DefaultPlacementBasis")
        })
        .count();
    assert_eq!(lines, 1, "{events:?}");
}

/// 印の項目の鍵と語、読み戻し（知らない語は warn! して無し・鍵なしは無し）。
#[test]
fn adjacency_entry_and_saved_adjacency_round_trip() {
    assert_eq!(
        adjacency_entry(3, AdjacencyMark::Separate),
        (PersistKey::WindowAdjacent { scope: 3 }, "none".to_string())
    );
    for mark in [
        AdjacencyMark::Left,
        AdjacencyMark::Right,
        AdjacencyMark::Separate,
    ] {
        assert_eq!(saved_adjacency(&[adjacency_entry(1, mark)], 1), Some(mark));
    }
    assert_eq!(
        saved_adjacency(&[adjacency_entry(1, AdjacencyMark::Left)], 2),
        None
    );

    let entries = vec![(PersistKey::WindowAdjacent { scope: 1 }, "up".to_string())];
    let (got, events) = capture(|| saved_adjacency(&entries, 1));
    assert_eq!(got, None);
    assert!(
        events
            .iter()
            .any(|e| e.level == tracing::Level::WARN && e.field("scope") == Some("1")),
        "{events:?}"
    );
}

/// 並べ直しの材料（shell の descript の値だけ）を置く（並べ終えた時点の保存のテストも使う）。
pub(super) fn with_basis(world: &mut World, shell_kv: &[(&str, &str)]) {
    world.insert_resource(crate::reset_pos::DefaultPlacementBasis {
        source: crate::source::DescriptSource {
            ghost_kv: Default::default(),
            shell_kv: crate::test_support::kv(shell_kv),
            shell_dir: std::path::PathBuf::from("shell/master"),
            titles: GhostTitles::from_scope_titles((0..3).map(|s| (s, format!("s{s}")))),
        },
        balloon: crate::reseed::BalloonPlacementInputs {
            author_dpi: 96,
            windowpositions: Default::default(),
        },
    });
}

/// 左の組は隙間 0 のほか、そのスコープの `defaultx` の隙間（±2 px）も `left`。右の組は隙間 0
/// だけが `right`（resolver の P2 は左へだけ空ける・audit-placement-core 494）。
#[test]
fn a_defaultx_gap_on_the_left_is_marked_left() {
    for (x1, expected) in [
        (1000 - 300 - 36, "left"),
        (1000 - 300 - 38, "left"),
        (1000 - 300, "left"),
        (1000 - 300 - 60, "none"),
        (1400, "right"),
        (1400 + 36, "none"),
    ] {
        // 0: 1000..1400、1: x1..x1+300。
        let mut world = world_with(&[(0, 1000, 400), (1, x1, 300)]);
        with_basis(&mut world, &[("kero.defaultx", "36")]);
        let out = with_adjacency_marks(&world, pos_batch(0));
        assert_eq!(marks(&out), vec![(1, expected.to_string())], "x1={x1}");
    }
}

/// 鎖の前の窓は、左右の辺に揃えるスコープ（descript の揃え方）を飛ばした一番近いスコープ。
/// 辺のスコープ自身の印は `none`（鎖の外・audit-placement-core 494・495）。
#[test]
fn a_side_edge_scope_is_skipped_as_the_previous_chain_window() {
    // 0: 1000..1400、1: 右の辺 2000..2300、2: 700..1000（本体の左に隙間 0）。
    let mut world = world_with(&[(0, 1000, 400), (1, 2000, 300), (2, 700, 300)]);
    with_basis(&mut world, &[("kero.seriko.alignmenttodesktop", "right")]);
    let out = with_adjacency_marks(&world, pos_batch(0));
    assert_eq!(
        marks(&out),
        vec![(2, "left".to_string())],
        "本体の位置を書くと、本体を前の窓とする scope 2 の印を書く"
    );
    let out = with_adjacency_marks(&world, pos_batch(1));
    assert_eq!(
        marks(&out),
        vec![(1, "none".to_string()), (2, "left".to_string())]
    );
}

/// 一覧は記憶に位置（x・y の両方）があるスコープだけを鍵にし、印が無ければ値は無し。
#[test]
fn restored_scope_marks_keys_only_scopes_with_saved_position() {
    let merged: Vec<ScopePlacement> = (0..4).map(|s| placement(s, 100 * s as i32, 50)).collect();
    let mut entries = char_pos_entries(0, PointPx { x: 1, y: 2 });
    entries.extend(char_pos_entries(1, PointPx { x: 3, y: 4 }));
    entries.push(adjacency_entry(1, AdjacencyMark::Right));
    entries.extend(char_pos_entries(2, PointPx { x: 5, y: 6 }));
    entries.push((PersistKey::WindowAdjacent { scope: 2 }, "??".to_string()));
    // スコープ 3 は印と x だけ（y が無い）＝位置の無いスコープ。
    entries.push((
        PersistKey::WindowPos {
            scope: 3,
            axis: Axis::X,
        },
        "7".to_string(),
    ));
    entries.push(adjacency_entry(3, AdjacencyMark::Left));

    let got = restored_scope_marks(&entries, &merged);
    assert_eq!(
        got.into_iter().collect::<Vec<_>>(),
        vec![(0, None), (1, Some(AdjacencyMark::Right)), (2, None)]
    );
}
