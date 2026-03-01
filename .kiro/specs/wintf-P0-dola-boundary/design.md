# Technical Design Document

| 項目               | 内容                                                  |
| ------------------ | ----------------------------------------------------- |
| **Document Title** | dola ランタイム責務境界定義（dola-boundary）技術設計書 |
| **Version**        | 1.0                                                   |
| **Date**           | 2026-03-01                                            |
| **Requirements**   | v4.2                                                  |
| **Status**         | 📐 Generated                                         |

---

## Overview

dola クレートに離散コマンドスケジューリング基盤（`cue/` モジュール）を新設し、wintf の cue モジュールから ECS 非依存の演出ロジック（`CueCommand`、ドメイン型、時刻スケジューリング）を移管する。同時に `DolaRuntime` の API を `tick/last_result` に分離し、wintf に `DolaAnimator` ECS Component を新設する。

本設計は dola の「アニメーション実現のための汎用道具集」という位置づけを具現化し、2 エンジン（連続値アニメ / 離散コマンド配信）の独立性を型レベルで保証する。

### Goals

- dola に `cue/` モジュールを新設し `TimedSchedule<T>`, `CueSheet`, `CueCommand`, ドメイン型を提供する
- `DolaRuntime` API を `tick()` + `last_result()` に分離し、`TimedSchedule` の `advance()` + `ready()` と対称にする
- wintf に `DolaAnimator` Component を新設し、エンティティごとの独立アニメーション状態を実現する
- wintf `cue/` モジュールから誤配置 DolaRuntime コードを除去する
- 移管した型は re-export により wintf 側の後方互換を維持する

### Non-Goals

- `CueQueue` の完全再設計（Phase 3 で実施、本仕様では方針決定のみ）
- `UpdateResult` の具体的消費実装（balloon06-text-effects に委譲）
- pasta DSL パーサーの実装（インターフェース設計の考慮のみ）
- dola ランタイム内部リファクタリング（`tick/last_result` 分離に必要な範囲のみ）

---

## Architecture

### Existing Architecture Analysis

現行は wintf `ecs/cue/` モジュール内に全演出ロジック（型定義・スケジューリング・ECS 統合・DolaRuntime ラッパー）が混在している。

**問題点**:
1. `DolaRuntime` が `#[derive(Resource)]` として cue モジュールに誤配置（cue パイプライン未消費）
2. `CueCommand` が `bevy_ecs::entity::Entity` に直接依存（dola からの独立使用不可）
3. `CueQueue` 内に `TimedSchedule` 相当のジェネリックロジックが埋没
4. ドメイン型（`ActorKey`, `CueTarget`, `EntityKey`, `Cue`）が ECS レイヤーに配置

### Architecture Pattern & Boundary Map

改修後のクレート境界とモジュール構成:

```mermaid
graph TB
    subgraph dola_crate[dola crate - ECS 非依存]
        subgraph dola_cue[cue module - NEW]
            TS[TimedSchedule T]
            BK[BarrierKind]
            CC[CueCommand 9var]
            CS[CueSheet]
            CF[compile_sheet]
            DT[ActorKey CueTarget EntityKey Cue]
        end
        subgraph dola_runtime[runtime module - EXISTING]
            DR[DolaRuntime]
            UR[UpdateResult]
        end
        DV[DynamicValue]
    end

    subgraph wintf_crate[wintf crate - ECS 依存]
        subgraph wintf_dola[ecs/dola module - NEW]
            DA[DolaAnimator Component]
            TDA[tick_dola_animators System]
        end
        subgraph wintf_cue[ecs/cue module - REFACTORED]
            CQ[CueQueue Component]
            CST[CueSheetTracker]
            ER[EntityRegistry Resource]
            DP[dispatch system]
        end
    end

    DA -->|owns| DR
    TDA -->|Query mut| DA
    CQ -->|wraps| TS
    CC -->|uses| DV
    DP -->|uses| CF
    CQ -->|re-export| CC
```

**境界原則**: dola → wintf の一方向依存。dola は `bevy_ecs` を知らない。wintf は dola 型を re-export し ECS Component/System でラップする。

---

## Technology Stack

| レイヤー | ツール/ライブラリ | バージョン | 役割 |
|---------|-----------------|-----------|------|
| アニメーションエンジン | dola | workspace | 連続値 + 離散コマンドの 2 エンジン基盤 |
| ECS フレームワーク | bevy_ecs | 0.18.0 | Component/System/Query による型安全な排他アクセス |
| シリアライゼーション | serde | 1 | CueCommand/CueSheet/ドメイン型の JSON/TOML/YAML 対応 |
| エラーハンドリング | thiserror | 2 | 構造化エラー enum 定義 |

**変更なし**: 新規依存の追加なし。既存の dola 依存（serde, thiserror）と wintf 依存（bevy_ecs, dola）の範囲内。

---

## System Flows

### Flow 1: CueSheet 配送フロー（改修後）

