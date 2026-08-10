use bevy_ecs::prelude::*;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, WindowPos};

use super::test_support::{
    drag_event, drag_event_at, dragging_state, fake_handle, position_of, single_monitor_snapshot,
    window_pos_at, window_pos_sized,
};
use super::{Anchored, BalloonFollow, move_window_to, on_char_drag};
use crate::placement::resolver::Anchor;
use crate::placement::resolver::PointPx;

// -------------------------------------------------------------------------
// on_balloon_drag: バルーン単独ドラッグの相対位置記憶（task 8.3・4.8・DD16）
// wndproc がバルーン窓を移動済み（WindowPos 更新済み）の状態を模して呼び、
// offset の記憶更新・キャラ窓不動・以後の consumer 追従を決定論検証する。
// 座標は 96 の倍数を避け、x/y で符号・値の異なる offset を使う
// （符号取り違え・軸取り違えの檻）。
// -------------------------------------------------------------------------

use super::on_balloon_drag;

/// Tunnel フェーズは無視する（on_char_drag と同じ規約・offset 不変）。
#[test]
fn on_balloon_drag_tunnel_phase_is_ignored() {
    let mut world = World::new();
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(701, 383)))
        .id();
    let initial = PointPx { x: 11, y: 22 };
    let char_w = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(1207, 653),
            BalloonFollow {
                balloon,
                offset: initial,
            },
        ))
        .id();

    let ev = Phase::Tunnel(drag_event(balloon));
    assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));
    assert_eq!(
        world.get::<BalloonFollow>(char_w).unwrap().offset,
        initial,
        "Tunnel では offset を更新しない"
    );
}

/// (a)(c) バルーン単独ドラッグ: 所有キャラ窓の `BalloonFollow.offset` が
/// `balloon_pos − char_pos` へ更新され（4.8）、キャラ窓は不動。
/// x/y の符号が異なる期待値で減算の向き（balloon − char）を固定する檻。
#[test]
fn on_balloon_drag_updates_offset_and_char_window_is_unmoved() {
    let mut world = World::new();
    // wndproc がドラッグ中に更新した後のバルーン位置を模す
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(1729, 401)))
        .id();
    let char_w = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(1207, 653),
            BalloonFollow {
                balloon,
                offset: PointPx { x: -412, y: -25 },
            },
        ))
        .id();

    let ev = Phase::Bubble(drag_event(balloon));
    // イベントは消費しない（伝播続行＝false）
    assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));

    // offset = balloon − char = (1729−1207, 401−653) = (+522, −252)
    // （char − balloon なら (−522, +252)＝符号取り違えの檻）
    assert_eq!(
        world.get::<BalloonFollow>(char_w).unwrap().offset,
        PointPx { x: 522, y: -252 }
    );
    // (c) キャラ窓は不動（4.8: バルーンのみ移動・bottom 吸着の対象外）
    assert_eq!(position_of(&world, char_w), Point { x: 1207, y: 653 });
    // バルーン自身もハンドラでは動かさない（wndproc の領分）
    assert_eq!(position_of(&world, balloon), Point { x: 1729, y: 401 });
}

/// (b) バルーン単独ドラッグ後の `move_window_to`: 調整後 offset で追従する
/// （初期 offset へのスナップバックは仕様退役・4.8）。
#[test]
fn move_window_to_after_balloon_drag_follows_adjusted_offset() {
    let mut world = World::new();
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(0, 0)))
        .id();
    let initial = PointPx { x: -412, y: -25 };
    let char_w = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(1207, 653),
            BalloonFollow {
                balloon,
                offset: initial,
            },
        ))
        .id();

    // バルーン単独ドラッグ（wndproc がバルーンを (613, 407) へ移動済み）
    world.get_mut::<WindowPos>(balloon).unwrap().position = Some(Point { x: 613, y: 407 });
    let ev = Phase::Bubble(drag_event(balloon));
    assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));

    let adjusted = PointPx {
        x: 613 - 1207,
        y: 407 - 653,
    };
    assert_ne!(adjusted, initial, "檻の前提: 調整後 offset は初期値と異なる");
    assert_eq!(world.get::<BalloonFollow>(char_w).unwrap().offset, adjusted);

    // 次のキャラ窓移動 API は調整後 offset で追従（consumer 無改変・DD16）
    assert!(move_window_to(&mut world, char_w, 1751, 893));
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 1751 + adjusted.x,
            y: 893 + adjusted.y
        }
    );
}

