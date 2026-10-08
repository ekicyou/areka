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
//! 印の付いた依頼（`OnShellChanging`）は、印のイベントを送ってその応答の台詞を追う
//! （[`Marked`]）。印の台詞の終わり方は完了通知（[`note_marked_done`]）と番号の食い違いで知り、
//! 印の台詞の利用者の中断は終了の予約があっても終了系列へ結ばない（[`is_marked_break`]・要件 5.2）。

use super::change::is_change_phase;
use super::{Action, Input, Phase, State, current_talk_id, events};
use crate::change::{GapLeft, GapRaise, MarkedEnd, RaiseOutcome, TalkGap};
use crate::talk::{TalkDone, TalkEndReason, TalkId};

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
/// 4. 印あり → 印のイベントを送り（許可表の照合と組み立ては汎用の入口と同じ関数）、応答を待つ
/// 5. 印なし → 見張る（結果は同じ `step` の末尾の [`observe`] が決める）
pub(super) fn begin(mut state: State, raise: Option<GapRaise>) -> (State, Vec<Action>) {
    let mut watch = GapWatch {
        marked: Marked::None,
        outcome: None,
    };
    let mut actions = Vec::new();
    if !matches!(state.phase, Phase::Steady { .. }) || state.pending_close.is_some() {
        settle(
            &mut watch,
            TalkGap::Left {
                reason: GapLeft::NotSteady,
            },
        );
    } else if let Some(raise) = raise {
        match events::allowed_static(&raise.id) {
            None => {
                tracing::warn!(target: "kanade", event = "raise_event_not_allowed", id = %raise.id, "許可表に無い印のイベント——送らずに捨てる");
                let not_sent = TalkGap::NotSent {
                    outcome: RaiseOutcome::NotAllowed,
                };
                settle(&mut watch, not_sent);
            }
            Some(id) => {
                tracing::debug!(target: "kanade", event = "talk_gap_watch", marked = id, "印のイベントを送って台詞の切れ目の見張りを始めた");
                let call = events::raise(id, raise.references, raise.method, &state.snapshot());
                actions.push(Action::ShioriRequest(call));
                watch.marked = Marked::AwaitingReply(id);
            }
        }
    } else {
        tracing::debug!(target: "kanade", event = "talk_gap_watch", "台詞の切れ目の見張りを始めた");
    }
    state.talk_gap = Some(watch);
    (state, actions)
}

/// 印のイベントの応答の直前のトーク（[`marked_reply`] が遷移の前に控え、[`observe`] が遷移の後に
/// 今のトークと突き合わせる）。
pub(super) struct MarkedReply {
    before: Option<TalkId>,
}

/// 入力が印のイベントの応答なら、遷移の前のトークを控える（[`super::step`] が `route` の前に呼ぶ）。
pub(super) fn marked_reply(state: &State, input: &Input) -> Option<MarkedReply> {
    let Some(GapWatch {
        marked: Marked::AwaitingReply(id),
        outcome: None,
    }) = &state.talk_gap
    else {
        return None;
    };
    match input {
        Input::ShioriReply { origin, .. } if origin == id => Some(MarkedReply {
            before: current_talk_id(&state.phase),
        }),
        _ => None,
    }
}

/// 印の台詞を利用者が中断したか（[`super::on_talk_done`] が中断の帳簿を空にする前に呼ぶ）。
///
/// 真なら、終了の予約があっても終了系列へ結ばない（要件 5.2）。結果が決まった後の見張りには
/// 効かせない（殻は決まった時点で見張りを取り出すので、例外を効かせる相手がもう居ない）。
pub(super) fn is_marked_break(state: &State, done: &TalkDone) -> bool {
    matches!(
        &state.talk_gap,
        Some(GapWatch { marked: Marked::Talk(t), outcome: None })
            if *t == done.talk_id
                && done.reason == TalkEndReason::Interrupted
                && state.user_break_talk.is_some_and(|note| note.talk_id == *t)
    )
}

/// 現行トークの完了を印の追跡へ写す（[`super::on_talk_done`] が現行トークと突き合わせた後に呼ぶ）。
///
/// 印の台詞なら、利用者の中断は [`Marked::BrokenByUser`]、それ以外（最後まで・選択肢の時間切れの
/// 解除・`\-` への到達）は「最後まで」にする。`\-` への到達は相が終了系列へ入るので、見極めは
/// 「終了で達しない」を先に選ぶ。印の台詞でない完了（置き換えた台詞の完了）は何もしない。
pub(super) fn note_marked_done(state: &mut State, done: &TalkDone, broken_by_user: bool) {
    if let Some(GapWatch {
        marked,
        outcome: None,
    }) = state.talk_gap.as_mut()
        && matches!(marked, Marked::Talk(t) if *t == done.talk_id)
    {
        *marked = if broken_by_user {
            Marked::BrokenByUser
        } else {
            Marked::Ended(MarkedEnd::Completed)
        };
    }
}

/// 毎 `step` の後の見極め（設計 Flow 2）。見張りが無いか、既に決まっていれば何もしない。
///
/// `reply` が在れば（入力が印のイベントの応答だった）、先に印の追跡を進める: 応答の直後に今の
/// トークが変わっていればそれを印の台詞として控え、変わっていなければ「台詞なし」にする。
pub(super) fn observe(state: &mut State, reply: Option<MarkedReply>) {
    let now = current_talk_id(&state.phase);
    let Some(watch) = state.talk_gap.as_mut() else {
        return;
    };
    if watch.outcome.is_some() {
        return;
    }
    if let Some(MarkedReply { before }) = reply {
        watch.marked = match now {
            Some(talk) if now != before => Marked::Talk(talk),
            _ => Marked::Ended(MarkedEnd::NoTalk),
        };
    }
    let Some(watch) = state.talk_gap.as_ref() else {
        return;
    };
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
        // 印の台詞の完了が届かないまま切れ目に来た＝印の番号と今の番号が食い違い、置き換えた台詞が
        // 終わった（置き換えられた印の台詞の完了は dispatcher か選択の 1 世代の控えが捨てる）。
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
