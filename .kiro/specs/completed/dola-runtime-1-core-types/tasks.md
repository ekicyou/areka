# 実装計画 — dola-runtime-1-core-types

## タスク概要

Tier 1 基盤型（InstanceState, EvaluatedValue, RuntimeError, StartResult）およびイージング補間計算（Interpolator）の実装完成度検証。feature gate `runtime` 下の `crates/dola/src/runtime/` サブモジュールとして既に約95%実装済み。残作業は最終検証と追加テストケース（必要時）。

---

## 実装タスク

- [x] 1. プロジェクト基盤のセットアップ（完了）
  - `Cargo.toml` に `interpolation` 依存と `runtime` feature を追加済み
  - `crates/dola/src/runtime/mod.rs` で re-export 定義済み
  - `crates/dola/src/lib.rs` に `#[cfg(feature = "runtime")] mod runtime;` 追加済み
  - _Requirements: —（基盤セットアップ）_

- [x] 2. InstanceState と状態遷移ロジックの実装（完了）
  - 7バリアント enum 定義済み
  - `is_terminal()` 実装済み
  - `from_policy() -> Option<Self>` 実装済み（Never → None）
  - `try_transition() -> Result<InstanceState, InstanceState>` 実装済み
  - 全遷移パターンテスト実装済み
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 3. EvaluatedValue, RuntimeError, StartResult の実装（完了）
  - `EvaluatedValue` 3バリアント + `Display` 実装済み
  - `RuntimeError` 5バリアント + `Display` / `Error` / `From<Vec<DolaError>>` 実装済み
  - `StartResult` 構造体定義済み
  - 各型のテスト実装済み
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3_

- [x] 4. Interpolator の実装（完了）
  - `EasingName` → `EaseFunction` 30バリアントマッピング実装済み
  - `apply_easing()` 全パス実装済み
  - `interpolate()` 型別ディスパッチ実装済み
  - 全ヘルパー関数実装済み
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9, 7.1, 7.2, 7.3_

- [x] 5. 包括的テストの実装（完了）
  - 単体テスト 14件実装済み（全パス）
  - 統合テスト 34件実装済み（全パス）
  - 30バリアント全マッピングテスト実装済み
  - 境界値・丸め・即時切替・パラメトリックテスト実装済み
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9, 7.1, 7.2, 7.3_

- [x] 6. 実装完成度の最終検証
  - 全7要件の各 Acceptance Criteria を実装・テストと照合し充足を確認 ✅
  - `From<Vec<DolaError>>` の動作テストを追加（`?` 演算子による自動変換確認）✅
  - `cargo clippy --features runtime` でコード品質を確認（runtime モジュール警告ゼロ）✅
  - `cargo test --features runtime --all-targets` で全テスト通過を最終確認 ✅
  - すべての public 型・メソッドにドキュメントコメントが存在するか確認（26/26 完全）✅
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9, 7.1, 7.2, 7.3_

---

## 完成基準

- ✅ 全48テスト（14 unit + 34 integration）がパス
- ✅ 全7要件の Acceptance Criteria 充足
- ✅ clippy 警告ゼロ
- ✅ すべての public API にドキュメントコメント
- ✅ 既存 dola クレートとの統合が正常動作
