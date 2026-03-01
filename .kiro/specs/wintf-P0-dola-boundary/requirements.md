# Requirements Document

| 項目               | 内容                                                  |
| ------------------ | ----------------------------------------------------- |
| **Document Title** | dola ランタイム責務境界定義（dola-boundary）要件定義書 |
| **Version**        | 4.2                                                   |
| **Date**           | 2026-03-01                                            |
| **Priority**       | P0（wintf-P0-cue-system の unblock 条件）             |
| **Status**         | 📋 Generated（v4.2: 命名確定・内部整合リファイン）    |

---

## Introduction

本仕様書は、dola ランタイム（`dola::runtime::DolaRuntime`）と wintf ECS アーキテクチャの**責務境界**を定義する。スコープは「DolaRuntime の誤配置修正」にとどまらず、**dola と wintf の責務境界の根本定義**を包含する。

### dola クレートの位置づけ

`dola` クレートは**アニメーション実現のための汎用道具集**（Declarative Orchestration for Live Animation）であり、bevy_ecs に依存しない範疇で 2 つのエンジンを提供する:

| エンジン | 記述言語文脈での呼称 | 中心型 | 性質 |
|---------|-------------------|--------|------|
| 連続値アニメ宣言エンジン | **dola**（言語） | `DolaDocument` / `DolaRuntime` | 変数補間・冪等 |
| 離散コマンド配信エンジン | **キューシート** | `CueSheet` / `TimedSchedule<T>` | コマンド列・消費型 |

設計議論では「dola 側」「キューシート側」と呼べば一意に区別できる。wintf はこの 2 エンジンを ECS Integration Layer（Component / System）として包む。

### 背景

wintf-P0-cue-system 実装時、dola ランタイムが ECS Resource として cue モジュール内に誤配置された。

| ファイル | 内容 | 問題 |
|----------|------|------|
| `ecs/cue/runtime.rs` | `DolaRuntime` を `#[derive(Resource)]` でラップ | cue パイプラインは DolaRuntime を一切消費しない |
| `ecs/cue/systems.rs` | `update_dola_runtime` システム | 戻り値 `UpdateResult` を `_result` として破棄 |
| `ecs/cue/mod.rs` | `pub use runtime::DolaRuntime` | cue モジュールの公開 API として不適切に露出 |

根本原因は、dola と wintf の責務境界が未定義のまま実装が先行したことにある。

### dola ランタイムの本質

`dola::runtime::DolaRuntime` は**タイミングエンジン**（Facade パターン）であり、内部に以下を所有する:

- **DocumentStore** — DolaDocument の保持・バリデーション
- **InstanceManager** — ストーリーボード実行インスタンスの状態遷移（group_id 別）
- **TimelineManager** — 変数ごとのタイムテーブル・補間評価
- **SubscriptionManager** — 購読変数の差分検出

外部から時刻を注入される純粋な計算エンジンであり、自ら時刻取得（`clock::now()`）を行わない。複数インスタンスの並行使用が可能（グローバル状態なし）。

### 設計上の制約

- DolaRuntime は専用 ECS Component（`DolaAnimator`）としてエンティティごとに所有する
- `DolaAnimator` は内部に `Rc<DynamicValue>`（インターニング用 `ObjectInternPool`）を含むため `unsafe impl Send + Sync` を行う。安全性は `tick_dola_animators` システムの `Query<&mut DolaAnimator>` 排他アクセスにより保証される
- DolaRuntime の API を `tick(&mut self, current_time: f64)` と `last_result(&self) -> &UpdateResult` に分離する

### 責務分割方針

dola は bevy_ecs に依存しない範疇で、可能な限りアニメーションエンジンとしての責務を担う。離散コマンドスケジューリング（時刻ベース実行キュー + バリア状態機械 + コアコマンド enum）および演出ドメイン型を dola に移管する。pasta DSL を利用した高レベル演出表現も dola のスコープとする。

