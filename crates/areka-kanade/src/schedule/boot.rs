//! boot 系列の Phase 分岐（Req 1・状態機械図の Idle→…→Steady）。
//!
//! 本モジュールは boot 各待ち点（[`Phase::Idle`] からの起動・BootInit / BootType /
//! BootMain / BootVersion の応答進行）の遷移本体を担う。タスク 2.3 が本体を実装する。
//! 2.1 では骨格（呼出面）のみを用意し、[`crate::schedule::mod`] の `step` から
//! フェーズ分岐として呼び出せるようにする。

use super::{events, snapshot_of, Action, ActiveTalk, Input, Phase, State};
use crate::msg::{CloseReason, KanadeConfig, ShioriOutcome};
use crate::status::ExecutionSnapshot;
use crate::talk::{StartTalk, TalkId};

/// boot 系列（Idle / BootInit / BootType / BootMain / BootVersion）のフェーズ分岐。
///
/// 起動指示受領から基盤バージョン通知完了までの正典順序を状態機械として実装する
/// （Req 1.1–1.4・1.6）。各段階でスクリプト（Value）が返った場合は一意な talk_id を
/// 採番して再生起動要求（[`Action::StartTalk`]）を発行する（Req 2.1）。起動種別問い合わせ
/// （BootType）にスクリプトが返った場合は `OnBoot` を飛ばして basewareversion へ進む
/// （正典フォールスルー打ち切り・design §"boot 系列"）。204 は talk なし（Req 2.3）。
///
/// boot 中の [`Input::CloseRequest`] は `pending_close` として記録するのみで boot は
/// 中断しない（握手開始は steady/close 層の責務・design System Flows 補足）。
pub(crate) fn step(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match input {
        // 起動指示（Idle 前提・非 Idle は mod.rs 側で warn!＋無視済み）。
        Input::Boot => boot_start(state),
        // 応答進行（各待ち点で直前呼出の結果を受ける）。
        Input::ShioriReply { outcome } => on_reply(state, outcome, config),
        // boot 中の close 指示は保留記録のみ（boot 継続・握手は後続層）。
        Input::CloseRequest { reason } => record_pending_close(state, reason),
        // 上記以外（Tick・TalkDone など）は boot 進行に無関係——防御的に無視する。
        _ => {
            tracing::warn!(target: "kanade", event = "boot_input_ignored", "boot 系列に無関係な入力を無視");
            (state, Vec::new())
        }
    }
}

/// Idle + Boot: `OnInitialize` NOTIFY を発行し BootInit へ（Req 1.1）。
fn boot_start(mut state: State) -> (State, Vec<Action>) {
    tracing::info!(target: "kanade", event = "boot_start", "起動指示を受領——OnInitialize NOTIFY を発行");
    state.phase = Phase::BootInit;
    (
        state,
        vec![Action::ShioriRequest(events::on_initialize(&ExecutionSnapshot::INACTIVE))],
    )
}

/// boot 各待ち点の応答進行（正典順序の状態機械）。
fn on_reply(state: State, outcome: ShioriOutcome, config: &KanadeConfig) -> (State, Vec<Action>) {
    match state.phase {
        // BootInit + Notified: OnInitialize 完了→OnFirstBoot GET 発行・BootType へ（Req 1.2）。
        Phase::BootInit => match outcome {
            ShioriOutcome::Notified => {
                let mut state = state;
                state.phase = Phase::BootType;
                (
                    state,
                    vec![Action::ShioriRequest(events::on_first_boot(&ExecutionSnapshot::INACTIVE))],
                )
            }
            other => unexpected_reply(state, "BootInit", other),
        },
        // BootType: 204→OnBoot GET（BootMain へ・Req 1.3）。Value→OnBoot をスキップし
        // StartTalk＋basewareversion（正典フォールスルー打ち切り・Req 2.1）。
        Phase::BootType => match outcome {
            ShioriOutcome::NoContent => {
                let mut state = state;
                state.phase = Phase::BootMain;
                (state, vec![Action::ShioriRequest(events::on_boot(config, &ExecutionSnapshot::INACTIVE))])
            }
            ShioriOutcome::Value(script) => {
                tracing::info!(target: "kanade", event = "boot_type_script", "起動種別にスクリプト——OnBoot をスキップし basewareversion へ");
                to_baseware_version(state, Some(script), config)
            }
            other => unexpected_reply(state, "BootType", other),
        },
        // BootMain + OnBoot 応答: Value→StartTalk（Req 1.4・2.1）／204→talk なし（Req 2.3）。
        // いずれも basewareversion NOTIFY を発行し BootVersion へ。
        Phase::BootMain => match outcome {
            ShioriOutcome::Value(script) => to_baseware_version(state, Some(script), config),
            ShioriOutcome::NoContent => to_baseware_version(state, None, config),
            other => unexpected_reply(state, "BootMain", other),
        },
        // BootVersion + Notified: basewareversion 完了→boot 系列完了・Steady へ（Req 1.4 完了）。
        // DD-IT-12: 追跡中の挨拶 talk（BootVersion{talk: Some}）を Steady{talk} へそのまま引き継ぐ
        // （204 経路は None のまま＝従来意味論を保存）。以後の Tick は Steady{Some} 由来で
        // NOTIFY・Ref3=0・Status: talking を発行し、挨拶 TalkDone は slot と照合される。
        Phase::BootVersion { .. } => match outcome {
            ShioriOutcome::Notified => {
                let mut state = state;
                tracing::info!(target: "kanade", event = "boot_complete", "basewareversion 完了——boot 系列完了・定常運転へ");
                if let Phase::BootVersion { talk } = state.phase {
                    state.phase = Phase::Steady { talk };
                }
                (state, Vec::new())
            }
            other => unexpected_reply(state, "BootVersion", other),
        },
        // Idle（応答待ちでない）への ShioriReply は構造上発生しない（防御アーム）。
        _ => {
            tracing::warn!(target: "kanade", event = "boot_reply_ignored", "応答待ちでない boot Phase への SHIORI 応答を無視");
            (state, Vec::new())
        }
    }
}

