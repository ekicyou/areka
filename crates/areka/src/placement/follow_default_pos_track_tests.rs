//! 既定位置の追跡規則（design D9／D16・要件 6.2）の決定論テスト。
//!
//! # ここが押さえる是正
//!
//! 連鎖の再解決は「現在位置が既定位置と一致するか」で**明示的に再配置されたスコープ**を
//! 除外する（`scope-chain-gap` 7.3）。拡大率が変わると全スコープの位置がシステム側の都合で
//! 動くため、既定位置を据え置いたままだと全件が「明示的に動かされた」へ倒れ、遷移後の
//! 解き直しが丸ごと空振りする。要件 6.2 はこれを名指しで禁じており、D9／D16 が
//! **単一の窓書込口で既定位置を追随させる**規則を定めた。
//!
//! 本ファイルはその規則そのもの——3 つの条件（route がシステム由来・対象がキャラ窓・
//! 書込前の位置が既定位置と一致）の**各条件が実際に効いていること**を、条件ごとに 1 つずつ
//! 外して固定する。遷移を通した観察可能な帰結（連鎖が対象から外さない）は
//! `emo2_boot/frame_default_pos_track_tests.rs` が持つ。
//!
//! # 零件の主張には陽性の対を置く
//!
//! 「追随しない」という主張は、追随の実装を丸ごと消しても恒真で通る。ゆえに追随しない側を
//! 問う各テストは、**同じ書込先**で route ないし対象だけを差し替えた陽性側を、同じテスト
//! 本体の中で続けて主張する。

use bevy_ecs::prelude::*;
use wintf::ecs::WindowPos;
use wintf::ecs::window::drain_window_pos_commands;

use super::test_support::fake_handle;
use super::{PlacementRoute, enqueue_window_set_pos, route_applies_visibility_guard};
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};

/// 檻が回すスコープ。
const SCOPE: usize = 0;

/// spawn 時の既定キャラ位置（`GhostWindows.default_char_pos` の初期値）。
const DEFAULT_POS: PointPx = PointPx { x: 1483, y: 757 };

/// システム由来の再アンカーが書き込む先（既定位置とは異なる値）。
const REANCHORED: PointPx = PointPx { x: 1266, y: 22 };

/// 利用者がドラッグで置いた位置（既定位置と一致しなくなる形を作るための値）。
const DRAGGED: PointPx = PointPx { x: 900, y: 400 };

/// キャラ窓 1 枚・バルーン窓 1 枚のスコープを spawn し、両窓へ偽 `WindowHandle` を付ける。
///
/// `spawn_ghost_windows` が `GhostWindows` 資源を挿すので、既定位置の台帳は本番と同じ経路で
/// 用意される（`default_char_pos` を手で書き込まない＝台帳の初期化そのものも檻に入る）。
fn world_with_one_scope() -> (World, Entity, Entity) {
    // 前のテストが残した窓書込指令を捨てる（**実行はしない**＝実 `SetWindowPos` を呼ばない）。
    let _residue = drain_window_pos_commands();

    let placements = vec![ScopePlacement {
        scope: SCOPE,
        char_pos: DEFAULT_POS,
        char_size: SizePx { w: 434, h: 687 },
        balloon_pos: PointPx { x: 1071, y: 732 },
        balloon_size: SizePx { w: 223, h: 158 },
        balloon_offset: PointPx { x: -412, y: -25 },
        // 関門を素通しさせる（本ファイルの主題は既定位置の追跡であって limit 補正ではない）。
        balloon_limit: false,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }];
    let mut world = World::new();
    let gw = spawn_ghost_windows(
        &mut world,
        &placements,
        &GhostTitles::from_scope_titles([(SCOPE, "a".to_string())]),
    );
    let char_window = gw.char_window(SCOPE).expect("char 窓がある");
    let balloon_window = gw.balloon_window(SCOPE).expect("balloon 窓がある");
    world.entity_mut(char_window).insert(fake_handle(0x100));
    world.entity_mut(balloon_window).insert(fake_handle(0x200));
    (world, char_window, balloon_window)
}

/// 台帳が持つ既定位置。
fn default_pos(world: &World) -> Option<PointPx> {
    world
        .get_resource::<GhostWindows>()
        .expect("GhostWindows がある")
        .default_char_pos(SCOPE)
}

/// 窓の現在位置（ミラー）。
fn current_pos(world: &World, window: Entity) -> PointPx {
    let p = world
        .get::<WindowPos>(window)
        .expect("WindowPos がある")
        .position
        .expect("position がある");
    PointPx { x: p.x, y: p.y }
}

