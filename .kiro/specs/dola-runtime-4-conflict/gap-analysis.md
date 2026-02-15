# ギャップ分析 — dola-runtime-4-conflict

> 更新日: 2026-02-15 | 要件: Req 1〜8（29 AC） | 対象: `crates/dola/src/runtime/`

## 分析概要

本文書は `dola-runtime-4-conflict` の要件（Req 1〜8、29受入基準）と既存コードベースのギャップを分析し、設計フェーズへの入力とする。

---

## 1. 現状調査

### 1.1 既存コードベースの構造

| モジュール | ファイル | 行数 | 役割 | 本仕様との関連 |
|-----------|---------|------|------|--------------|
| `facade.rs` | `runtime/facade.rs` | 326 | 公開 API、start フロー制御 | **Tier 3 Hook 挿入点**（L116-117: `// 7. [Tier 3 Hook] 競合解決`） |
| `instance_manager.rs` | `runtime/instance_manager.rs` | 357 | インスタンス状態管理 | 状態遷移先 `Cancelled`/`Trimmed`/`Compressed` 実装済み |
| `timeline_manager.rs` | `runtime/timeline_manager.rs` | 408 | タイムテーブル評価 | **競合検出・エントリ操作の主要対象** |
| `instance_state.rs` | `runtime/instance_state.rs` | 80 | 状態 enum + 遷移 | 全7バリアント + `from_policy()` 完備 |
| `subscription_manager.rs` | `runtime/subscription_manager.rs` | 280 | 購読・差分配信 | `force_update_last_values()` で Conclude/Compress 時の値伝播 |
| `interpolator.rs` | `runtime/interpolator.rs` | 388 | イージング + 補間 | `Interpolator::interpolate()` を値計算に利用 |
| `types.rs` | `runtime/types.rs` | 101 | 公開型定義 | `EvaluatedValue`, `RuntimeError`, `StartResult` |
| `mod.rs` | `runtime/mod.rs` | 22 | モジュール公開制御 | **`mod conflict_resolver;` 追加が必要** |
| `storyboard.rs` | `storyboard.rs` | 124 | `InterruptionPolicy` 定義 | 5バリアント定義済み、`default_interruption_policy() = Conclude` |

### 1.2 既存のフック・拡張ポイント

#### facade.rs の Tier 3 Hook（L116-117）

```rust
// 7. [Tier 3 Hook] 競合解決
// Tier 2: スキップ
```

`start()` メソッド内で、タイムテーブル挿入（L119: `self.timeline_manager.insert_entries()`）の**直前**に明示的なコメントフックが配置済み。このフック位置で ConflictResolver を呼び出す設計。

#### InstanceState の終了状態4種

`InstanceState` enum は `Concluded`, `Cancelled`, `Trimmed`, `Compressed` の全4終了バリアントを定義済み。`from_policy()` メソッドも `InterruptionPolicy` → `InstanceState` の変換を提供:

```rust
pub fn from_policy(policy: InterruptionPolicy) -> Option<InstanceState> {
    match policy {
        InterruptionPolicy::Cancel => Some(Self::Cancelled),
        InterruptionPolicy::Conclude => Some(Self::Concluded),
        InterruptionPolicy::Trim => Some(Self::Trimmed),
        InterruptionPolicy::Compress => Some(Self::Compressed),
        InterruptionPolicy::Never => None,
    }
}
```

#### facade.rs の既存 conclude/cancel パス

| メソッド | 操作フロー | 競合解決との関係 |
|---------|-----------|---------------|
| `conclude_internal()` | `collect_final_values()` → `force_update_last_values()` → Concluded 遷移 → `remove_entries()` | **Compress 戦略で再利用可能**（全最終値ジャンプ）。Conclude 戦略には**不適合**（全体最終値ではなく現在セグメント最終値が必要） |
| `cancel()` | terminal check → Cancelled 遷移 → `remove_entries()` → `remove()` | **Cancel 戦略の参考パターン**。ただし「現在値凍結」の値伝播ロジックは含まれていない |

#### Tier 2 の暫定動作（競合未実装時）

- 同一変数に複数 `group_id` エントリが共存 → 最新（最大）`group_id` の値が evaluate で採用される（`timeline_manager.rs` L100-105: `entry.group_id <= *best_gid` チェック）
- Tier 3 Hook は `// Tier 2: スキップ` で空の状態