/// StartTalk（Value 時のみ・一意採番）を積み、basewareversion NOTIFY を発行し BootVersion へ。
///
/// Value（`Some`）なら [`State::next_talk_id`] を採番して [`Action::StartTalk`] を先頭に積み
/// （単調増番・再利用しない・Req 2.1）、その挨拶を [`ActiveTalk`]（`origin="boot"`）として
/// `Phase::BootVersion{talk: Some(_)}` で**正規追跡する**（DD-IT-12）。204（`None`）なら
/// StartTalk なし・`BootVersion{talk: None}`（Req 2.3・従来意味論を保存）。boot の talk は
/// fire-and-forget（完了を待たず basewareversion へ進む）だが、追跡 slot は boot 完了時に
/// `Steady{talk}` へ引き継がれる（`Steady{talk: None}` へ丸めない）。
///
/// `baseware_version` の `Status` は**フェーズ更新後**のスナップショットから導出する
/// （DD-IT-12・DD-IT-4「送出時点の phase」）。Value 経路は `BootVersion{Some}`→`talk_active=true`
/// ＝`Status: talking`、204 経路は `BootVersion{None}`→非アクティブ＝Status 行なし。
fn to_baseware_version(
    mut state: State,
    script: Option<String>,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    let mut actions: Vec<Action> = Vec::new();
    let talk = if let Some(script) = script {
        let talk_id = TalkId(state.next_talk_id);
        state.next_talk_id += 1;
        tracing::info!(target: "kanade", event = "boot_talk", talk_id = talk_id.0, "起動グリーティングを再生起動");
        actions.push(Action::StartTalk(StartTalk { talk_id, script }));
        Some(ActiveTalk {
            talk_id,
            origin: "boot",
        })
    } else {
        None
    };
    // フェーズを先に確定してからスナップショットを撮る（DD-IT-4: 送出時点の phase）。
    state.phase = Phase::BootVersion { talk };
    actions.push(Action::ShioriRequest(events::baseware_version(
        config,
        &snapshot_of(&state.phase),
    )));
    (state, actions)
}

/// boot 中の CloseRequest: `pending_close` に記録するのみ（boot 継続・現 Phase 維持）。
fn record_pending_close(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    tracing::info!(target: "kanade", event = "boot_pending_close", reason = reason.as_ref_str(), "boot 中の close 指示——保留記録（boot は継続・握手は Steady 遷移時）");
    state.pending_close = Some(reason);
    (state, Vec::new())
}

