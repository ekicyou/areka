//! `CuePlayer` — cue 再生の受動的注入時刻ランタイム。
//!
//! cue 再生の"制御"（再生状態の状態機械・外部解決待ちのバリア seam・選択肢の配送列合流＋
//! 解決照合バッグ・占有 horizon による完了判定）を、散在していた sakura `drive` と旧世代 wintf `ecs/cue`
//! （`CueQueue`）から dola へ一本化する受動ランタイム（D7）。dola は
//! `"Declarative Orchestration for Live Animation"` 層であり、cue 再生制御の正しい住処。
//!
//! # 受動ライブラリ（スレッド/channel を持たない）
//!
//! `CuePlayer` は**外部から注入された時刻**（[`tick`](CuePlayer::tick)）でのみ進行する受動的な
//! オブジェクトである。スレッドや channel は持たず、アクター化（時刻源・inbox）は上位の
//! sakura talk アクターの領分（[[areka-concurrency-model]] 整合）。これにより headless で
//! 決定論的にテストできる。
//!
//! # 2 フェーズ API（`TimedSchedule` と対称）
//!
//! ```text
//! player.tick(current_time)  → 内部 schedule を進め、バリア到達・choice 配送列合流を処理
//! player.ready()             → 直前 tick で配送可能になった cue（Choice も順序保持で合流）
//! ```
//!
//! # port 範囲（Task 4.2）
//!
//! 旧 `CueQueue` の状態機械のうち、**バリア seam＋choice 配送列合流／解決照合バッグ＋占有 horizon
//! 完了**のみを移植する。動的な一時停止/再開（`Paused`/`pause`/`resume`）は Non-Goals ゆえ**持ち込まない**
//! （dola へ pause/resume 状態を持ち込まない・§Non-Goals）。
//!
//! # 配送・完了・中断（Task 4.3）
//!
//! Task 4.2 の状態機械骨格の上に、演者への配送口と caller 向けの終了 API を載せる:
//!
//! - **broadcast fan-out**（[`register_sink`](CuePlayer::register_sink)／[`tick`](CuePlayer::tick)）:
//!   準備完了した action cue を、登録された**全** [`CueSink`] へ**選別せず一斉配送**する
//!   （中央 router による事前振り分けは廃止・どの action を演じるかは演者側 relevance の責務・
//!   D4）。全 sink が同一の cue 列を同一絶対時刻で受け、協調なしに同期が成立する（R2.1/R1.4）。
//! - **占有終了検知**（[`is_completed`](CuePlayer::is_completed)）: 配送完了（entry 枯渇）でなく
//!   占有 horizon 到達で初めて完了とする（末尾 Wait・最終 Text の duration を終端で落とさない・
//!   早期終了しない・R2.5/D6）。
//! - **中断/破棄**（[`stop`](CuePlayer::stop)）: 残 entry を破棄して以降の配送を止める（Close/
//!   中断の discard プリミティブ・アクター層の Close funnel 自体は上流 sakura の領分）。

use super::command::{BarrierKind, CueCommand, TalkCue};
use super::schedule::TimedSchedule;
use super::sheet::{CueSheet, to_talk_schedule};
use super::sink::CueSink;

/// 選択肢バリア（`WaitForChoice`）の手前で蓄積された選択肢データ（**解決照合専用のバッグ**）。
///
/// 上流の台本は選択肢を `WaitForChoice` バリアの直前に連続投入する。案C（R1.8/R8.6）では
/// `CuePlayer` はこれらを [`ready`](CuePlayer::ready) の配送列へ**順序を保って合流**させる
/// （配送列＝配置/表示情報の単一真実源）と同時に、本型として蓄積して
/// [`pending_choices`](CuePlayer::pending_choices) で取得可能にする（バッグ＝解決照合の単一
/// 真実源＝[`resolve_choice`](CuePlayer::resolve_choice) の id 照合用）。この責務二分により、
/// choice の表示・ヒットテストは配送列を消費する下流（choice-render）が担い、選択解決は
/// バッグの id 照合が担う。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingChoice {
    /// 選択肢 ID（解決時に [`resolve_choice`](CuePlayer::resolve_choice) へ渡す照合キー）。
    pub id: String,
    /// 選択肢の表示テキスト。
    pub text: String,
}

