# ギャップ分析 — dola-runtime-3-facade

## 分析サマリー

- **スコープ**: 5 新規モジュール（`document_store.rs`, `instance_manager.rs`, `timeline_manager.rs`, `subscription_manager.rs`, `facade.rs`）を `crates/dola/src/runtime/` に追加
- **基盤**: Tier 1 core-types（`InstanceState`, `EvaluatedValue`, `RuntimeError`, `StartResult`, `Interpolator`）は実装完了・テスト済み
- **既存資産活用度**: コンパイラ (`compile_storyboard`)、データモデル (`DolaDocument`, `CompiledStoryboard`, `CompiledSegment`)、serde 基盤は全て再利用可能
- **Feature Gate 課題**: `runtime` feature gate が未削除のまま残存。facade 実装は同 feature gate 内で進行するか、このタイミングで常時有効化するかの判断が必要

---

## 1. 現状調査

### 1.1 ディレクトリ構成

```
crates/dola/src/
├── lib.rs                    # #[cfg(feature = "runtime")] pub mod runtime
├── document.rs               # DolaDocument（serde 定義済み）
├── compile.rs                # compile_storyboard()（753行、完全実装）
├── storyboard.rs             # Storyboard, InterruptionPolicy（120行）
├── variable.rs               # AnimationVariableDef（Float/Integer/Object）
├── transition.rs             # TransitionDef, TransitionValue
├── value.rs                  # DynamicValue
├── easing.rs                 # EasingFunction, EasingName, ParametricEasing
├── error.rs                  # DolaError（バリデーション/コンパイルエラー）
├── playback.rs               # PlaybackState, ScheduleRequest（旧型、facade では不使用）
├── builder.rs                # Builder API
├── validate.rs               # バリデーション
└── runtime/
    ├── mod.rs                # InstanceState, Interpolator, types re-export
    ├── instance_state.rs     # InstanceState 7バリアント（Tier 1 完了）
    ├── interpolator.rs       # Interpolator + 全31イージング（Tier 1 完了、343行）
    └── types.rs              # EvaluatedValue, RuntimeError, StartResult（Tier 1 完了）
```

### 1.2 既存テスト配置

| テストファイル | 内容 | 行数 |
|--------------|------|------|
| `tests/runtime_core_types_test.rs` | InstanceState 遷移、EvaluatedValue、RuntimeError、Interpolator | 366 |
| `tests/integration_test.rs` | DolaDocument serde ラウンドトリップ（JSON/TOML/YAML） | 741 |
| `tests/compile_test.rs` | compile_storyboard 単体テスト | — |
| `tests/compile_integration_test.rs` | コンパイラ統合テスト | — |

### 1.3 依存関係と Feature Gate

```toml
# 現在の Cargo.toml
[dependencies]
serde = { version = "1", features = ["derive"] }

[dependencies.interpolation]
version = "0.3.0"
optional = true

[features]
runtime = ["dep:interpolation"]
```

- `runtime` feature がまだ存在。統合指針では Tier 1 で削除予定だったが、core-types は feature gate 内で実装完了
- `toml` クレートは `optional = true`。facade の `DocumentStore` はデシリアライズ済み `DolaDocument` を受け取るため、シリアライズ形式への依存なし

### 1.4 コーディング慣習

| 項目 | 慣習 |
|------|------|
| 可視性 | Tier 1 型は `pub`（`mod.rs` で re-export）。内部型は `pub(crate)` |
| テスト配置 | 単体テストは `#[cfg(test)] mod tests` 埋め込み。統合テストは `tests/` |
| エラー戦略 | `Result<T, RuntimeError>`。`?` 操作子対応（`From` 実装あり） |
| unsafe | 不使用 |
| ドキュメント | `///` + `//!` 日本語（モジュールヘッダは英語） |

---

## 2. 要件ごとのギャップ分析

### 2.1 要件 → 既存資産マップ

