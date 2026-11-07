//! crate の中のテストが共に使う補助（テスト専用）。
//!
//! 翻訳の行動を台詞そのままで入れ直して続きを返す [`pass_translate`] のほか、状態・一括・設定の
//! 下ごしらえと見比べの道具を置く。

use super::log_capture::{CapturedEvent, assert_not_logged, capture, logged_once};
use super::sstp::BOOT_GREETING_ORIGIN;
use super::{
    Action, ActiveTalk, Input, Phase, State, TermCause, current_talk_id, phase_label, step,
};
use crate::change::{ChangeOrigin, ChangeRequest, ChangeTarget};
use crate::msg::{ChoiceInput, KanadeConfig, MonotonicMs, ShioriCall, ShioriOutcome};
use crate::talk::TalkId;
use tracing::Level;

/// `step` の返り値の一括が翻訳の行動 1 つだけなら、結果を台詞そのまま（`Ok(script)`）で入れ直して
/// 続きを返す。それ以外の一括はそのまま返す。
pub(crate) fn pass_translate(
    (state, actions): (State, Vec<Action>),
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    match actions.as_slice() {
        [Action::Translate(request)] => {
            let script = request.script.clone();
            step(
                state,
                Input::TranslateDone(Ok(crate::schedule::translate::Translated::local(script))),
                config,
            )
        }
        _ => (state, actions),
    }
}

/// 一括が GET 1 本だけであることを確かめ、(イベント名, Reference の並び) を返す。
pub(crate) fn only_get(actions: &[Action]) -> (String, Vec<String>) {
    assert_eq!(
        actions.len(),
        1,
        "GET 1 本だけのはず（{} 件）",
        actions.len()
    );
    super::steady::test_support::expect_get_call(&actions[0])
}

/// 選択肢の ID（`On` で始まらない正典の形＝選択はカスケードで `OnChoiceSelectEx`／`OnChoiceSelect` を送る）。
pub(crate) const CHOICE_ID: &str = "選択1";

/// 選択の 3 つのイベントと選択肢の ID そのものの名前の往復を数える。
pub(crate) fn choice_event_count(actions: &[Action]) -> usize {
    actions
        .iter()
        .filter(|action| match action {
            Action::ShioriRequest(ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. }) => {
                matches!(
                    id.as_str(),
                    "OnChoiceSelectEx" | "OnChoiceSelect" | "OnChoiceTimeout"
                ) || id.as_str() == CHOICE_ID
            }
            _ => false,
        })
        .count()
}

pub(crate) fn strings(refs: &[&str]) -> Vec<String> {
    refs.iter().map(|r| r.to_string()).collect()
}

pub(crate) fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

pub(crate) fn active(talk_id: u64, origin: &'static str) -> Option<ActiveTalk> {
    Some(ActiveTalk {
        talk_id: TalkId(talk_id),
        origin,
        script: "\\h流れている台本\\e".to_string(),
    })
}

pub(crate) fn steady(talk: Option<ActiveTalk>) -> State {
    State {
        phase: Phase::Steady { talk },
        last_now: Some(MonotonicMs(500)),
        next_talk_id: 10,
        ..State::initial()
    }
}

/// `Steady{talk: None}`（時刻 1,000・次の番号 5・保留と控えなし）。ここから実際の入力で前提へ進める。
pub(crate) fn steady_none() -> State {
    State {
        phase: Phase::Steady { talk: None },
        last_now: Some(MonotonicMs(1_000)),
        next_talk_id: 5,
        ..State::initial()
    }
}

/// 定常でない相の状態を 1 つずつ（次の番号 10・ほかは初期値）。外からの入口が受け付けない表の行。
pub(crate) fn non_steady_states() -> Vec<(&'static str, State)> {
    let deadline = Some(MonotonicMs(9_000));
    let talk_id = TalkId(9);
    [
        ("Idle", Phase::Idle),
        ("BootInit", Phase::BootInit),
        ("BootPrefetch", Phase::BootPrefetch),
        ("BootType", Phase::BootType),
        ("BootMain", Phase::BootMain),
        ("BootVersion{None}", Phase::BootVersion { talk: None }),
        (
            "BootVersion{Some}",
            Phase::BootVersion {
                talk: active(1, BOOT_GREETING_ORIGIN),
            },
        ),
        ("ClosePending", Phase::ClosePending),
        ("CloseTalkWait", Phase::CloseTalkWait { talk_id, deadline }),
        ("ChangePending", Phase::ChangePending),
        (
            "ChangeTalkWait",
            Phase::ChangeTalkWait { talk_id, deadline },
        ),
        ("ChangeClosePending", Phase::ChangeClosePending),
        (
            "ChangeCloseTalkWait",
            Phase::ChangeCloseTalkWait { talk_id, deadline },
        ),
        (
            "Unloading",
            Phase::Unloading {
                cause: TermCause::Quit,
            },
        ),
        ("Stopped", Phase::Stopped),
    ]
    .into_iter()
    .map(|(label, phase)| {
        (
            label,
            State {
                phase,
                next_talk_id: 10,
                ..State::initial()
            },
        )
    })
    .collect()
}

