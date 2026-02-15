# Research & Design Decisions — dola-runtime-4-conflict

## Summary
- **Feature**: `dola-runtime-4-conflict`
- **Discovery Scope**: Extension（既存ランタイムシステムへの Tier 3 機能追加）
- **Key Findings**:
  - Tier 3 Hook が facade.rs L116-117 に明示的に配置済み。挿入ポイントが確定している
  - 既存の `InstanceState`、`transition()`、`remove_entries()`、`collect_final_values()` が多くの AC を直接カバー
  - borrowck 制約は個別引数渡し（S1）で自然に回避可能

---

## Research Log

### 拡張ポイント分析

- **Context**: ConflictResolver の挿入位置と、既存モジュールとの接合面を確認
- **Sources Consulted**: `facade.rs` L90-130、`instance_manager.rs` 全体、`timeline_manager.rs` 全体、親仕様 `integration-guide.md` Section 2.3, 5.3
- **Findings**:
  - `facade.rs` L116-117: `// 7. [Tier 3 Hook] 競合解決` — `insert_entries()` の**直前**に配置済み
  - facade は `DolaRuntime` struct で `instance_manager`, `timeline_manager`, `subscription_manager` を直接所有
  - facade の `start()` メソッド内で個別フィールドの `&mut` 参照を分離すれば、フリー関数に渡せる
  - `InstanceManager.transition()` は `Concluded` のみ自動削除。他の終了状態は手動 `remove()` が必要
  - `TimelineManager` の `calculate_effective_time()` と `evaluate_segments()` はモジュールプライベート
- **Implications**: フリー関数 + 個別引数渡し（Option C + S1）が最も自然な設計

### Conclude vs Compress の値取得差異

- **Context**: 両戦略は「最終値にジャンプ」だが取得対象が異なる
- **Sources Consulted**: `timeline_manager.rs` の `collect_final_values()`、`evaluate_segments()` ロジック
- **Findings**:
  - `collect_final_values()`: **ストーリーボード全体の最終セグメント**の `to_value` を取得 → **Compress に適合**
  - Conclude に必要な**現在再生中セグメント**の最終値を取得するメソッドは存在しない
  - `evaluate_segments()` のアクティブセグメント走査ロジックを参考に `collect_current_segment_final_values()` を新設する必要がある
  - 新メソッドは `calculate_effective_time()` → アクティブセグメント特定 → `Interpolator::interpolate(seg, type, 1.0)` の3ステップ
- **Implications**: `collect_current_segment_final_values()` を TimelineManager の `pub(crate)` メソッドとして追加

### Trim 後のエントリ表現: 方式 A vs 方式 B

- **Context**: Trim 戦略で割り込み時点以降のセグメントを除去した後、タイムテーブル上のエントリをどう扱うか
- **Sources Consulted**: `remove_entries()` の挙動、`force_update_last_values()` の挙動、facade.rs の `cancel()` パターン
- **Findings**:
  - **方式 A（セグメント列切断）**: アクティブセグメント以降を `truncate` + end_time 更新。evaluate が自然に最終値を返すが、セグメント内部書き換えが必要でエッジケースが複雑
  - **方式 B（エントリ全削除 + 値伝播）**: `remove_entries()` + `force_update_last_values()` で Cancel と同等パターン。実装がシンプルで既存 API で完結
  - 方式 B では Trim 後に「切断された値」がタイムテーブル上に残らないが、`InstanceState::Trimmed` で意味の区別は可能
  - Cancel の facade パターン（`transition()` → `remove_entries()` → `remove()`）との対称性あり
- **Implications**: **方式 B を採用**。Trim 固有のセグメント切断ロジックは不要となり、値伝播で対応

### InstanceManager cleanup 一貫性

- **Context**: `transition()` が `Concluded` のみ自動削除する非対称性の解消
- **Sources Consulted**: `instance_manager.rs` L84-101（transition メソッド）、`facade.rs` cancel() の手動 `remove()` パターン
- **Findings**:
  - 現状: `Concluded` 遷移時のみ `self.instances.remove(group_id)` で自動削除
  - `Cancelled`/`Trimmed`/`Compressed` は `transition()` 後に手動 `remove()` が必要
  - facade の `cancel()` は `transition(Cancelled)` 後に明示的 `remove()` を呼んでいる
  - **方式 A**: `transition()` を全終了状態で自動削除に統一 — `is_terminal()` チェック。既存 cancel() の `remove()` は冗長になるが無害
  - **方式 B**: ConflictResolver 内で `transition()` + `remove()` をセットで呼ぶ — 既存コード無変更
