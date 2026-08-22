use bevy_ecs::prelude::*;
use wintf::ecs::Point;
use wintf::ecs::SizeI;
use wintf::ecs::pointer::Phase;

use super::test_support::{
    arrangement_at, arrangement_offset_of, drag_event_at, dragging_state, fake_handle, position_of,
    single_monitor_snapshot, size_of, window_pos_at, window_pos_sized,
};
use super::{Anchored, BalloonFollow, move_window_to, on_char_drag};
use crate::placement::resolver::{Anchor, PointPx, SizePx};

// -------------------------------------------------------------------------
// move_window_to（R7 公開 API・7.1/7.2/7.3・U4）
// -------------------------------------------------------------------------

/// 観測可能な完了状態: headless World 上で move_window_to を呼ぶと
/// 対象窓の WindowPos が期待座標へ更新される（物理 px 素通し・U4）。
/// 座標は 96 の倍数を避けた値を使い、隠れた dpi/96 再スケールがあれば
/// 完全一致が崩れる檻とする（07-05 欠陥の再発防止・3.2/3.3）。
#[test]
fn move_window_to_updates_window_pos_physical_px() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x1234), window_pos_at(10, 20)))
        .id();

    assert!(move_window_to(&mut world, window, 1531, 883));
    assert_eq!(position_of(&world, window), Point { x: 1531, y: 883 });
}

/// WindowHandle 未付与（窓生成前）は false を返し、位置も変更しない。
#[test]
fn move_window_to_without_handle_returns_false() {
    let mut world = World::new();
    let window = world.spawn(window_pos_at(10, 20)).id();

    assert!(!move_window_to(&mut world, window, 500, 600));
    assert_eq!(position_of(&world, window), Point { x: 10, y: 20 });
}

/// despawn 済み（対象不在）の entity も false（silent no-op にしない・panic しない）。
#[test]
fn move_window_to_on_despawned_entity_returns_false() {
    let mut world = World::new();
    let window = world.spawn((fake_handle(0x1234), window_pos_at(0, 0))).id();
    world.despawn(window);

    assert!(!move_window_to(&mut world, window, 100, 200));
}

/// BalloonFollow を持つ対象の移動はバルーンも offset 維持で随伴移動する
/// （T-I4: 移動後も balloon_pos − char_pos ≡ offset が保存される）。
#[test]
fn move_window_to_moves_balloon_with_offset_preserved() {
    let mut world = World::new();
    let balloon = world.spawn((fake_handle(0x2000), window_pos_at(0, 0))).id();
    let offset = PointPx { x: -412, y: -25 };
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(50, 60),
            BalloonFollow { balloon, offset },
        ))
        .id();

    assert!(move_window_to(&mut world, window, 907, 1201));

    let char_pos = position_of(&world, window);
    let balloon_pos = position_of(&world, balloon);
    assert_eq!(char_pos, Point { x: 907, y: 1201 });
    assert_eq!(
        balloon_pos,
        Point {
            x: 907 + offset.x,
            y: 1201 + offset.y
        }
    );
    // offset 保存則（balloon_pos − char_pos ≡ offset）
    assert_eq!(balloon_pos.x - char_pos.x, offset.x);
    assert_eq!(balloon_pos.y - char_pos.y, offset.y);
}

/// 対象自身に WindowHandle が無ければ false で、バルーンも動かさない。
#[test]
fn move_window_to_target_without_handle_does_not_move_balloon() {
    let mut world = World::new();
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(70, 80)))
        .id();
    let window = world
        .spawn((
            window_pos_at(50, 60),
            BalloonFollow {
                balloon,
                offset: PointPx { x: 11, y: 22 },
            },
        ))
        .id();

    assert!(!move_window_to(&mut world, window, 907, 1201));
    assert_eq!(position_of(&world, window), Point { x: 50, y: 60 });
    assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
}

/// バルーン側に WindowHandle が無い場合: 対象の移動自体は成功（true）し、
/// バルーンは動かない（warn ログ・silent failure ではない）。
#[test]
fn move_window_to_balloon_without_handle_still_moves_target() {
    let mut world = World::new();
    let balloon = world.spawn(window_pos_at(70, 80)).id();
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(50, 60),
            BalloonFollow {
                balloon,
                offset: PointPx { x: 11, y: 22 },
            },
        ))
        .id();

    assert!(move_window_to(&mut world, window, 907, 1201));
    assert_eq!(position_of(&world, window), Point { x: 907, y: 1201 });
    assert_eq!(position_of(&world, balloon), Point { x: 70, y: 80 });
}

// -------------------------------------------------------------------------
// Arrangement.offset 同期（task 8.3-fix・4.8 実機ブロッカ）
//
// enqueue_window_set_pos は WindowPos を bypass_change_detection() で書くため
// Changed<WindowPos> が発火せず、wintf の
// sync_window_arrangement_from_window_pos は走らない。同期を怠ると
// GlobalArrangement（αマスクヒットテストの境界）が spawn 位置に取り残され、
// 移動後のバルーンがクリック死する（実機で確認された 4.8 ブロッカ）。
// 実 pipeline では window entity に Arrangement が付く（Visual::on_add）が、
// bare World には無いので spawn 時 offset 付きで手動挿入して檻にする。
// 期待値は wintf DragEnd 直接同期と同じ `as f32` 転写の完全一致。
// -------------------------------------------------------------------------

use wintf::ecs::layout::Offset;

