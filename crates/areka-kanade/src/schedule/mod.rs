//! schedule — 純粋運行状態機械（`src/schedule/`）。
//!
//! 全運行判断を純粋関数 [`step`] として実装する層である（I/O・スレッド・channel
//! 非依存＝決定的単体テストの本体）。[`step`] は現在の [`State`] と [`Input`] から
//! 次の [`State`] と副作用指示 [`Action`] の列を返す唯一の遷移入口であり、`tracing`
//! によるログ発行以外の副作用を持たない（可観測性の側効果であり状態・出力の決定性に
//! 影響しない・DD-3）。DD-9 により本モジュールは `pub(crate)` に閉じる。
//!
//! # 責務分割（本タスク 2.1 の担当範囲）
//! 本 `mod.rs` は**由来・状態を問わない横断遷移**（[`Input::TalkDone`] の理由が
//! [`TalkEndReason::Quit`] の場合／[`Input::ForceQuit`]／[`Input::ShioriDown`]／[`Input::ShioriReply`] の
//! 失敗）と、Unload 完了・防御アーム（未知 talk_id・Idle 以外の Boot・応答待ちでない
//! ShioriReply）を実装する。フェーズ固有の遷移は [`boot`]／[`steady`]／[`close`] の
//! 各サブモジュールへ委譲する。
//!
//! # ログ規律（steering: areka-log-first-no-silent-failure）
//! すべての失敗・防御アームは `tracing::error!`／`tracing::warn!` を発行する。沈黙の
//! 失敗経路は存在しない。panic は新規導入しない（回復不能はすべて
//! `Unloading{Fault}`→`Stopped` の正規遷移で表現する・Req 6.4）。

use crate::msg::{CloseReason, KanadeConfig, ShioriFault, ShioriOutcome};
use crate::status::ExecutionStateUpdate;
use crate::talk::{ScriptOrigin, StartTalk, TalkDone, TalkEndReason, TalkId};
// テストの `use super::*` が引く名前（型の塊を `types.rs` へ出す前と同じ顔ぶれを保つ）。
#[cfg(test)]
use crate::msg::{ChoiceInput, MonotonicMs, ShioriCall};
#[cfg(test)]
use crate::status::ExecutionSnapshot;

/// アンカーの選択の受理と 2 段の送出（帳簿なし・段の記憶だけ）。
pub(crate) mod anchor;
/// バルーンの 3 つのイベントを送るかどうかの判断と、その控え（トークの控え・中断の控え）。
pub(crate) mod balloon_events;
pub(crate) mod boot;
pub(crate) mod change;
pub(crate) mod choice;
pub(crate) mod close;
/// ukadoc Reference 表の実装正本（純粋関数群）。DD-9 の例外として `pub`。
/// クレート公開面への露出は [`crate::events`] ファサード経由（[`crate::lib`] 参照）。
pub mod events;
/// タスク 6.1: 純粋 step 層の失敗・防御アームのログ発火検証（テスト専用）。
#[cfg(test)]
pub(crate) mod log_capture;
/// SHIORI Resource 照会の許可集合（イベント檻とは別族・Req4.1）。DD-9 の例外として `pub`。
/// submit ガードはイベント許可 ∨ リソース許可で判定する（`crate::actor` の egress チョークポイント）。
pub mod resources;
/// script 種別のログ（表示の語・出口の記録）。クレート公開面へは [`crate::script_log`] ファサード経由。
pub mod script_log;
/// SSTP の入口（外からの台本・外から頼まれたイベント）。
pub(crate) mod sstp;
pub(crate) mod steady;
pub(crate) mod talk_gap;
/// SHIORI の応答で始まるトークの出どころ（出口の付与と引き下げの記録）。
mod talk_origin;
/// 運行表の翻訳（`OnTranslate`）——帳簿の型・元のイベントの控え・応答の読み。
pub(crate) mod translate;
/// 既存テスト用の「翻訳を台詞そのままで通す」補助（テスト専用）。
#[cfg(test)]
pub(crate) mod translate_test_support;
/// 運行表の型の塊（同じ名前で見せる）。
mod types;
pub(crate) mod user_break;

pub(crate) use types::{
    Action, ActiveTalk, CascadeNext, ChoicePhase, ChoiceState, Input, Phase, State, TermCause,
};

/// 選択帳簿が **choice in-flight**（選択由来の SHIORI 呼出の応答待ち）かを判定する（DD-12）。
///
/// [`choice_phase_active`]（＝選択待ち継続中・`choosing` の源）とは別軸である: `Waiting` は
/// 選択待ち継続中だが in-flight ではない。横断 `Failed`→`Unloading{Fault}` の免除
/// （[`on_shiori_reply`] の先行アーム・C4 規則 8）は in-flight のときだけ効かせる——選択待ち中に
/// 届く `Failed` は pump／マウス由来であり、免除すれば SHIORI 失敗の終了規律（Req6.1）が壊れる。
///
/// wildcard を置かないため、段フェーズの追加時は本表での判断がコンパイル時に要求される。
fn choice_in_flight(phase: &ChoicePhase) -> bool {
    match phase {
        ChoicePhase::Cascading { .. } | ChoicePhase::TimeoutInFlight => true,
        ChoicePhase::Waiting => false,
    }
}

