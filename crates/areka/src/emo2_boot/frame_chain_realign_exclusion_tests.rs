//! 拡大率の後の詰め直しで動かさない組（外す理由）の檻
//! （`dpi-realign-remembered-chain` 要件 2・3・7.2・8.2）。
//!
//! # ここが押さえるもの
//!
//! - 変わる直前の矩形の控え（[`snapshot_char_rects`]）に、位置か大きさの無いキャラ窓と
//!   バルーン窓が入らないこと（要件 7.2 の「控えに無い窓」の入口）。
//!
//! - 組ごとの外す理由（離れていた・重なっていた・組の窓・ドラッグ中・武装の後の台本・
//!   控えに無い）で組を動かさないこと（要件 2.1〜2.4・3.1・7.2）。それぞれ同じテストの中に
//!   陽性の対（外す理由が無ければ詰まる）を置く——詰め直しを丸ごと止めても「動かない」だけなら
//!   恒真で通るからである。
//! - 解決の後の台本の移動を上書きせず 2 度目を走らせないこと（要件 3.2）、武装の前に台本で
//!   置かれ隣り合っていた窓は詰まること（要件 3.3）、ぱすたの形（同じ左上の組の窓）を横に
//!   並べないこと（要件 3.4）。
//!
//! 起動は記憶から戻した形（全スコープの既定の位置を外した形）で行うので、決めが既定の位置を
//! 読まないことも同時に押さえる。3 体が要る「控えに無い窓」は窓の一式を 3 スコープで組む。

use areka_emo_compose::ScaleRatio;
use bevy_ecs::change_detection::DetectChangesMut;
use wintf::ecs::drag::DraggingState;
use wintf::ecs::{Point, WindowPos};

use crate::emo2_boot::move_cue::{apply_move_directive, parse_move_directive};
use crate::placement::chain_finalize::{ADJACENT_TOLERANCE_PX, ChainRect, ScriptPlaced};
use crate::placement::chain_realign::{
    ChainRealignPending, arm_chain_realign, note_script_moved, realign_chain_once_with,
    snapshot_char_rects,
};
use crate::placement::sticky_group_ledger::seed_sticky_descript;

use super::chain_realign_adjacency_tests::{
    GAP_WITHOUT_REALIGN, boot_remembered_at_high, chain3_pos, chain3_scan, chain3_world, char_pos,
    gap, halve_keeping_bottom_center, low_sizes, lower_the_scale, place_char_window_x,
};
use super::test_support::{FakeReports, FrameHarness, capture_logs, pos_of, size_of};

/// 当該スコープのキャラ窓の今の矩形（位置と大きさが揃っている前提）。
fn char_rect(harness: &FrameHarness, scope: usize) -> ChainRect {
    let window = harness.char_window(scope);
    let pos = pos_of(&harness.world, window).expect("前提: キャラ窓に位置がある");
    let size = size_of(&harness.world, window).expect("前提: キャラ窓に大きさがある");
    ChainRect {
        x: pos.x,
        y: pos.y,
        w: size.width,
        h: size.height,
    }
}

/// 位置と大きさの揃ったキャラ窓は控えに入り、位置か大きさの無いキャラ窓は入らない。
///
/// 陽性の対（全員が揃っていれば全員が今の矩形で入る）を先に置くので、控えが常に空でも
/// 後半の「入らない」は通らない。
#[test]
fn a_char_window_without_position_or_size_is_left_out_of_the_snapshot() {
    let mut harness = FrameHarness::new();
    let expected: Vec<(usize, ChainRect)> = harness
        .scopes()
        .iter()
        .map(|&scope| (scope, char_rect(&harness, scope)))
        .collect();
    assert_eq!(expected.len(), 2, "前提: 2 スコープの窓がある");

    // 陽性の対: 全員が揃っていれば、全員がキャラ窓の今の矩形で控えに入る。
    assert_eq!(snapshot_char_rects(&harness.world), expected);

    // 位置の無いキャラ窓（scope 0）は控えに入らず、ほかのスコープは入ったまま。
    let char0 = harness.char_window(0);
    let char0_position = pos_of(&harness.world, char0);
    harness
        .world
        .get_mut::<WindowPos>(char0)
        .expect("前提: WindowPos がある")
        .position = None;
    assert_eq!(
        snapshot_char_rects(&harness.world),
        vec![expected[1]],
        "位置の無いキャラ窓が控えに入った（変わる直前の位置を発明してはならない・要件 7.2）"
    );

    // 大きさの無いキャラ窓（scope 1）も控えに入らない。
    let char1 = harness.char_window(1);
    harness
        .world
        .get_mut::<WindowPos>(char1)
        .expect("前提: WindowPos がある")
        .size = None;
    assert!(
        snapshot_char_rects(&harness.world).is_empty(),
        "大きさの無いキャラ窓が控えに入った（変わる直前の大きさを発明してはならない・要件 7.2）"
    );

    // `WindowPos` そのものが無いキャラ窓も控えに入らない（scope 0 を戻して対を作る）。
    harness
        .world
        .get_mut::<WindowPos>(char0)
        .expect("前提: WindowPos がある")
        .position = char0_position;
    assert_eq!(snapshot_char_rects(&harness.world), vec![expected[0]]);
    harness.world.entity_mut(char0).remove::<WindowPos>();
    assert!(
        snapshot_char_rects(&harness.world).is_empty(),
        "WindowPos の無いキャラ窓が控えに入った"
    );
}

