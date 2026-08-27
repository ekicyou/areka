use crate::placement::follow::OffsetBase;
use bevy_ecs::prelude::*;
use wintf::ecs::pointer::Phase;
use wintf::ecs::{Point, SizeI, WindowPos};

use super::super::test_support::{ExpectField, LogEvent, capture_logs, expect_one};
use super::test_support::{drag_end_event_at, drag_event, fake_handle, point_of, rect};
use super::{
    BalloonFollow, MonitorSnapshot, PlacementRoute, on_balloon_drag, on_balloon_drag_end,
    resize_window_to,
};
use crate::placement::config::BalloonXMode;
use crate::placement::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{BalloonKeywordBase, spawn_ghost_windows};

// -------------------------------------------------------------------------
// 実表示寸確定時のキーワード基本位置の再導出（要件 4.2/4.3/4.5/4.7/5.2・
// design C4／DD7・2026-08-14 の実機サインオフで見つかった欠陥の檻）
//
// # 何が抜けていたのか
//
// 既存の決定論テストは、配置を**ひと組の寸法**だけで組み立てていた——採寸した寸が
// そのまま表示される、という暗黙の前提である。実機ではその前提が崩れた:
// 採寸 434x687／実表示 382x547 で、中央揃えの `(char_w − balloon_w) / 2` が
// 採寸値のまま据え置かれ、バルーンが差の半分（26px）だけ横へずれた。ずれた位置は
// 作業領域の右端を越え、limit の関門がさらに引き戻したため画面上では大きく偏った。
//
// ゆえに本ファイルの檻はすべて「**採寸寸 A で組み、実表示寸 B で確定させる**」形を
// 持つ。A ≠ B であることが檻の生命線であり、探針の自己検査でそれを固定する。
//
// # 一度きりであること（要件 4.7）
//
// キーワードは「初期既定位置の供給」にとどまる。よって再導出は実表示寸が確定する
// 一度だけで、素材（`BalloonKeywordBase`）は消費して除去する。以後の寸法変化では
// `Side` と同じく配置時確定の静的 offset として振る舞う。
// -------------------------------------------------------------------------

/// 再導出ログの判定語（本体定数とは独立に檻側へ literal で置く既存規約）。
const REDERIVE_TAG: &str = "[balloon-keyword] Rederive";
/// 解決不能（縮退）の判定語。
const KEYWORD_UNRESOLVED_TAG: &str = "[balloon-keyword] Unresolved";

/// 作業領域（原点非 (0,0)・全辺 96 の非倍数＝隠れた再スケールの檻）。
fn wa() -> RectPx {
    rect(1013, 37, 2880, 1441)
}

fn snapshot() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![wa()],
    }
}

/// 採寸した寸（実機セッションの実測値）。
fn measured_char() -> SizePx {
    SizePx { w: 434, h: 687 }
}

/// 実際に表示された寸（実機セッションの実測値・採寸と**異なる**）。
fn displayed_char() -> SizePx {
    SizePx { w: 382, h: 547 }
}

fn balloon() -> SizePx {
    SizePx { w: 400, h: 224 }
}

/// キーワード基本位置のキャラ窓左上相対 offset（**檻側の独立実装**——本体の
/// `keyword_balloon_pos` を呼ばず、要件 4.2/4.3/4.4 の文言から直接書き下す）。
fn expected_offset(
    mode: BalloonXMode,
    char_size: SizePx,
    balloon_size: SizePx,
    adjust: PointPx,
) -> PointPx {
    let y = match mode {
        // バルーン下端がシェル画像上端に接する（4.2）
        BalloonXMode::CenterTop => -balloon_size.h,
        // バルーン上端がシェル画像下端に接する（4.3）
        BalloonXMode::CenterBottom => char_size.h,
        BalloonXMode::Side => unreachable!("Side はキーワード基本位置を持たない"),
    };
    PointPx {
        // 水平＝シェル画像の中央へバルーンの中央を揃える
        x: (char_size.w - balloon_size.w) / 2 + adjust.x,
        y: y + adjust.y,
    }
}

