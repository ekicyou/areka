//! boot 系列の Phase 分岐（状態機械図の Idle→…→Steady）。
//!
//! 本モジュールは boot 各待ち点（[`Phase::Idle`] からの起動・BootInit / BootPrefetch /
//! BootType / BootMain / BootVersion の応答進行）の遷移本体を担い、運行表の `step`
//! （`schedule/mod.rs`）からフェーズ分岐として呼ばれる。

use super::{Action, ActiveTalk, Input, Phase, State, events, resources};
use crate::change::{BootOrigin, KanadeNotice, ReloadScope, ShioriMethod};
use crate::msg::{CloseReason, KanadeConfig, ShioriCall, ShioriOutcome};
use crate::talk::{ScriptOrigin, StartTalk, TalkDone, TalkMeta};
use resources::ResourceOutcome;

/// boot 系列（Idle / BootInit / BootPrefetch / BootType / BootMain / BootVersion）のフェーズ分岐。
///
/// 起動指示受領から基盤バージョン通知完了までの正典順序を状態機械として実装する
/// 各段階でスクリプト（Value）が返った場合は一意な talk_id を
/// 採番して再生起動要求（[`Action::StartTalk`]）を発行する。起動種別問い合わせ
/// （BootType）にスクリプトが返った場合は `OnBoot` を飛ばして basewareversion へ進む
/// （正典フォールスルー打ち切り）。204 は talk なし。
///
/// boot 中の [`Input::CloseRequest`] は `pending_close` として記録するのみで boot は
/// 中断しない（握手開始は steady/close 層の責務）。
pub(crate) fn step(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match input {
        // 非 Idle は `mod.rs` 側で warn! して捨て済み。
        Input::Boot => boot_start(state, config),
        Input::ShioriReply { outcome, .. } => on_reply(state, outcome, config),
        Input::CloseRequest { reason } => record_pending_close(state, reason),
        // 突合と非 quit の判定は `mod.rs` で済んでいる。
        Input::TalkDone(done) if matches!(state.phase, Phase::BootVersion { talk: Some(_) }) => {
            on_talk_done(state, done)
        }
        _ => {
            tracing::warn!(target: "kanade", event = "boot_input_ignored", "boot 系列に無関係な入力を無視");
            (state, Vec::new())
        }
    }
}

/// Idle + Boot: `OnInitialize` NOTIFY を発行し BootInit へ。
///
/// SHIORI だけの読み直し（[`BootOrigin::Reloaded`] の [`ReloadScope::Shiori`]）では Ref0＝`reload`
/// を載せる（ukadoc `OnInitialize`「リロード時にreload」）。それ以外は
/// References なし（ゴーストの読み直しも含む）。
fn boot_start(mut state: State, config: &KanadeConfig) -> (State, Vec<Action>) {
    tracing::info!(target: "kanade", event = "boot_start", "起動指示を受領——OnInitialize NOTIFY を発行");
    state.phase = Phase::BootInit;
    let snapshot = &state.snapshot_without_talk();
    let call = if matches!(
        config.boot_origin,
        BootOrigin::Reloaded {
            scope: ReloadScope::Shiori
        }
    ) {
        events::raise(
            "OnInitialize",
            vec!["reload".to_string()],
            ShioriMethod::Notify,
            snapshot,
        )
    } else {
        events::on_initialize(snapshot)
    };
    (state, vec![Action::ShioriRequest(call)])
}