/// cue 再生ランタイムの状態。
///
/// 旧 `CueQueue` の `CueQueueState` から、動的一時停止（`Paused`）を除いた最小集合
/// （`pause`/`resume` は Non-Goals ゆえ本ランタイムには存在しない・§Non-Goals）。
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

/// tick 内でバリア到達を判定した結果（`schedule` の借用を跨がないための中間表現）。
enum BarrierReached {
    /// クリック/入力待ち → `WaitingForInput` へ停止。
    Input,
    /// 選択肢待ち → `WaitingForChoice` へ停止。
    Choice,
    /// 指定時間経過待ち → `Playing` を維持（`TimedSchedule` が継続 tick で自動解除）。
    Timeout,
}

/// cue 再生の受動的注入時刻ランタイム。
///
/// 内部に canonical 変換 [`to_talk_schedule`] が組んだ [`TimedSchedule`]`<`[`TalkCue`]`>` を
/// 保持し、その上に再生状態機械（[`CuePlayerState`]）・選択肢先積み（[`PendingChoice`]）・
/// バリア解決 seam を載せる。旧 wintf `CueQueue`（状態機械）と sakura `drive.on_tick`
/// （schedule tick＋ready 処理）の制御責務を一本化したもの（D7）。
pub struct CuePlayer {
    /// 時刻管理の中核（canonical 変換が組む・占有 horizon 保持）。
    schedule: TimedSchedule<TalkCue>,
    /// 現在の再生状態。
    state: CuePlayerState,
    /// 選択肢バリアの手前で蓄積された選択肢データ（解決照合専用バッグ・配送列とは別軸）。
    pending_choices: Vec<PendingChoice>,
    /// 直前 tick で配送可能になった cue のバッファ（案C＝Choice も順序保持で合流する配送列）。
    filtered_ready: Vec<TalkCue>,
    /// 登録された出力先（演者）。**登録順**で保持し、broadcast はこの順で全 sink へ配る
    /// （どの sink も全 cue を受ける・演者側 relevance が action を選別する・D4）。
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
            .field("filtered_ready", &self.filtered_ready)
            .field("sink_count", &self.sinks.len())
            .finish()
    }
}

impl CuePlayer {
    /// 台本 [`CueSheet`] から構築する（唯一の canonical 変換 [`to_talk_schedule`] を内部で通す）。
    ///
    /// 台本の [`absolute_start_time`](CueSheet::absolute_start_time)（dispatch 刻印まで 0.0）が
    /// スケジュールのアンカーへ流れ、各 cue の相対 `start_time`＋duration がそのまま保存される。
    /// 変換は**この 1 本のみ**——`CueSheet→schedule` の二重実装を作らない（R11.1・D7）。
    pub fn from_sheet(sheet: &CueSheet) -> Self {
        Self::from_schedule(to_talk_schedule(sheet))
    }

    /// canonical 変換で得た [`TimedSchedule`]`<`[`TalkCue`]`>` を直接包んで構築する。
    ///
    /// [`from_sheet`](Self::from_sheet) は内部でこの構築口を経由する。既に変換済みの
    /// スケジュールを保持している呼び出し側（後続 Task で talk アクターが刻印後の台本を
    /// 変換する経路）が合成しやすいよう公開する。
    pub fn from_schedule(schedule: TimedSchedule<TalkCue>) -> Self {
        Self {
            schedule,
            state: CuePlayerState::Playing,
            pending_choices: Vec::new(),
            filtered_ready: Vec::new(),
            sinks: Vec::new(),
        }
    }

    // ── sink 登録（broadcast 配送先） ──

