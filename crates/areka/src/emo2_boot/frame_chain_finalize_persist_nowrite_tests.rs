//! 書かない時点で記憶が書き換わらないことを固定する（areka-P0-char-position-save-on-exit
//! タスク 4.2・要件 1.2・3.2・3.3・5.3・design Testing Strategy「並べ直しから記憶まで」15）。
//!
//! 窓の一式・記憶の送り口・最小のゴーストは共有の部品
//! `frame_chain_finalize_persist_test_support.rs` から取る。
//!
//! 記憶は `target\` の下の実物のファイル（`PersistStore::real`）に置き、書き手が送られた分を
//! 書き終えるのを待ってから、ファイルの中身をバイトのまま前後で比べる。比べる前に記憶へ
//! 中身を入れておく（空どうしの比較にしない）。

use super::chain_finalize_persist_test_support::{
    MinimalGhost, PREP_SIZES, PersistStore, SHOWN_SIZES, WORK_AREA, persist_world, prep_placements,
    shown_sizes, spawn_shown, work_area_snapshot,
};
use super::test_support::{PerTargetSizes, pos_of, s2_monitors_with_work_area, size_of};
use super::*;

use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_parsers::charset::DefaultEncoding;
use areka_sylphya::{Axis, PersistKey};
use wintf::ecs::window::drain_window_pos_commands;

use crate::app_exit::close_windows_for_restart;
use crate::placement::chain_finalize::ChainFinalized;
use crate::placement::follow::move_window_to;
use crate::placement::persist::{char_pos_entries, persist_entries};
use crate::placement::resolver::{PointPx, RectPx};

/// 書き手が送られた分を書き終えるのを待ってから、記憶のファイルの中身をそのまま読む。
fn memory(store: &PersistStore, ghost: &MinimalGhost) -> String {
    let _ = store.load();
    let path = profile_areka_root(&ghost.root().join("ghost").join("master")).join("sylphya.toml");
    std::fs::read_to_string(&path).expect("記憶のファイルがある")
}

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

/// 起動の準備の本番の関数で記憶を読み直し、本番（`ghost_session.rs` の窓を作る閉包）と同じ順で、
/// 窓を作る → 返った集合のスコープの既定の位置を消す → 絵が出て置き直す。並べ直しは回さない。
/// 返すのは窓の一式と、起動の準備の関数が返した配置。
fn boot_remembered(
    world: &mut World,
    ghost: &MinimalGhost,
) -> (
    GhostWindows,
    Vec<crate::placement::resolver::ScopePlacement>,
) {
    let (placements, restored) = crate::restore_merged_placements(
        ghost.root(),
        prep_placements(),
        &work_area_snapshot(),
        DefaultEncoding::Ansi,
    );
    assert_eq!(
        restored.iter().copied().collect::<Vec<_>>(),
        vec![0, 1],
        "前提: 両方のスコープの位置が記憶にある"
    );
    let not_shown = PerTargetSizes::new([(0, None), (1, None)]);
    let gw = spawn_shown(world, &placements, &not_shown);
    {
        let mut ghost_windows = world.resource_mut::<GhostWindows>();
        for scope in &restored {
            assert!(ghost_windows.clear_default_char_pos(*scope));
        }
    }
    resnap_with(&shown_sizes(), world);
    (gw, placements)
}

