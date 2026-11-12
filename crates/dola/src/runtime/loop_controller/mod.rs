//! ループ再生の周回判定・進行・タイムテーブル再利用のためのオフセット調整。
//!
//! 純粋関数群として実装。全状態は `StoryboardInstance` に保持される。

use rand::Rng;
use rand::RngExt;

use crate::easing::EasingFunction;

use super::instance_manager::StoryboardInstance;
use super::interpolator::apply_easing;

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
    // 整数変換の安全性(D1a-V): loop_count は facade::compile_and_validate が
    // 「-1 または正値」のみ許可する（loop_count <= 0 かつ != -1 は InvalidLoopCount）。
    // -1 は上の早期 return で処理済みのため、ここでの `as u64` は常に正値の
    // 無損失変換であり、負値の wrap（例: -2 → 約 1.8e19 ≒ 実質無限ループ化）は
    // 発火しない。
    debug_assert!(
        instance.loop_count >= 1,
        "loop_count invariant violated: {}",
        instance.loop_count
    );
    instance.loops_completed < instance.loop_count as u64
}

/// ランダム遅延生成。
///
/// uniform `[0,1]` → easing → `[0,1]` へクランプ → `[min, max]` mapping。
/// `min == max` の場合は early return で固定遅延を返す。
pub(crate) fn generate_delay(
    min: f64,
    max: f64,
    easing: &EasingFunction,
    rng: &mut impl Rng,
) -> f64 {
    if min == max {
        return min;
    }
    let t: f64 = rng.random_range(0.0..1.0);
    // Back・Elastic・ベジェは [0,1] の外を出す。そのままだと待ちが min を割り、負なら tick が戻らない
    let eased = apply_easing(t, Some(easing)).clamp(0.0, 1.0);
    min + eased * (max - min)
}

/// 周回進行: 待ちを先に求め、次の周回を待ちの後に置く。
///
/// - `loops_completed += 1`
/// - `delay` = `loop_offset_min.is_some()` なら `generate_delay(...)`・無ければ 0
/// - `loop_start_time = 前の end_time + delay - pause_accumulated`
///   （評価で一時停止の累計を足し戻すと壁時計の起点になる）
/// - `end_time = 前の end_time + loop_duration + delay`
/// - 全 `trigger_states` の `fired = false` にリセット
pub(crate) fn advance_loop(instance: &mut StoryboardInstance, rng: &mut impl Rng) {
    instance.loops_completed += 1;

    let delay = match instance.loop_offset_min {
        Some(min) => generate_delay(
            min,
            instance.loop_offset_max,
            &instance.loop_offset_easing,
            rng,
        ),
        None => 0.0,
    };
    instance.loop_start_time = instance.end_time + delay - instance.pause_accumulated;
    instance.end_time += instance.loop_duration + delay;

    // トリガー状態リセット（周回ごとに再発火可能にする）
    for ts in &mut instance.trigger_states {
        ts.fired = false;
    }
}

/// 1つのインスタンスのループ処理を実行する。
///
/// `current_time >= end_time` の場合、while ループで全終了済み周回を処理し、
/// 各周回について `loops_completed` をインクリメントして継続可否を判定する。
/// 遅延が `end_time` に加算されるため、複数周回の一括スキップは自然に抑制される。
///
/// # Arguments
/// - `instance`: 対象インスタンスの可変参照
/// - `current_time`: 現在時刻
/// - `rng`: 乱数生成器
///
/// # Returns
/// - `LoopAction::Continue`: ループ継続またはループ対象外
/// - `LoopAction::Conclude`: ループ完了 — 呼び出し側で `conclude_internal()` を実行
pub(crate) fn process_loops(
    instance: &mut StoryboardInstance,
    current_time: f64,
    rng: &mut impl Rng,
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
    // 時刻境界の注意(D1a-V): 反復回数は (current_time - end_time) / 周回長 に比例する。
    // 無限ループ（loop_count == -1）は MIN_LOOP_DURATION（0.1s）が周回長の下限のため
    // 停止性は保証されるが、巨大な時刻ジャンプ（壁時計補正等）では反復が膨大になり得る。
    // 反復上限キャップや剰余スキップの導入は外部観測可能な挙動
    // （loops_completed / loop_start_time / 乱数消費）を変えるため入れない
    // （周回は最短 0.1s なので、8 時間の休止の後でも約 29 万回の軽い反復で済む）。
    while current_time >= instance.end_time {
        let before = instance.end_time;
        advance_loop(instance, rng);

        // どの待ちを引いても時刻が進まない周回は、残りの周回もこの瞬間に終わる:
        // 長さ 0・待ちなしの有限のループと、足しても丸めで消えるほど end_time が大きいとき
        // （呼び出し側が巨大な start_time を渡したとき）。乱数の待ちが 0 を引いただけなら続ける
        let max_delay = instance
            .loop_offset_min
            .map_or(0.0, |_| instance.loop_offset_max);
        if !should_continue_loop(instance)
            || before + (instance.loop_duration + max_delay) <= before
        {
            return LoopAction::Conclude;
        }
    }

    LoopAction::Continue
}

#[cfg(test)]
mod tests;
