# D1b-S: dola 補間・状態 × シンプル化

- status: completed
- commit: refactor(D1b): 競合解決の終了戦略4経路を共通ヘルパへ統合・dead code と陳腐化コメントを削除

## findings

### S6（karpathy-guidelines）基準で検証した候補一覧

境界: `crates/dola/src/runtime/{conflict_resolver,document_store,types,clock,instance_state}.rs`, `runtime/{interpolator,instance_manager}/`, `src/{storyboard,transition,easing,value,variable}.rs`（D1b-T 由来 36 件を含む既存テストが回帰検知器）:

| # | 候補 | S6 根拠 | 判定 |
|---|------|---------|------|
| 1 | `conflict_resolver.rs`: `resolve_conflicts`（非除外ラッパ、`#[allow(dead_code)]`）の削除（D1b-T 申し送り） | ワークスペース全体 grep で呼び出しゼロ（テスト含む）を実証。`pub(crate)` のため公開 API 非該当。私有 dead code の削除 | **適用** |
| 2 | `conflict_resolver.rs`: `apply_cancel`/`apply_conclude`/`apply_trim`/`apply_compress` の4関数が「値収集→購読者伝播→終了遷移→エントリ除去」の同一4ステップを重複実装（Cancel と Trim は終了状態以外完全同一）。さらに dispatch の match が `InstanceState::from_policy` と同じ policy→終了状態対応を再実装 | 自明な重複（差分は「収集する値」と「終了状態」の2点のみ）。戦略ごとの値収集 match + 共通ヘルパ `terminate_instance` に統合し、終了状態は既存 `from_policy` を再利用。呼び出し順序・エラー処理は機械的に保存 | **適用** |
| 3 | `conflict_resolver.rs`: `detect_overlaps` の未使用引数 `_start_time` | 私有関数の未使用パラメータ（P11 で wall-clock 対応時の活用が提案済みだが、現実装では純粋なノイズ。P11 採用時に再導入すればよい） | **適用** |
| 4 | `instance_manager/mod.rs`: `transition`/`pause`/`set_pause_start`/`resume`/`set_finish_deadline` が `self.instances.get_mut(&gid).ok_or(InvalidGroupId)` を5回重複実装（既存メソッド `get_mut` と同一） | 既存ヘルパの再利用（5箇所 → `self.get_mut(group_id)?`）。エラー値・順序は同一 | **適用** |
| 5 | `instance_manager/mod.rs`: `pause` の本体が `transition(gid, Paused)` と完全等価（Paused は非 terminal のため自動削除分岐も不発で一致）。内部に陳腐化した設計メモコメント4行 | 重複実装を委譲に置換 + 陳腐化コメント除去。エラーマッピング（存在しない/不正遷移とも InvalidGroupId）は同一。`pause_invalid_group_id` / `pause_created_state_fails` 等が回帰検知器 | **適用** |
| 6 | `storyboard.rs`/`transition.rs`/`easing.rs`/`value.rs`/`variable.rs` 先頭の `// TODO: Implement ...`（全型実装済み） | 陳腐化コメント（D1a-S の playback.rs と同型の申し送りパターン）。モジュール doc コメントに置換 | **適用** |
| 7 | `interpolator/mod.rs`: `interpolate` の doc が存在しない `intern_pool` 引数に言及（`interpolate_with_pool` の記述が混入） | doc の正確性修正（該当行を `interpolate_with_pool` 側へ移設）。コード変更なし | **適用** |
| 8 | `document_store.rs`: `get_storyboard`（`#[allow(dead_code)]`、利用は自ファイル内テスト3件のみ） | 削除は既存テストの削除を要し「テスト弱体化禁止」と R2.9（deprecated かつ利用ゼロ）に抵触 | 見送り→ **P13 提案記録** |
| 9 | `interpolator/mod.rs`: `apply_named_easing`/`apply_parametric_easing` と loop_controller（D1a 境界）の重複 | P7 で記録済みのセル境界をまたぐ統合。指示どおり本セルでも実施せず、現状維持を確認 | 見送り（P7 既存） |
| 10 | `clock.rs`: `now()` が毎回 `QueryPerformanceFrequency` を再取得 | 簡素化ではなく最適化（OnceLock キャッシュ等の複雑性追加）。壊れていないものを直さない（S6 #3） | 見送り |
| 11 | `conflict_resolver.rs`: Never 事前チェックループの `.any()` 化 | 現行 for ループで十分可読。書き換え利得僅少（churn） | 見送り |
| 12 | `types.rs` / `instance_state.rs` / `interpolator::ObjectInternPool` | 重複・過剰抽象なし。簡素化候補なし | 候補なし |

