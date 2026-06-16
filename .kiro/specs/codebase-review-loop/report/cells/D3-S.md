# D3-S: dola 検証・Cue × シンプル化

- status: completed
- commit: refactor(D3): validate/cue/document の挙動非破壊な簡素化4件を適用・陳腐化 TODO を削除

## findings

### S6（karpathy-guidelines）基準で検証した候補一覧

境界: `crates/dola/src/validate/`（mod.rs / rules.rs）, `crates/dola/src/cue/`（command.rs / schedule.rs / sheet.rs / mod.rs）, `crates/dola/src/{document,lib}.rs`（D3-T 由来 34 件を含む既存テスト — バリデーションルール形状・cue 配信順特性化 — が回帰検知器）:

| # | 候補 | S6 根拠 | 判定 |
|---|------|---------|------|
| 1 | `validate/mod.rs` — トリガー検証 4 ブロックが各々 `entry.trigger_storyboard` を個別に判定（V16t は `is_some()` + ネスト if、V17t は本体が空の `if` 文＝死コード、V14t/V18t は同一 `if let` の重複） | 同一条件の 4 重判定と空 if 文。単一の `if let Some(ref trigger_target)` へ統合し、V17t は空 if を除去して設計根拠コメントのみ温存。エラー push 順（V16t → V14t → V18t）は逐語的に保存 | **適用** |
| 2 | `validate/rules.rs::validate_transition_type_constraints` — Object 型の from/relative_to/easing 禁止チェックがフィールド名のみ異なる 3 重コピー（V10） | 自明な重複。`(field, is_some)` 配列のループへ統合。エラー構築・push 順（from → relative_to → easing）は同一 | **適用** |
| 3 | `validate/rules.rs::validate_transition_type_constraints` — 数値型の from/to 値域検査（V12）がフィールド名のみ異なる 2 重コピー | 自明な重複。`(field, value)` 配列のループへ統合。push 順（from → to）・エラー内容は同一 | **適用** |
| 4 | `cue/schedule.rs::tick()` — Barrier 処理の「Timeout バリアの即時解除チェック」専用ブロックが直後の汎用タイムアウトチェックと完全冗長（Timeout は `timeout_dur = Some(duration)` のため `offset >= barrier_offset + duration` の同一比較・同一 continue を 2 度実行） | 到達後の挙動が汎用経路と厳密に一致する冗長分岐（同一被演算子の f64 加算 → ビット同一の比較）。専用ブロックを除去しコメントを実態に更新。`timeout_barrier_auto_releases` / `timeout_barrier_skipped_when_already_past` が回帰検知器 | **適用** |
| 5 | `document.rs` 冒頭の `// TODO: Implement DolaDocument`（実装済み、D3-T 申し送り） | 陳腐化コメント。モジュール doc コメント（`//!`）へ置換（A1/D1a/D1b/D2-S と同一パターン） | **適用** |
| 6 | `validate/rules.rs::validate_variable_ranges` — Float/Integer 分岐の initial 値域検査が約 25 行 × 2 の構造的重複 | f64 統一ヘルパへの統合は Integer 側の比較を i64 → f64 の損失変換（\|v\| > 2^53 で丸め）に変える観測可能な挙動変更。挙動保存のままの統合はジェネリクス + 変換クロージャを要し 2 箇所のための抽象として複雑さが純増 | 見送り（**P24** 提案記録） |
| 7 | `validate/rules.rs::dfs_detect_cycle` — 再帰 DFS の反復化 | 構造的ロジック変更。循環報告のメンバー・パス順序（D3-T の循環形状テストで特性化済み）の同一性証明を要するため指示どおり不変 | 見送り（**P23** 提案記録） |
| 8 | `cue/schedule.rs` — `insert()`/`extend()` の同時刻配信順不整合（FIFO/LIFO） | P22 既存提案・特性化テストで固定済み。指示どおり不変 | 見送り（P22 既存） |
| 9 | V13 の Dynamic 型不一致チェック 2 箇所（from/to でメッセージ文字列のみ相違） | ループ化には `format!` によるメッセージ組み立てへの変更が必要で、テストで固定された exact-string リテラルの greppability を損なう。利得僅少（S6 #3） | 見送り |
| 10 | `cue/sheet.rs::actors()` の `Vec::contains` 線形走査、`next_routing()` の `remove(0)` | 簡素化ではなく効率改善（データ構造変更）であり本観点の対象外。台本規模では実害なし | 見送り |
| 11 | `cue/command.rs`（ドメイン型）、`cue/mod.rs`、`lib.rs` | 型定義・再エクスポートのみで重複・過剰抽象なし。公開 API シグネチャは一切不変 | 候補なし |

