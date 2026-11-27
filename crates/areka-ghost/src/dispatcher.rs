//! sakura dispatcher（永続的な要求元⇄一時的な再生アクターの非対称吸収）。
//!
//! 再生開始要求を受けて per-talk の再生アクターを起動し、同時に 1 本だけ再生する単一 slot を維持する。
//! 完了通知の中継・stale 通知の棄却・Close funnel・Tick 中継を担う。UI の押下の選択（選択肢・
//! アンカー）は判断せずに kanade へ取り次ぐ。選択待ちの始まりの知らせと同じ inbox に並べるので、
//! 押下の選択がそれを追い越さない。
//!
//! # self-sender ハンドオフ
//!
//! per-talk transient（[`spawn_talk`]）の完了通知ポートには dispatcher 自身の inbox
//! （`Sender<DispatcherMsg>`）のクローンが要るが、[`spawn_actor`] は body へ `Receiver` しか
//! 渡さない。そこで `spawn_actor` の直後（`spawn_dispatcher` が `tx` を返す前）に、body が受信ループの
//! 前でブロックしている `self_tx_recv.recv` へ [`reply_channel`] で `tx.clone()` を一度だけ送る。
//! この self-sender を持つので inbox は全 `Sender` drop で切断されず、唯一の停止経路は `Close`。

use std::ops::ControlFlow;
use std::sync::mpsc::Sender;

use areka_actor::{ActorHandle, reply_channel, run_inbox, spawn_actor};
use areka_kanade::{AnchorInput, ChoiceInput, KanadeMsg};
use areka_sakura::contract::{
    ChoiceWaiting, CueSink, SakuraMsg, StartTalk, TalkCommand, TalkDone, TalkHandle, TalkId,
};
use areka_sakura::drive::spawn_talk;
use areka_sakura::strict::StrictHooks;
use dola::TimePoint;

use crate::runtime::SystemVarSource;
use crate::sink::BootCueSink;

/// dispatcher の inbox（1 アクター 1 enum・areka-actor inbox 規約）。
pub enum DispatcherMsg {
    /// kanade からの talk 起動。
    Start(StartTalk),
    /// per-talk transient からの完了通知。
    Done(TalkDone),
    /// ticker からの時刻前進（運ぶ時刻を変えずに active talk へ中継）。
    Tick { now: TimePoint },
    /// 停止規約の Close（active talk を終了させてから停止）。
    Close,
    /// kanade からの選択待ちバリア解決指示。`talk_id` は stale ガード用、`id` は選択肢 ID。
    ResolveChoice { talk_id: TalkId, id: String },
    /// kanade からの選択待ち解除＋トーク終了指示（Close funnel へ写像される）。
    CancelChoice { talk_id: TalkId },
    /// per-talk transient からの選択待ち成立通知。
    ChoiceWaiting(ChoiceWaiting),
    /// UI の押下の選択（選択肢）。判断せず `KanadeMsg::Choice` としてそのまま送る。
    Choice(ChoiceInput),
    /// UI の押下の選択（アンカー）。判断せず `KanadeMsg::Anchor` としてそのまま送る。
    Anchor(AnchorInput),
}

impl From<StartTalk> for DispatcherMsg {
    fn from(start: StartTalk) -> Self {
        DispatcherMsg::Start(start)
    }
}

/// kanade の talk 指示チャンネル（`Sender<TalkCommand>`）から start-relay 経由で投函される
/// 唯一の変換点。単一チャンネル＋単一 inbox を通ることが `TalkCommand` の順序保存の前提なので、
/// 系統別にチャンネルを分けてはならない。
impl From<TalkCommand> for DispatcherMsg {
    fn from(command: TalkCommand) -> Self {
        match command {
            TalkCommand::Start(start) => DispatcherMsg::Start(start),
            TalkCommand::ResolveChoice { talk_id, id } => {
                DispatcherMsg::ResolveChoice { talk_id, id }
            }
            TalkCommand::CancelChoice { talk_id } => DispatcherMsg::CancelChoice { talk_id },
        }
    }
}

