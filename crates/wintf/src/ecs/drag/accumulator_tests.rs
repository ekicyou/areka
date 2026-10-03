//! `DragAccumulator` の決定論テスト（積んだ順の待ち行列と、開始の無い終了を捨てる入口の判断）。

use super::*;

fn entity(idx: u32) -> Entity {
    Entity::from_raw_u32(idx).expect("valid entity index")
}

/// new()/Default は累積デルタ・位置ゼロ、ドラッグ対象 None、遷移は空。
#[test]
fn test_new_is_empty() {
    let acc = DragAccumulator::new();
    let result = {
        let mut acc = acc;
        acc.flush()
    };
    assert_eq!((result.delta.x, result.delta.y), (0, 0));
    assert!(result.transitions.is_empty());
    assert!(result.current_dragging_entity.is_none());
    assert_eq!(
        (result.current_position.x, result.current_position.y),
        (0, 0)
    );
}

/// accumulate_delta は複数回の呼び出しで加算される。
#[test]
fn test_accumulate_delta_sums() {
    let mut acc = DragAccumulator::new();
    acc.accumulate_delta(PhysicalPoint::new(3, 5));
    acc.accumulate_delta(PhysicalPoint::new(-1, 2));
    let result = acc.flush();
    assert_eq!((result.delta.x, result.delta.y), (2, 7));
}

/// flush 後は累積デルタがゼロにリセットされる（位置・対象は保持）。
#[test]
fn test_flush_resets_delta_but_keeps_position_and_entity() {
    let mut acc = DragAccumulator::new();
    let e = entity(1);
    acc.set_transition(DragTransition::Started {
        entity: e,
        start_pos: PhysicalPoint::new(0, 0),
        timestamp: Instant::now(),
    });
    acc.accumulate_delta(PhysicalPoint::new(10, 20));
    acc.update_position(PhysicalPoint::new(100, 200));

    let first = acc.flush();
    assert_eq!((first.delta.x, first.delta.y), (10, 20));
    assert_eq!(first.current_dragging_entity, Some(e));

    // 2 回目: delta は 0、position と entity は維持、transitions は消費済みで空
    let second = acc.flush();
    assert_eq!((second.delta.x, second.delta.y), (0, 0));
    assert!(second.transitions.is_empty());
    assert_eq!(second.current_dragging_entity, Some(e));
    assert_eq!(
        (second.current_position.x, second.current_position.y),
        (100, 200)
    );
}

/// set_transition(Started) は current_dragging_entity を Some にし、flush で take される。
#[test]
fn test_set_transition_started_sets_entity_and_is_taken_on_flush() {
    let mut acc = DragAccumulator::new();
    let e = entity(2);
    acc.set_transition(DragTransition::Started {
        entity: e,
        start_pos: PhysicalPoint::new(1, 1),
        timestamp: Instant::now(),
    });
    let result = acc.flush();
    assert!(matches!(
        result.transitions.as_slice(),
        [DragTransition::Started { .. }]
    ));
    assert_eq!(result.current_dragging_entity, Some(e));
}

/// set_transition(Ended) は current_dragging_entity を None に戻す。
#[test]
fn test_set_transition_ended_clears_entity() {
    let mut acc = DragAccumulator::new();
    let e = entity(3);
    acc.set_transition(DragTransition::Started {
        entity: e,
        start_pos: PhysicalPoint::new(0, 0),
        timestamp: Instant::now(),
    });
    acc.set_transition(DragTransition::Ended {
        entity: e,
        end_pos: PhysicalPoint::new(5, 5),
        cancelled: false,
    });
    let result = acc.flush();
    assert!(matches!(
        result.transitions.as_slice(),
        [.., DragTransition::Ended { .. }]
    ));
    assert!(
        result.current_dragging_entity.is_none(),
        "Ended 遷移で current_dragging_entity は None になるべき"
    );
}

