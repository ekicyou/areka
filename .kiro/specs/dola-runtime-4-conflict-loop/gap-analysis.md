# ギャップ分析 — dola-runtime-4-conflict-loop

## 分析概要

本文書は `dola-runtime-4-conflict-loop` の要件（Req 1〜11）と既存コードベースのギャップを分析し、設計フェーズへの入力とする。

---

## 1. 現状調査

### 1.1 既存コードベースの構造

| モジュール | ファイル | 役割 | 本仕様との関連 |
|-----------|---------|------|--------------|
| `facade.rs` | `crates/dola/src/runtime/facade.rs` (326行) | 公開 API、start フロー制御 | **Tier 3 Hook 挿入点**（L116-117） |
| `instance_manager.rs` | `crates/dola/src/runtime/instance_manager.rs` (357行) | インスタンス状態管理 | **状態遷移先として Cancelled/Trimmed/Compressed 追加済み** |
| `timeline_manager.rs` | `crates/dola/src/runtime/timeline_manager.rs` (408行) | タイムテーブル評価 | **競合検出・エントリ操作の主要対象** |
| `instance_state.rs` | `crates/dola/src/runtime/instance_state.rs` (80行) | 状態 enum + 遷移 | **全7バリアント実装済み、`from_policy()` も完備** |
| `subscription_manager.rs` | `crates/dola/src/runtime/subscription_manager.rs` (280行) | 購読・差分配信 | 間接影響（Conclude/Cancel 時の値伝播） |
| `interpolator.rs` | `crates/dola/src/runtime/interpolator.rs` (388行) | イージング + 補間 | 値計算のユーティリティとして利用 |
| `types.rs` | `crates/dola/src/runtime/types.rs` (101行) | 公開型定義 | `EvaluatedValue`, `RuntimeError` |
| `mod.rs` | `crates/dola/src/runtime/mod.rs` (22行) | モジュール公開制御 | **新モジュール追加必要** |
| `storyboard.rs` | `crates/dola/src/storyboard.rs` (124行) | `InterruptionPolicy` 定義 | 5バリアント定義済み、デフォルト=Conclude |

### 1.2 既存のフック・拡張ポイント

#### facade.rs の Tier 3 Hook

```rust
// facade.rs L116-117
// 7. [Tier 3 Hook] 競合解決
// Tier 2: スキップ
```

`start()` メソッド内で、タイムテーブル挿入（L119）の直前に明示的なコメントフックが配置済み。

#### InstanceState の終了状態4種

`InstanceState` enum はすでに `Concluded`, `Cancelled`, `Trimmed`, `Compressed` の全4終了バリアントを定義済み。`from_policy()` メソッドも `InterruptionPolicy` → `InstanceState` の変換を提供:

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

#### Tier 2 の暫定動作

- **競合未解決**: 同一変数に複数 `group_id` エントリが共存し、最新 `group_id` の値が evaluate で採用される
- **ループ未実装**: `loop_count` フィールドは `StoryboardInstance` に保持されるが、実際には常に1回再生

### 1.3 コーディング規約・パターン

| パターン | 観察 |
|---------|------|
| 公開範囲 | `pub(crate)` を内部コンポーネントに使用、`pub` は facade の `DolaRuntime` のみ |
| エラー処理 | `RuntimeError` enum + `Result<T, RuntimeError>` |
| テスト配置 | 各モジュール内に `#[cfg(test)] mod tests`、統合テストは `tests/` ディレクトリ |
| HashMap/BTreeMap | 内部データは `HashMap`、外部定義は `BTreeMap` |
| ヘルパー関数 | モジュール内にプライベート関数（例: `calculate_effective_time()`, `evaluate_segments()`） |
| 所有権パターン | facade が全コンポーネントを所有し `&mut self` 経由で操作 |

---

## 2. 要件→既存資産マッピング

### 2.1 ConflictResolver（Req 1〜8）

