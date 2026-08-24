use crate::placement::diag::DESPAWNED_SKIP_TAG;
use crate::placement::follow::BalloonFollow;
use crate::placement::test_support::{capture_logs as capture_diag_logs, expect_one};
use areka_emo_compose::ScaleRatio;
use bevy_ecs::prelude::Entity;
use wintf::ecs::DPI;
use wintf::ecs::WindowPos;

use super::test_support::{
    FakeReports, WRITER_WITNESS, arrangement_offset_of, assert_no_write, dpi_world, pos_of,
    reset_write_witness, s2_assert_work_area_bottom_moves, s2_ground_point, s2_snapshot,
    s2_work_area_for_dpi, size_of, window_move_lines, window_move_routes_of,
};
use super::*;

// ── task 5.2: S2 是正（位置の権威と寸の権威の分離）の檻 ──────────────────────
//
// 上の `s2_red_*` 4 件が「接地点が保たれる」ことを結果として主張するのに対し、本ブロックは
// **分離そのもの**（寸を触らずに位置だけが射影を通ること・バルーンは自分では動かないこと・
// 入力が欠けたら現状維持でログを残すこと）を個別に固定する。

/// **分離の本体**: 再導出結果が得られない（`None`）走行で、**窓寸は一切変わらず**位置だけが
/// 変化後の work area へ再射影される（S2 是正・design D7・Req 4.1／4.2／4.6）。
///
/// 併せて Req 4.5 の**裏面**を固定する——`s2_dpi_phase_writes_nothing_*` が「接地点が成立して
/// いれば書かない」を主張するのに対し、本件は「**現位置が接地点規約に違反していれば書く**」を
/// 主張する。両者が対で初めて「書込が起きるのは規約違反のときだけ」が檻になる。
///
/// 随伴（Req 4.4）も同時に固定する: バルーン窓は自分の `None` 経路では動かず、
/// **キャラ窓確定後の追従**（`route=BalloonFollow`）だけが動かし、恒等式
/// `balloon − char ≡ BalloonFollow.offset` が保たれる。
#[test]
fn s2_none_report_path_reprojects_position_without_touching_size() {
    let (mut world, gw) = dpi_world();
    world.insert_resource(s2_snapshot(96));
    let char0 = gw.char_window(0).expect("char 窓がある");
    let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");
    let size_before = size_of(&world, char0).expect("char 寸がある");
    let ground_before = s2_ground_point(&world, char0);
    let balloon_before = pos_of(&world, balloon0).expect("balloon 位置がある");
    assert_eq!(
        ground_before.1,
        s2_work_area_for_dpi(96).bottom,
        "前提: 変化前は 96 の work area 下端へ接地している"
    );

    // 初回 run（`SystemState::new` の全窓マッチ）を「再導出結果なし」固定の報告源で消費する。
    let mut source = FakeReports::default();
    let mut state = None;
    dpi_phase_with(&mut source, &mut state, &mut world);
    reset_write_witness(&mut world, &gw);
    source.calls.clear();

    // 96→192: work area 下端が動き、窓 DPI も変わる。**寸の再導出結果は無いまま**。
    s2_assert_work_area_bottom_moves(96, 192);
    world.insert_resource(s2_snapshot(192));
    for e in [char0, balloon0] {
        world.entity_mut(e).insert(DPI::from_dpi(192, 192));
    }
    let (_, events) = capture_diag_logs(|| dpi_phase_with(&mut source, &mut state, &mut world));

    // 非空虚性: 両 target を実際に訪れ、いずれも `None` を受け取っている（空マップゆえ）。
    let refreshed = source.calls_of("refresh");
    assert!(
        refreshed.contains(&shell_target(0).0) && refreshed.contains(&balloon_target(0).0),
        "非空虚性: DPI 相が char／balloon の両 target を訪れていない: {refreshed:?}"
    );

    // 寸の権威は触られていない（分離の片側）。
    assert_eq!(
        size_of(&world, char0),
        Some(size_before),
        "None 経路では窓寸を変えない（前寸維持・Req 4.5 の寸側）"
    );
    // 位置の権威は独立に働く（分離のもう片側）。
    assert_eq!(
        s2_ground_point(&world, char0),
        (ground_before.0, s2_work_area_for_dpi(192).bottom),
        "None 経路でも接地点の X が保存され Y が変化後の work area 下端へ再射影される"
    );
    assert_eq!(
        window_move_routes_of(&events, char0),
        vec!["DpiReproject"],
        "None 経路の書込はちょうど 1 回・経路語は DpiReproject: {:?}",
        window_move_lines(&events)
    );

    // 随伴: バルーンは自分の None 経路では動かず、キャラ確定後の追従だけが動かす。
    assert_eq!(
        window_move_routes_of(&events, balloon0),
        vec!["BalloonFollow"],
        "バルーンを動かすのはキャラ窓確定後の追従のみ: {:?}",
        window_move_lines(&events)
    );
    let balloon_after = pos_of(&world, balloon0).expect("balloon 位置がある");
    assert_ne!(
        balloon_after, balloon_before,
        "非空虚性: 随伴でバルーンが実際に動いている（動かないなら恒等式は自明に成立する）"
    );
    let offset = world
        .get::<BalloonFollow>(char0)
        .expect("char 窓は BalloonFollow を持つ")
        .offset;
    let cp = pos_of(&world, char0).expect("char 位置がある");
    assert_eq!(
        (balloon_after.x - cp.x, balloon_after.y - cp.y),
        (offset.x, offset.y),
        "随伴恒等式 balloon − char ≡ BalloonFollow.offset が崩れている（Req 4.4）"
    );
}

