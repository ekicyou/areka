//! 運行表の外から頼まれたイベントの腕と、その応答の腕の決定論テスト（`schedule/sstp.rs` の兄弟）。
//!
//! `on_notify`（任意の名前の GET を 1 つ送り、応答待ちの控えを置く）と `on_notify_reply`（控えを
//! 必ず取り出し、空でない台本なら外からの台本と同じ掃除と採番で新しいトークを始める）を固定する。
//! 翻訳の Reference（外からの台本・返った台本）と strict の控えもここで縛る。

use super::{NOTIFY_ORIGIN, NotifyStage, StrictNote, accepts};
use crate::change::{ChangeOrigin, ChangeRequest, ChangeTarget};
use crate::msg::{
    ChoiceInput, CloseReason, EventId, KanadeConfig, MonotonicMs, ShioriCall, ShioriFailure,
    ShioriOutcome,
};
use crate::schedule::events::{SourceEvent, on_translate, on_translate_sstp};
use crate::schedule::log_capture::{CapturedEvent, assert_not_logged, capture, logged_once};
use crate::schedule::translate::{ScriptSource, TranslateRequest};
use crate::schedule::{
    Action, ActiveTalk, ChoicePhase, ChoiceState, Input, Phase, State, TermCause, current_talk_id,
    phase_label, step,
};
use crate::shiori::real::ABSENT_REFERENCE;
use crate::status::{ExecutionSnapshot, ExecutionStatus};
use crate::talk::{TalkDone, TalkEndReason, TalkId};
use tracing::Level;

fn strings(refs: &[&str]) -> Vec<String> {
    refs.iter().map(|r| r.to_string()).collect()
}

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

fn notify(event: &str, refs: &[&str], strict: Option<bool>) -> Input {
    Input::SstpNotify {
        event: event.to_string(),
        references: strings(refs),
        strict,
    }
}

