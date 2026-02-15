# 実装計画 — dola-runtime-2-clock

## タスク概要

OS 起動時からの高精度な現在時刻（f64 秒）を取得するユーティリティ関数 `clock::now()` を実装する。QueryPerformanceCounter / QueryPerformanceFrequency ベースの Windows 専用実装。

---

## 実装タスク

- [ ] 1. (P) Cargo.toml と module 構成のセットアップ
  - `crates/dola/Cargo.toml` に `[target.'cfg(windows)'.dependencies]` セクションを追加
  - `windows = { workspace = true, features = ["Win32_System_Performance"] }` を定義
  - `crates/dola/src/runtime/mod.rs` に `#[cfg(target_os = "windows")] pub mod clock;` を追加して clock サブモジュールを条件付き公開
  - `cargo build` で Windows ターゲット時にビルド成功を確認
  - 非 Windows ターゲット（例: `--target x86_64-unknown-linux-gnu`）で clock モジュールが除外されることを確認
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 5.1_

- [ ] 2. clock::now() 関数の実装
  - `crates/dola/src/runtime/clock.rs` を新規作成
  - ファイル全体を `#[cfg(target_os = "windows")]` で囲む
  - `pub fn now() -> f64` 関数を定義（ドキュメントコメント付き）
  - QueryPerformanceCounter で現在カウントを取得し、QueryPerformanceFrequency で周波数を取得
  - `(counter as f64) / (frequency as f64)` で秒数を計算して返す
  - `use` 文を関数内に局所化し、`unsafe` ブロックを Win32 API 呼び出しのみに限定（wintf の `FrameTime::get_precise_time()` パターン準拠）
  - ステートレス設計（グローバル状態なし、frequency を毎回取得）
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 2.4, 5.2, 5.3_

- [ ] 3. ユニットテストの実装
  - `clock.rs` 内に `#[cfg(all(test, target_os = "windows"))] mod tests` テストモジュールを追加
  - 単調増加テスト: `now()` を連続 2 回呼び出し、`t2 >= t1` であることを `assert!` で検証
  - 正の有限値テスト: `now()` の戻り値が `value > 0.0 && value.is_finite()` であることを検証
  - ms 精度テスト: `std::thread::sleep(Duration::from_millis(1))` 前後で `now()` の差分が `> 0.0` であることを検証
  - `cargo test` で全テスト通過を確認（Windows 環境）
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [ ]* 4. パフォーマンスベンチマークの追加（任意）
  - `now()` 関数のベンチマークを追加（`criterion` または `std::time::Instant` による手動計測）
  - 「性能影響はナノ秒オーダー」の主張を検証（Req 2.4 の frequency 毎回取得コストの妥当性確認）
  - ベンチマーク結果を `.kiro/specs/dola-runtime-2-clock/research.md` に追記
  - 60FPS（16.67ms フレーム間隔）に対して無視できるオーバーヘッドであることを確認
  - _Requirements: 2.4_
