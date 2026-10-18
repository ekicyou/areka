//! 組を外す判定・組の操作列・持ち主の輪の確かめ・組の受け渡しの記録——決定論的テスト
//! （要件 1.1／2.1／4.1／4.4／4.5／6.4／7.1〜7.4／8.1）。
//!
//! 実機も実ディスプレイも World の巡回も要らない。`HWND` は Win32 へ 1 度も渡さない偽の値で
//! あり、`Entity` は空の `World` から順に採るだけの値である。持ち主を読む口は引数の閉包で
//! 差し替える（`check_owner_loop` は Win32 を呼ばない）。
//!
//! 記録の捕捉はスレッドローカルの差し替えなので、この檻は Schedule を一切使わない
//! （兄弟の `zorder_chain_tests.rs` と同じ規律）。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;
use windows::core::HRESULT;

use super::{
    ChainPlan, OWNER_WALK_LIMIT, OwnerLoopCheck, PairOp, PairReleased, PairState, check_owner_loop,
    log_chain_link_refused, log_chain_pair_released, log_chain_pair_restored,
    pair_released_by_chain, plan_pair_ops,
};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::zorder_chain_diag::{
    LinkRefusedReason, chain_record_tags, link_refused_line, pair_handover_tags,
    pair_released_line, pair_restored_line,
};

/// 実機サインオフが用いる `RUST_LOG` の鎖側の指定そのもの。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_chain=debug";

/// 鎖系の記録の出力先（`zorder_chain` の module path 既定）。
const CHAIN_TARGET: &str = "wintf::ecs::window::zorder_chain";

/// 行を組む純関数の層の出力先（ここから出てはならない）。
const DIAG_TARGET: &str = "wintf::ecs::window::zorder_chain_diag";

// ===========================================================================
// 道具立て
// ===========================================================================

fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

/// テスト用の Entity を n 個採り、`Entity` の順序で昇順に並べて返す（`e[0] < e[1] < …`）。
///
/// 確保の順と `Entity` の順序は一致しない（実測では後に確保した方が小さい）ので、
/// 並べてから返す。
fn entities(n: usize) -> Vec<Entity> {
    let mut world = World::new();
    let mut e: Vec<Entity> = (0..n).map(|_| world.spawn_empty().id()).collect();
    e.sort();
    e
}

/// `members` だけを持つ望む鎖（繋ぎの欄は組の判定に効かない）。
fn chain(members: &[Entity]) -> ChainPlan {
    ChainPlan {
        members: members.to_vec(),
        ..ChainPlan::default()
    }
}

fn pair(balloon: Entity, peer: Entity, released: bool) -> PairState {
    PairState {
        balloon,
        peer,
        released,
    }
}

fn fake_error() -> windows::core::Error {
    windows::core::Error::from(HRESULT(0x8007_0005u32 as i32))
}

/// 持ち主の表から読む口を作る（表に無い窓は持ち主なし＝null・`fail` に載る窓は読めない）。
fn reader(
    table: Vec<(usize, usize)>,
    fail: Vec<usize>,
) -> impl FnMut(HWND) -> windows::core::Result<HWND> {
    move |hwnd: HWND| {
        let v = hwnd.0 as usize;
        if fail.contains(&v) {
            return Err(fake_error());
        }
        Ok(table
            .iter()
            .find(|(owned, _)| *owned == v)
            .map(|(_, owner)| fake_hwnd(*owner))
            .unwrap_or(HWND(std::ptr::null_mut())))
    }
}

fn only_line_with<'a>(out: &'a str, tag: &str) -> &'a str {
    let found: Vec<&str> = out.lines().filter(|l| l.contains(tag)).collect();
    assert_eq!(found.len(), 1, "`{tag}` の行がちょうど 1 本ではない: {out}");
    found[0]
}

// ===========================================================================
// 組を外す判定
// ===========================================================================

/// 鎖が無ければ、どの組も外さない（要件 4.1——指定が無い間は組の重なりのまま）。
#[test]
fn no_chain_releases_no_pair() {
    let e = entities(2);
    assert!(!pair_released_by_chain(None, e[0], e[1]));
}

/// キャラクターがバルーンのすぐ奥なら外さない（要件 1.6／4.4——組の結び目が受け持つ）。
#[test]
fn a_contiguous_pair_is_kept() {
    let e = entities(4); // b0, s0, b1, s1
    let plan = chain(&[e[0], e[1], e[2], e[3]]);
    assert!(!pair_released_by_chain(Some(&plan), e[0], e[1]));
    assert!(!pair_released_by_chain(Some(&plan), e[2], e[3]));
}