/// テスト 15（要件 3.2・3.3・5.3）: 並べ終えて記憶に中身がある状態から、絵の大きさを変えて
/// 置き直す → 移動の指示と同じ口で動かす → 作業領域を変えて寄せ直す → ゴーストの切り替えの
/// 閉じ方で閉じる → 終了の手順、を順に行い、どの段の後も記憶はバイトのまま前後で同じ。
///
/// 段ごとに、その段が窓を実際に動かした（大きさ・位置が変わった・窓が閉じた）ことを先に
/// 確かめる（何も起きないから書かない、という空の緑にしない）。記憶にはバルーンの相対位置の
/// 目印も入れておく（バルーンの相対位置を書く時機はバルーンのドラッグの確定だけ・要件 3.3）。
///
/// 終了の手順は `quit_app`（本番の終了の入口がすべて通る関数）を呼ぶ。テストの World には
/// 終了の受け口 `AppExit` が無いので、`quit_app` は窓を閉じたあと `error!` を残して戻る
/// （その分かれ道も `quit_app` の本文にある）。そのあとの実行系の後始末はメッセージループの
/// 外で World を持たないので、ここからは回せない。窓を閉じる段に窓が残っているよう、
/// 切り替えの閉じ方の後は本番の切り替えと同じく記憶から窓を作り直し、並べ終えてから終える
/// （作り直した窓は記憶に位置があるので、並べ終えた時点でも書かない＝要件 1.2）。
#[test]
fn memory_is_unchanged_by_resize_move_work_area_close_and_quit() {
    let ghost = MinimalGhost::plant("nowrite");
    let store = PersistStore::real(&ghost);
    let (placements, restored) = crate::restore_merged_placements(
        ghost.root(),
        prep_placements(),
        &work_area_snapshot(),
        DefaultEncoding::Ansi,
    );
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
    let saved = memory(&store, &ghost);
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
    assert_eq!(memory(&store, &ghost), saved, "置き直しは記憶を変えない");

    // (b) SHIORI の移動の指示と同じ口で動かす。
    let start = pos_of(&world, char1).expect("相方の位置");
    assert!(
        move_window_to(&mut world, char1, start.x - 77, start.y - 33),
        "前提: 移動の指示が窓へ届く"
    );
    assert_ne!(pos_of(&world, char1), Some(start), "前提: 相方が動く");
    assert_eq!(memory(&store, &ghost), saved, "移動の指示は記憶を変えない");

    // (c) 作業領域が変わって寄せ直す（本番の毎フレームの処理と同じ、同期 → 寄せ直しの順）。
    let before = pos_of(&world, char0).expect("本体の位置");
    let raised = RectPx {
        bottom: WORK_AREA.bottom - 100,
        ..WORK_AREA
    };
    let change = work_area_sync::sync_monitor_snapshot_with(
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
        memory(&store, &ghost),
        saved,
        "作業領域の寄せ直しは記憶を変えない"
    );

    // (d) ゴーストの切り替えの閉じ方で閉じる。
    let closed = close_windows_for_restart(&mut world);
    assert!(closed.closed() > 0, "前提: 窓が閉じる");
    assert_eq!(
        memory(&store, &ghost),
        saved,
        "切り替えの閉じ方は記憶を変えない"
    );

    // 本番の切り替えと同じく、記憶から窓を作り直して並べ終える（作業領域は元に戻す。同期段が
    // 作業領域の表と拡大率の表の両方を差し替える）。
    let _ = work_area_sync::sync_monitor_snapshot_with(
        &mut world,
        &s2_monitors_with_work_area(96, WORK_AREA),
    )
    .expect("前提: 作業領域の表が元へ差し替わる");
    let (gw2, _) = boot_remembered(&mut world, &ghost);
    finalize_chain_once_with(&shown, &mut world);
    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提: 作り直した窓を並べ終える"
    );
    assert_eq!(
        memory(&store, &ghost),
        saved,
        "記憶から作り直した窓は並べ終えた時点でも書かない"
    );

    // (e) 終了の手順。作り直した窓は記憶の位置に立つので、そのままでは終了のときに今の位置を
    // 書いても同じバイトになる。先に相方を記憶の位置から動かしておく。
    let partner = gw2.char_window(1).unwrap();
    let start = pos_of(&world, partner).expect("相方の位置");
    assert!(
        move_window_to(&mut world, partner, start.x - 50, start.y - 20),
        "前提: 移動の指示が窓へ届く"
    );
    assert_ne!(pos_of(&world, partner), Some(start), "前提: 相方が動く");
    assert_eq!(memory(&store, &ghost), saved, "移動の指示は記憶を変えない");
    let closed = quit_app(&mut world, ExitOrigin::Smoke);
    assert!(closed > 0, "前提: 終了の手順が窓を閉じる");
    assert_eq!(memory(&store, &ghost), saved, "終了の手順は記憶を変えない");

    store.finish();
    let _residue = drain_window_pos_commands();
}

/// 要件 1.2（・5.3）: 記憶の位置が画面の外にあるスコープを、記憶を重ねる本番の関数で画面の中へ
/// 寄せ直して表示し（表示している位置は記憶の値と違う）、既定の位置を消す関数で記憶から戻した
/// 扱いにしてから並べ直しを回しても、記憶はバイトのまま前後で同じ（寄せ直した位置を書き戻さない）。
#[test]
fn offscreen_saved_position_survives_finalize_while_shown_pulled_in() {
    let ghost = MinimalGhost::plant("nowrite-offscreen");
    let store = PersistStore::real(&ghost);
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
        persist_entries(&seed, entries);
    }
    assert_positions_saved(&store);
    let saved = memory(&store, &ghost);

    let mut world = persist_world();
    store.wire(&mut world);
    let (gw, placements) = boot_remembered(&mut world, &ghost);
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
        memory(&store, &ghost),
        saved,
        "寄せ直して表示している記憶は並べ終えた時点でも書き換わらない"
    );
    store.finish();
    let _residue = drain_window_pos_commands();
}
