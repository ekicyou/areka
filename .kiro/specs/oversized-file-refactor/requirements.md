# Requirements Document

## Introduction

本仕様は、wintf / dola / areka の各クレートに散在する肥大化ソースファイルを責務境界で分割し、参照ゼロが確認済みの「真の死体」コードを削除することで、コードベースの可読性・保守性・レビュー容易性を改善する。全工程を通じて挙動は一切変更せず、`cargo test` のグリーンと公開APIの後方互換を維持する純粋な構造リファクタリングである。対象は生きた src ファイルと tests ファイルであり、examples と deprecated 3ファイル（`win_message_handler.rs` / `win_thread_mgr.rs` / `winproc.rs`）は本仕様のスコープ外として明示的に除外する。「600行」は上限ではなく目安であり、明確な超過を凝集度を壊さない責務境界で分割することを目標とする。

このリファクタリングの受益者は本プロジェクトのメンテナ（開発者）であり、ここでの「観測可能な振る舞い」はビルド成功・テスト合格・公開API不変・ファイル規模縮小・死体コード消滅というメンテナ視点の成果として定義される。

## Boundary Context

- **In scope**:
  - 参照ゼロ確認済みの「真の死体」deprecated コードの削除（`ecs/pointer` の `mouse_*` エイリアス群、`taffy_flex_demo_old.rs`、`ecs/layout/metrics.rs` の `opacity` deprecated static）
  - 生きた src ファイルのうち600行を明確に超過するものの責務境界での分割（約10ファイル）
  - 生きた tests ファイルのうち600行を明確に超過するものの責務境界での分割（約12ファイル）
  - 分割に伴う `mod` 宣言・`pub use` 再エクスポートの再構成（呼び出し側を無改変に保つ）
  - クレート単位（wintf → dola → areka）での `cargo test` グリーン維持
- **Out of scope**:
  - examples の600行超過分の分割（手動検証用のため対象外）
  - deprecated 3ファイル（`win_message_handler.rs` / `win_thread_mgr.rs` / `winproc.rs`）の分割・新API移行・削除（別仕様へ延期、600行ルールの例外扱い）
  - `dola::runtime::facade::update()` の削除（20以上のテストで現役のため後方互換維持、削除しない）
  - 機能追加・挙動変更・パフォーマンス最適化・公開APIの再設計やリネーム
  - 600行ルールを enforce する CI/lint の導入
- **Adjacent expectations**:
  - steering の `structure.md`（命名規約・テスト分割規約）と `tech.md`（thiserror/bevy_ecs等の規約）に準拠する
  - 相補的な `codebase-review-loop` 仕様（全域レビュー手順）が提供するレビュー観点に、本仕様の分割成果が整合する
  - ビルド・テストは Windows 環境（DirectComposition対応）で実行されることを前提とする

## Requirements

### Requirement 1: 真の死体コードの削除

**Objective:** メンテナとして、参照ゼロが確認済みの deprecated コードを削除したい。それによりコードベースから不要なノイズが消え、把握すべきコード量が減るため。

#### Acceptance Criteria

1. When 死体削除フェーズを実行するとき, the リファクタリング作業 shall `crates/wintf/src/ecs/pointer/types.rs` の deprecated エイリアス（`MouseButton`, `MouseState`, `MouseLeave`, `WindowMouseTracking`, `MouseBuffer`）を削除する。
2. When 死体削除フェーズを実行するとき, the リファクタリング作業 shall `crates/wintf/src/ecs/pointer/systems.rs` の deprecated 関数（`clear_transient_mouse_state`, `debug_mouse_state_changes`, `debug_mouse_leave`）を削除する。
3. When 死体削除フェーズを実行するとき, the リファクタリング作業 shall `crates/wintf/src/ecs/mod.rs` の `mouse` モジュール宣言と関連する `#[allow(deprecated)]` 再エクスポートを削除する。
4. When 死体削除フェーズを実行するとき, the リファクタリング作業 shall `crates/wintf/examples/taffy_flex_demo_old.rs`（参照ゼロの旧実装example）を削除する。
5. When 死体削除フェーズを実行するとき, the リファクタリング作業 shall `crates/wintf/src/ecs/layout/metrics.rs` の `opacity` deprecated static を削除する。
6. The リファクタリング作業 shall `dola::runtime::facade::update()`（`facade.rs`）を削除せず現状のまま維持する（20以上のテストで現役のため後方互換を維持する）。
7. If 削除候補のいずれかが削除直前に実参照を持つと判明した場合, then the リファクタリング作業 shall 当該項目を削除せず、参照状況を報告対象として残す。
8. When 死体削除フェーズの削除完了後, the 対象クレート shall `cargo test` がグリーンであることを示す。

### Requirement 2: 生きた src ファイルの責務境界分割

**Objective:** メンテナとして、600行を明確に超過する生きた src ファイルを責務境界で複数モジュールへ分割したい。それにより各ファイルの全体把握と差分レビューのコストが下がるため。

#### Acceptance Criteria

1. When src 分割フェーズを実行するとき, the リファクタリング作業 shall 確定済みの対象ファイル（`ecs/drag/state.rs`, `areka/src/main.rs`, `ecs/graphics/compositor_systems/render.rs`, `ecs/cue/queue.rs`, `ecs/layout/hit_region/tests.rs`, `ecs/window/window_pos.rs`, `ecs/pointer/types.rs`, `ecs/layout/hit_test/tests_ex.rs`, `dola/src/runtime/loop_controller.rs`, `ecs/widget/text/typewriter.rs`）を分割対象とする。
2. When 1ファイルを分割するとき, the リファクタリング作業 shall 型定義・システム・ヘルパー・テストなどの責務 seam を境界として複数モジュールへ抽出する。
3. While 分割を行っている間, the リファクタリング作業 shall `pub use` により公開APIを据え置き、呼び出し側のコードを無改変に保つ。
4. The 分割後の各ファイル shall 目安600行に収まることを目標とし、凝集度を損なう無理な機械分割を行わない。
5. Where in-source の `#[cfg(test)] mod tests` を含むファイルを分割する場合, the リファクタリング作業 shall `structure.md` の既存パターン（`{module}/tests.rs` 分離またはディレクトリモジュール化）に従う。
6. When 各 src ファイルの分割完了後, the 対象クレート shall `cargo test` がグリーンであることを示す。
7. The リファクタリング作業 shall ファイル分割において機能追加・挙動変更・パフォーマンス最適化を一切行わない。