/// 採寸寸 A で組んだ 1 scope ぶんの配置（resolver 出力を模す）。
///
/// `keyword` が `Some` のとき、その scope はキーワード由来の初期既定位置を持つ
/// （`balloon_offset` は採寸寸 A から導いた値）。`None` は `Side`（数値指定・未指定）。
fn placement_measured_at(
    keyword: Option<(BalloonXMode, PointPx)>,
    balloon_limit: bool,
) -> ScopePlacement {
    let cs = measured_char();
    let bs = balloon();
    // P1/P2 の右下密着（キーワードでもキャラ窓の配置規則は不変・要件 5.3）
    let char_pos = PointPx {
        x: wa().right - cs.w,
        y: wa().bottom - cs.h,
    };
    let balloon_offset = match keyword {
        Some((mode, adjust)) => expected_offset(mode, cs, bs, adjust),
        // `Side` の既定（left 側・上端揃え）
        None => PointPx { x: -bs.w, y: 0 },
    };
    ScopePlacement {
        scope: 0,
        char_pos,
        char_size: cs,
        balloon_pos: PointPx {
            x: char_pos.x + balloon_offset.x,
            y: char_pos.y + balloon_offset.y,
        },
        balloon_size: bs,
        balloon_offset,
        balloon_offset_base: OffsetBase::unpinned(balloon_offset),
        balloon_limit,
        anchor: Anchor::Bottom,
        balloon_keyword_base: keyword,
    }
}

/// 実 spawn で World を組む（`BalloonKeywordBase` の付与も本番の構築経路そのもの）。
fn world_with(placement: ScopePlacement) -> (World, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(snapshot());
    let titles = GhostTitles::from_scope_titles([(0, "むらさき".to_string())]);
    let ghost_windows = spawn_ghost_windows(&mut world, &[placement], &titles);
    let char_window = ghost_windows
        .char_window(0)
        .expect("キャラ窓が spawn される");
    let balloon_window = ghost_windows
        .balloon_window(0)
        .expect("バルーン窓が spawn される");
    world.entity_mut(char_window).insert(fake_handle(0x1000));
    world.entity_mut(balloon_window).insert(fake_handle(0x2000));
    (world, char_window, balloon_window)
}

fn offset_of(world: &World, char_window: Entity) -> PointPx {
    world
        .get::<BalloonFollow>(char_window)
        .expect("キャラ窓に BalloonFollow があるはず")
        .offset
}

/// ユーザーのバルーン単独ドラッグを**本番のハンドラで**再現する（Component を手で
/// 書き換えない）。本欠陥は「ドラッグ経路が素材を退役させない」ことであり、
/// 手で `BalloonFollow.offset` を差し替えるとその経路をまるごと迂回してしまう。
///
/// 実 flow の順序どおり: wndproc（`DragConfig{move_window:true}`）が実窓を動かした
/// 状態を作る → [`on_balloon_drag`]（連続・in-session offset 更新）→
/// [`on_balloon_drag_end`]（確定・永続 write-through）。
fn user_drags_balloon_to(world: &mut World, balloon: Entity, to: Point) {
    world
        .get_mut::<WindowPos>(balloon)
        .expect("バルーン窓に WindowPos があるはず")
        .position = Some(to);
    let drag = Phase::Bubble(drag_event(balloon));
    assert!(
        !on_balloon_drag(world, balloon, balloon, &drag),
        "ドラッグ中ハンドラはイベントを消費しない"
    );
    let end = Phase::Bubble(drag_end_event_at(balloon, (to.x, to.y)));
    assert!(
        !on_balloon_drag_end(world, balloon, balloon, &end),
        "DragEnd ハンドラはイベントを消費しない"
    );
}

fn assert_no_event(events: &[LogEvent], needle: &str) {
    let hits: Vec<&str> = events
        .iter()
        .map(LogEvent::message)
        .filter(|m| m.contains(needle))
        .collect();
    assert!(
        hits.is_empty(),
        "`{needle}` を含むログが出ている（出てはならない）: {hits:?}"
    );
}

// -------------------------------------------------------------------------
// 探針の自己検査（檻が空虚にならないための前提）
// -------------------------------------------------------------------------

