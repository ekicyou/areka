# Requirements Document

| 項目               | 内容                                                       |
| ------------------ | ---------------------------------------------------------- |
| **Document Title** | dola ランタイム責務境界定義（dola-boundary）要件定義書      |
| **Version**        | 3.0                                                        |
| **Date**           | 2026-02-28                                                 |
| **Priority**       | P0（wintf-P0-cue-system の unblock 条件）                  |
| **Status**         | 📋 Generated（v3.0: CueCommand 全バリアント dola 移管方針を反映）|

---

## Introduction

本仕様書は、dola ランタイム（`dola::runtime::DolaRuntime`）を wintf ECS アーキテクチャ内でどのように配置・所有・利用すべきかの**責務境界**を定義する。

### 経緯

wintf-P0-cue-system 実装時、設計指示「dola の思想を共有する」を「dola ランタイムを ECS Resource として cue モジュール内に配置する」と解釈し、以下の3ファイルが作成された。

| ファイル | 内容 | 問題 |
|----------|------|------|
| `ecs/cue/runtime.rs` | `DolaRuntime` を `#[derive(Resource)]` でラップ | cue パイプラインは DolaRuntime を一切消費しない |
| `ecs/cue/systems.rs` | `update_dola_runtime` システム | 戻り値 `UpdateResult` を `_result` として破棄 |
| `ecs/cue/mod.rs` | `pub use runtime::DolaRuntime` | cue モジュールの公開 API として露出 |

**根本原因**: dola ランタイムの責務境界が未定義のまま実装が先行した。

### dola ランタイムの本質

`dola::runtime::DolaRuntime` は**タイミングエンジン**（Facade パターン）であり、内部に以下を所有する:

- **DocumentStore** — DolaDocument の保持・バリデーション
- **InstanceManager** — ストーリーボード実行インスタンスの状態遷移（group_id 別）
- **TimelineManager** — 変数ごとのタイムテーブル・補間評価
- **SubscriptionManager** — 購読変数の差分検出

`update(current_time)` を呼ぶと、タイムラインを評価し `UpdateResult { changes, triggered }` を返す。これは ECS の Resource でも Component でもなく、アニメーション変数の補間・再生・購読を行う**内部実装の道具**である。

### 設計上の制約

- `DolaRuntime` は `Rc<DynamicValue>` を内部に持つため `Send` / `Sync` ではない（wintf は単一 UI スレッドで動作するため `unsafe impl` で回避可能だが、設計として妥当か要検討）
- 複数インスタンスの並行使用が可能（グローバル状態なし）
- `dola::runtime::clock::now()` は `QueryPerformanceCounter` ベースで、wintf の `FrameTime(f64)` と同じ時刻基準を共有

### 方針決定（2026-02-28）

gap-analysis および議論を経て、以下の方針が承認された：

> **dola は `bevy_ecs` に依存しない範疇で、可能な限りアニメーションエンジンとしての責務を移譲させる。**
> 具体的には、**離散コマンドスケジューリング**（時刻ベース実行キュー + バリア状態機械 + コアコマンド enum）を dola に持たせる。
> pasta DSL を利用した高レベル演出表現（さくらスクリプトの置き換え）も dola のスコープとする。

この決定により、本仕様のスコープは「DolaRuntime の誤配置修正」から「**dola と wintf の責務境界の根本定義**」へと拡張された。

#### 移管対象（ECS 非依存 → dola）

| 概念 | 現在の所在 | 移管後 |
|------|-----------|--------|
| `TimedSchedule<T>` — 汎用絶対時刻キュー + `pop_ready()` | wintf `CueQueue` 内に暗黙的に埋め込み | dola の新規ジェネリック型 |
| バリア状態機械 — WaitForInput / WaitForChoice / timeout | wintf `CueQueue.barrier_state` | dola `TimedSchedule<T>` に統合 |
| `CueScript` — 相対時刻コマンド列（= CueSheet の dola 版） | wintf `CueSheet` | dola の新規型 |
| `compile_script` — 相対→絶対時刻変換 | wintf `dispatch` の一部 | dola の新規関数 |
| `CueCommand` **全 11 バリアント** | wintf `command.rs` | dola の新規 enum |
| `ActorKey(String)` — アクター識別子 | wintf `cue/mod.rs` | dola に移管 |
| `CueTarget` (Shell \| Balloon) — 配送先スロット | wintf `cue/mod.rs` | dola に移管 |
| `EntityKey` — ルーティングキー | wintf `command.rs` | dola に移管 |
| `Cue` (actor + start_time + command) — 個々の演出指示 | wintf `cue/mod.rs` | dola に移管 |

