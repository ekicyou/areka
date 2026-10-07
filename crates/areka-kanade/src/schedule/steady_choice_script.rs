//! `script:` の選択肢を受け付けた後の結末（台本を新しいトークとして始める／空なので何もしない）。
//!
//! 呼び手は [`super::on_choice`] の `script:` の腕だけである。ここでは解決（選択肢の待ちを
//! 閉じる指示）も選択肢の帳簿も扱わない——解決は `on_choice` が [`super::resolve_choice`] で
//! 出し、帳簿は `on_choice` が取り出し済みである。SHIORI への往復も翻訳の依頼も作らない。

use super::super::choice::script_body;
use super::super::{Action, ActiveTalk, Phase, State};
use crate::msg::ChoiceInput;
use crate::talk::{StartTalk, TalkId};

/// `script:` の選択肢を受け付けた後の結末を決める。
///
/// 台本が 1 文字以上なら新しいトークを始めて `Some(再生開始の行動)` を返し、空なら何も始めず
/// （状態を変えず）`None` を返す。どちらの結末でも記録を 1 件以上出す。
///
/// - 第 3 引数以降（`input.references`）は台本に含めず、どこへも渡さない。あれば数だけを警告
///   `choice_script_unused_args` に残す（括り忘れに気付けるように）。
/// - 新しいトークを始めたら、元のトークの番号を `choice_prev_talk` に 1 世代だけ控える
///   （元のトークの遅れた完了の知らせをエラーにしないため。選択肢のイベントが台本を返した
///   ときと同じ）。
pub(in crate::schedule) fn begin(
    state: &mut State,
    prev_talk_id: TalkId,
    input: &ChoiceInput,
) -> Option<Action> {
    if !input.references.is_empty() {
        tracing::warn!(
            target: "kanade",
            event = "choice_script_unused_args",
            choice_id = %input.id,
            talk_id = prev_talk_id.0,
            count = input.references.len(),
            "script: の選択肢の第 3 引数以降は使わない（台本に , を含めるなら \"…\" で括る）"
        );
    }
    // 判定は `on_choice` が済ませているので `None` は起きないが、起きても空と同じに扱う。
    let script = script_body(&input.id).unwrap_or("");
    if script.is_empty() {
        tracing::warn!(
            target: "kanade",
            event = "choice_script_empty",
            choice_id = %input.id,
            talk_id = prev_talk_id.0,
            "script: の後ろが空——トークを始めず選択肢の待ちだけを閉じる"
        );
        return None;
    }
    let (talk_id, action) = start_talk(state, "choice_script", script.to_string());
    state.choice_prev_talk = Some(prev_talk_id);
    tracing::info!(
        target: "kanade",
        event = "choice_script_started",
        choice_id = %input.id,
        talk_id = talk_id.0,
        prev_talk_id = prev_talk_id.0,
        "script: の後ろを新しいトークとして始める"
    );
    Some(action)
}

/// 台本 1 つを新しいトークとして始める（採番・枠の差し替え・再生開始の行動を返す）。
///
/// 選択肢の帳簿・`choice_prev_talk`・解決・記録は呼び手の仕事で、ここでは触らない。
pub(in crate::schedule) fn start_talk(
    state: &mut State,
    origin: &'static str,
    script: String,
) -> (TalkId, Action) {
    let talk_id = TalkId(state.next_talk_id);
    state.next_talk_id += 1;
    state.phase = Phase::Steady {
        talk: Some(ActiveTalk {
            talk_id,
            origin,
            script: script.clone(),
        }),
    };
    (talk_id, Action::StartTalk(StartTalk::new(talk_id, script)))
}

#[cfg(test)]
#[path = "steady_choice_script_tests.rs"]
mod tests;