/// 採寸寸と実表示寸が**本当に違い**、キーワード offset も本当に変わること。
///
/// ここが崩れると以下の全檻が「再導出が起きなかったこと」を「再導出が正しかったこと」と
/// 取り違える。実機で観測された 26px のずれ（＝差の半分）も併せて固定する。
#[test]
fn probe_self_check_measured_and_displayed_sizes_really_differ() {
    let (a, b, bs) = (measured_char(), displayed_char(), balloon());
    assert_ne!(a, b, "採寸寸と実表示寸が同一（檻が空虚になる）");

    for mode in [BalloonXMode::CenterTop, BalloonXMode::CenterBottom] {
        let zero = PointPx { x: 0, y: 0 };
        let from_a = expected_offset(mode, a, bs, zero);
        let from_b = expected_offset(mode, b, bs, zero);
        assert_ne!(
            from_a, from_b,
            "mode={mode:?}: 採寸寸と実表示寸で offset が同じ（差が観測できない）"
        );
        assert_eq!(
            from_a.x - from_b.x,
            (a.w - b.w) / 2,
            "mode={mode:?}: 横ずれは寸法差の半分（実機で観測された 26px）"
        );
    }
    assert_eq!(
        (a.w - b.w) / 2,
        26,
        "実機で観測された横ずれ量（434 と 382 の差の半分）"
    );
}

// -------------------------------------------------------------------------
// (a) 実表示寸での再導出（本欠陥の核）
// -------------------------------------------------------------------------

/// 実表示寸が確定した書込で、キーワード由来の offset が**実表示寸から**導出し直される。
///
/// 中央上・中央下の両方を見るのは、中央下の垂直位置が `char_h` に依存するため
/// ——水平だけ直して垂直を採寸値のまま残す実装を弾く。
#[test]
fn reported_size_reconcile_rederives_the_keyword_offset_from_the_displayed_size() {
    for mode in [BalloonXMode::CenterTop, BalloonXMode::CenterBottom] {
        let adjust = PointPx { x: 0, y: 0 };
        let (mut world, char_window, balloon_window) =
            world_with(placement_measured_at(Some((mode, adjust)), false));

        let stale = offset_of(&world, char_window);
        assert_eq!(
            stale,
            expected_offset(mode, measured_char(), balloon(), adjust),
            "mode={mode:?}: 前提＝spawn 直後は採寸寸から導いた offset を持つ"
        );

        let (moved, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                displayed_char(),
                PlacementRoute::ReportedSizeReconcile,
            )
        });
        assert!(moved, "mode={mode:?}: 実表示寸の反映そのものは成立する");

        let fresh = expected_offset(mode, displayed_char(), balloon(), adjust);
        assert_eq!(
            offset_of(&world, char_window),
            fresh,
            "mode={mode:?}: 実表示寸から導出し直されていない（採寸寸のまま据え置き＝本欠陥）"
        );
        assert_ne!(
            fresh, stale,
            "mode={mode:?}: 再導出の前後が同値（檻が空虚）"
        );

        // 表示位置でも中央が揃っていること（offset だけ直って追従が古いままを弾く）。
        let char_pos = point_of(&world, char_window);
        let balloon_pos = point_of(&world, balloon_window);
        assert_eq!(
            balloon_pos.x + balloon().w / 2,
            char_pos.x + displayed_char().w / 2,
            "mode={mode:?}: 実表示寸のシェル画像中央にバルーン中央が揃っていない"
        );

        // 再導出は観測できる（scope・新旧 offset・新旧キャラ寸・バルーン寸）。
        let rec = expect_one(&events, REDERIVE_TAG);
        assert_eq!(rec.level, tracing::Level::INFO, "再導出の記録は info 水準");
        assert_eq!(
            rec.expect_field("scope"),
            "0",
            "再導出ログに scope が載っていない"
        );
        assert_eq!(
            rec.expect_field("old_offset"),
            format!("{stale:?}"),
            "再導出ログに再導出**前**の offset が載っていない"
        );
        assert_eq!(
            rec.expect_field("new_offset"),
            format!("{fresh:?}"),
            "再導出ログに再導出**後**の offset が載っていない"
        );
        // 採寸寸は「窓生成直後で読めない」ことがあり得るため `Option` のまま載る
        // （読めなかったことを `Some` へ潰して嘘をつかない）。
        assert_eq!(
            rec.expect_field("old_char_size"),
            format!("{:?}", Some(measured_char())),
            "再導出ログに採寸寸が載っていない"
        );
        assert_eq!(
            rec.expect_field("new_char_size"),
            format!("{:?}", displayed_char()),
            "再導出ログに実表示寸が載っていない"
        );
        assert_eq!(
            rec.expect_field("balloon_size"),
            format!("{:?}", balloon()),
            "再導出ログにバルーン寸が載っていない"
        );
    }
}

