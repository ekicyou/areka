use crate::placement::follow::OffsetBase;
use bevy_ecs::prelude::*;
use wintf::ecs::SizeI;
use wintf::ecs::layout::Offset;
use wintf::ecs::{Point, WindowPos};

use super::test_support::{
    arrangement_at, arrangement_offset_of, fake_handle, odd_edge_snapshot, position_of, rect,
    single_monitor_snapshot, size_of, window_pos_at, window_pos_sized,
};
use super::{Anchored, BalloonFollow, MonitorSnapshot};
use crate::placement::resolver::{Anchor, PointPx, SizePx};

// -------------------------------------------------------------------------
// resize_window_to（単一ライター反映口・task 2.4・
// Req1.1/1.3/1.7/3.1/3.4＋2.6/3.3・design Integration Tests #1・#4 一部）
//
// 新しい表示寸法へアンカー射影 T を再適用し、確定 position＋size を単一ライター
// 経路で一度だけ書く（bottom は wa.bottom−h' 再計算）。観測境界は headless World
// （偽 HWND）の WindowPos.position／WindowPos.size ミラー——SetWindowPosCommand
// キューは private TLS で flush せず flags/width/height を覗けないため。縮退
// （べき等・非正寸・不在・Anchored 欠落）は false＋状態不変で固定する。座標・
// 寸法は 96 の非倍数を使い、隠れた dpi/96 再スケールの檻とする。
// -------------------------------------------------------------------------

use super::{PlacementRoute, resize_window_to};

/// #1 一度書き＋re-snap（Req1.1/1.3/1.7/2.1）: `Anchored(Bottom)` の char 窓を
/// 新寸へ resize すると、`WindowPos.size` が新寸・`position.y` が `wa.bottom − h'`
/// へ更新され `true`。**原点＝下端中央**ゆえ x は「中央を保つ」よう付け替わる
/// （伺かの立ち絵は足元中央が接地点＝寸法が変わっても原点は動かない）。
/// 下端・寸法とも 96 非倍数で dpi/96 再スケール混入の檻。
#[test]
fn resize_window_to_bottom_resnaps_size_and_position_once() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687), // 旧寸で下端釘付け済み
            Anchored(Anchor::Bottom),
        ))
        .id();

    // 新寸 (517×823・いずれも 96 非倍数): Y=1043−823=220。
    // X は下端中央保持: 旧中央 731+434/2=948 → 新 x = 948−517/2 = 690。
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point {
            x: 690,
            y: 1043 - 823
        },
        "下端中央保持（旧中央 948 を維持）・Y=wa.bottom−h'（bottom 再計算）"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #4 べき等 skip（Req3.1）: 既に射影済み位置＋同寸の窓へ同寸 resize すると、
/// 書込なし・`false`・状態不変（冗長な再配置を避ける）。
#[test]
fn resize_window_to_is_idempotent_on_same_size_and_position() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043・Y=1043−687=356
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687), // 既に bottom 射影済み
            Anchored(Anchor::Bottom),
        ))
        .id();

    // 同寸 → 導出 (731,356)＋(434,687) は現在値と同一 → 書込なし・false
    assert!(!resize_window_to(
        &mut world,
        window,
        SizePx { w: 434, h: 687 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}

/// #4 非正寸縮退（Req3.4）: w≤0 or h≤0 は T 再適用せず `false`・位置/寸不変
/// （warn・`BottomSnapPolicy` の非正寸縮退と整合）。
#[test]
fn resize_window_to_nonpositive_size_holds_state() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
        ))
        .id();

    for bad in [
        SizePx { w: 0, h: 823 },
        SizePx { w: 517, h: 0 },
        SizePx { w: -517, h: -823 },
    ] {
        assert!(
            !resize_window_to(&mut world, window, bad, PlacementRoute::Resnap),
            "{bad:?}: 非正寸は false"
        );
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    }
}

/// #4 不在/未付与（Req3.3）: `WindowHandle` 未付与の char 窓は `false`・状態不変
/// （`enqueue_window_set_pos` の warn no-op を継承・随伴バルーンも動かさない）。
#[test]
fn resize_window_to_without_handle_returns_false_and_leaves_state() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            // WindowHandle なし（窓生成前）
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
        ))
        .id();

    assert!(!resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}