```mermaid
sequenceDiagram
    participant App as areka/pasta
    participant CS as dola CueSheet
    participant CF as dola compile_sheet
    participant DP as wintf dispatch
    participant ER as wintf EntityRegistry
    participant CQ as wintf CueQueue
    participant TS as dola TimedSchedule

    App->>CS: CueSheet::new(cues)
    App->>DP: PendingCueSheet spawn
    DP->>CF: compile_sheet(sheet, start_time)
    CF-->>DP: Vec of CompiledCue(abs_time, actor, command)
    DP->>ER: resolve actor to Entity
    DP->>CQ: push_compiled_cue(abs_time, command)
    CQ->>TS: insert(Entry::Payload(time, cmd))
```

### Flow 2: フレーム更新フロー（DolaAnimator + CueQueue 消費）

```mermaid
sequenceDiagram
    participant FT as FrameTime
    participant TDA as tick_dola_animators
    participant DA as DolaAnimator
    participant DR as DolaRuntime
    participant Consumer as 消費者 System
    participant CQ as CueQueue
    participant TS as TimedSchedule

    Note over FT: フレーム開始
    FT->>TDA: Res FrameTime
    TDA->>DA: Query mut DolaAnimator
    DA->>DR: tick(FrameTime.0)
    DR-->>DA: 内部に UpdateResult 格納
    Consumer->>DA: Query ref DolaAnimator
    DA->>DR: last_result()
    DR-->>Consumer: UpdateResult ref
    Consumer->>CQ: Query mut CueQueue
    CQ->>TS: advance(FrameTime.0)
    TS-->>CQ: ready() -> commands slice
    Consumer-->>Consumer: コマンド処理
```

---

## Requirements Traceability

| 要件 | サマリー | コンポーネント | インターフェース | フロー |
|------|---------|--------------|----------------|--------|
| 1.1 | bevy_ecs 非依存 | dola::cue::* | — | — |
| 1.2 | TimedSchedule API | TimedSchedule | advance/ready | Flow 2 |
| 1.3 | バリア管理 | TimedSchedule | current_barrier/resolve_barrier | Flow 2 |
| 1.4 | CueSheet + compile_sheet | CueSheet, compile_sheet | compile_sheet() | Flow 1 |
| 1.5 | CueCommand 9 バリアント | CueCommand | is_routing_command() | Flow 1 |
| 1.6 | ドメイン型 | ActorKey, CueTarget, EntityKey, Cue | — | Flow 1 |
| 1.7 | tick/last_result 分離 | DolaRuntime | tick(), last_result() | Flow 2 |
| 1.8 | 連続値/離散の責務分離 | cue/ vs runtime/ | — | — |
| 1.9 | pasta DSL 互換設計 | CueSheet | — | — |
| 2.1 | DolaAnimator Component | DolaAnimator | new(), tick(), last_result() | Flow 2 |
| 2.2 | tick_dola_animators | tick_dola_animators | System signature | Flow 2 |
| 2.3 | 消費者パターン | — | Query ref + .after() | Flow 2 |
| 2.4 | 配置先モジュール | ecs/dola/ | — | — |
| 2.5 | balloon06 整合 | — | （文書化） | — |
| 3.1 | DolaRuntime 除去 | cue/runtime.rs 削除 | — | — |
| 3.2 | CueQueue リファクタリング方針 | CueQueue | — | — |
| 3.3 | TimedSchedule 委譲 | CueQueue | inner: TimedSchedule | Flow 2 |
| 3.4 | u64 ↔ Entity 変換 | CueQueue | push_entity_ref/pop 境界 | Flow 1 |
| 3.5 | re-export 後方互換 | cue/command.rs | type alias | — |
| 3.6 | 移行戦略 | — | （文書化） | — |
| 4.1-4.3 | UpdateResult 消費方針 | — | （文書化、balloon06 委譲） | — |
| 5.1-5.4 | ドキュメント整合性 | — | （文書化） | — |
| NFR-1 | 後方互換性 | 全コンポーネント | テスト 920+ パス | — |

---

## Components and Interfaces

### Component Summary

| コンポーネント | ドメイン | Intent | 要件カバレッジ | 主要依存 |
|--------------|---------|--------|---------------|---------|
| `dola::cue::TimedSchedule<T>` | dola / 離散配信 | 汎用絶対時刻配信エンジン | 1.2, 1.3 | — |
| `dola::cue::CueCommand` | dola / 演出コマンド | 型安全な演出コマンド enum | 1.5 | DynamicValue |
| `dola::cue::CueSheet` | dola / 演出台本 | 相対時刻コマンド列 + compile | 1.4, 1.9 | CueCommand |
| `dola::cue::{domain types}` | dola / 演出ドメイン | ActorKey, CueTarget, EntityKey, Cue | 1.6 | CueCommand |
| `dola::runtime::DolaRuntime` | dola / 連続値エンジン | tick/last_result API 分離 | 1.7, 1.8 | — |
| `wintf::ecs::dola::DolaAnimator` | wintf / ECS 統合 | DolaRuntime の ECS Component ラッパー | 2.1, 2.2, 2.3 | DolaRuntime, bevy_ecs |
| `wintf::ecs::cue::CueQueue` | wintf / ECS 統合 | TimedSchedule の ECS Component ラッパー（リファクタリング方針） | 3.2, 3.3, 3.4 | TimedSchedule, bevy_ecs |

