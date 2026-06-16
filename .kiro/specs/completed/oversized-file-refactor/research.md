# Gap Analysis: oversized-file-refactor

分析日: 2026-06-16 / 対象: 要件 R1〜R6 / 種別: brownfield 構造リファクタリング（挙動非破壊）

## 1. 分析サマリ

- 本仕様は新機能ゼロの純構造リファクタ。ギャップは「機能の欠如」ではなく **各肥大ファイルの分割 seam の特定** と **死体削除の安全性** に集約される。実コード解剖により全対象の分割案・難易度・可視性リスクを確定済み。
- **死体削除（R1）はすべて参照ゼロを再確認**。grep 実利用検索でヒットゼロ。削除は低リスク（in-file 合計 約-60行 ＋ `taffy_flex_demo_old.rs` 1ファイル削除）。**注**: `pointer/types.rs` は死体削除でエイリアス10行（5箇所×2行）が消えても 712→約702行で 600行を超過したままのため、R2 の分割対象を維持する（死体削除では分割不要にはならない）。`dola::facade::update()` は現役のため除外を厳守。
- **分割機構は既存パターンで全カバー可能**。製品コードは `{module}/mod.rs + サブファイル` 方式、in-source テストはディレクトリモジュール化、統合テストは `tests/{domain}.rs` 入口での `#[path] mod` 追加方式。新規アーキテクチャ不要。
- **最大の技術リスクは可視性（visibility）**。分割で `pub(crate)`/private 項目がモジュール跨ぎになる箇所が複数。`pub use` での公開API据え置きと `pub(super)`/`pub(crate)` の適切な付与が後方互換維持の鍵。
- **未確定（Research Needed）**: tests #9 `taffy_layout_integration_test.rs` / #11 `arrangement_bounds_test.rs` / #12 `compile/integration_test.rs` の3ファイルはテスト群の内部構造が未精査。設計フェーズで分割軸を確定する。

## 2. Current State Investigation（既存パターン）

### モジュール宣言パターン
- 製品コードはディレクトリモジュール化が標準。例: `drag/mod.rs` が `state, accumulator, capture_guard, context, dispatch, systems` を `mod`、`window/mod.rs` が `window_pos` を `pub use` で再export。
- 分割で公開シンボルを動かしても、親 mod.rs の `pub use xxx::*` を据え置けば**呼び出し側は無改変**（R2.3 / R4.1 を満たす）。

### in-source テストの分離パターン
- 現状の肥大 in-source テストは単一ファイル mod 宣言: `hit_region/mod.rs:512 #[cfg(test)] mod tests;`、`hit_test/mod.rs:558 #[cfg(test)] mod tests_ex;`。
- 複数ファイルへ割るには **ディレクトリモジュール化**へ変換: `tests/` ディレクトリ + `tests/mod.rs`（または `#[path]` 宣言）でサブテストファイルを束ねる。`use super::*` 依存は `use super::super::*` 等に調整が必要。

### 統合テストの分割機構（確定）
1. 入口ファイル `tests/{domain}.rs` は `#[path="..."]` による `mod` 宣言のみの束ね役。
2. 実テストは `tests/{domain}/{subname}_test.rs`。
3. 共有ヘルパーは `tests/{domain}/common/mod.rs`（`compile/`, `validation/` 等に既存）。
4. **分割手順**: 大ファイルを論理グループ別サブファイルに分割 → 入口ファイルに `#[path] mod` 宣言を追加するだけ。命名は `taffy_` 等サブドメインプレフィックス維持規約（structure.md）に従う。

## 3. Requirement-to-Asset Map（分割案・確定分）

### R1: 死体削除（全件 参照ゼロ確認済み / 種別: 低リスク削除）

| 対象 | ファイル:行 | 削除量 | 安全性 |
|---|---|---|---|
| `MouseButton`/`MouseState`/`MouseLeave`/`WindowMouseTracking`/`MouseBuffer` | `ecs/pointer/types.rs` 77,156,174,189,280 | 5行 | 実利用ゼロ ✓ |
| `clear_transient_mouse_state`/`debug_mouse_state_changes`/`debug_mouse_leave` | `ecs/pointer/systems.rs` 36,103,126 | 16行 | 委譲のみ・実利用ゼロ ✓ |
| `mouse` モジュール + `#[allow(deprecated)]` 再export | `ecs/mod.rs` 19-22, 44-48 | 9行 | 再export容器のみ ✓ |
| `Opacity` deprecated static | `ecs/layout/metrics.rs` 65-92 | 21行 | Visual.opacity に移行済み ✓ |
| `taffy_flex_demo_old.rs`（example） | 全体 | ファイル削除 | `_old` 旧実装・参照ゼロ ✓ |
| **除外** `facade::update()` | `dola/runtime/facade.rs:327` | 削除しない | 20+テストで現役（R1.6）|