/// 作者指定の調整量（`windowposition.y` の数値・`balloon.offsetx/offsety`）は
/// 再導出でも保存される（4.4 の加算規則が再導出側で落ちない）。
#[test]
fn rederivation_keeps_the_author_adjustment() {
    let adjust = PointPx { x: 24, y: -32 };
    for mode in [BalloonXMode::CenterTop, BalloonXMode::CenterBottom] {
        let (mut world, char_window, _) =
            world_with(placement_measured_at(Some((mode, adjust)), false));

        resize_window_to(
            &mut world,
            char_window,
            displayed_char(),
            PlacementRoute::ReportedSizeReconcile,
        );

        assert_eq!(
            offset_of(&world, char_window),
            expected_offset(mode, displayed_char(), balloon(), adjust),
            "mode={mode:?}: 再導出で作者指定の調整量が落ちている（4.4）"
        );
    }
}

// -------------------------------------------------------------------------
// (b) 一度きりの消費（要件 4.7）
// -------------------------------------------------------------------------

/// 素材は最初の再導出で消費され除去される——**2 度目のリサイズは offset を動かさない**。
///
/// これがキーワード適用を「初期既定位置の供給」にとどめる（4.7）ことの実体である。
#[test]
fn keyword_base_is_consumed_by_the_first_rederivation_and_never_fires_again() {
    let adjust = PointPx { x: 0, y: 0 };
    let mode = BalloonXMode::CenterTop;
    let (mut world, char_window, _) =
        world_with(placement_measured_at(Some((mode, adjust)), false));

    assert!(
        world.get::<BalloonKeywordBase>(char_window).is_some(),
        "前提＝キーワード scope のキャラ窓は素材を持って spawn される"
    );

    resize_window_to(
        &mut world,
        char_window,
        displayed_char(),
        PlacementRoute::ReportedSizeReconcile,
    );
    let after_first = offset_of(&world, char_window);
    assert_eq!(
        after_first,
        expected_offset(mode, displayed_char(), balloon(), adjust),
        "1 度目で実表示寸から導出し直される"
    );
    assert!(
        world.get::<BalloonKeywordBase>(char_window).is_none(),
        "素材が消費されず残っている（一度きりでなくなる）"
    );

    // 2 度目: さらに別の寸へ変えても offset は動かない（Side と同じ静的 offset）。
    let third = SizePx { w: 300, h: 500 };
    let (_, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            char_window,
            third,
            PlacementRoute::ReportedSizeReconcile,
        )
    });

    assert_eq!(
        offset_of(&world, char_window),
        after_first,
        "2 度目のリサイズで offset が動いた（一度きりの契約に反する）"
    );
    assert_ne!(
        expected_offset(mode, third, balloon(), adjust),
        after_first,
        "3 つ目の寸が中央値を変えない寸になっている（檻が空虚）"
    );
    assert_no_event(&events, REDERIVE_TAG);
}

