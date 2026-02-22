# Research & Design Decisions: test-module-separation

## Summary
- **Feature**: `test-module-separation`
- **Discovery Scope**: Simple Addition（既存パターンの機械的適用）
- **Key Findings**:
  - `bitmap_source/` が既にディレクトリモジュール化の実績モデルとして存在し、そのパターンをそのまま踏襲可能
  - Rust の可視性規則により、ディレクトリモジュール化後も可視性変更は一切不要
  - Option D（リネームのみ）は技術的に不可能であり、ディレクトリモジュール化が唯一の実現手段

## Research Log

### Rust モジュール解決と `#[path]` の技術的背景
- **Context**: `#[path]` がなぜ使用されているのかを明確化する必要があった
- **Sources Consulted**: Rust Reference (Module system), Rust Edition Guide 2024
- **Findings**:
  - フラットファイル `foo.rs` から `mod tests;` とした場合、Rust は同一ディレクトリの `foo/tests.rs`（ディレクトリモジュール時）または `tests.rs`（単一ファイル時）を探す
  - 単一ファイルモジュールが複数存在するディレクトリでは、各モジュールの `mod tests;` が同一の `tests.rs` を参照しようとして衝突する
  - `#[path]` はこの衝突を回避する目的で導入された（プライベートアクセス目的ではない）
- **Implications**: `#[path]` 除去にはディレクトリモジュール化が不可避

### 子モジュールからの可視性アクセス
- **Context**: ディレクトリモジュール化後にプライベート/`pub(crate)` アイテムへのテストアクセスが維持されるか確認
- **Sources Consulted**: Rust Reference (Visibility and privacy)
- **Findings**:
  - Rust では子モジュールは親モジュールのプライベートアイテムにアクセス可能（`hit_region::point_in_polygon` のケース）
  - `pub(crate)` アイテムは同一クレート内のどこからでもアクセス可能
  - ディレクトリモジュール化（`foo.rs` → `foo/mod.rs`）後も `mod tests;` で宣言されるテストモジュールは `foo` の子モジュールのまま
- **Implications**: 可視性変更は一切不要。全テストがそのまま動作する

### git 履歴の追跡
- **Context**: `foo.rs` → `foo/mod.rs` の移動で git の履歴追跡がどう扱われるか
- **Sources Consulted**: git documentation (diff.renameLimit, -M option)
- **Findings**:
  - `git mv foo.rs foo/mod.rs` は内容が同一であればリネームとして検出される（similarity index 100%）
  - テストファイルの移動（`foo_tests.rs` → `foo/tests.rs`）も同様にリネーム検出される
  - `graphics_tests.rs` → `graphics/tests.rs` は mod 名変更を伴うが内容の大部分は同一
- **Implications**: git の変更追跡に問題なし

### `graphics_tests.rs` の import パターン分析
- **Context**: `graphics_tests.rs` が `use crate::` パスを使用している理由の確認
- **Sources Consulted**: プロジェクト内のコード調査
- **Findings**:
  - `graphics_tests.rs` は3つのネストされたサブモジュール（`graphics_core_tests`, 他）を含む
  - 各サブモジュール内で `use super::*` とした場合、`super` は `tests` モジュールを指し、`graphics` モジュールには到達しない
  - `use crate::ecs::graphics::*` という絶対パスが使用されており、これはディレクトリモジュール化後も変更不要
- **Implications**: `graphics/tests.rs` では `use crate::` パスをそのまま維持する（確定済み設計判断）

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: ディレクトリモジュール化 | `foo.rs` → `foo/mod.rs` + `foo/tests.rs` | 既存モデル踏襲、全要件充足、一貫性 | ディレクトリ数 +7 | **確定** |
| B: インライン化 | テストをソースファイル末尾にインライン | シンプル | ファイルサイズ肥大化、Req 1.3 違反 | 棄却 |
| C: ハイブリッド | 規模に応じて A/B を選択 | 柔軟 | 一貫性欠如（Req 3.1 違反）、閾値の恣意性 | 棄却 |
| D: リネーム | `tests.rs` 衝突回避を期待 | 最少変更 | 技術的に不可能 | 棄却 |

## Design Decisions

### Decision: 全箇所でディレクトリモジュール化を採用
- **Context**: `#[path]` 除去のための構造変更方針の選択
- **Alternatives Considered**:
  1. Option A — ディレクトリモジュール化（全面）
  2. Option B — インライン化（全面）
  3. Option C — ハイブリッド
  4. Option D — リネームのみ（技術的に不可能と判明）
