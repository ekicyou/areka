//! script 種別のログ——再生を始めたトーク 1 つにつき 1 件（表示の語の決め方の正本）。
//!
//! [`step`](super::step) は `route` の前に [`source_of`] で、この入力で始まるトークの出どころを控え、
//! 出口で [`record`] を呼ぶ。返す一括の [`Action::StartTalk`] のうち台本が 0 文字でないものごとに
//! [`emit`] を 1 回だけ通す（翻訳に預けた一括は出口に出ないので重ならない）。0 文字の台本を残さない
//! のは、毎秒の `OnSecondChange` に空の応答を返す SHIORI で、空の記録が種別ごとの上限を埋めて本物の
//! 台本を押し出すのを防ぐため。
//!
//! 表示の語: 外からの台本は [`SSTP_SEND_LABEL`]、SHIORI のイベントへの応答は [`event_label`]
//! （`Ghost:<名前>`）、SHIORI の往復から生まれていないトークは [`PLAIN_LABEL`]。

use super::translate::{Replied, ScriptSource};
use super::{Action, Input, State};
use crate::msg::KanadeConfig;

/// 取り決めの target（`crates/areka/src/log_history.rs` の `TARGET_SCRIPT` と同じ字・areka のテストで一致を固定）。
pub const TARGET: &str = "areka::log::script";
/// 外からの台本（MCP の `sakurascript`）の表示の語。
pub const SSTP_SEND_LABEL: &str = "SSTP(Local,Auth)";
/// SHIORI の往復から生まれていないトーク（選択肢の `script:` など）の表示の語。
pub const PLAIN_LABEL: &str = "Ghost";

/// SHIORI のイベントへの応答の表示の語 `Ghost:<名前>`。
pub fn event_label(name: &str) -> String {
    format!("{PLAIN_LABEL}:{name}")
}

/// 1 件を出す（記録の形の正本・運行表の出口と MCP の側のテストが同じここを通る）。
pub fn emit(ghost: &str, label: &str, script: &str) {
    tracing::info!(target: TARGET, ghost = %ghost, label = %label, "{script}");
}

/// `route` の前に呼ぶ（入力が消費される前に出どころを控える）。
///
/// SHIORI の応答 → 台詞の応答で元のイベントが分かるときのその出どころ、翻訳の結果 → 翻訳の帳簿の
/// 出どころ、外からの台本 → [`ScriptSource::SstpSend`]、それ以外 → `None`（[`PLAIN_LABEL`]）。
pub(super) fn source_of(state: &State, input: &Input, replied: &Replied) -> Option<ScriptSource> {
    match input {
        Input::ShioriReply { .. } => replied.talk_source(),
        Input::TranslateDone(_) => state.translate.as_ref().map(|wait| wait.source.clone()),
        Input::SstpSend { .. } => Some(ScriptSource::SstpSend),
        _ => None,
    }
}

/// 出口で呼ぶ。返す一括の [`Action::StartTalk`] のうち台本が 0 文字でないものごとに 1 件出す。
pub(super) fn record(config: &KanadeConfig, actions: &[Action], source: Option<&ScriptSource>) {
    let label = match source {
        Some(ScriptSource::Event(event)) => event_label(event.id.as_str()),
        Some(ScriptSource::SstpSend) => SSTP_SEND_LABEL.to_string(),
        None => PLAIN_LABEL.to_string(),
    };
    for action in actions {
        if let Action::StartTalk(start) = action
            && !start.script.is_empty()
        {
            emit(&config.ghost_name, &label, &start.script);
        }
    }
}

#[cfg(test)]
#[path = "script_log_tests.rs"]
mod tests;