- **Implications**: **方式 A を採用**。`transition()` 内で `is_terminal()` 時に自動削除に統一。既存の facade `cancel()` の `remove()` は今回のスコープで削除可能（冗長のため）

### 延期キュー（DeferredEntry）の保持場所

- **Context**: Never 戦略で生成される延期エントリの保持・解放設計
- **Sources Consulted**: 親仕様 design.md Implementation Extensions セクション、`timeline_manager.rs` データ構造
- **Findings**:
  - 保持場所候補: TimelineManager 内部、ConflictResolver struct 内部、facade 直接保持
  - TimelineManager 内部が最も自然: データ的にタイムテーブルエントリの「予約」であり責務が一致
  - 解放トリガー: 先行 group_id が終了状態に遷移した時
  - ConflictResolver がフリー関数パターン（Option C）では状態を持てない → TimelineManager に配置が必須
  - `flush_deferred(terminated_group_ids)` メソッドを TimelineManager に新設
- **Implications**: `DeferredEntry` 型定義と `deferred_entries: Vec<DeferredEntry>` を TimelineManager に配置。`flush_deferred()` で解放

### 延期キュー解放トリガーの挿入位置

- **Context**: 先行 group_id 終了時に延期キューを走査する呼び出し箇所
- **Sources Consulted**: facade.rs の `start()`、`update()`、`conclude_internal()`、`cancel()` フロー
- **Findings**:
  - `resolve_conflicts()` 内部: 競合解決で終了させた group_id について即座に flush 可能
  - facade `conclude_internal()`: 明示的 Conclude 後に flush
  - facade `cancel()`: 明示的 Cancel 後に flush
  - facade `update()`: 自然終了（evaluate で全セグメント完了）後に flush
  - ConflictResolver フリー関数内で `flush_deferred()` を直接呼ぶのが最もシンプル
  - facade の既存終了パス（conclude/cancel）にも flush 呼び出しを追加する必要がある
- **Implications**: resolve_conflicts() 内と facade 終了パスの両方に flush を配置

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **A: struct ベース** | `ConflictResolver` struct を新規作成 | 統合指針完全準拠、独立テスト容易 | `&mut self` 借用制約で facade が同時参照できない | DeferredEntry を struct 内に保持すると borrowck 問題再発 |
| **B: facade 拡張** | facade.rs 内にプライベートメソッド追加 | 借用単純 | facade 肥大化（326→600行超）、統合指針違反 | 非推奨 |
| **C: ハイブリッド** ★ | フリー関数群 + DeferredEntry は TimelineManager 保持 | borrowck 自然回避、テスト容易、親仕様 trait interface 準拠 | モジュール凝集度がやや低い | 推奨 |

| 借用回避策 | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **S1: 個別引数渡し** ★ | 各コンポーネントの `&mut` を個別に渡す | 最もシンプル、borrowck フレンドリー | 引数が多い（3-5個） | 推奨 |
| **S2: split borrow** | facade 内で destructure | Rust 的に正当 | facade メソッド内部に限定 | S1 と実質同等 |
| **S3: 中間結果** | ConflictAction の Vec を返し、facade が apply | テスト最容易 | 2パス、Trim 値確定の精度 | 複雑化 |

---

## Design Decisions

### Decision: アーキテクチャパターン — Option C + S1

- **Context**: ConflictResolver のモジュール構成と borrowck 回避策の選択
- **Alternatives Considered**:
  1. Option A (struct) + S2 — struct ベースで split borrow
  2. Option B (facade 拡張) — ロジックを facade に直接実装
  3. Option C (フリー関数) + S1 — 個別引数渡し
- **Selected Approach**: Option C + S1。`conflict_resolver.rs` にフリー関数群を配置し、facade が各コンポーネントの `&mut` を個別に渡す
- **Rationale**:
  - 親仕様 design.md の trait interface `fn resolve_conflicts(&self, ..., timelines: &mut ..., instances: &mut ...)` に最も近い
  - borrowck を自然に回避: facade 内で `&mut self.timeline_manager`, `&mut self.instance_manager` を個別に取り出せる
  - DeferredEntry を TimelineManager に配置 → 終了イベント時に自然に検知可能