/// 寸が変わらない書込（アンカー変化で位置だけ引き直す等）は素材を**消費しない**。
///
/// 発火条件を route ではなく寸の変化に置いていることの檻である。route で絞ると
/// 「どの route が最初に実表示寸を運ぶか」という frame の相順の知識を配置層が持つことに
/// なり、相順が変われば静かに壊れる。逆に寸を見ずに消費すると、寸を伴わない再配置が
/// 素材を食い潰し、**本命の実表示寸確定が来たときには直す手段が無くなる**。
#[test]
fn a_write_that_does_not_change_the_size_preserves_the_material() {
    let adjust = PointPx { x: 0, y: 0 };
    let mode = BalloonXMode::CenterTop;
    let (mut world, char_window, _) =
        world_with(placement_measured_at(Some((mode, adjust)), false));
    let before = offset_of(&world, char_window);

    // 接地していない位置へずらす——こうしないと同寸・同位置のべき等 skip が手前で
    // 打ち切り、本檻が「ガードが効いた」ではなく「そもそも来なかった」で緑になる。
    world
        .get_mut::<WindowPos>(char_window)
        .expect("キャラ窓に WindowPos があるはず")
        .position = Some(Point {
        x: point_of(&world, char_window).x,
        y: wa().top,
    });

    // 同寸で呼ぶ（アンカー変化トリガが現在の表示寸で射影 T を引き直す形）。
    let (moved, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            char_window,
            measured_char(),
            PlacementRoute::AnchorChange,
        )
    });
    assert!(
        moved,
        "前提＝書込が実際に起きている（起きていなければ寸ガードを一度も踏んでいない）"
    );

    assert_no_event(&events, REDERIVE_TAG);
    assert_eq!(
        offset_of(&world, char_window),
        before,
        "寸が変わらない書込で offset が動いた"
    );
    assert!(
        world.get::<BalloonKeywordBase>(char_window).is_some(),
        "寸が変わらない書込で素材が消費された（本命の実表示寸確定で直せなくなる）"
    );

    // その後に本命（実表示寸）が来たら、ちゃんと導出し直される。
    resize_window_to(
        &mut world,
        char_window,
        displayed_char(),
        PlacementRoute::ReportedSizeReconcile,
    );
    assert_eq!(
        offset_of(&world, char_window),
        expected_offset(mode, displayed_char(), balloon(), adjust),
        "温存した素材が実表示寸確定で使われていない"
    );
}

/// 実表示寸を運ぶ route は 1 つではない——`DpiReproject`／`Resnap` が先に実表示寸を
/// 運んだ場合も、その寸で導出し直される（route 決め打ちにしていないことの檻）。
#[test]
fn any_route_that_first_brings_a_different_size_triggers_the_rederivation() {
    let adjust = PointPx { x: 0, y: 0 };
    let mode = BalloonXMode::CenterTop;
    for route in [PlacementRoute::DpiReproject, PlacementRoute::Resnap] {
        let (mut world, char_window, _) =
            world_with(placement_measured_at(Some((mode, adjust)), false));

        resize_window_to(&mut world, char_window, displayed_char(), route);

        assert_eq!(
            offset_of(&world, char_window),
            expected_offset(mode, displayed_char(), balloon(), adjust),
            "route={route:?}: 実表示寸を運んだのに導出し直されていない"
        );
    }
}

// -------------------------------------------------------------------------
// (c) `Side` の bit 同一（要件 4.5／5.2）
// -------------------------------------------------------------------------

/// `Side`（数値指定・未指定）の scope は素材を持たず、リサイズで offset が
/// **1 ビットも**変わらない。
#[test]
fn side_scopes_carry_no_keyword_base_and_resize_leaves_the_offset_bit_identical() {
    let (mut world, char_window, _) = world_with(placement_measured_at(None, false));

    assert!(
        world.get::<BalloonKeywordBase>(char_window).is_none(),
        "Side のキャラ窓に素材が付いている（4.5/5.2 の構造保証が壊れている）"
    );
    let before = offset_of(&world, char_window);

    let (_, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            char_window,
            displayed_char(),
            PlacementRoute::ReportedSizeReconcile,
        )
    });

    assert_eq!(
        offset_of(&world, char_window),
        before,
        "Side の offset がリサイズで変わった（数値指定の分岐は不変でなければならない）"
    );
    assert_no_event(&events, REDERIVE_TAG);
    assert_no_event(&events, KEYWORD_UNRESOLVED_TAG);
}

// -------------------------------------------------------------------------
// (d) 縮退（log-first・要件 6.3 の流儀）
// -------------------------------------------------------------------------

