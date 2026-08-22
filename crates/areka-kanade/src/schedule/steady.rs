//! 定常運転の Phase 分岐（Req 2・3・pump ゲート・talk 調停・保留 close）。
//!
//! 本モジュールは [`Phase::Steady`] における Tick pump（OnSecondChange GET/NOTIFY の
//! 使い分け）・talk 調停（Value→StartTalk）・boot 中／active talk 中に受領した close
//! 指示の保留処理を担う。加えて選択系の受領（[`on_choice_waiting`] の選択待ち帳簿確立）も
//! 本層に置く——選択待ちは active talk に紐づく状態であり、受理条件が Steady に閉じるため
//! （設計 C4）。
//!
//! # pump ゲート（Req 3.1・3.4・DD-6）
//! 定常運転でのみ毎秒 pump を発行する（boot 中・close 握手中以降・終了系列では発行しない
//! ——ゲートは Steady に閉じている）。再生中でないとき（`talk: None`）は問い合わせ（GET・
//! Ref3=1）、再生中（`talk: Some`）は NOTIFY（Ref3=0）で発行し分ける。NOTIFY 応答は
//! 構造的に破棄されるため、active talk 中に Value が届く重複調停を**発生源から断つ**
//! （DD-6）。
//!
//! # 保留 close の消化（System Flows 補足遷移）
//! boot 中・active talk 中に受領した close は `pending_close` に記録され（本層では作らない・
//! boot.rs／CloseRequest アームが記録する）、次の消化点で握手を開始する:
//! - `Steady{talk: None}` の次 Tick（Steady 遷移直後の握手開始）
//! - active talk の `TalkDone{reason: Ended | Interrupted}` 受領時
//!
//! 握手開始＝`OnClose` GET 発行＋`ClosePending` への遷移（ClosePending 以降は close.rs＝
//! タスク 2.5 の責務）。

use super::choice::{CascadePlan, choice_deadline, plan_cascade};
use super::{
    Action, ActiveTalk, CascadeNext, ChoicePhase, ChoiceState, Input, Phase, State, events,
};
use crate::msg::{
    ChoiceInput, CloseReason, KanadeConfig, MonotonicMs, MouseEventKind, MouseInput, ShioriCall,
    ShioriOutcome,
};
use crate::status::ExecutionSnapshot;
use crate::talk::{StartTalk, TalkDone, TalkId};

/// 定常運転（Steady）のフェーズ分岐。
///
/// [`Phase::Steady`] にルーティングされた入力（Tick／ShioriReply／
/// TalkDone{reason: Ended | Interrupted}／CloseRequest）を処理する。Tick は pump ゲート
/// （GET/NOTIFY 使い分け）・ShioriReply は talk 調停（Value→StartTalk）・TalkDone/CloseRequest
/// は保留 close の記録／消化を担う。
pub(crate) fn step(state: State, input: Input, _config: &KanadeConfig) -> (State, Vec<Action>) {
    match input {
        Input::Tick { now } => on_tick(state, now),
        // origin（DD-IE-3）はマウス GET origin 別 reply 政策（本タスク 2.3）で消費する:
        // 応答の出所を ActiveTalk.origin へ転記し、talk 再生中の置換／防御破棄の分岐に用いる。
        Input::ShioriReply { outcome, origin } => on_reply(state, outcome, origin),
        Input::TalkDone(done) => on_talk_done(state, done),
        Input::CloseRequest { reason } => on_close_request(state, reason),
        // 上記以外（Boot・ForceQuit 等）は横断アームで捌かれ Steady には届かない。
        other => {
            let _ = &other;
            tracing::warn!(target: "kanade", event = "steady_input_ignored", "Steady に無関係な入力を無視");
            (state, Vec::new())
        }
    }
}

/// Steady でのマウス入力受理（DD-IE-1／DD-IE-8・Req 1.4／2.1／3.1）。
///
/// mod.rs の横断アームが Steady フェーズのマウス入力のみを本関数へ委譲する。受領した
/// [`MouseInput`] から正典イベント（`OnMouseMove` / `OnMouseDoubleClick`）の GET を 1 件構築し
/// 発行する。GET の応答（204／Value）に対する reply／置換政策は本関数の責務ではなく後続の
/// `on_reply` アーム（タスク 2.3）が担う——本関数は**GET 発行まで**である。
///
/// # close 保留中の防御（close 優先・DD-IE-8）
/// close 保留中（`pending_close.is_some()`）はマウス GET を発行しない。close 握手はマウス入力に
/// 割り込まれてはならないため、guard を最初に置き GET 発行より優先する（`pending_close` は
/// 消費しない・trace で観測）。
///
/// # Status の併送（DD-IE-1）
/// talk 再生中の Steady（`Steady{Some}`）でもマウス GET は**抑止せず常に発行**し、`Status: talking`
/// を [`super::State::snapshot`] 由来のスナップショットから併送する（NOTIFY 化しない）。GET/NOTIFY を
/// 使い分ける OnSecondChange pump とは異なり、マウス系は常に GET である。
///
/// いずれの経路もフェーズ遷移は起こさない（in-flight ≤ 1・GET 発行時と reply 到着時の
/// フェーズ同一性はシェルの同期往復が保証する）。
pub(super) fn on_mouse(state: State, input: MouseInput) -> (State, Vec<Action>) {
    if state.pending_close.is_some() {
        tracing::trace!(
            target: "kanade",
            event = "mouse_close_pending",
            ?input,
            "close 保留中——マウス GET を発行しない（close 優先・DD-IE-8）"
        );
        return (state, Vec::new());
    }
    // 送出時点の運行状態から Status を導出する（active talk 中は talking を併送・DD-IE-1／
    // 選択待ち中は choosing も併送・C5）。
    let snapshot = state.snapshot();
    let call = match input.kind {
        MouseEventKind::Move => events::on_mouse_move(
            input.x,
            input.y,
            input.scope,
            input.region.as_deref(),
            &snapshot,
        ),
        MouseEventKind::DoubleClick { button } => events::on_mouse_double_click(
            input.x,
            input.y,
            input.scope,
            input.region.as_deref(),
            button,
            &snapshot,
        ),
    };
    tracing::trace!(
        target: "kanade",
        event = "mouse_get",
        ?input,
        "Steady のマウス入力受理——正典イベント GET を発行（DD-IE-1・フェーズ遷移なし）"
    );
    (state, vec![Action::ShioriRequest(call)])
}

