use crate::placement::follow::OffsetBase;
use bevy_ecs::prelude::*;
use wintf::ecs::SizeI;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{WindowHandle, WindowPos};

use super::super::diag::DESPAWNED_SKIP_TAG;
use super::super::test_support::{ExpectField, LogEvent, capture_logs, expect_one};
use super::test_support::{
    CLAMP_TAG, DPIS, GUARD_TAG_PREFIX, NEAREST_TAG, UNRESOLVED_TAG, balloon_size, char_size,
    drag_end_event_at, drag_event_at, dragging_state, fake_handle, gap_center_x, grounded_y,
    guard_events, left_wa, mixed_layout, narrow_char_size, point, point_of, px, right_wa,
    unguarded_projection, visible_in, wide_char_size, window_pos_sized,
};
use super::{
    Anchored, BalloonFollow, MonitorSnapshot, PlacementRoute, on_char_drag, on_char_drag_end,
    resize_window_to, route_applies_visibility_guard,
};
use crate::placement::resolver::{Anchor, PointPx, SizePx};

// -------------------------------------------------------------------------
// バルーン矩形への遷移ガード配線（task 6.2・S3′ 是正・Req 3.4・D6）
//
// task 2.2 は `guard_visibility` のバルーン矩形ケース（純関数）を、task 6.1 は
// キャラ窓経路の配線を固めた。本節が檻に入れるのは**バルーン随伴で実際に走るか・
// どの引き金で走るか**である（diagnosis-report.md §1.4「純関数が在ることは S3′ の
// 充足ではない」）。
//
// 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
//   (1) 探針の自己検査——ガード**無し**のバルーン提案が本当に全 work area 非交差で
//       あること／旧バルーン矩形は可視であること（どちらかが崩れると ClampX 腕へ
//       入らず「緑」が何も意味しない・[[2.2 の教訓]]）
//   (2) **キャラ窓は clamp されない**こと——キャラ側のガードが動かした結果を
//       バルーンの成果と読み違えない（S3 と S3′ の分離）
//   (3) 引き金による発火条件——**ドラッグ随伴では位置が素の恒等式と 1 bit も違わない**。
//       ログ側の否定 assert だけに依存しない（[[5.2 の教訓＝空虚性 6 例目]]:
//       不変量がログ側にしか無いと別ファイルの水準変更で守りが消える）
//   (4) 判定語のリテラル——`CLAMP_TAG`／`NEAREST_TAG`／`UNRESOLVED_TAG` を檻側にも持つ
//
// 座標はすべて論理値 × DPI（96/120/192）で構築し、絶対 px の固定値を持たない（Req 5.6）。
// -------------------------------------------------------------------------

use super::BalloonFollowTrigger;

/// キャラ窓の初期位置（**接地していない** Y）。同寸の [`resize_window_to`] でも
/// 射影 T が Y を `wa.bottom − h` へ動かす＝手順 4 のべき等 skip に落ちない。
fn char_start_pos(dpi: i32) -> PointPx {
    point(px(1500, dpi), px(100, dpi))
}

/// 射影 T 適用後のキャラ窓確定位置（右モニタへ接地・**可視のまま**）。
fn char_settled_pos(dpi: i32) -> PointPx {
    point(px(1500, dpi), grounded_y(right_wa(dpi), char_size(dpi)))
}

/// 全 work area の外を指す追従 offset（キャラの右上へ px(500)／−px(400)）。
///
/// キャラ窓（右端 `px(1800)`）は右モニタ内に留まる一方、バルーン（幅 `px(500)`）は
/// `px(2000)` 以降＝`right_wa.right = px(1920)` の外側へ丸ごと出る。左モニタは負座標
/// ゆえ交差し得ない＝**バルーンだけが完全不可視**になる S3′ の合成そのもの。
fn far_out_offset(dpi: i32) -> PointPx {
    point(px(500, dpi), -px(400, dpi))
}

/// 旧バルーン位置（右モニタ内＝**可視**。ゆえに「可視→不可視の遷移」になる）。
fn visible_balloon_pos(dpi: i32) -> PointPx {
    point(px(800, dpi), px(240, dpi))
}

/// 「キャラ窓は可視のまま・offset 恒等式の提案位置だけが全 work area 非交差」へ
/// 落ちる合成 World を組む（S3′＝*キャラは見えているのに会話が読めない*）。
fn char_with_far_balloon_world(
    dpi: i32,
    balloon_pos: PointPx,
    offset: PointPx,
) -> (World, Entity, Entity) {
    let c = char_size(dpi);
    let b = balloon_size(dpi);
    let start = char_start_pos(dpi);
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_sized(balloon_pos.x, balloon_pos.y, b.w, b.h),
        ))
        .id();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(start.x, start.y, c.w, c.h),
            Anchored(Anchor::Bottom),
            BalloonFollow::new(balloon, OffsetBase::unpinned(offset)),
        ))
        .id();
    (world, char_window, balloon)
}

