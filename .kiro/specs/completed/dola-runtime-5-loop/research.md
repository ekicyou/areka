# Research & Design Decisions — dola-runtime-5-loop

## Summary
- **Feature**: `dola-runtime-5-loop`
- **Discovery Scope**: Extension（既存 Tier 2 ランタイムにループ再生機能を追加）
- **Key Findings**:
  - 既存の `calculate_effective_time()` は `start_time` ベースで時間計算しており、`loop_start_time` への置換で自然にループ対応可能
  - フリー関数（Option C）が borrowck 制約回避に最適。親 design.md の trait ベース設計を進化
  - `end_time` を「次の周回終了時刻」として管理する方式 A が有限/無限ループを統一的に処理できる

## Research Log

### 既存コードのループ対応状態
- **Context**: gap-analysis §1 で調査。Tier 2 暫定実装がどの程度ループを考慮しているか
- **Sources Consulted**: `facade.rs`, `instance_manager.rs`, `timeline_manager.rs` のソースコード
- **Findings**:
  - `StoryboardInstance` に `loop_count: i32`, `loops_completed: u32` フィールド既存
  - `loop_count` は `create_instance()` でコピーされるが、ループ判定ロジック未実装
  - `loops_completed` は初期値 0 で固定、インクリメントされない
  - `end_time = INFINITY` パターンが無限ループ用に存在するが、周回終了検出には使えない
- **Implications**: フィールドの型変更（u32→u64）と新規フィールド追加が必要。既存 API への影響は最小限

### effective_time 計算とループオフセットの設計
- **Context**: ループ継続時にタイムテーブルを再利用するには、effective_time 計算にループオフセットを統合する必要がある
- **Sources Consulted**: `timeline_manager.rs` L163-180 の `calculate_effective_time()` 関数
- **Findings**:
  - 現行: `effective_time = (current_time - start_time - pause_accumulated) * time_scale`
  - `start_time` を `loop_start_time` に置換するだけでループ対応可能
  - `pause_accumulated` は既存のまま独立動作（加算的分離）
- **Implications**: `loop_start_time` 方式は `start_time` 方式の自然な拡張。`pause_accumulated` との干渉なし

### facade の update() フローとループ処理挿入位置
- **Context**: `update()` の Step 2（自然終了検知）がループ処理の挿入ポイント
- **Sources Consulted**: `facade.rs` L260-278
- **Findings**:
  - Step 2 は `current_time >= inst.end_time` でフィルタし `conclude_internal()` を呼ぶ
  - ループ対応: conclude ではなく `loop_controller::process_loops()` を呼び、LoopAction を判定
  - `evaluate()` 前にループ処理を完了すれば、エントリ保持問題は自然に解決
- **Implications**: Step 2 の置換のみで対応。Step 1, 3, 4 は変更不要

### borrowck と設計パターン選択
- **Context**: `DolaRuntime` が `instance_manager` と `timeline_manager` を同時に可変参照する場面の回避
- **Sources Consulted**: 親仕様 design.md の LoopController 設計、gap-analysis §3
- **Findings**:
  - 親仕様は `trait LoopControllerApi` で `&StoryboardInstance` / `&mut StoryboardInstance` を受け取る設計
  - フリー関数化（Option C）すれば、facade が個別の参照を渡すだけで分割借用が不要
  - `conclude_internal()` のような `&mut self` メソッドとの組み合わせも自然に動作
- **Implications**: フリー関数群が Rust の所有権モデルに最適

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: struct ベース | `LoopController` 構造体を新規作成 | 統合指針に完全準拠 | `&mut self` 分割借用の問題 | 親 design.md の原案 |
| B: facade 拡張 | facade.rs 内のプライベートメソッド | 単純 | facade 肥大化、テスト困難 | 非推奨 |
| **C: フリー関数** | `loop_controller.rs` にフリー関数群 | borrowck 回避、テスト容易 | 状態が分散 | **採用** |

## Design Decisions