/// Steady での選択待ち成立通知の受領と帳簿確立（設計 C4 規則 4・Req7.1／7.6／7.7）。
///
/// mod.rs の横断アームが Steady フェーズの [`Input::ChoiceWaiting`] のみを本関数へ委譲する。
///
/// # 受理条件（一致のみ受理・他は棄却）
/// `Steady{talk: Some(active)}` かつ `active.talk_id == talk_id` のときだけ受理する。再生中で
/// ない（`talk: None`）／識別子が現行トークと一致しない通知は、既に終わったトーク宛の遅延通知か
/// 別トーク宛の誤配であり、帳簿を確立せず `choice_waiting_stale`（warn）で記録して棄却する
/// （状態は既存帳簿を含めて一切変えない・Req1.3 の二重防御の kanade 側）。
///
/// # 期限の確定（DD-8・Req7.6／7.7）
/// タイムアウト指令から期限への写像は [`choice_deadline`] が単一の入口として持つ（本層で
/// 再実装しない）。`None`＝未指定は `config.choice_timeout_default_ms` へ委譲し、`v <= 0.0` は
/// 無効化＝無期限（`deadline: None`）、`v > 0.0` は明示秒指定である。無効化でも**帳簿自体は
/// 確立する**——計測を開始しないだけで選択待ちは無期限に継続する（Req7.6）。
///
/// # 確立後の状態
/// 帳簿は [`ChoicePhase::Waiting`]（入力待ち）で確立し、候補 ID 列を通知どおりの表示順で保持する
/// （DD-7）。これがタスク 4.3 の受領検証（候補集合照合・talk_id 突合）と、タスク 4.5 の deadline
/// 到達判定が読む前提になる。[`Phase`] は一切触らない（DD-3）。Action は発行しない
/// （通知は kanade 内部の帳簿確立のみで完結する）。
pub(super) fn on_choice_waiting(
    mut state: State,
    talk_id: TalkId,
    choice_ids: Vec<String>,
    display_end: MonotonicMs,
    timeout_directive_secs: Option<f64>,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    let active_talk_id = match &state.phase {
        Phase::Steady { talk: Some(active) } => Some(active.talk_id),
        _ => None,
    };
    if active_talk_id != Some(talk_id) {
        let reason = if active_talk_id.is_none() {
            "no_active_talk"
        } else {
            "talk_id_mismatch"
        };
        tracing::warn!(
            target: "kanade",
            event = "choice_waiting_stale",
            reason,
            talk_id = talk_id.0,
            active_talk_id = ?active_talk_id.map(|t| t.0),
            choice_count = choice_ids.len(),
            "現行トークと一致しない選択待ち通知——帳簿を確立せず棄却（C4 規則 4）"
        );
        return (state, Vec::new());
    }
    let deadline = choice_deadline(
        display_end,
        timeout_directive_secs,
        config.choice_timeout_default_ms,
    );
    tracing::info!(
        target: "kanade",
        event = "choice_waiting_established",
        talk_id = talk_id.0,
        choice_count = choice_ids.len(),
        deadline_ms = ?deadline.map(|d| d.0),
        display_end_ms = display_end.0,
        timeout_directive_secs = ?timeout_directive_secs,
        "選択待ち帳簿を確立——以降の選択確定を受理可能（C4 規則 4・期限写像は DD-8）"
    );
    state.choice = Some(ChoiceState {
        talk_id,
        candidates: choice_ids,
        deadline,
        phase: ChoicePhase::Waiting,
    });
    (state, Vec::new())
}