/// #4 Anchored 欠落: 単一真実源 `Anchored` 未付与の窓は `false`・状態不変
/// （char 窓は spawn で必ず付与＝異常系・warn no-op）。
#[test]
fn resize_window_to_without_anchored_returns_false_and_leaves_state() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            // Anchored なし
        ))
        .id();

    assert!(!resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
}

/// #1 随伴バルーン維持（Req2.6）＝**窓相対 offset 不変**（Bottom）:
/// `BalloonFollow` 付き Bottom char 窓を resize しても `BalloonFollow.offset` は
/// 書き換わらず、バルーンは `new_char_pos + offset` へ随伴して恒等式
/// `balloon_pos − char_pos ≡ offset` を保つ。
///
/// キャラ窓自身の原点は下端中央（`char_pos` は中央 x を保って再導出される）が、
/// **バルーンの追従は原点基準ではなく窓（左上）相対**である——受理オラクルは
/// 参照実装 SSP の実測で、SSP のバルーンは観測時つねに現在表示中のキャラ窓に対して
/// 窓相対にある（2026-07-31 実機裁定）。以前の「下端中央基準の offset 補正」は Bottom だけを
/// 窓相対から外し、実機でバルーンを旧絶対位置に置き去りにしていた（本檻はその反転）。
/// 実寸オラクルは `resize_window_to_bottom_keeps_ssp_window_relative_balloon_offset`。
#[test]
fn resize_window_to_bottom_preserves_balloon_follow_offset() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043
    let balloon = world.spawn((fake_handle(0x2000), window_pos_at(0, 0))).id();
    let offset = PointPx { x: -412, y: -25 };
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
            BalloonFollow::new(balloon, OffsetBase::unpinned(offset)),
        ))
        .id();

    // 旧原点（下端中央）: x=731+434/2=948・y=356+687=1043。
    // 旧バルーン絶対位置: (731−412, 356−25)=(319, 331)。
    let old_origin = (731 + 434 / 2, 356 + 687);
    let old_balloon = Point {
        x: 731 + offset.x,
        y: 356 + offset.y,
    };

    // 新寸 (517×823): char は下端中央保持で x=948−517/2=690・y=1043−823=220。
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    let char_pos = position_of(&world, window);
    let balloon_pos = position_of(&world, balloon);
    assert_eq!(
        char_pos,
        Point {
            x: 690,
            y: 1043 - 823
        }
    );

    // キャラ窓の原点（下端中央）は寸法変動で動かない（step 3b の契約・無改変）。
    let new_origin = (char_pos.x + 517 / 2, char_pos.y + 823);
    assert_eq!(
        new_origin, old_origin,
        "原点（下端中央）は寸法変動で動かない"
    );

    // バルーンは**窓相対**: 新 char 左上 + 不変 offset。
    assert_eq!(
        balloon_pos,
        Point {
            x: char_pos.x + offset.x,
            y: char_pos.y + offset.y
        },
        "バルーンは窓（左上）相対 offset で追随する"
    );
    // offset 恒等式（balloon_pos − char_pos ≡ offset）の維持。
    assert_eq!(balloon_pos.x - char_pos.x, offset.x);
    assert_eq!(balloon_pos.y - char_pos.y, offset.y);
    assert_eq!(
        world.get::<BalloonFollow>(window).unwrap().offset(),
        offset,
        "BalloonFollow.offset は resize で補正されない"
    );
    // 旧「下端中央基準」実装は原点不動ゆえバルーン絶対位置も不動にしていた——
    // 窓上端が 136px 上がった本ケースでは窓相対と弁別できる（反転の証明）。
    assert_ne!(
        balloon_pos, old_balloon,
        "下端中央基準補正の復活検出: 窓が動いた以上バルーンも動く"
    );
}