/// boot 進行で想定外の応答（GET 待ちに Notified 等）を受けた場合の防御アーム（warn!＋維持）。
fn unexpected_reply(state: State, phase: &str, _outcome: ShioriOutcome) -> (State, Vec<Action>) {
    tracing::warn!(target: "kanade", event = "boot_unexpected_reply", phase = phase, "boot 待ち点で想定外の SHIORI 応答——現 Phase 維持で継続");
    (state, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{step, ActiveTalk};

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    /// boot 進行は `State::initial()` を起点に `step()` 経由で駆動する（統合貫通テスト）。
    fn initial() -> State {
        State::initial()
    }

    /// Action が期待の GET（id・references が events:: と一致）であることを検証する。
    fn assert_get(action: &Action, expected: &crate::msg::ShioriCall) {
        match (action, expected) {
            (
                Action::ShioriRequest(crate::msg::ShioriCall::Get { id, references, .. }),
                crate::msg::ShioriCall::Get {
                    id: eid,
                    references: erefs,
                    ..
                },
            ) => {
                assert_eq!(id, eid, "GET id 不一致");
                assert_eq!(references, erefs, "GET references 不一致");
            }
            _ => panic!("expected ShioriRequest(Get) matching events output"),
        }
    }

    /// Action が期待の NOTIFY（id・references が events:: と一致）であることを検証する。
    fn assert_notify(action: &Action, expected: &crate::msg::ShioriCall) {
        match (action, expected) {
            (
                Action::ShioriRequest(crate::msg::ShioriCall::Notify { id, references, .. }),
                crate::msg::ShioriCall::Notify {
                    id: eid,
                    references: erefs,
                    ..
                },
            ) => {
                assert_eq!(id, eid, "NOTIFY id 不一致");
                assert_eq!(references, erefs, "NOTIFY references 不一致");
            }
            _ => panic!("expected ShioriRequest(Notify) matching events output"),
        }
    }

    // --- Full happy path: Idle→…→Steady（各段の Phase＋Action を厳密検証） ---

    #[test]
    fn full_boot_sequence_carries_greeting_talk_into_steady() {
        let cfg = config();

        // 1. Idle + Boot → OnInitialize NOTIFY / BootInit（Req 1.1）。
        let (s, actions) = step(initial(), Input::Boot, &cfg);
        assert!(matches!(s.phase, Phase::BootInit));
        assert_eq!(actions.len(), 1);
        assert_notify(&actions[0], &events::on_initialize(&ExecutionSnapshot::INACTIVE));

        // 2. BootInit + Notified → OnFirstBoot GET / BootType（Req 1.2）。
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
            &cfg,
        );
        assert!(matches!(s.phase, Phase::BootType));
        assert_eq!(actions.len(), 1);
        assert_get(&actions[0], &events::on_first_boot(&ExecutionSnapshot::INACTIVE));

        // 3. BootType + NoContent(204) → OnBoot GET / BootMain（Req 1.3）。
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
            },
            &cfg,
        );
        assert!(matches!(s.phase, Phase::BootMain));
        assert_eq!(actions.len(), 1);
        assert_get(&actions[0], &events::on_boot(&cfg, &ExecutionSnapshot::INACTIVE));

        // 4. BootMain + Value("greeting") → StartTalk(id=1) + basewareversion NOTIFY /
        //    BootVersion{talk: Some(挨拶)}（DD-IT-12: 挨拶を正規追跡）。
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("greeting".to_string()),
            },
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
            Action::StartTalk(StartTalk { talk_id, script }) => {
                assert_eq!(*talk_id, TalkId(1), "初回 boot talk_id は 1");
                assert_eq!(script, "greeting");
            }
            _ => panic!("expected StartTalk first"),
        }
        // baseware_version の id/references（Status の検証は専用檻
        // `baseware_version_status_reflects_greeting_tracking` が担う）。
        assert_notify(&actions[1], &events::baseware_version(&cfg, &ExecutionSnapshot::INACTIVE));
        // 採番カウンタが進む。
        assert_eq!(s.next_talk_id, 2);

        // 5. BootVersion{Some} + Notified → Steady{talk: Some(挨拶)}（DD-IT-12: 挨拶を引き継ぐ）。
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
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
        assert!(actions.is_empty(), "boot 完了は副作用なし");
        assert!(s.pending_close.is_none());
    }

    // --- Fallthrough cutoff: BootType + Value → OnBoot SKIPPED ---

    #[test]
    fn boot_type_value_skips_onboot_and_starts_talk() {
        let cfg = config();
        // BootType へ進める（Boot→Notified）。
        let (s, _) = step(initial(), Input::Boot, &cfg);
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
            &cfg,
        );
        assert!(matches!(s.phase, Phase::BootType));

        // BootType + Value("earlygreet") → StartTalk + basewareversion NOTIFY / BootVersion。
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("earlygreet".to_string()),
            },
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
            Action::StartTalk(StartTalk { talk_id, script }) => {
                assert_eq!(*talk_id, TalkId(1));
                assert_eq!(script, "earlygreet");
            }
            _ => panic!("expected StartTalk first"),
        }
        assert_notify(&actions[1], &events::baseware_version(&cfg, &ExecutionSnapshot::INACTIVE));

        // OnBoot GET が一切発行されていないことを確認（フォールスルー打ち切り）。
        for a in &actions {
            if let Action::ShioriRequest(crate::msg::ShioriCall::Get { id, .. }) = a {
                assert_ne!(*id, "OnBoot", "OnBoot はスキップされるべき");
            }
        }
    }

    // --- BootMain + NoContent → no StartTalk（Req 2.3） ---

    #[test]
    fn boot_main_no_content_emits_no_talk() {
        let cfg = config();
        let s = State {
            phase: Phase::BootMain,
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
        };
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
            },
            &cfg,
        );
        assert!(
            matches!(s.phase, Phase::BootVersion { talk: None }),
            "204 は挨拶を追跡せず BootVersion{{talk: None}}（Req 2.3）"
        );
        // basewareversion NOTIFY のみ（StartTalk なし）。
        assert_eq!(actions.len(), 1);
        assert_notify(&actions[0], &events::baseware_version(&cfg, &ExecutionSnapshot::INACTIVE));
        assert!(
            !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
            "204 は talk 起動しない（Req 2.3）"
        );
        // 採番カウンタは動かない。
        assert_eq!(s.next_talk_id, 1);
    }

    // --- talk_id uniqueness/monotonicity: 連続 Value 採番は単調増番 ---

    #[test]
    fn boot_talk_ids_are_unique_and_monotonic() {
        let cfg = config();
        // BootMain + Value → id=1。
        let s1 = State {
            phase: Phase::BootMain,
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
        };
        let (s1, actions1) = step(
            s1,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("a".to_string()),
            },
            &cfg,
        );
        let id1 = match &actions1[0] {
            Action::StartTalk(StartTalk { talk_id, .. }) => *talk_id,
            _ => panic!("expected StartTalk"),
        };
        assert_eq!(id1, TalkId(1));
        assert_eq!(s1.next_talk_id, 2);

        // 引き継いだカウンタ（2）から次の Value → id=2（別 run 想定・単調増番）。
        let s2 = State {
            phase: Phase::BootMain,
            last_now: None,
            next_talk_id: s1.next_talk_id,
            pending_close: None,
        };
        let (s2, actions2) = step(
            s2,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("b".to_string()),
            },
            &cfg,
        );
        let id2 = match &actions2[0] {
            Action::StartTalk(StartTalk { talk_id, .. }) => *talk_id,
            _ => panic!("expected StartTalk"),
        };
        assert_eq!(id2, TalkId(2), "id は単調増番で再利用しない");
        assert_eq!(s2.next_talk_id, 3);
        assert_ne!(id1, id2);
    }

    // --- CloseRequest during boot → pending_close recorded, phase unchanged, no actions ---

    #[test]
    fn close_request_during_boot_records_pending_only() {
        let cfg = config();
        // 各 boot Phase で保留記録のみ・Phase 不変・Action なしを確認。
        for phase in [
            Phase::BootInit,
            Phase::BootType,
            Phase::BootMain,
            Phase::BootVersion { talk: None },
        ] {
            let s = State {
                phase,
                last_now: None,
                next_talk_id: 1,
                pending_close: None,
            };
            let phase_before = std::mem::discriminant(&s.phase);
            let (s, actions) = step(
                s,
                Input::CloseRequest {
                    reason: CloseReason::User,
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
                matches!(s.pending_close, Some(CloseReason::User)),
                "pending_close に理由が記録される"
            );
        }
    }

    // --- pending_close survives boot completion to Steady{talk: None} ---

    #[test]
    fn pending_close_survives_boot_completion() {
        let cfg = config();
        // Boot 開始→BootInit で close 保留→そのまま boot 完走。
        let (s, _) = step(initial(), Input::Boot, &cfg);
        let (s, _) = step(
            s,
            Input::CloseRequest {
                reason: CloseReason::System,
            },
            &cfg,
        );
        assert!(matches!(s.phase, Phase::BootInit));
        assert!(matches!(s.pending_close, Some(CloseReason::System)));
        // BootInit→BootType→BootMain→BootVersion→Steady を進める。
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
            &cfg,
        );
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
            },
            &cfg,
        );
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
            },
            &cfg,
        );
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
            },
            &cfg,
        );
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
                    if *id == "basewareversion" =>
                {
                    Some(status.render())
                }
                _ => None,
            })
        }

        // Value（挨拶）経路: BootMain + Value → Status: talking。
        let greeting = State {
            phase: Phase::BootMain,
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
        };
        let (_, actions) = step(
            greeting,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("hi".to_string()),
            },
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
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
        };
        let (_, actions) = step(
            no_greeting,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
            },
            &cfg,
        );
        assert_eq!(
            baseware_status(&actions),
            Some(None),
            "204 経路の basewareversion は Status 行を出さない（None）"
        );
    }
}