/// 選択帳簿を消去する単一の掃除ヘルパ（C4 規則 7・Req1.3／6.2）。
///
/// 規則 7 の不変条件は「帳簿の対象 talk ≠ 現行 talk なら即 `None`」である。その不変条件が破れる
/// 遷移点——**対象トークの完了・slot 置換（マウス由来を含む）・close 系遷移**——で本関数を呼び、
/// 帳簿の対象と現行トークが食い違う状態を残さない。掃除は状態遷移であって失敗ではないため
/// `trace!` で観測する（沈黙で捨てる経路は作らない・log-first）。`at` は掃除点の識別子である。
///
/// 棄却経路（規則 1 の受領検証）からは呼ばない——棄却の定義は**状態不変**であり、既存帳簿を
/// 含めて一切書き換えないことが規則 1 の要求だからである。
pub(super) fn clear_choice_ledger(state: &mut State, at: &'static str) {
    if let Some(ledger) = state.choice.take() {
        tracing::trace!(
            target: "kanade",
            event = "choice_ledger_cleared",
            at = at,
            talk_id = ledger.talk_id.0,
            stage = steady::choice_phase_label(&ledger.phase),
            "選択帳簿を消去——帳簿の対象と現行トークの食い違いを残さない（C4 規則 7）"
        );
    }
}

/// 台本 1 つを新しいトークとして始める（採番・枠の差し替え・再生開始の行動を返す）。
///
/// 選択肢の帳簿・`choice_prev_talk`・解決・記録は呼び手の仕事で、ここでは触らない。
/// `label` は `ActiveTalk::origin`（記録用の綴り）、`origin` は再生の開始に付ける出どころ。
pub(in crate::schedule) fn start_talk(
    state: &mut State,
    label: &'static str,
    script: String,
    origin: ScriptOrigin,
) -> (TalkId, Action) {
    let talk_id = TalkId(state.next_talk_id);
    state.next_talk_id += 1;
    state.phase = Phase::Steady {
        talk: Some(ActiveTalk {
            talk_id,
            origin: label,
            script: script.clone(),
        }),
    };
    let mut start = StartTalk::new(talk_id, script);
    start.origin = origin;
    (talk_id, Action::StartTalk(start))
}

/// 唯一の遷移入口。現在の [`State`] と [`Input`] から次の [`State`] と副作用指示
/// [`Action`] の列を返す純粋関数（`tracing` ログ発行のみ側効果として許容）。
///
/// 本タスク 2.1 は**横断遷移**（由来・状態を問わず終了系列へ進む共通ロジック）と
/// 防御アームを実装し、フェーズ固有の遷移は各サブモジュールへ委譲する。処理順は
/// 「横断遷移を先に判定 → 該当しなければフェーズ分岐」である。
///
/// どの入力でも、遷移の前後で元のイベントの控えと翻訳の出口の規則（[`translate::before`]・
/// [`translate::after`]）を扱い、遷移の後に台詞の切れ目の見極め（[`talk_gap::observe`]）を 1 回だけ走らせる
/// （見張りが無ければ何もしない）。
pub(crate) fn step(mut state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    // 印のイベントの応答なら、遷移の前のトークを控える（応答で台詞が始まったかを後で突き合わせる）。
    let marked_reply = talk_gap::marked_reply(&state, &input);
    // 応答の入力なら、応答を待っていた GET の元のイベントの控えを取り出す（1 回だけ使う）。
    let replied = translate::before(&mut state, &input);
    // この入力で始まるトークの出どころ（script 種別のログの表示の語の素・入力が消費される前に控える）。
    let talk_source = script_log::source_of(&state, &input, &replied);
    let reply_origin = replied.reply_origin();
    let (mut state, actions) = route(state, input, config);
    // 翻訳へ預ける前に、応答で始まるトークへ出どころを付ける（腕は触らない）。
    let actions = talk_origin::stamp(reply_origin, actions);
    let actions = translate::after(&mut state, replied, actions);
    talk_gap::observe(&mut state, marked_reply);
    // 出口: 相手を失った控えの掃除と、再生を始めたトークの控え（最終の台本で取る）。
    balloon_events::settle(&mut state, &actions);
    script_log::record(config, &actions, talk_source.as_ref());
    (state, actions)
}