> 注: `mouse` 再export を消す際は `ecs/mod.rs` の `pub use` も同時削除（R1.3）。削除直前に再 grep し、もし参照が現れたら削除せず報告（R1.7）。

### R2: src 分割（10ファイル / 製品コードは `{module}/mod.rs + サブファイル`方式）

| ファイル(行) | 分割案（推定行数） | 可視性リスク | 難度 |
|---|---|---|---|
| `ecs/drag/state.rs` (1034) | state_types(250) / state_store(50) / state_transitions(300) / state_check(40) / 既存testsは留置 | `DragStateSnapshot` を typewriter 層が参照→`pub use`必須 | M |
| `ecs/graphics/compositor_systems/render.rs` (850) | guards(130) / context(90) / traverse(150) / composite(260) / present(70) | `ClipGuard`/`DcTargetGuard` を `pub(super)` 化、再帰Query走査は同居 | M |
| `ecs/cue/queue.rs` (781) | queue_types(40) / queue_component(100) / queue_schedule(100) / queue_execution(200) / queue_control(90) | `BarrierResponse`/`CueQueueState` は既に `pub use`、`TimedSchedule` は read-only accessor | M |
| `ecs/window/window_pos.rs` (720) | zorder(50) / window_pos_data(120) / window_pos_builder(180) / window_pos_flags(80) / window_pos_coords(80) / window_parent_command(30) | `WindowPos` は `pub(crate)` Component、builder は Self返しで単純集約 | **S** |
| `ecs/pointer/types.rs` (712→約702 死体削除後も要分割) | physical_point / input_types(50) / state_types(80) / components(120) / buffers(140) / hit_test(30) | `PointerState`/`PointerButton`/`PointerBuffer` は `pub use` 据え置き | **S** |
| `ecs/widget/text/typewriter.rs` (602) | typewriter_def(60) / typewriter_hooks(40) / typewriter_state(200) / typewriter_control(100) / layout_cache(100) | on_add hook 自動挿入で def→state 初期化依存、COM Send/Sync は windows-rs付与済み | M |
| `ecs/layout/hit_region/tests.rs` (734, in-source test) | tests_polygon(200) / tests_builder(150) / tests_color_map(250) / tests_integration(134) | `mod tests;`→ディレクトリモジュール化、`use super::*` 調整 | M |
| `ecs/layout/hit_test/tests_ex.rs` (686, in-source test) | tests_entity_ex(200) / tests_tree_ex(150) / tests_bounds_alpha(180) / tests_integration(156) | 同上（`mod tests_ex;`→ディレクトリ化）| M |
| `dola/runtime/loop_controller.rs` (627) | easing(100) / advance(100) / processor(100) / loop_controller(150 再export) | 52テストを各モジュール `mod tests` へ再配置 | M |
| `areka/src/main.rs` (857, binary) | setup(150) / ui_builder(250) / event_handlers(100) / main(150 エントリ+定数+Marker) | 現在全fn/Marker private→pub昇格時に単体テスト境界要検討 | M |

### R3: tests 分割（12ファイル / `tests/{domain}.rs` 入口 + `#[path] mod` 追加方式）

| ファイル(行) | 分割案（ファイル数 × 推定） | 難度 |
|---|---|---|
| `dola/tests/runtime/conflict_resolution_test.rs` (1116) | conflict_detection / termination_strategy / error_handling（3×~400） | M |
| `dola/tests/compile/time_resolution_test.rs` (934) | sequential / parallel_concurrent / relative_resolution / complex_scenarios（4×~225） | M |
| `dola/tests/runtime/facade_test.rs` (894) | load_start / update_flow / termination / diff_delivery（4×~225） | M |
| `wintf/tests/layout/taffy_advanced_test.rs` (780) | taffy_computation / layout_conversion / hierarchy_sync / incremental（4×~200） | L |
| `dola/tests/runtime/loop_offset_test.rs` (769) | loop_offset_serde / _validation / _compile（3×~240） | M |
| `wintf/tests/layout/boxstyle_coordinate_separation_test.rs` (747) | boxstyle_inset / changed_timing / drag_lifecycle / window_sync（4×~185） | M |
| `dola/tests/general/integration_test.rs` (711) | serialization_round_trip / e2e_full_flow / domain_integration（3×~235） | M |
| `dola/tests/validation/transition_test.rs` (705) | transition_v7_v10 / _v11_v12 / _v13_nan（3×~235） | **S** |
| `dola/tests/general/core_types_test.rs` (662) | document_variable / dynamic_value / easing_transition / storyboard_playback（4×~210） | **S** |
| `wintf/tests/layout/taffy_layout_integration_test.rs` (671) | **Research Needed**（テスト群未精査） | TBD |
| `wintf/tests/layout/arrangement_bounds_test.rs` (614) | **Research Needed**（テスト群未精査） | TBD |
| `dola/tests/compile/integration_test.rs` (609) | **Research Needed**（テスト群未精査） | TBD |

