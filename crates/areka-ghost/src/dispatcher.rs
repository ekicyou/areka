//! sakura dispatcher（永続的な要求元⇄一時的な再生アクターの非対称吸収）。
//!
//! 再生開始要求を受けて per-talk の再生アクターを起動し、同時に1本だけ再生する
//! 単一 slot を維持する。完了通知の運行系への中継・stale 通知の棄却・Close funnel・
//! Tick 中継を担う（design.md「ghost::dispatcher」）。
//!
//! # self-sender ハンドオフ（design.md 参照）
//!
//! per-talk transient（[`spawn_talk`]）の完了通知ポートには、dispatcher 自身の inbox
//! （`Sender<DispatcherMsg>`）のクローンを渡す必要がある。しかし [`spawn_actor`] は
//! body へ `Receiver` のみを渡し、呼び出し側へ返す `Sender` のクローンを body へは
//! 渡さない。この鶏卵を [`reply_channel`] による一度限りの内部ハンドオフで解決する:
//! `spawn_actor` 呼出直後（かつ `spawn_dispatcher` が `tx` を外部へ返す前）に、
//! body が受信ループへ入る前段でブロックしている `self_tx_recv.recv()` へ
//! `self_tx_reply.send(tx.clone())` を送る。この self-sender 保持により dispatcher の
//! inbox は全 `Sender` drop による切断へ到達し得ない（唯一の停止経路は `Close`）。

use std::ops::ControlFlow;
use std::sync::mpsc::Sender;

use areka_actor::{ActorHandle, reply_channel, run_inbox, spawn_actor};
use areka_kanade::{KanadeMsg, MonotonicMs};
use areka_sakura::contract::{
    ChoiceWaiting, CueSink, SakuraMsg, StartTalk, TalkCommand, TalkDone, TalkHandle, TalkId,
};
use areka_sakura::drive::spawn_talk;

use crate::runtime::SystemVarSource;
use crate::sink::BootCueSink;

/// dispatcher の inbox（1 アクター 1 enum・areka-actor inbox 規約）。
pub enum DispatcherMsg {
    /// kanade からの talk 起動（start-relay が `From<TalkCommand>` 変換で投函）。
    Start(StartTalk),
    /// per-talk transient からの完了通知（`spawn_talk` の done ポートが `From` 変換で投函）。
    Done(TalkDone),
    /// ticker からの時刻前進（dispatcher が active talk の経過秒へ換算して中継）。
    Tick { now: MonotonicMs },
    /// 停止規約の Close（active talk を終了させてから停止）。
    Close,
    /// kanade からの選択待ちバリア解決指示（start-relay が `From<TalkCommand>` 変換で投函・DD-5）。
    ///
    /// `talk_id` は stale ガード用、`id` は解決に用いる選択肢 ID。
    ResolveChoice { talk_id: TalkId, id: String },
    /// kanade からの選択待ち解除＋トーク終了指示（DD-5／DD-11・Close funnel へ写像される）。
    CancelChoice { talk_id: TalkId },
    /// per-talk transient からの選択待ち成立通知（`Done` と同一ポートを流れる・DD-6）。
    ChoiceWaiting(ChoiceWaiting),
}

impl From<StartTalk> for DispatcherMsg {
    fn from(start: StartTalk) -> Self {
        DispatcherMsg::Start(start)
    }
}

/// kanade の talk 指示チャンネル（`Sender<TalkCommand>`）から start-relay 経由で投函される
/// 唯一の変換点（DD-5・design C9）。3 形すべてを情報無損失で対応アームへ写す——**単一チャンネル
/// ＋単一 inbox** を通ることが `TalkCommand` の順序保存契約（DD-4 の前提）であり、ここで系統別に
/// チャンネルを分けてはならない。
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

/// per-talk transient の done ポート境界（`D: From<TalkDone> + From<ChoiceWaiting>`）を満たす
/// 変換（DD-6・design C8/C9）。`TalkDone` と同一ポートを流れるため、同一 talk についての
/// 選択待ち成立と再生完了は因果順が保存される。
impl From<ChoiceWaiting> for DispatcherMsg {
    fn from(waiting: ChoiceWaiting) -> Self {
        DispatcherMsg::ChoiceWaiting(waiting)
    }
}