### Requirement 3: 生きた tests ファイルの責務境界分割

**Objective:** メンテナとして、600行を明確に超過する生きた tests ファイルを分割したい。それにより大規模テストファイルの把握とレビューが容易になるため。

#### Acceptance Criteria

1. When tests 分割フェーズを実行するとき, the リファクタリング作業 shall 確定済みの対象ファイル（`dola/tests/runtime/conflict_resolution_test.rs`, `dola/tests/compile/time_resolution_test.rs`, `dola/tests/runtime/facade_test.rs`, `wintf/tests/layout/taffy_advanced_test.rs`, `dola/tests/runtime/loop_offset_test.rs`, `wintf/tests/layout/boxstyle_coordinate_separation_test.rs`, `dola/tests/general/integration_test.rs`, `dola/tests/validation/transition_test.rs`, `wintf/tests/layout/taffy_layout_integration_test.rs`, `dola/tests/general/core_types_test.rs`, `wintf/tests/layout/arrangement_bounds_test.rs`, `dola/tests/compile/integration_test.rs`）を分割対象とする。
2. When tests ファイルを分割するとき, the リファクタリング作業 shall `structure.md` のテスト命名規約（`tests/{domain}.rs` 入口は束ね役、`#[path]` による `mod` 宣言、ドメインプレフィックス除去、`taffy_` 等サブドメインプレフィックスの維持）に従う。
3. While tests 分割を行っている間, the リファクタリング作業 shall 既存テストケースの内容・アサーションを変更せず、分割前と同一のテストが実行されることを保つ。
4. The 分割後の各 tests ファイル shall 目安600行に収まることを目標とする。
5. When 各 tests ファイルの分割完了後, the 対象クレート shall `cargo test` がグリーンであり、分割前と同一のテストケースが実行されることを示す。

### Requirement 4: 挙動非破壊と後方互換の維持

**Objective:** メンテナとして、全リファクタリング工程を通じて挙動が一切変わらず公開APIの後方互換が保たれることを保証したい。それにより回帰リスクを最小化し、安全にマージできるため。

#### Acceptance Criteria

1. The リファクタリング作業 shall 全工程を通じて公開APIのシグネチャ・可視性・パスを後方互換に保つ。
2. The リファクタリング作業 shall 死体削除と分割を通じてランタイムの観測可能な挙動を変更しない。
3. When 各クレート（wintf, dola, areka）の作業を完了したとき, the 対象クレート shall `cargo test` がグリーンであることを示す。
4. If リファクタリング途中で `cargo test` が失敗した場合, then the リファクタリング作業 shall グリーンに回復するまで当該変更を是正し、未解決の失敗を残したまま完了扱いとしない。
5. The リファクタリング作業 shall ビルド・テストの検証を Windows 環境（DirectComposition対応）で実行する。

### Requirement 5: スコープ境界の厳守

**Objective:** メンテナとして、対象外として確定したファイル群に一切手を触れないことを保証したい。それによりビルド破壊リスクを回避し、本仕様の作業範囲を明確に保つため。

#### Acceptance Criteria

1. The リファクタリング作業 shall examples の600行超過分（`dcomp_demo.rs`, `multi_backend_demo.rs`, `taffy_flex_demo/setup.rs` 等）を分割しない（ただし参照ゼロの死体example `taffy_flex_demo_old.rs` の削除は R1 で扱う唯一の例外であり、本基準が禁ずる「分割」には該当しない）。
2. The リファクタリング作業 shall deprecated 3ファイル（`win_message_handler.rs`, `win_thread_mgr.rs`, `winproc.rs`）を分割・移行・削除せず、600行ルールの例外として現状のまま維持する。
3. While deprecated 3ファイルが現役の参照（`process_singleton.rs` の `winproc` 参照、`app.rs`/`world/mod.rs`/`vsync.rs` の `win_thread_mgr` グローバルatomic参照、12個の examples の参照）を保持している間, the リファクタリング作業 shall 当該参照を破壊しない。
4. The リファクタリング作業 shall 公開APIの再設計・リネームなど後方互換を壊す変更を行わない。

### Requirement 6: フェーズ分離とクレート単位ウェーブ実行

**Objective:** メンテナとして、作業を独立して着手・レビュー可能な単位に分けて段階実行したい。それにより1ファイル＝1レビュー単位の明確さを保ち、リスクを局所化するため。

#### Acceptance Criteria

1. The リファクタリング作業 shall 「死体削除フェーズ」と「分割フェーズ」を独立して着手・レビュー可能な2フェーズとして扱う。
2. When 分割フェーズを実行するとき, the リファクタリング作業 shall クレート単位（wintf → dola → areka）のウェーブで進行する。
3. When 各ファイルの分割または削除を完了するごとに, the リファクタリング作業 shall 当該変更を独立してレビュー可能な単位として区切る。
4. While あるクレートのウェーブを実行している間, the リファクタリング作業 shall 当該クレートが独立してビルド・テスト可能であることを利用して `cargo test` を実行する。
