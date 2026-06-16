# Brief: oversized-file-refactor

## Problem
ワークスペース全域（crates/wintf, crates/dola, crates/areka）に巨大化したソースファイルが多数存在し、可読性・保守性・レビュー容易性を損なっている。最大は `win_message_handler.rs` の1399行、生きた製品コードでも `ecs/drag/state.rs` が1034行に達する。また `#[deprecated]` の札が貼られたまま実際には参照ゼロの「死んだコード」が散在し、コードベースのノイズになっている。これらを保守する開発者（=本プロジェクトのメンテナ）が、肥大ファイルの全体把握と差分レビューに過大なコストを払っている。

## Current State
- 全 `.rs` ファイル303個のうち、600行を超えるものが24個（src 11、tests/examples 13）。
- 最大の `win_message_handler.rs`(1399) / `win_thread_mgr.rs`(370) / `winproc.rs`(181) は `#[deprecated]` 指定だが**実際には生きている**: `process_singleton.rs`(製品コード)が `winproc` を、`app.rs`/`world/mod.rs`/`vsync.rs` が `win_thread_mgr` のグローバルatomicを、12個のexamplesが3モジュール全てを参照中。今このまま削除するとビルドが壊れる。
- 一方で「真の死体」も存在: `ecs/pointer` 配下の `mouse_*` deprecatedエイリアス（types.rs 5個・systems.rs 3個・mod.rs の `mouse` モジュール）は再export以外に実参照ゼロ。`examples/taffy_flex_demo_old.rs` は `_old` サフィックスの旧実装で参照ゼロ。`ecs/layout/metrics.rs:65` の `opacity` deprecated staticはテスト値のみ。
- 直前に完了した `codebase-review-loop` 仕様は「全域レビューの手順」を定めたもので、本仕様の「600行ポリシーの具体適用」とは別物（相補的）。

## Desired Outcome
- 対象（src + tests、examplesは除く、deprecated 3ファイルは除く）の生きたソースファイルが、目安600行に収まる。明確に超過するものは責務境界で分割されている。
- 「真の死体」コードが削除され、コードベースから不要なノイズが消えている。
- 全工程を通じて**挙動非破壊**: `cargo test` がグリーンを維持し、公開APIの後方互換が保たれる（直近コミット「挙動非破壊の品質改善」のパターンを踏襲）。

## Approach
**挙動非破壊のモジュール抽出（責務境界での分割）+ 死体削除**。
1. **死体削除フェーズ**: 参照ゼロを確認済みの deprecated 項目（mouse_* エイリアス10個、taffy_flex_demo_old.rs、opacity static）を削除。`dola::runtime::facade::update()` は20+テストで現役のため**削除しない**（後方互換維持）。
2. **分割フェーズ**: 各肥大ファイルを責務の seam（型定義／システム／ヘルパー／テスト など）で複数モジュールへ抽出。ディレクトリモジュール化（`module/mod.rs` + サブファイル）や in-source テストの `tests.rs` 分離（structure.md の既存パターン）を活用。`pub use` で公開APIを据え置き、呼び出し側を無改変に保つ。
3. 各ファイル分割後に該当crateの `cargo test` でグリーン確認。crate単位（wintf → dola → areka）でウェーブ実行。

なぜこの方式か: 既存の命名・テスト規約（structure.md）に沿い、機械的・低リスクで、レビュー単位（1ファイル=1タスク相当）が明確。挙動を一切変えないため回帰リスクが最小。

## Scope
- **In**:
  - 生きた src ファイルの600行超過分の分割（約10ファイル）
  - 生きた tests ファイルの600行超過分の分割（約12ファイル）
  - 「真の死体」コードの削除（mouse_* エイリアス、taffy_flex_demo_old.rs、opacity static）
  - 分割に伴う `mod`/`pub use` 再構成、テストのグリーン維持
- **Out**:
  - examples の600行超過分の分割（手動検証用のため対象外）
  - deprecated 3ファイル（win_message_handler.rs / win_thread_mgr.rs / winproc.rs）の分割・移行・削除 → 別仕様へ延期。600行ルールの**例外**扱い
  - 機能追加・挙動変更・パフォーマンス最適化（純粋な構造リファクタのみ）
  - `dola::facade::update()` の削除（20+テストで現役、後方互換維持）

## Boundary Candidates
- **死体削除** vs **ファイル分割**（独立して着手・レビュー可能な2フェーズ）
- crate境界: wintf / dola / areka（各々独立してビルド・テスト可能）
- src分割 vs tests分割（テストは挙動への影響がさらに小さく、独立ウェーブ化しやすい）
- in-source `#[cfg(test)] mod tests` の `tests.rs` 分離 vs 製品コード本体の分割（別パターン）

