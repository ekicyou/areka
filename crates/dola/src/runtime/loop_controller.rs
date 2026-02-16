//! ループ再生の周回判定・進行・タイムテーブル再利用のためのオフセット調整。
//!
//! 純粋関数群として実装。全状態は `StoryboardInstance` に保持される。

use super::instance_manager::StoryboardInstance;

/// ループ処理の結果を示す判別 enum。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopAction {
    /// ループ継続（または周回内で変化なし）
    Continue,
    /// ループ完了 — Conclude すべき
    Conclude,
}

/// ループ継続の可否を判定する純粋関数。
///
/// `loop_count == -1` の場合は常に `true`（無限ループ）。
/// それ以外は `loops_completed < loop_count as u64` で判定。
pub(crate) fn should_continue_loop(instance: &StoryboardInstance) -> bool {
    if instance.loop_count == -1 {
        return true;
    }
    instance.loops_completed < instance.loop_count as u64
}

/// 周回進行: 1周回分のオフセット調整を実行する。
///
/// - `loops_completed += 1`
/// - `loop_start_time += loop_duration`
/// - `end_time += loop_duration`
pub(crate) fn advance_loop(instance: &mut StoryboardInstance) {
    instance.loops_completed += 1;
    instance.loop_start_time += instance.loop_duration;
    instance.end_time += instance.loop_duration;
}

