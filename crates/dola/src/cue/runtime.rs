//! `CuePlayer` — cue 再生の受動的注入時刻ランタイム。
//!
//! cue 再生の制御（再生状態の状態機械・外部解決待ちのバリア seam・選択肢の配送列合流＋
//! 解決照合バッグ・占有 horizon による完了判定）を担う受動ランタイム。
//!
//! # 受動ライブラリ（スレッド/channel を持たない）
//!
//! `CuePlayer` は**外部から注入された時刻**（[`tick`](CuePlayer::tick)）でのみ進行する受動的な
//! オブジェクトである。スレッドや channel は持たず、アクター化（時刻源・inbox）は上位の
//! sakura talk アクターの領分。これにより headless で
//! 決定論的にテストできる。
//!
//! # 時刻注入 API
//!
//! ```text
//! player.step(now)                 → 届いた 1 件を進める（合図を全 sink へ配る・区切りで止まる）
//! player.tick(now)                 → step を Delivered の間くり返す
//! player.reanchor(now, at, t)      → 起点を置き直し、変わったら全 sink へ知らせる
//! player.shift_origin(now, delta)  → 同上（起点を delta だけ動かす）
//! ```
//!
//! 時刻に依る事（どれが届いたか・完了したか）を決めるのは `step` だけである。区切りを解く口は
//! 時刻を受けず、区切りの後ろの配送と完了は、次の `step`／`tick` が決める。
//!
//! # 持たないもの
//!
//! 動的な一時停止/再開（`Paused`/`pause`/`resume`）は持たない。
//!
//! # 配送・完了・中断
//!
//! - **broadcast fan-out**（[`register_sink`](CuePlayer::register_sink)／[`tick`](CuePlayer::tick)）:
//!   準備完了した action cue を、登録された**全** [`CueSink`] へ**選別せず一斉配送**する
//!   （どの action を演じるかは演者側 relevance の責務）。全 sink が同一の cue 列を同一絶対時刻で
//!   受け、協調なしに同期が成立する。
//! - **占有終了検知**（[`is_completed`](CuePlayer::is_completed)）: 配送完了（entry 枯渇）でなく
//!   占有 horizon 到達で初めて完了とする（末尾 Wait・最終 Text の duration を終端で落とさない）。
//! - **中断/破棄**: 専用の口は持たない。上流が player を捨てれば以降の配送は無い（Close funnel
//!   自体は上流 sakura の領分）。

use super::command::{BarrierKind, CueCommand, PendingCue, TalkCue};
use super::schedule::{Due, OriginNotice, TimedSchedule};
use super::sheet::{CueSheet, to_talk_schedule};
use super::sink::CueSink;
use crate::time::{TimeDelta, TimePoint, TimeSpan};

/// cue の層の誤り。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CueError {
    /// 台本の起点が押されていない。
    #[error("cue sheet has no absolute start time")]
    OriginNotStamped,
}

/// [`CuePlayer::step`] の結果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepOutcome {
    /// 合図を 1 件、全部の受け口へ配った。
    Delivered,
    /// 区切りで止まっている（着いた・待っている）。
    Barrier,
    /// 届いた物が無く、まだ終わっていない。
    Idle,
    /// 占有の終わりに届いた（完了の後も）。
    Completed,
}

/// 選択肢バリア（`WaitForChoice`）の手前で蓄積された選択肢データ（**解決照合専用のバッグ**）。
///
/// 上流の台本は選択肢を `WaitForChoice` バリアの直前に連続投入する。
/// `CuePlayer` はこれらを sink への配送列へ**順序を保って合流**させる
/// （配送列＝配置/表示情報の単一真実源）と同時に、本型として蓄積して
/// [`pending_choices`](CuePlayer::pending_choices) で取得可能にする（バッグ＝解決照合の単一
/// 真実源＝[`resolve_choice`](CuePlayer::resolve_choice) の id 照合用）。この責務二分により、
/// choice の表示・ヒットテストは配送列を消費する下流（choice-render）が担い、選択解決は
/// バッグの id 照合が担う。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingChoice {
    /// 選択肢 ID（解決時に [`resolve_choice`](CuePlayer::resolve_choice) へ渡す照合キー）。
    pub id: String,
}