/// SSP オラクル檻（2026-07-31 実機裁定・実 DPI 120／k=1.25 のむらさき実寸）:
/// talk 中のサーフェス切替で Bottom キャラ窓が 543×859 → 478×684（下端 2100 固定）へ
/// 縮んでも、バルーンは**窓相対 offset (−167,−161) を保ったまま**追随する。
///
/// 参照実装 SSP は同時点で char 477×683@(3363,1417)／balloon (3195,1256)＝offset
/// (−168,−161) を保っており、本檻の (−167,−161) とは x が 1px だけ違う。この 1px は
/// サーフェス寸の丸め権威（SSP と areka のスケール丸め）由来であって、追従セマンティクス
/// とは無関係——本変更の受理判定には影響しない。
///
/// 欠陥（削除した step 6＝Bottom 限定の下端中央基準 offset 補正）が残っていると、
/// offset は (−167+(478/2−543/2), −161+(684−859)) = (−199,−336) へ書き換わり、
/// バルーンは旧絶対位置 (3130,1080) に貼り付いたまま新窓上端の 336px 上空へ浮く
/// ——実機で観測された症状そのもの。本檻はその恒久回帰檻。
#[test]
fn resize_window_to_bottom_keeps_ssp_window_relative_balloon_offset() {
    let mut world = World::new();
    // 実機 4K 縦 2100 の work area（下端 2100・むらさきが載っていたモニタ）。
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![rect(2560, 0, 3840, 2100)],
    });
    // boot 直後の実測: char 543×859 @ (3297,1241)／balloon (3130,1080)。
    let offset = PointPx { x: -167, y: -161 };
    let balloon = world
        .spawn((fake_handle(0x2000), window_pos_at(3130, 1080)))
        .id();
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(3297, 1241, 543, 859),
            Anchored(Anchor::Bottom),
            BalloonFollow::new(balloon, OffsetBase::unpinned(offset)),
        ))
        .id();

    // サーフェス切替後の実測寸 478×684 へ resize。
    // route は `dpi-window-vanish` の D11 配管でシグネチャに入った引数。本檻の主題は
    // 追従セマンティクス（窓相対）ゆえ、遷移ガードが発火する配置系 route の代表値
    // （`Resnap`）を渡す——ガードが働く経路でも offset が補正されないことを見る。
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 478, h: 684 },
        PlacementRoute::Resnap
    ));
    let char_pos = position_of(&world, window);
    let balloon_pos = position_of(&world, balloon);

    // char は下端中央原点を保つ: 中央 x = 3297+271 = 3568 → 左上 x = 3568−239 = 3329・
    // y = wa.bottom − h' = 2100−684 = 1416（実機実測 (3329,1416) と一致）。
    assert_eq!(
        char_pos,
        Point { x: 3329, y: 1416 },
        "char は下端中央原点維持（実機実測 (3329,1416)）"
    );
    // バルーンは**窓相対**: 新窓左上 + offset = (3329−167, 1416−161) = (3162,1255)。
    assert_eq!(
        balloon_pos,
        Point {
            x: 3329 - 167,
            y: 1416 - 161
        },
        "バルーンは窓相対 offset で追随する（SSP と同セマンティクス）"
    );
    // 恒等式 balloon_pos − char_pos ≡ offset（resize で補正しない）。
    assert_eq!(balloon_pos.x - char_pos.x, offset.x);
    assert_eq!(balloon_pos.y - char_pos.y, offset.y);
    assert_eq!(
        world.get::<BalloonFollow>(window).unwrap().offset(),
        offset,
        "BalloonFollow.offset は resize で書き換わらない"
    );
    // 欠陥時の値（旧絶対位置に貼り付く）を明示的に排除する。
    assert_ne!(
        balloon_pos,
        Point { x: 3130, y: 1080 },
        "step 6 復活の検出: 旧絶対位置に貼り付いてはならない"
    );
}