/// 逆転（キャラクターがバルーンより手前）は外す（要件 1.1）。
#[test]
fn an_inverted_pair_is_released() {
    let e = entities(4); // s0, b0, b1, s1 （s0,b0,s1 の補い済み）
    let (s0, b0, b1, s1) = (e[0], e[1], e[2], e[3]);
    let plan = chain(&[s0, b0, b1, s1]);
    assert!(pair_released_by_chain(Some(&plan), b0, s0));
    // 同じ鎖の中の連続する組は外さない（要件 4.4）
    assert!(!pair_released_by_chain(Some(&plan), b1, s1));
}

/// 非隣接（2 枚の間に他の窓が挟まる）は外す（要件 2.1）。
#[test]
fn a_split_pair_is_released() {
    let e = entities(4); // b1, s0, s1, b0
    let (b1, s0, s1, b0) = (e[0], e[1], e[2], e[3]);
    let plan = chain(&[b1, s0, s1, b0]);
    assert!(pair_released_by_chain(Some(&plan), b1, s1));
    assert!(pair_released_by_chain(Some(&plan), b0, s0));
}

/// 片方が並びに居なければ外さない（どちらかの窓がまだ無い・もう無い）。
#[test]
fn a_pair_with_a_member_missing_from_the_chain_is_kept() {
    let e = entities(4);
    let (s0, b0, s1, outside) = (e[0], e[1], e[2], e[3]);
    let plan = chain(&[s0, b0, s1]);
    // バルーンが居ない
    assert!(!pair_released_by_chain(Some(&plan), outside, s1));
    // キャラクターが居ない（バルーンの次は在るが、相手ではない）
    assert!(!pair_released_by_chain(Some(&plan), b0, outside));
    // 対照: 両方が居て連続していなければ外す
    assert!(pair_released_by_chain(Some(&plan), b0, s0));
}

/// バルーンが鎖の末尾（すぐ奥が無い）でキャラクターが手前に居れば外す。
#[test]
fn a_balloon_at_the_tail_with_its_character_in_front_is_released() {
    let e = entities(2);
    let (s0, b0) = (e[0], e[1]);
    let plan = chain(&[s0, b0]);
    assert!(pair_released_by_chain(Some(&plan), b0, s0));
}

// ===========================================================================
// 組の操作列
// ===========================================================================

/// 印と望みが一致する組には操作を出さない。
#[test]
fn pairs_whose_mark_matches_the_wish_get_no_op() {
    let e = entities(4); // s0, b0, b1, s1
    let (s0, b0, b1, s1) = (e[0], e[1], e[2], e[3]);
    let plan = chain(&[s0, b0, b1, s1]);
    let ops = plan_pair_ops(Some(&plan), &[pair(b0, s0, true), pair(b1, s1, false)]);
    assert!(ops.is_empty(), "一致する組に操作が出ている: {ops:?}");
    // 鎖なし・印なしでも空
    assert!(plan_pair_ops(None, &[pair(b1, s1, false)]).is_empty());
    assert!(plan_pair_ops(None, &[]).is_empty());
}

/// 外しが全部先・結び直しが全部後、各群はバルーンの実体の昇順。
#[test]
fn releases_all_come_first_then_restores_each_in_ascending_balloon_order() {
    // 実体の昇順: e[0] < e[1] < ... < e[7]
    let e = entities(8);
    let (b0, s0, b1, s1, b2, s2, b3, s3) = (e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7]);
    // 望み: b0 と b2 は逆転（外す）、b1 と b3 は連続（外さない）
    let plan = chain(&[s2, b2, s0, b0, b1, s1, b3, s3]);
    // 現況: b1・b3 は印あり（結び直す）、b0・b2 は印なし（外す）。並びはわざと崩す
    let pairs = [
        pair(b3, s3, true),
        pair(b2, s2, false),
        pair(b1, s1, true),
        pair(b0, s0, false),
    ];
    let ops = plan_pair_ops(Some(&plan), &pairs);
    assert_eq!(
        ops,
        vec![
            PairOp::Release {
                balloon: b0,
                peer: s0
            },
            PairOp::Release {
                balloon: b2,
                peer: s2
            },
            PairOp::Restore {
                balloon: b1,
                peer: s1
            },
            PairOp::Restore {
                balloon: b3,
                peer: s3
            },
        ]
    );
}