/// バルーン窓は控えに入らない（詰め直しの組の要素はキャラ窓だけ）。
///
/// バルーン窓の矩形はキャラ窓と違う値を持つので、控えの矩形がキャラ窓の値と一致することで
/// バルーン窓を読んでいないことまで確かめる。
#[test]
fn balloon_windows_are_never_in_the_snapshot() {
    let harness = FrameHarness::new();
    let snapshot = snapshot_char_rects(&harness.world);
    let scopes: Vec<usize> = snapshot.iter().map(|&(scope, _)| scope).collect();
    assert_eq!(scopes, harness.scopes(), "スコープごとに 1 つだけ入る");
    for &(scope, rect) in &snapshot {
        let balloon = harness.balloon_window(scope);
        let balloon_size =
            size_of(&harness.world, balloon).expect("前提: バルーン窓に大きさがある");
        assert_ne!(
            (rect.w, rect.h),
            (balloon_size.width, balloon_size.height),
            "前提が崩れている: キャラ窓とバルーン窓が同じ大きさだと区別できない"
        );
        assert_eq!(rect, char_rect(&harness, scope), "控えはキャラ窓の矩形");
    }
}

// ---------------------------------------------------------------------------
// 組ごとの外す理由（要件 2・3・7.2）
// ---------------------------------------------------------------------------

/// 200% で記憶から戻した形に起動する（scope1 は scope0 の左に隣り合う）。
fn booted() -> (FrameHarness, FakeReports) {
    let mut harness = FrameHarness::new();
    let mut source = FakeReports::default();
    boot_remembered_at_high(&mut harness, &mut source);
    (harness, source)
}

/// 隙間が許す幅以内か（詰まったか）。
fn is_tight(harness: &FrameHarness) -> bool {
    gap(harness).abs() <= ADJACENT_TOLERANCE_PX
}

/// 外す理由の無い同じ駆動で組が詰まる（各テストの陽性の対）。
fn assert_positive_pair_tightens(what: &str) {
    let (mut harness, mut source) = booted();
    lower_the_scale(&mut harness, &mut source);
    assert_eq!(
        gap(&harness),
        GAP_WITHOUT_REALIGN,
        "前提が崩れている: 変わった直後に隙間が開いていない"
    );
    harness.run_chain_realign(&low_sizes());
    assert!(
        is_tight(&harness),
        "陽性の対が成立していない（{what}・隙間 {}）——詰め直しが死んでいれば「動かない」は空虚である",
        gap(&harness)
    );
}

/// 解決で scope1 のキャラ窓が横に動かないことを確かめる（直前の x と比べる）。
fn assert_scope1_kept(harness: &mut FrameHarness, what: &str) {
    let x_before = char_pos(harness, 1).0;
    harness.run_chain_realign(&low_sizes());
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_none(),
        "前提が崩れている: 解決が走っていない（{what}）"
    );
    assert_eq!(
        char_pos(harness, 1).0,
        x_before,
        "{what} の組を詰め直しが動かした"
    );
}

/// `\![move]` を `ScaleRatio::ONE` で当てる（本番の移動の口・`ScriptPlaced` を差し込む）。
fn apply_move(harness: &mut FrameHarness, scope: u32, tokens: &[&str]) {
    let tokens: Vec<String> = tokens.iter().map(|t| t.to_string()).collect();
    assert!(
        apply_move_directive(
            &mut harness.world,
            &parse_move_directive(scope, &tokens).directive,
            ScaleRatio::ONE
        ),
        "前提: 移動の指示が窓へ届く"
    );
}