**CueCommand 全 11 バリアント dola 移管（v3.0 決定）**:
- データ（6）: `Text`, `Clear`, `Emote`, `Choice`, `EntityRef(u64)`, `Custom`
- バリア（2）: `WaitForChoice`, `WaitForClick`
- ルーティング（3）: `RouteAdd`, `RouteSwitch`, `RouteRemove`

**EntityRef(Entity) → EntityRef(u64) 変換**: bevy_ecs の `Entity::to_bits() -> u64` / `Entity::from_bits(u64)` を利用し、dola 側では u64 として保持。wintf が push/pop 境界で変換する。

ルーティングコマンドが使用する `CueTarget`・`EntityKey`・`ActorKey` はすべて文字列ベースのドメイン型であり、bevy_ecs に依存しない。

#### wintf に残る部分（ECS 依存）

| 概念 | ECS 依存の根拠 |
|------|---------------|
| `CueQueue` — ECS Component ラッパー | `#[derive(Component)]`, `SparseSet` storage |
| `u64 ↔ Entity` 変換ユーティリティ | `bevy_ecs::entity::Entity::to_bits()` / `from_bits()` |
| `EntityRegistry` — ECS Resource | ActorKey → ECS Entity の解決 |
| `CueSheetTracker` — ECS Component | ECS Query を通じたトラッキング |
| dispatch システム — ECS System | bevy_ecs スケジューラ統合 |

## Project Description (Input)

DolaRuntime の使い方が間違っている件の是正。cue-system 実装時に DolaRuntime を bevy_ecs Resource としてシングルトン化し cue モジュール内に配置したが、これは設計ミスである。

### 背景
- cue-system の設計指示は「dola の思想を共有する」であり、「dola ランタイムを利用する」ではなかった
- 「今後使うことになる」とは伝えられたが、どう使うべきか・誰が所有すべきかの線引きがされていない
- 結果、cue モジュール内に DolaRuntime が Resource として配置され、update_dola_runtime システムが存在するが、cue パイプラインはこれを一切消費していない

### 解決すべき問題
1. **DolaRuntime は ECS の Resource でもなく Component でもない** — 単なるタイミングエンジン（アニメーション変数の補間・再生・購読を行う内部実装の道具）
2. **cue モジュール内に配置する理由がない** — cue は演出指令の配送基盤であり、アニメーション実行エンジンの管理場所ではない
3. **シングルトン前提が間違っている** — 複数の DolaRuntime が同時に動作する可能性がある
4. **dola を使うか・使わないか・拡張するか・wintf 側を拡張すべきか** — 責務境界の定義が必要
5. **CueQueue の時刻スケジューリングロジック（ECS 非依存部分）が wintf に誤残している** — `TimedSchedule<T>`, バリア状態機械, コアコマンド enum は dola が担うべき汎用エンジン機能である

### 調査・決定すべきこと
- dola ランタイムを wintf のどのレイヤーで使うのか（Balloon? Spot? 別のコンポーネント?）
- dola ランタイムの所有者は誰か（EcsWorld 外部? コンポーネントの内部フィールド? 利用者ごとのインスタンス?）
- cue-system から DolaRuntime 関連コードを除去すべきか
- update_dola_runtime システムは廃止か・移動か・再設計か
- ~~dola に離散コマンドスケジューリングを持たせるか~~ → **承認済み（2026-02-28）**

---

## Requirements

### Requirement 0: dola クレートへの離散コマンドスケジューリング移管

**Objective:** dola 開発者として、bevy_ecs に依存しない演出スケジューリング機能（時刻キュー・バリア・コアコマンド）を dola クレートに持たせたい。dola を汎用アニメーションエンジンとして育て、wintf だけでなく pasta DSL 等からも利用可能にするため。

#### Acceptance Criteria

