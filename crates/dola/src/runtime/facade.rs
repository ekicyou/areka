//! DolaRuntime — 唯一の公開 API（Facade パターン）。

use std::collections::HashMap;

use crate::compile::compile_storyboard;
use crate::document::DolaDocument;

use super::conflict_resolver;
use super::document_store::DocumentStore;
use super::instance_manager::InstanceManager;
use super::instance_state::InstanceState;
use super::loop_controller::{self, LoopAction};
use super::subscription_manager::SubscriptionManager;
use super::timeline_manager::TimelineManager;
use super::types::{EvaluatedValue, MIN_LOOP_DURATION, RuntimeError, StartResult};

/// dola ランタイムエンジンの唯一の公開 API。
///
/// 全外部操作のエントリーポイント。内部コンポーネントへの委譲とフロー制御を行う。
///
/// # Usage
/// ```ignore
/// let mut rt = DolaRuntime::new();
/// rt.subscribe(1, "opacity");
/// rt.load_document(doc)?;
/// let result = rt.start("fade_in", 0.0)?;
/// let changes = rt.update(1, 0.5);
/// ```
pub struct DolaRuntime {
    document_store: DocumentStore,
    instance_manager: InstanceManager,
    timeline_manager: TimelineManager,
    subscription_manager: SubscriptionManager,
    next_group_id: u64,
}

impl DolaRuntime {
    /// 新しい DolaRuntime を生成する。
    pub fn new() -> Self {
        Self {
            document_store: DocumentStore::new(),
            instance_manager: InstanceManager::new(),
            timeline_manager: TimelineManager::new(),
            subscription_manager: SubscriptionManager::new(),
            next_group_id: 1,
        }
    }

    // =========================================================================
    // オーケストレーター向け API
    // =========================================================================

    /// 指示書読み込み。
    ///
    /// バリデーション成功時のみ内部に保持し、失敗時は既存定義を維持する。
    /// 新定義への差し替え時、同名変数は SubscriptionManager の `last_values` で
    /// 自動的に値が引き継がれる。
    pub fn load_document(&mut self, doc: DolaDocument) -> Result<(), RuntimeError> {
        self.document_store
            .store(doc)
            .map_err(RuntimeError::CompileError)
    }

    /// ストーリーボード開始。
    ///
    /// 指示書からストーリーボード定義を取得し、コンパイル → インスタンス作成 →
    /// タイムテーブル挿入 → Playing 遷移の一連のフローを実行する。
    pub fn start(&mut self, name: &str, start_time: f64) -> Result<StartResult, RuntimeError> {
        // 1. ドキュメント取得
        let doc = self
            .document_store
            .document()
            .ok_or_else(|| RuntimeError::StoryboardNotFound(name.to_string()))?;

        // ストーリーボード存在確認
        if !doc.storyboard.contains_key(name) {
            return Err(RuntimeError::StoryboardNotFound(name.to_string()));
        }

        // 2. コンパイル
        let compiled =
            compile_storyboard(doc, name, start_time).map_err(RuntimeError::CompileError)?;

        // 3. loop_count バリデーション
        if compiled.loop_count <= 0 && compiled.loop_count != -1 {
            return Err(RuntimeError::InvalidLoopCount(compiled.loop_count));
        }

        let loop_duration = compiled.total_base_duration / compiled.time_scale;

        if loop_duration == 0.0 && compiled.loop_count == -1 {
            return Err(RuntimeError::ZeroDurationWithLoop {
                storyboard: name.to_string(),
            });
        }
        if loop_duration < MIN_LOOP_DURATION && compiled.loop_count == -1 {
            return Err(RuntimeError::TooShortDurationWithInfiniteLoop {
                storyboard: name.to_string(),
                duration: loop_duration,
            });
        }

        // 4. end_time 算出（無限ループでも1周分の end_time を設定）
        let end_time = start_time + loop_duration;

        // 5. group_id 採番
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        // 6. インスタンス作成
        self.instance_manager.create_instance(
            group_id,
            name,
            compiled.interruption_policy,
            start_time,
            compiled.time_scale,
            compiled.total_base_duration,
            compiled.loop_count,
            end_time,
            start_time, // loop_start_time = start_time（初期値）
            loop_duration,
        );

        // 7. [Tier 3 Hook] 競合解決
        let affected = conflict_resolver::resolve_conflicts(
            group_id,
            &compiled,
            start_time,
            &mut self.timeline_manager,
            &mut self.instance_manager,
            &mut self.subscription_manager,
        )?; // Never 競合時はここで Err(RuntimeError::Conflict) を返す

        // 8. タイムテーブル挿入
        self.timeline_manager.insert_entries(group_id, &compiled);

        // 9. 状態遷移 Created → Playing
        self.instance_manager
            .transition(group_id, InstanceState::Playing)?;

        Ok(StartResult {
            group_id,
            end_time,
            affected_group_ids: affected,
        })
    }

