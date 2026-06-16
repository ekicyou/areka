# D1a-S: dola ランタイム中核 × シンプル化

- status: completed
- commit: refactor(D1a): facade バリデーション重複の共通化と playback.rs の陳腐化 TODO 除去

## findings

### S6（karpathy-guidelines）基準で検証した候補一覧

境界: `crates/dola/src/runtime/{facade,loop_controller}.rs`, `runtime/{timeline_manager,subscription_manager}/`, `src/playback.rs`（D1a-T 由来 38 件を含む 248 テスト = dola unit 122 + tests/runtime 126 が回帰検知器）:

| # | 候補 | S6 根拠 | 判定 |
|---|------|---------|------|
| 1 | `facade.rs`: `start_internal` / `calculate_end_time` のバリデーション二重実装（D1a-T 所見3） | 自明な重複（同一の取得→存在確認→コンパイル→loop_count/duration 検査が 2 箇所、片側変更で乖離するリスク） | **適用** |
| 2 | `playback.rs:1` の `// TODO: Implement PlaybackState, ScheduleRequest`（D1a-T 所見4） | 陳腐化コメント（両型は実装済み）。構造的整理 | **適用** |
| 3 | `facade.rs`: `cancel()`/`finish()` の `is_terminal()` 防御分岐（D1a-T 所見2: 実質到達不能） | 到達不能性が境界外 `instance_manager` の不変条件（terminal 遷移＝自動削除）に依存。防御分岐の除去はロジック変更を伴う | 見送り→ **P6 提案記録** |
| 4 | `loop_controller.rs`: `apply_easing` が `interpolator::apply_named_easing`/`apply_parametric_easing` と完全重複（32+2 アームの match × 2 箇所） | 重複統合には interpolator 側（D1b-S 境界）の可視性変更が必要で、どちらのセル単独でも完結しない | 見送り→ **P7 提案記録** |
| 5 | `loop_controller.rs`: `process_loops` の `loop_count == 1` 特別扱い | 冗長に見えるが除去すると `advance_loop` が走り内部状態が変化（`process_loops_single_loop_conclude` / `process_loops_loop_count_1_ignores_offset` が `loops_completed == 0` を固定）。冗長ではない | 見送り |
| 6 | `timeline_manager/mod.rs`: `collect_final_values` / `evaluate_all_for_group` / `collect_current_segment_final_values` の「timelines 走査 + group_id フィルタ」3 重出現 | ループ本体は各々異なり、抽出ヘルパの利得は約 6 行。単純で慣用的な for ループの抽象化は S6 #2（churn 回避・最小 diff）に照らし利得僅少 | 見送り |
| 7 | `timeline_manager/mod.rs`: `has_entries` の `#[allow(dead_code)]` | tests.rs から 7 箇所使用されており dead ではない（非テストビルドでの lint 抑制として機能中）。`#[cfg(test)]` への変更は等価で churn | 見送り |
| 8 | `subscription_manager/mod.rs`: `diff_and_update` の clone 削減・`evaluate` の best_value 追跡の書き換え | 動作する慣用コードのマイクロ最適化＝壊れていないものを直さない（S6 #3） | 見送り |
| 9 | `facade.rs`: `update()`（`#[deprecated]`） | tests/ 5 ファイルで使用中（`#![allow(deprecated)]`）→ R2.9 の「利用ゼロ実証」を満たさず削除不可。かつ公開 API 変更は本セルの制約外。既に P5（playback 旧型）と同種の整理対象として認識済みのため新規提案は不要 | 見送り |

### 適用した簡素化と根拠

1. **`facade.rs`: バリデーション共通化** — `start_internal` と `calculate_end_time` に二重実装されていた「ドキュメント取得 → ストーリーボード存在確認 → コンパイル → loop_count / loop_duration バリデーション」を私有ヘルパ `compile_and_validate(name, base_time)` へ統合（`facade.rs:198`）。検査順序・エラー値・`compile_storyboard` への base_time 引数（start_internal は 0.0、calculate_end_time は start_time）は機械的に保存。D1a-T が両経路を同一ケースで固定した回帰検知器（`InvalidLoopCount`×2 / `ZeroDurationWithLoop` / `TooShortDuration` / `StoryboardNotFound` 等 9 件）の保護下で実施。`start_internal` のステップ番号コメントを統合後の構成に追従（1–9 → 1–7）。
2. **`playback.rs`: 陳腐化 TODO 除去** — 1 行目の `// TODO: Implement PlaybackState, ScheduleRequest` を削除（両型は実装済み、D1a-T 所見4の申し送り）。コード変更なし。

diff 合計（コード）: facade.rs 34 insertions / 41 deletions、playback.rs 0/1。公開 API シグネチャ変更なし、テストファイル変更なし（リネーム非発生のため機械的追従も不要）。

### 適用見送りと根拠

- 候補 3・4: proposals 参照（P6 / P7）。
- 候補 5〜9: 上表のとおり。いずれも「挙動非破壊で書き換え可能だが、S6（最小 diff・単一用途への抽象化禁止・壊れていないものを直さない）に照らし利得が薄い」または「冗長に見えて実は意味を持つ」と判定。
- 非推奨コード（R2.9/R2.10）: 境界内の `#[deprecated]` は `facade.rs` の `update()` 1 件のみで、ワークスペース内利用 5 ファイル（grep で実証）→ 利用ゼロでないため削除せず（候補 9）。`playback.rs` の旧型は deprecated 未指定で P5 記録済み。

### 検証（S2）

- BEFORE: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（18 スイート、1032 passed / 0 failed / 32 ignored）
- AFTER: `cargo build --workspace` 成功（警告 0）/ `cargo test --workspace` 全グリーン（BEFORE と同一の 18 スイート、1032 passed / 0 failed / 32 ignored）。既存テストの失敗 0、アサーション変更 0、テストコード変更 0

## flaky

なし（wintf cue_performance_test は BEFORE / AFTER とも初回グリーン。隔離再実行は不要だった）

## proposals

- P6（report/proposals.md へ追記）: facade `cancel()`/`finish()` の到達不能な is_terminal 防御分岐の整理（ロジック変更を要する簡素化）
- P7（report/proposals.md へ追記）: イージング適用ロジックの重複統合 loop_controller ↔ interpolator（セル境界をまたぐため本ループ見送り）
