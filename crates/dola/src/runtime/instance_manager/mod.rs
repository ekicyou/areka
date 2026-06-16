//! ストーリーボード実行インスタンスのコレクション管理と状態遷移制御。

use std::collections::HashMap;

use crate::easing::EasingFunction;
use crate::storyboard::InterruptionPolicy;

use super::instance_state::InstanceState;
use super::types::RuntimeError;

/// トリガー発火状態（StoryboardInstance に埋め込み）
#[derive(Debug, Clone)]
pub(crate) struct TriggerState {
    /// CompiledTrigger のインデックス
    pub trigger_index: usize,
    /// 当該周回で発火済みか
    pub fired: bool,
}

/// ストーリーボード実行インスタンス。
///
/// 各 `start()` 呼び出しで 1 つ生成され、`group_id` で一意に識別される。
#[allow(dead_code)]
pub(crate) struct StoryboardInstance {
    pub group_id: u64,
    pub storyboard_name: String,
    pub state: InstanceState,
    pub interruption_policy: InterruptionPolicy,
    pub start_time: f64,
    pub time_scale: f64,
    pub base_duration: f64,
    pub pause_accumulated: f64,
    pub pause_start: Option<f64>,
    /// 1=1回, n≥2=n回, -1=無限ループ
    pub loop_count: i32,
    /// 完了済み周回数
    pub loops_completed: u64,
    pub finish_deadline: Option<f64>,
    /// end_time (loop_start_time + loop_duration + pause_accumulated)
    pub end_time: f64,
    /// 現在の周回の開始時刻（wall clock ベース）。
    /// 初期値は `start_time` と同一。周回終了ごとに `+= loop_duration` で更新。
    /// Pause/Resume では変更されない（独立性: Req 4.3）。
    pub loop_start_time: f64,
    /// 1周分の再生時間（wall clock ベース）。
    /// `base_duration / time_scale` で算出。インスタンス生存中は定数。
    pub loop_duration: f64,
    /// ループオフセット最小値（f64秒）。None = オフセットなし。
    pub loop_offset_min: Option<f64>,
    /// ループオフセット最大値（f64秒）。
    pub loop_offset_max: f64,
    /// ループオフセット用イージング関数。
    pub loop_offset_easing: EasingFunction,
    /// トリガー発火状態（周回ごとにリセット）
    pub trigger_states: Vec<TriggerState>,
}

/// 実行インスタンスのコレクション管理と状態遷移制御。
pub(crate) struct InstanceManager {
    instances: HashMap<u64, StoryboardInstance>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    /// インスタンス作成（Created 状態）。
    pub fn create_instance(
        &mut self,
        group_id: u64,
        name: &str,
        policy: InterruptionPolicy,
        start_time: f64,
        time_scale: f64,
        base_duration: f64,
        loop_count: i32,
        end_time: f64,
        loop_start_time: f64,
        loop_duration: f64,
        loop_offset_min: Option<f64>,
        loop_offset_max: f64,
        loop_offset_easing: EasingFunction,
        trigger_count: usize,
    ) -> &StoryboardInstance {
        let trigger_states = (0..trigger_count)
            .map(|i| TriggerState {
                trigger_index: i,
                fired: false,
            })
            .collect();
        let instance = StoryboardInstance {
            group_id,
            storyboard_name: name.to_string(),
            state: InstanceState::Created,
            interruption_policy: policy,
            start_time,
            time_scale,
            base_duration,
            pause_accumulated: 0.0,
            pause_start: None,
            loop_count,
            loops_completed: 0,
            finish_deadline: None,
            end_time,
            loop_start_time,
            loop_duration,
            loop_offset_min,
            loop_offset_max,
            loop_offset_easing,
            trigger_states,
        };
        // NOTE(不変条件): group_id は facade の next_group_id 単調増加カウンタで採番され
        // 再利用されないため、正当な経路で既存キーと衝突しない。衝突時は旧インスタンスを
        // 黙って上書きする（tests.rs::create_instance_with_same_group_id_overwrites で特性化済み）。
        self.instances.insert(group_id, instance);
        // SAFETY(panic 経路): 直前の insert により必ず Some。
        self.instances.get(&group_id).unwrap()
    }

