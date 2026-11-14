//! 維持系と沈降の観測が、望む鎖が外させる組に触れないことのテスト
//! （要件 4.4・5.1・5.2・5.4・5.5・6.3）。
//!
//! - 外す組に付いた再断行の要求（未適用＝バルーンの再表示の形・検証待ち）は、指令を積まず
//!   `ReleasedByChain` を記録して落とす。実窓の重なりは 1 つも動かない。
//! - 外す組に付いた活性化の目印は、測らずに外して見送りを記録し、指令も要求も生まない。
//! - 連続する組は、同じ鎖の下でも今と同じに是正・観測される。
//!
//! 「指令が出ない」は `guarded_set_window_pos` の実施記録を数えて測り、捕捉が生きている
//! ことを対照の指令 1 本で示す（兄弟の `zorder_pair_maintain_tests.rs` と同じ作法）。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;
use windows::core::w;

use super::IssuedPairFix;
use crate::api::get_window_below;
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::zorder_pair::{ExpectedOrder, InsertSpec};
use crate::ecs::window::zorder_pair_sink::{SinkObservationPending, mark_pair_sink_observation};
use crate::ecs::window::zorder_pair_test_support::{
    CAPTURE_DIRECTIVES as DIRECTIVES, FIX_TAG, SET_WINDOW_POS_TAG, SINK_OBSERVED_TAG, Trio,
    VERIFY_FAILED_TAG, issue_control_command, lines_with, maintain_schedule, released_skips,
    spawn_window,
};
use crate::ecs::window::{
    ChainPlan, KeepDirectlyAbove, ReassertZOrder, ZOrderChainPlan, ZOrderPairStrategy,
    flush_window_pos_commands,
};

/// 組を宣言し、(バルーン, キャラ) を返す。
fn pair(world: &mut World, balloon: HWND, character: HWND) -> (Entity, Entity) {
    let character = spawn_window(world, character);
    let balloon = spawn_window(world, balloon);
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });
    (balloon, character)
}

fn publish(world: &mut World, members: Vec<Entity>) {
    world.insert_resource(ZOrderChainPlan {
        chain: Some(ChainPlan {
            members,
            cross_edges: Vec::new(),
            absent: Vec::new(),
        }),
        dirty: true,
    });
}

fn new_world() -> World {
    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    world
}

/// 逆転の計画（キャラがバルーンより手前）の下で、バルーンの再表示が挿す未適用の要求は
/// 指令を積まずに見送りで落ち、実窓の重なりは動かない（要件 5.2・5.5・6.3）。
#[test]
fn a_reshow_request_on_a_released_pair_moves_nothing_and_is_dropped() {
    let trio = Trio::create(w!("wintf-zorder-maintain-release"));
    let mut world = new_world();
    let (balloon, character) = pair(&mut world, trio.balloon, trio.character);
    publish(&mut world, vec![character, balloon]);
    // 再表示の挿入元と同じ形（未適用の既定値）。
    world.entity_mut(balloon).insert(ReassertZOrder::default());

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
        flush_window_pos_commands();
        issue_control_command(trio.balloon);
    });

    assert_eq!(released_skips(&out, balloon), 1, "見送りは 1 本\n{out}");
    assert_eq!(lines_with(&out, FIX_TAG).len(), 0, "是正しない\n{out}");
    assert_eq!(
        lines_with(&out, SET_WINDOW_POS_TAG).len(),
        1,
        "対照の 1 本だけ\n{out}"
    );
    assert!(world.get::<ReassertZOrder>(balloon).is_none());
    assert_eq!(
        get_window_below(trio.balloon).unwrap(),
        Some(trio.intruder),
        "重なりは 1 つも動かない"
    );
}

