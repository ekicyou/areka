//! user_break — 利用者の中断（バルーンの左ダブルクリック）の受理規則。
//!
//! [`super::step`] の横断の腕 `Input::UserBreak` から、場面を問わず呼ばれる。規則は 1 つ
//! ——「再生中なら止める／同じ相手へ二重に止めない／再生中でなければ何もしない」である。
//! 場面（起動の挨拶・定常のトーク・別れの台詞）でもスコープ番号でも結論を変えない（要件 2.5）。
//!
//! 止めるのは既存の単一の閉じ口（[`Action::CancelChoice`]）であり、第 2 の停止経路は作らない
//! （要件 3.2）。中断を理由とする SHIORI への要求は 1 件も積まない（要件 3.9・裁定 1）。

use super::{Action, State, clear_choice_ledger, current_talk_id, phase_label};

/// 利用者の中断の要求を受ける。再生中なら停止の指示を 1 件返し、相手を帳簿に控える。
///
/// `scope`（どのバルーンで起きたか）は記録へ載せるだけで、受理の判断には使わない（要件 2.5）。
/// `pending_close` の有無でも断らない——中断は SHIORI へ何も出さないため、マウス入力の
/// 「close 保留中は GET を出さない」規則の対象外である。
pub(super) fn on_user_break(mut state: State, scope: u32) -> (State, Vec<Action>) {
    let Some(talk_id) = current_talk_id(&state.phase) else {
        tracing::debug!(
            target: "kanade",
            event = "balloon_break_no_talk",
            scope,
            reason = "not_playing",
            phase = phase_label(&state.phase),
            "中断の要求を受けたが再生中のトークが無い——何も止めない（要件 2.2）"
        );
        return (state, Vec::new());
    };

    if state.user_break_talk == Some(talk_id) {
        tracing::debug!(
            target: "kanade",
            event = "balloon_break_no_talk",
            scope,
            reason = "already_breaking",
            "同じトークへの 2 件目の中断——二重に止めない（要件 2.4）"
        );
        return (state, Vec::new());
    }

    tracing::info!(
        target: "kanade",
        event = "balloon_break_accepted",
        scope,
        talk_id = talk_id.0,
        phase = phase_label(&state.phase),
        "利用者の中断を受け入れた——現行のトークを止める（要件 2.1・6.1）"
    );
    state.user_break_talk = Some(talk_id);
    // 選択の帳簿は**受理のこの時点で**消す（要件 2.6・裁定 4）。完了通知を待って消す形だと、
    // 受理から `TalkDone` が戻るまでの間に期限を過ぎた `Tick` が届いたとき、
    // `steady::fire_choice_timeout_if_due`（中断の有無を見ない）が `OnChoiceTimeout` を
    // SHIORI へ送ってしまう。利用者は選ばずに終わらせると決めたのだから、帳簿はここで役目を終える。
    clear_choice_ledger(&mut state, "user_break");
    (state, vec![Action::CancelChoice { talk_id }])
}

#[cfg(test)]
#[path = "user_break_tests.rs"]
mod tests;