---

### Component: `dola::cue::TimedSchedule<T>`

**Intent**: 0 ベース相対オフセットの汎用配信エンジン。`Entry<T>` の型レベル 3 種分離により Payload / Barrier / Routing を区別し、2 フェーズ API（`advance` / `ready`）で消費者に時刻到達済みコマンドを提供する。絶対時刻との変換は new(start_time) で担当。

**Requirements**: 1.2, 1.3

**Dependencies**:

| 依存先 | 方向 | 重要度 | 説明 |
|--------|------|--------|------|
| `T: Clone + Debug` | Inbound (型制約) | P0 | ジェネリック型パラメータの最小制約 |

**Contracts**:

##### Service Interface

```rust
/// 0 ベース相対オフセットの汎用配信エンジン。
/// Entry<T> により Payload / Barrier / Routing を型レベルで 3 種分離する。
pub struct TimedSchedule<T> {
    // ── 内部フィールド ──
    // start_time: f64         — 絶対時刻での開始時刻
    // entries: Vec<Entry<T>>  — 降順ソート（0 ベース相対オフセット）
    // ready_buffer: Vec<T>    — advance() で収集した Payload
    // current_barrier: Option<BarrierKind>
}

/// エントリの型レベル 3 種分離
/// f64 = スケジュール開始からの相対オフセット（0 ベース）
pub enum Entry<T> {
    /// 時刻付きデータペイロード（実行すべきコマンド）
    Payload(f64, T),
    /// 時刻付きバリア（進行停止点、TimedSchedule が消費）
    Barrier(f64, BarrierKind),
    /// 時刻付きルーティング（配送制御、CueQueue 層が消費）
    Routing(f64, RoutingCommand),
}

/// バリア種別（3 種）
pub enum BarrierKind {
    /// クリック/キー入力待ち
    WaitForInput { timeout: Option<f64> },
    /// 選択肢待ち
    WaitForChoice { timeout: Option<f64> },
    /// 指定時間経過待ち
    Timeout { duration: f64 },
}

/// ルーティングコマンド（3 種）
pub enum RoutingCommand {
    /// スロット追加（既存ルーティングを維持したまま追加先を登録）
    RouteAdd { target: CueTarget, to: EntityKey },
    /// スロット切替（既存ルーティングを上書き）
    RouteSwitch { target: CueTarget, to: EntityKey },
    /// スロット除去（指定ターゲットのルーティングを削除）
    RouteRemove { target: CueTarget },
}

impl<T: Clone + Debug> TimedSchedule<T> {
    /// 絶対時刻 start_time でスケジュールを構築。
    /// エントリの f64 は 0 ベースの相対オフセット、advance() は絶対時刻を受け取る。
    pub fn new(start_time: f64) -> Self;

    /// エントリを時刻順ソート維持で挿入（0 ベース相対オフセット）
    pub fn insert(&mut self, entry: Entry<T>);

    /// 複数エントリを一括挿入（内部で再ソート）
    pub fn extend(&mut self, entries: impl IntoIterator<Item = Entry<T>>);

    /// 時刻到達済み Payload を内部バッファに収集。
    /// current_time は絶対時刻、内部で start_time との差分で相対オフセットに変換。
    /// バリア/ルーティング到達または末尾到達まで進行。冪等（同一時刻の再呼び出し安全）。
    pub fn advance(&mut self, current_time: f64);

    /// 直前の advance() で収集された Payload のスライスを返す。
    /// 次の advance() 呼び出しまで何度でも参照可能。
    pub fn ready(&self) -> &[T];

    /// 現在停止中のバリア種別を照会（UI 表示用）
    pub fn current_barrier(&self) -> Option<&BarrierKind>;

    /// バリア解除通知（外部イベント駆動）。
    /// WaitForInput: choice_id = None, WaitForChoice: choice_id = Some(選択ID)
    pub fn notify_barrier_resolved(&mut self, choice_id: Option<String>);

    /// 時刻到達済みルーティングコマンドを取得（CueQueue 層が消費）
    pub fn next_routing(&mut self) -> Option<RoutingCommand>;

    /// 残エントリ数
    pub fn remaining(&self) -> usize;

    /// 全エントリが消費済みか
    pub fn is_completed(&self) -> bool;

    /// 全エントリをクリア
    pub fn clear(&mut self);
}
```

##### State Management

- **状態モデル**: `Idle` → `Advancing`（advance 中）→ `Blocked`（バリア到達）→ `Completed`（全消費）の内部状態遷移。外部からは `current_barrier()` と `is_completed()` で照会
- **冪等性**: `advance(t)` を同一 `t` で複数回呼び出しても `ready_buffer` は変化しない
- **消費型**: 一度 `advance()` で収集された `Payload` は内部キューから除去される（不可逆）
- **時刻変換**: `new(start_time)` で絶対時刻を保持、`advance(current_time)` で `current_time - start_time` を相対オフセットに変換して内部処理