/// Steady での選択確定の受領検証とカスケード第 1 段の発行（設計 C4 規則 1／2）。
///
/// mod.rs の横断アームが Steady フェーズの [`Input::Choice`] のみを本関数へ委譲する。
///
/// # 受領検証（規則 1・いずれの棄却も warn 記録・状態不変・継続）
/// 1. 選択待ち帳簿が無い（未成立・解決済み・タイムアウト済み）→ `choice_rejected_no_wait`（Req1.3）。
/// 2. 帳簿の対象 talk が現行 talk と一致しない（トーク切替で選択肢が消滅済み・再生中でない）
///    → `choice_rejected_no_wait`（Req1.3）。
/// 3. 段フェーズが [`ChoicePhase::Waiting`] でない（カスケード／タイムアウト応答待ち中の二重確定）
///    → `choice_rejected_busy`（Req1.1）。
/// 4. 選択肢 ID が候補集合に無い → `choice_rejected_unknown_id`（Req1.4・DD-7）。
///
/// 棄却は**状態不変が定義**であり、既存帳簿を含めて一切書き換えない（検証で取り出した帳簿は
/// 棄却経路で必ず戻す）。
///
/// # 受理（規則 2）
/// [`plan_cascade`] が段列を一意に決める（本層で再判定しない・Req2.5）:
/// - [`CascadePlan::Unsupported`]（`script:` 前置）→ SHIORI イベントを発行せず
///   `choice_unsupported_category`（warn）を記録し、[`Action::ResolveChoice`] のみ発行して帳簿を
///   消す（会話を停止させない・Req2.7・裁定 7）。
/// - [`CascadePlan::Named`]（`On` 始まり）→ 任意名イベント **1 段のみ**を発行する
///   （`OnChoiceSelectEx`／`OnChoiceSelect` を先行発火しない・Req2.1・裁定 1）。残段なし。
/// - [`CascadePlan::Canonical`] → `OnChoiceSelectEx` を先行段として発行し、残段に無印 1 段
///   （[`CascadeNext::Select`]）を積む（Req2.2）。
///
/// 受理は帳簿の段フェーズを [`ChoicePhase::Cascading`] へ進めるのみで [`Phase`] を触らない
/// （DD-3）。応答の処理（次段前進・解決・起動）は [`on_reply`] の choice 先行アーム（規則 3）。
pub(super) fn on_choice(mut state: State, input: ChoiceInput) -> (State, Vec<Action>) {
    let active_talk_id = match &state.phase {
        Phase::Steady { talk: Some(active) } => Some(active.talk_id),
        _ => None,
    };
    // 検証のため帳簿を取り出す。棄却経路は**必ず戻して**状態不変を保つ。
    let Some(mut ledger) = state.choice.take() else {
        tracing::warn!(
            target: "kanade",
            event = "choice_rejected_no_wait",
            reason = "no_choice_wait",
            choice_id = %input.id,
            scope = input.scope,
            "選択待ちが存在しない状態の選択確定——状態不変で棄却（C4 規則 1・Req1.3）"
        );
        return (state, Vec::new());
    };
    if active_talk_id != Some(ledger.talk_id) {
        tracing::warn!(
            target: "kanade",
            event = "choice_rejected_no_wait",
            reason = if active_talk_id.is_none() { "no_active_talk" } else { "talk_id_mismatch" },
            choice_id = %input.id,
            scope = input.scope,
            ledger_talk_id = ledger.talk_id.0,
            active_talk_id = ?active_talk_id.map(|t| t.0),
            "終了済み選択待ち宛の選択確定——状態不変で棄却（C4 規則 1・Req1.3）"
        );
        state.choice = Some(ledger);
        return (state, Vec::new());
    }
    if !matches!(ledger.phase, ChoicePhase::Waiting) {
        tracing::warn!(
            target: "kanade",
            event = "choice_rejected_busy",
            choice_id = %input.id,
            scope = input.scope,
            talk_id = ledger.talk_id.0,
            stage = choice_phase_label(&ledger.phase),
            "段の進行中に届いた二重の選択確定——状態不変で棄却（C4 規則 1・Req1.1）"
        );
        state.choice = Some(ledger);
        return (state, Vec::new());
    }
    if !ledger.candidates.contains(&input.id) {
        tracing::warn!(
            target: "kanade",
            event = "choice_rejected_unknown_id",
            choice_id = %input.id,
            scope = input.scope,
            talk_id = ledger.talk_id.0,
            candidate_count = ledger.candidates.len(),
            "候補集合に無い選択肢 ID——選択待ちを変えずに棄却（C4 規則 1・Req1.4）"
        );
        state.choice = Some(ledger);
        return (state, Vec::new());
    }

    // --- 受理（規則 2）---
    let talk_id = ledger.talk_id;
    let plan = plan_cascade(&input.id);
    tracing::info!(
        target: "kanade",
        event = "choice_accepted",
        choice_id = %input.id,
        label = %input.label,
        scope = input.scope,
        reference_count = input.references.len(),
        plan = ?plan,
        talk_id = talk_id.0,
        "選択確定を受理——カスケードを開始（C4 規則 2）"
    );
    // GET の共通ヘッダは送出時点の運行状態から導出する（Req3.6・DD-IT-3）。帳簿は検証のため
    // 手元（`ledger`）へ取り出し済みで `state.choice` は空なので、選択待ち継続中であることを
    // ここで明示的に与える——`State::snapshot()` をそのまま呼ぶとカスケード段の GET から
    // `choosing` が落ちる（C5 の源は `Waiting|Cascading|TimeoutInFlight` の全段）。
    let snapshot = state.snapshot_with_choice(true);
    let (call, next) = match plan {
        CascadePlan::Unsupported => {
            // M1 未対応カテゴリ（裁定 7）: イベントを発行せず解決だけ行う（Req2.7）。
            tracing::warn!(
                target: "kanade",
                event = "choice_unsupported_category",
                choice_id = %input.id,
                talk_id = talk_id.0,
                "M1 未対応カテゴリの選択肢 ID——イベントを発行せず選択解決のみ行う（Req2.7）"
            );
            return (
                state,
                vec![resolve_choice(talk_id, input.id, "unsupported")],
            );
        }
        // 任意名 1 段のみ（先行 Ex／無印を発行しない・裁定 1）。ID はイベント名側が運ぶため
        // Reference には付随参照列のみを載せる（Req3.3）。
        CascadePlan::Named => (
            events::on_choice_named(input.id.clone(), &input.references, &snapshot),
            None,
        ),
        // 正典形は Ex 先行・無印を残段に積む（Req2.2・裁定 2）。
        CascadePlan::Canonical => (
            events::on_choice_select_ex(&input.label, &input.id, &input.references, &snapshot),
            Some(CascadeNext::Select),
        ),
    };
    tracing::trace!(
        target: "kanade",
        event = "choice_cascade_stage",
        choice_id = %input.id,
        talk_id = talk_id.0,
        stage = call_id(&call),
        has_next = next.is_some(),
        "カスケード段の GET を送出（C4 規則 2）"
    );
    ledger.phase = ChoicePhase::Cascading {
        choice_id: input.id,
        next,
    };
    state.choice = Some(ledger);
    (state, vec![Action::ShioriRequest(call)])
}