/// update_position は最新位置で上書きされる（累積ではない）。
#[test]
fn test_update_position_overwrites() {
    let mut acc = DragAccumulator::new();
    acc.update_position(PhysicalPoint::new(10, 10));
    acc.update_position(PhysicalPoint::new(50, 60));
    let result = acc.flush();
    assert_eq!(
        (result.current_position.x, result.current_position.y),
        (50, 60)
    );
}

/// DragAccumulatorResource は Arc<Mutex> 越しに同じ内部状態を共有する（clone 後も同一）。
#[test]
fn test_resource_shares_inner_state_across_clone() {
    let res = DragAccumulatorResource::new();
    let clone = res.clone();
    // clone 経由で累積し、元の res からの flush で観測できること
    clone.accumulate_delta(PhysicalPoint::new(7, 8));
    clone.update_position(PhysicalPoint::new(70, 80));
    let result = res.flush().expect("lock 取得できるはず");
    assert_eq!((result.delta.x, result.delta.y), (7, 8));
    assert_eq!(
        (result.current_position.x, result.current_position.y),
        (70, 80)
    );
}

/// Resource 経由の set_transition も内部状態に反映される。
#[test]
fn test_resource_set_transition_reflected_in_flush() {
    let res = DragAccumulatorResource::new();
    let e = entity(4);
    res.set_transition(DragTransition::Started {
        entity: e,
        start_pos: PhysicalPoint::new(2, 2),
        timestamp: Instant::now(),
    });
    let result = res.flush().expect("lock 取得できるはず");
    assert!(matches!(
        result.transitions.as_slice(),
        [DragTransition::Started { .. }]
    ));
    assert_eq!(result.current_dragging_entity, Some(e));
}

fn started(e: Entity) -> DragTransition {
    DragTransition::Started {
        entity: e,
        start_pos: PhysicalPoint::new(0, 0),
        timestamp: Instant::now(),
    }
}

fn ended(e: Entity, cancelled: bool) -> DragTransition {
    DragTransition::Ended {
        entity: e,
        end_pos: PhysicalPoint::new(5, 5),
        cancelled,
    }
}

/// 開始を積んでいないドラッグの終了（閾値前の離し・取り消し）は置かれない。
#[test]
fn ended_without_started_is_dropped() {
    for cancelled in [false, true] {
        let mut acc = DragAccumulator::new();
        acc.set_transition(ended(entity(5), cancelled));
        let result = acc.flush();
        assert!(
            result.transitions.is_empty(),
            "cancelled={cancelled}: 開始の無い終了は捨てられるはず: {:?}",
            result.transitions
        );
        assert!(result.current_dragging_entity.is_none());
    }
}

/// 1 回の flush に開始と終了が重なっても、両方が積んだ順に返る（速いドラッグ）。
#[test]
fn started_then_ended_in_one_flush_keeps_both_in_order() {
    let mut acc = DragAccumulator::new();
    let e = entity(6);
    acc.set_transition(started(e));
    acc.set_transition(ended(e, false));
    let result = acc.flush();
    assert!(
        matches!(
            result.transitions.as_slice(),
            [
                DragTransition::Started { .. },
                DragTransition::Ended {
                    cancelled: false,
                    ..
                }
            ]
        ),
        "{:?}",
        result.transitions
    );
    assert!(result.current_dragging_entity.is_none());
    assert!(acc.flush().transitions.is_empty(), "取り出すと空になる");
}

/// 前の flush で開始した後、終了 → 次の開始が同じ flush に積まれても順序が保たれる。
#[test]
fn ended_then_started_in_one_flush_keeps_order() {
    let mut acc = DragAccumulator::new();
    let first = entity(7);
    let second = entity(8);
    acc.set_transition(started(first));
    acc.flush();
    acc.set_transition(ended(first, false));
    acc.set_transition(started(second));
    let result = acc.flush();
    assert!(
        matches!(
            result.transitions.as_slice(),
            [
                DragTransition::Ended { entity: a, .. },
                DragTransition::Started { entity: b, .. }
            ] if *a == first && *b == second
        ),
        "{:?}",
        result.transitions
    );
    assert_eq!(result.current_dragging_entity, Some(second));
}