共有ヘルパー: `compile/common/mod.rs`（`make_doc_with_storyboard`）継続。新規候補 `runtime/common/mod.rs`（facade/conflict 向けドキュメント生成）、`layout/common/mod.rs`（taffy_ 向け）。

## 4. Implementation Approach Options

### Option A: 機械的レイヤー分割（型→データ→ロジック→システム→テスト）＋ pub use 据え置き 【推奨】
- 各ファイルを責務レイヤーで分け、親 mod.rs の `pub use` で公開APIを完全据え置き。呼び出し側ゼロ改変。
- **✅** 低リスク・機械的・1ファイル=1レビュー単位（R6.3）に自然対応。既存パターンに完全準拠。**❌** モジュール数が増える／レイヤー境界が薄い小ファイルが出る可能性。

### Option B: 意味的再配置（項目を本来あるべき既存隣接モジュールへ移動）
- 例: `pointer/buffers.rs` が既存するなら types.rs のバッファ群をそちらへ統合。
- **✅** モジュール構成がより自然に。**❌** 移動先の選定に判断が要り機械性が落ちる、差分が広がりレビュー単位が曖昧化、後方互換のための `pub use` 経路が複雑化。

### Option C: ハイブリッド（大半は Option A、自然な移動先がある箇所のみ B）
- **✅** バランス良。**❌** 一貫性確保のためのルール定義が設計フェーズに必要。

**推奨: Option A を基線**とし、明確に自然な移動先がある少数ケース（例: 既存 `pointer/buffers.rs` への統合）でのみ Option B を局所適用。これで機械性・低リスク・レビュー単位の明確さ（R6）を最大化しつつ、無理な機械分割（R2.4 凝集度維持）を避ける。

## 5. Effort & Risk

| 単位 | Effort | Risk | 根拠 |
|---|---|---|---|
| R1 死体削除（wintf一括） | S | Low | 全件参照ゼロ確認済み、合計-51行＋example削除 |
| R2 wintf src 分割（8ファイル） | L | Medium | 可視性調整が中心、量が多い。drag/render/typewriter が M |
| R2 dola/areka src 分割（2ファイル） | M | Medium | main.rs の private→pub 昇格判断、loop_controller のテスト再配置 |
| R3 tests 分割（12ファイル） | L | Low〜Medium | 機構は確定・挙動非破壊だが量が多い。taffy_advanced が L、3件は要追加調査 |
| 全体 | XL（2+週間相当） | Medium | 22分割+削除。クレートウェーブ（R6.2）で局所化すればリスク低減 |

## 6. 設計フェーズへの申し送り（Recommendations / Research Needed）

- **採用方針**: Option A（機械的レイヤー分割 + `pub use` 据え置き）を基線に設計する。
- **可視性ポリシーの明文化**: 分割で跨ぐ項目に `pub(crate)`/`pub(super)` を付与するルール、公開シンボルの `pub use` 集約点を design.md で定義する（R4.1 後方互換の中核）。
- **in-source テストのディレクトリモジュール化手順**を design に具体化（`mod tests;` → `tests/` ディレクトリ + サブモジュール、`use super::*` の階層調整）。
- **Research Needed（設計フェーズで精査）**:
  1. tests #9 `taffy_layout_integration_test.rs` (671) のテスト群と分割軸
  2. tests #11 `arrangement_bounds_test.rs` (614) のテスト群と分割軸
  3. tests #12 `compile/integration_test.rs` (609) のテスト群と分割軸（既存 `compile/common` との関係）
  4. `areka/main.rs` 分割時、main エントリ/イベントハンドラの可視性をどこまで pub に昇格するか（テスト境界との整合）
- **検証**: 各ファイル分割・各クレートウェーブごとに Windows 環境で `cargo test` グリーン確認（R4.3/R4.5/R6.4）。死体削除は削除直前 grep 再確認（R1.7）。
