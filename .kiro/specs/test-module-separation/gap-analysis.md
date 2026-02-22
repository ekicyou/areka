# Gap Analysis: test-module-separation

## 1. Current State Investigation

### ドメイン関連アセット

プロジェクトは **Rust edition 2024** を使用するワークスペース構成（`dola`, `wintf`, `areka` クレート）。

#### `#[path]` パターンの分布

| # | クレート | ソースファイル | テストファイル | mod名 | 特殊性 |
|---|---------|--------------|--------------|-------|--------|
| 1 | dola | `runtime/instance_manager.rs` | `instance_manager_tests.rs` | `tests` | — |
| 2 | dola | `runtime/interpolator.rs` | `interpolator_tests.rs` | `tests` | — |
| 3 | dola | `runtime/subscription_manager.rs` | `subscription_manager_tests.rs` | `tests` | — |
| 4 | dola | `runtime/timeline_manager.rs` | `timeline_manager_tests.rs` | `tests` | — |
| 5 | wintf | `ecs/pointer/dispatch.rs` | `dispatch_tests.rs` | `tests` | — |
| 6 | wintf | `ecs/layout/hit_region.rs` | `hit_region_tests.rs` | `tests` | private 関数テスト |
| 7 | wintf | `ecs/layout/hit_test.rs` | `hit_test_tests.rs` | `tests` | 2ファイル構成 |
| 8 | wintf | `ecs/layout/hit_test.rs` | `hit_test_ex_tests.rs` | `tests_ex` | 上記の2つ目 |
| 9 | wintf | `ecs/graphics/mod.rs` | `../graphics_tests.rs` | `graphics_tests` | 親Dir参照・`use crate::` |

#### 既存ディレクトリ構造の特徴

- **dola `runtime/`**: フラット構成（16ファイル、サブディレクトリなし）
- **wintf `layout/`**: `systems/` サブディレクトリあり、他はフラット
- **wintf `pointer/`**: フラット（6ファイル）。namespace-refactoring により `nchittest_cache.rs` が `ecs/` から移動済み
- **wintf `graphics/`**: 既にディレクトリモジュール化済み（`mod.rs` + `compositor_systems/`, `systems/` 等のサブモジュール群）
- **wintf `ecs/`**: namespace-refactoring により `monitor.rs` → `window/monitor.rs`、`window_system.rs` → `window/window_system.rs`、`nchittest_cache.rs` → `pointer/nchittest_cache.rs` に移動済み。`app.rs` のみ `ecs/` ルートに残存

#### 既存の慣用パターン（参考モデル）

`bitmap_source/mod.rs` が `#[path]` なしの理想形:
```
bitmap_source/
├── mod.rs           # #[cfg(test)] mod tests;
├── tests.rs         # use super::*;
├── alpha_mask.rs
└── ...
```
- ディレクトリモジュール方式のため、`mod tests;` が `tests.rs` を自動解決
- テストファイルは `use super::*;` パターンで親アクセス

### コンベンション

| 項目 | 現状パターン |
|------|-------------|
| テストインポート | `use super::*;` が主流（8/9件）、`graphics_tests.rs` のみ `use crate::` |
| テストmod名 | `tests` が主流（7/9件）、`tests_ex`, `graphics_tests` が例外 |
| 可視性 | `pub(crate)` が多用（dola）、`pub` 混在（wintf） |
| テストファイル命名 | `{source_name}_tests.rs` パターン |

### 統合サーフェス

- **親モジュール宣言**: `mod instance_manager;` 等はファイル→ディレクトリ変更時も修正不要（Rust モジュール解決が吸収）
- **`#[cfg(test)]` ガード**: 全箇所で付与済み
- **git 履歴**: `foo.rs` → `foo/mod.rs` は `git mv` で追跡可能（similarity threshold 内）
- **テスト命名規約**: `structure.md` に `{module}/tests.rs` パターンが文書化済み（namespace-refactoring にて追記）

## 2. Requirements Feasibility Analysis

### 要件→技術ニーズのマッピング

| 要件 | 技術ニーズ | ギャップ |
|------|-----------|---------|
| Req 1: 慣用的構造への移行 | `#[path]` 除去、標準モジュール解決へ切替 | フラットファイル→ディレクトリモジュール変換が必要 |
| Req 2: テストカバレッジ維持 | `use super::*;` やプライベートアクセスの維持 | 問題なし（子モジュール関係維持） |
| Req 3: ファイル構造の整合 | 統一パターン（ディレクトリモジュール化） | 問題なし |
| Req 4: プロダクション影響ゼロ | 可視性・ロジック変更なし | 問題なし |
| Req 5: 段階的移行 | モジュール単位の独立実行 | 問題なし |

