# ギャップ分析 — dola-nested-storyboard

## 1. 現状調査

### 1.1 関連アセット一覧

| レイヤー       | ファイル                          | 責務                                                               |
| -------------- | --------------------------------- | ------------------------------------------------------------------ |
| データモデル   | `storyboard.rs`                   | `Storyboard`, `StoryboardEntry`, `KeyframeRef`, `BetweenKeyframes` |
| データモデル   | `document.rs`                     | `DolaDocument`（variable/transition/storyboard の BTreeMap）       |
| データモデル   | `transition.rs`                   | `TransitionDef`, `TransitionRef`, `TransitionValue`                |
| データモデル   | `value.rs`                        | `DynamicValue`（動的値型）                                         |
| バリデーション | `validate.rs`                     | `Validate` trait — V1〜V13 のバリデーションルール                  |
| エラー         | `error.rs`                        | `DolaError` enum（13 バリアント）                                  |
| コンパイラ     | `compile.rs`                      | `compile_storyboard()` — 依存グラフ→トポソート→セグメント生成      |
| ランタイム     | `runtime/facade.rs`               | `DolaRuntime` — start/update/pause/resume/conclude/cancel API      |
| ランタイム     | `runtime/instance_manager.rs`     | `StoryboardInstance`, `InstanceManager`                            |
| ランタイム     | `runtime/timeline_manager.rs`     | `TimelineManager` — 変数ごとのタイムテーブル評価                   |
| ランタイム     | `runtime/loop_controller.rs`      | `process_loops()` — ループ周回進行                                 |
| ランタイム     | `runtime/conflict_resolver.rs`    | `resolve_conflicts()` — 5種終了戦略                                |
| ランタイム     | `runtime/subscription_manager.rs` | 購読・差分配信                                                     |
| 公開API        | `lib.rs`                          | re-export 一覧                                                     |
| テスト         | `tests/`                          | 10+ テストファイル                                                 |

### 1.2 アーキテクチャパターン

- **宣言→コンパイル→ランタイム** の3段パイプライン
- `StoryboardEntry` は現在 **4配置パターン** のバリエーション（前エントリ連結 / KF起点 / KF間 / 純粋KF）
- コンパイラは **エントリ単位でトポロジカルソート** し、キーフレーム時刻を逐次解決
- ランタイムは `update()` 内で **ループ処理→評価→差分配信** のみ行い、新規 `start()` は外部から呼ぶ設計
- 競合解決は `start()` 時に `resolve_conflicts()` で同期実行

### 1.3 重要な制約

1. **`update()` は現在 `&mut self` のみで副作用が差分配信に限定** — `start()` を内部から呼ぶには `&mut self` の再帰借用問題あり
2. **`compile_storyboard()` は単一ストーリーボードスコープ** — 他のストーリーボードへの依存は想定していない
3. **`DolaDocument` は全ストーリーボード定義を保持** — トリガー先のストーリーボードは同一ドキュメント内に存在する前提で動作可能
4. **`CompiledStoryboard` にはトリガー概念がない** — 変数タイムラインのみで構成される

---

## 2. 要件→アセット対応マップ

### Requirement 1: ストーリーボードトリガーエントリ

| AC                                | 技術的ニーズ                     | 既存アセット                            | ギャップ                                                                                   |
| --------------------------------- | -------------------------------- | --------------------------------------- | ------------------------------------------------------------------------------------------ |
| 1.1 trigger_storyboard フィールド | `StoryboardEntry` に新フィールド | `storyboard.rs`                         | **Missing**: `trigger_storyboard: Option<String>` フィールドが未存在                       |
| 1.2 4配置パターン統合             | コンパイラのタイミング解決       | `compile.rs` — `resolve_entry_timing()` | **要拡張**: 純粋KFと同様にタイミング解決が必要だが、variable/transition がないケースの分岐 |
| 1.3 variable/transition と排他    | バリデーション                   | `validate.rs` — V7〜V9                  | **Missing**: trigger エントリの排他チェック                                                |
| 1.4 keyframe 許容                 | 既存 keyframe 処理               | `compile.rs` — `keyframe_times` 管理    | **互換**: 純粋KFと同等の keyframe 登録で対応可能                                           |
| 1.5 トリガー先存在確認            | バリデーション                   | `validate.rs`                           | **Missing**: ストーリーボード名の存在確認ルール                                            |

### Requirement 2: トリガー実行とランタイム統合