/// `crate::ticker::Tick` からの変換（ticker 側は `D: From<Tick>` を要求する forward-compat
/// 汎用境界・design.md「`DispatcherMsg` への forward dependency」参照）。`Tick{now}` と
/// フィールド形状が一致するため単純な移送のみ。
impl From<crate::ticker::Tick> for DispatcherMsg {
    fn from(tick: crate::ticker::Tick) -> Self {
        DispatcherMsg::Tick { now: tick.now }
    }
}

/// 稼働中 talk の単一 slot 内容。
struct ActiveTalk {
    /// talk 相関 ID（stale 通知の棄却判定・要件 4.4）。
    talk_id: TalkId,
    /// per-talk transient への投函端＋join ハンドル。
    handle: TalkHandle,
    /// この talk に対する最初の `Tick` を観測した時刻（経過秒換算の起点・要件 5.2）。
    base_now: Option<MonotonicMs>,
}

/// dispatcher body の状態（`Option<ActiveTalk>` が単一 slot・要件 4.2）。
struct DispatcherState {
    active: Option<ActiveTalk>,
    kanade: Sender<KanadeMsg>,
    /// 構築時注入の可変長 sink 列（S-3・登録順＝broadcast 順）。talk 起動ごとに各要素を
    /// `clone_box` して per-talk の `spawn_talk` へ手渡す（要件 4.6/8.5）。
    sinks: Vec<Box<dyn BootCueSink>>,
    /// システム変数の供給シーム（S-3・R7.3/7.4）。talk 起動ごとに一度呼び出して凍結
    /// スナップショットを得、per-talk の `spawn_talk` へ手渡す（凍結像の刻印点）。
    system_vars: SystemVarSource,
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
        }
    }

    /// 現行 slot が占有している talk の `talk_id`（空なら `None`）。
    ///
    /// 選択系 3 アームの stale 判定（一致＝現行／不一致・空＝stale）は**この単一の引き口**を
    /// 基準に行う——`Done` の既存 stale 判定（[`on_done`](Self::on_done)）と同じ「現行 slot と
    /// 突き合わせる」規律であり、選択系のために新しい調停を発明しない（Req4.4）。
    fn current_talk_id(&self) -> Option<TalkId> {
        self.active.as_ref().map(|active| active.talk_id)
    }

    /// `ResolveChoice` 受領（DD-5・design C9・Req1.3/5.5）。
    ///
    /// 現行 slot と `talk_id` が一致するときのみ、選択肢 ID を無改変で
    /// `SakuraMsg::ResolveChoice{id}` として talk の型付き入力へ転送する。解決はバリアを解くだけで
    /// talk の同一性も時刻起点も変えないため、**slot と `base_now` は不変**である（バリア状態を
    /// dispatcher 側に複製しない・Req5.6）。
    ///
    /// 不一致・slot 空は stale（既に差し替わった／終了した選択待ち宛の遅延指示）として
    /// `resolve_choice_stale` で info 棄却する（[`on_done`](Self::on_done) の stale 棄却と同型の
    /// 「現行 slot と突き合わせる」規律・Req1.3/5.5）。転送の送出失敗（talk が直前に消滅）は
    /// 黙って捨てず debug で記録して運行を続ける。
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

    /// `CancelChoice` 受領（DD-5／DD-11・design C9・Req7.5）。
    ///
    /// 一致時は `SakuraMsg::Close` を talk へ**転送するだけ**で、slot も join ハンドルも保持する。
    /// [`close_active_if_any`](Self::close_active_if_any) は**使わない**——即 join＋slot 先行解放を
    /// すると、talk が返す `TalkDone{Interrupted}` が [`on_done`](Self::on_done) の一致判定で stale
    /// 化し、kanade が選択待ちから復帰できなくなる。slot を維持して中断 ACK を正規経路で kanade へ
    /// 転送させるのが Close funnel 写像の要点である（`skip_barrier` の外部到達口は新設しない）。
    ///
    /// 不一致・slot 空は `cancel_choice_stale` で info 棄却する（現行 talk を巻き添えに終了させない）。
    /// 転送の送出失敗（talk が直前に消滅）は debug で記録して継続する——その talk は既に自力で
    /// 終端しており、`TalkDone` が別途 slot を解放する。
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

    /// `ChoiceWaiting` 受領（DD-6／DD-9・design C9・Req5.5/7.2）。
    ///
    /// 一致時のみ、talk が通知した**占有 horizon（絶対 elapsed 秒＝duration 権威）**を
    /// `display_end_ms = base_now + round(display_end_elapsed_secs * 1000)` で単調 ms へ換算し、
    /// `KanadeMsg::ChoiceWaiting` として kanade へ転送する。`base_now` は
    /// [`on_tick`](Self::on_tick) が刻印した**Tick 中継の既存起点そのもの**であり、本アームは
    /// その換算（`(now - base) / 1000.0`）の逆写像を行うだけで**新しい時間基準を作らない**
    /// （Req7.2・DD-9）。タイムアウト指令は写像せず素通しする（deadline 写像は kanade・DD-8）。
    ///
    /// `base_now` 未確定（初回 Tick 前）は構造上あり得ない——通知は tick 駆動のバリア到達で出る——が、
    /// 起点を勝手にでっち上げて誤った deadline を配らないよう、`choice_waiting_stale` の warn 防御で
    /// 記録して転送しない。不一致・slot 空は同語彙の info で棄却する（Req1.3）。
    fn on_choice_waiting(&mut self, waiting: ChoiceWaiting) -> ControlFlow<()> {
        let base_now = match self.active.as_ref() {
            Some(active) if active.talk_id == waiting.talk_id => active.base_now,
            _ => {
                tracing::info!(
                    event = "choice_waiting_stale",
                    talk_id = waiting.talk_id.0,
                    choice_count = waiting.choice_ids.len(),
                    current_talk_id = ?self.current_talk_id().map(|t| t.0),
                    "stale ChoiceWaiting discarded (slot already replaced or empty)"
                );
                return ControlFlow::Continue(());
            }
        };

        let Some(base) = base_now else {
            tracing::warn!(
                event = "choice_waiting_stale",
                talk_id = waiting.talk_id.0,
                display_end_elapsed_secs = waiting.display_end_elapsed_secs,
                "ChoiceWaiting arrived before the first Tick anchored base_now; dropping forward \
                 (structurally unreachable — refusing to invent a time origin)"
            );
            return ControlFlow::Continue(());
        };

        // Tick 中継（`(now - base) / 1000.0` 秒）の逆写像。四捨五入は ms 分解能への丸め規約。
        let display_end = MonotonicMs(
            base.0
                .saturating_add((waiting.display_end_elapsed_secs * 1000.0).round() as u64),
        );

        let talk_id = waiting.talk_id;
        if self
            .kanade
            .send(KanadeMsg::ChoiceWaiting {
                talk_id,
                choice_ids: waiting.choice_ids,
                display_end,
                timeout_directive_secs: waiting.timeout_directive_secs,
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
    /// 新規 talk を spawn して差し替える（要件 4.1/4.2・Close-then-spawn は既存有無に
    /// 関わらず同じ手順を踏む）。
    fn on_start(&mut self, start: StartTalk) -> ControlFlow<()> {
        self.close_active_if_any();

        let talk_id = start.talk_id;

        // 凍結像の刻印点（design.md「GhostBootOptions S-3」）: 保持する各 sink を per-talk に
        // clone_box して独立インスタンスの `Vec<Box<dyn CueSink + Send>>` を組む（登録順＝broadcast 順）。
        // `Box<dyn BootCueSink>` は上位境界 `CueSink + Send` を持つため upcast できる。
        let sinks: Vec<Box<dyn CueSink + Send>> = self
            .sinks
            .iter()
            .map(|sink| sink.clone_box() as Box<dyn CueSink + Send>)
            .collect();

        // 凍結像の刻印点（design.md「GhostBootOptions S-3＋provider」・R7.3/7.4）: provider を
        // **この talk の起動時点で一度だけ**呼び出し、返った凍結スナップショットを per-talk へ
        // 手渡す。talk ごとに独立して凍結される（sylphya の per-talk 凍結と同形）＝boot 時 1 回
        // きりの固定像ではない。sakura は値源を所有せず、この凍結像だけを参照する（差替シーム:
        // provider を sylphya 読み口へ差し替えても本層は無改変）。
        let system_vars = (self.system_vars)();

        let handle = spawn_talk(start, self.self_sender.clone(), sinks, system_vars);
        self.active = Some(ActiveTalk {
            talk_id,
            handle,
            base_now: None,
        });
        ControlFlow::Continue(())
    }

    /// `Done` 受領: 現 slot と `talk_id` が一致する場合のみ kanade へ転送して slot を解放する。
    /// 不一致（既に差し替え済みの旧 talk からの通知）は stale として棄却する（要件 4.3/4.4）。
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
                talk_id = done.talk_id.0,
                "stale TalkDone discarded (slot already replaced or empty)"
            );
        }
        ControlFlow::Continue(())
    }

    /// `Tick{now}` 受領: active があれば、この talk に対する初回 tick で `base_now` を確定
    /// （elapsed=0.0 起点）し、以降 `(now - base) / 1000.0` 秒を `SakuraMsg::Tick(f64)` として
    /// 中継する（要件 5.2）。active が無ければ no-op。
    fn on_tick(&mut self, now: MonotonicMs) -> ControlFlow<()> {
        if let Some(active) = self.active.as_mut() {
            let base = *active.base_now.get_or_insert(now);
            let elapsed_seconds = now.0.saturating_sub(base.0) as f64 / 1000.0;
            if active
                .handle
                .inbox
                .send(SakuraMsg::Tick(elapsed_seconds))
                .is_err()
            {
                tracing::debug!(
                    talk_id = active.talk_id.0,
                    "active talk inbox disconnected; dropping Tick relay (talk already ended)"
                );
            }
        }
        ControlFlow::Continue(())
    }

    /// `Close` 受領: 稼働中の active があれば終了させてから、dispatcher 自身を停止する
    /// （要件 4.5）。
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

