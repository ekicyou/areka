//! boot 系列の Phase 分岐（Req 1・状態機械図の Idle→…→Steady）。
//!
//! 本モジュールは boot 各待ち点（[`Phase::Idle`] からの起動・BootInit / BootType /
//! BootMain / BootVersion の応答進行）の遷移本体を担う。タスク 2.3 が本体を実装する。
//! 2.1 では骨格（呼出面）のみを用意し、[`crate::schedule::mod`] の `step` から
//! フェーズ分岐として呼び出せるようにする。

use super::{Action, ActiveTalk, Input, Phase, State, events, resources, snapshot_of};
use crate::msg::{CloseReason, KanadeConfig, ShioriOutcome};
use crate::status::ExecutionSnapshot;
use crate::talk::{StartTalk, TalkId};
use resources::ResourceOutcome;

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
        vec![Action::ShioriRequest(events::on_initialize(
            &ExecutionSnapshot::INACTIVE,
        ))],
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
        Phase::BootPrefetch => on_prefetch_reply(state, outcome, config),
        // BootType: 204→OnBoot GET（BootMain へ・Req 1.3）。Value→OnBoot をスキップし
        // StartTalk＋basewareversion（正典フォールスルー打ち切り・Req 2.1）。
        Phase::BootType => match outcome {
            ShioriOutcome::NoContent => {
                let mut state = state;
                state.phase = Phase::BootMain;
                (
                    state,
                    vec![Action::ShioriRequest(events::on_boot(
                        config,
                        &ExecutionSnapshot::INACTIVE,
                    ))],
                )
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
fn on_prefetch_reply(
    mut state: State,
    outcome: ShioriOutcome,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
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
    // sink 呼出は request 送出**より先**に実行される（Action 順・シェルは順次実行）。
    let sink = Action::ResourceOutcome {
        id: "username",
        outcome: resource_outcome,
    };
    // 初回ゲート（design C9・3.1/3.3）: prefetch 段自体は不変（3.5）——照会応答後の分岐のみ。
    if config.first_boot {
        // 初回起動（記録なし）: 従来どおり OnFirstBoot GET（Ref0=vanish_count・4.1）→ BootType。
        // OnFirstBoot 204 は BootType アームで OnBoot へフォールスルーする（3.2・不変）。
        state.phase = Phase::BootType;
        (
            state,
            vec![
                sink,
                Action::ShioriRequest(events::on_first_boot(
                    &ExecutionSnapshot::INACTIVE,
                    config.vanish_count,
                )),
            ],
        )
    } else {
        // 2 回目以降（記録あり）: OnFirstBoot と BootType を飛ばし OnBoot（BootMain）直行（3.3）。
        tracing::info!(target: "kanade", event = "boot_gate", "boot_gate skip_first_boot");
        state.phase = Phase::BootMain;
        (
            state,
            vec![
                sink,
                Action::ShioriRequest(events::on_boot(config, &ExecutionSnapshot::INACTIVE)),
            ],
        )
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
        // Value 経路: 挨拶 talk を採番し、first_boot_epilogue を末尾へ添付して起動する
        // （epilogue 空＝従来 StartTalk::new と同値・design C9 Some アーム）。
        let talk_id = TalkId(state.next_talk_id);
        state.next_talk_id += 1;
        tracing::info!(target: "kanade", event = "boot_talk", talk_id = talk_id.0, "起動グリーティングを再生起動");
        actions.push(Action::StartTalk(StartTalk {
            talk_id,
            script: script.clone(),
            epilogue: config.first_boot_epilogue.clone(),
        }));
        // script は起動値の保持（DD-10）。boot 起動 talk も steady 起動 talk と同一の追跡 slot
        // （[`ActiveTalk`]）に載るため、`OnChoiceTimeout` Ref0 の供給源は起動経路に依らず同一。
        Some(ActiveTalk {
            talk_id,
            origin: "boot",
            script,
        })
    } else if !config.first_boot_epilogue.is_empty() {
        // 204 かつ epilogue 非空（初回かつ挨拶トーク皆無・204-204）: epilogue-only StartTalk
        // （空 script・末尾 SET 1 件＝即時完走）を発行し、Some として正規追跡する（design C9 None アーム）。
        // これにより記録は挨拶 talk と同一の書込経路（cue 1 本）に載る。
        let talk_id = TalkId(state.next_talk_id);
        state.next_talk_id += 1;
        tracing::info!(target: "kanade", event = "boot_talk", talk_id = talk_id.0, "epilogue-only 起動記録トークを再生起動（挨拶トーク皆無）");
        actions.push(Action::StartTalk(StartTalk {
            talk_id,
            script: String::new(),
            epilogue: config.first_boot_epilogue.clone(),
        }));
        Some(ActiveTalk {
            talk_id,
            origin: "boot",
            // epilogue-only talk の起動 script は空——保持規律は挨拶経路と同一（DD-10）。
            script: String::new(),
        })
    } else {
        // 204 かつ epilogue 空（通常起動・既存全ケース）: StartTalk なし・BootVersion{talk: None}（不変）。
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
#[path = "boot_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "boot_sequence_tests.rs"]
mod sequence_tests;

#[cfg(test)]
#[path = "boot_reply_branch_tests.rs"]
mod reply_branch_tests;