// -------------------------------------------------------------------------
// resize_window_to 5 アンカー統合網羅（task 2.5・テスト固定タスク・
// Req1.1/2.1-2.6/3.1/3.3/3.4・design Integration Tests #2・#3・#4）
//
// task 2.4 が Bottom で押さえた「一度書き＋re-snap／べき等／非正寸／不在／
// Anchored 欠落／随伴バルーン維持」を、残る Top/Left/Right/Free へ拡張する。
// resize_window_to 本体は 2.4 で完成済み＝本群は「既存配線が 5 アンカーで
// 正しく `Anchored.0` を転送している（非 Bottom を `Anchor::Bottom` へ
// ハードコードしていない）」ことを固定する回帰檻（非 Bottom 配線バグ＝
// 2.4 エスケープの捕捉）。
//
// 全辺 96 非倍数の odd_edge_snapshot（rect(53,37,1877,1043)）で各アンカー辺の
// 再計算を dpi/96 再スケール混入の檻とし、各アンカーで「固定辺の座標」と
// 「非アンカー軸の保持」を両方 assert する（Top↔Bottom は Y・Left↔Right は X が
// 合わず落ちる取り違え耐性）。
// -------------------------------------------------------------------------

/// #2 Top resize（Req2.2）: `Anchored(Top)` を新寸へ resize すると `WindowPos.size`
/// 新寸・`position.y = wa.top`（上端固定）・`position.x` 保持で `true`。
/// Bottom と取り違えれば Y が `wa.bottom−h'` になって落ちる（辺取り違え耐性）。
#[test]
fn resize_window_to_top_pins_top_edge_and_keeps_x() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Top),
        ))
        .id();

    // 新寸 (517×823・いずれも 96 非倍数): Y=wa.top=37・X=731 保持
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 731, y: 37 },
        "X 保持・Y=wa.top（上端固定・Bottom と取り違えたら 1043−823 で落ちる）"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #2 Left resize（Req2.3）: `Anchored(Left)` を新寸へ resize すると `WindowPos.size`
/// 新寸・`position.x = wa.left`（左端固定）・`position.y` 保持で `true`。
/// Right と取り違えれば X が `wa.right−w'` になって落ちる（辺取り違え耐性）。
#[test]
fn resize_window_to_left_pins_left_edge_and_keeps_y() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Left),
        ))
        .id();

    // 新寸 (517×823): X=wa.left=53・Y=500 保持
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 53, y: 500 },
        "X=wa.left（左端固定・Right と取り違えたら 1877−517 で落ちる）・Y 保持"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #2 Right resize（Req2.4）: `Anchored(Right)` を新寸へ resize すると `WindowPos.size`
/// 新寸・`position.x = wa.right − w'`（右端固定）・`position.y` 保持で `true`。
/// Left と取り違えれば X が `wa.left` になって落ちる（辺取り違え耐性）。
#[test]
fn resize_window_to_right_pins_right_edge_and_keeps_y() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Right),
        ))
        .id();

    // 新寸 (517×823): X = wa.right − w' = 1877 − 517 = 1360・Y=500 保持
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point {
            x: 1877 - 517,
            y: 500
        },
        "X=wa.right−w'（右端固定・Left と取り違えたら 53 で落ちる）・Y 保持"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #2 Free resize（Req2.5）: `Anchored(Free)` はアンカー辺を持たず position を
/// 保持し、`WindowPos.size` のみ新寸へ反映する。size が変わるので冗長でなく
/// `true`（書込あり）。Bottom へ取り違えれば position.y が動いて落ちる
/// （射影なし・寸法反映のみの区別）。
#[test]
fn resize_window_to_free_keeps_position_and_updates_size_only() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Free),
        ))
        .id();

    // Free: 射影なし＝position 不変・size のみ新寸（size 変化ゆえ冗長でなく true）
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 731, y: 500 },
        "Free は position 再計算なし（現在位置保持・Bottom 取り違えなら Y が動く）"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #3 随伴バルーン維持（Left・Req2.6）: `Anchored(Left)`＋`BalloonFollow` の