/// カスケード応答の処理（設計 C4 規則 3・DD-4・origin 非依存）。
///
/// [`on_reply`] の choice 先行アームが [`ChoicePhase::Cascading`] の帳簿を分解して委譲する。
/// **origin を見ない**——応答の出所が任意名（`"OnChoiceEvent"`）でも `OnChoiceSelectEx`／
/// `OnChoiceSelect` でも同一に捌く（in-flight 帳簿の照合が正・DD-1）。
///
/// - [`ShioriOutcome::Value`] → 以降の段を発行せず（Req2.4）、新 talk_id を採番して slot を
///   差し替え（Req4.1／4.3）、`[ResolveChoice{old}, StartTalk(new)]` を**この順**で同一バッチに
///   載せる（DD-4・Req4.6／5.1）。旧 talk_id は 1 世代だけ `choice_prev_talk` へ保持する
///   （遅延 `TalkDone` の info 降格に使う・遷移規則 9。消費側の防御アームは
///   [`super::on_talk_done`]）。
/// - [`ShioriOutcome::NoContent`]／[`ShioriOutcome::Failed`]（error 記録・Req4.5）→ 残段あり:
///   次段 GET を発行し `Cascading{next: None}` を維持（Req2.3）。残段なし: 帳簿を消し
///   `[ResolveChoice{old}]` のみを発行する（起動なし・Req4.2／5.3）。
/// - 構造上起こらない応答（GET に対する `Notified`／`Unloaded`）は防御的に警告し、会話を選択待ちの
///   まま停止させないため 204 と同一に扱う（Error Strategy「選択系の失敗は会話を止めない」）。
fn on_cascade_reply(
    mut state: State,
    ledger: ChoiceState,
    outcome: ShioriOutcome,
    origin: &'static str,
) -> (State, Vec<Action>) {
    let ChoiceState {
        talk_id: old_talk_id,
        candidates,
        deadline,
        phase,
    } = ledger;
    let (choice_id, next) = match phase {
        ChoicePhase::Cascading { choice_id, next } => (choice_id, next),
        // 呼び出し元が Cascading のみを委譲する（構造上到達しない）。帳簿を復元して防御する。
        other => {
            state.choice = Some(ChoiceState {
                talk_id: old_talk_id,
                candidates,
                deadline,
                phase: other,
            });
            return steady_reply_unexpected(state, "Steady{choice}", outcome);
        }
    };

    let outcome = match outcome {
        ShioriOutcome::Value(script) => {
            // Value 短絡（Req2.4）: 以降の段を発行せず、解決と新トーク起動を同一バッチへ。
            let new_talk_id = TalkId(state.next_talk_id);
            state.next_talk_id += 1;
            tracing::info!(
                target: "kanade",
                event = "steady_talk",
                talk_id = new_talk_id.0,
                origin = origin,
                prev_talk_id = old_talk_id.0,
                "選択由来の応答にスクリプト——単一 slot 調停で差し替え再生起動（Req4.1／4.3）"
            );
            state.phase = Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id: new_talk_id,
                    origin,
                    script: script.clone(),
                }),
            };
            // choice 起因の slot 差替——旧 talk_id を 1 世代保持する（遷移規則 9・消費は 4.6）。
            state.choice_prev_talk = Some(old_talk_id);
            return (
                state,
                vec![
                    resolve_choice(old_talk_id, choice_id, "value"),
                    Action::StartTalk(StartTalk::new(new_talk_id, script)),
                ],
            );
        }
        ShioriOutcome::NoContent => "no_content",
        ShioriOutcome::Failed(failure) => {
            tracing::error!(
                target: "kanade",
                event = "choice_shiori_failed_as_204",
                error = %failure,
                choice_id = %choice_id,
                talk_id = old_talk_id.0,
                origin = origin,
                "選択由来の SHIORI 呼出が失敗——無応答（204）と同じ扱いで継続（Req4.5）"
            );
            "failed"
        }
        other => {
            // GET に対する Notified／Unloaded は構造上あり得ない。会話を選択待ちのまま停止
            // させないため、記録の上で 204 と同一に扱う（沈黙も停止もさせない）。
            tracing::warn!(
                target: "kanade",
                event = "steady_unexpected_reply",
                phase = "Steady{Cascading}",
                choice_id = %choice_id,
                origin = origin,
                "カスケード段に想定外の SHIORI 応答——204 相当で継続（会話を止めない）"
            );
            let _ = other;
            "unexpected"
        }
    };

    match next {
        // 残段あり（正典形の無印段）→ 次段 GET（Ref0=ID・Req2.3／3.2）。
        Some(CascadeNext::Select) => {
            // 帳簿は分解済み（`state.choice` は空）だが選択待ちは継続中である——次段の GET にも
            // `choosing` を載せるため明示的に真を与える（C5・on_choice と同じ理由）。
            let snapshot = state.snapshot_with_choice(true);
            let call = events::on_choice_select(&choice_id, &snapshot);
            tracing::trace!(
                target: "kanade",
                event = "choice_cascade_stage",
                choice_id = %choice_id,
                talk_id = old_talk_id.0,
                stage = call_id(&call),
                has_next = false,
                outcome = outcome,
                "先行段が応答を返さず次段の GET を送出（C4 規則 3・Req2.3）"
            );
            state.choice = Some(ChoiceState {
                talk_id: old_talk_id,
                candidates,
                deadline,
                phase: ChoicePhase::Cascading {
                    choice_id,
                    next: None,
                },
            });
            (state, vec![Action::ShioriRequest(call)])
        }
        // 残段なし → トーク起動なし・選択解決のみ（Req4.2／5.3・裁定 3）。
        None => (state, vec![resolve_choice(old_talk_id, choice_id, outcome)]),
    }
}

/// [`Action::ResolveChoice`] を組み立て、発行を info で記録する（Req5.1・単一の発行点）。
///
/// 呼び出しごとにちょうど 1 つの解決指示を返すため、「1 選択＝高々 1 解決」（Req5.4）は
/// 呼び出し点（未対応カテゴリの即時解決・カスケード終端）が排他であることで成立する。
fn resolve_choice(talk_id: TalkId, id: String, outcome: &'static str) -> Action {
    tracing::info!(
        target: "kanade",
        event = "choice_resolved",
        talk_id = talk_id.0,
        choice_id = %id,
        outcome = outcome,
        "選択待ちの解決を指示（Req5.1／5.3）"
    );
    Action::ResolveChoice { talk_id, id }
}

/// 送出する呼出のイベント ID（wire 形）を取り出す（カスケード段のログ観測用）。
fn call_id(call: &ShioriCall) -> &str {
    match call {
        ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. } => id.as_str(),
    }
}

/// 段フェーズの静的ラベル（ログ観測用・[`super::phase_label`] と同型の可観測性ヘルパ）。
pub(super) fn choice_phase_label(phase: &ChoicePhase) -> &'static str {
    match phase {
        ChoicePhase::Waiting => "Waiting",
        ChoicePhase::Cascading { .. } => "Cascading",
        ChoicePhase::TimeoutInFlight => "TimeoutInFlight",
    }
}