### 適用した簡素化と根拠

1. **`validate/mod.rs`: トリガー検証の単一 `if let` への統合と空 if 文（V17t）の除去**（候補1）— 4 回の同一 Option 判定を 1 回に集約。V17t の「追加チェック不要」という設計根拠コメントは統合ブロック内に温存。エラー追加順は V16t → V14t → V18t のまま逐語的に保存（`tests/trigger/validation_test.rs` の排他・自己参照・対象不在テストが回帰検知器）。
2. **`validate/rules.rs`: V10 Object 禁止フィールド検査の 3 重コピーをループ化**（候補2）— `tests/validation/transition_test.rs` の field 別 `ObjectTransitionViolation` テスト 3 件が回帰検知器。
3. **`validate/rules.rs`: V12 from/to 値域検査の 2 重コピーをループ化**（候補3）— `tests/validation/transition_test.rs` の `ValueOutOfRange`（variable/field 検証）テストが回帰検知器。
4. **`cue/schedule.rs`: tick() の冗長な Timeout 専用即時解除ブロックを除去**（候補4）— Timeout 種別は `timeout_dur` 経由の汎用チェックが同一比較（同一被演算子の加算 → 同一 f64 値）・同一動作（continue / barrier_timeout_offset 設定 / current_barrier 設定）を行うため厳密に等価。
5. **`document.rs`: 陳腐化 TODO をモジュール doc へ置換**（候補5、D3-T 申し送り）。

diff 合計: 4 ファイル、35 insertions / 64 deletions（net -29 行）。公開 API シグネチャ変更なし（変更箇所はすべて trait 実装内部・pub(super) ヘルパ・`TimedSchedule::tick` の私有ロジック）、バリデーションエラーのメッセージ・フィールド値・発生順は不変、テストのアサーション変更 0・テストファイル変更 0。

### 適用見送りと根拠

- 候補 6・7: ロジック変更を要する簡素化として P24・P23 へ提案記録。
- 候補 8: P22 既存提案・特性化済みのため指示どおり不変。
- 候補 9・10: S6 #3（壊れていないものを直さない・churn 回避）に照らし利得僅少または観点外。
- 非推奨コード（R2.9/R2.10）: 境界内に `#[deprecated]` 指定・`#[allow(dead_code)]` なし。
- 補足: `cargo clippy -p dola --lib` の警告は 26 → 21 件（collapsible_if 5 件が統合により自然解消）。新規警告 0。`cargo fmt --check` の差分はすべて既存（本セル変更行に fmt 差分なし、既存差分への波及修正は churn 回避のため未実施）。

### 検証（S2）

- BEFORE: HEAD de2a7ab で `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（18 スイート、1160 passed / 0 failed / 32 ignored）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace --no-fail-fast` で 1159 passed / 1 failed / 32 ignored — 唯一の failed は既知フレーキー（境界外、下記）で、隔離再実行で安定合格。境界内（dola 全スイート）は 0 failed で BEFORE と同一件数

## flaky

- AFTER の `cargo test --workspace --no-fail-fast` で wintf tests/ecs スイートが 78 passed / 1 failed（既知の `cue_performance_test::bench_pop_ready_empty_queue`、境界外）。プロトコルに従い隔離再実行: 1 回目（cue_performance フィルタ実行）で再失敗、2 回目（単独実行）で ok を確認し、既知フレーキーのパススルーと判定（性能閾値系テストの特性と整合）。

## proposals

- P23（report/proposals.md へ追記）: `dfs_detect_cycle` の再帰 DFS 反復化 — 循環報告順序の同一性証明を要する構造的ロジック変更のため記録のみ
- P24（report/proposals.md へ追記）: `validate_variable_ranges` の Float/Integer 重複統合 — i64 → f64 損失変換（2^53 超の極値で検出有無が変わる）の設計判断を要するため記録のみ