**Implementation Notes**

- **Integration**: wintf `CueQueue` が `TimedSchedule<CueCommand>` を内包する構成。`CueQueue::pop_ready()` は内部で `schedule.advance()` → `schedule.ready()` を呼び出す。バリア解除は wintf のイベントハンドラが `notify_barrier_resolved()` を呼び出す
- **新CueSheet投入**: 既存スケジュールは全破棄（`clear()` + `new(start_time)` + `extend()`）。バリア中でも強制的に新スケジュールへ切り替え。Actor単位で独立したTimedSchedule
- **同一時刻の処理**:
  - **Payload**: キーフレームベース。`ready()` が `&[T]` スライスで返す。実行順序は不定（並列実行）
  - **Barrier**: シーケンシャル。同一時刻に複数ある場合、最初の1つのみ有効（推奨: 1つのみ記述）
  - **Routing**: シーケンシャル。同一時刻に複数ある場合、配列順（記述順）に `next_routing()` で順次取得
- **タイムアウト判定**: `advance()` で `offset = current_time - start_time` を計算。Barrier到達時、タイムアウト付き（WaitForInput/WaitForChoice）の場合は `timeout_offset = barrier_offset + timeout_duration` と比較し、`offset >= timeout_offset` なら自動解除
- **Validation**: `Entry` の f64 オフセットは非負値を前提（バリデーションは insert 時のデバッグアサートで実施）
- **Risks**: `ready_buffer` の `Vec<T>` アロケーション。実用上 1 フレーム内の到達コマンド数は少数（1〜10）のためパフォーマンス問題なし

---

### Component: `dola::cue::CueCommand`

**Intent**: 型安全な演出コマンド enum。データ系 6 バリアントのみ。バリアは `BarrierKind`、ルーティングは `RoutingCommand` として `Entry` レベルで分離済み。

**Requirements**: 1.5

**Dependencies**:

| 依存先 | 方向 | 重要度 | 説明 |
|--------|------|--------|------|
| `DynamicValue` | Outbound | P0 | `Custom` バリアントのパラメータ型 |

**Contracts**:

##### Service Interface

```rust
/// 演出コマンド（6 バリアント、データ系のみ）
/// Clone + Debug + PartialEq, serde::Serialize + serde::Deserialize
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CueCommand {
    // ── データ（6） ──
    Text(String),
    Clear,
    Emote { key: String },
    Choice { id: String, text: String },
    EntityRef(u64),   // bevy Entity::to_bits() で変換済み
    Custom { command: String, params: DynamicValue },
}
```

---

### Component: `dola::cue::RoutingCommand`

**Intent**: ルーティング制御コマンド enum（3 バリアント）。CueQueue 層が消費し、消費者（`ready()` 利用側）には届かない。

**Requirements**: 1.5a

**Dependencies**:

| 依存先 | 方向 | 重要度 | 説明 |
|--------|------|--------|------|
| `CueTarget` | Outbound | P0 | 配送先スロット指定 |
| `EntityKey` | Outbound | P0 | ルーティングキー識別子 |

**Contracts**:

##### Service Interface

```rust
/// ルーティングコマンド（3 バリアント）
/// Clone + Debug + PartialEq, serde::Serialize + serde::Deserialize
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RoutingCommand {
    /// スロット追加（既存ルーティングを維持したまま追加先を登録）
    RouteAdd { target: CueTarget, to: EntityKey },
    /// スロット切替（既存ルーティングを上書き）
    RouteSwitch { target: CueTarget, to: EntityKey },
    /// スロット除去（指定ターゲットのルーティングを削除）
    RouteRemove { target: CueTarget },
}
```

**Implementation Notes** (CueCommand)

- **Integration**: wintf は `type CueCommand = dola::CueCommand;` で re-export（D4 決定）。`EntityRef(Entity)` → `EntityRef(u64)` の変換は wintf dispatch 層が `Entity::to_bits()` で実施
- **Validation**: `PartialEq` は `DynamicValue` の `PartialEq` 実装に依存。`DynamicValue` が `PartialEq` 未実装の場合、`CueCommand` から `PartialEq` derive を除外し手動実装を検討
- **Risks**: `EntityRef(u64)` の `from_bits()` 復元時に無効な Entity が生成される可能性。wintf 消費者が `Entity::from_bits()` 後に ECS Query で存在確認すること

**Implementation Notes** (RoutingCommand)

- **Integration**: wintf `CueQueue` は `next_routing()` でルーティングコマンドを取得し、内部の `EntityRegistry` を更新する。消費者には `ready()` 経由で届かない
- **Validation**: `EntityKey` の妥当性検証は wintf dispatch 層の責務
- **Risks**: ルーティング変更とコマンド配信のタイミング競合。同一時刻のルーティング変更は次フレームから反映される設計で回避

---

### Component: `dola::cue::CuePayload`

**Intent**: CueSheet 記述時の統一型。コマンド・バリア・ルーティングを同一インターフェースで記述可能にする。

