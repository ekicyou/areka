# ギャップ分析 — dola-runtime-5-loop

## 分析概要

本文書は `dola-runtime-5-loop` の要件（Req 1〜2）と既存コードベースのギャップを分析し、設計フェーズへの入力とする。

---

## 1. 現状調査

### 1.1 既存コードベースの構造

| モジュール | ファイル | 役割 | 本仕様との関連 |
|-----------|---------|------|--------------|
| `facade.rs` | `crates/dola/src/runtime/facade.rs` (326行) | 公開 API、update フロー制御 | **自然終了検知ロジック**（L270）にループ判定を挿入 |
| `instance_manager.rs` | `crates/dola/src/runtime/instance_manager.rs` (357行) | インスタンス状態管理 | **`loop_count`, `loops_completed` フィールド保持** |
| `timeline_manager.rs` | `crates/dola/src/runtime/timeline_manager.rs` (408行) | タイムテーブル評価 | evaluate() 内でループオフセット調整が必要 |
| `instance_state.rs` | `crates/dola/src/runtime/instance_state.rs` (80行) | 状態 enum + 遷移 | 終了状態への遷移（ループ完了時） |
| `storyboard.rs` | `crates/dola/src/storyboard.rs` (124行) | `loop_count` 定義 | `i32` 型: 1=1回, n≥2=n回, -1=無限ループ |

### 1.2 既存のループ関連フィールド

#### StoryboardInstance の loop フィールド

```rust
pub struct StoryboardInstance {
    // ... 他のフィールド ...
    /// 1=1回, n≥2=n回, -1=無限ループ
    pub loop_count: i32,
    /// Tier 2: 常に 0
    pub loops_completed: u32,
    // ...
}
```

`loop_count` は Tier 2 でコピーされるが、`loops_completed` は常に 0 で未使用。

### 1.3 Tier 2 の暫定動作

- **ループ未実装**: `loop_count` は無視され、常に1回再生
- **自然終了検知**: `update()` 内で `current_time >= inst.end_time` を検知し、`conclude_internal()` を呼ぶ

### 1.4 コーディング規約・パターン

| パターン | 観察 |
|---------|------|
| 公開範囲 | `pub(crate)` を内部コンポーネントに使用、`pub` は facade の `DolaRuntime` のみ |
| エラー処理 | `RuntimeError` enum + `Result<T, RuntimeError>` |
| テスト配置 | 各モジュール内に `#[cfg(test)] mod tests`、統合テストは `tests/` ディレクトリ |
| 時間オフセット | `pause_accumulated` フィールドを Pause/Resume で使用 |
| 所有権パターン | facade が全コンポーネントを所有し `&mut self` 経由で操作 |

---

## 2. 要件→既存資産マッピング

### 2.1 LoopController（Req 1〜2）

| 要件 | 必要な機能 | 既存資産 | ギャップ |
|------|-----------|---------|---------|
| Req 1: 基本ループ | loop_count 判定 → 周回制御 | `StoryboardInstance.loop_count` / `loops_completed` フィールド存在 | **Missing**: 周回判定ロジック |
| Req 2 AC1: タイムテーブル1周分のみ | ループ展開しない設計 | `insert_entries()` は既に1周分のみ生成 | **Existing**: 実装済み |
| Req 2 AC2: loop_count チェック | 全セグメント終了時の判定 | `update()` の自然終了検知（L270） | **Missing**: ループ継続判定ロジック |
| Req 2 AC3: 時間オフセット調整 | pause_accumulated 機構でオフセット調整 | `pause_accumulated` フィールド + Resume の加算ロジック存在 | **Missing**: ループ用オフセット調整（同一機構の再利用） |
| Req 2 AC4: ループ完了時の終了 | 終了状態遷移 + エントリ削除 | `conclude_internal()` パターン存在 | **Existing**: 呼び出しパスのみ必要 |

### 2.2 ギャップサマリー

| カテゴリ | ステータス |
|---------|----------|
| **Existing** (そのまま使用) | `loop_count` / `loops_completed` フィールド, タイムテーブル1周分設計, `conclude_internal()` パターン |
| **Partial** (拡張必要) | `facade.update()` の自然終了検知ロジック |
| **Missing** (新規作成) | `LoopController` モジュール, 周回判定ロジック, ループ用時間オフセット調整 |

---

## 3. 実装アプローチ選択肢

### Option A: 新モジュール作成（推奨）

統合指針 Section 5.3 に従い、`loop_controller.rs` を新規作成する。

**対象ファイル**:

| 操作 | ファイル | 内容 |
|------|---------|------|
| **新規作成** | `runtime/loop_controller.rs` | `LoopController` struct + 周回制御 |
| **修正** | `runtime/mod.rs` | `mod loop_controller;` 追加 |
| **修正** | `runtime/facade.rs` | `update()` 内の自然終了検知にループ制御を挿入 |
| **修正** | `runtime/instance_manager.rs` | ループ関連ヘルパー追加（必要に応じて） |

**トレードオフ**:
- ✅ 統合指針に完全準拠、モジュール構成が明確
- ✅ 独立テスト可能な単位として設計できる
- ✅ 既存 facade 層への影響を最小化
- ❌ facade との結合点を明確にする必要あり