/// 鎖が無くなれば、印の付いた組は全部結び直す（要件 4.5——解除で組の重なりへ戻る）。
#[test]
fn when_the_chain_goes_away_every_marked_pair_is_restored() {
    let e = entities(6);
    let (b0, s0, b1, s1, b2, s2) = (e[0], e[1], e[2], e[3], e[4], e[5]);
    let ops = plan_pair_ops(
        None,
        &[pair(b2, s2, true), pair(b1, s1, false), pair(b0, s0, true)],
    );
    assert_eq!(
        ops,
        vec![
            PairOp::Restore {
                balloon: b0,
                peer: s0
            },
            PairOp::Restore {
                balloon: b2,
                peer: s2
            },
        ]
    );
}

/// 印は値を持たない（付いているかどうかだけが事実）。
#[test]
fn the_release_mark_carries_no_value() {
    assert_eq!(std::mem::size_of::<PairReleased>(), 0);
    assert_eq!(PairReleased, PairReleased::default());
}

// ===========================================================================
// 持ち主の輪の確かめ
// ===========================================================================

/// 持ち主を辿って持ち主の無い窓に着けば `Clear`。
#[test]
fn walking_to_an_unowned_window_is_clear() {
    // owner=0x20 → 0x30 → なし。owned=0x10 には出会わない
    let mut read = reader(vec![(0x20, 0x30)], vec![]);
    assert_eq!(
        check_owner_loop(fake_hwnd(0x10), fake_hwnd(0x20), &mut read),
        OwnerLoopCheck::Clear
    );
}

/// 持ち主を辿って所有側に出会えば `Loop`（例: `s0 ← b0` が残ったまま `b0 ← s0` を張る）。
#[test]
fn meeting_the_owned_window_on_the_walk_is_a_loop() {
    // s0=0x10 の持ち主が b0=0x20。付与 b0 ← s0 を確かめる
    let mut read = reader(vec![(0x10, 0x20)], vec![]);
    assert_eq!(
        check_owner_loop(fake_hwnd(0x20), fake_hwnd(0x10), &mut read),
        OwnerLoopCheck::Loop
    );
    // 自分自身を持ち主にする付与も輪
    let mut read = reader(vec![], vec![]);
    assert_eq!(
        check_owner_loop(fake_hwnd(0x10), fake_hwnd(0x10), &mut read),
        OwnerLoopCheck::Loop
    );
}

/// 辿る途中で読めなければ `Unreadable`。
#[test]
fn a_failed_read_on_the_walk_is_unreadable() {
    let mut read = reader(vec![(0x20, 0x30)], vec![0x30]);
    assert_eq!(
        check_owner_loop(fake_hwnd(0x10), fake_hwnd(0x20), &mut read),
        OwnerLoopCheck::Unreadable
    );
}

/// 辿りは上限 64 回。64 回目の読みで持ち主の無い窓に着けば `Clear`、65 回要れば `Loop`。
#[test]
fn the_walk_is_bounded_by_the_limit_and_beyond_it_counts_as_a_loop() {
    assert_eq!(OWNER_WALK_LIMIT, 64);
    // 0x1000 + i の持ち主が 0x1000 + i + 1 の一本道を n 段
    let line = |steps: usize| -> Vec<(usize, usize)> {
        (0..steps).map(|i| (0x1000 + i, 0x1000 + i + 1)).collect()
    };
    let owned = fake_hwnd(0x10);
    let owner = fake_hwnd(0x1000);

    // 63 段の後の窓は持ち主なし → 読みは 64 回で Clear
    let mut reads = 0usize;
    let mut base = reader(line(OWNER_WALK_LIMIT - 1), vec![]);
    let mut counted = |h: HWND| {
        reads += 1;
        base(h)
    };
    assert_eq!(
        check_owner_loop(owned, owner, &mut counted),
        OwnerLoopCheck::Clear
    );
    assert_eq!(reads, OWNER_WALK_LIMIT);

    // 64 段（読みが 65 回要る）→ 上限を超えるので Loop。読みは上限で止まる
    let mut reads = 0usize;
    let mut base = reader(line(OWNER_WALK_LIMIT), vec![]);
    let mut counted = |h: HWND| {
        reads += 1;
        base(h)
    };
    assert_eq!(
        check_owner_loop(owned, owner, &mut counted),
        OwnerLoopCheck::Loop
    );
    assert_eq!(reads, OWNER_WALK_LIMIT);
}

// ===========================================================================
// 組の受け渡しの記録——行の逐語・語の一覧・出口
// ===========================================================================

