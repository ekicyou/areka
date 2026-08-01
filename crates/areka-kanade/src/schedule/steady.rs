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

use super::choice::{choice_deadline, plan_cascade, CascadePlan};
use super::{
    events, Action, ActiveTalk, CascadeNext, ChoicePhase, ChoiceState, Input, Phase, State,
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
            return (state, vec![resolve_choice(talk_id, input.id, "unsupported")]);
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
                    vec![Action::ShioriRequest(events::on_second_change(now, &snapshot))],
                )
            }
        }
        Phase::Steady { talk: Some(_) } => {
            // active talk 中は NOTIFY（Ref3=0・応答は構造的に破棄）。pending_close があっても
            // ここでは握手を開始しない——当該 talk の TalkDone を待つ（DD-6・補足遷移）。
            (
                state,
                vec![Action::ShioriRequest(events::on_second_change(now, &snapshot))],
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
                (state, vec![Action::StartTalk(StartTalk::new(talk_id, script))])
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
                    (state, vec![Action::StartTalk(StartTalk::new(talk_id, script))])
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
mod tests {
    use super::*;
    use crate::msg::ShioriCall;
    use crate::schedule::step;
    use crate::talk::TalkEndReason;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    /// Steady{talk: None}（pending_close なし）を任意時刻・任意採番で構築する。
    fn steady_none(next_id: u64) -> State {
        State {
            phase: Phase::Steady { talk: None },
            last_now: Some(MonotonicMs(500)),
            next_talk_id: next_id,
            pending_close: None,
            choice: None,
            choice_prev_talk: None,
        }
    }

    /// Steady{talk: Some(id)} を構築する。
    fn steady_some(talk_id: TalkId, next_id: u64) -> State {
        State {
            phase: Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id,
                    origin: "OnSecondChange",
                    script: String::new(),
                }),
            },
            last_now: Some(MonotonicMs(500)),
            next_talk_id: next_id,
            pending_close: None,
            choice: None,
            choice_prev_talk: None,
        }
    }

    /// 単一 Action が期待 ShioriCall（GET/NOTIFY・id・references）と一致することを検証する。
    fn assert_shiori(action: &Action, expected: &ShioriCall) {
        match (action, expected) {
            (
                Action::ShioriRequest(ShioriCall::Get {
                    id,
                    references,
                    status,
                }),
                ShioriCall::Get {
                    id: eid,
                    references: erefs,
                    status: estatus,
                },
            ) => {
                assert_eq!(id, eid, "GET id 不一致");
                assert_eq!(references, erefs, "GET references 不一致");
                assert_eq!(status, estatus, "GET status 不一致");
            }
            (
                Action::ShioriRequest(ShioriCall::Notify {
                    id,
                    references,
                    status,
                }),
                ShioriCall::Notify {
                    id: eid,
                    references: erefs,
                    status: estatus,
                },
            ) => {
                assert_eq!(id, eid, "NOTIFY id 不一致");
                assert_eq!(references, erefs, "NOTIFY references 不一致");
                assert_eq!(status, estatus, "NOTIFY status 不一致");
            }
            _ => panic!("ShioriRequest の GET/NOTIFY 種別が期待と不一致"),
        }
    }

    /// Action 列に OnSecondChange の ShioriRequest が一切ないことを検証する（ゲート閉）。
    fn assert_no_second_change(actions: &[Action]) {
        for a in actions {
            if let Action::ShioriRequest(
                ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. },
            ) = a
            {
                assert_ne!(id.as_str(), "OnSecondChange", "ゲートが閉じておらず OnSecondChange を発行した");
            }
        }
    }

    // === pump ゲート表駆動（観測可能な完了条件） ===
    // {起動中, Steady(None), Steady(Some), close 握手中以降} × Tick の発行有無・種別。

    // --- Steady{None} + Tick → OnSecondChange GET（Ref3=1）・Steady{None}・last_now 更新 ---

    #[test]
    fn steady_none_tick_emits_get_and_updates_last_now() {
        let now = MonotonicMs(7_200_000); // 2 hours。
        let (next, actions) = step(steady_none(5), Input::Tick { now }, &config());
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert_eq!(next.last_now, Some(now), "last_now は Tick ごとに更新される");
        assert_eq!(actions.len(), 1);
        // GET（Ref3=1）——events:: の出力と厳密一致。
        assert_shiori(&actions[0], &events::on_second_change(now, &ExecutionSnapshot { talk_active: false, choice_active: false }));
    }

    // --- Steady{Some} + Tick → OnSecondChange NOTIFY（Ref3=0）・Steady{Some} ---

    #[test]
    fn steady_some_tick_emits_notify() {
        let now = MonotonicMs(3_600_000); // 1 hour。
        let (next, actions) = step(steady_some(TalkId(3), 6), Input::Tick { now }, &config());
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3), "active talk は維持される"),
            _ => panic!("expected Steady{{Some}}"),
        }
        assert_eq!(next.last_now, Some(now));
        assert_eq!(actions.len(), 1);
        // NOTIFY（Ref3=0）——events:: の出力と厳密一致。
        assert_shiori(&actions[0], &events::on_second_change(now, &ExecutionSnapshot { talk_active: true, choice_active: false }));
    }

    // --- Steady{None} + Tick with pending_close → OnClose GET・ClosePending・pending 消化 ---

    #[test]
    fn steady_none_tick_with_pending_close_begins_handshake() {
        let now = MonotonicMs(1_000);
        let mut s = steady_none(5);
        s.pending_close = Some(CloseReason::User);
        let (next, actions) = step(s, Input::Tick { now }, &config());
        assert!(
            matches!(next.phase, Phase::ClosePending { reason: CloseReason::User }),
            "pending_close あり Tick は握手を開始し ClosePending へ"
        );
        assert!(next.pending_close.is_none(), "pending_close は消化される");
        assert_eq!(next.last_now, Some(now), "握手開始でも last_now は更新される");
        assert_eq!(actions.len(), 1);
        assert_shiori(&actions[0], &events::on_close(CloseReason::User, &ExecutionSnapshot::INACTIVE));
        // OnSecondChange は発行しない。
        assert_no_second_change(&actions);
    }

    // --- boot 中（BootMain）+ Tick → OnSecondChange なし（ゲート閉・boot は pump しない） ---

    #[test]
    fn boot_phase_tick_emits_no_second_change() {
        let s = State {
            phase: Phase::BootMain,
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
            choice: None,
            choice_prev_talk: None,
        };
        let (_next, actions) = step(s, Input::Tick { now: MonotonicMs(1_000) }, &config());
        // boot::step は pump を発行しない（ゲートは Steady に閉じている）。
        assert_no_second_change(&actions);
    }

    // --- close 握手中以降（ClosePending / CloseTalkWait）+ Tick → OnSecondChange なし ---

    #[test]
    fn close_pending_tick_emits_no_second_change() {
        let s = State {
            phase: Phase::ClosePending {
                reason: CloseReason::System,
            },
            last_now: None,
            next_talk_id: 1,
            pending_close: None,
            choice: None,
            choice_prev_talk: None,
        };
        let (_next, actions) = step(s, Input::Tick { now: MonotonicMs(1_000) }, &config());
        // close::step は現状 stub（pump 非発行）——OnSecondChange が出ないことを検証。
        assert_no_second_change(&actions);
    }

    #[test]
    fn close_talk_wait_tick_emits_no_second_change() {
        let s = State {
            phase: Phase::CloseTalkWait {
                talk_id: TalkId(2),
                deadline: None,
            },
            last_now: None,
            next_talk_id: 3,
            pending_close: None,
            choice: None,
            choice_prev_talk: None,
        };
        let (_next, actions) = step(s, Input::Tick { now: MonotonicMs(1_000) }, &config());
        assert_no_second_change(&actions);
    }

    // === talk 調停（ShioriReply） ===

    // --- Steady{None} + Value → StartTalk(id) + Steady{Some(id)}・id 単調増番 ---

    #[test]
    fn steady_none_value_starts_talk_and_ids_are_monotonic() {
        // 1 本目: next_id=5 → id=5。
        let (s1, actions1) = step(
            steady_none(5),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("hello".to_string()),
                origin: "OnSecondChange",
            },
            &config(),
        );
        match s1.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, origin, .. }),
            } => {
                assert_eq!(talk_id, TalkId(5));
                assert_eq!(origin, "OnSecondChange", "origin は応答の出所を転記（pump 起動）");
            }
            _ => panic!("expected Steady{{Some}}"),
        }
        assert_eq!(s1.next_talk_id, 6, "採番カウンタが進む");
        assert_eq!(actions1.len(), 1);
        match &actions1[0] {
            Action::StartTalk(StartTalk { talk_id, script, .. }) => {
                assert_eq!(*talk_id, TalkId(5));
                assert_eq!(script, "hello");
            }
            _ => panic!("expected StartTalk"),
        }

        // 2 本目（引き継いだカウンタ 6 で別 Steady{None} を想定）→ id=6（再利用しない）。
        let (s2, actions2) = step(
            steady_none(s1.next_talk_id),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("world".to_string()),
                origin: "OnSecondChange",
            },
            &config(),
        );
        let id2 = match &actions2[0] {
            Action::StartTalk(StartTalk { talk_id, .. }) => *talk_id,
            _ => panic!("expected StartTalk"),
        };
        assert_eq!(id2, TalkId(6), "id は単調増番・再利用しない");
        assert_eq!(s2.next_talk_id, 7);
    }

    // --- Steady{None} + NoContent(204) → no StartTalk・Steady{None} 維持 ---

    #[test]
    fn steady_none_no_content_starts_no_talk() {
        let (next, actions) = step(
            steady_none(5),
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "test",
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert_eq!(next.next_talk_id, 5, "204 は採番しない");
        assert!(actions.is_empty(), "204 は talk 起動しない（Req 2.3）");
    }

    // --- Steady{Some} + Notified → Steady{Some} 維持（NOTIFY pump の応答・無視） ---

    #[test]
    fn steady_some_notified_stays_and_emits_nothing() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::ShioriReply {
                outcome: ShioriOutcome::Notified,
                origin: "test",
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3)),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert!(actions.is_empty());
    }

    // --- Steady{Some} + 非マウス Value → warn!+破棄・Steady{Some} 維持・StartTalk なし（DD-6 防御） ---
    // origin は非マウス（OnSecondChange）——マウス origin は置換アームへ抜けるため、DD-6 破棄は
    // 非マウス origin 限定に狭まった（origin 別 reply 政策・DD-IE-2）。

    #[test]
    fn steady_some_value_is_discarded_without_start_talk() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("late".to_string()),
                origin: "OnSecondChange",
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3), "active talk は維持される"),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert_eq!(next.next_talk_id, 6, "破棄ゆえ採番しない");
        assert!(
            !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
            "Value-during-talk は StartTalk しない（DD-6）"
        );
        assert!(actions.is_empty(), "キュー・中断も発行しない");
    }

    // === origin 別 reply 政策: 置換 vs DD-6 防御破棄（Req 4.1／4.3／4.4・DD-IE-2／DD-IE-3） ===
    // 置換檻（マウス origin→置換）と DD-6 保存檻（非マウス origin→warn＋破棄）は**対**であり
    // 同一テスト群に配置する。実機では実 pasta の talking 自衛（204 相当）により置換が構造的に
    // 発火しないため mock 檻が唯一の検証手段。origin の match は wildcard にしない（第 3 の origin
    // 追加時にレビューで必ず政策を意識させるため）。

    // --- (c) Steady{None} + Value（マウス origin）→ StartTalk・ActiveTalk.origin=マウス名（4.1・DD-IE-3） ---

    #[test]
    fn steady_none_mouse_value_starts_talk_with_mouse_origin() {
        let (next, actions) = step(
            steady_none(5),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("nade".to_string()),
                origin: "OnMouseMove",
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, origin, .. }),
            } => {
                assert_eq!(talk_id, TalkId(5));
                assert_eq!(origin, "OnMouseMove", "origin は応答の出所（マウス名）を帯びる（動的化）");
            }
            _ => panic!("expected Steady{{Some}}"),
        }
        assert_eq!(next.next_talk_id, 6);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            Action::StartTalk(StartTalk { talk_id, script, .. }) => {
                assert_eq!(*talk_id, TalkId(5));
                assert_eq!(script, "nade");
            }
            _ => panic!("expected StartTalk"),
        }
    }

    // --- (c') Steady{Some(id=3)} + Value + origin=OnMouseDoubleClick → 置換（新 talk_id・slot 上書き・4.3） ---

    #[test]
    fn steady_some_mouse_value_replaces_slot_with_new_talk_id() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("menu".to_string()),
                origin: "OnMouseDoubleClick",
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, origin, .. }),
            } => {
                assert_eq!(talk_id, TalkId(6), "slot は新 talk_id で上書きされる（置換）");
                assert_eq!(origin, "OnMouseDoubleClick", "slot の origin も置換 origin へ更新");
            }
            _ => panic!("expected Steady{{Some}} replaced"),
        }
        assert_eq!(next.next_talk_id, 7, "置換は新 talk_id を採番する");
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            Action::StartTalk(StartTalk { talk_id, script, .. }) => {
                assert_eq!(*talk_id, TalkId(6), "StartTalk は新 talk_id（旧 talk は dispatcher が Close-then-spawn）");
                assert_eq!(script, "menu");
            }
            _ => panic!("expected StartTalk（置換）"),
        }
    }

    // --- DD-6 保存: Steady{Some} + Value + 非マウス origin(OnSecondChange) → warn＋破棄・維持（4.3/4.4） ---
    // 置換檻（上）と対。DD-6 防御の意味は「全 origin 防御」から「非マウス origin 限定の防御」へ
    // 狭まる——マウス origin は上の置換アームへ抜けるため、本檻は非マウス origin でのみ発火する。

    #[test]
    fn steady_some_non_mouse_value_is_discarded_dd6() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("late".to_string()),
                origin: "OnSecondChange",
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3), "非マウス origin の Value は置換せず維持（DD-6）"),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert_eq!(next.next_talk_id, 6, "破棄ゆえ採番しない");
        assert!(
            !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
            "非マウス Value-during-talk は StartTalk しない（DD-6）"
        );
        assert!(actions.is_empty(), "キュー・中断も発行しない");
    }

    // --- talk_id 単調性: マウス起動と OnSecondChange 起動を混在させても再利用しない ---

    #[test]
    fn talk_ids_never_reused_across_mixed_origins() {
        // OnSecondChange 起動（id=5）。
        let (s1, _) = step(
            steady_none(5),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("a".to_string()),
                origin: "OnSecondChange",
            },
            &config(),
        );
        assert_eq!(s1.next_talk_id, 6);
        // 当該 talk 完了 → 定常復帰。
        let (s2, _) = step(
            s1,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(5),
                reason: TalkEndReason::Ended,
            }),
            &config(),
        );
        // マウス起動（id=6・再利用しない）。
        let (s3, actions3) = step(
            s2,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("b".to_string()),
                origin: "OnMouseMove",
            },
            &config(),
        );
        let id = match &actions3[0] {
            Action::StartTalk(StartTalk { talk_id, .. }) => *talk_id,
            _ => panic!("expected StartTalk"),
        };
        assert_eq!(id, TalkId(6), "id は混在起動でも単調・再利用しない");
        assert_eq!(s3.next_talk_id, 7);
    }

    // --- 204: マウス origin の NoContent（Steady{None}）→ StartTalk なし（4.2） ---

    #[test]
    fn steady_none_mouse_no_content_starts_no_talk() {
        let (next, actions) = step(
            steady_none(5),
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnMouseDoubleClick",
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert_eq!(next.next_talk_id, 5, "204 は採番しない");
        assert!(actions.is_empty(), "マウス origin の 204 も talk 起動しない（Req 4.2）");
    }

    // === TalkDone{reason: Ended | Interrupted}（非 quit の 2 値ルーティング網羅） ===

    // --- Steady{Some(id)} + TalkDone{id, Ended}, pending None → Steady{None}・次 Tick で pump 再開 ---

    #[test]
    fn steady_talk_done_ended_resumes_steady_and_pump_restarts() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Ended,
            }),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }), "talk 完了で定常復帰");
        assert!(actions.is_empty(), "TalkDone 自体は副作用なし");

        // 復帰後の次 Tick で pump（GET）が再開することを確認（Req 3.4）。
        let now = MonotonicMs(9_000);
        let (after, tick_actions) = step(next, Input::Tick { now }, &config());
        assert!(matches!(after.phase, Phase::Steady { talk: None }));
        assert_eq!(tick_actions.len(), 1);
        assert_shiori(&tick_actions[0], &events::on_second_change(now, &ExecutionSnapshot { talk_active: false, choice_active: false }));
    }

    // --- Steady{Some(id)} + TalkDone{id, Interrupted}, pending None → 同じく Steady{None} 復帰 ---
    // kanade の 3 値ルーティング（本タスクの担当）: Interrupted は Ended と同一経路（非 quit）
    // として steady::on_talk_done に到達する（mod.rs が防御的に非 quit 扱いへ振る）。

    #[test]
    fn steady_talk_done_interrupted_resumes_steady_same_as_ended() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Interrupted,
            }),
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::Steady { talk: None }),
            "Interrupted も Ended と同じく定常復帰へ"
        );
        assert!(actions.is_empty(), "TalkDone 自体は副作用なし");
    }

    // --- Steady{Some(id)} + TalkDone{id, Ended}, pending Some → OnClose GET + ClosePending ---

    #[test]
    fn steady_talk_done_with_pending_close_begins_handshake() {
        let mut s = steady_some(TalkId(3), 6);
        s.pending_close = Some(CloseReason::System);
        let (next, actions) = step(
            s,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Ended,
            }),
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::ClosePending { reason: CloseReason::System }),
            "talk 完了時に保留 close を消化して握手開始"
        );
        assert!(next.pending_close.is_none(), "pending_close は消化される");
        assert_eq!(actions.len(), 1);
        assert_shiori(&actions[0], &events::on_close(CloseReason::System, &ExecutionSnapshot::INACTIVE));
    }

    // === CloseRequest ===

    // --- Steady{None} + CloseRequest → OnClose GET + ClosePending（即握手） ---

    #[test]
    fn steady_none_close_request_begins_handshake_now() {
        let (next, actions) = step(
            steady_none(5),
            Input::CloseRequest {
                reason: CloseReason::User,
            },
            &config(),
        );
        assert!(matches!(next.phase, Phase::ClosePending { reason: CloseReason::User }));
        assert_eq!(actions.len(), 1);
        assert_shiori(&actions[0], &events::on_close(CloseReason::User, &ExecutionSnapshot::INACTIVE));
    }

    // --- Steady{Some} + CloseRequest → pending_close 記録・Steady{Some} 維持（OnClose まだ） ---

    #[test]
    fn steady_some_close_request_records_pending_only() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::CloseRequest {
                reason: CloseReason::User,
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3), "active talk 中は Steady{{Some}} を維持"),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert!(
            matches!(next.pending_close, Some(CloseReason::User)),
            "pending_close に記録される（TalkDone を待つ）"
        );
        assert!(actions.is_empty(), "OnClose はまだ発行しない");
    }

    // === マウス GET 発行（Req 1.4／2.1／3.1・DD-IE-1／DD-IE-8） ===
    // seam（on_mouse）の意図的充填。step() 経由（横断アーム込み）で駆動し、期待 GET は
    // events:: の構築子と共有する（Reference 手書き重複を作らない）。

    use crate::msg::{MouseButton, MouseEventKind, MouseInput};

    fn mouse_move_input(region: Option<&str>) -> MouseInput {
        MouseInput {
            scope: 0,
            x: 10,
            y: 20,
            region: region.map(str::to_string),
            kind: MouseEventKind::Move,
        }
    }

    fn mouse_dbl_input(button: MouseButton) -> MouseInput {
        MouseInput {
            scope: 0,
            x: 10,
            y: 20,
            region: Some("Bust".to_string()),
            kind: MouseEventKind::DoubleClick { button },
        }
    }

    // --- Steady{None} + Move(region=Some) → OnMouseMove GET・Steady{None} 維持 ---

    #[test]
    fn steady_none_mouse_move_emits_get_and_keeps_phase() {
        let (next, actions) = step(
            steady_none(5),
            Input::Mouse(mouse_move_input(Some("Head"))),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }), "マウス GET は phase を変えない");
        assert_eq!(next.next_talk_id, 5, "マウス GET は採番しない");
        assert_eq!(actions.len(), 1, "GET を 1 件だけ発行");
        // Reference 完全一致は構築子と共有（talk 非アクティブ→INACTIVE・Status 行なし）。
        assert_shiori(
            &actions[0],
            &events::on_mouse_move(10, 20, 0, Some("Head"), &ExecutionSnapshot { talk_active: false, choice_active: false }),
        );
    }

    // --- Steady{None} + DoubleClick{Left/Right} → OnMouseDoubleClick GET・Ref5 分岐 ---

    #[test]
    fn steady_none_mouse_double_click_left_emits_get_ref5_zero() {
        let (next, actions) = step(
            steady_none(5),
            Input::Mouse(mouse_dbl_input(MouseButton::Left)),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert_eq!(actions.len(), 1);
        assert_shiori(
            &actions[0],
            &events::on_mouse_double_click(
                10,
                20,
                0,
                Some("Bust"),
                MouseButton::Left,
                &ExecutionSnapshot { talk_active: false, choice_active: false },
            ),
        );
    }

    #[test]
    fn steady_none_mouse_double_click_right_emits_get_ref5_one() {
        let (_next, actions) = step(
            steady_none(5),
            Input::Mouse(mouse_dbl_input(MouseButton::Right)),
            &config(),
        );
        assert_eq!(actions.len(), 1);
        assert_shiori(
            &actions[0],
            &events::on_mouse_double_click(
                10,
                20,
                0,
                Some("Bust"),
                MouseButton::Right,
                &ExecutionSnapshot { talk_active: false, choice_active: false },
            ),
        );
    }

    // --- Steady{Some(active)} + Move → GET は抑止せず発行・Status: talking を帯びる（DD-IE-1） ---
    // active talk 中でもマウス GET は NOTIFY 化せず GET のまま。State::snapshot()（Steady{Some}）から
    // talking が導出され Status ヘッダに載る。active talk は維持される。

    #[test]
    fn steady_some_mouse_move_emits_get_with_talking_status() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::Mouse(mouse_move_input(Some("Head"))),
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3), "active talk は維持される"),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert_eq!(actions.len(), 1, "active talk 中でもマウス GET を発行（抑止しない・DD-IE-1）");
        // 期待 GET は talk_active=true スナップショット由来＝Status: talking を帯びる。
        let expected =
            events::on_mouse_move(10, 20, 0, Some("Head"), &ExecutionSnapshot { talk_active: true, choice_active: false });
        assert_shiori(&actions[0], &expected);
        // GET のまま（NOTIFY 化しない）ことも明示。
        assert!(
            matches!(&actions[0], Action::ShioriRequest(ShioriCall::Get { .. })),
            "マウス系は常に GET（NOTIFY 化しない・DD-IE-1）"
        );
    }

    // --- pending_close 中は Steady でもマウス GET を発行しない（close 優先・DD-IE-8） ---

    #[test]
    fn steady_mouse_with_pending_close_emits_no_get() {
        let mut s = steady_none(5);
        s.pending_close = Some(CloseReason::System);
        let (next, actions) = step(s, Input::Mouse(mouse_move_input(Some("Head"))), &config());
        assert!(matches!(next.phase, Phase::Steady { talk: None }), "phase 不変");
        assert!(
            matches!(next.pending_close, Some(CloseReason::System)),
            "guard は pending_close を消費しない"
        );
        assert!(actions.is_empty(), "close 保留中はマウス GET を発行しない（close 優先）");
    }

    // === ActiveTalk.script の保持（タスク 4.1・DD-10・Req4.4） ===
    //
    // `OnChoiceTimeout` の Reference0 は「タイムアウトした選択肢を含むトークのスクリプト」
    // （Req3.4）である。その供給源は **kanade が `StartTalk` で自ら作った script** であり
    // （DD-10: 通知同梱でなく kanade 内で完結）、起動時に `ActiveTalk` へ転記して保持する。
    // 本檻は起動 2 経路（新規起動・マウス由来の置換）を実際に `step()` で通し、`ActiveTalk.script`
    // が発行された `StartTalk.script` と一致することを固定する（Ref0 の割付自体はタスク 4.5）。

    /// 現 Phase の `ActiveTalk.script` を取り出す（Steady{Some} 以外は panic）。
    fn active_script(phase: &Phase) -> &str {
        match phase {
            Phase::Steady {
                talk: Some(active), ..
            } => &active.script,
            _ => panic!("expected Steady{{Some}}"),
        }
    }

    /// 単一 StartTalk Action の script を取り出す（StartTalk 以外は panic）。
    fn started_script(action: &Action) -> &str {
        match action {
            Action::StartTalk(StartTalk { script, .. }) => script,
            _ => panic!("expected StartTalk"),
        }
    }

    #[test]
    fn steady_none_value_records_started_script_in_active_talk() {
        let (next, actions) = step(
            steady_none(5),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value(r"\0script-a\e".to_string()),
                origin: "OnSecondChange",
            },
            &config(),
        );
        assert_eq!(actions.len(), 1);
        assert_eq!(
            started_script(&actions[0]),
            r"\0script-a\e",
            "発行された StartTalk の script（既存挙動）"
        );
        assert_eq!(
            active_script(&next.phase),
            r"\0script-a\e",
            "起動した talk の script が ActiveTalk へ保持される（DD-10）"
        );
    }

    #[test]
    fn steady_some_mouse_replacement_records_new_script_in_active_talk() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::ShioriReply {
                outcome: ShioriOutcome::Value(r"\0script-b\e".to_string()),
                origin: "OnMouseDoubleClick",
            },
            &config(),
        );
        assert_eq!(actions.len(), 1);
        assert_eq!(started_script(&actions[0]), r"\0script-b\e");
        assert_eq!(
            active_script(&next.phase),
            r"\0script-b\e",
            "置換で差し替わった slot の script も新 talk のものへ更新される（DD-10）"
        );
    }

    // ============================================================
    // 選択確定の受領検証とカスケード駆動（タスク 4.3・C4 規則 1／2／3・DD-4）
    // ============================================================

    /// 檻用の選択確定入力（id／label／付随参照列を明示して組む・scope は 0 固定）。
    fn choice_input_of(id: &str, label: &str, references: &[&str]) -> ChoiceInput {
        ChoiceInput {
            id: id.to_string(),
            label: label.to_string(),
            scope: 0,
            references: references.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// 選択待ち帳簿つきの `Steady{Some(talk_id)}` を構築する（帳簿の talk は現行 talk と一致）。
    fn steady_with_ledger(
        talk_id: TalkId,
        next_id: u64,
        candidates: &[&str],
        phase: ChoicePhase,
    ) -> State {
        let mut s = steady_some(talk_id, next_id);
        s.choice = Some(ChoiceState {
            talk_id,
            candidates: candidates.iter().map(|c| c.to_string()).collect(),
            deadline: Some(MonotonicMs(32_000)),
            phase,
        });
        s
    }

    /// GET Action から (イベント ID の wire 形, Reference 列) を取り出す（GET 以外は panic）。
    fn expect_get_call(action: &Action) -> (String, Vec<String>) {
        match action {
            Action::ShioriRequest(ShioriCall::Get {
                id, references, ..
            }) => (id.as_str().to_string(), references.clone()),
            _ => panic!("expected GET ShioriRequest"),
        }
    }

    /// 帳簿の段フェーズを取り出す（帳簿不在は panic）。
    fn expect_ledger(state: &State) -> &ChoiceState {
        state.choice.as_ref().expect("選択待ち帳簿が存在するはず")
    }

    // --- A. 棄却分岐（規則 1）: すべて状態不変・Action なし ---

    /// Req1.3: 選択待ち帳簿が無い（解決済み・未成立）状態の選択確定は棄却する。
    #[test]
    fn choice_without_ledger_is_rejected_and_leaves_state_unchanged() {
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
            &config(),
        );
        assert!(
            matches!(next.phase, Phase::Steady { talk: Some(_) }),
            "棄却は Phase を触らない"
        );
        assert!(next.choice.is_none(), "棄却は帳簿を作らない");
        assert_eq!(next.next_talk_id, 6, "棄却は採番しない");
        assert!(actions.is_empty(), "棄却は Action を発行しない");
    }

    /// Req1.3: 帳簿の対象 talk が現行 talk と食い違う場合は棄却する（帳簿も無傷）。
    #[test]
    fn choice_with_ledger_of_other_talk_is_rejected() {
        let mut s = steady_some(TalkId(3), 6);
        s.choice = Some(ChoiceState {
            talk_id: TalkId(999),
            candidates: vec!["OnMenu".to_string()],
            deadline: None,
            phase: ChoicePhase::Waiting,
        });
        let (next, actions) = step(
            s,
            Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
            &config(),
        );
        let ledger = expect_ledger(&next);
        assert_eq!(ledger.talk_id, TalkId(999), "既存帳簿は棄却で変わらない");
        assert!(matches!(ledger.phase, ChoicePhase::Waiting));
        assert!(actions.is_empty());
    }

    /// Req1.3: 再生中でない（`Steady{None}`＝トーク切替で選択肢が消滅済み）なら棄却する。
    #[test]
    fn choice_without_active_talk_is_rejected() {
        let mut s = steady_none(5);
        s.choice = Some(ChoiceState {
            talk_id: TalkId(3),
            candidates: vec!["OnMenu".to_string()],
            deadline: None,
            phase: ChoicePhase::Waiting,
        });
        let (next, actions) = step(
            s,
            Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert!(matches!(expect_ledger(&next).phase, ChoicePhase::Waiting));
        assert!(actions.is_empty());
    }

    /// Req1.1: 段の進行中（`Cascading`／`TimeoutInFlight`）の二重確定は棄却する。
    #[test]
    fn choice_during_cascade_or_timeout_is_rejected_as_busy() {
        for phase in [
            ChoicePhase::Cascading {
                choice_id: "OnMenu".to_string(),
                next: None,
            },
            ChoicePhase::TimeoutInFlight,
        ] {
            let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], phase);
            let (next, actions) = step(
                s,
                Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
                &config(),
            );
            assert!(
                actions.is_empty(),
                "in-flight 中の二重確定は Action を発行しない（Req1.1）"
            );
            assert_eq!(next.next_talk_id, 6, "二重確定は採番しない");
            let ledger = expect_ledger(&next);
            assert!(
                !matches!(ledger.phase, ChoicePhase::Waiting),
                "棄却は段フェーズを巻き戻さない"
            );
        }
    }

    /// Req1.4: 候補集合に無い ID は棄却し、選択待ち状態を変更しない。
    #[test]
    fn choice_with_id_outside_candidates_is_rejected() {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu", "choice1"], ChoicePhase::Waiting);
        let (next, actions) = step(
            s,
            Input::Choice(choice_input_of("choice9", "他", &[])),
            &config(),
        );
        assert!(actions.is_empty(), "候補外 ID は Action を発行しない");
        let ledger = expect_ledger(&next);
        assert!(
            matches!(ledger.phase, ChoicePhase::Waiting),
            "候補外 ID の棄却は選択待ちを継続させる"
        );
        assert_eq!(
            ledger.candidates,
            vec!["OnMenu".to_string(), "choice1".to_string()],
            "候補列は棄却で変わらない"
        );
    }

    // --- B. 受理とカスケード第 1 段（規則 2・裁定 1／7） ---

    /// Req2.1・裁定 1: `On` 始まり ID は任意名 1 段のみ（Ex／無印を先行発火しない）。
    #[test]
    fn named_choice_emits_only_the_named_event() {
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["Onおしゃべり頻度メニュー"],
            ChoicePhase::Waiting,
        );
        let (next, actions) = step(
            s,
            Input::Choice(choice_input_of(
                "Onおしゃべり頻度メニュー",
                "おしゃべり頻度",
                &["a0", "a1"],
            )),
            &config(),
        );
        assert_eq!(actions.len(), 1, "第 1 段の GET を 1 件だけ発行する");
        let (id, refs) = expect_get_call(&actions[0]);
        assert_eq!(id, "Onおしゃべり頻度メニュー", "任意名イベントを逐語発火");
        assert_ne!(id, "OnChoiceSelectEx", "Ex を先行発火しない（裁定 1）");
        assert_ne!(id, "OnChoiceSelect", "無印を先行発火しない（裁定 1）");
        assert_eq!(
            refs,
            vec!["a0".to_string(), "a1".to_string()],
            "Ref0 以降＝付随参照列のみ（Req3.3）"
        );
        match &expect_ledger(&next).phase {
            ChoicePhase::Cascading { choice_id, next } => {
                assert_eq!(choice_id, "Onおしゃべり頻度メニュー");
                assert!(next.is_none(), "任意名形に残段は無い（1 段のみ）");
            }
            _ => panic!("受理で Cascading へ進む"),
        }
        assert!(
            matches!(next.phase, Phase::Steady { talk: Some(_) }),
            "受理は Phase を触らない（DD-3）"
        );
    }

    /// Req2.2／3.1: 正典形は `OnChoiceSelectEx` が先行し Reference が正典 layout で並ぶ。
    #[test]
    fn canonical_choice_emits_choice_select_ex_with_canonical_layout() {
        let s = steady_with_ledger(TalkId(3), 6, &["choice1"], ChoicePhase::Waiting);
        let (next, actions) = step(
            s,
            Input::Choice(choice_input_of("choice1", "ラベル", &["r0", "r1"])),
            &config(),
        );
        assert_eq!(actions.len(), 1);
        let (id, refs) = expect_get_call(&actions[0]);
        assert_eq!(id, "OnChoiceSelectEx", "正典形は Ex が先行段（Req2.2）");
        assert_eq!(
            refs,
            vec![
                "ラベル".to_string(),
                "choice1".to_string(),
                "r0".to_string(),
                "r1".to_string()
            ],
            "Ref0=ラベル／Ref1=ID／Ref2 以降=付随参照列（Req3.1）"
        );
        match &expect_ledger(&next).phase {
            ChoicePhase::Cascading { choice_id, next } => {
                assert_eq!(choice_id, "choice1");
                assert!(
                    matches!(next, Some(CascadeNext::Select)),
                    "正典形は無印 1 段を残段に持つ（Req2.2）"
                );
            }
            _ => panic!("受理で Cascading へ進む"),
        }
    }

    /// Req2.7・裁定 7: `script:` 前置はイベントを発行せず選択解決のみを行う。
    #[test]
    fn unsupported_choice_resolves_without_emitting_any_event() {
        let s = steady_with_ledger(TalkId(3), 6, &["script:\\e"], ChoicePhase::Waiting);
        let (next, actions) = step(
            s,
            Input::Choice(choice_input_of("script:\\e", "実行", &[])),
            &config(),
        );
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, Action::ShioriRequest(_))),
            "未対応カテゴリは SHIORI イベントを発行しない（Req2.7）"
        );
        match actions.as_slice() {
            [Action::ResolveChoice { talk_id, id }] => {
                assert_eq!(*talk_id, TalkId(3));
                assert_eq!(id, "script:\\e");
            }
            _ => panic!("未対応カテゴリは ResolveChoice のみを発行する"),
        }
        assert!(next.choice.is_none(), "解決で帳簿は消える");
        assert_eq!(next.next_talk_id, 6, "未対応カテゴリは talk を起動しない");
    }

    // --- C. カスケード応答（規則 3・DD-4） ---

    /// Req4.3／4.6／5.1・DD-4: 応答スクリプトは `[ResolveChoice, StartTalk]` をこの順で同一バッチに載せる。
    #[test]
    fn cascade_value_emits_resolve_then_start_in_this_order() {
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["OnMenu"],
            ChoicePhase::Cascading {
                choice_id: "OnMenu".to_string(),
                next: None,
            },
        );
        let (next, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value(r"\0次のシーン\e".to_string()),
                origin: "OnChoiceEvent",
            },
            &config(),
        );
        match actions.as_slice() {
            [
                Action::ResolveChoice { talk_id, id },
                Action::StartTalk(StartTalk {
                    talk_id: new_id,
                    script,
                    ..
                }),
            ] => {
                assert_eq!(*talk_id, TalkId(3), "解決対象は旧 talk");
                assert_eq!(id, "OnMenu");
                assert_eq!(*new_id, TalkId(6), "新 talk_id を採番する（Req4.1）");
                assert_eq!(script, r"\0次のシーン\e");
            }
            _ => panic!("[ResolveChoice, StartTalk] のこの順で発行されること（DD-4）"),
        }
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id,
                    origin,
                    ref script,
                }),
            } => {
                assert_eq!(talk_id, TalkId(6), "slot は新 talk へ差し替わる（Req4.3）");
                assert_eq!(origin, "OnChoiceEvent", "応答の出所を転記する");
                assert_eq!(script, r"\0次のシーン\e", "起動 script を保持（DD-10）");
            }
            _ => panic!("expected Steady{{Some}} replaced"),
        }
        assert_eq!(next.next_talk_id, 7);
        assert!(next.choice.is_none(), "解決で帳簿は消える");
        assert_eq!(
            next.choice_prev_talk,
            Some(TalkId(3)),
            "choice 起因の slot 差替で旧 talk_id を 1 世代保持する（遷移規則 9）"
        );
    }

    /// Req2.3: 204 かつ残段ありなら次段（無印・Ref0=ID）を発行する。
    #[test]
    fn cascade_no_content_advances_to_choice_select_stage() {
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["choice1"],
            ChoicePhase::Cascading {
                choice_id: "choice1".to_string(),
                next: Some(CascadeNext::Select),
            },
        );
        let (next, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceSelectEx",
            },
            &config(),
        );
        assert_eq!(actions.len(), 1, "次段の GET を 1 件だけ発行する");
        let (id, refs) = expect_get_call(&actions[0]);
        assert_eq!(id, "OnChoiceSelect", "残段は無印イベント（Req2.2）");
        assert_eq!(refs, vec!["choice1".to_string()], "Ref0=ID のみ（Req3.2）");
        match &expect_ledger(&next).phase {
            ChoicePhase::Cascading { choice_id, next } => {
                assert_eq!(choice_id, "choice1");
                assert!(next.is_none(), "無印段の後に残段は無い");
            }
            _ => panic!("次段発行後も Cascading を維持する"),
        }
        assert_eq!(next.next_talk_id, 6, "204 は採番しない");
    }

    /// Req2.3／5.3: 204 かつ残段なしなら選択解決のみ（起動なし）。
    #[test]
    fn cascade_no_content_at_last_stage_resolves_without_start() {
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["choice1"],
            ChoicePhase::Cascading {
                choice_id: "choice1".to_string(),
                next: None,
            },
        );
        let (next, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceSelect",
            },
            &config(),
        );
        match actions.as_slice() {
            [Action::ResolveChoice { talk_id, id }] => {
                assert_eq!(*talk_id, TalkId(3));
                assert_eq!(id, "choice1");
            }
            _ => panic!("最終段 204 は ResolveChoice のみ（DD-4・Req5.3）"),
        }
        assert!(
            !actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
            "最終段 204 で talk を起動しない（Req4.2）"
        );
        assert!(next.choice.is_none(), "解決で帳簿は消える");
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(3), "現行 talk は維持される"),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert_eq!(next.next_talk_id, 6);
    }

    /// Req4.5・規則 3: 段の失敗は error 記録の上で 204 と同一遷移（残段ありなら次段）。
    ///
    /// 本檻は steady 側の 204 相当処理そのものを `steady::step` の直接駆動で固定する（層を分けた
    /// 単体檻）。`step()` 経由の end-to-end——横断 `Failed`→`Unloading{Fault}` アームの免除
    /// （DD-12）が効いて終了系列へ倒れないこと——は
    /// [`cascade_failed_via_step_does_not_fall_into_unloading_fault`] が固定する。
    #[test]
    fn cascade_failed_is_treated_as_no_content_stage_advance() {
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["choice1"],
            ChoicePhase::Cascading {
                choice_id: "choice1".to_string(),
                next: Some(CascadeNext::Select),
            },
        );
        let (next, actions) = super::step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(crate::msg::ShioriFailure::Timeout(
                    "30s".to_string(),
                )),
                origin: "OnChoiceSelectEx",
            },
            &config(),
        );
        assert!(
            !matches!(next.phase, Phase::Unloading { .. }),
            "選択由来の失敗で終了系列へ倒れない（Req4.5）"
        );
        assert_eq!(actions.len(), 1);
        let (id, _) = expect_get_call(&actions[0]);
        assert_eq!(id, "OnChoiceSelect", "失敗は 204 と同一遷移＝次段へ前進");
    }

    /// Req4.5／5.3: 最終段の失敗も 204 と同一＝選択解決のみで会話を止めない。
    #[test]
    fn cascade_failed_at_last_stage_resolves_without_start() {
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["choice1"],
            ChoicePhase::Cascading {
                choice_id: "choice1".to_string(),
                next: None,
            },
        );
        let (next, actions) = super::step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(crate::msg::ShioriFailure::Ipc(
                    "pipe closed".to_string(),
                )),
                origin: "OnChoiceSelect",
            },
            &config(),
        );
        assert!(!matches!(next.phase, Phase::Unloading { .. }));
        match actions.as_slice() {
            [Action::ResolveChoice { id, .. }] => assert_eq!(id, "choice1"),
            _ => panic!("最終段の失敗は ResolveChoice のみ（Req5.3）"),
        }
        assert!(next.choice.is_none());
    }

    // --- D. 完了状態（一回性・Req1.1／4.6／5.4） ---

    /// 1 回の選択確定は高々 1 カスケード・高々 1 選択解決・高々 1 起動要求しか生じない（任意名形）。
    #[test]
    fn one_choice_yields_at_most_one_cascade_resolve_and_start() {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
        let (s1, a1) = step(
            s,
            Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
            &config(),
        );
        let (s2, a2) = step(
            s1,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("script".to_string()),
                origin: "OnChoiceEvent",
            },
            &config(),
        );
        // 解決後に遅れて届く応答・遅延した選択確定はいずれも追加のカスケードを起こさない。
        let (s3, a3) = step(
            s2,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceSelect",
            },
            &config(),
        );
        let (s4, a4) = step(
            s3,
            Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
            &config(),
        );
        let all: Vec<&Action> = a1.iter().chain(&a2).chain(&a3).chain(&a4).collect();
        assert_eq!(
            all.iter()
                .filter(|a| matches!(a, Action::ShioriRequest(_)))
                .count(),
            1,
            "カスケードは高々 1 回（Req1.1）"
        );
        assert_eq!(
            all.iter()
                .filter(|a| matches!(a, Action::ResolveChoice { .. }))
                .count(),
            1,
            "選択解決は高々 1 回（Req5.4）"
        );
        assert_eq!(
            all.iter()
                .filter(|a| matches!(a, Action::StartTalk(_)))
                .count(),
            1,
            "起動要求は高々 1 つ（Req4.6）"
        );
        assert!(s4.choice.is_none(), "解決後に帳簿は復活しない");
    }

    /// 正典形の 2 段が両方 204 でも、選択解決はちょうど 1 回・起動要求は 0（Req2.3／4.2／5.3）。
    #[test]
    fn canonical_two_stage_204_yields_single_resolve_and_no_start() {
        let s = steady_with_ledger(TalkId(3), 6, &["choice1"], ChoicePhase::Waiting);
        let (s1, a1) = step(
            s,
            Input::Choice(choice_input_of("choice1", "ラベル", &[])),
            &config(),
        );
        let (s2, a2) = step(
            s1,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceSelectEx",
            },
            &config(),
        );
        let (s3, a3) = step(
            s2,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceSelect",
            },
            &config(),
        );
        let all: Vec<&Action> = a1.iter().chain(&a2).chain(&a3).collect();
        let gets: Vec<String> = all
            .iter()
            .filter(|a| matches!(a, Action::ShioriRequest(_)))
            .map(|a| expect_get_call(a).0)
            .collect();
        assert_eq!(
            gets,
            vec!["OnChoiceSelectEx".to_string(), "OnChoiceSelect".to_string()],
            "Ex 先行→無印後続の 2 段（Req2.2／2.3）"
        );
        assert_eq!(
            all.iter()
                .filter(|a| matches!(a, Action::ResolveChoice { .. }))
                .count(),
            1,
            "選択解決はちょうど 1 回（Req5.3／5.4）"
        );
        assert!(
            !all.iter().any(|a| matches!(a, Action::StartTalk(_))),
            "全段 204 では起動要求を生じない（Req4.2）"
        );
        assert!(s3.choice.is_none());
    }

    /// Req2.4: 先行段が応答スクリプトを返したら以降の段を発行しない（正典形の短絡）。
    #[test]
    fn canonical_value_at_first_stage_skips_the_remaining_stage() {
        let s = steady_with_ledger(TalkId(3), 6, &["choice1"], ChoicePhase::Waiting);
        let (s1, _) = step(
            s,
            Input::Choice(choice_input_of("choice1", "ラベル", &[])),
            &config(),
        );
        let (s2, a2) = step(
            s1,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value("script".to_string()),
                origin: "OnChoiceSelectEx",
            },
            &config(),
        );
        assert!(
            !a2.iter().any(|a| matches!(a, Action::ShioriRequest(_))),
            "Value を返した段の後に無印段を発行しない（Req2.4）"
        );
        assert!(matches!(
            a2.as_slice(),
            [Action::ResolveChoice { .. }, Action::StartTalk(_)]
        ));
        assert!(s2.choice.is_none());
    }

    // --- E. 既存挙動の保存（DD-6 防御アームへ choice 応答が到達しないこと） ---

    /// C4 Implementation Notes: choice 応答は先行アームで捌かれ `steady_value_during_talk`
    /// （DD-6 防御）へ**到達しない**。到達すると選択応答が warn 破棄で沈黙する（既知の罠）。
    #[test]
    fn cascade_reply_does_not_reach_the_dd6_defense_arm() {
        let cfg = config();
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["OnMenu"],
            ChoicePhase::Cascading {
                choice_id: "OnMenu".to_string(),
                next: None,
            },
        );
        let mut actions = Vec::new();
        let ev = crate::schedule::log_capture::capture(|| {
            let (_next, a) = step(
                s,
                Input::ShioriReply {
                    outcome: ShioriOutcome::Value("script".to_string()),
                    origin: "OnChoiceEvent",
                },
                &cfg,
            );
            actions = a;
        });
        assert!(
            !ev.iter()
                .any(|e| e.event.as_deref() == Some("steady_value_during_talk")),
            "choice 応答が DD-6 防御アームへ落ちてはならない。\n捕捉={ev:#?}"
        );
        assert!(
            actions.iter().any(|a| matches!(a, Action::StartTalk(_))),
            "choice 応答は先行アームで置換起動される"
        );
    }

    // ============================================================
    // 選択待ち中の実行状態導出（タスク 4.4・Req6.1〜6.5・C5・裁定 6）
    // ============================================================

    /// ShioriRequest（GET/NOTIFY 問わず）の共通ヘッダ `Status` の wire 値を取り出す。
    fn status_wire(action: &Action) -> Option<String> {
        match action {
            Action::ShioriRequest(
                ShioriCall::Get { status, .. } | ShioriCall::Notify { status, .. },
            ) => status.render(),
            _ => panic!("expected ShioriRequest"),
        }
    }

    /// Req6.1／6.3／6.4・裁定 6: 選択待ち中の周期リクエストは **NOTIFY**（Ref3=`"0"`）で送出され、
    /// `Status` に複合値 `talking,choosing` が**正典順**で載る。
    ///
    /// NOTIFY は応答スクリプトを運べない型であり（[`ShioriOutcome::Notified`] のみ）、選択待ち中の
    /// 自発トーク抑止は既存 pump 分岐の構造だけで成立する（Req6.5・新しい抑止機構を作らない）。
    #[test]
    fn tick_during_choice_waiting_notifies_with_talking_and_choosing() {
        let now = MonotonicMs(1_000);
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
        let (next, actions) = step(s, Input::Tick { now }, &config());
        assert_eq!(actions.len(), 1, "選択待ち中も周期イベントは 1 件発行する");
        match &actions[0] {
            Action::ShioriRequest(ShioriCall::Notify {
                id,
                references,
                status,
            }) => {
                assert_eq!(id.as_str(), "OnSecondChange");
                assert_eq!(
                    references[3], "0",
                    "選択待ち中も再生中扱い＝再生可否 Reference は \"0\"（Req6.4）"
                );
                assert_eq!(
                    status.render(),
                    Some("talking,choosing".to_string()),
                    "複合値は正典順で連結される（Req6.1／6.3・裁定 6）"
                );
            }
            _ => panic!("選択待ち中の周期イベントは NOTIFY で送出される（Req6.4／6.5）"),
        }
        assert!(
            matches!(next.phase, Phase::Steady { talk: Some(_) }),
            "選択待ち中も slot 占有（Steady{{Some}}）が維持される（Req6.4）"
        );
        assert!(
            matches!(expect_ledger(&next).phase, ChoicePhase::Waiting),
            "pump は選択帳簿を触らない"
        );
    }

    /// Req6.2: 選択が解決して帳簿が消えた後の周期リクエストからは `choosing` が消える。
    ///
    /// 実解決経路（カスケード最終段 204 → `ResolveChoice` 発行・帳簿消去）を通してから pump を
    /// 採る——帳簿を手で消すのではなく、解決の実装が `choosing` を落とすことを固定する。
    #[test]
    fn tick_after_choice_resolution_drops_choosing() {
        let cfg = config();
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["OnMenu"],
            ChoicePhase::Cascading {
                choice_id: "OnMenu".to_string(),
                next: None,
            },
        );
        let (resolved, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceEvent",
            },
            &cfg,
        );
        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::ResolveChoice { .. })),
            "最終段 204 は選択解決を発行する"
        );
        assert!(resolved.choice.is_none(), "解決で帳簿が消える（Req6.2 の源）");
        let (_next, tick_actions) = step(
            resolved,
            Input::Tick {
                now: MonotonicMs(1_000),
            },
            &cfg,
        );
        assert_eq!(
            status_wire(&tick_actions[0]),
            Some("talking".to_string()),
            "解決後は choosing が消え talking のみが残る（Req6.2）"
        );
    }

    /// Req6.1・C5: **カスケード各段の GET も** 選択待ち継続中として `choosing` を帯びる。
    ///
    /// `on_choice`／`on_cascade_reply` は検証・分解のため帳簿を `State` から取り出した状態で
    /// スナップショットを採る。`State::snapshot` をそのまま呼ぶと当該 2 点だけ `choosing` が
    /// 落ちるため、両段の wire 値を実値で突合して固定する。
    #[test]
    fn cascade_stage_gets_carry_choosing() {
        let cfg = config();
        // 第 1 段（on_choice: 帳簿を take 済みの状態で採るスナップショット）。
        let s = steady_with_ledger(TalkId(3), 6, &["choice1"], ChoicePhase::Waiting);
        let (next, stage1) = step(
            s,
            Input::Choice(choice_input_of("choice1", "ラベル", &[])),
            &cfg,
        );
        let (stage1_id, _) = expect_get_call(&stage1[0]);
        assert_eq!(stage1_id, "OnChoiceSelectEx", "正典形の先行段");
        assert_eq!(
            status_wire(&stage1[0]),
            Some("talking,choosing".to_string()),
            "第 1 段の GET に choosing が載る（C5）"
        );
        // 第 2 段（on_cascade_reply: 帳簿を分解済みの状態で採るスナップショット）。
        let (_next2, stage2) = step(
            next,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceSelectEx",
            },
            &cfg,
        );
        let (stage2_id, _) = expect_get_call(&stage2[0]);
        assert_eq!(stage2_id, "OnChoiceSelect", "残段（無印）へ前進している");
        assert_eq!(
            status_wire(&stage2[0]),
            Some("talking,choosing".to_string()),
            "次段の GET にも choosing が載る（C5）"
        );
    }

    // ============================================================
    // 選択肢タイムアウト（タスク 4.5・C4 規則 5／6・Req7.3〜7.5・DD-10／DD-11）
    // ============================================================
    //
    // 完了状態の要求は「**注入時刻のみ**で期限到達・イベント発行・空応答時の解除までが再現し、
    // 実時間待機に依存しない」ことである。よって本群は一切スリープせず、`Input::Tick { now }` の
    // 注入値と帳簿の `deadline` の比較だけで全分岐を踏む（`deterministic-test-coverage-mandate`）。

    use crate::schedule::log_capture::{assert_logged, capture, logged_once};
    use tracing::Level;

    /// 選択待ち（`Waiting`）帳簿つきの `Steady{Some}` を、起動スクリプトと期限を指定して構築する。
    ///
    /// `deadline: None` は無期限（`0`／`-1` 指令が [`choice_deadline`] で写った形・Req7.6）。
    fn steady_waiting(
        talk_id: TalkId,
        next_id: u64,
        script: &str,
        deadline: Option<MonotonicMs>,
    ) -> State {
        let mut s = steady_some(talk_id, next_id);
        match &mut s.phase {
            Phase::Steady { talk: Some(active) } => active.script = script.to_string(),
            _ => unreachable!("steady_some は Steady{{Some}} を返す"),
        }
        s.choice = Some(ChoiceState {
            talk_id,
            candidates: vec!["OnMenu".to_string()],
            deadline,
            phase: ChoicePhase::Waiting,
        });
        s
    }

    /// `step` を捕捉つきで駆動する（遷移結果と捕捉ログを同時に取る）。
    fn step_capturing(
        state: State,
        input: Input,
        config: &KanadeConfig,
    ) -> (State, Vec<Action>, Vec<crate::schedule::log_capture::CapturedEvent>) {
        let mut out = None;
        let ev = capture(|| {
            out = Some(step(state, input, config));
        });
        let (next, actions) = out.expect("step は必ず結果を返す");
        (next, actions, ev)
    }

    // --- A. 期限到達（規則 5・Req7.3） ---

    /// Req7.3・DD-10: 注入時刻が期限**ちょうど**（`now == deadline`）で到達し、`OnChoiceTimeout` を
    /// Ref0＝起動スクリプトで発行する。帳簿は `TimeoutInFlight` へ進み、当該 Tick は pump を出さない。
    #[test]
    fn choice_timeout_fires_at_deadline_with_script_ref0_and_suppresses_pump() {
        let script = r"\0えらんでね\q[はい,OnMenu]\e";
        let s = steady_waiting(TalkId(3), 6, script, Some(MonotonicMs(32_000)));
        let (next, actions, ev) = step_capturing(
            s,
            Input::Tick {
                now: MonotonicMs(32_000),
            },
            &config(),
        );
        assert_eq!(actions.len(), 1, "期限到達 Tick は GET を 1 件だけ発行する");
        let (id, refs) = expect_get_call(&actions[0]);
        assert_eq!(id, "OnChoiceTimeout", "期限到達で OnChoiceTimeout を発行（Req7.3）");
        assert_eq!(
            refs,
            vec![script.to_string()],
            "Ref0＝タイムアウトした選択肢を含むトークの起動スクリプト（Req3.4・DD-10）"
        );
        // 当該周期では通常の周期送出を行わない（規則 5）。
        assert_no_second_change(&actions);
        assert!(
            matches!(expect_ledger(&next).phase, ChoicePhase::TimeoutInFlight),
            "期限到達で帳簿は TimeoutInFlight へ進む"
        );
        assert!(
            matches!(next.phase, Phase::Steady { talk: Some(_) }),
            "タイムアウト発行は Phase を触らない（DD-3）"
        );
        assert_eq!(
            next.last_now,
            Some(MonotonicMs(32_000)),
            "発行有無に依らず last_now は進む（既存規律）"
        );
        // 4.4 申し送り: 5 番目の呼出点にも choosing が載る（TimeoutInFlight も選択待ち継続中）。
        assert_eq!(
            status_wire(&actions[0]),
            Some("talking,choosing".to_string()),
            "OnChoiceTimeout GET の Status に choosing が載る（C5・裁定 6）"
        );
        let fired = logged_once(&ev, Level::INFO, "choice_timeout_fired");
        assert_eq!(
            fired.fields.get("talk_id").map(String::as_str),
            Some("3"),
            "発火ログは対象 talk_id を載せる。\n捕捉={ev:#?}"
        );
    }

    /// 境界: `now > deadline`（Tick が期限を跨いだ）でも到達（`>=` 判定）。
    #[test]
    fn choice_timeout_fires_when_tick_overshoots_deadline() {
        let s = steady_waiting(TalkId(3), 6, r"\e", Some(MonotonicMs(32_000)));
        let (next, actions) = step(
            s,
            Input::Tick {
                now: MonotonicMs(33_000),
            },
            &config(),
        );
        let (id, _) = expect_get_call(&actions[0]);
        assert_eq!(id, "OnChoiceTimeout");
        assert!(matches!(
            expect_ledger(&next).phase,
            ChoicePhase::TimeoutInFlight
        ));
    }

    /// 境界: `now < deadline`（1ms 手前）では発行せず、通常の周期送出を続ける。
    #[test]
    fn choice_timeout_does_not_fire_before_deadline() {
        let s = steady_waiting(TalkId(3), 6, r"\e", Some(MonotonicMs(32_000)));
        let (next, actions) = step(
            s,
            Input::Tick {
                now: MonotonicMs(31_999),
            },
            &config(),
        );
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            Action::ShioriRequest(ShioriCall::Notify { id, .. }) => {
                assert_eq!(id.as_str(), "OnSecondChange", "期限前は通常の周期送出（NOTIFY）")
            }
            _ => panic!("期限前は選択待ち中の通常 pump（NOTIFY）が出る"),
        }
        assert!(
            matches!(expect_ledger(&next).phase, ChoicePhase::Waiting),
            "期限前は選択待ちのまま"
        );
    }

    /// Req7.6: 無期限（`deadline == None`）は計測を開始せず、どれだけ時刻が進んでも発行しない。
    /// 選択待ちは継続し、pump は通常どおり発行される。
    #[test]
    fn choice_timeout_never_fires_for_indefinite_deadline() {
        let cfg = config();
        let mut s = steady_waiting(TalkId(3), 6, r"\e", None);
        for now in [1_000_u64, 60_000, u64::MAX] {
            let (next, actions) = step(
                s,
                Input::Tick {
                    now: MonotonicMs(now),
                },
                &cfg,
            );
            assert_eq!(actions.len(), 1, "無期限でも周期送出は続く（Req7.6）");
            match &actions[0] {
                Action::ShioriRequest(ShioriCall::Notify { id, status, .. }) => {
                    assert_eq!(id.as_str(), "OnSecondChange");
                    assert_eq!(
                        status.render(),
                        Some("talking,choosing".to_string()),
                        "無期限の選択待ちは継続する（choosing が載り続ける）"
                    );
                }
                _ => panic!("無期限では OnChoiceTimeout を発行しない（Req7.6）"),
            }
            assert!(matches!(expect_ledger(&next).phase, ChoicePhase::Waiting));
            s = next;
        }
    }

    /// 規則 5: 発行した Tick でのみ pump を止める——**次 Tick からは再開**する
    /// （応答待ち中も slot 占有は継続＝NOTIFY・choosing 継続）。
    #[test]
    fn pump_resumes_on_the_tick_after_choice_timeout_fired() {
        let cfg = config();
        let s = steady_waiting(TalkId(3), 6, r"\e", Some(MonotonicMs(32_000)));
        let (fired, actions) = step(
            s,
            Input::Tick {
                now: MonotonicMs(32_000),
            },
            &cfg,
        );
        assert_eq!(expect_get_call(&actions[0]).0, "OnChoiceTimeout");
        let (next, resumed) = step(
            fired,
            Input::Tick {
                now: MonotonicMs(33_000),
            },
            &cfg,
        );
        assert_eq!(resumed.len(), 1, "次 Tick から周期送出が再開する（規則 5）");
        match &resumed[0] {
            Action::ShioriRequest(ShioriCall::Notify { id, status, .. }) => {
                assert_eq!(id.as_str(), "OnSecondChange");
                assert_eq!(
                    status.render(),
                    Some("talking,choosing".to_string()),
                    "応答待ち（TimeoutInFlight）中も選択待ちは継続中（C5）"
                );
            }
            _ => panic!("次 Tick は通常の pump（NOTIFY）"),
        }
        assert!(
            matches!(expect_ledger(&next).phase, ChoicePhase::TimeoutInFlight),
            "再開した pump は帳簿を触らない（二重発行もしない）"
        );
        assert_eq!(
            resumed
                .iter()
                .filter(|a| matches!(
                    a,
                    Action::ShioriRequest(ShioriCall::Get { .. })
                ))
                .count(),
            0,
            "OnChoiceTimeout を二重発行しない"
        );
    }

    // --- B. タイムアウト応答（規則 6・Req7.4／7.5） ---

    /// Req7.4: 応答スクリプトは**既存の起動経路**で置換再生する（新 talk_id 採番・slot 差替・
    /// 旧 talk_id は 1 世代保持）。解決指示（`ResolveChoice`）は発行しない——旧トークは
    /// dispatcher の Close-then-spawn で終了する（F3）。
    #[test]
    fn choice_timeout_value_replaces_talk_via_existing_start_path() {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
        let (next, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value(r"\0時間切れ\e".to_string()),
                origin: "OnChoiceTimeout",
            },
            &config(),
        );
        match actions.as_slice() {
            [Action::StartTalk(StartTalk {
                talk_id, script, ..
            })] => {
                assert_eq!(*talk_id, TalkId(6), "新 talk_id を採番する（Req7.4／4.1）");
                assert_eq!(script, r"\0時間切れ\e");
            }
            _ => panic!("タイムアウト Value は StartTalk のみを発行する（F3）"),
        }
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id,
                    origin,
                    ref script,
                }),
            } => {
                assert_eq!(talk_id, TalkId(6), "slot は新 talk へ差し替わる");
                assert_eq!(origin, "OnChoiceTimeout", "応答の出所を転記する");
                assert_eq!(script, r"\0時間切れ\e", "起動 script を保持（DD-10）");
            }
            _ => panic!("expected Steady{{Some}} replaced"),
        }
        assert_eq!(next.next_talk_id, 7);
        assert!(next.choice.is_none(), "置換起動で帳簿は消える（Req7.4）");
        assert_eq!(
            next.choice_prev_talk,
            Some(TalkId(3)),
            "タイムアウト Value も choice 起因の slot 差替＝旧 talk_id を 1 世代保持（遷移規則 9）"
        );
    }

    /// Req7.5・DD-11: 204 は選択待ちを解除し `CancelChoice` を発行する（Close funnel の入口）。
    /// 独自のバリア状態・小細工（skip_barrier 等）は使わない——発行するのは正規の型付き指示のみ。
    #[test]
    fn choice_timeout_no_content_cancels_choice() {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
        let (next, actions, ev) = step_capturing(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceTimeout",
            },
            &config(),
        );
        match actions.as_slice() {
            [Action::CancelChoice { talk_id }] => {
                assert_eq!(*talk_id, TalkId(3), "解除対象は選択待ちの talk（Req7.5）")
            }
            _ => panic!("204 は CancelChoice のみを発行する（DD-11・F3）"),
        }
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, Action::StartTalk(_) | Action::ResolveChoice { .. })),
            "204 は起動も解決も行わない（解除して終了させる・Req7.5）"
        );
        assert!(next.choice.is_none(), "解除で帳簿は消える（Req6.2／7.5）");
        assert_eq!(next.next_talk_id, 6, "204 は採番しない");
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(
                talk_id,
                TalkId(3),
                "slot は維持したまま Close funnel の完了（TalkDone{{Interrupted}}）を待つ（DD-11）"
            ),
            _ => panic!("expected Steady{{Some}} preserved"),
        }
        assert_logged(&ev, Level::INFO, "choice_timeout_cancelled");
    }

    /// Req4.5／7.5: タイムアウト GET の失敗も 204 と同一＝解除で継続（終了系列へ倒れない）。
    ///
    /// カスケード段の檻と同じく `steady::step` の直接駆動で steady 側の処理を固定する（層を分けた
    /// 単体檻）。`step()` 経由の end-to-end（DD-12 の免除アームが効くこと）は
    /// [`timeout_failed_via_step_cancels_without_unloading`] が固定する。
    #[test]
    fn choice_timeout_failed_is_treated_as_no_content_cancel() {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
        let mut out = None;
        let ev = capture(|| {
            out = Some(super::step(
                s,
                Input::ShioriReply {
                    outcome: ShioriOutcome::Failed(crate::msg::ShioriFailure::Timeout(
                        "30s".to_string(),
                    )),
                    origin: "OnChoiceTimeout",
                },
                &config(),
            ));
        });
        let (next, actions) = out.expect("step は必ず結果を返す");
        assert!(
            !matches!(next.phase, Phase::Unloading { .. }),
            "選択由来の失敗で終了系列へ倒れない（Req4.5）"
        );
        match actions.as_slice() {
            [Action::CancelChoice { talk_id }] => assert_eq!(*talk_id, TalkId(3)),
            _ => panic!("失敗は 204 と同一＝CancelChoice のみ（Req7.5）"),
        }
        assert!(next.choice.is_none());
        assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
        assert_logged(&ev, Level::INFO, "choice_timeout_cancelled");
    }

    // --- C. 解除後の棄却（Req7.5 後半・DD-11 の正規到達点） ---

    /// Req7.5: 解除後に到着する当該選択待ち宛の選択確定は棄却する。
    ///
    /// 経路は正規（小細工なし）: 期限到達 → `OnChoiceTimeout` → 204 → `CancelChoice` →
    /// （dispatcher が Close を転送し talk が返す）`TalkDone{Interrupted}` → `Steady{None}` 復帰。
    /// この `Interrupted` こそ DD-11 が本設計で**正規到達点**にした経路である（mod.rs 防御アーム）。
    #[test]
    fn choice_after_timeout_cancel_is_rejected() {
        let cfg = config();
        // 1) 期限到達 → OnChoiceTimeout。
        let s = steady_waiting(TalkId(3), 6, r"\e", Some(MonotonicMs(32_000)));
        let (s1, fired) = step(
            s,
            Input::Tick {
                now: MonotonicMs(32_000),
            },
            &cfg,
        );
        assert_eq!(expect_get_call(&fired[0]).0, "OnChoiceTimeout");
        // 2) 204 → CancelChoice（選択待ち解除）。
        let (s2, cancelled) = step(
            s1,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceTimeout",
            },
            &cfg,
        );
        assert!(matches!(
            cancelled.as_slice(),
            [Action::CancelChoice { .. }]
        ));
        // 3) Close funnel の完了通知（正規の TalkDone{Interrupted}）→ Steady{None} 復帰。
        let (s3, done_actions, ev) = step_capturing(
            s2,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Interrupted,
            }),
            &cfg,
        );
        assert_logged(&ev, Level::INFO, "talk_done_interrupted_as_non_quit");
        assert!(
            matches!(s3.phase, Phase::Steady { talk: None }),
            "CancelChoice→Close→Interrupted で Steady{{None}} へ復帰する（DD-11）"
        );
        assert!(done_actions.is_empty());
        // 4) 以降に届く当該選択待ち宛の選択確定は棄却される（Req7.5）。
        let (s4, late, ev2) = step_capturing(
            s3,
            Input::Choice(choice_input_of("OnMenu", "メニュー", &[])),
            &cfg,
        );
        assert!(late.is_empty(), "解除後の選択確定は Action を発行しない（Req7.5）");
        assert!(s4.choice.is_none(), "棄却は帳簿を復活させない");
        assert_logged(&ev2, Level::WARN, "choice_rejected_no_wait");
    }

    /// 選択待ち中の周期送出（choosing）が、解除後の pump からは消える（Req6.2 のタイムアウト側）。
    #[test]
    fn tick_after_timeout_cancel_drops_choosing() {
        let cfg = config();
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
        let (cancelled, _) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::NoContent,
                origin: "OnChoiceTimeout",
            },
            &cfg,
        );
        let (_next, tick_actions) = step(
            cancelled,
            Input::Tick {
                now: MonotonicMs(40_000),
            },
            &cfg,
        );
        assert_eq!(
            status_wire(&tick_actions[0]),
            Some("talking".to_string()),
            "解除後は choosing が消える（Req6.2）"
        );
    }

    // ============================================================
    // 帳簿の掃除・選択起因の失敗例外（タスク 4.6・C4 規則 7／8・DD-12・Req1.3／4.5／6.2）
    // ============================================================

    use crate::msg::ShioriFailure;
    use crate::schedule::TermCause;
    use crate::schedule::log_capture::{assert_no_error_logs, assert_not_logged};

    /// 規則 7 の不変条件: 帳簿があるなら、その対象 talk は現行 talk と一致する。
    ///
    /// 「帳簿の対象と現行トークが食い違う状態を残さない」ことの直接表明であり、掃除点ごとの
    /// 個別 assert（`choice.is_none()`）と併せて各掃除点で突合する。
    fn assert_choice_invariant(state: &State) {
        let active = match &state.phase {
            Phase::Steady { talk: Some(active) } => Some(active.talk_id),
            _ => None,
        };
        if let Some(ledger) = state.choice.as_ref() {
            assert_eq!(
                Some(ledger.talk_id),
                active,
                "帳簿の対象と現行トークが食い違う状態を残してはならない（C4 規則 7）"
            );
        }
    }

    // --- A. 帳簿の掃除（規則 7・Req1.3／6.2） ---

    /// 規則 7: 対象トークの完了（`TalkDone{Ended}`）で帳簿を消去する。
    #[test]
    fn talk_done_of_target_talk_clears_choice_ledger() {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
        let (next, actions) = step(
            s,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Ended,
            }),
            &config(),
        );
        assert!(matches!(next.phase, Phase::Steady { talk: None }));
        assert!(next.choice.is_none(), "対象トークの完了で帳簿は消える（規則 7）");
        assert!(actions.is_empty());
        assert_choice_invariant(&next);
    }

    /// 規則 7: 対象トークの `TalkDone{Quit}`（終了系列直行）でも帳簿を消去する。
    #[test]
    fn talk_done_quit_clears_choice_ledger() {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
        let (next, actions) = step(
            s,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Quit,
            }),
            &config(),
        );
        assert!(matches!(
            next.phase,
            Phase::Unloading {
                cause: TermCause::Quit
            }
        ));
        assert!(next.choice.is_none(), "終了系列（Quit）へ進む際も帳簿は消える");
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
        assert_choice_invariant(&next);
    }

    /// 規則 7: **マウス由来**の slot 置換でも帳簿を消去し、1 世代保持も持ち越さない。
    #[test]
    fn mouse_slot_replacement_clears_choice_ledger_and_stale_slot() {
        let mut s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
        s.choice_prev_talk = Some(TalkId(1));
        let (next, actions) = step(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Value(r"\0つつかれた\e".to_string()),
                origin: "OnMouseDoubleClick",
            },
            &config(),
        );
        match next.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(6), "マウス由来 Value は slot を置換する"),
            _ => panic!("expected Steady{{Some}} replaced"),
        }
        assert!(matches!(actions.as_slice(), [Action::StartTalk(_)]));
        assert!(
            next.choice.is_none(),
            "slot 置換（マウス由来含む）で帳簿は消える（規則 7）"
        );
        assert!(
            next.choice_prev_talk.is_none(),
            "次の slot 差替で 1 世代保持は消える（規則 9）"
        );
        assert_choice_invariant(&next);
    }

    /// 規則 7: close 握手の開始（`ClosePending` への遷移）で帳簿を消去する。
    ///
    /// 経路は正規: active talk 中の `CloseRequest` は保留記録のみ（選択待ちは継続——ここで
    /// 帳簿を消すと選択が棄却され、バリアが解けず `TalkDone` が来ないため握手が進まない）。
    /// 掃除が起きるのは実際に握手へ**遷移する**点（`TalkDone` 消化→`ClosePending`）である。
    #[test]
    fn close_handshake_transition_clears_choice_ledger() {
        let cfg = config();
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
        let (pending, actions) = step(
            s,
            Input::CloseRequest {
                reason: CloseReason::User,
            },
            &cfg,
        );
        assert!(actions.is_empty(), "active talk 中の close は保留記録のみ");
        assert!(
            pending.choice.is_some(),
            "保留記録は遷移ではない——選択待ちは継続する（バリアを解けなくしない）"
        );
        assert_choice_invariant(&pending);
        let (next, close_actions) = step(
            pending,
            Input::TalkDone(TalkDone {
                talk_id: TalkId(3),
                reason: TalkEndReason::Ended,
            }),
            &cfg,
        );
        assert!(matches!(next.phase, Phase::ClosePending { .. }));
        assert_eq!(close_actions.len(), 1, "握手開始は OnClose GET を発行する");
        assert!(next.choice.is_none(), "close 系遷移で帳簿は消える（規則 7）");
        assert_choice_invariant(&next);
    }

    /// 規則 7: `ForceQuit` の横断遷移で帳簿を消去する。
    #[test]
    fn force_quit_clears_choice_ledger() {
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["OnMenu"],
            ChoicePhase::Cascading {
                choice_id: "OnMenu".to_string(),
                next: None,
            },
        );
        let (next, actions) = step(
            s,
            Input::ForceQuit {
                reason: CloseReason::System,
            },
            &config(),
        );
        assert!(matches!(
            next.phase,
            Phase::Unloading {
                cause: TermCause::Forced
            }
        ));
        assert_eq!(actions.len(), 2, "既存の ForceQuit 発行列は不変");
        assert!(next.choice.is_none(), "終了系遷移で帳簿は消える（規則 7）");
        assert_choice_invariant(&next);
    }

    /// 規則 7: `ShioriDown`（Fault 直行）で帳簿を消去する。
    #[test]
    fn shiori_down_clears_choice_ledger() {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting);
        let (next, actions) = step(
            s,
            Input::ShioriDown {
                reason: "helper crashed".to_string(),
            },
            &config(),
        );
        assert!(matches!(
            next.phase,
            Phase::Unloading {
                cause: TermCause::Fault
            }
        ));
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
        assert!(next.choice.is_none(), "終了系遷移で帳簿は消える（規則 7）");
        assert_choice_invariant(&next);
    }

    // --- B. 選択起因の失敗例外（規則 8・DD-12・Req4.5） ---

    /// DD-12: **`step()` 経由**でカスケード段の `Failed` を受けても `Unloading{Fault}` へ倒れず、
    /// 204 相当（残段ありなら次段 GET）で継続する（横断 Failed アームの免除）。
    #[test]
    fn cascade_failed_via_step_does_not_fall_into_unloading_fault() {
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["choice1"],
            ChoicePhase::Cascading {
                choice_id: "choice1".to_string(),
                next: Some(CascadeNext::Select),
            },
        );
        let (next, actions, ev) = step_capturing(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                origin: "OnChoiceSelectEx",
            },
            &config(),
        );
        assert!(
            !matches!(next.phase, Phase::Unloading { .. }),
            "選択由来の SHIORI 失敗で終了系列へ倒れない（Req4.5・DD-12）"
        );
        assert!(
            !actions.iter().any(|a| matches!(a, Action::ShioriUnload)),
            "免除アームは Unload を発行しない"
        );
        let (id, refs) = expect_get_call(&actions[0]);
        assert_eq!(id, "OnChoiceSelect", "失敗は 204 と同一＝次段へ前進（Req2.3）");
        assert_eq!(refs, vec!["choice1".to_string()]);
        assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
        assert_not_logged(&ev, "shiori_failed");
    }

    /// DD-12: 最終段の `Failed` も `step()` 経由で解決のみ（起動なし）へ倒れる。
    #[test]
    fn cascade_failed_at_last_stage_via_step_resolves_and_continues() {
        let s = steady_with_ledger(
            TalkId(3),
            6,
            &["choice1"],
            ChoicePhase::Cascading {
                choice_id: "choice1".to_string(),
                next: None,
            },
        );
        let (next, actions, ev) = step_capturing(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Ipc("pipe closed".to_string())),
                origin: "OnChoiceSelect",
            },
            &config(),
        );
        assert!(!matches!(next.phase, Phase::Unloading { .. }));
        match actions.as_slice() {
            [Action::ResolveChoice { talk_id, id }] => {
                assert_eq!(*talk_id, TalkId(3));
                assert_eq!(id, "choice1");
            }
            _ => panic!("最終段の失敗は ResolveChoice のみ（Req5.3）"),
        }
        assert!(next.choice.is_none());
        assert_choice_invariant(&next);
        assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
    }

    /// DD-12: タイムアウト GET の `Failed` も `step()` 経由で `CancelChoice` へ進む。
    #[test]
    fn timeout_failed_via_step_cancels_without_unloading() {
        let s = steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::TimeoutInFlight);
        let (next, actions, ev) = step_capturing(
            s,
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                origin: "OnChoiceTimeout",
            },
            &config(),
        );
        assert!(
            !matches!(next.phase, Phase::Unloading { .. }),
            "タイムアウト GET の失敗でも終了系列へ倒れない（Req4.5・DD-12）"
        );
        match actions.as_slice() {
            [Action::CancelChoice { talk_id }] => assert_eq!(*talk_id, TalkId(3)),
            _ => panic!("タイムアウト失敗は CancelChoice のみ（Req7.5）"),
        }
        assert!(next.choice.is_none());
        assert_logged(&ev, Level::ERROR, "choice_shiori_failed_as_204");
        assert_logged(&ev, Level::INFO, "choice_timeout_cancelled");
    }

    /// DD-12 の免除は **choice in-flight に限る**——選択待ち（`Waiting`）中や帳簿不在の
    /// `Failed` は従来どおり `Unloading{Fault}` へ倒れる（非 choice 経路を巻き込まない）。
    #[test]
    fn non_choice_failed_still_falls_into_unloading_fault() {
        let cfg = config();
        // 帳簿なし（既存の pump 失敗経路）。
        let (next, actions) = step(
            steady_some(TalkId(3), 6),
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                origin: "OnSecondChange",
            },
            &cfg,
        );
        assert!(matches!(
            next.phase,
            Phase::Unloading {
                cause: TermCause::Fault
            }
        ));
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
        // 帳簿はあるが応答待ちではない（`Waiting`）＝当該 Failed は choice 起因ではない。
        let (next, actions) = step(
            steady_with_ledger(TalkId(3), 6, &["OnMenu"], ChoicePhase::Waiting),
            Input::ShioriReply {
                outcome: ShioriOutcome::Failed(ShioriFailure::Timeout("30s".to_string())),
                origin: "OnSecondChange",
            },
            &cfg,
        );
        assert!(
            matches!(
                next.phase,
                Phase::Unloading {
                    cause: TermCause::Fault
                }
            ),
            "選択待ち中の pump 失敗は免除対象ではない（規則 8 の条件は in-flight のみ）"
        );
        assert!(matches!(actions.as_slice(), [Action::ShioriUnload]));
        assert!(next.choice.is_none(), "終了系遷移で帳簿は消える（規則 7）");
    }

    // --- C. 正常系のユーザー操作は error を出さない（完了状態） ---

    /// 完了状態: 選択の happy path（受理→カスケード→Value→解決→起動→旧 talk の遅延 Done）で
    /// **error レベルのログが 1 件も出ない**（1 世代 stale 防御が `unknown_talk_done` を封じる）。
    #[test]
    fn choice_happy_path_emits_no_error_level_logs() {
        let cfg = config();
        let mut phases = None;
        let ev = capture(|| {
            // 1) 選択待ち成立の通知 → 帳簿確立。
            let (s1, _) = step(
                steady_some(TalkId(3), 6),
                Input::ChoiceWaiting {
                    talk_id: TalkId(3),
                    choice_ids: vec!["OnMenu".to_string()],
                    display_end: MonotonicMs(10_000),
                    timeout_directive_secs: None,
                },
                &cfg,
            );
            // 2) ユーザーの選択確定 → 任意名イベントの GET。
            let (s2, staged) = step(
                s1,
                Input::Choice(choice_input_of("OnMenu", "メニュー", &["a0"])),
                &cfg,
            );
            // 3) Value 応答 → [ResolveChoice, StartTalk]・slot 差替。
            let (s3, resolved) = step(
                s2,
                Input::ShioriReply {
                    outcome: ShioriOutcome::Value(r"\0次のシーン\e".to_string()),
                    origin: "OnChoiceEvent",
                },
                &cfg,
            );
            // 4) 旧 talk の遅延 Done（F1 残余レース）→ 1 世代 stale 防御で info 棄却。
            let (s4, late) = step(
                s3,
                Input::TalkDone(TalkDone {
                    talk_id: TalkId(3),
                    reason: TalkEndReason::Ended,
                }),
                &cfg,
            );
            phases = Some((staged, resolved, late, s4));
        });
        let (staged, resolved, late, s4) = phases.expect("全 step が完了する");
        assert_eq!(expect_get_call(&staged[0]).0, "OnMenu", "任意名 1 段のみ");
        assert!(matches!(
            resolved.as_slice(),
            [Action::ResolveChoice { .. }, Action::StartTalk(_)]
        ));
        assert!(late.is_empty(), "遅延 Done は Action を発行しない");
        match s4.phase {
            Phase::Steady {
                talk: Some(ActiveTalk { talk_id, .. }),
            } => assert_eq!(talk_id, TalkId(6), "遅延 Done は新 talk を殺さない"),
            _ => panic!("expected Steady{{Some(new)}} preserved"),
        }
        assert_logged(&ev, Level::INFO, "talk_done_stale_choice");
        assert_no_error_logs(&ev);
    }
}
