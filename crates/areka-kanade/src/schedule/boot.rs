//! boot 系列の Phase 分岐（Req 1・状態機械図の Idle→…→Steady）。
//!
//! 本モジュールは boot 各待ち点（[`Phase::Idle`] からの起動・BootInit / BootType /
//! BootMain / BootVersion の応答進行）の遷移本体を担う。タスク 2.3 が本体を実装する。
//! 2.1 では骨格（呼出面）のみを用意し、[`crate::schedule::mod`] の `step` から
//! フェーズ分岐として呼び出せるようにする。

use super::{events, resources, snapshot_of, Action, ActiveTalk, Input, Phase, State};
use crate::msg::{CloseReason, KanadeConfig, ShioriOutcome};
use resources::ResourceOutcome;
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
        // 応答進行（各待ち点で直前呼出の結果を受ける）。origin（DD-IE-3）は boot では未使用。
        Input::ShioriReply { outcome, .. } => on_reply(state, outcome, config),
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
        // BootInit + Notified: OnInitialize 完了→**username リソース照会（prefetch）GET** を発行し
        // BootPrefetch へ（R4.1・prefetch は OnInitialize 後・OnFirstBoot 前に 1 回）。既存 shiori
        // request 経路（単一 in-flight・shiori_tx 専有）をそのまま使う。応答受領後に OnFirstBoot へ進む。
        Phase::BootInit => match outcome {
            ShioriOutcome::Notified => {
                let mut state = state;
                state.phase = Phase::BootPrefetch;
                (
                    state,
                    vec![Action::ShioriRequest(resources::resource_username(
                        &ExecutionSnapshot::INACTIVE,
                    ))],
                )
            }
            other => unexpected_reply(state, "BootInit", other),
        },
        // BootPrefetch + 応答: username 照会結果を写像・sink へ渡し（同期）・完了固定ログを出し、
        // OnFirstBoot GET を発行して BootType へ進む（R4.1・R9.3）。失敗でも boot を殺さず続行する。
        Phase::BootPrefetch => on_prefetch_reply(state, outcome),
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