/// 外す組に付いた検証待ちの要求は、照合せずに見送りで落とし、持ち越しも外す（要件 5.5）。
#[test]
fn a_pending_verification_on_a_released_pair_is_dropped_without_checking() {
    let trio = Trio::create(w!("wintf-zorder-maintain-release"));
    let mut world = new_world();
    let (balloon, character) = pair(&mut world, trio.balloon, trio.character);
    // 非隣接の鎖（間に別の窓が挟まる）。
    let gap = world.spawn_empty().id();
    publish(&mut world, vec![balloon, gap, character]);
    world.entity_mut(balloon).insert((
        ReassertZOrder {
            pending_verify: Some(ExpectedOrder {
                above: trio.balloon,
                below: trio.character,
            }),
        },
        IssuedPairFix {
            insert_after: InsertSpec::After(trio.intruder),
        },
    ));

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(DIRECTIVES, || {
        schedule.run(&mut world);
        flush_window_pos_commands();
        issue_control_command(trio.balloon);
    });

    assert_eq!(released_skips(&out, balloon), 1, "見送りは 1 本\n{out}");
    assert_eq!(
        lines_with(&out, VERIFY_FAILED_TAG).len(),
        0,
        "照合しない\n{out}"
    );
    assert_eq!(
        lines_with(&out, FIX_TAG).len(),
        0,
        "照合の記録も出さない\n{out}"
    );
    assert_eq!(
        lines_with(&out, SET_WINDOW_POS_TAG).len(),
        1,
        "対照の 1 本だけ\n{out}"
    );
    assert!(world.get::<ReassertZOrder>(balloon).is_none());
    assert!(world.get::<IssuedPairFix>(balloon).is_none());
}

/// 同じ鎖の下でも、連続する組は今と同じに是正される（要件 4.4）。
#[test]
fn a_pair_the_chain_keeps_contiguous_is_still_fixed() {
    let trio = Trio::create(w!("wintf-zorder-maintain-release"));
    let mut world = new_world();
    let (balloon, character) = pair(&mut world, trio.balloon, trio.character);
    publish(&mut world, vec![balloon, character]);
    world.entity_mut(balloon).insert(ReassertZOrder::default());

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(DIRECTIVES, || {
        schedule.run(&mut world);
        flush_window_pos_commands();
        schedule.run(&mut world);
    });

    assert_eq!(released_skips(&out, balloon), 0, "{out}");
    assert_eq!(lines_with(&out, VERIFY_FAILED_TAG).len(), 0, "{out}");
    assert_eq!(
        lines_with(&out, SET_WINDOW_POS_TAG).len(),
        1,
        "是正の 1 本\n{out}"
    );
    assert_eq!(
        get_window_below(trio.balloon).unwrap(),
        Some(trio.character)
    );
    assert!(world.get::<ReassertZOrder>(balloon).is_none());
}

/// 活性化の目印（押す・他のアプリを選ぶ）が外す組に付いても、沈降の観測は測らず、
/// 指令も要求も生まない。連続する組の目印は今と同じに測られる（要件 5.1・5.4・4.4）。
#[test]
fn a_sink_mark_on_a_released_pair_is_dropped_unmeasured() {
    let released_trio = Trio::create(w!("wintf-zorder-maintain-release"));
    let kept_trio = Trio::create(w!("wintf-zorder-maintain-release"));
    let mut world = new_world();
    let (b0, s0) = pair(&mut world, released_trio.balloon, released_trio.character);
    let (b1, s1) = pair(&mut world, kept_trio.balloon, kept_trio.character);
    publish(&mut world, vec![s0, b0, b1, s1]);
    // 非活性化の枝と同じ入口で目印を付ける（キャラ窓が非活性化された形）。
    assert_eq!(mark_pair_sink_observation(&mut world, s0), Some(b0));
    assert_eq!(mark_pair_sink_observation(&mut world, s1), Some(b1));

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(DIRECTIVES, || {
        schedule.run(&mut world);
        flush_window_pos_commands();
        issue_control_command(released_trio.balloon);
    });

    assert_eq!(released_skips(&out, b0), 1, "外す組は見送り 1 本\n{out}");
    assert_eq!(released_skips(&out, b1), 0, "{out}");
    assert_eq!(
        lines_with(&out, SINK_OBSERVED_TAG).len(),
        1,
        "測るのは連続する組だけ\n{out}"
    );
    assert_eq!(
        lines_with(&out, SET_WINDOW_POS_TAG).len(),
        1,
        "対照の 1 本だけ\n{out}"
    );
    for balloon in [b0, b1] {
        assert!(world.get::<SinkObservationPending>(balloon).is_none());
        assert!(world.get::<ReassertZOrder>(balloon).is_none());
    }
}
