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

**Objective:** ランタイム実装者として、OS 起動時起点の高精度時刻を取得する適切な手段を選定・使用したい。アニメーションエンジンにおけるフレーム間の微小な時間差を正確に計測するため、ハードウェアレベルの高精度タイマーを使用する。

#### Acceptance Criteria

1. The Clock module shall Win32 API `QueryPerformanceCounter` および `QueryPerformanceFrequency` を使用して時刻を取得する。
2. The Clock module shall QueryPerformanceCounter の戻り値を QueryPerformanceFrequency で除算し、OS 起動時からの経過秒数（f64）を生成する。
3. The Clock module shall `unsafe` ブロックを時刻取得の Win32 API 呼び出し箇所のみに限定する。
4. The Clock module shall 外部クレート依存を追加しない（`windows` クレートの `Win32_System_Performance` feature のみ使用）。

> **設計根拠**: アニメーションエンジンであれば、フレーム間の微小な時間差（<1ms）を正確に計測する必要がある。`GetTickCount64` は分解能が 10～16ms と低精度であり不適格。`QueryPerformanceCounter` はハードウェアタイマーを使用し、マイクロ秒級の高精度を提供する。OS 起動時起点であり、マルチプロセスで共有可能。COM 依存の `IUIAnimationTimer::GetTime()` よりもシンプルで、dola クレートの COM 非依存方針に合致する。

---

### Requirement 3: 条件コンパイル (Windows 専用)

_Parent: 統合指針 Section 5_

**Objective:** ランタイム実装者として、Windows 専用の時刻取得ユーティリティを OS 条件コンパイルで分離したい。これにより、非 Windows 環境でも dola クレート全体をビルド可能にする。

> **設計決定**: feature gate ではなく `#[cfg(target_os = "windows")]` を使用する。clock::now() は完全なユーティリティ関数であり、利用者の選択肢ではなく OS の自動判定で十分。`runtime` feature も本仕様実装時に削除済み（常時有効化）。

#### Acceptance Criteria

1. The Clock module shall `crates/dola/src/runtime/clock.rs` に配置し、`#[cfg(target_os = "windows")]` で条件コンパイルする。
2. The `runtime/mod.rs` shall `#[cfg(target_os = "windows")] pub mod clock;` で clock サブモジュールを条件付き公開する。
3. When Windows 以外の OS でビルドする場合, the Clock module shall コンパイル対象から完全に除外される。
4. The `runtime/` モジュール自体には条件コンパイルを設定しない（`lib.rs` の `pub mod runtime;` は無条件）。

---

### Requirement 4: Cargo.toml 依存定義

_Parent: 統合指針 Section 5.2_

**Objective:** ランタイム実装者として、Windows ターゲット時に必要な依存クレートを正しく定義したい。

#### Acceptance Criteria

1. The `Cargo.toml` shall `[target.'cfg(windows)'.dependencies]` セクションで `windows` クレートを定義する。
2. The `windows` dependency shall `workspace = true` でワークスペースバージョンを参照する。
3. The `windows` dependency shall `features = ["Win32_System_Performance"]` を指定する（`QueryPerformanceCounter` / `QueryPerformanceFrequency` が含まれるモジュール）。
4. The `windows` dependency shall Windows 以外の OS では依存関係に含まれない。

---

### Requirement 5: モジュール公開と re-export

_Parent: 統合指針 Section 2.4, 5.3_

**Objective:** ランタイム利用者として、`dola::runtime::clock::now()` のパスで時刻取得関数にアクセスしたい。公開 API 境界を統合指針に準拠させる。

#### Acceptance Criteria

1. The `runtime/mod.rs` shall Windows ターゲット時に `clock` サブモジュールを条件付きで公開する（`#[cfg(target_os = "windows")] pub mod clock;`）。
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
4. The Clock module shall `#[cfg(all(test, target_os = "windows"))]` テストモジュールを `clock.rs` 内に配置する、または `tests/` ディレクトリに統合テストファイルを配置する。

