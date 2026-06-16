//! DolaRuntime — 唯一の公開 API（Facade パターン）。

use std::collections::{HashMap, HashSet};

use crate::compile::{CompiledStoryboard, CompiledTrigger, compile_storyboard};
use crate::document::DolaDocument;
use crate::easing::{EasingFunction, EasingName};
use crate::storyboard::LoopOffset;

use super::conflict_resolver;
use super::document_store::DocumentStore;
use super::instance_manager::InstanceManager;
use super::instance_state::InstanceState;
use super::loop_controller::{self, LoopAction};
use super::subscription_manager::SubscriptionManager;
use super::timeline_manager::TimelineManager;
use super::types::{MIN_LOOP_DURATION, RuntimeError, StartResult, TriggerResult, UpdateResult};

/// dola ランタイムエンジンの唯一の公開 API。
///
/// 全外部操作のエントリーポイント。内部コンポーネントへの委譲とフロー制御を行う。
///
/// # Usage
/// ```ignore
/// let mut rt = DolaRuntime::new();
/// let opacity_id = rt.subscribe("opacity");
/// rt.load_document(doc)?;
/// let result = rt.start("fade_in", 0.0)?;
/// let changes = rt.update(0.5);
/// ```
pub struct DolaRuntime {
    document_store: DocumentStore,
    instance_manager: InstanceManager,
    timeline_manager: TimelineManager,
    subscription_manager: SubscriptionManager,
    next_group_id: u64,
    /// group_id → CompiledTrigger リスト（start() 時に格納、conclude/cancel で削除）
    trigger_store: HashMap<u64, Vec<CompiledTrigger>>,
    /// 直前の tick() 結果を保持するフィールド
    last_update_result: UpdateResult,
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
            trigger_store: HashMap::new(),
            last_update_result: UpdateResult {
                changes: Vec::new(),
                triggered: Vec::new(),
            },
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
        self.start_internal(name, start_time, &HashSet::new())
    }

    /// ストーリーボード開始（内部）。
    ///
    /// `skip_conflict_gids` に含まれる group_id は競合検出から除外される。
    /// トリガー起動時、親インスタンスを除外するために使用。
    fn start_internal(
        &mut self,
        name: &str,
        start_time: f64,
        skip_conflict_gids: &HashSet<u64>,
    ) -> Result<StartResult, RuntimeError> {
        // 1. ドキュメント取得 → コンパイル → バリデーション
        //    （セグメントは常に相対時間 0.0 起点で構築。
        //    壁時計時刻への変換は loop_start_time で行う）
        let (compiled, loop_duration) = self.compile_and_validate(name, 0.0)?;

        // 2. end_time 算出（無限ループでも1周分の end_time を設定）
        let end_time = start_time + loop_duration;

        // 3. group_id 採番
        let group_id = self.next_group_id;
        self.next_group_id += 1;

        // 3.5 loop_offset 展開
        let (lo_min, lo_max, lo_easing) = match &compiled.loop_offset {
            Some(LoopOffset::Scalar(v)) => {
                (Some(0.0), *v, EasingFunction::Named(EasingName::Linear))
            }
            Some(LoopOffset::Range(r)) => (Some(r.min), r.max, r.easing.clone()),
            None => (None, 0.0, EasingFunction::Named(EasingName::Linear)),
        };

        // 4. インスタンス作成
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
            lo_min,
            lo_max,
            lo_easing,
            compiled.triggers.len(),
        );

        // 5. [Tier 3 Hook] 競合解決（skip_conflict_gids 内のインスタンスは除外）
        let affected = conflict_resolver::resolve_conflicts_excluding(
            group_id,
            &compiled,
            start_time,
            &mut self.timeline_manager,
            &mut self.instance_manager,
            &mut self.subscription_manager,
            skip_conflict_gids,
        )?; // Never 競合時はここで Err(RuntimeError::Conflict) を返す

        // 6. タイムテーブル挿入
        self.timeline_manager.insert_entries(group_id, &compiled);

        // 6.5 トリガー格納
        if !compiled.triggers.is_empty() {
            self.trigger_store
                .insert(group_id, compiled.triggers.clone());
        }

        // 7. 状態遷移 Created → Playing
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
        let (_, loop_duration) = self.compile_and_validate(name, start_time)?;
        Ok(start_time + loop_duration)
    }

    /// ドキュメント取得 → ストーリーボード存在確認 → コンパイル → ループ系バリデーション。
    ///
    /// `start_internal` と `calculate_end_time` で共通の検証フロー。
    /// 成功時は（コンパイル結果, 1周分の実時間 loop_duration）を返す。
    fn compile_and_validate(
        &self,
        name: &str,
        base_time: f64,
    ) -> Result<(CompiledStoryboard, f64), RuntimeError> {
        let doc = self
            .document_store
            .document()
            .ok_or_else(|| RuntimeError::StoryboardNotFound(name.to_string()))?;

        // ストーリーボード存在確認
        if !doc.storyboard.contains_key(name) {
            return Err(RuntimeError::StoryboardNotFound(name.to_string()));
        }

        let compiled =
            compile_storyboard(doc, name, base_time).map_err(RuntimeError::CompileError)?;

        // loop_count バリデーション
        if compiled.loop_count <= 0 && compiled.loop_count != -1 {
            return Err(RuntimeError::InvalidLoopCount(compiled.loop_count));
        }

        // NOTE(D1a-V 脆弱性所見): time_scale はスキーマ/コンパイル時に正値検証されて
        // いないため、time_scale == 0.0 のとき loop_duration は +inf
        // （total_base_duration > 0）または NaN（total_base_duration == 0）になる。
        // inf/NaN は下の == / < 比較を素通りし end_time = start_time + loop_duration も
        // inf/NaN となるため、該当インスタンスは自然終了・ループ・トリガー発火が
        // 一切起きないまま生存し続ける（負値なら end_time が過去になり即 Conclude）。
        // 現行挙動は tests/runtime/facade_test.rs::time_scale_boundary で固定済み。
        // 入力検証の追加は外部観測可能な挙動変更となるため report/proposals.md P8 に記録。
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

        Ok((compiled, loop_duration))
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
        // トリガーストア削除
        self.trigger_store.remove(&group_id);
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
    /// 同一変数名の再呼び出しは同一IDを返却する（冪等）。
    pub fn subscribe(&mut self, variable_name: &str) -> i64 {
        self.subscription_manager.subscribe(variable_name)
    }

    /// 購読解除（variable_id 指定）。
    pub fn unsubscribe(&mut self, variable_id: i64) -> Result<(), RuntimeError> {
        self.subscription_manager.unsubscribe(variable_id)
    }

    /// 全購読解除。
    pub fn unsubscribe_all(&mut self) {
        self.subscription_manager.unsubscribe_all();
    }

    // =========================================================================
    // 2 フェーズ API（新規）
    // =========================================================================

    /// Phase 1: 内部状態を current_time まで進行し、結果を内部フィールドに格納。
    ///
    /// `TimedSchedule::tick()` と対称な設計。
    pub fn tick(&mut self, current_time: f64) {
        self.last_update_result = self.update_internal(current_time);
    }

    /// Phase 2: 直前の tick() 結果を読み取り専用で返す。
    ///
    /// tick() 未呼び出し時は空の `UpdateResult` を返す。
    /// `TimedSchedule::ready()` と対称な設計。
    pub fn last_result(&self) -> &UpdateResult {
        &self.last_update_result
    }

    /// 差分更新取得（後方互換）。
    ///
    /// 内部フロー:
    /// 1. finish deadline チェック → deadline 到達インスタンスは Conclude 相当
    /// 1.5 トリガー収集・実行（Playing インスタンスのトリガーを先に処理）
    /// 2. 自然終了検知（current_time >= end_time の Playing インスタンス）
    /// 3. 購読変数の evaluate ループ
    /// 4. diff_and_update
    #[deprecated(note = "use tick() + last_result() instead")]
    pub fn update(&mut self, current_time: f64) -> UpdateResult {
        self.tick(current_time);
        self.last_update_result.clone()
    }

    /// 差分更新の内部実装。
    fn update_internal(&mut self, current_time: f64) -> UpdateResult {
        // Step 1: Finish Deadline チェック
        let expired = self.instance_manager.check_finish_deadlines(current_time);
        for gid in expired {
            self.conclude_internal(gid);
        }

        // Step 1.5: トリガー収集・実行（Conclude 前に発火させる）
        let trigger_results = self.process_triggers(current_time);

        // Step 2: ループ処理 + 自然終了検知
        let mut rng = rand::rng();
        let loop_results: Vec<(u64, LoopAction)> = self
            .instance_manager
            .instances_mut()
            .iter_mut()
            .filter(|(_, inst)| {
                inst.state == InstanceState::Playing && current_time >= inst.end_time
            })
            .map(|(gid, inst)| {
                let action = loop_controller::process_loops(inst, current_time, &mut rng);
                (*gid, action)
            })
            .collect();

        for (gid, action) in loop_results {
            if action == LoopAction::Conclude {
                self.conclude_internal(gid);
            }
        }

        // Step 3: 購読変数の評価（残存インスタンス対象）
        let var_names = self.subscription_manager.get_subscribed_variable_names();
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
        let changes = self.subscription_manager.diff_and_update(values);

        UpdateResult {
            changes,
            triggered: trigger_results,
        }
    }

    // =========================================================================
    // 内部ヘルパー
    // =========================================================================

    /// トリガー収集・実行。
    ///
    /// Playing 状態のインスタンスに紐づくトリガーを走査し、
    /// 発火条件（current_time >= 発火時刻 かつ 未発火）を満たすものを
    /// fire-and-forget で子ストーリーボードとして開始する。
    fn process_triggers(&mut self, current_time: f64) -> Vec<TriggerResult> {
        // Step 1: 発火対象を収集（借用を分離するため先に Vec へ集約）
        struct PendingTrigger {
            parent_group_id: u64,
            trigger_index: usize,
            target_storyboard: String,
            child_start_time: f64,
            source_storyboard: String,
        }

        let mut pending: Vec<PendingTrigger> = Vec::new();

        for (gid, inst) in self.instance_manager.instances().iter() {
            if inst.state != InstanceState::Playing {
                continue;
            }
            if let Some(triggers) = self.trigger_store.get(gid) {
                for (ti, ts) in inst.trigger_states.iter().enumerate() {
                    if ts.fired {
                        continue;
                    }
                    // SAFETY(panic 経路): trigger_states は create_instance で
                    // 0..compiled.triggers.len() の添字から構築され、trigger_store には
                    // 同一の compiled.triggers が同じ group_id で格納される
                    // （start_internal ステップ 4 / 6.5）。両者は conclude/cancel で
                    // 同時に削除され group_id は再利用されないため、
                    // ts.trigger_index < triggers.len() が常に成立し添字 panic は発火しない。
                    debug_assert!(
                        ts.trigger_index < triggers.len(),
                        "trigger_states/trigger_store invariant violated: index {} >= len {}",
                        ts.trigger_index,
                        triggers.len()
                    );
                    let trigger = &triggers[ts.trigger_index];
                    // fire_time は compile 時の絶対時刻（start_time 基準）。
                    // ループ内での壁時計発火時刻:
                    //   loop_start_time + (fire_time - original_start_time) / time_scale
                    // NOTE(D1a-V): time_scale == 0.0 では wall_fire_time が inf/NaN となり
                    // トリガーは発火しない（compile_and_validate の NOTE / P8 参照）。
                    let relative_base_time = trigger.fire_time - inst.start_time;
                    let wall_fire_time =
                        inst.loop_start_time + relative_base_time / inst.time_scale;

                    if current_time >= wall_fire_time {
                        let child_start = wall_fire_time + trigger.start_offset.unwrap_or(0.0);
                        pending.push(PendingTrigger {
                            parent_group_id: *gid,
                            trigger_index: ti,
                            target_storyboard: trigger.target_storyboard.clone(),
                            child_start_time: child_start,
                            source_storyboard: inst.storyboard_name.clone(),
                        });
                    }
                }
            }
        }

        // Step 2: fired フラグを立てる
        // （p.trigger_index は Step 1 で同一インスタンスの trigger_states を
        //   enumerate した添字。Step 1〜2 間に trigger_states への変更はないため
        //   範囲外 panic は発火しない）
        for p in &pending {
            if let Ok(inst) = self.instance_manager.get_mut(p.parent_group_id) {
                inst.trigger_states[p.trigger_index].fired = true;
            }
        }

        // Step 3: 子ストーリーボードを start（fire-and-forget）
        //         親インスタンスを競合検出から除外する
        let mut results: Vec<TriggerResult> = Vec::new();
        for p in pending {
            let skip = HashSet::from([p.parent_group_id]);
            match self.start_internal(&p.target_storyboard, p.child_start_time, &skip) {
                Ok(start_result) => {
                    results.push(TriggerResult::Started {
                        source_storyboard: p.source_storyboard,
                        target_storyboard: p.target_storyboard,
                        start_result,
                    });
                }
                Err(e) => {
                    results.push(TriggerResult::Error {
                        source_storyboard: p.source_storyboard,
                        target_storyboard: p.target_storyboard,
                        error: e,
                    });
                }
            }
        }

        results
    }

    /// Conclude 相当の内部処理。
    ///
    /// 操作順序: 値取得 → name→id変換 → last_values 更新 → 状態遷移(Concluded) → エントリ削除
    fn conclude_internal(&mut self, group_id: u64) {
        // 1. 最終値取得（変数名ベース）
        let final_values = self.timeline_manager.collect_final_values(group_id);

        // 2. name→id 変換 + last_values 強制更新
        if !final_values.is_empty() {
            let id_values = self
                .subscription_manager
                .convert_names_to_ids(&final_values);
            self.subscription_manager
                .force_update_last_values(&id_values);
        }

        // 3. 状態遷移 → Concluded（自動削除）
        let _ = self
            .instance_manager
            .transition(group_id, InstanceState::Concluded);

        // 4. タイムテーブルエントリ削除
        self.timeline_manager.remove_entries(group_id);

        // 5. トリガーストア削除
        self.trigger_store.remove(&group_id);
    }
}

impl Default for DolaRuntime {
    fn default() -> Self {
        Self::new()
    }
}