| AC                                | 技術的ニーズ                  | 既存アセット                                                      | ギャップ                                                            |
| --------------------------------- | ----------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------- |
| 2.1 update() でのトリガー自動実行 | ランタイム内 start() 呼び出し | `facade.rs` — `update()`                                          | **Missing**: トリガー時刻到達検知 + 内部 start() 呼び出しメカニズム |
| 2.2 競合エラーの返却              | update() の返却型拡張         | `facade.rs` — `update()` 返却型は `Vec<(String, EvaluatedValue)>` | **Missing**: トリガー結果を返す型が必要、返却型の変更または拡張     |
| 2.3 独立インスタンス管理          | 既存 instance_manager         | `instance_manager.rs`                                             | **互換**: 既存の `start()` 機構で独立 group_id を自動発番           |
| 2.4 親子ライフサイクル独立        | 設計上の保証                  | 既存設計                                                          | **互換**: 現在のインスタンス管理は既に独立動作                      |
| 2.5 トリガー結果の追跡            | update() 返却型拡張           | —                                                                 | **Missing**: `StartResult` 情報を含む新しい返却構造体               |

### Requirement 3: コンパイル時バリデーション

| AC                                   | 技術的ニーズ                     | 既存アセット                         | ギャップ                                                                |
| ------------------------------------ | -------------------------------- | ------------------------------------ | ----------------------------------------------------------------------- |
| 3.1 自己参照検出                     | ドキュメントレベルバリデーション | `validate.rs`                        | **Missing**: ストーリーボード間参照の検証（現在はエントリ内 KF のみ）   |
| 3.2 循環参照検出                     | グラフトラバーサル               | `compile.rs` — `topological_sort()`  | **要新規**: 既存はKF依存のみ。ストーリーボード間の有向グラフ検出が必要  |
| 3.3 duration にトリガー先を含めない  | コンパイラ duration 計算         | `compile.rs` — `total_base_duration` | **互換**: トリガーエントリはセグメントを生成しないため自然に除外        |
| 3.4 トランジション固有フィールド拒否 | バリデーション                   | `validate.rs`                        | **Missing**: trigger エントリに transition フィールドがある場合のエラー |
| 3.5 DolaError 拡張                   | エラーバリアント追加             | `error.rs`                           | **Missing**: トリガー関連の新しいエラーバリアント                       |

### Requirement 4: 宣言的フォーマット対応

| AC                                 | 技術的ニーズ             | 既存アセット                                     | ギャップ                                                  |
| ---------------------------------- | ------------------------ | ------------------------------------------------ | --------------------------------------------------------- |
| 4.1 JSON/TOML/YAML 対応            | serde 属性               | `storyboard.rs` — `Serialize`/`Deserialize` 既存 | **互換**: フィールド追加 + serde 属性で自動対応           |
| 4.2 最小構成                       | serde default/skip       | 既存パターン                                     | **互換**: `Option<String>` + `skip_serializing_if` で対応 |
| 4.3 trigger_start_offset           | 新フィールド             | —                                                | **Missing**: `trigger_start_offset: Option<f64>`          |
| 4.4 at/between/keyframe 組み合わせ | 既存配置パターン         | `compile.rs`                                     | **互換**: 既存の純粋KFパターンを拡張                      |
| 4.5 混在配置                       | 配列内型ポリモーフィズム | `Vec<StoryboardEntry>`                           | **互換**: 単一構造体にフィールド追加で対処可能            |

### Requirement 5: ループとの相互作用

| AC                                 | 技術的ニーズ                     | 既存アセット           | ギャップ                                                  |
| ---------------------------------- | -------------------------------- | ---------------------- | --------------------------------------------------------- |
| 5.1 ループ反復ごとの再実行         | トリガー時刻の周回オフセット計算 | `loop_controller.rs`   | **Missing**: ループ周回時のトリガー再発火メカニズム       |
| 5.2 無限ループでのトリガー         | 同上                             | 同上                   | **Missing**: 同上                                         |
| 5.3 子ストーリーボードのループ独立 | 設計上の保証                     | 既存設計               | **互換**: 独立インスタンス管理で自然に満たされる          |
| 5.4 競合解決での処理               | 既存競合解決                     | `conflict_resolver.rs` | **互換**: 既存の `resolve_conflicts()` がそのまま適用可能 |

---

## 3. 実装アプローチ評価

### Option A: StoryboardEntry フィールド拡張 + CompiledStoryboard にトリガーリスト

**概要**: 既存の `StoryboardEntry` に `trigger_storyboard` / `trigger_start_offset` フィールドを追加。コンパイル結果に `CompiledTrigger` リストを新設。ランタイムの `update()` でトリガー時刻を監視し、内部 `start()` を実行。

**変更対象ファイル**:
- `storyboard.rs` — フィールド追加
- `compile.rs` — `CompiledTrigger` 構造体追加、トリガーエントリのコンパイル処理
- `validate.rs` — 排他チェック、自己参照/循環検出
- `error.rs` — 新エラーバリアント
- `runtime/facade.rs` — `update()` 内トリガー実行、返却型拡張
- `runtime/timeline_manager.rs` — トリガーエントリの保持（or 新モジュール `trigger_manager.rs`）
- `lib.rs` — 新型の re-export

