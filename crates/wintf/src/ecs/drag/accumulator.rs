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

    /// ドラッグ中のエンティティ（Started を積んでから Ended を積むまで Some・flush では消えない）
    current_dragging_entity: Option<Entity>,

    /// 現在のマウス位置（Dragging状態の時のみ有効）
    current_pos: PhysicalPoint,

    /// 前回 flush 以降に積まれた状態遷移イベント（積んだ順）
    pending_transitions: Vec<DragTransition>,
}

impl DragAccumulator {
    /// 新しいDragAccumulatorを作成
    pub fn new() -> Self {
        Self {
            accumulated_delta: PhysicalPoint::new(0, 0),
            current_dragging_entity: None,
            current_pos: PhysicalPoint::new(0, 0),
            pending_transitions: Vec::new(),
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

    /// 状態遷移を積む
    ///
    /// Started を積んでいないドラッグの Ended（閾値前の離し・取り消し）は置かずに捨てる。
    pub fn set_transition(&mut self, transition: DragTransition) {
        match &transition {
            DragTransition::Started { entity, .. } => {
                self.current_dragging_entity = Some(*entity);
            }
            DragTransition::Ended {
                entity, cancelled, ..
            } => {
                if self.current_dragging_entity.is_none() {
                    tracing::debug!(
                        entity = ?entity,
                        cancelled,
                        "[DragAccumulator] Ended without Started dropped"
                    );
                    return;
                }
                self.current_dragging_entity = None;
            }
        }
        self.pending_transitions.push(transition);
    }

    /// 累積量と遷移をflushして返す
    ///
    /// 返却後、accumulated_deltaはゼロにリセットされ、pending_transitionsは空になる
    pub fn flush(&mut self) -> FlushResult {
        let delta = self.accumulated_delta;
        let transitions = std::mem::take(&mut self.pending_transitions);
        let entity = self.current_dragging_entity;
        let position = self.current_pos;

        // デルタをリセット
        self.accumulated_delta = PhysicalPoint::new(0, 0);

        FlushResult {
            delta,
            transitions,
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

    /// 状態遷移（積んだ順・空なら知らせ無し）
    pub transitions: Vec<DragTransition>,

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
#[path = "accumulator_tests.rs"]
mod accumulator_tests;
