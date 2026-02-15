# 子仕様統合指針 — dola-runtime-engine

> 本文書は親仕様 `dola-runtime-engine` の design.md に基づき、4つの子仕様が共通参照する統合指針を定義する。子仕様の要件定義・設計・タスク生成時にはこの文書を必ず参照すること。

---

## 1. コンポーネント所属マップ

各ランタイムコンポーネントがどの子仕様に所属するかを明確にする。

| コンポーネント | 所属子仕様 | Tier | 要件カバー | 備考 |
|---------------|-----------|------|-----------|------|
| InstanceState | core-types | 1 | Req 8 | 7バリアント enum + 状態遷移ロジック |
| EvaluatedValue | core-types | 1 | — | 補間出力の共通値型 |
| RuntimeError | core-types | 1 | — | 全子仕様のエラーハンドリング基盤 |
| StartResult | core-types | 1 | — | Start/CalculateEndTime 返却型 |
| Interpolator | core-types | 1 | Req 10 | イージング適用 + 補間計算 |
| Clock | clock | 1 | Req 11 | OS時刻取得ユーティリティ |
| DocumentStore | facade | 2 | Req 1 | 指示書管理 |
| InstanceManager | facade | 2 | Req 2, 3, 8 | インスタンスライフサイクル |
| TimelineManager | facade | 2 | Req 5, 6, 9 | タイムテーブル管理 |
| SubscriptionManager | facade | 2 | Req 4, 5 | 購読・差分配信 |
| DolaRuntime | facade | 2 | 全要件 | Facade API |
| ConflictResolver | conflict-loop | 3 | Req 7 | 競合検出 + 5戦略適用 |
| LoopController | conflict-loop | 3 | Req 12 | ループ周回管理 |

---

## 2. インターフェース契約

### 2.1 子仕様間の依存方向

```
core-types ← facade ← conflict-loop
clock ···← facade  (オプショナル依存: 利用者がclock経由で時刻を取得してfacadeに渡す)
```

- **矢印の意味**: `A ← B` は「B が A に依存する」
- **clock → facade**: clock は facade 内部に組み込まれない。利用者（wintf 等）が `clock::now()` で時刻を取得し、`runtime.update(subscriber_id, time)` に渡す。facade は clock を直接参照しない

### 2.2 core-types が提供する契約（Tier 1 → Tier 2 境界）

facade 子仕様は core-types から以下を消費する:

| 型 / 関数 | 用途 | 消費者 |
|-----------|------|--------|
| `InstanceState` | インスタンス状態管理 | InstanceManager |
| `InstanceState::try_transition()` | 状態遷移検証 | InstanceManager |
| `InstanceState::from_policy()` | InterruptionPolicy→終了状態変換 | InstanceManager, ConflictResolver |
| `InstanceState::is_terminal()` | 終了判定 | InstanceManager |
| `EvaluatedValue` | 補間結果の値型 | TimelineManager, SubscriptionManager |
| `RuntimeError` | エラー返却 | DolaRuntime（全メソッド） |
| `StartResult` | Start 返却値 | DolaRuntime |
| `Interpolator::interpolate()` | セグメント補間計算 | TimelineManager |

### 2.3 facade が提供する契約（Tier 2 → Tier 3 境界）

conflict-loop 子仕様は facade から以下を消費する:

| 型 / 構造体 | 用途 | 消費者 |
|------------|------|--------|
| `StoryboardInstance` | インスタンス状態読み書き | ConflictResolver, LoopController |
| `VariableTimeline` / `TimelineEntry` | タイムテーブル読み書き | ConflictResolver |
| `InstanceManager` (内部API) | 状態遷移通知 | ConflictResolver, LoopController |
| `TimelineManager` (内部API) | エントリ操作・延期キュー | ConflictResolver |

**重要**: ConflictResolver と LoopController は facade 内部の可変参照を受け取る設計。facade の `pub` API には競合解決・ループ制御の個別メソッドは露出しない（Start 内部で自動適用）。

