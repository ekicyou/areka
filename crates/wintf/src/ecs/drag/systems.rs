//! ドラッグシステム
//!
//! ドラッグ状態クリーンアップと、ドラッグ中の次画面更新の予約を提供する。

use super::DragEndEvent;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::*;

/// ドラッグ状態クリーンアップシステム
///
/// DragEndEventを監視し、DraggingStateを削除する。
pub fn cleanup_drag_state(
    mut commands: Commands,
    mut drag_end_events: MessageReader<DragEndEvent>,
) {
    for event in drag_end_events.read() {
        // DraggingStateを削除
        commands
            .entity(event.target)
            .remove::<crate::ecs::drag::DraggingState>();

        tracing::debug!(
            target = ?event.target,
            "[cleanup_drag_state] DraggingState removed"
        );
    }
}

/// ドラッグ中は次の画面更新も回す（自分で立て直す・設計 C16 の `DRAG`）。
///
/// ドラッグ中の窓とバルーンの追従は毎画面更新の反映を前提にしている（要件 4.3）。
/// ポインタの移動そのものは `POINTER` を立てるが、指を止めたままの間はメッセージが
/// 1 通も来ないので、それだけでは省略へ倒れてしまう。そこで tick の末尾
/// （`FrameFinalize`）で「まだドラッグ中である」ことを旗に写し、次の画面更新を予約する。
///
/// [`DraggingState`](crate::ecs::drag::DraggingState) が 1 つも無くなれば旗は立たなく
/// なるので、ドラッグの終わりで自然に止まる。
pub fn rearm_tick_while_dragging(dragging: Query<(), With<crate::ecs::drag::DraggingState>>) {
    if !dragging.is_empty() {
        crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::DRAG);
    }
}

#[cfg(test)]
mod tests {
    //! ドラッグ中の予約（[`rearm_tick_while_dragging`]）の判断だけを見る。
    //!
    //! 見るのは「[`DraggingState`](crate::ecs::drag::DraggingState) が在れば旗を立て、
    //! 無ければ立てない」の 1 点である。旗はプロセス共有なので、共有の錠
    //! （`TICK_WAKE_TEST_LOCK`）を取ってから触る（毒化に耐える取り方）。

    use super::*;
    use crate::ecs::drag::DraggingState;
    use crate::ecs::types::Point;
    use crate::ecs::world::tick_wake;
    use std::time::Instant;

    /// 1 度回して、その巡で立った旗を読んで返す。
    fn run_once(with_dragging: bool) -> u32 {
        let mut world = World::new();
        if with_dragging {
            world.spawn(DraggingState {
                drag_start_pos: Point::new(10, 20),
                initial_inset: (0.0, 0.0),
            });
        }
        let mut schedule = Schedule::default();
        schedule.add_systems(rearm_tick_while_dragging);

        // 直前に残っている旗を落としてから回す（共有の実体なので必ず掃く）。
        let _ = tick_wake::take(Instant::now());
        schedule.run(&mut world);
        tick_wake::take(Instant::now()).bits
    }

    #[test]
    fn dragging_entity_rearms_the_next_frame() {
        let _guard = crate::ecs::world::TICK_WAKE_TEST_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());

        assert_eq!(
            run_once(true) & tick_wake::DRAG.bits(),
            tick_wake::DRAG.bits(),
            "ドラッグ中は次の画面更新を予約する"
        );
    }

    #[test]
    fn no_dragging_entity_leaves_the_flag_down() {
        let _guard = crate::ecs::world::TICK_WAKE_TEST_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());

        assert_eq!(
            run_once(false) & tick_wake::DRAG.bits(),
            0,
            "ドラッグしていなければ旗は立たない"
        );
    }
}