#### アニメーション層構造

本仕様が定義する責務は 2 層に分かれる。両層とも「外部から `f64` 時刻を注入し tick で内部状態を更新する純粋な計算エンジン」という契約を共有するが、性質は異なる。

| 層 | 型 | 性質 | 責務 |
|---|---|---|---|
| **上位層** — 配信エンジン | `TimedSchedule<T>` / `CueQueue` | 消費型・不可逆（pop は破壊的） | エンティティ間の協調演出を時刻ベースで配信する |
| **下位層** — 変数遷移エンジン | `DolaRuntime` per entity | 参照型・冪等（同一時刻を再渡し可能） | 単一エンティティ内の変数遷移・補間を管理する |

#### 移管対象（ECS 非依存 → dola）

| 概念 | 現在の所在 | 移管後 |
|------|-----------|--------|
| `TimedSchedule<T>` — 汎用絶対時刻配信エンジン（`advance()` / `ready()`） | wintf `CueQueue` 内に暗黙 | dola 新規ジェネリック型 |
| バリア状態（`Entry<T> = Payload(f64, T) \| Barrier(f64, BarrierKind)`） | wintf `CueQueue.barrier_state` + コマンド判定 | dola `TimedSchedule<T>` に型レベルで統合 |
| `CueSheet` — 相対時刻コマンド列 | wintf `CueSheet`（**削除**・dola 型に置換） | dola 新規型 |
| `compile_sheet` — 相対→絶対時刻変換 | wintf `dispatch` の一部 | dola 新規関数 |
| `CueCommand` 9 バリアント（データ 6 + ルーティング 3）| wintf `command.rs` | dola 新規 enum |
| `BarrierKind` — WaitForInput / WaitForChoice / Timeout（3 種） | wintf `CueCommand` のバリア 2 バリアント（WaitForChoice / WaitForClick）を再設計・統合 | `Entry::Barrier` として `TimedSchedule<T>` に統合 |
| ドメイン型（`ActorKey`, `CueTarget`, `EntityKey`, `Cue`） | wintf `cue/` | dola ドメイン型 |

**CueCommand 9 バリアント**（バリアは `Entry::Barrier` として分離）:
- データ（6）: `Text`, `Clear`, `Emote`, `Choice`, `EntityRef(u64)`, `Custom`
- ルーティング（3）: `RouteAdd`, `RouteSwitch`, `RouteRemove`

`TimedSchedule<T>` 上のバリアエントリは `BarrierKind { WaitForInput { timeout }, WaitForChoice { timeout }, Timeout { duration } }` として `Entry::Barrier` に格納され、`CueCommand` enum とは独立する。旧 wintf `WaitForClick` は `WaitForInput`（クリック/キー入力）に統合され、新規に `Timeout` を追加する。

`EntityRef(Entity)` は `Entity::to_bits() -> u64` / `Entity::from_bits(u64)` で変換し、dola 側では u64 として保持する。wintf が push/pop 境界で変換する。

#### wintf に残る部分（ECS 依存）

| 概念 | ECS 依存の根拠 |
|------|---------------|
| `CueQueue` — ECS Component ラッパー | `#[derive(Component)]`, `SparseSet` storage |
| `u64 ↔ Entity` 変換 | `bevy_ecs::entity::Entity` |
| `EntityRegistry` — ECS Resource | ActorKey → ECS Entity 解決 |
| `CueSheetTracker` — ECS Component | ECS Query トラッキング |
| dispatch システム — ECS System | bevy_ecs スケジューラ統合 |

## Project Description (Input)

DolaRuntime の使い方が間違っている件の是正。cue-system 実装時に DolaRuntime を bevy_ecs Resource としてシングルトン化し cue モジュール内に配置したが、これは設計ミスである。

### 背景
- cue-system の設計指示は「dola の思想を共有する」であり、「dola ランタイムを利用する」ではなかった
- どう使うべきか・誰が所有すべきかの線引きがないまま実装が先行した
- 結果、cue モジュール内の DolaRuntime は cue パイプラインに一切消費されていない

