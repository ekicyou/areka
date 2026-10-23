//! 拡大率の後の詰め直しで動かさない組（外す理由）の檻
//! （`dpi-realign-remembered-chain` 要件 2・3・7.2・8.2）。
//!
//! # ここが押さえるもの
//!
//! - 変わる直前の矩形の控え（[`snapshot_char_rects`]）に、位置か大きさの無いキャラ窓と
//!   バルーン窓が入らないこと（要件 7.2 の「控えに無い窓」の入口）。
//!
//! 組ごとの外す理由（離れていた・重なっていた・組の窓・ドラッグ中・台本・控えに無い）は
//! 同じファイルへ足す（タスク 2.5）。

use wintf::ecs::WindowPos;

use crate::placement::chain_finalize::ChainRect;
use crate::placement::chain_realign::snapshot_char_rects;

use super::test_support::{FrameHarness, pos_of, size_of};

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