- **Trade-offs**: フリー関数のためモジュール凝集度がやや低い。テスト時に各コンポーネントのセットアップが必要
- **Follow-up**: facade.rs 内での split borrow パターンを実装タスクで具体化

### Decision: Trim 値確定 — 方式 B（エントリ全削除 + 値伝播）

- **Context**: Trim 戦略で割り込み時点の値をどう確定するか
- **Alternatives Considered**:
  1. 方式 A — セグメント列を切断し、タイムテーブルに切断済みエントリを残す
  2. 方式 B — エントリ全削除 + `force_update_last_values()` で値を購読者に伝播
- **Selected Approach**: 方式 B
- **Rationale**: Cancel パターンとの対称性。既存 API（`remove_entries()` + `force_update_last_values()`）で完結し、新規のセグメント書き換えロジックが不要
- **Trade-offs**: Trim 後にタイムテーブル上に「切断された値」が残らない。区別は `InstanceState::Trimmed` で表現
- **Follow-up**: なし

### Decision: InstanceManager transition() — 全終了状態で自動削除

- **Context**: `transition()` の `Concluded` のみ自動削除の非対称性解消
- **Alternatives Considered**:
  1. 方式 A — `transition()` 内で `is_terminal()` 時に統一削除
  2. 方式 B — ConflictResolver 内で `transition()` + `remove()` をセットで呼ぶ
- **Selected Approach**: 方式 A
- **Rationale**: 終了状態の扱いを統一し、呼び出し側が `remove()` を忘れるリスクを排除。既存 `cancel()` の `remove()` は冗長になるが無害
- **Trade-offs**: 既存テストが `Cancelled` 後もインスタンスにアクセスする想定の場合、修正が必要
- **Follow-up**: 既存 `runtime_facade_test.rs` への影響を実装タスクで確認

### Decision: 延期キュー保持場所 — TimelineManager 内部

- **Context**: `DeferredEntry` の保持場所選択
- **Alternatives Considered**:
  1. ConflictResolver struct 内部
  2. TimelineManager 内部
  3. facade 直接保持
- **Selected Approach**: TimelineManager 内部
- **Rationale**: タイムテーブルエントリの「予約」として責務が一致。Option C（フリー関数）では ConflictResolver が状態を持てない。facade 直接保持は凝集度が低い
- **Trade-offs**: TimelineManager の責務がやや拡大
- **Follow-up**: `add_deferred()` / `flush_deferred()` メソッドの設計を design.md で確定

### Decision: collect_current_segment_final_values() の配置 — TimelineManager

- **Context**: Conclude 戦略で必要な「現在再生中セグメントの最終値」取得メソッドの配置先
- **Alternatives Considered**:
  1. TimelineManager の `pub(crate)` メソッド
  2. conflict_resolver 内のヘルパー関数
- **Selected Approach**: TimelineManager の `pub(crate)` メソッド
- **Rationale**: `collect_final_values()` との対称性。内部の `calculate_effective_time()` と `evaluate_segments()` のロジックを再利用する必要があり、同モジュール内が自然
- **Trade-offs**: TimelineManager に ConflictResolver 専用のメソッドが追加される
- **Follow-up**: なし

---

## Risks & Mitigations

- **Trim エッジケース**: duration=0 セグメント、割り込み時点が最終セグメント終了後、全セグメント未開始 — 各パターンの単体テストで検証
- **延期キューメモリ**: 無限ループ先行時に DeferredEntry が永続保持 — 仕様上許容。将来的にサイズ上限を追加可能
- **既存テスト破壊**: `transition()` の自動削除統一が既存テストに影響 — 修正対象を実装タスクで特定
- **borrowck 複雑性**: facade の split borrow が多数のコンポーネントに及ぶ — S1 の個別引数渡しで最小化

---

## References

- [親仕様 design.md](../../dola-runtime-engine/design.md) — ConflictResolver コンポーネント定義、競合解決フロー、Never 延期キュー実装ノート
- [親仕様 integration-guide.md](../../dola-runtime-engine/integration-guide.md) — Tier 2→3 境界契約、モジュール構成、Feature Gate 戦略
- [gap-analysis.md](./gap-analysis.md) — 29 AC の既存資産マッピング、技術的課題と解決策候補
