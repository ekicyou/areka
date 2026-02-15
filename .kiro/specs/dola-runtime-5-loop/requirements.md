# Requirements Document — dola-runtime-5-loop

## Introduction

本ドキュメントは dola ランタイムエンジンのループ再生機能を定義する子仕様 `dola-runtime-5-loop` の機能要件を定義する。親仕様 `dola-runtime-engine` の Req 12（ループ再生）を子仕様の粒度に詳細化する。

本子仕様は Tier 3 に位置し、`dola-runtime-1-core-types`（Tier 1）と `dola-runtime-3-facade`（Tier 2）に依存する。facade が提供する `StoryboardInstance`, `VariableTimeline`, `InstanceManager` の `pub(crate)` 内部 API を消費する。

**重要**: 本仕様は競合解決機能（`dola-runtime-4-conflict`）と独立している。LoopController はループ中のインスタンスを `Playing` 状態に保つだけで、競合検出は ConflictResolver の責務である。ループ中でも通常の競合解決が適用される。

> 統合指針: `.kiro/specs/dola-runtime-engine/integration-guide.md` Section 2.3, 5.3 参照

---

## Requirements

### Requirement 1: ループ再生 — 基本動作

_Parent: Req 12.1, 12.2, 12.3_

**Objective:** ランタイムとして、ストーリーボードの `loop_count` に基づいてループ再生を実現したい。これにより、繰り返しアニメーションやアイドルモーションを宣言的に定義できる。

#### Acceptance Criteria

1. When `loop_count` が `1` の場合, the LoopController shall 1回のみ再生し、終了後にインスタンスを終了状態へ遷移させる。
2. When `loop_count` が `-1` の場合, the LoopController shall 無限にループ再生を継続する。
3. When `loop_count` が `n` (`n ≥ 2`) の場合, the LoopController shall n 回のループ再生後にインスタンスを終了状態へ遷移させる。

---

### Requirement 2: ループ再生 — タイムテーブル再利用

_Parent: Req 12.4, 12.5, 12.6, 12.7_

**Objective:** ランタイムとして、タイムテーブルを1周分のみ保持しつつ効率的なループ再生を実現したい。これにより、メモリ消費を抑えながらも正確な周回管理を可能にする。

#### Acceptance Criteria

1. The LoopController shall ループ再生時もタイムテーブルを1周分のみ生成し、ループ展開を行わない。
2. When 1周目の全セグメントが終了した場合, the LoopController shall `loop_count` をチェックしてループ継続の可否を判定する。
3. When ループを継続する場合, the LoopController shall タイムテーブルを破棄せず、時間オフセット機構を調整して再利用する。
4. When ループが完了した場合, the LoopController shall インスタンスを終了状態へ遷移させ、タイムテーブルを破棄する。