/// 引き金の表（D13 帰結⑴⑵ の**キャラ窓と同一の表**）を固定する。
///
/// バルーンは別規則を持たない——違うのは「何を入力に引くか」だけで、引くのは
/// キャラ窓と同じ [`route_applies_visibility_guard`] である。ドラッグ腕が真へ倒れる
/// 変異（＝明示操作の尊重の破壊）は挙動檻
/// [`balloon_drag_trigger_neither_clamps_nor_warns`] が第一の守りとして捕まえる。
#[test]
fn balloon_follow_trigger_table_mirrors_the_char_window_table() {
    assert!(
        !BalloonFollowTrigger::Drag.applies_visibility_guard(),
        "ドラッグ随伴でガードが発火する（明示操作の尊重が壊れている・Req 3.1）"
    );
    for route in PlacementRoute::ALL {
        assert_eq!(
            BalloonFollowTrigger::Placement(route).applies_visibility_guard(),
            route_applies_visibility_guard(route),
            "route={route} の引き金判定がキャラ窓の表と食い違う"
        );
    }
    // 表が「全部真」「全部偽」へ潰れていないこと（自明な述語への退化の検出）。
    let fired = PlacementRoute::ALL
        .into_iter()
        .filter(|r| BalloonFollowTrigger::Placement(*r).applies_visibility_guard())
        .count();
    assert_eq!(fired, 6, "発火する引き金が 6 種でない（表が潰れている）");
}

/// **Req 3.4 の本体**: 非ドラッグの配置系 4 経路が引き金のとき、offset 恒等式が出した
/// バルーン提案位置が全 work area 非交差へ落ちるなら、X の clamp で救われる。
///
/// キャラ窓は終始可視（clamp されない）＝救われたのは**バルーンだけ**である。
#[test]
fn balloon_visibility_guard_clamps_x_on_non_drag_placement_triggers() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        let old_pos = visible_balloon_pos(dpi);
        for route in [
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::ReportedSizeReconcile,
        ] {
            let (mut world, char_window, balloon) =
                char_with_far_balloon_world(dpi, old_pos, offset);

            // (1) 探針の自己検査: 恒等式の素の提案は**本当に**全 work area 非交差／
            //     旧バルーン矩形は可視。どちらかが崩れると ClampX 腕へ入らず空虚になる。
            let settled = char_settled_pos(dpi);
            let bare = point(settled.x + offset.x, settled.y + offset.y);
            assert!(
                !visible_in(&layout, bare, b_size),
                "dpi={dpi}: 探針が不動点——素のバルーン提案 {bare:?} が既に可視"
            );
            assert!(
                visible_in(&layout, old_pos, b_size),
                "dpi={dpi}: 旧バルーンが非交差では『遷移』でなく留置＝Keep が正解になる"
            );

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, char_window, char_size(dpi), route));
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            // (2) キャラ窓は clamp されていない＝救われたのはバルーンだけである。
            assert_eq!(
                point_of(&world, char_window),
                settled,
                "dpi={dpi} route={route}: キャラ窓が動いた＝S3′ ではなく S3 の檻になっている"
            );

            // Req 3.4: 書かれたバルーン矩形はいずれかの work area と交差する。
            let pos = point_of(&world, balloon);
            assert!(
                visible_in(&layout, pos, b_size),
                "dpi={dpi} route={route}: Req 3.4 違反——バルーン {pos:?} が全 work area と非交差"
            );
            assert_eq!(
                pos.y, bare.y,
                "dpi={dpi} route={route}: バルーンの Y は恒等式の所有＝ガードが触ってはならない"
            );
            assert_ne!(
                pos.x, bare.x,
                "dpi={dpi} route={route}: バルーンの X が引き戻されていない（ガード未発火）"
            );
            let wa = right_wa(dpi);
            assert!(
                wa.left <= pos.x && pos.x <= wa.right - b_size.w,
                "dpi={dpi} route={route}: clamp 先が work area {wa:?} の外: {pos:?}"
            );

            // (4) 判定語: ClampX の warn が 1 行・水準は WARN（縮退シームの記録）。
            let clamped = expect_one(&events, CLAMP_TAG);
            assert_eq!(
                clamped.level,
                tracing::Level::WARN,
                "dpi={dpi} route={route}: バルーンの clamp が warn 水準でない"
            );
            // 提案位置の中心はどの work area にも属さない＝食い違いの兆候も 1 行残る。
            assert_eq!(
                expect_one(&events, NEAREST_TAG).level,
                tracing::Level::WARN,
                "dpi={dpi} route={route}: 最近傍フォールバックが warn へ昇格していない"
            );
        }
    }
}

/// **Req 3.1 の裏面**: 明示操作系・非配置系の引き金では、バルーン位置が素の offset
/// 恒等式と 1 bit も違わず、ガードのログも 1 行も出ない。
#[test]
fn balloon_visibility_guard_does_not_fire_on_explicit_or_non_placement_triggers() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        let old_pos = visible_balloon_pos(dpi);
        for route in [
            PlacementRoute::SpawnInitial,
            PlacementRoute::Restore,
            PlacementRoute::KeepPositionResize,
            PlacementRoute::BalloonFollow,
            PlacementRoute::MoveCue,
        ] {
            let (mut world, char_window, balloon) =
                char_with_far_balloon_world(dpi, old_pos, offset);
            let settled = char_settled_pos(dpi);
            let bare = point(settled.x + offset.x, settled.y + offset.y);
            // 探針の自己検査: 発火条件は揃っている（引き金だけが違う）。
            assert!(
                !visible_in(&layout, bare, b_size),
                "dpi={dpi}: 探針が不動点——発火条件が揃っていない"
            );

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, char_window, char_size(dpi), route));
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            assert_eq!(
                point_of(&world, balloon),
                bare,
                "dpi={dpi} route={route}: 適用外の引き金でバルーンが動いた（明示操作の尊重が壊れている）"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} route={route}: 適用外の引き金でガードが喋っている: {events:?}"
            );
        }
    }
}