### 2.4 公開 API 境界

外部（wintf 等）に公開されるのは以下のみ:

- `DolaRuntime` 構造体 + `DolaRuntimeApi` trait のメソッド群
- `EvaluatedValue`, `RuntimeError`, `StartResult` 型
- `clock::now()` 関数（feature gate `windows-clock` 有効時のみ）

以下は**公開しない**:
- `InstanceState`（外部からの状態問い合わせ API なし — ステートレス設計）
- `DocumentStore`, `InstanceManager`, `TimelineManager`, `SubscriptionManager`（内部コンポーネント）
- `ConflictResolver`, `LoopController`（内部コンポーネント）
- `StoryboardInstance`, `VariableTimeline`, `TimelineEntry`（内部データ構造）

---

## 3. 共有型カタログ

複数の子仕様にまたがって参照される型を整理する。「定義元」が型を作成・エクスポートし、「参照元」がそれを使用する。

### 3.1 core-types 定義の共有型

| 型名 | 定義元 | 参照元 | 役割 |
|------|--------|--------|------|
| `InstanceState` | core-types | facade, conflict-loop | インスタンス状態 7バリアント |
| `EvaluatedValue` | core-types | facade, conflict-loop | 補間出力値 (Float/Integer/Object) |
| `RuntimeError` | core-types | facade | エラー型 6バリアント |
| `StartResult` | core-types | facade | Start 返却 (group_id + end_time) |

### 3.2 既存 dola データモデル層の共有型

以下は既存 dola 層で定義済み。全子仕様が参照可能:

| 型名 | 定義元モジュール | 参照する子仕様 | 役割 |
|------|-----------------|---------------|------|
| `DolaDocument` | document | facade | 指示書パース結果 |
| `CompiledStoryboard` | compile | facade, conflict-loop | コンパイル済みデータ |
| `CompiledSegment` | compile | core-types, facade, conflict-loop | セグメント（from/to/easing） |
| `InterruptionPolicy` | storyboard | core-types, facade, conflict-loop | 5種中断戦略 |
| `EasingFunction` / `EasingName` | easing | core-types | イージング定義 |
| `ParametricEasing` | easing | core-types | パラメトリックイージング |
| `VariableTypeHint` | variable | core-types, facade | 変数型ヒント (Float/Integer/Object) |
| `TransitionValue` / `DynamicValue` | value | core-types | トランジション値 |
| `DolaError` | error | core-types (RuntimeError内), facade | コンパイルエラー |
| `compile_storyboard()` | compile | facade | コンパイラ関数 |

### 3.3 facade 定義の内部型（conflict-loop が参照）

| 型名 | 定義元 | 参照元 | 公開範囲 |
|------|--------|--------|---------|
| `StoryboardInstance` | facade | conflict-loop | `pub(crate)` |
| `VariableTimeline` | facade | conflict-loop | `pub(crate)` |
| `TimelineEntry` | facade | conflict-loop | `pub(crate)` |
| `DeferredEntry` | conflict-loop | conflict-loop | `pub(crate)` |

---

## 4. 依存グラフと実装順序

### 4.1 Tier 構成

```
Tier 1 (基盤・並行可能)
├── 仕様1: dola-runtime-1-core-types  ← 依存なし
└── 仕様2: dola-runtime-2-clock       ← 依存なし

Tier 2 (ランタイム本体)
└── 仕様3: dola-runtime-3-facade      ← 仕様1 に依存

Tier 3 (高度機能)
└── 仕様4: dola-runtime-4-conflict-loop ← 仕様3 に依存
```

### 4.2 各 Tier 完了時の検証可能状態

| Tier | 完了後に検証可能な機能 |
|------|---------------------|
| Tier 1 完了 | InstanceState 全遷移テスト、Interpolator 全イージング出力検証、Clock 時刻単調増加テスト |
| Tier 2 完了 | フル再生サイクル（load → start → update → 終了）、Pause/Resume、購読差分配信、**競合は未解決**（同一変数への多重 Start は後勝ちで上書き） |
| Tier 3 完了 | 5種競合戦略、Never 延期キュー、ループ再生、**全統合テスト通過** |