### Option B: facade 拡張（非推奨）

LoopController のロジックを facade.rs 内のプライベートメソッドとして実装する。

**トレードオフ**:
- ✅ 実装が単純（単一ファイル内で完結）
- ❌ facade.rs がさらに肥大化
- ❌ 統合指針の設計に反する
- ❌ 単体テストが困難

### Option C: ハイブリッド（関数ベース + 新モジュール）

LoopController を struct ではなくフリー関数群として実装し、facade が `&mut self` 内部のフィールド参照を個別に渡す。

**トレードオフ**:
- ✅ borrowck の制約を自然に回避
- ✅ テスト容易（純粋関数に近い設計）
- ✅ シンプルな関数シグネチャ（`should_continue_loop(instance: &StoryboardInstance) -> bool` 等）
- ❌ 状態管理が分散（LoopController が状態を持たない）

---

## 4. 技術的課題と調査事項

### 4.1 ループのオフセット調整と pause_accumulated の統合

**課題**: ループ継続時に `pause_accumulated` を調整してタイムテーブルを再利用する設計。既存の `pause_accumulated` は Pause/Resume 用で加算のみ。ループでは「1周分の duration」を加算する別用途が発生する。

**既存資産**:
- `StoryboardInstance.pause_accumulated`: f64 フィールド（加算済み一時停止時間）
- `calculate_effective_time()`: `pause_accumulated` を差し引いて effective_time を算出

**注意点**: `pause_accumulated` にループオフセットを加算すると、Pause/Resume の一時停止時間との混同リスクがある。別フィールド（例: `loop_offset`）を追加するか、`pause_accumulated` を汎用化するかの判断が必要。

> **Research Needed**: 設計フェーズでフィールド設計を確定

### 4.2 evaluate() フロー内でのループ完了検出

**課題**: 現在の `evaluate_segments()` は全セグメント終了時に `None` を返し、呼び出し元がエントリを expired として削除する。ループ時はこの動作を変更し、「全セグメント終了 → ループ判定 → 継続/終了」のフローに切り替える必要がある。

**影響範囲**:
- `timeline_manager.rs` の `evaluate()` メソッド
- `facade.rs` の `update()` 内の自然終了検知ロジック

**設計候補**:

| 方式 | 説明 | 評価 |
|------|------|------|
| **facade 内判定** | `update()` で `end_time` 到達を検知し、LoopController を呼ぶ | ✅ シンプル、既存フローに近い |
| **evaluate 内判定** | `evaluate()` がループ情報を参照してエントリ削除を制御 | ❌ TimelineManager の責務拡大 |

> **Research Needed**: facade 内判定を推奨（既存の自然終了検知パターンと一貫）

### 4.3 ループ中の競合検出保証

**課題**: 親仕様 Req 12.8「ループ中も競合検出の対象」は、実際には LoopController の要件ではなく ConflictResolver の不変条件。

**解決**: 本仕様では特別な考慮は不要。LoopController は単に `Playing` 状態を維持するだけで、ConflictResolver が自然に競合を検出する。

---

## 5. 複雑度とリスク評価

### 5.1 工数見積もり

| コンポーネント | 工数 | 根拠 |
|--------------|------|------|
| LoopController (Req 1-2) | **S** (1-3日) | 周回判定とオフセット調整。既存の pause_accumulated 機構を参考にできる |
| facade 統合 | **S** (1日) | `update()` へのループ制御挿入、mod.rs 更新 |
| テスト | **S** (2-3日) | 3種ループ（1回/n回/無限） × 周回境界テスト |
| **合計** | **S〜M** (4-7日) | |

### 5.2 リスク評価

| リスク | レベル | 説明 |
|--------|--------|------|
| pause_accumulated の混同 | **Low-Medium** | ループオフセットを同一フィールドに格納する場合、Pause/Resume との意味的分離が必要。ただし実装は明確 |
| evaluate() フローへの影響 | **Low** | 自然終了検知ロジックの拡張のみ。既存パターンを踏襲可能 |
| 既存テストへの影響 | **Low** | LoopController は新規モジュール。facade.rs の修正は1箇所のみ |

---

## 6. 設計フェーズへの推奨事項

### 6.1 推奨アプローチ

**Option C（ハイブリッド: フリー関数 + 新モジュール）** を推奨。

理由:
1. 統合指針のモジュール構成に準拠（`loop_controller.rs`）
2. シンプルな関数シグネチャ（状態を持たない純粋関数群）
3. facade との結合が明確（`update()` の自然終了検知ロジック内から呼び出し）

### 6.2 設計フェーズで確定すべき事項

1. **ループオフセットフィールド**: `pause_accumulated` 再利用 vs `loop_offset` 新設
2. **evaluate() へのループ統合**: facade 内判定（推奨） vs evaluate 内判定
3. **loops_completed の更新タイミング**: ループ継続判定前 vs 判定後

### 6.3 設計フェーズで不要な調査

- 外部クレート追加: 不要
- Feature gate 変更: 不要
- 公開 API 変更: 不要（LoopController は非公開）
- 競合検出との統合: 不要（ConflictResolver の責務）
