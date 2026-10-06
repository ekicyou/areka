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

use areka_actor::{ActorHandle, ReplyError, ReplySender, reply_channel, run_inbox, spawn_actor};

use crate::change::{ChangeHandoff, KanadeNotice, RaiseOutcome, TalkGap};
use crate::msg::{
    EventId, KanadeConfig, KanadeMsg, KanadeStopCause, KanadeStopped, ShioriCall, ShioriFailure,
    ShioriFault, ShioriMsg, ShioriOutcome,
};
use crate::online::OnlineCounter;
use crate::schedule::resources::ResourceSink;
use crate::schedule::translate::TranslateResult;
use crate::schedule::{Action, Input, Phase, State, TermCause, step};
use crate::shiori::real::ABSENT_REFERENCE;
use crate::status::ExecutionStatus;
use crate::talk::TalkCommand;
use crate::translate::TranslateSeams;

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
    spawn_kanade_with_stop_sink(config, shiori, sakura, resource_sink, None)
}

/// 停止通知の投函端つきで kanade アクターを起動する（R15.3・design D15 の 2）。
///
/// 引数は [`spawn_kanade`] と同一で、末尾に `stop_sink` が 1 つ増えるだけの派生である
/// （[`spawn_kanade`] は `None` を渡す薄い包み＝既存の呼び手は 1 つも変わらない）。
///
/// `stop_sink` は運行の通知（[`KanadeNotice`]）の送出端である。`Some` なら、[`Action::StopSelf`]
/// の実行点で [`KanadeNotice::Stopped`]`(`[`KanadeStopped`]`{ cause, handoff })` を**ちょうど 1 度**
/// 送る。受信端が既に落ちていても `warn!`（`event="stop_notify_failed"`）を残して停止は完走する
/// （panic しない・log-first）。運行表の「通知を送る」（[`Action::Notice`]）もこの送出端へ流す。
/// `None` なら通知は一切出ない（従来どおり）。
///
/// # 誰が受けるのか
///
/// 本番は UI スレッドの毎フレーム結線（`areka` の `emo2_boot::frame`）が受信端を持ち、通知 1 件で
/// 全ゴースト窓を閉じる。ゴーストの終了挨拶が終わってから窓が消えるという順序は、この通知が
/// **終了系列の完了後**に出ることで成立する（R15.4）。
///
/// 翻訳の口は素通し（[`TranslateSeams::passthrough`]）を渡す（[`spawn_kanade_translating`] の薄い包み）。
pub fn spawn_kanade_with_stop_sink(
    config: KanadeConfig,
    shiori: Sender<ShioriMsg>,
    sakura: Sender<TalkCommand>,
    resource_sink: ResourceSink,
    stop_sink: Option<Sender<KanadeNotice>>,
) -> (Sender<KanadeMsg>, ActorHandle) {
    spawn_kanade_translating(
        config,
        shiori,
        sakura,
        resource_sink,
        stop_sink,
        TranslateSeams::passthrough(),
    )
}

