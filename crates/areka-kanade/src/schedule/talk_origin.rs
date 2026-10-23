//! 運行表のトークの出どころ（SHIORI の応答で始まるトークへの付与と、下げたことの記録）。
//!
//! [`step`](super::step) の出口で、振り分けの後・翻訳へ預ける前に [`stamp`] を 1 回呼ぶ。
//! 腕（起動・切替・終了・常駐）は出どころを決めず、ここで一括に付ける。

use super::{Action, State};
use crate::talk::{ScriptOrigin, TalkId};
use areka_talk::SecurityLevel;

/// 台詞の応答の一括の中のすべての再生の開始に、`reply_origin`（元のイベントの基本の出どころを
/// 応答のレベルで下げた値）を付ける。`None`（台詞の応答でない・元のイベントが分からない）なら
/// 腕の値のまま（元のイベントが分からないときの記録は翻訳の出口の `translate_source_missing`）。
///
/// `External` に下がったトークごとに `script_security_lowered`（`by=response`）を 1 件残す。
pub(super) fn stamp(reply_origin: Option<ScriptOrigin>, mut actions: Vec<Action>) -> Vec<Action> {
    let Some(origin) = reply_origin else {
        return actions;
    };
    for action in &mut actions {
        if let Action::StartTalk(start) = action {
            if origin.level == SecurityLevel::External
                && start.origin.level != SecurityLevel::External
            {
                tracing::info!(target: "kanade", event = "script_security_lowered", talk_id = start.talk_id.0, by = "response", "応答の SecurityLevel で台本を外部扱いへ下げる");
            }
            start.origin = origin;
        }
    }
    actions
}

/// 選択肢の `script:` で始める台本の出どころ: 最後に再生を始めたトークの控えが元のトーク
/// （`prev_talk_id`）なら、その最終の出どころ（翻訳の後・外部扱いを含む）をそのまま写す（要件 1.6）。
///
/// 一致しない・控えが無い（構造上は起きない）ときは error `choice_script_origin_missing` を残し、
/// 外部扱いへ倒す（止める側）。
pub(super) fn of_shown(state: &State, prev_talk_id: TalkId) -> ScriptOrigin {
    match &state.shown {
        Some(shown) if shown.talk_id == prev_talk_id => shown.origin,
        other => {
            tracing::error!(target: "kanade", event = "choice_script_origin_missing", prev_talk_id = prev_talk_id.0, shown_talk_id = other.as_ref().map(|shown| shown.talk_id.0), "元のトークの出どころの控えが無い——選択肢の script: を外部扱いへ倒す");
            ScriptOrigin::GHOST.lowered_by(SecurityLevel::External)
        }
    }
}

#[cfg(test)]
#[path = "talk_origin_tests.rs"]
mod tests;