/// boot 各待ち点の応答進行（正典順序の状態機械）。
fn on_reply(state: State, outcome: ShioriOutcome, config: &KanadeConfig) -> (State, Vec<Action>) {
    match state.phase {
        Phase::BootInit => match outcome {
            ShioriOutcome::Notified => {
                let mut state = state;
                state.phase = Phase::BootPrefetch;
                let call = resources::resource_get("username", &state.snapshot_without_talk());
                (state, vec![Action::ShioriRequest(call)])
            }
            other => unexpected_reply(state, "BootInit", other),
        },
        // BootPrefetch + 応答: username 照会結果を写像・sink へ渡し（同期）・完了固定ログを出し、
        // 起動の根（boot_root）を発行する。失敗でも boot を殺さず続行する。
        Phase::BootPrefetch => on_prefetch_reply(state, outcome, config),
        Phase::BootType => match outcome {
            // ネットワーク更新の起動の知らせ（根が OnFirstBoot でない）は 204 でも OnBoot へ続けない。
            ShioriOutcome::NoContent
                if !config.first_boot
                    && matches!(config.boot_origin, BootOrigin::Updated { .. }) =>
            {
                tracing::info!(target: "kanade", event = "boot_update_no_content", "ネットワーク更新の起動の知らせが 204——OnBoot へ進まず basewareversion へ");
                to_baseware_version(state, None, config)
            }
            ShioriOutcome::NoContent => {
                let mut state = state;
                state.phase = Phase::BootMain;
                let call = events::on_boot(config, &state.snapshot_without_talk());
                (state, vec![Action::ShioriRequest(call)])
            }
            ShioriOutcome::Value(script, _) => {
                tracing::info!(target: "kanade", event = "boot_type_script", "起動種別にスクリプト——OnBoot をスキップし basewareversion へ");
                to_baseware_version(state, Some(script), config)
            }
            other => unexpected_reply(state, "BootType", other),
        },
        Phase::BootMain => match outcome {
            ShioriOutcome::Value(script, _) => to_baseware_version(state, Some(script), config),
            ShioriOutcome::NoContent => to_baseware_version(state, None, config),
            other => unexpected_reply(state, "BootMain", other),
        },
        // 追跡中の挨拶 talk（BootVersion{talk: Some}）を Steady{talk} へそのまま引き継ぐ
        // （204 経路は None のまま＝従来意味論を保存）。以後の Tick は Steady{Some} 由来で
        // NOTIFY・Ref3=0・Status: talking を発行し、挨拶 TalkDone は slot と照合される。
        Phase::BootVersion { .. } => match outcome {
            ShioriOutcome::Notified => {
                let mut state = state;
                tracing::info!(target: "kanade", event = "boot_complete", "basewareversion 完了——boot 系列完了・定常運転へ");
                if let Phase::BootVersion { talk } = state.phase {
                    state.phase = Phase::Steady { talk };
                }
                (state, vec![Action::Notice(KanadeNotice::Steady)])
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
/// 起動の根（[`boot_root`]）を発行する（根があれば BootType・無ければ OnBoot で BootMain）。
/// 読み直しの起動（[`BootOrigin::Reloaded`]）は根も OnBoot も送らず basewareversion へ直行する。
///
/// 写像: 200 Value→[`ResourceOutcome::Value`]／204→[`ResourceOutcome::NoContent`]／失敗（タイムアウト・
/// IPC 断）→[`ResourceOutcome::Failed`]。失敗時は `warn!` を残しつつ **boot を殺さず**続行する（起動を
/// 殺さない）。GET 応答が構造上あり得ない Notified/Unloaded で来た場合も防御的に `Failed` 扱いとし
/// 起動を続行する。
///
/// リソース照会は talk を生成しない（Invariant）——結果は [`Action::ResourceOutcome`] で sink へ渡すのみで
/// StartTalk へは流さない。返す Action 列は `[ResourceOutcome, ShioriRequest(根または OnBoot)]` の順
/// （読み直しでは `[ResourceOutcome, (初回の記録だけの StartTalk)?, ShioriRequest(basewareversion の
/// NOTIFY)]`）であり、シェルは sink 呼出（同期・返るまで待つ）を先に実行してから次の GET を送出する。
///
/// # 完了固定ログ（grep 証跡）
/// 経路によらず `info!(target: "areka_kanade::resource", id = "username", outcome = <value|no_content|
/// failed>, "shiori resource prefetch done")` を**ちょうど 1 回**発行する（実照会経路の実機 signoff 証跡）。
fn on_prefetch_reply(
    mut state: State,
    outcome: ShioriOutcome,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    // 写像はメニューの照会と同じ 1 つ（想定外の答えの warn もそちら）。boot は失敗の warn と完了の記録だけ持つ。
    let resource_outcome = resources::outcome_of("username", outcome);
    let label = match &resource_outcome {
        ResourceOutcome::Value(_) => "value",
        ResourceOutcome::NoContent => "no_content",
        ResourceOutcome::Failed(reason) => {
            tracing::warn!(
                target: "areka_kanade::resource",
                id = "username",
                error = %reason,
                "username リソース照会が失敗——warn＋Failed を sink へ渡し boot 続行（起動を殺さない・R4.1）"
            );
            "failed"
        }
    };
    // grep 証跡: prefetch 完了固定ログ（target・fields・message 固定・必ず 1 回）。
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
    // 読み直しで起こし直した → 起動の根も OnBoot も見ずに basewareversion へ直行する（`first_boot`
    // も見ない＝OnFirstBoot も送らない）。初回の記録の epilogue は
    // `to_baseware_version` の既存の腕に任せる（台詞は出ず記録だけ）。
    if matches!(config.boot_origin, BootOrigin::Reloaded { .. }) {
        tracing::info!(target: "kanade", event = "boot_reload_silent", "読み直しの起動——起動の知らせを送らず basewareversion へ");
        let (state, mut actions) = to_baseware_version(state, None, config);
        actions.insert(0, sink);
        return (state, actions);
    }
    // 起動の根（username 照会の応答後に 1 つだけ選ぶ・照会の段は不変）。根があれば BootType で
    // その応答を待ち、既存の腕（204 → OnBoot・台本 → OnBoot を飛ばす）へ合流する。根が無ければ
    // OnBoot（BootMain）へ直行する（boot_gate の記録の枝）。
    if let Some(root) = boot_root(&state, config) {
        state.phase = Phase::BootType;
        (state, vec![sink, Action::ShioriRequest(root)])
    } else {
        tracing::info!(target: "kanade", event = "boot_gate", "boot_gate skip_first_boot");
        state.phase = Phase::BootMain;
        let call = events::on_boot(config, &state.snapshot_without_talk());
        (state, vec![sink, Action::ShioriRequest(call)])
    }
}

/// 起動の根の表（ukadoc の「204 なら続けて」の木の根を 1 つ選ぶ）。
///
/// 起動記録なし（`first_boot`）→ `OnFirstBoot`（由来を問わず最優先）。切替で来た
/// （[`BootOrigin::ChangedFrom`]）→ `OnGhostChanged`。ネットワーク更新で読み直した
/// （[`BootOrigin::Updated`]）→ 渡された名前と Reference の GET（204 でも `OnBoot` へ続けない）。
/// それ以外（`Plain`・`Halted`）→ 根なし
/// （`OnBoot` だけ。`Halted` の Ref6/7 は [`events::on_boot`] が載せる）。範囲外の根は
/// [`BootOrigin`] に値を足し、この表に行を足すだけで入る。スナップショットは会話なしの作り方
/// （online と balloon だけ写しから残す）。
fn boot_root(state: &State, config: &KanadeConfig) -> Option<ShioriCall> {
    let snapshot = &state.snapshot_without_talk();
    if config.first_boot {
        return Some(events::on_first_boot(snapshot, config.vanish_count));
    }
    match &config.boot_origin {
        BootOrigin::ChangedFrom(from) => Some(events::on_ghost_changed(
            from,
            &config.shell_folder,
            snapshot,
        )),
        // ネットワーク更新で読み直した → 渡された名前と Reference の GET（`OnGhostChanged` と同じく
        // 根は許可表を通さない）。
        BootOrigin::Updated { id, references } => {
            tracing::info!(target: "kanade", event = "boot_update_root", id = *id, "起動の根にネットワーク更新の起動の知らせを選んだ");
            Some(events::raise(
                id,
                references.clone(),
                ShioriMethod::Get,
                snapshot,
            ))
        }
        BootOrigin::Plain | BootOrigin::Halted { .. } => None,
        // 読み直しは `on_prefetch_reply` が手前で basewareversion へ直行させる（網羅のため・到達しない）。
        BootOrigin::Reloaded { .. } => None,
    }
}

/// StartTalk（Value 時のみ・一意採番）を積み、basewareversion NOTIFY を発行し BootVersion へ。
///
/// Value（`Some`）なら [`State::next_talk_id`] を採番して [`Action::StartTalk`] を先頭に積み
/// （単調増番・再利用しない）、その挨拶を [`ActiveTalk`]（`origin="boot"`）として
/// `Phase::BootVersion{talk: Some(_)}` で**正規追跡する**。204（`None`）なら
/// StartTalk なし・`BootVersion{talk: None}`（従来意味論を保存）。boot の talk は
/// fire-and-forget（完了を待たず basewareversion へ進む）だが、追跡 slot は boot 完了時に
/// `Steady{talk}` へ引き継がれる（`Steady{talk: None}` へ丸めない）。
///
/// `baseware_version` の `Status` は**フェーズ更新後**のスナップショットから導出する
/// （送出時点の phase）。Value 経路は `BootVersion{Some}`→`talk_active=true`
/// ＝`Status: talking`、204 経路は `BootVersion{None}`→非アクティブ＝Status 行なし。
fn to_baseware_version(
    mut state: State,
    script: Option<String>,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    let mut actions: Vec<Action> = Vec::new();
    let talk = if let Some(script) = script {
        // Value 経路: 挨拶 talk を採番し、first_boot_epilogue を末尾へ添付して起動する
        // （epilogue 空＝従来 StartTalk::new と同値 Some アーム）。
        let talk_id = super::take_talk_id(&mut state);
        tracing::info!(target: "kanade", event = "boot_talk", talk_id = talk_id.0, "起動グリーティングを再生起動");
        actions.push(Action::StartTalk(StartTalk {
            talk_id,
            script: script.clone(),
            epilogue: config.first_boot_epilogue.clone(),
            origin: ScriptOrigin::GHOST,
            strict: false,
            meta: TalkMeta::default(),
        }));
        // script は起動値の保持。boot 起動 talk も steady 起動 talk と同一の追跡 slot
        // （[`ActiveTalk`]）に載るため、`OnChoiceTimeout` Ref0 の供給源は起動経路に依らず同一。
        Some(ActiveTalk {
            talk_id,
            origin: super::sstp::BOOT_GREETING_ORIGIN,
            script,
        })
    } else if !config.first_boot_epilogue.is_empty() {
        // 204 かつ epilogue 非空（初回かつ挨拶トーク皆無・204-204）: epilogue-only StartTalk
        // （空 script・末尾 SET 1 件＝即時完走）を発行し、Some として正規追跡する。
        // これにより記録は挨拶 talk と同一の書込経路（cue 1 本）に載る。
        let talk_id = super::take_talk_id(&mut state);
        tracing::info!(target: "kanade", event = "boot_talk", talk_id = talk_id.0, "epilogue-only 起動記録トークを再生起動（挨拶トーク皆無）");
        actions.push(Action::StartTalk(StartTalk {
            talk_id,
            script: String::new(),
            epilogue: config.first_boot_epilogue.clone(),
            origin: ScriptOrigin::GHOST,
            strict: false,
            meta: TalkMeta::default(),
        }));
        Some(ActiveTalk {
            talk_id,
            origin: super::sstp::BOOT_GREETING_ORIGIN,
            // epilogue-only talk の起動 script は空——保持規律は挨拶経路と同一。
            script: String::new(),
        })
    } else {
        // 204 かつ epilogue 空（通常起動・既存全ケース）: StartTalk なし・BootVersion{talk: None}（不変）。
        None
    };
    // フェーズを先に確定してからスナップショットを撮る（送出時点の phase）。
    state.phase = Phase::BootVersion { talk };
    actions.push(Action::ShioriRequest(events::baseware_version(
        config,
        &state.snapshot(),
    )));
    (state, actions)
}

/// BootVersion{talk: Some} に届いた追跡中トークの完了（非 quit・突合済み）を受理する。
///
/// 呼出契約: `step` のガード腕からのみ呼ぶ（相は `BootVersion{talk: Some(_)}` で確定）。
/// 追跡を外して枠を空にし、basewareversion 応答待ちを維持する。副作用指示は返さない。
/// `pending_close` には触れない（起動中は握手を始めない・消化は Steady 遷移後の Tick）。
/// 選択肢の帳簿も掃除しない（起動中は `state.choice` が構造上 `None`）。
fn on_talk_done(mut state: State, done: TalkDone) -> (State, Vec<Action>) {
    tracing::info!(target: "kanade", event = "boot_talk_done", talk_id = done.talk_id.0, "起動挨拶 talk 完了——basewareversion 応答待ちを維持しつつ枠を空にする");
    state.phase = Phase::BootVersion { talk: None };
    (state, Vec::new())
}

/// boot 中の CloseRequest: `pending_close` に記録するのみ（boot 継続・現 Phase 維持）。
fn record_pending_close(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    tracing::info!(target: "kanade", event = "boot_pending_close", reason = reason.as_ref_str(), "boot 中の close 指示——保留記録（boot は継続・握手は Steady 遷移時）");
    state.pending_close = Some(reason);
    (state, Vec::new())
}

/// boot 進行で想定外の応答（GET 待ちに Notified 等）を受けた場合の防御アーム（warn!＋維持）。
fn unexpected_reply(state: State, phase: &str, outcome: ShioriOutcome) -> (State, Vec<Action>) {
    tracing::warn!(target: "kanade", event = "boot_unexpected_reply", phase = phase, reply = outcome.kind(), "boot 待ち点で想定外の SHIORI 応答——現 Phase 維持で継続");
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

#[cfg(test)]
#[path = "boot_root_tests.rs"]
mod root_tests;

#[cfg(test)]
#[path = "boot_reload_tests.rs"]
mod reload_tests;
