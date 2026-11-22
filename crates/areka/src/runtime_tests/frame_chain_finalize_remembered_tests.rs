//! 起動・作り直しの並べ直しが、記憶から戻した組を覚えた印で今の幅で隣り合わせる檻
//!
//! # ここが押さえるもの
//!
//! 200% で隣り合って覚えた組を 100% で起動すると、記憶の位置（下端の中央）のまま今の幅で立つ
//! ので隙間 359 が開く。並べ直しは覚えた印 `left` を見てこの組を今の幅で隣り合わせ、印 `none`・
//! 印の無い古い記憶・覚えたときと同じ幅では誰も動かさない（その檻は `areka-runtime` の同名の
//! ファイル）。隣り合わせた位置は記憶に書かない。
//!
//! 窓を作る閉包（`ghost_session.rs`）はテストから呼べないので、同じ順を
//! `persist_test_support::boot_remembered` で踏む。

use areka_sylphya::PersistKey;
use bevy_ecs::prelude::World;
use wintf::ecs::window::drain_window_pos_commands;

use super::finalize_chain_once_with;
use super::shared_test_support::{
    GAP_WITHOUT_REALIGN, HIGH_SIZE_0, LOW_SIZE_0, LOW_SIZE_1, PAIR_LINE, boot_remembered_at_high,
    gap, low_sizes, lower_the_scale, pair_gap, remembered_placements, seat_remembered_pair,
};
use super::test_support::{FakeReports, FrameHarness, PerTargetSizes, pos_of};
use areka_placement::chain_finalize::{AdjacencyMark, ChainFinalized};
use areka_placement::persist::{char_pos_entries, persist_entries};
use areka_placement::resolver::{PointPx, ScopePlacement};
use areka_placement::spawn::GhostWindows;
use areka_runtime::frame::persist_test_support::{PersistStore, WORK_AREA, persist_world};
use areka_session::app_exit::close_windows_for_restart;
use log_capture_kit::{LineFormat, capture_lines};

/// 窓の一式を作り、全スコープを記憶から戻した形にして（既定の位置を外し、相方に `mark` を据える）、
/// `shown` の絵で置き直してから並べ直しを 1 回回す。返すのは並べ直しで出た窓への指令の数と、
/// 並べ直しの間のログ。
fn boot(
    world: &mut World,
    mark: Option<AdjacencyMark>,
    shown: &PerTargetSizes,
) -> (usize, Vec<String>) {
    boot_from(world, &remembered_placements(), mark, shown)
}

/// [`boot`] の配置を `placements` に替えた形。
fn boot_from(
    world: &mut World,
    placements: &[ScopePlacement],
    mark: Option<AdjacencyMark>,
    shown: &PerTargetSizes,
) -> (usize, Vec<String>) {
    seat_remembered_pair(world, placements, mark, shown);
    let logs = capture_lines(LineFormat::LevelTargetFields, || {
        finalize_chain_once_with(shown, world)
    })
    .1;
    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提が崩れている: 並べ直しが駆動していない"
    );
    (drain_window_pos_commands().len(), logs)
}

fn window_pos_keys(saved: &std::collections::HashMap<PersistKey, String>) -> Vec<String> {
    saved
        .keys()
        .filter(|k| matches!(k, PersistKey::WindowPos { .. }))
        .map(|k| format!("{k:?}"))
        .collect()
}

/// ⑴ 実行中に 200% → 100% で詰めた後、記憶へ送られた束に位置が無い。
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
/// 窓の一式を閉じて作り直しても同じ（読み直し・切り替えは同じ閉包を通る）。
#[test]
fn remembered_left_pair_stands_adjacent_at_current_widths() {
    let mut world = persist_world();
    let store = PersistStore::fake();
    store.wire(&mut world);

    let (moved, logs) = boot(&mut world, Some(AdjacencyMark::Left), &low_sizes());
    assert_eq!(
        (moved > 0, pair_gap(&world, LOW_SIZE_1.0)),
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
    let (moved, _) = boot(&mut world, Some(AdjacencyMark::Left), &low_sizes());
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

/// ⑹ 隣り合っていた組（印 `left`）で、本体が作業領域の左端に見えていて相方が作業領域の外に
/// 戻った形。相方は見えている本体の隣へ付け直され、左の隣は作業領域の外なので右の隣へ並べ替えて
/// 作業領域の中に立つ。本体は動かず、隣り合わせた位置は記憶に書かない。
#[test]
fn remembered_left_partner_outside_rejoins_inside_work_area() {
    let mut world = persist_world();
    let store = PersistStore::fake();
    store.wire(&mut world);
    let mut placements = remembered_placements();
    // 置き直しは下端の中央を保って今の幅で立つので、今の幅で作業領域の左端に立つ x を置く。
    placements[0].char_pos.x = WORK_AREA.left - (HIGH_SIZE_0.0 - LOW_SIZE_0.0) as i32 / 2;
    // 相方はどの作業領域とも交わらない位置（本体の左の隣は作業領域の外）。
    placements[1].char_pos.x = WORK_AREA.left - 3000;

    let (moved, logs) = boot_from(
        &mut world,
        &placements,
        Some(AdjacencyMark::Left),
        &low_sizes(),
    );

    let gw = world.resource::<GhostWindows>();
    let body = pos_of(&world, gw.char_window(0).unwrap()).unwrap();
    let partner = pos_of(&world, gw.char_window(1).unwrap()).unwrap();
    assert_eq!(body.x, WORK_AREA.left, "本体は動かない");
    assert_eq!(
        (moved > 0, partner.x),
        (true, body.x + LOW_SIZE_0.0 as i32),
        "(並べ直しで窓へ指令が出た, 相方は本体の右の隣)"
    );
    assert!(
        partner.x + LOW_SIZE_1.0 as i32 <= WORK_AREA.right && partner.y >= WORK_AREA.top,
        "相方は作業領域の中: {partner:?}"
    );
    assert!(
        logs.iter().any(|l| l.contains("[work-area-clamp] Clamp")
            && l.contains("scene=\"remembered-join\"")
            && l.contains("how=\"flipped\"")),
        "記憶からの隣り合わせの場面で並べ替えた行が出る: {logs:?}"
    );
    assert_eq!(
        window_pos_keys(&store.load()),
        Vec::<String>::new(),
        "隣り合わせて寄せた位置は記憶に書かない（2.4）"
    );
}