/// バルーンの現寸が解決できないときは `warn!` を残し、offset には**一切触れない**。
///
/// 未確定表現は `Option::None` だけではない——`CW_USEDEFAULT`（`i32::MIN` センチネル）も
/// 同じ腕へ落とす（`guard_balloon_position`／runtime 関門と同型のガード）。
#[test]
fn unresolvable_balloon_size_warns_and_leaves_the_offset_untouched() {
    let adjust = PointPx { x: 0, y: 0 };
    let mode = BalloonXMode::CenterTop;
    for (label, size) in [
        ("寸なし", None),
        ("センチネル", Some(SizeI::new(i32::MIN, i32::MIN))),
    ] {
        let (mut world, char_window, balloon_window) =
            world_with(placement_measured_at(Some((mode, adjust)), false));
        world
            .get_mut::<WindowPos>(balloon_window)
            .expect("バルーン窓に WindowPos があるはず")
            .size = size;

        let before = offset_of(&world, char_window);
        let (_, events) = capture_logs(|| {
            resize_window_to(
                &mut world,
                char_window,
                displayed_char(),
                PlacementRoute::ReportedSizeReconcile,
            )
        });

        let warn = expect_one(&events, KEYWORD_UNRESOLVED_TAG);
        assert_eq!(
            warn.level,
            tracing::Level::WARN,
            "[{label}] 縮退は warn 水準"
        );
        assert_no_event(&events, REDERIVE_TAG);
        assert_eq!(
            offset_of(&world, char_window),
            before,
            "[{label}] 解決不能なのに offset を書き換えた"
        );
        assert!(
            world.get::<BalloonKeywordBase>(char_window).is_some(),
            "[{label}] 導出できなかったのに素材を消費した（次の機会が失われる）"
        );
    }
}

// -------------------------------------------------------------------------
// (e) ユーザーが動かした相対位置が素材を退役させる（要件 4.7 の While 節・task 4.6）
//
// # なぜ起動時の規則だけでは足りなかったのか
//
// task 4.5 は「保存値が効いている scope には素材を付けない」を `persist::merge_scope`
// ——すなわち**起動時**——にだけ置いた。要件 4.7 は While 節（状態）であり、保存された
// 相対位置は**セッション中にも生まれる**（バルーン単独ドラッグの DragEnd 書込）。その
// 経路に対応する退役点が無いと、次の寸法変化でキーワード既定が利用者のドラッグを
// 上書きする。
//
// # これは隅の症状ではない
//
// 再解決は同寸の書込を skip する（`drain_resnap::resnap_from_sizes`）。採寸寸と実表示寸が
// 一致する**ふつうの**ゴーストでは素材が起動時に消費されず、装填されたまま残り続ける
// ——面替え・再吸着・DPI 移動のどれかが来た瞬間に発火する。
// -------------------------------------------------------------------------