/// 選択待ちの期限到達を判定し、到達していれば `OnChoiceTimeout` を発行する（C4 規則 5・Req7.3）。
///
/// [`on_tick`] が既存 pump 処理に**先行して**呼ぶ。発行した場合は [`Some`]（発行 Action 列）を返し、
/// 呼び手はその Tick の pump を発行しない（規則 5「この Tick は pump を発行しない・次 Tick から
/// 再開」）。発行しない場合は [`None`] を返し、Tick は通常どおり進む。
///
/// # 発行条件（すべて満たすときのみ・時刻は注入値のみで判定する）
/// 1. 選択待ち帳簿があり段フェーズが [`ChoicePhase::Waiting`]（入力待ち）である
///    ——[`ChoicePhase::Cascading`]／[`ChoicePhase::TimeoutInFlight`] は既に SHIORI 応答待ちであり、
///    ここから二重に発火させない。
/// 2. 期限が確定している（`deadline: Some`）——`None`＝無効化指令の写り（`0`／`-1`）は**計測を
///    開始しない**という意味であり、選択待ちを無期限に継続する（Req7.6・DD-8）。
/// 3. `now >= deadline`（期限ちょうども到達・時刻の比較のみで判定し実時間を読まない）。
/// 4. 帳簿の対象 talk が現行 talk と一致する——Ref0 の供給源が当該トークの起動スクリプト
///    （`ActiveTalk.script`・DD-10）だからである。
///
/// # 条件 4 の防御を残す理由（`choice_timeout_ledger_stale`）
/// **現状この分岐は構造上到達不能である**——帳簿は対象 talk 一致時にしか確立されず
/// （`on_choice_waiting`）、slot 差替・トーク完了・close 系遷移のすべてで掃除規律
/// （C4 規則 7・[`super::clear_choice_ledger`]）が帳簿を落とすため、不一致のまま Tick へ
/// 到達する経路を静的に構成できない。実際、檻は 1 件も存在しない。
///
/// それでも除去せず残すのは、これが破れたときの帰結が「`OnChoiceTimeout` の Ref0 に**他トークの
/// 起動スクリプト**が載る」＝正典違反の送出であり、不変条件（帳簿の対象＝現行 talk）が将来の
/// 改修で崩れた瞬間に無言で誤送出しないための最終防御だからである。発火せず trace で観測して
/// 通常 Tick へ譲る（沈黙で捨てない・log-first）。到達したら不変条件の破れを意味する。
fn fire_choice_timeout_if_due(state: &mut State, now: MonotonicMs) -> Option<Vec<Action>> {
    let ledger = state.choice.as_ref()?;
    if !matches!(ledger.phase, ChoicePhase::Waiting) {
        return None;
    }
    // 無期限（`None`）は計測を開始しない（Req7.6）。期限つきは `>=` で到達判定（Req7.3）。
    let deadline = ledger.deadline?;
    if now.0 < deadline.0 {
        return None;
    }
    let ledger_talk_id = ledger.talk_id;
    let script = match &state.phase {
        Phase::Steady { talk: Some(active) } if active.talk_id == ledger_talk_id => {
            active.script.clone()
        }
        _ => {
            tracing::trace!(
                target: "kanade",
                event = "choice_timeout_ledger_stale",
                talk_id = ledger_talk_id.0,
                deadline_ms = deadline.0,
                now_ms = now.0,
                "現行トークと一致しない選択待ち帳簿——タイムアウトを発火せず通常 Tick へ（掃除は C4 規則 7）"
            );
            return None;
        }
    };
    // 帳簿を応答待ちへ進めてからスナップショットを採る——`TimeoutInFlight` も選択待ち継続中
    // であり（C5 の源は 3 段すべて）、`OnChoiceTimeout` GET の Status には `choosing` が載る。
    // 再取得するのは借用の都合のみ（上の判定で存在を確認済みであり、間に帳簿を消す処理は無い。
    // 万一空なら帳簿自体が不在であり、次 Tick は冒頭の帳簿判定で即座に非発火となる）。
    if let Some(ledger) = state.choice.as_mut() {
        ledger.phase = ChoicePhase::TimeoutInFlight;
    }
    tracing::info!(
        target: "kanade",
        event = "choice_timeout_fired",
        talk_id = ledger_talk_id.0,
        deadline_ms = deadline.0,
        now_ms = now.0,
        "選択待ちが期限に到達——OnChoiceTimeout を発行し当該周期の pump を止める（C4 規則 5・Req7.3）"
    );
    let snapshot = state.snapshot();
    Some(vec![Action::ShioriRequest(events::on_choice_timeout(
        &script, &snapshot,
    ))])
}

