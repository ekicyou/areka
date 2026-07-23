//! ランタイム層: kanade アクターシェル（`src/actor.rs`）。
//!
//! [`spawn_kanade`] は運行状態機械（[`crate::schedule`]）を独立スレッドで駆動する
//! アクターシェルである。受領した [`KanadeMsg`] を [`Input`](crate::schedule::Input) へ
//! 写像して [`step`](crate::schedule::step) を呼び、返った [`Action`](crate::schedule::Action)
//! を実行する（SHIORI 往復・再生起動要求送出・停止）。SHIORI 呼出は handler 内で
//! oneshot 往復（`reply_channel`）を閉じ、結果を `Input::ShioriReply` として即座に状態機械へ
//! 再投入する（DD-2・同時進行 ≤ 1）。
//!
//! # 駆動モデル（DD-2 同期往復ループ・execute-batch/reinject-last）
//! `step` が返す Action バッチを**先頭から順に全て実行**し、その中で発生した SHIORI 往復の
//! **最後の応答**のみを `Input::ShioriReply` として再投入して `step` を再度呼ぶ（Actions が
//! 尽きるまで反復）。通常バッチは shiori Action を高々 1 本しか含まないため in-flight ≤ 1 が
//! 保たれる。唯一の例外は `ForceQuit` の `[OnClose NOTIFY, Unload]`（DD-10 best-effort）で、
//! 「バッチ全実行→最後（Unload）の応答のみ再投入」により OnClose 一報→Unload→StopSelf の
//! 正しい順序が成立する（先頭 NOTIFY の応答を即再投入すると Unloading{Forced} が unload 完了と
//! 誤認し実 Unload を飛ばす）。
//!
//! # 停止・切断（Req 4.8/4.9）
//! `KanadeMsg::Close` は step を経ず即時 Break（停止規約）。`Action::StopSelf` は shiori へ
//! `ShioriMsg::Close` を送り Break。全 `Sender<KanadeMsg>` drop（結線側・sakura の切断）で inbox が
//! 切断され受信ループが正常終了する——本体は自身の inbox Sender を保持しない構造でこれを担保する。
//!
//! # 失敗経路のログ規律（steering: areka-log-first-no-silent-failure）
//! SHIORI 送出失敗・応答 oneshot 切断は `error!` の上で `ShioriOutcome::Failed(Ipc)` へ写像し
//! 再投入する（→ Unloading{Fault}・宙吊りなし）。StartTalk 送出失敗は `error!` の上で運行を
//! 継続する（当該 talk は不成立・TalkDone は来ないが M1 は許容）。沈黙の失敗経路は存在しない。

use std::convert::Infallible;
use std::ops::ControlFlow;
use std::sync::mpsc::Sender;

use areka_actor::{ActorHandle, ReplyError, reply_channel, run_inbox, spawn_actor};

use crate::msg::{KanadeConfig, KanadeMsg, ShioriCall, ShioriFailure, ShioriMsg, ShioriOutcome};
use crate::schedule::{Action, Input, State, step};
use crate::talk::StartTalk;

/// kanade アクターを起動する（areka-actor 規約: スレッド名 "kanade"）。
///
/// inbox の送信端（[`Sender<KanadeMsg>`]）と [`ActorHandle`] を返す。body は独立スレッド上で
/// [`State::initial`] から運行状態機械を駆動する。`shiori`／`sakura` は**送出先**の送信端であり
/// （body が保持するのは outbound のみ・自身の inbox Sender は保持しない）、これらと結線側が
/// 全て drop されると inbox が切断され body は正常終了する（Req 4.9）。
///
/// # 運用規約（デッドロック注意・Req 4.8）
/// 停止は `KanadeMsg::Close` 送信・`Action::StopSelf`（終了系列完了）・全 `Sender<KanadeMsg>` drop の
/// いずれかで駆動する。`Sender<KanadeMsg>` を握ったまま停止も送らずに [`ActorHandle::join`] すると
/// body は受信待ちのままデッドロックし得る（結線側は drop→join 順を厳守すること）。
pub fn spawn_kanade(
    config: KanadeConfig,
    shiori: Sender<ShioriMsg>,
    sakura: Sender<StartTalk>,
) -> (Sender<KanadeMsg>, ActorHandle) {
    spawn_actor("kanade", move |rx| {
        let mut state = State::initial();
        run_inbox::<KanadeMsg, Infallible>(rx, move |msg| {
            // 停止規約: Close は step を経ず即時 Break（積み残しは rx drop で破棄）。
            let input = match msg {
                KanadeMsg::Close => {
                    tracing::info!(target: "kanade", event = "close", "停止指示（Close）を受領——即時停止");
                    return Ok(ControlFlow::Break(()));
                }
                KanadeMsg::Boot => Input::Boot,
                KanadeMsg::Tick { now } => Input::Tick { now },
                KanadeMsg::TalkDone(td) => Input::TalkDone(td),
                KanadeMsg::CloseRequest { reason } => Input::CloseRequest { reason },
                KanadeMsg::ForceQuit { reason } => Input::ForceQuit { reason },
                KanadeMsg::ShioriDown { reason } => Input::ShioriDown { reason },
                KanadeMsg::Mouse(m) => Input::Mouse(m),
            };
            match drive(&mut state, input, &config, &shiori, &sakura) {
                Drive::Continue => Ok(ControlFlow::Continue(())),
                Drive::Stop => Ok(ControlFlow::Break(())),
            }
        });
    })
}