/// (b)(c) 8.2＋8.3 の合成: BottomSnap キャラ窓の場合——バルーンドラッグでは
/// キャラ窓の Y 釘付けは発火せず（不動・4.8「bottom 吸着の対象外」）、
/// その後のキャラ窓ドラッグは Y 釘付けの**後**に調整後 offset で追従する。
#[test]
fn on_char_drag_after_balloon_drag_pins_y_and_follows_adjusted_offset() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(500, 300)))
        .id();
    let initial = PointPx { x: -412, y: -25 };
    // 釘付け済み位置（Y=1043−687=356）から開始する BottomSnap キャラ窓
    let char_w = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687),
            Anchored(Anchor::Bottom),
            BalloonFollow {
                balloon,
                offset: initial,
            },
        ))
        .id();

    // バルーン単独ドラッグ（wndproc がバルーンを (831, 149) へ移動済み）
    world.get_mut::<WindowPos>(balloon).unwrap().position = Some(Point { x: 831, y: 149 });
    let ev = Phase::Bubble(drag_event(balloon));
    assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));

    // キャラ窓は不動（Y 釘付けも発火しない・4.8）
    assert_eq!(position_of(&world, char_w), Point { x: 1207, y: 356 });
    let adjusted = PointPx {
        x: 831 - 1207,
        y: 149 - 356,
    };
    assert_eq!(world.get::<BalloonFollow>(char_w).unwrap().offset, adjusted);

    // 次のキャラ窓ドラッグ（move_window=false 単一ライター・8.2R）:
    // DraggingState を注入し、カーソルが下端から浮く位置まで動いた DragEvent を配送
    let start = (1300, 700);
    world
        .entity_mut(char_w)
        .insert(dragging_state((1207, 356), start));
    let ev = Phase::Bubble(drag_event_at(char_w, start, (996, 555)));
    assert!(!on_char_drag(&mut world, char_w, char_w, &ev));

    // 8.2R: raw=(903, 211) → 適用後 (903, 356)（Y 釘付け・X 素通し）
    assert_eq!(position_of(&world, char_w), Point { x: 903, y: 356 });
    // 8.3: バルーンは釘付け後座標＋**調整後** offset（初期 offset だと不一致）
    assert_eq!(
        position_of(&world, balloon),
        Point {
            x: 903 + adjusted.x,
            y: 356 + adjusted.y
        }
    );
}

/// (d) 複数スコープ: ドラッグしたバルーンを所有するキャラ窓の offset だけが
/// 更新され、他スコープの offset・窓位置は不干渉（誤マッチの檻）。
#[test]
fn on_balloon_drag_updates_only_matching_scope_offset() {
    let mut world = World::new();
    let balloon0 = world
        .spawn((fake_handle(0x2000), window_pos_at(701, 383)))
        .id();
    let char0 = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(1207, 653),
            BalloonFollow {
                balloon: balloon0,
                offset: PointPx { x: -412, y: -25 },
            },
        ))
        .id();
    let balloon1 = world
        .spawn((fake_handle(0x4000), window_pos_at(1334, 1044)))
        .id();
    let offset1 = PointPx { x: 285, y: -19 };
    let char1 = world
        .spawn((
            fake_handle(0x3000),
            window_pos_at(1049, 1063),
            BalloonFollow {
                balloon: balloon1,
                offset: offset1,
            },
        ))
        .id();

    let ev = Phase::Bubble(drag_event(balloon0));
    assert!(!on_balloon_drag(&mut world, balloon0, balloon0, &ev));

    // scope0 の offset は balloon0 − char0 = (−506, −270) へ更新
    assert_eq!(
        world.get::<BalloonFollow>(char0).unwrap().offset,
        PointPx { x: -506, y: -270 }
    );
    // scope1 の offset・窓位置は不変（誤マッチなし）
    assert_eq!(world.get::<BalloonFollow>(char1).unwrap().offset, offset1);
    assert_eq!(position_of(&world, char1), Point { x: 1049, y: 1063 });
    assert_eq!(position_of(&world, balloon1), Point { x: 1334, y: 1044 });
}

/// (+) バルーンの `WindowPos.position` 不在は no-op（false・panic なし・
/// offset 不変）。所有キャラ窓の position 不在も skip で panic しない。
#[test]
fn on_balloon_drag_without_positions_is_graceful() {
    let mut world = World::new();

    // バルーン側 position 不在 → offset 不変
    let mut wp = window_pos_at(0, 0);
    wp.position = None;
    let balloon = world.spawn((fake_handle(0x2000), wp)).id();
    let initial = PointPx { x: 11, y: 22 };
    let char_w = world
        .spawn((
            fake_handle(0x1000),
            window_pos_at(50, 60),
            BalloonFollow {
                balloon,
                offset: initial,
            },
        ))
        .id();
    let ev = Phase::Bubble(drag_event(balloon));
    assert!(!on_balloon_drag(&mut world, balloon, balloon, &ev));
    assert_eq!(world.get::<BalloonFollow>(char_w).unwrap().offset, initial);

    // キャラ側 position 不在 → skip（panic なし・offset 不変）
    let balloon2 = world
        .spawn((fake_handle(0x4000), window_pos_at(70, 80)))
        .id();
    let mut char_wp = window_pos_at(0, 0);
    char_wp.position = None;
    let char2 = world
        .spawn((
            fake_handle(0x3000),
            char_wp,
            BalloonFollow {
                balloon: balloon2,
                offset: initial,
            },
        ))
        .id();
    let ev = Phase::Bubble(drag_event(balloon2));
    assert!(!on_balloon_drag(&mut world, balloon2, balloon2, &ev));
    assert_eq!(world.get::<BalloonFollow>(char2).unwrap().offset, initial);

    // 所有キャラ窓が 1 つも無いバルーン → no-op（panic なし）
    let orphan = world
        .spawn((fake_handle(0x5000), window_pos_at(10, 20)))
        .id();
    let ev = Phase::Bubble(drag_event(orphan));
    assert!(!on_balloon_drag(&mut world, orphan, orphan, &ev));
}
