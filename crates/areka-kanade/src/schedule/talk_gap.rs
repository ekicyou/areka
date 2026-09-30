//! 台詞の切れ目の口（`src/schedule/talk_gap.rs`・areka-P0-shell-balloon-switch）。
//!
//! 依頼した側（UI）へ「台詞の切れ目に達した／達しないと決まった」を 1 回だけ返すための判断を置く。
//! UI は kanade が終了系列へ入ったことも、台詞が置き換わったことも知らないので、切れ目の判定は
//! 推し量らずに kanade へ任せる（設計「kanade の口の広さ」）。
//!
//! - 始め方（[`begin`]）: 横断の腕 `Input::AwaitTalkGap` から呼ばれ、見張り（[`GapWatch`]）を 1 つ立てる。
//!   届いたときに定常でないものは、ここで「定常でない」に決まる。
//! - 見極め（[`observe`]）: [`super::step`] の末尾で毎回 1 度だけ走り、設計 Flow 2 の順で結果を決める。
//!   1 度決まった結果は変えない。
//! - 取り出し（[`take_outcome`]）: 殻（`actor.rs`）が各依頼の処理の後に呼び、決まっていれば見張りを
//!   終えて結果を返信端へ 1 回だけ送る。
//!
//! 判断は純粋で、返信の送り手は殻が持つ。`RaiseEvent` と [`super::change::on_raise_event`] には
//! 触れない（印の無い依頼の振る舞いは 1 行も変えない＝要件 8.3）。
//!
//! 印の付いた依頼（`OnShellChanging`）の送出・印の台詞の追跡・中断の例外は後続のタスク 2.2 が
//! 埋める。本タスクでは印の許可表の照合までを行い、許可表に在る印は `warn!` を残して「送らなかった」
//! で返す（黙って印を落とさない）。[`Marked`] の印の追跡の値と、見極めの印の腕（応答待ち・利用者の
//! 中断・終わり方）は Flow 2 の順どおりに先に置いてある。

use super::change::is_change_phase;
use super::{Action, Phase, State, events};
use crate::change::{GapLeft, GapRaise, MarkedEnd, RaiseOutcome, TalkGap};
use crate::talk::TalkId;

/// 台詞の切れ目の見張り（[`State::talk_gap`]・高々 1 つ）。
pub(crate) struct GapWatch {
    /// 印の台詞の追跡（印の無い依頼は [`Marked::None`] のまま）。
    pub marked: Marked,
    /// 決まった結果（1 度決まったら変えない・殻が [`take_outcome`] で取り出す）。
    pub outcome: Option<TalkGap>,
}

