# Research Notes — dola-runtime-1-core-types

## 調査概要

本仕様は Extension（既存実装の追認・整備）に分類。実装の約80%が完了済みであり、Light Discovery プロセスを適用した。

---

## 1. Extension Point Analysis

### 実装状況

| ファイル | 行数 | 完成度 | 備考 |
|---------|------|--------|------|
| `runtime/mod.rs` | 13 | 100% | re-export のみ |
| `runtime/instance_state.rs` | 72 | 100% | 全メソッド実装済み、テスト済み |
| `runtime/types.rs` | 89 | 95% | `From<Vec<DolaError>>` 未実装 |
| `runtime/interpolator.rs` | 343 | 100% | 14テスト + 30バリアントマッピングテスト |

### 既存パターンとの整合

- **モジュール構成**: `runtime/` は `mod.rs` + 個別ファイル。既存 dola の `compile.rs`, `easing.rs` 等と同じフラット構成
- **可視性**: `pub` で re-export。既存パターン（`pub use`）に準拠
- **エラー型**: `RuntimeError` は既存 `DolaError` と並列の独立エラー型
- **テスト配置**: 単体テストは `interpolator.rs` 内 `#[cfg(test)]` モジュール、統合テストは `tests/runtime_core_types_test.rs`

### 後方互換性

新規モジュール（feature gate `runtime` 背後）のため、既存コードへの影響なし。

---

## 2. Dependency Check

### interpolation 0.3.0

- **使用 API**: `Ease` trait（`f64::calc()`）, `EaseFunction` enum, `quad_bez()`, `cub_bez()`, `lerp()`
- **feature gate**: `runtime = ["dep:interpolation"]` で条件付き依存
- **互換性**: Cargo.toml に既に定義済み、全テストパス
- **ライセンス**: Apache-2.0（プロジェクト互換）

### 既存 dola 型の消費

| 消費元型 | 定義モジュール | 消費方法 |
|---------|---------------|---------|
| `InterruptionPolicy` | `storyboard.rs` | `from_policy()` の引数 |
| `CompiledSegment` | `compile.rs` | `interpolate()` の入力 |
| `VariableTypeHint` | `compile.rs` | 型別ディスパッチ |
| `EasingFunction` / `EasingName` | `easing.rs` | イージングマッピング |
| `ParametricEasing` | `easing.rs` | ベジェ曲線計算 |
| `TransitionValue` | `transition.rs` | from/to 値取得 |
| `DynamicValue` | `value.rs` | Object 型ラップ |
| `DolaError` | `error.rs` | CompileError ラップ |

API契約変更なし。全消費パスが安定。

---

## 3. 設計判断の記録

### 3.1 `from_policy(Never)` → `Option<Self>`

- **背景**: 親仕様設計時は `panic!` を採用していたが、Never は競合解決時に正常に呼ばれ得るパス
- **決定**: `Option<Self>` を返し、`None` で「対応する終了状態なし」を表現
- **根拠**: パニック禁止原則。正常パスで `panic!` は Rust 慣習に反する

### 3.2 `try_transition` → `Result<InstanceState, InstanceState>`

- **背景**: 親仕様設計時は `Result<(), RuntimeError>` を採用
- **決定**: `Result<InstanceState, InstanceState>` に変更
- **根拠**: ドメイン層（InstanceState）は API 層（RuntimeError）に依存すべきでない。InstanceManager が必要に応じて RuntimeError に変換する責務を持つ

### 3.3 RuntimeError 5バリアント化

- **背景**: 親仕様設計時は 7バリアント（TerminatedInstance, InvalidStateTransition 含む）
- **削除**: `TerminatedInstance` — 終了インスタンスは即座に削除されるため、「終了済みへの操作」は `InvalidGroupId` で統一
- **削除**: `InvalidStateTransition` — try_transition が `Result<InstanceState, InstanceState>` を返すため、InstanceState 層で完結。実装が正しければ発生しないパス
- **根拠**: 終了時即削除の設計方針（ライトオンリー）。レイヤー分離原則

### 3.4 `CompileError(Vec<DolaError>)`

- **背景**: `compile_storyboard()` が `Vec<DolaError>` を返す既存 API
- **決定**: `CompileError(Vec<DolaError>)` で複数エラーを保持
- **From 変換**: `From<Vec<DolaError>>` を提供し `?` 演算子対応

### 3.5 Interpolator の構造

- **実装パターン**: `Interpolator` はステートレスな ZST（zero-sized type）。内部ロジックは自由関数
- **根拠**: 状態を持たない純粋な計算。関連関数のグルーピングのみが目的

---

## 4. Integration Risk Assessment

| リスク | 影響度 | 確率 | 緩和策 |
|--------|--------|------|--------|
| 既存コードへの影響 | なし | — | feature gate で完全隔離 |
| パフォーマンス | なし | — | 全操作 O(1) 算術演算 |
| セキュリティ | なし | — | I/O なし、unsafe なし |
| テスト不足 | 低 | 低 | 14 単体 + 統合テスト既存 |

### 残作業

- `From<Vec<DolaError>>` 実装（Req 4 AC4）
- `use super::InstanceState` の未使用 import 整理（types.rs）

---

## 5. テスト方針

### 既存テスト（パス済み）

- `interpolator.rs`: 14テスト（線形、境界値、クランプ、Integer丸め、Object切替、イージング別、ベジェ、30バリアント）
- `tests/runtime_core_types_test.rs`: 統合テスト（InstanceState遷移、from_policy、is_terminal、Display）

### 追加検討

- `From<Vec<DolaError>>` 変換テスト（実装後）
- `RuntimeError::Display` 全5バリアント出力テスト
