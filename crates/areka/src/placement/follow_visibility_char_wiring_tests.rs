use bevy_ecs::prelude::*;
use wintf::ecs::SizeI;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, WindowPos};

use super::test_support::{
    CLAMP_TAG, DPIS, GUARD_TAG_PREFIX, NEAREST_TAG, OFFSCREEN_PULL_TAG, UNRESOLVED_TAG,
    drag_event_at, dragging_state, fake_handle, gap_center_x, guard_events, left_wa, mixed_layout,
    narrow_char_size, point_of, px, right_wa, unguarded_projection, visible_in, wide_char_size,
    win, window_pos_sized,
};
use super::super::test_support::{capture_logs, expect_one};
use super::{
    Anchored, MonitorSnapshot, PlacementRoute, WorkAreaResolution, on_char_drag, project_anchor,
    resize_window_to, work_area_for_window_with_origin,
};
use crate::placement::resolver::{Anchor, PointPx, SizePx};

// -------------------------------------------------------------------------
// 遷移ガードの**配線**（task 6.1・S3 是正・Req 3.1/3.2/3.3・D5/D6/D13）
//
// task 2.2 は `guard_visibility`／`work_area_for_window_with_origin` を純関数として
// 用意したが**本番呼出はゼロ**だった（diagnosis-report.md §1.3「純関数が在ることは
// S3 の充足ではない」）。本節が檻に入れるのは純関数の判定規則ではなく、
// **`resize_window_to` の中でそれが実際に走るか・どの route で走るか**である。
//
// 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
//   (1) 探針の自己検査——ガード**無し**の提案が本当に全 work area 非交差であること
//       （交差する探針では ClampX 腕へ一度も入らず「緑」が何も意味しない・[[2.2 の教訓]]）
//   (2) 位置の不変条件——clamp 後の矩形がいずれかの work area と交差する（Req 3.1）
//   (3) route による発火条件——適用外 route（`MoveCue`／`Restore` 等）とドラッグ経路
//       では**位置が素の射影と 1 bit も違わない**こと。ログ側の否定 assert だけに
//       依存しない（[[5.2 の教訓＝空虚性 6 例目]]: 不変量がログ側にしか無いと
//       別ファイルの水準変更で守りが消える）
//   (4) 判定語のリテラル——手順書 §3.3 の grep 語を檻側にも literal で持つ
//       （[[5.1 → 7.2 の申し送り]]「判定語に使っているのに檻が無い」型の再発防止）
//
// 座標はすべて論理値 × DPI（96/120/192）で構築し、絶対 px の固定値を持たない（Req 5.6）。
// -------------------------------------------------------------------------

use super::route_applies_visibility_guard;

/// 「旧矩形は可視・新提案は全 work area 非交差」へ落ちるキャラ窓 World を組む。
///
/// 旧寸 [`wide_char_size`] の窓を、下端中央付替え（`resize_window_to` 手順 3b）後の
/// 中心が帯へ落ちる位置に置く。新寸 [`narrow_char_size`] は帯より狭いので、射影 T が
/// 出す提案矩形は帯へ収まり **どの work area とも交差しない**——S3 が言う
/// 「非ドラッグ要因で不可視へ遷移する」状態そのものを合成する。
fn gap_bound_char_world(dpi: i32) -> (World, Entity, PointPx) {
    let old = wide_char_size(dpi);
    let old_pos = PointPx {
        x: gap_center_x(dpi) - old.w / 2,
        y: left_wa().bottom - old.h,
    };
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(old_pos.x, old_pos.y, old.w, old.h),
            Anchored(Anchor::Bottom),
        ))
        .id();
    (world, e, old_pos)
}

/// 発火条件の**表そのもの**を固定する（D13 帰結⑴⑵）。挙動側の檻（下 2 件）と
/// 二段構えにしてあるのは、語彙が 12 種あるのに `resize_window_to` を実際に通るのは
/// 現状 4 種だけで、残り 8 種の判定が挙動檻だけでは**合成でしか**検査できないため。
/// [`PlacementRoute::ALL`] を回すので、語彙が増えたら本檻も落ちる。
///
/// 発火側は 6 種ある。うち
/// [`WorkAreaResnap`](PlacementRoute::WorkAreaResnap)／[`ChainRealign`](PlacementRoute::ChainRealign)
/// は areka-P0-dpi-transition-atomicity が足した**システム由来の再アンカー**で、ユーザーの
/// 明示操作ではない（同 design D9 が既定位置の追跡対象として挙げる 6 経路と同じ区分）。
/// 書込を出す側は task 5.2／5.6 が新設するため、現時点で判定を検査できるのは本表だけである。
#[test]
fn visibility_guard_route_table_matches_the_d13_decision() {
    for route in PlacementRoute::ALL {
        let expected = matches!(
            route,
            PlacementRoute::AnchorChange
                | PlacementRoute::Resnap
                | PlacementRoute::DpiReproject
                | PlacementRoute::ReportedSizeReconcile
                | PlacementRoute::WorkAreaResnap
                | PlacementRoute::ChainRealign
        );
        assert_eq!(
            route_applies_visibility_guard(route),
            expected,
            "route={route} の発火判定が D13 帰結⑴⑵ と食い違う"
        );
    }
    // 表が「全部真」「全部偽」へ潰れていないこと（自明な述語への退化の検出）。
    let fired = PlacementRoute::ALL
        .into_iter()
        .filter(|r| route_applies_visibility_guard(*r))
        .count();
    assert_eq!(fired, 6, "発火 route が 6 種でない（表が潰れている）");
}

