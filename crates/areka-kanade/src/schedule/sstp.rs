//! 運行表の SSTP の入口（外からの台本・外から頼まれたイベント）。
//!
//! 受け付けの判定（[`accepts`]）と外からの台本の腕（[`on_send`]）、外から頼まれたイベントの腕
//! （[`on_notify`]）とその応答の腕（[`on_notify_reply`]）を置く。

use super::translate::{self, ScriptSource};
use super::{Action, Phase, State, events};
use crate::msg::{KanadeConfig, ShioriOutcome};
use crate::talk::{ScriptOrigin, TalkId};

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
    let (talk_id, start) = super::start_talk(
        &mut state,
        SEND_ORIGIN,
        script.clone(),
        ScriptOrigin::MCP_SEND,
    );
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

/// 任意の名前の GET の応答待ちの控え（[`State::sstp_notify`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NotifyStage {
    pub strict: Option<bool>,
}

/// `Input::SstpNotify` の腕。受け付けないなら状態を変えず `debug!`（`sstp_not_accepted`）だけで空の一括。
///
/// 名前か Reference が [`crate::sstp::validate_notify`] を破っていれば（呼び手の不具合）送らずに状態を
/// 変えず、`warn!`（`sstp_notify_invalid`）を 1 件残して空の一括を返す（送出の檻で `Failed(Internal)` に
/// なりゴーストが倒れる道へ入れない）。受け付けたら応答待ちの控えを置いて任意の名前の GET を 1 つ返す。
pub(super) fn on_notify(
    mut state: State,
    event: String,
    references: Vec<String>,
    strict: Option<bool>,
) -> (State, Vec<Action>) {
    if !accepts(&state) {
        tracing::debug!(target: "kanade", event = "sstp_not_accepted", kind = "notify", phase = super::phase_label(&state.phase), "普段の状態でない——外から頼まれたイベントを受け付けない");
        return (state, Vec::new());
    }
    if let Err(error) = crate::sstp::validate_notify(&event, &references) {
        tracing::warn!(target: "kanade", event = "sstp_notify_invalid", error = ?error, name = ?event, reference_count = references.len(), "外から頼まれたイベントの名前か Reference が検査を破っている——送らない");
        return (state, Vec::new());
    }
    state.sstp_notify = Some(NotifyStage { strict });
    tracing::debug!(target: "kanade", event = "sstp_notify_sent", name = %event, reference_count = references.len(), "外から頼まれたイベントを送る");
    let call = events::sstp_notify(event, references, &state.snapshot());
    (state, vec![Action::ShioriRequest(call)])
}

/// 応答待ちの控えが在るときの `Input::ShioriReply` の腕（`route` が `on_shiori_reply` より先に呼ぶ）。
///
/// 控えを必ず取り出す。相が `Steady` で空でない（trim して 1 文字以上の）台本なら [`on_send`] と同じ
/// 掃除・採番で新しいトークを始め（`origin = NOTIFY_ORIGIN`）、strict を番号と結んで控える。翻訳は
/// `step` の出口の規則が元のイベント（`EventId::Sstp`）の出どころで預ける。空・空白だけ・204（エラー
/// 応答は殻が 204 に写す）は何もせず今のトークを残す。それ以外（失敗など）は今日の道へ渡す。
pub(super) fn on_notify_reply(
    mut state: State,
    outcome: ShioriOutcome,
    origin: &'static str,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    let Some(NotifyStage { strict }) = state.sstp_notify.take() else {
        return super::on_shiori_reply(state, outcome, origin, config);
    };
    if !matches!(state.phase, Phase::Steady { .. }) {
        return super::on_shiori_reply(state, outcome, origin, config);
    }
    match outcome {
        ShioriOutcome::Value(script, _) if !script.trim().is_empty() => {
            super::clear_choice_ledger(&mut state, "sstp_notify");
            state.choice_prev_talk = None;
            let (talk_id, start) =
                super::start_talk(&mut state, NOTIFY_ORIGIN, script, ScriptOrigin::MCP_NOTIFY);
            state.sstp_strict = Some(StrictNote { talk_id, strict });
            tracing::info!(target: "kanade", event = "sstp_notify_talk", talk_id = talk_id.0, strict = ?strict, "外から頼まれたイベントの台本を新しいトークとして始める");
            (state, vec![start])
        }
        ShioriOutcome::Value(_, _) | ShioriOutcome::NoContent => {
            tracing::debug!(target: "kanade", event = "sstp_notify_no_script", "外から頼まれたイベントは台本を返さなかった——今のトークを続ける");
            (state, Vec::new())
        }
        outcome => super::on_shiori_reply(state, outcome, origin, config),
    }
}

#[cfg(test)]
#[path = "sstp_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "sstp_notify_tests.rs"]
mod notify_tests;
