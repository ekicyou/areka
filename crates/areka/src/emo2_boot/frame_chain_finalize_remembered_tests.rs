//! 起動・作り直しの並べ直しが、記憶から戻した組を覚えた印で今の幅で隣り合わせる檻
//! （`dpi-realign-remembered-chain` 要件 5.1・5.2・6.1・6.3・6.4・6.6・7.1・8.6・design BootRemembered）。
//!
//! # ここが押さえるもの
//!
//! 200% で隣り合って覚えた組を 100% で起動すると、記憶の位置（下端の中央）のまま今の幅で立つ
//! ので隙間 359 が開く。並べ直しは覚えた印 `left` を見てこの組を今の幅で隣り合わせ、印 `none`・
//! 印の無い古い記憶・覚えたときと同じ幅では誰も動かさない。隣り合わせた位置は記憶に書かない。
//!
//! 窓を作る閉包（`ghost_session.rs`）はテストから呼べないので、同じ順（窓を作る → 記憶から
//! 戻したスコープの既定の位置を外して覚えた印を据える → 絵が出て置き直す → 並べ直す）を
//! 同じ口で踏む。

use std::collections::BTreeMap;

use areka_sylphya::PersistKey;
use bevy_ecs::prelude::World;
use wintf::ecs::window::drain_window_pos_commands;

use super::chain_finalize_persist_test_support::{
    PersistStore, WORK_AREA, persist_world, spawn_shown,
};
use super::chain_realign_adjacency_tests::{
    GAP_WITHOUT_REALIGN, boot_remembered_at_high, gap, low_sizes, lower_the_scale,
};
use super::test_support::{FakeReports, FrameHarness, PerTargetSizes, capture_logs, pos_of};
use super::{finalize_chain_once_with, resnap_with};
use crate::app_exit::close_windows_for_restart;
use crate::placement::chain_finalize::{AdjacencyMark, ChainFinalized};
use crate::placement::follow::OffsetBase;
use crate::placement::persist::{char_pos_entries, persist_entries};
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::spawn::GhostWindows;

/// 覚えたとき（200%）の絵の大きさ（本体・相方）。
const HIGH: [(u32, u32); 2] = [(764, 1094), (672, 596)];
/// 今（100%）の絵の大きさ。
const LOW: [(u32, u32); 2] = [(382, 547), (336, 298)];
/// 本体の覚えた左上の x（200%）。
const BODY_X: i32 = 1600;
/// 相方の覚えた左上の x（200% で本体の左に隣り合う＝本体の x − 相方の幅）。
const PARTNER_X: i32 = BODY_X - HIGH[1].0 as i32;
/// 組の判断の行の本文（`finalize_chain_once_with` の info）。
const PAIR_LINE: &str = "記憶から戻した組の判断";

fn sizes(s: [(u32, u32); 2]) -> PerTargetSizes {
    PerTargetSizes::new([(0, Some(s[0])), (1, Some(s[1]))])
}

/// 200% で覚えた並び（下端に接地・相方は本体の左に隣り合う）を、200% の大きさの配置にする。
/// 置き直しで 100% の絵を出すと、下端の中央を保って今の幅で立つ（記憶から戻した形と同じ）。
fn remembered_placements() -> Vec<ScopePlacement> {
    [(0usize, BODY_X), (1, PARTNER_X)]
        .into_iter()
        .map(|(scope, x)| {
            let char_size = SizePx {
                w: HIGH[scope].0 as i32,
                h: HIGH[scope].1 as i32,
            };
            let char_pos = PointPx {
                x,
                y: WORK_AREA.bottom - char_size.h,
            };
            let balloon_offset = PointPx { x: -230, y: 0 };
            ScopePlacement {
                scope,
                char_pos,
                char_size,
                balloon_pos: PointPx {
                    x: char_pos.x + balloon_offset.x,
                    y: char_pos.y,
                },
                balloon_size: SizePx { w: 223, h: 158 },
                balloon_offset,
                balloon_offset_base: OffsetBase::unpinned(balloon_offset),
                balloon_limit: false,
                anchor: Anchor::Bottom,
                balloon_keyword_base: None,
            }
        })
        .collect()
}

/// 窓の一式を作り、全スコープを記憶から戻した形にして（既定の位置を外し、相方に `mark` を据える）、
/// `shown` の絵で置き直してから並べ直しを 1 回回す。返すのは並べ直しで出た窓への指令の数と、
/// 並べ直しの間のログ。
fn boot(
    world: &mut World,
    mark: Option<AdjacencyMark>,
    shown: [(u32, u32); 2],
) -> (usize, Vec<String>) {
    let not_shown = PerTargetSizes::new([(0, None), (1, None)]);
    let _gw = spawn_shown(world, &remembered_placements(), &not_shown);
    let restored: BTreeMap<usize, Option<AdjacencyMark>> = [(0, None), (1, mark)].into();
    {
        let mut gw = world.resource_mut::<GhostWindows>();
        for (&scope, &m) in &restored {
            assert!(gw.clear_default_char_pos(scope));
            gw.set_remembered_adjacency(scope, m);
        }
    }
    resnap_with(&sizes(shown), world);
    let _resnap = drain_window_pos_commands();
    let logs = capture_logs(|| finalize_chain_once_with(&sizes(shown), world));
    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提が崩れている: 並べ直しが駆動していない"
    );
    (drain_window_pos_commands().len(), logs)
}