/// **バルーン窓の `None` は位置据置き**（tasks 5.2 の明示制約・design D7）。
///
/// バルーンの位置は従属量ゆえ、バルーン自身の DPI 変化で位置を動かしてはならない
/// （動かすとキャラ確定前の位置へ一度飛び、同フレームの追従で再び動く＝二重書込）。
///
/// 「動かない」の主張が不動点に落ちないよう、⑴同一ハーネスがバルーンの書込を検出できること
/// を positive witness で先に示し、⑵**警告以上のログが 1 行も出ないこと**も併せて主張する
/// （`resize_window_to` へ流す誤実装はバルーンに `Anchored` が無いため `warn!` を出す＝
/// 書込ゼロだけを見ていると素通ししてしまう・記憶〈2.2 の空虚性の教訓〉）。
#[test]
fn s2_none_report_path_leaves_the_balloon_in_place() {
    // --- positive witness: 同一ハーネスはバルーンの書込を実際に検出できる ---
    {
        let (mut world, gw) = dpi_world();
        world.insert_resource(s2_snapshot(96));
        let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");
        let native = size_of(&world, balloon0).expect("balloon 寸がある");
        world.entity_mut(balloon0).insert(DPI::from_dpi(192, 192));
        let mut source = FakeReports::default();
        source.refresh.insert(
            balloon_target(0).0,
            ScaleRatio::new(192, 96)
                .expect("非ゼロ比")
                .scaled_extent(native.width as u32, native.height as u32),
        );
        let mut state = None;
        dpi_phase_with(&mut source, &mut state, &mut world);
        assert_ne!(
            arrangement_offset_of(&world, balloon0),
            WRITER_WITNESS,
            "positive witness: 異寸報告のあるバルーンは実際に書かれる（書込 witness が生きている証拠）"
        );
    }

    // --- 本題: work area が動いてもバルーン単独の None 経路では書かない ---
    let (mut world, gw) = dpi_world();
    world.insert_resource(s2_snapshot(96));
    let char0 = gw.char_window(0).expect("char 窓がある");
    let balloon0 = gw.balloon_window(0).expect("balloon 窓がある");
    let balloon_before = pos_of(&world, balloon0).expect("balloon 位置がある");

    let mut source = FakeReports::default(); // 「再導出結果なし」固定
    let mut state = None;
    dpi_phase_with(&mut source, &mut state, &mut world); // 初回 run（全窓マッチ）を消費
    reset_write_witness(&mut world, &gw);
    source.calls.clear();

    // work area を動かし、**バルーンだけ**の DPI を変える（char は Changed<DPI> にしない）。
    s2_assert_work_area_bottom_moves(96, 192);
    world.insert_resource(s2_snapshot(192));
    world.entity_mut(balloon0).insert(DPI::from_dpi(192, 192));
    let (_, events) = capture_diag_logs(|| dpi_phase_with(&mut source, &mut state, &mut world));

    assert_eq!(
        source.calls_of("refresh"),
        vec![balloon_target(0).0],
        "非空虚性: DPI 相はバルーン窓だけを訪れている（char は Changed<DPI> でない）: {:?}",
        source.calls
    );
    assert_no_write(&world, balloon0, "バルーン単独の None 経路（位置据置き）");
    assert_eq!(
        pos_of(&world, balloon0),
        Some(balloon_before),
        "バルーン位置は据え置かれる（随伴はキャラ窓確定後の追従が担う）"
    );
    assert_no_write(&world, char0, "Changed<DPI> でない char 窓");
    assert!(
        events.iter().all(|e| e.level > tracing::Level::INFO),
        "バルーンの位置据置きは正常系であり警告以上を出さない（`Anchored` 欠落 warn が出るなら射影経路へ流している）: {events:?}"
    );
}

