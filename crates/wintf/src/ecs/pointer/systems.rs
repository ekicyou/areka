//! ポインターシステム
//!
//! ECSスケジュールで実行されるポインター関連システム関数。
//! バッファ内容の反映、一時状態のクリア、デバッグ監視を提供する。

use bevy_ecs::prelude::*;

use super::types::{PointerLeave, PointerState, WheelDelta};

// ============================================================================
// システム
// ============================================================================

/// 一時的ポインター状態クリアシステム（FrameFinalize）
///
/// CommitComposition 後に実行され、1フレームのみ有効な状態をリセットする。
pub fn clear_transient_pointer_state(
    mut query: Query<&mut PointerState>,
    mut commands: Commands,
    leave_query: Query<Entity, With<PointerLeave>>,
) {
    for mut pointer in query.iter_mut() {
        pointer.wheel = WheelDelta::default();
    }

    for entity in leave_query.iter() {
        commands.entity(entity).remove::<PointerLeave>();
    }
}

// ============================================================================
// デバッグ用監視システム
// ============================================================================

/// PointerState の Added/Changed を監視するデバッグシステム
///
/// Inputスケジュールで実行し、PointerStateの変化をログ出力する。
/// デバッグ用途のため、リリースビルドでは使用しないこと。
pub fn debug_pointer_state_changes(
    added_query: Query<(Entity, &PointerState), Added<PointerState>>,
    changed_query: Query<(Entity, &PointerState), Changed<PointerState>>,
) {
    use tracing::debug;

    for (entity, pointer) in added_query.iter() {
        debug!(
            entity = ?entity,
            client_x = pointer.client_point.x,
            client_y = pointer.client_point.y,
            left = pointer.left_down,
            right = pointer.right_down,
            middle = pointer.middle_down,
            shift = pointer.shift_down,
            ctrl = pointer.ctrl_down,
            "[PointerState Added] Enter detected"
        );
    }

    for (entity, pointer) in changed_query.iter() {
        // Added の tick も Changed に含まれる（スキップはしない）。ここで出すのはホイールの回転だけ。
        if pointer.wheel.vertical != 0 || pointer.wheel.horizontal != 0 {
            debug!(
                entity = ?entity,
                vertical = pointer.wheel.vertical,
                horizontal = pointer.wheel.horizontal,
                "[PointerState Changed] Wheel detected"
            );
        }
    }
}

/// PointerLeave マーカーを監視するデバッグシステム
///
/// Inputスケジュールで実行し、PointerLeaveの付与をログ出力する。
pub fn debug_pointer_leave(leave_query: Query<Entity, Added<PointerLeave>>) {
    use tracing::debug;

    for entity in leave_query.iter() {
        debug!(
            entity = ?entity,
            "[PointerLeave Added] Leave detected"
        );
    }
}

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// ボタンの旗は配り（Input）の後に下り、wheel は clear_transient_pointer_state
    /// （FrameFinalize）で既定値に戻る。
    #[test]
    fn test_button_flag_falls_after_dispatch_and_wheel_resets() {
        let mut world = World::new();
        let e = world
            .spawn(PointerState {
                wheel: WheelDelta {
                    vertical: 120,
                    horizontal: -60,
                },
                left_down: true,
                ..Default::default()
            })
            .id();

        assert!(
            world.get::<PointerState>(e).unwrap().left_down,
            "押した tick は真"
        );
        crate::ecs::pointer::dispatch_pointer_events(&mut world);
        assert!(
            !world.get::<PointerState>(e).unwrap().left_down,
            "配りの後は偽"
        );

        let mut schedule = Schedule::default();
        schedule.add_systems(clear_transient_pointer_state);
        schedule.run(&mut world);

        let s = world.get::<PointerState>(e).unwrap();
        assert_eq!(s.wheel, WheelDelta::default(), "wheel はリセット");
        assert!(!s.left_down, "偽のまま");
    }

    /// clear_transient_pointer_state は PointerLeave マーカーを除去する。
    #[test]
    fn test_clear_transient_removes_pointer_leave_marker() {
        let mut world = World::new();
        // PointerLeave マーカーのみを持つエンティティ（PointerState は削除済みを想定）
        let leaving = world.spawn(PointerLeave).id();
        // PointerState を持ち PointerLeave のないエンティティ（除去対象外）
        let staying = world.spawn(PointerState::default()).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(clear_transient_pointer_state);
        schedule.run(&mut world);

        assert!(
            world.get::<PointerLeave>(leaving).is_none(),
            "PointerLeave マーカーは除去される"
        );
        assert!(
            world.get_entity(staying).is_ok(),
            "PointerState 側エンティティは存続"
        );
    }

    /// PointerState も PointerLeave もない状態でもパニックしない（空 World）。
    #[test]
    fn test_clear_transient_no_targets_is_noop() {
        let mut world = World::new();
        let e = world.spawn_empty().id();

        let mut schedule = Schedule::default();
        schedule.add_systems(clear_transient_pointer_state);
        schedule.run(&mut world);

        assert!(world.get_entity(e).is_ok());
    }
}