/// 1 メッセージ分の駆動結果（継続 or 停止）。
enum Drive {
    Continue,
    Stop,
}

/// DD-2 同期往復ループ: `step` の Action バッチを全実行し、最後の SHIORI 往復応答のみを
/// `Input::ShioriReply` として再投入して Actions が尽きるまで反復する（execute-batch/reinject-last）。
fn drive(
    state: &mut State,
    input: Input,
    config: &KanadeConfig,
    shiori: &Sender<ShioriMsg>,
    sakura: &Sender<StartTalk>,
) -> Drive {
    // 初回 step。以降は state を差し替えつつ actions を回す。
    let (mut st, mut actions) = step(std::mem::replace(state, State::initial()), input, config);
    loop {
        // 直前の往復応答と、その応答が由来する呼出イベント ID（origin・DD-IE-3）を控える。
        // reinject する `Input::ShioriReply` へ origin を転記し、後続処理が応答の出所を識別できる
        // ようにする（マウス GET の origin 別 reply 政策はタスク 2.2）。
        let mut last_reply: Option<(ShioriOutcome, &'static str)> = None;
        let mut stop = false;
        for action in actions {
            match action {
                Action::StartTalk(start) => {
                    // 再生起動要求を sakura へ送出。切断時は error!＋運行継続（当該 talk 不成立）。
                    if sakura.send(start).is_err() {
                        tracing::error!(
                            target: "kanade",
                            event = "start_talk_send_failed",
                            "再生起動要求の送出に失敗（sakura 切断）——当該 talk は不成立・運行は継続"
                        );
                    }
                }
                Action::ShioriRequest(call) => {
                    // 送出前に call のイベント ID を控える（round_trip_request が call を消費するため）。
                    let origin = match &call {
                        ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. } => *id,
                    };
                    last_reply = Some((round_trip_request(shiori, call), origin));
                }
                Action::ShioriUnload => {
                    // unload には出所イベントが無いため "Unload" を転記する（Unloading 応答は
                    // origin を参照しないが、契約上必ず値を持たせる）。
                    last_reply = Some((round_trip_unload(shiori), "Unload"));
                }
                Action::StopSelf => {
                    // 終了系列完了: shiori へ Close を送り自身も停止する。
                    let _ = shiori.send(ShioriMsg::Close);
                    stop = true;
                    break;
                }
            }
        }
        if stop {
            *state = st;
            return Drive::Stop;
        }
        match last_reply {
            // 往復応答を再投入して次の遷移を得る（Actions が尽きるまで反復）。origin を転記する。
            Some((outcome, origin)) => {
                let (s, a) = step(st, Input::ShioriReply { outcome, origin }, config);
                st = s;
                actions = a;
            }
            // バッチに SHIORI 往復が無い＝この入力の処理は完了。
            None => {
                *state = st;
                return Drive::Continue;
            }
        }
    }
}

