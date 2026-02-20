# Requirements Document

## Introduction

全クレート（`areka`, `dola`, `wintf`）のソースコードを対象に、AIフレンドリーなファイルサイズへの分割リファクタリングを実施する。LLMのコンテキストウィンドウで効率的に処理でき、AIコーディングアシスタントが正確に理解・編集できるファイルサイズを目標とする。

### 現状分析

全131ファイルの行数分布（テスト・exampleを含む）：

| 行数範囲   | ファイル数 | 代表的なファイル                                                                                                                                         |
| ---------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1000行超   | 6          | `taffy_flex_demo.rs`(2027), `win_message_handler.rs`(1378), `graphics/systems.rs`(1373), `compile_test.rs`(1300), `hit_test.rs`(1247), `window.rs`(1094) |
| 500〜999行 | 15         | `hit_region.rs`(937), `pointer/mod.rs`(917), `d2d/command.rs`(909), `compile.rs`(743), `world.rs`(626) 他                                                |
| 300〜499行 | 22         | `areka/main.rs`(354), `tree_system.rs`(348), `win_style.rs`(326) 他                                                                                      |
| 300行未満  | 88         | 大半のファイル（分割不要）                                                                                                                               |

## Requirements

### Requirement 1: ファイルサイズ閾値の定義

**Objective:** 開発者として、AIコーディングアシスタントが1ファイルを完全に把握できるファイルサイズ基準を設けたい。これにより、AI支援による開発効率を最大化できる。

#### 背景・根拠

- LLMの一般的なコンテキストウィンドウは数万〜十数万トークン
- Rustコードは1行あたり平均3〜5トークン（コメント・空行含む）
- 300行 ≈ 900〜1500トークンは、周辺コンテキストを含めても余裕を持って処理可能
- 500行を超えると、AIが構造全体を把握しつつ局所変更を行う精度が低下しやすい
- テストファイルは構造が反復的であるためやや緩和可能だが、1000行超は分割が望ましい

#### Acceptance Criteria

1. The refactoring tool shall ソースファイル（`src/`配下）の推奨上限を **300行**、ハード上限を **500行** と定義する
2. The refactoring tool shall テストファイル（`tests/`配下）の推奨上限を **500行**、ハード上限を **800行** と定義する
3. The refactoring tool shall サンプルファイル（`examples/`配下）の推奨上限を **500行**、ハード上限を **800行** と定義する
4. The refactoring tool shall `mod.rs` および `lib.rs` は re-export とモジュール宣言のみとし、**100行以下** を目標とする

### Requirement 2: ソースファイル分割（必須対象）

**Objective:** 開発者として、500行を超える全ソースファイルを機能単位で分割したい。これにより、各ファイルが単一責務を持ち、AI・人間双方にとって理解しやすいコードベースを実現できる。

#### 分割対象ファイル（ソース・500行超）

| #   | ファイル                             | 行数 | クレート |
| --- | ------------------------------------ | ---- | -------- |
| 1   | `win_message_handler.rs`             | 1378 | wintf    |
| 2   | `ecs/graphics/systems.rs`            | 1373 | wintf    |
| 3   | `ecs/layout/hit_test.rs`             | 1247 | wintf    |
| 4   | `ecs/window.rs`                      | 1094 | wintf    |
| 5   | `ecs/layout/hit_region.rs`           | 937  | wintf    |
| 6   | `ecs/pointer/mod.rs`                 | 917  | wintf    |
| 7   | `com/d2d/command.rs`                 | 909  | wintf    |
| 8   | `ecs/layout/systems.rs`              | 748  | wintf    |
| 9   | `compile.rs`                         | 743  | dola     |
| 10  | `ecs/graphics/compositor_systems.rs` | 663  | wintf    |
| 11  | `ecs/world.rs`                       | 626  | wintf    |
| 12  | `ecs/window_proc/mouse_button.rs`    | 614  | wintf    |
| 13  | `window_proc/window_pos.rs`          | 560  | wintf    |
| 14  | `widget/text/typewriter_system.rs`   | 539  | wintf    |
| 15  | `runtime/loop_controller.rs`         | 536  | dola     |
| 16  | `validate.rs`                        | 518  | dola     |

#### Acceptance Criteria

1. When ソースファイルが500行を超えている場合, the refactoring shall そのファイルを論理的な機能単位（構造体定義、impl ブロック、システム関数群など）で分割する
2. The refactoring shall 分割後の各ファイルが300行以下になることを目標とする（最大500行）
3. The refactoring shall 分割時に既存のpublic APIを変更しない（re-exportにより外部インターフェースを保持する）
4. The refactoring shall 分割後のモジュール名が内容を明確に表す命名とする（例: `window.rs` → `window/components.rs`, `window/systems.rs`）

