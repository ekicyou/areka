//! ドラッグ累積器（スレッド間データ転送用）

use crate::ecs::pointer::PhysicalPoint;
use bevy_ecs::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// ドラッグ状態遷移イベント
#[derive(Debug, Clone)]
pub enum DragTransition {
    /// ドラッグ開始
    Started {
        entity: Entity,
        start_pos: PhysicalPoint,
        timestamp: Instant,
    },

    /// ドラッグ終了
    Ended {
        entity: Entity,
        end_pos: PhysicalPoint,
        cancelled: bool,
    },
}

/// ドラッグ累積器（wndproc→ECS転送用）
#[derive(Debug)]
pub struct DragAccumulator {
    /// 前回flush以降の累積デルタ
    accumulated_delta: PhysicalPoint,

    /// ドラッグ中のエンティティ（Dragging状態の時のみSome）
    current_dragging_entity: Option<Entity>,

    /// 現在のマウス位置（Dragging状態の時のみ有効）
    current_pos: PhysicalPoint,

    /// 保留中の状態遷移イベント
    pending_transition: Option<DragTransition>,
}

impl DragAccumulator {
    /// 新しいDragAccumulatorを作成
    pub fn new() -> Self {
        Self {
            accumulated_delta: PhysicalPoint::new(0, 0),
            current_dragging_entity: None,
            current_pos: PhysicalPoint::new(0, 0),
            pending_transition: None,
        }
    }

    /// デルタを累積
    pub fn accumulate_delta(&mut self, delta: PhysicalPoint) {
        self.accumulated_delta.x += delta.x;
        self.accumulated_delta.y += delta.y;
    }

    /// 現在のマウス位置を更新
    pub fn update_position(&mut self, pos: PhysicalPoint) {
        self.current_pos = pos;
    }

    /// 状態遷移を設定
    pub fn set_transition(&mut self, transition: DragTransition) {
        match &transition {
            DragTransition::Started { entity, .. } => {
                self.current_dragging_entity = Some(*entity);
            }
            DragTransition::Ended { .. } => {
                self.current_dragging_entity = None;
            }
        }
        self.pending_transition = Some(transition);
    }

    /// 累積量と遷移をflushして返す
    ///
    /// 返却後、accumulated_deltaはゼロにリセットされ、pending_transitionはクリアされる
    pub fn flush(&mut self) -> FlushResult {
        let delta = self.accumulated_delta;
        let transition = self.pending_transition.take();
        let entity = self.current_dragging_entity;
        let position = self.current_pos;

        // デルタをリセット
        self.accumulated_delta = PhysicalPoint::new(0, 0);

        FlushResult {
            delta,
            transition,
            current_dragging_entity: entity,
            current_position: position,
        }
    }
}

impl Default for DragAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

/// flush()の返却値
#[derive(Debug)]
pub struct FlushResult {
    /// 累積デルタ
    pub delta: PhysicalPoint,

    /// 状態遷移（あれば）
    pub transition: Option<DragTransition>,

    /// 現在ドラッグ中のエンティティ（Dragging状態の時のみSome）
    pub current_dragging_entity: Option<Entity>,

    /// 現在のマウス位置
    pub current_position: PhysicalPoint,
}

/// DragAccumulatorのECSリソースラッパー
#[derive(Resource, Clone)]
pub struct DragAccumulatorResource {
    inner: Arc<Mutex<DragAccumulator>>,
}

impl DragAccumulatorResource {
    /// 新しいDragAccumulatorResourceを作成
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DragAccumulator::new())),
        }
    }

    /// デルタを累積（wndprocから呼ばれる）
    pub fn accumulate_delta(&self, delta: PhysicalPoint) {
        if let Ok(mut acc) = self.inner.lock() {
            acc.accumulate_delta(delta);
        }
    }

    /// 現在のマウス位置を更新（wndprocから呼ばれる）
    pub fn update_position(&self, pos: PhysicalPoint) {
        if let Ok(mut acc) = self.inner.lock() {
            acc.update_position(pos);
        }
    }

    /// 状態遷移を設定（wndprocから呼ばれる）
    pub fn set_transition(&self, transition: DragTransition) {
        if let Ok(mut acc) = self.inner.lock() {
            acc.set_transition(transition);
        }
    }

    /// 累積量と遷移をflush（dispatch_drag_eventsから呼ばれる）
    pub fn flush(&self) -> Option<FlushResult> {
        self.inner.lock().ok().map(|mut acc| acc.flush())
    }
}

impl Default for DragAccumulatorResource {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(idx: u32) -> Entity {
        Entity::from_raw_u32(idx).expect("valid entity index")
    }

    /// new()/Default は累積デルタ・位置ゼロ、ドラッグ対象 None、遷移 None。
    #[test]
    fn test_new_is_empty() {
        let acc = DragAccumulator::new();
        let result = {
            let mut acc = acc;
            acc.flush()
        };
        assert_eq!((result.delta.x, result.delta.y), (0, 0));
        assert!(result.transition.is_none());
        assert!(result.current_dragging_entity.is_none());
        assert_eq!((result.current_position.x, result.current_position.y), (0, 0));
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

        // 2 回目: delta は 0、position と entity は維持、transition は消費済みで None
        let second = acc.flush();
        assert_eq!((second.delta.x, second.delta.y), (0, 0));
        assert!(second.transition.is_none());
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
            result.transition,
            Some(DragTransition::Started { .. })
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
            result.transition,
            Some(DragTransition::Ended { .. })
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
            result.transition,
            Some(DragTransition::Started { .. })
        ));
        assert_eq!(result.current_dragging_entity, Some(e));
    }
}