/// **窓寸が未確定（窓生成前）なら現状維持のまま打ち切り、その事実をログに残す**
/// （tasks 5.2 の明示制約・記憶〈ログ無し失敗経路の禁止〉）。
///
/// 現寸が読めなければ「現寸のまま射影する」という是正そのものが成立しないため、位置を
/// 一切触らずに `warn!` で打ち切る（実在 entity ゆえ真の異常＝debug で黙らせない）。
/// 「書かない」の主張が空虚にならないよう、**同一条件で寸が読めれば書かれる**ことを
/// positive witness で先に示す。
#[test]
fn s2_none_report_path_holds_state_and_warns_when_the_window_size_is_undetermined() {
    /// 96 へ接地した World の初回 run を「再導出結果なし」で消費し、96→192 の変化を注入する。
    fn arm(
        world: &mut World,
        gw: &GhostWindows,
        char0: Entity,
    ) -> (FakeReports, Option<SystemState<DpiChangedQuery>>) {
        let mut source = FakeReports::default();
        let mut state = None;
        dpi_phase_with(&mut source, &mut state, world);
        // 初回 run（`SystemState::new` の全窓マッチ）は既に接地済みゆえべき等 skip で
        // 書込ゼロ——この前提が崩れると下の「現状維持」比較が別の理由で動いてしまう。
        assert_no_write(world, char0, "初回 run（既に 96 へ接地済み）");
        reset_write_witness(world, gw);
        source.calls.clear();
        world.insert_resource(s2_snapshot(192));
        world.entity_mut(char0).insert(DPI::from_dpi(192, 192));
        (source, state)
    }

    s2_assert_work_area_bottom_moves(96, 192);

    // --- positive witness: 寸が読める同一条件では実際に書かれる ---
    {
        let (mut world, gw) = dpi_world();
        world.insert_resource(s2_snapshot(96));
        let char0 = gw.char_window(0).expect("char 窓がある");
        let (mut source, mut state) = arm(&mut world, &gw, char0);
        dpi_phase_with(&mut source, &mut state, &mut world);
        assert_ne!(
            arrangement_offset_of(&world, char0),
            WRITER_WITNESS,
            "positive witness: 寸が読めれば None 経路でも位置は書かれる（この探針が書込を検出できる証拠）"
        );
    }

    // --- 本題: WindowPos.size が未確定なら現状維持＋warn ---
    let (mut world, gw) = dpi_world();
    world.insert_resource(s2_snapshot(96));
    let char0 = gw.char_window(0).expect("char 窓がある");
    let pos_before = pos_of(&world, char0).expect("char 位置がある");
    let (mut source, mut state) = arm(&mut world, &gw, char0);
    // 窓生成前の状態（位置はあるが寸が未確定）を作る。
    world
        .get_mut::<WindowPos>(char0)
        .expect("WindowPos がある")
        .size = None;

    let (_, events) = capture_diag_logs(|| dpi_phase_with(&mut source, &mut state, &mut world));

    assert!(
        source.calls_of("refresh").contains(&shell_target(0).0),
        "非空虚性: DPI 相は当該窓を実際に訪れている: {:?}",
        source.calls
    );
    assert_no_write(&world, char0, "窓寸が未確定の DPI 相（現状維持）");
    assert_eq!(
        pos_of(&world, char0),
        Some(pos_before),
        "窓寸が未確定なら位置も変えない（現状維持）"
    );
    let held = expect_one(&events, "WindowPos.size 未確定");
    assert_eq!(
        held.level,
        tracing::Level::WARN,
        "実在 entity の寸未確定は真の異常＝warn 水準（破棄済みの debug 打ち切りと混ぜない）"
    );
}

/// **破棄済み窓は正常終了系**（要件 6.2/6.3・3.2 が敷いた区別を新しい消費点へも適用する）。
///
/// 終了処理でゴースト窓が破棄された後のフレームでも DPI 相は走り得る。ここで
/// 「寸が読めない」を一律 `warn!` にすると終了時ログが良性ノイズで埋まり、上の
/// 「実在するが寸未確定」（真の異常）が読めなくなる。
#[test]
fn s2_reproject_on_despawned_entity_is_debug_only_normal_termination() {
    let (mut world, gw) = dpi_world();
    world.insert_resource(s2_snapshot(96));
    let char0 = gw.char_window(0).expect("char 窓がある");
    world.despawn(char0);

    let (wrote, events) = capture_diag_logs(|| {
        reproject_char_window_at_current_size(&mut world, char0, PlacementRoute::DpiReproject)
    });

    assert!(!wrote, "破棄済み窓へは書けない（false・panic しない）");
    // `tracing::Level` の Ord は ERROR < WARN < INFO < DEBUG < TRACE ゆえ
    // 「INFO より verbose」＝ debug/trace のみ、が静穏性の表現（follow.rs の同型檻と同じ流儀）。
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
