//! 運行表の SSTP の入口（外からの台本・外から頼まれたイベント）。
//!
//! 受け付けの判定（[`accepts`]）と外からの台本の腕（[`on_send`]）を置く。

use super::translate::{self, ScriptSource};
use super::{Action, Phase, State};
use crate::talk::TalkId;

/// SSTP SEND のトークの `ActiveTalk::origin`（記録用）。
pub(crate) const SEND_ORIGIN: &str = "SstpSend";
/// SSTP NOTIFY の応答の `origin`（殻が `EventId::Sstp` の往復に載せる固定のラベル）。
pub(crate) const NOTIFY_ORIGIN: &str = "SstpNotify";
/// 起動の挨拶のトークの `ActiveTalk::origin`（`schedule/boot.rs` の `to_baseware_version` が置く綴り）。
pub(super) const BOOT_GREETING_ORIGIN: &str = "boot";

/// 最後に始めた外からの台本のトークの strict（[`State::sstp_strict`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StrictNote {
    pub talk_id: TalkId,
    pub strict: Option<bool>,
}

/// 普段の状態か: 相が `Steady`・流れているトークが起動の挨拶でない・保留の終了も保留の切替も無い。
///
/// 殻（受け付けの控え）と運行表の腕の両方から呼ぶ純粋な関数で、2 か所の判断が食い違わない。
/// 中断の無効化モード（`no_user_break`）と選択肢の最中は見ない（受け付けた後は置き換える）。
pub(crate) fn accepts(state: &State) -> bool {
    let steady = match &state.phase {
        Phase::Steady { talk } => talk
            .as_ref()
            .is_none_or(|talk| talk.origin != BOOT_GREETING_ORIGIN),
        _ => false,
    };
    steady && state.pending_close.is_none() && state.pending_change.is_none()
}

/// `Input::SstpSend` の腕。受け付けないなら状態を変えず `debug!`（`sstp_not_accepted`）だけで空の一括。
///
/// 受け付けたら選択肢の帳簿と 1 世代の控えを消し、共通の手で採番・枠の差し替えをし、strict を
/// 番号と結んで控え、翻訳へ預ける（0 文字なら預けずに再生の開始をそのまま返す）。旧トークの
/// 後始末は今日の置き換えと同じく dispatcher と [`super::balloon_events::settle`] に任せる。
pub(super) fn on_send(
    mut state: State,
    script: String,
    strict: Option<bool>,
) -> (State, Vec<Action>) {
    if !accepts(&state) {
        tracing::debug!(target: "kanade", event = "sstp_not_accepted", kind = "send", phase = super::phase_label(&state.phase), "普段の状態でない——外からの台本を受け付けない");
        return (state, Vec::new());
    }
    super::clear_choice_ledger(&mut state, "sstp_send");
    state.choice_prev_talk = None;
    state.sstp_strict = None;
    let (talk_id, start) = super::start_talk(&mut state, SEND_ORIGIN, script.clone());
    state.sstp_strict = Some(StrictNote { talk_id, strict });
    tracing::info!(target: "kanade", event = "sstp_send_started", talk_id = talk_id.0, strict = ?strict, "外からの台本を新しいトークとして始める");
    if script.is_empty() {
        return (state, vec![start]);
    }
    let actions = translate::defer(
        &mut state,
        talk_id,
        ScriptSource::SstpSend,
        script,
        vec![start],
    );
    (state, actions)
}

#[cfg(test)]
#[path = "sstp_tests.rs"]
mod tests;