/// **本タスクの中核の守り（[[6.1 → 6.2 の申し送り]]）**: ドラッグ随伴では発火しない。
///
/// `follow_balloon` は配置系（[`resize_window_to`]）とドラッグ
/// （[`on_char_drag`]／[`on_char_drag_end`]）の**双方**から呼ばれる。無条件適用すると
/// ユーザーがキャラを画面端へ運んだときにバルーンだけが引き戻され、Req 3.1 の
/// 「明示操作の尊重」が壊れる——その変異を**位置 assert**で捕まえる（ログ側の否定
/// assert だけに依存しない・[[5.2 の教訓]]）。
///
/// # 前提（windowposition-limit 7.3・要件 2.5／5.4）
///
/// 本檻が固定するのは「**ドラッグ中は無介入**」であって「バルーンは決してクランプ
/// されない」ではない。`windowposition.limit` が有効なバルーンでは、ドラッグ**解放時**
/// に作業領域内への補正が入る——その檻は `follow_drag_end_limit_tests.rs`
/// （`on_balloon_drag`／`on_balloon_drag_end`・route `BalloonLimitRelease`）が所有し、
/// 本檻は所有しない。ここで補正が起きないのは、[`char_with_far_balloon_world`] の
/// バルーン entity に `BalloonLimit` を付けていないためである（`enqueue_window_set_pos`
/// の runtime 関門は `BalloonLimit(true)` を持つ窓だけに作用する・DD1 のデータ駆動）。
#[test]
fn balloon_drag_trigger_neither_clamps_nor_warns() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let c_size = char_size(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        let old_pos = visible_balloon_pos(dpi);
        let start = char_start_pos(dpi);
        let cursor = (px(800, dpi), px(400, dpi));
        // カーソルを右へ px(100) 動かす＝生ドラッグ x は px(1600)。
        let moved = (cursor.0 + px(100, dpi), cursor.1);
        // 射影 T 適用後のキャラ確定位置（下端接地・X は素通し）。
        let settled = point(px(1600, dpi), grounded_y(right_wa(dpi), c_size));
        let bare = point(settled.x + offset.x, settled.y + offset.y);

        // 探針の自己検査: ドラッグ随伴の提案は**本当に**全 work area 非交差
        //（＝ガードが配線されていれば必ず clamp する状況である）。旧矩形は可視。
        assert!(
            !visible_in(&layout, bare, b_size),
            "dpi={dpi}: 探針が不動点——ドラッグ随伴の提案 {bare:?} が可視のまま"
        );
        assert!(
            visible_in(&layout, old_pos, b_size),
            "dpi={dpi}: 旧バルーンが非交差では『留置の尊重』と区別が付かない"
        );

        for entry in ["on_char_drag", "on_char_drag_end"] {
            let (mut world, char_window, balloon) =
                char_with_far_balloon_world(dpi, old_pos, offset);
            world
                .entity_mut(char_window)
                .insert(dragging_state((start.x, start.y), cursor));

            let (_, events) = capture_logs(|| match entry {
                "on_char_drag" => {
                    let ev = Phase::Bubble(drag_event_at(char_window, cursor, moved));
                    on_char_drag(&mut world, char_window, char_window, &ev)
                }
                _ => {
                    let ev = Phase::Bubble(drag_end_event_at(char_window, moved));
                    on_char_drag_end(&mut world, char_window, char_window, &ev)
                }
            });

            assert_eq!(
                point_of(&world, char_window),
                settled,
                "dpi={dpi} {entry}: 前提——ドラッグの確定位置が想定と違う"
            );
            assert_eq!(
                point_of(&world, balloon),
                bare,
                "dpi={dpi} {entry}: ドラッグ随伴でバルーンが引き戻された（Req 3.1 違反）"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} {entry}: ドラッグ随伴でガードが喋っている（spam・水準分岐の破壊）: {events:?}"
            );
        }
    }
}

/// ユーザーが画面外へ留置したバルーンは、配置系の引き金でも引き戻さない
/// （キャラ窓と完全に同一の規則＝`Keep` 腕・Req 3.1 の「明示操作の尊重」）。
#[test]
fn balloon_parked_off_screen_is_respected_on_placement_trigger() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        // 旧バルーンは既に全 work area の外（ユーザー留置）。
        let parked = point(px(2400, dpi), px(240, dpi));
        assert!(
            !visible_in(&layout, parked, b_size),
            "dpi={dpi}: 前提——旧バルーンは既に非交差（留置）"
        );

        let (mut world, char_window, balloon) = char_with_far_balloon_world(dpi, parked, offset);
        let settled = char_settled_pos(dpi);
        let bare = point(settled.x + offset.x, settled.y + offset.y);
        assert!(
            !visible_in(&layout, bare, b_size),
            "dpi={dpi}: 前提——提案も非交差（`Keep` 腕を通る条件）"
        );

        let (ok, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                char_size(dpi),
                PlacementRoute::DpiReproject,
            )
        });
        assert!(ok);
        assert_eq!(
            point_of(&world, balloon),
            bare,
            "dpi={dpi}: 留置バルーンが引き戻された（Keep 腕が効いていない）"
        );
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: 留置バルーンに ClampX が出ている: {events:?}"
        );
    }
}

