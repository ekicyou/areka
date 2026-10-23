//! 運行表の SSTP の入口の決定論テスト（`schedule/sstp.rs` の兄弟）。
//!
//! `events.rs` の 2 つの組み立て（`sstp_notify`・`on_translate_sstp`）と、受け付けの判定・
//! 外からの台本の腕（`accepts`・`on_send`）を固定する。

use super::{BOOT_GREETING_ORIGIN, SEND_ORIGIN, StrictNote, accepts};
use crate::change::{ChangeOrigin, ChangeRequest, ChangeTarget};
use crate::msg::{
    ChoiceInput, CloseReason, EventId, KanadeConfig, MonotonicMs, ShioriCall, ShioriOutcome,
};
use crate::schedule::events::{REF1_SSTP_SEND_OWNED, on_translate_sstp, sstp_notify};
use crate::schedule::log_capture::{CapturedEvent, assert_not_logged, capture, logged_once};
use crate::schedule::translate::ScriptSource;
use crate::schedule::{
    Action, ActiveTalk, ChoicePhase, ChoiceState, Input, Phase, State, TermCause, current_talk_id,
    phase_label, step,
};
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use crate::talk::{TalkDone, TalkEndReason, TalkId};
use tracing::Level;

fn strings(refs: &[&str]) -> Vec<String> {
    refs.iter().map(|r| r.to_string()).collect()
}

/// 任意の名前の GET は `EventId::Sstp` で、渡された Reference をそのまま載せる（要件 2.1・2.2）。
#[test]
fn sstp_notify_is_a_get_with_the_given_references_verbatim() {
    let cases: [&[&str]; 4] = [&[], &["a"], &["", "x", ""], &["0\u{1}1", "改\r行"]];
    for refs in cases {
        match sstp_notify(
            "OnNotListed_ほげ".to_string(),
            strings(refs),
            &ExecutionSnapshot::INACTIVE,
        ) {
            ShioriCall::Get {
                id,
                references,
                status,
            } => {
                assert_eq!(id, EventId::Sstp("OnNotListed_ほげ".to_string()));
                assert_eq!(references, strings(refs), "Reference={refs:?}");
                assert_eq!(
                    status,
                    ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE)
                );
            }
            ShioriCall::Notify { .. } => panic!("SstpNotify は GET で送る"),
        }
    }
}

/// 許可の表に在る名前でも Reference を補わない（0 個なら 0 個・要件 2.2）。
#[test]
fn sstp_notify_does_not_pad_references_for_listed_names() {
    for name in ["OnBalloonTimeout", "OnAnchorSelect", "OnBoot"] {
        match sstp_notify(name.to_string(), Vec::new(), &ExecutionSnapshot::INACTIVE) {
            ShioriCall::Get { id, references, .. } => {
                assert_eq!(id, EventId::Sstp(name.to_string()));
                assert!(references.is_empty(), "{name}: Reference を補わない");
            }
            ShioriCall::Notify { .. } => panic!("SstpNotify は GET で送る"),
        }
    }
}

/// 実行状態は渡した snapshot から導く（話し中なら `talking`）。
#[test]
fn sstp_notify_derives_status_from_the_snapshot() {
    let snap = ExecutionSnapshot {
        talk_active: true,
        ..ExecutionSnapshot::INACTIVE
    };
    match sstp_notify("OnX".to_string(), Vec::new(), &snap) {
        ShioriCall::Get { status, .. } => assert_eq!(status, ExecutionStatus::derive(&snap)),
        ShioriCall::Notify { .. } => panic!("SstpNotify は GET で送る"),
    }
}

/// SSTP SEND の台本の `OnTranslate`: Ref0＝展開済み・Ref1＝`sstp-send,owned`・Ref2 以降なし（要件 4.4）。
#[test]
fn on_translate_sstp_has_reference0_and_reference1_only() {
    assert_eq!(REF1_SSTP_SEND_OWNED, "sstp-send,owned");
    let status = ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE);
    match on_translate_sstp("\\h外からの台本\\e", status.clone()) {
        ShioriCall::Get {
            id,
            references,
            status: got,
        } => {
            assert_eq!(
                id,
                EventId::Translate(crate::request_origin::RequestOrigin::MCP_SEND_TRANSLATE)
            );
            assert_eq!(
                references,
                vec![
                    "\\h外からの台本\\e".to_string(),
                    "sstp-send,owned".to_string()
                ]
            );
            assert_eq!(got, status);
        }
        ShioriCall::Notify { .. } => panic!("OnTranslate は GET で送る"),
    }
}