1. The dola crate shall `bevy_ecs` クレートへの依存を持たない
2. The dola crate shall `TimedSchedule<T>` 型（または同等の汎用時刻スケジューリング型）を提供する — ジェネリックペイロード `T` と絶対時刻 `f64` を対応付け、`pop_ready(current_time: f64) -> Vec<T>` で時刻到達済みエントリを返す
3. The dola crate shall バリア状態機械を提供する — WaitForInput（クリック/キー）/ WaitForChoice（選択肢）/ タイムアウト の3状態を `TimedSchedule<T>` と協調して管理する
4. The dola crate shall `CueScript` 型（相対時刻コマンド列）と `compile_script` 関数（相対時刻 → 絶対時刻変換）を提供する
5. The dola crate shall 演出コマンド enum を提供する — 全 11 バリアント: データ 6（`Text(String)`, `Clear`, `Emote { key }`, `Choice { id, text }`, `EntityRef(u64)`, `Custom { command, params: DynamicValue }`）、バリア 2（`WaitForChoice { timeout }`, `WaitForClick { timeout }`）、ルーティング 3（`RouteAdd { target, to }`, `RouteSwitch { target, to }`, `RouteRemove { target }`）
6. The dola crate shall 演出コマンドのドメイン型を提供する — `ActorKey(String)`（アクター識別子）、`CueTarget`（配送先スロット: Shell / Balloon）、`EntityKey`（ルーティングキー: Actor / Spot / Balloon）
7. When pasta DSL との統合が将来必要になった場合、dola shall pasta DSL の出力を `CueScript` として受け取るインターフェースを提供できる設計とする
8. The dola crate shall 既存の連続値タイムライン機能（`DolaRuntime`, `DolaDocument`, `compile_storyboard`）との責務分離を明確にする — 連続値補間 vs 離散コマンドスケジューリング

---

### Requirement 1: DolaRuntime の所有モデル定義

**Objective:** wintf 開発者として、DolaRuntime のインスタンス所有権と生存期間のルールを確立したい。将来のアニメーション消費者（Balloon、Spot 等）が混乱なく DolaRuntime を利用できるようにするため。

#### Acceptance Criteria

1. The wintf architecture specification shall DolaRuntime の所有モデルを以下のいずれかに決定し文書化する: (a) コンポーネント内部フィールド（エンティティごとのインスタンス）、(b) EcsWorld 外部の所有（アプリケーション層管理）、(c) 専用モジュールの ECS Resource（用途別シングルトン）
2. The wintf architecture specification shall 選択した所有モデルの根拠と、棄却した選択肢の棄却理由を記録する
3. The wintf architecture specification shall DolaRuntime の生存期間（いつ生成し、いつ破棄するか）のルールを定義する
4. While DolaRuntime が複数インスタンスとして使用される場合、wintf shall 各インスタンスが独立した状態を持ち相互干渉しないことを保証する設計とする

---

### Requirement 2: cue モジュールからの DolaRuntime 除去

**Objective:** wintf 開発者として、cue モジュールから DolaRuntime 関連コードを除去したい。cue モジュールの責務を「演出指令の配送基盤」に限定し、アニメーション実行エンジンの管理責務を排除するため。

#### Acceptance Criteria

1. When この仕様が実装される場合、wintf shall `ecs/cue/runtime.rs` を削除または空にする
2. When この仕様が実装される場合、wintf shall `ecs/cue/systems.rs` から `update_dola_runtime` 関数を削除する
3. When この仕様が実装される場合、wintf shall `ecs/cue/mod.rs` から `pub use runtime::DolaRuntime` を削除する
4. When DolaRuntime 関連コードが除去された後、wintf shall 既存の cue パイプライン（CueSheet → dispatch → CueQueue → pop_ready → CueSheetTracker）が変更なく動作し続ける
5. When DolaRuntime 関連コードが除去された後、wintf shall 既存の cue テスト 75 件がすべてパスする

---

### Requirement 3: DolaRuntime の配置先決定

**Objective:** wintf 開発者として、DolaRuntime を wintf のどのモジュールに配置すべきかを決定したい。レイヤー依存方向（COM → ECS → Message Handling）を遵守し、将来の消費者が自然にアクセスできるようにするため。

#### Acceptance Criteria

1. The wintf architecture specification shall DolaRuntime ラッパー（`unsafe impl Send/Sync` を含む）の配置先モジュールを決定する
2. If DolaRuntime を ECS Resource として配置する場合、wintf shall cue モジュール以外の適切なモジュール（例: `ecs/animation/` や `ecs/dola/`）に配置する
3. If DolaRuntime をコンポーネント内部フィールドとして使用する場合、wintf shall ラッパー型を共通ユーティリティモジュールに配置し、各コンポーネントが個別にインスタンスを保持する設計とする
4. The wintf architecture specification shall 配置先が ECS レイヤー依存方向を遵守していることを検証する
5. When 配置先が決定された後、wintf shall `update_dola_runtime` システムの処理（FrameTime → DolaRuntime.update() → UpdateResult 活用）を新しい配置先で再実装するか、または廃止の判断を文書化する

---

### Requirement 4: UpdateResult の活用方針

**Objective:** wintf 開発者として、`DolaRuntime::update()` の戻り値 `UpdateResult { changes, triggered }` の消費方法を定義したい。現在の実装では戻り値が `_result` として破棄されており、dola の購読差分検出機能が無駄になっているため。

#### Acceptance Criteria

