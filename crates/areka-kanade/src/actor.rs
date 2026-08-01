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
//! 再投入する（→ Unloading{Fault}・宙吊りなし）。talk 指示（[`TalkCommand`]）の送出失敗は
//! `error!` の上で運行を継続する（当該指示は不成立・TalkDone は来ないが M1 は許容）。
//! 沈黙の失敗経路は存在しない。

use std::convert::Infallible;
use std::ops::ControlFlow;
use std::sync::mpsc::Sender;

use areka_actor::{ActorHandle, ReplyError, reply_channel, run_inbox, spawn_actor};

use crate::msg::{
    EventId, KanadeConfig, KanadeMsg, ShioriCall, ShioriFailure, ShioriMsg, ShioriOutcome,
};
use crate::schedule::resources::ResourceSink;
use crate::schedule::{Action, Input, State, step};
use crate::talk::TalkCommand;

/// kanade アクターを起動する（areka-actor 規約: スレッド名 "kanade"）。
///
/// inbox の送信端（[`Sender<KanadeMsg>`]）と [`ActorHandle`] を返す。body は独立スレッド上で
/// [`State::initial`] から運行状態機械を駆動する。`shiori`／`sakura` は**送出先**の送信端であり
/// （body が保持するのは outbound のみ・自身の inbox Sender は保持しない）、これらと結線側が
/// 全て drop されると inbox が切断され body は正常終了する（Req 4.9）。
///
/// # talk 再生系への送出口（DD-5・design C6・Req 5.6）
/// `sakura` は [`TalkCommand`]（`Start` / `ResolveChoice` / `CancelChoice`）の単一チャンネルである。
/// 起動系と選択解決系を別チャンネルへ分けないことが順序保存の契約であり（`areka-talk` の
/// [`TalkCommand`] doc・DD-4 の前提）、kanade が投函した順序が relay ＋ dispatcher 単一 inbox を
/// 経て FIFO で下流へ届く。選択待ちの解決を再生層の正規入力経路で行う（kanade 側にバリア状態・
/// 再生状態を持たない）という Req 5.6 は、この単一送出口によって構造的に成立する。
///
/// # 運用規約（デッドロック注意・Req 4.8）
/// 停止は `KanadeMsg::Close` 送信・`Action::StopSelf`（終了系列完了）・全 `Sender<KanadeMsg>` drop の
/// いずれかで駆動する。`Sender<KanadeMsg>` を握ったまま停止も送らずに [`ActorHandle::join`] すると
/// body は受信待ちのままデッドロックし得る（結線側は drop→join 順を厳守すること）。
///
/// # リソース照会シンク（R4.1）
/// `resource_sink` は boot 系列の username prefetch（OnInitialize 後・OnFirstBoot 前）が受け取る
/// [`ResourceOutcome`](crate::schedule::resources::ResourceOutcome) を**同期的に**受ける注入クロージャ
/// （kanade は sylphya へ依存しない疎結合シーム）。sink が返るまで boot は次段へ進まない。結果を使わない
/// 構成（既存テスト等）では no-op sink（`Box::new(|_, _| {})`）を渡す。
pub fn spawn_kanade(
    config: KanadeConfig,
    shiori: Sender<ShioriMsg>,
    sakura: Sender<TalkCommand>,
    resource_sink: ResourceSink,
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
                // 選択系 2 入力（additive・Req 4.4）。境界型をそのまま状態機械の入力へ写す
                // （シェルは判断しない——受領検証・帳簿確立は schedule 層の責務）。
                KanadeMsg::Choice(c) => Input::Choice(c),
                KanadeMsg::ChoiceWaiting {
                    talk_id,
                    choice_ids,
                    display_end,
                    timeout_directive_secs,
                } => Input::ChoiceWaiting {
                    talk_id,
                    choice_ids,
                    display_end,
                    timeout_directive_secs,
                },
            };
            match drive(&mut state, input, &config, &shiori, &sakura, &resource_sink) {
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
    sakura: &Sender<TalkCommand>,
    resource_sink: &ResourceSink,
) -> Drive {
    // 初回 step。以降は state を差し替えつつ actions を回す。
    let (mut st, mut actions) = step(std::mem::replace(state, State::initial()), input, config);
    loop {
        let BatchResult { last_reply, stop } =
            execute_actions(actions, shiori, sakura, resource_sink);
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

/// Action バッチ 1 回分の実行結果（[`drive`] の反復条件）。
struct BatchResult {
    /// バッチ中で最後に発生した SHIORI 往復の応答と、その応答が由来する呼出イベント ID
    /// （origin・DD-IE-3）。`None` はバッチに SHIORI 往復が無かったこと（＝再投入しない）を表す。
    last_reply: Option<(ShioriOutcome, &'static str)>,
    /// [`Action::StopSelf`] を実行した（以降の Action は実行しない・呼び手は停止する）。
    stop: bool,
}

/// Action バッチを**先頭から順に全て実行**する（execute-batch/reinject-last の execute 側）。
///
/// [`drive`] の反復本体から切り出してあるのは、[`Action`] → [`TalkCommand`] 写像を発行点
/// （タスク 4.3／4.5）の実装を待たずに実行で檻に入れられるようにするためである（design C6）。
///
/// # talk 指示 3 形の写像（design C6・DD-5・Req 5.6）
/// [`Action::StartTalk`]／[`Action::ResolveChoice`]／[`Action::CancelChoice`] はそれぞれ
/// [`TalkCommand::Start`]／[`TalkCommand::ResolveChoice`]／[`TalkCommand::CancelChoice`] へ
/// **そのまま包んで**同一チャンネルへ送出する（値の解釈・書き換えをしない）。起動系と解決系を
/// 別チャンネルへ分けないことが順序保存の契約であり、状態機械が 1 バッチで並べた順序が
/// そのまま下流で観測される。送出失敗は [`send_talk_command`] が `error!` を残し**運行は継続**
/// する——バッチも中断しない（design「Error Strategy」: 選択・再生の失敗でゴーストを終了させない）。
fn execute_actions(
    actions: Vec<Action>,
    shiori: &Sender<ShioriMsg>,
    sakura: &Sender<TalkCommand>,
    resource_sink: &ResourceSink,
) -> BatchResult {
    let mut last_reply: Option<(ShioriOutcome, &'static str)> = None;
    for action in actions {
        match action {
            Action::StartTalk(start) => {
                send_talk_command(sakura, TalkCommand::Start(start));
            }
            Action::ResolveChoice { talk_id, id } => {
                send_talk_command(sakura, TalkCommand::ResolveChoice { talk_id, id });
            }
            Action::CancelChoice { talk_id } => {
                send_talk_command(sakura, TalkCommand::CancelChoice { talk_id });
            }
            Action::ShioriRequest(call) => {
                // 送出前に call のイベント ID を控える（round_trip_request が call を消費するため）。
                // origin は `&'static str` 契約を維持する（DD-1）: スケジューラ起源は固定 ID を
                // そのまま転記し、選択起源（任意名・`&'static` にできない）は固定ラベル
                // `"OnChoiceEvent"` を載せる（ログ／防御用。選択応答のルーティングは帳簿照合が正）。
                let origin = match &call {
                    ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. } => match id {
                        EventId::Static(s) => *s,
                        EventId::Choice(_) => "OnChoiceEvent",
                    },
                };
                last_reply = Some((round_trip_request(shiori, call), origin));
            }
            Action::ResourceOutcome { id, outcome } => {
                // リソース照会結果を注入クロージャへ**同期的に**渡す（返るまで次段へ進まない・R4.1）。
                // 副作用は sink 内部（ghost の publish＋barrier）——talk は生成しない（Invariant）。
                // last_reply は変えない（SHIORI 往復ではないため再投入対象にならない）。
                resource_sink(id, outcome);
            }
            Action::ShioriUnload => {
                // unload には出所イベントが無いため "Unload" を転記する（Unloading 応答は
                // origin を参照しないが、契約上必ず値を持たせる）。
                last_reply = Some((round_trip_unload(shiori), "Unload"));
            }
            Action::StopSelf => {
                // 終了系列完了: shiori へ Close を送り自身も停止する。
                let _ = shiori.send(ShioriMsg::Close);
                return BatchResult {
                    last_reply,
                    stop: true,
                };
            }
        }
    }
    BatchResult {
        last_reply,
        stop: false,
    }
}

/// GET／NOTIFY の同期往復。SHIORI へ出る**唯一の実行点**であり（本番・mock 双方が必ず通る・
/// DD-IT-7）、送出前に送出イベント ID が**出所カテゴリごとの受理規則**を満たすことを検証する
/// egress チョークポイントである（Req2.6／2.9／3.1・design C6・DD-2）。
///
/// - 受理されない ID（スケジューラ起源の `OnTalk`／`OnHour` 等・Req3.2、選択起源の `On` 非接頭・
///   Req2.6）: SHIORI へ**送出せず** `error!`（event=`event_id_not_allowed`）を残し、内部規律違反の
///   失敗語彙 `ShioriOutcome::Failed(ShioriFailure::Internal(..))` を返す（DD-IT-11・状態機械は
///   既存の fault 経路で処理＝檻専用の応答を発明しない・panic しない・宙吊りにしない）。
/// - 許可集合内: 送出前に Method・イベント ID・参照値・実行状態の wire 証跡を `trace!`（event=
///   `shiori_request`）で残して送出する（Req6.2）。往復失敗は error!＋`Failed(Ipc)` へ写像（宙吊りなし）。
fn round_trip_request(shiori: &Sender<ShioriMsg>, call: ShioriCall) -> ShioriOutcome {
    // 送出しようとしているイベントの Method／ID（出所カテゴリ込み）／参照値／実行状態を取り出す。
    // `status.render()` は `None` ⇔ Status ヘッダ行なし（Req6.2・DD-IT-5 の kanade 層観測）。
    let (method, event_id, references, status_wire) = match &call {
        ShioriCall::Get { id, references, status } => ("GET", id, references, status.render()),
        ShioriCall::Notify { id, references, status } => {
            ("NOTIFY", id, references, status.render())
        }
    };
    // ログ証跡は wire 形（[`EventId::as_str`]）で残す——出所カテゴリは表現を変えない（DD-1）。
    let id = event_id.as_str();

    // ID 受理檻（Req2.6/2.9/3.1/3.2/4.1・design C6・DD-2・DD-IT-7/DD-IT-11）: 受理されない ID は
    // 送出せず内部規律違反として失敗させる。送出可否は**出所カテゴリ別**に判定する。
    //
    // - スケジューラ起源（`Static`）: 従来どおり「イベント許可 ∨ リソース許可」の論理和——固定表
    //   （`ALLOWED_EVENT_IDS`）と別族のリソース許可集合（`ALLOWED_RESOURCE_IDS`・M1: username）。
    //   `OnTalk`／`OnHour` の恒久禁止（自発生成との二重駆動）はこちら側で**不変**（Req3.2）。
    // - 選択起源（`Choice`）: `is_allowed_choice_event`（`On` 接頭のみ）。作者が `\q` の ID に書いた
    //   名前を事前登録なしに逐語で発火するため固定表を要求せず、スケジューラ起源の恒久禁止も
    //   適用しない（Req2.9・裁定 8＝両禁止規則は非交差）。
    let allowed = match event_id {
        EventId::Static(s) => {
            crate::schedule::events::is_allowed_event_id(s)
                || crate::schedule::resources::is_allowed_resource_id(s)
        }
        EventId::Choice(name) => crate::schedule::events::is_allowed_choice_event(name),
    };
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

/// talk 再生系へ [`TalkCommand`] を送出する**唯一の実行点**（design C6・Req 5.6）。
///
/// 送出失敗（sakura／中継の切断）は `error!`（event=`talk_command_send_failed`）を残したうえで
/// **運行を継続する**——当該指示は不成立（起動なら talk が起きず TalkDone も来ない・解決なら
/// バリアが解けない）だが、選択・再生の失敗でゴーストを終了させないという既存の起動失敗規律と
/// 同一の扱いである（design「Error Strategy」・steering: areka-log-first-no-silent-failure）。
/// 種別は `kind` フィールドで区別でき、沈黙の失敗経路は持たない。
fn send_talk_command(sakura: &Sender<TalkCommand>, command: TalkCommand) {
    // ログ用の種別ラベル（送出で command が消費されるため先に取り出す）。
    let (kind, talk_id) = match &command {
        TalkCommand::Start(start) => ("start", start.talk_id.0),
        TalkCommand::ResolveChoice { talk_id, .. } => ("resolve_choice", talk_id.0),
        TalkCommand::CancelChoice { talk_id } => ("cancel_choice", talk_id.0),
    };
    if sakura.send(command).is_err() {
        tracing::error!(
            target: "kanade",
            event = "talk_command_send_failed",
            kind = %kind,
            talk_id = talk_id,
            "talk 指示の送出に失敗（再生系切断）——当該指示は不成立・運行は継続"
        );
    }
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

    /// リソース照会結果を捨てる no-op sink（結果を使わない結線・R4.1 Implementation Notes）。
    /// prefetch は駆動されるが sink は何もしない——本モジュールの停止・順序・失敗系テストは
    /// prefetch 結果に依存しないため、副作用のないクロージャで十分。
    fn noop_sink() -> crate::schedule::resources::ResourceSink {
        Box::new(|_, _| {})
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
                            ShioriCall::Get { id, .. } => Recorded::Get(id.as_str().to_string()),
                            ShioriCall::Notify { id, .. } => Recorded::Notify(id.as_str().to_string()),
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
        let (sakura_tx, _sakura_rx) = mpsc::channel::<TalkCommand>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx, noop_sink());

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
        let (sakura_tx, _sakura_rx) = mpsc::channel::<TalkCommand>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx, noop_sink());

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
        let (sakura_tx, _sakura_rx) = mpsc::channel::<TalkCommand>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx, noop_sink());

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

    // --- 3b. sakura 切断: `TalkCommand::Start` 送出失敗 → error!＋運行継続（終了しない）→ Close で停止 ---

    #[test]
    fn sakura_disconnected_start_talk_failure_continues_run() {
        // OnBoot（BootMain の GET）に Value を返す mock shiori: これで起動指示が発行される。
        let (shiori_tx, _rec_rx, shiori_handle) = spawn_mock_shiori(|rec| match rec {
            Recorded::Notify(_) => ShioriOutcome::Notified,
            Recorded::Unload => ShioriOutcome::Unloaded,
            // OnFirstBoot(GET)→204、OnBoot(GET)→Value でグリーティング talk を起こす。
            Recorded::Get(id) if id == "OnBoot" => ShioriOutcome::Value(r"\0hi\e".to_string()),
            Recorded::Get(_) => ShioriOutcome::NoContent,
            Recorded::Close => ShioriOutcome::Notified,
        });

        // sakura の Receiver を即 drop → talk 指示の送出は必ず失敗する。
        let (sakura_tx, sakura_rx) = mpsc::channel::<TalkCommand>();
        drop(sakura_rx);

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx, noop_sink());

        // Boot → boot 系列を最後まで駆動（OnBoot Value で起動指示の送出が失敗するが継続）。
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
                    .expect("kanade continues past TalkCommand send failure, then stops on Close");
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
        let (sakura_tx, _sakura_rx) = mpsc::channel::<TalkCommand>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx, noop_sink());

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
        let (sakura_tx, _sakura_rx) = mpsc::channel::<TalkCommand>();

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx, noop_sink());

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
            id: EventId::Static("OnBoot"),
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

    // --- 6. talk 指示送出の失敗ログ＋運行継続（タスク 1.4・Req 4.4／5.6・design C6） ---
    //
    // talk 再生系へ出る唯一の実行点 `send_talk_command` を**直接**呼び、受信端が切断された
    // 状態で (i) 規約の `error!`（event=talk_command_send_failed）を発火し、(ii) 返り値も
    // panic も持たず呼び手が運行を継続できる（＝既存の起動失敗規律と同一の扱い）ことを固定する。
    // ヘルパは同期関数ゆえログはテストスレッドで発行され log_capture で確実に捕捉される。

    #[test]
    fn talk_command_send_failure_logs_error_and_continues() {
        use crate::talk::{StartTalk, TalkId};

        let (sakura_tx, sakura_rx) = mpsc::channel::<TalkCommand>();
        drop(sakura_rx); // 送出は必ず Err。

        let events = capture(|| {
            // 3 形すべてが同一の失敗規律（error! ＋継続）を通る。
            send_talk_command(
                &sakura_tx,
                TalkCommand::Start(StartTalk::new(TalkId(1), r"\e")),
            );
            send_talk_command(
                &sakura_tx,
                TalkCommand::ResolveChoice {
                    talk_id: TalkId(1),
                    id: "x".to_string(),
                },
            );
            send_talk_command(
                &sakura_tx,
                TalkCommand::CancelChoice {
                    talk_id: TalkId(1),
                },
            );
        });

        // 規約の error! 発火（削除・語彙変更・レベル変更で失敗する回帰檻）。
        assert_logged(&events, Level::ERROR, "talk_command_send_failed");
        // 3 回とも記録される（黙って捨てる経路が 1 本も無い・log-first）。
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event.as_deref() == Some("talk_command_send_failed"))
                .count(),
            3,
            "3 形すべての送出失敗が個別に記録されるべき（沈黙の失敗経路なし）"
        );
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
                        id: EventId::Static(forbidden),
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
            id: EventId::Static(id),
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

    // --- 9. boot prefetch → ResourceSink 同期呼出（タスク 6.2・R4.1・design boot 図） ---
    //
    // アクター経路（spawn_kanade）で boot を駆動し、username リソース照会（prefetch）が
    // (i) OnInitialize NOTIFY の後・OnFirstBoot GET の前に mock shiori へ 1 回だけ届き、
    // (ii) その応答（Value）が注入 ResourceSink へ `("username", Value(..))` として同期的に渡る
    // ことを観測する。sink は別スレッド（kanade アクター）から呼ばれるため Arc<Mutex<Vec>> に記録する。

    #[test]
    fn boot_prefetch_issues_username_between_initialize_and_firstboot_and_calls_sink() {
        use crate::schedule::resources::{ResourceOutcome, ResourceSink};
        use std::sync::{Arc, Mutex};

        // mock shiori: OnInitialize→Notified・username GET→Value("bob")・他 GET→NoContent・Unload→Unloaded。
        // username=Value で prefetch 経路の Value 写像を踏む（boot は挨拶なし 204 経路で Steady へ）。
        let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(|rec| match rec {
            Recorded::Notify(_) => ShioriOutcome::Notified,
            Recorded::Unload => ShioriOutcome::Unloaded,
            Recorded::Get(id) if id == "username" => ShioriOutcome::Value("bob".to_string()),
            Recorded::Get(_) => ShioriOutcome::NoContent,
            Recorded::Close => ShioriOutcome::Notified,
        });
        let (sakura_tx, _sakura_rx) = mpsc::channel::<TalkCommand>();

        // 記録 sink: 呼出 (id, outcome) を蓄積する（同期呼出・別スレッドから）。
        let seen: Arc<Mutex<Vec<(&'static str, ResourceOutcome)>>> = Arc::new(Mutex::new(Vec::new()));
        let seen_body = Arc::clone(&seen);
        let sink: ResourceSink =
            Box::new(move |id, outcome| seen_body.lock().unwrap().push((id, outcome)));

        let (kanade_tx, kanade_handle) = spawn_kanade(config(), shiori_tx, sakura_tx, sink);

        // 起動を駆動 → boot 系列（prefetch 含む）が同期完走する。その後 Close で停止。
        kanade_tx.send(KanadeMsg::Boot).expect("send Boot");
        kanade_tx.send(KanadeMsg::Close).expect("send Close");
        drop(kanade_tx);

        run_bounded("kanade join after Boot+Close", Duration::from_secs(5), move || {
            kanade_handle.join().expect("kanade stops cleanly after boot prefetch");
        });

        // (i) mock 受領列の先頭 3 件: OnInitialize NOTIFY → username GET → OnFirstBoot GET。
        //     prefetch が OnInitialize 後・OnFirstBoot 前に 1 回だけ挟まる（design boot 図）。
        let mut seq: Vec<Recorded> = Vec::new();
        while seq.len() < 3 {
            match rec_rx.recv_timeout(Duration::from_secs(5)) {
                Ok(r) => seq.push(r),
                Err(_) => break,
            }
        }
        assert_eq!(
            seq,
            vec![
                Recorded::Notify("OnInitialize".to_string()),
                Recorded::Get("username".to_string()),
                Recorded::Get("OnFirstBoot".to_string()),
            ],
            "prefetch（username GET）は OnInitialize 後・OnFirstBoot 前に 1 回だけ届く"
        );

        // (ii) sink は ("username", Value("bob")) で同期的に呼ばれた（ちょうど 1 回）。
        let calls = seen.lock().unwrap().clone();
        assert_eq!(
            calls,
            vec![("username", ResourceOutcome::Value("bob".to_string()))],
            "ResourceSink は ('username', Value(\"bob\")) でちょうど 1 回呼ばれるべき（R4.1）"
        );

        drop(shiori_handle);
    }

    // --- 10. egress ガードの出所カテゴリ分岐（タスク 2.3・Req 2.6／2.9・design C6／DD-2／裁定 8） ---
    //
    // submit ガードが `EventId` の出所カテゴリ別 match へ変わったことを、両カテゴリの実送出パスから
    // 檻に入れる。スケジューラ起源（`Static`）は固定表 ∨ リソース表で従来どおり——`OnTalk`／`OnHour`
    // の恒久禁止は不変（セクション 7 (b) が保存）——選択起源（`Choice`）は `On` 接頭のみで受理し、
    // 恒久禁止 ID であっても逐語で送出される（Req2.9・恒久禁止の根拠は選択起源に該当しない）。

    /// テスト用の選択起源 GET 呼出（任意名イベント・[`EventId::Choice`]）。
    fn probe_choice_call(id: &str) -> ShioriCall {
        ShioriCall::Get {
            id: EventId::Choice(id.to_string()),
            references: Vec::new(),
            status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
        }
    }

    // (a) 選択起源の恒久禁止 ID（OnTalk／OnHour）→ ガードを通過して逐語で送出される（Req2.9）。
    //     同じ ID がスケジューラ起源では拒否されること（セクション 7 (b) の既存檻）と対になる、
    //     完了状態が要求する「両方向」の許可側。
    #[test]
    fn choice_origin_scheduler_forbidden_ids_are_sent_verbatim() {
        for forbidden_for_scheduler in ["OnTalk", "OnHour"] {
            let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);

            let outcome = round_trip_request(&shiori_tx, probe_choice_call(forbidden_for_scheduler));

            assert!(
                matches!(outcome, ShioriOutcome::NoContent),
                "{forbidden_for_scheduler} は選択起源なら送出され mock の良性応答が返るべき（Req2.9）"
            );
            // wire へ載る ID は逐語（イベント名の書き換え・別 ID への写像をしない・Req2.6）。
            assert_eq!(
                rec_rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("選択起源の許可 ID はチャネルへ送出され mock が受領するはず"),
                Recorded::Get(forbidden_for_scheduler.to_string())
            );

            drop(shiori_tx);
            drop(shiori_handle);
        }
    }

    // (a') 対の拒否側（Req2.9 の非交差を 1 テスト内で明示）: 同じ `OnTalk` でもスケジューラ起源は
    //      従来どおり送出されず `event_id_not_allowed` を記録して Failed(Internal) へ写像される。
    #[test]
    fn same_id_is_allowed_from_choice_origin_and_rejected_from_scheduler_origin() {
        // 選択起源: 送出される。
        let (choice_tx, choice_rx, choice_handle) = spawn_mock_shiori(benign);
        let choice_outcome = round_trip_request(&choice_tx, probe_choice_call("OnTalk"));
        assert!(
            matches!(choice_outcome, ShioriOutcome::NoContent),
            "OnTalk は選択起源なら送出されるべき（Req2.9）"
        );
        assert_eq!(
            choice_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("選択起源 OnTalk はチャネルへ送出されるはず"),
            Recorded::Get("OnTalk".to_string())
        );
        drop(choice_tx);
        drop(choice_handle);

        // スケジューラ起源: 送出されず error!＋Failed(Internal)（恒久禁止は Static 側で不変）。
        let (static_tx, static_rx, static_handle) = spawn_mock_shiori(benign);
        let mut static_outcome: Option<ShioriOutcome> = None;
        let events = capture(|| {
            static_outcome = Some(round_trip_request(
                &static_tx,
                ShioriCall::Get {
                    id: EventId::Static("OnTalk"),
                    references: Vec::new(),
                    status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
                },
            ));
        });
        assert!(
            matches!(
                static_outcome,
                Some(ShioriOutcome::Failed(ShioriFailure::Internal(_)))
            ),
            "OnTalk はスケジューラ起源では従来どおり Failed(Internal) へ写像されるべき（Req3.2）"
        );
        assert_logged(&events, Level::ERROR, "event_id_not_allowed");
        assert!(
            static_rx.try_recv().is_err(),
            "スケジューラ起源 OnTalk はチャネルへ送出されてはならない（Req3.2）"
        );
        drop(static_tx);
        drop(static_handle);
    }

    // (b) 選択起源の `On` 非接頭 ID → 送出されず `event_id_not_allowed` を記録して
    //     Failed(Internal) へ写像される（受理規則違反も従来どおりの拒否語彙・design C6）。
    #[test]
    fn choice_origin_without_on_prefix_is_not_sent_and_logs_error() {
        for rejected in ["foo", "on", ""] {
            let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);

            let mut outcome: Option<ShioriOutcome> = None;
            let events = capture(|| {
                outcome = Some(round_trip_request(&shiori_tx, probe_choice_call(rejected)));
            });

            assert!(
                matches!(
                    outcome,
                    Some(ShioriOutcome::Failed(ShioriFailure::Internal(_)))
                ),
                "{rejected:?} は On 接頭でないゆえ送出せず Failed(Internal) へ写像されるべき"
            );
            assert_logged(&events, Level::ERROR, "event_id_not_allowed");
            assert!(
                rec_rx.try_recv().is_err(),
                "{rejected:?} はチャネルへ送出されてはならない"
            );

            drop(shiori_tx);
            drop(shiori_handle);
        }
    }

    // (c) 境界入力: 選択起源の `"On"` 単独は受理され送出される（接頭辞判定ただ 1 条件の境界）。
    #[test]
    fn choice_origin_bare_on_is_accepted_and_sent() {
        let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);

        let outcome = round_trip_request(&shiori_tx, probe_choice_call("On"));

        assert!(
            matches!(outcome, ShioriOutcome::NoContent),
            "\"On\" 単独は On 接頭ゆえ受理され送出されるべき（境界入力）"
        );
        assert_eq!(
            rec_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("\"On\" はチャネルへ送出され mock が受領するはず"),
            Recorded::Get("On".to_string())
        );

        drop(shiori_tx);
        drop(shiori_handle);
    }

    // (d) 選択関連の固定 3 ID は**スケジューラ起源**（`Static`）として送出される（DD-2 の表追加）。
    #[test]
    fn choice_fixed_ids_pass_the_static_guard_and_are_sent() {
        for id in ["OnChoiceSelectEx", "OnChoiceSelect", "OnChoiceTimeout"] {
            let (shiori_tx, rec_rx, shiori_handle) = spawn_mock_shiori(benign);

            let outcome = round_trip_request(
                &shiori_tx,
                ShioriCall::Get {
                    id: EventId::Static(id),
                    references: vec!["ID".to_string()],
                    status: ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE),
                },
            );

            assert!(
                matches!(outcome, ShioriOutcome::NoContent),
                "{id} は許可表に載ったため Static ガードを通過して送出されるべき（DD-2）"
            );
            assert_eq!(
                rec_rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("固定 3 ID はチャネルへ送出され mock が受領するはず"),
                Recorded::Get(id.to_string())
            );

            drop(shiori_tx);
            drop(shiori_handle);
        }
    }

    // --- 11. Action → TalkCommand 写像（タスク 4.1・design C6・Req4.4） ---
    //
    // drive の Action バッチ実行本体（[`execute_actions`]）を**直接**呼び、選択系 2 Action が
    // `TalkCommand::{ResolveChoice, CancelChoice}` へそのまま包まれて単一チャンネルへ出ることを
    // 実行で観測する。発行点は `ResolveChoice`＝選択調停（steady・タスク 4.3）・`CancelChoice`
    // ＝タイムアウト解除（steady・タスク 4.5）と別々の入力に分かれており、`step` 経由では 3 形を
    // 1 バッチに揃えて駆動できない（順序保存の契約は 1 バッチでしか観測できない）。写像を担う
    // 実コードはここで通る（檻専用の写像を別に作らない）。

    /// 檻用の talk 指示 3 形バッチ（投函順が下流での観測順である＝順序保存の契約）。
    fn talk_action_batch() -> Vec<Action> {
        use crate::talk::{StartTalk, TalkId};
        vec![
            Action::ResolveChoice {
                talk_id: TalkId(7),
                id: "OnMenu".to_string(),
            },
            Action::StartTalk(StartTalk::new(TalkId(8), r"\0next\e")),
            Action::CancelChoice {
                talk_id: TalkId(8),
            },
        ]
    }

    #[test]
    fn choice_actions_map_to_talk_commands_and_preserve_order() {
        use crate::talk::TalkId;

        let (shiori_tx, _rec_rx, shiori_handle) = spawn_mock_shiori(benign);
        let (sakura_tx, sakura_rx) = mpsc::channel::<TalkCommand>();

        let result = execute_actions(talk_action_batch(), &shiori_tx, &sakura_tx, &noop_sink());

        assert!(
            result.last_reply.is_none(),
            "talk 指示は SHIORI 往復を起こさない（再投入対象にならない）"
        );
        assert!(!result.stop, "talk 指示は停止要求ではない");

        let got: Vec<TalkCommand> = sakura_rx.try_iter().collect();
        assert_eq!(got.len(), 3, "3 形すべてが同一チャンネルへ送出される");
        match &got[0] {
            TalkCommand::ResolveChoice { talk_id, id } => {
                assert_eq!(*talk_id, TalkId(7), "ResolveChoice の talk_id をそのまま包む");
                assert_eq!(id, "OnMenu", "ResolveChoice の id をそのまま包む");
            }
            other => panic!("commands[0] は ResolveChoice のはず: {other:?}"),
        }
        match &got[1] {
            TalkCommand::Start(start) => {
                assert_eq!(start.talk_id, TalkId(8), "既存 StartTalk 写像は不変");
                assert_eq!(start.script, r"\0next\e");
            }
            other => panic!("commands[1] は Start のはず: {other:?}"),
        }
        match &got[2] {
            TalkCommand::CancelChoice { talk_id } => {
                assert_eq!(*talk_id, TalkId(8), "CancelChoice の talk_id をそのまま包む");
            }
            other => panic!("commands[2] は CancelChoice のはず: {other:?}"),
        }

        drop(shiori_tx);
        drop(shiori_handle);
    }

    // 送出失敗（受信端 drop）でも `error!(talk_command_send_failed)` を残して**バッチ実行は継続**する
    // （design「Error Strategy」: 選択・再生の失敗でゴーストを終了させない）。継続の証跡として、
    // 失敗する talk 指示 2 件の**後ろ**に置いた ShioriUnload が実行され last_reply が満ちることを見る。
    #[test]
    fn talk_command_send_failure_does_not_abort_the_action_batch() {
        use crate::talk::TalkId;

        let (shiori_tx, _rec_rx, shiori_handle) = spawn_mock_shiori(benign);
        let (sakura_tx, sakura_rx) = mpsc::channel::<TalkCommand>();
        drop(sakura_rx); // talk 指示の送出は必ず Err。

        let mut result: Option<BatchResult> = None;
        let events = capture(|| {
            result = Some(execute_actions(
                vec![
                    Action::ResolveChoice {
                        talk_id: TalkId(7),
                        id: "OnMenu".to_string(),
                    },
                    Action::CancelChoice {
                        talk_id: TalkId(7),
                    },
                    Action::ShioriUnload,
                ],
                &shiori_tx,
                &sakura_tx,
                &noop_sink(),
            ));
        });

        assert_logged(&events, Level::ERROR, "talk_command_send_failed");
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event.as_deref() == Some("talk_command_send_failed"))
                .count(),
            2,
            "選択系 2 形の送出失敗が個別に記録される（沈黙の失敗経路なし）"
        );
        let result = result.expect("execute_actions returns");
        assert!(
            matches!(result.last_reply, Some((ShioriOutcome::Unloaded, "Unload"))),
            "送出失敗の後ろに置いた Action も実行される＝バッチは中断しない（運行継続）"
        );
        assert!(!result.stop);

        drop(shiori_tx);
        drop(shiori_handle);
    }
}
