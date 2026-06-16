# D2-S: dola コンパイル・DSL × シンプル化

- status: completed
- commit: refactor(D2): compile の死コード除去・重複ロジック共通化・陳腐化 TODO 削除

## findings

### S6（karpathy-guidelines）基準で検証した候補一覧

境界: `crates/dola/src/compile/`（mod.rs / resolve.rs / types.rs）, `crates/dola/src/{builder,error}.rs`（D2-T 由来 28 件を含む既存テスト — exact-string の Display テスト・コンパイル時刻解決テスト — が回帰検知器）:

| # | 候補 | S6 根拠 | 判定 |
|---|------|---------|------|
| 1 | `resolve.rs::topological_sort` の死コード（in_degree の2重計算と全面上書き、`+= 0; // ensure entry exists` の no-op 文）＋ push のたびに `queue.sort_by` で全体再ソート（D2-T 申し送り3） | 明白な死コード。キーは常に `0..entry_count` のため `in_degree`/`reverse_deps` を Vec 化し、「最小 index 優先」キューを `BinaryHeap<Reverse<usize>>`（min-heap、標準の正攻法）へ置換。取り出し順（最小 index 先行）は同一 | **適用** |
| 2 | `resolve.rs::find_previous_entry_in_sort_order` — 名前に反して `_sorted_indices` 未使用、実体は「元配列 index - 1」（D2-T 申し送り4） | 誤解を招く名前の単純関数。`entry_idx.checked_sub(1)` ＋意図コメントへ置換して関数を削除。連動して `resolve_pure_keyframe_time` の未使用 `sorted_indices` パラメータも除去（pub(super) 私有、mod.rs 呼び出し2箇所を機械的追従） | **適用** |
| 3 | `resolve.rs::resolve_keyframe_ref_time` — 「全KF解決済みなら最遅時刻」ロジックが3箇所重複（Multiple / WithOffset-Single / WithOffset-Multiple のうち2箇所が逐語的コピー） | 自明な重複。私有ヘルパ `latest_keyframe_time` へ抽出（空リスト→None・1つでも未解決→None の挙動を逐語的に保存） | **適用** |
| 4 | `resolve.rs::build_dependency_graph` — 未使用パラメータ `_storyboard_name`/`_errors`、between の from/to で同一5行の重複、暗黙KF名 `format!("__implicit_{}", idx)` の生成式が mod.rs と二重定義 | 未使用パラメータ除去（連動して mod.rs の到達不能な `if !errors.is_empty()` チェック — errors は直前に生成され同関数は一切 push しない — も除去）。from/to を配列ループへ統合。暗黙KF名は `entry_keyframe_name` ヘルパへ一元化（知識の二重定義解消） | **適用** |
| 5 | `mod.rs` エントリループ — `is_trigger` bool 判定後に `entry.trigger_storyboard.clone().unwrap()` で再取得（panic 経路） | `if let Some(ref target_storyboard)` への書き換えで unwrap を構文的に排除。トリガー処理後に到達する純粋KF判定から冗長な `!is_trigger` 項を除去（continue 済みのため等価） | **適用** |
| 6 | `builder.rs`/`error.rs` 冒頭の `// TODO: Implement ...`（いずれも実装済み、D2-T 申し送り5） | 陳腐化コメント。モジュール doc コメント（`//!`）へ置換（D1a/D1b-S と同一パターン） | **適用** |
| 7 | `tests/compile/integration_test.rs` ローカル `make_doc` が `common/mod.rs::make_doc_with_storyboard` と逐語的に同一（D2-T 申し送り） | 重複ヘルパ削除。`use super::common::make_doc_with_storyboard as make_doc;` のエイリアス import に置換し、呼び出し7箇所・アサーションは無変更（最小機械的変更） | **適用** |
| 8 | P18 対象の防御分岐（`resolve_transition` の Named 未定義/transition 欠落、`var_def` 取得失敗 continue、Object 型 easing 強制 None、`build_variable_type_hint` の None→Float、`resolve_to_value` の非 Scalar スキップ） | 指示どおり不変（validate() 前提のクロスコンポーネント不変条件、P18 既存提案） | 見送り（P18 既存） |
| 9 | P17 対象（overlap エラーの entry_index 固定0・循環報告の過大包含・`__implicit_` 露出） | エラー内容＝外部観測可能挙動の変更。指示どおり不変 | 見送り（P17 既存） |
| 10 | `mod.rs` Step 5 の overlap 検査 `windows(2)` 化・`base_duration` の match 化 | 現行で十分可読、書き換え利得僅少（churn、S6 #3） | 見送り |
| 11 | `builder.rs` Builder API 本体・`error.rs` Display 実装・`types.rs` | 重複・過剰抽象なし（Display は exact-string テストで固定済みのため一切不変）。Builder の public API シグネチャは無変更 | 候補なし |

### 適用した簡素化と根拠

