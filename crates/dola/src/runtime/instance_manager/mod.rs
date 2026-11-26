//! ストーリーボード実行インスタンスのコレクション管理と状態遷移制御。

use std::collections::BTreeMap;

use crate::easing::EasingFunction;
use crate::storyboard::InterruptionPolicy;
use crate::time::{TimeDelta, TimePoint, TimeScale, TimeSpan};

use super::instance_state::InstanceState;
use super::types::RuntimeError;

/// トリガー発火状態（StoryboardInstance に埋め込み）
#[derive(Debug, Clone)]
pub(crate) struct TriggerState {
    /// CompiledTrigger のインデックス
    pub trigger_index: usize,
    /// 周回の始まりから発火までの経った長さ（発火の台本の時刻に届く最初の長さ）。
    /// 周回の長さより後ろのトリガーは None（親の周回が先に終わる）
    pub lap_offset: Option<TimeDelta>,
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
    pub time_scale: TimeScale,
    /// 止めていた長さの和（符号つき）
    pub pause_accumulated: TimeDelta,
    pub pause_start: Option<TimePoint>,
    /// 1=1回, n≥2=n回, -1=無限ループ
    pub loop_count: i32,
    /// 完了済み周回数
    pub loops_completed: u64,
    pub finish_deadline: Option<TimePoint>,
    /// 今の周回の始まり。初期値は `start()` に渡した開始時刻。
    /// 周回終了ごとに「前の周回の終わり＋待ち−pause_accumulated」で更新し、Pause/Resume では変えない。
    pub loop_start: TimePoint,
    /// 1 周の経った長さ（`CompiledStoryboard::lap_length`）。インスタンス生存中は定数。
    pub lap_length: TimeSpan,
    /// ループオフセット最小値。None = オフセットなし。
    pub loop_offset_min: Option<TimeSpan>,
    /// ループオフセット最大値。
    pub loop_offset_max: TimeSpan,
    /// ループオフセット用イージング関数。
    pub loop_offset_easing: EasingFunction,
    /// トリガー発火状態（周回ごとにリセット）
    pub trigger_states: Vec<TriggerState>,
}

impl StoryboardInstance {
    /// 周回の始まりからの経った長さ。止まっていれば止めた時刻で求める。
    pub fn elapsed(&self, now: TimePoint) -> TimeDelta {
        let now = self.pause_start.unwrap_or(now);
        now - self.loop_start - self.pause_accumulated
    }

    /// 今の周回の終わりの時刻。型の範囲を越えれば None（その周回は終わらない）。
    pub fn lap_end(&self) -> Option<TimePoint> {
        self.loop_start
            .checked_add_delta(self.pause_accumulated)?
            .checked_add(self.lap_length)
    }

    /// 再生の全体の終わりの時刻。今決まっていなければ None
    /// （終わらないループ・周回の間の待ちをこれから引くループ・和が型の範囲を越えるとき）。
    pub fn planned_end(&self) -> Option<TimePoint> {
        let lap_end = self.lap_end()?;
        if self.loop_count == 1 {
            return Some(lap_end);
        }
        // 負の loop_count は -1（無限）だけ（validate が限る）
        let remaining = u64::try_from(self.loop_count)
            .ok()?
            .checked_sub(self.loops_completed + 1)?;
        if remaining == 0 {
            return Some(lap_end);
        }
        let wait = match self.loop_offset_min {
            None => TimeSpan::ZERO,
            Some(min) if min == self.loop_offset_max => min,
            Some(_) => return None,
        };
        let per_lap = wait.checked_add(self.lap_length)?.as_ticks();
        let rest = per_lap.checked_mul(i64::try_from(remaining).ok()?)?;
        lap_end.checked_add(TimeSpan::try_from_ticks(rest).ok()?)
    }
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
            // （`tests.rs` の `invalid_transition_on_existing_instance_reports_invalid_group_id`）。
            Err(_current) => Err(RuntimeError::InvalidGroupId(group_id)),
        }
    }

    /// Pause: Paused 遷移と pause_start の記録を 1 段で行う（Paused なら pause_start は必ずある）。
    pub fn pause(&mut self, group_id: u64, now: TimePoint) -> Result<(), RuntimeError> {
        self.transition(group_id, InstanceState::Paused)?;
        self.get_mut(group_id)?.pause_start = Some(now);
        Ok(())
    }

    /// Resume: pause_accumulated 加算 + Playing 遷移。戻した後の再生の全体の終わりの時刻を返す。
    pub fn resume(
        &mut self,
        group_id: u64,
        now: TimePoint,
    ) -> Result<Option<TimePoint>, RuntimeError> {
        let instance = self.get_mut(group_id)?;

        match instance.state.try_transition(InstanceState::Playing) {
            Ok(new_state) => {
                // Paused からの遷移なので pause_start は pause が入れてある。
                // now < pause_start（非単調な時刻入力）だと止めていた長さが負になり、
                // 周回の終わりが前へ動く（早期終了の誘発）。呼び出し側の時刻の単調性は検証しない
                // （`tests.rs` の `resume_with_time_before_pause_start_shrinks_end_time`）。
                let pause_start = instance.pause_start.take().unwrap_or(now);
                instance.pause_accumulated = instance.pause_accumulated + (now - pause_start);
                instance.state = new_state;
                Ok(instance.planned_end())
            }
            Err(_) => Err(RuntimeError::InvalidGroupId(group_id)),
        }
    }

    /// Finish deadline 設定。
    pub fn set_finish_deadline(
        &mut self,
        group_id: u64,
        deadline: TimePoint,
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
    /// is_terminal チェックは transition() が終了状態を削除するので、現状では常に false の防御。
    pub fn check_finish_deadlines(&self, now: TimePoint) -> Vec<u64> {
        self.instances
            .iter()
            .filter(|(_, inst)| {
                if let Some(deadline) = inst.finish_deadline {
                    !inst.state.is_terminal() && now >= deadline
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
