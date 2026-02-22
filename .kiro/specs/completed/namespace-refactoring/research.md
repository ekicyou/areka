# Research & Design Decisions: namespace-refactoring

## Summary
- **Feature**: `namespace-refactoring`
- **Discovery Scope**: Extension（既存コードベースのモジュール構造リファクタリング）
- **Key Findings**:
  - Rust integration test のサブディレクトリ化にはエントリポイント方式が最適（Cargo.toml 変更不要、共通ヘルパー解決が自然）
  - wintf `ecs/` 直下の3ファイル移動は `pub use` 再エクスポート経由で外部 API 影響を最小化可能
  - `#[path]` パターンの完全除去は別仕様 `test-module-separation` で対応済み — 本仕様の Req 3 は命名規約の文書化と統合テスト命名に集中

## Research Log

### Rust Integration Test サブディレクトリ方式の選定
- **Context**: Req 1・Req 2 でテストを機能ドメイン別サブディレクトリに分類する必要がある。Rust の `tests/` 直下 `.rs` のみが独立テストバイナリとして認識される制約への対処。
- **Sources Consulted**: Cargo Book — Integration Tests, gap-analysis.md セクション 3
- **Findings**:
  - **Option A（エントリポイント方式）**: `tests/compile.rs` がエントリポイントとなり `tests/compile/` サブディレクトリ内のモジュールを `mod` で取り込む。Cargo.toml 変更不要。共通ヘルパーは `tests/compile/common/mod.rs` に自然配置。
  - **Option B（`[[test]]` 方式）**: 各ファイルを Cargo.toml で個別指定。62ファイル分の定義で Cargo.toml 肥大化。運用コスト高。
  - **Option C（ハイブリッド）**: 本仕様では tests/ 直下にファイルを残さない方針が確定済みのため、純粋な Option A を全ドメインに適用する形に帰着。
- **Implications**: 全ドメインにエントリポイント方式を一律適用。各ドメインのテストは単一バイナリにまとまるが、テスト数（最大6件/ドメイン）では並列性への影響は軽微。

### wintf `ecs/` 直下ファイルの移動影響
- **Context**: Req 4 で `monitor.rs`, `window_system.rs`, `nchittest_cache.rs` をサブモジュールに移動する。
- **Sources Consulted**: `ecs/mod.rs`, `areka/src/main.rs`, `world/mod.rs`, `window_proc/mouse_move.rs`, `layout/systems/monitor_systems.rs`
- **Findings**:
  - `monitor.rs` は `pub use monitor::*;` で再エクスポート済み → `window/` に移動後も `ecs/mod.rs` の `pub use` を更新すれば外部 API 維持可能
  - `window_system.rs` は `pub` でなく `mod` 宣言 → 内部参照のみ（`world/mod.rs` が `crate::ecs::window_system::create_windows` で使用）
  - `nchittest_cache.rs` も `mod` 宣言 → 内部参照のみ（`world/mod.rs`, `window_proc/mouse_move.rs`）
  - 外部クレート（`areka`, `examples`）は `window_system` / `nchittest_cache` を直接参照していない
- **Implications**: `monitor.rs` のみ `pub use` 再エクスポートの更新が必要。`window_system.rs` と `nchittest_cache.rs` は内部パスの更新のみ。外部 API への影響なし。

### 共通ヘルパーの重複と統合
- **Context**: gap-analysis.md で識別された重複ヘルパー関数の共通化が必要。
- **Findings**:
  - **dola**: `compile_common/` (4/6テスト利用), `trigger_common/` (3/4テスト利用) は既存。`validation` ドメインに `minimal_valid_doc()` が3ファイルに重複 → `validation/common/mod.rs` に統合すべき。
  - **wintf**: `setup_graphics()` が visual 系5ファイルに重複 → `visual/common/mod.rs` に統合すべき。
  - **dola `compile_integration_test.rs`** のローカル `make_doc()` は `compile_common::make_doc_with_storyboard` と同等 → 統合候補。
- **Implications**: エントリポイント方式では `mod common;` でサブディレクトリ内の共通モジュールを自然に参照できるため、共通化のタイミングはテスト移動と同時が最適。

### `#[path]` パターンと test-module-separation 仕様の関係
- **Context**: 本仕様の Req 3 は「テスト命名規約の統一」および「`#[path]` テストの命名一貫性確認・修正」をカバー。一方、別仕様 `test-module-separation` が `#[path]` パターンの完全除去（ディレクトリモジュール化）を独立して取り扱う。
- **Findings**:
  - `test-module-separation` は Option A（ディレクトリモジュール化）を確定済み。9箇所すべてを `foo.rs` → `foo/mod.rs` + `foo/tests.rs` に変換予定。
  - `namespace-refactoring` Req 3 基準 3.2〜3.3 は `#[path]` テストの命名に言及しているが、`test-module-separation` 完了後は `#[path]` 自体が消滅するため、命名規約は「ディレクトリモジュール化されたテストファイルの命名」として再解釈される。
  - 実装順序: `test-module-separation` → `namespace-refactoring` の順が自然（`#[path]` 除去後にディレクトリ構造を整理）。