/// 任意の `WindowPos` を持つバルーンで [`char_with_far_balloon_world`] 相当を組む
/// （未確定表現の探針用）。
fn char_with_balloon_window_pos(
    dpi: i32,
    balloon_pos: WindowPos,
    offset: PointPx,
) -> (World, Entity, Entity) {
    let c = char_size(dpi);
    let start = char_start_pos(dpi);
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let balloon = world.spawn((fake_handle(0x2000), balloon_pos)).id();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(start.x, start.y, c.w, c.h),
            Anchored(Anchor::Bottom),
            BalloonFollow::new(balloon, OffsetBase::unpinned(offset)),
        ))
        .id();
    (world, char_window, balloon)
}

/// **バルーン寸の未確定は `Option::None` だけではない**（[[4.6 の教訓]]・6.1 の
/// `old_rect` 導出と同型の罠）: `WindowPos::default()` は position・size の**両方**を
/// `CW_USEDEFAULT`（`i32::MIN` センチネル）で持つ。
///
/// センチネルを素の矩形として交差判定へ入れると `saturating_add` で逆転矩形になり、
/// 判定そのものが意味を失う。是正版は**寸が未確定なら位置に一切手を入れず** `warn!` を残す。
///
/// # 檻の非空虚性（[[5.2 の教訓]]＝ログ側だけの守りにしない）
///
/// 寸フィルタを外す変異では、位置センチネルが `old_rect = None`（不明）へ落ちるため
/// 安全側 `ClampX` が走り、`clamp_x_into(x, i32::MIN, wa)` が `wa.left` を返す
/// ＝**提案位置と違う座標が書かれる**。提案 X を `left_wa().left` より左へ置いてあるのは
/// そのためで、位置 assert が第一の守りになる。
#[test]
fn balloon_undetermined_size_holds_proposed_position_and_warns() {
    for dpi in DPIS {
        // 提案 X は左モニタ work area の左端よりさらに左（センチネル素通し変異で
        // 必ず `left_wa().left` へ引き戻される位置）。
        let offset = point(-px(4500, dpi), -px(400, dpi));
        let settled = char_settled_pos(dpi);
        let bare = point(settled.x + offset.x, settled.y + offset.y);
        assert!(
            bare.x < left_wa().left,
            "dpi={dpi}: 探針が不動点——センチネル素通し変異でも X が動かない配置になっている"
        );

        // 窓生成直後の実表現（position・size ともに CW_USEDEFAULT センチネル）。
        let (mut world, char_window, balloon) =
            char_with_balloon_window_pos(dpi, WindowPos::default(), offset);

        let (ok, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                char_size(dpi),
                PlacementRoute::ReportedSizeReconcile,
            )
        });
        assert!(ok);
        assert_eq!(
            point_of(&world, balloon),
            bare,
            "dpi={dpi}: 寸未確定（センチネル）なのに位置へ手が入った"
        );
        let warned = expect_one(&events, UNRESOLVED_TAG);
        assert_eq!(
            warned.level,
            tracing::Level::WARN,
            "dpi={dpi}: 判定不能が warn として残っていない（Req 3.3）"
        );
        // **フィールド集合の固定**（`diagnosis-procedure.md` §3.1／§6.3 の振り分け規則が
        // これに依存する）: `route=BalloonFollow` で窓種別が引け、**`proposed` の有無**が
        // 本行（良性の判定不能）と装置異常（`MonitorSnapshot` 不在・モニタ 0 台）を分ける。
        // どちらを落としても実機判定が反転するので、literal で固定する
        // （[[5.1 → 7.2 の申し送り＝判定語に使っているのに檻が無い型]] の再発防止）。
        assert_eq!(
            warned.expect_field("route"),
            "BalloonFollow",
            "dpi={dpi}: 判定不能行が窓種別を名乗っていない（§3.1 の振り分けが成立しない）"
        );
        assert_eq!(
            warned.expect_field("proposed"),
            format!("{bare:?}"),
            "dpi={dpi}: 判定不能行の `proposed` が提案位置と違う（§6.3 の判別子）"
        );
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: 寸が読めないのに clamp している: {events:?}"
        );
    }
}

