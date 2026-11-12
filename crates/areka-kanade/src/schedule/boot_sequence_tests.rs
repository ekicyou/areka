use super::test_support::{
    assert_get, assert_notify, boot_until_version_with_greeting, config, drive_to_main,
    drive_to_prefetch, reply,
};
use super::*;
use crate::schedule::translate_test_support::pass_translate;
use crate::schedule::{ActiveTalk, step};
use crate::status::ExecutionSnapshot;
use crate::talk::TalkId;

// --- Full happy path: Idle→…→Steady（各段の Phase＋Action を厳密検証） ---

#[test]
fn full_boot_sequence_carries_greeting_talk_into_steady() {
    let cfg = config();

    // 1. Idle + Boot → OnInitialize NOTIFY / BootInit（Req 1.1）。
    let (s, actions) = step(State::initial(), Input::Boot, &cfg);
    assert!(matches!(s.phase, Phase::BootInit));
    assert_eq!(actions.len(), 1);
    assert_notify(
        &actions[0],
        &events::on_initialize(&ExecutionSnapshot::INACTIVE),
    );

    // 2. BootInit + Notified → username リソース照会（prefetch）GET / BootPrefetch（R4.1）。
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Notified,
            origin: "test",
        },
        &cfg,
    );
    assert!(matches!(s.phase, Phase::BootPrefetch));
    assert_eq!(actions.len(), 1);
    assert_get(
        &actions[0],
        &resources::resource_get("username", &ExecutionSnapshot::INACTIVE),
    );

    // 2b. BootPrefetch + NoContent(204) → [ResourceOutcome, OnFirstBoot GET] / BootType（R4.1）。
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "test",
        },
        &cfg,
    );
    assert!(matches!(s.phase, Phase::BootType));
    assert_eq!(actions.len(), 2, "sink 呼出指示＋OnFirstBoot GET の 2 件");
    assert!(
        matches!(actions[0], Action::ResourceOutcome { .. }),
        "sink 先行"
    );
    assert_get(
        &actions[1],
        &events::on_first_boot(&ExecutionSnapshot::INACTIVE, 0),
    );

    // 3. BootType + NoContent(204) → OnBoot GET / BootMain（Req 1.3）。
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "test",
        },
        &cfg,
    );
    assert!(matches!(s.phase, Phase::BootMain));
    assert_eq!(actions.len(), 1);
    assert_get(
        &actions[0],
        &events::on_boot(&cfg, &ExecutionSnapshot::INACTIVE),
    );

    // 4. BootMain + Value("greeting") → StartTalk(id=1) + basewareversion NOTIFY /
    //    BootVersion{talk: Some(挨拶)}（DD-IT-12: 挨拶を正規追跡）。
    let (s, actions) = pass_translate(
        step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value(
                    "greeting".to_string(),
                    areka_talk::SecurityLevel::Local,
                ),
                origin: "test",
            },
            &cfg,
        ),
        &cfg,
    );
    assert!(
        matches!(
            s.phase,
            Phase::BootVersion {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(1),
                    ..
                })
            }
        ),
        "挨拶 talk を BootVersion{{talk: Some}} で追跡する"
    );
    assert_eq!(actions.len(), 2);
    match &actions[0] {
        Action::StartTalk(StartTalk {
            talk_id, script, ..
        }) => {
            assert_eq!(*talk_id, TalkId(1), "初回 boot talk_id は 1");
            assert_eq!(script, "greeting");
        }
        _ => panic!("expected StartTalk first"),
    }
    // baseware_version の id/references（Status の検証は専用檻
    // `baseware_version_status_reflects_greeting_tracking` が担う）。
    assert_notify(
        &actions[1],
        &events::baseware_version(&cfg, &ExecutionSnapshot::INACTIVE),
    );
    // 採番カウンタが進む。
    assert_eq!(s.next_talk_id, 2);

    // 5. BootVersion{Some} + Notified → Steady{talk: Some(挨拶)}（DD-IT-12: 挨拶を引き継ぐ）。
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Notified,
            origin: "test",
        },
        &cfg,
    );
    assert!(
        matches!(
            s.phase,
            Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(1),
                    ..
                })
            }
        ),
        "挨拶 talk は boot 完了後も Steady へ引き継がれる（Steady{{talk: None}} へ丸めない）"
    );
    assert!(
        matches!(
            actions.as_slice(),
            [Action::Notice(crate::change::KanadeNotice::Steady)]
        ),
        "boot 完了は定常到達の通知 1 件だけ"
    );
    assert!(s.pending_close.is_none());
}

