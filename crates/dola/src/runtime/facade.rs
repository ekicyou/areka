//! DolaRuntime — 唯一の公開 API（Facade パターン）。

use std::collections::{BTreeSet, HashMap};

use crate::DolaError;
use crate::compile::{CompiledStoryboard, CompiledTrigger, compile_all, compile_validated};
use crate::document::DolaDocument;
use crate::easing::{EasingFunction, EasingName};
use crate::storyboard::LoopOffset;
use crate::time::{TimeDelta, TimePoint, TimeSpan};

use super::conflict_resolver;
use super::instance_manager::{InstanceManager, StoryboardInstance, TriggerState};
use super::instance_state::InstanceState;
use super::loop_controller::{self, LoopAction};
use super::subscription_manager::SubscriptionManager;
use super::timeline_manager::TimelineManager;
use super::types::{MIN_LOOP_DURATION, RuntimeError, StartResult, TriggerResult, UpdateResult};

/// dola ランタイムエンジンの唯一の公開 API。
///
/// 全外部操作のエントリーポイント。
///
/// # Usage
/// ```ignore
/// let mut rt = DolaRuntime::new();
/// let opacity_id = rt.subscribe("opacity");
/// rt.load_document(doc)?;
/// let result = rt.start("fade_in", TimePoint::ZERO)?;
/// rt.tick(TimePoint::ZERO + TimeSpan::from_millis(500));
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
    /// 同名変数は SubscriptionManager の `last_values` で値が引き継がれる。
    pub fn load_document(&mut self, doc: DolaDocument) -> Result<(), RuntimeError> {
        compile_all(&doc).map_err(RuntimeError::CompileError)?;
        // 周回の長さの規則も読み込みで確かめ、読み込めた文書が start で落ちないようにする
        for name in doc.storyboard.keys() {
            let compiled = compile_validated(&doc, name).map_err(RuntimeError::CompileError)?;
            lap_length(name, &compiled)?;
        }
        self.document = Some(doc);
        self.timeline_manager.release_unused_intern_values();
        Ok(())
    }

    /// ストーリーボード開始。
    ///
    /// 同じ変数に生きた項目を持つ古いインスタンスは、`start_time` が未来でも呼んだ時点で
    /// 競合として扱う。
    pub fn start(
        &mut self,
        name: &str,
        start_time: TimePoint,
    ) -> Result<StartResult, RuntimeError> {
        self.start_internal(name, start_time, &BTreeSet::new())
    }

    /// ストーリーボード開始（内部）。
    ///
    /// `skip_conflict_gids` に含まれる group_id は競合検出から除外される。
    /// トリガー起動時、親インスタンスを除外するために使用。
    fn start_internal(
        &mut self,
        name: &str,
        start_time: TimePoint,
        skip_conflict_gids: &BTreeSet<u64>,
    ) -> Result<StartResult, RuntimeError> {
        // セグメントは storyboard の頭からの隔たりで持つ。絶対の時刻は loop_start からの経った長さで求める
        let (mut compiled, lap_length) = self.compile_and_validate(name)?;

        let group_id = self.next_group_id;
        self.next_group_id += 1;

        let (lo_min, lo_max, lo_easing) = match &compiled.loop_offset {
            Some(LoopOffset::Scalar(v)) => (
                Some(TimeSpan::ZERO),
                *v,
                EasingFunction::Named(EasingName::Linear),
            ),
            Some(LoopOffset::Range(r)) => (Some(r.min), r.max, r.easing.clone()),
            None => (
                None,
                TimeSpan::ZERO,
                EasingFunction::Named(EasingName::Linear),
            ),
        };

        let lap_script = TimeDelta::from(compiled.total_base_duration);
        let trigger_states = compiled
            .triggers
            .iter()
            .enumerate()
            .map(|(trigger_index, trigger)| TriggerState {
                trigger_index,
                // 内か外かは台本の軸で比べる。内なら発火の時刻は周回の終わりを越えない
                lap_offset: (trigger.fire_time <= lap_script)
                    .then(|| compiled.time_scale.first_reaching(trigger.fire_time))
                    .flatten(),
                fired: false,
            })
            .collect();
        let instance = StoryboardInstance {
            storyboard_name: name.to_string(),
            state: InstanceState::Created,
            interruption_policy: compiled.interruption_policy,
            time_scale: compiled.time_scale,
            pause_accumulated: TimeDelta::ZERO,
            pause_start: None,
            loop_count: compiled.loop_count,
            loops_completed: 0,
            finish_deadline: None,
            loop_start: start_time,
            lap_length,
            loop_offset_min: lo_min,
            loop_offset_max: lo_max,
            loop_offset_easing: lo_easing,
            trigger_states,
        };
        let end_time = instance.planned_end();
        // group_id は単調増加の採番で既存の鍵と衝突しない
        self.instance_manager
            .instances_mut()
            .insert(group_id, instance);

        let affected = conflict_resolver::resolve_conflicts_excluding(
            group_id,
            &compiled,
            start_time,
            &mut self.timeline_manager,
            &mut self.instance_manager,
            &mut self.subscription_manager,
            skip_conflict_gids,
        )?;
        for gid in &affected {
            self.trigger_store.remove(gid);
        }

        self.timeline_manager.insert_entries(group_id, &compiled);

        if !compiled.triggers.is_empty() {
            self.trigger_store
                .insert(group_id, std::mem::take(&mut compiled.triggers));
        }

        self.instance_manager
            .transition(group_id, InstanceState::Playing)?;

        Ok(StartResult {
            group_id,
            end_time,
            affected_group_ids: affected,
        })
    }

    /// 成功時は（コンパイル結果, 1 周の経った長さ）を返す。
    fn compile_and_validate(
        &self,
        name: &str,
    ) -> Result<(CompiledStoryboard, TimeSpan), RuntimeError> {
        let doc = self
            .document
            .as_ref()
            .ok_or_else(|| RuntimeError::StoryboardNotFound(name.to_string()))?;

        if !doc.storyboard.contains_key(name) {
            return Err(RuntimeError::StoryboardNotFound(name.to_string()));
        }

        let compiled = compile_validated(doc, name).map_err(RuntimeError::CompileError)?;

        // load_document が同じ規則を通しているので、ここでは落ちない
        let lap_length = lap_length(name, &compiled)?;
        Ok((compiled, lap_length))
    }

    /// 一時停止。
    pub fn pause(&mut self, group_id: u64, now: TimePoint) -> Result<(), RuntimeError> {
        self.instance_manager.pause(group_id, now)
    }

    /// 再開。戻した後の、再生の全体の終わりの時刻を返す。決まっていなければ None
    /// （`StartResult.end_time` と同じ意味）。
    pub fn resume(
        &mut self,
        group_id: u64,
        now: TimePoint,
    ) -> Result<Option<TimePoint>, RuntimeError> {
        self.instance_manager.resume(group_id, now)
    }

    /// 最終値ジャンプ終了（Conclude）。
    pub fn conclude(&mut self, group_id: u64) -> Result<(), RuntimeError> {
        self.instance_manager.get(group_id)?;
        self.conclude_internal(group_id);
        Ok(())
    }

    /// 現在値凍結破棄（Cancel）。
    ///
    /// エントリ削除のみ（last_values は前回 tick の値が自然に残る）。
    pub fn cancel(&mut self, group_id: u64) -> Result<(), RuntimeError> {
        let instance = self.instance_manager.get(group_id)?;
        if instance.state.is_terminal() {
            return Err(RuntimeError::InvalidGroupId(group_id));
        }

        self.instance_manager
            .transition(group_id, InstanceState::Cancelled)?;
        self.timeline_manager.remove_entries(group_id);
        self.trigger_store.remove(&group_id);
        Ok(())
    }

    /// 遅延 Conclude。
    ///
    /// `deadline` は `tick()` に渡す時刻と同じ物差しの絶対時刻。`tick()` 内で `now >= deadline` の
    /// タイミングで Conclude 相当の処理を実行する。
    pub fn finish(&mut self, group_id: u64, deadline: TimePoint) -> Result<(), RuntimeError> {
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
    // 2 フェーズ API
    // =========================================================================

    /// Phase 1: 内部状態を now まで進行し、結果を内部フィールドに格納。
    ///
    /// `TimedSchedule::tick()` と対称な設計。
    pub fn tick(&mut self, now: TimePoint) {
        self.last_update_result = self.update_internal(now);
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
    /// 0〜2 を、now までの出来事の時刻（`next_event_time`）の順に 1 つずつ回す。
    /// 同じ時刻では 0 → 1 → 1.5 → 2 の順（トリガーどうしは親の group_id の順）
    /// 0. トリガーなしの無限のループを、その時刻より前に終わる周回まで進める
    /// 1. finish deadline 到達インスタンスは Conclude 相当
    ///    1.5 トリガー収集・実行（Playing インスタンスのトリガーを先に処理）
    /// 2. 自然終了検知（その時刻が周回の終わり以後の Playing インスタンス）
    ///
    /// 最後に、出来事に入れない無限のループ（トリガーなし）を now まで進めてから、
    /// 購読変数の評価と差分検出を行う。
    fn update_internal(&mut self, now: TimePoint) -> UpdateResult {
        // 時刻の順に回すのは、子を起こす順がぶつかる子の勝ち負けを決めるから。
        // 1 コマずつ回したときと同じ順になる
        let mut trigger_results = Vec::new();
        let mut rng = rand::rng();
        while let Some(t) = self.next_event_time().filter(|&t| t <= now) {
            // Step 0: トリガーなしの無限のループを t の 100ns 手前までに終わる周回まで進める（60fps の前のコマと同じ周回にする）。
            // ほかの Playing の周回の終わりは出来事なので t 以上で、ここでは動かない。t ちょうどの周回は Step 2 で
            if let Some(before) = t.checked_add_delta(TimeDelta::from_ticks(-1)) {
                self.process_loop_ends(before, &mut rng);
            }
            for gid in self.instance_manager.check_finish_deadlines(t) {
                self.conclude_internal(gid);
            }
            // Conclude 前に発火させる
            trigger_results.extend(self.process_triggers(t));
            self.process_loop_ends(t, &mut rng);
        }
        self.process_loop_ends(now, &mut rng);

        // 評価と差分検出は同じ周回で行う
        let timeline_manager = &mut self.timeline_manager;
        let instances = self.instance_manager.instances();
        let changes = self
            .subscription_manager
            .diff_and_update(|name| timeline_manager.evaluate(name, now, instances));

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
    fn next_event_time(&self) -> Option<TimePoint> {
        let mut next: Option<TimePoint> = None;
        let mut take = |t: TimePoint| next = Some(next.map_or(t, |n| n.min(t)));
        for (gid, inst) in self.instance_manager.instances() {
            if let Some(deadline) = inst.finish_deadline {
                take(deadline);
            }
            if inst.state != InstanceState::Playing {
                continue;
            }
            if (inst.loop_count != -1 || !inst.trigger_states.is_empty())
                && let Some(end) = inst.lap_end()
            {
                take(end);
            }
            if self.trigger_store.contains_key(gid) {
                for ts in inst.trigger_states.iter().filter(|ts| !ts.fired) {
                    if let Some(wall) = trigger_wall_time(inst, ts) {
                        take(wall);
                    }
                }
            }
        }
        next
    }

    /// `time` までに周回の終わった Playing のインスタンスを進め、終わったものを Conclude する
    fn process_loop_ends(&mut self, time: TimePoint, rng: &mut impl rand::Rng) {
        let loop_results: Vec<(u64, LoopAction)> = self
            .instance_manager
            .instances_mut()
            .iter_mut()
            .filter(|(_, inst)| {
                inst.state == InstanceState::Playing
                    && inst.lap_end().is_some_and(|end| time >= end)
            })
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
    /// トリガーしか集まらない。トリガーの循環は検証が拒むので、繰り返しは
    /// ストーリーボードの数 + 1 回で止まる。
    fn process_triggers(&mut self, now: TimePoint) -> Vec<TriggerResult> {
        let mut results = Vec::new();
        loop {
            let fired = self.fire_due_triggers(now);
            if fired.is_empty() {
                return results;
            }
            results.extend(fired);
        }
    }

    /// Playing 状態のインスタンスに紐づくトリガーを走査し、
    /// 発火条件（now >= 発火時刻 かつ 未発火）を満たすものを
    /// fire-and-forget で子ストーリーボードとして開始する（1 回分）。
    fn fire_due_triggers(&mut self, now: TimePoint) -> Vec<TriggerResult> {
        // 借用を分離するため、発火対象を先に Vec へ集約する
        struct PendingTrigger {
            parent_group_id: u64,
            trigger_index: usize,
            target_storyboard: String,
            child_start_time: TimePoint,
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
                    // SAFETY(panic 経路): trigger_states は 0..compiled.triggers.len() の添字から構築され、
                    // trigger_store には同一の compiled.triggers が同じ group_id で格納される。
                    // trigger_store は自然終了・cancel・競合の終わりで消え、group_id は再利用されないため、
                    // ts.trigger_index < triggers.len() が常に成立し添字 panic は発火しない。
                    debug_assert!(
                        ts.trigger_index < triggers.len(),
                        "trigger_states/trigger_store invariant violated: index {} >= len {}",
                        ts.trigger_index,
                        triggers.len()
                    );
                    let trigger = &triggers[ts.trigger_index];
                    if let Some(wall_fire_time) = trigger_wall_time(inst, ts)
                        && now >= wall_fire_time
                    {
                        let child_start =
                            wall_fire_time + trigger.start_offset.unwrap_or(TimeDelta::ZERO);
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

        // p.trigger_index は収集時に同一インスタンスの trigger_states を enumerate した添字で、
        // 収集からここまで trigger_states は変わらないため範囲外 panic は発火しない
        for p in &pending {
            if let Ok(inst) = self.instance_manager.get_mut(p.parent_group_id) {
                inst.trigger_states[p.trigger_index].fired = true;
            }
        }

        // 親インスタンスは競合検出から除外する
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
    fn conclude_internal(&mut self, group_id: u64) {
        let final_values = self.timeline_manager.collect_final_values(group_id);

        if !final_values.is_empty() {
            let id_values = self
                .subscription_manager
                .convert_names_to_ids(&final_values);
            self.subscription_manager
                .force_update_last_values(&id_values);
        }

        let _ = self
            .instance_manager
            .transition(group_id, InstanceState::Concluded);
        self.timeline_manager.remove_entries(group_id);
        self.trigger_store.remove(&group_id);
    }
}

/// トリガーの壁時計の発火時刻: 周回の起点 + 発火の台本の時刻に届く最初の経った長さ。
/// 周回の長さより後ろのトリガーは None（親の周回が先に終わる）。
fn trigger_wall_time(inst: &StoryboardInstance, trigger: &TriggerState) -> Option<TimePoint> {
    trigger
        .lap_offset
        .map(|offset| inst.loop_start + inst.pause_accumulated + offset)
}

impl Default for DolaRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// 1 周の経った長さ。無限のループの周回が 0 か `MIN_LOOP_DURATION` 未満なら拒む。
fn lap_length(name: &str, compiled: &CompiledStoryboard) -> Result<TimeSpan, RuntimeError> {
    // 表せない 1 周はコンパイルが拒むので、ここでは落ちない
    let lap = compiled.lap_length().ok_or_else(|| {
        RuntimeError::CompileError(vec![DolaError::LoopDurationOutOfRange {
            location: format!("storyboard '{name}'"),
        }])
    })?;
    if compiled.loop_count == -1 {
        if lap == TimeSpan::ZERO {
            return Err(RuntimeError::ZeroDurationWithLoop {
                storyboard: name.to_string(),
            });
        }
        if lap < MIN_LOOP_DURATION {
            return Err(RuntimeError::TooShortDurationWithInfiniteLoop {
                storyboard: name.to_string(),
                duration: lap,
            });
        }
    }
    Ok(lap)
}

#[cfg(test)]
#[path = "facade_tests.rs"]
mod tests;
