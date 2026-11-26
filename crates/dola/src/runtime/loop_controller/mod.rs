//! ループ再生の周回判定・進行・待ち（ループオフセット）の調整。全状態は `StoryboardInstance` に保持される。

use rand::Rng;
use rand::RngExt;

use crate::easing::EasingFunction;
use crate::time::{TimePoint, TimeSpan};

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
    // loop_count は validate が「-1 または 1 以上」に限る（load_document が InvalidNumber で拒む）。
    // -1 は上で返しているので `as u64` は正値の無損失変換で、負値の wrap（-2 → 約 1.8e19）は起きない。
    debug_assert!(
        instance.loop_count >= 1,
        "loop_count invariant violated: {}",
        instance.loop_count
    );
    instance.loops_completed < instance.loop_count as u64
}

/// ランダム遅延生成。
///
/// uniform `[0,1]` → easing → `[0,1]` へクランプ → `[min, max]` へ写す。
/// `min == max` の場合は固定遅延を返す。
pub(crate) fn generate_delay(
    min: TimeSpan,
    max: TimeSpan,
    easing: &EasingFunction,
    rng: &mut impl Rng,
) -> TimeSpan {
    if min == max {
        return min;
    }
    let t: f64 = rng.random_range(0.0..1.0);
    // Back・Elastic・ベジェは [0,1] の外を出す。そのままだと待ちが min を割る
    let eased = apply_easing(t, Some(easing)).clamp(0.0, 1.0);
    // max < min は validate が拒む（LoopOffsetRangeInverted）
    let width = max.checked_sub(min).unwrap_or(TimeSpan::ZERO);
    min + width.mul_unit(eased)
}

/// 周回進行: 待ちを先に求め、次の周回を待ちの後に置く。
///
/// - `loops_completed += 1`
/// - `delay` = `loop_offset_min.is_some()` なら `generate_delay(...)`・無ければ 0
/// - `loop_start = 前の周回の終わり + delay - pause_accumulated`
///   （評価で一時停止の累計を足し戻すと壁時計の起点になる）
/// - 全 `trigger_states` の `fired = false` にリセット
///
/// 次の周回の始まりか終わりが型の範囲を越えるなら、周回を延ばせないので false。
pub(crate) fn advance_loop(instance: &mut StoryboardInstance, rng: &mut impl Rng) -> bool {
    let lap_end = instance.lap_end();
    instance.loops_completed += 1;

    let delay = match instance.loop_offset_min {
        Some(min) => generate_delay(
            min,
            instance.loop_offset_max,
            &instance.loop_offset_easing,
            rng,
        ),
        None => TimeSpan::ZERO,
    };

    // 周回ごとに再発火できるようにする
    for ts in &mut instance.trigger_states {
        ts.fired = false;
    }

    let Some(next_start) = lap_end
        .and_then(|end| end.checked_add(delay))
        .and_then(|t| t.checked_add_delta(-instance.pause_accumulated))
    else {
        return false;
    };
    instance.loop_start = next_start;
    instance.lap_end().is_some()
}

/// 1つのインスタンスのループ処理を実行する。
///
/// `now` が周回の終わり以後の場合、while ループで全終了済み周回を処理し
/// （トリガーを持つインスタンスは 1 周だけ）、各周回について `loops_completed` を
/// インクリメントして継続可否を判定する。
/// 遅延が周回の終わりに加算されるため、複数周回の一括スキップは自然に抑制される。
///
/// # Returns
/// - `LoopAction::Continue`: ループ継続またはループ対象外
/// - `LoopAction::Conclude`: ループ完了 — 呼び出し側で `conclude_internal()` を実行
pub(crate) fn process_loops(
    instance: &mut StoryboardInstance,
    now: TimePoint,
    rng: &mut impl Rng,
) -> LoopAction {
    let lap_ended = |inst: &StoryboardInstance| inst.lap_end().is_some_and(|end| now >= end);
    if !lap_ended(instance) {
        return LoopAction::Continue;
    }

    if instance.loop_count == 1 {
        return LoopAction::Conclude;
    }

    // 反復回数は (now - 周回の終わり) / 周回長 に比例する。無限ループ（loop_count == -1）は
    // MIN_LOOP_DURATION（0.1s）が周回長の下限なので停止はするが、巨大な時刻ジャンプ（壁時計補正等）では
    // 反復が膨大になり得る。上限キャップや剰余スキップは外部から観測できる挙動
    // （loops_completed / loop_start / 乱数消費）を変えるため入れない。
    // トリガーなしの無限のループは出来事に入らず、1 周は数を足すだけ（0.1s の周回で 8 時間飛んでも約 29 万回の軽い反復）。
    // shortcut: トリガーなしの有限のループは周回の終わりごとに tick の出来事になり、1 周ごとに全部のインスタンスを見る
    // （0.01s × 100 万周を一度に飛ばすと約 75ms・ほかに無限のループが 10 個あると約 0.16 秒。直しの前は約 1.2ms）。
    // 大きな loop_count の文書が本番に入ったら、next_event_time で最後の周回の終わりの下限だけを出来事にする。
    // shortcut: トリガーを持つループは 1 周ずつ戻り、飛ばした周回ごとに子を 1 つ起こす
    // （コンパイル・競合の解決・Conclude と TriggerResult 1 件）。0.1s の周回で 8 時間飛ぶと
    // 1 回の tick に約 0.67 秒・28.8 万件。トリガーを使う文書が本番に入ったら飛ばす周回の子の扱いを決め直す。
    while lap_ended(instance) {
        let extended = advance_loop(instance, rng);

        // どの待ちを引いても時刻が進まない周回（長さ 0・待ちなしの有限のループ）は、残りの周回も
        // この瞬間に終わる。乱数の待ちが 0 を引いただけなら続ける
        let max_delay = instance
            .loop_offset_min
            .map_or(TimeSpan::ZERO, |_| instance.loop_offset_max);
        if !should_continue_loop(instance)
            || !extended
            || instance.lap_length + max_delay == TimeSpan::ZERO
        {
            return LoopAction::Conclude;
        }
        // トリガーを持つなら 1 周で戻る。呼び出し側がこの周回のトリガーを発火させてから次を進める
        if !instance.trigger_states.is_empty() {
            break;
        }
    }

    LoopAction::Continue
}

#[cfg(test)]
mod tests;
