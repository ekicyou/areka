# D1a-T: dola ランタイム中核 × テスト網羅性

- status: completed
- commit: test(D1a): ランタイム中核のテスト空白38件を補完（facade エラーパス・timeline 内部評価・購読境界）

## findings

### モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象 | 既存テスト | 追加 | 備考 |
|------------|------|-----------|------|------|
| `runtime/facade.rs` (446 LOC) | `start` 正常系/StoryboardNotFound/ドキュメント未読込/TooShortDuration | `tests/runtime/facade_test.rs`, `loop_integration_test.rs` | — | 既存で十分 |
| 〃 | `start` の `InvalidLoopCount`(0/-2)・`ZeroDurationWithLoop`・空SBの非ループ開始 | なし | 4件 | ドキュメントバリデーションは loop_count を検査しないため公開 API から到達可能（実証済み） |
| 〃 | `calculate_end_time` エラーパス（未読込/未定義SB/InvalidLoopCount/ZeroDuration/TooShort） | 正常系のみ | 5件 | `start` と重複実装のバリデーション分岐を独立に固定 |
| 〃 | `pause`/`resume`/`finish` の無効 group_id、`cancel`/`finish` の終了済みインスタンス | `finish` 正常系・`conclude`/`cancel` 無効IDのみ | 5件 | cancel_after_conclude / finish_after_conclude / finish 無効ID / pause・resume 無効ID |
| 〃 | `unsubscribe`/`unsubscribe_all` のファサード経由挙動・`Default` 実装 | なし（unit のみ） | 4件 | 購読解除→update 配信停止のエンドツーエンド、`Default`+`last_result` 初期値 |
| 〃 | `tick`/`last_result`/`update`(deprecated) 等価性 | `tests/cue/tick_last_result_test.rs` | — | 既存で十分 |
| 〃 | `process_triggers`（発火・1周回1回・fire-and-forget・親競合除外） | `tests/trigger/runtime_test.rs` | — | trigger ドメインの既存テストが網羅 |
| `runtime/loop_controller.rs` (536 LOC) | `should_continue_loop`/`advance_loop`/`process_loops`/`generate_delay`/`apply_easing` | in-source 25件（決定的 RNG・分布検定含む） | 1件 | 唯一の空白だった `ParametricEasing::QuadraticBezier` 分岐を追加 |
| `runtime/timeline_manager/` (306 LOC) | `insert_entries`/`evaluate` 基本/期限切れ破棄/最新gid優先/pause凍結/time_scale | `timeline_manager/tests.rs` 8件 | — | 既存で十分 |
| 〃 | `evaluate` エッジ（インスタンス欠損破棄・delay未到達=from_value・即時遷移=to_value永続・複数セグメント進行） | なし | 4件 | `evaluate_segments` の全分岐を経由 |
| 〃 | `calculate_effective_time`（Playing 基本式・Paused 凍結・pause_start=None フォールバック） | 間接のみ | 3件 | pub(crate) 純関数の直接検証 |
| 〃 | `evaluate_all_for_group` / `collect_current_segment_final_values`（Cancel/Trim/Conclude 戦略用） | conflict_resolution_test.rs の間接カバーのみ | 7件 | unit 空白を解消（全変数収集・他group除外・インスタンス欠損・delay中空・全終了後=最終セグメント） |
| 〃 | `collect_final_values` 未知 group / `get_timeline` | なし | 2件 | |
| `runtime/subscription_manager/` (163 LOC) | subscribe 冪等/採番/unsubscribe/差分検出/凍結値/force_update/ptr_eq | `subscription_manager/tests.rs` 17件 | — | 既存で十分 |
| 〃 | 購読解除済み ID/名前の境界（force_update 無視・convert 除外・diff 無視） | なし | 3件 | |
| `playback.rs` (24 LOC) | `PlaybackState`/`ScheduleRequest` serde ラウンドトリップ | `tests/general/core_types_test.rs`（全5バリアント+構造体） | — | 既存で十分 |
| `runtime/types.rs`（参考・境界外） | InstanceState 遷移/EvaluatedValue/RuntimeError | `tests/runtime/core_types_test.rs` | — | D1b 領域、変更なし |

追加テスト合計 38 件（統合 18 件: `tests/runtime/facade_test.rs` / in-source 20 件: `timeline_manager/tests.rs` 16, `subscription_manager/tests.rs` 3, `loop_controller.rs` 1）。配置は S9 準拠（統合テストはドメインサブディレクトリ `tests/runtime/`、ユニットテストは Separated 方式 `{module}/tests.rs` または Inline `mod tests`、既存パターンを踏襲）。

### 除外テスト

0 件。重複候補として精査した `tests/runtime/core_types_test.rs` と `tests/general/core_types_test.rs` は対象型が異なり（ランタイム型 vs データモデル型）重複ではない。`facade_test.rs` の `invalid_document_preserves_existing` は表明が弱いが空ドキュメント受理という実挙動を固定しており死テストではないため温存。

### テスト不能箇所・深掘り所見

1. **`update_internal` Step 2 の `rand::rng()`（スレッド RNG）** — loop_offset のランダム遅延はファサード経由では非決定的。既存・追加テストとも `min == max`（固定遅延）または `generate_delay` 単体の SeedableRng で決定性を確保しており、ランダム経路の統計的性質は `loop_controller.rs` の分布検定（mean 検定 2 件）が担う。ファサードへの RNG 注入はシグネチャ変更（テスト容易性リファクタの範囲超）のため見送り。
2. **`cancel()`/`finish()` の `is_terminal()` 防御分岐は実質到達不能** — terminal 遷移（Concluded/Cancelled 等）で `instance_manager` がインスタンスを自動削除するため、終了済み group_id への操作は常に手前の `get()` で `InvalidGroupId` になる。追加テスト（cancel_after_conclude_fails / finish_after_conclude_fails）は到達可能な外側挙動を固定した。分岐自体の除去はロジック変更を伴う簡素化として D1a-S の判断材料（除去せずとも挙動同一のため低優先）。
3. **`start_internal` と `calculate_end_time` のバリデーション重複** — loop_count/duration 検査が二重実装されており、片側だけ変更されると乖離する。今回両者を同一ケースでテスト固定したため、将来の共通化リファクタ（S 観点）は回帰検知器つきで実施可能。
4. **`playback.rs` 冒頭の `// TODO: Implement PlaybackState, ScheduleRequest` は陳腐化**（両型は実装済み）。コメント除去は S 観点の構造的整理として D1a-S へ申し送り。型自体の整理は P5 として提案記録。

### 検証（S2）

- BEFORE: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（994 passed / 0 failed / 32 ignored）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（1032 passed / 0 failed / 32 ignored、+38 はすべて追加分。既存スイートはベースラインと同一結果）
- RED フェーズ: 既存挙動の特性化テストのため N/A（欠落の証跡はベースラインのテスト名一覧と上表の対応関係）
- wintf `cue_performance` フレーキーは本実行では発生せず（全スイート初回グリーン）

## proposals

- P5（report/proposals.md へ追記）: dola::playback 旧型（PlaybackState / ScheduleRequest）の整理（非推奨化または削除）