/// 本体の左端 − 相方の右端。
fn pair_gap(world: &World, width_1: u32) -> i32 {
    let gw = world.resource::<GhostWindows>();
    let x = |scope| pos_of(world, gw.char_window(scope).unwrap()).unwrap().x;
    x(0) - (x(1) + width_1 as i32)
}

fn window_pos_keys(saved: &std::collections::HashMap<PersistKey, String>) -> Vec<String> {
    saved
        .keys()
        .filter(|k| matches!(k, PersistKey::WindowPos { .. }))
        .map(|k| format!("{k:?}"))
        .collect()
}

/// ⑴ 実行中に 200% → 100% で詰めた後、記憶へ送られた束に位置が無い（6.1）。
#[test]
fn realign_after_200_to_100_writes_no_position() {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_remembered_at_high(&mut harness, &mut source);
    let store = PersistStore::fake();
    store.wire(&mut harness.world);

    lower_the_scale(&mut harness, &mut source);
    assert_eq!(gap(&harness), GAP_WITHOUT_REALIGN, "前提: 隙間が開いている");
    harness.run_chain_realign(&low_sizes());
    assert_eq!(gap(&harness), 0, "前提: 詰め直しで隙間が閉じる");

    assert_eq!(
        window_pos_keys(&store.load()),
        Vec::<String>::new(),
        "詰め直した位置は記憶に書かない"
    );
    // 較正: 同じ送り口から位置を送れば読み戻しに現れる（上の 0 件が見えない 0 件でない）。
    persist_entries(&harness.world, char_pos_entries(0, PointPx { x: 1, y: 2 }));
    assert_eq!(
        window_pos_keys(&store.load()).len(),
        2,
        "較正: x・y が読み戻せる"
    );
}

/// ⑵ 200% で `left` と覚えた組を 100% の幅で起動すると隣り合って立ち、位置は記憶に書かない。
/// 窓の一式を閉じて作り直しても同じ（読み直し・切り替えは同じ閉包を通る・5.1・5.2・6.3・6.6・7.1）。
#[test]
fn remembered_left_pair_stands_adjacent_at_current_widths() {
    let mut world = persist_world();
    let store = PersistStore::fake();
    store.wire(&mut world);

    let (moved, logs) = boot(&mut world, Some(AdjacencyMark::Left), LOW);
    assert_eq!(
        (moved > 0, pair_gap(&world, LOW[1].0)),
        (true, 0),
        "(並べ直しで窓へ指令が出た, 隙間)"
    );
    let first = pos_of(
        &world,
        world.resource::<GhostWindows>().char_window(1).unwrap(),
    );
    let lines: Vec<&String> = logs.iter().filter(|l| l.contains(PAIR_LINE)).collect();
    assert_eq!(lines.len(), 1, "組ごとに 1 行: {logs:?}");
    for field in [
        "decision=\"adjacent-left\"",
        &format!("dx={GAP_WITHOUT_REALIGN}"),
        "scope=1",
    ] {
        assert!(lines[0].contains(field), "{field} が無い: {}", lines[0]);
    }
    assert_eq!(
        window_pos_keys(&store.load()),
        Vec::<String>::new(),
        "隣り合わせた位置は記憶に書かない（6.6）"
    );

    let _ = close_windows_for_restart(&mut world);
    let (moved, _) = boot(&mut world, Some(AdjacencyMark::Left), LOW);
    assert_eq!(
        (
            moved > 0,
            pos_of(
                &world,
                world.resource::<GhostWindows>().char_window(1).unwrap()
            )
        ),
        (true, first),
        "(作り直しの並べ直しで窓へ指令が出た, 相方の位置)"
    );
}

/// ⑶ 印 `none`（覚えたときに離れていた・重なっていた）の組は詰めない（6.4）。
#[test]
fn remembered_separate_pair_is_not_tightened() {
    let mut world = persist_world();
    let (moved, logs) = boot(&mut world, Some(AdjacencyMark::Separate), LOW);
    assert_eq!(
        (moved, pair_gap(&world, LOW[1].0)),
        (0, GAP_WITHOUT_REALIGN),
        "(並べ直しの指令の数, 隙間)"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains(PAIR_LINE) && l.contains("decision=\"remembered-separate\"")),
        "動かさない判断も組ごとに残る: {logs:?}"
    );
}

/// ⑷ 覚えたときと今の幅が同じなら、印 `left` でも誰も動かない（6.4 の後段）。
#[test]
fn same_widths_as_remembered_move_nobody() {
    let mut world = persist_world();
    let (moved, _) = boot(&mut world, Some(AdjacencyMark::Left), HIGH);
    assert_eq!(
        (moved, pair_gap(&world, HIGH[1].0)),
        (0, 0),
        "(並べ直しの指令の数, 隙間)"
    );
}

/// ⑸ 印の無い古い記憶は位置のまま立つ（6.5 の前段）。
#[test]
fn old_memory_without_mark_stands_at_its_position() {
    let mut world = persist_world();
    let (moved, logs) = boot(&mut world, None, LOW);
    assert_eq!(
        (moved, pair_gap(&world, LOW[1].0)),
        (0, GAP_WITHOUT_REALIGN),
        "(並べ直しの指令の数, 隙間)"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains(PAIR_LINE) && l.contains("decision=\"no-remembered-mark\"")),
        "印が無いことも組ごとに残る: {logs:?}"
    );
}