/// 変わる直前に離れていた組は動かさない（要件 2.1）。
#[test]
fn a_pair_apart_before_the_change_is_not_moved() {
    assert_positive_pair_tightens("離れていない組");

    let (mut harness, mut source) = booted();
    let x = char_pos(&harness, 1).0;
    place_char_window_x(&mut harness, 1, x - 100);
    assert_eq!(gap(&harness), 100, "前提が崩れている: 離れていない");
    lower_the_scale(&mut harness, &mut source);
    assert_scope1_kept(&mut harness, "離れていた");
    assert!(!is_tight(&harness), "離れていた組が隣り合った");
}

/// 変わる直前に重なっていた組は動かさない（要件 2.2）。
#[test]
fn a_pair_overlapped_before_the_change_is_not_moved() {
    assert_positive_pair_tightens("重なっていない組");

    let (mut harness, mut source) = booted();
    let x = char_pos(&harness, 1).0;
    place_char_window_x(&mut harness, 1, x + 100);
    assert_eq!(gap(&harness), -100, "前提が崩れている: 重なっていない");
    lower_the_scale(&mut harness, &mut source);
    assert_scope1_kept(&mut harness, "重なっていた");
}

/// 窓の組（`StickyGroupLedger`）に入った窓は、隣り合っていても動かさない（要件 2.3）。
#[test]
fn a_window_in_a_sticky_group_is_not_moved() {
    assert_positive_pair_tightens("組の無い窓");

    let (mut harness, mut source) = booted();
    lower_the_scale(&mut harness, &mut source);
    seed_sticky_descript(&mut harness.world, Some("0,1"));
    assert_scope1_kept(&mut harness, "組の窓");
    assert_eq!(gap(&harness), GAP_WITHOUT_REALIGN, "組の窓の隙間が変わった");
}

/// ドラッグの最中の窓が組のどちら側にあっても、その組は動かさず寄せもしない（要件 2.4）。
#[test]
fn a_dragging_window_and_its_neighbour_are_not_moved() {
    assert_positive_pair_tightens("ドラッグしていない組");

    for (dragged, what) in [(1, "ドラッグ中の窓"), (0, "ドラッグ中の窓の隣")] {
        let (mut harness, mut source) = booted();
        lower_the_scale(&mut harness, &mut source);
        let window = harness.char_window(dragged);
        let start = pos_of(&harness.world, window).expect("位置がある");
        harness.world.entity_mut(window).insert(DraggingState {
            drag_start_pos: Point::new(start.x + 50, start.y + 50),
            initial_inset: (start.x as f32, start.y as f32),
        });
        let x0 = char_pos(&harness, 0).0;
        assert_scope1_kept(&mut harness, what);
        assert_eq!(char_pos(&harness, 0).0, x0, "{what}: 本体が動いた");
        assert_eq!(gap(&harness), GAP_WITHOUT_REALIGN, "{what}: 寄せた");
    }
}

/// 武装の後に `\![move]` で置いた窓は動かさない（要件 3.1）。
#[test]
fn a_window_moved_by_script_after_arming_is_not_moved() {
    assert_positive_pair_tightens("台本の置いていない窓");

    let (mut harness, mut source) = booted();
    lower_the_scale(&mut harness, &mut source);
    assert!(
        harness
            .world
            .get_resource::<ChainRealignPending>()
            .is_some(),
        "前提が崩れている: 武装していない"
    );
    apply_move(&mut harness, 1, &["--base=me", "--X=0", "--Y=0"]);
    assert_scope1_kept(&mut harness, "武装の後に `\\![move]` で置いた");
}

/// 武装の後に `\![execute,resetwindowpos]` で既定の位置へ戻した窓は動かさない
/// （要件 3.1・受け口 `note_script_moved`）。
#[test]
fn a_window_reset_by_script_after_arming_is_not_moved() {
    assert_positive_pair_tightens("既定の位置へ戻していない窓");

    let (mut harness, mut source) = booted();
    lower_the_scale(&mut harness, &mut source);
    let char1 = harness.char_window(1);
    note_script_moved(&mut harness.world, char1);
    assert_scope1_kept(&mut harness, "武装の後に既定の位置へ戻した");
}

