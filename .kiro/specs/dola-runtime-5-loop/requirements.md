# Requirements Document — dola-runtime-5-loop

## Introduction

本ドキュメントは dola ランタイムエンジンのループ再生機能を定義する子仕様 `dola-runtime-5-loop` の機能要件を定義する。親仕様 `dola-runtime-engine` の Req 12（ループ再生）を子仕様の粒度に詳細化する。

本子仕様は Tier 3 に位置し、`dola-runtime-1-core-types`（Tier 1）と `dola-runtime-3-facade`（Tier 2）に依存する。facade が提供する `StoryboardInstance`, `VariableTimeline`, `InstanceManager` の `pub(crate)` 内部 API を消費する。

**重要**: 本仕様は競合解決機能（`dola-runtime-4-conflict`）と独立している。LoopController はループ中のインスタンスを `Playing` 状態に保つだけで、競合検出は ConflictResolver の責務である。ループ中でも通常の競合解決が適用される（親要件 Req 12.8 対応）。

> 統合指針: `.kiro/specs/dola-runtime-engine/integration-guide.md` Section 2.3, 5.3 参照

### loop_count セマンティクス（参照）

| 値 | 意味 |
|----|------|
| `1` | 1回再生（デフォルト、ループなし） |
| `n` (`n ≥ 2`) | n 回再生 |
| `-1` | 無限ループ |
| `0` 以下（`-1` 除く） | `InvalidLoopCount` エラー |

---

## Requirements

### Requirement 1: ループ再生 — 基本動作

_Parent: Req 12.1, 12.2, 12.3_

**Objective:** ランタイムとして、ストーリーボードの `loop_count` に基づいてループ再生を実現したい。これにより、繰り返しアニメーションやアイドルモーションを宣言的に定義できる。

#### Acceptance Criteria

1. When `loop_count` が `1` の場合, the LoopController shall 1回のみ再生し、全セグメント終了後にインスタンスを終了状態へ遷移させる（既存の Tier 2 動作と同一）。
2. When `loop_count` が `-1` の場合, the LoopController shall 全セグメント終了時にループを再開し、外部からの明示的な停止（Cancel 等）がない限り無限にループ再生を継続する。
3. When `loop_count` が `n` (`n ≥ 2`) の場合, the LoopController shall n 回のループ再生を完了した後にインスタンスを終了状態へ遷移させる。
4. When `update()` 呼び出し時に複数周回が終了している場合, the LoopController shall 終了した全周回分を一括処理し、周回数を正確に更新する。
5. While ループ再生が継続中の場合, the LoopController shall インスタンスを `Playing` 状態に維持する。

---

### Requirement 2: ループ再生 — タイムテーブル再利用

_Parent: Req 12.4, 12.5, 12.6, 12.7_

**Objective:** ランタイムとして、タイムテーブルを1周分のみ保持しつつ効率的なループ再生を実現したい。これにより、メモリ消費を抑えながらも正確な周回管理を可能にする。

#### Acceptance Criteria

1. The LoopController shall ループ再生時もタイムテーブルを1周分のみ生成し、n 周分のタイムテーブル展開を行わない。
2. When 周回終了時刻に到達した場合（`current_time >= end_time`）, the LoopController shall 反復ループで全ての終了済み周回を処理し、各周回について完了数をインクリメントして継続可否を判定する。
3. When ループを継続する場合, the LoopController shall タイムテーブルを破棄せず、現在の周回開始時刻を更新してタイムテーブルを再利用する。
4. The LoopController shall 周回開始時刻の更新において、1周分の duration を加算することで次周回の開始タイミングを正確に維持する。
5. When ループが完了した場合（`loops_completed >= loop_count`）, the LoopController shall インスタンスを終了状態へ遷移させ、タイムテーブルを破棄する。

---

### Requirement 3: ループ周回トラッキング

_Parent: Req 12.5（周回管理の内部状態）_

**Objective:** ランタイムとして、ループの周回進捗を正確に追跡したい。これにより、ループ完了判定とデバッグ時の状態確認が可能になる。

#### Acceptance Criteria

1. The LoopController shall 各インスタンスの完了周回数（`loops_completed`）を管理する。
2. When 周回終了時刻に到達した場合（`current_time >= end_time`）, the LoopController shall ループ継続判定の前に `loops_completed` を 1 インクリメントする。
3. The LoopController shall `loops_completed` の初期値を `0` とする。
4. While `loop_count` が `-1`（無限ループ）の場合, the LoopController shall `loops_completed` を周回ごとにインクリメントし続ける（オーバーフロー保護は `u32::MAX` で飽和）。

---

### Requirement 4: Pause/Resume との相互作用

_Parent: Req 12.6（時間オフセット機構の共存）_

**Objective:** ランタイムとして、ループ再生中の Pause/Resume が正しく動作することを保証したい。これにより、一時停止機能とループ再生を組み合わせた利用が可能になる。

#### Acceptance Criteria

1. When ループ再生中にインスタンスが Pause された場合, the LoopController shall 現在の周回内の再生位置を保持し、ループ周回数をリセットしない。
2. When Pause 中のインスタンスが Resume された場合, the LoopController shall Pause 前の周回と再生位置からループ再生を正確に再開する。
3. The LoopController shall ループの周回開始時刻（`loop_start_time`）と Pause/Resume の一時停止時間（`pause_accumulated`）を独立したフィールドで管理し、相互に干渉しない。

---

### Requirement 5: ループと外部制御の境界

_Parent: Req 12.7, 12.3_

**Objective:** ランタイムとして、ループ再生中の外部操作（Cancel など）が正しく処理されることを保証したい。これにより、ループ再生を停止する手段が常に利用可能であることを担保する。

#### Acceptance Criteria

1. When ループ再生中にインスタンスが Cancel された場合, the LoopController shall 即座にループ再生を停止し、インスタンスを Cancelled 状態へ遷移させる。
2. The LoopController shall ループ制御ロジックを競合解決ロジック（ConflictResolver）と独立して動作させ、ループ中でも通常の競合検出・中断戦略が適用される状態を維持する。
3. While ループ再生中の場合, the LoopController shall インスタンスの `Playing` 状態を維持することで、ConflictResolver による通常の競合検出対象とする。
