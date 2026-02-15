# Requirements Document — dola-runtime-4-conflict

## Introduction

本ドキュメントは dola ランタイムエンジンの競合解決機能を定義する子仕様 `dola-runtime-4-conflict` の機能要件を定義する。親仕様 `dola-runtime-engine` の Req 7（競合検出と終了戦略）を子仕様の粒度に詳細化する。

本子仕様は Tier 3 に位置し、`dola-runtime-1-core-types`（Tier 1）と `dola-runtime-3-facade`（Tier 2）に依存する。facade が提供する `StoryboardInstance`, `VariableTimeline`, `TimelineEntry`, `InstanceManager`, `TimelineManager` の `pub(crate)` 内部 API を消費する。

**重要**: 本仕様はループ再生機能（`dola-runtime-5-loop`）と独立している。ConflictResolver は `loop_count` の値に関わらず、`Playing` 状態の全インスタンスを競合検出対象とする。

> 統合指針: `.kiro/specs/dola-runtime-engine/integration-guide.md` Section 2.3, 5.3 参照

---

## Requirements

### Requirement 1: 競合検出

_Parent: Req 7.1_

**Objective:** ランタイムとして、同一変数に対する時間的に重複するトランジション（競合）を検出したい。これにより、複数ストーリーボードが同時に同じ変数を操作する衝突を早期に発見できる。

#### Acceptance Criteria

1. When 新ストーリーボードの Start が発行された場合, the ConflictResolver shall 新セグメントの時間範囲が既存タイムテーブルエントリと重複するかチェックする。
2. When 重複が検出された場合, the ConflictResolver shall 競合する既存 `group_id` のリストを返す。
3. If 重複がない場合, then the ConflictResolver shall 空のリストを返し、競合解決をスキップする。
4. When 新ストーリーボードが複数の変数にセグメントを持つ場合, the ConflictResolver shall 各変数について独立して重複チェックを行い、変数ごとの競合 `group_id` リストを集約する。
5. The ConflictResolver shall `Playing` 状態のインスタンスのみを競合検出対象とする（`Paused`、`Created`、終了状態のインスタンスは除外）。

---

### Requirement 2: group_id 単位の一括適用

_Parent: Req 7.2, 7.3_

**Objective:** ランタイムとして、1つの変数での競合を検出した場合に、同じ group_id の全変数に対して終了戦略を一括適用したい。これにより、ストーリーボード単位の一貫したライフサイクル管理を保証する。

#### Acceptance Criteria

1. When 競合が検出された場合, the ConflictResolver shall 既存ストーリーボード実行インスタンス（`group_id` 単位）に対して、その `group_id` が持つ `interruption_policy` に従った終了戦略を適用する。
2. When 1つの変数で競合が検出された場合, the ConflictResolver shall 同じ `group_id` を持つ全変数のタイムテーブルに対して、その `group_id` の終了戦略を一括適用する。
3. When 複数の既存 `group_id` が同時に競合した場合, the ConflictResolver shall 各 `group_id` に対して、それぞれの `group_id` が持つ `interruption_policy` に従った終了戦略を個別に適用する。

---

### Requirement 3: Cancel 戦略

_Parent: Req 7.4_

**Objective:** ランタイムとして、Cancel 戦略を適用して既存インスタンスの現在値で凍結・破棄したい。これにより、アニメーション途中の値をそのまま維持した即時中断を実現する。

#### Acceptance Criteria

1. When 終了戦略が Cancel の場合, the ConflictResolver shall 既存インスタンスの現在の補間値でそのまま凍結する。
2. When 終了戦略が Cancel の場合, the ConflictResolver shall 既存インスタンスの状態を `Cancelled` に遷移させる。
3. When 終了戦略が Cancel の場合, the ConflictResolver shall 既存インスタンスのタイムテーブルエントリを除去する。

---

### Requirement 4: Conclude 戦略

_Parent: Req 7.5_

**Objective:** ランタイムとして、Conclude 戦略を適用して既存インスタンスの最終値にジャンプさせたい。これにより、現在再生中トランジションの到達先まで即座に遷移して終了する。

