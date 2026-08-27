use crate::placement::follow::OffsetBase;
use bevy_ecs::prelude::*;
use wintf::ecs::Point;
use wintf::ecs::pointer::Phase;

use super::test_support::{
    drag_event_at, dragging_state, fake_handle, odd_edge_snapshot, position_of,
    single_monitor_snapshot, window_pos_at, window_pos_sized,
};
use super::{
    Anchored, BalloonFollow, PlacementRoute, anchor_changed_system, move_window_to, on_char_drag,
    resize_window_keep_position, resize_window_to,
};
use crate::placement::resolver::{Anchor, PointPx, SizePx};

// -------------------------------------------------------------------------
// task 3.2: 消費側の存在確認と警告水準の区別（Req 6.2/6.3・design D8 消費側・
// design「guard_visibility > Implementation Notes > 消費側の区別」）
//
// 追従層の消費入口（[`resize_window_to`]／[`resize_window_keep_position`]）は
// **2 つの事象を混ぜてはならない**:
//   (a) entity 不在（既に despawn 済み）＝終了処理の正常系 → `debug!` で打ち切り
//   (b) entity は実在するが接地点規約の component（`Anchored`）が欠落＝真の異常 → `warn!`
// (a) を warn のままにすると終了時ログが良性ノイズで埋まり（Req 6.2 違反）、(b) を
// debug へ落とすと本物の結線バグが観測から消える。**同じ檻の中で両方**を見る。
// -------------------------------------------------------------------------

/// Req 6.2/6.3（追従層・キャラ窓入口）: despawn 済み entity への resize は正常終了系
/// として `debug!` 1 行で打ち切られ、**warn 以上を 1 行も出さない**。
#[test]
fn resize_window_to_on_despawned_entity_is_debug_only_normal_termination() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
        ))
        .id();
    world.despawn(window);

    let (ok, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap,
        )
    });

    assert!(!ok, "破棄済み窓へは書けない（false・panic しない）");
    // `tracing::Level` の Ord は ERROR < WARN < INFO < DEBUG < TRACE ゆえ
    // 「INFO より verbose」＝ debug/trace のみ、が静穏性の表現になる（spawn.rs T-V1 と同型）。
    assert!(
        events.iter().all(|e| e.level > tracing::Level::INFO),
        "破棄済み窓に対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
    );
    let skipped = expect_one(&events, DESPAWNED_SKIP_TAG);
    assert_eq!(
        skipped.level,
        tracing::Level::DEBUG,
        "破棄済みの打ち切りは debug 水準（正常終了系）"
    );
}

/// Req 6.2 の裏面（真の異常を殺さない）: **生存している** entity の接地点規約 component
/// （`Anchored`）欠落は従来どおり `warn!`。存在確認の導入でこちらまで静穏化してはならない。
#[test]
fn resize_window_to_missing_anchored_on_living_entity_still_warns() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            // Anchored なし（entity は実在する）
        ))
        .id();

    let (ok, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            window,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap,
        )
    });

    assert!(!ok, "Anchored 欠落は書かない（false）");
    let warned = expect_one(&events, "Anchored 未付与");
    assert_eq!(
        warned.level,
        tracing::Level::WARN,
        "実在 entity の Anchored 欠落は真の異常＝warn のまま（Req 6.2 の区別）"
    );
    assert!(
        !events
            .iter()
            .any(|e| e.message().contains(DESPAWNED_SKIP_TAG)),
        "実在 entity を『破棄済み』と誤判定している: {events:?}"
    );
}

/// Req 6.2/6.3（追従層・バルーン窓入口）: despawn 済み entity への位置据置きリサイズも
/// 正常終了系（`debug!`）として打ち切られ、warn 以上を出さない。
#[test]
fn resize_window_keep_position_on_despawned_entity_is_debug_only_normal_termination() {
    let mut world = World::new();
    let window = world
        .spawn((fake_handle(0x3000), window_pos_sized(731, 356, 434, 687)))
        .id();
    world.despawn(window);

    let (ok, events) =
        capture_logs(|| resize_window_keep_position(&mut world, window, SizePx { w: 517, h: 823 }));

    assert!(!ok, "破棄済み窓へは書けない（false・panic しない）");
    assert!(
        events.iter().all(|e| e.level > tracing::Level::INFO),
        "破棄済み窓に対して警告以上のログが出ている（Req 6.2 違反）: {events:?}"
    );
    let skipped = expect_one(&events, DESPAWNED_SKIP_TAG);
    assert_eq!(skipped.level, tracing::Level::DEBUG);
}