impl From<TalkDone> for DispatcherMsg {
    fn from(done: TalkDone) -> Self {
        DispatcherMsg::Done(done)
    }
}

/// per-talk transient の done ポート境界（`D: From<TalkDone> + From<ChoiceWaiting>`）を満たす変換。
/// `TalkDone` と同一ポートを流れるので、同一 talk の選択待ち成立と再生完了の因果順が保たれる。
impl From<ChoiceWaiting> for DispatcherMsg {
    fn from(waiting: ChoiceWaiting) -> Self {
        DispatcherMsg::ChoiceWaiting(waiting)
    }
}

/// `crate::ticker::Tick` からの変換（ticker 側は `D: From<Tick>` を要求する）。
impl From<crate::ticker::Tick> for DispatcherMsg {
    fn from(tick: crate::ticker::Tick) -> Self {
        DispatcherMsg::Tick { now: tick.now }
    }
}

/// 稼働中 talk の単一 slot 内容。
struct ActiveTalk {
    /// talk 相関 ID（stale 通知の棄却判定）。
    talk_id: TalkId,
    /// per-talk transient への投函端＋join ハンドル。
    handle: TalkHandle,
}

/// dispatcher body の状態（`Option<ActiveTalk>` が単一 slot）。
struct DispatcherState {
    active: Option<ActiveTalk>,
    kanade: Sender<KanadeMsg>,
    /// 構築時注入の sink 列（登録順＝broadcast 順）。talk 起動ごとに各要素を `clone_box` して
    /// `spawn_talk` へ渡す。
    sinks: Vec<Box<dyn BootCueSink>>,
    /// システム変数の供給シーム。talk 起動ごとに一度呼んで凍結スナップショットを得、`spawn_talk` へ渡す。
    system_vars: SystemVarSource,
    /// strict の記録の束ね（ゴースト名と判定）。`None` は判定の差し込みが無い構築。
    strict: Option<StrictHooks>,
    /// per-talk transient へ渡す自身の inbox クローン（self-sender ハンドオフ）。
    self_sender: Sender<DispatcherMsg>,
}

impl DispatcherState {
    fn handle(&mut self, msg: DispatcherMsg) -> ControlFlow<()> {
        match msg {
            DispatcherMsg::Start(start) => self.on_start(start),
            DispatcherMsg::Done(done) => self.on_done(done),
            DispatcherMsg::Tick { now } => self.on_tick(now),
            DispatcherMsg::Close => self.on_close(),
            DispatcherMsg::ResolveChoice { talk_id, id } => self.on_resolve_choice(talk_id, &id),
            DispatcherMsg::CancelChoice { talk_id } => self.on_cancel_choice(talk_id),
            DispatcherMsg::ChoiceWaiting(waiting) => self.on_choice_waiting(waiting),
            DispatcherMsg::Choice(input) => self.forward_selection(
                input.id.clone(),
                input.scope,
                KanadeMsg::Choice(input),
                "choice_forward_failed",
            ),
            DispatcherMsg::Anchor(input) => self.forward_selection(
                input.id.clone(),
                input.scope,
                KanadeMsg::Anchor(input),
                "anchor_forward_failed",
            ),
        }
    }

    /// UI の押下の選択（`Choice`／`Anchor`）を kanade へそのまま送る。
    ///
    /// slot とは突き合わせない（stale の棄却は kanade が決める）。inbox に入った順に kanade へ着き、
    /// `ChoiceWaiting` は UI が選択待ちの始まりを受けるより先に inbox へ入るので、押下の選択は必ずその後に着く。
    /// kanade の停止後は UI 側（`choice_drain`）と同じ名前の `warn!` を残して続ける（panic しない）。
    fn forward_selection(
        &self,
        id: String,
        scope: u32,
        msg: KanadeMsg,
        forward_failed: &'static str,
    ) -> ControlFlow<()> {
        if self.kanade.send(msg).is_err() {
            tracing::warn!(
                event = forward_failed,
                scope,
                id = %id,
                "kanade Sender 送出失敗（actor 停止後）: 記録して次の伝言へ継続"
            );
        }
        ControlFlow::Continue(())
    }