/// cue 再生ランタイムの状態。
///
/// 動的一時停止（`Paused`）を持たない最小集合（`pause`/`resume` は存在しない）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CuePlayerState {
    /// 再生中（時刻注入で進行する）。
    Playing,
    /// クリック/キー入力待ちバリアで停止中（外部解決待ち）。
    WaitingForInput,
    /// 選択肢バリアで停止中（外部解決待ち）。
    WaitingForChoice,
    /// 占有終了（全 cue 配送済みかつ占有 horizon 到達）。
    Completed,
}

/// cue 再生の受動的注入時刻ランタイム。
///
/// 内部に canonical 変換 [`to_talk_schedule`] が組んだ [`TimedSchedule`]`<`[`PendingCue`]`>` を
/// 保持し、その上に再生状態機械（[`CuePlayerState`]）・選択肢先積み（[`PendingChoice`]）・
/// バリア解決 seam を載せる。
pub struct CuePlayer {
    /// 時刻管理の中核（canonical 変換が組む・占有 horizon 保持）。
    schedule: TimedSchedule<PendingCue>,
    /// 現在の再生状態。
    state: CuePlayerState,
    /// 選択肢バリアの手前で蓄積された選択肢データ（解決照合専用バッグ・配送列とは別軸）。
    pending_choices: Vec<PendingChoice>,
    /// 登録された出力先（演者）。**登録順**で保持し、broadcast はこの順で全 sink へ配る
    /// （どの sink も全 cue を受ける・演者側 relevance が action を選別する）。
    sinks: Vec<Box<dyn CueSink>>,
}

// `Box<dyn CueSink>` は `Debug` を要求しない（`CueSink` は infallible な emit のみの契約）ため
// `#[derive(Debug)]` は使えない。sink 群は数だけを出し、他フィールドは委譲する手動実装とする。
impl std::fmt::Debug for CuePlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CuePlayer")
            .field("schedule", &self.schedule)
            .field("state", &self.state)
            .field("pending_choices", &self.pending_choices)
            .field("sink_count", &self.sinks.len())
            .finish()
    }
}

impl CuePlayer {
    /// 台本 [`CueSheet`] から構築する（唯一の canonical 変換 [`to_talk_schedule`] を内部で通す）。
    ///
    /// 台本の [`absolute_start_time`](CueSheet::absolute_start_time) がスケジュールの起点へ流れ、
    /// 各 cue の相対 `start_time`＋duration がそのまま保存される。起点の押されていない台本は
    /// `Err(CueError::OriginNotStamped)`。
    /// 変換は**この 1 本のみ**——`CueSheet→schedule` の二重実装を作らない。
    pub fn from_sheet(sheet: &CueSheet) -> Result<Self, CueError> {
        let schedule = to_talk_schedule(sheet)?;
        Ok(Self {
            schedule,
            state: CuePlayerState::Playing,
            pending_choices: Vec::new(),
            sinks: Vec::new(),
        })
    }

    // ── sink 登録（broadcast 配送先） ──

    /// 出力先（演者）を 1 つ登録する。演者は再生開始前／開始時に登録する想定で、broadcast は
    /// **登録順**で全 sink へ配る（決定論的順序）。
    ///
    /// 登録された全 sink は [`tick`](Self::tick) で準備完了した action cue を**選別なく**受け取る
    /// （broadcast＝どの sink も全 cue を受ける）。自分が担当する action か否かの選別は**演者側の
    /// relevance 判定**（[`cue_target_of`](super::sink::cue_target_of) が単一権威）が行い、
    /// 担当外 cue でも duration は honor する——この振り分けは演者の責務であり本ランタイムは
    /// 中央 router として事前振り分けしない。
    ///
    /// 登録の列へ足す**前**に、その時の起点を [`CueSink::origin_set`]（最初の起点の知らせ）で、
    /// 次にまだ配っていない合図の列を [`CueSink::preview`] で 1 度渡す（先渡し）。`emit` は
    /// 登録済みの受け手にしか行かないので、どちらも必ずその受け手への最初の `emit` より前になる。
    /// 先渡しの `fire_time` はその時の起点での予定の値で、後の置き直しで `emit` の値と違いうる
    /// （途中で登録すれば残りだけ）。
    pub fn register_sink(&mut self, mut sink: Box<dyn CueSink>) {
        sink.origin_set(OriginNotice::first(self.schedule.origin()));
        // ponytail: 登録のたびに列を写す（受け手 1 つにつき 1 回）。受け手は十数個・合図は
        // 多くて数百なので足りる。重くなったら再生機に 1 度だけ作って持たせる形へ替える。
        let upcoming: Vec<TalkCue> = self
            .schedule
            .pending_payloads()
            .map(|(fire_time, at, cue)| cue.clone().fire(fire_time, at))
            .collect();
        sink.preview(&upcoming);
        self.sinks.push(sink);
    }

