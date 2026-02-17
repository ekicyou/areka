# Research & Design Decisions — dola-nested-storyboard

## Summary
- **Feature**: `dola-nested-storyboard`
- **Discovery Scope**: Extension（既存 dola クレートの拡張）
- **Key Findings**:
  - 既存の純粋キーフレームエントリと同パターンで「再生時間0秒」のトリガーエントリを実現可能
  - `update()` の `&mut self` 制約は中間バッファ方式で解決可能
  - ストーリーボード間の循環検出は `validate.rs` に新規追加。既存 KF 内トポソートとは独立したドキュメントレベル検証

## Research Log

### update() 内での start() 呼び出し: &mut self 再帰借用問題
- **Context**: `DolaRuntime::update()` は `&mut self` を取る。トリガー発火時に内部で `start()` を呼ぶと二重借用が発生
- **Sources Consulted**: facade.rs L270-353（update 実装）、facade.rs L68-145（start 実装）
- **Findings**:
  - `update()` は Step1(deadline) → Step2(loop/終了) → Step3(evaluate) → Step4(diff) の4段階
  - `start()` は document取得 → compile → instance作成 → 競合解決 → timeline挿入 → Playing遷移
  - 両メソッドとも `instance_manager`, `timeline_manager`, `subscription_manager` を同時に参照する
- **Implications**:
  - 解決策: `update()` の Step2 と Step3 の間に「トリガー収集→実行」フェーズを挿入
  - トリガー対象を `Vec<PendingTrigger>` に収集後、借用が解放された状態で `start()` を順次実行
  - `start()` ロジックは既存インターフェースをそのまま再利用可能

### CompiledStoryboard へのトリガー情報格納
- **Context**: トリガーエントリのコンパイル結果をどこに保持するか
- **Sources Consulted**: compile.rs（CompiledStoryboard 構造体）、timeline_manager.rs（insert_entries）
- **Findings**:
  - 現在の `CompiledStoryboard` は `timelines: BTreeMap<String, CompiledVariableTimeline>` のみ
  - トリガーはセグメント（from→to 補間）ではないため、タイムラインとは別の構造が必要
  - `CompiledTrigger { fire_time: f64, target_storyboard: String, start_offset: Option<f64> }` を新設
- **Implications**:
  - `CompiledStoryboard` に `triggers: Vec<CompiledTrigger>` フィールドを追加
  - `total_base_duration` 計算にはトリガーの fire_time は影響しない（0秒完了原則）
  - シリアライズ互換: `#[serde(default)]` で既存JSONとの後方互換を維持

### ストーリーボード間循環検出の実装位置
- **Context**: トリガーチェーン A→B→C→A の循環検出をどこで行うか
- **Sources Consulted**: validate.rs（V1-V13）、compile.rs（topological_sort）
- **Findings**:
  - 既存の `topological_sort()` はエントリ内 KF 依存のみ（単一ストーリーボードスコープ）
  - トリガー循環検出は**ドキュメントレベル**（全ストーリーボード横断）で行う必要がある
  - `validate()` 内に新規パスとして追加するのが最も自然（既存 V1-V13 の延長）
- **Implications**:
  - `validate.rs` に V14（自己参照検出）、V15（循環参照検出）を追加
  - DFS による SB 間有向グラフの循環検出（SB 数は通常少数のため O(V+E) で十分高速）
  - `compile.rs` の `topological_sort()` は変更不要（KF 依存解決は既存のまま）

### トリガー発火状態のランタイム管理
- **Context**: `update()` 内でトリガーの発火済み/未発火を追跡する方法
- **Sources Consulted**: instance_manager.rs（StoryboardInstance）、loop_controller.rs（process_loops）
- **Findings**:
  - 各トリガーに `fire_time`（絶対時刻）が設定される
  - `current_time >= fire_time` かつ未発火であればトリガーを実行
  - ループ時は周回ごとに `fire_time` が `loop_start_time` 基準で再計算される
  - `StoryboardInstance` にトリガー状態を保持するか、別の `TriggerTracker` を用意するか
- **Implications**:
  - `StoryboardInstance` に `trigger_states: Vec<TriggerState>` を追加
  - 各 `TriggerState` は `{ compiled_trigger_index: usize, fired: bool }`
  - ループ周回時（`advance_loop`）に全トリガーの `fired = false` にリセット

## Architecture Pattern Evaluation

