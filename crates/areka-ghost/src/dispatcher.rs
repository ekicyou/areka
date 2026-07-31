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
    /// # 暫定アーム（タスク 1.4 の型結線に対する一時措置・恒久実装ではない）
    /// 本タスクは「kanade → relay → dispatcher」の**経路の成立**までを担う。現行 slot を引く
    /// ところまでを通し、指示は talk へ転送せず記録して捨てる。**恒久実装はタスク 3.3 の担当**
    /// であり、そこで現行 slot と `talk_id` が一致するときのみ `SakuraMsg::ResolveChoice{id}` を
    /// 転送し（slot・`base_now` は不変）、不一致・空は `resolve_choice_stale` で info 棄却する
    /// stale ガードが与えられる。それまでは黙って捨てず warn! で記録する
    /// （steering: areka-log-first-no-silent-failure）。
    fn on_resolve_choice(&mut self, talk_id: TalkId, id: &str) -> ControlFlow<()> {
        tracing::warn!(
            event = "resolve_choice_dropped_not_wired",
            talk_id = talk_id.0,
            choice_id = %id,
            current_talk_id = ?self.current_talk_id().map(|t| t.0),
            "ResolveChoice を受領したが talk へ未転送のため棄却——転送と stale ガードはタスク 3.3"
        );
        ControlFlow::Continue(())
    }

    /// `CancelChoice` 受領（DD-5／DD-11・design C9・Req7.5）。
    ///
    /// # 暫定アーム（タスク 1.4 の型結線に対する一時措置・恒久実装ではない）
    /// 現行 slot を引くところまでを通し、指示は talk へ転送せず記録して捨てる。**恒久実装は
    /// タスク 3.3 の担当**であり、そこで一致時のみ `SakuraMsg::Close` を「転送」し
    /// （[`close_active_if_any`](Self::close_active_if_any) は使わない——slot と join を保持して
    /// talk 発の `TalkDone{Interrupted}` を正規経路で kanade へ届けさせるため）、不一致は
    /// `cancel_choice_stale` で info 棄却する Close funnel 写像が与えられる。
    fn on_cancel_choice(&mut self, talk_id: TalkId) -> ControlFlow<()> {
        tracing::warn!(
            event = "cancel_choice_dropped_not_wired",
            talk_id = talk_id.0,
            current_talk_id = ?self.current_talk_id().map(|t| t.0),
            "CancelChoice を受領したが talk へ未転送のため棄却——Close funnel 転送はタスク 3.3"
        );
        ControlFlow::Continue(())
    }

    /// `ChoiceWaiting` 受領（DD-6／DD-9・design C9・Req5.5/7.2）。
    ///
    /// # 暫定アーム（タスク 1.4 の型結線に対する一時措置・恒久実装ではない）
    /// 現行 slot を引くところまでを通し、通知は kanade へ転送せず記録して捨てる。**恒久実装は
    /// タスク 3.3 の担当**であり、そこで一致時のみ
    /// `display_end_ms = base_now + round(display_end_elapsed_secs * 1000)` へ換算して
    /// `KanadeMsg::ChoiceWaiting` を kanade へ転送し（`base_now` は Tick 中継の既存起点＝
    /// 時間基準を新設しない）、`base_now == None` は warn 防御・不一致は info 棄却する規則が
    /// 与えられる。
    fn on_choice_waiting(&mut self, waiting: ChoiceWaiting) -> ControlFlow<()> {
        tracing::warn!(
            event = "choice_waiting_dropped_not_wired",
            talk_id = waiting.talk_id.0,
            choice_count = waiting.choice_ids.len(),
            display_end_elapsed_secs = waiting.display_end_elapsed_secs,
            timeout_directive_secs = ?waiting.timeout_directive_secs,
            current_talk_id = ?self.current_talk_id().map(|t| t.0),
            "ChoiceWaiting を受領したが kanade へ未転送のため棄却——ms 換算と転送はタスク 3.3"
        );
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
mod tests {
    use super::*;
    use areka_sakura::contract::{CueCommand, TalkCue, TalkEndReason};
    use std::sync::mpsc::{self, sync_channel};
    use std::thread;
    use std::time::Duration;

    /// task 8.3: 退役した `crate::runtime::default_system_vars()` の忠実な代役スタンドイン。
    ///
    /// `{"username": DEFAULT_USERNAME}` のみを充填した凍結スナップショットを毎回新規構築して
    /// 返す（退役前 provider と同一挙動）。`spawn_dispatcher` の刻印点は [`SystemVarSource`] のまま
    /// 無改変で、既存テストは従来どおり既定 username 前提の直接注入を保つ（R7.1・R9.1）。
    fn test_system_vars() -> SystemVarSource {
        Box::new(|| {
            let mut snapshot = areka_sakura::contract::SystemVarSnapshot::default();
            snapshot.insert("username", areka_sakura::sysvar::DEFAULT_USERNAME);
            snapshot
        })
    }

    /// テスト用の有界待機ヘルパ: 別スレッドで `f` を走らせ、期限内に完了しなければ
    /// テストを失敗させる（どのテストもハングしないことを保証する・areka-actor 流儀）。
    fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
        let (done_tx, done_rx) = sync_channel::<()>(0);
        thread::spawn(move || {
            f();
            let _ = done_tx.send(());
        });
        assert!(
            done_rx.recv_timeout(timeout).is_ok(),
            "'{what}' did not complete within {timeout:?} (possible hang)"
        );
    }

    /// テスト専用の `Clone` 可能な記録 sink（sakura `MockSink` は `Clone` でないため、
    /// dispatcher の per-talk 注入（`S: Clone`/`T: Clone`）を満たすために本モジュール限定で
    /// 定義する・sakura の凍結面 `sink.rs` には手を入れない）。
    #[derive(Clone)]
    struct RecordingSink {
        records: std::sync::Arc<std::sync::Mutex<Vec<TalkCue>>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                records: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            }
        }

        fn records(&self) -> std::sync::Arc<std::sync::Mutex<Vec<TalkCue>>> {
            std::sync::Arc::clone(&self.records)
        }
    }

    // broadcast: 単一の `CueSink` として登録され、全 cue を受ける（surface/text スロットの別なく
    // 両スロットが同一の全 cue を受信する）。演者側 relevance が action を選別する（本 sink は記録のみ）。
    impl CueSink for RecordingSink {
        fn emit(&mut self, cue: TalkCue) {
            self.records
                .lock()
                .expect("records mutex poisoned")
                .push(cue);
        }
    }

    /// テスト専用の `Clone` 可能なチャンネル中継 sink（発火の到着を barrier として同期受信する
    /// ため、`recv_timeout` で1件ずつ決定的に観測できる・sakura drive.rs のテスト流儀を踏襲）。
    #[derive(Clone)]
    struct ChannelSink {
        tx: mpsc::Sender<TalkCue>,
    }

    impl CueSink for ChannelSink {
        fn emit(&mut self, cue: TalkCue) {
            let _ = self.tx.send(cue);
        }
    }

    /// task 1.4（DD-5・C9）: `From<TalkCommand> for DispatcherMsg` の**全 variant** 網羅変換。
    ///
    /// kanade の送出口（`Sender<TalkCommand>`）から start-relay を経て dispatcher inbox へ入る
    /// 唯一の変換点であり、ここで variant が落ちると当該指示が物理的に到達不能になる。3 形すべてが
    /// 情報無損失で対応アームへ写ることを固定する（新 variant 追加時は本檻がコンパイルエラーで気づく）。
    #[test]
    fn talk_command_converts_to_dispatcher_msg_for_every_variant() {
        use areka_sakura::contract::TalkCommand;

        // Start: StartTalk を情報無損失で包み直すだけ。
        let start = StartTalk {
            talk_id: TalkId(101),
            script: r"\s[0]hi\e".to_string(),
            epilogue: Vec::new(),
        };
        match DispatcherMsg::from(TalkCommand::Start(start.clone())) {
            DispatcherMsg::Start(got) => {
                assert_eq!(got.talk_id, start.talk_id);
                assert_eq!(got.script, start.script);
                assert!(got.epilogue.is_empty());
            }
            _ => panic!("TalkCommand::Start は DispatcherMsg::Start へ変換されるべき"),
        }

        // ResolveChoice: talk_id（stale ガード用）と選択肢 id をそのまま運ぶ。
        match DispatcherMsg::from(TalkCommand::ResolveChoice {
            talk_id: TalkId(102),
            id: "choice-1".to_string(),
        }) {
            DispatcherMsg::ResolveChoice { talk_id, id } => {
                assert_eq!(talk_id, TalkId(102));
                assert_eq!(id, "choice-1");
            }
            _ => panic!("TalkCommand::ResolveChoice は DispatcherMsg::ResolveChoice へ変換されるべき"),
        }

        // CancelChoice: talk_id をそのまま運ぶ。
        match DispatcherMsg::from(TalkCommand::CancelChoice {
            talk_id: TalkId(103),
        }) {
            DispatcherMsg::CancelChoice { talk_id } => assert_eq!(talk_id, TalkId(103)),
            _ => panic!("TalkCommand::CancelChoice は DispatcherMsg::CancelChoice へ変換されるべき"),
        }
    }

    /// task 1.4（DD-5・C9）: `From<ChoiceWaiting> for DispatcherMsg` の全フィールド無損失変換。
    ///
    /// `ChoiceWaiting` は talk → dispatcher の done ポート（`spawn_talk` の `D: From<..>` 境界）を
    /// 流れるため、この `From` が無ければ通知経路が型として成立しない。搬送値（候補 id 列・
    /// 表示完了時刻・タイムアウト指令）が一切改変されずに届くことを固定する。
    #[test]
    fn choice_waiting_converts_to_dispatcher_msg_without_loss() {
        use areka_sakura::contract::ChoiceWaiting;

        let waiting = ChoiceWaiting {
            talk_id: TalkId(201),
            choice_ids: vec!["a".to_string(), "b".to_string()],
            display_end_elapsed_secs: 1.25,
            timeout_directive_secs: Some(12.0),
        };
        match DispatcherMsg::from(waiting.clone()) {
            DispatcherMsg::ChoiceWaiting(got) => assert_eq!(got, waiting),
            _ => panic!("ChoiceWaiting は DispatcherMsg::ChoiceWaiting へ変換されるべき"),
        }

        // 未指定（None＝下流既定値へ委譲）も同様に無改変で運ばれる。
        let unspecified = ChoiceWaiting {
            talk_id: TalkId(202),
            choice_ids: Vec::new(),
            display_end_elapsed_secs: 0.0,
            timeout_directive_secs: None,
        };
        match DispatcherMsg::from(unspecified.clone()) {
            DispatcherMsg::ChoiceWaiting(got) => assert_eq!(got, unspecified),
            _ => panic!("ChoiceWaiting は DispatcherMsg::ChoiceWaiting へ変換されるべき"),
        }
    }

    /// シナリオ1: 単一 slot 維持・置き換え。`Start(A)` 稼働中に `Start(B)` を送ると、A は
    /// Close-then-join で終了してから B が spawn される。A の完了通知（`Interrupted`）は
    /// 既に B へ差し替わった後に dispatcher inbox へ届く stale 通知となり、kanade へは決して
    /// 転送されない（要件 4.1/4.2・stale 棄却は Close-then-spawn の直接帰結として自然発生する）。
    #[test]
    fn start_then_start_replaces_active_talk_and_discards_stale_done_from_replaced_talk() {
        let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
        let surface = RecordingSink::new();
        let text = RecordingSink::new();

        let (tx, handle) = spawn_dispatcher(
            kanade_tx,
            vec![Box::new(surface), Box::new(text)],
            test_system_vars(),
        );

        let talk_a = TalkId(1);
        let talk_b = TalkId(2);

        // A: 長い待ちを持つ script（差し替えまで一切 Tick を送らないので自然完了しない）。
        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: talk_a,
            script: r"\s[1]A\w[50]A_END\e".to_string(),
        }))
        .expect("send Start(A)");

        // B: 短い script（差し替え後にこれを完走させる）。
        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: talk_b,
            script: r"\s[2]B\w[2]B_END\e".to_string(),
        }))
        .expect("send Start(B)");

        // B を完走させる（D 焼き込み後 B/B_END の再生完了＋\w[2] を含む占有 horizon=0.40 を跨ぐ
        // elapsed 0.5・base_now は最初の Tick の now で確定）。
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send Tick(base)");
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(1_500),
        })
        .expect("send Tick(base+500ms)");

        // kanade は B の TalkDone のみを受け取る（A の stale Interrupted は転送されない）。
        let done = kanade_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("kanade should receive TalkDone for B");
        match done {
            KanadeMsg::TalkDone(done) => {
                assert_eq!(
                    done.talk_id, talk_b,
                    "forwarded TalkDone must be for B, not A"
                );
                assert_eq!(done.reason, TalkEndReason::Ended);
            }
            _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
        }

        // A についての通知は決して kanade へ届かない（stale 棄却の直接観測）。
        assert!(
            kanade_rx.try_recv().is_err(),
            "no further KanadeMsg (in particular no stale TalkDone for A) should reach kanade"
        );

        tx.send(DispatcherMsg::Close).expect("send Close");
        run_bounded(
            "dispatcher join after Close",
            Duration::from_secs(5),
            move || {
                handle
                    .join()
                    .expect("dispatcher terminates normally after Close");
            },
        );
    }

    /// シナリオ2: 明示的な stale `Done` の棄却。A→B へ差し替え済みの状態で、A の
    /// `talk_id` を持つ `Done` を手動投函しても kanade へは転送されず、B の slot は
    /// 乱されない（要件 4.4 の直接固定）。
    #[test]
    fn explicit_stale_done_after_replacement_is_discarded_without_disturbing_current_slot() {
        let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
        let surface = RecordingSink::new();
        let text = RecordingSink::new();

        let (tx, handle) = spawn_dispatcher(
            kanade_tx,
            vec![Box::new(surface), Box::new(text)],
            test_system_vars(),
        );

        let talk_a = TalkId(11);
        let talk_b = TalkId(12);

        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: talk_a,
            script: r"\s[1]A\w[50]A_END\e".to_string(),
        }))
        .expect("send Start(A)");
        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: talk_b,
            script: r"\s[2]B\w[2]B_END\e".to_string(),
        }))
        .expect("send Start(B)");

        // 手動で A の stale Done（自然発生分に加えた明示的な追験）を投函する。
        tx.send(DispatcherMsg::Done(TalkDone {
            talk_id: talk_a,
            reason: TalkEndReason::Interrupted,
        }))
        .expect("send manual stale Done(A)");

        // B の slot は乱されず、B を完走させれば正しく kanade へ転送される
        // （D 焼き込み後の占有 horizon=0.40 を跨ぐ elapsed 0.5）。
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(2_000),
        })
        .expect("send Tick(base)");
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(2_500),
        })
        .expect("send Tick(base+500ms)");

        let done = kanade_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("kanade should receive TalkDone for B despite the stale A notification");
        match done {
            KanadeMsg::TalkDone(done) => {
                assert_eq!(
                    done.talk_id, talk_b,
                    "slot must still be B after stale Done(A)"
                );
                assert_eq!(done.reason, TalkEndReason::Ended);
            }
            _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
        }
        assert!(
            kanade_rx.try_recv().is_err(),
            "the stale Done(A) must never surface to kanade as a KanadeMsg::TalkDone"
        );

        tx.send(DispatcherMsg::Close).expect("send Close");
        run_bounded(
            "dispatcher join after Close",
            Duration::from_secs(5),
            move || {
                handle
                    .join()
                    .expect("dispatcher terminates normally after Close");
            },
        );
    }

    /// シナリオ3: 停止時のクリーンアップ。稼働中の talk がある状態で `Close` を送ると、
    /// dispatcher は active talk へ `SakuraMsg::Close` を送って join してから、自身も
    /// 停止する。`close_active_if_any` は `Break` より前に talk actor の join を完了させる
    /// ため、dispatcher 自身の join が有界時間内に成功すること自体が、内側の talk actor が
    /// 先に正常終了していたことの直接証跡になる（要件 4.5）。
    #[test]
    fn close_while_active_closes_and_joins_active_talk_before_stopping_dispatcher() {
        let (kanade_tx, _kanade_rx) = mpsc::channel::<KanadeMsg>();
        let surface = RecordingSink::new();
        let text = RecordingSink::new();

        let (tx, handle) = spawn_dispatcher(
            kanade_tx,
            vec![Box::new(surface), Box::new(text)],
            test_system_vars(),
        );

        // 長い待ちを持つ script（Close 時点では自然完了していない）。
        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: TalkId(21),
            script: r"\s[1]X\w[50]X_END\e".to_string(),
        }))
        .expect("send Start");

        tx.send(DispatcherMsg::Close).expect("send Close");

        run_bounded(
            "dispatcher join after Close with active talk",
            Duration::from_secs(5),
            move || {
                handle.join().expect(
                    "dispatcher (and therefore its active talk, joined synchronously beforehand) \
                     terminates normally after Close",
                );
            },
        );
    }

    /// シナリオ4: 経過時間換算を伴う Tick 中継。複数の `Tick{now}` を送ると、dispatcher は
    /// 最初の tick を経過秒 0.0 の起点として記録し、以降 `(now - base) / 1000.0` 秒を
    /// `SakuraMsg::Tick(f64)` として active talk へ中継する（要件 5.2）。barrier 技法
    /// （sakura drive.rs 流儀）で、各 Tick が対応する発火群のみを解放することを決定的に確認する。
    #[test]
    fn tick_relay_converts_absolute_now_to_elapsed_seconds_from_first_tick() {
        let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
        let (text_tx, text_rx) = mpsc::channel::<TalkCue>();
        let surface = RecordingSink::new();
        let text = ChannelSink { tx: text_tx };

        let (tx, handle) = spawn_dispatcher(
            kanade_tx,
            vec![Box::new(surface), Box::new(text)],
            test_system_vars(),
        );

        // \w[4]=200ms・\w[6]=300ms。D 焼き込み後の発火（broadcast ゆえ text sink も全 cue を受ける）:
        //   ClearAll@0.0・Emote{5}@0.0・FIRST@0.0 / Wait@0.25 / SECOND@0.45（FIRST の D=0.25 + \w[4]=0.20）/
        //   Wait@0.75 / THIRD@1.05（SECOND の D=0.30 + \w[6]=0.30）。占有 horizon=1.30（THIRD 再生完了）。
        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: TalkId(31),
            script: r"\s[5]FIRST\w[4]SECOND\w[6]THIRD\e".to_string(),
        }))
        .expect("send Start");

        // broadcast: text sink には Emote/Wait 等の担当外 cue も届く。本テストは Text 発火の
        // 順序・保留のみを観測するため、次の目的 Text 発火まで担当外 cue を読み飛ばす barrier ヘルパを使う。
        let recv_text = |want: &str| {
            loop {
                let cue = text_rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("due な Text 発火は届くこと");
                if cue.command == CueCommand::Text(want.into()) {
                    return cue;
                }
            }
        };

        // 初回 tick: elapsed=0.0 起点 → ClearAll@0.0・Emote@0.0・FIRST@0.0 のみ due（SECOND/THIRD は未 due）。
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(5_000),
        })
        .expect("send first Tick (anchors base_now)");
        let first = recv_text("FIRST");
        assert_eq!(first.command, CueCommand::Text("FIRST".into()));
        // FIRST まで drain した時点で Wait@0.25/SECOND@0.45/THIRD は未 due（保留の決定的証明）。
        assert!(
            text_rx.try_recv().is_err(),
            "SECOND/THIRD must not fire before their elapsed time is reached"
        );

        // 2 回目 tick: now - base = 500ms → elapsed=0.5 → Wait@0.25・SECOND@0.45 due（THIRD@1.05 はまだ）。
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(5_500),
        })
        .expect("send second Tick (elapsed 0.5)");
        let second = recv_text("SECOND");
        assert_eq!(second.command, CueCommand::Text("SECOND".into()));
        assert!(
            text_rx.try_recv().is_err(),
            "THIRD must not fire before elapsed 1.05 is reached"
        );

        // 3 回目 tick: now - base = 1100ms → elapsed=1.1 → Wait@0.75・THIRD@1.05 due。
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(6_100),
        })
        .expect("send third Tick (elapsed 1.1)");
        let third = recv_text("THIRD");
        assert_eq!(third.command, CueCommand::Text("THIRD".into()));

        // 4 回目 tick: elapsed=1.4 → 占有 horizon=1.30 到達で自然終端（末尾テキストの D を落とさない）。
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(6_400),
        })
        .expect("send fourth Tick (elapsed 1.4 ≥ horizon 1.30)");

        let done = kanade_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("kanade should receive TalkDone after natural completion");
        match done {
            KanadeMsg::TalkDone(done) => {
                assert_eq!(done.talk_id, TalkId(31));
                assert_eq!(done.reason, TalkEndReason::Ended);
            }
            _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
        }

        tx.send(DispatcherMsg::Close).expect("send Close");
        run_bounded(
            "dispatcher join after Close",
            Duration::from_secs(5),
            move || {
                handle
                    .join()
                    .expect("dispatcher terminates normally after Close");
            },
        );
    }

    /// シナリオ5: 完了通知の転送（happy path）。talk が自然完了すると `KanadeMsg::TalkDone`
    /// が正しい `talk_id`/`reason` で kanade へ届き、slot は解放される。解放されたことは、
    /// 後続の `Start` が（明示 Close を要さず）新しい talk を正しく再生し、2 件目の
    /// `TalkDone` も過不足なく届くことで確認する（要件 4.3）。
    #[test]
    fn natural_completion_forwards_talkdone_and_clears_slot_for_next_start() {
        let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
        let surface = RecordingSink::new();
        let text = RecordingSink::new();
        let surface_records = surface.records();

        let (tx, handle) = spawn_dispatcher(
            kanade_tx,
            vec![Box::new(surface), Box::new(text)],
            test_system_vars(),
        );

        let talk_c = TalkId(41);
        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: talk_c,
            script: r"\s[9]hello\w[2]world\e".to_string(),
        }))
        .expect("send Start(C)");
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(9_000),
        })
        .expect("send Tick(base)");
        // D 焼き込み後 C の占有 horizon=0.60（world 再生完了）を跨ぐ elapsed 0.7。
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(9_700),
        })
        .expect("send Tick(base+700ms)");

        let done_c = kanade_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("kanade should receive TalkDone for C");
        match done_c {
            KanadeMsg::TalkDone(done) => {
                assert_eq!(done.talk_id, talk_c);
                assert_eq!(done.reason, TalkEndReason::Ended);
            }
            _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
        }

        // slot は解放済み: 後続 Start は Close を要さず新規 talk をそのまま再生できる。
        let talk_d = TalkId(42);
        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: talk_d,
            script: r"\s[8]again\e".to_string(),
        }))
        .expect("send Start(D)");
        // D（`again`＝5 char・D=0.25）の占有 horizon=0.25 を跨ぐため base(10_000)＋elapsed 0.3 の 2 tick。
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(10_000),
        })
        .expect("send Tick(base) for D");
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(10_300),
        })
        .expect("send Tick(base+300ms) for D");

        let done_d = kanade_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("kanade should receive TalkDone for D");
        match done_d {
            KanadeMsg::TalkDone(done) => {
                assert_eq!(done.talk_id, talk_d, "second slot occupant must be D");
                assert_eq!(done.reason, TalkEndReason::Ended);
            }
            _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
        }
        assert!(
            kanade_rx.try_recv().is_err(),
            "exactly two TalkDone (C then D) — no stray duplicates or stale entries"
        );

        // broadcast: surface sink には両 talk の全 cue（ClearAll/Text/Wait 含む）が届くため、
        // Emote 発火だけを抽出して「C=scope9・D=scope8 が 1 件ずつ」を確認する（partition は演者側 relevance の責務）。
        let surface = surface_records.lock().expect("records mutex poisoned");
        let emotes: Vec<&CueCommand> = surface
            .iter()
            .map(|c| &c.command)
            .filter(|c| matches!(c, CueCommand::Emote { .. }))
            .collect();
        assert_eq!(
            emotes,
            vec![
                &CueCommand::Emote { key: "9".into() },
                &CueCommand::Emote { key: "8".into() },
            ],
            "broadcast 経由でも Emote 発火は C(scope9)→D(scope8) の 1 件ずつ"
        );

        tx.send(DispatcherMsg::Close).expect("send Close");
        run_bounded(
            "dispatcher join after Close",
            Duration::from_secs(5),
            move || {
                handle
                    .join()
                    .expect("dispatcher terminates normally after Close");
            },
        );
    }

    /// シナリオ6（task 6.2・凍結像の刻印点）: `system_vars` provider が talk 起動ごとに
    /// 一度呼び出され、その時点で凍結されたスナップショットが sakura 側のコンパイルへ流れる
    /// ことを end-to-end に固定する（R7.3/7.4）。
    ///
    /// 呼び出しのたびに `username` を `user1`→`user2`… と変える counter provider を注入し、
    /// `%username` を含む talk を 2 回起動する。各 talk の `%username` は**その talk の起動
    /// 時点で凍結された**値（1 本目=`user1`／2 本目=`user2`）の Text cue へ展開され、broadcast
    /// で観測できる。値が talk 間で異なること自体が「talk ごとに独立して凍結される」意味論
    /// （sylphya の per-talk 凍結と同形）の直接証跡になる。provider の呼出回数が talk 起動数と
    /// 一致することも固定する（＝ per-talk 刻印であって boot 時 1 回きりの固定像ではない）。
    ///
    /// task 6.1 の暫定既定橋渡し（`SystemVarSnapshot::default()`）のままでは provider は
    /// 一度も呼ばれず、`%username` は既定値 `DEFAULT_USERNAME` へ展開されるため、本檻は
    /// `user1`/`user2` を観測できず（かつ呼出回数 0）RED になる。
    #[test]
    fn system_vars_provider_is_invoked_and_frozen_per_talk_start() {
        use crate::runtime::SystemVarSource;
        use areka_sakura::contract::SystemVarSnapshot;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (kanade_tx, _kanade_rx) = mpsc::channel::<KanadeMsg>();
        let (text_tx, text_rx) = mpsc::channel::<TalkCue>();
        let surface = RecordingSink::new();
        let text = ChannelSink { tx: text_tx };

        // 呼び出しごとに username を `user{n}` と変える provider（凍結＝各 talk が自分の
        // 起動時点の値を見ることの証明用）。呼出回数も観測する。
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_provider = Arc::clone(&calls);
        let provider: SystemVarSource = Box::new(move || {
            let n = calls_for_provider.fetch_add(1, Ordering::SeqCst) + 1;
            let mut snapshot = SystemVarSnapshot::default();
            snapshot.insert("username", format!("user{n}"));
            snapshot
        });

        let (tx, handle) =
            spawn_dispatcher(kanade_tx, vec![Box::new(surface), Box::new(text)], provider);

        // broadcast: text sink には ClearAll/Emote 等の担当外 cue も届く。次の Text 発火まで読み飛ばす。
        let recv_text = |want: &str| -> TalkCue {
            loop {
                let cue = text_rx
                    .recv_timeout(Duration::from_secs(5))
                    .expect("due な Text 発火は届くこと");
                if cue.command == CueCommand::Text(want.into()) {
                    return cue;
                }
            }
        };

        // talk 1: `%username`（→ 起動時点で凍結された provider 値 `user1` へ展開）。
        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: TalkId(61),
            script: r"\s[0]%username\e".to_string(),
        }))
        .expect("send Start(1)");
        // 初回 Tick で base_now 刻印＋elapsed=0.0 群（ClearAll/Emote/Text@0.0）を発火。
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(1_000),
        })
        .expect("send Tick for talk 1");
        let first = recv_text("user1");
        assert_eq!(
            first.command,
            CueCommand::Text("user1".into()),
            "talk 1 の %username は起動時点で凍結された provider 値 user1 へ展開される（既定値でない）"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "provider は talk 1 の起動で 1 回だけ呼ばれる（刻印点）"
        );

        // talk 2: 差し替え起動（talk 1 は Close funnel で終了）。provider の次の呼出＝`user2`。
        tx.send(DispatcherMsg::Start(StartTalk {
            epilogue: Vec::new(),
            talk_id: TalkId(62),
            script: r"\s[0]%username\e".to_string(),
        }))
        .expect("send Start(2)");
        tx.send(DispatcherMsg::Tick {
            now: MonotonicMs(2_000),
        })
        .expect("send Tick for talk 2");
        let second = recv_text("user2");
        assert_eq!(
            second.command,
            CueCommand::Text("user2".into()),
            "talk 2 は自分の起動時点で凍結された provider 値 user2 を見る（talk ごと独立凍結）"
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "provider の呼出回数が talk 起動数と一致（per-talk 刻印・boot 時 1 回固定でない）"
        );

        tx.send(DispatcherMsg::Close).expect("send Close");
        run_bounded(
            "dispatcher join after Close",
            Duration::from_secs(5),
            move || {
                handle
                    .join()
                    .expect("dispatcher terminates normally after Close");
            },
        );
    }
}