/// 本欠陥の再現: 素材が残ったままユーザーがバルーンをずらし、その後にキャラ寸が
/// 変わると、キーワード既定がドラッグ結果を上書きする。
///
/// 前段で「採寸寸＝実表示寸」の同寸再解決を通しているのが要点である——これが
/// **ふつうのゴースト**の姿であり、素材が装填されたまま残る条件そのものである。
#[test]
fn a_dragged_relative_position_survives_a_later_char_size_change() {
    let adjust = PointPx { x: 0, y: 0 };
    let mode = BalloonXMode::CenterTop;
    let (mut world, char_window, balloon_window) =
        world_with(placement_measured_at(Some((mode, adjust)), false));

    // 実表示寸が採寸寸と一致するゴーストでは、最初の再解決が寸を変えないので素材は
    // 消費されない（同寸 skip）。ここが「装填されたまま」の出発点。
    resize_window_to(
        &mut world,
        char_window,
        measured_char(),
        PlacementRoute::ReportedSizeReconcile,
    );
    assert!(
        world.get::<BalloonKeywordBase>(char_window).is_some(),
        "前提＝同寸の再解決では素材が残る（ふつうのゴーストの姿）"
    );

    // ユーザーがバルーンをずらす（本番ハンドラ経路・DragEnd で永続の相対位置が生まれる）。
    let char_pos = point_of(&world, char_window);
    let dragged = PointPx { x: -173, y: -389 };
    user_drags_balloon_to(
        &mut world,
        balloon_window,
        Point {
            x: char_pos.x + dragged.x,
            y: char_pos.y + dragged.y,
        },
    );
    assert_eq!(
        offset_of(&world, char_window),
        dragged,
        "前提＝ドラッグで相対位置がユーザーの値になる"
    );

    // その後にキャラ寸が変わる書込（面替え・再吸着・DPI 移動のいずれでも起こる）。
    let (_, events) = capture_logs(|| {
        resize_window_to(
            &mut world,
            char_window,
            displayed_char(),
            PlacementRoute::ReportedSizeReconcile,
        )
    });

    let keyword_default = expected_offset(mode, displayed_char(), balloon(), adjust);
    assert_ne!(
        dragged, keyword_default,
        "檻が空虚（ドラッグ値とキーワード既定が同値では上書きを観測できない）"
    );
    assert_eq!(
        offset_of(&world, char_window),
        dragged,
        "ユーザーがずらした相対位置がキーワード既定へ引き戻された（要件 4.7 違反）"
    );
    assert_no_event(&events, REDERIVE_TAG);

    // 表示位置も追従はドラッグ値のまま（offset だけ守って窓が別位置に居る形を弾く）。
    let char_pos = point_of(&world, char_window);
    assert_eq!(
        point_of(&world, balloon_window),
        PointPx {
            x: char_pos.x + dragged.x,
            y: char_pos.y + dragged.y,
        },
        "バルーンの表示位置がドラッグ後の相対位置に従っていない"
    );
}

/// 退役点は DragEnd（＝永続の相対位置が生まれる観測点）である——素材はそこで除去される。
#[test]
fn the_balloon_drag_end_retires_the_keyword_material() {
    let adjust = PointPx { x: 0, y: 0 };
    let mode = BalloonXMode::CenterTop;
    let (mut world, char_window, balloon_window) =
        world_with(placement_measured_at(Some((mode, adjust)), false));

    assert!(
        world.get::<BalloonKeywordBase>(char_window).is_some(),
        "前提＝キーワード scope のキャラ窓は素材を持って spawn される"
    );

    let char_pos = point_of(&world, char_window);
    user_drags_balloon_to(
        &mut world,
        balloon_window,
        Point {
            x: char_pos.x - 40,
            y: char_pos.y - 260,
        },
    );

    assert!(
        world.get::<BalloonKeywordBase>(char_window).is_none(),
        "DragEnd が素材を退役させていない（起動時の規則だけでは要件 4.7 が成立しない）"
    );
}

/// `Side`（数値指定・未指定）のバルーンドラッグは本件について完全な no-op である
/// ——除去すべき素材がそもそも無い。**警告も panic も出してはならない**
/// （`BalloonLimit` と同じデータ駆動の流儀＝持たない窓は 1 bit も触らない）。
#[test]
fn a_drag_on_a_side_scope_is_a_no_op_for_the_material() {
    let (mut world, char_window, balloon_window) = world_with(placement_measured_at(None, false));
    assert!(
        world.get::<BalloonKeywordBase>(char_window).is_none(),
        "前提＝Side のキャラ窓に素材は付かない"
    );

    let char_pos = point_of(&world, char_window);
    let (_, events) = capture_logs(|| {
        user_drags_balloon_to(
            &mut world,
            balloon_window,
            Point {
                x: char_pos.x - 40,
                y: char_pos.y - 260,
            },
        )
    });

    // 捕捉ハーネスが本当に生きていることを、必ず**在ること**の主張で先に確かめる
    // （檻の外で握り潰されていると、以下の「無いこと」は恒真になる）。
    expect_one(&events, "PersistWiring 不在");
    assert_no_event(&events, KEYWORD_UNRESOLVED_TAG);
    assert_no_event(&events, REDERIVE_TAG);
    assert!(
        world.get::<BalloonKeywordBase>(char_window).is_none(),
        "Side の scope に素材が生えている"
    );
}