fn reply(outcome: ShioriOutcome) -> Input {
    Input::ShioriReply {
        outcome,
        origin: NOTIFY_ORIGIN,
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

/// 状態の見比べの指紋（`State` は `PartialEq` を持たないので、腕が触り得る欄を綴りにする）。
fn fingerprint(s: &State) -> String {
    format!(
        "phase={} talk={:?} next={} choice={} prev={:?} close={} change={} translate={} strict={:?} notify={:?} reply={}",
        phase_label(&s.phase),
        current_talk_id(&s.phase).map(|t| t.0),
        s.next_talk_id,
        s.choice.is_some(),
        s.choice_prev_talk.map(|t| t.0),
        s.pending_close.is_some(),
        s.pending_change.is_some(),
        s.translate.is_some(),
        s.sstp_strict,
        s.sstp_notify,
        s.reply_source.is_some(),
    )
}

fn warn_or_above(events: &[CapturedEvent]) -> Vec<&CapturedEvent> {
    events
        .iter()
        .filter(|e| e.level == Level::WARN || e.level == Level::ERROR)
        .collect()
}

/// 一括が任意の名前の GET 1 つで、名前と Reference が渡されたままであることを確かめる。
fn assert_single_get(actions: &[Action], event: &str, refs: &[&str]) {
    match actions {
        [Action::ShioriRequest(ShioriCall::Get { id, references, .. })] => {
            assert_eq!(id, &EventId::Sstp(event.to_string()));
            assert_eq!(references, &strings(refs), "Reference は渡されたまま");
        }
        _ => panic!("任意の名前の GET 1 つのはず"),
    }
}

/// 外から頼まれたイベントを受け付けて GET を送り、その応答を入れる（2 歩）。
fn notify_then_reply(
    state: State,
    event: &str,
    refs: &[&str],
    strict: Option<bool>,
    outcome: ShioriOutcome,
) -> (State, Vec<Action>) {
    let cfg = config();
    let (state, actions) = step(state, notify(event, refs, strict), &cfg);
    assert_single_get(&actions, event, refs);
    assert_eq!(state.sstp_notify, Some(NotifyStage { strict }));
    let (state, actions) = step(state, reply(outcome), &cfg);
    assert!(
        state.sstp_notify.is_none(),
        "応答待ちの控えは応答で必ず空になる"
    );
    (state, actions)
}

fn translate_request(actions: &[Action]) -> &TranslateRequest {
    match actions {
        [Action::Translate(req)] => req,
        _ => panic!("翻訳の依頼 1 つのはず"),
    }
}

fn assert_playing_notify_talk(state: &State, talk_id: u64) {
    match &state.phase {
        Phase::Steady { talk: Some(talk) } => {
            assert_eq!(talk.talk_id, TalkId(talk_id));
            assert_eq!(talk.origin, NOTIFY_ORIGIN);
        }
        _ => panic!("外から頼まれたイベントの台本のトークが流れている Steady のはず"),
    }
}

/// ⑸ 許可の表に無い名前が渡された Reference のまま出て、返った台本で新しいトークが始まる
/// （要件 2.1・2.3・4.5・5.1）。翻訳は元のイベントの出どころ（`EventId::Sstp`）で預けられる。
#[test]
fn unlisted_name_goes_out_verbatim_and_its_script_starts_a_talk() {
    let mut out = None;
    let events = capture(|| {
        out = Some(notify_then_reply(
            steady(None),
            "OnNotListed_ほげ",
            &["a", "", "0\u{1}1"],
            Some(true),
            ShioriOutcome::Value(
                "\\h返った台本\\e".to_string(),
                areka_talk::SecurityLevel::Local,
            ),
        ))
    });
    let (state, actions) = out.unwrap();
    assert!(
        warn_or_above(&events).is_empty(),
        "許可の表に無い名前でも警告しない: {events:#?}"
    );
    assert_playing_notify_talk(&state, 10);
    assert_eq!(state.next_talk_id, 11);
    let req = translate_request(&actions);
    assert_eq!(req.script, "\\h返った台本\\e", "翻訳の前の台本を預ける");
    assert_eq!(
        req.source,
        ScriptSource::Event(SourceEvent {
            id: EventId::Sstp("OnNotListed_ほげ".to_string()),
            references: strings(&["a", "", "0\u{1}1"]),
        })
    );
    let (_, actions) = step(
        state,
        Input::TranslateDone(Ok("\\h訳\\e".into())),
        &config(),
    );
    assert!(matches!(
        actions.as_slice(),
        [Action::StartTalk(start)] if start.talk_id == TalkId(10) && start.script == "\\h訳\\e"
    ));
}

/// ⑵ 再生中は返った台本が次の番号で差し替わる（中断の無効化モードでも同じ・要件 3.2・3.4）。
#[test]
fn returned_script_during_playback_replaces_with_the_next_id_even_with_no_user_break() {
    for no_user_break in [false, true] {
        let mut state = steady(active(9, "OnMouseDoubleClick"));
        state.external.no_user_break = no_user_break;
        state.choice_prev_talk = Some(TalkId(8));
        let (state, actions) = notify_then_reply(
            state,
            "OnAiTalk",
            &[],
            None,
            ShioriOutcome::Value(
                "\\h割り込み\\e".to_string(),
                areka_talk::SecurityLevel::Local,
            ),
        );
        assert_playing_notify_talk(&state, 10);
        assert_eq!(state.choice_prev_talk, None);
        assert_eq!(translate_request(&actions).script, "\\h割り込み\\e");
    }
}

/// ⑹ 返り値なし・空・空白だけ（エラー応答は殻が 204 に写す）は一括が空で、今のトークと
/// 選択肢と strict の控えが残る（要件 2.4・3.2）。
#[test]
fn no_value_empty_or_blank_keeps_the_current_talk() {
    let outcomes = [
        ("204", ShioriOutcome::NoContent),
        (
            "空",
            ShioriOutcome::Value(String::new(), areka_talk::SecurityLevel::Local),
        ),
        (
            "空白だけ",
            ShioriOutcome::Value(" \t\r\n　".to_string(), areka_talk::SecurityLevel::Local),
        ),
    ];
    for (label, outcome) in outcomes {
        let mut state = steady(active(9, "OnMouseDoubleClick"));
        state.choice = Some(ChoiceState {
            talk_id: TalkId(9),
            candidates: vec!["選択1".to_string()],
            deadline: Some(MonotonicMs(1_000)),
            phase: ChoicePhase::Waiting,
        });
        state.sstp_strict = Some(StrictNote {
            talk_id: TalkId(7),
            strict: Some(false),
        });
        let mut out = None;
        let events = capture(|| {
            out = Some(notify_then_reply(
                state,
                "OnNotListed",
                &["r0"],
                Some(true),
                outcome,
            ))
        });
        let (state, actions) = out.unwrap();
        assert!(actions.is_empty(), "{label}: 一括は空のはず");
        assert_eq!(current_talk_id(&state.phase), Some(TalkId(9)), "{label}");
        assert_eq!(state.next_talk_id, 10, "{label}: 採番しない");
        assert!(state.choice.is_some(), "{label}: 選択肢は残る");
        assert!(state.translate.is_none(), "{label}");
        assert_eq!(
            state.sstp_strict,
            Some(StrictNote {
                talk_id: TalkId(7),
                strict: Some(false)
            }),
            "{label}: strict の控えは変えない"
        );
        assert!(warn_or_above(&events).is_empty(), "{label}: {events:#?}");
        logged_once(&events, Level::DEBUG, "sstp_notify_no_script");
    }
}

/// ⑺ 許可の表に在る名前でも Reference を補わない（0 個なら 0 個・要件 2.2）。
#[test]
fn listed_names_get_no_padded_references() {
    for name in [
        "OnBalloonTimeout",
        "OnAnchorSelect",
        "OnChoiceSelect",
        "OnBoot",
    ] {
        let (state, actions) = step(steady(None), notify(name, &[], None), &config());
        assert_single_get(&actions, name, &[]);
        assert_eq!(state.sstp_notify, Some(NotifyStage { strict: None }));
    }
}

/// ⑶ 選択肢の最中に台本が返ると帳簿が消え、その後の時間切れ・選択のイベントは 0 回（要件 3.3）。
#[test]
fn returned_script_during_choice_clears_the_ledger_and_no_choice_events_follow() {
    let mut state = steady(active(9, "OnMouseDoubleClick"));
    state.choice = Some(ChoiceState {
        talk_id: TalkId(9),
        candidates: vec!["選択1".to_string()],
        deadline: Some(MonotonicMs(1_000)),
        phase: ChoicePhase::Waiting,
    });
    let cfg = config();
    let (state, _) = notify_then_reply(
        state,
        "OnAiTalk",
        &[],
        None,
        ShioriOutcome::Value(
            "\\h割り込み\\e".to_string(),
            areka_talk::SecurityLevel::Local,
        ),
    );
    assert!(state.choice.is_none(), "選択肢の帳簿は消えるはず");
    let (state, _) = step(
        state,
        Input::TranslateDone(Ok("\\h割り込み\\e".into())),
        &cfg,
    );
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
    assert!(
        !tick.iter().any(|a| matches!(
            a,
            Action::ShioriRequest(ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. })
                if matches!(id.as_str(), "OnChoiceSelectEx" | "OnChoiceSelect" | "OnChoiceTimeout" | "選択1")
        )),
        "時間切れのイベントを送らない"
    );
    assert_not_logged(&events, "choice_timeout_ledger_stale");
    let mut out = None;
    let events = capture(|| {
        out = Some(step(
            state,
            Input::Choice(ChoiceInput {
                id: "選択1".to_string(),
                label: "選択その1".to_string(),
                scope: 0,
                references: Vec::new(),
            }),
            &cfg,
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
        Some("no_choice_wait")
    );
}

/// ⑷ 起動前・起動の手順・起動の挨拶の最中・終了と切替の手順・保留の終了では受け付けず、
/// SHIORI へ送らない（空の一括・状態不変・warn 以上 0 件・要件 3.6・3.9）。
#[test]
fn notify_is_not_accepted_outside_the_steady_state() {
    let deadline = Some(MonotonicMs(9_000));
    let talk_id = TalkId(9);
    let mut states: Vec<(&str, State)> = [
        ("Idle", Phase::Idle),
        ("BootInit", Phase::BootInit),
        ("BootPrefetch", Phase::BootPrefetch),
        ("BootType", Phase::BootType),
        ("BootMain", Phase::BootMain),
        ("BootVersion{None}", Phase::BootVersion { talk: None }),
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
    .collect();
    states.push((
        "起動の挨拶の最中",
        steady(active(1, super::BOOT_GREETING_ORIGIN)),
    ));
    let mut pending_close = steady(active(9, "OnMouseDoubleClick"));
    pending_close.pending_close = Some(CloseReason::System);
    states.push(("保留の終了", pending_close));
    let mut pending_change = steady(active(9, "OnMouseDoubleClick"));
    pending_change.pending_change = Some(change_request());
    states.push(("保留の切替", pending_change));
    states.push((
        "起動の挨拶の最中（実の起動）",
        boot_until_greeting_playing(&config()),
    ));
    for (label, state) in states {
        assert!(!accepts(&state), "{label}: 受け付けないはず");
        let before = fingerprint(&state);
        let mut out = None;
        // 引数が検査を破っていても、受け付けない状態では断りの debug! だけ。
        let events =
            capture(|| out = Some(step(state, notify("OnX", &["改\n行"], None), &config())));
        let (state, actions) = out.unwrap();
        assert!(actions.is_empty(), "{label}: 空の一括のはず");
        assert_eq!(fingerprint(&state), before, "{label}: 状態は変わらないはず");
        assert!(warn_or_above(&events).is_empty(), "{label}: {events:#?}");
        logged_once(&events, Level::DEBUG, "sstp_not_accepted");
    }
}

fn change_request() -> ChangeRequest {
    ChangeRequest {
        target: ChangeTarget {
            sakura_name: "ポスト".to_string(),
            name: "R_POST_and_KOMAINU".to_string(),
            dir: "C:/areka/ghost/r_post".to_string(),
        },
        origin: ChangeOrigin::Manual,
        raise_event: true,
    }
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
    let (s, _) = step(s, Input::TranslateDone(Ok("\\h挨拶\\e".into())), cfg);
    let s = reply(s, ShioriOutcome::Notified); // → Steady{挨拶}
    assert!(
        matches!(&s.phase, Phase::Steady { talk: Some(t) } if t.origin == super::BOOT_GREETING_ORIGIN),
        "挨拶が流れている Steady のはず"
    );
    s
}

/// 起動の挨拶の `TalkDone` の後は外から頼まれたイベントを受け付け、GET を 1 つ送る（要件 3.6）。
#[test]
fn notify_is_accepted_after_the_boot_greeting() {
    let cfg = config();
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
    let (s, actions) = step(s, notify("OnAiTalk", &["r0"], Some(false)), &cfg);
    assert_single_get(&actions, "OnAiTalk", &["r0"]);
    assert_eq!(
        s.sstp_notify,
        Some(NotifyStage {
            strict: Some(false)
        })
    );
}

/// 名前か Reference が検査を破っていれば送らず、状態を変えず、warn 1 件で空の一括（ゴーストを倒さない）。
#[test]
fn invalid_arguments_send_nothing_and_warn_once() {
    let cases: [(&str, &[&str]); 6] = [
        ("", &[]),
        ("  \t", &[]),
        ("On\rX", &[]),
        ("On\nX", &[]),
        ("OnX", &["ok", "改\n行"]),
        ("OnX", &["nul\0"]),
    ];
    for (event, refs) in cases {
        let state = steady(active(9, "OnMouseDoubleClick"));
        let before = fingerprint(&state);
        let mut out = None;
        let events = capture(|| out = Some(step(state, notify(event, refs, None), &config())));
        let (state, actions) = out.unwrap();
        assert!(actions.is_empty(), "{event:?} {refs:?}: 送らない");
        assert_eq!(fingerprint(&state), before, "{event:?} {refs:?}");
        assert!(matches!(state.phase, Phase::Steady { .. }), "倒れない");
        assert_eq!(warn_or_above(&events).len(), 1, "{events:#?}");
        logged_once(&events, Level::WARN, "sstp_notify_invalid");
    }
}

/// 往復の失敗は控えを取り出したうえで今日の道（終了系列）へ渡す。
#[test]
fn failed_reply_takes_the_note_and_goes_down_today_s_path() {
    let (state, actions) = notify_then_reply(
        steady(active(9, "OnMouseDoubleClick")),
        "OnX",
        &[],
        None,
        ShioriOutcome::Failed(ShioriFailure::Ipc("broken".to_string())),
    );
    assert!(
        matches!(state.phase, Phase::Unloading { .. }),
        "今日どおり終了系列へ: {}",
        phase_label(&state.phase)
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ShioriUnload | Action::ShioriRequest(_)))
    );
}

/// ⑼ 外からの台本と返った台本の翻訳の Reference（要件 4.4・4.5）。殻が出どころで選ぶ組み立てを
/// 依頼の出どころに当てて、送る Reference の列を確かめる。
#[test]
fn translate_references_for_send_and_returned_script() {
    let build = |req: &TranslateRequest| match &req.source {
        ScriptSource::Event(event) => on_translate(&req.script, event, req.status.clone()),
        ScriptSource::SstpSend => on_translate_sstp(&req.script, req.status.clone()),
    };
    let refs_of = |call: ShioriCall| match call {
        ShioriCall::Get { id, references, .. } => {
            assert_eq!(id, EventId::Static("OnTranslate"));
            references
        }
        ShioriCall::Notify { .. } => panic!("OnTranslate は GET"),
    };

    let send = Input::SstpSend {
        script: "\\h外\\e".to_string(),
        strict: None,
    };
    let (_, actions) = step(steady(None), send, &config());
    assert_eq!(
        refs_of(build(translate_request(&actions))),
        strings(&["\\h外\\e", "sstp-send,owned"]),
        "外からの台本: Ref1＝sstp-send,owned・Ref2 以降なし"
    );

    let (_, actions) = notify_then_reply(
        steady(None),
        "OnAiTalk",
        &["r0", "r1"],
        None,
        ShioriOutcome::Value("\\h返\\e".to_string(), areka_talk::SecurityLevel::Local),
    );
    assert_eq!(
        refs_of(build(translate_request(&actions))),
        vec![
            "\\h返\\e".to_string(),
            ABSENT_REFERENCE.to_string(),
            "OnAiTalk".to_string(),
            "r0\u{1}r1".to_string(),
        ],
        "返った台本: Ref1 欠番・Ref2＝名前・Ref3＝Reference をバイト値 1 で連ねたもの"
    );
}

/// ⑼ strict が真・偽・省略の 3 通りで、返った台本のトークの番号と値が控えと一致する（要件 5.1・5.3）。
#[test]
fn strict_is_noted_with_the_returned_talk_id() {
    for strict in [Some(true), Some(false), None] {
        let (state, _) = notify_then_reply(
            steady(None),
            "OnAiTalk",
            &[],
            strict,
            ShioriOutcome::Value("\\h返\\e".to_string(), areka_talk::SecurityLevel::Local),
        );
        assert_eq!(
            state.sstp_strict,
            Some(StrictNote {
                talk_id: TalkId(10),
                strict
            })
        );
    }
}

/// 実行状態は送る時点の状態から導く（話し中なら `talking`）。
#[test]
fn notify_get_status_is_derived_from_the_state() {
    let state = steady(active(9, "OnMouseDoubleClick"));
    let want = ExecutionStatus::derive(&state.snapshot());
    assert_ne!(
        want,
        ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
        "話し中で INACTIVE と見分けがつくこと"
    );
    let (_, actions) = step(state, notify("OnX", &[], None), &config());
    match actions.as_slice() {
        [Action::ShioriRequest(ShioriCall::Get { status, .. })] => assert_eq!(status, &want),
        _ => panic!("GET 1 つのはず"),
    }
}