// --- Fallthrough cutoff: BootType + Value → OnBoot SKIPPED ---

#[test]
fn boot_type_value_skips_onboot_and_starts_talk() {
    let cfg = config();
    // BootType へ進める（prefetch の username 照会に 204 を返して OnFirstBoot GET まで）。
    let s = reply(drive_to_prefetch(&cfg), ShioriOutcome::NoContent, &cfg).0;
    assert!(matches!(s.phase, Phase::BootType));

    // BootType + Value("earlygreet") → StartTalk + basewareversion NOTIFY / BootVersion。
    let (s, actions) = reply(
        s,
        ShioriOutcome::Value("earlygreet".to_string(), areka_talk::SecurityLevel::Local),
        &cfg,
    );
    assert!(
        matches!(
            s.phase,
            Phase::BootVersion {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(1),
                    ..
                })
            }
        ),
        "OnBoot をスキップし BootVersion{{talk: Some(挨拶)}} へ（DD-IT-12）"
    );
    assert_eq!(actions.len(), 2);
    match &actions[0] {
        Action::StartTalk(StartTalk {
            talk_id, script, ..
        }) => {
            assert_eq!(*talk_id, TalkId(1));
            assert_eq!(script, "earlygreet");
        }
        _ => panic!("expected StartTalk first"),
    }
    assert_notify(
        &actions[1],
        &events::baseware_version(&cfg, &ExecutionSnapshot::INACTIVE),
    );
}

// --- BootMain + NoContent → no StartTalk（Req 2.3） ---

#[test]
fn boot_main_no_content_emits_no_talk() {
    let cfg = config();
    let s = State {
        phase: Phase::BootMain,
        ..State::initial()
    };
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "test",
        },
        &cfg,
    );
    assert!(
        matches!(s.phase, Phase::BootVersion { talk: None }),
        "204 は挨拶を追跡せず BootVersion{{talk: None}}（Req 2.3）"
    );
    // basewareversion NOTIFY のみ（StartTalk なし）。
    assert_eq!(actions.len(), 1);
    assert_notify(
        &actions[0],
        &events::baseware_version(&cfg, &ExecutionSnapshot::INACTIVE),
    );
    // 採番カウンタは動かない。
    assert_eq!(s.next_talk_id, 1);
}

// --- talk_id uniqueness/monotonicity: 連続 Value 採番は単調増番 ---

#[test]
fn boot_talk_ids_are_unique_and_monotonic() {
    let cfg = config();
    // BootMain + Value → id=1。
    let (s1, actions1) = reply(
        drive_to_main(&cfg),
        ShioriOutcome::Value("a".to_string(), areka_talk::SecurityLevel::Local),
        &cfg,
    );
    let id1 = match &actions1[0] {
        Action::StartTalk(StartTalk { talk_id, .. }) => *talk_id,
        _ => panic!("expected StartTalk"),
    };
    assert_eq!(id1, TalkId(1));
    assert_eq!(s1.next_talk_id, 2);

    // 引き継いだカウンタ（2）から次の Value → id=2（別 run 想定・単調増番）。
    let mut s2 = drive_to_main(&cfg);
    s2.next_talk_id = s1.next_talk_id;
    let (s2, actions2) = reply(
        s2,
        ShioriOutcome::Value("b".to_string(), areka_talk::SecurityLevel::Local),
        &cfg,
    );
    let id2 = match &actions2[0] {
        Action::StartTalk(StartTalk { talk_id, .. }) => *talk_id,
        _ => panic!("expected StartTalk"),
    };
    assert_eq!(id2, TalkId(2), "id は単調増番で再利用しない");
    assert_eq!(s2.next_talk_id, 3);
}

// --- CloseRequest during boot → pending_close recorded, phase unchanged, no actions ---