/// タイムアウト応答の処理（設計 C4 規則 6・F3・Req7.4／7.5）。
///
/// [`on_reply`] の choice 先行アームが [`ChoicePhase::TimeoutInFlight`] の帳簿を渡して委譲する
/// （カスケード応答と同じく **origin を見ない**——in-flight 帳簿の照合が正・DD-1）。
///
/// - [`ShioriOutcome::Value`] → **既存の起動経路**で置換再生する（新 talk_id 採番・slot 差替・
///   帳簿消去・Req7.4）。旧トークの終了は dispatcher の Close-then-spawn が担うため、ここで
///   解決指示（[`Action::ResolveChoice`]）は発行しない（F3）。旧 talk_id は 1 世代だけ
///   `choice_prev_talk` へ保持する（遷移規則 9・消費側の防御アームは [`super::on_talk_done`]）。
/// - [`ShioriOutcome::NoContent`]／[`ShioriOutcome::Failed`]（error 記録・Req4.5）→ 帳簿を消し
///   [`Action::CancelChoice`] を発行する（Req7.5）。これは **Close funnel の正規の入口**であり
///   （DD-11）、dispatcher が slot を維持したまま `Close` を転送し、talk が返す
///   `TalkDone{Interrupted}` で `Steady{None}` へ復帰する——独自のバリア状態や `skip_barrier` の
///   外部到達口は作らない（steering `canonical-not-minimal-lifecycle`）。帳簿が消えるため、
///   以降に到着する当該選択待ち宛の選択確定は [`on_choice`] の受領検証で棄却される（Req7.5）。
/// - 構造上起こらない応答（GET に対する `Notified`／`Unloaded`）は警告のうえ 204 と同一に扱う
///   （会話を選択待ちのまま停止させない）。
fn on_timeout_reply(
    mut state: State,
    ledger: ChoiceState,
    outcome: ShioriOutcome,
    origin: &'static str,
) -> (State, Vec<Action>) {
    let talk_id = ledger.talk_id;
    let outcome = match outcome {
        ShioriOutcome::Value(script) => {
            // 置換起動（Req7.4）: 既存の起動経路（採番＋slot 上書き＋StartTalk）をそのまま使う。
            let new_talk_id = TalkId(state.next_talk_id);
            state.next_talk_id += 1;
            tracing::info!(
                target: "kanade",
                event = "steady_talk",
                talk_id = new_talk_id.0,
                origin = origin,
                prev_talk_id = talk_id.0,
                "タイムアウト応答にスクリプト——既存の起動経路で置換再生（Req7.4）"
            );
            state.phase = Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id: new_talk_id,
                    origin,
                    script: script.clone(),
                }),
            };
            // choice 起因の slot 差替——旧 talk_id を 1 世代保持する（遷移規則 9・消費は 4.6）。
            state.choice_prev_talk = Some(talk_id);
            return (
                state,
                vec![Action::StartTalk(StartTalk::new(new_talk_id, script))],
            );
        }
        ShioriOutcome::NoContent => "no_content",
        ShioriOutcome::Failed(failure) => {
            tracing::error!(
                target: "kanade",
                event = "choice_shiori_failed_as_204",
                error = %failure,
                talk_id = talk_id.0,
                origin = origin,
                stage = "timeout",
                "タイムアウト GET が失敗——無応答（204）と同じ扱いで継続（Req4.5）"
            );
            "failed"
        }
        other => {
            // GET に対する Notified／Unloaded は構造上あり得ない。会話を選択待ちのまま停止
            // させないため、記録の上で 204 と同一に扱う（沈黙も停止もさせない）。
            tracing::warn!(
                target: "kanade",
                event = "steady_unexpected_reply",
                phase = "Steady{TimeoutInFlight}",
                talk_id = talk_id.0,
                origin = origin,
                "タイムアウト応答に想定外の SHIORI 応答——204 相当で継続（会話を止めない）"
            );
            let _ = other;
            "unexpected"
        }
    };
    // 204 相当（Req7.5）: 選択待ちを解除し、Close funnel の正規入口へ倒す（DD-11）。
    tracing::info!(
        target: "kanade",
        event = "choice_timeout_cancelled",
        talk_id = talk_id.0,
        outcome = outcome,
        "タイムアウト後に応答なし——選択待ちを解除しトークを終了させる（Req7.5・DD-11）"
    );
    (state, vec![Action::CancelChoice { talk_id }])
}

/// Steady での Tick（pump ゲート・Req 3.1／3.4／DD-6・選択タイムアウト先行判定）。
///
/// まず `last_now` を必ず更新する（時刻は発行有無に依らず進む・close 期限計算の基準）。
/// 次に選択待ちの期限到達を**既存 pump 処理に先行して**判定する（C4 規則 5・Req7.3）——到達
/// していれば `OnChoiceTimeout` を発行し、その周期の pump は発行しない（次 Tick から再開）。
/// 期限に達していなければ（無期限指定を含む）通常の pump へ進む:
/// - `talk: None` かつ `pending_close` あり → close 握手開始（OnClose GET・ClosePending へ）。
/// - `talk: None` かつ `pending_close` なし → OnSecondChange **GET**（Ref3=1・pump 問い合わせ）。
/// - `talk: Some` → OnSecondChange **NOTIFY**（Ref3=0・応答無視・pending_close は消化しない）。
fn on_tick(mut state: State, now: MonotonicMs) -> (State, Vec<Action>) {
    state.last_now = Some(now);
    // 選択タイムアウトは pump に先行する（C4 規則 5）。発火した周期は pump を発行しない。
    if let Some(actions) = fire_choice_timeout_if_due(&mut state, now) {
        return (state, actions);
    }
    // GET/NOTIFY・Ref3・Status を単一スナップショットから導出する（DD-IT-3）。State::snapshot は
    // Steady{None}→talk 非アクティブ（GET・Ref3=1・status 空）、Steady{Some}→talk アクティブ
    // （NOTIFY・Ref3=0・status talking）を与える——既存の wire 挙動を保存する。選択待ち中は
    // これに choosing が加わり `talking,choosing` となる（Req6.1／6.4・裁定 6）。slot 占有は
    // 継続しているため GET/NOTIFY の別と Ref3 は talk 軸のまま＝pump は NOTIFY のままである
    // （応答スクリプトを運べない型＝自発トーク抑止が構造で成立する・Req6.5）。
    let snapshot = state.snapshot();
    match state.phase {
        Phase::Steady { talk: None } => {
            if let Some(reason) = state.pending_close.take() {
                begin_close(state, reason)
            } else {
                // pump 問い合わせ（GET・Ref3=1）。応答待ちのまま Steady{None} を維持する。
                (
                    state,
                    vec![Action::ShioriRequest(events::on_second_change(
                        now, &snapshot,
                    ))],
                )
            }
        }
        Phase::Steady { talk: Some(_) } => {
            // active talk 中は NOTIFY（Ref3=0・応答は構造的に破棄）。pending_close があっても
            // ここでは握手を開始しない——当該 talk の TalkDone を待つ（DD-6・補足遷移）。
            (
                state,
                vec![Action::ShioriRequest(events::on_second_change(
                    now, &snapshot,
                ))],
            )
        }
        // Steady 以外はルーティング上到達しない（防御アーム）。
        _ => steady_phase_unexpected(state, "Tick"),
    }
}

