# Research & Design Decisions

## Summary
- **Feature**: `codebase-file-split-refactor`
- **Discovery Scope**: Extension（既存コードベースのリファクタリング）
- **Key Findings**:
  - 500行超ソースファイル16件中、6件は内部テストが大部分を占め、本体は適正サイズ
  - `win_message_handler.rs`（1378行）は全体が非推奨（`#[deprecated]`）で削除候補
  - 真に分割が必要なファイルは実質10〜12件、各ファイルに明確な責務境界が存在

## Research Log

### ファイルサイズと分割必要性の評価

- **Context**: 500行超ソースファイル16件の内部構造を精査し、「テスト膨張型」と「実装肥大型」を分類
- **Findings**:
  - **テスト膨張型**（テスト切り出しのみで解決）: `hit_test.rs`（本体565行/テスト905行）、`hit_region.rs`（本体498行/テスト555行）、`loop_controller.rs`（本体168行/テスト430行）
  - **実装肥大型**（機能単位分割が必要）: `graphics/systems.rs`、`window.rs`、`pointer/mod.rs`、`d2d/command.rs`、`layout/systems.rs`、`compile.rs`、`compositor_systems.rs`、`world.rs`、`typewriter_systems.rs`、`validate.rs`
  - **非推奨・削除候補**: `win_message_handler.rs`（全体が `#[deprecated]`、後継は `ecs::window_proc`）
- **Implications**: 分割戦略はファイル特性に応じて3パターン（テスト外部化、機能単位分割、削除）を使い分ける

### Rustモジュール分割パターン

- **Context**: ファイル→ディレクトリモジュール変換時のパス互換性確保手法
- **Findings**:
  - `foo.rs` → `foo/mod.rs` + `foo/bar.rs` の変換でモジュールパス `crate::foo` は変化しない
  - `pub use` による re-export で外部APIを保持可能
  - 内部テスト `#[cfg(test)] mod tests` は `#[path = "tests.rs"] mod tests;` で外部ファイル化可能
  - `pub(crate)` 関数は同一クレート内のみアクセス可能なため、テストを統合テスト（`tests/`）に移動すると `pub(crate)` アクセスを失う
- **Implications**: テスト外部化は `#[path]` 属性または同一ディレクトリ内配置を使用。統合テストフォルダへの移動は `pub(crate)` 制約に注意

### テスト・サンプルファイルの分割手法

- **Context**: テストファイル（`tests/`直下）とサンプル（`examples/`）の分割可否
- **Findings**:
  - テストファイル（統合テスト）: 同じ `tests/` ディレクトリに複数ファイルとして分割可能。共有ヘルパーは `tests/common/mod.rs` に配置
  - サンプルファイル: `examples/` 直下の `.rs` ファイルは各々独立したバイナリ。分割する場合はディレクトリ例（`examples/foo/main.rs` + `examples/foo/helper.rs`）を使用
  - `taffy_flex_demo.rs`（2027行）: 単一サンプルが2000行超は異例。ヘルパー関数・設定定義・各デモパターンの切り分けが自然
- **Implications**: テスト分割は低リスク。サンプル分割はディレクトリ構造への変換が必要

## Architecture Pattern Evaluation

| Option           | Description                                                            | Strengths                     | Risks / Limitations                         | Notes                         |
| ---------------- | ---------------------------------------------------------------------- | ----------------------------- | ------------------------------------------- | ----------------------------- |
| 機能単位分割     | 同一ファイル内の論理グループを別ファイルに抽出し `mod.rs` で re-export | API互換性維持、段階的適用可能 | `mod.rs` が肥大化する可能性                 | 全ファイルで採用              |
| テスト外部化     | `#[cfg(test)] mod tests` を `#[path]` 属性で別ファイルに移動           | 本体サイズ削減、テスト独立性  | `pub(crate)` テストはクレート内に留まる必要 | テスト膨張型ファイルで採用    |
| 非推奨コード削除 | `#[deprecated]` マーク済みモジュールを完全削除                         | 最大のサイズ削減              | 利用箇所が残存する場合ビルドエラー          | `win_message_handler.rs` のみ |

## Design Decisions

### Decision: テスト膨張型ファイルの処理方針

- **Context**: `hit_test.rs`、`hit_region.rs`、`loop_controller.rs` は本体が適正サイズだがテストが巨大
- **Alternatives Considered**:
  1. テストを `#[path]` 属性で同ディレクトリの別ファイルに分離
  2. テストを `tests/` 統合テストフォルダに移行
  3. 現状維持（テストは本体と一体が慣習的）
- **Selected Approach**: Option 1 — `#[path]` 属性によるテスト外部ファイル化
- **Rationale**: `pub(crate)` 関数のユニットテストを維持しつつ、本体ファイルのコンテキストサイズを削減。Rustの慣習とも矛盾しない
- **Trade-offs**: テスト編集時にファイル切り替えが必要になるが、AI支援では各ファイルが小さい方が有利

### Decision: `win_message_handler.rs` の処理方針

- **Context**: 全体が `#[deprecated]` で後継モジュール `ecs::window_proc` が存在
- **Alternatives Considered**:
  1. 機能単位で分割
  2. 全体を削除
  3. 現状維持（非推奨のまま残す）
- **Selected Approach**: Option 3 — 現状維持（本スペックのスコープ外）
- **Rationale**: 削除は別途の breaking change スペックで管理すべき。本リファクタリングは「分割」が目的であり、機能削除は含まない。非推奨の1378行ファイルを分割するのは無意味
- **Trade-offs**: 巨大ファイルが1件残存するが、非推奨マーク済みのため混乱は限定的

### Decision: `loop_controller.rs` の処理方針

- **Context**: 実装168行 + テスト430行 = 599行。テスト外部化も可能だが実装部分は十分に小さい
- **Alternatives Considered**:
  1. テスト外部化
  2. 現状維持
- **Selected Approach**: Option 2 — 現状維持
- **Rationale**: 実装168行はAIが十分把握できるサイズ。テスト含めて599行はソースファイルのハード上限500行を超えるが、テスト部分は `#[cfg(test)]` で実質無視可能。分割の労力に見合わない
- **Trade-offs**: テスト込み599行が閾値を上回るが、実質的影響は軽微

## Risks & Mitigations

- **依存パス破壊**: `use crate::ecs::window::*` 等のパスが変更される → `mod.rs` での `pub use` re-export で完全互換維持
- **コンパイルエラー連鎖**: 1ファイルの分割ミスが他モジュールに波及 → ファイル単位で段階的に分割し、各段階で `cargo build` 検証
- **テスト不整合**: 分割後にテストが失敗する → 各段階で `cargo test` 実行。テスト外部化は `#[path]` 属性で同一コンパイル単位を維持
- **非推奨ファイルの扱いの混乱**: `win_message_handler.rs` を分割対象から除外したことへの疑問 → design.md に明記し、別スペックでの削除を推奨

## References

- [Rust Module System](https://doc.rust-lang.org/reference/items/modules.html) — `#[path]` 属性、ディレクトリモジュール
- [Cargo Test Organization](https://doc.rust-lang.org/cargo/guide/tests.html) — 統合テスト vs ユニットテスト
- [bevy_ecs](https://docs.rs/bevy_ecs/) — ECS system 関数シグネチャパターン