- **Selected Approach**: Option A（ディレクトリモジュール化）
- **Rationale**: プロジェクト内に `bitmap_source/` という既存モデルがあり、全要件を満たす唯一のオプション
- **Trade-offs**: ディレクトリ数が +7 増加するが、モジュール構造の明示性と一貫性を優先
- **Follow-up**: 実装は モジュール単位で段階的に実行

### Decision: `graphics/tests.rs` は `use crate::` パスを維持
- **Context**: `graphics_tests.rs` のインポートパスを `use super::*` に統一するか
- **Alternatives Considered**:
  1. `use super::*` に統一
  2. `use crate::ecs::graphics::*` を維持
- **Selected Approach**: `use crate::` パスを維持
- **Rationale**: テストファイル内に3つのネストされたサブモジュールがあり、`super` は `tests` モジュールを指すため `use super::*` では `graphics` のアイテムに到達できない
- **Trade-offs**: プロジェクト内の他テストファイルとインポートスタイルが異なるが、構造上の必然
- **Follow-up**: なし

### Decision: `tests_ex` モジュール名を維持
- **Context**: `hit_test.rs` の第2テストモジュール `tests_ex` の命名を標準化するか
- **Alternatives Considered**:
  1. `tests_ex` を維持
  2. `tests_extended` にリネーム
  3. 一般的な名前に変更
- **Selected Approach**: `tests_ex` を維持
- **Rationale**: テスト対象の `_ex` 系関数群（`hit_test_entity_ex`, `hit_test_ex`, `hit_test_in_window_ex`）を反映した意味ある命名
- **Trade-offs**: なし
- **Follow-up**: なし

### Decision: Phase 1 の作業粒度は実装者判断
- **Context**: dola runtime の4モジュール（instance_manager, interpolator, subscription_manager, timeline_manager）を個別コミットすべきか一括コミットすべきか
- **Alternatives Considered**:
  1. 4モジュールをまとめて1コミット（作業効率優先）
  2. 1つずつ移行・コミット（Req 5.2 厳守）
  3. どちらでもよい（実装者判断）
- **Selected Approach**: どちらでもよい（実装者判断に委ねる）
- **Rationale**: 4モジュールは互いに独立しており、個別に移行してもビルド・テストは成功する（Req 5.2 充足）。一方で同一ディレクトリ・同一パターンであり、まとめて移行しても問題ない。作業効率とリスク管理のバランスを実装者が判断してよい。
- **Trade-offs**: 一括コミットは効率的だが問題発生時の切り分けがやや困難。個別コミットは切り分け容易だがコミット回数が増える。
- **Follow-up**: design.md の Phase 1 に「個別コミット/一括コミット両対応」と明記

## Risks & Mitigations
- **Risk 1**: `git mv` 後の履歴追跡漏れ → `git log --follow` で変更追跡を確認
- **Risk 2**: rust-analyzer の一時的な認識遅延 → ディレクトリモジュール化は rust-analyzer が完全サポートする標準パターンであり問題なし
- **Risk 3**: `graphics_tests.rs` 移動時の import 漏れ → mod 名変更（`graphics_tests` → `tests`）のみで import パスの変更は不要

## namespace-refactoring 影響評価

namespace-refactoring（`phase: implementation-complete`）実施後の影響を評価。

### 影響なし（全対象ファイルはパス変更なし）

全17ファイル（9ソース + 8テスト）の配置に変化なし。namespace-refactoring の移動対象（`monitor.rs`, `window_system.rs`, `nchittest_cache.rs`）は test-module-separation の対象外。

### 間接的影響（2件）

1. **`pointer/` ディレクトリに `nchittest_cache.rs` が追加**: `ecs/nchittest_cache.rs` → `ecs/pointer/nchittest_cache.rs` に移動済み。`dispatch.rs` のディレクトリモジュール化には影響なし（ディレクトリ内のファイル数が1つ増えるのみ）。
2. **`structure.md` にテスト命名規約が追加**: Test Naming Conventions セクションに「Separated: `{module}/tests.rs` — ディレクトリモジュール化パターン（`bitmap_source/` を参照）」が文書化済み。test-module-separation の移行先パターンと完全に一致。

## References
- [Rust Reference: Modules](https://doc.rust-lang.org/reference/items/modules.html) — モジュール解決規則
- [Rust Reference: Visibility and Privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html) — 子モジュールのプライベートアクセス規則
- プロジェクト内参考モデル: `crates/wintf/src/ecs/widget/bitmap_source/` — ディレクトリモジュール化の実績