/// **Req 3.1 の本体**: 非ドラッグの配置系 4 経路（D13 帰結⑴）では、全 work area
/// 非交差への遷移が X の clamp で阻止され、`warn!` が 1 行残る。
///
/// Y は射影 T の所有ゆえ 1 bit も動かない（[`guard_visibility`] の事後条件）。
#[test]
fn visibility_guard_clamps_x_on_non_drag_placement_routes() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        for route in [
            PlacementRoute::AnchorChange,
            PlacementRoute::Resnap,
            PlacementRoute::DpiReproject,
            PlacementRoute::ReportedSizeReconcile,
        ] {
            let (mut world, e, old_pos) = gap_bound_char_world(dpi);

            // (1) 探針の自己検査: 素の射影は**本当に**不可視へ落ちる／旧矩形は可視。
            //     どちらかが崩れると ClampX 腕に入らず、この檻は空虚になる。
            let bare = unguarded_projection(dpi, old_pos, new);
            assert!(
                !visible_in(&layout, bare, new),
                "dpi={dpi}: 探針が不動点——ガード無しの提案 {bare:?} が既に可視で ClampX 腕へ入らない"
            );
            assert!(
                visible_in(&layout, old_pos, wide_char_size(dpi)),
                "dpi={dpi}: 旧矩形が非交差では『遷移』でなく留置＝Keep が正解になってしまう"
            );

            let (ok, events) = capture_logs(|| resize_window_to(&mut world, e, new, route));
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            // (2) 位置の不変条件（Req 3.1）: 書かれた矩形はどこかの work area と交差する。
            let pos = point_of(&world, e);
            assert!(
                visible_in(&layout, pos, new),
                "dpi={dpi} route={route}: Req 3.1 違反——{pos:?} は全 work area と非交差"
            );
            assert_eq!(
                pos.y, bare.y,
                "dpi={dpi} route={route}: Y は射影 T の所有＝ガードが触ってはならない"
            );
            assert_ne!(
                pos.x, bare.x,
                "dpi={dpi} route={route}: X が引き戻されていない（ガード未発火）"
            );
            // clamp 先は射影が Y に用いた work area（右モニタ）の水平範囲内。
            let wa = right_wa(dpi);
            assert!(
                wa.left <= pos.x && pos.x <= wa.right - new.w,
                "dpi={dpi} route={route}: clamp 先が射影の work area {wa:?} の外: {pos:?}"
            );

            // (4) 判定語: ClampX の warn が 1 行・水準は WARN（Req 3.1/3.2 の観測）。
            let clamped = expect_one(&events, CLAMP_TAG);
            assert_eq!(
                clamped.level,
                tracing::Level::WARN,
                "dpi={dpi} route={route}: clamp の記録が warn 水準でない"
            );
        }
    }
}

/// **Req 3.1 の裏面（D13 帰結⑵）**: 明示操作系・非配置系の route では、位置が素の
/// 射影と 1 bit も違わず、ガードのログも 1 行も出ない。
///
/// `MoveCue`（`\![move]`）と `Restore`（位置復元）を引き戻すのは、スクリプト／
/// 永続化が決めた位置の否定であり本 spec の Out of scope である。**ここが緑のまま
/// 「常に発火」へ変異させられると S3 是正が明示操作の尊重を壊す**ため、位置側の
/// assert（ログではなく挙動）を第一の守りに置く。
#[test]
fn visibility_guard_does_not_fire_on_explicit_or_non_placement_routes() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        for route in [
            PlacementRoute::SpawnInitial,
            PlacementRoute::Restore,
            PlacementRoute::KeepPositionResize,
            PlacementRoute::BalloonFollow,
            PlacementRoute::MoveCue,
        ] {
            let (mut world, e, old_pos) = gap_bound_char_world(dpi);
            let bare = unguarded_projection(dpi, old_pos, new);
            // 探針の自己検査: ガードが**発火する条件は揃っている**（route だけが違う）。
            assert!(
                !visible_in(&layout, bare, new),
                "dpi={dpi}: 探針が不動点——発火条件が揃っていない"
            );

            let (ok, events) = capture_logs(|| resize_window_to(&mut world, e, new, route));
            assert!(ok, "dpi={dpi} route={route}: 書込は成立する前提");

            assert_eq!(
                point_of(&world, e),
                bare,
                "dpi={dpi} route={route}: 適用外 route で位置が動いた（明示操作の尊重が壊れている）"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} route={route}: 適用外 route でガードが喋っている: {events:?}"
            );
        }
    }
}