| 要件 | 必要な機能 | 既存資産 | ギャップ |
|------|-----------|---------|---------|
| Req 1: 競合検出 | 新セグメントの時間範囲 vs 既存エントリの重複チェック | `VariableTimeline.entries` へのアクセスあり | **Missing**: 重複検出ロジック |
| Req 2: group_id 一括適用 | 競合 group_id の全変数へ戦略適用 | `remove_entries(group_id)` が全変数横断削除を実装済み | **Missing**: 戦略別の横断適用ロジック |
| Req 3: Cancel 戦略 | 現在値凍結 + Cancelled 遷移 | `cancel()` メソッドが facade に存在 | **Partial**: facade の cancel はコマンド用。競合トリガー版が必要 |
| Req 4: Conclude 戦略 | 現在再生中トランジション最終値ジャンプ + 未開始スキップ | `conclude_internal()` / `collect_final_values()` が存在 | **Partial**: 既存 `collect_final_values()` はストーリーボード全体の最終値（=Compress 相当）を返す。Conclude には「現在再生中セグメントの最終値」を取得する新メソッドが必要。また、競合トリガー版の呼び出しパスも未実装 |
| Req 5: Trim 戦略 | 割り込み時点での切断 + 補間値更新 | なし | **Missing**: Trim 固有の切断ロジック（セグメント分割/値確定） |
| Req 6: Compress 戦略 | 全トランジション最終値ジャンプ | `collect_final_values()` で最終値取得可能 | **Missing**: Compress 固有フロー（全完走扱い） |
| Req 7: Never + 延期キュー | 延期キュー（`DeferredEntry`） | なし | **Missing**: `DeferredEntry` 型、延期キュー、再評価トリガー |
| Req 8: デフォルト戦略 | 未指定時 Conclude | `default_interruption_policy()` = Conclude | **Existing**: storyboard.rs で既にデフォルト定義済み |

### 2.2 LoopController（Req 9〜11）

| 要件 | 必要な機能 | 既存資産 | ギャップ |
|------|-----------|---------|---------|
| Req 9: 基本ループ | loop_count 判定 → 周回制御 | `StoryboardInstance.loop_count` / `loops_completed` フィールド存在 | **Missing**: 周回判定ロジック |
| Req 10: タイムテーブル再利用 | pause_accumulated 機構でオフセット調整 | `pause_accumulated` フィールド + Resume の加算ロジック存在 | **Missing**: ループ用オフセット調整（同一機構の再利用） |
| Req 11: ループ中競合 | ループ中の競合検出保証 | evaluate の最新 group_id 優先 | **Missing**: ループ中のエントリが ConflictResolver の対象になる保証 |

### 2.3 ギャップサマリー

| カテゴリ | ステータス |
|---------|----------|
| **Existing** (そのまま使用) | `InstanceState` 全バリアント, `from_policy()`, `InterruptionPolicy` enum, デフォルト戦略, `conclude_internal()`, `remove_entries()`, `collect_final_values()` |
| **Partial** (拡張必要) | `facade.start()` の Tier 3 Hook, `InstanceManager` の状態遷移, `TimelineManager` のエントリ操作 |
| **Missing** (新規作成) | `ConflictResolver` モジュール, `LoopController` モジュール, `DeferredEntry` 型, 時間重複検出ロジック, Trim 切断ロジック, ループ周回制御 |

---

## 3. 実装アプローチ選択肢

### Option A: 新モジュール作成（推奨）

統合指針 Section 5.3 に従い、`conflict_resolver.rs` と `loop_controller.rs` を新規作成する。

**対象ファイル**:

| 操作 | ファイル | 内容 |
|------|---------|------|
| **新規作成** | `runtime/conflict_resolver.rs` | `ConflictResolver` struct + 5戦略実装 |
| **新規作成** | `runtime/loop_controller.rs` | `LoopController` struct + 周回制御 |
| **修正** | `runtime/mod.rs` | `mod conflict_resolver; mod loop_controller;` 追加 |
| **修正** | `runtime/facade.rs` | `start()` 内の Tier 3 Hook を実装に置換、`update()` にループ制御を挿入 |
| **修正** | `runtime/timeline_manager.rs` | 延期キュー (`deferred_entries`)、時間重複チェック用メソッド追加 |
| **修正** | `runtime/instance_manager.rs` | `instances_mut()` (可変アクセス)、ループ関連ヘルパー追加 |

