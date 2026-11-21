//! 整合待ちの札の監視が鳴る対象の檻。
//!
//! 単一の窓書込口（`window_move::enqueue_window_set_pos`）の入口には「[`DpiSyncHold`] を
//! 持つ窓への窓書込は 0」という不変条件の監視がある。監視が鳴るべき相手は
//! 見送りが覆うべき経路であって、すべての経路ではない。利用者のドラッグや
//! `\![move]` のようなスクリプトの明示操作は、そもそも見送らない（`move_window_to`
//! の doc が「スクリプトの明示操作ゆえ遷移ガードは適用しない」と述べる）。
//!
//! 2 種類の証拠を置く。
//! ⑴ 経路語ごとの分類（13 語＋route 無し）を純関数に対して総当たりで確かめる。
//! ⑵ 本物の書込口を通して、免除側は鳴らず・鳴る側は落ちることを実行で示す。
//! ⑵ の対は同じ土台・同じ札で組み、変えるのは経路語 1 点だけ。

use bevy_ecs::prelude::*;
use wintf::ecs::Point;
use wintf::ecs::pointer::Phase;

use super::test_support::{
    arrangement_at, drag_event_at, dragging_state, fake_handle, position_of,
    single_monitor_snapshot, window_pos_sized,
};
use super::window_move::deferral_covers_route;
use super::{
    Anchored, DpiSyncHold, PlacementRoute, move_window_to, move_window_with_route, on_char_drag,
    resize_window_keep_position,
};
use crate::resolver::{Anchor, SizePx};
use log_capture_kit::{CapturedEvent, capture};

/// 監視の警告本文（実機ログの grep 語と同一）。
const INVARIANT_WARNING: &str = "整合待ちの札がある窓へ窓書込が到達した";

fn warnings_containing(events: &[CapturedEvent], needle: &str) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .filter(|e| e.message().contains(needle))
        .count()
}

/// 書込が成立する最小の窓（`WindowHandle`・`WindowPos`・`Arrangement`）を 1 枚だけ持つ World。
///
/// `hold` で整合待ちの札の有無だけを切り替え、鳴る側と鳴らない側の対を同一の土台で組む。
fn held_world(hold: bool) -> (World, Entity) {
    let mut world = World::new();
    let mut entity = world.spawn((
        fake_handle(0x7500),
        window_pos_sized(1207, 356, 434, 687),
        arrangement_at(1207.0, 356.0),
    ));
    if hold {
        entity.insert(DpiSyncHold { since_frame: 3 });
    }
    let window = entity.id();
    (world, window)
}

/// このスレッドに積まれた書込指令を捨てる（後始末のみ。キューはスレッドごとなのでテスト間の分離にはならない）。
fn drop_queued_writes() {
    let _residue = wintf::ecs::window::drain_window_pos_commands();
}

// -------------------------------------------------------------------------
// ⑴ 経路語ごとの分類（13 語＋route 無しを 1 つも落とさない）
// -------------------------------------------------------------------------

/// 各経路語について「監視が鳴るか」をテスト側にも独立に書き下し、本番の網羅 match と突き合わせる（二重記帳）。
const EXPECTED: [(PlacementRoute, bool); 13] = [
    // 見送りが覆う経路（鳴る）
    (PlacementRoute::AnchorChange, true),
    (PlacementRoute::Resnap, true),
    (PlacementRoute::DpiReproject, true),
    (PlacementRoute::KeepPositionResize, true),
    (PlacementRoute::ReportedSizeReconcile, true),
    (PlacementRoute::WorkAreaResnap, true),
    (PlacementRoute::ChainRealign, true),
    (PlacementRoute::PatternMove, true),
    // 見送らないことが正しい経路（鳴らない）
    (PlacementRoute::SpawnInitial, false),
    (PlacementRoute::Restore, false),
    (PlacementRoute::BalloonFollow, false),
    (PlacementRoute::MoveCue, false),
    (PlacementRoute::BalloonLimitRelease, false),
];

/// 13 語すべてが分類され、期待と一致する（語の取りこぼしが無い）。
#[test]
fn every_placement_route_is_classified_for_the_hold_watch() {
    assert_eq!(
        EXPECTED.len(),
        PlacementRoute::ALL.len(),
        "分類表の語数が経路語彙の総数と食い違う（語が増えたのに分類していない）"
    );
    for route in PlacementRoute::ALL {
        let (_, expected) = EXPECTED
            .iter()
            .copied()
            .find(|(r, _)| *r == route)
            .unwrap_or_else(|| panic!("分類表に {route} が無い（新設語の分類漏れ）"));
        assert_eq!(
            deferral_covers_route(Some(route)),
            expected,
            "{route} の分類が期待と食い違う"
        );
    }
}

/// route を名乗らない書込（＝ドラッグ経路のキャラ窓書込）は鳴らない。
///
/// `None` の発行元はドラッグだけで、利用者の明示操作なので見送らず、監視の対象にもしない。
#[test]
fn a_write_without_a_route_is_not_watched() {
    assert!(!deferral_covers_route(None));
}

// -------------------------------------------------------------------------
// ⑵ 本物の書込口を通した対（同じ土台・同じ札・違うのは経路語 1 点だけ）
// -------------------------------------------------------------------------

