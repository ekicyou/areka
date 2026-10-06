//! バルーンの 3 つのイベントの判断の檻（areka-P0-balloon-lifecycle-events）。
//!
//! どれも最上位の [`step`] に実際の入力を順に入れて駆動する（時刻は `Tick` で注入する）。
//! ここではトークの控え（`State.shown`）と中断の控え（`State.user_break_talk`）の出入りを固定する
//! ——翻訳の後の最終の台本が控えに入ること、切替の `OnClose` の別れの台詞も控えに入ること、
//! 別の応答による置き換えで中断の控えが捨てられ理由の記録が 1 行出ること（要件 1.2・1.3・1.8・
//! 2.2・3.2・7.2）。表 B の檻は `balloon_events_done_tests.rs` に置き、ここの補助を使う。

use super::super::log_capture::{capture, logged_once};
use super::super::steady::test_support::base_state;
use super::super::{Input, Phase, step};
use super::{BreakNote, ShownTalk};
use crate::change::{ChangeOrigin, ChangeRequest, ChangeTarget};
use crate::msg::{
    KanadeConfig, MonotonicMs, MouseButton, MouseEventKind, MouseInput, ShioriOutcome,
};
use crate::schedule::{Action, State};
use crate::talk::TalkId;
use tracing::Level;

/// SHIORI が返す元の台本。
pub(super) const ORIGINAL: &str = "\\0元の台本\\e";
/// 翻訳の結果（最終の台本）。
pub(super) const FINAL: &str = "\\0最終の台本\\e";

pub(super) fn cfg() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// `Steady{talk: None}`（終了の保留なし・控えなし・次の番号 5）。
pub(super) fn steady_none() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(1_000)),
        next_talk_id: 5,
        ..base_state()
    }
}

/// 入力を最上位の [`step`] に入れ、状態を進めて指示の列を返す。
pub(super) fn feed(s: &mut State, input: Input) -> Vec<Action> {
    let state = std::mem::replace(s, State::initial());
    let (state, actions) = step(state, input, &cfg());
    *s = state;
    actions
}

/// 台本の応答を入れる（出所は応答を待っている GET の ID）。
pub(super) fn reply_value(s: &mut State, script: &str, origin: &'static str) -> Vec<Action> {
    feed(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(script.to_string()),
            origin,
        },
    )
}

pub(super) fn started(actions: &[Action]) -> (TalkId, &str) {
    actions
        .iter()
        .find_map(|a| match a {
            Action::StartTalk(t) => Some((t.talk_id, t.script.as_str())),
            _ => None,
        })
        .expect("再生の開始があるはず")
}

/// 毎秒のポンプの応答で `ORIGINAL` を返し、翻訳の結果 `final_script` で再生を始める。
pub(super) fn start_translated(s: &mut State, now: u64, final_script: &str) -> TalkId {
    feed(
        s,
        Input::Tick {
            now: MonotonicMs(now),
        },
    );
    let actions = reply_value(s, ORIGINAL, "OnSecondChange");
    assert!(
        matches!(actions.as_slice(), [Action::Translate(_)]),
        "台本の応答は翻訳へ預けられる"
    );
    let actions = feed(s, Input::TranslateDone(Ok(final_script.to_string())));
    started(&actions).0
}

/// 翻訳の後の最終の台本が控えに入り、送り済み・預かりの印は新しいトークで下りる（要件 1.2・2.2・3.2）。
#[test]
fn shown_talk_keeps_the_final_script_after_translation() {
    let mut s = steady_none();
    // 前のトークの印が立っていても、新しいトークの開始で丸ごと置き換わる。
    s.shown = Some(ShownTalk {
        talk_id: TalkId(4),
        script: "\\0前のトーク\\e".to_string(),
        close_sent: true,
        timeout_sent: true,
        timeout_pending: false,
    });

    feed(
        &mut s,
        Input::Tick {
            now: MonotonicMs(2_000),
        },
    );
    let actions = reply_value(&mut s, ORIGINAL, "OnSecondChange");
    assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
    assert_eq!(
        s.shown.as_ref().map(|shown| shown.talk_id),
        Some(TalkId(4)),
        "翻訳へ預けている間は再生が始まっていないので、控えは前のトークのまま"
    );

    let actions = feed(&mut s, Input::TranslateDone(Ok(FINAL.to_string())));
    let (talk_id, _) = started(&actions);
    assert_eq!(
        s.shown,
        Some(ShownTalk {
            talk_id,
            script: FINAL.to_string(),
            close_sent: false,
            timeout_sent: false,
            timeout_pending: false,
        }),
        "控えは再生を始めたトークの番号と翻訳の後の最終の台本（印はすべて下りる）"
    );
}