### 1.3 コーディング規約・パターン

| パターン | 観察 | ソース |
|---------|------|--------|
| 公開範囲 | `pub(crate)` を内部コンポーネントに使用、`pub` は facade の `DolaRuntime` のみ | `instance_manager.rs`, `timeline_manager.rs` |
| エラー処理 | `RuntimeError` enum + `Result<T, RuntimeError>` | `types.rs` |
| テスト配置 | 各モジュール内に `#[cfg(test)] mod tests`、統合テストは `tests/` ディレクトリ | 全モジュール |
| HashMap/BTreeMap | 内部データは `HashMap`、外部定義（compile 出力）は `BTreeMap` | `instance_manager.rs`, `compile.rs` |
| ヘルパー関数 | モジュール内にプライベート関数 | `calculate_effective_time()`, `evaluate_segments()` |
| 所有権パターン | facade が全コンポーネントを所有し `&mut self` 経由で操作 | `facade.rs` L27-34 |
| 状態遷移 | `InstanceManager.transition()` 経由、`try_transition()` バリデーション | `instance_manager.rs` L84-101 |
| インスタンス削除 | **Concluded のみ** `transition()` 内で自動削除 | `instance_manager.rs` L97-99 |

**重要な観察**: 現在の `InstanceManager.transition()` は `Concluded` 遷移時のみインスタンスを自動削除する（L97-99）。`Cancelled`/`Trimmed`/`Compressed` の自動削除は実装されていない。facade の `cancel()` では明示的に `self.instance_manager.remove(group_id)` を呼んでいる。**ConflictResolver 実装では4終了状態すべてで適切な cleanup が必要。**

---

## 2. 要件→既存資産マッピング

### 2.1 AC 別詳細マッピング

#### Req 1: 競合検出（5 AC）

| AC | 必要機能 | 既存資産 | ギャップ |
|----|---------|---------|---------|
| 1.1 新セグメント vs 既存エントリの重複チェック | 時間範囲重複検出 | `VariableTimeline.entries` アクセス可能、`TimelineEntry.segments[].start_time/end_time` | **Missing**: 重複検出ロジック。`segments` の時間範囲を走査し交差判定する関数が必要 |
| 1.2 競合 group_id リスト返却 | 重複する既存 group_id の収集 | なし | **Missing**: 変数ごとの group_id 収集 + 集約 |
| 1.3 重複なし → 空リスト、スキップ | 早期リターン | なし（自然に実装可能） | **Trivial** |
| 1.4 複数変数の独立チェック＋集約 | 変数別の並行走査 | `TimelineManager.timelines: HashMap<String, VariableTimeline>` | **Missing**: 変数横断の集約ロジック。`CompiledStoryboard.timelines` から変数名を取得し、各変数で独立チェック |
| 1.5 Playing 状態フィルタ | インスタンス状態参照 | `InstanceManager.instances()` + `StoryboardInstance.state` | **Partial**: 参照は可能だが、フィルタ統合ロジックが必要 |

#### Req 2: group_id 一括適用（3 AC）

| AC | 必要機能 | 既存資産 | ギャップ |
|----|---------|---------|---------|
| 2.1 group_id 単位で終了戦略一括適用 | その group_id の `interruption_policy` 取得 + 戦略ディスパッチ | `StoryboardInstance.interruption_policy`, `InstanceState::from_policy()` | **Missing**: policy → 戦略実行のディスパッチャー |
| 2.2 同一 group_id の全変数に適用 | group_id による全変数横断 | `TimelineManager.remove_entries(group_id)` が横断削除を実装済み | **Partial**: 削除は可能、戦略別の横断操作（値取得、Trim 切断等）が必要 |
| 2.3 複数 group_id 同時競合時の個別適用 | 各 group_id が持つ独自の policy に従う | なし | **Missing**: 競合 group_id リストのイテレーション + 各 group_id の policy 取得 + 個別戦略適用 |

#### Req 3: Cancel 戦略（3 AC）