    /// 現行 slot が占有している talk の `talk_id`（空なら `None`）。
    ///
    /// 選択系 3 アームの stale 判定（一致＝現行／不一致・空＝stale）の基準。`Done` の stale 判定
    /// （[`on_done`](Self::on_done)）と同じく現行 slot と突き合わせる。
    fn current_talk_id(&self) -> Option<TalkId> {
        self.active.as_ref().map(|active| active.talk_id)
    }

    /// `ResolveChoice` 受領。
    ///
    /// 現行 slot と `talk_id` が一致するときのみ、選択肢 ID を無改変で `SakuraMsg::ResolveChoice{id}` として
    /// talk へ転送する。バリアを解くだけで talk の同一性も時刻起点も変えないので、slot は
    /// 不変（バリア状態を dispatcher 側に複製しない）。
    ///
    /// 不一致・slot 空は stale（既に差し替わった／終了した選択待ち宛の遅延指示）として
    /// `resolve_choice_stale` で info 棄却する。転送の送出失敗（talk が直前に消滅）は debug で記録して続ける。
    fn on_resolve_choice(&mut self, talk_id: TalkId, id: &str) -> ControlFlow<()> {
        match self.active.as_ref() {
            Some(active) if active.talk_id == talk_id => {
                if active
                    .handle
                    .inbox
                    .send(SakuraMsg::ResolveChoice { id: id.to_string() })
                    .is_err()
                {
                    tracing::debug!(
                        talk_id = talk_id.0,
                        choice_id = %id,
                        "active talk inbox disconnected; dropping ResolveChoice relay (talk already ended)"
                    );
                }
            }
            _ => {
                tracing::info!(
                    event = "resolve_choice_stale",
                    talk_id = talk_id.0,
                    choice_id = %id,
                    current_talk_id = ?self.current_talk_id().map(|t| t.0),
                    "stale ResolveChoice discarded (slot already replaced or empty)"
                );
            }
        }
        ControlFlow::Continue(())
    }

    /// `CancelChoice` 受領。
    ///
    /// 一致時は `SakuraMsg::Close` を talk へ転送するだけで、slot も join ハンドルも保持する。
    /// [`close_active_if_any`](Self::close_active_if_any) を使って即 join＋slot 解放すると、talk が返す
    /// `TalkDone{Interrupted}` が [`on_done`](Self::on_done) の一致判定で stale 化し、kanade が選択待ちから
    /// 復帰できなくなる。slot を維持して中断 ACK を正規経路で kanade へ転送させる。
    ///
    /// 不一致・slot 空は `cancel_choice_stale` で info 棄却する（現行 talk を巻き添えにしない）。
    /// 転送の送出失敗（talk が直前に消滅）は debug で記録して続ける。その talk は既に自力で終端しており、
    /// `TalkDone` が別途 slot を解放する。
    fn on_cancel_choice(&mut self, talk_id: TalkId) -> ControlFlow<()> {
        match self.active.as_ref() {
            Some(active) if active.talk_id == talk_id => {
                if active.handle.inbox.send(SakuraMsg::Close).is_err() {
                    tracing::debug!(
                        talk_id = talk_id.0,
                        "active talk inbox disconnected; dropping CancelChoice Close relay (talk already ended)"
                    );
                }
            }
            _ => {
                tracing::info!(
                    event = "cancel_choice_stale",
                    talk_id = talk_id.0,
                    current_talk_id = ?self.current_talk_id().map(|t| t.0),
                    "stale CancelChoice discarded (slot already replaced or empty)"
                );
            }
        }
        ControlFlow::Continue(())
    }