/// GET／NOTIFY の同期往復。SHIORI へ出る**唯一の実行点**であり（本番・mock 双方が必ず通る・
/// DD-IT-7）、送出前に送出イベント ID がホワイトリストの要素であることを検証する egress
/// チョークポイントである（Req3.1）。
///
/// - 許可集合外（`OnTalk`／`OnHour` 等・Req3.2）: SHIORI へ**送出せず** `error!`（event=
///   `event_id_not_allowed`）を残し、内部規律違反の失敗語彙 `ShioriOutcome::Failed(ShioriFailure::
///   Internal(..))` を返す（DD-IT-11・状態機械は既存の fault 経路で処理＝檻専用の応答を発明しない・
///   panic しない・宙吊りにしない）。
/// - 許可集合内: 送出前に Method・イベント ID・参照値・実行状態の wire 証跡を `trace!`（event=
///   `shiori_request`）で残して送出する（Req6.2）。往復失敗は error!＋`Failed(Ipc)` へ写像（宙吊りなし）。
fn round_trip_request(shiori: &Sender<ShioriMsg>, call: ShioriCall) -> ShioriOutcome {
    // 送出しようとしているイベントの Method／ID／参照値／実行状態（wire 値）を取り出す。
    // `status.render()` は `None` ⇔ Status ヘッダ行なし（Req6.2・DD-IT-5 の kanade 層観測）。
    let (method, id, references, status_wire) = match &call {
        ShioriCall::Get { id, references, status } => ("GET", *id, references, status.render()),
        ShioriCall::Notify { id, references, status } => {
            ("NOTIFY", *id, references, status.render())
        }
    };

    // ID ホワイトリスト檻（Req3.1/3.2/4.1・DD-IT-7/DD-IT-11）: 許可集合外は送出せず内部規律違反
    // として失敗させる。送出可否は「イベント許可 ∨ リソース許可」の論理和で判定する——イベント檻
    // （`ALLOWED_EVENT_IDS`・8 ID）とは別族のリソース許可集合（`ALLOWED_RESOURCE_IDS`・M1: username）を
    // additive に OR する（既存イベント許可路・許可外拒否は無改変・design 論点1）。
    let allowed = crate::schedule::events::is_allowed_event_id(id)
        || crate::schedule::resources::is_allowed_resource_id(id);
    if !allowed {
        tracing::error!(
            target: "kanade",
            event = "event_id_not_allowed",
            id = %id,
            "送出禁止イベント ID——ホワイトリスト違反ゆえ送出せず内部規律違反として失敗させる"
        );
        return ShioriOutcome::Failed(ShioriFailure::Internal(format!(
            "event_id_not_allowed: {id}"
        )));
    }

    // 送出前の wire 証跡（Req6.2）。status=None は Status ヘッダ欠落として観測可能（DD-IT-5）。
    tracing::trace!(
        target: "kanade",
        event = "shiori_request",
        method = %method,
        id = %id,
        references = ?references,
        status = ?status_wire,
        "SHIORI 送出"
    );

    let (reply_tx, reply_rx) = reply_channel::<ShioriOutcome>();
    round_trip(shiori, ShioriMsg::Request { call, reply: reply_tx }, reply_rx)
}

/// unload の同期往復（送出＋応答受領）。失敗は error!＋`Failed(Ipc)` へ写像（宙吊りなし）。
fn round_trip_unload(shiori: &Sender<ShioriMsg>) -> ShioriOutcome {
    let (reply_tx, reply_rx) = reply_channel::<ShioriOutcome>();
    round_trip(shiori, ShioriMsg::Unload { reply: reply_tx }, reply_rx)
}

/// 送出＋応答受領の共通往復。shiori 切断（送出 Err）・応答 oneshot 切断（`ReplyError::Dropped`）を
/// いずれも error!＋`ShioriOutcome::Failed(ShioriFailure::Ipc)` へ写像する（Req 6.2/6.3・宙吊りなし）。
/// 再投入された Failed は状態機械を Unloading{Fault} へ倒す。
fn round_trip(
    shiori: &Sender<ShioriMsg>,
    msg: ShioriMsg,
    reply_rx: areka_actor::ReplyReceiver<ShioriOutcome>,
) -> ShioriOutcome {
    if let Err(_undelivered) = send_shiori(shiori, msg) {
        tracing::error!(
            target: "kanade",
            event = "shiori_send_failed",
            "SHIORI 呼出の送出に失敗（shiori 切断）——終了系列（Fault）へ"
        );
        return ShioriOutcome::Failed(ShioriFailure::Ipc("shiori channel disconnected".into()));
    }
    match reply_rx.recv() {
        Ok(outcome) => outcome,
        Err(ReplyError::Dropped) => {
            tracing::error!(
                target: "kanade",
                event = "shiori_reply_dropped",
                "SHIORI 応答 oneshot が切断（shiori アクター異常終了等）——終了系列（Fault）へ"
            );
            ShioriOutcome::Failed(ShioriFailure::Ipc("shiori reply dropped".into()))
        }
        Err(ReplyError::Timeout) => {
            // recv（無限待ち）は Timeout を返さない。防御的に Ipc 写像する（宙吊りなし）。
            tracing::error!(
                target: "kanade",
                event = "shiori_reply_timeout",
                "SHIORI 応答が期限内に受信されず——終了系列（Fault）へ"
            );
            ShioriOutcome::Failed(ShioriFailure::Ipc("shiori reply timeout".into()))
        }
    }
}

