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
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    // ── EntityRef ラウンドトリップ（push/pop 境界の bit 変換） ──

    /// `push_entity_command` が `entity.to_bits()` と一致する EntityRef を挿入し、
    /// `resolve_entity_ref` が同一 Entity を復元する（往復恒等）。
    #[test]
    fn push_entity_command_inserts_entity_ref_with_matching_bits() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();

        let mut queue = CueQueue::new();
        queue.push_entity_command(0.0, entity).unwrap();

        let cmds = queue.pop_ready(1.0);
        assert_eq!(cmds.len(), 1);
        // 挿入されたコマンドは EntityRef(entity.to_bits())
        match &cmds[0] {
            CueCommand::EntityRef(bits) => assert_eq!(*bits, entity.to_bits()),
            other => panic!("Expected EntityRef, got {other:?}"),
        }
        // 復元は同一 Entity
        assert_eq!(CueQueue::resolve_entity_ref(&cmds[0]), Some(entity));
    }

    /// 複数の異なる Entity が EntityRef ラウンドトリップで取り違えなく復元される。
    #[test]
    fn distinct_entities_roundtrip_distinctly() {
        let mut world = World::new();
        let e1 = world.spawn_empty().id();
        let e2 = world.spawn_empty().id();
        let e3 = world.spawn_empty().id();
        assert!(e1 != e2 && e2 != e3 && e1 != e3);

        let mut queue = CueQueue::new();
        // 同一時刻に 3 体投入（offset 0.0）。pop は降順 entries の末尾から FIFO 順。
        queue.push_entity_command(0.0, e1).unwrap();
        queue.push_entity_command(0.0, e2).unwrap();
        queue.push_entity_command(0.0, e3).unwrap();

        let cmds = queue.pop_ready(0.0);
        assert_eq!(cmds.len(), 3);

        let restored: Vec<Entity> = cmds
            .iter()
            .map(|c| CueQueue::resolve_entity_ref(c).expect("EntityRef"))
            .collect();
        // 集合として 3 体が一致（順序は TimedSchedule の同時刻挿入順 = FIFO）
        assert!(restored.contains(&e1));
        assert!(restored.contains(&e2));
        assert!(restored.contains(&e3));
        // 取り違えなし: 復元集合に重複なし
        assert!(e1 != e2);
        assert_eq!(restored[0], e1, "same-offset insert preserves FIFO order");
        assert_eq!(restored[1], e2);
        assert_eq!(restored[2], e3);
    }

    /// `resolve_entity_ref` は EntityRef 以外のコマンドに対して None を返す。
    #[test]
    fn resolve_entity_ref_returns_none_for_non_entity_ref() {
        assert_eq!(
            CueQueue::resolve_entity_ref(&CueCommand::Text("hi".into())),
            None
        );
        assert_eq!(CueQueue::resolve_entity_ref(&CueCommand::Clear), None);
        assert_eq!(
            CueQueue::resolve_entity_ref(&CueCommand::Emote { key: "smile".into() }),
            None
        );
    }

    /// W8-V 脆弱性点検: `resolve_entity_ref` は `Entity::from_bits()`（非フォールバック版）
    /// を用いるため、不正ビット（`to_bits()` 由来でない値）に対して **panic する**。
    /// `CueCommand::EntityRef(u64)` は `Deserialize` 可能（dola cue/command.rs:117-128）で
    /// 外部 CueSheet 由来の任意 u64 を運び得るため、これは外部入力到達可能な panic 経路である。
    /// 下位 32bit（index ワード）が 0 のビット列は `EntityIndex::try_from_bits`（`NonZero::new`
    /// 検査・bevy_ecs entity/mod.rs:201-208）が拒否し None → `Entity::from_bits` が panic する
    /// （entity/mod.rs:576-581。`NonMaxU32` の transmute 表現により raw=0 が無効インデックスに対応。
    /// W8-V が一時 probe で実測: bits=0x0000_0001_0000_0000 → panic、0x0000_0000_FFFF_FFFF → 正常）。
    /// 本テストは **現状の panic 挙動を固定**する特性化（回帰検知）であり、堅牢化
    /// （from_bits→try_from_bits で None 縮退）は観測挙動を変えるため本ループ非適用 = proposals P69。
    /// P69 適用時は本テストが RED 化し、`#[should_panic]` の除去 + None 期待への更新で追随する。
    #[test]
    #[should_panic(expected = "invalid bits")]
    fn resolve_entity_ref_panics_on_malformed_bits() {
        // 下位 32bit(index ワード)=0 は EntityIndex として不正 → from_bits が panic。
        // generation=1, index=0 のビット列（to_bits() 由来でない外部 u64 を模す）。
        let malformed = CueCommand::EntityRef(0x0000_0001_0000_0000);
        let _ = CueQueue::resolve_entity_ref(&malformed);
    }

    // ── 2 フェーズ API の tick 冪等性（TimedSchedule 委譲の観測） ──

    /// 未完了キュー（後続エントリ残あり）での同一時刻 tick 再呼び出しは
    /// ready バッファを保持する（TimedSchedule の冪等性ガードを CueQueue 経由で観測）。
    /// 1.0 で a が ready・2.0 の b が未到達のため state は Playing のまま。
    #[test]
    fn tick_is_idempotent_at_same_time_when_not_completed() {
        let mut queue = CueQueue::new();
        queue
            .insert(Entry::Payload(1.0, CueCommand::Text("a".into())))
            .unwrap();
        // 後続エントリ（未到達）を残すことで tick(1.0) 後も Completed にならない
        queue
            .insert(Entry::Payload(2.0, CueCommand::Text("b".into())))
            .unwrap();

        queue.tick(1.0);
        assert_eq!(queue.ready().len(), 1);
        assert_eq!(*queue.state(), CueQueueState::Playing);

        // 同一時刻で再 tick → ready は変わらず 1 件（冪等。二重消費しない）
        queue.tick(1.0);
        assert_eq!(queue.ready().len(), 1);
        assert!(matches!(&queue.ready()[0], CueCommand::Text(t) if t == "a"));
    }

    /// 全消費で Completed に遷移したキューへの再 tick は ready をクリアする
    /// （Playing 以外は line 212 で early-return しバッファを空にする挙動）。
    /// 完了後の tick が ready を保持しないことを特性化する（冪等ではない側）。
    #[test]
    fn tick_after_completion_clears_ready() {
        let mut queue = CueQueue::new();
        queue
            .insert(Entry::Payload(1.0, CueCommand::Text("a".into())))
            .unwrap();

        queue.tick(1.0);
        assert_eq!(queue.ready().len(), 1);
        // 唯一のエントリ消費で Completed
        assert_eq!(*queue.state(), CueQueueState::Completed);

        // Completed 状態での再 tick は ready をクリア（早期 return）
        queue.tick(1.0);
        assert!(queue.ready().is_empty());
        assert_eq!(*queue.state(), CueQueueState::Completed);
    }

    /// tick で時刻を進めると前回 ready は新しい到達コマンドで置き換わる。
    #[test]
    fn tick_advancing_time_replaces_ready_buffer() {
        let mut queue = CueQueue::new();
        queue
            .insert(Entry::Payload(1.0, CueCommand::Text("a".into())))
            .unwrap();
        queue
            .insert(Entry::Payload(2.0, CueCommand::Text("b".into())))
            .unwrap();

        queue.tick(1.0);
        assert_eq!(queue.ready().len(), 1);
        assert!(matches!(&queue.ready()[0], CueCommand::Text(t) if t == "a"));

        // 次の時刻 → ready は b に置き換わる（a は再返却されない）
        queue.tick(2.0);
        assert_eq!(queue.ready().len(), 1);
        assert!(matches!(&queue.ready()[0], CueCommand::Text(t) if t == "b"));
    }

    // ── capacity 境界（insert / extend_entries） ──

    /// `insert` は capacity ちょうどまで許容し、超過で CapacityExceeded を返す。
    #[test]
    fn insert_allows_up_to_capacity_then_errors() {
        let mut queue = CueQueue::with_capacity(2);
        assert!(queue.insert(Entry::Payload(1.0, CueCommand::Clear)).is_ok());
        assert!(queue.insert(Entry::Payload(2.0, CueCommand::Clear)).is_ok());
        // capacity == len で超過
        match queue.insert(Entry::Payload(3.0, CueCommand::Clear)) {
            Err(CueSystemError::CapacityExceeded { capacity }) => assert_eq!(capacity, 2),
            other => panic!("Expected CapacityExceeded, got {other:?}"),
        }
        assert_eq!(queue.len(), 2);
    }

    /// `extend_entries` は投入後に capacity を超える一括挿入を拒否し、キューを変更しない。
    #[test]
    fn extend_entries_rejects_batch_exceeding_capacity_atomically() {
        let mut queue = CueQueue::with_capacity(2);
        let result = queue.extend_entries(vec![
            Entry::Payload(1.0, CueCommand::Clear),
            Entry::Payload(2.0, CueCommand::Clear),
            Entry::Payload(3.0, CueCommand::Clear),
        ]);
        match result {
            Err(CueSystemError::CapacityExceeded { capacity }) => assert_eq!(capacity, 2),
            other => panic!("Expected CapacityExceeded, got {other:?}"),
        }
        // 一括拒否: 1 件も挿入されていない
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    /// `extend_entries` は capacity ちょうどの一括挿入を許容する。
    #[test]
    fn extend_entries_allows_batch_filling_capacity_exactly() {
        let mut queue = CueQueue::with_capacity(3);
        queue
            .extend_entries(vec![
                Entry::Payload(1.0, CueCommand::Clear),
                Entry::Payload(2.0, CueCommand::Clear),
                Entry::Payload(3.0, CueCommand::Clear),
            ])
            .unwrap();
        assert_eq!(queue.len(), 3);
    }

    // ── Completed → Playing 再活性化 ──

    /// Completed 状態の CueQueue へ insert すると Playing に復帰する。
    #[test]
    fn insert_reactivates_completed_queue_to_playing() {
        let mut queue = CueQueue::new();
        queue
            .insert(Entry::Payload(1.0, CueCommand::Clear))
            .unwrap();
        // 全消費 → Completed
        queue.pop_ready(2.0);
        assert_eq!(*queue.state(), CueQueueState::Completed);

        // insert で Playing 復帰
        queue
            .insert(Entry::Payload(3.0, CueCommand::Clear))
            .unwrap();
        assert_eq!(*queue.state(), CueQueueState::Playing);
    }

    /// Completed 状態の CueQueue へ extend_entries すると Playing に復帰する。
    #[test]
    fn extend_entries_reactivates_completed_queue_to_playing() {
        let mut queue = CueQueue::new();
        queue
            .insert(Entry::Payload(1.0, CueCommand::Clear))
            .unwrap();
        queue.pop_ready(2.0);
        assert_eq!(*queue.state(), CueQueueState::Completed);

        queue
            .extend_entries(vec![Entry::Payload(3.0, CueCommand::Clear)])
            .unwrap();
        assert_eq!(*queue.state(), CueQueueState::Playing);
    }

    // ── 再生速度・補助アクセサ ──

    /// `playback_rate` の既定は 1.0、set で更新される。
    #[test]
    fn playback_rate_default_and_set() {
        let mut queue = CueQueue::new();
        assert_eq!(queue.playback_rate(), 1.0);
        queue.set_playback_rate(2.5);
        assert_eq!(queue.playback_rate(), 2.5);
    }

    /// `set_cue_sheet` / `cue_sheet_entity` の往復（供給元 Tracker 参照）。
    #[test]
    fn cue_sheet_entity_set_and_get() {
        let mut world = World::new();
        let tracker = world.spawn_empty().id();

        let mut queue = CueQueue::new();
        assert_eq!(queue.cue_sheet_entity(), None);
        queue.set_cue_sheet(tracker);
        assert_eq!(queue.cue_sheet_entity(), Some(tracker));
    }

    /// `with_capacity` 生成キューも既定状態は Playing・空。
    #[test]
    fn with_capacity_initial_state_is_playing_and_empty() {
        let queue = CueQueue::with_capacity(5);
        assert_eq!(*queue.state(), CueQueueState::Playing);
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.playback_rate(), 1.0);
    }

    /// `schedule()` アクセサは内部 TimedSchedule の残数を反映する。
    #[test]
    fn schedule_accessor_reflects_remaining() {
        let mut queue = CueQueue::new();
        assert_eq!(queue.schedule().remaining(), 0);
        queue
            .insert(Entry::Payload(1.0, CueCommand::Clear))
            .unwrap();
        assert_eq!(queue.schedule().remaining(), 1);
    }

    // ── reset_schedule の状態リセット ──

    /// `reset_schedule` はエントリ・選択肢・ready バッファ・バリア状態を全リセットし Playing へ。
    #[test]
    fn reset_schedule_clears_all_state() {
        let mut queue = CueQueue::new();
        queue
            .extend_entries(vec![
                Entry::Payload(
                    1.0,
                    CueCommand::Choice {
                        id: "x".into(),
                        text: "X".into(),
                    },
                ),
                Entry::Barrier(2.0, BarrierKind::WaitForChoice { timeout: None }),
            ])
            .unwrap();
        queue.tick(1.5); // Choice 蓄積
        queue.tick(2.5); // WaitForChoice バリア
        assert_eq!(*queue.state(), CueQueueState::WaitingForChoice);
        assert_eq!(queue.pending_choices().len(), 1);

        queue.reset_schedule(10.0);
        assert_eq!(*queue.state(), CueQueueState::Playing);
        assert!(queue.is_empty());
        assert!(queue.pending_choices().is_empty());
        assert!(queue.ready().is_empty());
        assert!(!queue.check_timeout(1000.0));
    }
}