/// 印の台詞の追跡。
pub(crate) enum Marked {
    /// 印の無い依頼。
    None,
    /// 印のイベント（名前）の応答待ち。
    AwaitingReply(&'static str),
    /// 印の応答が始めた台詞を追っている。
    Talk(TalkId),
    /// 印の台詞の終わり方が決まった。
    Ended(MarkedEnd),
    /// 印の台詞を利用者が中断した（終了系列へは進めていない）。
    BrokenByUser,
}

/// 横断の腕 `Input::AwaitTalkGap` の始め方（設計 State Management の `begin`）。
///
/// 次の順で判定する。受けた時点で定常でないものは、どれも「届いたときに定常にない」＝要件 1.14
/// として `Left{NotSteady}` にまとめる。見張りが既に在れば新しい依頼で置き換える（古い返信端を
/// 捨てる記録は、送り手を持つ殻が残す）。
/// 1. 相が定常でない → `Left{NotSteady}`
/// 2. 終了の保留あり → `Left{NotSteady}`（終了の保留に印のイベントを送らない＝要件 5.7）
/// 3. 印が許可表に無い → `NotSent{NotAllowed}`
/// 4. 印あり → タスク 2.2 で印のイベントの送出に置き換える（今は `warn!` の上で `NotSent`）
/// 5. 印なし → 見張る（結果は同じ `step` の末尾の [`observe`] が決める）
pub(super) fn begin(mut state: State, raise: Option<GapRaise>) -> (State, Vec<Action>) {
    let mut watch = GapWatch {
        marked: Marked::None,
        outcome: None,
    };
    if !matches!(state.phase, Phase::Steady { .. }) || state.pending_close.is_some() {
        settle(
            &mut watch,
            TalkGap::Left {
                reason: GapLeft::NotSteady,
            },
        );
    } else if let Some(raise) = raise {
        let not_sent = TalkGap::NotSent {
            outcome: RaiseOutcome::NotAllowed,
        };
        match events::allowed_static(&raise.id) {
            None => {
                tracing::warn!(target: "kanade", event = "raise_event_not_allowed", id = %raise.id, "許可表に無い印のイベント——送らずに捨てる");
            }
            Some(id) => {
                tracing::warn!(target: "kanade", event = "talk_gap_marked_unsupported", id, "印の付いた切れ目の依頼はまだ扱えない——印のイベントを送らずに捨てる");
            }
        }
        settle(&mut watch, not_sent);
    } else {
        tracing::debug!(target: "kanade", event = "talk_gap_watch", "台詞の切れ目の見張りを始めた");
    }
    state.talk_gap = Some(watch);
    (state, Vec::new())
}

/// 毎 `step` の後の見極め（設計 Flow 2）。見張りが無いか、既に決まっていれば何もしない。
pub(super) fn observe(state: &mut State) {
    let Some(watch) = state.talk_gap.as_ref() else {
        return;
    };
    if watch.outcome.is_some() {
        return;
    }
    let Some(outcome) = decide(state, &watch.marked) else {
        return;
    };
    if let Some(watch) = state.talk_gap.as_mut() {
        settle(watch, outcome);
    }
}

/// 殻が各依頼の処理の後に呼ぶ。結果が決まっていれば見張りを終えて結果を返す（ちょうど 1 回）。
pub(crate) fn take_outcome(state: &mut State) -> Option<TalkGap> {
    state.talk_gap.as_ref()?.outcome.as_ref()?;
    state.talk_gap.take()?.outcome
}

/// 設計 Flow 2 の順で結果を決める（`None` はまだ決まらない）。
///
/// 1. 相が定常でない（切替の相・ゴースト切替で降ろす途中なら `GhostChange`、それ以外は `Closing`）
/// 2. 終了の保留 → `Closing`
/// 3. ゴースト切替の保留 → `GhostChange`
/// 4. 印の応答待ち → 待つ
/// 5. 印の台詞の利用者の中断 → `CancelledByUser`
/// 6. 再生中 → 待つ
/// 7. 切れ目 → `Reached`
fn decide(state: &State, marked: &Marked) -> Option<TalkGap> {
    let left = |reason| Some(TalkGap::Left { reason });
    match &state.phase {
        Phase::Steady { .. } => {}
        // `raise_event` 無しのゴースト切替は切替の相を経ずに降ろす相へ入る。帳簿で見分ける。
        phase if is_change_phase(phase) || state.change.is_some() => {
            return left(GapLeft::GhostChange);
        }
        _ => return left(GapLeft::Closing),
    }
    if state.pending_close.is_some() {
        return left(GapLeft::Closing);
    }
    if state.pending_change.is_some() {
        return left(GapLeft::GhostChange);
    }
    match marked {
        Marked::AwaitingReply(_) => return None,
        Marked::BrokenByUser => return Some(TalkGap::CancelledByUser),
        Marked::None | Marked::Talk(_) | Marked::Ended(_) => {}
    }
    if matches!(state.phase, Phase::Steady { talk: Some(_) }) {
        return None;
    }
    let marked = match marked {
        Marked::None => None,
        Marked::Ended(end) => Some(*end),
        // 印の台詞が完了の突き合わせを経ずに消えた＝別のトークに置き換わっていた。
        Marked::Talk(_) => Some(MarkedEnd::Replaced),
        // 上の腕で返している。
        Marked::AwaitingReply(_) | Marked::BrokenByUser => return None,
    };
    Some(TalkGap::Reached { marked })
}

/// 結果を見張りへ書き込み、決まったことを記録する。
fn settle(watch: &mut GapWatch, outcome: TalkGap) {
    match &outcome {
        TalkGap::Reached { marked } => {
            tracing::info!(target: "kanade", event = "talk_gap_reached", marked = ?marked, "台詞の切れ目に達した");
        }
        TalkGap::CancelledByUser => {
            tracing::info!(target: "kanade", event = "talk_gap_marked_break", "印の台詞を利用者が中断した——終了系列へは進めない");
        }
        TalkGap::Left { reason } => {
            tracing::info!(target: "kanade", event = "talk_gap_left", reason = left_label(*reason), "台詞の切れ目に達しないと決まった");
        }
        TalkGap::NotSent { outcome } => {
            tracing::info!(target: "kanade", event = "talk_gap_not_sent", outcome = ?outcome, "印のイベントを送らなかった");
        }
    }
    watch.outcome = Some(outcome);
}

/// 記録に載せる理由の綴り。
fn left_label(reason: GapLeft) -> &'static str {
    match reason {
        GapLeft::NotSteady => "not_steady",
        GapLeft::Closing => "closing",
        GapLeft::GhostChange => "ghost_change",
    }
}

#[cfg(test)]
#[path = "talk_gap_tests.rs"]
mod tests;