// ============================================================
// 受け付けの判定と外からの台本の腕（`accepts`・`on_send`）
// ============================================================

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

fn send(script: &str, strict: Option<bool>) -> Input {
    Input::SstpSend {
        script: script.to_string(),
        strict,
    }
}

fn active(talk_id: u64, origin: &'static str) -> Option<ActiveTalk> {
    Some(ActiveTalk {
        talk_id: TalkId(talk_id),
        origin,
        script: "\\h流れている台本\\e".to_string(),
    })
}

fn steady(talk: Option<ActiveTalk>) -> State {
    State {
        phase: Phase::Steady { talk },
        last_now: Some(MonotonicMs(500)),
        next_talk_id: 10,
        ..State::initial()
    }
}

fn change_request() -> ChangeRequest {
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
fn fingerprint(s: &State) -> String {
    format!(
        "phase={} talk={:?} next={} choice={} prev={:?} close={} change={} translate={} strict={:?} reply={}",
        phase_label(&s.phase),
        current_talk_id(&s.phase).map(|t| t.0),
        s.next_talk_id,
        s.choice.is_some(),
        s.choice_prev_talk.map(|t| t.0),
        s.pending_close.is_some(),
        s.pending_change.is_some(),
        s.translate.is_some(),
        s.sstp_strict,
        s.reply_source.is_some(),
    )
}

fn warn_or_above(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == Level::WARN || e.level == Level::ERROR)
        .collect()
}

/// 受け付けない状態: 空の一括・状態は変わらない・warn 以上 0 件・`debug!` 1 件（要件 3.6・3.9）。
fn assert_rejected(label: &str, state: State) {
    assert!(!accepts(&state), "{label}: 受け付けないはず");
    let before = fingerprint(&state);
    let mut out = None;
    let events = capture(|| out = Some(step(state, send("\\h外\\e", Some(true)), &config())));
    let (state, actions) = out.unwrap();
    assert!(actions.is_empty(), "{label}: 空の一括のはず");
    assert_eq!(fingerprint(&state), before, "{label}: 状態は変わらないはず");
    assert!(
        warn_or_above(&events).is_empty(),
        "{label}: warn 以上を出さない: {events:#?}"
    );
    logged_once(&events, Level::DEBUG, "sstp_not_accepted");
}

/// 受け付けた一括は翻訳の依頼 1 つ（出どころは外からの台本）で、預けた一括の再生の開始は新しい番号。
fn assert_deferred(state: &State, actions: &[Action], talk_id: u64, script: &str) {
    match actions {
        [Action::Translate(req)] => {
            assert_eq!(req.script, script);
            assert_eq!(req.source, ScriptSource::SstpSend);
        }
        _ => panic!("翻訳の依頼 1 つのはず"),
    }
    let wait = state.translate.as_ref().expect("翻訳の帳簿があるはず");
    assert_eq!(wait.talk_id, TalkId(talk_id));
    assert!(matches!(
        wait.deferred.as_slice(),
        [Action::StartTalk(start)] if start.talk_id == TalkId(talk_id) && start.script == script
    ));
}

fn assert_playing(state: &State, talk_id: u64) {
    match &state.phase {
        Phase::Steady { talk: Some(talk) } => {
            assert_eq!(talk.talk_id, TalkId(talk_id));
            assert_eq!(talk.origin, SEND_ORIGIN);
        }
        _ => panic!("外からの台本のトークが流れている Steady のはず"),
    }
}