| AC | 必要機能 | 既存資産 | ギャップ |
|----|---------|---------|---------|
| 3.1 現在補間値で凍結 | 値保持（暗黙的） | `SubscriptionManager.last_values` に前回 evaluate 値が残る | **Existing**: facade の `cancel()` と同様に、`last_values` の自然残存で実現可能 |
| 3.2 Cancelled 状態遷移 | `transition(gid, Cancelled)` | `InstanceManager.transition()` + `InstanceState::try_transition()` | **Existing** |
| 3.3 タイムテーブルエントリ除去 | `remove_entries(gid)` | `TimelineManager.remove_entries()` | **Existing** |

#### Req 4: Conclude 戦略（3 AC）

| AC | 必要機能 | 既存資産 | ギャップ |
|----|---------|---------|---------|
| 4.1 **現在再生中トランジション**の最終値ジャンプ＋未開始スキップ | アクティブセグメント特定 + `to_value` 取得 | `evaluate_segments()` がアクティブセグメント走査を実装（ただし `fn` プライベート） | **Missing**: `collect_current_segment_final_values()` — effective_time でアクティブなセグメントを見つけ、そのセグメントの `to_value` (progress_t=1.0) を返す新メソッドが必要。既存 `collect_final_values()` は**ストーリーボード全体の最終値（=Compress 相当）**であり Conclude には不適合 |
| 4.2 Concluded 状態遷移 | `transition(gid, Concluded)` | `InstanceManager.transition()` — Concluded 遷移時に自動削除 | **Existing** |
| 4.3 タイムテーブルエントリ除去 | `remove_entries(gid)` | `TimelineManager.remove_entries()` | **Existing** |

#### Req 5: Trim 戦略（4 AC）

| AC | 必要機能 | 既存資産 | ギャップ |
|----|---------|---------|---------|
| 5.1 割り込み時点（新SB開始時刻）で切断 | effective_time 計算 + セグメント切断 | `calculate_effective_time()` (プライベート関数) | **Missing**: セグメント列の途中切断ロジック |
| 5.2 割り込み時点の補間値をタイムテーブルに反映 | 補間値計算 + エントリ書き換え | `Interpolator::interpolate()` | **Missing**: 既存エントリの上書き/書き換えメソッド |
| 5.3 割り込み時点以降のセグメント除去 | セグメント列の部分削除 | なし | **Missing** |
| 5.4 Trimmed 状態遷移 | `transition(gid, Trimmed)` | `InstanceManager.transition()` | **Existing**（ただし Trimmed 後の手動 `remove()` が必要） |

#### Req 6: Compress 戦略（4 AC）

| AC | 必要機能 | 既存資産 | ギャップ |
|----|---------|---------|---------|
| 6.1 ストーリーボード全体の最終値ジャンプ | 全セグメント最終値取得 | `TimelineManager.collect_final_values()` — 最終セグメントの `to_value` (progress_t=1.0) | **Existing** |
| 6.2 全トランジション完走扱い | `force_update_last_values()` で値伝播 | `SubscriptionManager.force_update_last_values()` | **Existing** |
| 6.3 Compressed 状態遷移 | `transition(gid, Compressed)` | `InstanceManager.transition()` | **Existing**（ただし Compressed 後の手動 `remove()` が必要） |
| 6.4 タイムテーブルエントリ除去 | `remove_entries(gid)` | `TimelineManager.remove_entries()` | **Existing** |

#### Req 7: Never + 延期キュー（5 AC）

| AC | 必要機能 | 既存資産 | ギャップ |
|----|---------|---------|---------|
| 7.1 既存インスタンスの中断拒否 | 戦略ディスパッチで Never 分岐 | `InstanceState::from_policy(Never) → None` | **Partial**: 分岐判定は可能、スキップロジックが必要 |
| 7.2 延期キュー格納 | `DeferredEntry` 型 + 格納先 | なし | **Missing**: `DeferredEntry` 型定義、格納コレクション |
| 7.3 先行 group_id 終了時の解放 | 終了トリガー → 延期キュー走査 → タイムテーブル追加 | なし | **Missing**: 終了イベントの検知パスと延期キュー走査ロジック |
| 7.4 無限ループ中の永続保持 | `loop_count == -1` チェック | `StoryboardInstance.loop_count` | **Partial**: フィールド参照は可能、保持ロジックが必要 |
| 7.5 複数変数延期の個別管理＋一括解放 | 変数別 DeferredEntry + blocked_by 走査 | なし | **Missing** |