/// Req 6.2 の裏面（バルーン窓入口）: **生存している** entity の `WindowPos` 欠落
/// （窓生成前の異常系）は従来どおり `warn!`。
#[test]
fn resize_window_keep_position_missing_window_pos_on_living_entity_still_warns() {
    let mut world = World::new();
    let window = world.spawn(fake_handle(0x3000)).id(); // WindowPos なし・entity は実在

    let (ok, events) =
        capture_logs(|| resize_window_keep_position(&mut world, window, SizePx { w: 517, h: 823 }));

    assert!(!ok);
    let warned = expect_one(&events, "WindowPos 未付与");
    assert_eq!(
        warned.level,
        tracing::Level::WARN,
        "実在 entity の WindowPos 欠落は真の異常＝warn のまま"
    );
}

// -------------------------------------------------------------------------
// 窓移動レコード（Req 1.2／2.4・task 1.4・design「placement::diag > Invariants」
// ＋「PlacementRoute 配管＋guard_visibility > Integration」・D11）
//
// 単一ライター `enqueue_window_set_pos` の**書込成功時**に 1 レコードを専用 target
// （`areka::placement::diag`）へ出す。檻の要点:
//   (1) 経路名が呼出点と 1:1（route を取り違えたら赤）
//   (2) route・entity・種別・scope・位置・寸・DPI の**全フィールド**が揃う
//       （entity は wintf 側ログとの結合キーゆえ必ず入る＝Req 1.9 の 2 段 grep 条件）
//   (3) 書込が起きない経路（べき等 skip・`WindowHandle` 未付与）ではレコードが出ない
//   (4) 既定 `RUST_LOG=info` では 1 行も出ない（Req 1.7）
//
// 観測境界は tracing イベント本体（`test_support::capture_logs`）——本レコードは
// `WindowPos` ミラーと違い「書込が起きた事実」そのものの証跡だからである。
// 座標・寸・DPI は 96 の非倍数／非既定値を使い、取り違えを差で炙り出す。
// -------------------------------------------------------------------------

use std::sync::{Arc, Mutex};

use tracing_subscriber::EnvFilter;
use wintf::ecs::DPI;

use super::super::diag::{DESPAWNED_SKIP_TAG, WINDOW_MOVE_RECORD_TAG};
use super::super::spawn::{BalloonWindowMarker, CharWindowMarker};
use super::super::test_support::{LogEvent, capture_logs, ensure_interest_probes, expect_one};

/// 捕捉イベントから窓移動レコード行だけを抜く（他の debug ログは無視）。
fn window_move_lines(events: &[LogEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| e.message().to_string())
        .filter(|m| m.starts_with(WINDOW_MOVE_RECORD_TAG))
        .collect()
}

/// ちょうど 1 行の窓移動レコードを取り出す（0 件・複数件は落とす）。
fn only_window_move_line(events: &[LogEvent]) -> String {
    let lines = window_move_lines(events);
    assert_eq!(
        lines.len(),
        1,
        "窓移動レコードがちょうど 1 行ではない: {lines:?} / all={events:?}"
    );
    lines.into_iter().next().expect("1 件あることは検査済み")
}

/// 釘付け済みキャラ窓（marker/DPI 付き）1 枚だけの World。
///
/// `DPI` は **`WindowHandle` 付与の後**に入れる——wintf の `WindowHandle` on_add フックが
/// `GetDpiForWindow` を引き（偽 HWND では失敗＝96）`DPI` を上書きするため、同一 spawn の
/// タプルへ混ぜると意図した DPI が 96 に潰れる（混在 DPI の檻が自己整合で無力化する罠）。
fn char_window_world(scope: usize, dpi: u16) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope },
        ))
        .id();
    world.entity_mut(e).insert(DPI::from_dpi(dpi, dpi));
    (world, e)
}