**Requirements**: 1.4, 1.5, 1.5a

**Contracts**:

##### Service Interface

```rust
/// CueSheet 記述時の統合型（3 種）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CuePayload {
    /// データコマンド
    Command(CueCommand),
    /// バリア（進行停止点）
    Barrier(BarrierKind),
    /// ルーティング（配送制御）
    Routing(RoutingCommand),
}

// 自然な記述のための Into 実装
impl From<CueCommand> for CuePayload { /* ... */ }
impl From<BarrierKind> for CuePayload { /* ... */ }
impl From<RoutingCommand> for CuePayload { /* ... */ }

impl CuePayload {
    /// Entry<CueCommand> への変換（compile_sheet 内で使用）
    pub fn into_entry(self, time: f64) -> Entry<CueCommand>;
}
```

**Implementation Notes**

- **Integration**: `CueSheet` の `Cue::payload` フィールドとして使用。`compile_sheet()` が `CuePayload::into_entry()` を呼び出して `Entry<CueCommand>` に変換
- **記述例**:
  ```rust
  let mut cues = vec![
      Cue { actor: actor.clone(), start_time: 0.0, payload: CueCommand::Text("hello".into()).into() },
      Cue { actor: actor.clone(), start_time: 1.0, payload: BarrierKind::WaitForInput { timeout: None }.into() },
      Cue { actor: actor.clone(), start_time: 2.0, payload: RoutingCommand::RouteSwitch { target: CueTarget::Balloon, to: key }.into() },
  ];
  ```
- **Risks**: `Into` trait の多重実装による型推論の曖昧性。実用上は `.into()` で明示的に解決

---

### Component: `dola::cue::CueSheet` + `compile_sheet`

**Intent**: 相対時刻コマンド列（演出台本）と、相対→絶対時刻変換関数。wintf の既存 `CueSheet` を置換する。

**Requirements**: 1.4, 1.9

**Dependencies**:

| 依存先 | 方向 | 重要度 | 説明 |
|--------|------|--------|------|
| `CueCommand` | Outbound | P0 | Cue 内のコマンド型 |
| `ActorKey` | Outbound | P0 | Cue 内のアクター識別子 |

**Contracts**:

##### Service Interface

```rust
/// 相対時刻の演出台本
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CueSheet(Vec<Cue>);

impl CueSheet {
    /// start_time 昇順でソートして構築
    pub fn new(cues: Vec<Cue>) -> Self;
    /// 全 Cue のスライス
    pub fn cues(&self) -> &[Cue];
    /// アクターでフィルタリング
    pub fn filter_by_actor(&self, key: &ActorKey) -> Vec<&Cue>;
    /// 全アクターを重複なしで取得
    pub fn actors(&self) -> Vec<&ActorKey>;
    pub fn is_empty(&self) -> bool;
    pub fn len(&self) -> usize;
}

/// コンパイル済みの 0 ベース相対オフセットエントリ
pub struct CompiledCue {
    pub offset: f64,  // 0 ベース相対オフセット
    pub actor: ActorKey,
    pub entry: Entry<CueCommand>,
}

/// 相対時刻 → 0 ベース相対オフセット正規化。
/// CueSheet::Cue::start_time を最小値 0 基準に正規化し、CuePayload を Entry<CueCommand> に変換:
/// - CuePayload::Command → Entry::Payload
/// - CuePayload::Barrier → Entry::Barrier
/// - CuePayload::Routing → Entry::Routing
pub fn compile_sheet(sheet: &CueSheet) -> Vec<CompiledCue>;
```

**Implementation Notes**

- **Integration**: wintf `dispatch_pending_cue_sheets` が `compile_sheet()` を呼び出し、`CompiledCue` を actor ごとに分配。各 actor の `TimedSchedule::new(current_time)` で絶対時刻スケジュールを構築し、`extend(compiled_entries)` で 0 ベースエントリを投入。Actor → Entity 解決は wintf `EntityRegistry` が担当
- **pasta DSL 互換**: `CueSheet` は `Serialize + Deserialize` を実装し、pasta DSL の出力を JSON/TOML 経由で受け取り可能な設計とする（1.9）

---

### Component: `dola::cue` ドメイン型

**Intent**: 演出パイプラインのドメイン概念を型で表現。ECS 非依存な識別子・配送先・ルーティングキー。

**Requirements**: 1.6

**Contracts** (Summary — 現行 wintf 実装からの移植):

```rust
/// アクター識別子（NewType）
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorKey(String);

/// コマンド配送先スロット
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CueTarget { Shell, Balloon }

/// EntityRegistry のキー識別子
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKey {
    Actor(ActorKey, CueTarget),
    Spot(String),
    Balloon(String),
}

/// 個々の演出指示（相対時刻）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cue {
    pub actor: ActorKey,
    pub start_time: f64,
    pub payload: CuePayload,
}
```

**Implementation Notes**

- **Integration**: wintf は `pub use dola::cue::{ActorKey, CueTarget, EntityKey, Cue};` で re-export
- **移植元**: `crates/wintf/src/ecs/cue/mod.rs` L316-L441 および `crates/wintf/src/ecs/cue/command.rs` L11-L22