### 解決すべき問題
1. DolaRuntime の所有権と配置先が未定義
2. cue モジュールの責務外にある DolaRuntime コードの除去
3. dola と wintf の責務境界の根本定義
4. CueQueue の時刻スケジューリングロジック（ECS 非依存部分）の正しい配置

---

## Requirements

### Requirement 1: dola 演出スケジューリング基盤

**Objective:** dola 開発者として、bevy_ecs に依存しない演出スケジューリング機能（時刻キュー・バリア・コマンド・ドメイン型）を dola クレートに統合し、DolaRuntime の API を改善したい。wintf だけでなく pasta DSL 等からも利用可能な汎用エンジンとするため。

#### Acceptance Criteria

1. The dola crate shall `bevy_ecs` クレートへの依存を持たない
2. The dola crate shall `TimedSchedule<T>` 型を提供する — 内部エントリは `Entry<T> { Payload(f64, T) | Barrier(f64, BarrierKind) | Routing(f64, RoutingCommand) }` の型レベル 3 種分離とし、f64 は 0 ベースの相対オフセット（スケジュール開始からの経過時間）とする。2 フェーズ API を持つ: `advance(&mut self, current_time: f64)` で時刻到達済みの `Payload` を内部バッファに収集して停止（バリアまたはルーティング到達、または末尾到達まで。冪等、同一時刻の再呼び出し安全）し、`ready(&self) -> &[T]` で次の `advance()` 呼び出しまでそのバッファを何度でも読み取り専用で返す。`DolaRuntime` の `tick/last_result` と対称的な設計とする。**同一時刻の処理モデル**: Payload はキーフレームベース（`ready()` が複数を返す、実行順序不定）、Barrier/Routing はシーケンシャル（同一時刻に複数ある場合、Barrier は最初の1つのみ有効、Routing は配列順に処理。推奨: 各時刻に1つのみ記述）
3. The dola crate shall `TimedSchedule<T>` のバリア管理 API を提供する — `current_barrier(&self) -> Option<&BarrierKind>` で現在の停止理由を照会し、`notify_barrier_resolved(&mut self, choice_id: Option<String>)` で外部イベントからバリア解除を通知する（WaitForInput: choice_id=None, WaitForChoice: choice_id=Some(選択ID)）。`BarrierKind` は WaitForInput（クリック/キー）/ WaitForChoice（選択肢）/ Timeout の 3 種とする。Timeout は時刻到達で自動解除。ルーティングエントリは `next_routing(&mut self) -> Option<RoutingCommand>` で取得し、CueQueue 層が消費する（`ready()` には含まれない）
4. The dola crate shall `CueSheet` 型（相対時刻コマンド列）と `compile_sheet` 関数（0 ベース Entry 生成）を提供する — `compile_sheet` は `CueSheet` の相対時刻を 0 ベースの相対オフセットに正規化して `Entry<CueCommand>` を生成する。絶対時刻への変換は `TimedSchedule::new(start_time)` の責務。`CueSheet` は dola 言語（`DolaDocument` / `Storyboard` による連続値アニメ宣言）とは独立した離散コマンド列であり、記述言語の文脈では「dola（連続値アニメ）」と「キューシート（離散コマンド）」として区別する
5. The dola crate shall `CueCommand` enum として全 6 バリアントを提供する — データ系のみ（`Text(String)`, `Clear`, `Emote { key: String }`, `Choice { id: String, text: String }`, `EntityRef(u64)`, `Custom { command: String, params: DynamicValue }`）。`Clone + Debug + PartialEq` を満たしシリアライズ可能（serde 対応）とする。バリアコマンド（旧 `WaitForChoice`, `WaitForClick`）は `BarrierKind` として、ルーティングコマンド（旧 `RouteAdd`, `RouteSwitch`, `RouteRemove`）は `RoutingCommand` として、それぞれ `Entry` レベルで分離済み
5a. The dola crate shall `RoutingCommand` enum として全 3 バリアントを提供する — `RouteAdd { target: CueTarget, to: EntityKey }`, `RouteSwitch { target: CueTarget, to: EntityKey }`, `RouteRemove { target: CueTarget }`。CueQueue 層が消費し、消費者（ready() 利用側）には届かない
6. The dola crate shall 演出ドメイン型を提供する — `ActorKey(String)`（アクター識別子）、`CueTarget`（配送先スロット: Shell / Balloon）、`EntityKey`（ルーティングキー: Actor / Spot / Balloon）、`Cue`（actor + start_time + command）
7. The dola crate shall `DolaRuntime` の API を `tick(&mut self, current_time: f64)` と `last_result(&self) -> &UpdateResult` の 2 メソッドに分離する — `tick()` は内部状態を進行し結果を内部フィールドに格納、`last_result()` は直前の結果を読み取り専用で返す
8. The dola crate shall 離散コマンドスケジューリング機能と既存の連続値タイムライン機能（`DolaRuntime`, `DolaDocument`, `compile_storyboard`）の責務を明確に分離する
9. Where pasta DSL との統合が必要になった場合、dola shall pasta DSL の出力を `CueSheet` として受け取るインターフェースを提供できる設計とする

