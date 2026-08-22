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
        ShioriCall::Get {
            id,
            references,
            status,
        } => ("GET", id, references, status.render()),
        ShioriCall::Notify {
            id,
            references,
            status,
        } => ("NOTIFY", id, references, status.render()),
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
    round_trip(
        shiori,
        ShioriMsg::Request {
            call,
            reply: reply_tx,
        },
        reply_rx,
    )
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
#[path = "actor_tests.rs"]
mod tests;