/// char 窓を resize すると、char は左端固定（Y 保持）へ移り、バルーンは
/// `new_char_pos + offset` へ随伴し `balloon_pos − char_pos ≡ offset` を維持する。
///
/// 本檻はかつて「非 Bottom だけの例外」を主張していたが、2026-07-31 実機裁定で
/// Bottom の下端中央基準補正が撤去され、窓相対追従が**全アンカー共通の規範**になった
/// ——Bottom 版は `resize_window_to_bottom_preserves_balloon_follow_offset`。
/// 本檻はその規範をアンカー辺 x 固定（Left）側で固定する。
#[test]
fn resize_window_to_left_preserves_balloon_follow_offset() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // 左端 53
    let balloon = world.spawn((fake_handle(0x2000), window_pos_at(0, 0))).id();
    let offset = PointPx { x: -412, y: -25 };
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Left),
            BalloonFollow::new(balloon, OffsetBase::unpinned(offset)),
        ))
        .id();

    // 新寸 (517×823) → char 左端固定 (53, 500)・balloon (53−412, 500−25)
    assert!(resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    let char_pos = position_of(&world, window);
    let balloon_pos = position_of(&world, balloon);
    assert_eq!(char_pos, Point { x: 53, y: 500 }, "左端固定・Y 保持");
    assert_eq!(
        balloon_pos,
        Point {
            x: 53 + offset.x,
            y: 500 + offset.y
        }
    );
    // offset 恒等式（balloon_pos − char_pos ≡ offset）の維持
    assert_eq!(balloon_pos.x - char_pos.x, offset.x);
    assert_eq!(balloon_pos.y - char_pos.y, offset.y);
}

/// #4 べき等（非 Bottom・Req3.1）: 既に左端一致（x=wa.left）の位置＋同寸へ
/// `Anchored(Left)` を resize すると、導出 (position, size) が現在値と同一ゆえ
/// 書込なし・`false`・状態不変（Bottom 版 idempotent の非 Bottom 対応・
/// 同一寸法/同一アンカーの再適用が窓状態を変更しない＝冗長書込をしない）。
#[test]
fn resize_window_to_left_is_idempotent_on_same_size_and_position() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // 左端 53
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(53, 500, 517, 823), // 既に左端射影済み・同寸
            Anchored(Anchor::Left),
        ))
        .id();

    // 同寸・既に左端一致 → 導出 (53,500)＋(517,823) は現在値と同一 → 書込なし・false
    assert!(!resize_window_to(
        &mut world,
        window,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, window), Point { x: 53, y: 500 });
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// #4 非 Bottom 縮退（Req3.3/3.4）: 縮退経路がアンカー非依存（Bottom 特化でない）
/// ことを代表として Top で固定する。task 2.4 が Bottom で押さえた縮退を、
/// 別アンカーでも配線が同一であることの確認（過剰重複を避け 1 件へ集約）。
/// - 非正寸（w≤0 or h≤0）: project_anchor 前に弾かれ `false`・位置/寸不変。
/// - `WindowHandle` 未付与: 射影は走るが enqueue が warn no-op＝`false`・位置/寸不変。
#[test]
fn resize_window_to_non_bottom_degrades_on_nonpositive_and_missing_handle() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot());

    // (a) Top＋非正寸: project_anchor 前に弾かれ false・状態不変（Bottom と同一縮退）
    let with_handle = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Top),
        ))
        .id();
    for bad in [
        SizePx { w: 0, h: 823 },
        SizePx { w: 517, h: 0 },
        SizePx { w: -517, h: -823 },
    ] {
        assert!(
            !resize_window_to(&mut world, with_handle, bad, PlacementRoute::Resnap),
            "{bad:?}: 非正寸は false（Top でも Bottom と同一縮退）"
        );
        assert_eq!(position_of(&world, with_handle), Point { x: 731, y: 500 });
        assert_eq!(size_of(&world, with_handle), SizeI::new(434, 687));
    }

    // (b) Top＋WindowHandle 未付与: 射影は走るが enqueue が warn no-op＝false・状態不変
    let no_handle = world
        .spawn((
            // WindowHandle なし（窓生成前）
            window_pos_sized(731, 500, 434, 687),
            Anchored(Anchor::Top),
        ))
        .id();
    assert!(!resize_window_to(
        &mut world,
        no_handle,
        SizePx { w: 517, h: 823 },
        PlacementRoute::Resnap
    ));
    assert_eq!(position_of(&world, no_handle), Point { x: 731, y: 500 });
    assert_eq!(size_of(&world, no_handle), SizeI::new(434, 687));
}

