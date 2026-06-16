//! CueQueue — 演出指示キューコンポーネント。
//!
//! 各演者エンティティが保持する時刻ベースの演出指示キュー。
//! 内部は `dola::cue::TimedSchedule<CueCommand>` に委譲し、
//! ECS 固有の状態（再生状態・選択肢蓄積・シートエンティティ参照）を保持する。

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

use super::command::{BarrierKind, CueCommand, Entry, TimedSchedule};
use super::error::CueSystemError;

/// Choice コマンドの先積みデータ
#[derive(Clone, Debug)]
pub struct PendingChoice {
    pub id: String,
    pub text: String,
}

/// キュー状態
#[derive(Clone, Debug, PartialEq)]
pub enum CueQueueState {
    /// 再生中
    Playing,
    /// 一時停止中
    Paused,
    /// クリック/入力待ちバリア中
    WaitingForClick,
    /// 選択肢バリア中
    WaitingForChoice,
    /// エラー発生
    Error(CueSystemError),
    /// 全コマンド消費完了
    Completed,
}

/// バリア応答値。消費者がハンドラーとして返す、またはスキップする。
#[derive(Clone, Debug)]
pub enum BarrierResponse {
    /// 非ハンドラー（自ドメイン外のバリア）
    Skipped,
    /// クリック応答
    Click,
    /// 選択応答
    Choice { id: String },
    /// タイムアウト
    Timeout,
}

/// 各演者エンティティが保持する時刻ベースの演出指示キュー。
///
/// 内部は `TimedSchedule<CueCommand>` に委譲し、
/// ECS 固有の状態管理（再生状態・選択肢蓄積・シートエンティティ参照）を追加する。
///
/// # 2 フェーズ API
///
/// ```text
/// queue.tick(current_time)  → TimedSchedule に委譲、バリア・Choice 処理
/// queue.ready()             → 消費可能なコマンドスライス（Choice 除外済み）
/// ```
#[derive(Component, Debug)]
#[component(storage = "SparseSet")]
pub struct CueQueue {
    /// dola TimedSchedule にコア時刻管理を委譲
    schedule: TimedSchedule<CueCommand>,
    /// 現在の状態
    state: CueQueueState,
    /// 再生速度倍率
    playback_rate: f64,
    /// キャパシティ上限（None = 無制限）
    capacity: Option<usize>,
    /// Choice バリアの先積みデータ
    pending_choices: Vec<PendingChoice>,
    /// 現在この CueQueue にコマンドを供給している CueSheet の Tracker エンティティ
    cue_sheet_entity: Option<Entity>,
    /// tick() で Choice を除外した ready コマンドのバッファ
    filtered_ready: Vec<CueCommand>,
    /// バリア進入時刻（check_timeout 計算用）
    barrier_entered_time: Option<f64>,
    /// バリアタイムアウト値（check_timeout 計算用）
    barrier_timeout: Option<f64>,
}

impl CueQueue {
    /// 新しい CueQueue を生成する。
    ///
    /// `start_time` は TimedSchedule の絶対時刻基準。
    /// dispatch 時に設定されるため、初期値は 0.0。
    pub fn new() -> Self {
        Self {
            schedule: TimedSchedule::new(0.0),
            state: CueQueueState::Playing,
            playback_rate: 1.0,
            capacity: None,
            pending_choices: Vec::new(),
            cue_sheet_entity: None,
            filtered_ready: Vec::new(),
            barrier_entered_time: None,
            barrier_timeout: None,
        }
    }