    /// `ChoiceWaiting` 受領。
    ///
    /// 一致時のみ、talk が通知した表示の終わり（再生機の占有の終わり・刻みと同じ物差しの絶対の時刻）と
    /// タイムアウト指令を換算せずに `KanadeMsg::ChoiceWaiting` として kanade へ転送する（deadline 写像は kanade）。
    /// 不一致・slot 空は `choice_waiting_stale` の info で棄却する。
    fn on_choice_waiting(&mut self, waiting: ChoiceWaiting) -> ControlFlow<()> {
        if self.current_talk_id() != Some(waiting.talk_id) {
            tracing::info!(
                event = "choice_waiting_stale",
                talk_id = waiting.talk_id.0,
                choice_count = waiting.choice_ids.len(),
                current_talk_id = ?self.current_talk_id().map(|t| t.0),
                "stale ChoiceWaiting discarded (slot already replaced or empty)"
            );
            return ControlFlow::Continue(());
        }

        let talk_id = waiting.talk_id;
        if self
            .kanade
            .send(KanadeMsg::ChoiceWaiting {
                talk_id,
                choice_ids: waiting.choice_ids,
                display_end: waiting.display_end,
                timeout_directive: waiting.timeout_directive,
            })
            .is_err()
        {
            tracing::debug!(
                talk_id = talk_id.0,
                "kanade already stopped; dropping ChoiceWaiting forward (expected during shutdown)"
            );
        }
        ControlFlow::Continue(())
    }

    /// `Start` 受領: 既存 active があれば Close funnel（Close→join）で終了させてから、
    /// 新規 talk を spawn して差し替える。
    fn on_start(&mut self, start: StartTalk) -> ControlFlow<()> {
        self.close_active_if_any();

        let talk_id = start.talk_id;
        let origin = start.origin;

        // sink を talk ごとに `clone_box` して独立インスタンスを組む（登録順＝broadcast 順）。
        // 複製の直後、talk スレッドへ渡す前にこのトークの番号と出どころを渡す（begin_talk）。
        // 古いトークの複製は上の close_active_if_any の join で既に落ちている。
        let sinks: Vec<Box<dyn CueSink + Send>> = self
            .sinks
            .iter()
            .map(|sink| {
                let mut sink = sink.clone_box();
                sink.begin_talk(talk_id, origin);
                sink as Box<dyn CueSink + Send>
            })
            .collect();

        // provider を talk の起動時点で一度だけ呼び、凍結スナップショットを talk へ渡す
        // （boot 時 1 回きりの固定像ではない）。sakura は値源を所有せず、この凍結像だけを参照する。
        let system_vars = (self.system_vars)();

        let strict = strict_hooks_for(&start, self.strict.as_ref());
        let handle = spawn_talk(start, self.self_sender.clone(), sinks, system_vars, strict);
        self.active = Some(ActiveTalk { talk_id, handle });
        ControlFlow::Continue(())
    }

    /// `Done` 受領: 現 slot と `talk_id` が一致する場合のみ kanade へ転送して slot を解放する。
    /// 不一致（既に差し替え済みの旧 talk からの通知）は stale として棄却する。
    fn on_done(&mut self, done: TalkDone) -> ControlFlow<()> {
        let is_current = matches!(&self.active, Some(active) if active.talk_id == done.talk_id);

        if is_current {
            if self.kanade.send(KanadeMsg::TalkDone(done)).is_err() {
                tracing::debug!(
                    talk_id = done.talk_id.0,
                    "kanade already stopped; dropping TalkDone forward (expected during shutdown)"
                );
            }
            self.active = None;
        } else {
            tracing::info!(
                event = "talk_done_stale",
                talk_id = done.talk_id.0,
                "stale TalkDone discarded (slot already replaced or empty)"
            );
        }
        ControlFlow::Continue(())
    }

    /// `Tick{now}` 受領: active があれば `now` を変えずに `SakuraMsg::Tick` として中継する
    /// （台本の起点は talk が最初の刻みで押す）。active が無ければ何もしない。
    fn on_tick(&self, now: TimePoint) -> ControlFlow<()> {
        if let Some(active) = self.active.as_ref()
            && active.handle.inbox.send(SakuraMsg::Tick(now)).is_err()
        {
            tracing::debug!(
                talk_id = active.talk_id.0,
                "active talk inbox disconnected; dropping Tick relay (talk already ended)"
            );
        }
        ControlFlow::Continue(())
    }

