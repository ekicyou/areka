//! ストーリーボード実行インスタンスのコレクション管理と状態遷移制御。

use std::collections::HashMap;

use crate::storyboard::InterruptionPolicy;

use super::instance_state::InstanceState;
use super::types::RuntimeError;

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
    ) -> &StoryboardInstance {
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

    /// 状態遷移（try_transition 経由）。
    pub fn transition(&mut self, group_id: u64, to: InstanceState) -> Result<(), RuntimeError> {
        let instance = self
            .instances
            .get_mut(&group_id)
            .ok_or(RuntimeError::InvalidGroupId(group_id))?;

        match instance.state.try_transition(to) {
            Ok(new_state) => {
                instance.state = new_state;
                // Concluded 状態に遷移した場合、インスタンスを削除 (Req 9.7)
                if to == InstanceState::Concluded {
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
mod tests {
    use super::*;

    fn create_test_instance(mgr: &mut InstanceManager, group_id: u64) {
        mgr.create_instance(
            group_id,
            "test_sb",
            InterruptionPolicy::Conclude,
            0.0, // start_time
            1.0, // time_scale
            2.0, // base_duration
            1,   // loop_count
            2.0, // end_time
            0.0, // loop_start_time
            2.0, // loop_duration
        );
    }

    #[test]
    fn create_instance_initial_state() {
        let mut mgr = InstanceManager::new();
        let inst = mgr.create_instance(
            1,
            "fade",
            InterruptionPolicy::Conclude,
            10.0,
            1.0,
            5.0,
            1,
            15.0,
            10.0, // loop_start_time
            5.0,  // loop_duration
        );
        assert_eq!(inst.group_id, 1);
        assert_eq!(inst.storyboard_name, "fade");
        assert_eq!(inst.state, InstanceState::Created);
        assert_eq!(inst.start_time, 10.0);
        assert_eq!(inst.base_duration, 5.0);
        assert_eq!(inst.pause_accumulated, 0.0);
        assert!(inst.pause_start.is_none());
        assert!(inst.finish_deadline.is_none());
    }

    #[test]
    fn transition_created_to_playing() {
        let mut mgr = InstanceManager::new();
        create_test_instance(&mut mgr, 1);
        assert!(mgr.transition(1, InstanceState::Playing).is_ok());
        assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
    }

    #[test]
    fn transition_playing_to_paused() {
        let mut mgr = InstanceManager::new();
        create_test_instance(&mut mgr, 1);
        mgr.transition(1, InstanceState::Playing).unwrap();
        assert!(mgr.transition(1, InstanceState::Paused).is_ok());
        assert_eq!(mgr.get(1).unwrap().state, InstanceState::Paused);
    }

    #[test]
    fn transition_paused_to_playing() {
        let mut mgr = InstanceManager::new();
        create_test_instance(&mut mgr, 1);
        mgr.transition(1, InstanceState::Playing).unwrap();
        mgr.transition(1, InstanceState::Paused).unwrap();
        assert!(mgr.transition(1, InstanceState::Playing).is_ok());
        assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
    }

    #[test]
    fn transition_to_concluded_removes_instance() {
        let mut mgr = InstanceManager::new();
        create_test_instance(&mut mgr, 1);
        mgr.transition(1, InstanceState::Playing).unwrap();
        mgr.transition(1, InstanceState::Concluded).unwrap();
        // Concluded 遷移後はインスタンス削除
        assert!(mgr.get(1).is_err());
    }

    #[test]
    fn invalid_transition_rejected() {
        let mut mgr = InstanceManager::new();
        create_test_instance(&mut mgr, 1);
        // Created → Paused は不正
        assert!(mgr.transition(1, InstanceState::Paused).is_err());
    }

    #[test]
    fn nonexistent_group_id_error() {
        let mgr = InstanceManager::new();
        assert!(mgr.get(999).is_err());
    }

    #[test]
    fn pause_and_resume() {
        let mut mgr = InstanceManager::new();
        create_test_instance(&mut mgr, 1);
        mgr.transition(1, InstanceState::Playing).unwrap();

        // Pause at t=1.0
        mgr.pause(1).unwrap();
        mgr.set_pause_start(1, 1.0).unwrap();
        assert_eq!(mgr.get(1).unwrap().state, InstanceState::Paused);
        assert_eq!(mgr.get(1).unwrap().pause_start, Some(1.0));

        // Resume at t=3.0 (2秒間 pause)
        let new_end = mgr.resume(1, 3.0).unwrap();
        assert_eq!(mgr.get(1).unwrap().state, InstanceState::Playing);
        assert_eq!(mgr.get(1).unwrap().pause_accumulated, 2.0);
        assert!(mgr.get(1).unwrap().pause_start.is_none());
        // end_time = 2.0 + 2.0(pause) = 4.0
        assert_eq!(new_end, 4.0);
    }

    #[test]
    fn finish_deadline_check() {
        let mut mgr = InstanceManager::new();
        create_test_instance(&mut mgr, 1);
        mgr.transition(1, InstanceState::Playing).unwrap();
        mgr.set_finish_deadline(1, 5.0).unwrap();

        // t=4.0: まだ deadline に達していない
        assert!(mgr.check_finish_deadlines(4.0).is_empty());

        // t=5.0: deadline に達した
        let expired = mgr.check_finish_deadlines(5.0);
        assert_eq!(expired, vec![1]);
    }

    #[test]
    fn finish_deadline_on_terminal_state_rejected() {
        let mut mgr = InstanceManager::new();
        create_test_instance(&mut mgr, 1);
        mgr.transition(1, InstanceState::Playing).unwrap();
        mgr.transition(1, InstanceState::Cancelled).unwrap();
        // Cancelled は terminal → deadline 設定不可（ただしインスタンスは残る）
        // Note: Cancelled は Concluded と違い自動削除しない
        assert!(mgr.set_finish_deadline(1, 5.0).is_err());
    }

    #[test]
    fn multiple_independent_instances() {
        let mut mgr = InstanceManager::new();
        create_test_instance(&mut mgr, 1);
        create_test_instance(&mut mgr, 2);
        mgr.transition(1, InstanceState::Playing).unwrap();
        mgr.transition(2, InstanceState::Playing).unwrap();
        mgr.pause(1).unwrap();
        // Instance 1 is Paused, Instance 2 is still Playing
        assert_eq!(mgr.get(1).unwrap().state, InstanceState::Paused);
        assert_eq!(mgr.get(2).unwrap().state, InstanceState::Playing);
    }
}
