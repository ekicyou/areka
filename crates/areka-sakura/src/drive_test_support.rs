use super::*;
use crate::contract::{CueCommand, TalkCue};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ── テスト用 done ポート型（`D: From<TalkDone> + From<ChoiceWaiting>` 境界の充足） ──

/// 完了通知ポートを流れる 2 種の通知を合流させる最小の enum（本番は ⓪ghost の
/// `DispatcherMsg` が同じ役を担う）。[`TalkDone`] と [`ChoiceWaiting`] が**同一ポート**を
/// 流れることで因果順が保存される（DD-6）ため、檻もこの単一チャンネルで観測する。
#[derive(Debug, Clone, PartialEq)]
pub(super) enum TalkNotice {
    /// 再生完了/中断 ACK（通算高々 1 回）。
    Done(TalkDone),
    /// 選択待ち成立（バリアごとに 1 回）。
    ChoiceWaiting(ChoiceWaiting),
}

impl From<TalkDone> for TalkNotice {
    fn from(done: TalkDone) -> Self {
        TalkNotice::Done(done)
    }
}

impl From<ChoiceWaiting> for TalkNotice {
    fn from(waiting: ChoiceWaiting) -> Self {
        TalkNotice::ChoiceWaiting(waiting)
    }
}

/// `TalkDone` **だけ**を待つ受信ヘルパ（間に挟まる `ChoiceWaiting` は読み飛ばす）。
///
/// 既存檻が `done_rx.recv_timeout(..)` で観測していた意味論（「この窓の中で `TalkDone` が
/// 来るか否か」）をそのまま保つための薄いフィルタである。与えた総待ち時間を deadline として
/// 守るため、`ChoiceWaiting` の読み飛ばしで窓が延びることはない（負の窓の判定が緩まない）。
pub(super) fn recv_done(
    rx: &mpsc::Receiver<TalkNotice>,
    timeout: Duration,
) -> Result<TalkDone, RecvTimeoutError> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining)? {
            TalkNotice::Done(done) => return Ok(done),
            TalkNotice::ChoiceWaiting(_) => continue,
        }
    }
}

// ── テスト用 CueSink 群（broadcast: 登録された全 sink が全 cue を受ける） ──

/// broadcast で届いた全 cue を共有蓄積へ FIFO 追記する記録 sink（`Clone` で観測ハンドル取得）。
#[derive(Clone)]
pub(super) struct RecordingSink {
    records: Arc<Mutex<Vec<TalkCue>>>,
}

impl RecordingSink {
    pub(super) fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub(super) fn records(&self) -> Arc<Mutex<Vec<TalkCue>>> {
        Arc::clone(&self.records)
    }
}

impl CueSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.records
            .lock()
            .expect("RecordingSink records mutex poisoned")
            .push(cue);
    }
}

/// broadcast の 2 つ目のスロットを埋める no-op sink（多くのテストは片方の記録 sink のみ観測する）。
pub(super) struct NoopSink;

impl CueSink for NoopSink {
    fn emit(&mut self, _cue: TalkCue) {}
}

/// テスト用: 2 演者 sink を register 順（S-3・登録順＝broadcast 順）で `spawn_talk` の
/// `Vec<Box<dyn CueSink + Send>>` へ束ねるヘルパ。broadcast ゆえ両 sink は同一 cue 列を受け、
/// 順序は broadcast 順にのみ効く（観測 sink をどちらへ置いても記録内容は不変）。
pub(super) fn two_sinks(
    first: impl CueSink + Send + 'static,
    second: impl CueSink + Send + 'static,
) -> Vec<Box<dyn CueSink + Send>> {
    vec![Box::new(first), Box::new(second)]
}

/// 発火の到着を barrier として同期受信するチャンネル sink（保留の決定的証明に使う）。
pub(super) struct ChannelSink {
    pub(super) tx: mpsc::Sender<TalkCue>,
}

impl CueSink for ChannelSink {
    fn emit(&mut self, cue: TalkCue) {
        let _ = self.tx.send(cue);
    }
}

/// command 抽出ヘルパ。
pub(super) fn commands(records: &Arc<Mutex<Vec<TalkCue>>>) -> Vec<CueCommand> {
    records
        .lock()
        .unwrap()
        .iter()
        .map(|c| c.command.clone())
        .collect()
}

// ── task 7.2: 完了通知を占有 horizon まで遅らせる drive-level 注入時刻檻（R2.5/D6） ──
//
// これらは **drive-level**（実 talk アクター＋done チャンネル）でしか捕捉できない早期終了の檻
// である（compile-level の extent 檻は「配送し終えた」時点の完了を検知できない）。共通の骨子:
//
// - **負の窓（早期終了しないことの決定的証明）**: horizon 未満の注入時刻まで駆動した後、
//   `recv_done(&done_rx, NEG_WINDOW)` が **timeout（`is_err()`）** することを主張する。完了通知は
//   `is_completed()`（占有 horizon gated）でしか送られないため、horizon 未満では送信自体が起きず
//   recv は必ず timeout する。逆にもし「entry 枯渇＝完了」の早期終了バグがあれば、この窓で
//   `TalkDone` が既に届き `recv_timeout` が **成功**して `is_err()` が偽になり檻が落ちる（バグ検出）。
//   窓長（数百 ms）はアクターの tick 処理（μs 台）を遥かに上回るため、正常系では送信が無く必ず
//   timeout、バグ系では送信が窓内に届く——両方向に決定的（実時計依存は安全余裕であって精度要件でない）。
// - **時間障壁の兼用**: `recv_timeout(NEG_WINDOW)` の待機中にアクターは投函済み Tick を全消化して
//   recv でブロックするため、窓明けの `records`（全 cue 配送済み）と `is_finished()==false`
//   （駆動継続）は race なく観測できる。
// - **正の確認**: その後 horizon 到達の Tick を投函し `recv_timeout(5s)` で `TalkDone` を受けることで
//   「horizon 到達で初めて完了する」を示す（末尾の待ち・最終テキストの duration が終端で切り捨てられない）。

/// 早期終了バグを疑って完了通知を待つ負の窓長（正常系の timeout 待機・アクター処理 μs を遥かに上回る）。
pub(super) const NEG_WINDOW: Duration = Duration::from_millis(200);