1. The wintf architecture specification shall `UpdateResult.changes`（変化した購読変数のリスト）の消費パターンを定義する — (a) ECS コンポーネントへの反映、(b) イベント送信、(c) 消費者ごとの直接参照、のいずれか
2. The wintf architecture specification shall `UpdateResult.triggered`（トリガー実行結果）の消費パターンを定義する — (a) 連鎖アニメーション起動、(b) ECS イベント変換、(c) 不使用（dola 単体のトリガー機構に委譲）、のいずれか
3. If `UpdateResult` の消費パターンが本仕様のスコープ外と判断される場合、wintf shall その旨と、どの将来仕様で扱うべきかを文書化する

---

### Requirement 5: 時刻基準の統一保証

**Objective:** wintf 開発者として、dola ランタイムと cue-system が同一の時刻基準を使用することを保証したい。FrameTime(f64) と dola::runtime::clock::now() はどちらも QueryPerformanceCounter ベースだが、統一ルールが明文化されていないため。

#### Acceptance Criteria

1. The wintf architecture specification shall DolaRuntime に対する時刻供給元を一意に定義する — (a) FrameTime.0 の値を渡す、(b) DolaRuntime 内部で clock::now() を直接呼ぶ、のいずれか
2. When FrameTime.0 を時刻供給元とする場合、wintf shall DolaRuntime の更新タイミングを ECS スケジュール内で明示的に順序付ける（Update スケジュールの先頭等）
3. While cue-system と DolaRuntime が同じ EcsWorld 内で動作する場合、wintf shall 両者が同一フレーム内で同一の時刻値を参照することを保証する

---

### Requirement 6: dola 統合ガイドラインの文書化

**Objective:** wintf 開発者として、将来のアニメーション消費者（Balloon テキストアニメーション、Spot サーフェス遷移、等）が dola を正しく統合するためのガイドラインを持ちたい。「dola の思想を共有する」と「dola ランタイムを使う」の区別が曖昧だったことによる混乱を防ぐため。

#### Acceptance Criteria

1. The wintf architecture specification shall 「dola の思想を共有する」の意味を定義する — 宣言的構造 → コンパイル → 時刻ベース実行のパイプラインパターンを採用すること
2. The wintf architecture specification shall 「dola ランタイムを使う」の意味を定義する — `dola::runtime::DolaRuntime` のインスタンスを直接利用して補間・購読・トリガーを実行すること
3. The wintf architecture specification shall 各アニメーション消費者が dola 統合時に従うべき手順を定める — (a) DolaDocument のロード、(b) 変数の購読、(c) update ループへの組み込み、(d) UpdateResult の消費
4. If 将来の消費者が dola ランタイムを使わず独自のタイムライン実装を選択する場合、wintf shall その判断基準（パフォーマンス、依存削減等）と、cue-system との互換性維持方法を文書化する

---

### Requirement 8: CueCommand 全バリアントの dola 移管

**Objective:** wintf 開発者として、wintf の `CueCommand` enum（全 11 バリアント）およびドメイン型（`ActorKey`, `CueTarget`, `EntityKey`, `Cue`, `CueSheet`）を dola に全面移管したい。dola に依存しない消費者（pasta DSL 処理系など）が同じコマンド型・ルーティング型を使えるようにし、wintf は ECS 結合層のみを担うため。

#### Acceptance Criteria

1. The dola crate shall `CueCommand` enum として全 11 バリアントを提供する — データ 6（`Text(String)`, `Clear`, `Emote { key: String }`, `Choice { id: String, text: String }`, `EntityRef(u64)`, `Custom { command: String, params: DynamicValue }`）、バリア 2（`WaitForChoice { timeout: Option<f64> }`, `WaitForClick { timeout: Option<f64> }`）、ルーティング 3（`RouteAdd { target: CueTarget, to: EntityKey }`, `RouteSwitch { target: CueTarget, to: EntityKey }`, `RouteRemove { target: CueTarget }`）
2. When wintf が `EntityRef` コマンドを CueQueue に投入する場合、wintf shall `bevy_ecs::entity::Entity::to_bits() -> u64` で変換し、dola の `EntityRef(u64)` として格納する
3. When wintf が CueQueue から `EntityRef(u64)` を取り出す場合、wintf shall `bevy_ecs::entity::Entity::from_bits(u64)` で ECS Entity に復元する
4. The dola crate shall `CueCommand` enum が `Clone + Debug + PartialEq` を満たし、シリアライズ可能（serde 対応）とする
5. The dola crate shall ドメイン型 `ActorKey(String)`、`CueTarget`（Shell / Balloon）、`EntityKey`（Actor / Spot / Balloon）を提供する
6. If 移管によって既存の wintf cue テスト（75 件）に影響が生じる場合、wintf shall `type CueCommand = dola::CueCommand` 型エイリアスまたは re-export で後方互換性を維持する
7. The dola crate shall `CueCommand::is_barrier()` および `CueCommand::is_routing_command()` メソッドを提供する（既存の分類ロジックを移管）