#### Req 8: デフォルト終了戦略（2 AC）

| AC | 必要機能 | 既存資産 | ギャップ |
|----|---------|---------|---------|
| 8.1 未指定時 Conclude 適用 | デフォルト値参照 | `default_interruption_policy() = Conclude` in `storyboard.rs` | **Existing**: serde デフォルトで Conclude が設定される |
| 8.2 InterruptionPolicy デフォルトとの一致保証 | テストでの検証 | なし（テスト追加で対応） | **Trivial** |

### 2.2 ギャップサマリー

| カテゴリ | AC 数 | 項目 |
|---------|------|------|
| **Existing** (そのまま使用可能) | 11 | AC 3.1〜3.3, 4.2, 4.3, 5.4, 6.1〜6.4, 8.1 — `InstanceState` 全バリアント, `from_policy()`, `transition()`, `remove_entries()`, `collect_final_values()`, `force_update_last_values()`, デフォルト戦略 serde |
| **Partial** (拡張必要) | 4 | AC 1.5, 2.2, 7.1, 7.4 — Playing 状態フィルタ、group_id 全変数横断削除、Never 分岐判定、無限ループチェック |
| **Missing** (新規作成) | 12 | AC 1.1, 1.2, 1.4, 2.1, 2.3, 4.1, 5.1〜5.3, 7.2, 7.3, 7.5 — 時間重複検出、変数横断集約、戦略ディスパッチャー、`collect_current_segment_final_values()`、Trim 切断ロジック、`DeferredEntry` 型、延期キュー管理、終了トリガー→解放パス |
| **Trivial** (自明に実装可能) | 2 | AC 1.3, 8.2 — 空リスト返却（早期リターン）、デフォルト値一致テスト |

### 2.3 既存メソッドの pub(crate) 化が必要な関数

| 関数 | 現在の公開範囲 | 必要な変更 | 理由 |
|------|-------------|-----------|------|
| `calculate_effective_time()` | `fn`（モジュールプライベート） | `pub(crate) fn` | Trim 戦略で effective_time 計算に使用 |
| `evaluate_segments()` | `fn`（モジュールプライベート） | `pub(crate) fn` | Conclude 戦略でアクティブセグメント特定パターンを参考にする可能性 |

---

## 3. 実装アプローチ選択肢

### Option A: struct ベースの新モジュール

統合指針 Section 5.3 に従い、`ConflictResolver` struct を持つ `conflict_resolver.rs` を新規作成する。

**対象ファイル**:

| 操作 | ファイル | 内容 |
|------|---------|------|
| **新規作成** | `runtime/conflict_resolver.rs` | `ConflictResolver` struct + 5戦略実装 + `DeferredEntry` |
| **修正** | `runtime/mod.rs` | `mod conflict_resolver;` 追加 |
| **修正** | `runtime/facade.rs` | `start()` 内の Tier 3 Hook 実装置換 + 延期キュー解放フック |
| **修正** | `runtime/timeline_manager.rs` | `calculate_effective_time()` と `evaluate_segments()` の `pub(crate)` 化 |
| **修正** | `runtime/instance_manager.rs` | `instances_mut()` メソッド追加（または不要 — 後述の借用回避策次第） |

**トレードオフ**:
- ✅ 統合指針に完全準拠、モジュール構成が明確
- ✅ 独立テスト可能な単位
- ✅ 延期キューの状態を struct 内部に保持できる
- ❌ `&mut self` の借用制約で ConflictResolver → TimelineManager/InstanceManager の連鎖に設計工夫が必要
- ❌ ConflictResolver が延期キューを保持する場合、facade が `&mut conflict_resolver` と `&mut timeline_manager` を同時に渡せない

### Option B: facade 拡張（非推奨）

ConflictResolver のロジックを facade.rs 内のプライベートメソッドとして実装する。

**トレードオフ**:
- ✅ 借用制約が単純（全フィールドが `self` 配下）
- ❌ facade.rs が肥大化（326行 → 推定600行超）
- ❌ 統合指針の設計に反する
- ❌ 単体テストが困難

### Option C: ハイブリッド（フリー関数 + 新モジュール）— **推奨**

ConflictResolver をステートレスなフリー関数群として実装し、延期キューは TimelineManager 内部に保持する。facade が `&mut self` 内部のフィールド参照を個別に渡す。