/// [`step`] の入力の振り分け（横断遷移 → フェーズ固有遷移への委譲）。
fn route(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match input {
        // --- 横断遷移: 由来・状態を問わず終了系列へ進む共通ロジック ---

        // ForceQuit（全 Phase・DD-10）: best-effort OnClose NOTIFY を Action 先頭に積み、
        // quit ゲートを迂回して Unloading{Forced} へ直行する（Req 4.4）。
        Input::ForceQuit { reason } => force_quit(state, reason),

        // 死活報告（暫定 seam・DD-4）: error! 記録の上 Unloading{Fault} へ（Req 5.4）。
        Input::ShioriDown { kind, reason } => {
            tracing::error!(target: "kanade", event = "shiori_down", reason = %reason, "SHIORI 死活報告を受領——終了系列（Fault）へ");
            to_unloading_fault(state, ShioriFault::from_down(kind, reason))
        }

        // TalkDone: reason（3 値）・talk_id 突合を横断的に判定する（Req 2.5・4.3・6.2）。
        Input::TalkDone(done) => on_talk_done(state, done, config),

        // ShioriReply: Unload 完了・呼出失敗（Failed）の横断判定を先に行い、
        // それ以外は応答待ちフェーズへ委譲する（Req 6.1）。origin は応答の出所（DD-IE-3）。
        // 任意の名前の GET の応答待ちの控えが在れば、通常の応答の処理より先に sstp::on_notify_reply が
        // 控えを取り出す（失敗などは向こうから on_shiori_reply へ戻る）。
        Input::ShioriReply { outcome, origin } if state.sstp_notify.is_some() => {
            sstp::on_notify_reply(state, outcome, origin, config)
        }
        Input::ShioriReply { outcome, origin } => on_shiori_reply(state, outcome, origin, config),

        // Mouse（DD-IE-8）: Steady でのみ受理し steady::on_mouse へ委譲する。他フェーズ
        // （boot／close／terminate 後）では状態を変えず安全に無視する。boot 時のマウス移動は
        // 正常な環境入力ゆえ warn ではなく trace で観測する（沈黙の無視経路は作らない）。
        Input::Mouse(m) => match state.phase {
            Phase::Steady { .. } => steady::on_mouse(state, m),
            _ => {
                tracing::trace!(
                    target: "kanade",
                    event = "mouse_input_ignored",
                    phase = phase_label(&state.phase),
                    input = ?m,
                    "非 Steady フェーズのマウス入力——状態を変えず無視（DD-IE-8）"
                );
                (state, Vec::new())
            }
        },

        // ChoiceWaiting（C4 規則 4・Req7.1）: Steady でのみ受理し steady::on_choice_waiting へ
        // 委譲する（現行トークとの識別子突合・DD-8 の期限写像・帳簿確立は委譲先の責務）。
        // 非 Steady フェーズ（boot 中・close 握手以降・終了系列）には受理すべき選択待ちが
        // 構造上存在しないため、帳簿を確立せず warn 記録の上で棄却する（状態不変）。
        Input::ChoiceWaiting {
            talk_id,
            choice_ids,
            display_end,
            timeout_directive_secs,
        } => match state.phase {
            Phase::Steady { .. } => steady::on_choice_waiting(
                state,
                talk_id,
                choice_ids,
                display_end,
                timeout_directive_secs,
                config,
            ),
            _ => {
                tracing::warn!(
                    target: "kanade",
                    event = "choice_waiting_stale",
                    reason = "non_steady_phase",
                    talk_id = talk_id.0,
                    choice_count = choice_ids.len(),
                    phase = phase_label(&state.phase),
                    "非 Steady フェーズの選択待ち通知——帳簿を確立せず棄却（C4 規則 4）"
                );
                (state, Vec::new())
            }
        },

        // Choice（C4 規則 1・Req1.1／1.3）: Steady でのみ受理し steady::on_choice へ委譲する
        // （帳簿突合・候補照合・段列決定は委譲先の責務）。非 Steady フェーズ（boot 中・close
        // 握手以降・終了系列）には受理すべき選択待ちが構造上存在しないため、状態を一切変えず
        // warn 記録の上で棄却して処理を継続する（沈黙の棄却経路は作らない）。
        Input::Choice(c) => match state.phase {
            Phase::Steady { .. } => steady::on_choice(state, c),
            _ => {
                tracing::warn!(
                    target: "kanade",
                    event = "choice_rejected_no_wait",
                    reason = "non_steady_phase",
                    choice_id = %c.id,
                    scope = c.scope,
                    reference_count = c.references.len(),
                    phase = phase_label(&state.phase),
                    "非 Steady フェーズの選択確定——状態不変で棄却（C4 規則 1）"
                );
                (state, Vec::new())
            }
        },

        // Anchor: 受理の規則（定常だけ・それ以外は警告で棄却）ごと anchor::on_anchor へ渡す。
        Input::Anchor(a) => anchor::on_anchor(state, a),

        // UserBreak: 場面で振り分けず、受理の規則ごと user_break::on_user_break へ渡す。
        Input::UserBreak { scope } => user_break::on_user_break(state, scope),

        // ChangeGhost: 受理の規則ごと change::on_change_ghost へ渡す。
        Input::ChangeGhost(req) => change::on_change_ghost(state, req),

        // RaiseEvent: 許可表と定常の判定ごと change::on_raise_event へ渡す。
        Input::RaiseEvent {
            id,
            references,
            method,
        } => change::on_raise_event(state, id, references, method),

        // AwaitTalkGap: 始め方ごと talk_gap::begin へ渡す（結果は遷移の後の見極めが決める）。
        Input::AwaitTalkGap { raise } => talk_gap::begin(state, raise),

        // ExecutionState: 相を問わず（終了中・停止後も）写しを更新するだけ（行動は返さない）。
        Input::ExecutionState(update) => on_execution_state(state, update),

        // TranslateDone: 預けた一括の再開・故障・帳簿なしの判断ごと translate::on_done へ渡す。
        Input::TranslateDone(result) => translate::on_done(state, result),

        // BalloonTimeout: 番号の照合と送るかどうかごと balloon_events::on_timeout_notice へ渡す（表 D）。
        Input::BalloonTimeout { talk_id } => balloon_events::on_timeout_notice(state, talk_id),

        // SstpSend: 受け付けの判定ごと sstp::on_send へ渡す。
        Input::SstpSend { script, strict } => sstp::on_send(state, script, strict),

        // SstpNotify: 受け付けの判定と引数の検査ごと sstp::on_notify へ渡す。
        Input::SstpNotify {
            event,
            references,
            strict,
        } => sstp::on_notify(state, event, references, strict),

        // --- 防御アーム・フェーズ固有遷移への委譲 ---

        // Idle 以外での Boot は不整合（warn!＋現 Phase 維持・Req 6.2）。Idle のみ boot へ委譲。
        Input::Boot => match state.phase {
            Phase::Idle => boot::step(state, Input::Boot, config),
            _ => {
                tracing::warn!(target: "kanade", event = "boot_ignored", "Idle 以外での Boot 指示を無視");
                (state, Vec::new())
            }
        },

        // Tick・CloseRequest はフェーズ固有遷移（本体は各サブモジュール）。
        Input::Tick { now } => dispatch_phase(state, Input::Tick { now }, config),
        Input::CloseRequest { reason } => {
            dispatch_phase(state, Input::CloseRequest { reason }, config)
        }
    }
}

