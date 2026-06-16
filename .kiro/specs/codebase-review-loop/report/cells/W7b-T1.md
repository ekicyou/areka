# W7b-T1: ECS基盤 共通インフラ（ecs/common/） × テスト網羅性

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W7b-T1（領域 W7b「ECS基盤・World」の**事前分割サブセル1/2** × 観点 T「テスト網羅性」）。担当は **`ecs/common/` のみ**。`ecs/world/`・`ecs/app.rs` は 18.2 W7b-T2 の担当ゆえ一切触れていない。
- 性質: 既存挙動の特性化テスト追加（挙動変更なし。R2.1, R5.1）。`ecs/common/` のジェネリック階層伝播ロジックのモジュール×テスト対応表をゼロから作成した。
- requirements: 1.3（大領域の細分化 = T セル事前分割の根拠）, 2.1, 2.5, 2.7, 2.8, 4.1, 5.1
- design: プロジェクト・プロファイル S2/S9（テスト命名・配置規約 = structure.md 命名規約）、レビュー観点列 T、CellExecutor 観点別規則（T）、W7b 領域定義（ECS基盤・World）と T セル事前分割、セル断片様式、提案記録様式
- 参考: `report/cells/W6b-T.md`（World ベース・in-source `mod tests` パターン）・`W7a-T1.md`（直前の事前分割 T セルの様式）・`feedback_loop_convergence_test.rs`（既存の Schedule ベース伝播テスト）

## 対象ファイル一覧（W7b-T1 = `crates/wintf/src/ecs/common/`）

- `mod.rs`（re-export + モジュール doc のみ、85 LOC。テスト対象ロジックなし）
- `tree_iter.rs`（`DepthFirstReversePostOrder`: `new`/`next`/`collect`。深さ優先・逆順・後順走査イテレータ。改善前 239 → 改善後 329 LOC）
- `tree_system.rs`（**ジェネリック階層伝播の中核**、370 LOC。`sync_simple_transforms<L,G,M>` / `mark_dirty_trees<L,G,M>` / `propagate_parent_transforms<L,G,M>` / `propagation_worker`（private） / `propagate_descendants_unchecked`（private unsafe） / `NodeQuery` 型エイリアス / `WorkQueue`（Default/send_batches/send_batches_with））

合計 約 784 LOC（うち本番ロジックは tree_iter 239 + tree_system 370）。境界 = `ecs/common/` のみ。**`ecs/world/`・`ecs/app.rs` は未参照**。

## モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象（主要 関数/型） | デバイス依存 | 既存テスト所在・件数 | 追加 | 所見 |
|------------|------|------|-----------|------|------|
| `mod.rs` | re-export + doc のみ | なし | — | 0件 | テスト対象ロジックなし |
| `tree_iter.rs` | `DepthFirstReversePostOrder`（`new`/`next`/`collect`。Children 配列の最後＝最前面から DFS 逆後順走査） | **なし（純粋な bevy World 走査、デバイス非依存）** | in-source `mod tests` **4件**（基本6ノード順序・単一ノード・深い4階層・幅広4兄弟。いずれも `collect` の最終順序のみ固定） | **4件** | 空白: `next` の逐次契約（1件ずつ取得）、走査完了後の `None` 終端（空スタック `pop()?` の安定性・多重呼び出し）、`next` 逐次列と `collect` の等価性、混在兄弟（子あり/子なしの interleaving 順序）、`Children` コンポーネント非保持ノードの葉終端（`world.get::<Children>`=None 枝）が未固定だった |
| `tree_system.rs` → `sync_simple_transforms<L,G,M>` | ルート（`Without<ChildOf>` かつ `Without<Children>`）の `G = L.into()` 更新（p0: `Changed<L>`/`Added<G>`、p1: `RemovedComponents<ChildOf>` 孤立経路） | **なし（bevy World + ParamSet、デバイス非依存）**。`par_iter_mut` は ComputeTaskPool 利用だが結果決定的 | **なし（0件・直接特性化なし）**。`feedback_loop_convergence_test` は本関数を**ラッパ `sync_simple_arrangements` 経由で incidental に駆動**するが、検証対象は WindowPos→Arrangement 同期であり本関数の root-sync 契約は未固定 | **4件**（新規統合 `tree_propagation_test.rs`） | **空白（直接 0件）**: ルートの `G` を `From` 代数で設定（scale=1・scale 付き両方）、`Without<Children>` で子持ちエンティティを除外、`ChildOf` 除去後の孤立エンティティ再同期（p1 経路）を特性化 |
| `tree_system.rs` → `mark_dirty_trees<L,G,M>` | 変更エンティティ（`Changed<L>`/`Changed<ChildOf>`/`Added<G>`）+ 孤立から **祖先方向**へ `M` のダーティビットを `set_changed` 伝播。`is_changed && !is_added` でサブツリー処理済み枝刈り | **なし（bevy World、デバイス非依存）** | **なし（0件・直接特性化なし）** | **2件**（同上） | **空白（直接 0件）**: 深いリーフの `Arrangement` 変更が root の `ArrangementTreeChanged` を `Changed` にする（祖先伝播）、`ChildOf` 変更（再ペアレント）で新親の祖先チェーンが Changed になる。検出システム（`Ref<M>::is_changed`）で観測 |
| `tree_system.rs` → `propagate_parent_transforms<L,G,M>` | ルート（`&Children` 必須・`Changed<M>`）から子へ BFS で `G = parent_G * child_L`。`propagation_worker`/`propagate_descendants_unchecked` 経由。`set_if_neq`+`is_changed` で静的サブツリースキップ。`assert_eq!(child_of.parent(), parent)` で非循環保証 | **bevy World 部分はデバイス非依存**（小規模階層は単一ワーカーのインライン経路で完結し決定的）。**マルチスレッド fan-out（`ComputeTaskPool::scope` で追加ワーカー spawn）は大規模階層のみ到達するパフォーマンス経路**（結果は同一。所見1） | **なし（0件・直接特性化なし）**。`feedback_loop_convergence_test::test_feedback_loop_converges_dpi_192` が唯一 2階層を伝播するが child の `Arrangement.offset` のみ検証し、伝播された `GlobalArrangement.bounds` は未検証 | **8件**（同上） | **最大の空白（直接 0件）**: 2階層の親スケール積算、3階層チェーン累積、幅広ツリーの全子処理、変更なし再実行の冪等性（`set_if_neq` 無更新）、片ブランチ変更時の兄弟不変、再ペアレントでの新親累積変換による再計算、子なしルートの全パイプライン処理（sync_simple 委譲）、深い4階層チェーンの DFS 連続反復 |
| `tree_system.rs` → `WorkQueue` | `Default`（mpsc channel + Parallel）、`send_batches`/`send_batches_with`（CHUNK_SIZE=512 でチャンク送信・clear）。`CHUNK_SIZE` 定数 | **なし（純粋データ構造）だが private**（`send_batches*` は `fn`、フィールドも private） | なし | 0件 | `send_batches*` は private 関数で、`propagate_parent_transforms` 経由でのみ駆動される。8件の伝播テストが間接的にバッチ送信経路を実行している（小規模のため単一チャンク）。public API からの直接テストは不能（所見2） |
| `tree_system.rs` → `NodeQuery` | 型エイリアス | — | — | 0件 | 型定義のみ。`propagate_parent_transforms` のシグネチャで使用され、伝播テストで間接的にインスタンス化される |

追加テスト合計 **18件**（tree_iter in-source **4件** + tree_system の伝播特性化 統合 **14件**）。**プロダクションコードの変更なし**（R5.1 充足。git diff: tree_iter.rs in-source 追加 `#[test]`=4・削除0、すべて `#[cfg(test)]` 内。新規統合ファイル `tests/ecs/tree_propagation_test.rs`=14件。`tree_system.rs`/`mod.rs` は無変更）。

## 追加したテスト一覧（ファイル・テスト名・狙い）

**`crates/wintf/src/ecs/common/tree_iter.rs`（in-source `mod tests` 既存4件へ追記, +4件）**
- `test_next_returns_none_after_exhaustion` — 単一ノードで `next`→Some→None、完了後の追加呼び出しも None（空スタック安定終端・多重呼び出し安全）
- `test_next_step_by_step_matches_collect` — `next` 逐次取得列が `collect`（whileループ）結果と完全一致（gc→child→root）
- `test_mixed_siblings_with_and_without_children` — 子あり/子なし混在兄弟の interleaving（最前面のサブツリー先消化 → 子なし兄弟 → 親）: `[b1, b, a, root]`
- `test_node_without_children_component_is_leaf` — `Children` 非保持ノード（`get::<Children>`=None 枝）が葉として即返却・以降 None