    /// 登録された全部の受け手へ [`CueSink::choice_wait_began`] を登録順に 1 回ずつ配る。
    ///
    /// 呼ぶのは再生の側（選択待ちに入った事実を完了の口へ送った後）で、`tick` の中からは
    /// 呼ばない（状態の移りの中で配ると、選択待ちの知らせより先に受け手へ着きうる）。
    pub fn announce_choice_wait(&mut self) {
        for sink in self.sinks.iter_mut() {
            sink.choice_wait_began();
        }
    }

    // ── 時刻注入 ──

    /// 届いた項目を 1 件だけ進める。
    ///
    /// 届いた合図は、取り出した時の起点＋相対の発火時刻を付けて全 sink へ登録順に配る
    /// （`Choice` は解決照合用に [`pending_choices`](Self::pending_choices) へも積む）。区切りに
    /// 着いたら待機状態へ移って止まる。届いた物が無ければ、占有の終わりに届いていれば
    /// `Completed` へ移る。待機中は `Barrier`、完了の後は `Completed` を返し、進まない。
    pub fn step(&mut self, now: TimePoint) -> StepOutcome {
        match self.state {
            CuePlayerState::Playing => {}
            CuePlayerState::WaitingForInput | CuePlayerState::WaitingForChoice => {
                return StepOutcome::Barrier;
            }
            CuePlayerState::Completed => return StepOutcome::Completed,
        }
        match self.schedule.pop_due(now) {
            Some(Due::Payload {
                fire_time,
                at,
                payload,
            }) => {
                let cue = payload.fire(fire_time, at);
                if let CueCommand::Choice { id, .. } = &cue.command {
                    self.pending_choices.push(PendingChoice { id: id.clone() });
                }
                for sink in self.sinks.iter_mut() {
                    sink.emit(cue.clone());
                }
                StepOutcome::Delivered
            }
            // 注: `WaitForChoice` の手前に選択肢が 1 件も無い台本は不正だが、待機状態は
            // 観測可能（`state`＋空の `pending_choices`）で、上流が player を捨てて抜ける。
            Some(Due::Barrier(kind)) => {
                self.state = match kind {
                    BarrierKind::WaitForInput => CuePlayerState::WaitingForInput,
                    BarrierKind::WaitForChoice { .. } => CuePlayerState::WaitingForChoice,
                };
                StepOutcome::Barrier
            }
            // 配り終えただけでは完了とせず、占有の終わりに届いて初めて Completed とする。
            None if self.schedule.is_completed_at(now) => {
                self.state = CuePlayerState::Completed;
                StepOutcome::Completed
            }
            None => StepOutcome::Idle,
        }
    }

    /// 届いた項目を全部進める（[`step`](Self::step) を `Delivered` の間くり返す）。
    pub fn tick(&mut self, now: TimePoint) {
        while self.step(now) == StepOutcome::Delivered {}
    }

    // ── 起点の置き直し ──

    /// 「時刻 `at` が台本の時刻 `script_time` である」ように起点を置き直し、起点が変わったら
    /// 知らせを全 sink へ登録順に配って返す。`now` は呼び手の現在の時刻（知らせの効く時刻）で、
    /// 覚えない。完了の後と、起点が変わらない時は、何もせず `None`。
    pub fn reanchor(
        &mut self,
        now: TimePoint,
        at: TimePoint,
        script_time: TimeSpan,
    ) -> Option<OriginNotice> {
        if self.is_completed() {
            return None;
        }
        let notice = self.schedule.reanchor(now, at, script_time)?;
        self.broadcast_origin(notice);
        Some(notice)
    }

    /// 起点を `delta` だけ動かす（負は前へ）。`now` と戻り値は [`reanchor`](Self::reanchor) と同じ。
    pub fn shift_origin(&mut self, now: TimePoint, delta: TimeDelta) -> Option<OriginNotice> {
        if self.is_completed() {
            return None;
        }
        let notice = self.schedule.shift_origin(now, delta)?;
        self.broadcast_origin(notice);
        Some(notice)
    }