/// 単一の窓書込口を 1 回叩く（位置のみ・寸は触らない）。
fn write_at(world: &mut World, window: Entity, to: PointPx, route: Option<PlacementRoute>) {
    assert!(
        enqueue_window_set_pos(world, window, to.x, to.y, None, route),
        "窓書込が成立していない（檻の前提が崩れている・route={route:?}）"
    );
    let _writes = drain_window_pos_commands();
}

// ---------------------------------------------------------------------------
// 追随する側（要件 6.2・D9 の 3 条件がすべて揃う形）
// ---------------------------------------------------------------------------

/// **是正前は赤・是正後は緑**: システム由来の再アンカーが、一致していた既定位置を書込先へ
/// 連れて行く。
///
/// 是正前は既定位置が `DEFAULT_POS` に据え置かれ、現在位置（`REANCHORED`）と食い違う
/// ——連鎖の除外判定がこの食い違いを「明示的に動かされた」と読むのが要件 6.2 の欠陥である。
#[test]
fn a_system_reanchor_carries_the_default_position_to_the_written_spot() {
    let (mut world, char_window, _balloon) = world_with_one_scope();
    assert_eq!(
        default_pos(&world),
        Some(DEFAULT_POS),
        "前提が崩れている: spawn 直後の既定位置が resolver 出力と一致していない"
    );

    write_at(
        &mut world,
        char_window,
        REANCHORED,
        Some(PlacementRoute::DpiReproject),
    );

    assert_eq!(
        default_pos(&world),
        Some(REANCHORED),
        "システム由来の再アンカーで既定位置が書込先へ追随していない（要件 6.2・D9）"
    );
    assert_eq!(
        current_pos(&world, char_window),
        REANCHORED,
        "現在位置が書込先になっていない（檻の前提が崩れている）"
    );
}

/// D9 が挙げる**システム由来の 6 経路すべて**で追随する（1 経路だけ配線した実装を弾く）。
#[test]
fn every_system_route_carries_the_default_position() {
    for route in PlacementRoute::ALL
        .into_iter()
        .filter(|r| r.is_system_reanchor())
    {
        let (mut world, char_window, _balloon) = world_with_one_scope();
        write_at(&mut world, char_window, REANCHORED, Some(route));
        assert_eq!(
            default_pos(&world),
            Some(REANCHORED),
            "route={route} で既定位置が追随していない（D9 のシステム由来 6 経路）"
        );
    }
}

// ---------------------------------------------------------------------------
// 追随しない側（条件を 1 つずつ外す・各件に陽性の対を置く）
// ---------------------------------------------------------------------------

/// 条件 3 を外す: ドラッグで動かした後（現在位置 ≠ 既定位置）はシステム由来でも追随しない。
///
/// 追随させてしまうと「利用者が動かした」という事実そのものが消え、連鎖が明示配置の窓を
/// 既定位置へ引き戻す（`scope-chain-gap` 7.3 の破壊）。
#[test]
fn a_dragged_window_keeps_its_default_position_untouched() {
    let (mut world, char_window, _balloon) = world_with_one_scope();

    // ドラッグは route を持たない書込（`None`）である。
    write_at(&mut world, char_window, DRAGGED, None);
    assert_eq!(
        default_pos(&world),
        Some(DEFAULT_POS),
        "ドラッグ（route なし）で既定位置が動いている（明示操作は追随の対象外・D9）"
    );

    // ここからシステム由来で動かしても、既に食い違っているので追随しない。
    write_at(
        &mut world,
        char_window,
        REANCHORED,
        Some(PlacementRoute::DpiReproject),
    );
    assert_eq!(
        default_pos(&world),
        Some(DEFAULT_POS),
        "ドラッグ後の窓でシステム由来の書込が既定位置を動かした（条件 3 が効いていない）"
    );

    // 陽性の対: 同じ route・同じ書込先でも、一致していた窓なら追随する。
    let (mut fresh, fresh_char, _fresh_balloon) = world_with_one_scope();
    write_at(
        &mut fresh,
        fresh_char,
        REANCHORED,
        Some(PlacementRoute::DpiReproject),
    );
    assert_eq!(
        default_pos(&fresh),
        Some(REANCHORED),
        "陽性の対が成立していない（駆動が死んでいれば上の主張は空虚である）"
    );
}