/// 外から届いた実行状態の知らせを写しへ反映する横断の腕（相を問わず受理・行動なし）。
///
/// 捨てる経路を持たない（終了中・停止後も写す）ので `warn!` は無く、変化の有無を `trace!` で残す。
fn on_execution_state(mut state: State, update: ExecutionStateUpdate) -> (State, Vec<Action>) {
    let kind = match update {
        ExecutionStateUpdate::NoUserBreak(_) => "no_user_break",
        ExecutionStateUpdate::Balloons(_) => "balloons",
    };
    let changed = state.external.apply(update);
    tracing::trace!(
        target: "kanade",
        event = "execution_state_updated",
        kind,
        changed,
        phase = phase_label(&state.phase),
        "外から届いた実行状態を写しへ反映"
    );
    (state, Vec::new())
}

/// 相が再生中のトークを運ぶか（DD-IT-3・スナップショットの talk 軸）。
///
/// 再生中＝`current_talk_id` が再生中のトークの番号を引ける相（相の表はそこの 1 つだけ）。普段の会話
/// （`Steady{Some}`）・起動の挨拶（`BootVersion{Some}`・DD-IT-12）と、お別れの台詞の 3 つの場面＝終了の挨拶
/// （`CloseTalkWait`）・切り替えの送り出しの台詞（`ChangeTalkWait`）・切り替えの別れの台詞
/// （`ChangeCloseTalkWait`）が当たる。利用者の中断で止められる場面と `talking` の場面はこれで一致する。
/// 選択待ちと写しの 3 状態は `Phase` の外にあるため、スナップショットは [`State::snapshot`]／
/// [`State::snapshot_without_talk`] が組み立てる。
pub(crate) fn talk_active_of(phase: &Phase) -> bool {
    current_talk_id(phase).is_some()
}

