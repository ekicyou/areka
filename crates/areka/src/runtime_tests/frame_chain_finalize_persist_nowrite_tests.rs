//! 書かない時点で記憶が書き換わらないことを固定する。
//!
//! 窓の一式と記憶の送り口は `areka_runtime::frame::persist_test_support`、最小のゴーストは
//! `frame_chain_finalize_persist_test_support.rs` から取る。記憶は `target\` の下の実物の
//! ファイル（`PersistStore::real`）に置き、書き手が送られた分を書き終えるのを待ってから、
//! ファイルの中身をバイトのまま前後で比べる。比べる前に記憶へ中身を入れておく。

use super::chain_finalize_persist_test_support::MinimalGhost;
use super::test_support::{PerTargetSizes, pos_of, s2_monitors_with_work_area, size_of};
use super::*;
use areka_runtime::frame::persist_test_support::{
    PREP_SIZES, PersistStore, SHOWN_SIZES, WORK_AREA, boot_remembered, persist_world,
    prep_placements, shown_sizes, spawn_shown, work_area_snapshot,
};

use areka_sylphya::{Axis, PersistKey};
use wintf::ecs::window::drain_window_pos_commands;

use crate::session::app_exit::{ExitOrigin, close_windows_for_restart, quit_app};
use areka_placement::chain_finalize::AdjacencyMark;
use areka_placement::chain_finalize::ChainFinalized;
use areka_placement::follow::move_window_to;
use areka_placement::persist::{adjacency_entry, char_pos_entries, persist_entries};
use areka_placement::resolver::{PointPx, RectPx};

/// 記憶にスコープ 0・1 の位置（x・y）がそろっていること（比べる前の中身の確かめ）。
fn assert_positions_saved(store: &PersistStore) {
    let saved = store.load();
    for scope in [0, 1] {
        for axis in [Axis::X, Axis::Y] {
            assert!(
                saved.contains_key(&PersistKey::WindowPos { scope, axis }),
                "前提: スコープ {scope} の {axis:?} が記憶にある: {saved:?}"
            );
        }
    }
}

/// 起動の準備の本番の関数で記憶を読み直し、[`boot_remembered`] で窓を立てる。並べ直しは回さない。
/// 返すのは窓の一式と、起動の準備の関数が返した配置。
fn boot_from_memory(
    world: &mut World,
    ghost: &MinimalGhost,
) -> (GhostWindows, Vec<areka_placement::resolver::ScopePlacement>) {
    let (placements, restored, _) =
        crate::restore_merged_placements(ghost.root(), prep_placements(), &work_area_snapshot());
    assert_eq!(
        restored.keys().copied().collect::<Vec<_>>(),
        vec![0, 1],
        "前提: 両方のスコープの位置が記憶にある"
    );
    let gw = boot_remembered(world, &placements, &restored, &shown_sizes());
    (gw, placements)
}