// -------------------------------------------------------------------------
// anchor_changed_system（アンカー変化トリガ・task 2.6・Req1.4・
// design「Anchored（Component）/ anchor_changed_system」「System Flows >
// アンカー変化トリガ」「File Structure Plan > follow.rs」）
//
// producer（seriko の `\![set,alignmenttodesktop]` routing）は本 spec 非所有＝
// 本群は `Changed<Anchored>` に反応する **consumer** のみを固定し、テストは
// `Anchored` を直接書き換えて駆動する。change tick を正しく管理するため system は
// `Schedule` に登録して run し（同一 Schedule インスタンスを使い回すことで
// 永続 `SystemState` の `last_run` を run 跨ぎで効かせる）、初回 run の全マッチは
// resize_window_to のべき等 skip で吸収する。全辺 96 非倍数の odd_edge_snapshot
// （rect(53,37,1877,1043)）で dpi/96 再スケール混入の檻とする。
// -------------------------------------------------------------------------

use super::anchor_changed_system;

/// #1 アンカー変化で再射影（Req1.4 の核）: `Anchored(Bottom)` の釘付け済み char 窓を
/// spawn し、初回 run はべき等 skip（初回 Changed 付与を resize が同寸・同位置で吸収
/// ＝位置不変）。次に `Anchored` を Top へ**直接書換**→再 run で「現在の表示寸法の
/// まま」新アンカー辺（y=wa.top）へ再配置され、X 保持・size 不変（新寸を与えない
/// ので size は変わらない）。
#[test]
fn anchor_changed_system_reprojects_to_new_anchor_edge_at_current_size() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    // Bottom 釘付け済み: y = wa.bottom − h = 1043 − 687 = 356・x=731（96 非倍数）
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(anchor_changed_system);

    // 初回 run: 初回 Changed 付与で発火し得るが、Bottom は現寸で y=356 のまま
    // ＝べき等 skip で吸収（位置・寸法不変）。
    schedule.run(&mut world);
    assert_eq!(
        position_of(&world, e),
        Point { x: 731, y: 356 },
        "初回 run はべき等 skip（位置不変）"
    );
    assert_eq!(
        size_of(&world, e),
        SizeI::new(434, 687),
        "初回 run: size 不変"
    );

    // Anchored を Top へ直接書換（producer=seriko の代替＝consumer 駆動の檻）。
    world.get_mut::<Anchored>(e).unwrap().0 = Anchor::Top;

    // 再 run: 現在の表示寸法(434×687)のまま新アンカー辺 y=wa.top=37 へ再射影。
    schedule.run(&mut world);
    assert_eq!(
        position_of(&world, e),
        Point { x: 731, y: 37 },
        "新アンカー辺 y=wa.top へ再配置・X=731 保持（Bottom のままなら y=356 で落ちる）"
    );
    assert_eq!(
        size_of(&world, e),
        SizeI::new(434, 687),
        "現在の表示寸法のまま（新寸を与えないので size は不変）"
    );
}

/// #2 Anchored 未変化では発火しない（変更検知の正しさの檻・最重要）: 初回 run で
/// 初回 Changed を消費した後、`Anchored` を触らずに `WindowPos.position` を故意に
/// アンカー辺から外して再 run しても**再スナップされない**（system は `Anchored`
/// 変化にのみ反応し `WindowPos` 変化には反応しない）。毎 run 全マッチ実装
/// （fresh QueryState の last_run=0）ならここで y=356 へ戻り落ちる。
#[test]
fn anchor_changed_system_does_not_fire_when_anchor_unchanged() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // 下端 1043
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687), // Bottom 釘付け済み（y=1043−687）
            Anchored(Anchor::Bottom),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(anchor_changed_system);

    // 初回 run で初回 Changed<Anchored> を消費（べき等 skip・位置不変）。
    schedule.run(&mut world);
    assert_eq!(position_of(&world, e), Point { x: 731, y: 356 });

    // Anchored は触らず、WindowPos.position をアンカー辺から外れた位置へ手動移動。
    world.get_mut::<WindowPos>(e).unwrap().position = Some(Point { x: 731, y: 900 });

    // 再 run: Anchored 未変化ゆえ Changed にマッチせず再スナップしない。
    schedule.run(&mut world);
    assert_eq!(
        position_of(&world, e),
        Point { x: 731, y: 900 },
        "Anchored 未変化では再スナップしない（毎 run 全マッチ実装ならここで y=356 へ戻り落ちる）"
    );
}

