//! 起動の並べ直しが、記憶から戻した組を動かさない場合の檻。印 `none`・覚えたときと同じ幅・
//! 印の無い古い記憶では誰も動かさない。動かす場合と作り直しは本体の同名のファイルにある。

use super::persist_test_support::persist_world;
use super::shared_test_support::{
    GAP_WITHOUT_REALIGN, HIGH_SIZE_1, LOW_SIZE_1, PAIR_LINE, high_sizes, low_sizes, pair_gap,
    remembered_placements, seat_remembered_pair,
};
use super::test_support::PerTargetSizes;
use super::*;
use areka_placement::chain_finalize::{AdjacencyMark, ChainFinalized};
use log_capture_kit::{LineFormat, capture_lines};
use wintf::ecs::window::drain_window_pos_commands;

/// 記憶から戻した組を立て、並べ直しを 1 回回す。返すのは並べ直しで出た窓への指令の数と、
/// 並べ直しの間のログ。
fn boot(
    world: &mut World,
    mark: Option<AdjacencyMark>,
    shown: &PerTargetSizes,
) -> (usize, Vec<String>) {
    seat_remembered_pair(world, &remembered_placements(), mark, shown);
    let logs = capture_lines(LineFormat::LevelTargetFields, || {
        finalize_chain_once_with(shown, world)
    })
    .1;
    assert!(
        world.contains_resource::<ChainFinalized>(),
        "前提が崩れている: 並べ直しが駆動していない"
    );
    (drain_window_pos_commands().len(), logs)
}

/// ⑶ 印 `none`（覚えたときに離れていた・重なっていた）の組は詰めない。
#[test]
fn remembered_separate_pair_is_not_tightened() {
    let mut world = persist_world();
    let (moved, logs) = boot(&mut world, Some(AdjacencyMark::Separate), &low_sizes());
    assert_eq!(
        (moved, pair_gap(&world, LOW_SIZE_1.0)),
        (0, GAP_WITHOUT_REALIGN),
        "(並べ直しの指令の数, 隙間)"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains(PAIR_LINE) && l.contains("decision=\"remembered-separate\"")),
        "動かさない判断も組ごとに残る: {logs:?}"
    );
}

/// ⑷ 覚えたときと今の幅が同じなら、印 `left` でも誰も動かない。
#[test]
fn same_widths_as_remembered_move_nobody() {
    let mut world = persist_world();
    let (moved, _) = boot(&mut world, Some(AdjacencyMark::Left), &high_sizes());
    assert_eq!(
        (moved, pair_gap(&world, HIGH_SIZE_1.0)),
        (0, 0),
        "(並べ直しの指令の数, 隙間)"
    );
}

/// ⑸ 印の無い古い記憶は位置のまま立つ。
#[test]
fn old_memory_without_mark_stands_at_its_position() {
    let mut world = persist_world();
    let (moved, logs) = boot(&mut world, None, &low_sizes());
    assert_eq!(
        (moved, pair_gap(&world, LOW_SIZE_1.0)),
        (0, GAP_WITHOUT_REALIGN),
        "(並べ直しの指令の数, 隙間)"
    );
    assert!(
        logs.iter()
            .any(|l| l.contains(PAIR_LINE) && l.contains("decision=\"no-remembered-mark\"")),
        "印が無いことも組ごとに残る: {logs:?}"
    );
}
