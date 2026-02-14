# Requirements Document — dola-runtime-4-conflict-loop

## Introduction

本ドキュメントは dola ランタイムエンジンの高度機能を定義する子仕様 `dola-runtime-conflict-loop` の機能要件を定義する。親仕様 `dola-runtime-engine` の Req 7（競合検出と終了戦略）と Req 12（ループ再生）を子仕様の粒度に詳細化する。

本子仕様は Tier 3 に位置し、`dola-runtime-core-types`（Tier 1）と `dola-runtime-facade`（Tier 2）に依存する。facade が提供する `StoryboardInstance`, `VariableTimeline`, `TimelineEntry`, `InstanceManager`, `TimelineManager` の `pub(crate)` 内部 API を消費する。

> 統合指針: `.kiro/specs/dola-runtime-engine/integration-guide.md` Section 2.3 参照

---

## Requirements

### Requirement 1: 競合検出

_Parent: Req 7.1_

**Objective:** ランタイムとして、同一変数に対する時間的に重複するトランジション（競合）を検出したい。

#### Acceptance Criteria

1. When 新ストーリーボードの Start が発行された場合, the ConflictResolver shall 新セグメントの時間範囲が既存タイムテーブルエントリと重複するかチェックする。
2. The ConflictResolver shall 重複が検出された場合、競合する既存 `group_id` のリストを返す。
3. The ConflictResolver shall 重複がない場合、空のリストを返す。

---

### Requirement 2: group_id 単位の一括適用

_Parent: Req 7.2, 7.3_

**Objective:** ランタイムとして、1つの変数での競合を検出した場合に、同じ group_id の全変数に対して終了戦略を一括適用したい。

#### Acceptance Criteria

1. When 競合が検出された場合, the ConflictResolver shall 既存ストーリーボード実行インスタンス（`group_id` 単位）に対して終了戦略を一括適用する。
2. When 1つの変数で競合が検出された場合, the ConflictResolver shall 同じ `group_id` を持つ全変数のタイムテーブルに対して終了戦略を一括適用する。

---

### Requirement 3: Cancel 戦略

_Parent: Req 7.4_

**Objective:** ランタイムとして、Cancel 戦略を適用して既存インスタンスの現在値で凍結・破棄したい。

#### Acceptance Criteria

1. When 終了戦略が Cancel の場合, the ConflictResolver shall 既存インスタンスの現在の補間値でそのまま凍結する。
2. The ConflictResolver shall 既存インスタンスの状態を `Cancelled` に遷移させる。

---

### Requirement 4: Conclude 戦略

_Parent: Req 7.5_

**Objective:** ランタイムとして、Conclude 戦略を適用して既存インスタンスの最終値にジャンプさせたい。

#### Acceptance Criteria

1. When 終了戦略が Conclude の場合, the ConflictResolver shall 既存インスタンスの現在再生中トランジションの最終値にジャンプさせる。
2. The ConflictResolver shall 未開始トランジションをスキップして既存インスタンスを終了する。
3. The ConflictResolver shall 既存インスタンスの状態を `Concluded` に遷移させる。

---

### Requirement 5: Trim 戦略

_Parent: Req 7.6_

**Objective:** ランタイムとして、Trim 戦略を適用して既存インスタンスを割り込み時点で切断したい。

#### Acceptance Criteria

1. When 終了戦略が Trim の場合, the ConflictResolver shall 既存インスタンスを割り込み開始時点まで再生して切断する。
2. The ConflictResolver shall 切断後の補間値でタイムテーブルを更新する。
3. The ConflictResolver shall 既存インスタンスの状態を `Trimmed` に遷移させる。

---

### Requirement 6: Compress 戦略

_Parent: Req 7.7_

**Objective:** ランタイムとして、Compress 戦略を適用してストーリーボード全体の最終値にジャンプさせたい。

#### Acceptance Criteria

1. When 終了戦略が Compress の場合, the ConflictResolver shall 既存インスタンスのストーリーボード全体の最終値にジャンプさせる。
2. The ConflictResolver shall 全トランジションを完走扱いとする。
3. The ConflictResolver shall 既存インスタンスの状態を `Compressed` に遷移させる。

---

### Requirement 7: Never 戦略と延期キュー

_Parent: Req 7.8_

**Objective:** ランタイムとして、Never 戦略で既存インスタンスの中断を拒否し、新ストーリーボードの当該変数エントリを延期したい。

#### Acceptance Criteria

1. When 終了戦略が Never の場合, the ConflictResolver shall 既存インスタンスの中断を拒否する。
2. The ConflictResolver shall 新ストーリーボードの当該変数へのセグメント追加を延期キュー（`DeferredEntry`）に格納する。
3. When 先行 group_id のインスタンスが終了状態に遷移した場合, the ConflictResolver shall 延期キューを走査し、`blocked_by` が一致するエントリをタイムテーブルに追加する。
4. When 先行 group_id が無限ループ（`loop_count = Some(0)`）の場合, the ConflictResolver shall 延期エントリを永続的に保持する。

---

### Requirement 8: デフォルト終了戦略

_Parent: Req 7.9_

**Objective:** ランタイムとして、未指定の終了戦略にデフォルトを適用したい。

#### Acceptance Criteria

1. If ストーリーボード定義に終了戦略が未指定の場合, then the ConflictResolver shall デフォルトとして Conclude を適用する。

---

### Requirement 9: ループ再生 — 基本動作

_Parent: Req 12.1, 12.2, 12.3_

**Objective:** ランタイムとして、ストーリーボードの `loop_count` に基づいてループ再生を実現したい。

#### Acceptance Criteria

1. When `loop_count` が `None` の場合, the LoopController shall 1回のみ再生し、終了後にインスタンスを終了状態へ遷移させる。
2. When `loop_count` が `Some(0)` の場合, the LoopController shall 無限にループ再生を継続する。
3. When `loop_count` が `Some(n)` (n > 0) の場合, the LoopController shall n 回のループ再生後にインスタンスを終了状態へ遷移させる。

---

### Requirement 10: ループ再生 — タイムテーブル再利用

_Parent: Req 12.4, 12.5, 12.6, 12.7_

**Objective:** ランタイムとして、タイムテーブルを1周分のみ保持しつつ効率的なループ再生を実現したい。

#### Acceptance Criteria

1. The LoopController shall ループ再生時もタイムテーブルを1周分のみ生成し、ループ展開を行わない。
2. When 1周目の全セグメントが終了した場合, the LoopController shall `loop_count` をチェックしてループ継続の可否を判定する。
3. When ループを継続する場合, the LoopController shall タイムテーブルを破棄せず、時間オフセット（`pause_accumulated` 機構）を調整して再利用する。
4. When ループが完了した場合, the LoopController shall インスタンスを終了状態へ遷移させ、タイムテーブルを破棄する。

---

### Requirement 11: ループ中の競合

_Parent: Req 12.8_

**Objective:** ランタイムとして、ループ中でも他のストーリーボードによる競合検出・中断戦略の適用を保証したい。

#### Acceptance Criteria

1. The LoopController shall ループ中の各周回も競合検出・中断戦略の対象とする。
2. When ループ中に他のストーリーボードによる競合が発生した場合, the ConflictResolver shall 通常の競合解決プロセスを適用する。