/// (2) 全フィールドの檻: 書込成功で**ちょうど 1 行**、route・entity・kind・scope・
/// 物理位置・物理寸・DPI が揃う（1 つでも落ちたら赤）。
#[test]
fn window_move_record_carries_route_entity_kind_scope_position_size_and_dpi() {
    let (mut world, e) = char_window_world(1, 192);

    let (ok, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            e,
            SizePx { w: 517, h: 823 },
            PlacementRoute::DpiReproject,
        )
    });
    assert!(ok, "前提: 書込は成立する");

    // 期待値は resize_window_to の既存檻と同一の導出（下端中央保持 x=690・Y=1043−823）。
    assert_eq!(
        only_window_move_line(&events),
        format!(
            "[diag.window_move] route=DpiReproject entity={e:?} kind=char scope=1 \
             x=690 y=220 w=517 h=823 dpi=192"
        )
    );
}

/// (2) 結合キーの檻: entity は wintf 側ログ（`entity = ?e`＝`Debug` 表現・scope を
/// 持たない）と同一表現で出る——Req 1.9 の scope 別計数（2 段 grep）の成立条件。
#[test]
fn window_move_record_entity_matches_wintf_debug_rendering() {
    let (mut world, e) = char_window_world(0, 120);

    let (_, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            e,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap,
        )
    });
    let line = only_window_move_line(&events);
    assert!(
        line.contains(&format!("entity={e:?}")),
        "wintf 側ログと結合できる Debug 表現になっていない: {line}"
    );
    assert!(line.contains("scope=0") && line.contains("kind=char"));
}

/// (1) 経路名は**呼出側が渡した route と 1:1**（`resize_window_to` は 3 経路の共通
/// 反映口ゆえ、ここを取り違えると書き手の名指し＝Req 2.4 が丸ごと嘘になる）。
#[test]
fn window_move_record_route_follows_the_argument_of_the_shared_resize_entry() {
    for route in [
        PlacementRoute::AnchorChange,
        PlacementRoute::Resnap,
        PlacementRoute::DpiReproject,
    ] {
        let (mut world, e) = char_window_world(0, 96);
        let (ok, events) =
            capture_logs(|| resize_window_to(&mut world, e, SizePx { w: 517, h: 823 }, route));
        assert!(ok);
        let line = only_window_move_line(&events);
        assert!(
            line.contains(&format!("route={}", route.as_str())),
            "route={route} を渡したのにレコードが一致しない: {line}"
        );
        // 他 9 経路の語が混ざらない（取り違えの檻）。
        for other in PlacementRoute::ALL {
            if other == route {
                continue;
            }
            assert!(
                !line.contains(&format!("route={}", other.as_str())),
                "route={other} が混入: {line}"
            );
        }
    }
}

/// (1) 呼出点割当の檻: アンカー変化トリガ（`anchor_changed_system`）は
/// `AnchorChange` を渡す（system 側の割当ミスを検出する）。
#[test]
fn anchor_changed_system_records_the_anchor_change_route() {
    let mut world = World::new();
    world.insert_resource(odd_edge_snapshot()); // rect(53, 37, 1877, 1043)
    let e = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 1 },
        ))
        .id();
    world.entity_mut(e).insert(DPI::from_dpi(120, 120)); // on_add フックの後に入れる
    let mut schedule = Schedule::default();
    schedule.add_systems(anchor_changed_system);
    // 初回 run はべき等 skip（＝レコードも出ない＝(3) の裏取りも兼ねる）。
    let (_, first) = capture_logs(|| schedule.run(&mut world));
    assert!(
        window_move_lines(&first).is_empty(),
        "べき等 skip でレコードが出た: {first:?}"
    );

    world.get_mut::<Anchored>(e).unwrap().0 = Anchor::Top;
    let (_, second) = capture_logs(|| schedule.run(&mut world));
    let line = only_window_move_line(&second);
    assert!(
        line.contains("route=AnchorChange"),
        "アンカー変化の書込が AnchorChange として記録されない: {line}"
    );
    assert!(line.contains("y=37") && line.contains("dpi=120"), "{line}");
}