**トレードオフ**:
- ✅ 既存 `StoryboardEntry` の serde 互換性を維持（新フィールドは全て Optional）
- ✅ 既存の配置パターン解決ロジックを直接再利用
- ✅ `CompiledTrigger` はセグメントと分離されるため `total_base_duration` に自然に影響しない
- ❌ `StoryboardEntry` のフィールド数が増加（6 → 8）、認知負荷が上がる
- ❌ `update()` 内の `&mut self` 再帰借用を解決するために中間バッファが必要

### Option B: StoryboardEntry を enum 化（エントリタイプ分離）

**概要**: `StoryboardEntry` を enum に変更し、`TransitionEntry` と `TriggerEntry` のバリアントを分離。各バリアントは必要最小限のフィールドのみを持つ。

**変更対象ファイル**: Option A と同一 + 既存テストの大規模修正

**トレードオフ**:
- ✅ 型レベルでの排他制約（variable+transition vs trigger_storyboard を型で区別）
- ✅ 各バリアントのフィールドが明確
- ❌ **破壊的変更**: 全既存テストと JSON フォーマットに影響
- ❌ serde の `untagged` enum は JSON パース時のエラーメッセージが不親切
- ❌ 既存の `at`/`between`/`keyframe` が両バリアントで重複定義になる

### Option C: ハイブリッド（フィールド拡張 + 内部正規化）

**概要**: 外部形式は Option A（フィールド追加）、コンパイラ内部で正規化してトリガーとトランジションを分離処理。

**変更対象ファイル**: Option A と同一

**トレードオフ**:
- ✅ 外部フォーマット互換維持（Option A と同等）
- ✅ 内部的には型安全な分離が可能
- ✅ 将来的に Option B への移行パスを残せる
- ❌ 正規化ステップの追加で複雑性が若干増加

---

## 4. `update()` 内トリガー実行の技術的課題

### 課題: `&mut self` 再帰借用

現在の `update()` は `&mut self` を取る。内部で `self.start()` を呼ぶと二重借用になる。

**解決策**:
1. **中間バッファ方式**: `update()` の前半でトリガー対象を `Vec<PendingTrigger>` に収集し、後半で順次 `start()` を実行。ループ処理→トリガー収集→トリガー実行→評価→差分配信の順序。
2. **返却型でのトリガー委譲**: トリガー情報を `update()` の返却値に含め、呼び出し元が `start()` を実行。ランタイム内部の責務を最小化。

**推奨**: 方式1。ランタイムの宣言的オーケストレーションの意義を維持し、利用者の負担を増やさない。

### 課題: トリガー時刻の追跡

`CompiledStoryboard` にトリガーリストを保持し、各トリガーの `fire_time` と `fired` フラグを `update()` 内で管理する必要がある。ループ時は周回ごとにフラグをリセット。

---

## 5. 複雑性・リスク評価

| 項目       | 評価           | 根拠                                                                                                                                                      |
| ---------- | -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **工数**   | **M** (3〜7日) | 既存パターンの拡張が中心。新モジュール追加は少ないが、コンパイラ/ランタイム/バリデーションの3層すべてに変更が必要                                         |
| **リスク** | **Medium**     | `update()` の再帰借用は中間バッファで解決可能。循環検出は新規ロジックだが既存のトポソートパターンを参考にできる。既存テストへの影響は Option A なら最小限 |

### リスク詳細

| リスク                           | 影響度 | 軽減策                                                              |
| -------------------------------- | ------ | ------------------------------------------------------------------- |
| `update()` 返却型変更が破壊的    | 中     | 新しい `UpdateResult` 構造体に `changes` と `triggers` を包含       |
| ループ＋トリガーの組み合わせバグ | 中     | 周回ごとのトリガー状態リセットを `loop_controller` に統合           |
| 循環検出の漏れ                   | 低     | 全ストーリーボードの有向グラフを `validate()` 時に構築して DFS 検査 |

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ: **Option C（ハイブリッド）**

外部フォーマット互換を維持しつつ、内部で型安全な処理を行う。

### 設計フェーズで決定すべき事項

1. **`update()` 返却型**: 既存の `Vec<(String, EvaluatedValue)>` を維持 vs 新しい `UpdateResult` 構造体
2. **トリガー状態管理**: `timeline_manager` に統合 vs 新しい `trigger_manager.rs` モジュール
3. **循環検出の実装位置**: `validate.rs`（ドキュメントレベル）vs `compile.rs`（コンパイル時）
4. **ループ周回でのトリガーフラグリセット**: `loop_controller` 拡張 vs `facade.rs` 内処理

### Research Needed

- (なし — 全て既存パターンの拡張で対応可能。外部依存の追加は不要)