- **Implications**: Req 3 の設計では `#[path]` の存在を前提とせず、`test-module-separation` 完了後のファイル構造を前提とする。Req 3 は命名規約の文書化（structure.md 追記）と統合テスト命名の標準化に集中。

### `graphics_tests.rs` 親ディレクトリ参照パターン
- **Context**: `ecs/graphics/mod.rs` が `#[path = "../graphics_tests.rs"]` で `ecs/graphics_tests.rs` を参照する異常パターン。
- **Findings**:
  - `test-module-separation` 仕様で `graphics_tests.rs` → `graphics/tests.rs` への移動が計画済み
  - 本仕様の Req 4 でも `graphics_tests.rs` のサブモジュールへの配置が求められている
  - 両仕様で同一ファイルを扱うが、内容は矛盾しない（移動先が一致）
- **Implications**: `test-module-separation` が先に実行されれば、本仕様での追加対応は不要。設計では依存関係を明記。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| エントリポイント方式 (A) | `tests/domain.rs` + `tests/domain/` | Cargo.toml 変更不要、共通ヘルパー自然配置 | ドメイン内テストが単一バイナリに統合 | 全ドメインに一律適用で確定 |
| `[[test]]` 方式 (B) | Cargo.toml で各テストを個別定義 | 個別バイナリ維持 | 62行の追加定義、運用コスト高 | 棄却 |
| 最小限移動 (prod) | ecs/ 直下3ファイルのみ移動 | 低リスク | — | 採用 |

## Design Decisions

### Decision: テストサブディレクトリ化にエントリポイント方式を一律採用
- **Context**: tests/ 直下にファイルを残さない方針（開発者確認済み）と、Cargo.toml 肥大化の回避
- **Alternatives Considered**:
  1. `[[test]]` 方式 — 独立バイナリ維持だが運用コスト大
  2. ハイブリッド方式 — tests/ 直下なし方針により純粋 A に帰着
- **Selected Approach**: エントリポイント方式（Option A）を全ドメインに適用
- **Rationale**: Cargo 標準メカニズムで動作、共通ヘルパー配置が自然、tests/ 直下なし方針に適合
- **Trade-offs**: ドメイン内テストが単一コンパイル単位に統合されるが、各ドメイン最大6テストのため影響軽微
- **Follow-up**: コンパイル時間への影響を実装後に計測

### Decision: wintf ecs/ 直下ファイルの移動先
- **Context**: `monitor.rs`, `window_system.rs`, `nchittest_cache.rs` の適切なサブモジュール配置
- **Selected Approach**:
  - `monitor.rs` → `window/monitor.rs`
  - `window_system.rs` → `window/window_system.rs`
  - `nchittest_cache.rs` → `pointer/nchittest_cache.rs`
- **Rationale**: ギャップ分析で識別済みのドメイン対応、内部参照パスの更新のみで外部 API 影響なし
- **Trade-offs**: `pub use monitor::*` の更新箇所あり
- **Follow-up**: `ecs/mod.rs` の `pub use` チェーンの更新手順を設計に明記

### Decision: Req 3 の #[path] 関連スコープ
- **Context**: 別仕様 `test-module-separation` が `#[path]` 除去をカバー
- **Selected Approach**: Req 3 は統合テスト命名規約の文書化と統合テスト命名の標準化に集中。`#[path]` テストの構造変更は `test-module-separation` に委譲
- **Rationale**: 責務の明確な分離、重複作業の回避
- **Trade-offs**: 2仕様間の実装順序依存が発生
- **Follow-up**: 実装時に `test-module-separation` 完了を前提条件とする

## Risks & Mitigations
- **ドメイン分類の誤り** — テスト移動後に `cargo test` で即座に検証。分類変更は git で容易にロールバック可能
- **共通ヘルパーのパス解決エラー** — エントリポイント方式では `mod common;` が自然に解決。移動完了後に `cargo test` で検証
- **pub use チェーン破損** — `ecs/mod.rs` の更新時にコンパイルエラーで即座に検出。影響範囲は内部パスのみ
- **test-module-separation との実装順序** — 本仕様の Req 3 実装時に `test-module-separation` 未完了の場合、命名規約の文書化のみ先行実施可能

## References
- Cargo Book — Integration Tests: https://doc.rust-lang.org/cargo/guide/project-layout.html
- `.kiro/specs/test-module-separation/` — `#[path]` パターン除去の姉妹仕様
- `.kiro/specs/namespace-refactoring/gap-analysis.md` — 詳細な現状分析