### Decision: loop_start_time + loop_duration フィールド設計
- **Context**: ループ継続時にタイムテーブルを再利用するための時間オフセット機構
- **Alternatives Considered**:
  1. `loop_offset: f64` — 累積加算方式（pause_accumulated と同パターン）
  2. `loop_start_time: f64` + `loop_duration: f64` — 周回開始時刻の直接管理
- **Selected Approach**: Option 2（loop_start_time + loop_duration）
- **Rationale**: 
  - 各周回の開始時刻が明示的で、デバッグ時に「現在何秒目から再生中か」が即座にわかる
  - `effective_time = (current_time - loop_start_time - pause_accumulated) * time_scale` で計算がシンプル
  - offset 累積方式は「現在のオフセット値」から逆算が必要で直感性に劣る
- **Trade-offs**: フィールド2個追加（loop_offset なら1個）。ただし明確性が勝る
- **Follow-up**: `calculate_effective_time()` の `start_time` を `loop_start_time` に変更

### Decision: end_time を「次の周回終了時刻」として管理（方式 A）
- **Context**: 無限ループ（loop_count=-1）の周回終了をどう検出するか
- **Alternatives Considered**:
  1. 方式 A: end_time を1周分に設定し、ループ時に再計算
  2. 方式 B: evaluate 結果で検出
  3. 方式 C: cycle_end_time フィールド新設
- **Selected Approach**: 方式 A
- **Rationale**: 既存の `current_time >= inst.end_time` フィルタをそのまま活用。有限/無限ループの統一処理。INFINITY を使わない
- **Trade-offs**: 無限ループの `end_time` が `INFINITY` ではなくなるため、外部から `end_time` を参照する既存コードに影響する可能性（ただし end_time は `pub(crate)` なので内部のみ）
- **Follow-up**: `start()` の end_time 算出ロジック変更、`calculate_end_time()` の返却値も変更

### Decision: loops_completed を u64 に変更
- **Context**: 無限ループでの周回数カウントの型選択
- **Alternatives Considered**:
  1. `u32` + `saturating_add` — オーバーフロー保護
  2. `u64` + wrapping — 実質無制限
  3. `u64` + カウント停止（無限ループ時）
- **Selected Approach**: Option 2（u64 + wrapping 許容）
- **Rationale**: u64 で実質的にオーバーフロー不可能（1秒ループでも5845億年）。万一 wrapping しても無限ループの動作に影響なし
- **Trade-offs**: 既存の `loops_completed: u32` との破壊的変更だが、Tier 2 では未使用なので影響なし
- **Follow-up**: `StoryboardInstance` のフィールド型変更、`create_instance()` の初期値そのまま

### Decision: フリー関数群（Option C）の採用
- **Context**: LoopController の実装パターン選択
- **Alternatives Considered**: 上記 Architecture Pattern Evaluation 参照
- **Selected Approach**: Option C（フリー関数群）
- **Rationale**: borrowck 制約を自然に回避。純粋関数に近い設計でテスト容易。統合指針のモジュール構成に準拠
- **Trade-offs**: ループ状態が `StoryboardInstance` のフィールドに分散するが、フィールド数は3個（loop_start_time, loop_duration, loops_completed）と少なく管理可能
- **Follow-up**: `loop_controller.rs` に `should_continue_loop()`, `advance_loop()`, `process_loops()` を定義

## Risks & Mitigations
- **周回終了検出の精度**: end_time ベース（方式 A）を採用し、evaluate との不整合を排除
- **Pause/Resume との組み合わせ**: `loop_start_time` と `pause_accumulated` の独立性により干渉なし。テストで組み合わせ検証
- **既存テストへの影響**: 新規モジュール中心。facade 修正は `start()` の end_time 計算 + `update()` の Step 2 のみ

## References
- `.kiro/specs/dola-runtime-engine/design.md` — 親仕様の LoopController 設計（L608-645）
- `.kiro/specs/dola-runtime-engine/integration-guide.md` — 子仕様統合指針（§2.3, 5.3）
- `.kiro/specs/dola-runtime-5-loop/gap-analysis.md` — ギャップ分析（§4.1-4.5）
- `.kiro/specs/dola-runtime-5-loop/requirements.md` — 要件定義（Req 1-5, 20 AC）
