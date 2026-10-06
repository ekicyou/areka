//! balloon_events — バルーンの 3 つのイベント（`OnBalloonBreak`・`OnBalloonClose`・`OnBalloonTimeout`）
//! を送るかどうかの判断を 1 か所に集める（areka-P0-balloon-lifecycle-events）。
//!
//! ここが持つのは 2 つの控えと、[`super::step`] の出口の処理 [`settle`]、トークの完了の後の判断
//! [`after_talk_done`]（設計の表 B）、再生中のトークが無いときのダブルクリックの判断
//! [`on_idle_double_click`]（表 C）、時間切れの知らせの判断 [`on_timeout_notice`]（表 D）である。
//! - [`ShownTalk`]（`State.shown`）——最後に再生を始めたトークの番号と最終の台本。3 つのイベントの
//!   Reference0 の源で、古い時間切れの知らせを退ける照合の相手。
//! - [`BreakNote`]（`State.user_break_talk`）——利用者の中断を出した相手のトークと、ダブルクリック
//!   されたバルーンの scope。立てるのは中断の受理（[`super::user_break::on_user_break`]）、取り出すのは
//!   現行トークの完了の 1 回だけ（[`super::user_break::take_break`]）。
//!
//! 判断と控えは純粋で、副作用は `tracing` の記録だけである。SHIORI への要求は 1 つの入力につき
//! 高々 1 本（GET）を指示の列に足すだけで、応答の扱いは定常の応答の腕に任せる。

use super::{Action, Phase, State, current_talk_id, events, phase_label};
use crate::talk::{TalkDone, TalkEndReason, TalkId};

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
        log_not_sent(state, "OnBalloonBreak", "talk_gone", Some(note.talk_id));
    }
    if let Some(shown) = state.shown.as_mut()
        && shown.timeout_pending
        && Some(shown.talk_id) != current
    {
        shown.timeout_pending = false;
        let talk_id = shown.talk_id;
        log_not_sent(state, "OnBalloonTimeout", "talk_gone", Some(talk_id));
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

/// 送る場面に当たったが送らなかった、の 1 行（要件 7.2）。`talk_id` は控えが無いとき載せない。
fn log_not_sent(state: &State, id: &'static str, reason: &'static str, talk_id: Option<TalkId>) {
    tracing::info!(
        target: "kanade",
        event = "balloon_event_not_sent",
        id,
        reason,
        talk_id = talk_id.map(|t| t.0),
        phase = phase_label(&state.phase),
        "バルーンのイベントを送らなかった（要件 7.2）"
    );
}

/// 3 つのイベントのきっかけ（送る GET と記録の語）。
#[derive(Debug, Clone, Copy)]
enum Cause {
    /// 利用者の中断で止まった（`OnBalloonBreak`・scope 付き）。
    Break { scope: u32 },
    /// 読み終えたバルーンを閉じた（`OnBalloonClose`）。
    Close,
    /// 時間切れでバルーンが隠れた（`OnBalloonTimeout`）。
    Timeout,
}

impl Cause {
    fn id(self) -> &'static str {
        match self {
            Cause::Break { .. } => "OnBalloonBreak",
            Cause::Close => "OnBalloonClose",
            Cause::Timeout => "OnBalloonTimeout",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Cause::Break { .. } => "break",
            Cause::Close => "close",
            Cause::Timeout => "timeout",
        }
    }
}