### プライベート/`pub(crate)` アクセスの詳細分析

| ファイル | テスト対象アイテム | 可視性 | ディレクトリモジュール化後 |
|---------|-------------------|--------|------------------------|
| `hit_region.rs` | `point_in_polygon` | **private** | ✅ 子モジュールからアクセス可 |
| `instance_manager.rs` | `InstanceManager`, `StoryboardInstance` | `pub(crate)` | ✅ 同一クレート内子モジュール |
| `subscription_manager.rs` | `SubscriptionManager` | `pub(crate)` | ✅ 同上 |
| `timeline_manager.rs` | `TimelineManager` 等 | `pub(crate)` | ✅ 同上 |
| `interpolator.rs` | `ObjectInternPool` | `pub(crate)` | ✅ 同上 |
| `hit_test.rs` | `RegionHit`, `hit_test_entity_ex` | `pub(crate)` | ✅ 同上 |

**結論**: ディレクトリモジュール化で `mod tests` は子モジュールとして維持されるため、**可視性の変更は一切不要**。

### 複雑性シグナル

| カテゴリ | 対象 | 複雑度 | 根拠 |
|---------|------|--------|------|
| 標準ケース | #1-5 | 低 | 機械的なファイル移動のみ |
| private テスト | #6 | 低 | ディレクトリモジュール化で子モジュール関係維持 |
| 複数テストファイル | #7-8 | 中 | `tests.rs` + `tests_ex.rs` の共存 |
| 親ディレクトリ参照 | #9 | 中 | ファイル移動 + mod名変更 + import パス確認 |

## 3. Implementation Approach Options

### Option A: ディレクトリモジュール化（全面採用） — **確定**

全対象の `foo.rs` を `foo/mod.rs` に変換し、テストファイルを `foo/tests.rs` として配置。プロジェクト内の既存参考モデル（`bitmap_source/`）と完全に同一のパターン。

### Option B: インラインテスト化 — **棄却**
ファイルサイズ増大（最大1498行）、既存の分離方針と矛盾、Req 1.3 違反。

### Option C: ハイブリッド — **棄却**
一貫性欠如（Req 3.1 違反）、閾値の恣意性、将来の再移行リスク。

### Option D: リネームのみ — **実現不可能**
フラットファイル構成で同一ディレクトリ内の `tests.rs` 衝突が不可避。

## 4. Implementation Complexity & Risk

### Effort: **S（1-3日）**
- 既存パターン踏襲、設計判断は最小限
- 各ファイルの移動とモジュール宣言変更は機械的作業
- テストコードのロジック変更なし
- `graphics_tests.rs` のみ mod 名変更と配置移動が必要

### Risk: **Low**
- `bitmap_source/` で実績あり
- モジュール単位で独立して実行・検証可能
- `cargo test` で即座にリグレッション検出
- `git checkout` で即座にロールバック可能

## 5. 確定済み設計判断

ユーザーとの議論（要件定義フェーズ）で以下3点が確定:

1. **戦略の統一**: 全箇所にディレクトリモジュール化方式を適用（インライン化は不採用）
2. **`graphics/tests.rs` の import パス**: `use crate::` パスを維持（内部サブモジュール構造上 `use super::*` は不適切）
3. **`tests_ex` モジュール名**: 維持（テスト対象 `_ex` 系関数群を反映した意味ある命名）

## 6. Requirement-to-Asset Map

| 要件 | 既存アセット | ギャップ | 状態 |
|------|------------|---------|------|
| Req 1: 慣用的構造 | `bitmap_source/` 参考モデル | フラットファイルのディレクトリ化 | 対応可能 |
| Req 2: カバレッジ維持 | 全テスト（合計 2645 行） | なし（子モジュール関係維持） | 問題なし |
| Req 3: ファイル構造整合 | `bitmap_source/` パターン | `graphics_tests.rs` 移動が最複雑 | 対応可能 |
| Req 4: 影響最小化 | — | なし | 問題なし |
| Req 5: 段階的移行 | — | なし（各モジュール独立） | 問題なし |
