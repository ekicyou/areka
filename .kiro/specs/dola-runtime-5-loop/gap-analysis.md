# ギャップ分析 — dola-runtime-5-loop

## 分析概要

本文書は `dola-runtime-5-loop` の要件（Req 1〜5、20 AC）と既存コードベースのギャップを分析し、設計フェーズへの入力とする。

---

## 1. 現状調査

### 1.1 既存コードベースの構造

| モジュール | ファイル | 行数 | 役割 | 本仕様との関連 |
|-----------|---------|------|------|--------------|
| `facade.rs` | `runtime/facade.rs` | 326 | 公開 API、update フロー制御 | **自然終了検知ロジック**（L268-278）にループ判定を挿入 |
| `instance_manager.rs` | `runtime/instance_manager.rs` | 357 | インスタンス状態管理 | **`loop_count`, `loops_completed` フィールド保持**、Pause/Resume 実装 |
| `timeline_manager.rs` | `runtime/timeline_manager.rs` | 408 | タイムテーブル評価 | `calculate_effective_time()` でループオフセット統合が必要 |
| `instance_state.rs` | `runtime/instance_state.rs` | 80 | 状態 enum（7 バリアント） + 遷移 | 終了状態への遷移（ループ完了時）、`Playing` 状態維持 |
| `storyboard.rs` | `src/storyboard.rs` | 124 | `loop_count` 定義 | `i32` 型: 1=1回, n≥2=n回, -1=無限ループ |
| `types.rs` | `runtime/types.rs` | 101 | RuntimeError enum | `InvalidLoopCount`, `ZeroDurationWithLoop` 既存 |
| `mod.rs` | `runtime/mod.rs` | 23 | re-export | `mod loop_controller;` 追加が必要 |

### 1.2 既存のループ関連フィールド

#### StoryboardInstance の loop フィールド

```rust
pub struct StoryboardInstance {
    // ... 他のフィールド ...
    pub pause_accumulated: f64,   // Pause/Resume の時間補正
    pub pause_start: Option<f64>, // Pause 開始時刻
    /// 1=1回, n≥2=n回, -1=無限ループ
    pub loop_count: i32,
    /// Tier 2: 常に 0
    pub loops_completed: u32,
    pub end_time: f64,
}
```

- `loop_count`: Tier 2 でコンパイル結果からコピーされるが、ループ判定ロジックは未実装
- `loops_completed`: 初期値 `0` で固定、インクリメントされない
- `pause_accumulated`: Pause/Resume 専用。ループの時間オフセットには未使用

#### end_time の算出（facade.rs L96-101）

```rust
let end_time = if compiled.loop_count == -1 {
    f64::INFINITY  // 無限ループ → INFINITY
} else {
    // Tier 2: ループ未実装、1回再生として扱う
    start_time + compiled.total_base_duration / compiled.time_scale
};
```

### 1.3 Tier 2 の暫定動作

- **ループ未実装**: `loop_count` は無視され、常に1回再生
- **自然終了検知**: `update()` 内で `current_time >= inst.end_time` を検知し、`conclude_internal()` を呼ぶ
- **evaluate_segments()**: 全セグメント終了時に `None` を返し、エントリを expired として削除

### 1.4 effective_time 計算（timeline_manager.rs L166-180）

```rust
fn calculate_effective_time(current_time: f64, instance: &StoryboardInstance) -> f64 {
    let raw_time = if instance.state == InstanceState::Paused {
        match instance.pause_start {
            Some(pause_start) => pause_start - instance.start_time - instance.pause_accumulated,
            None => current_time - instance.start_time - instance.pause_accumulated,
        }
    } else {
        current_time - instance.start_time - instance.pause_accumulated
    };
    raw_time * instance.time_scale
}
```

ループ対応には、`pause_accumulated` と同様の減算オフセット機構が必要。

### 1.5 コーディング規約・パターン

| パターン | 観察 |
|---------|------|
| 公開範囲 | `pub(crate)` を内部コンポーネントに使用、`pub` は facade の `DolaRuntime` のみ |
| エラー処理 | `RuntimeError` enum + `Result<T, RuntimeError>` |
| テスト配置 | 各モジュール内に `#[cfg(test)] mod tests`、統合テストは `tests/` ディレクトリ |
| 時間オフセット | `pause_accumulated` フィールドを Pause/Resume で使用（加算のみ） |
| 所有権パターン | facade が全コンポーネントを所有し `&mut self` 経由で操作 |
| 自然終了パターン | `conclude_internal()`: 最終値取得 → last_values 更新 → Concluded 遷移 → エントリ削除 |

