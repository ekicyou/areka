//! `CuePlayer` — cue 再生の受動的注入時刻ランタイム。
//!
//! cue 再生の"制御"（再生状態の状態機械・外部解決待ちのバリア seam・選択肢の先積み・
//! 占有 horizon による完了判定）を、散在していた sakura `drive` と旧世代 wintf `ecs/cue`
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
//! player.tick(current_time)  → 内部 schedule を進め、バリア到達・Choice 先積みを処理
//! player.ready()             → 直前 tick で配送可能になった cue（Choice 除外済み）
//! ```
//!
//! # port 範囲（Task 4.2）
//!
//! 旧 `CueQueue` の状態機械のうち、**バリア seam＋Choice 先積み＋占有 horizon 完了**のみを
//! 移植する。動的な一時停止/再開（`Paused`/`pause`/`resume`）は Non-Goals ゆえ**持ち込まない**
//! （dola へ pause/resume 状態を持ち込まない・§Non-Goals）。broadcast fan-out・sink 登録・
//! caller 向け完了問い合わせ API・中断/破棄は後続 Task 4.3 の領分（本 module では扱わない）。

use super::command::{BarrierKind, CueCommand, TalkCue};
use super::schedule::TimedSchedule;
use super::sheet::{CueSheet, to_talk_schedule};

/// 選択肢バリア（`WaitForChoice`）の手前で先積みされた選択肢データ。
///
/// 上流の台本は選択肢を `WaitForChoice` バリアの直前に連続投入する（先積みプロトコル）。
/// `CuePlayer` はこれらを [`ready`](CuePlayer::ready) の action cue として surface せず、
/// 本型として蓄積し、[`pending_choices`](CuePlayer::pending_choices) で取得可能にする。
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
#[derive(Debug)]
pub struct CuePlayer {
    /// 時刻管理の中核（canonical 変換が組む・占有 horizon 保持）。
    schedule: TimedSchedule<TalkCue>,
    /// 現在の再生状態。
    state: CuePlayerState,
    /// 選択肢バリアの手前で先積みされた選択肢データ。
    pending_choices: Vec<PendingChoice>,
    /// 直前 tick で配送可能になった cue（Choice 除外済み）のバッファ。
    filtered_ready: Vec<TalkCue>,
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
        }
    }

    // ── 2 フェーズ API ──

    /// Phase 1: 注入時刻でランタイムを進める。
    ///
    /// `current_time` は絶対時刻（スケジュールのアンカー基準）。内部 [`TimedSchedule::tick`] に
    /// 委譲し、到達済み cue のうち `Choice` を [`pending_choices`](Self::pending_choices) へ
    /// 先積み（action cue としては surface しない）、残りを [`ready`](Self::ready) バッファへ
    /// 収集する。バリア到達（`WaitForInput`/`WaitForChoice`）で待機状態へ遷移して**停止**し、
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

        self.schedule.tick(current_time);

        // ready() から Choice を分離（Choice は先積みし action cue として surface しない）。
        self.filtered_ready.clear();
        for cue in self.schedule.ready() {
            match &cue.command {
                CueCommand::Choice { id, text } => {
                    self.pending_choices.push(PendingChoice {
                        id: id.clone(),
                        text: text.clone(),
                    });
                }
                _ => self.filtered_ready.push(cue.clone()),
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

    /// Phase 2: 直前 tick で配送可能になった cue のスライス（`Choice` は除外済み）。
    ///
    /// 選択肢は [`pending_choices`](Self::pending_choices) で取得する。次の [`tick`](Self::tick)
    /// まで何度でも参照可能。
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

    // ── 状態照会 ──

    /// 現在の再生状態。
    pub fn state(&self) -> &CuePlayerState {
        &self.state
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