/// Steady での ShioriReply（origin 別 talk 起動政策・Req 2.1／2.3／3.3／4.1／4.3／4.4・DD-IE-2／DD-IE-3）。
///
/// mod.rs は非 Failed 応答のみを Steady へ委譲する（Failed は横断アームで Unloading{Fault}）。`origin`
/// は応答の出所イベント ID（actor が発行 call の id を転記・pump は "OnSecondChange"／マウスは
/// "OnMouseMove"／"OnMouseDoubleClick"）。これを ActiveTalk へ転記し、再生中の分岐に用いる:
/// - `talk: None` + `Value(script)` → 一意 talk_id 採番＋StartTalk・`Steady{Some(origin)}` へ（origin ラベルは
///   応答の実イベント名・OnSecondChange 起動／マウス起動を同一経路で扱う・Req 4.1）。
/// - `talk: None` + `NoContent`（204）→ StartTalk なし・`Steady{None}` 維持（Req 2.3／4.2）。
/// - `talk: Some` + `Value` + origin ∈ {OnMouseMove, OnMouseDoubleClick} → **置換**: 新 talk_id 採番＋slot
///   上書き＋StartTalk（dispatcher の既存 Close-then-spawn が旧 talk を閉じ、旧 Done を stale 破棄する・
///   kanade 側は slot 上書きと採番のみ＝新調停なし・Req 4.3・DD-IE-2）。
/// - `talk: Some` + `Value` + その他 origin（例 OnSecondChange）→ 既存 DD-6 防御破棄・warn!＋破棄・維持。
/// - `talk: Some` + `Notified` → NOTIFY pump の応答・`Steady{Some}` 維持（無視）。
fn on_reply(
    mut state: State,
    outcome: ShioriOutcome,
    origin: &'static str,
) -> (State, Vec<Action>) {
    // === choice 先行アーム（C4 規則 3・origin 非依存）===
    // カスケード段の応答は既存の origin 政策 match より**先に**捌く。これを落とすと選択応答が
    // 下の DD-6 防御アーム（`steady_value_during_talk`）で warn 破棄され、選択が沈黙する
    // （design Risks の既知の罠）。判定は origin 文字列でなく in-flight 帳簿の照合で行う（DD-1）。
    if let Some(ledger) = state.choice.take() {
        match ledger.phase {
            ChoicePhase::Cascading { .. } => {
                return on_cascade_reply(state, ledger, outcome, origin);
            }
            // タイムアウト GET の応答も同じく先行して捌く（C4 規則 6・Req7.4／7.5）。
            ChoicePhase::TimeoutInFlight => {
                return on_timeout_reply(state, ledger, outcome, origin);
            }
            // 選択待ち（入力待ち）中の応答は本アームの対象外——in-flight な choice 呼出が無い
            // 以上、当該応答は pump／マウス由来である。帳簿を戻して既存の origin 政策へ委ねる。
            ChoicePhase::Waiting => {
                state.choice = Some(ledger);
            }
        }
    }
    match state.phase {
        Phase::Steady { talk: None } => match outcome {
            ShioriOutcome::Value(script) => {
                let talk_id = TalkId(state.next_talk_id);
                state.next_talk_id += 1;
                tracing::info!(target: "kanade", event = "steady_talk", talk_id = talk_id.0, origin = origin, "応答にスクリプト——再生起動");
                // origin は応答の出所（動的化・DD-IE-3）。pump なら "OnSecondChange"、マウスなら
                // 当該マウスイベント名がそのまま ActiveTalk のラベルに載る。script は起動値の
                // 保持（DD-10・`OnChoiceTimeout` Ref0 の供給源。新しい情報源を作らない）。
                state.phase = Phase::Steady {
                    talk: Some(ActiveTalk {
                        talk_id,
                        origin,
                        script: script.clone(),
                    }),
                };
                (
                    state,
                    vec![Action::StartTalk(StartTalk::new(talk_id, script))],
                )
            }
            ShioriOutcome::NoContent => {
                // 204: talk なし（Req 2.3／4.2）。Steady{None} を維持し次 Tick で pump 再開。
                (state, Vec::new())
            }
            other => steady_reply_unexpected(state, "Steady{None}", other),
        },
        Phase::Steady { talk: Some(_) } => match outcome {
            ShioriOutcome::Notified => {
                // NOTIFY pump（Ref3=0）の応答。構造的に無視し Steady{Some} を維持する。
                (state, Vec::new())
            }
            // 出所別の Value 政策（DD-IE-2）。origin の match は **wildcard にしない**——マウス系を
            // 明示列挙し、第 3 の origin 追加時にレビューで必ず政策判断を要求する（design Risks）。
            ShioriOutcome::Value(script) => match origin {
                "OnMouseMove" | "OnMouseDoubleClick" => {
                    // 置換（Req 4.3・DD-IE-2）: マウス由来の Value は active talk を差し替える。
                    // kanade 側は新 talk_id 採番＋slot 上書き＋StartTalk のみ。旧 talk の後始末
                    // （Close-then-spawn・旧 Done の stale 破棄）は dispatcher 既存実装へ完全委譲する。
                    let talk_id = TalkId(state.next_talk_id);
                    state.next_talk_id += 1;
                    tracing::info!(target: "kanade", event = "steady_talk_replace", talk_id = talk_id.0, origin = origin, "active talk 中にマウス由来 Value——単一 slot 置換（新 talk_id 採番）");
                    // slot 置換の掃除点（C4 規則 7・**マウス由来を含む**）: 旧トークの選択待ちは
                    // 置換で消滅する。加えて 1 世代 stale 保持も「次の slot 差替」で消す（規則 9）
                    // ——choice 起因でない置換が挟まれた時点で、保持していた旧 id は 2 世代前になる。
                    super::clear_choice_ledger(&mut state, "steady_talk_replace");
                    state.choice_prev_talk = None;
                    state.phase = Phase::Steady {
                        talk: Some(ActiveTalk {
                            talk_id,
                            origin,
                            script: script.clone(),
                        }),
                    };
                    (
                        state,
                        vec![Action::StartTalk(StartTalk::new(talk_id, script))],
                    )
                }
                // DD-6 防御破棄（非マウス origin 限定）。本アームの意味は「全 origin 防御」から
                // **「非マウス origin 限定の防御」へ狭まった**——マウス origin は上の置換アームへ
                // 抜けるため、ここへ届くのは pump（OnSecondChange）等の非マウス Value のみ。active
                // talk は NOTIFY を発行するため構造上ここへは届かないはずだが、万一届いても
                // StartTalk・キュー・中断を一切行わず破棄し、idle-talk 檻の防御規律を保存する。
                _ => {
                    tracing::warn!(target: "kanade", event = "steady_value_during_talk", origin = origin, "active talk 中に非マウス Value——構造上想定外・破棄（キュー/中断なし・DD-6）");
                    (state, Vec::new())
                }
            },
            other => steady_reply_unexpected(state, "Steady{Some}", other),
        },
        _ => steady_phase_unexpected(state, "ShioriReply"),
    }
}