/// **§6.3 の判別子の裏面**: 真の観測装置異常（`MonitorSnapshot` 不在）はキャラ窓・
/// バルーン窓の**双方**から `WorkAreaUnresolved` を出すが、いずれも **`proposed` を
/// 持たない**。
///
/// 手順書はこの 1 点で「良性の判定不能（バルーン寸未確定）」と「セッション全体を
/// 無効にする装置異常」を分ける。`route=` だけでは分けられない——装置異常も
/// バルーン随伴で起きれば `route=BalloonFollow` を名乗るからである。
#[test]
fn missing_monitor_snapshot_warns_for_both_windows_without_the_proposed_field() {
    for dpi in DPIS {
        let (mut world, char_window, _balloon) =
            char_with_far_balloon_world(dpi, visible_balloon_pos(dpi), far_out_offset(dpi));
        world.remove_resource::<MonitorSnapshot>();
        // 射影が identity へ縮退しても書込が起きるよう、寸を変える（高さのみ＝
        // 手順 3b の x 付替えを避ける）。同寸だとべき等 skip で随伴まで届かない。
        let new = SizePx {
            w: char_size(dpi).w,
            h: px(200, dpi),
        };

        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, char_window, new, PlacementRoute::Resnap));
        assert!(ok, "dpi={dpi}: 寸の反映自体は従来どおり成立する");

        let warned = guard_events(&events, UNRESOLVED_TAG);
        assert_eq!(
            warned.len(),
            2,
            "dpi={dpi}: 装置異常はキャラ窓とバルーン窓の双方から出るはず: {events:?}"
        );
        let routes: Vec<&str> = warned.iter().map(|e| e.expect_field("route")).collect();
        assert!(
            routes.contains(&"Resnap") && routes.contains(&"BalloonFollow"),
            "dpi={dpi}: 2 行の route が {routes:?}（キャラ窓＋バルーン窓の対になっていない）"
        );
        for e in &warned {
            assert_eq!(
                e.level,
                tracing::Level::WARN,
                "dpi={dpi}: 水準が warn でない"
            );
            assert!(
                e.field("proposed").is_none(),
                "dpi={dpi}: 装置異常の行が `proposed` を持っている＝§6.3 の判別子が壊れる: {:?}",
                e.fields_map()
            );
        }
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: work area 不明なのに clamp している: {events:?}"
        );
    }
}

/// **旧位置の未確定も `Option::None` だけではない**: 寸だけ確定して位置が
/// `CW_USEDEFAULT` のままの窓は、素通しすると矩形が `i32::MIN` 近傍へ落ちて
/// 「もともと画面外に留置されていた」と誤判定され、**安全側 clamp の腕が丸ごと死ぬ**
/// （6.1 が寸について踏んだのと同型の罠を、位置について踏まないための檻）。
///
/// 負座標そのものは正当（左モニタは `-1920..0`）ゆえ、判定は符号ではなく
/// wintf 正典のセンチネル一致で行う。
#[test]
fn balloon_undetermined_position_is_treated_as_unknown_rect_and_clamps() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let b_size = balloon_size(dpi);
        let offset = far_out_offset(dpi);
        let settled = char_settled_pos(dpi);
        let bare = point(settled.x + offset.x, settled.y + offset.y);
        assert!(
            !visible_in(&layout, bare, b_size),
            "dpi={dpi}: 探針が不動点——提案が既に可視で安全側 clamp の腕へ入らない"
        );

        // 寸は確定済み・位置だけ CW_USEDEFAULT（wintf 正典の未確定表現）。
        let window_pos = WindowPos {
            size: Some(SizeI::new(b_size.w, b_size.h)),
            ..Default::default()
        };
        let (mut world, char_window, balloon) =
            char_with_balloon_window_pos(dpi, window_pos, offset);

        let (ok, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                char_size(dpi),
                PlacementRoute::DpiReproject,
            )
        });
        assert!(ok);
        assert!(
            visible_in(&layout, point_of(&world, balloon), b_size),
            "dpi={dpi}: 位置未確定（センチネル）を『留置』と誤読して clamp を見送っている"
        );
        expect_one(&events, CLAMP_TAG);
    }
}

/// 破棄済みバルーンへの随伴は**正常終了系**として `debug!` で打ち切る（Req 6.2/6.3・
/// task 3.2 と同じ区別）。ここを `warn!` にすると終了時ログが良性ノイズで埋まり、
/// 本物の異常（実在窓の寸未確定）が読めなくなる。
#[test]
fn balloon_despawned_skips_guard_without_warning() {
    for dpi in DPIS {
        let (mut world, char_window, balloon) =
            char_with_far_balloon_world(dpi, visible_balloon_pos(dpi), far_out_offset(dpi));
        world.despawn(balloon);

        let (_, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                char_size(dpi),
                PlacementRoute::Resnap,
            )
        });

        assert!(
            guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
            "dpi={dpi}: 破棄済みバルーンに対してガードが喋っている（Req 6.2 違反）: {events:?}"
        );
        // **task 7.3 で強化**: 6.2 が固定していたのは「ガードが喋らない」だけで、
        // 随伴書込そのもの（`enqueue_window_set_pos`）が破棄済みバルーンに対して
        // `warn!` を出していた（6.2 → 7.3 の申し送り）。終了時静穏（Req 6.2）は
        // **経路全体**の主張ゆえ、ここで警告以上ゼロを丸ごと見る。
        assert!(
            events.iter().all(|e| e.level > tracing::Level::INFO),
            "dpi={dpi}: 破棄済みバルーンに対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
        );
        // **相ごとに数える**——総数で数えると、片方の打ち切りを外しても他方が同じ
        // 判定語を出して総数が偶然一致し、檻が空虚になる（3.2 の教訓と同型）。
        let skips = despawn_skip_lines(&events);
        assert!(
            skips.iter().all(|e| e.level == tracing::Level::DEBUG),
            "dpi={dpi}: 破棄済み打ち切りが debug 水準でない: {skips:?}"
        );
        assert_eq!(
            skips
                .iter()
                .filter(|e| e.message().contains("可視性の遷移ガード"))
                .count(),
            1,
            "dpi={dpi}: 遷移ガード相の打ち切りが 1 行でない: {events:?}"
        );
        assert_eq!(
            skips
                .iter()
                .filter(|e| e.message().contains("窓移動"))
                .count(),
            1,
            "dpi={dpi}: 随伴書込相の打ち切りが 1 行でない: {events:?}"
        );
    }
}