/// (1) 呼出点割当の檻: バルーン窓の位置据置きリサイズは `KeepPositionResize`。
/// 種別・scope はバルーン marker から読む（キャラと取り違えない）。
#[test]
fn resize_window_keep_position_records_the_keep_position_route() {
    let mut world = World::new();
    let window = world
        .spawn((
            fake_handle(0x3000),
            window_pos_sized(731, 356, 434, 687),
            BalloonWindowMarker { scope: 1 },
        ))
        .id();
    world.entity_mut(window).insert(DPI::from_dpi(192, 192)); // on_add フックの後に入れる

    let (ok, events) =
        capture_logs(|| resize_window_keep_position(&mut world, window, SizePx { w: 517, h: 823 }));
    assert!(ok);
    assert_eq!(
        only_window_move_line(&events),
        format!(
            "[diag.window_move] route=KeepPositionResize entity={window:?} kind=balloon \
             scope=1 x=731 y=356 w=517 h=823 dpi=192"
        )
    );
}

/// (1)(2) `\![move]` cue（[`move_window_to`]）は**対象窓を `MoveCue`**・**随伴バルーンを
/// `BalloonFollow`** として記録する（D13: スクリプト明示移動は固有の経路語を持つ＝Q3
/// 「ドラッグ以外の経路での消失」の観測穴を塞ぐ）。移動専用ゆえ寸は番兵（`w=-`／`h=-`）で
/// 欠落させない（フィールド語彙は経路によらず不変）。
#[test]
fn move_cue_write_is_recorded_as_move_cue_with_a_balloon_follow_companion() {
    let mut world = World::new();
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(180, 383),
            BalloonWindowMarker { scope: 0 },
        ))
        .id();
    // `DPI` 未付与の窓（component 欠落の防御経路）を作る——`WindowHandle` on_add フックが
    // 常に `DPI` を挿すため、番兵 `dpi=-` を単一ライター越しに固定するには外す必要がある。
    world.entity_mut(balloon).remove::<DPI>();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(731, 356, 434, 687),
            CharWindowMarker { scope: 0 },
            BalloonFollow::new(balloon, OffsetBase::unpinned(PointPx { x: -551, y: 27 })),
        ))
        .id();
    // 96 非倍数の DPI を明示付与（on_add フックの後に入れる＝96 へ潰されない）。
    world
        .entity_mut(char_window)
        .insert(DPI::from_dpi(120, 120));

    let (ok, events) = capture_logs(|| move_window_to(&mut world, char_window, 999, 777));
    assert!(ok);
    // 対象窓＝MoveCue／随伴バルーン＝BalloonFollow の 2 行（発行順＝書込順）。
    assert_eq!(
        window_move_lines(&events),
        vec![
            format!(
                "[diag.window_move] route=MoveCue entity={char_window:?} kind=char scope=0 \
                 x=999 y=777 w=- h=- dpi=120"
            ),
            format!(
                "[diag.window_move] route=BalloonFollow entity={balloon:?} kind=balloon scope=0 \
                 x=448 y=804 w=- h=- dpi=-"
            ),
        ]
    );
    // 位置自体は従来どおり両方書かれている（挙動不変の裏取り）。
    assert_eq!(position_of(&world, char_window), Point { x: 999, y: 777 });
    assert_eq!(position_of(&world, balloon), Point { x: 448, y: 804 });
}