    /// `Close` 受領: 稼働中の active があれば終了させてから、dispatcher 自身を停止する。
    fn on_close(&mut self) -> ControlFlow<()> {
        self.close_active_if_any();
        ControlFlow::Break(())
    }

    /// 稼働中 active があれば `SakuraMsg::Close` を送り、その actor を join してから slot を
    /// 解放する（join の panic は `error!` の上で継続・Close funnel の共通実装）。
    fn close_active_if_any(&mut self) {
        if let Some(active) = self.active.take() {
            if active.handle.inbox.send(SakuraMsg::Close).is_err() {
                tracing::debug!(
                    talk_id = active.talk_id.0,
                    "active talk inbox already disconnected before Close (talk already ended)"
                );
            }
            if let Err(err) = active.handle.actor.join() {
                tracing::error!(
                    talk_id = active.talk_id.0,
                    error = %err,
                    "active talk actor panicked while closing; continuing"
                );
            }
        }
    }
}

/// 印が真の台本のときだけ束ねを渡す。判定の差し込みが無い構築では、名と判定が揃わず記録を書けないので
/// strict の台本も記録せずに再生する。
fn strict_hooks_for(start: &StartTalk, hooks: Option<&StrictHooks>) -> Option<StrictHooks> {
    if !start.strict {
        return None;
    }
    if hooks.is_none() {
        tracing::debug!(
            talk_id = start.talk_id.0,
            "strict talk without strict judges; playing without strict records"
        );
    }
    hooks.cloned()
}

/// dispatcher を起動する。`sinks` は構築時注入の sink 列で、各 talk へは `clone_box` した専用インスタンス列を
/// 渡す（登録順＝broadcast 順）。`system_vars` は talk 起動ごとに一度呼んで凍結スナップショットを得る。
/// `strict` は strict の記録の束ねで、印が真の台本のときだけ `spawn_talk` へ渡す。
///
/// self-sender ハンドオフ（モジュール doc 参照）により、inbox は全 `Sender` drop で切断されず、
/// 唯一の停止経路は [`DispatcherMsg::Close`]。
pub fn spawn_dispatcher(
    kanade: Sender<KanadeMsg>,
    sinks: Vec<Box<dyn BootCueSink>>,
    system_vars: SystemVarSource,
    strict: Option<StrictHooks>,
) -> (Sender<DispatcherMsg>, ActorHandle) {
    let (self_tx_reply, self_tx_recv) = reply_channel::<Sender<DispatcherMsg>>();

    let (tx, handle) = spawn_actor::<DispatcherMsg, _>("ghost-dispatcher", move |rx| {
        let self_sender: Sender<DispatcherMsg> = self_tx_recv
            .recv()
            .expect("self sender is always sent before the actor thread can observe anything else");

        let mut state = DispatcherState {
            active: None,
            kanade,
            sinks,
            system_vars,
            strict,
            self_sender,
        };

        run_inbox::<DispatcherMsg, std::convert::Infallible>(rx, move |msg| Ok(state.handle(msg)));
    });

    // self-sender ハンドオフ（モジュール doc 参照）。
    let _ = self_tx_reply.send(tx.clone());

    (tx, handle)
}

#[cfg(test)]
#[path = "dispatcher_begin_talk_tests.rs"]
mod begin_talk_tests;
#[cfg(test)]
#[path = "dispatcher_choice_tests.rs"]
mod choice_tests;
#[cfg(test)]
#[path = "dispatcher_choice_timeout_tests.rs"]
mod choice_timeout_tests;
#[cfg(test)]
#[path = "dispatcher_choice_waiting_tests.rs"]
mod choice_waiting_tests;
#[cfg(test)]
#[path = "dispatcher_selection_tests.rs"]
mod selection_tests;
#[cfg(test)]
#[path = "dispatcher_slot_tests.rs"]
mod slot_tests;
#[cfg(test)]
#[path = "dispatcher_strict_tests.rs"]
mod strict_tests;