/// ⑴ 普段の状態（トーク無し）で新しいトークになり、strict を番号と結んで控える（要件 1.1・1.2・5.1・6.3）。
#[test]
fn send_in_idle_steady_starts_a_new_talk_and_notes_strict() {
    for strict in [Some(true), Some(false), None] {
        let state = steady(None);
        assert!(accepts(&state));
        let (state, actions) = step(state, send("\\h外からの台本\\e", strict), &config());
        assert_playing(&state, 10);
        assert_eq!(state.next_talk_id, 11);
        assert_eq!(
            state.sstp_strict,
            Some(StrictNote {
                talk_id: TalkId(10),
                strict
            })
        );
        assert_deferred(&state, &actions, 10, "\\h外からの台本\\e");
        // 翻訳の結果で預けた再生の開始が返る（ゴースト自身の台本と同じ道）。
        let (_, actions) = step(
            state,
            Input::TranslateDone(Ok(crate::schedule::translate::Translated::local(
                "\\h訳\\e",
            ))),
            &config(),
        );
        assert!(matches!(
            actions.as_slice(),
            [Action::StartTalk(start)] if start.talk_id == TalkId(10) && start.script == "\\h訳\\e"
        ));
    }
}

/// 0 文字の台本は翻訳へ預けず、そのまま再生の開始を返す。
#[test]
fn empty_send_is_not_deferred_to_translate() {
    let (state, actions) = step(steady(None), send("", None), &config());
    assert_playing(&state, 10);
    assert!(state.translate.is_none());
    assert!(matches!(
        actions.as_slice(),
        [Action::StartTalk(start)] if start.talk_id == TalkId(10) && start.script.is_empty()
    ));
}

/// ⑵ 再生中は次の番号で差し替わり、前の控えは新しい控えで上書きされる（中断の無効化モードでも同じ・要件 3.1・3.4）。
#[test]
fn send_during_playback_replaces_with_the_next_id_even_with_no_user_break() {
    for no_user_break in [false, true] {
        let mut state = steady(active(9, "OnMouseDoubleClick"));
        state.external.no_user_break = no_user_break;
        state.choice_prev_talk = Some(TalkId(8));
        state.sstp_strict = Some(StrictNote {
            talk_id: TalkId(7),
            strict: Some(true),
        });
        assert!(accepts(&state));
        let (state, actions) = step(state, send("\\h割り込み\\e", None), &config());
        assert_playing(&state, 10);
        assert_eq!(
            state.choice_prev_talk, None,
            "1 世代の控えは次の差し替えで消える"
        );
        assert_eq!(
            state.sstp_strict,
            Some(StrictNote {
                talk_id: TalkId(10),
                strict: None
            })
        );
        assert_deferred(&state, &actions, 10, "\\h割り込み\\e");
    }
}

/// 選択肢の ID（`On` で始まらない正典の形＝選択はカスケードで `OnChoiceSelectEx`／`OnChoiceSelect` を送る）。
const CHOICE_ID: &str = "選択1";

