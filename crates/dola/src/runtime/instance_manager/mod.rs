//! ストーリーボード実行インスタンスのコレクション管理と状態遷移制御。

use std::collections::BTreeMap;

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
/// 各 `start()` 呼び出しで 1 つ生成され、`InstanceManager` の表の鍵の group_id で識別される。
pub(crate) struct StoryboardInstance {
    pub storyboard_name: String,
    pub state: InstanceState,
    pub interruption_policy: InterruptionPolicy,
    pub time_scale: f64,
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
    /// 初期値は `start()` に渡した開始時刻。周回終了ごとに「前の end_time＋待ち−pause_accumulated」で更新。
    /// Pause/Resume では変更されない。
    pub loop_start_time: f64,
    /// 1周分の再生時間（wall clock ベース）。
    /// `total_base_duration / time_scale` で算出。インスタンス生存中は定数。
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
    instances: BTreeMap<u64, StoryboardInstance>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
        }
    }

    /// 参照取得。
    pub fn get(&self, group_id: u64) -> Result<&StoryboardInstance, RuntimeError> {
        self.instances
            .get(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))
    }

    /// 可変参照取得。
    pub fn get_mut(&mut self, group_id: u64) -> Result<&mut StoryboardInstance, RuntimeError> {
        self.instances
            .get_mut(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))
    }

    /// 状態遷移。
    ///
    /// 終了状態への遷移は同時にインスタンスを自動削除するため、
    /// `instances` マップに終了状態のインスタンスは存在しない。
    pub fn transition(&mut self, group_id: u64, to: InstanceState) -> Result<(), RuntimeError> {
        let instance = self.get_mut(group_id)?;

        match instance.state.try_transition(to) {
            Ok(new_state) => {
                instance.state = new_state;
                if new_state.is_terminal() {
                    self.instances.remove(&group_id);
                }
                Ok(())
            }
            // 遷移エラー専用のバリアントが無いので、不正な状態遷移も InvalidGroupId として報告される
            Err(_current) => Err(RuntimeError::InvalidGroupId(group_id)),
        }
    }

    /// Pause: Paused 遷移と pause_start の記録を 1 段で行う（Paused なら pause_start は必ずある）。
    pub fn pause(&mut self, group_id: u64, current_time: f64) -> Result<(), RuntimeError> {
        self.transition(group_id, InstanceState::Paused)?;
        self.get_mut(group_id)?.pause_start = Some(current_time);
        Ok(())
    }

    /// Resume: pause_accumulated 加算 + Playing 遷移 + end_time 再計算。
    pub fn resume(&mut self, group_id: u64, current_time: f64) -> Result<f64, RuntimeError> {
        let instance = self.get_mut(group_id)?;

        match instance.state.try_transition(InstanceState::Playing) {
            Ok(new_state) => {
                // Paused からの遷移なので pause_start は pause が入れてある。
                // current_time < pause_start（非単調な時刻入力）だと pause_duration が負になり、
                // pause_accumulated / end_time が過去方向へ補正される（早期終了の誘発）。
                // 呼び出し側の時刻の単調性は検証しない。
                let pause_start = instance.pause_start.take().unwrap_or(current_time);
                let pause_duration = current_time - pause_start;
                instance.pause_accumulated += pause_duration;
                instance.end_time += pause_duration;
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

        // transition() が終了状態のインスタンスを削除するので、この分岐は現状では到達しない防御。
        if instance.state.is_terminal() {
            return Err(RuntimeError::InvalidGroupId(group_id));
        }

        instance.finish_deadline = Some(deadline);
        Ok(())
    }

    /// Finish deadline が到達した group_id のリストを返却。
    ///
    /// deadline が NaN の場合 `current_time >= deadline` は常に false となり、当該 deadline は
    /// 黙って発火しない（指示書の数値は validate が有限に限るので、NaN が入るのは呼び手の渡す
    /// 時刻が NaN のときだけ）。is_terminal チェックは transition() が終了状態を削除するので
    /// 現状では常に false の防御。
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
    pub fn instances(&self) -> &BTreeMap<u64, StoryboardInstance> {
        &self.instances
    }

    /// 全インスタンスへの可変参照。
    pub fn instances_mut(&mut self) -> &mut BTreeMap<u64, StoryboardInstance> {
        &mut self.instances
    }

    /// 指定 group_id のインスタンスを削除する（Concluded/Cancelled 後の cleanup）。
    pub fn remove(&mut self, group_id: u64) {
        self.instances.remove(&group_id);
    }
}

#[cfg(test)]
mod tests;