| 要件 | 既存資産 | ギャップ | タグ |
|------|---------|---------|------|
| **Req 1**: 指示書受信 | `DolaDocument`（serde 定義済み） | **Missing**: `DocumentStore` 構造体（`DolaDocument` 保持のみ、パースは外部責務） |
| **Req 2**: 指示書差し替え | `DolaDocument` フィールドは全て `BTreeMap` | **Missing**: 変数引き継ぎロジック、凍結状態管理 |
| **Req 3**: Start | `compile_storyboard()` 完全実装、`CompiledStoryboard` | **Missing**: `InstanceManager`、group_id 生成、タイムテーブル挿入 |
| **Req 4**: Start エラー | `RuntimeError::StoryboardNotFound`, `ZeroDurationWithLoop` 定義済み | **Missing**: `calculate_end_time()` 実装、`loop_count` + `total_base_duration` からの計算 |
| **Req 5**: 制御コマンド | `InstanceState::try_transition()` 完全実装 | **Missing**: `StoryboardInstance` 構造体、Pause/Resume 時間計算、Finish deadline |
| **Req 6**: 購読管理 | なし | **Missing**: `SubscriptionManager` 全体 |
| **Req 7**: Update 差分配信 | `Interpolator::interpolate()` 完全実装 | **Missing**: `TimelineManager.evaluate()`、差分検出 |
| **Req 8**: タイムテーブル | `CompiledSegment` の `start_time`/`end_time` | **Missing**: `VariableTimeline`, `TimelineEntry`, effective_time 計算 |
| **Req 9**: 状態遷移適用 | `InstanceState::try_transition()`, `from_policy()`, `is_terminal()` | **Missing**: InstanceManager でのラッピング（RuntimeError 変換） |
| **Req 10**: 同時再生 | `HashMap` ベースの設計（制約なし） | 設計で保証（人為的上限なし） |
| **Req 11**: Tier 2 暫定 | なし | **Missing**: 最新 group_id 優先ロジック、loop_count 無視、拡張ポイント設計 |

### 2.2 重要な不整合

#### ⚠️ `runtime` Feature Gate 未削除

- **統合指針**: 「仕様1 (1-core-types) 実装時: `interpolation = "0.3.0"` 常時依存化（runtime feature 削除）」
- **現状**: `Cargo.toml` に `runtime = ["dep:interpolation"]` が残存。`lib.rs` も `#[cfg(feature = "runtime")]`
- **影響**: facade の5モジュールも `runtime` feature gate 内に配置するか、このタイミングで削除するかの選択
- **タグ**: **Constraint** — 設計フェーズで方針決定要

#### ✅ `toml` Feature と `load_document`（解決済み）

- **Req 1 修正済み**: `load_document(doc: DolaDocument)` はデシリアライズ済みオブジェクトを受け取る
- シリアライズ形式（TOML/JSON/YAML）の選択は呼び出し側の責務であり dola スコープ外
- `toml` feature は既存 serde ラウンドトリップ用途で維持。facade に影響なし

---

## 3. 実装アプローチ評価

### Option A: 統合指針準拠の新規モジュール追加（推奨）

design.md が既に詳細なコンポーネント設計を持っているため、そのまま5ファイルを新規追加する。

| 新規ファイル | 責務 | 推定行数 |
|------------|------|---------|
| `document_store.rs` | DolaDocument 保持、差し替え | ~60 |
| `instance_manager.rs` | StoryboardInstance + HashMap 管理 | ~200 |
| `timeline_manager.rs` | VariableTimeline + evaluate + effective_time | ~250 |
| `subscription_manager.rs` | 購読状態 + 差分検出 | ~120 |
| `facade.rs` | DolaRuntime（委譲のみ） | ~200 |

**Trade-offs**:
- ✅ design.md と 1:1 対応で追跡容易
- ✅ 既存コード変更は `mod.rs` の `mod` 追加のみ
- ✅ Tier 3 拡張ポイントを自然に設計可能
- ❌ 5ファイル同時追加（ただし各ファイルは小規模）

### Option B: Facade 統合型（単一ファイル）

全コンポーネントを `facade.rs` 1ファイルに統合。

**Trade-offs**:
- ✅ ファイル数最小（1ファイル）
- ❌ 800行超の大ファイル、責務分離不明確
- ❌ Tier 3 での ConflictResolver/LoopController 注入が困難
- ❌ テスト分離が困難

### Option C: Trait ベース抽象化

各コンポーネントに trait を定義し、テスト時にモック差し替え可能にする。