#[test]
fn close_request_during_boot_records_pending_only() {
    let cfg = config();
    // 各 boot Phase で保留記録のみ・Phase 不変・Action なしを確認。
    for phase in [
        Phase::BootInit,
        Phase::BootPrefetch,
        Phase::BootType,
        Phase::BootMain,
        Phase::BootVersion { talk: None },
        Phase::BootVersion {
            talk: Some(ActiveTalk {
                talk_id: TalkId(1),
                origin: "boot",
                script: String::new(),
            }),
        },
    ] {
        let s = State {
            phase,
            ..State::initial()
        };
        let phase_before = std::mem::discriminant(&s.phase);
        let (s, actions) = step(
            s,
            Input::CloseRequest {
                reason: CloseReason::User { scope: 0 },
            },
            &cfg,
        );
        assert_eq!(
            std::mem::discriminant(&s.phase),
            phase_before,
            "boot 中の CloseRequest は Phase を変えない"
        );
        assert!(actions.is_empty(), "握手はここで開始しない（保留のみ）");
        assert!(
            matches!(s.pending_close, Some(CloseReason::User { scope: 0 })),
            "pending_close に理由が記録される"
        );
    }
}

// --- pending_close survives boot completion to Steady{talk: None} ---

#[test]
fn pending_close_survives_boot_completion() {
    let cfg = config();
    // Boot 開始→BootInit で close 保留→そのまま boot 完走。
    let (s, _) = step(State::initial(), Input::Boot, &cfg);
    let (s, _) = step(
        s,
        Input::CloseRequest {
            reason: CloseReason::System,
        },
        &cfg,
    );
    assert!(matches!(s.phase, Phase::BootInit));
    assert!(matches!(s.pending_close, Some(CloseReason::System)));
    // BootInit→BootPrefetch→BootType→BootMain→BootVersion（204・挨拶なし）→Steady{None} を進める。
    let mut s = s;
    for outcome in [
        ShioriOutcome::Notified,
        ShioriOutcome::NoContent,
        ShioriOutcome::NoContent,
        ShioriOutcome::NoContent,
        ShioriOutcome::Notified,
    ] {
        s = reply(s, outcome, &cfg).0;
    }
    assert!(matches!(s.phase, Phase::Steady { talk: None }));
    assert!(
        matches!(s.pending_close, Some(CloseReason::System)),
        "pending_close は Steady 遷移後も保持され後続層が消費する"
    );
}

// --- DD-IT-12 追加檻: baseware_version の Status が挨拶追跡を反映する ---

/// Testing Strategy「DD-IT-12 追加檻（Unit）」: 挨拶（Value）経路の `baseware_version` は
/// `Status: talking`（フェーズ更新後スナップショット＝`BootVersion{Some}`）、204 経路は
/// Status 行なし（`None`）を運ぶ。events.rs の status 檻の boot 版であり、`baseware_version`
/// 送出値が phase 更新の**後**に撮られること（DD-IT-4）を wire 値で担保する。
#[test]
fn baseware_version_status_reflects_greeting_tracking() {
    let cfg = config();

    // 送出された basewareversion NOTIFY の Status wire 値（`None` ⇔ ヘッダ行なし）を取り出す。
    fn baseware_status(actions: &[Action]) -> Option<Option<String>> {
        actions.iter().find_map(|a| match a {
            Action::ShioriRequest(crate::msg::ShioriCall::Notify { id, status, .. })
                if id.as_str() == "basewareversion" =>
            {
                Some(status.render())
            }
            _ => None,
        })
    }

    // Value（挨拶）経路: BootMain + Value → Status: talking。
    let (_, actions) = reply(
        drive_to_main(&cfg),
        ShioriOutcome::Value("hi".to_string(), areka_talk::SecurityLevel::Local),
        &cfg,
    );
    assert_eq!(
        baseware_status(&actions),
        Some(Some("talking".to_string())),
        "挨拶起動後の basewareversion は Status: talking を運ぶ（DD-IT-12）"
    );

    // 204 経路: BootMain + NoContent → Status 行なし。
    let no_greeting = State {
        phase: Phase::BootMain,
        ..State::initial()
    };
    let (_, actions) = step(
        no_greeting,
        Input::ShioriReply {
            outcome: ShioriOutcome::NoContent,
            origin: "test",
        },
        &cfg,
    );
    assert_eq!(
        baseware_status(&actions),
        Some(None),
        "204 経路の basewareversion は Status 行を出さない（None）"
    );
}

// --- DD-IT-12 追加檻: 挨拶 TalkDone が slot と照合され unknown_talk_done ERROR が出ない ---