## Out of Boundary
- deprecated 3ファイルの新API（`ecs/window_proc/`）への移行と最終削除（規模大、examples改修を伴う → 将来の独立仕様）
- 600行ルールを enforce する CI/lint の導入（自動化はスコープ外、必要なら別途）
- 公開APIの再設計やリネーム（後方互換を壊す変更）

## Upstream / Downstream
- **Upstream**: steering の `structure.md`（命名・テスト分割規約）、`tech.md`（thiserror/bevy_ecs等の規約）。直近の `codebase-review-loop` 仕様（全域レビュー手順）。
- **Downstream**: 将来の「deprecated旧API退役（win_message_handler等の移行・削除）」仕様。クリーンになった分割後モジュールは以後の機能追加の土台になる。

## Existing Spec Touchpoints
- **Extends**: なし（新規の独立した保守タスク）
- **Adjacent**: `codebase-review-loop`（レビュー手順を提供する相補的仕様。本仕様の分割成果はそのレビュー対象になる）

## Constraints
- **言語/規約**: Rust 2024 Edition。`snake_case.rs`、in-sourceテストは `mod tests` または `{module}/tests.rs` 分離（structure.md準拠）。
- **挙動非破壊が絶対条件**: `cargo test` グリーン維持、公開API後方互換厳守。
- **600は目安**: 上限ではなく目標。明確な超過を責務境界で分割し、無理な機械分割で凝集度を壊さない。
- **対象外の厳守**: examples と deprecated 3ファイルには触れない（ビルド破壊リスク回避）。
- **Windows専用ビルド**: `cargo test` はWindows環境（DirectComposition対応）で実行する必要がある。

## Reference: 確定済みファイルリスト（仕様生成時の入力データ）

### 削除対象（真の死体・参照ゼロ確認済み）
- `crates/wintf/src/ecs/pointer/types.rs` — `MouseButton`(L77), `MouseState`(L156), `MouseLeave`(L174), `WindowMouseTracking`(L189), `MouseBuffer`(L280) の各deprecatedエイリアス
- `crates/wintf/src/ecs/pointer/systems.rs` — `clear_transient_mouse_state`(L35), `debug_mouse_state_changes`(L102), `debug_mouse_leave`(L125)
- `crates/wintf/src/ecs/mod.rs` — `mouse` モジュール（L18-22）と関連 `#[allow(deprecated)]` 再export（L44-48）
- `crates/wintf/examples/taffy_flex_demo_old.rs` — 旧実装example（参照ゼロ）
- `crates/wintf/src/ecs/layout/metrics.rs:65` — `opacity` deprecated static
- ※ `dola::runtime::facade::update()`(facade.rs:327) は20+テストで現役のため**削除しない**

### 分割対象（src・600行超・生きている）
| ファイル | 行数 |
|---|---|
| `crates/wintf/src/ecs/drag/state.rs` | 1034 |
| `crates/areka/src/main.rs` | 857 |
| `crates/wintf/src/ecs/graphics/compositor_systems/render.rs` | 850 |
| `crates/wintf/src/ecs/cue/queue.rs` | 781 |
| `crates/wintf/src/ecs/layout/hit_region/tests.rs` | 734 (in-source test) |
| `crates/wintf/src/ecs/window/window_pos.rs` | 720 |
| `crates/wintf/src/ecs/pointer/types.rs` | 712 (mouse_*削除で減少見込み) |
| `crates/wintf/src/ecs/layout/hit_test/tests_ex.rs` | 686 (in-source test) |
| `crates/dola/src/runtime/loop_controller.rs` | 627 |
| `crates/wintf/src/ecs/widget/text/typewriter.rs` | 602 |

### 分割対象（tests・600行超）
| ファイル | 行数 |
|---|---|
| `crates/dola/tests/runtime/conflict_resolution_test.rs` | 1116 |
| `crates/dola/tests/compile/time_resolution_test.rs` | 934 |
| `crates/dola/tests/runtime/facade_test.rs` | 894 |
| `crates/wintf/tests/layout/taffy_advanced_test.rs` | 780 |
| `crates/dola/tests/runtime/loop_offset_test.rs` | 769 |
| `crates/wintf/tests/layout/boxstyle_coordinate_separation_test.rs` | 747 |
| `crates/dola/tests/general/integration_test.rs` | 711 |
| `crates/dola/tests/validation/transition_test.rs` | 705 |
| `crates/wintf/tests/layout/taffy_layout_integration_test.rs` | 671 |
| `crates/dola/tests/general/core_types_test.rs` | 662 |
| `crates/wintf/tests/layout/arrangement_bounds_test.rs` | 614 |
| `crates/dola/tests/compile/integration_test.rs` | 609 |

### 対象外（明示）
- examples 全般の分割（`dcomp_demo.rs` 672 / `multi_backend_demo.rs` 597 / `taffy_flex_demo/setup.rs` 585 等）
- deprecated 3ファイル: `win_message_handler.rs`(1399) / `win_thread_mgr.rs`(370) / `winproc.rs`(181)