/// 1つのインスタンスのループ処理を実行する。
///
/// `current_time >= end_time` の場合、while ループで全終了済み周回を処理し、
/// 各周回について `loops_completed` をインクリメントして継続可否を判定する。
/// 複数周回が一度に終了する場合も正確に処理する。
///
/// # Arguments
/// - `instance`: 対象インスタンスの可変参照
/// - `current_time`: 現在時刻
///
/// # Returns
/// - `LoopAction::Continue`: ループ継続またはループ対象外
/// - `LoopAction::Conclude`: ループ完了 — 呼び出し側で `conclude_internal()` を実行
pub(crate) fn process_loops(
    instance: &mut StoryboardInstance,
    current_time: f64,
) -> LoopAction {
    // 周回内であれば何もしない
    if current_time < instance.end_time {
        return LoopAction::Continue;
    }

    // loop_count=1 の場合はループ不要 → 即座に Conclude
    if instance.loop_count == 1 {
        return LoopAction::Conclude;
    }

    // while ループで全終了済み周回を処理
    while current_time >= instance.end_time {
        advance_loop(instance);

        if !should_continue_loop(instance) {
            return LoopAction::Conclude;
        }
    }

    LoopAction::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storyboard::InterruptionPolicy;
    use crate::runtime::instance_state::InstanceState;

    /// テスト用インスタンスヘルパー
    fn make_instance(loop_count: i32, loop_duration: f64) -> StoryboardInstance {
        StoryboardInstance {
            group_id: 1,
            storyboard_name: "test".to_string(),
            state: InstanceState::Playing,
            interruption_policy: InterruptionPolicy::Conclude,
            start_time: 0.0,
            time_scale: 1.0,
            base_duration: loop_duration,
            pause_accumulated: 0.0,
            pause_start: None,
            loop_count,
            loops_completed: 0,
            finish_deadline: None,
            end_time: loop_duration,
            loop_start_time: 0.0,
            loop_duration,
        }
    }

    // =======================================================================
    // should_continue_loop
    // =======================================================================

    #[test]
    fn should_continue_loop_basic() {
        let mut inst = make_instance(3, 1.0);
        // loops_completed=0, loop_count=3 → true
        assert!(should_continue_loop(&inst));

        inst.loops_completed = 2;
        assert!(should_continue_loop(&inst));

        inst.loops_completed = 3;
        assert!(!should_continue_loop(&inst));
    }

    #[test]
    fn should_continue_loop_infinite() {
        let mut inst = make_instance(-1, 1.0);
        assert!(should_continue_loop(&inst));

        inst.loops_completed = u64::MAX - 1;
        assert!(should_continue_loop(&inst));
    }

    #[test]
    fn should_continue_loop_single() {
        let mut inst = make_instance(1, 1.0);
        // loops_completed=0 → true (まだ1周目未完了)
        assert!(should_continue_loop(&inst));

        inst.loops_completed = 1;
        // loops_completed=1 >= loop_count=1 → false
        assert!(!should_continue_loop(&inst));
    }

    // =======================================================================
    // advance_loop
    // =======================================================================

    #[test]
    fn advance_loop_updates_fields() {
        let mut inst = make_instance(3, 2.0);
        // 初期: loops_completed=0, loop_start_time=0.0, end_time=2.0
        assert_eq!(inst.loops_completed, 0);
        assert_eq!(inst.loop_start_time, 0.0);
        assert_eq!(inst.end_time, 2.0);

        advance_loop(&mut inst);

        assert_eq!(inst.loops_completed, 1);
        assert_eq!(inst.loop_start_time, 2.0);
        assert_eq!(inst.end_time, 4.0);

        advance_loop(&mut inst);

        assert_eq!(inst.loops_completed, 2);
        assert_eq!(inst.loop_start_time, 4.0);
        assert_eq!(inst.end_time, 6.0);
    }

    // =======================================================================
    // process_loops
    // =======================================================================

    #[test]
    fn process_loops_within_loop() {
        // current_time < end_time → Continue, フィールド変化なし
        let mut inst = make_instance(3, 2.0);
        let action = process_loops(&mut inst, 1.0);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 0);
        assert_eq!(inst.loop_start_time, 0.0);
        assert_eq!(inst.end_time, 2.0);
    }

    #[test]
    fn process_loops_one_loop_completed_continue() {
        // loop_count=3, 1周終了 → Continue, loops_completed=1
        let mut inst = make_instance(3, 2.0);
        let action = process_loops(&mut inst, 2.5);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 1);
        assert_eq!(inst.loop_start_time, 2.0);
        assert_eq!(inst.end_time, 4.0);
    }

    #[test]
    fn process_loops_all_loops_completed() {
        // loop_count=3, 3周終了 → Conclude
        let mut inst = make_instance(3, 2.0);
        let action = process_loops(&mut inst, 6.5);
        assert_eq!(action, LoopAction::Conclude);
        assert_eq!(inst.loops_completed, 3);
    }

    #[test]
    fn process_loops_multi_loops_at_once() {
        // loop_count=5, 3周分一度に終了 → Continue, loops_completed=3
        let mut inst = make_instance(5, 1.0);
        let action = process_loops(&mut inst, 3.5);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 3);
        assert_eq!(inst.loop_start_time, 3.0);
        assert_eq!(inst.end_time, 4.0);
    }

    #[test]
    fn process_loops_all_loops_completed_overshoot() {
        // loop_count=3, 5周分超過 → Conclude, loops_completed=3
        let mut inst = make_instance(3, 1.0);
        let action = process_loops(&mut inst, 5.5);
        assert_eq!(action, LoopAction::Conclude);
        assert_eq!(inst.loops_completed, 3);
    }

    #[test]
    fn process_loops_infinite_multi_loops() {
        // loop_count=-1, 5周分 → Continue, loops_completed=5
        let mut inst = make_instance(-1, 1.0);
        let action = process_loops(&mut inst, 5.5);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 5);
        assert_eq!(inst.loop_start_time, 5.0);
        assert_eq!(inst.end_time, 6.0);
    }

    #[test]
    fn process_loops_single_loop_conclude() {
        // loop_count=1, 終了 → Conclude（whileループに入らず即Conclude）
        let mut inst = make_instance(1, 2.0);
        let action = process_loops(&mut inst, 2.5);
        assert_eq!(action, LoopAction::Conclude);
        // loop_count=1 の場合は advance_loop を呼ばない
        assert_eq!(inst.loops_completed, 0);
    }

    #[test]
    fn process_loops_exact_boundary() {
        // current_time == end_time → 周回処理が発生する
        let mut inst = make_instance(3, 2.0);
        let action = process_loops(&mut inst, 2.0);
        assert_eq!(action, LoopAction::Continue);
        assert_eq!(inst.loops_completed, 1);
    }
}