/// フェーズの静的ラベル（ログ観測用）。`Phase` は Debug を持たないため、可観測性ログ
/// （例 `mouse_input_ignored`）に添える人間可読な識別子をここで与える。
fn phase_label(phase: &Phase) -> &'static str {
    match phase {
        Phase::Idle => "Idle",
        Phase::BootInit => "BootInit",
        Phase::BootPrefetch => "BootPrefetch",
        Phase::BootType => "BootType",
        Phase::BootMain => "BootMain",
        Phase::BootVersion { .. } => "BootVersion",
        Phase::Steady { .. } => "Steady",
        Phase::ClosePending { .. } => "ClosePending",
        Phase::CloseTalkWait { .. } => "CloseTalkWait",
        Phase::ChangePending => "ChangePending",
        Phase::ChangeTalkWait { .. } => "ChangeTalkWait",
        Phase::ChangeClosePending => "ChangeClosePending",
        Phase::ChangeCloseTalkWait { .. } => "ChangeCloseTalkWait",
        Phase::Unloading { .. } => "Unloading",
        Phase::Stopped => "Stopped",
    }
}

/// ForceQuit の横断遷移（DD-10）: best-effort OnClose NOTIFY を先頭に積み Unloading{Forced} へ。
///
/// OnClose NOTIFY の構築は events.rs（[`events::on_close_notify`]）へ委譲する——events.rs が
/// `ShioriCall` 構築の単一列挙点であり、force_quit はもはや inline 構築しない（DD-IT-8）。
/// スナップショットは Unloading へ遷移**後**の会話なしの作り方（[`State::snapshot_without_talk`]・
/// online と balloon だけ写しから残す）を渡す（DD-IT-4・要件 5.1）。
fn force_quit(mut state: State, reason: CloseReason) -> (State, Vec<Action>) {
    tracing::warn!(target: "kanade", event = "force_quit", reason = reason.as_ref_str(), "強制終了指示——終了系列（Forced）へ直行");
    state.phase = Phase::Unloading {
        cause: TermCause::Forced,
    };
    // close 系遷移の掃除点（C4 規則 7）: 現行トークは失われるため選択帳簿を残さない。
    clear_choice_ledger(&mut state, "force_quit");
    let notify = Action::ShioriRequest(events::on_close_notify(
        reason,
        &state.snapshot_without_talk(),
    ));
    (state, vec![notify, Action::ShioriUnload])
}

/// 終了系列（Quit）の共通終端: Unloading{Quit}＋ShioriUnload。
///
/// 到達点は 3 つある——台本が `\-` に辿り着いた完了（`talk_done_quit`）、利用者の中断で
/// 止めた台本が終了を予約していた完了（`talk_done_break_quit`）、そして別れの台詞の完了
/// （[`close`] の `close_talk_done_quit`・終わり方を問わない）である。いずれも同じ遷移を採る
/// （設計「終了の予約」「close の規則改訂」）。`at` は対象トークの完了かつ close 系遷移の
/// 掃除点（C4 規則 7）の識別子。
fn to_unloading_quit(mut state: State, at: &'static str) -> (State, Vec<Action>) {
    state.phase = Phase::Unloading {
        cause: TermCause::Quit,
    };
    clear_choice_ledger(&mut state, at);
    (state, vec![Action::ShioriUnload])
}

/// 呼出失敗・死活報告の共通終端: Unloading{Fault}＋ShioriUnload（unload は best-effort）。
/// `fault` は失敗の種類と理由（停止通知まで運ぶ）。
fn to_unloading_fault(mut state: State, fault: ShioriFault) -> (State, Vec<Action>) {
    state.phase = Phase::Unloading {
        cause: TermCause::Fault(fault),
    };
    // close 系遷移の掃除点（C4 規則 7）。なお choice in-flight の `Failed` は本経路へ来ない
    // ——[`on_shiori_reply`] の先行アーム（DD-12）が steady へ委譲するためである。
    clear_choice_ledger(&mut state, "unloading_fault");
    (state, vec![Action::ShioriUnload])
}