pub(crate) fn change_request() -> ChangeRequest {
    ChangeRequest {
        target: ChangeTarget {
            sakura_name: "ポスト".to_string(),
            name: "R_POST_and_KOMAINU".to_string(),
            dir: r"C:\areka\ghost\r_post".to_string(),
        },
        origin: ChangeOrigin::Manual,
        raise_event: true,
    }
}

/// 状態の見比べの指紋（`State` は `PartialEq` を持たないので、腕が触り得る欄を綴りにする）。
pub(crate) fn fingerprint(s: &State) -> String {
    format!(
        "phase={} talk={:?} next={} choice={} prev={:?} close={} change={} translate={} notify={:?} reply={}",
        phase_label(&s.phase),
        current_talk_id(&s.phase).map(|t| t.0),
        s.next_talk_id,
        s.choice.is_some(),
        s.prev_talk.map(|t| t.0),
        s.pending_close.is_some(),
        s.pending_change.is_some(),
        s.translate.is_some(),
        s.sstp_notify,
        s.reply_source.is_some(),
    )
}

pub(crate) fn warn_or_above(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == Level::WARN || e.level == Level::ERROR)
        .collect()
}

/// 応答を `origin: "test"` で `step` に入れ、続きの状態を返す。
fn reply(s: State, outcome: ShioriOutcome, cfg: &KanadeConfig) -> State {
    step(
        s,
        Input::ShioriReply {
            outcome,
            origin: "test",
        },
        cfg,
    )
    .0
}

/// 起動の手順を実の `step` でたどり、起動の挨拶の台詞が翻訳を待っているところまで進める。
pub(crate) fn boot_until_greeting_deferred(cfg: &KanadeConfig) -> State {
    let (s, _) = step(State::initial(), Input::Boot, cfg);
    let s = reply(s, ShioriOutcome::Notified, cfg); // BootInit → BootPrefetch
    let s = reply(s, ShioriOutcome::NoContent, cfg); // BootPrefetch → BootType
    reply(
        s,
        ShioriOutcome::Value("\\h挨拶\\e".to_string(), areka_talk::SecurityLevel::Local),
        cfg,
    ) // → 翻訳待ち（BootVersion{挨拶} へ進む手前）
}

/// 起動の手順を実の `step` でたどり、起動の挨拶が流れている `Steady` まで進める。
pub(crate) fn boot_until_greeting_playing(cfg: &KanadeConfig) -> State {
    let s = boot_until_greeting_deferred(cfg);
    let (s, _) = step(
        s,
        Input::TranslateDone(Ok(crate::schedule::translate::Translated::local(
            "\\h挨拶\\e",
        ))),
        cfg,
    );
    let s = reply(s, ShioriOutcome::Notified, cfg); // → Steady{挨拶}
    assert!(
        matches!(&s.phase, Phase::Steady { talk: Some(t) } if t.origin == BOOT_GREETING_ORIGIN),
        "挨拶が流れている Steady のはず"
    );
    s
}

/// 選択肢の帳簿が消えた後の状態へ時間切れの刻みと選択を入れ、選択のイベントが 1 度も送られないことを
/// 確かめる（要件 3.3）。
///
/// 帳簿が残っていても下流の番号の照合が送出を止めるので、送出の 0 回に加えて「残った帳簿の防御が
/// 働かなかった」こと（期限切れの帳簿の trace が無い・選択の棄却の理由が帳簿なし）を記録で縛る。
/// 帳簿の掃除を外すとこの 2 つが赤になる。
pub(crate) fn assert_no_choice_events_follow(state: State, cfg: &KanadeConfig) {
    let mut out = None;
    let events = capture(|| {
        out = Some(step(
            state,
            Input::Tick {
                now: MonotonicMs(60_000),
            },
            cfg,
        ))
    });
    let (state, tick) = out.unwrap();
    assert_eq!(choice_event_count(&tick), 0, "時間切れのイベントを送らない");
    assert_not_logged(&events, "choice_timeout_ledger_stale");
    let mut out = None;
    let events = capture(|| {
        out = Some(step(
            state,
            Input::Choice(ChoiceInput {
                id: CHOICE_ID.to_string(),
                label: "選択その1".to_string(),
                scope: 0,
                references: Vec::new(),
            }),
            cfg,
        ))
    });
    let (_, choice) = out.unwrap();
    assert!(
        !choice.iter().any(|a| matches!(a, Action::ShioriRequest(_))),
        "選択のイベントを送らない"
    );
    let rejected = logged_once(&events, Level::WARN, "choice_rejected_no_wait");
    assert_eq!(
        rejected.fields.get("reason").map(String::as_str),
        Some("no_choice_wait"),
        "選択待ちが無いので棄却される（残った帳簿との番号の食い違いではない）"
    );
}