/// (a) 実 on_char_drag（Bubble DragEvent＋DraggingState・8.2R 単一ライター）:
/// 移動後、キャラ窓・随伴バルーンとも Arrangement.offset が
/// WindowPos.position と一致する（GA ヒットテスト境界の追従・4.8）。
#[test]
fn on_char_drag_syncs_arrangement_offset_of_char_and_balloon() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043・釘付け Y=356
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(795, 331),
            arrangement_at(795.0, 331.0),
        ))
        .id();
    let offset = PointPx { x: -412, y: -25 };
    let start = (1400, 600);
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687),
            arrangement_at(1207.0, 356.0),
            Anchored(Anchor::Bottom),
            BalloonFollow { balloon, offset },
            dragging_state((1207, 356), start),
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(window, start, (1450, 350)));
    assert!(!on_char_drag(&mut world, window, window, &ev));

    // 適用後キャラ窓 (1257, 356)・バルーン (1257−412, 356−25)
    let char_pos = position_of(&world, window);
    assert_eq!(char_pos, Point { x: 1257, y: 356 });
    assert_eq!(
        arrangement_offset_of(&world, window),
        Offset {
            x: char_pos.x as f32,
            y: char_pos.y as f32
        },
        "キャラ窓の Arrangement.offset が WindowPos に追従する"
    );
    let balloon_pos = position_of(&world, balloon);
    assert_eq!(balloon_pos, Point { x: 845, y: 331 });
    assert_eq!(
        arrangement_offset_of(&world, balloon),
        Offset {
            x: balloon_pos.x as f32,
            y: balloon_pos.y as f32
        },
        "バルーンの Arrangement.offset が WindowPos に追従する（クリック死の檻）"
    );
}

/// (b) move_window_to: 対象キャラ窓・随伴バルーンとも Arrangement.offset が
/// 移動後の WindowPos.position と一致する。
#[test]
fn move_window_to_syncs_arrangement_offset_of_target_and_balloon() {
    let mut world = World::new();
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(0, 0),
            arrangement_at(0.0, 0.0),
        ))
        .id();
    let offset = PointPx { x: -412, y: -25 };
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(50, 60),
            arrangement_at(50.0, 60.0),
            BalloonFollow { balloon, offset },
        ))
        .id();

    assert!(move_window_to(&mut world, window, 907, 1201));

    assert_eq!(
        arrangement_offset_of(&world, window),
        Offset {
            x: 907.0,
            y: 1201.0
        }
    );
    assert_eq!(
        arrangement_offset_of(&world, balloon),
        Offset {
            x: (907 + offset.x) as f32,
            y: (1201 + offset.y) as f32
        }
    );
}

/// (c) move_window_to（BalloonFollow なしの単独窓）: 自身の Arrangement.offset
/// が同期される（バルーン単独移動＝enqueue 共通経路の檻）。
#[test]
fn move_window_to_syncs_arrangement_offset_of_single_window() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(10, 20),
            arrangement_at(10.0, 20.0),
        ))
        .id();

    assert!(move_window_to(&mut world, window, 1531, 883));
    assert_eq!(
        arrangement_offset_of(&world, window),
        Offset {
            x: 1531.0,
            y: 883.0
        }
    );
}

// -------------------------------------------------------------------------
// enqueue_window_set_pos（size 対応一般化・task 2.3・Req1.5/3.3・
// design Testing Strategy > Integration Tests #5）
//
// 既存 move 専用発行口の一般化。`None` は移動専用の後方互換（position のみ
// ミラー・size 不変・SWP_NOSIZE 継続）、`Some` は位置＋寸を一度に反映
// （WindowPos.size も bypass ミラー）。観測境界は `WindowPos.position`／
// `WindowPos.size` のミラー——`SetWindowPosCommand` キューは private TLS で
// flush せず flags/width/height を覗けないため（design Validation の指定）。
// 座標・寸法は 96 の非倍数を使い、隠れた dpi/96 再スケールの檻とする。
// -------------------------------------------------------------------------

use super::enqueue_window_set_pos;

/// `None`（後方互換・移動専用）: position のみ更新し size は触らない
/// （既存移動専用挙動＝SWP_NOSIZE 継続の観測境界）。
#[test]
fn enqueue_window_set_pos_none_updates_position_leaves_size() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x1234), window_pos_sized(10, 20, 434, 687)))
        .id();

    assert!(enqueue_window_set_pos(
        &mut world, window, 1531, 883, None, None
    ));
    assert_eq!(position_of(&world, window), Point { x: 1531, y: 883 });
    // size は不変（移動専用＝寸法を書かない）
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}

/// `Some`: 位置と寸法の**双方**が更新される（WindowPos.size = SizeI::new(w,h)）。
#[test]
fn enqueue_window_set_pos_some_updates_position_and_size() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x1234), window_pos_sized(10, 20, 434, 687)))
        .id();

    assert!(enqueue_window_set_pos(
        &mut world,
        window,
        907,
        1201,
        Some(SizePx { w: 517, h: 823 }),
        None,
    ));
    assert_eq!(position_of(&world, window), Point { x: 907, y: 1201 });
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// 不在/未付与（Req3.3）: `WindowHandle` 無し entity は `false`＋位置/寸法不変
/// （warn no-op・`Some` 経路でも既存 warn 経路を継承）。
#[test]
fn enqueue_window_set_pos_without_handle_returns_false_and_leaves_state() {
    let mut world = World::new();
    let window = world.spawn(window_pos_sized(10, 20, 434, 687)).id();

    assert!(!enqueue_window_set_pos(
        &mut world,
        window,
        907,
        1201,
        Some(SizePx { w: 517, h: 823 }),
        None,
    ));
    assert_eq!(position_of(&world, window), Point { x: 10, y: 20 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}