| Option            | Description                                                             | Strengths                              | Risks / Limitations                  | Notes                   |
| ----------------- | ----------------------------------------------------------------------- | -------------------------------------- | ------------------------------------ | ----------------------- |
| A: フィールド拡張 | `StoryboardEntry` に `trigger_storyboard` / `trigger_start_offset` 追加 | serde 互換維持、既存配置パターン再利用 | フィールド数増加（6→8）、認知負荷    | 最もシンプル            |
| B: enum 化        | `StoryboardEntry` を enum に変更                                        | 型レベル排他制約                       | **破壊的変更**、既存テスト大規模修正 | リスク大                |
| C: ハイブリッド   | 外部=A、内部で正規化                                                    | 外部互換 + 内部型安全                  | 正規化ステップの複雑性               | 推奨（gap-analysis.md） |

→ **選択: Option C（ハイブリッド）**。外部フォーマット互換を維持しつつ、コンパイラ内部で正規化処理を行う。

## Design Decisions

### Decision: トリガーエントリの「0秒完了」semantics
- **Context**: トリガーエントリの keyframe 登録時刻として、子 SB の再生時間を参照するか否か
- **Alternatives Considered**:
  1. 子 SB の1回再生終了時刻を keyframe として登録 — ストーリーボード間依存解決が必要
  2. トリガー発火時刻を keyframe として登録（再生時間0秒） — 既存コンパイラスコープ内で完結
- **Selected Approach**: 案2（0秒完了）
- **Rationale**: 
  - 案1はドキュメントレベルのトポロジカルソート（SB 間依存解決）が必要で、compile.rs の大規模改造（推定300+行）が必要
  - 案2は既存の純粋キーフレームエントリと同パターンで、コンパイラ変更は最小限
  - dola の設計原則「決定論的動作」を維持し、複雑なランタイム制御は外部に委譲
- **Trade-offs**: シーケンシャル起動（子の終了を待ってから次の処理）は本仕様の対象外
- **Follow-up**: 需要があれば将来的に Event 機構を追加して対応

### Decision: update() 返却型の拡張
- **Context**: トリガー実行結果をどのように呼び出し元に通知するか
- **Alternatives Considered**:
  1. 既存の `Vec<(String, EvaluatedValue)>` を維持し、トリガー結果は別チャネルで通知
  2. 新しい `UpdateResult` 構造体で `changes` と `triggered` を包含
- **Selected Approach**: 案2（`UpdateResult` 構造体）
- **Rationale**:
  - トリガー結果は `update()` の文脈で発生するため、同一の返却値に含めるのが自然
  - `UpdateResult` で既存の changes と新規の triggered を構造化
  - 既存コードは `update_result.changes` でアクセス（移行コストは低い）
- **Trade-offs**: 既存 API の破壊的変更だが、型を変更するだけで移行は機械的

### Decision: ストーリーボード間循環検出の実装位置
- **Context**: validate.rs（バリデーション時）vs compile.rs（コンパイル時）
- **Alternatives Considered**:
  1. `validate.rs` に新規パスとして追加（ドキュメントレベル検証）
  2. `compile.rs` のトポロジカルソートを拡張
- **Selected Approach**: 案1（validate.rs）
- **Rationale**:
  - 循環検出は「静的検証」であり、コンパイルの前処理として行うのが適切
  - compile.rs の既存トポソートはエントリ内 KF 依存解決であり、SB 間検出とは異なるスコープ
  - validate.rs に集約することで検証ルールの一覧性が向上
- **Trade-offs**: compile_storyboard() 内の validate() 呼び出しで自動的にカバーされる

## Risks & Mitigations
- **update() 返却型の破壊的変更** — `UpdateResult` 構造体導入で段階的移行。既存テストの修正が必要だが変更は機械的
- **ループ＋トリガーの組み合わせバグ** — ループ周回時のトリガー状態リセットを `loop_controller` に統合し、テストで網羅
- **中間バッファ方式のパフォーマンス** — トリガー数は通常少数（SB あたり 1-3 個）のため `Vec` 確保のオーバーヘッドは無視可能

## References
- [gap-analysis.md](gap-analysis.md) — 既存アセット調査と実装アプローチ評価
- [requirements.md](requirements.md) — 要件定義（5 要件、20+ AC）
- compile.rs — 既存コンパイラ実装（依存グラフ、トポソート、セグメント生成）
- runtime/facade.rs — ランタイム Facade パターン（start/update フロー）