/// **ドラッグ経路は従来の水準のまま**（Req 3.3 の水準分岐・D5）: ユーザーが自分で
/// 帯へ運んだ窓は引き戻されず、毎イベント発火する経路に `warn!` を増やさない。
#[test]
fn drag_path_neither_clamps_nor_warns_when_leaving_every_work_area() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let size = narrow_char_size(dpi);
        // 開始位置は右モニタ上（可視）・接地済み。
        let start_pos = PointPx {
            x: px(200, dpi),
            y: right_wa(dpi).bottom - size.h,
        };
        assert!(
            visible_in(&layout, start_pos, size),
            "dpi={dpi}: 前提——ドラッグ開始位置は可視"
        );

        let mut world = World::new();
        world.insert_resource(mixed_layout(dpi));
        let cursor = (px(800, dpi), px(400, dpi));
        let window = world
            .spawn((
                fake_handle(0x1000),
                window_pos_sized(start_pos.x, start_pos.y, size.w, size.h),
                Anchored(Anchor::Bottom),
                dragging_state((start_pos.x, start_pos.y), cursor),
            ))
            .id();

        // カーソルを帯へ運ぶ: 生ドラッグ x = px(24) ＝ 帯の内側。
        let moved = (cursor.0 - (px(200, dpi) - px(24, dpi)), cursor.1);
        let ev = Phase::Bubble(drag_event_at(window, cursor, moved));
        let (consumed, events) = capture_logs(|| on_char_drag(&mut world, window, window, &ev));
        assert!(!consumed);

        let pos = point_of(&world, window);
        // 自己検査: ドラッグは**実際に**窓を全 work area の外へ運んだ（＝ガードが
        // 配線されていれば必ず clamp する状況である）。
        assert!(
            !visible_in(&layout, pos, size),
            "dpi={dpi}: 探針が不動点——ドラッグ先が可視のままでは『引き戻さない』を検査していない"
        );
        assert_eq!(
            pos.x,
            px(24, dpi),
            "dpi={dpi}: ドラッグの X は素通し（明示操作の尊重）"
        );
        assert!(
            guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
            "dpi={dpi}: ドラッグ経路でガードが喋っている（spam・水準分岐の破壊）: {events:?}"
        );
    }
}

/// **Req 3.2**: 最近傍フォールバック（窓中心がどのモニタにも属さない＝モニタ構成
/// 情報と実画面の食い違いの兆候）は、非ドラッグ経路で `warn!` へ昇格する。
///
/// この探針は **clamp を伴わない**（提案矩形は work area と交差したまま）——
/// `NearestFallback` の観測が `ClampX` の副産物ではなく独立に成立することを示す。
#[test]
fn nearest_fallback_warns_on_non_drag_route_even_without_clamping() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let old = wide_char_size(dpi);
        // 幅は据置き・高さだけ変える＝手順 3b で x は動かず、中心は帯に留まる。
        let new = SizePx {
            w: old.w,
            h: px(200, dpi),
        };
        let (mut world, e, old_pos) = gap_bound_char_world(dpi);

        // 探針の自己検査: **決めた位置**の work area 解決が本当に最近傍へ落ちる
        // （`Contains` なら昇格の腕へ入らず空虚になる）。かつ提案矩形は交差したまま
        // ＝clamp しない（`NearestFallback` が `ClampX` の副産物でないことの担保）。
        let bare = unguarded_projection(dpi, old_pos, new);
        let (_, resolution) = work_area_for_window_with_origin(&layout, win(bare, new))
            .expect("合成レイアウトは空でない");
        assert_eq!(
            resolution,
            WorkAreaResolution::NearestFallback,
            "dpi={dpi}: 探針が `Contains` に落ちている＝昇格の腕を検査していない"
        );
        assert!(
            visible_in(&layout, bare, new),
            "dpi={dpi}: 探針が clamp を伴っている＝`NearestFallback` 単独の檻になっていない"
        );

        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));
        assert!(ok);
        assert_eq!(
            point_of(&world, e),
            bare,
            "dpi={dpi}: Keep 腕で位置が動いた"
        );

        let warned = expect_one(&events, NEAREST_TAG);
        assert_eq!(
            warned.level,
            tracing::Level::WARN,
            "dpi={dpi}: 最近傍フォールバックが非ドラッグ経路で warn へ昇格していない"
        );
        assert!(
            guard_events(&events, CLAMP_TAG).is_empty(),
            "dpi={dpi}: clamp していないのに ClampX が出ている: {events:?}"
        );
    }
}