/// 切替の `OnClose` の別れの台詞（`ChangeCloseTalkWait`）も控えに入る（設計 D2）。
#[test]
fn shown_talk_keeps_the_change_farewell_script() {
    let mut s = steady_none();
    feed(
        &mut s,
        Input::ChangeGhost(ChangeRequest {
            target: ChangeTarget {
                sakura_name: "ポスト".to_string(),
                name: "R_POST_and_KOMAINU".to_string(),
                dir: r"C:\areka\ghost\r_post".to_string(),
            },
            origin: ChangeOrigin::Manual,
            raise_event: true,
        }),
    );
    feed(
        &mut s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "OnGhostChanging",
        },
    );
    assert!(matches!(s.phase, Phase::ChangeClosePending));
    let actions = reply_value(&mut s, ORIGINAL, "OnClose");
    assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
    let actions = feed(&mut s, Input::TranslateDone(Ok(FINAL.to_string())));
    let (talk_id, _) = started(&actions);
    assert!(
        matches!(s.phase, Phase::ChangeCloseTalkWait { talk_id: t, .. } if t == talk_id),
        "別れの台詞の完了待ちへ進む"
    );
    let shown = s.shown.as_ref().expect("別れの台詞も控えに入る");
    assert_eq!((shown.talk_id, shown.script.as_str()), (talk_id, FINAL));
}

/// 中断を出した後、止まり終える前に別の応答がトークを置き換えたら、置き換えの `step` の出口で
/// 中断の控えを捨て、理由 `talk_gone` を 1 行記録する（要件 1.8・7.2）。控えは新しいトークへ進む。
#[test]
fn replacing_the_talk_drops_the_break_note_with_one_talk_gone_line() {
    let mut s = steady_none();
    let first = start_translated(&mut s, 2_000, FINAL);
    feed(&mut s, Input::UserBreak { scope: 1 });
    assert_eq!(
        s.user_break_talk,
        Some(BreakNote {
            talk_id: first,
            scope: 1
        }),
        "受理で相手と scope を控える"
    );

    // 再生中のダブルクリックの応答が今のトークを置き換える。
    feed(
        &mut s,
        Input::Mouse(MouseInput {
            scope: 0,
            x: 10,
            y: 20,
            region: Some("Head".to_string()),
            kind: MouseEventKind::DoubleClick {
                button: MouseButton::Left,
            },
        }),
    );
    let ev = capture(|| {
        let actions = reply_value(&mut s, ORIGINAL, "OnMouseDoubleClick");
        assert!(matches!(actions.as_slice(), [Action::Translate(_)]));
    });
    assert!(s.user_break_talk.is_none(), "置き換えで中断の控えは捨てる");
    let line = logged_once(&ev, Level::INFO, "balloon_event_not_sent");
    assert_eq!(
        line.fields.get("reason").map(String::as_str),
        Some("talk_gone"),
        "捨てた理由は相手のトークが現行でなくなったこと。\n捕捉={ev:#?}"
    );
    assert_eq!(
        line.fields.get("talk_id").map(String::as_str),
        Some(first.0.to_string().as_str()),
        "捨てた控えの相手の番号を載せる。\n捕捉={ev:#?}"
    );

    let actions = feed(&mut s, Input::TranslateDone(Ok(FINAL.to_string())));
    let (second, _) = started(&actions);
    assert_ne!(second, first);
    assert_eq!(
        s.shown.as_ref().map(|shown| shown.talk_id),
        Some(second),
        "控えは置き換えたトークへ進む"
    );
}
