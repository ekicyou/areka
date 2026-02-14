# Requirements Document — dola-runtime-2-clock

## Introduction

本ドキュメントは dola ランタイムエンジンの時刻取得ユーティリティを定義する子仕様 `dola-runtime-clock` の機能要件を定義する。親仕様 `dola-runtime-engine` の Req 11（時刻ユーティリティ）を子仕様の粒度に詳細化する。

本子仕様は Tier 1（基盤）に位置し、他の子仕様への依存を持たない。facade は clock を直接参照せず、利用者（wintf 等）が `clock::now()` で時刻を取得して `runtime.update(subscriber_id, time)` に渡す設計。

> 統合指針: `.kiro/specs/dola-runtime-engine/integration-guide.md` Section 2.1 参照

---

## Requirements

### Requirement 1: 時刻取得関数の提供

_Parent: Req 11.1_

**Objective:** ランタイム利用者として、OS 起動時からの高精度な現在時刻を f64 秒として取得したい。これにより、`update()` 呼び出し時の時刻指定が簡便になる。

#### Acceptance Criteria

1. The Clock module shall `now() -> f64` 関数を提供する。
2. The `now()` function shall OS 起動時からの経過秒数を f64 で返す。
3. The `now()` function shall 単調増加であること（同一プロセス内で前回値以上を保証）。

---

### Requirement 2: 実装手段の選定

_Parent: Req 11.2, 11.3_

**Objective:** ランタイム実装者として、適切な時刻取得手段を選定して使用したい。

#### Acceptance Criteria

1. The Clock module shall まず適切な既存クレートの有無を調査する。
2. If 適切なクレートが存在する場合, then the Clock module shall そのクレートを使用する。
3. If 適切なクレートが存在しない場合, then the Clock module shall Win32 API `GetTickCount64` を使用して `GetTickCount64() as f64 / 1000.0` で時刻を生成する。
4. The Clock module shall ms 精度を最低要件とする（60fps アニメーションに十分）。

---

### Requirement 3: Feature Gate 分離

_Parent: 統合指針 Section 5_

**Objective:** ランタイム実装者として、時刻取得ユーティリティを独立した feature gate で分離したい。これにより、Windows 以外の環境でもランタイムコア（`runtime` feature）を単独でビルド・テスト可能にする。

#### Acceptance Criteria

1. The Clock module shall `windows-clock` feature gate で有効化される。
2. The `windows-clock` feature shall `runtime` feature とは独立であること（`runtime` は `windows-clock` を暗黙に有効化しない）。
3. The Clock module shall `crates/dola/src/runtime/clock.rs` に配置し、`#[cfg(feature = "windows-clock")]` で囲む。
4. When `windows-clock` feature が無効な場合, the Clock module shall コンパイル対象から除外される。

