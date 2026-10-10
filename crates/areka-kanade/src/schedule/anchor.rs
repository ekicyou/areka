//! anchor — アンカー（`\_a`）の選択の受理と 2 段の送出。
//!
//! 選択肢（[`super::choice`]・[`super::steady`] の選択の腕）と違い、帳簿・期限・照合の鍵を持たない。
//! 届いた知らせはそのまま送り（areka-P0-anchor-tag-canon 要件 4.12）、覚えるのは「どの段の応答を
//! 待っているか」（[`AnchorStage`]）だけである。
//!
//! - [`plan_anchor`]: ID からイベントの列を決める（`On` 始まりか、それ以外か）。
//! - [`on_anchor`]: 知らせの受理。GET を 1 本積み、段を [`State::anchor`](super::State) に置く。
//! - [`on_anchor_reply`]: 応答。台本は呼び手（[`super::steady`] の応答の腕）へ返し、それ以外は
//!   次の段へ進めるか、何もせずに終える。
//!
//! 殻は GET を送るとすぐ応答を入れ直すので、段の記憶は 1 回の知らせの処理の中だけで生きる
//! （応答の腕が必ず取り出す）。アンカーは選択待ちの印（`choosing`）を立てない。

use super::{Action, Phase, State, events};
use crate::anchor_input::AnchorInput;
use crate::msg::ShioriOutcome;

/// アンカーの ID から決まるイベントの列（選択肢の `script:` の約束はアンカーには無い）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnchorPlan {
    /// `On` で始まる ID → ID と同じ名前のイベントを 1 本だけ送る（要件 4.4）。
    Named,
    /// それ以外 → `OnAnchorSelectEx` を送り、204 なら `OnAnchorSelect` を続ける（要件 4.1・4.2）。
    Canonical,
}

/// アンカーの ID からイベントの列を決める。判定は逐語の前方一致で、大文字小文字を区別する
/// （選択肢の `On` 始まりと同じ規則。`"On"` だけでも [`AnchorPlan::Named`]・空は [`AnchorPlan::Canonical`]）。
pub(crate) fn plan_anchor(id: &str) -> AnchorPlan {
    if id.starts_with("On") {
        AnchorPlan::Named
    } else {
        AnchorPlan::Canonical
    }
}

/// アンカーのイベントの応答待ちの段。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AnchorStage {
    /// `OnAnchorSelectEx` の応答待ち（204 なら `OnAnchorSelect` へ進む）。
    SelectEx { id: String },
    /// 最終段（`OnAnchorSelect` か、`On` 始まりの名前のイベント）の応答待ち。
    /// `id` は記録にだけ使う（空の ID も正しい ID なので、どの段でも ID を覚えておく）。
    Final { id: String },
}

/// アンカーの知らせの受理。定常でだけ受理し、それ以外は警告を残して棄却する（状態は変えない）。
///
/// 受理したら [`plan_anchor`] の決めた最初のイベントの GET を 1 本積み、応答待ちの段を置く。
/// 再生中の台詞にも選択待ちの帳簿にも触れない（話している最中でも送る・要件 4.8／4.9 は応答の側）。
/// Status は送る時点の状態から導く（選択待ちの有無は帳簿から。アンカー自身は `choosing` を立てない）。
pub(super) fn on_anchor(mut state: State, input: AnchorInput) -> (State, Vec<Action>) {
    if !matches!(state.phase, Phase::Steady { .. }) {
        tracing::warn!(
            target: "kanade",
            event = "anchor_rejected_phase",
            id = %input.id,
            scope = input.scope,
            phase = super::phase_label(&state.phase),
            "定常以外で届いたアンカーの選択——何も送らずに棄却"
        );
        return (state, Vec::new());
    }
    let plan = plan_anchor(&input.id);
    tracing::info!(
        target: "kanade",
        event = "anchor_accepted",
        id = %input.id,
        text = %input.text,
        scope = input.scope,
        reference_count = input.references.len(),
        plan = ?plan,
        "アンカーの選択を受理——最初のイベントを送る"
    );
    let snapshot = state.snapshot();
    let (call, stage) = match plan {
        // ID はイベントの名前が運ぶので、Reference には引数だけを載せる（選択肢の `On` 始まりと同じ組み立て）。
        AnchorPlan::Named => (
            events::on_choice_named(input.id.clone(), &input.references, &snapshot),
            AnchorStage::Final { id: input.id },
        ),
        AnchorPlan::Canonical => (
            events::on_anchor_select_ex(&input.text, &input.id, &input.references, &snapshot),
            AnchorStage::SelectEx { id: input.id },
        ),
    };
    state.anchor = Some(stage);
    (state, vec![Action::ShioriRequest(call)])
}

/// アンカーのイベントの応答。`stage` は呼び手が [`State::anchor`](super::State) から取り出した段。
///
/// - 台本（`Value`）→ `None`。呼び手が今までの応答の腕へ流す（話していなければ起動・話していれば
///   置き換え）。続きのイベントは送らない（要件 4.3・4.8）。
/// - 204・送信の失敗（`error!`）・GET では起きない応答（`warn!`）→ `Some`。`OnAnchorSelectEx` の段なら
///   `OnAnchorSelect` の GET を積んで最終段を置き、最終段なら何もしない（要件 4.2・4.9・4.10）。
pub(super) fn on_anchor_reply(
    state: &mut State,
    stage: AnchorStage,
    outcome: &ShioriOutcome,
    origin: &'static str,
) -> Option<Vec<Action>> {
    // 記録に載せる段の名前と ID。
    let (stage_label, id) = match &stage {
        AnchorStage::SelectEx { id } => ("select_ex", id.as_str()),
        AnchorStage::Final { id } => ("final", id.as_str()),
    };
    match outcome {
        ShioriOutcome::Value(_) => return None,
        ShioriOutcome::NoContent => {}
        ShioriOutcome::Failed(failure) => tracing::error!(
            target: "kanade",
            event = "anchor_shiori_failed_as_204",
            id,
            stage = stage_label,
            origin,
            error = %failure,
            "アンカーのイベントの送信に失敗——何も返らなかった（204）ときと同じ扱いで続ける"
        ),
        ShioriOutcome::Notified | ShioriOutcome::Unloaded => tracing::warn!(
            target: "kanade",
            event = "anchor_unexpected_reply",
            id,
            stage = stage_label,
            origin,
            "アンカーのイベントに GET では起きない応答——204 と同じ扱いで続ける"
        ),
    }
    Some(match stage {
        AnchorStage::SelectEx { id } => {
            let call = events::on_anchor_select(&id, &state.snapshot());
            state.anchor = Some(AnchorStage::Final { id });
            vec![Action::ShioriRequest(call)]
        }
        AnchorStage::Final { .. } => Vec::new(),
    })
}

#[cfg(test)]
#[path = "anchor_tests.rs"]
mod tests;