/// 現行トークの完了の振り分けが返った直後に呼ぶ（設計の表 B。上から最初に当たった行だけを採る）。
///
/// - B1: 中断の控えも預かった時間切れの知らせも無い → 何もしない（記録しない）。
/// - B2: 完了の後が再生中のトークの無い定常でない・終了の要求を保留している → 送らず、きっかけごと
///   に理由を記録する（要件 1.6・4.4・設計 D3・D8）。
/// - B3: 中断の控えがあり `Interrupted` → `OnBalloonBreak`（要件 1.1〜1.5）。
/// - B4: 中断の控えがあり、トークは自分で終わっていた → `OnBalloonClose`（設計 D7）。
/// - B5: 預かった時間切れの知らせ → `OnBalloonTimeout`（要件 2.1）。
///
/// B3・B4 と B5 が同時なら中断の側を採り、預かりを捨てたことを記録する。預かりはどの行でもここで
/// 下ろす（出口の掃除が `talk_gone` と取り違えないため）。GET は渡された指示の列の末尾に高々 1 本
/// 足すので、切替の中止の知らせは GET より先に流れる。`broke` は [`super::user_break::take_break`] が
/// この完了で取り出した控え（相手がこのトークのときだけ `Some`）。
pub(super) fn after_talk_done(
    mut state: State,
    mut actions: Vec<Action>,
    done: &TalkDone,
    broke: Option<BreakNote>,
) -> (State, Vec<Action>) {
    let pending = state
        .shown
        .as_mut()
        .is_some_and(|shown| std::mem::take(&mut shown.timeout_pending));
    let causes: Vec<Cause> = broke
        .map(|note| match done.reason {
            TalkEndReason::Interrupted => Cause::Break { scope: note.scope },
            TalkEndReason::Ended | TalkEndReason::Quit => Cause::Close,
        })
        .into_iter()
        .chain(pending.then_some(Cause::Timeout))
        .collect();
    let Some((&first, rest)) = causes.split_first() else {
        return (state, actions);
    };
    if let Err(reason) = steady_idle(&state) {
        for cause in &causes {
            log_not_sent(&state, cause.id(), reason, Some(done.talk_id));
        }
        return (state, actions);
    }
    for cause in rest {
        log_not_sent(
            &state,
            cause.id(),
            "superseded_by_break",
            Some(done.talk_id),
        );
    }
    actions.extend(send(&mut state, first, Some(done.talk_id)));
    (state, actions)
}

/// 再生中のトークが無いときに届いた中断の合図（設計の表 C。上から最初に当たった行だけを採る）。
/// 読み終えたバルーンをダブルクリックで閉じた、の知らせで、相手は控えのトーク（直前に終えたトーク）。
///
/// - C1・C2: 再生中のトークの無い定常でない・終了の要求を保留している → 送らず理由を記録する
///   （要件 3.4・4.4）。
/// - C3: 控えのトークで `OnBalloonClose` を送り済み → 送らず理由を記録する（要件 4.5）。
/// - C4: それ以外 → `OnBalloonClose`（Reference0＝控えの台本。控えが無ければ空で送って警告）。
///
/// 中断の受理の規則（止める・二重に止めない）には触れない。scope は呼び手
/// [`super::user_break::on_user_break`] の記録に載っているので、ここでは受け取らない。
pub(super) fn on_idle_double_click(mut state: State) -> (State, Vec<Action>) {
    let talk_id = state.shown.as_ref().map(|shown| shown.talk_id);
    if let Err(reason) = steady_idle(&state) {
        log_not_sent(&state, Cause::Close.id(), reason, talk_id);
        return (state, Vec::new());
    }
    let actions = send(&mut state, Cause::Close, talk_id)
        .into_iter()
        .collect();
    (state, actions)
}

/// 時間切れでバルーンを隠した知らせ（設計の表 D。上から最初に当たった行だけを採る）。`talk_id` は
/// 表示の側が隠したバルーンに出ていたトークの番号で、控えのトーク（最後に再生を始めたトーク）と比べる。
///
/// - D1: 相が定常でない → 送らず理由を記録する（要件 2.6・4.4）。
/// - D2: 番号が控えと違う（控えが無い場合を含む）、または別のトークが再生中（台本を翻訳へ預けている
///   間は控えが前のトークのまま）→ 次のトークが既に始まっていた。送らず記録する（2.6）。
/// - D3: 終了の要求を保留している → 送らず理由を記録する。
/// - D4: 控えのトークで送り済み・預かり済み → 送らず理由を記録する（要件 4.5）。
/// - D5: そのトークがまだ再生中（完了の知らせが未着）→ 預かって記録する。完了で表 B の B5 が裁く。
/// - D6: 再生中のトークの無い定常 → `OnBalloonTimeout`（Reference0＝控えの台本・Reference1＝`0`）。
pub(super) fn on_timeout_notice(mut state: State, talk_id: TalkId) -> (State, Vec<Action>) {
    let shown = state
        .shown
        .as_ref()
        .filter(|shown| shown.talk_id == talk_id);
    let refused = if !matches!(state.phase, Phase::Steady { .. }) {
        Some("not_steady")
    } else if shown.is_none()
        || current_talk_id(&state.phase).is_some_and(|current| current != talk_id)
    {
        Some("next_talk_started")
    } else if state.pending_close.is_some() {
        Some("close_pending")
    } else if shown.is_some_and(|shown| shown.timeout_sent) {
        Some("already_sent")
    } else if shown.is_some_and(|shown| shown.timeout_pending) {
        Some("already_deferred")
    } else {
        None
    };
    if let Some(reason) = refused {
        log_not_sent(&state, Cause::Timeout.id(), reason, Some(talk_id));
        return (state, Vec::new());
    }
    // ここまでで、再生中のトークがあればそれはこのトークである（別のトークは D2 で断った。翻訳へ
    // 預けている間は控えが前のトークのままなので、控えの番号だけでは決められない）。
    if matches!(state.phase, Phase::Steady { talk: Some(_) }) {
        if let Some(shown) = state.shown.as_mut() {
            shown.timeout_pending = true;
        }
        tracing::info!(
            target: "kanade",
            event = "balloon_timeout_deferred",
            talk_id = talk_id.0,
            "時間切れの知らせを預かった——そのトークの完了で裁く（表 D の D5）"
        );
        return (state, Vec::new());
    }
    let actions = send(&mut state, Cause::Timeout, Some(talk_id))
        .into_iter()
        .collect();
    (state, actions)
}

