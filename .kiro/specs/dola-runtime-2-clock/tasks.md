# 実装計画 — dola-runtime-2-clock

## タスク概要

feature gate `windows-clock` 下の時刻取得ユーティリティ `clock::now()` を実装する。Win32 `GetTickCount64` を使用し、OS 起動時からの経過秒数を f64 で返す。

---

## 実装タスク

- [ ] 1. Cargo.toml と feature gate のセットアップ
  - `Cargo.toml` の `runtime` feature を削除し、`interpolation` を常時依存に変更（BREAKING CHANGE）
  - `Cargo.toml` に `windows-clock = ["dep:windows"]` feature と `windows` オプショナル依存を追加
  - `lib.rs` の `#[cfg(feature = "runtime")] pub mod runtime;` を `pub mod runtime;` に変更
  - `runtime/mod.rs` に `#[cfg(feature = "windows-clock")] pub mod clock;` を追加
  - `cargo build --features windows-clock` が成功することを確認
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5_

- [ ] 2. clock::now() の実装
  - `crates/dola/src/runtime/clock.rs` を作成
  - `GetTickCount64() as f64 / 1000.0` による時刻取得を実装
  - `#[cfg(feature = "windows-clock")]` で囲む
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4_

- [ ] 3. テストの作成
  - 単調増加テスト: 2回呼び出しで2回目 ≥ 1回目
  - 非ゼロテスト: 戻り値 > 0.0
  - 精度テスト: sleep(100ms) 前後の差分が 0.08..0.15 の範囲
  - `cargo test --features runtime,windows-clock` で全テスト通過を確認
  - _Requirements: 1.1, 1.2, 1.3_