### 適用した簡素化と根拠

1. **`conflict_resolver.rs`: dead code 削除 + 終了戦略4経路の統合**（52 insertions / 167 deletions）
   - `resolve_conflicts`（非除外ラッパ）を削除。grep で定義以外の出現ゼロ・削除後の `cargo build --workspace` 成功で利用ゼロを実証。`#[deprecated]` ではないが私有（pub(crate)）かつ完全未使用の dead code として削除（公開 API 変更なし）。ラッパの doc にあった Never→Err の挙動説明は `resolve_conflicts_excluding` の doc へ移設。
   - `apply_cancel`/`apply_conclude`/`apply_trim`/`apply_compress`（計約 100 行）を「戦略別の値収集 match（Cancel|Trim → `evaluate_all_for_group`、Conclude → `collect_current_segment_final_values`、Compress → `collect_final_values`）+ 共通後処理 `terminate_instance`（伝播→終了遷移→エントリ除去）」へ統合。終了状態の決定は match の再実装をやめ既存 `InstanceState::from_policy` を再利用（Never は直前の match で `unreachable!` 済みのため `expect`）。各ステップの呼び出し順序・引数・`let _ =` のエラー無視は機械的に保存。なお P12（trigger_store 残置リーク）の修正対象だった「4経路」は本統合で単一箇所になり、将来の修正がより小さくなる。
   - `detect_overlaps` の未使用引数 `_start_time` を除去（呼び出し側1箇所を機械的に追従）。
2. **`instance_manager/mod.rs`: 取得パターンの共通化 + `pause` の委譲化**（6 insertions / 35 deletions）— 5箇所の `self.instances.get_mut(&gid).ok_or(...)` を既存 `get_mut` の再利用に置換。`pause` は `transition(gid, Paused)` への委譲1行に置換し、内部の陳腐化コメント（「pause_start はfacade側の…」の設計メモ）を doc コメント1行に集約。D1b-T の `pause_invalid_group_id` / `resume_without_pause_start_is_noop` / 状態遷移テスト群が回帰検知器。
3. **陳腐化 TODO コメント除去（5ファイル）** — `storyboard.rs`/`transition.rs`/`easing.rs`/`value.rs`/`variable.rs` 先頭の `// TODO: Implement ...` をモジュール doc コメント（`//!`）に置換。全型は実装済み（D1a-S の playback.rs:1 と同一パターン）。
4. **`interpolator/mod.rs`: doc 正確性修正** — `interpolate` の doc から存在しない `intern_pool` 引数への言及を除去し、`interpolate_with_pool` の doc へ移設。コード変更なし。

diff 合計: 8 ファイル、65 insertions / 209 deletions（net -144 行）。公開 API シグネチャ変更なし（変更した可視性はすべて pub(crate) 以下）、テストファイル変更なし（リネーム非発生のため機械的追従も不要）、アサーション変更 0。

### 適用見送りと根拠

- 候補 8: P13 として proposals.md に記録（テスト専用 dead code、削除はテスト弱体化禁止と R2.9 に抵触）。
- 候補 9: P7 既存提案のとおりセル境界をまたぐため見送り（指示どおり確認のみ）。
- 候補 10・11: 上表のとおり S6（最小 diff・壊れていないものを直さない）に照らし利得僅少。
- 非推奨コード（R2.9/R2.10）: 境界内に `#[deprecated]` 指定の項目なし。dead code は候補 1（削除済み）と候補 8（P13）の2件のみ。

### 検証（S2）

- BEFORE: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（exit 0、全 18 スイート 0 failed、1073 passed / 32 ignored — D1b-T ベースライン一致）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 全グリーン（18 スイート、1073 passed / 0 failed / 32 ignored、BEFORE と同一件数）。既存テストの失敗 0、テストコード変更 0
- 補足: `cargo clippy -p dola --all-targets` に既存エラー4件（`approx_constant`: `tests/general/core_types_test.rs:167`、`tests/runtime/core_types_test.rs:265,266,305` の `3.14` リテラル、D1b-T 追加テスト由来）が本セル変更前から存在する。本セルの変更ファイルとは無関係のため未修正（テスト修正は境界外の判断となるため申し送りのみ）。

## flaky

なし（wintf cue_performance_test は BEFORE / AFTER とも初回グリーン。隔離再実行は不要だった）

## proposals

- P13（report/proposals.md へ追記）: document_store `get_storyboard` の整理 — テスト専用 dead code の非推奨化または削除（テスト削除を伴うため本ループでは見送り）