### 4.3 Tier 2 の竣工戦略（競合未実装時の振る舞い）

facade 子仕様（Tier 2）は conflict-loop（Tier 3）なしでも動作可能にする必要がある。暫定的な振る舞い:

- **競合未解決**: 同一変数に対する複数 group_id のエントリが共存し、最新 group_id の値が採用される（design.md の最新 group_id 優先ルール）
- **ループ未実装**: `loop_count` は無視され、常に1回再生
- **Tier 3 追加時**: ConflictResolver と LoopController を注入し、Start フローに競合解決ステップを挿入

この戦略により、Tier 2 単独でも基本的な再生パイプラインのテストが可能。

---

## 5. Feature Gate 戦略

### 5.1 feature 一覧

**重要**: 仕様2 (clock) 実装時に `runtime` と `windows-clock` 両 feature を削除する決定を行った。

- **`runtime` feature 削除理由**: dola の本質は「アニメーションエンジン」であり、ランタイムは常時有効化すべき（BREAKING CHANGE）
- **`windows-clock` feature 削除理由**: clock::now() は完全なユーティリティ関数であり、OS 自動判定 (`#[cfg(target_os = "windows")]`) で十分

**残存 feature**:

| Feature 名 | 用途 | 依存 | 理由 |
|------------|------|------|------|
| `json` | JSON パーサー | `serde_json` | 利用者の選択肢 |
| `toml` | TOML パーサー | `toml` | 利用者の選択肢 |
| `yaml` | YAML パーサー | `serde_yaml` | 利用者の選択肢 |

> **注**: `serde` は常時必須依存。feature 化しない。

### 5.2 段階的 Cargo.toml 変更計画

**仕様1 (1-core-types) 実装時:**
```toml
[dependencies]
interpolation = "0.3.0"  # 常時依存化（runtime feature 削除）

[features]
default = ["json"]
json = ["dep:serde_json"]
toml = ["dep:toml"]
yaml = ["dep:serde_yaml"]
```

> **BREAKING CHANGE**: `runtime` feature を削除。ランタイムエンジンは dola の本質であり、常時有効化する。

**仕様2 (2-clock) 実装時:**
```toml
[dependencies]
interpolation = "0.3.0"

[target.'cfg(windows)'.dependencies]
windows = { workspace = true, features = ["Win32_System_Performance"] }

[features]
default = ["json"]
json = ["dep:serde_json"]
toml = ["dep:toml"]
yaml = ["dep:serde_yaml"]
```

> **重要**: `windows-clock` feature も削除。clock::now() は完全なユーティリティ関数であり、`#[cfg(target_os = "windows")]` で条件コンパイルする。

**仕様3 (3-facade) 実装時:** 追加依存なし。

**仕様4 (4-conflict-loop) 実装時:** 追加依存なし。

### 5.3 モジュール構成

```
crates/dola/src/
├── runtime/              # 常時有効（runtime feature 削除済み）
│   ├── mod.rs            # 公開 API re-export
│   ├── instance_state.rs # 仕様1: InstanceState
│   ├── types.rs          # 仕様1: EvaluatedValue, RuntimeError, StartResult
│   ├── interpolator.rs   # 仕様1: Interpolator
│   ├── clock.rs          # 仕様2: #[cfg(target_os = "windows")]
│   ├── document_store.rs # 仕様3: DocumentStore
│   ├── instance_manager.rs # 仕様3: InstanceManager + StoryboardInstance
│   ├── timeline_manager.rs # 仕様3: TimelineManager + VariableTimeline
│   ├── subscription_manager.rs # 仕様3: SubscriptionManager
│   ├── facade.rs          # 仕様3: DolaRuntime
│   ├── conflict_resolver.rs # 仕様4: ConflictResolver
│   └── loop_controller.rs  # 仕様4: LoopController
```