/// (1) ドラッグ経路（連続イベント）はキャラ窓の書込を記録しない一方、随伴バルーンは
/// `BalloonFollow` として記録される（Req 2.5「バルーン消失は追従の随伴か」の判別材料）。
#[test]
fn drag_path_records_only_the_balloon_follow_write() {
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot()); // 下端 1043
    let balloon = world
        .spawn((
            fake_handle(0x2000),
            window_pos_at(180, 383),
            BalloonWindowMarker { scope: 0 },
        ))
        .id();
    let char_window = world
        .spawn((
            fake_handle(0x1000),
            window_pos_sized(1207, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 0 },
            BalloonFollow::new(balloon, OffsetBase::unpinned(PointPx { x: -551, y: 27 })),
            dragging_state((1207, 356), (1300, 500)),
        ))
        .id();

    let ev = Phase::Bubble(drag_event_at(char_window, (1300, 500), (1450, 520)));
    let (_, events) = capture_logs(|| on_char_drag(&mut world, char_window, char_window, &ev));

    let lines = window_move_lines(&events);
    assert_eq!(
        lines.len(),
        1,
        "ドラッグ 1 イベントの記録は随伴 1 行: {lines:?}"
    );
    assert!(
        lines[0].contains("route=BalloonFollow")
            && lines[0].contains(&format!("entity={balloon:?}")),
        "{lines:?}"
    );
    assert!(
        !lines[0].contains(&format!("entity={char_window:?}")),
        "ドラッグ経路のキャラ窓書込は本 target を通らない（wintf `[drag]` の所有）: {lines:?}"
    );
}

/// (3) 書込が起きなければレコードも出ない: べき等 skip（同寸・同位置）と
/// `WindowHandle` 未付与（失敗）の双方で 0 行。
#[test]
fn no_window_move_record_when_nothing_is_written() {
    // べき等 skip（Req3.1）
    let (mut world, e) = char_window_world(0, 120);
    let (wrote, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            e,
            SizePx { w: 434, h: 687 },
            PlacementRoute::Resnap,
        )
    });
    assert!(!wrote, "前提: 同寸・同位置はべき等 skip");
    assert!(
        window_move_lines(&events).is_empty(),
        "書込ゼロなのにレコードが出た: {events:?}"
    );

    // WindowHandle 未付与（Req3.3・enqueue が warn＋false）
    let mut world = World::new();
    world.insert_resource(single_monitor_snapshot());
    let no_handle = world
        .spawn((
            window_pos_sized(731, 356, 434, 687),
            Anchored(Anchor::Bottom),
            CharWindowMarker { scope: 0 },
        ))
        .id();
    let (wrote, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            no_handle,
            SizePx { w: 517, h: 823 },
            PlacementRoute::Resnap,
        )
    });
    assert!(!wrote);
    assert!(
        window_move_lines(&events).is_empty(),
        "失敗経路でレコードが出た: {events:?}"
    );
}

/// 与えた `RUST_LOG` 相当 directive で実際に濾した出力を集める（diag.rs の
/// `emit_all_under_filter` と同型——こちらは**単一ライター経由**で点灯を確かめる）。
fn window_move_output_under_filter(directives: &str) -> String {
    ensure_interest_probes();

    #[derive(Clone)]
    struct VecWriter(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for VecWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("捕捉バッファの毒化なし")
                .extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(directives))
        .with_ansi(false)
        .with_writer(move || VecWriter(sink.clone()))
        .finish();

    let (mut world, e) = char_window_world(1, 192);
    tracing::subscriber::with_default(subscriber, || {
        assert!(resize_window_to(
            &mut world,
            e,
            SizePx { w: 517, h: 823 },
            PlacementRoute::DpiReproject
        ));
    });

    String::from_utf8(buf.lock().expect("捕捉バッファの毒化なし").clone()).expect("UTF-8")
}

/// (4) 既定 `RUST_LOG=info`（`main.rs` のフォールバック）では窓移動レコードが
/// **1 行も出ない**（Req 1.7・恒久計装の既定 OFF）。
#[test]
fn window_move_records_are_silent_under_default_info_filter() {
    let out = window_move_output_under_filter("info");
    assert!(
        !out.contains(WINDOW_MOVE_RECORD_TAG),
        "既定 RUST_LOG=info で窓移動レコードが漏れている（Req 1.7 違反）: {out}"
    );
}

/// (4) 手順書の directive（`areka::placement::diag=debug`）で点灯する
/// ＝単一ライター経由でも target が手順書と 1:1 で結ばれている（Req 1.5/1.7）。
#[test]
fn window_move_records_light_up_under_the_procedure_directive() {
    let out = window_move_output_under_filter("info,areka::placement::diag=debug");
    assert!(
        out.contains(WINDOW_MOVE_RECORD_TAG) && out.contains("route=DpiReproject"),
        "手順書の RUST_LOG で単一ライターのレコードが点灯しない: {out}"
    );
}