/// `\![move]`（[`PlacementRoute::MoveCue`]）は札のある窓へ届いても鳴らない（明示操作は見送らないので、鳴らすのは偽の警報）。
#[test]
fn the_explicit_script_move_reaches_a_waiting_window_without_tripping_the_watch() {
    let (mut world, window) = held_world(true);

    let (moved, events) = capture(|| move_window_to(&mut world, window, 1301, 356));
    drop_queued_writes();

    assert!(moved, "明示操作の書込が成立していない");
    assert_eq!(
        position_of(&world, window),
        Point { x: 1301, y: 356 },
        "明示操作で窓が動いていない（見送られてしまっている）"
    );
    assert_eq!(
        warnings_containing(&events, INVARIANT_WARNING),
        0,
        "明示操作で監視が鳴っている（偽の警報）: {events:?}"
    );
}

/// 上と同じ土台・同じ札で、変えるのは経路語 1 点だけ。見送りが覆うべき経路
/// （[`PlacementRoute::ChainRealign`]＝システム由来）で届けば落ちる。
///
/// 本番の `ChainRealign` は `realign_chain_once_with` が「札を持つゴースト窓が 1 つも
/// 無い」を解決の条件に置いて止めている。ここは反映の手続きを直接呼んでその見送りを
/// 迂回し、将来その条件が壊れたときに鳴ってほしい形にしている。
#[test]
#[should_panic(expected = "DpiSyncHold")]
#[cfg_attr(not(debug_assertions), ignore = "debug_assert! に依る")]
fn a_system_write_that_bypasses_its_deferral_still_trips_the_watch() {
    let (mut world, window) = held_world(true);
    move_window_with_route(&mut world, window, 1301, 356, PlacementRoute::ChainRealign);
}

/// 鳴る側が札の有無に関わらず常に落ちるわけではないことの対（札だけを外す）。
#[test]
fn the_same_system_write_passes_when_the_window_is_not_waiting() {
    let (mut world, window) = held_world(false);

    let moved = move_window_with_route(&mut world, window, 1301, 356, PlacementRoute::ChainRealign);
    drop_queued_writes();

    assert!(moved, "札の無い窓へのシステム由来の書込が通らない");
    assert_eq!(position_of(&world, window), Point { x: 1301, y: 356 });
}

/// 位置据置きリサイズ（[`PlacementRoute::KeepPositionResize`]）も鳴る側である。
///
/// この語だけは `is_system_reanchor` が `false`（バルーン窓の従属量）でありながら鳴る。
/// 唯一の発行元 `reconcile_window_size` が拡大率の相と報告寸の突合の内側にあり、
/// 見送りが覆っているから。分類の軸が「誰が動かしたか」ではなく「見送りが覆うか」
/// であることを実行で示す。
#[test]
#[should_panic(expected = "DpiSyncHold")]
#[cfg_attr(not(debug_assertions), ignore = "debug_assert! に依る")]
fn the_keep_position_resize_of_a_waiting_balloon_trips_the_watch() {
    let (mut world, window) = held_world(true);
    resize_window_keep_position(&mut world, window, SizePx { w: 400, h: 300 });
}

/// 利用者のドラッグ（route を名乗らない書込）は札のある窓へ届いても鳴らない。
///
/// 上の 3 本と違い、合成した経路語ではなく本物のドラッグ経路（`on_char_drag` の
/// 単一ライター腕）を通す。`None` を発行する実在のトリガはこれだけで、利用者が
/// 任意の時点で起こせるため最も到達しやすい偽の警報になる。
#[test]
fn a_user_drag_reaches_a_waiting_window_without_tripping_the_watch() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=356
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x7501),
            window_pos_sized(1207, 356, 434, 687),
            arrangement_at(1207.0, 356.0),
            Anchored(Anchor::Bottom),
            dragging_state((1207, 356), start),
            DpiSyncHold { since_frame: 3 },
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(window, start, (1450, 350)));
    let (_, events) = capture(|| on_char_drag(&mut world, window, window, &ev));
    drop_queued_writes();

    assert_eq!(
        position_of(&world, window),
        Point { x: 1257, y: 356 },
        "ドラッグで窓が動いていない（駆動が死んでいる＝零件が空虚になる）"
    );
    assert_eq!(
        warnings_containing(&events, INVARIANT_WARNING),
        0,
        "利用者のドラッグで監視が鳴っている（偽の警報）: {events:?}"
    );
}

/// 随伴バルーンの追従（[`PlacementRoute::BalloonFollow`]）は免除される。
#[test]
fn the_trailing_balloon_follow_is_still_exempt() {
    let (mut world, balloon) = held_world(true);

    let (moved, events) = capture(|| {
        move_window_with_route(
            &mut world,
            balloon,
            1301,
            356,
            PlacementRoute::BalloonFollow,
        )
    });
    drop_queued_writes();

    assert!(moved, "随伴の追従が成立していない");
    assert_eq!(
        position_of(&world, balloon),
        Point { x: 1301, y: 356 },
        "随伴の追従で窓が動いていない（駆動が死んでいる）"
    );
    assert_eq!(
        warnings_containing(&events, INVARIANT_WARNING),
        0,
        "随伴の追従で監視が鳴っている（既存の免除が壊れた）: {events:?}"
    );
}