**対象ファイル**:

| 操作 | ファイル | 内容 |
|------|---------|------|
| **新規作成** | `runtime/conflict_resolver.rs` | フリー関数群（`resolve_conflicts()`, 5戦略関数） |
| **修正** | `runtime/mod.rs` | `mod conflict_resolver;` 追加 |
| **修正** | `runtime/facade.rs` | Tier 3 Hook 実装 + 延期キュー解放フック（update 内） |
| **修正** | `runtime/timeline_manager.rs` | `DeferredEntry` 型定義、`deferred_entries: Vec<DeferredEntry>` 追加、`collect_current_segment_final_values()` 新規、`pub(crate)` 化 |
| **修正** | `runtime/instance_manager.rs` | 軽微な拡張（必要に応じて） |

**トレードオフ**:
- ✅ borrowck の制約を自然に回避（各フィールドの個別 `&mut` が可能）
- ✅ テスト容易（純粋関数に近い設計）
- ✅ 親仕様 design.md の trait interface シグネチャ `fn resolve_conflicts(&self, ..., timelines: &mut ..., instances: &mut ...)` に近い形
- ✅ DeferredEntry を TimelineManager に配置 → 終了トリガー（evaluate/conclude 内）で自然に検知可能
- ❌ ステートレスなためモジュール凝集度がやや低い（テスト用のセットアップが散在）

**呼び出しパターン**: facade の `start()` 内で以下のように呼び出す:
```rust
// facade.rs start() 内
let affected = conflict_resolver::resolve_conflicts(
    &compiled,
    start_time,
    &mut self.timeline_manager,
    &mut self.instance_manager,
    &mut self.subscription_manager,  // Conclude/Compress 時の値伝播用
);
```

---

## 4. 技術的課題と調査事項

### 4.1 Rust 借用制約: facade 内部のコンポーネント間参照

**課題**: `resolve_conflicts()` は `TimelineManager`, `InstanceManager`, `SubscriptionManager` の3つを可変で操作する必要がある。`DolaRuntime` が全コンポーネントを所有するため、`&mut self` 経由では同時に取得できない。

**解決策候補**:

| 策 | 方式 | 評価 |
|----|------|------|
| **S1: 個別引数渡し** | `resolve(tm: &mut TimelineManager, im: &mut InstanceManager, sm: &mut SubscriptionManager, ...)` | ✅ 最もシンプル、borrowck フレンドリー。引数が多くなるが明快 |
| **S2: split borrow** | `let Self { timeline_manager, instance_manager, subscription_manager, .. } = self;` で destructure | ✅ Rust 的に正当。facade メソッド内での一時 destructure |
| **S3: 中間結果パターン** | ConflictResolver が `Vec<ConflictAction>` を返し、facade が `apply_actions()` で適用 | ✅ テスト最容易、最も分離が強い。❌ 2パスになるため Trim 値確定の精度に注意 |

**推奨**: **S1（個別引数渡し）** — 親仕様 design.md の trait interface と一致し、最もシンプル。

### 4.2 Conclude vs Compress: 値取得メソッドの差異

**課題**: Conclude と Compress で取得すべき「最終値」が異なる。

| 戦略 | 取得する値 | 既存メソッド |
|------|-----------|------------|
| **Conclude** | **現在再生中セグメント**の `to_value` (progress_t=1.0) | **なし** — 新規 `collect_current_segment_final_values()` が必要 |
| **Compress** | **ストーリーボード全体**の最終セグメントの `to_value` (progress_t=1.0) | `collect_final_values()` — **そのまま再利用可能** |

**新メソッドの概要**: `collect_current_segment_final_values(group_id, instances)`:
1. 各変数の TimelineEntry から group_id のエントリを探す
2. `calculate_effective_time()` で effective_time を算出
3. セグメント列を走査し、effective_time がカバーするアクティブセグメントを特定
4. そのセグメントの `to_value` を `Interpolator::interpolate(seg, type, 1.0)` で取得
5. 結果を `HashMap<String, EvaluatedValue>` で返す

### 4.3 Trim 戦略の値確定ロジック

**課題**: Trim は「割り込み開始時点まで再生して切断」を要求する。