    /// 終了予定時刻のみ計算（インスタンス非生成）。
    pub fn calculate_end_time(&self, name: &str, start_time: f64) -> Result<f64, RuntimeError> {
        let doc = self
            .document_store
            .document()
            .ok_or_else(|| RuntimeError::StoryboardNotFound(name.to_string()))?;

        if !doc.storyboard.contains_key(name) {
            return Err(RuntimeError::StoryboardNotFound(name.to_string()));
        }

        let compiled =
            compile_storyboard(doc, name, start_time).map_err(RuntimeError::CompileError)?;

        if compiled.loop_count <= 0 && compiled.loop_count != -1 {
            return Err(RuntimeError::InvalidLoopCount(compiled.loop_count));
        }

        let loop_duration = compiled.total_base_duration / compiled.time_scale;

        if loop_duration == 0.0 && compiled.loop_count == -1 {
            return Err(RuntimeError::ZeroDurationWithLoop {
                storyboard: name.to_string(),
            });
        }
        if loop_duration < MIN_LOOP_DURATION && compiled.loop_count == -1 {
            return Err(RuntimeError::TooShortDurationWithInfiniteLoop {
                storyboard: name.to_string(),
                duration: loop_duration,
            });
        }

        let end_time = start_time + loop_duration;

        Ok(end_time)
    }

    /// 一時停止。
    pub fn pause(&mut self, group_id: u64, current_time: f64) -> Result<(), RuntimeError> {
        self.instance_manager.pause(group_id)?;
        self.instance_manager
            .set_pause_start(group_id, current_time)?;
        Ok(())
    }

    /// 再開 — 再計算した end_time を返却。
    pub fn resume(&mut self, group_id: u64, current_time: f64) -> Result<f64, RuntimeError> {
        self.instance_manager.resume(group_id, current_time)
    }

    /// 最終値ジャンプ終了（Conclude）。
    ///
    /// 操作順序: 値取得 → last_values 更新 → 状態遷移 → エントリ削除
    pub fn conclude(&mut self, group_id: u64) -> Result<(), RuntimeError> {
        // group_id の存在確認
        self.instance_manager.get(group_id)?;

        self.conclude_internal(group_id);
        Ok(())
    }

    /// 現在値凍結破棄（Cancel）。
    ///
    /// エントリ削除のみ（last_values は前回 update の値が自然に残る）。
    pub fn cancel(&mut self, group_id: u64) -> Result<(), RuntimeError> {
        // group_id の存在確認 + terminal check
        let instance = self.instance_manager.get(group_id)?;
        if instance.state.is_terminal() {
            return Err(RuntimeError::InvalidGroupId(group_id));
        }

        // 状態遷移 → Cancelled（is_terminal → 自動削除）
        self.instance_manager
            .transition(group_id, InstanceState::Cancelled)?;
        // エントリ削除
        self.timeline_manager.remove_entries(group_id);
        Ok(())
    }