1. **`resolve.rs`: topological_sort の死コード除去と標準構造化**（候補1）— in_degree の2重計算（1回目は直後に全面上書きされる死コード）と no-op 文を削除し、`HashMap` ベースの in_degree/reverse_deps を `Vec` 化、ソート付き Vec キューを `BinaryHeap<Reverse<usize>>` へ置換。取り出し順序（同時 ready なら最小 index 先行）・循環検出（残 in_degree > 0 のエントリ列挙）は厳密に同一。`tests/compile/error_test.rs` の循環検出・`integration_test.rs` の複合配置・順序依存テストが回帰検知器。
2. **`resolve.rs`: 誤解を招く名前の関数と未使用パラメータの除去**（候補2）— `find_previous_entry_in_sort_order`（sorted_indices 未使用）を削除し `entry_idx.checked_sub(1)` ＋「配列直前エントリ＝元配列 index - 1」コメントへ置換。`resolve_pure_keyframe_time` から `sorted_indices` パラメータを除去（mod.rs の呼び出し2箇所を機械的追従）。D2-T 追加の純粋KF継承テスト2件が回帰検知器。
3. **`resolve.rs`: 最遅時刻ロジックの重複統合**（候補3）— 3箇所の逐語的コピーを私有 `latest_keyframe_time` へ統合。空リスト→None / 未解決1つ→None の境界挙動を保存（`?` 演算子による早期 return は元の `return None` と等価）。D2-T 追加の WithOffset{Multiple}・負オフセットテストが回帰検知器。
4. **`resolve.rs`/`mod.rs`: 依存グラフ構築の整理と暗黙KF名の一元化**(候補4) — `build_dependency_graph` の未使用2パラメータを除去し、mod.rs 側の到達不能なエラーチェック（errors は直前に空生成・同関数は push しない）を削除。between from/to の重複5行をループ化。`__implicit_{idx}` 生成式の二重定義を `entry_keyframe_name`（pub(super)）へ一元化。
5. **`mod.rs`: トリガー分岐の unwrap 排除**（候補5）— `is_trigger` bool ＋ `unwrap()` を `if let Some(ref target_storyboard)` へ置換し panic 経路を構文的に排除。純粋KF判定の冗長項を除去。`tests/trigger/compile_test.rs` 6件＋ D2-T 追加のトリガーKF登録テスト2件が回帰検知器。
6. **陳腐化 TODO 除去（2ファイル）**（候補6）— `builder.rs`/`error.rs` 冒頭の `// TODO: Implement ...` をモジュール doc コメントへ置換。
7. **テストヘルパ重複の解消**（候補7、D2-T 申し送り）— `integration_test.rs` の `make_doc`（25行）を共通ヘルパへのエイリアス import 1行に置換。呼び出し箇所・アサーションは無変更。

diff 合計: 5 ファイル、57 insertions / 133 deletions（net -76 行）。公開 API シグネチャ変更なし（変更した関数はすべて pub(super) 以下の私有）、エラー Display 文字列・CompiledStoryboard の内容・エラー発生条件は不変、テストのアサーション変更 0。

### 適用見送りと根拠

- 候補 8・9: P17/P18 既存提案のとおり指示に従い不変（エラー診断・防御分岐とも外部観測可能挙動またはクロスコンポーネント不変条件に関わる）。
- 候補 10: S6 #3（壊れていないものを直さない）に照らし利得僅少。
- 非推奨コード（R2.9/R2.10）: 境界内に `#[deprecated]` 指定・dead code（`#[allow(dead_code)]` 含む）なし。
- 検証中の深掘りで新規所見1件: 純粋KF/トリガー（at なし）の暗黙依存（配列直前エントリ）が依存グラフに反映されず、トポロジカル順によっては整形式文書がコンパイル失敗する（P19 として提案記録。本セルの簡素化は当該挙動を機械的に保存）。

### 検証（S2）

- BEFORE: HEAD cb9922c で `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（18 スイート、1115 passed / 0 failed / 32 ignored）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace --no-fail-fast` 全グリーン（18 スイート、1115 passed / 0 failed / 32 ignored、BEFORE と同一件数）。最終確認の再実行でも 1115 / 0 を再現
- 補足: `cargo clippy -p dola --all-targets` の既存指摘（テストの `approx_constant` 4件・`resolve_entry_timing` の引数数 8/7 等）は本セル変更前から存在し境界外または既存のため未修正。本セルが新規に持ち込んだ clippy 警告は 0（適用中に検出した collapsible_if 1件は continue ガードへ修正済み）

## flaky

- AFTER 初回の `cargo test --workspace` で wintf tests/ecs スイートが 78 passed / 1 failed（既知の `cue_performance_test::bench_pop_ready_empty_queue`、境界外）。プロトコルに従い `--no-fail-fast` 全体再実行（全グリーン 1115/0）＋該当テストの隔離実行（ok）で安定合格を確認し、既知フレーキーのパススルーと判定。

## proposals

- P19（report/proposals.md へ追記）: 純粋KF/トリガー（at なし）の暗黙依存（配列直前エントリ）が依存グラフに反映されない — 修正はエラー→成功の挙動変更を伴うため記録のみ