// -------------------------------------------------------------------------
// 画面外からの引き寄せの記録（areka-P0-dpi-transition-atomicity 要件 5.5・task 5.1）
//
// 上の `nearest_fallback_warns_on_non_drag_route_even_without_clamping` が見ているのは
// 射影が**決めた位置**の帰属である。射影の**入力**（＝Y を決めるのに使った矩形）が
// どの work area とも交差しない位置に在った場合、決めた位置がモニタ内へ収まってしまえば
// あちらの腕には入らない——窓は画面の外から黙って最近傍のモニタへ引き寄せられ、観測が
// 1 行も残らない（実測で 0 行だった）。副モニタを引き抜いたときにゴーストが主モニタへ
// 引き寄せられるのは**正しい挙動**（開発者の裁定 2026-08-20・位置は変えない）だが、
// 勝手に飛んだことは後から追えねばならない。
//
// **帰属だけを条件にしてはならない**——下端吸着の正常な resize では、旧位置に背の高い
// 新寸を当てた矩形の中心が work area 下端へちょうど載る（半開区間で非該当）ことが
// 珍しくなく、偽陽性になる。下の 4 本目がその形を名指しで固定する。
// -------------------------------------------------------------------------

/// 全 work area の**外**（上方）に居るキャラ窓。射影の入力は帰属せず、射影が決めた位置は
/// 最近傍モニタの下端＝帰属する（＝決めた位置側の腕には入らない構図）。
fn offscreen_char_world(dpi: i32) -> (World, Entity, PointPx, SizePx) {
    let size = wide_char_size(dpi);
    // x は右モニタの内側（帯や左モニタに引かれない）・y は全 work area の遥か上。
    let pos = PointPx {
        x: px(600, dpi),
        y: -px(3000, dpi),
    };
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let e = world
        .spawn((
            fake_handle(0x1600),
            window_pos_sized(pos.x, pos.y, size.w, size.h),
            Anchored(Anchor::Bottom),
        ))
        .id();
    (world, e, pos, size)
}

/// 探針の自己検査＋実行を 1 つにまとめる（入力側は最近傍・決めた位置は帰属、を毎回確かめる）。
fn run_offscreen_reprojection(
    dpi: i32,
    route: PlacementRoute,
) -> Vec<crate::placement::test_support::LogEvent> {
    let layout = mixed_layout(dpi);
    let (mut world, e, pos, size) = offscreen_char_world(dpi);

    // (1) 射影の**入力**は帰属しない（`Contains` なら本檻は何も見ていない）。
    let (_, input) = work_area_for_window_with_origin(&layout, win(pos, size))
        .expect("合成レイアウトは空でない");
    assert_eq!(
        input,
        WorkAreaResolution::NearestFallback,
        "dpi={dpi}: 探針の入力が帰属してしまっている＝観測すべき腕を通らない"
    );
    // (2) 射影が**決めた位置**は帰属する＝既存の観測（`NEAREST_TAG`）は鳴らない構図。
    let decided = project_anchor(Anchor::Bottom, pos, size, Some(&layout));
    let (_, resolved) = work_area_for_window_with_origin(&layout, win(decided, size))
        .expect("合成レイアウトは空でない");
    assert_eq!(
        resolved,
        WorkAreaResolution::Contains,
        "dpi={dpi}: 決めた位置まで帰属しない探針＝既存の観測と区別が付かない"
    );

    let (ok, events) = capture_logs(|| resize_window_to(&mut world, e, size, route));
    assert!(ok, "dpi={dpi}: 書込が成立していない（前提が崩れている）");
    events
}

/// **要件 5.5（記録の側）**: どの work area とも交差しない位置に居た窓が最近傍モニタへ
/// 引き寄せられたことを、非ドラッグ経路で `warn!` として残す（位置は変えない）。
#[test]
fn an_offscreen_projection_input_warns_on_a_non_drag_route() {
    for dpi in DPIS {
        let events = run_offscreen_reprojection(dpi, PlacementRoute::WorkAreaResnap);

        let warned = expect_one(&events, OFFSCREEN_PULL_TAG);
        assert_eq!(
            warned.level,
            tracing::Level::WARN,
            "dpi={dpi}: 画面外からの引き寄せが warn として残っていない"
        );
        // 決めた位置は帰属しているので、既存の観測は鳴らない＝2 語が別の事象を指している。
        assert!(
            guard_events(&events, NEAREST_TAG).is_empty(),
            "dpi={dpi}: 決めた位置の観測まで鳴っている＝2 語が同じ事象を二重に報告している: {events:?}"
        );
    }
}

/// 対（零件の主張の陽性側と対になる否定）: 入力が帰属している通常の配置では鳴らない。
///
/// これが無いと「毎回鳴る警告」でも上の檻は緑になる。
#[test]
fn an_onscreen_projection_input_stays_silent() {
    for dpi in DPIS {
        let new = narrow_char_size(dpi);
        let (mut world, e, old_pos) = gap_bound_char_world(dpi);
        // 入力（帯の中の窓）ではなく、右モニタの内側へ置き直した状態から始める。
        let inside = PointPx {
            x: right_wa(dpi).left + px(100, dpi),
            y: right_wa(dpi).bottom - wide_char_size(dpi).h,
        };
        world.entity_mut(e).insert(window_pos_sized(
            inside.x,
            inside.y,
            wide_char_size(dpi).w,
            wide_char_size(dpi).h,
        ));
        let _ = old_pos;

        let (_, events) =
            capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::WorkAreaResnap));
        assert!(
            guard_events(&events, OFFSCREEN_PULL_TAG).is_empty(),
            "dpi={dpi}: 画面内の入力で引き寄せの警告が出ている: {events:?}"
        );
    }
}

