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
        self.instances.insert(group_id, instance);
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
    pub fn transition(&mut self, group_id: u64, to: InstanceState) -> Result<(), RuntimeError> {
        let instance = self
            .instances
            .get_mut(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))?;

        match instance.state.try_transition(to) {
            Ok(new_state) => {
                instance.state = new_state;
                // 終了状態に遷移した場合、インスタンスを自動削除
                if new_state.is_terminal() {
                    self.instances.remove(&group_id);
                }
                Ok(())
            }
            Err(_current) => Err(RuntimeError::InvalidGroupId(group_id)),
        }
    }

    /// Pause: pause_start 記録 + Paused 遷移。
    pub fn pause(&mut self, group_id: u64) -> Result<(), RuntimeError> {
        let instance = self
            .instances
            .get_mut(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))?;

        // Paused 遷移を試行
        match instance.state.try_transition(InstanceState::Paused) {
            Ok(new_state) => {
                // pause_start は現在の effective_time 起点で記録
                // ただし actual time を記録（resume 時に差分計算する）
                // pause_start はfacade側のcurrent_timeを使うべきだが、
                // facade が呼ぶ時に設定する設計のため、ここでは状態遷移のみ
                instance.state = new_state;
                Ok(())
            }
            Err(_) => Err(RuntimeError::InvalidGroupId(group_id)),
        }
    }

    /// Pause 時の pause_start を設定する（facade から呼ばれる）。
    pub fn set_pause_start(
        &mut self,
        group_id: u64,
        current_time: f64,
    ) -> Result<(), RuntimeError> {
        let instance = self
            .instances
            .get_mut(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))?;
        instance.pause_start = Some(current_time);
        Ok(())
    }

    /// Resume: pause_accumulated 加算 + Playing 遷移 + end_time 再計算。
    pub fn resume(&mut self, group_id: u64, current_time: f64) -> Result<f64, RuntimeError> {
        let instance = self
            .instances
            .get_mut(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))?;

        match instance.state.try_transition(InstanceState::Playing) {
            Ok(new_state) => {
                // pause_accumulated 加算
                if let Some(pause_start) = instance.pause_start.take() {
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
        let instance = self
            .instances
            .get_mut(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))?;

        if instance.state.is_terminal() {
            return Err(RuntimeError::InvalidGroupId(group_id));
        }

        instance.finish_deadline = Some(deadline);
        Ok(())
    }

    /// Finish deadline が到達した group_id のリストを返却。
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
#[path = "instance_manager_tests.rs"]
mod tests;