/// 破棄済み判定語（[`DESPAWNED_SKIP_TAG`]）を含む行を抜く（相ごとの計数用）。
fn despawn_skip_lines(events: &[LogEvent]) -> Vec<&LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains(DESPAWNED_SKIP_TAG))
        .collect()
}

/// Req 6.2 の裏面（真の異常を殺さない・随伴書込相）: **生存している** entity の
/// `WindowHandle` 欠落（窓生成前）は従来どおり `warn!`。存在確認の導入でこちらまで
/// 静穏化してはならない——「窓がまだ無い」は結線の異常であって終了系ではない。
#[test]
fn balloon_without_handle_on_living_entity_still_warns_on_follow_write() {
    let dpi = 96;
    let (mut world, char_window, balloon) =
        char_with_far_balloon_world(dpi, visible_balloon_pos(dpi), far_out_offset(dpi));
    // entity は実在させたまま `WindowHandle` だけを剥がす（窓生成前と同じ状態）。
    world.entity_mut(balloon).remove::<WindowHandle>();

    let (_, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            char_window,
            char_size(dpi),
            PlacementRoute::Resnap,
        )
    });

    let warned = expect_one(&events, "WindowHandle 未付与");
    assert_eq!(
        warned.level,
        tracing::Level::WARN,
        "実在 entity の WindowHandle 欠落は真の異常＝warn のまま（Req 6.2 の区別）"
    );
    assert!(
        !despawn_skip_lines(&events)
            .iter()
            .any(|e| e.message().contains("窓移動")),
        "実在 entity を『破棄済み』と誤判定している: {events:?}"
    );
}

// -------------------------------------------------------------------------
// 混在 DPI・複数モニタ回帰檻の拡充（task 7.2・Req 3.4/4.4/5.1/5.2/5.3/5.6）
//
// task 6.1 は**キャラ窓だけ**が不可視へ落ちる合成を、task 6.2 は**バルーンだけ**が
// 落ちる合成（キャラは終始可視だと明示的に assert する）を固めた。どちらの檻も
// 「もう一方の窓は自明に安全」な世界で 1 つの連言肢を証明しており、Req 3.4 が
// 要求する **連言**——「キャラ窓とバルーン窓の *どちらも* 不可視状態に遷移させない」
// ——を 1 回の書込の中で見た檻は存在しない。本節が足すのはその連言と、
// 2 つのガードが**互いの結果に依存する**接続点である。
//
//   (A) 1 回の [`resize_window_to`] で**両窓が同時に**全 work area 非交差へ落ちる
//       合成。しかも救出先の work area が**別々のモニタ**になる配置で組むので、
//       clamp 先の解決が窓ごとに独立であること（キャラの clamp_wa を流用していない
//       こと）まで座標で固定される。
//   (B) バルーンが追従するのは **ガード適用後**のキャラ位置であること。手順 7 が
//       `new_pos` ではなく素の射影（`raw`／ガード前）を渡す変異は、6.2 の檻では
//       **不動点**になる（あちらはキャラが clamp されない合成ゆえ両者が同値）。
//       ここでは clamp 前後で px(40) ずれるので、恒等式の主張が実際に効く。
//
// 座標はすべて論理値 × DPI（96/120/192）で構築し、絶対 px の固定値を持たない
// （Req 5.6）。実 GPU・実高 DPI モニタを要さず決定論（Req 5.2）。
// -------------------------------------------------------------------------

/// [`gap_bound_char_world`] に随伴バルーンを足した World。
///
/// `offset` は**窓（char 窓左上）相対**の追従 offset。[`resize_window_to`] は寸法変動で
/// これを**一切書き換えない**（2026-07-31 実機 SSP 裁定・恒等式
/// `balloon_pos − char_pos ≡ offset` が全アンカーで不変）ので、spawn 時点の値が
/// そのまま追従に使われる。
fn gap_bound_char_world_with_balloon(
    dpi: i32,
    balloon_size: SizePx,
    balloon_pos: PointPx,
    offset: PointPx,
) -> (World, Entity, Entity, PointPx) {
    let old = wide_char_size(dpi);
    let old_pos = PointPx {
        x: gap_center_x(dpi) - old.w / 2,
        y: left_wa().bottom - old.h,
    };
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_sized(balloon_pos.x, balloon_pos.y, balloon_size.w, balloon_size.h),
        ))
        .id();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(old_pos.x, old_pos.y, old.w, old.h),
            Anchored(Anchor::Bottom),
            BalloonFollow::new(balloon, OffsetBase::unpinned(offset)),
        ))
        .id();
    (world, char_window, balloon, old_pos)
}