/// 明示操作の経路（ガード適用外）には効かせない——ドラッグ中や `\![move]` の最近傍落ちは
/// 正常な挙動であり、毎イベント warn を出すと本物の異常が埋まる。
#[test]
fn an_offscreen_projection_input_stays_silent_on_a_non_applying_route() {
    for dpi in DPIS {
        let events = run_offscreen_reprojection(dpi, PlacementRoute::MoveCue);
        assert!(
            guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
            "dpi={dpi}: 適用外 route でガードの観測が出ている: {events:?}"
        );
    }
}

/// **偽陽性の檻**: 下端吸着の正常な resize——旧位置に背の高い新寸を当てた入力矩形の中心が
/// work area 下端へちょうど載る形——では鳴らない。
///
/// 帰属（半開区間）だけを条件にすると、この正常系が「どのモニタにも属さない」と判定されて
/// 警告が鳴り続ける。実装中に既存の檻（`frame_diag_route_tests`）が実際にこの偽陽性を
/// 捕まえたので、同じ形をここで名指しして固定する（別ファイルの檻に守りを預けない）。
#[test]
fn a_bottom_snapped_resize_whose_input_center_sits_on_the_work_area_bottom_stays_silent() {
    for dpi in DPIS {
        let wa = right_wa(dpi);
        let old = SizePx {
            w: px(300, dpi),
            h: px(200, dpi),
        };
        // 旧位置は接地済み。新寸は背が高く、旧位置に当てると中心が下端の外（半開）へ出る。
        let pos = PointPx {
            x: wa.left + px(100, dpi),
            y: wa.bottom - old.h,
        };
        let new = SizePx {
            w: old.w,
            h: (wa.bottom - pos.y) * 2,
        };
        let layout = mixed_layout(dpi);

        // 探針の自己検査: 入力の中心は帰属しない（＝帰属だけを条件にすると鳴る形）が、
        // 入力矩形は work area と交差している（＝画面外ではない）。
        let raw = PointPx {
            x: pos.x + old.w / 2 - new.w / 2,
            y: pos.y,
        };
        let (_, input) = work_area_for_window_with_origin(&layout, win(raw, new))
            .expect("合成レイアウトは空でない");
        assert_eq!(
            input,
            WorkAreaResolution::NearestFallback,
            "dpi={dpi}: 探針の入力が帰属している＝偽陽性の形になっていない"
        );
        assert!(
            visible_in(&layout, raw, new),
            "dpi={dpi}: 探針の入力が本当に画面外＝偽陽性の形ではなく真陽性を見ている"
        );

        let mut world = World::new();
        world.insert_resource(layout);
        let e = world
            .spawn((
                fake_handle(0x1601),
                window_pos_sized(pos.x, pos.y, old.w, old.h),
                Anchored(Anchor::Bottom),
            ))
            .id();
        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::DpiReproject));
        assert!(ok, "dpi={dpi}: 書込が成立していない（前提が崩れている）");
        assert!(
            guard_events(&events, OFFSCREEN_PULL_TAG).is_empty(),
            "dpi={dpi}: 下端吸着の正常な resize で引き寄せの警告が出ている（偽陽性）: {events:?}"
        );
    }
}

/// **Req 3.3**: 位置決定に必要な入力（モニタ work area）が取得できない場合は、
/// 位置を変更せず現状のまま `warn!` を残す（架空の可視領域を発明しない）。
///
/// `MonitorSnapshot` 不在／空 snapshot のいずれでも、射影 T は identity へ縮退
/// 済みである＝ガードが位置へ手を入れないことが「現状維持」の内容になる。
#[test]
fn missing_work_area_holds_position_and_warns_on_non_drag_route() {
    for dpi in DPIS {
        for (label, snapshot) in [
            ("resource 不在", None),
            ("空 snapshot", Some(MonitorSnapshot { work_areas: vec![] })),
        ] {
            let new = narrow_char_size(dpi);
            let (mut world, e, old_pos) = gap_bound_char_world(dpi);
            world.remove_resource::<MonitorSnapshot>();
            if let Some(s) = snapshot {
                world.insert_resource(s);
            }
            // work area が無いときの射影は identity ＝ 手順 3b 後の raw そのもの。
            let old = wide_char_size(dpi);
            let identity = PointPx {
                x: old_pos.x + old.w / 2 - new.w / 2,
                y: old_pos.y,
            };

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));
            assert!(ok, "dpi={dpi} {label}: 寸の反映自体は従来どおり成立する");
            assert_eq!(
                point_of(&world, e),
                identity,
                "dpi={dpi} {label}: ガードが位置を動かした（現状維持の違反）"
            );

            let warned = expect_one(&events, UNRESOLVED_TAG);
            assert_eq!(
                warned.level,
                tracing::Level::WARN,
                "dpi={dpi} {label}: 入力欠落が warn として残っていない（Req 3.3）"
            );
            assert!(
                guard_events(&events, CLAMP_TAG).is_empty(),
                "dpi={dpi} {label}: work area 不明なのに clamp している: {events:?}"
            );
        }
    }
}

