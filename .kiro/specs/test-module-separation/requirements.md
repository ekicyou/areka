# Requirements Document

## Introduction

プロジェクト内の9箇所で使用されている `#[path = "xxx_tests.rs"]` パターンを、より慣用的な Rust のテストモジュール分離手法にリファクタリングする。`#[path]` アトリビュートはコンパイラ指示子としては正当だが、Rust コミュニティの標準的なプラクティスから外れており、コード可読性・保守性の観点で改善の余地がある。

### 背景

現在のパターン:
```rust
#[cfg(test)]
#[path = "instance_manager_tests.rs"]
mod tests;
```

このパターンは以下の点で問題がある:
- Rust エコシステムで一般的でなく、コードレビュー時に戸惑いを生む
- ファイル配置の意図が暗黙的であり、モジュールツリーと実ファイル構造の乖離を招く
- IDE のモジュール解決との相性が不安定な場合がある

### 対象箇所（9箇所）

| # | クレート | ソースファイル | テストファイル | テスト行数 |
|---|---------|--------------|--------------|-----------|
| 1 | dola | `runtime/instance_manager.rs` | `instance_manager_tests.rs` | 188 |
| 2 | dola | `runtime/interpolator.rs` | `interpolator_tests.rs` | 209 |
| 3 | dola | `runtime/subscription_manager.rs` | `subscription_manager_tests.rs` | 284 |
| 4 | dola | `runtime/timeline_manager.rs` | `timeline_manager_tests.rs` | 181 |
| 5 | wintf | `ecs/pointer/dispatch.rs` | `dispatch_tests.rs` | 163 |
| 6 | wintf | `ecs/layout/hit_region.rs` | `hit_region_tests.rs` | 554 |
| 7 | wintf | `ecs/layout/hit_test.rs` | `hit_test_tests.rs` | 330 |
| 8 | wintf | `ecs/layout/hit_test.rs` | `hit_test_ex_tests.rs` | 593 |
| 9 | wintf | `ecs/graphics/mod.rs` | `../graphics_tests.rs` | 143 |

### 制約条件

- **プライベートアイテムアクセス**: `hit_region.rs` の `point_in_polygon` 関数はプライベートで、テストが直接呼び出している
- **`pub(crate)` アクセス**: dola の多くのモジュールは `pub(crate)` を使用しており、テストは子モジュールとしてアクセスしている
- **複数テストファイル**: `hit_test.rs` は `tests` と `tests_ex` の2つの外部テストモジュールを持つ唯一のケース
- **親ディレクトリ参照**: `graphics/mod.rs` は `../graphics_tests.rs` を参照する特殊パターン
- **既存の慣用パターン**: `bitmap_source/mod.rs` は `#[path]` なしの `#[cfg(test)] mod tests;` を使用しており、これが参考モデルとなる

## Requirements

### Requirement 1: 慣用的テストモジュール構造への移行

**Objective:** 開発者として、`#[path]` アトリビュートを使わない慣用的な Rust テストモジュール構造に移行したい。コードベースの可読性と保守性を向上させ、Rust エコシステムの標準プラクティスに準拠するため。

#### Acceptance Criteria

1. When リファクタリングが完了した時, the build system shall `cargo test` で全既存テストが変更前と同じ結果（pass/fail）を維持すること
2. When リファクタリングが完了した時, the codebase shall `#[path = "..."]` アトリビュートをテストモジュール宣言に一切含まないこと
3. The codebase shall 全対象箇所においてディレクトリモジュール化方式（`foo.rs` → `foo/mod.rs` + `foo/tests.rs`）を採用すること（インライン化は行わない）
4. When テストファイルが外部ファイルとして分離される場合, the module system shall 標準モジュール解決（`<module_name>/tests.rs`）に準拠し、`mod tests;` が自動的にファイルを解決できること

### Requirement 2: テストカバレッジの維持

**Objective:** 開発者として、リファクタリングによってテストカバレッジが低下しないことを保証したい。既存テストの正確性を維持するため。

#### Acceptance Criteria

1. The refactoring shall 既存のテストケース数を減少させないこと（テスト関数の削除禁止）
2. The refactoring shall テストモジュールが `use super::*;` パターンで親モジュールのアイテムにアクセスする既存の慣習を維持すること（例外: `graphics/tests.rs` は内部にサブモジュールを持つ構造上 `use crate::` パスを使用しており、変更しない）
3. While プライベートアイテム（`hit_region.rs` の `point_in_polygon` 等）がテストされている場合, the refactoring shall 当該アイテムのテスト可能性を維持する構造を選択すること
4. While `pub(crate)` アイテムがテストされている場合, the refactoring shall テストモジュールが当該アイテムへのアクセス権を保持する構造を選択すること

### Requirement 3: ファイル構造の整合性

**Objective:** 開発者として、テストファイルの配置がモジュール構造と一致し、ファイルの発見容易性を維持したい。プロジェクトの構造的一貫性を保つため。

#### Acceptance Criteria

1. The refactoring shall 全9箇所に対してディレクトリモジュール化（`foo.rs` → `foo/mod.rs` + `foo/tests.rs`）を一貫して適用すること
2. The file structure shall 全対象モジュールにおいて `<module_name>/mod.rs`（プロダクションコード）と `<module_name>/tests.rs`（テストコード）の構成を取ること
3. The refactoring shall `graphics/mod.rs` の親ディレクトリ参照（`../graphics_tests.rs`）パターンを解消すること
4. When `hit_test.rs` のように1ソースから複数テストファイルが参照されている場合, the refactoring shall 複数テストモジュールの共存を慣用的な構造で実現すること

### Requirement 4: 既存コードへの影響最小化

**Objective:** 開発者として、リファクタリングの影響範囲をテストモジュール構造に限定したい。プロダクションコードの安定性を確保するため。

#### Acceptance Criteria

1. The refactoring shall プロダクションコード（非テストコード）のロジックを一切変更しないこと
2. The refactoring shall 既存の `pub` / `pub(crate)` 可視性を変更しないこと（テストのためだけに可視性を拡大することは行わない）
3. If テスト可能性を維持するために可視性の調整が不可避な場合, the refactoring shall その影響範囲を最小限（`pub(crate)` 以下）に留め、変更箇所を明文化すること
4. The refactoring shall `Cargo.toml` の依存関係や feature フラグに変更を加えないこと

### Requirement 5: 段階的移行の実行可能性

**Objective:** 開発者として、リファクタリングを段階的に（モジュール単位で）実行できるようにしたい。一度に全箇所を変更するリスクを避けるため。

#### Acceptance Criteria

1. The refactoring plan shall 各対象箇所を独立したタスクとして実行可能な単位に分割すること
2. When 個別のモジュールが移行された時, the build system shall 他の未移行モジュールに影響なく `cargo build` と `cargo test` が成功すること
3. The refactoring plan shall 難易度の低い箇所（`pub` アイテムのみのモジュール）から着手し、段階的に複雑な箇所（プライベートアイテムのテスト、複数テストファイル）に進む順序を提供すること