    /// キャパシティ指定で CueQueue を生成する。
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            schedule: TimedSchedule::new(0.0),
            state: CueQueueState::Playing,
            playback_rate: 1.0,
            capacity: Some(capacity),
            pending_choices: Vec::new(),
            cue_sheet_entity: None,
            filtered_ready: Vec::new(),
            barrier_entered_time: None,
            barrier_timeout: None,
        }
    }

    // ── 追加 ──

    /// 新しい CueSheet 開始時にスケジュールを再初期化する。
    ///
    /// 既存エントリを全破棄し、新たなスケジュールを受け入れ可能にする。
    /// Entry のオフセットは絶対時刻で挿入されるため、TimedSchedule の
    /// start_time は常に 0.0（offset == current_time）を維持する。
    pub fn reset_schedule(&mut self, _start_time: f64) {
        self.schedule = TimedSchedule::new(0.0);
        self.pending_choices.clear();
        self.filtered_ready.clear();
        self.state = CueQueueState::Playing;
        self.barrier_entered_time = None;
        self.barrier_timeout = None;
    }

    /// ECS Entity を u64 に変換して EntityRef コマンドとして挿入するヘルパー。
    ///
    /// # push 境界変換
    /// `Entity::to_bits()` で u64 に変換し、`Entry::Payload` として挿入する。
    /// 消費時は [`resolve_entity_ref`] で `Entity::from_bits()` 復元を行う。
    pub fn push_entity_command(
        &mut self,
        absolute_time: f64,
        entity: Entity,
    ) -> Result<(), CueSystemError> {
        let bits = entity.to_bits();
        self.insert(Entry::Payload(absolute_time, CueCommand::EntityRef(bits)))
    }

    /// EntityRef(u64) を ECS Entity に復元する（pop 境界変換）。
    ///
    /// `Entity::from_bits()` で復元する。無効な Entity が返る可能性があるため、
    /// 消費者は `Query` で存在確認を行うこと。
    ///
    /// # Returns
    /// - `Some(Entity)` — 復元成功
    /// - `None` — 引数が EntityRef でない
    pub fn resolve_entity_ref(cmd: &CueCommand) -> Option<Entity> {
        match cmd {
            CueCommand::EntityRef(bits) => Some(Entity::from_bits(*bits)),
            _ => None,
        }
    }

    /// Entry<CueCommand> を挿入する。
    pub fn insert(&mut self, entry: Entry<CueCommand>) -> Result<(), CueSystemError> {
        if let Some(cap) = self.capacity {
            if self.schedule.remaining() >= cap {
                return Err(CueSystemError::CapacityExceeded { capacity: cap });
            }
        }

        self.schedule.insert(entry);

        // Completed 状態だった場合は Playing に戻す
        if self.state == CueQueueState::Completed {
            self.state = CueQueueState::Playing;
        }

        Ok(())
    }

    /// 複数の Entry<CueCommand> を一括挿入する。
    pub fn extend_entries(
        &mut self,
        entries: impl IntoIterator<Item = Entry<CueCommand>>,
    ) -> Result<(), CueSystemError> {
        let entries: Vec<Entry<CueCommand>> = entries.into_iter().collect();

        if let Some(cap) = self.capacity {
            if self.schedule.remaining() + entries.len() > cap {
                return Err(CueSystemError::CapacityExceeded { capacity: cap });
            }
        }

        self.schedule.extend(entries);

        // Completed 状態だった場合は Playing に戻す
        if self.state == CueQueueState::Completed {
            self.state = CueQueueState::Playing;
        }

        Ok(())
    }

    // ── 2 フェーズ API ──

    /// Phase 1: 時刻を進めてコマンドを収集する。
    ///
    /// TimedSchedule に委譲し、バリア状態と Choice 蓄積を処理する。
    /// バリア中・一時停止中・完了済み・エラー時は何もしない。
    pub fn tick(&mut self, current_time: f64) {
        // バリア中・Paused・Completed・Error は進行しない
        if !matches!(self.state, CueQueueState::Playing) {
            self.filtered_ready.clear();
            return;
        }

        self.schedule.tick(current_time);

        // ready() から Choice を分離
        self.filtered_ready.clear();
        for cmd in self.schedule.ready() {
            match cmd {
                CueCommand::Choice { id, text } => {
                    tracing::trace!(id = %id, text = %text, "[tick] Choice accumulated");
                    self.pending_choices.push(PendingChoice {
                        id: id.clone(),
                        text: text.clone(),
                    });
                }
                other => {
                    self.filtered_ready.push(other.clone());
                }
            }
        }

        // バリア到達チェック
        if let Some(barrier) = self.schedule.current_barrier() {
            match barrier {
                BarrierKind::WaitForChoice { timeout } => {
                    if self.pending_choices.is_empty() {
                        tracing::error!(
                            "[tick] WaitForChoice with no preceding Choice commands"
                        );
                        self.state = CueQueueState::Error(CueSystemError::EmptyChoiceBarrier {
                            actor: "unknown".to_string(),
                        });
                        return;
                    }
                    tracing::trace!(
                        choices = self.pending_choices.len(),
                        "[tick] WaitForChoice barrier entered"
                    );
                    self.state = CueQueueState::WaitingForChoice;
                    self.barrier_entered_time = Some(current_time);
                    self.barrier_timeout = *timeout;
                }
                BarrierKind::WaitForInput { timeout } => {
                    tracing::trace!("[tick] WaitForInput barrier entered");
                    self.state = CueQueueState::WaitingForClick;
                    self.barrier_entered_time = Some(current_time);
                    self.barrier_timeout = *timeout;
                }
                BarrierKind::Timeout { .. } => {
                    // Timeout はTimedSchedule が自動管理する。
                    // current_barrier() がSome(Timeout)を返す場合はまだ
                    // duration が未経過。次の tick で自動解除される。
                    tracing::trace!("[tick] Timeout barrier (auto-managed by TimedSchedule)");
                    // Timeout 中は Playing を維持（TimedSchedule 側でブロック）
                }
            }
            return;
        }

        // 全コマンド消費完了チェック
        if self.schedule.is_completed() && self.state == CueQueueState::Playing {
            self.state = CueQueueState::Completed;
        }
    }

    /// Phase 2: 直前の tick() で収集された消費可能コマンドのスライス。
    ///
    /// Choice コマンドは除外済み（`pending_choices()` で取得）。
    /// 次の tick() まで何度でも参照可能。
    pub fn ready(&self) -> &[CueCommand] {
        &self.filtered_ready
    }

    /// 後方互換: current_time に到達した全コマンドを返却。
    ///
    /// 内部で `tick()` + `ready().to_vec()` を実行する。
    /// 新規コードは `tick()` + `ready()` の 2 フェーズ API を使用すること。
    pub fn pop_ready(&mut self, current_time: f64) -> Vec<CueCommand> {
        self.tick(current_time);
        self.filtered_ready.clone()
    }

    // ── バリア制御 ──

    /// クリック応答（WaitForClick 解除）
    pub fn resolve_click(&mut self) {
        if self.state == CueQueueState::WaitingForClick {
            self.schedule.notify_barrier_resolved(None);
            self.state = CueQueueState::Playing;
            self.barrier_entered_time = None;
            self.barrier_timeout = None;
            tracing::trace!("[resolve_click] WaitForClick barrier resolved");

            // 解除後に全消費済みなら Completed
            if self.schedule.is_completed() {
                self.state = CueQueueState::Completed;
            }
        }
    }

    /// 選択肢応答（WaitForChoice 解除）。該当 id を返す。
    pub fn resolve_choice(&mut self, choice_id: &str) -> Option<String> {
        if self.state != CueQueueState::WaitingForChoice {
            return None;
        }

        let found = self.pending_choices.iter().any(|c| c.id == choice_id);

        if found {
            self.pending_choices.clear();
            self.schedule.notify_barrier_resolved(Some(choice_id.to_string()));
            self.state = CueQueueState::Playing;
            self.barrier_entered_time = None;
            self.barrier_timeout = None;
            tracing::trace!(choice_id = %choice_id, "[resolve_choice] WaitForChoice barrier resolved");

            // 解除後に全消費済みなら Completed
            if self.schedule.is_completed() {
                self.state = CueQueueState::Completed;
            }

            Some(choice_id.to_string())
        } else {
            None
        }
    }

    /// タイムアウト検査。バリア進入時刻と timeout 値から判定する。
    pub fn check_timeout(&self, current_time: f64) -> bool {
        if matches!(
            self.state,
            CueQueueState::WaitingForClick | CueQueueState::WaitingForChoice
        ) {
            if let (Some(entered), Some(timeout)) = (self.barrier_entered_time, self.barrier_timeout) {
                return current_time - entered >= timeout;
            }
        }
        false
    }

    /// バリアを強制スキップ
    pub fn skip_barrier(&mut self) {
        if matches!(
            self.state,
            CueQueueState::WaitingForClick | CueQueueState::WaitingForChoice
        ) {
            self.pending_choices.clear();
            self.schedule.notify_barrier_resolved(None);
            self.state = CueQueueState::Playing;
            self.barrier_entered_time = None;
            self.barrier_timeout = None;
            tracing::trace!("[skip_barrier] Barrier force-skipped");

            // スキップ後に全消費済みなら Completed
            if self.schedule.is_completed() {
                self.state = CueQueueState::Completed;
            }
        }
    }

    /// 現在保留中のバリア種別（dola BarrierKind）
    pub fn pending_barrier_kind(&self) -> Option<&BarrierKind> {
        self.schedule.current_barrier()
    }

    // ── 制御 ──

    /// 一時停止
    pub fn pause(&mut self) {
        if self.state == CueQueueState::Playing {
            self.state = CueQueueState::Paused;
        }
    }

    /// 再開
    pub fn resume(&mut self) {
        if self.state == CueQueueState::Paused {
            self.state = CueQueueState::Playing;
        }
    }

    /// キュー全消去
    pub fn clear(&mut self) {
        self.schedule.clear();
        self.pending_choices.clear();
        self.filtered_ready.clear();
        self.state = CueQueueState::Playing;
        self.barrier_entered_time = None;
        self.barrier_timeout = None;
    }

    /// 供給元 Tracker エンティティを設定
    pub fn set_cue_sheet(&mut self, entity: Entity) {
        self.cue_sheet_entity = Some(entity);
    }

    /// 供給元 Tracker エンティティを取得
    pub fn cue_sheet_entity(&self) -> Option<Entity> {
        self.cue_sheet_entity
    }

    // ── 状態照会 ──

    /// 現在の状態
    pub fn state(&self) -> &CueQueueState {
        &self.state
    }

    /// キューが空か（スケジュール内残りエントリ数）
    pub fn is_empty(&self) -> bool {
        self.schedule.remaining() == 0
    }

    /// キュー内の残りエントリ数
    pub fn len(&self) -> usize {
        self.schedule.remaining()
    }

    /// 先積みされた Choice データ
    pub fn pending_choices(&self) -> &[PendingChoice] {
        &self.pending_choices
    }

    /// 再生速度倍率を取得
    pub fn playback_rate(&self) -> f64 {
        self.playback_rate
    }

    /// 再生速度倍率を設定
    pub fn set_playback_rate(&mut self, rate: f64) {
        self.playback_rate = rate;
    }

    /// 内部 TimedSchedule への読み取り参照（テスト・デバッグ用）
    pub fn schedule(&self) -> &TimedSchedule<CueCommand> {
        &self.schedule
    }
}

impl Default for CueQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