---

### Requirement 2: DolaAnimator コンポーネント設計

**Objective:** wintf 開発者として、DolaRuntime の ECS 統合方式を確立したい。専用 ECS Component `DolaAnimator` を通じてエンティティごとに独立したアニメーション状態を管理し、`tick_dola_animators` システムで一括更新する。

#### Acceptance Criteria

1. The wintf crate shall DolaRuntime を内部に所有する ECS Component `DolaAnimator` を提供する — 内部に `Rc` を含むため `unsafe impl Send + Sync` を行い、安全性は AC2 で保証する
2. The wintf crate shall `tick_dola_animators` システムを提供する — `Query<&mut DolaAnimator>` と `Res<FrameTime>` を用い、全エンティティの `tick(FrameTime.0)` を一括実行する。Update スケジュール先頭に配置する。`DolaRuntime::tick()` の呼び出しをこのシステム内に限定することで、`Query<&mut>` の排他アクセスが 1 チック 1 回・単一スレッドを型レベルで保証し、`unsafe impl Send + Sync` の安全性根拠となる
3. The wintf crate shall 消費者システムが `Query<&DolaAnimator>` の `last_result()` で `UpdateResult` を読み取る構成とし、`.after(tick_dola_animators)` で順序依存を ECS スケジュールで保証する
4. The wintf architecture specification shall `DolaAnimator` と `tick_dola_animators` の配置先モジュール（候補: `ecs/dola/`, `ecs/dola_bridge/`）を決定し、ECS レイヤー依存方向を遵守する
5. When 配置先が決定された後、wintf shall balloon06 の `DolaBridgeResource` 設計との整合性を検証し文書化する

---

### Requirement 3: cue モジュール整理

**Objective:** wintf 開発者として、cue モジュールから DolaRuntime 関連の誤配置コードを除去し、dola が提供する型を活用した再設計方針を確立したい。cue モジュールの責務を「ECS 統合レイヤー」に限定するため。

#### Acceptance Criteria

