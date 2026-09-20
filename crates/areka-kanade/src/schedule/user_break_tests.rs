// =============================================================================
// 利用者の中断（`Input::UserBreak`）の受理規則の決定論テスト
// =============================================================================
//
// 入口は `schedule::step` の横断の腕で、殻（`actor.rs`）が `KanadeMsg::UserBreak` をそのまま
// 写して渡す。ここでは `step` を直に呼び、返る状態と指示を見る。

use super::super::{Input, Phase, State, step};
use crate::msg::{KanadeConfig, MonotonicMs};

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// 再生中のトークが無い場面を `phase` だけ差し替えて組む。
fn not_playing(phase: Phase) -> State {
    State {
        phase,
        last_now: Some(MonotonicMs(500)),
        next_talk_id: 6,
        ..State::initial()
    }
}

/// 再生中のトークが無ければ、中断の要求は何も止めず、状態も指示も変えない（要件 2.2）。
/// 場面で振り分けない（要件 2.5）ので、定常でない場面でも同じ結果になる。
#[test]
fn user_break_without_playing_talk_changes_nothing() {
    let phases: [(&str, fn() -> Phase); 3] = [
        ("Steady{None}", || Phase::Steady { talk: None }),
        ("Idle", || Phase::Idle),
        ("Stopped", || Phase::Stopped),
    ];
    for (label, phase) in phases {
        let (next, actions) = step(
            not_playing(phase()),
            Input::UserBreak { scope: 1 },
            &config(),
        );
        assert!(actions.is_empty(), "{label}: 指示を 1 つも出さない");
        assert_eq!(
            std::mem::discriminant(&next.phase),
            std::mem::discriminant(&phase()),
            "{label}: 場面を変えない"
        );
        assert!(
            next.user_break_talk.is_none(),
            "{label}: 中断の相手を記録しない"
        );
        assert!(next.choice.is_none(), "{label}: 選択の帳簿を作らない");
        assert!(
            next.pending_close.is_none(),
            "{label}: 終了の保留を作らない"
        );
        assert_eq!(next.next_talk_id, 6, "{label}: 採番しない");
        assert_eq!(
            next.last_now,
            Some(MonotonicMs(500)),
            "{label}: 時刻を動かさない"
        );
    }
}
