//! balloon_events — バルーンの 3 つのイベント（`OnBalloonBreak`・`OnBalloonClose`・`OnBalloonTimeout`）
//! を送るかどうかの判断を 1 か所に集める（areka-P0-balloon-lifecycle-events）。
//!
//! ここが持つのは 2 つの控えと、[`super::step`] の出口の処理 [`settle`] である。
//! - [`ShownTalk`]（`State.shown`）——最後に再生を始めたトークの番号と最終の台本。3 つのイベントの
//!   Reference0 の源で、古い時間切れの知らせを退ける照合の相手。
//! - [`BreakNote`]（`State.user_break_talk`）——利用者の中断を出した相手のトークと、ダブルクリック
//!   されたバルーンの scope。立てるのは中断の受理（[`super::user_break::on_user_break`]）、取り出すのは
//!   現行トークの完了の 1 回だけ（[`super::user_break::take_break`]）。
//!
//! 控えは純粋に状態として持ち、副作用は `tracing` の記録だけである。

use super::{Action, State, current_talk_id, phase_label};
use crate::talk::TalkId;

/// 再生を始めたトークの控え。3 つのイベントの Reference0 の源で、古い知らせを退ける照合の相手。
///
/// 再生の開始（[`Action::StartTalk`]）が [`super::step`] を出るたびに丸ごと置き換える（印は新しい
/// トークで下りる）。ゴーストごとに新しい [`State`] なので、ゴーストをまたがない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShownTalk {
    pub talk_id: TalkId,
    /// 再生を始めた最終の台本（`OnTranslate` で書き換えた後）。
    pub script: String,
    /// このトークで `OnBalloonClose` を送ったか。
    pub close_sent: bool,
    /// このトークで `OnBalloonTimeout` を送ったか。
    pub timeout_sent: bool,
    /// 完了の知らせより先に届いた時間切れの知らせを預かっているか。
    pub timeout_pending: bool,
}

/// 利用者の中断の控え（止めた相手のトークと、ダブルクリックされたバルーンの scope）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BreakNote {
    pub talk_id: TalkId,
    pub scope: u32,
}

/// [`super::step`] の出口で 1 回呼ぶ。
///
/// ⑴ 掃除: 中断の控え・預かった時間切れの知らせの相手が、もう現行トークでなければ捨てて理由
/// `talk_gone` を 1 行記録する（完了を通らずに相が変わった・別の応答に置き換えられた）。
/// 中断の控えは今も「現行トークの完了で取り出すと相手が違うので何も起きない」だけなので、
/// ここで早く捨てても中断の振る舞いは変わらない（要件 1.9）。
/// ⑵ 返す指示の列に再生の開始があれば、控えをそのトークで丸ごと置き換える。翻訳にかけた台本は
/// 翻訳の結果の入力の `step` で最終の台本になって出るので、ここで取るのは常に最終の台本である。
pub(super) fn settle(state: &mut State, actions: &[Action]) {
    let current = current_talk_id(&state.phase);
    if let Some(note) = state.user_break_talk
        && Some(note.talk_id) != current
    {
        state.user_break_talk = None;
        log_talk_gone(state, "OnBalloonBreak", note.talk_id);
    }
    if let Some(shown) = state.shown.as_mut()
        && shown.timeout_pending
        && Some(shown.talk_id) != current
    {
        shown.timeout_pending = false;
        let talk_id = shown.talk_id;
        log_talk_gone(state, "OnBalloonTimeout", talk_id);
    }
    if let Some(start) = actions.iter().rev().find_map(|action| match action {
        Action::StartTalk(start) => Some(start),
        _ => None,
    }) {
        state.shown = Some(ShownTalk {
            talk_id: start.talk_id,
            script: start.script.clone(),
            close_sent: false,
            timeout_sent: false,
            timeout_pending: false,
        });
    }
}

/// 控えの相手が現行トークでなくなったので送らずに捨てた、の 1 行（要件 7.2）。
fn log_talk_gone(state: &State, id: &'static str, talk_id: TalkId) {
    tracing::info!(
        target: "kanade",
        event = "balloon_event_not_sent",
        id,
        reason = "talk_gone",
        talk_id = talk_id.0,
        phase = phase_label(&state.phase),
        "控えの相手のトークが現行でなくなった——送らずに捨てる（要件 7.2）"
    );
}

#[cfg(test)]
#[path = "balloon_events_tests.rs"]
mod tests;