    /// 出力先（演者）を 1 つ登録する。演者は再生開始前／開始時に登録する想定で、broadcast は
    /// **登録順**で全 sink へ配る（決定論的順序）。
    ///
    /// 登録された全 sink は [`tick`](Self::tick) で準備完了した action cue を**選別なく**受け取る
    /// （broadcast＝どの sink も全 cue を受ける）。自分が担当する action か否かの選別は**演者側の
    /// relevance 判定**（[`cue_target_of`](super::sink::cue_target_of) が単一権威）が行い、
    /// 担当外 cue でも duration は honor する——この振り分けは演者の責務であり本ランタイムは
    /// 中央 router として事前振り分けしない（D4）。
    pub fn register_sink(&mut self, sink: Box<dyn CueSink>) {
        self.sinks.push(sink);
    }

    // ── 2 フェーズ API ──

    /// Phase 1: 注入時刻でランタイムを進める。
    ///
    /// `current_time` は絶対時刻（スケジュールのアンカー基準）。内部 [`TimedSchedule::tick`] に
    /// 委譲し、到達済み cue を [`ready`](Self::ready) 配送列へ収集する。案C（R1.8/R8.6）: `Choice`
    /// も他 cue と**順序を保って配送列へ合流**しつつ、解決照合用に
    /// [`pending_choices`](Self::pending_choices) へも積む（配送列＝表示の真実源／バッグ＝照合の
    /// 真実源＝責務二分）。バリア到達（`WaitForInput`/`WaitForChoice`）で待機状態へ遷移して**停止**し、
    /// 以降の cue は外部解決まで配送しない。占有 horizon 到達（かつ entry 枯渇・バリアなし）で
    /// `Completed` へ遷移する。
    ///
    /// 待機中（`WaitingForInput`/`WaitingForChoice`）・完了後（`Completed`）は進行しない
    /// （`ready` バッファをクリアして早期 return）。再開は外部解決 seam
    /// （[`resolve_click`](Self::resolve_click)/[`resolve_choice`](Self::resolve_choice)/
    /// [`skip_barrier`](Self::skip_barrier)）が担う。
    ///
    /// なお `Timeout` バリアは待機状態にせず `Playing` を維持する——時間経過による解除は
    /// [`TimedSchedule`] が継続 tick の中で自動管理するため（外部解決を要さない）。
    pub fn tick(&mut self, current_time: f64) {
        // 待機中・完了後は進行しない（バリア seam の外部解決/占有終了で確定した状態を保持）。
        if !matches!(self.state, CuePlayerState::Playing) {
            self.filtered_ready.clear();
            return;
        }

        // schedule が本 tick で実際に前進したか（entry を pop したか）の判定材料。
        // schedule は前進時のみ ready_buffer を clear→再収集し entries を pop する（remaining 減少）。
        // 冪等な再 tick や Timeout バリア継続は early-return で ready_buffer を据え置く（remaining
        // 不変）ため、この差分で「新規に配送可能になった cue」だけを broadcast できる（下記参照）。
        let remaining_before = self.schedule.remaining();
        self.schedule.tick(current_time);

        // ready() を配送列へ collect する。案C（R1.8/R8.6）: Choice は他 cue（改行/カーソル/
        // テキスト等）と**順序を保って配送列へ合流**する（配送列から隠さない）。schedule の安定
        // FIFO をそのまま複写ゆえ、`\q \n \q \_l \q` の交互配置が broadcast 列に現れる。
        // 分離（Choice を配送列から隠す先積み一択）は廃止した。Choice の pending_choices への
        // 積み（解決照合専用のバッグ）は、下の配送ゲート内側で行う（冪等再 tick で二重積みしない
        // ことを構造で保証する・後述）。
        self.filtered_ready.clear();
        for cue in self.schedule.ready() {
            self.filtered_ready.push(cue.clone());
        }

        // broadcast fan-out ＋ 選択肢バッグ積み: 準備完了した cue を、登録された**全** sink へ選別
        // なく配る（R2.1/R1.4）。あわせて Choice cue は解決照合用に pending_choices へ積む
        // （責務二分＝配送列は表示の単一真実源／バッグは照合の単一真実源・R8.6）。両者とも schedule
        // が前進して entry を pop した tick（remaining 減少）に限り実行し、冪等再 tick・Timeout 継続
        // の early-return（remaining 不変・ready_buffer 据え置き）では実行しない——これにより各 cue は
        // 生涯 1 回のみ配送され、各 Choice は生涯 1 回のみバッグへ積まれる（同一時刻の再 tick で
        // 二重積みしない＝「bag 内容は tick 列に不変」をゲート内側化で構造保証）。バリア到達で待機へ
        // 遷移する場合も、バリア**手前**の cue はここで既に配り・積み済み（barrier match より前）。
        if self.schedule.remaining() < remaining_before {
            for cue in &self.filtered_ready {
                if let CueCommand::Choice { id, text, .. } = &cue.command {
                    self.pending_choices.push(PendingChoice {
                        id: id.clone(),
                        text: text.clone(),
                    });
                }
                for sink in self.sinks.iter_mut() {
                    sink.emit(cue.clone());
                }
            }
        }

        // バリア到達判定。`current_barrier()` の借用を閉じてから状態を書き換えるため、
        // 必要な分類だけを所有値（`BarrierReached`）へ写し取る。match は BarrierKind を
        // 網羅する（catch-all を置かない）ため、将来 variant 追加時にコンパイラが強制更新する。
        let reached = self.schedule.current_barrier().map(|kind| match kind {
            BarrierKind::WaitForInput { .. } => BarrierReached::Input,
            BarrierKind::WaitForChoice { .. } => BarrierReached::Choice,
            BarrierKind::Timeout { .. } => BarrierReached::Timeout,
        });
        match reached {
            Some(BarrierReached::Input) => {
                self.state = CuePlayerState::WaitingForInput;
                return;
            }
            Some(BarrierReached::Choice) => {
                // 注: `WaitForChoice` バリアの手前に選択肢が 1 件も無い（`pending_choices` 空）
                // 台本は不正だが、待機状態は観測可能（`state`＋空の `pending_choices`）であり、
                // `skip_barrier` で脱出可能ゆえ黙って詰まる（silent dead-end）にはならない。
                self.state = CuePlayerState::WaitingForChoice;
                return;
            }
            // `Timeout` は Playing を維持（TimedSchedule が継続 tick で自動解除する）。
            Some(BarrierReached::Timeout) => return,
            None => {}
        }

        // 占有終了検出: 全 entry 消費済み・バリアなし・かつ現在時刻が占有 horizon 到達。
        // entry 枯渇（＝最後の cue の配送時刻）だけでは完了とせず、末尾 Wait・最終 Text の
        // duration を含む占有 horizon 到達で初めて Completed とする（早期終了しない・D6/R2.5）。
        if self.schedule.is_completed() {
            self.state = CuePlayerState::Completed;
        }
    }

