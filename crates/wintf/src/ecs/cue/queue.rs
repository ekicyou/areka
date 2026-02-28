//! CueQueue — 演出指示キューコンポーネント。
//!
//! 各演者エンティティが保持する時刻付き演出指示のキュー。
//! 内部は start_time **降順** ソートの Vec<TimedCue>。
//! 消費は末尾からの pop（O(1)）で行い、先頭への挿入移動を回避する。

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;

use super::command::CueCommand;
use super::error::CueSystemError;

/// 絶対時刻に変換済みの消費可能コマンド。
/// dispatch 時に `cue.start_time + sheet_start_time` で生成される。
#[derive(Clone, Debug)]
pub struct TimedCue {
    /// 世界絶対時刻（秒）
    pub start_time: f64,
    /// 演出コマンド（actor 情報は分配済みのため不要）
    pub command: CueCommand,
}

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
    /// クリック待ちバリア中
    WaitingForClick,
    /// 選択肢バリア中
    WaitingForChoice,
    /// エラー発生
    Error(CueSystemError),
    /// 全コマンド消費完了
    Completed,
}

/// バリア種別
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BarrierKind {
    /// 選択肢バリア
    Choice,
    /// クリック待ちバリア
    Click,
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

/// CueQueue 内部のバリア状態管理
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct BarrierState {
    /// 最初に有効応答が到達した時点の BarrierResponse
    pub(crate) first_valid: Option<BarrierResponse>,
    /// バリア種別
    pub(crate) kind: BarrierKind,
    /// バリア開始時刻
    pub(crate) start_time: f64,
    /// タイムアウト値（秒）
    pub(crate) timeout: Option<f64>,
}

/// 各演者エンティティが保持する時刻付き演出指示のキュー。
///
/// CueSheet の配送（dispatch）により TimedCue が追加され、
/// 消費者システムが `pop_ready()` で時刻到達済みコマンドを取得する。
///
/// 内部は start_time **降順** ソートの Vec<TimedCue>。
/// 消費は末尾からの pop（O(1)）で行い、先頭への挿入移動を回避する。
#[derive(Component, Debug)]
#[component(storage = "SparseSet")]
pub struct CueQueue {
    /// 降順ソートされた TimedCue のキュー
    queue: Vec<TimedCue>,
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
    /// 現在アクティブなバリア
    barrier_state: Option<BarrierState>,
}

impl CueQueue {
    /// 新しい CueQueue を生成する。
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            state: CueQueueState::Playing,
            playback_rate: 1.0,
            capacity: None,
            pending_choices: Vec::new(),
            cue_sheet_entity: None,
            barrier_state: None,
        }
    }

    /// キャパシティ指定で CueQueue を生成する。
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: Vec::with_capacity(capacity),
            state: CueQueueState::Playing,
            playback_rate: 1.0,
            capacity: Some(capacity),
            pending_choices: Vec::new(),
            cue_sheet_entity: None,
            barrier_state: None,
        }
    }

    // ── 追加 ──

    /// TimedCue を降順ソート維持で挿入（O(log n) binary search + O(n) shift）
    pub fn push_sorted(&mut self, cue: TimedCue) -> Result<(), CueSystemError> {
        if let Some(cap) = self.capacity {
            if self.queue.len() >= cap {
                return Err(CueSystemError::CapacityExceeded { capacity: cap });
            }
        }

        // 降順ソート: start_time が大きいほど先頭（index 0）、小さいほど末尾
        let pos = self
            .queue
            .binary_search_by(|existing| {
                // 降順: existing > cue なら Less（先に並ぶべき）
                existing
                    .start_time
                    .partial_cmp(&cue.start_time)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .reverse()
            })
            .unwrap_or_else(|pos| pos);

        self.queue.insert(pos, cue);

        // Completed 状態だった場合は Playing に戻す
        if self.state == CueQueueState::Completed {
            self.state = CueQueueState::Playing;
        }

        Ok(())
    }

    /// 複数の TimedCue を一括追加（内部で再ソート）
    pub fn extend_sorted(
        &mut self,
        cues: impl IntoIterator<Item = TimedCue>,
    ) -> Result<(), CueSystemError> {
        let cues: Vec<TimedCue> = cues.into_iter().collect();

        if let Some(cap) = self.capacity {
            if self.queue.len() + cues.len() > cap {
                return Err(CueSystemError::CapacityExceeded { capacity: cap });
            }
        }

        self.queue.extend(cues);
        // 降順ソート（安定）
        self.queue.sort_by(|a, b| {
            b.start_time
                .partial_cmp(&a.start_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Completed 状態だった場合は Playing に戻す
        if self.state == CueQueueState::Completed {
            self.state = CueQueueState::Playing;
        }

        Ok(())
    }

    // ── 消費 ──

    /// current_time に到達した全コマンドを返却（O(1) per pop）
    ///
    /// - バリア中・一時停止中・完了済み・エラー時は空 Vec を返す
    /// - Choice コマンドは pending_choices に蓄積（返却しない）
    /// - WaitForChoice 到達時: pending_choices > 0 → ブロック、== 0 → Error
    /// - WaitForClick 到達時: ブロック
    pub fn pop_ready(&mut self, current_time: f64) -> Vec<CueCommand> {
        // バリア中・Paused・Completed・Error は消費しない
        if !matches!(self.state, CueQueueState::Playing) {
            return Vec::new();
        }

        let mut result = Vec::new();

        while let Some(last) = self.queue.last() {
            if last.start_time > current_time {
                break;
            }

            let timed_cue = self.queue.pop().unwrap();
            let cmd = timed_cue.command;

            match &cmd {
                CueCommand::Choice { id, text } => {
                    tracing::trace!(id = %id, text = %text, "[pop_ready] Choice accumulated");
                    self.pending_choices.push(PendingChoice {
                        id: id.clone(),
                        text: text.clone(),
                    });
                    // Choice は pending_choices に蓄積し返却しない
                    continue;
                }
                CueCommand::WaitForChoice { timeout } => {
                    if self.pending_choices.is_empty() {
                        tracing::error!(
                            "[pop_ready] WaitForChoice with no preceding Choice commands"
                        );
                        self.state = CueQueueState::Error(CueSystemError::EmptyChoiceBarrier {
                            actor: "unknown".to_string(),
                        });
                        return result;
                    }
                    tracing::trace!(
                        choices = self.pending_choices.len(),
                        "[pop_ready] WaitForChoice barrier entered"
                    );
                    self.state = CueQueueState::WaitingForChoice;
                    self.barrier_state = Some(BarrierState {
                        first_valid: None,
                        kind: BarrierKind::Choice,
                        start_time: current_time,
                        timeout: *timeout,
                    });
                    return result;
                }
                CueCommand::WaitForClick { timeout } => {
                    tracing::trace!("[pop_ready] WaitForClick barrier entered");
                    self.state = CueQueueState::WaitingForClick;
                    self.barrier_state = Some(BarrierState {
                        first_valid: None,
                        kind: BarrierKind::Click,
                        start_time: current_time,
                        timeout: *timeout,
                    });
                    return result;
                }
                _ => {
                    tracing::trace!(command = ?cmd, "[pop_ready] Command consumed");
                    result.push(cmd);
                }
            }
        }

        // 全コマンド消費完了チェック
        if self.queue.is_empty() && self.state == CueQueueState::Playing {
            self.state = CueQueueState::Completed;
        }

        result
    }

    /// 先頭（次に消費される）要素の参照（末尾 = 最小 start_time）
    pub fn peek(&self) -> Option<&TimedCue> {
        self.queue.last()
    }

    // ── バリア制御 ──

    /// クリック応答（WaitForClick 解除）
    pub fn resolve_click(&mut self) {
        if self.state == CueQueueState::WaitingForClick {
            self.state = CueQueueState::Playing;
            self.barrier_state = None;
            tracing::trace!("[resolve_click] WaitForClick barrier resolved");

            // 解除後に全消費済みなら Completed
            if self.queue.is_empty() {
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
            self.state = CueQueueState::Playing;
            self.barrier_state = None;
            tracing::trace!(choice_id = %choice_id, "[resolve_choice] WaitForChoice barrier resolved");

            // 解除後に全消費済みなら Completed
            if self.queue.is_empty() {
                self.state = CueQueueState::Completed;
            }

            Some(choice_id.to_string())
        } else {
            None
        }
    }

    /// タイムアウト検査。タイムアウト時は true を返す。
    pub fn check_timeout(&self, current_time: f64) -> bool {
        if let Some(ref barrier) = self.barrier_state {
            if let Some(timeout) = barrier.timeout {
                if current_time - barrier.start_time >= timeout {
                    return true;
                }
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
            self.state = CueQueueState::Playing;
            self.barrier_state = None;
            tracing::trace!("[skip_barrier] Barrier force-skipped");

            // スキップ後に全消費済みなら Completed
            if self.queue.is_empty() {
                self.state = CueQueueState::Completed;
            }
        }
    }

    /// 現在保留中のバリア種別
    pub fn pending_barrier_kind(&self) -> Option<&BarrierKind> {
        self.barrier_state.as_ref().map(|b| &b.kind)
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
        self.queue.clear();
        self.pending_choices.clear();
        self.barrier_state = None;
        self.state = CueQueueState::Playing;
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

    /// キューが空か
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// キュー内のコマンド数
    pub fn len(&self) -> usize {
        self.queue.len()
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
}

impl Default for CueQueue {
    fn default() -> Self {
        Self::new()
    }
}