**`crates/wintf/tests/ecs/tree_propagation_test.rs`（新規, 14件）**
ジェネリック伝播 3 関数を具体型 `(Arrangement, GlobalArrangement, ArrangementTreeChanged)` でインスタンス化し bevy `World`+`Schedule` で駆動（本番 `arrangement_systems.rs` と同順序の `chain()`）。期待値は `From`/`Mul`（`arrangement.rs`）代数から独立導出。
- `sync_simple_updates_orphan_root_from_local` — ルートの G を `From`（scale=1）で設定（transform M31/M32・bounds）
- `sync_simple_applies_scale_to_root_global` — scale 付きルートの G（M11/M22/M31=offset*scale・bounds スケール変換）
- `sync_simple_ignores_entity_with_children` — `Without<Children>` で子持ちエンティティを対象外（G 既定のまま）
- `sync_simple_resyncs_orphaned_entity_via_removed_components` — `ChildOf` 除去後の孤立エンティティ再同期（p1 経路 `iter_many_mut(orphaned.read())`）
- `mark_dirty_propagates_change_from_leaf_to_root` — 深いリーフの `Arrangement` 変更が root の `ArrangementTreeChanged` を Changed にする（祖先伝播。`Ref<M>::is_changed` 検出）
- `mark_dirty_reacts_to_childof_change` — `ChildOf` 変更（再ペアレント）で新親 root_b の M が Changed（`Changed<ChildOf>` 経路）
- `propagate_two_level_applies_parent_scale` — 親 scale=2 が子 bounds に積算（scaled_offset=offset*2・size*2）+ ルート自身の G 更新
- `propagate_three_level_chain_accumulates` — root2x→child→grandchild の累積変換（`hierarchical_bounds_test` 代数値 34/66/64/90 と一致）
- `propagate_wide_tree_processes_all_children` — ルートの複数子がそれぞれ独立に正しい global（BFS 全子処理）
- `propagate_is_idempotent_when_nothing_changes` — 収束後の変更なし再実行で global 不変（`set_if_neq`+`Changed<M>` ゲートのスキップ）
- `propagate_single_branch_change_preserves_sibling` — 片ブランチのみ変更で兄弟ブランチの global 不変
- `propagate_recomputes_child_after_reparent` — 子を root_a(scale2)→root_b(scale3) へ再ペアレントで新親累積変換により再計算（left 20→30・right 40→60）
- `propagate_full_pipeline_handles_childless_root` — 子なしルートは roots クエリ（`&Children` 必須）に非該当だが sync_simple 経路で G 設定（3 システム協調）
- `propagate_deep_chain_propagates_all_levels` — 4階層チェーンを 1 ルート伝播で全消化（`propagate_descendants_unchecked` の DFS 連続反復・bounds.left 累積 10/20/30/40）

## 除外したテスト
なし。`tree_iter.rs` の既存4件は走査順序を異なる形状（基本/単一/深い/幅広）で固定しており重複・死テストではない（追加4件は順序ではなく `next` 逐次契約・終端・混在 interleaving・None 枝という別観点）。`tree_system.rs` には既存 in-source テストが存在せず（除外対象自体なし）、統合側の `feedback_loop_convergence_test`（10件）は WindowPos→Arrangement 同期と収束を検証する別観点で、本セルの伝播契約特性化とは非重複（触れていない）。過不足整理の結論: **不足のみ存在（18件で充足）、過剰なし**。

**重複の意図的回避**: `feedback_loop_convergence_test::test_feedback_loop_converges_dpi_192` が 2階層（LayoutRoot→Window）で `propagate_global_arrangements` を駆動するが、検証は child の `Arrangement.offset`（=WindowPos 由来値）のみで、伝播された `GlobalArrangement.bounds` は未検証だった。本セルは伝播後の `GlobalArrangement` を直接アサートし、かつ 3階層以上・幅広・冪等性・兄弟独立・再ペアレント・孤立という伝播固有の挙動を新規に固定した（既存テストとアサーション対象が異なる）。

## テスト不能箇所・深掘り所見（R2.8）