/// Testing Strategy「DD-IT-12 追加檻」の相関節を in-source で実現する（Req 1.5/2.4・DD-IT-12）:
/// boot 挨拶（Value）で `Steady{talk: Some(挨拶, id=1)}` へ到達させ、その挨拶 talk の
/// `TalkDone{id=1, Ended}` が slot と照合されて (a) `Steady{talk: None}` へ復帰し、かつ
/// (b) `unknown_talk_done` ERROR が**発火しない**ことを検証する。
///
/// # なぜ in-source（log_capture）か
/// 統合層（close_test の boot 挨拶檻）は kanade アクターが別スレッドで走るため、thread-local な
/// [`crate::schedule::log_capture`] では挨拶 TalkDone の相関ログ（の不在）を直接観測できない。
/// 純粋 step 機械は本テストスレッド上で同期的に走るため、ここでこそ「未知 talk 扱いされていない
/// （＝`unknown_talk_done` ERROR が出ない）」を直接、ログの**不在**として表明できる。挨拶 talk は
/// boot 由来ゆえ本 boot モジュールの檻に置く（`Steady{Some}` へは `step` で boot を通して到達する）。
/// 相関ロジック（`current_talk_id` による slot 照合）が退行して挨拶 TalkDone が未知扱いになれば、
/// この不在表明が失敗する。
#[test]
fn boot_greeting_talkdone_correlates_without_unknown_error() {
    use crate::schedule::log_capture::{assert_not_logged, capture};
    use crate::talk::{TalkDone, TalkEndReason};

    let cfg = config();

    // boot を Idle→…→`Steady{talk: Some(挨拶, id=1)}` まで駆動する（挨拶 Value 経路・DD-IT-12）。
    let s = boot_until_version_with_greeting(&cfg, None);
    let (s, _) = reply(s, ShioriOutcome::Notified, &cfg); // BootVersion{Some}→Steady{talk: Some(id=1)}
    assert!(
        matches!(
            s.phase,
            Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id: TalkId(1),
                    ..
                })
            }
        ),
        "boot 挨拶を Steady{{talk: Some(id=1)}} で正規追跡しているはず（DD-IT-12）"
    );

    // 挨拶 talk の TalkDone{id=1, Ended} を捕捉付きで投入する（相関ロジックは mod.rs の横断アーム）。
    let mut phase_is_none = false;
    let mut actions_empty = false;
    let events = capture(|| {
        let (next, actions) = step(
            s,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(1),
                reason: TalkEndReason::Ended,
                quit_reserved: false,
            }),
            &cfg,
        );
        phase_is_none = matches!(next.phase, Phase::Steady { talk: None });
        actions_empty = actions.is_empty();
    });

    // (a) 挨拶 TalkDone{id=1, Ended} は slot と照合され `Steady{talk: None}` へ復帰する（副作用なし）。
    assert!(
        phase_is_none,
        "挨拶 TalkDone{{id=1, Ended}} は slot と照合され Steady{{talk: None}} へ復帰するはず"
    );
    assert!(actions_empty, "定常復帰（挨拶 talk 完了）は副作用なし");

    // (b) `unknown_talk_done` ERROR が発火していない（照合成立＝未知 talk 扱いされていない・DD-IT-12）。
    assert_not_logged(&events, "unknown_talk_done");
}

// --- 基盤バージョン通知の応答待ち（BootVersion{talk: Some}）中の挨拶 TalkDone 受理 ---

/// `BootVersion{talk: Some(id=1)}` に挨拶 talk の `TalkDone{id=1, reason}` を捕捉付きで投入し、
/// 受理（枠が空・指示 0 件・無視の警告なし・受理の info ちょうど 1 行・error なし）を表明して
/// 次の状態を返す。`Interrupted` では横断遷移が先に `talk_done_interrupted_as_non_quit` の info を
/// 書くが、`boot_talk_done` の行数には数えない（event 名で数えるため）。
fn assert_boot_version_talkdone_accepted(
    s: State,
    reason: crate::talk::TalkEndReason,
    cfg: &KanadeConfig,
) -> State {
    use crate::schedule::log_capture::{
        assert_no_error_logs, assert_not_logged, capture, logged_once,
    };
    use crate::talk::TalkDone;
    use tracing::Level;

    let mut out = None;
    let events = capture(|| {
        out = Some(step(
            s,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(1),
                reason,
                quit_reserved: false,
            }),
            cfg,
        ));
    });
    let (s, actions) = out.expect("step は捕捉窓の中で実行されたはず");

    assert!(
        matches!(s.phase, Phase::BootVersion { talk: None }),
        "応答待ち中の挨拶 TalkDone は受理され、追跡枠が空の BootVersion{{talk: None}} になるはず"
    );
    assert!(
        actions.is_empty(),
        "受理は副作用指示を返さないはず（実際 {} 件）",
        actions.len()
    );
    assert_not_logged(&events, "boot_input_ignored");
    let accepted = logged_once(&events, Level::INFO, "boot_talk_done");
    assert_eq!(
        accepted.fields.get("talk_id").map(String::as_str),
        Some("1"),
        "受理の info は talk_id を運ぶはず"
    );
    assert_no_error_logs(&events);
    s
}

