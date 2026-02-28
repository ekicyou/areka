//! CueSheetTracker — CueSheet の実行状態追跡コンポーネント。
//!
//! dispatch により spawn され、全配送先の CueQueue を監視する。
//! 上位層は `tracker.result()` を毎フレーム poll して完了を検知する。

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

use super::error::{CueSheetResult, CueSystemError};
use super::queue::{BarrierKind, BarrierResponse, CueQueueState};
use super::{ActorKey, CueTarget};

/// Tracker が systems に返すアクション指示
#[derive(Debug)]
pub enum TrackerAction {
    /// 何もしない
    None,
    /// 全スロットの WaitForClick を解除
    ResolveAllClicks,
    /// 全スロットのバリアをスキップ
    SkipAllBarriers,
    /// 全スロットをクリア（キャンセル時）
    ClearAll,
}

/// CueQueue の状態スナップショット（tracker.update に渡す）
#[derive(Debug)]
pub struct QueueSnapshot {
    pub entity: Entity,
    pub state: CueQueueState,
    pub timed_out: bool,
}

/// CueSheet の実行状態を追跡するコンポーネント。
/// dispatch により spawn され、全配送先の CueQueue を監視する。
#[derive(Component, Debug)]
#[component(storage = "SparseSet")]
pub struct CueSheetTracker {
    /// 配送先の (ActorKey, CueTarget, Entity) リスト
    targets: Vec<(ActorKey, CueTarget, Entity)>,
    /// 実行結果（Some になったら完了）
    result: Option<CueSheetResult>,
    /// キャンセル要求フラグ
    cancelled: bool,
    /// バリア状態: 最初に有効応答を得た BarrierResponse
    barrier_first_valid: Option<BarrierResponse>,
    /// バリア種別（バリア検知中のみ Some）
    barrier_kind: Option<BarrierKind>,
}

impl CueSheetTracker {
    /// 新しい CueSheetTracker を生成する。
    pub fn new(targets: Vec<(ActorKey, CueTarget, Entity)>) -> Self {
        Self {
            targets,
            result: None,
            cancelled: false,
            barrier_first_valid: None,
            barrier_kind: None,
        }
    }

    /// 実行結果を poll（None = 実行中）
    pub fn result(&self) -> Option<&CueSheetResult> {
        self.result.as_ref()
    }

    /// 外部からのキャンセル要求
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// 配送先リストの取得
    pub fn targets(&self) -> &[(ActorKey, CueTarget, Entity)] {
        &self.targets
    }

    /// 消費者からのバリア応答報告
    pub fn receive_barrier(&mut self, response: BarrierResponse) {
        // first valid wins: Skipped 以外が先に来たらそれを記録
        if self.barrier_first_valid.is_none() {
            match &response {
                BarrierResponse::Skipped => {} // Skipped は有効応答ではない
                _ => {
                    self.barrier_first_valid = Some(response);
                }
            }
        }
    }

    /// 毎フレーム呼び出し — バリアライフサイクル集中管理 + 完了判定
    ///
    /// CueQueue の状態スナップショットを受け取り、アクション指示を返す。
    /// 実際の CueQueue 操作はシステム側で行う。
    pub fn update(&mut self, snapshots: &[QueueSnapshot]) -> TrackerAction {
        // 結果確定済みなら何もしない
        if self.result.is_some() {
            return TrackerAction::None;
        }

        // Phase 0: キャンセル判定
        if self.cancelled {
            self.result = Some(CueSheetResult::Cancelled);
            return TrackerAction::ClearAll;
        }

        // エラー状態をチェック
        for snap in snapshots {
            if let CueQueueState::Error(err) = &snap.state {
                self.result = Some(CueSheetResult::Error(err.clone()));
                return TrackerAction::None;
            }
        }

        // Phase 1: バリア自動検知（barrier_kind が None のときのみ）
        if self.barrier_kind.is_none() {
            for snap in snapshots {
                match &snap.state {
                    CueQueueState::WaitingForClick => {
                        self.barrier_kind = Some(BarrierKind::Click);
                        break;
                    }
                    CueQueueState::WaitingForChoice => {
                        self.barrier_kind = Some(BarrierKind::Choice);
                        break;
                    }
                    _ => {}
                }
            }
        }

        // Phase 2: バリアタイムアウト検出
        if self.barrier_kind.is_some() && self.barrier_first_valid.is_none() {
            for snap in snapshots {
                if snap.timed_out {
                    self.barrier_first_valid = Some(BarrierResponse::Timeout);
                    break;
                }
            }
        }

        // Phase 3: バリア解決
        if self.barrier_first_valid.is_some() {
            return self.resolve_barrier();
        }

        // Phase 4: 完了判定
        let target_entities: Vec<Entity> = self.targets.iter().map(|(_, _, e)| *e).collect();
        let all_completed = target_entities.iter().all(|entity| {
            snapshots
                .iter()
                .find(|s| s.entity == *entity)
                .map(|s| s.state == CueQueueState::Completed)
                .unwrap_or(true) // despawn されたら完了扱い
        });

        if all_completed {
            self.result = Some(CueSheetResult::Completed);
        }

        TrackerAction::None
    }

    /// Phase 3: バリア解決 → アクション指示を返す
    fn resolve_barrier(&mut self) -> TrackerAction {
        let response = self.barrier_first_valid.take();
        let kind = self.barrier_kind.take();

        match (response, kind) {
            (Some(BarrierResponse::Click), Some(BarrierKind::Click)) => {
                TrackerAction::ResolveAllClicks
            }
            (Some(BarrierResponse::Choice { id }), Some(BarrierKind::Choice)) => {
                self.result = Some(CueSheetResult::Choice { id });
                TrackerAction::SkipAllBarriers
            }
            (Some(BarrierResponse::Timeout), _) => {
                self.result = Some(CueSheetResult::Timeout);
                TrackerAction::SkipAllBarriers
            }
            (Some(BarrierResponse::Skipped), Some(BarrierKind::Click)) => {
                TrackerAction::ResolveAllClicks
            }
            (Some(BarrierResponse::Skipped), Some(BarrierKind::Choice)) => {
                self.result = Some(CueSheetResult::Error(CueSystemError::EmptyChoiceBarrier {
                    actor: "all".to_string(),
                }));
                TrackerAction::SkipAllBarriers
            }
            _ => {
                self.barrier_first_valid = None;
                self.barrier_kind = None;
                TrackerAction::None
            }
        }
    }
}