1. **`propagate_parent_transforms` のマルチスレッド fan-out 経路（`ComputeTaskPool::scope` での追加ワーカー spawn）は大規模階層のみ到達するパフォーマンス経路** — 本関数は (a) ルートを `par_iter_mut().for_each_init` で並列初期化し各ローカルアウトボックスへ子をプッシュ、(b) `queue.send_batches()` でバッチ送信、(c) キューに作業があれば `task_pool.scope` で `(1..thread_num())` 個のワーカーを spawn しつつ自スレッドでも `propagation_worker` を実行、という構成。小規模階層（本セルのテスト規模）では最初のワーカー（ローカル実行分）が単一の `propagate_descendants_unchecked` 呼び出し（max_depth=1 のルート処理 + キュー経由の max_depth=10000 の子孫処理）で全作業を消化し、追加ワーカーが起動する前に `busy_threads==0` で終了する。`busy_threads` セマフォによる協調終了・`CHUNK_SIZE`(512) 超過時のトラバース中チャンク送信・複数ワーカー間のタスク奪い合いは、512 ノード超の幅または極端に多いルート数を持つ階層でのみ実走する。これは**パフォーマンス分割の経路でありアルゴリズムの正当性は単一/複数スレッドで不変**（bevy 上流 `propagate_parent_transforms` の設計どおり、結果決定的）。本セルでは伝播結果の正当性（累積変換・冪等性・サブツリースキップ・再ペアレント）を小規模階層で全面固定した。並列分割そのものの実走は実規模シーン（実起動 S7・実 UI 階層）が回帰検知器であり、ユニットでの数百ノード生成は決定的結果を変えないため提案化しない（scale 制約）。

2. **`WorkQueue::send_batches`/`send_batches_with` および `propagation_worker`/`propagate_descendants_unchecked` は private で public API からの直接テスト不能** — これらは `pub` でなく（`send_batches*` は `fn`、`propagation_worker`/`propagate_descendants_unchecked` は module-private `fn`、`WorkQueue` のフィールドも private）、唯一の駆動口は `pub fn propagate_parent_transforms`。本セルの 8 件の伝播テストはこの public 関数経由でバッチ送信・ワーカーループ・DFS 反復を間接的に実行している（小規模のため単一 CHUNK・単一ワーカー経路）。private 関数の単体直接呼び出しは crate 外統合テストからは不能だが、public 関数の挙動が決定的に固定されているため別途の API 露出は不要（R2.8 の「保護外領域」だが public 経由で網羅済み・提案化不要）。

3. **`propagate_descendants_unchecked` の `assert_eq!(child_of.parent(), parent)`（非循環保証アサート）は public API 経由では発火不能** — このアサートは unsafe な disjoint 並列ミューテーションの健全性を保証する不変条件（子の `ChildOf.parent()` が走査中の親と一致＝双方向整合・非循環）を表明する。bevy の `ChildOf`/`Children` は `add_children`/`insert(ChildOf)` 経由で双方向整合が維持されるため、public API のみを使う限りこのアサートは常に成立し、パニックを誘発するには `Children` と `ChildOf` を不整合にする unsafe な直接操作が必要となる。本セルの再ペアレントテスト（`propagate_recomputes_child_after_reparent`/`mark_dirty_reacts_to_childof_change`）は整合した再ペアレントを駆動しこのアサートを成立側で通過させている。不整合誘発はテスト境界外（安全な公開操作で到達不能）かつ意図的な防御であり提案化しない。

## proposals へ回した候補
なし（新規採番なし）。`ecs/common/` のジェネリック伝播ロジックは **挙動変更を要する欠陥・脆弱性・削除候補を検出せず**、デバイス非依存に高くテスト可能であった（伝播結果の正当性・冪等性・サブツリースキップ・再ペアレント・孤立・ダーティ伝播を全面特性化、初回 GREEN・バグ検出ゼロ）。所見1〜3 はいずれも環境/scale 制約または意図的防御で、新規仕様化を要しない。proposals.md 末尾は **P65**（変更なし。次セルの新規採番は P66 から）。

## verification (S2)