/// prefetch 応答（username GET）を [`ResourceOutcome`] へ写像し、sink 呼出指示＋完了固定ログを添えて
/// OnFirstBoot GET を発行し BootType へ進む（R4.1・R9.3）。
///
/// 写像: 200 Value→[`ResourceOutcome::Value`]／204→[`ResourceOutcome::NoContent`]／失敗（タイムアウト・
/// IPC 断）→[`ResourceOutcome::Failed`]。失敗時は `warn!` を残しつつ **boot を殺さず**続行する（起動を
/// 殺さない・R4.1）。GET 応答が構造上あり得ない Notified/Unloaded で来た場合も防御的に `Failed` 扱いとし
/// 起動を続行する。
///
/// リソース照会は talk を生成しない（Invariant）——結果は [`Action::ResourceOutcome`] で sink へ渡すのみで
/// StartTalk へは流さない。返す Action 列は `[ResourceOutcome, ShioriRequest(OnFirstBoot)]` の順であり、
/// シェルは sink 呼出（同期・返るまで待つ）を先に実行してから OnFirstBoot を送出する（design boot 図）。
///
/// # 完了固定ログ（R9.3 grep 証跡）
/// 経路によらず `info!(target: "areka_kanade::resource", id = "username", outcome = <value|no_content|
/// failed>, "shiori resource prefetch done")` を**ちょうど 1 回**発行する（実照会経路の実機 signoff 証跡）。
fn on_prefetch_reply(mut state: State, outcome: ShioriOutcome) -> (State, Vec<Action>) {
    let (resource_outcome, label) = match outcome {
        ShioriOutcome::Value(body) => (ResourceOutcome::Value(body), "value"),
        ShioriOutcome::NoContent => (ResourceOutcome::NoContent, "no_content"),
        ShioriOutcome::Failed(failure) => {
            tracing::warn!(
                target: "areka_kanade::resource",
                id = "username",
                error = %failure,
                "username リソース照会が失敗——warn＋Failed を sink へ渡し boot 続行（起動を殺さない・R4.1）"
            );
            (ResourceOutcome::Failed(failure.to_string()), "failed")
        }
        // GET 応答が Notified/Unloaded になることは構造上あり得ない（NOTIFY/Unload 専用の完了語彙）。
        // 防御的に Failed 扱いとし、固定ログを 1 回だけ出して boot を続行する（log-first・起動を殺さない）。
        other => {
            let kind = match other {
                ShioriOutcome::Notified => "notified",
                ShioriOutcome::Unloaded => "unloaded",
                _ => "unknown",
            };
            tracing::warn!(
                target: "areka_kanade::resource",
                id = "username",
                reply = %kind,
                "username 照会が想定外の応答（GET は Value/NoContent/Failed のみ）——防御的に Failed 扱いで続行"
            );
            (
                ResourceOutcome::Failed(format!("unexpected prefetch outcome: {kind}")),
                "failed",
            )
        }
    };
    // R9.3 grep 証跡: prefetch 完了固定ログ（target・fields・message 固定・必ず 1 回・研究 §12-10）。
    tracing::info!(
        target: "areka_kanade::resource",
        id = "username",
        outcome = label,
        "shiori resource prefetch done"
    );
    state.phase = Phase::BootType;
    (
        state,
        vec![
            // sink 呼出は OnFirstBoot 送出**より先**に実行される（Action 順・シェルは順次実行）。
            Action::ResourceOutcome {
                id: "username",
                outcome: resource_outcome,
            },
            // vanish_count は Task 5.3 で `config.vanish_count` を注入する。本 gate では
            // 従来挙動（Ref0="0"）を保存するため literal 0 を渡す。
            Action::ShioriRequest(events::on_first_boot(&ExecutionSnapshot::INACTIVE, 0)),
        ],
    )
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
        actions.push(Action::StartTalk(StartTalk::new(talk_id, script)));
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
        assert_get(&actions[0], &resources::resource_username(&ExecutionSnapshot::INACTIVE));

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
        assert!(matches!(actions[0], Action::ResourceOutcome { .. }), "sink 先行");
        assert_get(&actions[1], &events::on_first_boot(&ExecutionSnapshot::INACTIVE, 0));

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
        assert_get(&actions[0], &events::on_boot(&cfg, &ExecutionSnapshot::INACTIVE));

        // 4. BootMain + Value("greeting") → StartTalk(id=1) + basewareversion NOTIFY /
        //    BootVersion{talk: Some(挨拶)}（DD-IT-12: 挨拶を正規追跡）。
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("greeting".to_string()),
                origin: "test",
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
            Action::StartTalk(StartTalk { talk_id, script, .. }) => {
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
        assert!(actions.is_empty(), "boot 完了は副作用なし");
        assert!(s.pending_close.is_none());
    }

    // --- Fallthrough cutoff: BootType + Value → OnBoot SKIPPED ---

    #[test]
    fn boot_type_value_skips_onboot_and_starts_talk() {
        let cfg = config();
        // BootType へ進める（Boot→Notified→prefetch 204）。prefetch 段が OnInitialize と OnFirstBoot の
        // 間に挟まるため、username 照会応答（204）を 1 段挟んでから BootType へ到達する。
        let (s, _) = step(initial(), Input::Boot, &cfg);
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            &cfg,
        ); // BootInit→BootPrefetch（username GET）
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "test",
            },
            &cfg,
        ); // BootPrefetch→BootType（OnFirstBoot GET）
        assert!(matches!(s.phase, Phase::BootType));

        // BootType + Value("earlygreet") → StartTalk + basewareversion NOTIFY / BootVersion。
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("earlygreet".to_string()),
                origin: "test",
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
            Action::StartTalk(StartTalk { talk_id, script, .. }) => {
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
                origin: "test",
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
                origin: "test",
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
        // BootInit→BootPrefetch→BootType→BootMain→BootVersion→Steady を進める（prefetch 1 段追加）。
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            &cfg,
        ); // BootInit→BootPrefetch（username GET）
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "test",
            },
            &cfg,
        ); // BootPrefetch→BootType（OnFirstBoot GET）
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "test",
            },
            &cfg,
        ); // BootType→BootMain（OnBoot GET）
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "test",
            },
            &cfg,
        ); // BootMain→BootVersion（204・挨拶なし）
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            &cfg,
        ); // BootVersion→Steady{None}
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
                origin: "test",
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
    ///
    /// # `assert_logged` との対比
    /// `log_firing_tests` はログの**存在**を `assert_logged` で表明するが、本檻は逆に**不在**を
    /// 表明する必要があるため、捕捉列 `Vec<CapturedEvent>` を直接走査する（`target="kanade"`・ERROR・
    /// `event="unknown_talk_done"` を持つ要素が 1 件も無いこと）。相関ロジック（`current_talk_id` による
    /// slot 照合）が退行して挨拶 TalkDone が未知扱いになれば、この不在表明が失敗する。
    #[test]
    fn boot_greeting_talkdone_correlates_without_unknown_error() {
        use crate::schedule::log_capture::capture;
        use crate::talk::{TalkDone, TalkEndReason};
        use tracing::Level;

        let cfg = config();

        // boot を Idle→…→`Steady{talk: Some(挨拶, id=1)}` まで駆動する（挨拶 Value 経路・DD-IT-12）。
        let (s, _) = step(initial(), Input::Boot, &cfg); // Idle→BootInit（OnInitialize NOTIFY）
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            &cfg,
        ); // BootInit→BootType
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "test",
            },
            &cfg,
        ); // BootType→BootMain（OnBoot GET）
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("greeting".to_string()),
                origin: "test",
            },
            &cfg,
        ); // BootMain(Value)→BootVersion{talk: Some(id=1)}
        let (s, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            &cfg,
        ); // BootVersion{Some}→Steady{talk: Some(id=1)}
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
        //     log_capture の assert_logged は「存在」を表明するため、ここは捕捉列を直接走査して不在を表明する。
        let unknown_fired = events.iter().any(|e| {
            e.target == "kanade"
                && e.level == Level::ERROR
                && e.event.as_deref() == Some("unknown_talk_done")
        });
        assert!(
            !unknown_fired,
            "挨拶 TalkDone が slot と照合されれば unknown_talk_done ERROR は出ないはず: {events:#?}"
        );
    }

    // ========================================================================
    // タスク 6.2: username リソース照会 prefetch（OnInitialize 後・OnFirstBoot 前・R4.1/R9.3）
    // ========================================================================

    use crate::msg::ShioriFailure;
    use resources::ResourceOutcome;

    /// Idle→Boot→(OnInitialize Notified) まで駆動し `BootPrefetch`（username GET 発行済み）へ到達させる。
    fn drive_to_prefetch(cfg: &KanadeConfig) -> State {
        let (s, _) = step(initial(), Input::Boot, cfg); // Idle→BootInit（OnInitialize NOTIFY）
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            cfg,
        ); // BootInit→BootPrefetch（username GET 発行）
        assert!(
            matches!(s.phase, Phase::BootPrefetch),
            "OnInitialize 完了後は BootPrefetch（username 照会待ち）であるべき"
        );
        // BootInit の応答は username GET を 1 件だけ発行する（OnFirstBoot ではない）。
        assert_eq!(actions.len(), 1, "prefetch は GET を 1 件だけ発行する");
        assert_get(
            &actions[0],
            &resources::resource_username(&ExecutionSnapshot::INACTIVE),
        );
        s
    }

    /// Action 列から `Action::ResourceOutcome` の outcome を取り出す（無ければ panic）。
    fn resource_outcome_of(actions: &[Action]) -> &ResourceOutcome {
        actions
            .iter()
            .find_map(|a| match a {
                Action::ResourceOutcome { id, outcome } => {
                    assert_eq!(*id, "username", "リソース照会 id は username");
                    Some(outcome)
                }
                _ => None,
            })
            .expect("Action::ResourceOutcome が発行されるはず")
    }

    /// prefetch は OnInitialize（NOTIFY）の後・OnFirstBoot（GET）の前に username GET を 1 回だけ発行し、
    /// 応答受領後は OnFirstBoot GET を発行して BootType へ進む（順序の檻・R4.1）。
    #[test]
    fn prefetch_username_get_is_issued_once_between_initialize_and_firstboot() {
        let cfg = config();
        // Idle+Boot → OnInitialize NOTIFY（username GET はまだ出ない）。
        let (s, actions) = step(initial(), Input::Boot, &cfg);
        assert_notify(&actions[0], &events::on_initialize(&ExecutionSnapshot::INACTIVE));
        assert!(
            !actions.iter().any(is_username_get),
            "OnInitialize 段では username GET を発行しない"
        );

        // BootInit+Notified → username GET（OnFirstBoot はまだ出ない）。
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            &cfg,
        );
        assert!(matches!(s.phase, Phase::BootPrefetch));
        assert_eq!(
            actions.iter().filter(|a| is_username_get(a)).count(),
            1,
            "prefetch の username GET はちょうど 1 回"
        );
        assert!(
            !actions.iter().any(is_onfirstboot_get),
            "prefetch 応答前に OnFirstBoot を発行してはならない（照会後に続行）"
        );

        // BootPrefetch+NoContent → [ResourceOutcome, OnFirstBoot GET]・BootType。
        let (s, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "test",
            },
            &cfg,
        );
        assert!(matches!(s.phase, Phase::BootType), "prefetch 完了後は BootType（OnFirstBoot 待ち）");
        // sink 呼出指示（ResourceOutcome）が OnFirstBoot GET より前に積まれる（design boot 図: sink 先行）。
        assert!(
            matches!(actions[0], Action::ResourceOutcome { .. }),
            "ResourceOutcome は OnFirstBoot より前（sink 先行）"
        );
        assert_eq!(
            actions.iter().filter(|a| is_onfirstboot_get(a)).count(),
            1,
            "prefetch 応答後に OnFirstBoot GET をちょうど 1 回発行する"
        );
        // 二度目の username GET は出ない（prefetch は 1 回・後段で再照会しない）。
        assert!(
            !actions.iter().any(is_username_get),
            "prefetch は 1 回のみ——後段で username を再照会しない"
        );
    }

    /// 応答の [`ResourceOutcome`] 写像: 200 Value→Value(body)／204→NoContent／失敗→Failed（R4.1）。
    #[test]
    fn prefetch_maps_outcomes_to_resource_outcome() {
        let cfg = config();

        // 200 Value → ResourceOutcome::Value(body)。
        let (_, actions) = step(
            drive_to_prefetch(&cfg),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("bob".to_string()),
                origin: "test",
            },
            &cfg,
        );
        assert_eq!(
            resource_outcome_of(&actions),
            &ResourceOutcome::Value("bob".to_string())
        );

        // 204 → ResourceOutcome::NoContent。
        let (_, actions) = step(
            drive_to_prefetch(&cfg),
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "test",
            },
            &cfg,
        );
        assert_eq!(resource_outcome_of(&actions), &ResourceOutcome::NoContent);

        // 失敗（タイムアウト）→ ResourceOutcome::Failed（理由文字列を保持）。
        let (_, actions) = step(
            drive_to_prefetch(&cfg),
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                origin: "test",
            },
            &cfg,
        );
        match resource_outcome_of(&actions) {
            ResourceOutcome::Failed(reason) => {
                assert!(reason.contains("timeout"), "Failed の理由に失敗語彙が載る: {reason}");
            }
            other => panic!("失敗応答は ResourceOutcome::Failed へ写像されるべき: {other:?}"),
        }
    }

    /// 照会失敗（タイムアウト/IPC 断）でも boot は殺さず OnFirstBoot へ続行する（起動を殺さない・R4.1）。
    /// `step()`（mod.rs 横断アーム込み）を通し、BootPrefetch の Failed が Unloading{Fault} へ**倒れない**
    /// ことを固定する（横断 Failed→Fault 経路が prefetch では迂回される檻）。
    #[test]
    fn prefetch_failure_continues_boot_not_fault() {
        let cfg = config();
        let (s, actions) = step(
            drive_to_prefetch(&cfg),
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string())),
                origin: "test",
            },
            &cfg,
        );
        // 終了系列（Unloading/Stopped）へ倒れず、OnFirstBoot 待ち（BootType）へ続行する。
        assert!(
            matches!(s.phase, Phase::BootType),
            "prefetch 失敗は Unloading{{Fault}} へ倒さず OnFirstBoot へ続行する（起動を殺さない・R4.1）"
        );
        assert_eq!(
            resource_outcome_of(&actions),
            &ResourceOutcome::Failed("shiori ipc failure: pipe closed".to_string())
        );
        // OnFirstBoot GET が発行され boot が前進する（Unload ではない）。
        assert!(
            actions.iter().any(is_onfirstboot_get),
            "prefetch 失敗後も OnFirstBoot を発行して boot を継続する"
        );
        assert!(
            !actions.iter().any(|a| matches!(a, Action::ShioriUnload)),
            "prefetch 失敗で Unload（終了系列）を起こしてはならない"
        );
    }

    /// リソース照会は talk を生成しない（Invariant）: Value 応答でも StartTalk を発行しない。
    #[test]
    fn prefetch_value_does_not_produce_a_talk() {
        let cfg = config();
        let (_, actions) = step(
            drive_to_prefetch(&cfg),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value(r"\0greeting\e".to_string()),
                origin: "test",
            },
            &cfg,
        );
        assert!(
            !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
            "リソース照会 Value は StartTalk を生成しない（sink へ渡すのみ・Invariant）"
        );
    }

    /// 完了固定ログ（R9.3 grep 証跡）: 経路（value/no_content/failed）ごとに `info!` が
    /// `target="areka_kanade::resource"`・`outcome=<label>`・message 固定で**ちょうど 1 回**出る。
    #[test]
    fn prefetch_emits_fixed_completion_log_exactly_once() {
        use crate::schedule::log_capture::{assert_resource_prefetch_logged_once, capture};

        let cases: [(ShioriOutcome, &str); 3] = [
            (ShioriOutcome::Value("bob".to_string()), "value"),
            (ShioriOutcome::NoContent, "no_content"),
            (
                ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                "failed",
            ),
        ];
        for (outcome, label) in cases {
            let cfg = config();
            let s = drive_to_prefetch(&cfg);
            let events = capture(|| {
                let _ = step(
                    s,
                    Input::ShioriReply {
                        outcome,
                        origin: "test",
                    },
                    &cfg,
                );
            });
            assert_resource_prefetch_logged_once(&events, label);
        }
    }

    /// Action が username リソース GET（id="username"）か判定する。
    fn is_username_get(action: &Action) -> bool {
        matches!(
            action,
            Action::ShioriRequest(crate::msg::ShioriCall::Get { id, .. }) if *id == "username"
        )
    }

    /// Action が OnFirstBoot GET か判定する。
    fn is_onfirstboot_get(action: &Action) -> bool {
        matches!(
            action,
            Action::ShioriRequest(crate::msg::ShioriCall::Get { id, .. }) if *id == "OnFirstBoot"
        )
    }
}