    /// 参照取得（InvalidGroupId エラー対応）。
    pub fn get(&self, group_id: u64) -> Result<&StoryboardInstance, RuntimeError> {
        self.instances
            .get(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))
    }

    /// 可変参照取得（InvalidGroupId エラー対応）。
    pub fn get_mut(&mut self, group_id: u64) -> Result<&mut StoryboardInstance, RuntimeError> {
        self.instances
            .get_mut(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))
    }

    /// 状態遷移（try_transition 経由）。
    ///
    /// 不変条件: 終了状態への遷移は同時にインスタンスを自動削除するため、
    /// `instances` マップに終了状態のインスタンスは存在しない
    /// （`set_finish_deadline` / `check_finish_deadlines` の is_terminal チェックの前提）。
    pub fn transition(&mut self, group_id: u64, to: InstanceState) -> Result<(), RuntimeError> {
        let instance = self.get_mut(group_id)?;

        match instance.state.try_transition(to) {
            Ok(new_state) => {
                instance.state = new_state;
                // 終了状態に遷移した場合、インスタンスを自動削除
                if new_state.is_terminal() {
                    self.instances.remove(&group_id);
                }
                Ok(())
            }
            // NOTE(エラー表現): 不正な状態遷移も InvalidGroupId として報告される
            // （遷移エラー専用バリアントが存在しない現行仕様。
            // tests.rs::invalid_transition_on_existing_instance_reports_invalid_group_id で
            // 特性化済み。エラー種別の分離は挙動変更を伴うため proposals.md P16 参照）。
            Err(_current) => Err(RuntimeError::InvalidGroupId(group_id)),
        }
    }

    /// Pause: Paused 遷移（pause_start は facade が `set_pause_start` で設定する）。
    pub fn pause(&mut self, group_id: u64) -> Result<(), RuntimeError> {
        self.transition(group_id, InstanceState::Paused)
    }

    /// Pause 時の pause_start を設定する（facade から呼ばれる）。
    pub fn set_pause_start(
        &mut self,
        group_id: u64,
        current_time: f64,
    ) -> Result<(), RuntimeError> {
        self.get_mut(group_id)?.pause_start = Some(current_time);
        Ok(())
    }

    /// Resume: pause_accumulated 加算 + Playing 遷移 + end_time 再計算。
    pub fn resume(&mut self, group_id: u64, current_time: f64) -> Result<f64, RuntimeError> {
        let instance = self.get_mut(group_id)?;

        match instance.state.try_transition(InstanceState::Playing) {
            Ok(new_state) => {
                // pause_accumulated 加算
                if let Some(pause_start) = instance.pause_start.take() {
                    // NOTE(数値境界): current_time < pause_start（非単調な時刻入力）の場合、
                    // pause_duration が負となり pause_accumulated / end_time が過去方向へ
                    // 補正される（早期終了の誘発）。現行 API は呼び出し側の時刻単調性を
                    // 検証しない（検証追加は挙動変更を伴うため proposals.md P15 参照。
                    // tests.rs::resume_with_time_before_pause_start_shrinks_end_time で特性化済み）。
                    let pause_duration = current_time - pause_start;
                    instance.pause_accumulated += pause_duration;
                    // end_time 再計算
                    instance.end_time += pause_duration;
                }
                instance.state = new_state;
                Ok(instance.end_time)
            }
            Err(_) => Err(RuntimeError::InvalidGroupId(group_id)),
        }
    }

    /// Finish deadline 設定。
    pub fn set_finish_deadline(
        &mut self,
        group_id: u64,
        deadline: f64,
    ) -> Result<(), RuntimeError> {
        let instance = self.get_mut(group_id)?;

        // NOTE(防御): transition() の不変条件（終了遷移＝自動削除）により、マップ内に
        // 終了状態のインスタンスは存在せず、この分岐は現行不変条件の下では到達しない。
        if instance.state.is_terminal() {
            return Err(RuntimeError::InvalidGroupId(group_id));
        }

        instance.finish_deadline = Some(deadline);
        Ok(())
    }

    /// Finish deadline が到達した group_id のリストを返却。
    ///
    /// NOTE(数値境界): deadline が NaN の場合 `current_time >= deadline` は常に false と
    /// なり、当該 deadline は黙って発火しない（NaN 流入は指示書数値の有限性検証の
    /// 欠如による: proposals.md P8/P14 参照）。is_terminal チェックは transition() の
    /// 不変条件（終了遷移＝自動削除）により現行では常に false の防御チェック。
    pub fn check_finish_deadlines(&self, current_time: f64) -> Vec<u64> {
        self.instances
            .iter()
            .filter(|(_, inst)| {
                if let Some(deadline) = inst.finish_deadline {
                    !inst.state.is_terminal() && current_time >= deadline
                } else {
                    false
                }
            })
            .map(|(gid, _)| *gid)
            .collect()
    }

    /// 全インスタンスへの参照。
    pub fn instances(&self) -> &HashMap<u64, StoryboardInstance> {
        &self.instances
    }

    /// 全インスタンスへの可変参照。
    pub fn instances_mut(&mut self) -> &mut HashMap<u64, StoryboardInstance> {
        &mut self.instances
    }

    /// 指定 group_id のインスタンスを削除する（Concluded/Cancelled 後の cleanup）。
    pub fn remove(&mut self, group_id: u64) {
        self.instances.remove(&group_id);
    }
}

#[cfg(test)]
mod tests;