/// 基盤バージョン通知の応答待ち→Steady→close 指示までを、完了理由 `reason` で一周させる。
fn run_boot_version_talkdone_then_close(reason: crate::talk::TalkEndReason) {
    let cfg = config();
    let s = super::test_support::boot_until_version_with_greeting(&cfg, None);
    let s = assert_boot_version_talkdone_accepted(s, reason, &cfg);

    // 通知完了 → 枠が空の定常運転へ（挨拶を引き継がない）。
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Notified,
            origin: "test",
        },
        &cfg,
    );
    assert!(
        matches!(s.phase, Phase::Steady { talk: None }),
        "通知完了で追跡枠が空の Steady{{talk: None}} へ進むはず"
    );
    assert!(
        matches!(
            actions.as_slice(),
            [Action::Notice(crate::change::KanadeNotice::Steady)]
        ),
        "boot 完了は定常到達の通知 1 件だけを返す"
    );

    // close 指示 → 待つべき talk が無いので即握手（OnClose GET・ClosePending）。
    let (s, actions) = step(
        s,
        Input::CloseRequest {
            reason: CloseReason::User { scope: 0 },
        },
        &cfg,
    );
    assert_eq!(actions.len(), 1, "close 指示で OnClose GET が 1 件出るはず");
    assert_get(
        &actions[0],
        &events::on_close(CloseReason::User { scope: 0 }, &ExecutionSnapshot::INACTIVE),
    );
    assert!(matches!(s.phase, Phase::ClosePending));
}

#[test]
fn boot_version_talkdone_before_notified_empties_slot_and_close_handshakes() {
    run_boot_version_talkdone_then_close(crate::talk::TalkEndReason::Ended);
}

#[test]
fn boot_version_talkdone_interrupted_is_treated_as_ended() {
    run_boot_version_talkdone_then_close(crate::talk::TalkEndReason::Interrupted);
}

#[test]
fn boot_version_talkdone_keeps_pending_close_until_steady_tick() {
    use crate::msg::MonotonicMs;

    let cfg = config();
    let s = super::test_support::boot_until_version_with_greeting(&cfg, Some(CloseReason::System));
    let s = assert_boot_version_talkdone_accepted(s, crate::talk::TalkEndReason::Ended, &cfg);
    assert_eq!(
        s.pending_close.map(CloseReason::as_ref_str),
        Some("system"),
        "受理は保留中の close 指示に触れないはず"
    );

    // 通知完了 → Steady{None}。保留はまだ消化されない（握手は次の Tick）。
    let (s, actions) = step(
        s,
        Input::ShioriReply {
            outcome: ShioriOutcome::Notified,
            origin: "test",
        },
        &cfg,
    );
    assert!(matches!(s.phase, Phase::Steady { talk: None }));
    assert!(matches!(
        actions.as_slice(),
        [Action::Notice(crate::change::KanadeNotice::Steady)]
    ));
    assert_eq!(s.pending_close.map(CloseReason::as_ref_str), Some("system"));

    // 次の Tick → 保留を消化して OnClose GET・ClosePending。
    let (s, actions) = step(
        s,
        Input::Tick {
            now: MonotonicMs(1_000),
        },
        &cfg,
    );
    assert_eq!(
        actions.len(),
        1,
        "Tick で保留 close の OnClose GET が 1 件出るはず"
    );
    assert_get(
        &actions[0],
        &events::on_close(CloseReason::System, &ExecutionSnapshot::INACTIVE),
    );
    assert!(matches!(s.phase, Phase::ClosePending));
    assert!(s.pending_close.is_none());
}