---

## 2. 要件→既存資産マッピング

### 2.1 Req 1: ループ再生 — 基本動作

| AC | 必要な機能 | 既存資産 | ギャップ |
|----|-----------|---------|---------|
| AC1: loop_count=1 | 1回再生後に終了 | Tier 2 の自然終了検知がそのまま動作 | **Existing** |
| AC2: loop_count=-1 | 無限ループ継続 | `end_time = INFINITY` 設定済み → 自然終了検知は発動しない | **Missing**: 1周分終了時のループ再開ロジック |
| AC3: loop_count=n | n回再生後に終了 | `end_time` が1回分のみで計算 | **Missing**: 周回判定 + end_time 再計算 or ループオフセット |
| AC4: 周回終了時の判定 | loops_completed 更新 + 継続判定 | `loops_completed` フィールド存在、未使用 | **Missing**: 判定ロジック全体 |
| AC5: Playing 状態維持 | ループ中は conclude しない | `conclude_internal()` が呼ばれると Concluded 遷移 | **Missing**: ループ中の conclude 抑制 |

### 2.2 Req 2: タイムテーブル再利用

| AC | 必要な機能 | 既存資産 | ギャップ |
|----|-----------|---------|---------|
| AC1: 1周分のみ生成 | n周分のタイムテーブル展開なし | `insert_entries()` は既に1周分のみ生成 | **Existing** |
| AC2: loops_completed 比較 | loop_count と比較して継続判定 | フィールド存在、比較ロジックなし | **Missing**: 比較関数 |
| AC3: オフセット調整で再利用 | タイムテーブル破棄せず再利用 | `evaluate()` が expired エントリを削除する既存動作 | **Missing**: ループ時のエントリ保持 + オフセット調整 |
| AC4: duration 累積 | 1周分 duration を加算 | `pause_accumulated` は pause 専用 | **Missing**: ループオフセット機構 |
| AC5: ループ完了時の終了 | 終了遷移 + エントリ削除 | `conclude_internal()` パターン存在 | **Partial**: 呼び出しパスのみ追加 |

### 2.3 Req 3: ループ周回トラッキング

| AC | 必要な機能 | 既存資産 | ギャップ |
|----|-----------|---------|---------|
| AC1: loops_completed 管理 | 周回数の読み書き | `pub loops_completed: u32` フィールド存在 | **Partial**: 書き込みロジック不在 |
| AC2: インクリメント | 周回完了時に+1 | なし | **Missing**: インクリメントロジック |
| AC3: 初期値 0 | 生成時に 0 | `create_instance()` で `loops_completed: 0` | **Existing** |
| AC4: u32::MAX 飽和 | 無限ループ時のオーバーフロー保護 | なし | **Missing**: `saturating_add` 使用 |

### 2.4 Req 4: Pause/Resume との相互作用

| AC | 必要な機能 | 既存資産 | ギャップ |
|----|-----------|---------|---------|
| AC1: Pause 時のループ状態保持 | 周回数・再生位置を保持 | Pause は `pause_start` を記録し状態を固定 | **Partial**: ループ固有フィールドの Pause 時挙動確認が必要 |
| AC2: Resume 後の正確な再開 | 周回 + 位置から再開 | Resume は `pause_accumulated` を加算し `end_time` を再計算 | **Partial**: ループオフセットとの組み合わせ検証 |
| AC3: 独立オフセット管理 | ループ offset ≠ pause offset | `pause_accumulated` は pause 専用 | **Missing**: 別フィールドでのループオフセット管理 or 関心分離 |

### 2.5 Req 5: ループと外部制御の境界

| AC | 必要な機能 | 既存資産 | ギャップ |
|----|-----------|---------|---------|
| AC1: Cancel 時の即座停止 | ループ中でも Cancel 動作 | `cancel()` は Playing/Paused → Cancelled 遷移 + エントリ削除 | **Existing**: ループ中でもそのまま動作 |
| AC2: ConflictResolver との独立 | ループは競合を意識しない | 設計上独立 | **Existing by design** |
| AC3: Playing 状態維持 | 競合検出対象に含まれる | LoopController が Playing を維持すれば自動的に対象 | **Existing by design**: LoopController の Playing 維持がキー |

### 2.6 ギャップサマリー

