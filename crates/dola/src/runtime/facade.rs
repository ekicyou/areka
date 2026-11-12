//! DolaRuntime — 唯一の公開 API（Facade パターン）。

use std::collections::{BTreeSet, HashMap};

use crate::compile::{CompiledStoryboard, CompiledTrigger, compile_all, compile_validated};
use crate::document::DolaDocument;
use crate::easing::{EasingFunction, EasingName};
use crate::storyboard::LoopOffset;

use super::conflict_resolver;
use super::instance_manager::{InstanceManager, StoryboardInstance, TriggerState};
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
/// rt.tick(0.5);
/// let changes = rt.last_result();
/// ```
pub struct DolaRuntime {
    /// 検証とコンパイルを全部通った文書だけを持つ（`load_document`）
    document: Option<DolaDocument>,
    instance_manager: InstanceManager,
    timeline_manager: TimelineManager,
    subscription_manager: SubscriptionManager,
    next_group_id: u64,
    /// group_id → CompiledTrigger リスト（start() 時に格納、conclude/cancel/競合の終わりで削除）
    trigger_store: HashMap<u64, Vec<CompiledTrigger>>,
    /// 直前の tick() 結果を保持するフィールド
    last_update_result: UpdateResult,
}

impl DolaRuntime {
    /// 新しい DolaRuntime を生成する。
    pub fn new() -> Self {
        Self {
            document: None,
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
    /// 検証の後に全ストーリーボードをコンパイルにかけ、全部通ったときだけ差し替える。
    /// 失敗時は既存定義を維持する（読み込みで通った文書は start で落ちない）。
    /// 新定義への差し替え時、同名変数は SubscriptionManager の `last_values` で
    /// 自動的に値が引き継がれる。
    pub fn load_document(&mut self, doc: DolaDocument) -> Result<(), RuntimeError> {
        compile_all(&doc).map_err(RuntimeError::CompileError)?;
        // 周回の長さの規則も読み込みで確かめ、読み込めた文書が start で落ちないようにする
        for name in doc.storyboard.keys() {
            let compiled =
                compile_validated(&doc, name, 0.0).map_err(RuntimeError::CompileError)?;
            loop_duration(name, &compiled)?;
        }
        self.document = Some(doc);
        self.timeline_manager.release_unused_intern_values();
        Ok(())
    }

    /// ストーリーボード開始。
    ///
    /// 同じ変数に生きた項目を持つ古いインスタンスは、`start_time` が未来でも呼んだ時点で
    /// 競合として扱う（「生きた項目があれば競合」・2026-11-13 開発者の裁定）。
    ///
    /// 指示書からストーリーボード定義を取得し、コンパイル → インスタンス作成 →
    /// タイムテーブル挿入 → Playing 遷移の一連のフローを実行する。
    pub fn start(&mut self, name: &str, start_time: f64) -> Result<StartResult, RuntimeError> {
        self.start_internal(name, start_time, &BTreeSet::new())
    }

    /// ストーリーボード開始（内部）。
    ///
    /// `skip_conflict_gids` に含まれる group_id は競合検出から除外される。
    /// トリガー起動時、親インスタンスを除外するために使用。
    fn start_internal(
        &mut self,
        name: &str,
        start_time: f64,
        skip_conflict_gids: &BTreeSet<u64>,
    ) -> Result<StartResult, RuntimeError> {
        // 1. ドキュメント取得 → コンパイル → バリデーション
        //    （セグメントは常に相対時間 0.0 起点で構築。
        //    壁時計時刻への変換は loop_start_time で行う）
        let (mut compiled, loop_duration) = self.compile_and_validate(name, 0.0)?;

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

        // 4. インスタンス作成（Created 状態）。group_id は単調増加の採番で既存の鍵と衝突しない
        let trigger_states = (0..compiled.triggers.len())
            .map(|trigger_index| TriggerState {
                trigger_index,
                fired: false,
            })
            .collect();
        self.instance_manager.instances_mut().insert(
            group_id,
            StoryboardInstance {
                storyboard_name: name.to_string(),
                state: InstanceState::Created,
                interruption_policy: compiled.interruption_policy,
                time_scale: compiled.time_scale,
                pause_accumulated: 0.0,
                pause_start: None,
                loop_count: compiled.loop_count,
                loops_completed: 0,
                finish_deadline: None,
                end_time,
                loop_start_time: start_time,
                loop_duration,
                loop_offset_min: lo_min,
                loop_offset_max: lo_max,
                loop_offset_easing: lo_easing,
                trigger_states,
            },
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
        for gid in &affected {
            self.trigger_store.remove(gid);
        }

        // 6. タイムテーブル挿入
        self.timeline_manager.insert_entries(group_id, &compiled);

        // 6.5 トリガー格納
        if !compiled.triggers.is_empty() {
            self.trigger_store
                .insert(group_id, std::mem::take(&mut compiled.triggers));
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

    /// ドキュメント取得 → ストーリーボード存在確認 → コンパイル → ループ系バリデーション。
    ///
    /// 成功時は（コンパイル結果, 1周分の実時間 loop_duration）を返す。
    fn compile_and_validate(
        &self,
        name: &str,
        base_time: f64,
    ) -> Result<(CompiledStoryboard, f64), RuntimeError> {
        let doc = self
            .document
            .as_ref()
            .ok_or_else(|| RuntimeError::StoryboardNotFound(name.to_string()))?;

        // ストーリーボード存在確認
        if !doc.storyboard.contains_key(name) {
            return Err(RuntimeError::StoryboardNotFound(name.to_string()));
        }

        // 文書は load_document で検証済み
        let compiled =
            compile_validated(doc, name, base_time).map_err(RuntimeError::CompileError)?;

        // load_document が同じ規則を通しているので、ここでは落ちない
        let loop_duration = loop_duration(name, &compiled)?;
        Ok((compiled, loop_duration))
    }

    /// 一時停止。
    pub fn pause(&mut self, group_id: u64, current_time: f64) -> Result<(), RuntimeError> {
        self.instance_manager.pause(group_id, current_time)
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
    /// エントリ削除のみ（last_values は前回 tick の値が自然に残る）。
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
    /// `deadline` は呼び出し側が `clock::now() + offset_secs` として計算した
    /// 絶対時刻。`tick()` 内で `current_time >= deadline` の
    /// タイミングで Conclude 相当の処理を実行する。
    pub fn finish(&mut self, group_id: u64, deadline: f64) -> Result<(), RuntimeError> {
        let instance = self.instance_manager.get(group_id)?;
        if instance.state.is_terminal() {
            return Err(RuntimeError::InvalidGroupId(group_id));
        }
        self.instance_manager
            .set_finish_deadline(group_id, deadline)?;
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

    /// 終わっていないインスタンス（一時停止中を含む）が 1 つでもあれば true。
    ///
    /// 終わったインスタンスは表から消える（`InstanceManager::transition`）ので、表が空でないかを見る。
    pub fn has_instances(&self) -> bool {
        !self.instance_manager.instances().is_empty()
    }

    /// 差分更新の内部実装。
    ///
    /// 内部フロー:
    /// 0〜2 を、current_time までの出来事の時刻（`next_event_time`）の順に 1 つずつ回す。
    /// 同じ時刻では 0 → 1 → 1.5 → 2 の順（トリガーどうしは親の group_id の順）
    /// 0. トリガーなしの無限のループを、その時刻より前に終わる周回まで進める
    /// 1. finish deadline チェック → deadline 到達インスタンスは Conclude 相当
    ///    1.5 トリガー収集・実行（Playing インスタンスのトリガーを先に処理）
    /// 2. 自然終了検知（その時刻 >= end_time の Playing インスタンス）
    ///
    /// 最後に、出来事に入れない無限のループ（トリガーなし）を current_time まで進める
    /// 3. 購読変数の evaluate ループ
    /// 4. diff_and_update
    fn update_internal(&mut self, current_time: f64) -> UpdateResult {
        // 時刻の順に回すのは、子を起こす順がぶつかる子の勝ち負けを決めるから。
        // 1 コマずつ回したときと同じ順になる
        let mut trigger_results = Vec::new();
        let mut rng = rand::rng();
        while let Some(t) = self.next_event_time().filter(|&t| t <= current_time) {
            // Step 0: トリガーなしの無限のループを t より前に終わる周回まで進める（60fps の前のコマと同じ周回にする）。
            // ほかの Playing の end_time は出来事なので t 以上で、ここでは動かない。t ちょうどの周回は Step 2 で
            self.process_loop_ends(t.next_down(), &mut rng);
            // Step 1: Finish Deadline チェック
            for gid in self.instance_manager.check_finish_deadlines(t) {
                self.conclude_internal(gid);
            }
            // Step 1.5: トリガー収集・実行（Conclude 前に発火させる）
            trigger_results.extend(self.process_triggers(t));
            // Step 2: ループ処理 + 自然終了検知
            self.process_loop_ends(t, &mut rng);
        }
        self.process_loop_ends(current_time, &mut rng);

        // Step 3: 購読変数の評価（残存インスタンス対象）
        // Step 4: 差分検出（評価と同じ周回で行う）
        let timeline_manager = &mut self.timeline_manager;
        let instances = self.instance_manager.instances();
        let changes = self
            .subscription_manager
            .diff_and_update(|name| timeline_manager.evaluate(name, current_time, instances));

        UpdateResult {
            changes,
            triggered: trigger_results,
        }
    }

    // =========================================================================
    // 内部ヘルパー
    // =========================================================================

    /// 次の出来事の時刻: finish の期限・Playing のまだ起きていないトリガー（周回の長さの内）・
    /// 周回の終わりの最小。トリガーなしの無限のループの周回の終わりは入れない: 自然には終わらず、
    /// 出来事の時刻ごとに `process_loop_ends` がその時刻まで進めるので、1 周ずつ回さなくてよい。
    /// NaN の時刻は `f64::min` が捨てる（NaN の期限は起きない）
    fn next_event_time(&self) -> Option<f64> {
        let mut next: Option<f64> = None;
        let mut take = |t: f64| next = Some(next.map_or(t, |n| n.min(t)));
        for (gid, inst) in self.instance_manager.instances() {
            if let Some(deadline) = inst.finish_deadline {
                take(deadline);
            }
            if inst.state != InstanceState::Playing {
                continue;
            }
            if inst.loop_count != -1 || !inst.trigger_states.is_empty() {
                take(inst.end_time);
            }
            if let Some(triggers) = self.trigger_store.get(gid) {
                for ts in inst.trigger_states.iter().filter(|ts| !ts.fired) {
                    if let Some(wall) =
                        trigger_wall_time(inst, triggers[ts.trigger_index].fire_time)
                    {
                        take(wall);
                    }
                }
            }
        }
        next.filter(|t| !t.is_nan())
    }

    /// Step 2: `time` までに周回の終わった Playing のインスタンスを進め、終わったものを Conclude する
    fn process_loop_ends(&mut self, time: f64, rng: &mut impl rand::Rng) {
        let loop_results: Vec<(u64, LoopAction)> = self
            .instance_manager
            .instances_mut()
            .iter_mut()
            .filter(|(_, inst)| inst.state == InstanceState::Playing && time >= inst.end_time)
            .map(|(gid, inst)| (*gid, loop_controller::process_loops(inst, time, rng)))
            .collect();
        for (gid, action) in loop_results {
            if action == LoopAction::Conclude {
                self.conclude_internal(gid);
            }
        }
    }

    /// トリガー収集・実行。発火するものが無くなるまで繰り返す。
    ///
    /// 起こした子のトリガーも、子が自然の終わり（Step 2）で消える前に同じ tick で発火させる。
    /// 1 回の発火で印が立ち、印を戻すのは Step 2 の周回だけなので、2 回目からは新しく起きた子の
    /// トリガーしか集まらない。トリガーの循環は検証が拒む（V15t）ので、繰り返しは
    /// ストーリーボードの数 + 1 回で止まる。
    fn process_triggers(&mut self, current_time: f64) -> Vec<TriggerResult> {
        let mut results = Vec::new();
        loop {
            let fired = self.fire_due_triggers(current_time);
            if fired.is_empty() {
                return results;
            }
            results.extend(fired);
        }
    }

    /// Playing 状態のインスタンスに紐づくトリガーを走査し、
    /// 発火条件（current_time >= 発火時刻 かつ 未発火）を満たすものを
    /// fire-and-forget で子ストーリーボードとして開始する（1 回分）。
    fn fire_due_triggers(&mut self, current_time: f64) -> Vec<TriggerResult> {
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
                    // SAFETY(panic 経路): trigger_states は start_internal のステップ 4 で
                    // 0..compiled.triggers.len() の添字から構築され、trigger_store には
                    // 同一の compiled.triggers が同じ group_id で格納される
                    // （start_internal ステップ 4 / 6.5）。trigger_store は自然終了・cancel・
                    // 競合の終わり（ステップ 5 の直後）で消え、group_id は再利用されないため、
                    // ts.trigger_index < triggers.len() が常に成立し添字 panic は発火しない。
                    debug_assert!(
                        ts.trigger_index < triggers.len(),
                        "trigger_states/trigger_store invariant violated: index {} >= len {}",
                        ts.trigger_index,
                        triggers.len()
                    );
                    let trigger = &triggers[ts.trigger_index];
                    if let Some(wall_fire_time) = trigger_wall_time(inst, trigger.fire_time)
                        && current_time >= wall_fire_time
                    {
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
            let skip = BTreeSet::from([p.parent_group_id]);
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

/// トリガーの壁時計の発火時刻。周回の長さより後ろのトリガーは None（親の周回が先に終わる・要件 11.3）。
///
/// fire_time は 0.0 起点で compile した周回内の相対時刻で、壁時計の時刻は評価の式
/// calculate_effective_time の逆: loop_start_time + pause_accumulated + fire_time / time_scale。
/// 周回を飛ばした tick でも 1 コマずつ回したときと同じにするため、内か外かは時刻でなく長さで比べる。
/// end_time は別の式（一時停止・待ちを足し込む）なので、丸めで 1 ulp 後ろへずれた周回の終わりちょうどの
/// トリガーが Step 2 で消えないよう end_time で抑える
fn trigger_wall_time(inst: &StoryboardInstance, fire_time: f64) -> Option<f64> {
    let offset = fire_time / inst.time_scale;
    (offset <= inst.loop_duration)
        .then(|| (inst.loop_start_time + inst.pause_accumulated + offset).min(inst.end_time))
}

impl Default for DolaRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// 1 周分の実時間。無限のループの周回が 0 か `MIN_LOOP_DURATION` 未満なら拒む。
fn loop_duration(name: &str, compiled: &CompiledStoryboard) -> Result<f64, RuntimeError> {
    // time_scale は validate() の数値の規則で有限・正に限られる（0 なら +inf／NaN になる）。
    // 割った結果が有限であることはコンパイルが確かめる
    let duration = compiled.total_base_duration / compiled.time_scale;
    if compiled.loop_count == -1 {
        if duration == 0.0 {
            return Err(RuntimeError::ZeroDurationWithLoop {
                storyboard: name.to_string(),
            });
        }
        if duration < MIN_LOOP_DURATION {
            return Err(RuntimeError::TooShortDurationWithInfiniteLoop {
                storyboard: name.to_string(),
                duration,
            });
        }
    }
    Ok(duration)
}

#[cfg(test)]
#[path = "facade_tests.rs"]
mod tests;