**Trade-offs**:
- ✅ テスタビリティ最大
- ✅ Tier 3 での差し替えが型安全
- ❌ 過剰設計。内部コンポーネント（`pub(crate)`）に trait は不要
- ❌ Rust の所有権モデルでは、`&mut self` を持つ trait の合成が煩雑

### 推奨: **Option A**

design.md の設計がすでに十分に詳細であり、Option A がそのまま実装パスとなる。Tier 3 拡張はコメントベースのフックポイントで十分（trait 不要）。

---

## 4. 技術的課題と Research Needed

### 4.1 effective_time 計算の精度

```
effective_time = (current_time - start_time - pause_accumulated) * time_scale
```

- **懸念**: f64 精度（長時間稼働時の蓄積誤差）
- **緩和**: dola はデスクトップマスコット用途で、アニメーション時間は数十秒程度。f64 精度で十分
- **タグ**: Low Risk

### 4.2 変数引き継ぎの実装

- **Req 2.2-2.3**: 指示書差し替え時の同名変数引き継ぎと消失変数の凍結
- **課題**: TimelineManager 内の `VariableTimeline` と SubscriptionManager の `last_values` の両方を更新する必要がある
- **アプローチ**: `DolaRuntime::load_document()` 内で旧 document の変数名セットと新 document の変数名セットを比較し、差分ロジックを適用
- **タグ**: Medium Complexity

### 4.3 Finish deadline の自動実行

- **Req 5.5**: `finish(group_id, offset)` — 時間経過後に Conclude 相当
- **課題**: facade はタイマーを持たない（pull 型設計）。deadline チェックは `update()` 内で行う必要がある
- **アプローチ**: `update()` 呼び出し時に `finish_deadline` を持つインスタンスを走査し、`current_time >= finish_deadline` なら自動 Conclude
- **タグ**: **Research Needed** — design フェーズで evaluate フロー内での位置を確定

### 4.4 Conclude / Cancel のタイムテーブル操作

- **Req 5.3 (Conclude)**: 現在セグメントの最終値ジャンプ + 未開始スキップ
- **Req 5.4 (Cancel)**: 現在値凍結
- **課題**: TimelineManager の `evaluate()` が返す値と、InstanceManager の状態遷移を協調させる必要がある
- **アプローチ**: Conclude → 全セグメントの最終値で `last_values` 更新後にエントリ削除。Cancel → 現在値で `last_values` 更新後にエントリ削除
- **タグ**: Medium Complexity

---

## 5. 工数・リスク評価

### 工数: **M（3〜7日）**

- Tier 1 基盤が完成済みで型が確定
- design.md が詳細なインターフェース定義を持つ
- 新規パターン（タイムテーブル管理、差分検出）はあるが、アルゴリズム的に難易度は中程度
- 5モジュール × 平均150行 ≈ 750行の新規コード + テスト（同規模以上）

### リスク: **Medium**

| リスク要因 | 影響度 | 確率 | 緩和策 |
|-----------|--------|------|--------|
| `runtime` feature gate 判断 | 中 | 確実 | 設計フェーズで方針決定 |
| 変数引き継ぎ複雑性 | 中 | 中 | design.md の load_document フロー精緻化 |
| Finish deadline pull 型実装 | 低 | 低 | update() 内チェックで解決 |
| Tier 3 拡張ポイント不足 | 中 | 低 | コメント + 構造的分離で十分 |

---

## 6. 設計フェーズへの引き継ぎ事項

### 必須決定事項

1. **`runtime` feature gate**: 現行維持 or このタイミングで削除
2. **`load_document` 返り値**: パース責務の外部化に伴い、`Result<(), RuntimeError>` を維持するか infallible (`fn load_document(&mut self, doc: DolaDocument)`) にするか

### Research Needed（設計フェーズで調査）

1. **Finish deadline の update() 内での位置**: evaluate ループ前 or 後
2. **Conclude/Cancel 時のタイムテーブル操作手順**: 値取得→状態遷移→エントリ削除の順序
3. **指示書差し替え時の再生中インスタンスの扱い**: 即座に Conclude/Cancel するか、自然終了を待つか

### 推奨アプローチ

- **Option A（新規モジュール追加）** を採用
- design.md の設計をベースに、上記の不整合と未決定事項を反映
- テスト戦略: 各コンポーネントの単体テスト → facade 経由の統合テスト