/// 翻訳の口つきで kanade アクターを起動する（[`spawn_kanade_with_stop_sink`] の引数に `seams` を
/// 足した派生）。`seams` の展開と MAKOTO の口は、翻訳の依頼（`OnTranslate`）を実行するたびに
/// kanade のスレッドで同期に呼ぶ（返るまで運行は進まない）。
pub fn spawn_kanade_translating(
    config: KanadeConfig,
    shiori: Sender<ShioriMsg>,
    sakura: Sender<TalkCommand>,
    resource_sink: ResourceSink,
    stop_sink: Option<Sender<KanadeNotice>>,
    seams: TranslateSeams,
) -> (Sender<KanadeMsg>, ActorHandle) {
    spawn_actor("kanade", move |rx| {
        let mut state = State::initial();
        // 台詞の切れ目の口の返信端。結果が出るまでメッセージをまたいで 1 本だけ持つ。
        let mut gap_reply: Option<ReplySender<TalkGap>> = None;
        run_inbox::<KanadeMsg, Infallible>(rx, move |msg| {
            // 汎用の通知の入口の返信端と、返事の材料（名前・許可表に在るか）。運行表へ渡す前に控える。
            let mut raise_reply: Option<(ReplySender<RaiseOutcome>, String, bool)> = None;
            // 通信中の数はどのメッセージの処理よりも先に写す（新しい kanade も Boot の前に読む＝要件 2.6・5.1）。
            sync_online(config.online, &mut state);
            // 停止規約: Close は step を経ず即時 Break（積み残しは rx drop で破棄）。
            let input = match msg {
                KanadeMsg::Close => {
                    tracing::info!(target: "kanade", event = "close", "停止指示（Close）を受領——即時停止");
                    return Ok(ControlFlow::Break(()));
                }
                // リソース照会は step を経ずその場で答える（`drive` の「最後の応答だけ再投入」に
                // 乗らない・運行状態は読むだけ）。
                KanadeMsg::ResourceQuery { ids, reply } => {
                    crate::actor_resources::answer(&state, &shiori, ids, reply);
                    return Ok(ControlFlow::Continue(()));
                }
                // 実行の状態の問い合わせも step を経ずその場で答える（運行は 1 歩も進めない）。
                KanadeMsg::StatusQuery { reply } => {
                    answer_status(&state, reply);
                    return Ok(ControlFlow::Continue(()));
                }
                KanadeMsg::Boot => Input::Boot,
                KanadeMsg::Tick { now } => Input::Tick { now },
                KanadeMsg::TalkDone(td) => Input::TalkDone(td),
                KanadeMsg::CloseRequest { reason } => Input::CloseRequest { reason },
                KanadeMsg::ForceQuit { reason } => Input::ForceQuit { reason },
                KanadeMsg::ShioriDown { kind, reason } => Input::ShioriDown { kind, reason },
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
                KanadeMsg::UserBreak { scope } => Input::UserBreak { scope },
                KanadeMsg::ChangeGhost(req) => Input::ChangeGhost(req),
                KanadeMsg::RaiseEvent {
                    id,
                    references,
                    method,
                    reply,
                } => {
                    // 許可表の照合は運行表（`on_raise_event`）と同じ関数で行う。
                    raise_reply = reply.map(|tx| {
                        let allowed = crate::schedule::events::allowed_static(&id).is_some();
                        (tx, id.clone(), allowed)
                    });
                    Input::RaiseEvent {
                        id,
                        references,
                        method,
                    }
                }
                KanadeMsg::AwaitTalkGap { raise, reply } => {
                    hold_gap_reply(&mut gap_reply, reply);
                    Input::AwaitTalkGap { raise }
                }
                KanadeMsg::ExecutionState(update) => Input::ExecutionState(update),
            };
            let (flow, first_reply) = drive_translating(
                &mut state,
                input,
                &config,
                &shiori,
                &sakura,
                &resource_sink,
                stop_sink.as_ref(),
                &seams,
            );
            // 返事はこの依頼の処理（応答を入れ直して動作を実行し終えるまで）が済んだ後に 1 回だけ送る。
            // 往復が無かったのは、許可表に無い名前か定常以外での依頼（どちらも運行表が捨てた）。
            if let Some((tx, id, allowed)) = raise_reply {
                let outcome = match (allowed, first_reply) {
                    (false, _) => RaiseOutcome::NotAllowed,
                    (true, None) => RaiseOutcome::NotSteady,
                    (true, Some(outcome)) => outcome,
                };
                send_raise_reply(tx, &id, outcome);
            }
            // 台詞の切れ目の結果は、決まった依頼の処理の後に 1 回だけ送る（停止の直前でも送る）。
            if let Some(outcome) = crate::schedule::talk_gap::take_outcome(&mut state) {
                send_gap_reply(gap_reply.take(), outcome);
            }
            match flow {
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

/// 翻訳の口を素通しにした [`drive_translating`]（既存の殻のテストの入口）。
#[cfg(test)]
fn drive(
    state: &mut State,
    input: Input,
    config: &KanadeConfig,
    shiori: &Sender<ShioriMsg>,
    sakura: &Sender<TalkCommand>,
    resource_sink: &ResourceSink,
    stop_sink: Option<&Sender<KanadeNotice>>,
) -> (Drive, Option<RaiseOutcome>) {
    let seams = TranslateSeams::passthrough();
    drive_translating(
        state,
        input,
        config,
        shiori,
        sakura,
        resource_sink,
        stop_sink,
        &seams,
    )
}

/// DD-2 同期往復ループ: `step` の Action バッチを全実行し、最後の往復の結果のみを再投入して
/// Actions が尽きるまで反復する（execute-batch/reinject-last）。入れ直すものが SHIORI の応答なら
/// `Input::ShioriReply`、翻訳の結果なら `Input::TranslateDone` を入れる。
///
/// 戻り値の 2 つ目は、最初の一括の往復の結果を [`RaiseOutcome`] へ写したもの（往復が無ければ
/// `None`）。汎用の通知の入口の返事の材料で、他の入力では呼び手が読み捨てる。
#[allow(clippy::too_many_arguments)]
fn drive_translating(
    state: &mut State,
    input: Input,
    config: &KanadeConfig,
    shiori: &Sender<ShioriMsg>,
    sakura: &Sender<TalkCommand>,
    resource_sink: &ResourceSink,
    stop_sink: Option<&Sender<KanadeNotice>>,
    seams: &TranslateSeams,
) -> (Drive, Option<RaiseOutcome>) {
    // 終了原因の受け渡し（R15.3）: [`Action::StopSelf`] は必ず `Unloading{cause}` からの遷移で
    // 生まれる（発行点は `schedule::unloading_reply` の 1 箇所だけで、そこへ入れるのは
    // `Phase::Unloading` の先行アームだけ）。その遷移で `Phase` は `Stopped` へ書き換わり原因が
    // 消えるため、**step へ渡す直前**の運行状態から原因を控えておき、StopSelf の実行点で通知へ載せる。
    // 切替の中身（帳簿の `change`）も同じ時点で控える（原因と中身が別の時点の状態を指さない）。
    let mut term = (stop_cause_of(state), handoff_of(state));
    // 初回 step。以降は state を差し替えつつ actions を回す。
    let (mut st, mut actions) = step(std::mem::replace(state, State::initial()), input, config);
    let mut first_batch = true;
    let mut first_reply = None;
    loop {
        let BatchResult {
            last_reply,
            translated,
            stop,
        } = execute_batch(
            actions,
            shiori,
            sakura,
            resource_sink,
            stop_sink,
            term,
            seams,
        );
        // `ShioriOutcome` は複製できないので、入れ直す前に参照から写す。
        if std::mem::take(&mut first_batch) {
            first_reply = last_reply
                .as_ref()
                .map(|(outcome, _)| raise_outcome_of(outcome));
        }
        if stop {
            *state = st;
            return (Drive::Stop, first_reply);
        }
        // 往復の結果を再投入して次の遷移を得る（Actions が尽きるまで反復）。origin を転記する。
        let input = match (last_reply, translated) {
            (Some((outcome, origin)), _) => Input::ShioriReply { outcome, origin },
            (None, Some(result)) => Input::TranslateDone(result),
            // バッチに往復が無い＝この入力の処理は完了。
            (None, None) => {
                *state = st;
                return (Drive::Continue, first_reply);
            }
        };
        // 停止の原因と切替の中身は、どちらの再投入でも step へ渡す直前の状態から控える。
        term = (stop_cause_of(&st), handoff_of(&st));
        let (s, a) = step(st, input, config);
        st = s;
        actions = a;
    }
}

/// 通信中の数を読み、運行状態の写しと違えば写す（運行表はグローバルを読まない・殻の 1 か所＝要件 2.5・2.6・5.2）。
fn sync_online(online: &OnlineCounter, state: &mut State) {
    let online = online.is_online();
    if state.external.online != online {
        tracing::debug!(target: "kanade", event = "online_changed", online, "通信中の写しを更新");
        state.external.online = online;
    }
}

/// 今の実行の状態に答える（読むだけ・`KanadeMsg::StatusQuery`）。
///
/// SHIORI への要求に載せる `Status` と同じ素（`State::snapshot`）・同じ導き方（`ExecutionStatus::derive`）で
/// 作り、返信端へ 1 回送る。受け手がもう居ない（上限で待つのをやめた）のは異常ではないので `debug!` だけ。
fn answer_status(state: &State, reply: ReplySender<ExecutionStatus>) {
    if reply
        .send(ExecutionStatus::derive(&state.snapshot()))
        .is_err()
    {
        tracing::debug!(
            target: "kanade",
            event = "status_query_reply_dropped",
            "状態の問い合わせの受け手は既に待ちを諦めている——答えを捨てる"
        );
    }
}

/// SHIORI の往復の結果を汎用の通知の入口の返事へ写す。
///
/// 空の台本と空白だけの台本は返事なしに数える（運行表の再生の振る舞いは変えない）。エラー応答は
/// 送出点（[`round_trip_request`]）で既に 204／NOTIFY の完了へ写っているので返事なしになる。
/// `Unloaded` は通知の入口の往復からは生じない（降ろす往復の結果）が、写しの表を閉じるため
/// 返事なしに置く。
fn raise_outcome_of(outcome: &ShioriOutcome) -> RaiseOutcome {
    match outcome {
        ShioriOutcome::Value(script) if !script.trim().is_empty() => RaiseOutcome::Script,
        ShioriOutcome::Value(_)
        | ShioriOutcome::NoContent
        | ShioriOutcome::Notified
        | ShioriOutcome::Unloaded => RaiseOutcome::NoReply,
        ShioriOutcome::Failed(_) => RaiseOutcome::Failed,
    }
}

/// 汎用の通知の入口の返事を返信端へ送る。受け手が居なくても運行は続ける（`debug!` を 1 件）。
fn send_raise_reply(reply: ReplySender<RaiseOutcome>, id: &str, outcome: RaiseOutcome) {
    if reply.send(outcome).is_err() {
        tracing::debug!(
            target: "kanade",
            event = "raise_reply_dropped",
            id,
            outcome = ?outcome,
            "イベントの結果の受け手が居ない——返事は捨てて運行を続ける"
        );
    }
}

/// 台詞の切れ目の口の返信端を控える。見張りは高々 1 つなので、控えが在れば古い方を捨て
/// （受け手には `ReplyError::Dropped` が見える）、`warn!` を 1 件残す。
fn hold_gap_reply(slot: &mut Option<ReplySender<TalkGap>>, reply: ReplySender<TalkGap>) {
    if slot.replace(reply).is_some() {
        tracing::warn!(
            target: "kanade",
            event = "talk_gap_replaced",
            "見張りの最中に次の切れ目の依頼を受けた——古い返信端を捨てて新しい依頼を見張る"
        );
    }
}

/// 台詞の切れ目の結果を返信端へ送る。受け手が居なくても運行は続ける（`debug!` を 1 件）。
///
/// 返信端が無いのは、見張りを始めるのが返信端つきの依頼だけなので起きない想定である
/// （起きたら `warn!` を残して結果を捨てる）。
fn send_gap_reply(reply: Option<ReplySender<TalkGap>>, outcome: TalkGap) {
    let Some(reply) = reply else {
        tracing::warn!(
            target: "kanade",
            event = "talk_gap_no_reply",
            outcome = ?outcome,
            "切れ目の結果が出たが返信端が無い——結果を捨てる"
        );
        return;
    };
    if let Err(unsent) = reply.send(outcome) {
        tracing::debug!(
            target: "kanade",
            event = "talk_gap_reply_dropped",
            outcome = ?unsent,
            "切れ目の結果の受け手が居ない——結果は捨てて運行を続ける"
        );
    }
}

/// Action バッチ 1 回分の実行結果（[`drive_translating`] の反復条件）。
struct BatchResult {
    /// バッチ中で最後に発生した SHIORI 往復の応答と、その応答が由来する呼出イベント ID
    /// （origin・DD-IE-3）。`None` はバッチに SHIORI 往復が無かったこと（＝再投入しない）を表す。
    last_reply: Option<(ShioriOutcome, &'static str)>,
    /// バッチの最後の往復が翻訳の依頼（[`Action::Translate`]）だったときの結果。`last_reply` とは
    /// 同時に `Some` にならない（後に実行した往復の方だけを残す）。
    translated: Option<TranslateResult>,
    /// [`Action::StopSelf`] を実行した（以降の Action は実行しない・呼び手は停止する）。
    stop: bool,
}

/// Action バッチを**先頭から順に全て実行**する（execute-batch/reinject-last の execute 側）。
///
/// [`drive_translating`] の反復本体から切り出してあるのは、[`Action`] → [`TalkCommand`] 写像を発行点
/// （タスク 4.3／4.5）の実装を待たずに実行で檻に入れられるようにするためである（design C6）。
///
/// # talk 指示 3 形の写像（design C6・DD-5・Req 5.6）
/// [`Action::StartTalk`]／[`Action::ResolveChoice`]／[`Action::CancelChoice`] はそれぞれ
/// [`TalkCommand::Start`]／[`TalkCommand::ResolveChoice`]／[`TalkCommand::CancelChoice`] へ
/// **そのまま包んで**同一チャンネルへ送出する（値の解釈・書き換えをしない）。起動系と解決系を
/// 別チャンネルへ分けないことが順序保存の契約であり、状態機械が 1 バッチで並べた順序が
/// そのまま下流で観測される。送出失敗は [`send_talk_command`] が `error!` を残し**運行は継続**
/// する——バッチも中断しない（design「Error Strategy」: 選択・再生の失敗でゴーストを終了させない）。
///
/// # 停止通知（R15.3・design D15 の 2）
/// `stop_sink` が `Some` のとき、[`Action::StopSelf`] の実行点で [`KanadeNotice::Stopped`] を
/// 1 度だけ送る。[`Action::Notice`] はそのままの値で同じ送出端へ流す（[`send_notice`]）。
/// `term` は呼び手（[`drive_translating`]）が step へ渡す直前の運行状態から控えた（原因, 切替の中身）である。
///
/// [`Action::Translate`] は `actor_translate::run_translate` で実行し、その結果をこのバッチの
/// 「入れ直すもの」にする。
fn execute_batch(
    actions: Vec<Action>,
    shiori: &Sender<ShioriMsg>,
    sakura: &Sender<TalkCommand>,
    resource_sink: &ResourceSink,
    stop_sink: Option<&Sender<KanadeNotice>>,
    term: (Option<KanadeStopCause>, Option<ChangeHandoff>),
    seams: &TranslateSeams,
) -> BatchResult {
    let mut last_reply: Option<(ShioriOutcome, &'static str)> = None;
    let mut translated: Option<TranslateResult> = None;
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
                translated = None;
            }
            Action::Translate(request) => {
                translated = Some(crate::actor_translate::run_translate(
                    request, shiori, seams,
                ));
                last_reply = None;
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
                translated = None;
            }
            Action::Notice(notice) => send_notice(stop_sink, notice),
            Action::StopSelf => {
                // 終了系列完了: shiori へ Close を送り自身も停止する。
                let _ = shiori.send(ShioriMsg::Close);
                // 終了系列の完了を UI へ 1 度だけ知らせる（R15.3）。shiori を畳んだ**後**に置く
                // ——受け手はこの通知で窓を閉じるので、通知が先に出ると解放より先に窓が消え得る。
                notify_stop(stop_sink, term.0, term.1);
                return BatchResult {
                    last_reply,
                    translated,
                    stop: true,
                };
            }
        }
    }
    BatchResult {
        last_reply,
        translated,
        stop: false,
    }
}

/// 翻訳の口を素通しにした [`execute_batch`]（既存の殻のテストの入口）。
#[cfg(test)]
fn execute_actions(
    actions: Vec<Action>,
    shiori: &Sender<ShioriMsg>,
    sakura: &Sender<TalkCommand>,
    resource_sink: &ResourceSink,
    stop_sink: Option<&Sender<KanadeNotice>>,
    term: (Option<KanadeStopCause>, Option<ChangeHandoff>),
) -> BatchResult {
    let seams = TranslateSeams::passthrough();
    execute_batch(
        actions,
        shiori,
        sakura,
        resource_sink,
        stop_sink,
        term,
        &seams,
    )
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
/// - SHIORI のエラー応答（`Failed(ShioriFailure::Shiori)`）は、GET なら `NoContent`、NOTIFY なら
///   `Notified` に写して返し、`warn!`（event=`shiori_error_response`）を 1 件残す（会話を続ける）。
///
/// 検査と送出は [`round_trip_raw`] が受け持ち、ここはエラー応答の写しだけを足す。
pub(crate) fn round_trip_request(shiori: &Sender<ShioriMsg>, call: ShioriCall) -> ShioriOutcome {
    // エラー応答の記録に載せるため、call を渡す前に種別と ID を控える（固定 ID なら複製は無料）。
    let (method, event_id) = match &call {
        ShioriCall::Get { id, .. } => ("GET", id.clone()),
        ShioriCall::Notify { id, .. } => ("NOTIFY", id.clone()),
    };
    let outcome = round_trip_raw(shiori, call);

    // エラー応答（400・500 など）は致命の失敗にせず「返事なし」に写す（shiori-fault-notice
    // 要件 6.1・裁定 3）: GET は 204 相当の NoContent、NOTIFY は Notified として再投入する。
    // 運行表は応答が GET か NOTIFY かを知らないので、種別を知るここで写す。回数の閾値は置かない。
    // 他の 4 種（接続・期限切れ・通信・内部）は Failed のまま（Fault の判断は運行表のまま）。
    match outcome {
        ShioriOutcome::Failed(ShioriFailure::Shiori(e)) => {
            tracing::warn!(
                target: "kanade",
                event = "shiori_error_response",
                method,
                id = %event_id.as_str(),
                error = %e,
                "SHIORI がエラー応答——返事なし（204）と同じ扱いで会話を続ける"
            );
            if method == "GET" {
                ShioriOutcome::NoContent
            } else {
                ShioriOutcome::Notified
            }
        }
        other => other,
    }
}

/// 送出の檻を通して送り、SHIORI の結果を写さずに返す（エラー応答も `Failed` のまま）。
///
/// 許可表の検査・欠番の印の置き換え・`shiori_request` の記録・往復をここで行う。今日の呼び手は
/// [`round_trip_request`] を通し、翻訳の往復（`actor_translate`）はエラー応答を自分で読むので
/// これを直接呼ぶ（`shiori_error_response` を重ねて出さない）。
///
/// Reference の値が欠番の印（`ABSENT_REFERENCE`）と同じ位置は、`OnTranslate` の添字 1 を除き
/// 空文字に置き換えて `warn!`（event=`reference_absent_marker_replaced`）を 1 件ずつ残す。外から
/// 入った値が線の上で行ごと消えるのを止めるためで、送る 1 か所のここで全イベントに効かせる。
pub(crate) fn round_trip_raw(shiori: &Sender<ShioriMsg>, mut call: ShioriCall) -> ShioriOutcome {
    replace_absent_markers(&mut call);
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

/// Reference のうち欠番の印と同じ値を空文字に置き換え、位置ごとに `warn!` を 1 件残す。
///
/// 印を正当に載せるのは `OnTranslate` の添字 1（[`crate::events::on_translate`] が置く欠番）だけ。
fn replace_absent_markers(call: &mut ShioriCall) {
    let (ShioriCall::Get { id, references, .. } | ShioriCall::Notify { id, references, .. }) = call;
    for (index, value) in references.iter_mut().enumerate() {
        if value != ABSENT_REFERENCE || (index == 1 && matches!(id, EventId::Static("OnTranslate")))
        {
            continue;
        }
        tracing::warn!(
            target: "kanade",
            event = "reference_absent_marker_replaced",
            id = %id.as_str(),
            index,
            "Reference が欠番の印と同じ値——行が消えないよう空文字に置き換えて送る"
        );
        value.clear();
    }
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

/// 運行状態が終了系列（`Unloading{cause}`）に居るなら、その原因を公開語彙へ写す（R15.3）。
/// 運行状態は手放さないので、原因は clone して写す（Fault は種類と理由ごと）。
///
/// 写すのはここ 1 箇所だけである——内部の `TermCause` は `pub(crate)` に閉じたままで、公開面へ
/// 出るのは [`KanadeStopCause`] の 5 値だけになる（DD-9 の露出規律）。終了系列でなければ `None`。
///
/// wildcard を置かないため、`TermCause` に値が増えたときは本表での判断がコンパイル時に要求される。
fn stop_cause_of(state: &State) -> Option<KanadeStopCause> {
    let Phase::Unloading { cause } = &state.phase else {
        return None;
    };
    Some(match cause {
        TermCause::Quit => KanadeStopCause::Quit,
        TermCause::Forced => KanadeStopCause::Forced,
        TermCause::CloseSilent => KanadeStopCause::CloseSilent,
        TermCause::DeadlineExceeded => KanadeStopCause::DeadlineExceeded,
        TermCause::Fault(fault) => KanadeStopCause::Fault(fault.clone()),
    })
}

/// 運行状態の切替の帳簿を、停止通知に載せる切替の中身へ写す（切替の相を経ていなければ `None`）。
fn handoff_of(state: &State) -> Option<ChangeHandoff> {
    state.change.as_ref().map(|change| ChangeHandoff {
        script: change.script.clone(),
    })
}

/// 停止通知を投函する（R15.3・design D15 の 2）。
///
/// `sink` が `None` なら何もしない（通知端を結線していない構成＝既存の呼び手はすべてこちら）。
/// 送出失敗は受信端（UI）が既に落ちていることを意味するだけで異常ではないが、**沈黙で捨てる
/// 経路を作らない**（steering: areka-log-first-no-silent-failure）ため `warn!` を 1 件残す。
/// panic はしない——ここで落ちると終了系列そのものが完走しなくなる。
///
/// `cause` が `None` になるのは、`Unloading` を経ずに `StopSelf` が現れた場合だけである
/// （現在の運行表には存在しない経路）。その場合も通知は出す——窓を閉じる合図としての意味は
/// 原因に依らないためで、原因不明であることは記録に残し、種類 `Unknown` の Fault として送る。
fn notify_stop(
    sink: Option<&Sender<KanadeNotice>>,
    cause: Option<KanadeStopCause>,
    handoff: Option<ChangeHandoff>,
) {
    let Some(tx) = sink else {
        return;
    };
    if cause.is_none() {
        tracing::warn!(
            target: "kanade",
            event = "stop_cause_unknown",
            "終了系列の原因を控えられないまま StopSelf に至った——Fault として通知する"
        );
    }
    let cause = cause.unwrap_or_else(|| KanadeStopCause::Fault(ShioriFault::unknown()));
    // 送出に失敗した値は SendError に載って戻るので、記録にはそれを使う（clone しない）。
    let stopped = KanadeStopped { cause, handoff };
    // 戻ってくるのは送った「停止」そのものなので、腕は 1 つで足りる。
    if let Err(std::sync::mpsc::SendError(KanadeNotice::Stopped(unsent))) =
        tx.send(KanadeNotice::Stopped(stopped))
    {
        tracing::warn!(
            target: "kanade",
            event = "stop_notify_failed",
            cause = ?unsent.cause,
            "停止通知の送出に失敗（受信端＝UI は既に切断）——停止は完走する"
        );
    }
}

/// 運行表の「通知を送る」（[`Action::Notice`]）を送出端へ流す。
///
/// 送出端が無い構成（既存の呼び手）では `debug!` を 1 件残すだけ。受け口が消えていれば
/// `warn!`（`event="notice_send_failed"`）を残して続ける（panic しない・運行は止めない）。
fn send_notice(sink: Option<&Sender<KanadeNotice>>, notice: KanadeNotice) {
    let Some(tx) = sink else {
        tracing::debug!(
            target: "kanade",
            event = "notice_no_sink",
            notice = ?notice,
            "運行の通知の送出端が無い——通知は送らない"
        );
        return;
    };
    if let Err(std::sync::mpsc::SendError(unsent)) = tx.send(notice) {
        tracing::warn!(
            target: "kanade",
            event = "notice_send_failed",
            notice = ?unsent,
            "運行の通知の送出に失敗（受信端＝UI は既に切断）——運行は続ける"
        );
    }
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;

// 停止通知の発行点そのものの檻（areka-P0-emo2-conformance-e2e タスク 6.9・R15.3）。
// 発行は呼出スレッドで同期に走る素の関数ゆえ、スレッドローカルの捕捉窓で観測できる
// （アクタースレッドを跨がない＝全スレッド捕捉の窓口を要さない）。
#[cfg(test)]
#[path = "actor_stop_notify_tests.rs"]
mod stop_notify_tests;

// エラー応答の写しの檻（shiori-fault-notice 要件 6.1・7.3）。
#[cfg(test)]
#[path = "actor_error_response_tests.rs"]
mod error_response_tests;

// 汎用の通知の入口の返事の檻（areka-P0-ghost-install 要件 2.6・2.7・11.5・12.6）。
#[cfg(test)]
#[path = "actor_raise_reply_tests.rs"]
mod raise_reply_tests;

// 台詞の切れ目の口の返信端の檻（areka-P0-shell-balloon-switch 要件 1.14・2.3・5.7・8.11）。
#[cfg(test)]
#[path = "actor_talk_gap_tests.rs"]
mod talk_gap_tests;

// 殻が通信中の数を毎メッセージ読んで写しへ渡すことの檻（areka-P0-status-execution-states 要件 2.5・2.6・5.1）。
#[cfg(test)]
#[path = "actor_online_tests.rs"]
mod online_tests;

// バッチ実行の翻訳の腕が入れ直すものを決めることの檻（areka-P0-translate-pipeline タスク 3.3）。
#[cfg(test)]
#[path = "actor_translate_batch_tests.rs"]
mod translate_batch_tests;