**既存の再利用可能な関数**:
- `calculate_effective_time()`: 時刻→effective_time 変換 (**`pub(crate)` 化が必要**)
- `Interpolator::interpolate()`: 任意の progress_t で値を計算
- `evaluate_segments()`: アクティブセグメント走査ロジック (**参考パターン**)

**Trim の具体的操作**:
1. 新 SB の `start_time` を割り込み時点とする (Req 5 AC1)
2. `calculate_effective_time(start_time, instance)` で割り込み時点の effective_time を計算
3. 各変数のセグメント列を走査:
   - effective_time 時点でアクティブなセグメントを特定
   - `Interpolator::interpolate()` で割り込み時点の値を算出（= 確定値）
   - **アクティブセグメント以降のセグメントを除去**
4. 確定値を `force_update_last_values()` で購読者に伝播 (Req 5 AC2)
5. 状態遷移: Trimmed + cleanup

**Research Needed**: Trim 後のエントリ表現 — 2つの選択肢:
| 方式 | 説明 | 利点 | 欠点 |
|------|------|------|------|
| **A: セグメント列切断** | アクティブセグメント以降を `truncate` + end_time を割り込み時点に更新 | evaluate が自然に最終値を返す | セグメント内部の書き換えが必要 |
| **B: エントリ全削除 + 値のみ保持** | `remove_entries()` + `force_update_last_values()` | 既存 API で完結、実装がシンプル | Trim 後に「切断された値」としてタイムテーブルに残らない |

> **推奨**: **方式 B** — Cancel と同様にエントリ全削除 + 値伝播で十分。Trim の意味的な区別は `InstanceState::Trimmed` で表現される。

### 4.4 Never 延期キューの保持場所と解放トリガー

**課題**: `DeferredEntry` の保持場所と、先行 group_id 終了時の解放トリガー設計。

**設計候補比較**:

| 場所 | 利点 | 欠点 |
|------|------|------|
| **TimelineManager 内部** | 終了検知（evaluate 内の expired チェック、`remove_entries` 呼び出し時）で自然にトリガー可能 | TimelineManager の責務拡大 |
| **ConflictResolver struct 内部** | 責務明確 | facade が `&mut conflict_resolver` と `&mut timeline_manager` を同時に借用する必要 → Option A の場合 borrowck 問題再発 |
| **facade 直接保持** | 実装が最もシンプル | 設計の凝集度が低い |

**推奨**: **TimelineManager 内部** — 統合指針の「ConflictResolver が生成、TimelineManager が保持」のハイブリッド設計に合致。具体的には:
- `conflict_resolver::resolve_conflicts()` が `DeferredEntry` を生成し、`TimelineManager.add_deferred()` で格納
- `facade.update()` 内の自然終了/Conclude 処理後に `TimelineManager.flush_deferred(terminated_group_ids)` を呼び出して解放

**解放トリガーの範囲**: Req 7 AC3 は「Concluded / Cancelled / Trimmed / Compressed のいずれか」に遷移した場合と明記。全4終了状態で解放する。

### 4.5 InstanceManager の cleanup 一貫性

**課題**: 現在の `InstanceManager.transition()` は `Concluded` 遷移時のみ `self.instances.remove(group_id)` を行い、`Cancelled`/`Trimmed`/`Compressed` では行わない。facade の `cancel()` は別途 `self.instance_manager.remove(group_id)` を呼んでいる。

**影響**: ConflictResolver で4種の終了状態に遷移させる際、`Concluded` 以外は明示的な `remove()` が必要。

**設計選択肢**:
| 方式 | 説明 |
|------|------|
| **A: transition() を全終了状態で自動削除に変更** | `is_terminal()` チェックで統一。ただし既存 cancel() の `remove()` 呼び出しが冗長になる |
| **B: ConflictResolver 内で明示的に remove() を呼ぶ** | 既存コード変更なし。戦略関数内で `transition()` + `remove()` をセットで呼ぶ |

> **推奨**: **方式 A** — `transition()` を全終了状態で自動削除に統一。既存 `cancel()` の `remove()` は冗長になるが無害（`remove()` は存在しない key に対して何もしない）。デザインフェーズで判断。

---

## 5. 複雑度とリスク評価

### 5.1 工数見積もり