---

## 6. テスト責務マトリクス

### 6.1 単体テスト（各子仕様の lib tests）

| テスト対象 | 担当子仕様 | テスト概要 |
|-----------|-----------|-----------|
| InstanceState 遷移 | core-types | 全遷移パターン（許可/拒否）、is_terminal()、from_policy() |
| EvaluatedValue / RuntimeError | core-types | 型構築・Display フォーマット |
| Interpolator | core-types | 全31イージング + ParametricEasing、型別処理、境界値 (t=0, t=1) |
| Clock::now() | clock | 時刻単調増加、ms 精度 |
| DocumentStore | facade | パース成功/失敗、定義上書き、ロールバック |
| InstanceManager | facade | group_id 採番、状態遷移、終了時刻計算、Finish 遅延 |
| TimelineManager | facade | エントリ挿入、evaluate()、終了済み破棄 |
| SubscriptionManager | facade | subscribe/unsubscribe、差分検出、Drop 自動解除 |
| ConflictResolver | conflict-loop | 5戦略個別検証、group_id 一括適用、デフォルト戦略 |
| LoopController | conflict-loop | None/Some(0)/Some(n) 判定、オフセット調整 |

### 6.2 統合テスト（`tests/` ディレクトリ）

| テストシナリオ | 担当子仕様 | 対応要件 |
|--------------|-----------|---------|
| フル再生サイクル | facade | Req 1, 2, 5 |
| Pause/Resume サイクル | facade | Req 3 |
| 購読管理 | facade | Req 4 |
| 指示書差し替え | facade | Req 1.2-1.4 |
| 競合解決（5戦略） | conflict-loop | Req 7 |
| Never 延期キュー | conflict-loop | Req 7.8 |
| ループ再生 | conflict-loop | Req 12 |

### 6.3 性能テスト（`tests/` ディレクトリ、optional `*` マーク）

| テストシナリオ | 担当子仕様 | ベースライン |
|--------------|-----------|------------|
| 100変数同時購読 Update | facade | < 16ms (60fps) |
| 50ストーリーボード同時再生 | facade | < 10MB |
| 無限ループ長時間精度 | conflict-loop | time_scale 精度劣化観測 |

---

## 7. 子仕様作成ガイドライン

### 7.1 仕様サイクルの進め方

各子仕様は以下の工程で作成する:

1. **init**: `.kiro/specs/{child-spec-name}/spec.json` を作成
2. **requirements**: 親仕様の該当要件を子仕様の粒度に詳細化（番号体系は子仕様で独立）
3. **design**: 親仕様の design.md から該当コンポーネントの設計を抽出・詳細化
4. **tasks**: 子仕様の実装タスクを生成（ここで初めてコード実装レベルの粒度）

### 7.2 子仕様での要件番号体系

- 親仕様の要件番号（Req 1〜12）は各子仕様の requirements.md 内でトレーサビリティとして参照する
- 子仕様独自の要件番号体系を採用する（例: core-types の Req 1, 2, 3...）
- 子仕様 requirements.md の各要件に `_Parent: Req X.Y_` 形式で親要件への逆参照を記載する

### 7.3 design.md からの抽出範囲

| 子仕様 | 抽出するセクション |
|--------|------------------|
| core-types | Data Models (InstanceState, EvaluatedValue, RuntimeError, StartResult), Interpolator コンポーネント, 状態遷移図 |
| clock | Clock コンポーネント, Technology Stack (Win32 `QueryPerformanceCounter` / `QueryPerformanceFrequency`) |
| facade | 全 Components (DocumentStore, InstanceManager, TimelineManager, SubscriptionManager, DolaRuntime), System Flows (Update 評価サイクル, 状態遷移図), Data Models |
| conflict-loop | ConflictResolver コンポーネント, LoopController コンポーネント, System Flows (競合解決フロー), Implementation Extensions (Never 延期キュー) |