### Requirement 3: ソースファイル分割（推奨対象）

**Objective:** 開発者として、300〜500行のソースファイルについても、明確な責務分離が可能な場合は分割したい。

#### 推奨分割対象ファイル（ソース・300〜500行）

| #   | ファイル                          | 行数 | クレート |
| --- | --------------------------------- | ---- | -------- |
| 1   | `ecs/layout/high_level.rs`        | 482  | wintf    |
| 2   | `runtime/timeline_manager.rs`     | 462  | dola     |
| 3   | `ecs/window_proc/mouse_move.rs`   | 426  | wintf    |
| 4   | `runtime/facade.rs`               | 418  | dola     |
| 5   | `runtime/instance_manager.rs`     | 406  | dola     |
| 6   | `runtime/subscription_manager.rs` | 399  | dola     |
| 7   | `ecs/graphics/components.rs`      | 376  | wintf    |
| 8   | `widget/bitmap_source/systems.rs` | 373  | wintf    |
| 9   | `ecs/pointer/dispatch.rs`         | 365  | wintf    |
| 10  | `areka/main.rs`                   | 354  | areka    |
| 11  | `runtime/interpolator.rs`         | 354  | dola     |
| 12  | `ecs/drag/dispatch.rs`            | 352  | wintf    |
| 13  | `ecs/common/tree_system.rs`       | 348  | wintf    |
| 14  | `win_style.rs`                    | 326  | wintf    |
| 15  | `ecs/drag/state.rs`               | 315  | wintf    |
| 16  | `widget/text/typewriter.rs`       | 305  | wintf    |

#### Acceptance Criteria

1. While ソースファイルが300〜500行の範囲にある場合, the refactoring shall 明確な責務境界が存在するならば分割を実施する
2. If 300〜500行のファイルに明確な分割ポイントがない場合, the refactoring shall そのファイルを現状維持とし、分割を強制しない
3. The refactoring shall 推奨対象の分割は必須対象の完了後に実施する

### Requirement 4: テスト・サンプルファイル分割

**Objective:** 開発者として、大規模なテスト・サンプルファイルも適切なサイズに分割したい。テストの発見性と保守性を向上させるため。

#### 分割対象（テスト・サンプル・800行超）

| #   | ファイル                      | 行数 | 種別        |
| --- | ----------------------------- | ---- | ----------- |
| 1   | `examples/taffy_flex_demo.rs` | 2027 | example     |
| 2   | `tests/compile_test.rs`       | 1300 | test (dola) |
| 3   | `tests/trigger_test.rs`       | 980  | test (dola) |
| 4   | `tests/validation_test.rs`    | 910  | test (dola) |

#### Acceptance Criteria

1. When テスト/サンプルファイルが800行を超えている場合, the refactoring shall テストカテゴリまたは機能単位で分割する
2. The refactoring shall 分割後のテストファイルが500行以下になることを目標とする
3. The refactoring shall サンプルファイルの分割時に、各サンプルが独立して実行可能であることを保証する
4. The refactoring shall テスト分割時に、テストヘルパー関数を共有モジュールに抽出して重複を排除する

### Requirement 5: モジュール構造の整合性

**Objective:** 開発者として、分割後のモジュール構造が一貫性を保ち、コンパイルが通ることを保証したい。

#### Acceptance Criteria

1. The refactoring shall 分割対象がディレクトリモジュール化される場合（`foo.rs` → `foo/mod.rs` + `foo/bar.rs`）、既存のモジュールパスとの互換性を維持する
2. The refactoring shall 分割後に `cargo build` が成功すること
3. The refactoring shall 分割後に `cargo test` が全テストパスすること
4. The refactoring shall 分割後の `use` / `pub use` パスが最短かつ明確であること
5. If 循環依存が発生する分割案の場合, the refactoring shall その分割案を破棄し、代替の分割方法を採用する

### Requirement 6: コードフォーマット

**Objective:** 開発者として、全リファクタリング完了後にコードフォーマットを統一したい。

#### Acceptance Criteria

1. When 全ファイル分割が完了した後, the refactoring shall `cargo fmt --all` を実行してコードフォーマットを統一する
2. The refactoring shall `cargo fmt --all` の実行結果としてフォーマットエラーが発生しないこと
3. The refactoring shall フォーマット適用後に `cargo build` および `cargo test` が成功すること