/// 並べ終えて記憶に中身がある状態から、絵の大きさを変えて置き直す → 移動の指示と同じ口で
/// 動かす → 作業領域を変えて寄せ直す → ゴーストの切り替えの閉じ方で閉じる → 終了の手順、を
/// 順に行い、どの段の後も記憶はバイトのまま前後で同じ。
///
/// 段ごとに、その段が窓を実際に動かしたことを先に確かめる。記憶にはバルーンの相対位置の
/// 目印も入れておく（書く時機はバルーンのドラッグの確定だけ）。
///
/// テストの World には終了の受け口 `AppExit` が無いので、`quit_app` は窓を閉じたあと
/// `error!` を残して戻る。窓を閉じる段に窓が残るよう、切り替えの閉じ方の後は記憶から窓を
/// 作り直し、並べ終えてから終える。
#[test]
fn memory_is_unchanged_by_resize_move_work_area_close_and_quit() {
    let ghost = MinimalGhost::plant("nowrite");
    let store = PersistStore::real(&ghost.ghost_master());
    let (placements, restored, _) =
        crate::restore_merged_placements(ghost.root(), prep_placements(), &work_area_snapshot());
    assert!(restored.is_empty(), "前提: 初めの起動は記憶が無い");
    let mut world = persist_world();
    store.wire(&mut world);
    let shown = shown_sizes();
    let gw = spawn_shown(&mut world, &placements, &shown);
    finalize_chain_once_with(&shown, &mut world);
    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提: 並べ終える"
    );
    let offset_key = PersistKey::BalloonOffset {
        scope: 0,
        axis: Axis::X,
    };
    persist_entries(&world, vec![(offset_key, "12".to_string())]);
    assert_positions_saved(&store);
    let saved = store.file_text();
    let char0 = gw.char_window(0).unwrap();
    let char1 = gw.char_window(1).unwrap();

    // (a) 絵の大きさが変わって置き直す（高さも変える＝書けば上端の y が変わる）。
    let grown = (SHOWN_SIZES[0].0 + 14, SHOWN_SIZES[0].1 + 40);
    let resized = PerTargetSizes::new([(0, Some(grown)), (1, Some(SHOWN_SIZES[1]))]);
    resnap_with(&resized, &mut world);
    assert_eq!(
        size_of(&world, char0),
        Some(SizeI::new(grown.0 as i32, grown.1 as i32)),
        "前提: 置き直しが窓へ届く"
    );
    assert_eq!(store.file_text(), saved, "置き直しは記憶を変えない");

    let start = pos_of(&world, char1).expect("相方の位置");
    assert!(
        move_window_to(&mut world, char1, start.x - 77, start.y - 33),
        "前提: 移動の指示が窓へ届く"
    );
    assert_ne!(pos_of(&world, char1), Some(start), "前提: 相方が動く");
    assert_eq!(store.file_text(), saved, "移動の指示は記憶を変えない");

    // (c) 作業領域が変わって寄せ直す（同期 → 寄せ直しの順）。
    let before = pos_of(&world, char0).expect("本体の位置");
    let raised = RectPx {
        bottom: WORK_AREA.bottom - 100,
        ..WORK_AREA
    };
    let change = work_area_sync::test_support::sync_monitor_snapshot_with(
        &mut world,
        &s2_monitors_with_work_area(96, raised),
    )
    .expect("前提: 作業領域の表が差し替わる");
    work_area_sync::resnap_for_work_area_change(&mut world, &change);
    assert_eq!(
        pos_of(&world, char0).map(|p| p.y),
        Some(before.y - 100),
        "前提: 寄せ直しで本体が新しい下端へ移る"
    );
    assert_eq!(
        store.file_text(),
        saved,
        "作業領域の寄せ直しは記憶を変えない"
    );

    let closed = close_windows_for_restart(&mut world);
    assert!(closed.closed() > 0, "前提: 窓が閉じる");
    assert_eq!(store.file_text(), saved, "切り替えの閉じ方は記憶を変えない");

    // 記憶から窓を作り直して並べ終える（作業領域は元に戻す）。
    let _ = work_area_sync::test_support::sync_monitor_snapshot_with(
        &mut world,
        &s2_monitors_with_work_area(96, WORK_AREA),
    )
    .expect("前提: 作業領域の表が元へ差し替わる");
    let (gw2, _) = boot_from_memory(&mut world, &ghost);
    finalize_chain_once_with(&shown, &mut world);
    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提: 作り直した窓を並べ終える"
    );
    assert_eq!(
        store.file_text(),
        saved,
        "記憶から作り直した窓は並べ終えた時点でも書かない"
    );

    // (e) 作り直した窓は記憶の位置に立つので、先に相方を記憶の位置から動かしておく。
    let partner = gw2.char_window(1).unwrap();
    let start = pos_of(&world, partner).expect("相方の位置");
    assert!(
        move_window_to(&mut world, partner, start.x - 50, start.y - 20),
        "前提: 移動の指示が窓へ届く"
    );
    assert_ne!(pos_of(&world, partner), Some(start), "前提: 相方が動く");
    assert_eq!(store.file_text(), saved, "移動の指示は記憶を変えない");
    let closed = quit_app(&mut world, ExitOrigin::Smoke);
    assert!(closed > 0, "前提: 終了の手順が窓を閉じる");
    assert_eq!(store.file_text(), saved, "終了の手順は記憶を変えない");

    store.finish();
    let _residue = drain_window_pos_commands();
}

/// 記憶の位置が画面の外にあるスコープを画面の中へ寄せ直して表示し（表示している位置は記憶の
/// 値と違う）、並べ直しを回しても、記憶はバイトのまま前後で同じ（寄せ直した位置を書き戻さない）。
#[test]
fn offscreen_saved_position_survives_finalize_while_shown_pulled_in() {
    let ghost = MinimalGhost::plant("nowrite-offscreen");
    let store = PersistStore::real(&ghost.ghost_master());
    // 本体（スコープ 0）は作業領域の右の外、相方（スコープ 1）は中。値は下端の中央の x・上端の y。
    let offscreen = PointPx {
        x: WORK_AREA.right + 1500,
        y: WORK_AREA.bottom - PREP_SIZES[0].h,
    };
    let inside = PointPx {
        x: 1150,
        y: WORK_AREA.bottom - PREP_SIZES[1].h,
    };
    {
        let mut seed = persist_world();
        store.wire(&mut seed);
        let mut entries = char_pos_entries(0, offscreen);
        entries.extend(char_pos_entries(1, inside));
        // 印の無い古い記憶は印だけ書き足されるので、印のある記憶にしておく。
        entries.push(adjacency_entry(1, AdjacencyMark::Separate));
        persist_entries(&seed, entries);
    }
    assert_positions_saved(&store);
    let saved = store.file_text();

    let mut world = persist_world();
    store.wire(&mut world);
    let (gw, placements) = boot_from_memory(&mut world, &ghost);
    let pulled = placements[0].char_pos;
    assert!(
        pulled.x + PREP_SIZES[0].w <= WORK_AREA.right,
        "前提: 記憶を重ねる関数が本体を画面の中へ寄せる: {pulled:?}"
    );
    let body = pos_of(&world, gw.char_window(0).unwrap()).expect("本体の位置");
    assert_ne!(
        body.x + SHOWN_SIZES[0].0 as i32 / 2,
        offscreen.x,
        "前提: 表示している位置は記憶の値と違う"
    );

    finalize_chain_once_with(&shown_sizes(), &mut world);

    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提: 並べ終える"
    );
    assert_eq!(
        store.file_text(),
        saved,
        "寄せ直して表示している記憶は並べ終えた時点でも書き換わらない"
    );
    store.finish();
    let _residue = drain_window_pos_commands();
}