    /// Phase 2: 直前 tick で配送可能になった cue のスライス（案C＝`Choice` も順序保持で合流）。
    ///
    /// これが配置/表示情報の単一真実源であり、`Choice` cue も他 cue との相対順序を保って現れる
    /// （R1.8/R8.6）。同じ選択肢は解決照合用に [`pending_choices`](Self::pending_choices) にも
    /// 積まれる（責務二分）。次の [`tick`](Self::tick) まで何度でも参照可能。
    pub fn ready(&self) -> &[TalkCue] {
        &self.filtered_ready
    }

    // ── バリア解決 seam（外部解決で待機状態から Playing へ戻す） ──

    /// クリック/入力バリア（`WaitingForInput`）を外部解決して再開する。
    ///
    /// 非待機状態（`Playing`/`WaitingForChoice`/`Completed`）では no-op。解決後に既に占有
    /// horizon へ達していれば `Completed` へ遷移する。
    pub fn resolve_click(&mut self) {
        if self.state == CuePlayerState::WaitingForInput {
            self.schedule.notify_barrier_resolved(None);
            self.state = CuePlayerState::Playing;
            self.settle_completion_after_resolve();
        }
    }

    /// 選択肢バリア（`WaitingForChoice`）を、先積み選択肢に**該当する id** で外部解決して再開する。
    ///
    /// - 該当 id が先積み選択肢に存在: 先積みをクリアし `Playing` へ戻して `Some(id)` を返す。
    /// - 非待機状態、または該当 id が存在しない: 状態を変えず `None` を返す。
    ///
    /// 解決後に既に占有 horizon へ達していれば `Completed` へ遷移する。
    pub fn resolve_choice(&mut self, choice_id: &str) -> Option<String> {
        if self.state != CuePlayerState::WaitingForChoice {
            return None;
        }
        if self.pending_choices.iter().any(|c| c.id == choice_id) {
            self.pending_choices.clear();
            self.schedule
                .notify_barrier_resolved(Some(choice_id.to_string()));
            self.state = CuePlayerState::Playing;
            self.settle_completion_after_resolve();
            Some(choice_id.to_string())
        } else {
            None
        }
    }