---

### Component: `dola::runtime::DolaRuntime` — tick/last_result 分離

**Intent**: 既存 `update()` API を `tick()` + `last_result()` に分離し、`TimedSchedule` の `advance()` + `ready()` と対称にする。

**Requirements**: 1.7, 1.8

**Contracts**:

##### Service Interface

```rust
impl DolaRuntime {
    // ── 既存 API（後方互換） ──

    /// 非推奨: tick() + last_result().cloned() に分離
    #[deprecated(note = "use tick() + last_result() instead")]
    pub fn update(&mut self, current_time: f64) -> UpdateResult;

    // ── 新規 API ──

    /// 内部状態を current_time まで進行し、結果を内部フィールドに格納。
    pub fn tick(&mut self, current_time: f64);

    /// 直前の tick() の結果を読み取り専用で返す。
    /// tick() 未呼び出し時は空の UpdateResult を返す。
    pub fn last_result(&self) -> &UpdateResult;
}
```

**Implementation Notes**

- **Integration**: `DolaRuntime` 構造体に `last_update_result: UpdateResult` フィールド追加。`tick()` は現行 `update()` の本体を実行し結果をフィールドに格納。`last_result()` はフィールド参照を返却
- **後方互換**: `update()` は `tick()` 呼び出し後に `last_result().clone()` を返却。`#[deprecated]` 警告で移行を促す
- **Risks**: `EvaluatedValue::Object(Rc<DynamicValue>)` の `Clone` は `Rc::clone()`（参照カウント増のみ）のためコスト無視可能

---

### Component: `wintf::ecs::dola::DolaAnimator`

**Intent**: `DolaRuntime` を ECS Component として所有するラッパー。`tick_dola_animators` システムによる一括更新で `unsafe impl Send + Sync` の安全性を型レベルで保証する。

**Requirements**: 2.1, 2.2, 2.3, 2.4

**Dependencies**:

| 依存先 | 方向 | 重要度 | 説明 |
|--------|------|--------|------|
| `dola::runtime::DolaRuntime` | Outbound | P0 | 内部所有するアニメーションエンジン |
| `bevy_ecs` | Outbound | P0 | Component derive, Query, System |
| `FrameTime` | Inbound | P0 | フレーム時刻の注入元 |

**Contracts**:

##### Service Interface

```rust
/// DolaRuntime の ECS Component ラッパー。
/// 内部に Rc を含むため unsafe impl Send + Sync。
/// 安全性は tick_dola_animators の Query<&mut> 排他アクセスにより保証。
#[derive(Component)]
pub struct DolaAnimator {
    runtime: DolaRuntime,
}

// Safety: wintf は単一スレッド（Windows UI スレッド）でのみ動作。
// tick_dola_animators の Query<&mut DolaAnimator> が 1 tick 1 回・
// 単一スレッドの排他アクセスを型レベルで保証する。
unsafe impl Send for DolaAnimator {}
unsafe impl Sync for DolaAnimator {}

impl DolaAnimator {
    pub fn new() -> Self;
    pub fn with_runtime(runtime: DolaRuntime) -> Self;

    /// 内部 DolaRuntime の tick を実行
    pub fn tick(&mut self, current_time: f64);

    /// 直前の tick 結果を参照
    pub fn last_result(&self) -> &UpdateResult;

    /// DolaRuntime への参照（DolaDocument ロード等に使用）
    pub fn runtime(&self) -> &DolaRuntime;

    /// DolaRuntime への可変参照。
    /// pub(crate) に制限し、外部コードによる tick() の直接呼び出しを禁止する。
    /// 安全性根拠（Req 2.2）: tick() は tick_dola_animators システムのみが呼び出す。
    /// DolaDocument ロード等の正当なユースケースは load_document() 等の専用メソッドを追加する。
    pub(crate) fn runtime_mut(&mut self) -> &mut DolaRuntime;
}

/// 全 DolaAnimator を一括 tick するシステム。
/// Update スケジュール先頭に配置。
pub fn tick_dola_animators(
    mut query: Query<&mut DolaAnimator>,
    frame_time: Res<FrameTime>,
) {
    let current_time = frame_time.0;
    for mut animator in query.iter_mut() {
        animator.tick(current_time);
    }
}
```

##### State Management

- **状態モデル**: `DolaAnimator` は `DolaRuntime` の状態を透過的に委譲。追加の状態管理なし
- **ライフサイクル**: Entity spawn 時に `DolaAnimator::new()` で生成。Entity despawn 時に自動 drop
- **順序保証**: 消費者システムは `.after(tick_dola_animators)` で順序依存を宣言

**Implementation Notes**