/// #3 別遷移（Bottom→Left）: `Anchored` を Left へ直接書換すると、現在の表示寸法の
/// まま左端固定（x=wa.left=53）へ再射影され Y 保持（Top 以外の辺でも配線が
/// `Anchored.0` を正しく転送していることの補強）。
#[test]
fn anchor_changed_system_reprojects_bottom_to_left() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // 左端 53・下端 1043
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687), // Bottom 釘付け済み
            Anchored(Anchor::Bottom),
        ))
        .id();

    let mut schedule = Schedule::default();
    schedule.add_systems(anchor_changed_system);
    schedule.run(&mut world); // 初回 Changed 消費（べき等・位置不変）
    assert_eq!(position_of(&world, e), Point { x: 731, y: 356 });

    world.get_mut::<Anchored>(e).unwrap().0 = Anchor::Left;
    schedule.run(&mut world);
    // Left: x=wa.left=53・Y=356 保持・size 不変
    assert_eq!(
        position_of(&world, e),
        Point { x: 53, y: 356 },
        "x=wa.left=53（左端固定）・Y=356 保持"
    );
    assert_eq!(size_of(&world, e), SizeI::new(434, 687));
}

// -------------------------------------------------------------------------
// resize_window_keep_position（balloon 窓の位置維持リサイズ・
// areka-P0-emo-dpi-scaling task 2.2・R3.1/R4.2・
// design「areka / placement > follow.rs（additive・balloon 窓の k 追従）」・D8）
//
// 「書込ゼロ」の観測境界について: `SetWindowPosCommand` の TLS キューは
// wintf 私有（`WINDOW_POS_COMMANDS`）で件数を覗く公開 API が無く、`flush()` は
// 偽 HWND に対し実 `SetWindowPos` を撃ってしまうため使えない（既存
// enqueue_window_set_pos 群と同じ制約）。代わりに **`Arrangement.offset` 同期**
// を witness に使う——この同期は `enqueue_window_set_pos` 内で enqueue と
// 不可分に対で走るため、「stale な sentinel offset が据え置かれたまま」＝
// 単一ライター経路を一度も通っていない＝enqueue 件数 0 の決定論的証拠になる
// （逆に通れば offset は必ず `WindowPos.position` の `as f32` 転写になる）。
// 寸法・座標は 96 の非倍数を使い、隠れた dpi/96 再スケールの檻とする。
// -------------------------------------------------------------------------

use super::resize_window_keep_position;

/// 単一ライター経路を通ったか否かの witness 用 sentinel（実位置と重ならない値）。
const WRITER_WITNESS: Offset = Offset { x: -1.0, y: -1.0 };

/// 経路を通っていない＝書込ゼロ（sentinel が据え置かれている）。
fn assert_no_write(world: &World, entity: Entity) {
    assert_eq!(
        arrangement_offset_of(world, entity),
        WRITER_WITNESS,
        "単一ライター経路を通った痕跡がある（書込ゼロのはず）"
    );
}

/// べき等 skip（R4.2・D8「同寸なら書込ゼロで振動しない」）: 現寸と同じ寸を
/// 渡すと単一ライター経路を**一度も通らず** `false` を返し、位置・寸法とも不変。
#[test]
fn resize_window_keep_position_same_size_writes_nothing() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            window_pos_sized(731, 356, 434, 687),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(
        !resize_window_keep_position(&mut world, window, SizePx { w: 434, h: 687 }),
        "同寸はべき等 skip ゆえ false"
    );
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    assert_no_write(&world, window);
}