/// `ShioriMsg` を送出する。切断時は未達メッセージを `Err` で返す（呼び手が error! 写像）。
fn send_shiori(shiori: &Sender<ShioriMsg>, msg: ShioriMsg) -> Result<(), ShioriMsg> {
    shiori.send(msg).map_err(|e| e.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::CloseReason;
    use crate::status::{ExecutionSnapshot, ExecutionStatus};
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::Duration;

    /// テスト用の有界待機: 別スレッドで `f` を走らせ、期限内に完了しなければ失敗させる
    /// （どのテストもハングしない・areka-actor の run_bounded と同旨）。
    fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
        let (done_tx, done_rx) = mpsc::sync_channel::<()>(0);
        thread::spawn(move || {
            f();
            let _ = done_tx.send(());
        });
        assert!(
            done_rx.recv_timeout(timeout).is_ok(),
            "'{what}' did not complete within {timeout:?} (possible hang)"
        );
    }

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    /// mock shiori アクター: 任意の Request/Unload に良性応答を返し Close で停止する。
    /// 受領した ShioriMsg の要約を記録列へ蓄積する。
    #[derive(Debug, Clone, PartialEq)]
    enum Recorded {
        Get(String),
        Notify(String),
        Unload,
        Close,
    }

    /// mock shiori を spawn し、(送信端, 記録受信端, join ハンドル) を返す。
    /// `answer` は (受領イベント要約) に対し返す ShioriOutcome を決める。
    fn spawn_mock_shiori(
        answer: impl Fn(&Recorded) -> ShioriOutcome + Send + 'static,
    ) -> (
        Sender<ShioriMsg>,
        Receiver<Recorded>,
        ActorHandle,
    ) {
        let (rec_tx, rec_rx) = mpsc::channel::<Recorded>();
        let (tx, handle) = spawn_actor::<ShioriMsg, _>("mock-shiori", move |rx| {
            while let Ok(msg) = rx.recv() {
                match msg {
                    ShioriMsg::Request { call, reply } => {
                        let rec = match &call {
                            ShioriCall::Get { id, .. } => Recorded::Get((*id).to_string()),
                            ShioriCall::Notify { id, .. } => Recorded::Notify((*id).to_string()),
                        };
                        let _ = rec_tx.send(rec.clone());
                        let _ = reply.send(answer(&rec));
                    }
                    ShioriMsg::Unload { reply } => {
                        let _ = rec_tx.send(Recorded::Unload);
                        let _ = reply.send(answer(&Recorded::Unload));
                    }
                    ShioriMsg::Close => {
                        let _ = rec_tx.send(Recorded::Close);
                        break;
                    }
                }
            }
        });
        (tx, rec_rx, handle)
    }

    /// 良性の既定応答: NOTIFY 系→Notified・Unload→Unloaded・GET→NoContent。
    fn benign(rec: &Recorded) -> ShioriOutcome {
        match rec {
            Recorded::Notify(_) => ShioriOutcome::Notified,
            Recorded::Unload => ShioriOutcome::Unloaded,
            Recorded::Get(_) => ShioriOutcome::NoContent,
            Recorded::Close => ShioriOutcome::Notified,
        }
    }

    // --- 1. 停止観測（Req 4.8）: Close 送信 → join 成功（有界時間内） ---

    #[test]
    fn close_message_terminates_and_join_succeeds() {
        let (shiori_tx, _rec_rx, shiori_handle) = spawn_mock_shiori(benign);
        let (sakura_tx, _sakura_rx) = mpsc::channel::<StartTalk>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx);

        // 停止駆動: Close 送信後、送信端を drop（残余 Sender で inbox を生かさない）。
        kanade_tx.send(KanadeMsg::Close).expect("send Close");
        drop(kanade_tx);

        run_bounded("kanade join after Close", Duration::from_secs(5), move || {
            kanade_handle.join().expect("kanade body completes normally after Close");
        });
        // mock shiori も後片付け（kanade は StopSelf 経路を通らないため Close は来ない→
        // shiori 送信端 drop で自然終了）。ここでは shiori_handle を join せず drop で detach。
        drop(shiori_handle);
    }

    // --- 2. 全 Sender drop での正常終了（Req 4.9）: Close 未送信でも inbox 切断で join 成功 ---

    #[test]
    fn all_senders_dropped_terminates_normally() {
        let (shiori_tx, _rec_rx, shiori_handle) = spawn_mock_shiori(benign);
        let (sakura_tx, _sakura_rx) = mpsc::channel::<StartTalk>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx);

        // Close を送らず、全 Sender<KanadeMsg> を drop する（inbox 切断）。
        drop(kanade_tx);

        run_bounded(
            "kanade join after all senders dropped",
            Duration::from_secs(5),
            move || {
                kanade_handle
                    .join()
                    .expect("kanade body terminates on inbox disconnect (Req 4.9)");
            },
        );
        drop(shiori_handle);
    }

    // --- 3a. shiori 切断: 送出失敗 → 終了系列へ倒れて join 成功（宙吊りなし・Req 6.2/6.3） ---

    #[test]
    fn shiori_disconnected_send_failure_terminates_into_fault() {
        // mock shiori を spawn せず、Receiver を即 drop して送出を失敗させる。
        let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
        drop(shiori_rx); // 送出は必ず Err になる。
        let (sakura_tx, _sakura_rx) = mpsc::channel::<StartTalk>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx);

        // Boot → OnInitialize NOTIFY を発行 → 送出失敗 → Failed(Ipc) 再投入 → Unloading{Fault}
        // → Unload（これも送出失敗）→ Failed → Stopped → StopSelf（shiori.send も Err だが Break）。
        kanade_tx.send(KanadeMsg::Boot).expect("send Boot");
        drop(kanade_tx);

        run_bounded(
            "kanade join after shiori disconnect",
            Duration::from_secs(5),
            move || {
                kanade_handle
                    .join()
                    .expect("kanade terminates cleanly when shiori is disconnected");
            },
        );
    }

    // --- 3b. sakura 切断: StartTalk 送出失敗 → error!＋運行継続（終了しない）→ Close で停止 ---

    #[test]
    fn sakura_disconnected_start_talk_failure_continues_run() {
        // OnBoot（BootMain の GET）に Value を返す mock shiori: これで StartTalk が発行される。
        let (shiori_tx, _rec_rx, shiori_handle) = spawn_mock_shiori(|rec| match rec {
            Recorded::Notify(_) => ShioriOutcome::Notified,
            Recorded::Unload => ShioriOutcome::Unloaded,
            // OnFirstBoot(GET)→204、OnBoot(GET)→Value でグリーティング talk を起こす。
            Recorded::Get(id) if id == "OnBoot" => ShioriOutcome::Value(r"\0hi\e".to_string()),
            Recorded::Get(_) => ShioriOutcome::NoContent,
            Recorded::Close => ShioriOutcome::Notified,
        });

        // sakura の Receiver を即 drop → StartTalk 送出は必ず失敗する。
        let (sakura_tx, sakura_rx) = mpsc::channel::<StartTalk>();
        drop(sakura_rx);

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx);

        // Boot → boot 系列を最後まで駆動（OnBoot Value で StartTalk 送出失敗するが継続）。
        kanade_tx.send(KanadeMsg::Boot).expect("send Boot");
        // 運行が継続していることを確認するため、Close で明示的に停止させる。
        kanade_tx.send(KanadeMsg::Close).expect("send Close");
        drop(kanade_tx);

        run_bounded(
            "kanade join after sakura disconnect + Close",
            Duration::from_secs(5),
            move || {
                kanade_handle
                    .join()
                    .expect("kanade continues past StartTalk failure, then stops on Close");
            },
        );
        drop(shiori_handle);
    }

    // --- 3c. Mouse 写像（Task 1）: KanadeMsg::Mouse → Input::Mouse・非 Steady（Idle）では ---
    //         SHIORI 問い合わせを一切発行せず、その後 Close で正常停止する（DD-IE-8）。

    #[test]
    fn mouse_msg_maps_to_input_and_is_ignored_in_idle_phase() {
        use crate::msg::{MouseButton, MouseEventKind, MouseInput};

        let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);
        let (sakura_tx, _sakura_rx) = mpsc::channel::<StartTalk>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx);

        // 起動前（Idle）にマウス入力を注入 → KanadeMsg::Mouse→Input::Mouse 写像→非 Steady 無視。
        kanade_tx
            .send(KanadeMsg::Mouse(MouseInput {
                scope: 0,
                x: 5,
                y: 7,
                region: Some("head".to_string()),
                kind: MouseEventKind::DoubleClick {
                    button: MouseButton::Left,
                },
            }))
            .expect("send Mouse");
        // 停止させて有界に join する。
        kanade_tx.send(KanadeMsg::Close).expect("send Close");
        drop(kanade_tx);

        run_bounded("kanade join after Mouse+Close", Duration::from_secs(5), move || {
            kanade_handle
                .join()
                .expect("kanade stops cleanly after Mouse in Idle");
        });

        // Idle でのマウスは SHIORI へ何も届けない（Close も StopSelf 経路を通らないため未受領）。
        assert!(
            rec_rx.try_recv().is_err(),
            "Idle でのマウス入力は SHIORI へ問い合わせを発行しない（DD-IE-8）"
        );
        drop(shiori_handle);
    }

    // --- 4. ForceQuit 順序（DD-10）: OnClose NOTIFY → Unload → Close の順で shiori が受領 ---

    #[test]
    fn force_quit_emits_onclose_notify_then_unload_then_close_in_order() {
        let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);
        let (sakura_tx, _sakura_rx) = mpsc::channel::<StartTalk>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx);

        kanade_tx
            .send(KanadeMsg::ForceQuit {
                reason: CloseReason::System,
            })
            .expect("send ForceQuit");
        drop(kanade_tx);

        // 記録列を有界時間で 3 件収集する（別スレッドで recv_timeout・ハング防止）。
        let (collected_tx, collected_rx) = mpsc::channel::<Vec<Recorded>>();
        thread::spawn(move || {
            let mut seq = Vec::new();
            while let Ok(rec) = rec_rx.recv_timeout(Duration::from_secs(5)) {
                let is_close = rec == Recorded::Close;
                seq.push(rec);
                if is_close {
                    break;
                }
            }
            let _ = collected_tx.send(seq);
        });

        let seq = collected_rx
            .recv_timeout(Duration::from_secs(6))
            .expect("collector should finish within bound");

        // DD-10: OnClose NOTIFY（一報）→ Unload → Close の順。
        assert_eq!(
            seq,
            vec![
                Recorded::Notify("OnClose".to_string()),
                Recorded::Unload,
                Recorded::Close,
            ],
            "ForceQuit は OnClose NOTIFY → Unload → Close の順で shiori へ届く（execute-batch/reinject-last）"
        );

        run_bounded("kanade join after ForceQuit", Duration::from_secs(5), move || {
            kanade_handle.join().expect("kanade terminates after ForceQuit sequence");
        });
        drop(shiori_handle);
    }

    // --- 5. アクターシェルの失敗ログ＋応答写像（タスク 6.2・Req 6.1／6.2） ---
    //
    // シェルの往復ヘルパ（round_trip 系）をテストスレッド上で**直接**呼び、二つの失敗
    // サブ経路が (i) いずれも `ShioriOutcome::Failed(ShioriFailure::Ipc)` へ写像され、
    // (ii) それぞれ規約の `error!`（event=shiori_send_failed／shiori_reply_dropped）を
    // 発火することを検証する。ヘルパは同期関数ゆえログはテストスレッドで発行され、
    // `log_capture::capture`（thread-local subscriber）で確実に捕捉される（PITFALL:
    // spawn したアクタースレッドのログは捕えられない——ここでは往復ヘルパを直接呼ぶ）。
    // 6.1 が追加した `crate::schedule::log_capture` を再利用する（capturer 重複なし）。
    use crate::schedule::log_capture::{assert_logged, capture};
    use tracing::Level;

    /// テスト用 GET 呼出（round_trip_request の入力・内容は本テストで load-bearing でない）。
    fn probe_get_call() -> ShioriCall {
        ShioriCall::Get {
            id: "OnBoot",
            references: vec!["master".to_string()],
            status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
        }
    }

    // (a) shiori 送出失敗: Receiver を drop した Sender へ往復 → send_shiori が Err →
    //     `shiori_send_failed` を error! し `Failed(Ipc)` を返す。別スレッド不要。
    #[test]
    fn shiori_send_failure_maps_to_ipc_and_logs() {
        let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();
        drop(shiori_rx); // 送出は必ず Err。

        let mut outcome: Option<ShioriOutcome> = None;
        let events = capture(|| {
            outcome = Some(round_trip_request(&shiori_tx, probe_get_call()));
        });

        // (i) Failed(Ipc) 写像（ShioriOutcome は Debug 非実装ゆえ variant を match）。
        assert!(
            matches!(outcome, Some(ShioriOutcome::Failed(ShioriFailure::Ipc(_)))),
            "shiori 送出失敗は Failed(Ipc) へ写像されるべき"
        );
        // (ii) 規約の error! 発火（削除・語彙変更・レベル変更で失敗する回帰檻）。
        assert_logged(&events, Level::ERROR, "shiori_send_failed");
    }

    // (b) 応答 oneshot 切断 (ReplyError::Dropped): shiori 側が Request を**受領**したうえで
    //     reply を send せず drop → 往復ヘルパは reply_rx.recv() で Dropped を観測 →
    //     `shiori_reply_dropped` を error! し `Failed(Ipc)` を返す。ログはテストスレッドで
    //     発火するため捕捉される。受領ヘルパスレッドは有界 join する（ハングしない）。
    #[test]
    fn shiori_reply_dropped_maps_to_ipc_and_logs() {
        let (shiori_tx, shiori_rx) = mpsc::channel::<ShioriMsg>();

        // 受領ヘルパ: 1 件の Request を受け、その reply を send せず drop する。
        // 受領完了を done_tx で通知し、親は有界待機する（recv_timeout・sleep なし）。
        let (done_tx, done_rx) = mpsc::sync_channel::<()>(0);
        let helper = thread::spawn(move || {
            if let Ok(ShioriMsg::Request { call: _, reply }) = shiori_rx.recv() {
                drop(reply); // 応答を送らず切断（ReplyError::Dropped を誘発）。
            }
            let _ = done_tx.send(());
        });

        let mut outcome: Option<ShioriOutcome> = None;
        let events = capture(|| {
            outcome = Some(round_trip_request(&shiori_tx, probe_get_call()));
        });

        // (i) 応答切断は Failed(Ipc) へ写像される。
        assert!(
            matches!(outcome, Some(ShioriOutcome::Failed(ShioriFailure::Ipc(_)))),
            "応答 oneshot 切断は Failed(Ipc) へ写像されるべき"
        );
        // (ii) 規約の error! 発火（削除・語彙変更・レベル変更で失敗する回帰檻）。
        assert_logged(&events, Level::ERROR, "shiori_reply_dropped");

        // 受領ヘルパの有界 join（ハング防止）。
        assert!(
            done_rx.recv_timeout(Duration::from_secs(5)).is_ok(),
            "reply-drop helper が期限内に受領を完了すべき（possible hang）"
        );
        helper.join().expect("reply-drop helper joins cleanly");
    }

    // --- 7. egress チョークポイント判断分岐檻（タスク 2.5・Req 3.1／3.2／6.2・DD-IT-7／DD-IT-11） ---
    //
    // SHIORI へ出る唯一の実行点 `round_trip_request` を**直接**呼び、送出 ID の許可判定という
    // 入力依存の判断分岐を両側から檻に入れる（test-only-decision-branches-not-proven-wiring）。
    // ヘルパは同期関数ゆえログはテストスレッドで発行され `log_capture::capture` で確実に捕捉される。

    // (a) 許可 ID（OnBoot）→ チャネルへ送出され、送出前の wire 証跡 trace! が残る（Req6.2）。
    #[test]
    fn allowed_event_id_is_sent_and_wire_trace_logged() {
        let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);

        let mut outcome: Option<ShioriOutcome> = None;
        let events = capture(|| {
            outcome = Some(round_trip_request(&shiori_tx, probe_get_call()));
        });

        // 許可 ID の GET は送出され mock の良性応答（NoContent）が返る。
        assert!(
            matches!(outcome, Some(ShioriOutcome::NoContent)),
            "許可 ID の GET は送出され mock の良性応答が返るべき"
        );
        // mock が当該 Request を受領した＝チャネルへ送出された（Req3.1 許可路）。
        assert_eq!(
            rec_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("許可 ID はチャネルへ送出され mock が受領するはず"),
            Recorded::Get("OnBoot".to_string())
        );
        // Method・ID・参照値・実行状態の wire 証跡（Req6.2・DD-IT-7）。
        assert_logged(&events, Level::TRACE, "shiori_request");

        drop(shiori_tx);
        drop(shiori_handle);
    }

    // (b) 禁止 ID（OnTalk／OnHour）→ 送出されず error! を残し Failed(Internal) へ写像される（Req3.2）。
    //     チャネルへ何も届かないことを保持した record Receiver の try_recv=Empty で確認する。
    #[test]
    fn forbidden_event_id_is_not_sent_maps_to_internal_and_logs_error() {
        for forbidden in ["OnTalk", "OnHour"] {
            let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);

            let mut outcome: Option<ShioriOutcome> = None;
            let events = capture(|| {
                outcome = Some(round_trip_request(
                    &shiori_tx,
                    ShioriCall::Get {
                        id: forbidden,
                        references: vec!["master".to_string()],
                        status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
                    },
                ));
            });

            // (i) 送出せず内部規律違反の失敗語彙へ写像（DD-IT-11・状態機械は既存 fault 経路を使う）。
            assert!(
                matches!(
                    outcome,
                    Some(ShioriOutcome::Failed(ShioriFailure::Internal(_)))
                ),
                "{forbidden} は送出せず Failed(Internal) へ写像されるべき"
            );
            // (ii) 規約の error! 発火（削除・語彙変更・レベル変更で失敗する回帰檻）。
            assert_logged(&events, Level::ERROR, "event_id_not_allowed");
            // (iii) チャネルへ何も届かない＝禁止 ID は送出前に返る（Req3.2 恒久不送出）。
            assert!(
                rec_rx.try_recv().is_err(),
                "{forbidden} はチャネルへ送出されてはならない（Req3.2）"
            );

            drop(shiori_tx);
            drop(shiori_handle);
        }
    }

    // --- 8. submit ガードのリソース許可拡張（タスク 6.1・Req 4.1・design 論点1／別族許可集合） ---
    //
    // egress チョークポイント `round_trip_request` の submit ガードが
    // 「`is_allowed_event_id(id)` ∨ `is_allowed_resource_id(id)`」へ拡張されたことを、
    // リソース ID の実送出パスから両側で檻に入れる。イベント檻・許可外拒否の既存挙動は
    // 無改変であること（回帰檻）を並せて確認する。

    /// テスト用のリソース GET 呼出（M1 リソース ID `username`）。
    fn probe_resource_call(id: &'static str) -> ShioriCall {
        ShioriCall::Get {
            id,
            references: vec!["master".to_string()],
            status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
        }
    }

    // (a) 許可リソース ID（username）→ ガードを通過してチャネルへ送出され良性応答が返る。
    //     従来（OR 拡張前）は非イベント ID として Failed(Internal) 拒否されていた分岐が、
    //     リソース許可アームの追加により送出側へ倒れることを檻に入れる（Req4.1）。
    #[test]
    fn allowed_resource_id_passes_guard_and_is_sent() {
        let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);

        let outcome = round_trip_request(&shiori_tx, probe_resource_call("username"));

        // 許可リソース ID の GET は送出され mock の良性応答（NoContent）が返る。
        assert!(
            matches!(outcome, ShioriOutcome::NoContent),
            "許可リソース ID の GET は送出され mock の良性応答が返るべき（Req4.1）"
        );
        // mock が当該 Request を受領した＝ガードを通過しチャネルへ送出された。
        assert_eq!(
            rec_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("許可リソース ID はチャネルへ送出され mock が受領するはず"),
            Recorded::Get("username".to_string())
        );

        drop(shiori_tx);
        drop(shiori_handle);
    }

    // (b) 許可外 ID（イベントでもリソースでもない）→ 従来どおり送出されず Failed(Internal)
    //     へ写像される（既存不変量の回帰檻・Req3.2 の拒否経路が OR 拡張後も無改変）。
    #[test]
    fn disallowed_id_still_rejected_as_internal_after_resource_extension() {
        let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);

        // "notaresource" はイベント許可集合にもリソース許可集合にも属さない。
        let outcome = round_trip_request(&shiori_tx, probe_resource_call("notaresource"));

        assert!(
            matches!(
                outcome,
                ShioriOutcome::Failed(ShioriFailure::Internal(_))
            ),
            "許可外 ID は OR 拡張後も送出されず Failed(Internal) へ写像されるべき（既存不変量）"
        );
        // チャネルへ何も届かない＝許可外 ID は送出前に返る（従来挙動の保存）。
        assert!(
            rec_rx.try_recv().is_err(),
            "許可外 ID はチャネルへ送出されてはならない（既存不変量）"
        );

        drop(shiori_tx);
        drop(shiori_handle);
    }

    // (c) 既存の許可イベント ID（OnBoot）は OR 拡張の影響を受けず従来どおり送出される
    //     （イベント檻無改変の回帰檻）。
    #[test]
    fn allowed_event_id_unaffected_by_resource_or_extension() {
        let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);

        let outcome = round_trip_request(&shiori_tx, probe_get_call());

        assert!(
            matches!(outcome, ShioriOutcome::NoContent),
            "許可イベント ID の GET は OR 拡張後も従来どおり送出されるべき"
        );
        assert_eq!(
            rec_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("許可イベント ID はチャネルへ送出され mock が受領するはず"),
            Recorded::Get("OnBoot".to_string())
        );

        drop(shiori_tx);
        drop(shiori_handle);
    }
}
