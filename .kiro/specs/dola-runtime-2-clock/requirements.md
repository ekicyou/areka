# Requirements Document — dola-runtime-2-clock

## Introduction

本ドキュメントは dola ランタイムエンジンの時刻取得ユーティリティを定義する子仕様 `dola-runtime-2-clock` の機能要件を定義する。親仕様 `dola-runtime-engine` の Req 11（時刻ユーティリティ）を子仕様の粒度に詳細化する。

本子仕様は Tier 1（基盤）に位置し、他の子仕様への依存を持たない。facade は clock を直接参照せず、利用者（wintf 等）が `clock::now()` で時刻を取得して `runtime.update(subscriber_id, time)` に渡す設計。

> 統合指針: `.kiro/specs/dola-runtime-engine/integration-guide.md` Section 2.1 参照

---

## Requirements

### Requirement 1: 時刻取得関数の提供

_Parent: Req 11.1_

**Objective:** ランタイム利用者として、OS 起動時からの高精度な現在時刻を f64 秒として取得したい。これにより、`runtime.update(subscriber_id, time)` 呼び出し時の時刻指定が簡便になる。

#### Acceptance Criteria

1. The Clock module shall `now() -> f64` 公開関数を提供する。
2. The `now()` function shall OS 起動時からの経過秒数を f64 で返す。
3. The `now()` function shall 単調増加であること（同一プロセス内で前回呼び出し値以上を保証）。
4. The `now()` function shall ms 精度以上を保証する（16.67ms = 60fps フレーム間隔を十分に識別可能）。

---

### Requirement 2: 実装手段の選定

_Parent: Req 11.2, 11.3_

**Objective:** ランタイム実装者として、OS 起動時起点の時刻を取得する適切な手段を選定・使用したい。gap-analysis の調査結果に基づき、要件に合致する実装を確定する。

#### Acceptance Criteria

1. The Clock module shall Win32 API `GetTickCount64` を使用して時刻を取得する。
2. The Clock module shall `GetTickCount64() as f64 / 1000.0` の演算で OS 起動時からの経過秒数（f64）を生成する。
3. The Clock module shall `unsafe` ブロックを時刻取得の Win32 API 呼び出し箇所のみに限定する。
4. The Clock module shall 外部クレート依存を追加しない（`windows` クレートの `Win32_System_SystemInformation` feature のみ使用）。

> **設計根拠**: gap-analysis Section 4.2 より、「OS 起動時からの秒数（f64）」要件に合致する手段は `GetTickCount64`（ms 精度）と `IUIAnimationTimer::GetTime()`（COM 依存）の2つ。dola クレートの COM 非依存方針により `GetTickCount64` を採用。`quanta` / `std::time::Instant` / QPC は OS 起動時起点ではないため不適格。

---

### Requirement 3: Feature Gate 分離

_Parent: 統合指針 Section 5_

**Objective:** ランタイム実装者として、時刻取得ユーティリティを独立した feature gate で分離したい。これにより、Windows 以外の環境でもランタイムコア（`runtime` feature）を単独でビルド・テスト可能にする。

#### Acceptance Criteria

1. The Clock module shall `windows-clock` feature gate で有効化される。
2. The `windows-clock` feature shall `runtime` feature とは独立であること（`runtime` は `windows-clock` を暗黙に有効化しない）。
3. The Clock module shall `crates/dola/src/runtime/clock.rs` に配置し、`#[cfg(feature = "windows-clock")]` で条件コンパイルする。
4. When `windows-clock` feature が無効な場合, the Clock module shall コンパイル対象から完全に除外される。
5. The `windows-clock` feature shall `Cargo.toml` で `windows` クレートへのオプショナル依存を有効化する（`windows-clock = ["dep:windows"]`）。

---

### Requirement 4: Cargo.toml 依存定義

_Parent: 統合指針 Section 5.2_

**Objective:** ランタイム実装者として、`windows-clock` feature に必要な依存クレートを正しく定義したい。

#### Acceptance Criteria

1. The `Cargo.toml` shall `windows` クレートをオプショナル依存として追加する（`optional = true`）。
2. The `windows` dependency shall `Win32_System_SystemInformation` feature を指定する（`GetTickCount64` が含まれるモジュール）。
3. The `windows` dependency shall ワークスペースの既存バージョン（0.62）と整合する。
4. The `default` feature shall `windows-clock` を含まない。

---

### Requirement 5: モジュール公開と re-export

_Parent: 統合指針 Section 2.4, 5.3_

**Objective:** ランタイム利用者として、`dola::runtime::clock::now()` のパスで時刻取得関数にアクセスしたい。公開 API 境界を統合指針に準拠させる。

#### Acceptance Criteria

1. The `runtime/mod.rs` shall `windows-clock` feature 有効時に `clock` サブモジュールを条件付きで公開する（`#[cfg(feature = "windows-clock")] pub mod clock;`）。
2. The `clock::now()` function shall `pub` 可視性を持つ。
3. The Clock module shall `clock::now()` 以外の内部実装詳細を公開しない。

---

### Requirement 6: テスト可能性

_Parent: 統合指針 Section 6.1_

**Objective:** ランタイム実装者として、Clock モジュールの正確性を自動テストで検証したい。

#### Acceptance Criteria

1. The Clock module shall 時刻の単調増加性を検証するユニットテストを持つ（連続2回の `now()` 呼び出しで `t2 >= t1`）。
2. The Clock module shall 返却値が正の有限値であることを検証するユニットテストを持つ。
3. The Clock module shall ms 精度を検証するテストを持つ（`std::thread::sleep(1ms)` 後の差分が 0 より大きい）。
4. The Clock module shall `#[cfg(test)]` テストモジュールを `clock.rs` 内に配置する、または `tests/` ディレクトリに統合テストファイルを配置する。