    /// 遅延 Conclude。
    ///
    /// `offset` は呼び出し側が `clock::now() + offset_secs` として計算した
    /// 絶対 deadline 時刻を想定。`update()` 内で `current_time >= deadline` の
    /// タイミングで Conclude 相当の処理を実行する。
    pub fn finish(&mut self, group_id: u64, offset: f64) -> Result<(), RuntimeError> {
        let instance = self.instance_manager.get(group_id)?;
        if instance.state.is_terminal() {
            return Err(RuntimeError::InvalidGroupId(group_id));
        }
        self.instance_manager
            .set_finish_deadline(group_id, offset)?;
        Ok(())
    }

    // =========================================================================
    // 購読者向け API
    // =========================================================================

    /// 購読登録（指示書受信前でも呼び出し可能）。
    pub fn subscribe(&mut self, subscriber_id: u64, variable_name: &str) {
        self.subscription_manager
            .subscribe(subscriber_id, variable_name);
    }

    /// 購読解除。
    pub fn unsubscribe(&mut self, subscriber_id: u64, variable_name: &str) {
        self.subscription_manager
            .unsubscribe(subscriber_id, variable_name);
    }

    /// 全購読解除。
    pub fn unsubscribe_all(&mut self, subscriber_id: u64) {
        self.subscription_manager.unsubscribe_all(subscriber_id);
    }

    /// 差分更新取得。
    ///
    /// 内部フロー:
    /// 1. finish deadline チェック → deadline 到達インスタンスは Conclude 相当
    /// 2. 自然終了検知（current_time >= end_time の Playing インスタンス）
    /// 3. 購読変数の evaluate ループ
    /// 4. diff_and_update
    pub fn update(
        &mut self,
        subscriber_id: u64,
        current_time: f64,
    ) -> Vec<(String, EvaluatedValue)> {
        // Step 1: Finish Deadline チェック
        let expired = self.instance_manager.check_finish_deadlines(current_time);
        for gid in expired {
            self.conclude_internal(gid);
        }

        // Step 2: ループ処理 + 自然終了検知
        let loop_results: Vec<(u64, LoopAction)> = self
            .instance_manager
            .instances_mut()
            .iter_mut()
            .filter(|(_, inst)| {
                inst.state == InstanceState::Playing && current_time >= inst.end_time
            })
            .map(|(gid, inst)| {
                let action = loop_controller::process_loops(inst, current_time);
                (*gid, action)
            })
            .collect();

        for (gid, action) in loop_results {
            if action == LoopAction::Conclude {
                self.conclude_internal(gid);
            }
        }

        // Step 3: 購読変数の評価（残存インスタンス対象）
        let var_names = self
            .subscription_manager
            .get_subscribed_variables(subscriber_id);
        let mut values = HashMap::new();
        for name in &var_names {
            if let Some(val) = self.timeline_manager.evaluate(
                name,
                current_time,
                self.instance_manager.instances(),
            ) {
                values.insert(name.clone(), val);
            }
        }

        // Step 4: 差分検出
        self.subscription_manager
            .diff_and_update(subscriber_id, values)
    }

    // =========================================================================
    // 内部ヘルパー
    // =========================================================================

    /// Conclude 相当の内部処理。
    ///
    /// 操作順序: 値取得 → last_values 更新 → 状態遷移(Concluded) → エントリ削除
    fn conclude_internal(&mut self, group_id: u64) {
        // 1. 最終値取得
        let final_values = self.timeline_manager.collect_final_values(group_id);

        // 2. last_values 強制更新
        if !final_values.is_empty() {
            self.subscription_manager
                .force_update_last_values(&final_values);
        }

        // 3. 状態遷移 → Concluded（自動削除）
        let _ = self
            .instance_manager
            .transition(group_id, InstanceState::Concluded);

        // 4. タイムテーブルエントリ削除
        self.timeline_manager.remove_entries(group_id);
    }
}

impl Default for DolaRuntime {
    fn default() -> Self {
        Self::new()
    }
}