#[test]
fn the_pair_released_line_is_verbatim() {
    let e = entities(2);
    let line = pair_released_line(
        e[0],
        e[1],
        Some(fake_hwnd(0x10)),
        Some(fake_hwnd(0x20)),
        true,
    );
    assert_eq!(
        line,
        format!(
            "[zorder-chain] pair-released entity={:?} peer={:?} owned_hwnd=0x10 owner_hwnd=0x20 \
             unlinked=true",
            e[0], e[1]
        )
    );
    // 値が取れない欄は落とさず番兵
    let line = pair_released_line(e[0], e[1], None, None, false);
    assert!(
        line.ends_with("owned_hwnd=- owner_hwnd=- unlinked=false"),
        "{line}"
    );
}

#[test]
fn the_pair_restored_line_is_verbatim() {
    let e = entities(2);
    let line = pair_restored_line(e[0], e[1], Some(fake_hwnd(0x10)), Some(fake_hwnd(0x20)));
    assert_eq!(
        line,
        format!(
            "[zorder-chain] pair-restored entity={:?} peer={:?} owned_hwnd=0x10 owner_hwnd=0x20",
            e[0], e[1]
        )
    );
}

#[test]
fn the_link_refused_line_is_verbatim_for_both_reasons() {
    let e = entities(2);
    for (reason, word) in [
        (LinkRefusedReason::Loop, "Loop"),
        (LinkRefusedReason::Unreadable, "Unreadable"),
    ] {
        let line = link_refused_line(
            e[0],
            e[1],
            Some(fake_hwnd(0x10)),
            Some(fake_hwnd(0x20)),
            reason,
        );
        assert_eq!(
            line,
            format!(
                "[zorder-chain] link-refused owned={:?} owner={:?} owned_hwnd=0x10 \
                 owner_hwnd=0x20 reason={word}",
                e[0], e[1]
            )
        );
    }
}

/// 新しい 3 語は鎖の冠を持ち、互いにも既存 7 語とも重ならない（どれも他の語の部分にならない）。
#[test]
fn the_three_handover_tags_share_the_prefix_and_overlap_nothing() {
    let new = pair_handover_tags();
    assert_eq!(
        new,
        [
            "[zorder-chain] pair-released",
            "[zorder-chain] pair-restored",
            "[zorder-chain] link-refused",
        ]
    );
    // 既存の 7 語の一覧は変わらない
    assert_eq!(chain_record_tags().len(), 7);
    let all: Vec<&str> = chain_record_tags().into_iter().chain(new).collect();
    for tag in new {
        assert!(tag.starts_with("[zorder-chain] "), "冠が無い: {tag}");
    }
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            if i != j {
                assert!(!a.contains(b), "`{a}` が `{b}` を含む（grep が混ざる）");
            }
        }
    }
}

/// 3 本の出口は鎖のモジュールから、決めた水準で出る。
#[test]
fn the_handover_records_leave_from_the_chain_module_at_their_levels() {
    let e = entities(2);
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        log_chain_pair_released(e[0], e[1], Some(fake_hwnd(0x10)), None, false);
        log_chain_pair_restored(e[0], e[1], Some(fake_hwnd(0x10)), Some(fake_hwnd(0x20)));
        log_chain_link_refused(
            e[0],
            e[1],
            Some(fake_hwnd(0x10)),
            Some(fake_hwnd(0x20)),
            LinkRefusedReason::Loop,
        );
    });
    for (tag, level) in pair_handover_tags()
        .into_iter()
        .zip(["DEBUG", "DEBUG", "ERROR"])
    {
        let line = only_line_with(&out, tag);
        assert!(line.contains(CHAIN_TARGET), "出力先が違う: {line}");
        assert!(
            !line.contains(DIAG_TARGET),
            "行組立の層から出ている: {line}"
        );
        assert!(line.contains(level), "水準が {level} でない: {line}");
    }

    // 既定の水準（info）では debug の 2 語は出ず、error の 1 語だけが出る
    let quiet = capture_under_filter("info", || {
        log_chain_pair_released(e[0], e[1], None, None, true);
        log_chain_pair_restored(e[0], e[1], None, None);
        log_chain_link_refused(e[0], e[1], None, None, LinkRefusedReason::Unreadable);
    });
    assert!(!quiet.contains("[zorder-chain] pair-released"), "{quiet}");
    assert!(!quiet.contains("[zorder-chain] pair-restored"), "{quiet}");
    only_line_with(&quiet, "[zorder-chain] link-refused");
}
