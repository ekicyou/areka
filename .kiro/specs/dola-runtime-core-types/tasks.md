# 実装計画 — dola-runtime-core-types

## タスク概要

Tier 1 基盤型（InstanceState, EvaluatedValue, RuntimeError, StartResult）およびイージング補間計算（Interpolator）を実装する。feature gate `runtime` 下の `crates/dola/src/runtime/` サブモジュールとして構築する。

---

## 実装タスク

- [ ] 1. プロジェクト基盤のセットアップ
  - `Cargo.toml` に `interpolation = { version = "0.3.0", optional = true }` と `[features] runtime = ["dep:interpolation"]` を追加
  - `crates/dola/src/runtime/mod.rs` を作成し、サブモジュール宣言と `pub(crate)` re-export を定義
  - `crates/dola/src/lib.rs` に `#[cfg(feature = "runtime")] mod runtime;` を追加
  - `cargo build --features runtime` が成功することを確認
  - _Requirements: —（基盤セットアップ）_

- [ ] 2. InstanceState と状態遷移ロジックの実装 (P)
  - `crates/dola/src/runtime/instance_state.rs` を作成
  - 7バリアント enum（Created, Playing, Paused, Concluded, Cancelled, Trimmed, Compressed）を定義
  - `is_terminal()`, `from_policy()`, `try_transition()` メソッドを実装
  - 全42遷移パターン（7×7 - 対角7 = 42）の単体テストを作成
  - `from_policy()` の4正常パターン + Never panic テストを作成
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7_

- [ ] 3. EvaluatedValue, RuntimeError, StartResult の実装 (P)
  - `crates/dola/src/runtime/types.rs` を作成
  - `EvaluatedValue` 3バリアント + `Display` 実装
  - `RuntimeError` 7バリアント + `Display` / `Error` / `From<DolaError>` 実装
  - `StartResult` 構造体定義
  - 各型の構築・Display・比較テストを作成
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3_

- [ ] 4. Interpolator の実装
  - `crates/dola/src/runtime/interpolator.rs` を作成
  - `EasingName` → `EaseFunction` 30バリアントの1対1マッピング（`map_easing()`）
  - `apply_easing()` — Named / Parametric(QB) / Parametric(CB) / None の分岐
  - `interpolate()` — VariableTypeHint 別ディスパッチ（Float / Integer / Object）
  - `extract_scalar()`, `extract_object()` ヘルパー
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9, 7.1, 7.2_

- [ ] 5. Interpolator の包括的テスト
  - `crates/dola/tests/runtime_core_types_test.rs` 統合テストファイルを作成
  - 30バリアント全マッピング正確性テスト
  - 境界値テスト: t=0.0, t=1.0, t=-0.1(clamp), t=1.5(clamp)
  - Float 補間精度テスト: 線形、各イージングカーブの代表的 t 値
  - Integer 丸めテスト: 0.5 境界の round() 動作確認
  - Object 即時切替テスト: t < 1.0 → from, t >= 1.0 → to
  - QuadraticBezier / CubicBezier パラメトリック値範囲テスト
  - 全31イージング（30 Named + Linear）の非NaN検証
  - `cargo test --features runtime` で全テスト通過を確認
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 6.8, 6.9, 7.1, 7.2, 7.3_