/// 条件 1 を外す: 明示操作・従属量の経路は、位置が一致していても追随させない。
#[test]
fn explicit_routes_leave_the_default_position_alone_even_when_it_matches() {
    for route in PlacementRoute::ALL
        .into_iter()
        .filter(|r| !r.is_system_reanchor())
    {
        let (mut world, char_window, _balloon) = world_with_one_scope();
        write_at(&mut world, char_window, REANCHORED, Some(route));
        assert_eq!(
            default_pos(&world),
            Some(DEFAULT_POS),
            "route={route} は明示操作・従属量の経路なのに既定位置が追随した（条件 1）"
        );
    }

    // 陽性の対: 同じ書込先をシステム由来の route で叩けば追随する。
    let (mut fresh, fresh_char, _fresh_balloon) = world_with_one_scope();
    write_at(
        &mut fresh,
        fresh_char,
        REANCHORED,
        Some(PlacementRoute::WorkAreaResnap),
    );
    assert_eq!(
        default_pos(&fresh),
        Some(REANCHORED),
        "陽性の対が成立していない（駆動が死んでいれば上の主張は空虚である）"
    );
}

/// 条件 2 を外す: バルーン窓への書込はスコープの既定位置に触れない。
///
/// バルーン窓は既定位置を持たない従属量である。ここで触れると、キャラ窓が動いていないのに
/// 基準だけがバルーン座標へ飛ぶ。
#[test]
fn a_balloon_window_write_never_touches_the_scope_default_position() {
    let (mut world, _char_window, balloon_window) = world_with_one_scope();

    write_at(
        &mut world,
        balloon_window,
        REANCHORED,
        Some(PlacementRoute::DpiReproject),
    );
    assert_eq!(
        default_pos(&world),
        Some(DEFAULT_POS),
        "バルーン窓への書込でスコープの既定位置が動いた（条件 2 が効いていない）"
    );

    // 陽性の対: 同じ route・同じ書込先をキャラ窓へ叩けば追随する。
    let (mut fresh, fresh_char, _fresh_balloon) = world_with_one_scope();
    write_at(
        &mut fresh,
        fresh_char,
        REANCHORED,
        Some(PlacementRoute::DpiReproject),
    );
    assert_eq!(
        default_pos(&fresh),
        Some(REANCHORED),
        "陽性の対が成立していない（駆動が死んでいれば上の主張は空虚である）"
    );
}

/// 既定位置 `None`（保存位置が復元されたスコープ）は `None` のままである。
///
/// `None` は「そもそも既定配置ではない」を表す**標**であって位置の欠落ではない
/// （`scope-chain-gap` 7.3）。位置で塗り潰すと、復元スコープが連鎖の対象へ復活してしまう。
#[test]
fn a_restored_scope_stays_marked_as_not_default_placed() {
    let (mut world, char_window, _balloon) = world_with_one_scope();
    world
        .get_resource_mut::<GhostWindows>()
        .expect("GhostWindows がある")
        .clear_default_char_pos(SCOPE);
    assert_eq!(
        default_pos(&world),
        None,
        "前提が崩れている: 復元スコープの標が立っていない"
    );

    write_at(
        &mut world,
        char_window,
        REANCHORED,
        Some(PlacementRoute::DpiReproject),
    );

    assert_eq!(
        default_pos(&world),
        None,
        "復元スコープの `None` が位置で塗り潰された（D9: `None` は `None` のまま）"
    );
}

// ---------------------------------------------------------------------------
// 区分そのもの（定義元が 1 本であること）
// ---------------------------------------------------------------------------

/// システム由来の再アンカーは D9 が挙げる 6 経路**ちょうど**である。
#[test]
fn the_system_reanchor_partition_is_exactly_the_six_routes_named_by_d9() {
    let system: Vec<PlacementRoute> = PlacementRoute::ALL
        .into_iter()
        .filter(|r| r.is_system_reanchor())
        .collect();
    assert_eq!(
        system,
        vec![
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::ReportedSizeReconcile,
            PlacementRoute::WorkAreaResnap,
            PlacementRoute::ChainRealign,
        ],
        "システム由来の経路集合が D9 の 6 経路と食い違う"
    );
}

/// 可視性の遷移ガードの発火条件は、既定位置の追跡と**同一の区分**である（設計の明言）。
///
/// 委譲を解いて列を 2 本に戻した実装を弾く檻である。意図的に割る日が来たら、この檻を
/// 落としたうえで理由を設計へ登記すること。
#[test]
fn the_visibility_guard_reads_the_same_partition() {
    for route in PlacementRoute::ALL {
        assert_eq!(
            route_applies_visibility_guard(route),
            route.is_system_reanchor(),
            "route={route}: 可視性ガードの発火条件と既定位置の追跡区分が食い違っている"
        );
    }
}
