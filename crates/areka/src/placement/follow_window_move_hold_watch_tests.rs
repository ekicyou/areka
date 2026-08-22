//! 整合待ちの札の**監視**が鳴る対象の檻（task 7.5・設計 C5・要件 5.8）。
//!
//! # 何を固定するか
//!
//! 単一の窓書込口（`window_move::enqueue_window_set_pos`）の入口には「[`DpiSyncHold`] を
//! 持つ窓への窓書込は 0」という不変条件の監視がある。この監視が鳴るべき相手は
//! **見送りが覆うべき経路**であって、すべての経路ではない——利用者のドラッグや
//! `\![move]` のようなスクリプトの明示操作は、設計上そもそも見送らない（`move_window_to`
//! の doc が「スクリプトの明示操作ゆえ遷移ガードは適用しない」と述べる）。免除が
//! `BalloonFollow` の 1 語しか無いあいだ、これらの正当な書込は偽の警報を鳴らし、
//! debug ビルドでは `debug_assert!` で落ちていた。
//!
//! 本ファイルは 2 種類の証拠を置く——⑴ 経路語ごとの分類（12 語＋route 無しを 1 つも
//! 落とさない）を純関数に対して総当たりで確かめる ⑵ 本物の書込口を実際に通して、免除側は
//! 鳴らず・鳴る側は従来どおり落ちることを実行で示す。⑵ の対は**同じ土台・同じ札**で
//! 組み、変えるのは経路語 1 点だけである。

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
use crate::placement::resolver::{Anchor, SizePx};
use crate::placement::test_support::{LogEvent, capture_logs};

/// 監視の警告本文（実機ログの grep 語と同一）。
const INVARIANT_WARNING: &str = "整合待ちの札がある窓へ窓書込が到達した";

fn warnings_containing(events: &[LogEvent], needle: &str) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .filter(|e| e.message().contains(needle))
        .count()
}

/// 書込が成立する最小の窓（`WindowHandle`・`WindowPos`・`Arrangement`）を 1 枚だけ持つ World。
///
/// `hold` で整合待ちの札の有無だけを切り替える——鳴る側と鳴らない側の対を、この 1 bit と
/// 経路語 1 点以外は**同一の土台**で組むための口である。
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

/// 積まれた書込指令を捨てる（プロセス共有のキューをテスト間で持ち越さない）。
fn drop_queued_writes() {
    let _residue = wintf::ecs::window::drain_window_pos_commands();
}

// -------------------------------------------------------------------------
// ⑴ 経路語ごとの分類（12 語＋route 無しを 1 つも落とさない）
// -------------------------------------------------------------------------

/// 各経路語について「監視が鳴るか」を**テスト側にも独立に書き下し**、本番の網羅 match と
/// 突き合わせる（二重記帳）。本番だけが正しいと仮定しないための対である。
const EXPECTED: [(PlacementRoute, bool); 12] = [
    // 見送りが覆う経路（鳴る）。
    (PlacementRoute::AnchorChange, true),
    (PlacementRoute::Resnap, true),
    (PlacementRoute::DpiReproject, true),
    (PlacementRoute::KeepPositionResize, true),
    (PlacementRoute::ReportedSizeReconcile, true),
    (PlacementRoute::WorkAreaResnap, true),
    (PlacementRoute::ChainRealign, true),
    // 見送らないことが正しい経路（鳴らない）。
    (PlacementRoute::SpawnInitial, false),
    (PlacementRoute::Restore, false),
    (PlacementRoute::BalloonFollow, false),
    (PlacementRoute::MoveCue, false),
    (PlacementRoute::BalloonLimitRelease, false),
];

/// 12 語すべてが分類され、期待と一致する（語の取りこぼしが無い）。
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

/// 分類表に同じ語が 2 度現れない（重複で「網羅している」ように見せない）。
#[test]
fn the_classification_table_has_no_duplicate_route() {
    for (i, (route, _)) in EXPECTED.iter().enumerate() {
        assert!(
            !EXPECTED[i + 1..].iter().any(|(other, _)| other == route),
            "分類表に {route} が重複している"
        );
    }
}

/// route を名乗らない書込（＝ドラッグ経路のキャラ窓書込）は鳴らない。
///
/// `None` は「観測を所有しない書込」でありドラッグ以外に発行元が無い——利用者の明示操作
/// ゆえ見送らないことが正しく、監視の対象にもしない。
#[test]
fn a_write_without_a_route_is_not_watched() {
    assert!(!deferral_covers_route(None));
}

// -------------------------------------------------------------------------
// ⑵ 本物の書込口を通した対（同じ土台・同じ札・違うのは経路語 1 点だけ）
// -------------------------------------------------------------------------

/// `\![move]`（[`PlacementRoute::MoveCue`]）は札のある窓へ届いても鳴らない——明示操作は
/// そもそも見送らないので、鳴らすのは偽の警報である。
#[test]
fn the_explicit_script_move_reaches_a_waiting_window_without_tripping_the_watch() {
    let (mut world, window) = held_world(true);

    let (moved, events) = capture_logs(|| move_window_to(&mut world, window, 1301, 356));
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

/// 上と**同じ土台・同じ札**で、変えるのは経路語 1 点だけ——見送りが覆うべき経路
/// （[`PlacementRoute::ChainRealign`]＝システム由来）で届けば従来どおり落ちる。
///
/// `ChainRealign` は本番では `realign_chain_once_with` が「札を持つゴースト窓が 1 つも
/// 無い」を解決の条件に置いて止めている。ここは反映の手続きを直接呼んでその見送りを
/// 迂回した形＝将来その条件が壊れたときに鳴ってほしい形である。
#[test]
#[should_panic(expected = "DpiSyncHold")]
fn a_system_write_that_bypasses_its_deferral_still_trips_the_watch() {
    let (mut world, window) = held_world(true);
    move_window_with_route(&mut world, window, 1301, 356, PlacementRoute::ChainRealign);
}

/// 鳴る側が「札の有無に関わらず常に落ちる」ではないことの対（札だけを外す）。
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
/// この語だけは `is_system_reanchor` が `false`（バルーン窓の従属量）でありながら鳴る——
/// 唯一の発行元 `reconcile_window_size` が拡大率の相と報告寸の突合の**内側**にあり、
/// 見送りが覆っているからである。分類の軸が「誰が動かしたか」ではなく「見送りが覆うか」
/// であることを、実行で示すのがこの 1 本の役目である。
#[test]
#[should_panic(expected = "DpiSyncHold")]
fn the_keep_position_resize_of_a_waiting_balloon_trips_the_watch() {
    let (mut world, window) = held_world(true);
    resize_window_keep_position(&mut world, window, SizePx { w: 400, h: 300 });
}

/// 利用者のドラッグ（route を名乗らない書込）は札のある窓へ届いても鳴らない。
///
/// 上の 3 本と違い、ここは合成した経路語ではなく**本物のドラッグ経路**（`on_char_drag` の
/// 単一ライター腕）を通す——`None` を発行する実在のトリガはこれだけであり、利用者が
/// 任意の時点で起こせる＝最も到達しやすい偽の警報だったからである。
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
    let (_, events) = capture_logs(|| on_char_drag(&mut world, window, window, &ev));
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

/// 随伴バルーンの追従（[`PlacementRoute::BalloonFollow`]）の免除は従来どおり生きている。
#[test]
fn the_trailing_balloon_follow_is_still_exempt() {
    let (mut world, balloon) = held_world(true);

    let (moved, events) = capture_logs(|| {
        move_window_with_route(&mut world, balloon, 1301, 356, PlacementRoute::BalloonFollow)
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