/// dispatcher を起動する。`sinks` は構築時注入の可変長 sink 列（S-3・要件 4.6/8.5・setter なし）で、
/// 各 per-talk transient へは各 sink を `clone_box` した専用インスタンス列を渡す（登録順＝broadcast 順）。
/// `system_vars` は per-talk のシステム変数供給シーム（R7.3/7.4）で、talk 起動ごとに一度呼び出して
/// 得た凍結スナップショットを `spawn_talk` へ手渡す（凍結像の刻印点）。
///
/// self-sender ハンドオフ（モジュール doc 参照）により、dispatcher の inbox は全 `Sender`
/// drop による切断へ到達し得ない——唯一の停止経路は [`DispatcherMsg::Close`] である。
pub fn spawn_dispatcher(
    kanade: Sender<KanadeMsg>,
    sinks: Vec<Box<dyn BootCueSink>>,
    system_vars: SystemVarSource,
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
            self_sender,
        };

        run_inbox::<DispatcherMsg, std::convert::Infallible>(rx, move |msg| Ok(state.handle(msg)));
    });

    // self-sender ハンドオフ: spawn_actor 直後・tx を呼び出し元へ返す前に、body が受信ループへ
    // 入る前段でブロックしている self_tx_recv.recv() へクローンを渡す（モジュール doc 参照）。
    let _ = self_tx_reply.send(tx.clone());

    (tx, handle)
}

#[cfg(test)]
#[path = "dispatcher_choice_tests.rs"]
mod choice_tests;
#[cfg(test)]
#[path = "dispatcher_slot_tests.rs"]
mod slot_tests;
#[cfg(test)]
#[path = "dispatcher_test_support.rs"]
mod test_support;