| カテゴリ | 項目 |
|---------|------|
| **Existing** (8 AC) | Req 1 AC1, Req 2 AC1, Req 3 AC3, Req 5 AC1/AC2/AC3, end_time=INFINITY (loop_count=-1) |
| **Partial** (4 AC) | Req 2 AC5, Req 3 AC1, Req 4 AC1/AC2 |
| **Missing** (8 AC) | Req 1 AC2/AC3/AC4/AC5, Req 2 AC2/AC3/AC4, Req 3 AC2/AC4, Req 4 AC3 |

**コア欠落**: ループ周回判定ロジック、時間オフセット調整機構、`evaluate()` のエントリ保持制御

---

## 3. 実装アプローチ選択肢

### Option A: 新モジュール（struct ベース）

統合指針 Section 5.3 に従い、`loop_controller.rs` に `LoopController` struct を新規作成する。

**対象ファイル**:

| 操作 | ファイル | 内容 |
|------|---------|------|
| **新規作成** | `runtime/loop_controller.rs` | `LoopController` struct + 周回制御メソッド群 |
| **修正** | `runtime/mod.rs` | `mod loop_controller;` 追加 |
| **修正** | `runtime/facade.rs` | `update()` の自然終了検知をループ対応に拡張 |
| **修正** | `runtime/instance_manager.rs` | ループ関連ヘルパー追加（`increment_loop`, `reset_loop_offset` 等） |
| **修正** | `runtime/timeline_manager.rs` | `calculate_effective_time()` のループオフセット対応 |

**トレードオフ**:
- ✅ 統合指針に完全準拠、モジュール構成が明確
- ✅ 独立テスト可能な単位として設計できる
- ❌ facade が `LoopController` を所有し、borrowck の制約を受ける可能性
- ❌ `&mut self` の分割借用が必要になる場面がある

### Option B: facade 拡張（非推奨）

LoopController のロジックを facade.rs 内のプライベートメソッドとして実装する。

**トレードオフ**:
- ✅ 実装が単純（単一ファイル内で完結）
- ❌ facade.rs がさらに肥大化（326行 → 400行超）
- ❌ 統合指針の設計に反する
- ❌ 単体テストが困難

### Option C: ハイブリッド（フリー関数 + 新モジュール）— 推奨

`loop_controller.rs` を新規作成するが、struct ではなくフリー関数群として実装。facade が `&mut StoryboardInstance` や `&mut TimelineManager` の参照を個別に渡す。

**関数シグネチャ候補**:

```rust
/// ループ継続の可否を判定する純粋関数
pub(crate) fn should_continue_loop(instance: &StoryboardInstance) -> bool

/// 周回完了処理: loops_completed インクリメント + オフセット調整
pub(crate) fn advance_loop(instance: &mut StoryboardInstance)

/// ループ完了判定 + 自然終了の分岐
pub(crate) enum LoopAction { Continue, Conclude }
pub(crate) fn check_loop_completion(instance: &StoryboardInstance) -> LoopAction
```

**対象ファイル**:

| 操作 | ファイル | 内容 |
|------|---------|------|
| **新規作成** | `runtime/loop_controller.rs` | フリー関数群（`should_continue_loop`, `advance_loop` 等） |
| **修正** | `runtime/mod.rs` | `mod loop_controller;` 追加 |
| **修正** | `runtime/facade.rs` | `update()` の自然終了検知で `loop_controller::*` を呼び出し |
| **修正** | `runtime/instance_manager.rs` | `StoryboardInstance` にループオフセットフィールド追加（必要に応じて） |
| **修正** | `runtime/timeline_manager.rs` | `calculate_effective_time()` のループオフセット対応 |

**トレードオフ**:
- ✅ borrowck の制約を自然に回避（分割借用不要）
- ✅ テスト容易（純粋関数に近い設計、`StoryboardInstance` のモック不要）
- ✅ 統合指針のモジュール構成に準拠
- ✅ シンプルな関数シグネチャで責務が明確
- ❌ 状態管理が `StoryboardInstance` のフィールドに分散（LoopController 自体は状態を持たない）

---

## 4. 技術的課題と調査事項

### 4.1 ループ用周回開始時刻管理

**課題**: ループ継続時にタイムテーブルを再利用するには、現在の周回開始時刻を正確に管理する必要がある。`update()` での周回終了検出は遅延するため、次周回は「実際の周回終了時刻」から開始されなければならない。

**既存の effective_time 計算**:
```
effective_time = (current_time - start_time - pause_accumulated) * time_scale
```