**トレードオフ**:
- ✅ 統合指針に完全準拠、モジュール構成が明確
- ✅ 独立テスト可能な単位として設計できる
- ✅ 既存 facade 層への影響を最小化
- ❌ `&mut self` の借用制約で facade → ConflictResolver → TimelineManager/InstanceManager の連鎖呼び出しに設計工夫が必要

### Option B: facade 拡張（非推奨）

ConflictResolver/LoopController のロジックを facade.rs 内のプライベートメソッドとして実装する。

**トレードオフ**:
- ✅ 借用制約が単純（全フィールドが `self` 配下）
- ❌ facade.rs が肥大化（326行 → 推定600行超）
- ❌ 統合指針の設計に反する
- ❌ 単体テストが困難

### Option C: ハイブリッド（関数ベース + 新モジュール）

ConflictResolver/LoopController を struct ではなくフリー関数群として実装し、facade が `&mut self` 内部のフィールド参照を個別に渡す。

**トレードオフ**:
- ✅ borrowck の制約を自然に回避（各フィールドの個別 `&mut` が可能）
- ✅ テスト容易（純粋関数に近い設計）
- ✅ 統合指針の trait interface に近い
- ❌ trait メソッドではなくフリー関数のため、将来的なモック化がやや困難

---

## 4. 技術的課題と調査事項

### 4.1 Rust 借用制約: facade 内部のコンポーネント間参照

**課題**: `ConflictResolver::resolve_conflicts()` は `TimelineManager` と `InstanceManager` の両方を可変で操作する必要がある。しかし `DolaRuntime` が全コンポーネントを所有するため、`&mut self` 経由では同時に `&mut timeline_manager` と `&mut instance_manager` を取得できない。

**解決策候補**:

| 策 | 方式 | 評価 |
|----|------|------|
| **S1: 個別引数渡し** | `resolve(tm: &mut TimelineManager, im: &mut InstanceManager, ...)` | ✅ 最もシンプル、borrowck フレンドリー |
| **S2: 一時的な split borrow** | facade のフィールドを destructure して個別に渡す | ✅ Rust 的に正当、やや冗長 |
| **S3: 中間結果パターン** | ConflictResolver が計画（`Vec<ConflictAction>`）を返し、facade が適用 | ✅ テスト容易、分離が最も強い |

> **Research Needed**: 設計フェーズで S1/S3 を比較評価

### 4.2 Trim 戦略の値確定ロジック

**課題**: Trim は「割り込み開始時点まで再生して切断」を要求する。これは既存の `evaluate_segments()` で intermediate 値を計算し、その結果でタイムテーブルを書き換える操作を意味する。

**既存資産**:
- `calculate_effective_time()`: 時刻→effective_time 変換（再利用可能）
- `Interpolator::interpolate()`: 任意の progress_t で値を計算（再利用可能）
- `evaluate_segments()`: モジュールプライベート関数（`pub(crate)` 化が必要）

**ギャップ**: セグメント列を途中で切断し、残りを除去した新しいエントリを生成するロジックが存在しない。

> **Research Needed**: Trim 後のタイムテーブルエントリの具体的な表現（セグメント列の切断 vs 新しいエントリ差し替え）

### 4.3 Never 延期キューの保持場所

**課題**: `DeferredEntry` の保持場所として `ConflictResolver` 内部か `TimelineManager` 内部かの選択。

**設計候補**:

| 場所 | 利点 | 欠点 |
|------|------|------|
| `TimelineManager` 内部 | エントリ追加のトリガー（group_id 終了）をevaluate フロー内で検知可能 | TimelineManager の責務拡大 |
| `ConflictResolver` 内部 | 責務が明確（競合解決の一環） | 終了トリガーの通知パスが必要 |

> **Research Needed**: 統合指針の設計ノート (design.md L777-815) では「ConflictResolver が生成、TimelineManager が保持」のハイブリッドを示唆

### 4.4 ループのオフセット調整と pause_accumulated の統合

**課題**: ループ継続時に `pause_accumulated` を調整してタイムテーブルを再利用する設計。既存の `pause_accumulated` は Pause/Resume 用で加算のみ。ループでは「1周分の duration」を加算する別用途が発生する。