/// reason（3 値）・talk_id 突合。既知 talk の `TalkEndReason::Quit` は横断的に終了系列（Quit）へ。
///
/// `Ended` と `Interrupted` はいずれも非 quit としてフェーズ固有遷移（定常復帰・別れの台詞の
/// 終了）へ委譲する（設計「kanade schedule の 3 値写像」）。例外は 1 つだけで、利用者の中断で止めた
/// 台本が終了を予約していた `Interrupted`（[`user_break::is_break_quit`] が真）は `Quit` と
/// 同じ終了系列へ進む。dispatcher の slot 差替に伴う `Interrupted` は dispatcher が stale として
/// 破棄するため、ここへ届く `Interrupted` は利用者の中断か選択肢の時間切れの解除である。
/// いずれの場合も専用状態は起こさず、`info!` でどの reason だったかを観測する。
fn on_talk_done(mut state: State, done: TalkDone, config: &KanadeConfig) -> (State, Vec<Action>) {
    match current_talk_id(&state.phase) {
        Some(active) if active == done.talk_id => {
            // 現 talk の完了に到達した時点で 1 世代 stale 帳簿の役目は終わる（C4 規則 9）。
            // 保持を延長すると「1 世代のみ」の契約が壊れ、真に未知の id まで info へ降格し得る。
            state.choice_prev_talk = None;
            // 印の台詞（`OnShellChanging`）の利用者の中断かは、中断の帳簿を空にする前に見る。
            let marked_break = talk_gap::is_marked_break(&state, &done);
            talk_gap::note_marked_done(&mut state, &done, marked_break);
            // 現行トークの完了なので中断の帳簿はここで必ず空になる（終わり方を問わない・不変条件）。
            // 空にした結果が「利用者の中断で終わり、かつ終了の予約があった」かを持ち帰る（Req 3.8）。
            // 切替の相では予約を終了へ結ばない（中断は切替の中止＝切替の要件 5.4）。印の台詞の
            // 中断も終了へ結ばない（シェル切替の中止＝areka-P0-shell-balloon-switch 要件 5.2）。
            let broke = user_break::take_break(&mut state, &done);
            let break_quit = user_break::is_break_quit(broke, &done)
                && !change::is_change_phase(&state.phase)
                && !marked_break;
            let (state, actions) = match done.reason {
                TalkEndReason::Quit => {
                    // 既知 talk の Quit → 終了系列（Quit）へ直行（Req 4.3）。
                    tracing::info!(target: "kanade", event = "talk_done_quit", talk_id = done.talk_id.0, "reason=Quit——終了系列（Quit）へ");
                    change::quit_dropping_pending(state, "talk_done_quit")
                }
                TalkEndReason::Interrupted if break_quit => {
                    // 利用者の中断で止めた台本が終了を予約していた——`\-` に辿り着いたのと
                    // 同じ終了へ進む（Req 3.8・設計「終了の予約」）。
                    tracing::info!(target: "kanade", event = "talk_done_break_quit", talk_id = done.talk_id.0, "利用者の中断で止めた台本が終了を予約していた——終了系列（Quit）へ");
                    change::quit_dropping_pending(state, "talk_done_break_quit")
                }
                // 保留の切替を持つトークの完了（最後まで・`\-` の予約なしの中断とも）は切替の相へ。
                TalkEndReason::Interrupted | TalkEndReason::Ended
                    if change::has_pending(&state) =>
                {
                    change::consume_pending(state, done, config)
                }
                TalkEndReason::Interrupted => {
                    // 非 quit 扱い（観測用ログ）。本アームは元々「M1 では到達しない想定」の防御で
                    // あったが、**選択タイムアウトの解除経路により正規の到達点になった**（DD-11）:
                    // タイムアウト 204 → [`Action::CancelChoice`] → dispatcher が slot を維持したまま
                    // `Close` を転送 → talk が `TalkDone{Interrupted}` を正規送出 → ここへ到達 →
                    // フェーズ固有遷移（steady）が `Steady{None}` へ復帰させる（Req7.5）。
                    // 遷移・ログ語彙・レベルはいずれも無改変である（意味づけのみが変わった）。
                    tracing::info!(target: "kanade", event = "talk_done_interrupted_as_non_quit", talk_id = done.talk_id.0, "reason=Interrupted——非 quit 扱い・フェーズ固有遷移へ委譲（選択解除の正規到達点・DD-11）");
                    dispatch_phase(state, Input::TalkDone(done), config)
                }
                TalkEndReason::Ended => {
                    // Ended（定常復帰・close talk 完了）はフェーズ固有遷移へ委譲。
                    dispatch_phase(state, Input::TalkDone(done), config)
                }
            };
            // 振り分けの後の相を見て、バルーンの 3 つのイベントを送るかを決める（表 B）。
            balloon_events::after_talk_done(state, actions, &done, broke)
        }
        Some(_) | None => {
            // 1 世代 stale 防御（C4 規則 9・F1 残余レース・Req1.6）: choice 起因の slot 差替直後は、
            // 旧 talk の即時 `Done{Ended}`（再生層の即 settle）が dispatcher の slot 差替より前に
            // 投函され得る——この遅延 `Done` は**欠陥ではなく既知の順序レース**である。よって
            // 1 世代保持した旧 talk_id と照合し、一致するものは info で棄却して
            // `unknown_talk_done`（error）を真に未知の id 専用に保つ（正常系で error を出さない）。
            // 保持は消さない——消去点は現 talk の `TalkDone` 到達と次の slot 差替である（規則 9）。
            if state.choice_prev_talk == Some(done.talk_id) {
                tracing::info!(
                    target: "kanade",
                    event = "talk_done_stale_choice",
                    talk_id = done.talk_id.0,
                    "選択差替で置き換えた旧 talk の遅延完了通知——状態を変えず棄却（C4 規則 9・F1）"
                );
                return (state, Vec::new());
            }
            // 未知 talk_id の TalkDone → error!＋現 Phase 維持（Req 2.5・6.2）。
            tracing::error!(target: "kanade", event = "unknown_talk_done", talk_id = done.talk_id.0, "未知 talk_id の再生完了通知——現 Phase 維持で継続");
            (state, Vec::new())
        }
    }
}