    /// 待機中のバリア（入力/選択いずれも）を強制スキップして再開する。
    ///
    /// 先積み選択肢もクリアする。非待機状態では no-op。解決後に既に占有 horizon へ達して
    /// いれば `Completed` へ遷移する。
    pub fn skip_barrier(&mut self) {
        if matches!(
            self.state,
            CuePlayerState::WaitingForInput | CuePlayerState::WaitingForChoice
        ) {
            self.pending_choices.clear();
            self.schedule.notify_barrier_resolved(None);
            self.state = CuePlayerState::Playing;
            self.settle_completion_after_resolve();
        }
    }

    /// 外部解決の直後、既に占有 horizon へ達していれば `Completed` へ確定する共通処理。
    ///
    /// バリア解除後、後続 cue が存在せず現在時刻が既に horizon を越えている場合に、
    /// 次の tick を待たずその場で完了状態にする（旧 `CueQueue` の解決後 completed 確定と同型）。
    fn settle_completion_after_resolve(&mut self) {
        if self.schedule.is_completed() {
            self.state = CuePlayerState::Completed;
        }
    }

    // ── 中断/破棄 ──

    /// 再生を中断し、残 entry を破棄して以降の配送を止める（Close/中断の discard プリミティブ）。
    ///
    /// 呼び出し後、プレイヤーは終端（`Completed`）となり、[`tick`](Self::tick) は cue を配送しない。
    /// これは配送を止める最小プリミティブであり、アクター層の Close funnel（中断トリガの集約・
    /// `TalkDone` の中断 ACK 返却）自体は上流 sakura talk アクターの領分（本ランタイムは受動）。
    /// 中断による終端か占有 horizon による自然完了かの区別は、呼び出し側（drive）が保持する
    /// （本ランタイムはどちらも「以降配送しない終端」として同一に扱う）。
    pub fn stop(&mut self) {
        self.schedule.clear();
        self.pending_choices.clear();
        self.filtered_ready.clear();
        self.state = CuePlayerState::Completed;
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
    /// （配送 ≠ 再生完了・早期終了しない・R2.5/D6）。[`stop`](Self::stop) による中断終端でも `true`。
    pub fn is_completed(&self) -> bool {
        matches!(self.state, CuePlayerState::Completed)
    }

    /// 先積みされた選択肢データ（`WaitForChoice` バリアの手前で蓄積されたもの）。
    pub fn pending_choices(&self) -> &[PendingChoice] {
        &self.pending_choices
    }

    /// 現在停止中のバリア種別（待機中でなければ `None`）。
    pub fn current_barrier(&self) -> Option<&BarrierKind> {
        self.schedule.current_barrier()
    }

    /// スケジュールに残る未配送 entry 数（テスト・診断用の introspection）。
    ///
    /// 注: これは schedule の残数であり、caller 向けの占有終了完了問い合わせ API ではない
    /// （完了検知の contract は後続 Task の領分）。
    pub fn remaining(&self) -> usize {
        self.schedule.remaining()
    }
}