| コンポーネント | 工数 | 根拠 |
|--------------|------|------|
| 競合検出 (Req 1) | **S** (1-3日) | 時間範囲交差判定は単純アルゴリズム。Playing 状態フィルタ + 変数横断集約 |
| group_id 一括 + 戦略ディスパッチ (Req 2, 8) | **S** (1-2日) | policy 取得 + match 分岐。既存パターンの組み合わせ |
| Cancel 戦略 (Req 3) | **S** (1日) | 既存 `cancel()` パターンの模倣 |
| Conclude 戦略 (Req 4) | **M** (2-3日) | `collect_current_segment_final_values()` の新規実装が必要。アクティブセグメント特定のエッジケース検証（AC 3→1 Missing, 2 Existing） |
| Trim 戦略 (Req 5) | **M** (2-4日) | セグメント切断/値確定ロジックの新規実装。エッジケース（duration=0, 最終セグメント, 未開始セグメント）の検証 |
| Compress 戦略 (Req 6) | **S** (1日) | 既存 `conclude_internal()` パターンの模倣 |
| Never + 延期キュー (Req 7) | **M** (3-5日) | `DeferredEntry` 型定義、格納/解放ロジック、終了トリガー統合、無限ループとの組合せテスト |
| テスト（全戦略） | **M** (3-5日) | 5戦略 × 単体テスト + 統合テスト + エッジケース |
| **合計** | **M〜L** (8-16日) | |

### 5.2 リスク評価

| リスク | レベル | 説明 |
|--------|--------|------|
| 借用制約の設計 | **Medium** | facade の `&mut self` 制約。ただし個別引数渡し (S1) で解決パターンが明確 |
| Trim 戦略の正確性 | **Medium** | 割り込み時点の effective_time 計算 + セグメント切断。エッジケース検証が重要 |
| Conclude の「現在再生中セグメント」特定 | **Medium** | `evaluate_segments()` のパターンを参考にできるが、新メソッド実装 + セグメント境界のエッジケース |
| Never 延期キューのライフサイクル | **Medium** | 無限ループとの組み合わせ。永続保持時のメモリは仕様上許容だがテスト要確認 |
| InstanceManager の cleanup 変更 | **Low** | `transition()` の自動削除拡張。既存テスト通過は容易に確認可能 |
| 既存テストへの影響 | **Low** | ConflictResolver は新規モジュール。facade.rs 修正は Hook 挿入 + 延期解放フックのみ。既存 `runtime_facade_test.rs` はそのまま通過すべき |

---

## 6. 設計フェーズへの推奨事項

### 6.1 推奨アプローチ

**Option C（ハイブリッド: フリー関数 + 新モジュール）** + **S1（個別引数渡し）** を推奨。

理由:
1. 統合指針のモジュール構成に準拠（`conflict_resolver.rs`）
2. borrowck を自然に回避: facade が各コンポーネントの `&mut` を個別に渡す
3. 親仕様 design.md の trait interface に近い関数シグネチャ
4. DeferredEntry を TimelineManager に配置 → 終了トリガーで自然に検知可能
5. テスト容易: ConflictResolver 関数は純粋関数に近く、モック不要で単体テスト可能

### 6.2 設計フェーズで確定すべき事項

1. **Trim 後のエントリ表現**: 方式 A（セグメント列切断）vs 方式 B（エントリ全削除 + 値保持）— 方式 B を推奨
2. **InstanceManager の cleanup 統一**: `transition()` 全終了状態自動削除 vs ConflictResolver 内で明示 `remove()`
3. **延期キュー解放の facade フック位置**: `update()` 内の自然終了処理後 vs 別途メソッド
4. **`collect_current_segment_final_values()` の配置**: TimelineManager の新 `pub(crate)` メソッド vs conflict_resolver 内ヘルパー
5. **`evaluate_segments()` / `calculate_effective_time()` の公開範囲変更**: `pub(crate)` 化の可否

### 6.3 設計フェーズで不要な調査

- 外部クレート追加: 不要（`interpolation` 0.3.0 のみで十分）
- Feature gate 変更: 不要（integration-guide Section 5.2 で確認済み）
- 公開 API 変更: 不要（ConflictResolver は `pub(crate)` 非公開）
- `RuntimeError` 拡張: 不要（競合解決は Start 時に暗黙実行、エラー報告なし）