---

### Requirement 9: wintf cue モジュールの dola ベース再設計方針

**Objective:** wintf 開発者として、CueCommand 全面移管後に wintf の cue モジュールが「ECS 統合レイヤー」だけを責務として持つ設計に整理したい。dola が提供する `TimedSchedule<T>` と `CueCommand` を内包する形で `CueQueue` を再設計することで、wintf 側のドメインロジックを最小限に抑えるため。

#### Acceptance Criteria

1. The wintf architecture specification shall dola への移管後の `CueQueue` コンポーネントが `dola::TimedSchedule<dola::CueCommand>` を内包する設計かどうかを決定し文書化する
2. If `CueQueue` が `dola::TimedSchedule` を内包する場合、wintf shall `push_sorted` / `pop_ready` / `check_timeout` / バリア管理の実装を dola 側に委譲し、ECS Component としてのラッピング + `u64 ↔ Entity` 変換のみを wintf が担う
3. The wintf architecture specification shall 移行戦略を定義する — (a) 即時置換（本仕様で dola 実装後に再設計）、(b) 段階的移行（本仕様では除去のみ、別仕様で再設計）
4. When dola の `TimedSchedule<T>` と `CueCommand` が実装済みである場合、wintf shall cue モジュール内の重複するスケジューリングロジックとコマンド型定義を dola に委譲する
5. When wintf の cue モジュールが再設計された後、wintf shall `wintf::ecs::cue::CueCommand` を `type CueCommand = dola::CueCommand` として再公開し、既存の参照を維持する

---

### Requirement 7: cue-system 設計ドキュメントの是正

**Objective:** wintf 開発者として、wintf-P0-cue-system の設計ドキュメント内の DolaRuntime 関連記述を是正したい。現在の design.md は DolaRuntime を「必須リソース」「インフラ」として記載しており、実態と乖離しているため。

#### Acceptance Criteria

1. When この仕様が実装される場合、wintf shall wintf-P0-cue-system の design.md から DolaRuntime を「インフラ」「必須リソース」として記載している箇所を修正する
2. When この仕様が実装される場合、wintf shall wintf-P0-cue-system の design.md の Architecture Boundary Map（mermaid 図）から DolaRuntime ノードを除去またはスコープ外として明示する
3. When この仕様が実装される場合、wintf shall wintf-P0-cue-system の Requirements Traceability マトリクスから Req 6（dola 統合）の記述を本仕様への参照に更新する
4. When この仕様が実装される場合、wintf shall wintf-P0-cue-system の Component Summary から DolaRuntime 行を除去し、本仕様への参照ノートを追加する

---

## Non-Functional Requirements

### NFR-1: 後方互換性

**Objective:** 本仕様の実装により、既存の wintf 機能（cue-system 含む）および dola クレートの既存機能がリグレッションを起こさないことを保証する。

#### Acceptance Criteria

1. When DolaRuntime 関連コードが cue モジュールから除去された後、wintf shall 全テストスイート（920+ テスト）がパスする
2. When DolaRuntime 関連コードが cue モジュールから除去された後、wintf shall 全サンプルアプリケーション（taffy_flex_demo 等）がパニックなく起動する
3. The wintf crate shall 公開 API（`lib.rs` の `pub use`）から DolaRuntime を除去しても、現在 DolaRuntime を使用している外部コードが存在しないことを確認する
4. When dola クレートに新規型（`TimedSchedule<T>`, コア演出コマンド enum 等）が追加された後、dola shall 既存のすべての dola テストがパスする
5. When dola クレートに新規型が追加された後、dola shall 既存の連続値タイムライン機能（`DolaRuntime`, `compile_storyboard`, `DolaDocument`）の動作が変わらない

### NFR-2: 設計文書の一貫性

**Objective:** 本仕様の実装後、アーキテクチャ文書・仕様書・コード間に矛盾が生じないことを保証する。

#### Acceptance Criteria

1. The wintf architecture specification shall DolaRuntime の所有モデル・配置先・利用ガイドラインを doc/ARCHITECTURE.md または同等のアーキテクチャ文書に反映する
2. The wintf architecture specification shall steering ファイル（structure.md）の ECS モジュール記述に DolaRuntime の配置を反映する