- **配置先**: `crates/wintf/src/ecs/dola/mod.rs`。`ecs/dola/` は DolaAnimator 専用モジュールとし、将来の拡張（PropertyBinding 等）に備える。balloon06 の `dola_bridge/` 想定とは命名が異なるが、balloon06 は未実装のため本仕様が正となる
- **Validation**: DolaAnimator の Debug impl でランタイム状態の概要を出力（アクティブインスタンス数等）
- **Risks**: `unsafe impl Send + Sync` — 安全性根拠の文書化が必須。`tick_dola_animators` 以外からの `tick()` 呼び出しは API レベルでは禁止できない（規約による制約）

---

### Component: `wintf::ecs::cue::CueQueue` — リファクタリング方針

**Intent**: `dola::TimedSchedule<dola::CueCommand>` を内包する ECS Component ラッパーへの段階的リファクタリング方針を定義する。

**Requirements**: 3.2, 3.3, 3.4, 3.5

**Contracts** (リファクタリング後の構成):

```rust
/// リファクタリング後の CueQueue 構造（Phase 3 で実施）
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct CueQueue {
    /// dola TimedSchedule に委譲（コア時刻管理）
    schedule: TimedSchedule<CueCommand>,

    // ── wintf 固有（ECS 依存） ──
    state: CueQueueState,      // ECS 固有の状態管理
    playback_rate: f64,         // 再生速度
    capacity: Option<usize>,    // キャパシティ
    pending_choices: Vec<PendingChoice>,  // Choice 先積み
    cue_sheet_entity: Option<Entity>,     // Tracker Entity
}
```

**Implementation Notes**

- **移行戦略（D5 決定）**: Phase 2a（DolaRuntime 除去、Phase 1 非依存）→ Phase 1（dola 新規型）→ Phase 2b+3（CueCommand 移管 + CueQueue リファクタリング同時実施）
- **re-export（D4 決定）**: `pub use dola::cue::CueCommand;` により wintf `CueCommand` のインポートパスを維持
- **u64 ↔ Entity 変換**: CueQueue に `push_entity_command(time: f64, cmd: CueCommand, entity: Entity)` ヘルパーを追加。`EntityRef(entity.to_bits())` でエンキュー、pop 時に `Entity::from_bits()` で復元

---

## Data Models

### Domain Model

```mermaid
classDiagram
    class CueSheet {
        +Vec~Cue~ cues
        +new(cues) CueSheet
        +cues() ~Cue~ slice
        +filter_by_actor(key)
        +actors()
    }

    class Cue {
        +ActorKey actor
        +f64 start_time
        +CueCommand command
    }

    class CueCommand {
        <<enum>>
        Text(String)
        Clear
        Emote
        Choice
        EntityRef(u64)
        Custom
        RouteAdd
        RouteSwitch
        RouteRemove
    }

    class TimedSchedule~T~ {
        -Vec~Entry~T~~ entries
        -Vec~T~ ready_buffer
        +advance(f64)
        +ready() ~T~ slice
        +current_barrier()
        +resolve_barrier()
    }

    class Entry~T~ {
        <<enum>>
        Payload(f64 T)
        Barrier(f64 BarrierKind)
    }

    class BarrierKind {
        <<enum>>
        WaitForInput
        WaitForChoice
        Timeout
    }

    CueSheet "1" *-- "*" Cue
    Cue --> CueCommand
    Cue --> ActorKey
    TimedSchedule --> Entry
    Entry --> BarrierKind
```

**不変条件**:
- `CueSheet` 内の `Cue` は `start_time` 昇順
- `TimedSchedule` 内の `Entry` は f64 タイムスタンプ降順（末尾 pop で O(1) 消費）
- `advance()` 後の `ready_buffer` はバリア到達前の全 Payload を含む
- `resolve_barrier()` はバリア状態でのみ有効（非バリア時は no-op）

---

## Error Handling

### Error Strategy

dola `cue/` モジュールのエラーは `thiserror` ベースの enum で定義し、既存の `DolaError` とは独立した `CueError` を提供する。wintf 側のエラーは既存の `CueSystemError` を拡張する。

### Error Categories

| カテゴリー | エラー | 処理 |
|-----------|--------|------|
| バリデーション | 負のタイムスタンプ | `debug_assert!` + ログ警告（リリースでは許容） |
| 状態不正 | バリア未停止時の `resolve_barrier()` | no-op + ログ（静穏失敗） |
| キャパシティ | TimedSchedule 挿入限界超過 | `Result<(), CueError>` 返却 |
| Entity 復元 | `Entity::from_bits(u64)` の無効値 | wintf 消費者が Query 存在確認で検出 |

---

## Testing Strategy

### Unit Tests（dola crate）

| テスト対象 | 検証内容 |
|-----------|---------|
| `TimedSchedule::advance` | 時刻到達済み Payload の正確な収集、冪等性 |
| `TimedSchedule::barrier` | バリア到達で停止、resolve 後に再進行 |
| `CueCommand::is_routing_command` | 9 バリアントの分類正確性 |
| `CueSheet::new` | start_time 昇順ソート |
| `compile_sheet` | 相対→絶対変換の正確性 |
| `DolaRuntime::tick/last_result` | update() と同等の結果、last_result の冪等性 |

### Integration Tests（wintf crate）