/// **Req 3.4／5.3 の連言**: 1 回の非ドラッグ配置書込で、キャラ窓とバルーン窓の
/// **どちらも**全 work area 非交差にならない。しかも救出先は**別々のモニタ**である。
///
/// 合成の骨格（混在 DPI・複数モニタ・負座標・192 で 3200 超座標）:
/// - キャラ窓は帯（`0 ..= px(64)`＝どの work area にも属さない）へ落ちる幅の新寸を
///   受け取り、**右モニタ**へ引き戻される（[`gap_bound_char_world`] と同じ機序）。
/// - 随伴 offset は救出後のキャラ位置から見て遥か左（`-px(2600)`）を指すので、
///   バルーン提案矩形は**左モニタよりさらに左**の完全不可視域へ出る。最近傍は
///   左モニタゆえ **`left_wa().left` へ**引き戻される。
///
/// ゆえに 2 つの clamp 先が別モニタになる——キャラの `clamp_wa` を流用する実装は
/// バルーンを右モニタへ引き戻してしまい、`balloon.x == left_wa().left` の assert が
/// 落ちる。6.1／6.2 の単窓檻はどちらもこの取り違えに対して不動点である
/// （両窓の clamp 先が同じ右モニタになる合成しか持っていない）。
#[test]
fn both_windows_survive_a_single_write_onto_different_monitors() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        let b_size = balloon_size(dpi);
        // 窓相対の追従 offset。リサイズで補正されないので spawn 時点＝追従時点。
        // 救出後のキャラ位置から見て遥か左（左モニタよりさらに外）を指す。
        let offset = point(-px(2600, dpi), -px(600, dpi));
        // 旧バルーンは**左モニタ内**で可視（＝「遷移」であって留置ではない）。
        // 座標は左モニタ左端からの論理オフセット×DPI で組む（絶対 px を置かない・Req 5.6）。
        let old_balloon = point(left_wa().left + px(360, dpi), px(200, dpi));

        for route in [
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::ReportedSizeReconcile,
        ] {
            let (mut world, char_window, balloon, old_pos) =
                gap_bound_char_world_with_balloon(dpi, b_size, old_balloon, offset);

            // --- (1) 探針の自己検査（[[2.2 の教訓]]）---
            let char_bare = unguarded_projection(dpi, old_pos, new);
            let char_saved = point(right_wa(dpi).left, char_bare.y);
            let balloon_bare = point(char_saved.x + offset.x, char_saved.y + offset.y);
            assert!(
                visible_in(&layout, old_pos, wide_char_size(dpi)),
                "dpi={dpi}: 旧キャラ矩形が非交差では『遷移』にならない"
            );
            assert!(
                visible_in(&layout, old_balloon, b_size),
                "dpi={dpi}: 旧バルーン矩形が非交差では『遷移』にならない"
            );
            assert!(
                !visible_in(&layout, char_bare, new),
                "dpi={dpi}: 探針が不動点——ガード無しのキャラ提案 {char_bare:?} が既に可視"
            );
            assert!(
                !visible_in(&layout, balloon_bare, b_size),
                "dpi={dpi}: 探針が不動点——ガード無しのバルーン提案 {balloon_bare:?} が既に可視"
            );

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, char_window, new, route));
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            let char_pos = point_of(&world, char_window);
            let balloon_pos = point_of(&world, balloon);

            // --- (2) 連言そのもの（Req 3.4）: どちらも全 work area 非交差ではない ---
            assert!(
                visible_in(&layout, char_pos, new),
                "dpi={dpi} route={route}: キャラ窓 {char_pos:?} が全 work area と非交差"
            );
            assert!(
                visible_in(&layout, balloon_pos, b_size),
                "dpi={dpi} route={route}: バルーン窓 {balloon_pos:?} が全 work area と非交差"
            );

            // --- (3) 救出先は**別々のモニタ**（clamp 先の解決が窓ごとに独立）---
            assert_eq!(
                char_pos, char_saved,
                "dpi={dpi} route={route}: キャラは右モニタ左端へ引き戻されるはず"
            );
            assert_eq!(
                balloon_pos.x,
                left_wa().left,
                "dpi={dpi} route={route}: バルーンの clamp 先が左モニタでない\
                 （キャラの clamp_wa を流用している疑い）: {balloon_pos:?}"
            );

            // --- (4) Y は両窓とも射影／恒等式の所有＝ガードは触らない ---
            assert_eq!(
                char_pos.y, char_bare.y,
                "dpi={dpi} route={route}: キャラの Y が動いた"
            );
            assert_eq!(
                balloon_pos.y, balloon_bare.y,
                "dpi={dpi} route={route}: バルーンの Y が動いた"
            );

            // --- (5) 判定語: ClampX が**ちょうど 2 行**（両窓ぶん）・水準は WARN ---
            let clamps = guard_events(&events, CLAMP_TAG);
            assert_eq!(
                clamps.len(),
                2,
                "dpi={dpi} route={route}: ClampX が両窓ぶん 2 行でない: {events:?}"
            );
            for ev in clamps {
                assert_eq!(
                    ev.level,
                    tracing::Level::WARN,
                    "dpi={dpi} route={route}: clamp の記録が warn 水準でない"
                );
            }
        }
    }
}

