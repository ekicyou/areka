//! 定常運転の Phase 分岐（Req 2・3・pump ゲート・talk 調停・保留 close）。
//!
//! 本モジュールは [`Phase::Steady`] における Tick pump（OnSecondChange GET/NOTIFY の
//! 使い分け）・talk 調停（Value→StartTalk）・boot 中／active talk 中に受領した close
//! 指示の保留処理を担う。
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

use super::{events, Action, ActiveTalk, Input, Phase, State};
use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, MouseEventKind, MouseInput, ShioriOutcome};
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
/// を [`super::snapshot_of`] 由来のスナップショットから併送する（NOTIFY 化しない）。GET/NOTIFY を
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
    // 送出時点の Steady フェーズから Status を導出する（active talk 中は talking を併送・DD-IE-1）。
    let snapshot = super::snapshot_of(&state.phase);
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

/// Steady での Tick（pump ゲート・Req 3.1／3.4／DD-6）。
///
/// まず `last_now` を必ず更新する（時刻は発行有無に依らず進む・close 期限計算の基準）。
/// その上で:
/// - `talk: None` かつ `pending_close` あり → close 握手開始（OnClose GET・ClosePending へ）。
/// - `talk: None` かつ `pending_close` なし → OnSecondChange **GET**（Ref3=1・pump 問い合わせ）。
/// - `talk: Some` → OnSecondChange **NOTIFY**（Ref3=0・応答無視・pending_close は消化しない）。
fn on_tick(mut state: State, now: MonotonicMs) -> (State, Vec<Action>) {
    state.last_now = Some(now);
    // GET/NOTIFY・Ref3・Status を単一スナップショットから導出する（DD-IT-3）。snapshot_of は
    // Steady{None}→talk 非アクティブ（GET・Ref3=1・status 空）、Steady{Some}→talk アクティブ
    // （NOTIFY・Ref3=0・status talking）を与える——既存の wire 挙動を保存する。
    let snapshot = super::snapshot_of(&state.phase);
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
    match state.phase {
        Phase::Steady { talk: None } => match outcome {
            ShioriOutcome::Value(script) => {
                let talk_id = TalkId(state.next_talk_id);
                state.next_talk_id += 1;
                tracing::info!(target: "kanade", event = "steady_talk", talk_id = talk_id.0, origin = origin, "応答にスクリプト——再生起動");
                // origin は応答の出所（動的化・DD-IE-3）。pump なら "OnSecondChange"、マウスなら
                // 当該マウスイベント名がそのまま ActiveTalk のラベルに載る。
                state.phase = Phase::Steady {
                    talk: Some(ActiveTalk { talk_id, origin }),
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
                    state.phase = Phase::Steady {
                        talk: Some(ActiveTalk { talk_id, origin }),
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
        }
    }

    /// Steady{talk: Some(id)} を構築する。
    fn steady_some(talk_id: TalkId, next_id: u64) -> State {
        State {
            phase: Phase::Steady {
                talk: Some(ActiveTalk {
                    talk_id,
                    origin: "OnSecondChange",
                }),
            },
            last_now: Some(MonotonicMs(500)),
            next_talk_id: next_id,
            pending_close: None,
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
        assert_shiori(&actions[0], &events::on_second_change(now, &ExecutionSnapshot { talk_active: false }));
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
        assert_shiori(&actions[0], &events::on_second_change(now, &ExecutionSnapshot { talk_active: true }));
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
                talk: Some(ActiveTalk { talk_id, origin }),
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
                talk: Some(ActiveTalk { talk_id, origin }),
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
                talk: Some(ActiveTalk { talk_id, origin }),
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
        assert_shiori(&tick_actions[0], &events::on_second_change(now, &ExecutionSnapshot { talk_active: false }));
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
            &events::on_mouse_move(10, 20, 0, Some("Head"), &ExecutionSnapshot { talk_active: false }),
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
                &ExecutionSnapshot { talk_active: false },
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
                &ExecutionSnapshot { talk_active: false },
            ),
        );
    }

    // --- Steady{Some(active)} + Move → GET は抑止せず発行・Status: talking を帯びる（DD-IE-1） ---
    // active talk 中でもマウス GET は NOTIFY 化せず GET のまま。snapshot_of(Steady{Some}) から
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
            events::on_mouse_move(10, 20, 0, Some("Head"), &ExecutionSnapshot { talk_active: true });
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
}
