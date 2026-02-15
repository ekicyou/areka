# 実装計画 — dola-runtime-2-clock

## タスク概要

feature gate `windows-clock` 下の時刻取得ユーティリティ `clock::now()` を実装する。Win32 `GetTickCount64` を使用し、OS 起動時からの経過秒数を f64 で返す。

---

## 実装タスク

- [ ] 1. Cargo.toml のセットアップ
  - `Cargo.toml` の `runtime` feature を削除し、`interpolation` を常時依存に変更（BREAKING CHANGE）
  - `[target.'cfg(windows)'.dependencies]` で `windows` クレートを追加（`workspace = true`, `features = ["Win32_System_Performance"]`）
  - `lib.rs` の `#[cfg(feature = "runtime")] pub mod runtime;` を `pub mod runtime;` に変更
  - `runtime/mod.rs` に `#[cfg(target_os = "windows")] pub mod clock;` を追加
  - `cargo build` が Windows で成功し、非 Windows で clock モジュールが除外されることを確認
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4_

- [ ] 2. clock::now() の実装
  - `crates/dola/src/runtime/clock.rs` を作成
  - `#[cfg(target_os = "windows")]` でファイル全体を囲む
  - `QueryPerformanceCounter` と `QueryPerformanceFrequency` を使った高精度時刻取得を実装
  - counter / frequency による秒数計算（f64）
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4_

- [ ] 3. テストの作成
  - 単調増加テスト: 2回呼び出しで2回目 ≥ 1回目
  - 非ゼロテスト: 戻り値 > 0.0
  - 精度テスト: sleep(100ms) 前後の差分が 0.08..0.15 の範囲
  - `#[cfg(all(test, target_os = "windows"))]` でテストモジュールを囲む
  - `cargo test` で全テスト通過を確認
  - _Requirements: 1.1, 1.2, 1.3, 6.1, 6.2, 6.3, 6.4_