/// **Req 4.4 の恒等式は「ガード適用後のキャラ位置」に対して成立する**。
///
/// [`resize_window_to`] 手順 7 は確定位置（`new_pos`＝遷移ガード適用**後**）で
/// [`follow_balloon`] を呼ぶ。ここを素の射影（ガード前）へ差し替える変異は、
/// 6.2 の檻ではキャラが clamp されない合成ゆえ**不動点**になる。
///
/// 本檻はキャラだけが clamp される合成（clamp 前後で X が `px(40)` ずれる）を組み、
/// バルーンの追従先が**ずれた後**の位置であることを座標で固定する。バルーン自身は
/// clamp されない（＝救われたのはキャラだけ・`ClampX` はちょうど 1 行）ので、
/// 「バルーンが偶然どこかへ clamp されて結果が一致した」逃げ道も塞がる。
///
/// **区分（areka-P0-balloon-offset-dpi task 6.4・要件 7.4／7.6・design D13）＝本檻は
/// 「寸法変化に対する不変」群である。拡大率遷移を一度も起こさないので、追随の証拠にはならない。**
/// 本檻は `for dpi in DPIS` で**水準ごとに世界を組み直して** [`resize_window_to`] を直接呼ぶ
/// だけであり、`DPI` を書き換えて `Changed<DPI>` のエッジを立てることは無い。追随の発火条件
/// はそのエッジに限られる（`emo2_boot::frame::dpi` の相）ため、追随が入っても本檻には届かない
/// ——上の `stored_offset == offset`（窓相対契約＝リサイズで offset を補正しない・2026-07-31
/// 実機 SSP 裁定）は**是正後も真のまま**であり、主張は 1 文字も書き換えていない。
///
/// 実測（task 6.4）: 追随の適用（`emo2_boot::frame::balloon_offset_follow::rescale_balloon_follow_offset`
/// の呼出）を外した走行で、本件は**緑のまま**であった（同走行では追随の檻 13 件が赤になる）。
/// ゆえに**本件が緑であることを「追随を壊していない」の根拠に使ってはならない**。
#[test]
fn balloon_follows_the_guarded_char_position_not_the_raw_projection() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        // 帯（`0 ..= px(64)`）より**狭い**バルーン＝帯の中へ丸ごと収まり得る。
        let b_size = SizePx {
            w: px(48, dpi),
            h: px(300, dpi),
        };
        // 窓相対の追従 offset（リサイズで補正されない＝spawn 時点＝追従時点）。
        let offset = point(-px(12, dpi), -px(600, dpi));
        let old_balloon = visible_balloon_pos(dpi);

        for route in [
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::ReportedSizeReconcile,
        ] {
            let (mut world, char_window, balloon, old_pos) =
                gap_bound_char_world_with_balloon(dpi, b_size, old_balloon, offset);

            let char_bare = unguarded_projection(dpi, old_pos, new);
            let char_saved = point(right_wa(dpi).left, char_bare.y);
            let follows_guarded = point(char_saved.x + offset.x, char_saved.y + offset.y);
            let follows_raw = point(char_bare.x + offset.x, char_bare.y + offset.y);

            // --- 探針の自己検査: 2 つの追従先が**区別できる**こと ---
            assert_ne!(
                follows_guarded.x, follows_raw.x,
                "dpi={dpi}: 探針が不動点——ガード前後でキャラ X が動いていない"
            );
            assert!(
                !visible_in(&layout, char_bare, new),
                "dpi={dpi}: 探針が不動点——ガード無しのキャラ提案が既に可視"
            );
            assert!(
                visible_in(&layout, follows_guarded, b_size),
                "dpi={dpi}: 救出後のキャラに追従したバルーンは可視のはず（clamp 不要）"
            );
            assert!(
                !visible_in(&layout, follows_raw, b_size),
                "dpi={dpi}: 素の射影に追従したバルーン {follows_raw:?} が可視では変異を区別できない"
            );

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, char_window, new, route));
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            assert_eq!(
                point_of(&world, char_window),
                char_saved,
                "dpi={dpi} route={route}: キャラが右モニタ左端へ救出されていない"
            );
            assert_eq!(
                point_of(&world, balloon),
                follows_guarded,
                "dpi={dpi} route={route}: バルーンが**ガード適用後**のキャラ位置に追従していない\
                 （素の射影に追従した場合は {follows_raw:?}）"
            );
            assert!(
                visible_in(&layout, point_of(&world, balloon), b_size),
                "dpi={dpi} route={route}: 追従先のバルーンが全 work area と非交差"
            );

            // 恒等式（Req 4.4）: `balloon − char ≡ BalloonFollow.offset`。
            // 比較相手は**書込前から不変の**窓相対 offset（テスト側の定数）であり、
            // world から読み直した値ではない——読み直すと「恒等式を、それを作った
            // 当人に問う」恒真形になる（[[7.2 の空虚性 8 例目]]）。
            let stored_offset = world
                .get::<BalloonFollow>(char_window)
                .expect("char 窓は BalloonFollow を持つ")
                .offset();
            assert_eq!(
                stored_offset, offset,
                "dpi={dpi} route={route}: BalloonFollow.offset が書き換わった\
                 （窓相対契約＝リサイズで offset を補正しない・2026-07-31 実機 SSP 裁定）"
            );
            let c = point_of(&world, char_window);
            let b = point_of(&world, balloon);
            assert_eq!(
                point(b.x - c.x, b.y - c.y),
                offset,
                "dpi={dpi} route={route}: 追従恒等式が崩れている"
            );

            // 救われたのは**キャラだけ**＝`ClampX` はちょうど 1 行。
            let clamps = guard_events(&events, CLAMP_TAG);
            assert_eq!(
                clamps.len(),
                1,
                "dpi={dpi} route={route}: ClampX がキャラぶん 1 行でない\
                 （バルーンまで clamp されているなら追従先が偶然一致しただけ）: {events:?}"
            );
            assert_eq!(
                clamps[0].level,
                tracing::Level::WARN,
                "dpi={dpi} route={route}: clamp の記録が warn 水準でない"
            );
        }
    }
}