/// 適用外 route では、work area 不明であってもガードは 1 行も喋らない
/// （警告の出所が route 条件の**内側**にあることの檻）。
#[test]
fn missing_work_area_stays_silent_on_guard_exempt_routes() {
    for dpi in DPIS {
        let (mut world, e, _) = gap_bound_char_world(dpi);
        world.remove_resource::<MonitorSnapshot>();
        let (_, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                e,
                narrow_char_size(dpi),
                PlacementRoute::MoveCue,
            )
        });
        assert!(
            guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
            "dpi={dpi}: 適用外 route でガードが喋っている: {events:?}"
        );
    }
}

/// **旧矩形『不明』は `Option::None` だけではない**（[[4.6 の教訓]]）: wintf の
/// [`WindowPos::default`] は寸を `Some(CW_USEDEFAULT)`（＝`i32::MIN` センチネル）で
/// 持つ。これを素の矩形として交差判定へ入れると退化矩形が「もともと画面外に
/// 留置されていた」と誤判定され、**安全側 clamp の腕が丸ごと死ぬ**。
#[test]
fn undetermined_old_size_is_treated_as_unknown_rect_and_clamps() {
    for dpi in DPIS {
        let layout = mixed_layout(dpi);
        let new = narrow_char_size(dpi);
        // 手順 3b は旧寸が非正のとき付替えを行わない＝raw は現在位置そのもの。
        let raw = PointPx {
            x: gap_center_x(dpi) - new.w / 2,
            y: left_wa().bottom - new.h,
        };
        let mut world = World::new();
        world.insert_resource(mixed_layout(dpi));
        let e = world
            .spawn((
                fake_handle(0x1000),
                // 寸は `CW_USEDEFAULT` センチネルのまま（窓生成直後の実表現）。
                WindowPos {
                    position: Some(Point { x: raw.x, y: raw.y }),
                    ..Default::default()
                },
                Anchored(Anchor::Bottom),
            ))
            .id();

        // 探針の自己検査: 素の射影は不可視へ落ちる（＝安全側 clamp が要る状況）。
        let bare = project_anchor(Anchor::Bottom, raw, new, Some(&layout));
        assert!(
            !visible_in(&layout, bare, new),
            "dpi={dpi}: 探針が不動点——素の射影が既に可視"
        );

        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));
        assert!(ok);
        assert!(
            visible_in(&layout, point_of(&world, e), new),
            "dpi={dpi}: 寸未確定（センチネル）を『留置』と誤読して clamp を見送っている"
        );
        expect_one(&events, CLAMP_TAG);
    }
}

// -------------------------------------------------------------------------
// 位置の未確定表現（`CW_USEDEFAULT`）をキャラ窓経路でも打ち切る
// （task 6.3・S3 補・D15・Req 3.1/3.3）
//
// `resize_window_to` 手順 3 は `WindowPos.position` の `Option::None` しか縮退させて
// おらず、wintf 正典の**もう一つの未確定表現**（`CW_USEDEFAULT` ＝ `i32::MIN`・
// `WindowPos::default()` が position に持つ）を素通ししていた。素通しすると
//   ① 手順 3a の `old_rect` が `i32::MIN` 近傍の全 work area 非交差矩形になり、
//      `guard_visibility` が「もともと留置されていた」と誤読して `Keep` へ落ちる
//      ＝**6.1 が敷いた安全側 clamp の腕が黙って死ぬ**
//   ② 手順 3b の中央付替えと射影 T の入力（raw）も同時に汚染される
// D15 は (b) **resize 打ち切り**を採る——位置未確定は「保存すべき接地点が存在しない」
// ゆえ、`Option::None` と同じ腕（`warn!`＋`false`）へ合流させて①②を一括で断つ。
//
// 檻の要点（空虚化を避けるための自己検査を各檻が持つ）:
//   (1) 打ち切り檻の自己検査——**位置だけを実値に替えた対照窓**が同じ route・同じ寸で
//       確実に書込まで進むこと（進まないなら「打ち切れた」は何も意味しない）
//   (2) 書込ゼロの直接観測——`WindowPos` が呼出前後で**完全一致**（`PartialEq`）
//   (3) `warn!` ちょうど 1 件——ログ側の守りを位置 assert と二段構えにする
//       （[[5.2 の教訓＝空虚性 6 例目]]／[[6.2 の教訓＝檻の空虚性]]）
//   (4) **符号判定への変異の検出**——左モニタは `-1920..0` ＝負座標そのものは正当。
//       実在する負座標の窓が打ち切られないことを独立の檻で固定する
//
// なお寸センチネルとの**非対称は意図的**（D15 帰結⑴）: 寸未確定は接地点（位置）が
// 実在するので resize に意味があり、`old_rect` 不明の安全側 clamp で扱う
// （既存檻 `undetermined_old_size_is_treated_as_unknown_rect_and_clamps` が無改変で
// 緑のまま＝その非対称の檻を兼ねる）。
// -------------------------------------------------------------------------