/// Steady での TalkDone{reason: Ended | Interrupted}（Req 3.4 復帰／補足遷移）。
///
/// mod.rs は既知 talk の Ended／Interrupted（非 quit）のみを Steady へ委譲する（Quit は
/// 横断アームで Unloading{Quit}・未知 talk_id は error!＋維持）。ゆえに本アームは現 Steady talk の
/// 完了のみを受ける:
/// - `pending_close` あり → close 握手開始（OnClose GET・ClosePending へ・補足遷移）。
/// - `pending_close` なし → `Steady{None}` へ復帰（次 Tick で pump 再開・Req 3.4）。
fn on_talk_done(mut state: State, done: TalkDone) -> (State, Vec<Action>) {
    match state.phase {
        Phase::Steady { talk: Some(_) } => {
            let _ = done; // talk_id 突合は mod.rs 済み（既知 talk の非 quit のみ到達）。
            // 対象トークの完了＝帳簿の掃除点（C4 規則 7・Req1.3／6.2）。以降 slot は空くか
            // close 握手へ進むため、当該トークに紐づく選択待ちは構造上もう存在しない。
            super::clear_choice_ledger(&mut state, "steady_talk_done");
            if let Some(reason) = state.pending_close.take() {
                tracing::info!(target: "kanade", event = "steady_talk_done_close", reason = reason.as_ref_str(), "talk 完了——保留 close を消化し握手開始");
                begin_close(state, reason)
            } else {
                tracing::info!(target: "kanade", event = "steady_talk_done", "talk 完了——定常運転へ復帰");
                state.phase = Phase::Steady { talk: None };
                (state, Vec::new())
            }
        }
        _ => steady_phase_unexpected(state, "TalkDone"),
    }
}

/// Steady での CloseRequest（Req 4.1 起点／補足遷移）。
///
/// - `talk: None` → 待つべき talk がないため即握手開始（OnClose GET・ClosePending へ）。
/// - `talk: Some` → `pending_close` に記録するのみ・`Steady{Some}` 維持（TalkDone を待つ）。
fn on_close_request(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    match state.phase {
        Phase::Steady { talk: None } => {
            tracing::info!(target: "kanade", event = "steady_close_now", reason = reason.as_ref_str(), "close 指示——active talk なし・即握手開始");
            begin_close(state, reason)
        }
        Phase::Steady { talk: Some(_) } => {
            tracing::info!(target: "kanade", event = "steady_close_pending", reason = reason.as_ref_str(), "close 指示——active talk あり・保留記録（TalkDone を待つ）");
            state.pending_close = Some(reason);
            (state, Vec::new())
        }
        _ => steady_phase_unexpected(state, "CloseRequest"),
    }
}

/// close 握手開始: `OnClose` GET を発行し `ClosePending{reason}` へ遷移する。
///
/// この Steady→ClosePending 遷移は本タスク（2.4）の責務である。ClosePending 以降の握手
/// （応答処理・CloseTalkWait・期限・quit:false→Steady・204→無言終了）は close.rs（タスク
/// 2.5）が実装する。
fn begin_close(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    tracing::info!(target: "kanade", event = "close_handshake_begin", reason = reason.as_ref_str(), "OnClose GET を発行し握手を開始");
    state.phase = Phase::ClosePending { reason };
    // close 系遷移の掃除点（C4 規則 7）。`CloseRequest` の**受領**（active talk 中の保留記録）では
    // 掃除しない——保留は遷移ではなく、そこで帳簿を消すと選択が棄却されてバリアが解けず、
    // 待っている `TalkDone` が永遠に来ないため握手そのものが進まなくなる。
    super::clear_choice_ledger(&mut state, "close_handshake_begin");
    // 通常 close 握手は talk 非アクティブで行う（INACTIVE スナップショット）。
    (
        state,
        vec![Action::ShioriRequest(events::on_close(
            reason,
            &ExecutionSnapshot::INACTIVE,
        ))],
    )
}

/// Steady 以外の Phase が steady::step に届いた場合の防御アーム（構造上発生しない）。
fn steady_phase_unexpected(state: State, input: &str) -> (State, Vec<Action>) {
    tracing::warn!(target: "kanade", event = "steady_phase_unexpected", input = input, "Steady 以外の Phase への Steady 入力——現 Phase 維持で継続");
    (state, Vec::new())
}

/// Steady で想定外の SHIORI 応答（GET 待ちに Unloaded 等）を受けた場合の防御アーム。
fn steady_reply_unexpected(
    state: State,
    phase: &str,
    _outcome: ShioriOutcome,
) -> (State, Vec<Action>) {
    tracing::warn!(target: "kanade", event = "steady_unexpected_reply", phase = phase, "Steady 待ち点で想定外の SHIORI 応答——現 Phase 維持で継続");
    (state, Vec::new())
}

#[cfg(test)]
#[path = "steady_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "steady_flow_tests.rs"]
mod flow_tests;

#[cfg(test)]
#[path = "steady_choice_tests.rs"]
mod choice_tests;

#[cfg(test)]
#[path = "steady_choice_timeout_tests.rs"]
mod choice_timeout_tests;