1. The wintf crate shall cue モジュールから DolaRuntime 関連コード（`runtime.rs`, `update_dola_runtime`, `pub use DolaRuntime`）を除去する
2. The wintf architecture specification shall `CueQueue` のリファクタリング方針を決定し文書化する — `dola::TimedSchedule<dola::CueCommand>` を内包する設計とするか否か。Actor単位で独立した `TimedSchedule` を持ち、各ActorのCueQueueが独立したスケジュールを管理する
3. If `CueQueue` が `dola::TimedSchedule` を内包する場合、wintf shall `push_sorted` / `pop_ready` / バリア管理の実装を dola に委譲し、ECS Component ラッピング + `u64 ↔ Entity` 変換のみを wintf が担う。**新CueSheet投入時は既存スケジュールを全破棄**（`clear()` + `new(start_time)` + `extend()`）し、バリア中でも強制的に新スケジュールへ切り替え
4. The wintf crate shall EntityRef 投入時に `Entity::to_bits()` で u64 に変換し、取出時に `Entity::from_bits()` で ECS Entity に復元する
5. The wintf crate shall `type CueCommand = dola::CueCommand` re-export で既存コードの後方互換性を維持する
6. The wintf architecture specification shall 移行戦略を定義する — (a) 即時置換（dola 実装後に再設計）、(b) 段階的移行（除去のみ先行、再設計は別仕様）

---

### Requirement 4: UpdateResult 活用方針

**Objective:** wintf 開発者として、`DolaAnimator.last_result()` が返す `UpdateResult { changes, triggered }` の消費方法を定義したい。現在の実装では戻り値が破棄されており、dola の購読差分検出機能が活用されていないため。

#### Acceptance Criteria

1. The wintf architecture specification shall `UpdateResult.changes`（変化した購読変数のリスト）の消費パターンを定義する — (a) ECS コンポーネントへの反映、(b) イベント送信、(c) 消費者ごとの直接参照、のいずれか
2. The wintf architecture specification shall `UpdateResult.triggered`（トリガー実行結果）の消費パターンを定義する — (a) 連鎖アニメーション起動、(b) ECS イベント変換、(c) dola 単体のトリガー機構に委譲、のいずれか
3. If `UpdateResult` の消費パターンが本仕様のスコープ外と判断される場合、wintf shall その旨と、どの将来仕様で扱うべきかを文書化する

---

### Requirement 5: 設計ドキュメント整合性

**Objective:** wintf 開発者として、本仕様の実装後にアーキテクチャ文書・隣接仕様・コード間に矛盾がないことを保証したい。dola 統合ガイドラインを整備し、cue-system 設計ドキュメントの誤記述を是正するため。

#### Acceptance Criteria

1. The wintf architecture specification shall dola 統合ガイドラインを整備する — 「dola の思想を共有する」（宣言的パイプラインパターン採用）と「dola ランタイムを使う」（DolaRuntime インスタンスの直接利用）の区別を定義し、統合手順（DolaDocument ロード → 変数購読 → tick ループ → UpdateResult 消費）を定める
2. When この仕様が実装される場合、wintf shall wintf-P0-cue-system の design.md から DolaRuntime を「インフラ」「必須リソース」として記載している箇所を是正し、Architecture Boundary Map・Component Summary・Requirements Traceability を更新する
3. The wintf architecture specification shall `doc/ARCHITECTURE.md` および `.kiro/steering/structure.md` に DolaAnimator の所有モデル・配置先を反映する
4. If 将来の消費者が DolaRuntime を使わず独自のタイムライン実装を選択する場合、wintf shall その判断基準と cue-system との互換性維持方法を文書化する

---

## Non-Functional Requirements

### NFR-1: 後方互換性

**Objective:** 本仕様の実装により、既存の wintf 機能（cue-system 含む）および dola クレートの既存機能がリグレッションを起こさないことを保証する。

#### Acceptance Criteria

1. When 本仕様の変更が適用された後、wintf shall 全テストスイート（920+ テスト、cue 系 75 件含む）がパスし、全サンプルアプリケーションがパニックなく起動する
2. When dola クレートに新規型が追加された後、dola shall 既存テストがすべてパスし、連続値タイムライン機能（`DolaRuntime`, `compile_storyboard`, `DolaDocument`）の動作が変わらない
3. The wintf crate shall 公開 API から DolaRuntime を除去しても外部への破壊的影響がないことを確認する — 現在 DolaRuntime は EcsWorld に未登録であり、実稼働パスに組み込まれていない