/// wintf 正典の未確定センチネル（`== i32::MIN`）。**本体の import とは独立に**
/// 定義元から直接引き、判定式が正典と同式であることを檻側でも固定する
/// （`window_pos.rs:41`／`monitor_systems.rs:408` と同じ値）。
use windows::Win32::UI::WindowsAndMessaging::CW_USEDEFAULT as SENTINEL;

/// 手順 3 の位置センチネル打ち切りが名乗る語（**本体の文言とは独立に literal で置く**）。
const POSITION_SENTINEL_TAG: &str = "センチネル（位置未確定）";

/// 位置・寸を明示した単独キャラ窓の World（混在 DPI 合成レイアウト付き）。
fn char_world_with_window_pos(dpi: i32, position: Point, size: Option<SizeI>) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(mixed_layout(dpi));
    let e = world
        .spawn((
            fake_handle(0x1000),
            WindowPos {
                position: Some(position),
                size,
                ..Default::default()
            },
            Anchored(Anchor::Bottom),
        ))
        .id();
    (world, e)
}

/// 旧寸（[`wide_char_size`] の `SizeI` 表現）。
fn old_size_i(dpi: i32) -> SizeI {
    let s = wide_char_size(dpi);
    SizeI::new(s.w, s.h)
}

/// 左モニタ（**負座標** `-1920..0`）内の**実在する**接地位置。
///
/// 符号（`x < 0`）や大きさの閾値で未確定判定をすると、この正当な位置が巻き添えで
/// 打ち切られる＝檻 [`negative_real_position_is_not_aborted_and_still_resizes`] の被検体。
fn negative_real_pos(dpi: i32) -> Point {
    Point {
        x: left_wa().left / 2,
        y: left_wa().bottom - old_size_i(dpi).height,
    }
}

/// **探針の自己検査**: 位置**だけ**を実値に替えた対照窓は、同じ route・同じ新寸で
/// 必ず書込まで進む。これが崩れていると打ち切り檻の「何も起きなかった」は
/// センチネルの成果ではなく入力の不備になる（不動点の検出）。
fn assert_control_position_writes(dpi: i32, new: SizePx) {
    let (mut world, e) =
        char_world_with_window_pos(dpi, negative_real_pos(dpi), Some(old_size_i(dpi)));
    let before = *world.get::<WindowPos>(e).expect("WindowPos があるはず");
    assert!(
        resize_window_to(&mut world, e, new, PlacementRoute::Resnap),
        "dpi={dpi}: 探針が不動点——位置が実値の対照でも resize が成立しない"
    );
    assert_ne!(
        *world.get::<WindowPos>(e).expect("WindowPos があるはず"),
        before,
        "dpi={dpi}: 探針が不動点——対照でも WindowPos が 1 bit も変わらない"
    );
}

/// **位置がセンチネルの窓は log-first で打ち切る**（D15 採用案 (b)）: 戻り値 `false`・
/// `WindowPos` 書込ゼロ・`warn!` ちょうど 1 件。
///
/// 是正前はここで安全側 `ClampX` が走り、`clamp_x_into(i32::MIN, .., wa)` が返す
/// `wa.left` が**位置権威の無い窓へ書き込まれて**いた（＝位置権威の僭称）。
#[test]
fn undetermined_position_aborts_resize_without_writing() {
    for dpi in DPIS {
        let new = narrow_char_size(dpi);
        assert_control_position_writes(dpi, new);

        for (label, size) in [
            // `on_window_add` が挿す実表現そのもの（位置・寸とも未確定）。
            ("窓生成直後（位置・寸ともセンチネル）", None),
            // 寸だけ確定した窓＝汚染されるのは位置の側だけ、という切り分け。
            ("寸のみ確定・位置センチネル", Some(old_size_i(dpi))),
        ] {
            let position = Point {
                x: SENTINEL,
                y: SENTINEL,
            };
            let (mut world, e) = char_world_with_window_pos(dpi, position, size);
            // 探針の前提: 被検体が本当にセンチネルを持っている。
            assert_eq!(
                world
                    .get::<WindowPos>(e)
                    .expect("WindowPos があるはず")
                    .position,
                Some(position),
                "dpi={dpi} {label}: 探針がセンチネルを持っていない"
            );
            let before = *world.get::<WindowPos>(e).expect("WindowPos があるはず");

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));

            assert!(
                !ok,
                "dpi={dpi} {label}: 位置未確定（センチネル）なのに resize が成立している"
            );
            assert_eq!(
                *world.get::<WindowPos>(e).expect("WindowPos があるはず"),
                before,
                "dpi={dpi} {label}: 打ち切りのはずが WindowPos へ書き込まれている（Req 3.3 の現状維持違反）"
            );
            let warned = expect_one(&events, POSITION_SENTINEL_TAG);
            assert_eq!(
                warned.level,
                tracing::Level::WARN,
                "dpi={dpi} {label}: 打ち切りが warn として残っていない（log-first 違反）"
            );
            assert_eq!(
                warned.field("entity"),
                format!("{e:?}"),
                "dpi={dpi} {label}: 警告行が対象 entity を名乗っていない"
            );
            assert_eq!(
                warned.field("position"),
                format!("{position:?}"),
                "dpi={dpi} {label}: 警告行が問題の位置を載せていない"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} {label}: 打ち切ったのにガードが喋っている（射影 T の入力が汚染されている）: {events:?}"
            );
        }
    }
}