/// ShioriReply の横断判定（Unload 完了・Failed）＋応答待ちフェーズ委譲。
///
/// `origin`（応答の出所イベント ID・DD-IE-3）は横断判定では使わないが、応答待ちフェーズへ
/// 委譲する際に `Input::ShioriReply` として保持したまま流す（タスク 2.2 のマウス GET origin 別
/// reply 政策がフェーズ固有遷移側で参照する）。
fn on_shiori_reply(
    state: State,
    outcome: ShioriOutcome,
    origin: &'static str,
    config: &KanadeConfig,
) -> (State, Vec<Action>) {
    // Unloading 中の応答は Unload 完了として扱う。Unloaded／Failed のいずれも Stopped へ
    // 進む（Failed は error! の上で終了系列を継続・Error Handling「Unload 失敗」行）。
    if matches!(state.phase, Phase::Unloading { .. }) {
        return unloading_reply(state, outcome);
    }

    // prefetch 応答（username GET）は横断 Failed→Fault 経路に載せない（起動を殺さない・R4.1）。
    // Value/NoContent/Failed の全てを boot::step が [`resources::ResourceOutcome`] へ写像し、sink 呼出
    // 指示＋完了固定ログを添えて OnFirstBoot へ続行する。Failed の Fault 化を封じるため横断判定より先に捌く。
    if matches!(state.phase, Phase::BootPrefetch) {
        return boot::step(state, Input::ShioriReply { outcome, origin }, config);
    }

    // choice in-flight（カスケード段／タイムアウト GET の応答待ち）の応答は横断 Failed→Fault 経路に
    // 載せない（選択の失敗でゴーストを終了させない・Req4.5／DD-12・C4 規則 8）。prefetch の先行アームと
    // **同型**に、横断判定より先に steady へ委譲する。委譲先（steady の choice 先行アーム）が Failed を
    // `choice_shiori_failed_as_204`（error）記録の上で 204 と同一に扱い、会話を止めずに継続させる。
    //
    // 条件を `Steady` かつ in-flight（`Cascading|TimeoutInFlight`）に限るのが要点である: 選択待ち
    // （`Waiting`）中に届く Failed は pump／マウス由来であり、免除すると SHIORI 失敗時の終了規律
    // （Req6.1）が非 choice 経路まで緩む。非 choice 経路は無改変で従来どおり Unloading{Fault} へ倒れる。
    //
    // アンカーのイベントの応答待ち（段の記憶がある間）も同じ免除に入れる。失敗は steady の先頭の
    // 腕（`anchor::on_anchor_reply`）が `anchor_shiori_failed_as_204` を残して 204 と同じに扱う。
    if matches!(state.phase, Phase::Steady { .. })
        && (state.anchor.is_some()
            || state
                .choice
                .as_ref()
                .is_some_and(|ledger| choice_in_flight(&ledger.phase)))
    {
        return steady::step(state, Input::ShioriReply { outcome, origin }, config);
    }

    // 応答待ちフェーズでの Failed は横断的に Unloading{Fault} へ（Req 6.1）。
    if let ShioriOutcome::Failed(ref failure) = outcome
        && awaits_reply(&state.phase)
    {
        tracing::error!(target: "kanade", event = "shiori_failed", error = %failure, "SHIORI 呼出失敗——終了系列（Fault）へ");
        return to_unloading_fault(state, ShioriFault::from_failure(failure));
    }

    // 応答待ちでない Phase への ShioriReply は構造上発生しない（防御アーム・Req 6.2）。
    if !awaits_reply(&state.phase) {
        tracing::warn!(target: "kanade", event = "unexpected_reply", "応答待ちでない Phase への SHIORI 応答を無視");
        return (state, Vec::new());
    }

    // 定常の NOTIFY の応答（pump・汎用の入口）は正常な完了で、何もしない（トークも相も変えない）。
    if matches!(outcome, ShioriOutcome::Notified) && matches!(state.phase, Phase::Steady { .. }) {
        tracing::trace!(target: "kanade", event = "steady_notify_reply", origin, "定常の NOTIFY の応答——何もしない");
        return (state, Vec::new());
    }

    // 正常応答（Value／NoContent／Notified）は応答待ちフェーズ固有遷移へ委譲（origin を保持）。
    dispatch_phase(state, Input::ShioriReply { outcome, origin }, config)
}

/// Unloading 中の ShioriReply: Unloaded／Failed とも Stopped＋StopSelf へ（Failed は error!）。
fn unloading_reply(mut state: State, outcome: ShioriOutcome) -> (State, Vec<Action>) {
    if let ShioriOutcome::Failed(ref failure) = outcome {
        tracing::error!(target: "kanade", event = "unload_failed", error = %failure, "Unload 失敗——終了系列は継続し停止する");
    }
    state.phase = Phase::Stopped;
    (state, vec![Action::StopSelf])
}

