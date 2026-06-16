//! CueSheetTracker — CueSheet の実行状態追跡コンポーネント。
//!
//! dispatch により spawn され、全配送先の CueQueue を監視する。
//! 上位層は `tracker.result()` を毎フレーム poll して完了を検知する。

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

use super::error::{CueSheetResult, CueSystemError};
use super::queue::{BarrierResponse, CueQueueState};
use super::{ActorKey, CueTarget};

/// Tracker が検知したバリアの種類（dola BarrierKind の簡約版）。
///
/// CueQueue の CueQueueState から検知するため、
/// タイムアウト値等の詳細は不要。
#[derive(Clone, Debug, PartialEq)]
enum TrackedBarrierKind {
    Click,
    Choice,
}

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
    barrier_kind: Option<TrackedBarrierKind>,
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
                        self.barrier_kind = Some(TrackedBarrierKind::Click);
                        break;
                    }
                    CueQueueState::WaitingForChoice => {
                        self.barrier_kind = Some(TrackedBarrierKind::Choice);
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
            (Some(BarrierResponse::Click), Some(TrackedBarrierKind::Click)) => {
                TrackerAction::ResolveAllClicks
            }
            (Some(BarrierResponse::Choice { id }), Some(TrackedBarrierKind::Choice)) => {
                self.result = Some(CueSheetResult::Choice { id });
                TrackerAction::SkipAllBarriers
            }
            (Some(BarrierResponse::Timeout), _) => {
                self.result = Some(CueSheetResult::Timeout);
                TrackerAction::SkipAllBarriers
            }
            (Some(BarrierResponse::Skipped), Some(TrackedBarrierKind::Click)) => {
                TrackerAction::ResolveAllClicks
            }
            (Some(BarrierResponse::Skipped), Some(TrackedBarrierKind::Choice)) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    /// テスト用に entity を 1 体生成するヘルパー。
    fn make_entity() -> Entity {
        let mut world = World::new();
        world.spawn_empty().id()
    }

    fn targets_one(e: Entity) -> Vec<(ActorKey, CueTarget, Entity)> {
        vec![(ActorKey::from("sakura"), CueTarget::Balloon, e)]
    }

    // ── receive_barrier: first valid wins ──

    /// `receive_barrier(Skipped)` は有効応答ではなく記録されない。
    /// 後続の非 Skipped 応答が最初の有効応答として採用される。
    #[test]
    fn receive_barrier_skipped_is_not_recorded_then_first_valid_wins() {
        let e = make_entity();
        let mut tracker = CueSheetTracker::new(targets_one(e));

        // Skipped を複数受けても barrier_kind 検知後の解決には影響しない
        tracker.receive_barrier(BarrierResponse::Skipped);
        tracker.receive_barrier(BarrierResponse::Skipped);

        // バリア検知（WaitingForClick）→ Skipped のみなら Phase 4(完了判定)へ落ちず
        // バリア未解決のまま（result は None、まだ Playing 扱いの待機）
        let barrier_snap = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::WaitingForClick,
            timed_out: false,
        }];
        let _ = tracker.update(&barrier_snap);
        assert!(
            tracker.result().is_none(),
            "Skipped-only should not resolve the barrier"
        );

        // 有効応答 Click が来たら採用され ResolveAllClicks
        tracker.receive_barrier(BarrierResponse::Click);
        let action = tracker.update(&barrier_snap);
        assert!(matches!(action, TrackerAction::ResolveAllClicks));
    }

    /// 最初に記録された有効応答が後続の有効応答で上書きされない（first valid wins）。
    #[test]
    fn receive_barrier_keeps_first_valid_over_later_valid() {
        let e = make_entity();
        let mut tracker = CueSheetTracker::new(targets_one(e));

        // Choice バリアを検知させる
        let barrier_snap = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::WaitingForChoice,
            timed_out: false,
        }];
        let _ = tracker.update(&barrier_snap);

        // 最初の有効応答 = Choice "first"、後続 Choice "second" は無視される
        tracker.receive_barrier(BarrierResponse::Choice { id: "first".into() });
        tracker.receive_barrier(BarrierResponse::Choice { id: "second".into() });

        let _ = tracker.update(&barrier_snap);
        match tracker.result() {
            Some(CueSheetResult::Choice { id }) => assert_eq!(id, "first"),
            other => panic!("Expected Choice {{ first }}, got {other:?}"),
        }
    }

    // ── resolve_barrier: (response, kind) アクション写像 ──

    /// Click 応答 × Click バリア → ResolveAllClicks（result は確定しない＝続行）。
    #[test]
    fn resolve_click_barrier_returns_resolve_all_clicks_without_finishing() {
        let e = make_entity();
        let mut tracker = CueSheetTracker::new(targets_one(e));

        let barrier_snap = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::WaitingForClick,
            timed_out: false,
        }];
        let _ = tracker.update(&barrier_snap);
        tracker.receive_barrier(BarrierResponse::Click);

        let action = tracker.update(&barrier_snap);
        assert!(matches!(action, TrackerAction::ResolveAllClicks));
        // Click 解決は CueSheet を完了させない（演技継続）
        assert!(tracker.result().is_none());
    }

    /// タイムアウト検知 → Timeout 応答 × Click バリア → SkipAllBarriers ＋ result=Timeout。
    /// （Click バリアでもタイムアウトは種別不問で Timeout 結果になる経路を固定）
    #[test]
    fn timeout_on_click_barrier_finishes_with_timeout_result() {
        let e = make_entity();
        let mut tracker = CueSheetTracker::new(targets_one(e));

        let barrier_snap = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::WaitingForClick,
            timed_out: false,
        }];
        let _ = tracker.update(&barrier_snap);

        // Click バリア確定後にタイムアウト → barrier_first_valid=Timeout（種別不問）
        let timeout_snap = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::WaitingForClick,
            timed_out: true,
        }];
        let action = tracker.update(&timeout_snap);
        assert!(matches!(action, TrackerAction::SkipAllBarriers));
        assert!(matches!(tracker.result(), Some(CueSheetResult::Timeout)));
    }

    /// Choice 応答 × Choice バリア → SkipAllBarriers ＋ result=Choice。
    #[test]
    fn resolve_choice_barrier_finishes_with_choice_result() {
        let e = make_entity();
        let mut tracker = CueSheetTracker::new(targets_one(e));

        let barrier_snap = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::WaitingForChoice,
            timed_out: false,
        }];
        let _ = tracker.update(&barrier_snap);
        tracker.receive_barrier(BarrierResponse::Choice { id: "yes".into() });

        let action = tracker.update(&barrier_snap);
        assert!(matches!(action, TrackerAction::SkipAllBarriers));
        match tracker.result() {
            Some(CueSheetResult::Choice { id }) => assert_eq!(id, "yes"),
            other => panic!("Expected Choice, got {other:?}"),
        }
    }

    /// タイムアウト検知 → barrier_first_valid=Timeout → SkipAllBarriers ＋ result=Timeout。
    /// Choice バリアでも Timeout 応答は種別不問で Timeout 結果になる。
    #[test]
    fn timeout_on_choice_barrier_finishes_with_timeout_result() {
        let e = make_entity();
        let mut tracker = CueSheetTracker::new(targets_one(e));

        // Choice バリア検知
        let barrier_snap = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::WaitingForChoice,
            timed_out: false,
        }];
        let _ = tracker.update(&barrier_snap);

        // タイムアウト
        let timeout_snap = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::WaitingForChoice,
            timed_out: true,
        }];
        let action = tracker.update(&timeout_snap);
        assert!(matches!(action, TrackerAction::SkipAllBarriers));
        assert!(matches!(tracker.result(), Some(CueSheetResult::Timeout)));
    }

    // ── update の優先順位・防御 ──

    /// result 確定後は update しても None アクションで結果不変（安定）。
    #[test]
    fn update_after_result_is_stable_none() {
        let e = make_entity();
        let mut tracker = CueSheetTracker::new(targets_one(e));

        // 完了させる
        let done = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::Completed,
            timed_out: false,
        }];
        let _ = tracker.update(&done);
        assert!(matches!(tracker.result(), Some(CueSheetResult::Completed)));

        // 以降の update は None で結果不変
        let playing = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::Playing,
            timed_out: false,
        }];
        let action = tracker.update(&playing);
        assert!(matches!(action, TrackerAction::None));
        assert!(matches!(tracker.result(), Some(CueSheetResult::Completed)));
    }

    /// cancel はエラー状態や完了判定より優先され ClearAll + Cancelled。
    #[test]
    fn cancel_takes_priority_and_clears_all() {
        let e = make_entity();
        let mut tracker = CueSheetTracker::new(targets_one(e));
        tracker.cancel();

        // たとえ Completed スナップショットでも cancel が優先
        let snap = vec![QueueSnapshot {
            entity: e,
            state: CueQueueState::Completed,
            timed_out: false,
        }];
        let action = tracker.update(&snap);
        assert!(matches!(action, TrackerAction::ClearAll));
        assert!(matches!(tracker.result(), Some(CueSheetResult::Cancelled)));
    }

    /// エラー状態はバリア検知・完了判定より優先され result=Error（アクション None）。
    #[test]
    fn error_state_takes_priority_over_completion() {
        // 同一 World から 2 体生成して e1 != e2 を保証する
        // （別 World の「最初の entity」は同一ビットになり取り違えるため）。
        let mut world = World::new();
        let e1 = world.spawn_empty().id();
        let e2 = world.spawn_empty().id();
        assert!(e1 != e2);
        let mut tracker = CueSheetTracker::new(vec![
            (ActorKey::from("sakura"), CueTarget::Shell, e1),
            (ActorKey::from("sakura"), CueTarget::Balloon, e2),
        ]);

        let err = CueSystemError::EmptyChoiceBarrier {
            actor: "sakura".into(),
        };
        // 一方 Completed・他方 Error → Error が優先
        let snap = vec![
            QueueSnapshot {
                entity: e1,
                state: CueQueueState::Completed,
                timed_out: false,
            },
            QueueSnapshot {
                entity: e2,
                state: CueQueueState::Error(err.clone()),
                timed_out: false,
            },
        ];
        let action = tracker.update(&snap);
        assert!(matches!(action, TrackerAction::None));
        match tracker.result() {
            Some(CueSheetResult::Error(e)) => assert_eq!(e, &err),
            other => panic!("Expected Error, got {other:?}"),
        }
    }

    /// despawn された（スナップショット欠落）ターゲットは完了扱い（unwrap_or(true)）。
    #[test]
    fn missing_target_snapshot_counts_as_completed() {
        let mut world = World::new();
        let e1 = world.spawn_empty().id();
        let e2 = world.spawn_empty().id();
        let mut tracker = CueSheetTracker::new(vec![
            (ActorKey::from("sakura"), CueTarget::Shell, e1),
            (ActorKey::from("sakura"), CueTarget::Balloon, e2),
        ]);

        // e2 のスナップショットを欠落させる（despawn 相当）。e1 のみ Completed。
        let snap = vec![QueueSnapshot {
            entity: e1,
            state: CueQueueState::Completed,
            timed_out: false,
        }];
        let _ = tracker.update(&snap);
        // 欠落ターゲットは完了扱い → 全完了で Completed
        assert!(matches!(tracker.result(), Some(CueSheetResult::Completed)));
    }

    /// `targets()` はコンストラクタで渡した配送先を保持する。
    #[test]
    fn targets_accessor_returns_constructor_targets() {
        let mut world = World::new();
        let e1 = world.spawn_empty().id();
        let e2 = world.spawn_empty().id();
        let given = vec![
            (ActorKey::from("sakura"), CueTarget::Shell, e1),
            (ActorKey::from("kero"), CueTarget::Balloon, e2),
        ];
        let tracker = CueSheetTracker::new(given.clone());
        let got = tracker.targets();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], given[0]);
        assert_eq!(got[1], given[1]);
    }
}