/// **負座標そのものは正当**（合成レイアウトの左モニタは `-1920..0`）。
///
/// 判定を符号（`x < 0`）や大きさの閾値へ変異させると、この実在位置の窓まで打ち切られる。
/// ゆえに本檻は「打ち切られない」ことを**位置の実値**で固定する（従来経路の非退行）。
#[test]
fn negative_real_position_is_not_aborted_and_still_resizes() {
    for dpi in DPIS {
        let start = negative_real_pos(dpi);
        let new = narrow_char_size(dpi);
        let layout = mixed_layout(dpi);
        // 探針の自己検査: ①本当に負座標であり ②センチネルではなく
        // ③旧矩形が実際に可視（＝「もともと留置」腕へ落ちない通常経路の入力）。
        assert!(start.x < 0, "dpi={dpi}: 探針が負座標になっていない");
        assert_ne!(start.x, SENTINEL, "dpi={dpi}: 探針がセンチネルと衝突している");
        assert!(
            visible_in(
                &layout,
                PointPx {
                    x: start.x,
                    y: start.y
                },
                wide_char_size(dpi)
            ),
            "dpi={dpi}: 探針の旧矩形が既に不可視——通常経路を通らない"
        );

        let (mut world, e) = char_world_with_window_pos(dpi, start, Some(old_size_i(dpi)));
        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));

        assert!(
            ok,
            "dpi={dpi}: 正当な負座標が打ち切られた（符号での未確定判定＝D15 が禁じた式）"
        );
        assert_eq!(
            point_of(&world, e),
            unguarded_projection(
                dpi,
                PointPx {
                    x: start.x,
                    y: start.y
                },
                new
            ),
            "dpi={dpi}: 負座標の従来経路（手順 3b＋射影 T）が退行している"
        );
        assert!(
            guard_events(&events, POSITION_SENTINEL_TAG).is_empty(),
            "dpi={dpi}: 正当な負座標に対してセンチネル警告が出ている: {events:?}"
        );
        assert!(
            guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
            "dpi={dpi}: 可視 → 可視の遷移でガードが喋っている: {events:?}"
        );
    }
}

/// **片軸だけ**のセンチネルも打ち切る（`pos.x == SENTINEL || pos.y == SENTINEL`）。
///
/// `&&` への変異（両軸そろったときだけ打ち切る）を検出する。y のみのセンチネルは
/// wintf 正典の `window_center` が見ていない軸であり、`||` にしてある理由が
/// 「接地点（下端中央）は x・y の**両方**が揃って初めて意味を持つ」ことである。
#[test]
fn single_axis_position_sentinel_also_aborts() {
    for dpi in DPIS {
        let new = narrow_char_size(dpi);
        let real = negative_real_pos(dpi);
        assert_control_position_writes(dpi, new);

        for (label, position) in [
            (
                "x のみセンチネル",
                Point {
                    x: SENTINEL,
                    y: real.y,
                },
            ),
            (
                "y のみセンチネル",
                Point {
                    x: real.x,
                    y: SENTINEL,
                },
            ),
        ] {
            let (mut world, e) =
                char_world_with_window_pos(dpi, position, Some(old_size_i(dpi)));
            let before = *world.get::<WindowPos>(e).expect("WindowPos があるはず");

            let (ok, events) =
                capture_logs(|| resize_window_to(&mut world, e, new, PlacementRoute::Resnap));

            assert!(
                !ok,
                "dpi={dpi} {label}: 片軸センチネルが打ち切られていない"
            );
            assert_eq!(
                *world.get::<WindowPos>(e).expect("WindowPos があるはず"),
                before,
                "dpi={dpi} {label}: 打ち切りのはずが WindowPos へ書き込まれている"
            );
            let warned = expect_one(&events, POSITION_SENTINEL_TAG);
            assert_eq!(
                warned.level,
                tracing::Level::WARN,
                "dpi={dpi} {label}: 打ち切りが warn として残っていない"
            );
            assert!(
                guard_events(&events, GUARD_TAG_PREFIX).is_empty(),
                "dpi={dpi} {label}: 打ち切ったのにガードが喋っている: {events:?}"
            );
        }
    }
}