/// 解決の後の `\![move]` は上書きされず、詰め直しは 2 度目を走らせない（要件 3.2）。
#[test]
fn a_script_move_after_resolution_is_not_overwritten() {
    let (mut harness, mut source) = booted();
    lower_the_scale(&mut harness, &mut source);
    harness.run_chain_realign(&low_sizes());
    // 陽性の対: 解決では組が詰まる。
    assert!(
        is_tight(&harness),
        "陽性の対が成立していない（解決で詰まっていない）"
    );

    apply_move(&mut harness, 1, &["--base=me", "--X=-100", "--Y=0"]);
    let moved = char_pos(&harness, 1);
    assert!(
        !is_tight(&harness),
        "前提が崩れている: 台本の移動で離れていない"
    );
    let _move_writes = harness.drain_writes();

    harness.run_chain_realign(&low_sizes());
    assert_eq!(
        char_pos(&harness, 1),
        moved,
        "解決の後の台本の移動を詰め直しが上書きした"
    );
    assert!(
        harness.drain_writes().is_empty(),
        "解決の後に詰め直しが 2 度目を走らせた"
    );
}

/// 武装の前に台本で置かれ隣り合っていた窓は、詰め直しの対象に入る（要件 3.3）。
#[test]
fn a_window_placed_by_script_before_arming_still_tightens() {
    let (mut harness, mut source) = booted();
    apply_move(&mut harness, 1, &["--base=me", "--X=0", "--Y=0"]);
    let char1 = harness.char_window(1);
    assert!(
        harness.world.get::<ScriptPlaced>(char1).is_some(),
        "前提が崩れている: 台本の印が無い"
    );
    assert_eq!(gap(&harness), 0, "前提が崩れている: 隣り合っていない");

    lower_the_scale(&mut harness, &mut source);
    assert_eq!(gap(&harness), GAP_WITHOUT_REALIGN);
    harness.run_chain_realign(&low_sizes());
    assert!(
        is_tight(&harness),
        "武装の前に台本で置かれた窓が詰まっていない（隙間 {}）",
        gap(&harness)
    );
}

/// 控えに無い窓に触れる組は動かさず理由つきの `warn!` を出し、ほかの組は詰める（要件 7.2）。
///
/// 3 体の鎖（x: 1000／700／500・幅 400／300／200）で scope2 の大きさを欠いたまま控え、
/// 控えの後で大きさを戻して半分にする。組 (0, 1) は陽性の対として 950 へ詰まり、
/// 組 (1, 2) は今の x 550 のまま残る。
#[test]
fn a_window_missing_from_the_snapshot_is_not_moved_and_others_tighten() {
    let (mut world, gw) = chain3_world();
    let char2 = gw.char_window(2).expect("キャラ窓");
    let size2 = world.get::<WindowPos>(char2).expect("WindowPos").size;
    world.get_mut::<WindowPos>(char2).expect("WindowPos").size = None;
    let before = snapshot_char_rects(&world);
    assert_eq!(
        before.iter().map(|&(s, _)| s).collect::<Vec<_>>(),
        vec![0, 1],
        "前提が崩れている: scope2 が控えに入った"
    );
    world.get_mut::<WindowPos>(char2).expect("WindowPos").size = size2;

    halve_keeping_bottom_center(&mut world, &gw);
    arm_chain_realign(&mut world, before);
    let logs = capture_logs(|| realign_chain_once_with(&mut world, |w| Ok(chain3_scan(w))));

    assert_eq!(
        (0..3)
            .map(|s| chain3_pos(&world, &gw, s).0)
            .collect::<Vec<_>>(),
        vec![1100, 950, 550],
        "控えに無い窓が動いたか、ほかの組が詰まっていない"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains("level=WARN") && l.contains("missing_scope=2")),
        "控えに無い窓の理由つきの警告が出ていない: {logs:?}"
    );
}

/// ぱすたの形（組の窓が同じ左上で重なる）は、詰め直しで横に並ばない（要件 3.4）。
///
/// 組に入っていても入っていなくても（重なりだけでも）並べない。
#[test]
fn the_pasta_shape_is_not_laid_side_by_side() {
    for sticky in [true, false] {
        let (mut harness, mut source) = booted();
        if sticky {
            seed_sticky_descript(&mut harness.world, Some("0,1"));
        }
        let top_left = pos_of(&harness.world, harness.char_window(0)).expect("位置がある");
        let char1 = harness.char_window(1);
        harness
            .world
            .get_mut::<WindowPos>(char1)
            .expect("WindowPos がある")
            .bypass_change_detection()
            .position = Some(top_left);

        lower_the_scale(&mut harness, &mut source);
        assert_scope1_kept(&mut harness, "ぱすたの形");
        let right_gap = char_pos(&harness, 1).0
            - (char_pos(&harness, 0).0
                + size_of(&harness.world, harness.char_window(0))
                    .expect("大きさがある")
                    .width);
        assert!(
            !is_tight(&harness) && right_gap.abs() > ADJACENT_TOLERANCE_PX,
            "ぱすたの形が横に並んだ（組 {sticky}）"
        );
    }
}