**ループ対応後**:
```
effective_time = (current_time - loop_start_time - pause_accumulated) * time_scale
```

周回終了時:
```rust
loop_start_time += loop_duration  // 次周回の開始時刻に更新
end_time += loop_duration         // 次周回の終了時刻に更新
```

**必要なフィールド**:
- `loop_start_time: f64` — 現在の周回の開始時刻（初期値は `start_time`）
- `loop_duration: f64` — 1周分の時間 `base_duration / time_scale`（定数）

**利点**:
- ✅ 明確な関心分離（Req 4 AC3）: `loop_start_time` はループ専用、`pause_accumulated` は Pause 専用
- ✅ シンプルな計算: オフセット累積ではなく開始時刻の更新
- ✅ 遅延検出でも正確なタイミング維持

### 4.2 自然終了検知のループ分岐

**課題**: 現在の `update()` フロー（facade.rs L267-278）:

```rust
// Step 2: 自然終了検知
let naturally_ended: Vec<u64> = self.instance_manager.instances()
    .iter()
    .filter(|(_, inst)| inst.state == InstanceState::Playing && current_time >= inst.end_time)
    .map(|(gid, _)| *gid)
    .collect();

for gid in naturally_ended {
    self.conclude_internal(gid);
}
```

**ループ対応後のフロー**:

```rust
// Step 2: 周回終了検知（複数周回対応）
while current_time >= end_time {
    loops_completed += 1;
    
    if loops_completed >= loop_count {  // ループ完了
        conclude_internal(group_id);
        break;
    }
    
    // 次周回へ
    loop_start_time += loop_duration;
    end_time += loop_duration;
}
```

**重要**: `update()` の呼び出し間隔が長い場合、複数周回が一度に終了する可能性がある。`while` ループで全終了済み周回を一括処理することで、正確な周回数管理を実現する。

**解決策**: 4.3 で推奨する方式 A（end_time を1周分の「次の周回終了時刻」として管理）を採用すれば、有限/無限ループの統一処理が可能。

### 4.3 無限ループ (loop_count=-1) の周回終了検出

**課題**: `end_time = INFINITY` のため、facade の `current_time >= inst.end_time` フィルタに引っかからない。周回終了を別の方法で検出する必要がある。

**選択肢**:

| 方式 | 説明 | 評価 |
|------|------|------|
| **A: end_time を1周分に設定** | 無限ループでも `end_time = start_time + duration/time_scale` とし、ループ時に再計算 | ✅ 既存の自然終了検知をそのまま活用、✅ 有限/無限の統一処理 |
| **B: evaluate 結果で検出** | `evaluate_segments()` が `None` を返した時点で周回終了と判定 | ❌ evaluate のタイミングは変数依存、❌ 購読変数なしの場合検出不可 |
| **C: 周回 end_time フィールド新設** | `cycle_end_time: f64` を追加し、1周分の終了時刻を管理 | ✅ end_time（全体）と cycle_end_time（周回）の分離が明確、❌ フィールド増加 |

> **推奨**: **方式 A**。end_time を常に「次の周回終了時刻」として管理し、ループ完了時に初めて Conclude する。`INFINITY` は使用しない。これにより有限/無限ループの処理が統一できる。

### 4.4 evaluate() のエントリ保持とループ

**課題**: `evaluate()` は全セグメント終了時にエントリを expired として削除する（L112-114）。ループ再生時はエントリを保持し、オフセット調整後に再評価する必要がある。

**影響**: ループ中のインスタンスのエントリが `evaluate()` によって削除されると、次周回の評価値が失われる。

**解決候補**:
1. **facade 先行判定**: `update()` の evaluate 呼び出し**前に**ループ処理を完了し、`loop_offset` を調整。evaluate は調整済み effective_time で評価するため、セグメントが再度アクティブになる → **エントリ削除は発生しない**
2. **evaluate 内でループ対応**: evaluate がインスタンスの `loop_count` を参照し、ループ中の expired を保持 → **TimelineManager の責務拡大（非推奨）**

> **推奨**: 方式 1（facade 先行判定）。update() の Step 2 でループオフセット調整を完了すれば、Step 3 の evaluate は通常通り動作する。

### 4.5 Pause とループ開始時刻の独立性

**課題**: Req 4 AC3「独立して管理し、相互に干渉しない」。

**分析**: `loop_start_time` 方式（4.1）を採用すれば自動的に満たされる。

```
effective_time = (current_time - loop_start_time - pause_accumulated) * time_scale
```