/// 選択の 3 つのイベントと選択肢の ID そのものの名前の往復を数える。
fn choice_event_count(actions: &[Action]) -> usize {
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

/// ⑶ 選択肢の最中は帳簿が消え、その後の時間切れ・選択のイベントは 0 回（要件 3.3）。
#[test]
fn send_during_choice_clears_the_ledger_and_no_choice_events_follow() {
    let mut state = steady(active(9, "OnMouseDoubleClick"));
    state.choice = Some(ChoiceState {
        talk_id: TalkId(9),
        candidates: vec![CHOICE_ID.to_string()],
        deadline: Some(MonotonicMs(1_000)),
        phase: ChoicePhase::Waiting,
    });
    let cfg = config();
    let (state, _) = step(state, send("\\h割り込み\\e", None), &cfg);
    assert!(state.choice.is_none(), "選択肢の帳簿は消えるはず");
    let (state, _) = step(
        state,
        Input::TranslateDone(Ok(crate::schedule::translate::Translated::local(
            "\\h割り込み\\e",
        ))),
        &cfg,
    );
    // 帳簿が残っていても下流の番号の照合が送出を止めるので、送出の 0 回に加えて
    // 「残った帳簿の防御が働かなかった」こと（期限切れの帳簿の trace が無い・選択の棄却の理由が
    // 帳簿なし）を記録で縛る。帳簿の掃除を外すとこの 2 つが赤になる。
    let mut out = None;
    let events = capture(|| {
        out = Some(step(
            state,
            Input::Tick {
                now: MonotonicMs(60_000),
            },
            &cfg,
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
            &cfg,
        ))
    });
    let (_, choice) = out.unwrap();
    assert_eq!(choice_event_count(&choice), 0, "選択のイベントを送らない");
    let rejected = logged_once(&events, Level::WARN, "choice_rejected_no_wait");
    assert_eq!(
        rejected.fields.get("reason").map(String::as_str),
        Some("no_choice_wait"),
        "選択待ちが無いので棄却される（残った帳簿との番号の食い違いではない）"
    );
}

/// ⑷ 起動前・起動の手順・終了と切替の手順・保留の終了と切替では受け付けない（要件 3.6・3.9）。
#[test]
fn send_is_not_accepted_outside_the_steady_state() {
    let deadline = Some(MonotonicMs(9_000));
    let talk_id = TalkId(9);
    let phases: Vec<(&str, Phase)> = vec![
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
        (
            "ClosePending",
            Phase::ClosePending {
                reason: CloseReason::System,
            },
        ),
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
    ];
    for (label, phase) in phases {
        assert_rejected(
            label,
            State {
                phase,
                next_talk_id: 10,
                ..State::initial()
            },
        );
    }
    let mut pending_close = steady(active(9, "OnMouseDoubleClick"));
    pending_close.pending_close = Some(CloseReason::System);
    assert_rejected("保留の終了", pending_close);
    let mut pending_change = steady(active(9, "OnMouseDoubleClick"));
    pending_change.pending_change = Some(change_request());
    assert_rejected("保留の切替", pending_change);
}

/// 起動の手順を実の `step` でたどり、起動の挨拶が流れている `Steady` まで進める。
fn boot_until_greeting_playing(cfg: &KanadeConfig) -> State {
    let reply = |s, outcome| {
        step(
            s,
            Input::ShioriReply {
                outcome,
                origin: "test",
            },
            cfg,
        )
        .0
    };
    let (s, _) = step(State::initial(), Input::Boot, cfg);
    let s = reply(s, ShioriOutcome::Notified); // BootInit → BootPrefetch
    let s = reply(s, ShioriOutcome::NoContent); // BootPrefetch → BootType
    let s = reply(
        s,
        ShioriOutcome::Value("\\h挨拶\\e".to_string(), areka_talk::SecurityLevel::Local),
    ); // → BootVersion{挨拶}
    let (s, _) = step(
        s,
        Input::TranslateDone(Ok(crate::schedule::translate::Translated::local(
            "\\h挨拶\\e",
        ))),
        cfg,
    );
    let s = reply(s, ShioriOutcome::Notified); // → Steady{挨拶}
    assert!(
        matches!(&s.phase, Phase::Steady { talk: Some(t) } if t.origin == BOOT_GREETING_ORIGIN),
        "挨拶が流れている Steady のはず"
    );
    s
}

/// 起動の挨拶の最中は断り、挨拶の後は受け付ける。
///
/// 挨拶の見分けは `boot.rs` が置く綴りと [`BOOT_GREETING_ORIGIN`] の一致に頼るので、
/// `boot.rs` の綴りが変わればここが赤になる（要件 3.6）。
#[test]
fn send_during_the_boot_greeting_is_rejected_and_accepted_after_it() {
    assert_eq!(BOOT_GREETING_ORIGIN, "boot");
    let cfg = config();
    assert_rejected("起動の挨拶の最中", boot_until_greeting_playing(&cfg));

    let s = boot_until_greeting_playing(&cfg);
    let greeting = current_talk_id(&s.phase).unwrap();
    let (s, _) = step(
        s,
        Input::TalkDone(TalkDone {
            talk_id: greeting,
            reason: TalkEndReason::Ended,
            quit_reserved: false,
        }),
        &cfg,
    );
    assert!(accepts(&s), "挨拶の後は受け付けるはず");
    let next = s.next_talk_id;
    let (s, _) = step(s, send("\\h外\\e", None), &cfg);
    assert_playing(&s, next);
}