/// フェーズ固有遷移への委譲（boot／steady／close）。
fn dispatch_phase(state: State, input: Input, config: &KanadeConfig) -> (State, Vec<Action>) {
    match state.phase {
        Phase::Idle
        | Phase::BootInit
        | Phase::BootPrefetch
        | Phase::BootType
        | Phase::BootMain
        | Phase::BootVersion { .. } => boot::step(state, input, config),
        Phase::Steady { .. } => steady::step(state, input, config),
        Phase::ClosePending { .. } | Phase::CloseTalkWait { .. } => {
            close::step(state, input, config)
        }
        Phase::ChangePending
        | Phase::ChangeTalkWait { .. }
        | Phase::ChangeClosePending
        | Phase::ChangeCloseTalkWait { .. } => change::step(state, input, config),
        // 終了系列（Unloading／Stopped）に届いた非横断入力は防御的に無視する。
        Phase::Unloading { .. } | Phase::Stopped => {
            tracing::warn!(target: "kanade", event = "input_after_terminate", "終了系列で受領した入力を無視");
            (state, Vec::new())
        }
    }
}

/// 現フェーズが応答待ち（直前に GET/NOTIFY/unload を発行済み）かを判定する。
///
/// `Steady` も応答待ちに含める: Steady の Tick は `OnSecondChange`（GET/NOTIFY）を発行し、
/// その応答をなお `Steady` のまま受ける（in-flight ≤ 1・シェルが直後に `ShioriReply` を
/// 再投入する）。ここに Steady が無いと Steady の `ShioriReply` が防御アーム
/// （unexpected_reply）で握り潰され、OnSecondChange の Value/NoContent 処理が壊れる。
/// これにより Steady の Failed は `on_shiori_reply` 経由で `Unloading{Fault}` へ（Req 6.1・
/// pump 含む任意の SHIORI 失敗で M1 は放棄）、Value/NoContent/Notified は `dispatch_phase`
/// → `steady::step` へ正しく委譲される。
fn awaits_reply(phase: &Phase) -> bool {
    matches!(
        phase,
        Phase::BootInit
            | Phase::BootType
            | Phase::BootMain
            | Phase::BootVersion { .. }
            | Phase::Steady { .. }
            | Phase::ClosePending { .. }
            | Phase::ChangePending
            | Phase::ChangeClosePending
    )
}

/// 現フェーズが突合対象とする active talk の talk_id（無ければ None）。
fn current_talk_id(phase: &Phase) -> Option<TalkId> {
    match phase {
        // 挨拶追跡中の BootVersion も突合対象に含める（TalkDone が BootVersion 中に届いた場合の
        // 防御・DD-IT-12）。主要な突合は BootVersion→Steady 完了後の Steady{Some} で成立する。
        Phase::Steady {
            talk: Some(active), ..
        }
        | Phase::BootVersion {
            talk: Some(active), ..
        } => Some(active.talk_id),
        Phase::CloseTalkWait { talk_id, .. }
        | Phase::ChangeTalkWait { talk_id, .. }
        | Phase::ChangeCloseTalkWait { talk_id, .. } => Some(*talk_id),
        _ => None,
    }
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;

/// 運行表の型（`Action` ほか）の変種の網羅。
#[cfg(test)]
#[path = "schedule_variant_tests.rs"]
mod variant_tests;

/// 外から届いた実行状態の写しの決定論テスト。
#[cfg(test)]
#[path = "external_state_tests.rs"]
mod external_state_tests;

/// タスク 6.1: 純粋 step 層の失敗・防御アームがログを発火することの実行可能検証。
///
/// Req 6.3（ログ無しの失敗経路を持たない）・Req 6.1（区別語彙ごとにログ）を、コードレビュー
/// でなく**テスト**で担保する。各 `error!` / `warn!` アームを `step()`（または各サブモジュール
/// `step`）で駆動し、`log_capture` で捕捉したイベントに `target="kanade"`・所定の `event`・所定
/// レベル（ERROR/WARN）が存在することを表明する。ログが除去・語彙変更・レベル変更されると当該
/// テストが失敗する。
///
/// ルーティング上 `step()` からは到達不能な防御アーム（構造上発生しない系）は、当該サブモジュール
/// の `step` を直接駆動して検証する（各テストのコメントで明示）。これらは「あり得ない Phase/入力の
/// 組」に対する防御であり、直接駆動が唯一かつ正当な網羅手段である。
#[cfg(test)]
#[path = "schedule_log_firing_tests.rs"]
mod log_firing_tests;

/// 翻訳の経路（SHIORI の台詞で再生を始める 8 か所の腕）を最上位の `step` から通すテスト。
#[cfg(test)]
#[path = "translate_path_tests.rs"]
mod translate_path_tests;