- `loop_start_time`: LoopController のみが更新（周回終了時に `+= loop_duration`）
- `pause_accumulated`: Pause/Resume のみが更新（Pause 時に累積加算）
- 両者は独立したフィールド → 相互干渉なし

**Resume 時の end_time 再計算**: 既存の `resume()` は `end_time += pause_duration` で調整。ループ対応後も同一ロジックが適用可能（`end_time` は常に「次の周回終了時刻」）。`loop_start_time` は Pause/Resume で変更されない。

---

## 5. 複雑度とリスク評価

### 5.1 工数見積もり

| コンポーネント | 工数 | 根拠 |
|--------------|------|------|
| LoopController フリー関数群 (Req 1, 3) | **S** (1-2日) | 周回判定・インクリメント・飽和加算。純粋関数で実装シンプル |
| loop_start_time 機構 (Req 2, 4) | **S** (1-2日) | フィールド追加 + `calculate_effective_time()` 修正。既存パターン踏襲 |
| facade 統合 (Req 1-5) | **S** (1-2日) | `update()` の while ループ挿入、`start()` の end_time 計算修正 |
| 単体テスト (Req 1-5) | **S** (2-3日) | 下記テストケース群を実施 |
| 統合テスト | **S** (1日) | facade 経由のエンドツーエンド |
| **合計** | **S〜M** (6-10日) | |

#### 重要なテストケース

**基本ループ動作** (Req 1):
- loop_count=1: 1回再生後に Conclude
- loop_count=3: 3回再生後に Conclude
- loop_count=-1: 無限ループ（Cancel までの動作確認）

**複数周回一括処理** (Req 1 AC4, Req 2 AC2) — **抜けやすいクリティカルケース**:
- loop_count=3, duration=2秒, update(5秒) で一度に2周終了
- loop_count=5, duration=1秒, update(10秒) で全周回が一度に完了
- 無限ループで複数周回を飛ばした場合の loops_completed の正確性

**Pause/Resume 組合せ** (Req 4):
- ループ中に Pause → Resume 後の正確な周回・位置復帰
- 複数周回飛ばし + Pause の組合せ

**外部制御** (Req 5):
- ループ中の Cancel 即座停止

### 5.2 リスク評価

| リスク | レベル | 説明 |
|--------|--------|------|
| 周回終了検出タイミング | **Medium** | end_time ベース vs evaluate ベースの選択。設計フェーズで確定 |
| `loop_offset` + `pause_accumulated` の組合せ | **Low** | 加算的独立で干渉なし。テストで組合せ検証 |
| evaluate() のエントリ保持 | **Low** | facade 先行判定方式で解決可能（4.4 参照） |
| 既存テストへの影響 | **Low** | 新規モジュール中心。facade の修正は `update()` の1箇所 + `start()` の end_time 計算 |
| 無限ループの u32 オーバーフロー | **Low** | `saturating_add` で対処（Req 3 AC4） |

---

## 6. 設計フェーズへの推奨事項

### 6.1 推奨アプローチ

**Option C（ハイブリッド: フリー関数 + 新モジュール）** を推奨。

理由:
1. 統合指針のモジュール構成に準拠（`loop_controller.rs`）
2. シンプルな関数シグネチャ（状態を持たない純粋関数群）
3. borrowck の制約を自然に回避
4. facade との結合が明確（`update()` の自然終了検知ロジック内から呼び出し）

### 6.2 設計フェーズで確定すべき事項

1. **ループ周回管理フィールド**: `loop_start_time: f64` と `loop_duration: f64` を新設（Req 4 AC3 直接対応）
2. **周回終了検出方式**: end_time ベース（4.3 方式 A）。`end_time` を常に「次の周回終了時刻」として管理（無限ループでも `INFINITY` を使わない）
3. **update() 内のループ処理位置**: Step 2（自然終了検知）をループ対応に拡張。evaluate 前にループ処理を完了
4. **loops_completed の更新タイミング**: ループ継続判定**前**にインクリメント（判定は `loops_completed >= loop_count`）

### 6.3 設計フェーズで不要な調査

- 外部クレート追加: 不要
- Feature gate 変更: 不要
- 公開 API 変更: 不要（LoopController は `pub(crate)` フリー関数群）
- 競合検出との統合: 不要（ConflictResolver の責務、LoopController は Playing 維持のみ）
- `InstanceState` の変更: 不要（既存の 7 バリアントで十分）