/// 3 つの入口に共通する門（要件 4.4・設計 D8）: 相が `Steady{talk: None}` で、終了の要求を保留して
/// いないこと。送れなければ記録に載せる理由を返す。
fn steady_idle(state: &State) -> Result<(), &'static str> {
    if !matches!(state.phase, Phase::Steady { talk: None }) {
        Err("not_steady")
    } else if state.pending_close.is_some() {
        Err("close_pending")
    } else {
        Ok(())
    }
}

/// `talk_id` のトークについて `cause` のイベントを組み立てて返す。`OnBalloonClose`・
/// `OnBalloonTimeout` はそのトークで送り済みなら送らず理由を記録し、送るなら印を立てる（設計 D10）。
fn send(state: &mut State, cause: Cause, talk_id: Option<TalkId>) -> Option<Action> {
    let already = match (cause, state.shown.as_mut()) {
        (Cause::Close, Some(shown)) if Some(shown.talk_id) == talk_id => {
            std::mem::replace(&mut shown.close_sent, true)
        }
        (Cause::Timeout, Some(shown)) if Some(shown.talk_id) == talk_id => {
            std::mem::replace(&mut shown.timeout_sent, true)
        }
        _ => false,
    };
    if already {
        log_not_sent(state, cause.id(), "already_sent", talk_id);
        return None;
    }
    let script = shown_script(state, cause.id(), talk_id);
    let snapshot = state.snapshot();
    let (call, scope) = match cause {
        Cause::Break { scope } => (
            events::on_balloon_break(&script, scope, &snapshot),
            Some(scope),
        ),
        Cause::Close => (events::on_balloon_close(&script, &snapshot), None),
        Cause::Timeout => (events::on_balloon_timeout(&script, &snapshot), None),
    };
    tracing::info!(
        target: "kanade",
        event = "balloon_event_sent",
        id = cause.id(),
        cause = cause.label(),
        scope,
        talk_id = talk_id.map(|t| t.0),
        "バルーンのイベントを送る（要件 7.1）"
    );
    Some(Action::ShioriRequest(call))
}

/// `talk_id` のトークの台本（Reference0 の源）。控えが無い・番号が食い違う（構造上は起きない）
/// ときは空を返して記録する（出来事そのものは起きているので送る・設計 D9）。
fn shown_script(state: &State, id: &'static str, talk_id: Option<TalkId>) -> String {
    let talk_id = talk_id.map(|t| t.0);
    match &state.shown {
        Some(shown) if Some(shown.talk_id.0) == talk_id => shown.script.clone(),
        Some(shown) => {
            tracing::error!(target: "kanade", event = "balloon_event_script_missing", id, talk_id, shown_talk_id = shown.talk_id.0, "トークの控えの番号が食い違う——Reference0 を空で送る（設計 D9）");
            String::new()
        }
        None => {
            tracing::warn!(target: "kanade", event = "balloon_event_script_missing", id, talk_id, "トークの控えが無い——Reference0 を空で送る（設計 D9）");
            String::new()
        }
    }
}

#[cfg(test)]
#[path = "balloon_events_tests.rs"]
mod tests;

/// トークの完了の後の判断（表 B）の檻。
#[cfg(test)]
#[path = "balloon_events_done_tests.rs"]
mod done_tests;

/// 再生中のトークが無いときのダブルクリック（表 C）の檻。
#[cfg(test)]
#[path = "balloon_events_idle_tests.rs"]
mod idle_tests;

/// 時間切れの知らせ（表 D）の檻。
#[cfg(test)]
#[path = "balloon_events_timeout_tests.rs"]
mod timeout_tests;