#### Acceptance Criteria

1. When 終了戦略が Conclude の場合, the ConflictResolver shall 既存インスタンスの**現在再生中トランジション**の最終値にジャンプさせる（ストーリーボード全体の最終値ではない）。未開始のトランジションはスキップする。
2. When 終了戦略が Conclude の場合, the ConflictResolver shall 既存インスタンスの状態を `Concluded` に遷移させる。
3. When 終了戦略が Conclude の場合, the ConflictResolver shall 既存インスタンスのタイムテーブルエントリを除去する。

---

### Requirement 5: Trim 戦略

_Parent: Req 7.6_

**Objective:** ランタイムとして、Trim 戦略を適用して既存インスタンスを割り込み時点で切断したい。これにより、割り込み直前の補間値を確定させたうえで既存再生を終了する。

#### Acceptance Criteria

1. When 終了戦略が Trim の場合, the ConflictResolver shall 新ストーリーボードの開始時刻を割り込み時点として使用し、既存インスタンスを当該時点まで再生して切断する。
2. When 終了戦略が Trim の場合, the ConflictResolver shall 割り込み時点における各変数の補間値を確定値としてタイムテーブルに反映する。
3. When 終了戦略が Trim の場合, the ConflictResolver shall 割り込み時点以降のセグメントを除去する。
4. When 終了戦略が Trim の場合, the ConflictResolver shall 既存インスタンスの状態を `Trimmed` に遷移させる。

---

### Requirement 6: Compress 戦略

_Parent: Req 7.7_

**Objective:** ランタイムとして、Compress 戦略を適用してストーリーボード全体の最終値にジャンプさせたい。これにより、全トランジションを完走扱いとして即座に最終状態に到達する。

#### Acceptance Criteria

1. When 終了戦略が Compress の場合, the ConflictResolver shall 既存インスタンスのストーリーボード全体の最終値にジャンプさせる。
2. When 終了戦略が Compress の場合, the ConflictResolver shall 全トランジションを完走扱いとする。
3. When 終了戦略が Compress の場合, the ConflictResolver shall 既存インスタンスの状態を `Compressed` に遷移させる。
4. When 終了戦略が Compress の場合, the ConflictResolver shall 既存インスタンスのタイムテーブルエントリを除去する。

---

### Requirement 7: Never 戦略と延期キュー

_Parent: Req 7.8_

**Objective:** ランタイムとして、Never 戦略で既存インスタンスの中断を拒否し、新ストーリーボードの当該変数エントリを延期したい。これにより、先行アニメーションの完全な完了を保証しつつ、後続の再生を予約できる。

#### Acceptance Criteria

1. When 終了戦略が Never の場合, the ConflictResolver shall 既存インスタンスの中断を拒否する。
2. When 終了戦略が Never の場合, the ConflictResolver shall 新ストーリーボードの当該変数へのセグメント追加を延期キュー（`DeferredEntry`）に格納する。
3. When 先行 `group_id` のインスタンスが終了状態（`Concluded` / `Cancelled` / `Trimmed` / `Compressed` のいずれか）に遷移した場合, the ConflictResolver shall 延期キューを走査し、`blocked_by` が一致するエントリをタイムテーブルに追加する。
4. While 先行 `group_id` が無限ループ（`loop_count = -1`）で再生中の場合, the ConflictResolver shall 延期エントリを永続的に保持する。
5. When 同一 `group_id` 内の複数変数が Never で延期された場合, the ConflictResolver shall 各変数の延期エントリを個別に管理し、先行インスタンス終了時に一括解放する。

---

### Requirement 8: デフォルト終了戦略

_Parent: Req 7.9_

**Objective:** ランタイムとして、未指定の終了戦略にデフォルトを適用したい。これにより、終了戦略を省略した場合の一貫した振る舞いを保証する。

#### Acceptance Criteria

1. If ストーリーボード定義に終了戦略が未指定の場合, then the ConflictResolver shall デフォルトとして Conclude を適用する。
2. The ConflictResolver shall デフォルト値が既存の `InterruptionPolicy` enum のデフォルト（`Conclude`）と一致することを保証する。