- BEFORE: 親のベースライン（**1628 passed / 0 failed**・クリーンワークツリー）を信頼して流用（design のフェーズ0 ベースライン規定 + 親指示「BEFORE S2 は省略可」に従う）。触れる対象（wintf lib の `ecs/common/` in-source = 既存4件のみ、`tests/ecs` バイナリ = 既存79件）の BEFORE は改善前 grep / 既存テスト構成から確認済み（`tree_system.rs` は in-source テスト 0件）。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1646 passed / 0 failed**（32 ignored）。全テストバイナリで failed=0、全 `test result:` 行を awk で合算した実測（`passed=1646 failed=0 ignored=32`）。`test result: FAILED` 行ゼロ・`error[`/`panicked` 行ゼロ。
  - グローバル合計は 1628 → 1646（**+18**）。
  - 触れたファイルの新規 `#[test]` 件数内訳（git diff 実数と完全一致。`git diff --unified=0 -- crates/wintf/src/ecs/common/tree_iter.rs | grep -c "^+.*#\[test\]"`=4・削除0。新規 `tests/ecs/tree_propagation_test.rs` の `#[test]`=14）:
    - `tree_iter.rs`（in-source）: **4 → 8（+4）**
    - `tree_system.rs`: **0 → 0（変更なし。伝播は統合テストで特性化）**
    - `tests/ecs/tree_propagation_test.rs`（新規統合）: **+14**
    - 合計 **+18**（4+14）
  - 反復検証: `cargo test -p wintf --lib common::` で common in-source **8 passed / 0 failed**（既存4 + 追加4）。`cargo test -p wintf --test ecs tree_propagation_test` で **14 passed / 0 failed**。`tests/ecs` バイナリ全体は **93 passed / 0 failed**（既存79 + 追加14）。
  - 全18件が初回実行で合格（特性化テスト = GREEN by construction。後述 RED 代替を参照）。深掘りを要する初回失敗なし（バグ・前提誤りの検出なし）。

## flaky
- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（`tests/ecs` バイナリ内・W7b-T1 の追加対象外）: `cargo test --workspace` の全量実行で `... ok`（隔離再実行不要）。本セルの追加テストとは無関係。

## clippy（S3・記録のみ・非ブロッカー）
- `cargo clippy -p wintf --tests` は既存警告群（合計 173 warning 系。`com/d2d/command_sink.rs` の `not_unsafe_ptr_arg_deref` ほか既存ファイル由来）を出力。
  - **本セルで追加/変更したファイル（`src/ecs/common/tree_iter.rs` の `mod tests` 追記分・新規 `tests/ecs/tree_propagation_test.rs`）を指す診断はゼロ**（パスフィルタ `tree_iter\.rs`/`tree_propagation_test\.rs`/`tree_system`/`common` で grep し該当なしを確認）。
  - 本セルはテスト追加のみでプロダクションコード未変更のため、新規 clippy 警告/error の導入はゼロ。S3 規定によりブロッカーとせず記録に留める（簡素化は S 観点 W7b-S の担当）。

## RED フェーズ代替の検証
追加18件はすべて既存挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に各ソース仕様/代数から導出した:
- **tree_iter**: `next` の `stack.pop()?`→`expanded` 返却 / 未展開なら自身を `(e,true)` 再プッシュ＋`get::<Children>` の子を順方向プッシュ（tree_iter.rs:79-101）のアルゴリズムから、None 終端・逐次列・Children 非保持葉・混在兄弟順序を導出。
- **sync_simple_transforms**: p0 の `*global = (*transform).into()`（tree_system.rs:35-40）と p1 の `iter_many_mut(orphaned.read())`＋`!is_changed && !is_added` ガード（:42-48）、`Without<ChildOf>`/`Without<Children>` フィルタ（:23-26）から、root-sync・子持ち除外・孤立再同期を導出。
- **mark_dirty_trees**: `changed_transforms.iter().chain(orphaned.read())` の各起点から `while transforms.get_mut(next)` で `set_changed`＋`child_of.map(parent)` 上昇（:63-77）、`Changed<ChildOf>` を起点に含む（:55）ことから、祖先伝播・再ペアレント反応を導出。
- **propagate_parent_transforms**: roots の `*parent_transform = (*transform).into()`（:100）＋`propagate_descendants_unchecked` の `global_transform.set_if_neq(a*b)`（:282）と `!tree.is_changed() && !p_global.is_changed()` スキップ（:272-275）、`for depth in 1..=max_depth` の last_child 上書き反復（:257-309）から、累積変換・冪等性・サブツリースキップ・兄弟独立・深いチェーン消化を導出。期待 bounds は `From<Arrangement>`（translation*scale）/`Mul<Arrangement>`（scaled_offset=offset*parent_scale、`arrangement.rs:186-234`）の代数で手計算し、`hierarchical_bounds_test` の既知値（34/66/64/90 等）と相互検証。

初回実行で18件全件が導出どおり一致し、バグ・前提誤りは検出されなかった（深掘りを要する初回失敗なし）。
