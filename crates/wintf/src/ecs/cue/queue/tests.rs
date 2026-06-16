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