    fn broadcast_origin(&mut self, notice: OriginNotice) {
        for sink in self.sinks.iter_mut() {
            sink.origin_set(notice);
        }
    }

    // ── バリア解決 seam（外部解決で待機状態から Playing へ戻す） ──

    /// クリック/入力バリア（`WaitingForInput`）を外部解決して再開する。
    ///
    /// `\x` の受け皿（ukadoc の `\x` に時間切れの引数は無い）。
    ///
    /// 非待機状態（`Playing`/`WaitingForChoice`/`Completed`）では no-op。時刻を受けず、完了も
    /// 決めない（区切りの後ろの配送と完了は、次の [`step`](Self::step)／[`tick`](Self::tick)）。
    pub fn resolve_click(&mut self) {
        if self.state == CuePlayerState::WaitingForInput {
            self.schedule.notify_barrier_resolved();
            self.state = CuePlayerState::Playing;
        }
    }

    /// 選択肢バリア（`WaitingForChoice`）を、先積み選択肢に**該当する id** で外部解決して再開する。
    ///
    /// - 該当 id が先積み選択肢に存在: 先積みをクリアし `Playing` へ戻して `Some(id)` を返す。
    /// - 非待機状態、または該当 id が存在しない: 状態を変えず `None` を返す。
    ///
    /// 時刻を受けず、完了も決めない（区切りの後ろの配送と完了は、次の [`step`](Self::step)／
    /// [`tick`](Self::tick)）。
    pub fn resolve_choice(&mut self, choice_id: &str) -> Option<String> {
        if self.state != CuePlayerState::WaitingForChoice {
            return None;
        }
        if self.pending_choices.iter().any(|c| c.id == choice_id) {
            self.pending_choices.clear();
            self.schedule.notify_barrier_resolved();
            self.state = CuePlayerState::Playing;
            Some(choice_id.to_string())
        } else {
            None
        }
    }

    // ── 状態照会 ──

    /// 現在の再生状態。
    pub fn state(&self) -> &CuePlayerState {
        &self.state
    }

    /// 占有終了に達したか（caller 向けの完了問い合わせ）。
    ///
    /// `true` は占有 horizon 到達（全 entry 配送済み**かつ**現在注入時刻 ≥ 占有 horizon）で確定した
    /// `Completed` 状態を意味する。entry 枯渇（＝最後の cue の**配送時刻**到達）だけでは `false` の
    /// ままで、末尾 Wait・最終 Text の duration を含む占有 horizon 到達で初めて `true` になる
    /// （配送 ≠ 再生完了・早期終了しない）。
    pub fn is_completed(&self) -> bool {
        matches!(self.state, CuePlayerState::Completed)
    }

    /// 占有 horizon の**絶対時刻**（起点 ＋ 相対 horizon・duration 権威）。
    ///
    /// 内部 [`TimedSchedule::occupancy_horizon`] へ委譲する照会口であり、
    /// [`is_completed`](Self::is_completed) が閾値として見ているのと**同一**の horizon を返す
    /// （＝トークの絶対終了時刻）。選択待ちのタイムアウト計測はこの値を起点として上位層
    /// （kanade）が行い、再生層の外に独自の時間基準を持ち込まない。
    ///
    /// 値は [`tick`](Self::tick) の進行では変化せず（配送位置でなく占有区間の終端を指す）、
    /// 起点を置き直すと一緒に動く。
    /// 選択肢バリアで停止中（`WaitingForChoice`）でも照会できる。
    pub fn occupancy_horizon(&self) -> TimePoint {
        self.schedule.occupancy_horizon()
    }

    /// 今の起点（置き直すと動く）。
    pub fn origin(&self) -> TimePoint {
        self.schedule.origin()
    }

    /// 先積みされた選択肢データ（`WaitForChoice` バリアの手前で蓄積されたもの）。
    pub fn pending_choices(&self) -> &[PendingChoice] {
        &self.pending_choices
    }

    /// 現在停止中のバリア種別（待機中でなければ `None`）。
    pub fn current_barrier(&self) -> Option<&BarrierKind> {
        self.schedule.current_barrier()
    }
}