| テスト対象 | 検証内容 |
|-----------|---------|
| `DolaAnimator` Component | spawn/tick/last_result の一連の流れ |
| `tick_dola_animators` System | Query<&mut> による全エンティティ一括 tick |
| re-export 後方互換 | `wintf::ecs::cue::CueCommand` パスの維持 |
| CueQueue + TimedSchedule | push → advance → ready の統合フロー |
| 既存 cue テスト 75 件 | 全パス（リグレッションなし） |

### テスト移行（D8 決定）

| 現行テスト | 移行先 | 処理 |
|-----------|--------|------|
| `cue_dola_integration_test.rs` DolaRuntime 5 件 | `tests/ecs/dola/` | DolaAnimator テストに書き直し |
| `cue_dola_integration_test.rs` FrameTime 3 件 | `tests/ecs/graphics/` | 移動（FrameTime は graphics モジュール所属） |
| `cue_dola_integration_test.rs` ファイル | — | 削除 |

---

## Migration Strategy

### Phase Plan

```mermaid
flowchart LR
    P2a[Phase 2a: DolaRuntime 除去]
    P1[Phase 1: dola 新規型]
    P2b3[Phase 2b+3: CueCommand 移管 + CueQueue 再設計]
    P4[Phase 4: DolaAnimator 統合]
    Doc[Doc: ドキュメント更新]

    P2a --> P4
    P1 --> P2b3
    P2b3 --> P4
    P1 --> P4
    P4 --> Doc
```

| Phase | 内容 | 前提 | 成果物 |
|-------|------|------|--------|
| **2a** | DolaRuntime 除去 — `cue/runtime.rs` 削除、`update_dola_runtime` 削除、re-export 削除、テスト 5 件移行 | なし（消費者ゼロ） | wintf cue モジュール浄化 |
| **1** | dola 新規型 — `cue/` モジュール新設、`TimedSchedule<T>`, `CueCommand`, `CueSheet`, `compile_sheet`, ドメイン型、`tick/last_result` 分離 | なし | dola cue 基盤完成 |
| **2b+3** | CueCommand 移管 + CueQueue 再設計 — wintf `CueCommand` を dola re-export に変更、`CueQueue` 内部を `TimedSchedule<CueCommand>` に委譲 | Phase 1 完了 | wintf cue 型の dola 委譲 |
| **4** | DolaAnimator 統合 — `ecs/dola/` 新設、DolaAnimator Component + tick_dola_animators System、テスト書き直し | Phase 1（tick/last_result）、Phase 2a（旧 runtime 除去） | ECS アニメーション統合 |
| **Doc** | ドキュメント更新 — cue-system design.md 是正、ARCHITECTURE.md 更新、structure.md 更新、dola 統合ガイドライン | Phase 4 完了 | 文書整合性 |

### Rollback Triggers

- Phase 1: dola 既存テスト（compile, runtime, validation 等）が fail → ロールバック
- Phase 2a: wintf テスト 920+ のうち cue 関連 75 件が fail → ロールバック
- Phase 2b+3: re-export 変更後にコンパイルエラー → re-export パスの修正で対応
- Phase 4: `unsafe impl Send + Sync` による UB 検出 → DolaAnimator 設計見直し

### UpdateResult 消費方針（Req 4）

本仕様のスコープでは `UpdateResult` の消費パターンを**方針決定**にとどめる:

- **`changes`（購読変数差分）**: balloon06-text-effects の `dola_sync_system` が PropertyBinding → ECS Component 反映パターンを実装（選択肢 (a) ECS コンポーネントへの反映）
- **`triggered`（トリガー結果）**: dola 単体のトリガー機構に委譲（選択肢 (c)）。連鎖アニメーション起動は DolaRuntime 内部で完結
- **将来仕様**: `wintf-P0-balloon06-text-effects` で具体的消費実装を定義

---

## Supporting References

### dola cue/ モジュール構成

```
crates/dola/src/
├── cue/                    ← NEW
│   ├── mod.rs              ← re-exports
│   ├── schedule.rs         ← TimedSchedule<T>, Entry<T>, BarrierKind
│   ├── command.rs          ← CueCommand, ActorKey, CueTarget, EntityKey, Cue
│   └── sheet.rs            ← CueSheet, compile_sheet, CompiledCue
├── runtime/
│   ├── facade.rs           ← tick/last_result 追加
│   └── types.rs            ← UpdateResult (変更なし)
└── lib.rs                  ← pub mod cue; + re-exports 追加
```

### wintf モジュール変更

```
crates/wintf/src/ecs/
├── dola/                   ← NEW
│   └── mod.rs              ← DolaAnimator, tick_dola_animators
├── cue/
│   ├── command.rs          ← re-export only (pub use dola::cue::*)
│   ├── mod.rs              ← runtime/update_dola_runtime の re-export 除去
│   ├── queue.rs            ← Phase 3 で TimedSchedule 内包に再設計
│   ├── runtime.rs          ← Phase 2a で削除
│   └── systems.rs          ← update_dola_runtime 除去
└── mod.rs                  ← pub mod dola; 追加
```