**既存資産**:
- `StoryboardInstance.pause_accumulated`: f64 フィールド（加算済み一時停止時間）
- `calculate_effective_time()`: `pause_accumulated` を差し引いて effective_time を算出

**注意点**: `pause_accumulated` にループオフセットを加算すると、Pause/Resume の一時停止時間との混同リスクがある。別フィールド（例: `loop_offset`）を追加するか、`pause_accumulated` を汎用化するかの判断が必要。

> **Research Needed**: 設計フェーズでフィールド設計を確定

### 4.5 evaluate() フロー内でのループ完了検出

**課題**: 現在の `evaluate_segments()` は全セグメント終了時に `None` を返し、呼び出し元がエントリを expired として削除する。ループ時はこの動作を変更し、「全セグメント終了 → ループ判定 → 継続/終了」のフローに切り替える必要がある。

**影響範囲**:
- `timeline_manager.rs` の `evaluate()` メソッド
- `facade.rs` の `update()` 内の自然終了検知ロジック

---

## 5. 複雑度とリスク評価

### 5.1 工数見積もり

| コンポーネント | 工数 | 根拠 |
|--------------|------|------|
| ConflictResolver (Req 1-8) | **M** (3-7日) | 5戦略 × 個別実装 + Never 延期キュー。パターンは既存資産から類推可能だが、Trim/Never に固有の複雑性あり |
| LoopController (Req 9-11) | **S** (1-3日) | 周回判定とオフセット調整。既存の pause_accumulated 機構を参考にできる |
| facade 統合 | **S** (1-3日) | Tier 3 Hook の実装、update() へのループ統合、mod.rs 更新 |
| テスト | **M** (3-7日) | 5戦略 × 単体テスト + ループテスト + 統合テスト |
| **合計** | **M〜L** (7-14日) | |

### 5.2 リスク評価

| リスク | レベル | 説明 |
|--------|--------|------|
| 借用制約の設計 | **Medium** | facade の `&mut self` 制約で ConflictResolver/LoopController への可変参照渡しに工夫が必要。ただし解決パターンは明確 |
| Trim 戦略の正確性 | **Medium** | セグメント途中切断の補間値計算は精度が重要。既存 Interpolator を再利用できるが、エッジケース（duration=0, 最終セグメント）の検証が必要 |
| Never 延期キューのライフサイクル | **Medium** | 無限ループとの組み合わせで永続的に保持されるエントリのメモリ管理。仕様上は許容だが、テストでの確認が必要 |
| ループと競合の相互作用 | **Low-Medium** | ループ中の各周回が競合対象となる設計は明確だが、周回境界でのタイミング整合が必要 |
| 既存テストへの影響 | **Low** | ConflictResolver/LoopController は新規モジュール。facade.rs の修正は Tier 3 Hook 挿入のみで、既存テスト（502行）は暫定動作テストとして有効 |

---

## 6. 設計フェーズへの推奨事項

### 6.1 推奨アプローチ

**Option C（ハイブリッド: フリー関数 + 新モジュール）** を推奨。

理由:
1. 統合指針のモジュール構成に準拠（`conflict_resolver.rs`, `loop_controller.rs`）
2. `facade.rs` の Tier 3 Hook から分離されたフィールド参照を個別に渡すことで borrowck を自然に回避
3. 親仕様 design.md の trait interface に近い形で関数シグネチャを定義可能

### 6.2 設計フェーズで確定すべき事項

1. **ConflictResolver の呼び出しパターン**: 個別引数渡し (S1) vs 中間結果パターン (S3)
2. **DeferredEntry の保持場所**: TimelineManager vs ConflictResolver vs 新構造体
3. **ループオフセットフィールド**: `pause_accumulated` 再利用 vs `loop_offset` 新設
4. **Trim 後のエントリ表現**: セグメント列切断 vs 凍結値の直接格納
5. **evaluate() へのループ統合**: LoopController コールバック vs evaluate 内判定

### 6.3 設計フェーズで不要な調査

- 外部クレート追加: 不要（`interpolation` 0.3.0 のみで十分）
- Feature gate 変更: 不要（integration-guide Section 5.2 で確認済み）
- 公開 API 変更: 不要（ConflictResolver/LoopController は非公開）