/// 異寸（R3.1/R4.2）: 位置は**現在位置のまま**・寸法だけが新寸へ更新され `true`。
/// `resize_window_to` と違いアンカー射影 T を再適用しない（balloon は char 窓
/// 追従で位置が決まるため、DPI 追従では寸だけを差し替える）。
#[test]
fn resize_window_keep_position_new_size_keeps_position_and_writes_once() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            window_pos_sized(731, 356, 434, 687),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert_eq!(
        position_of(&world, window),
        Point { x: 731, y: 356 },
        "位置は維持される（再射影しない）"
    );
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
    // 単一ライター経路を通った証拠＝Arrangement.offset が現在位置の as f32 転写
    assert_eq!(
        arrangement_offset_of(&world, window),
        Offset { x: 731.0, y: 356.0 }
    );
}

/// 現寸不明（`WindowPos.size` が `None`＝窓生成直後）はべき等判定が成立しない
/// ため書込へ進む（位置維持・新寸反映）。
#[test]
fn resize_window_keep_position_with_unknown_current_size_writes() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            window_pos_at(731, 356),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(517, 823));
}

/// `WindowPos` 未付与（窓生成前の異常系）: warn＋`false`＋書込ゼロ
/// （silent no-op にしない）。
#[test]
fn resize_window_keep_position_without_window_pos_returns_false() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(!resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert_no_write(&world, window);
}

/// `WindowPos.position` 不在（窓生成前）: 現在位置を読めないため warn＋`false`＋
/// 書込ゼロ。`size` も書き換えない。
#[test]
fn resize_window_keep_position_without_position_returns_false() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            WindowPos {
                position: None,
                size: Some(SizeI::new(434, 687)),
                ..Default::default()
            },
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(!resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert!(
        world
            .get::<WindowPos>(window)
            .expect("WindowPos があるはず")
            .position
            .is_none(),
        "position は復活しない"
    );
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    assert_no_write(&world, window);
}

/// 非正寸（0・負）: warn＋`false`＋書込ゼロ（`resize_window_to` の非正寸縮退と
/// 同一流儀・`wa.right−w` 系の暴走を先に弾く）。
#[test]
fn resize_window_keep_position_nonpositive_size_holds_state() {
    for bad in [
        SizePx { w: 0, h: 687 },
        SizePx { w: 434, h: 0 },
        SizePx { w: 0, h: 0 },
        SizePx { w: -517, h: 823 },
        SizePx { w: 517, h: -823 },
    ] {
        let mut world = World::new();
        let window = world
            .spawn((
                fake_handle(0x3000),
                window_pos_sized(731, 356, 434, 687),
                arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
            ))
            .id();

        assert!(
            !resize_window_keep_position(&mut world, window, bad),
            "非正寸 {bad:?} は false"
        );
        assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
        assert_eq!(size_of(&world, window), SizeI::new(434, 687));
        assert_no_write(&world, window);
    }
}

/// `WindowHandle` 未付与（窓生成前）: 判定を二重化せず `enqueue_window_set_pos`
/// の既存 warn 経路へ委譲し `false`＋状態不変（単一ライター規律の継承）。
#[test]
fn resize_window_keep_position_without_handle_returns_false() {
    let mut world = World::new();
    let window = world
        .spawn((
            window_pos_sized(731, 356, 434, 687),
            arrangement_at(WRITER_WITNESS.x, WRITER_WITNESS.y),
        ))
        .id();

    assert!(!resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
    assert_eq!(position_of(&world, window), Point { x: 731, y: 356 });
    assert_eq!(size_of(&world, window), SizeI::new(434, 687));
    assert_no_write(&world, window);
}

/// despawn 済み（対象不在）でも panic せず `false`。
#[test]
fn resize_window_keep_position_on_despawned_entity_returns_false() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x3000), window_pos_sized(731, 356, 434, 687)))
        .id();
    world.despawn(window);

    assert!(!resize_window_keep_position(
        &mut world,
        window,
        SizePx { w: 517, h: 823 }
    ));
}
