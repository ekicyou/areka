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
| 6 | wintf | `ecs/layout/hit_region.rs` | `hit_region_tests.rs` | `tests` | プライベート関数テスト |
| 7 | wintf | `ecs/layout/hit_test.rs` | `hit_test_tests.rs` | `tests` | 2ファイル構成 |
| 8 | wintf | `ecs/layout/hit_test.rs` | `hit_test_ex_tests.rs` | `tests_ex` | 上記の2つ目 |
| 9 | wintf | `ecs/graphics/mod.rs` | `../graphics_tests.rs` | `graphics_tests` | 親Dir参照・非標準mod名 |

#### 既存ディレクトリ構造の特徴

- **dola `runtime/`**: 完全にフラット（サブディレクトリなし）。16ファイルが同一ディレクトリに共存。
- **wintf `layout/`**: `systems/` サブディレクトリを持つが、他はフラット。
- **wintf `pointer/`**: フラット。
- **wintf `graphics/`**: 既にディレクトリモジュール化済み（`mod.rs` + サブモジュール群）。`compositor_systems/`, `systems/` サブディレクトリあり。

#### 既存の慣用パターン（参考モデル）

**`bitmap_source/mod.rs`** が `#[path]` なしの理想形:
```
bitmap_source/
├── mod.rs           # #[cfg(test)] mod tests;
├── tests.rs         # use super::*;
├── alpha_mask.rs
├── bitmap_source.rs
└── ...
```
- ディレクトリモジュール方式のため、`mod tests;` が自然に `tests.rs` を解決
- テストファイルは `use super::*;` パターンで親アクセス

**インラインテスト** も12箇所で使用中（`types.rs`, `loop_controller.rs` 等）。テスト規模が小さい場合に選択される傾向。

### コンベンション

| 項目 | 現状パターン |
|------|-------------|
| テストインポート | `use super::*;` が主流（8/9件）、`graphics_tests.rs` のみ `use crate::` |
| テストmod名 | `tests` が主流（7/9件）、`tests_ex`, `graphics_tests` が例外 |
| 可視性 | `pub(crate)` が多用（dola crate）、`pub` も混在（wintf crate） |
| テストファイル命名 | `{source_name}_tests.rs` パターン |

### 統合サーフェス

- **親モジュール宣言** (`mod.rs`): `mod instance_manager;` 等の宣言はファイル→ディレクトリ変更時も修正不要（Rust のモジュール解決が吸収）
- **`#[cfg(test)]` ガード**: テストモジュール宣言は既に全箇所で `#[cfg(test)]` 付き
- **git 履歴**: ファイル移動は `git mv` で追跡可能だが、`foo.rs` → `foo/mod.rs` は実質的にリネーム

## 2. Requirements Feasibility Analysis

### 要件→技術ニーズのマッピング

| 要件 | 技術ニーズ | ギャップ |
|------|-----------|---------|
| Req 1: 慣用的構造への移行 | `#[path]` を完全除去し、Rust 標準モジュール解決に切り替え | **構造変更が必要**: フラットファイルからディレクトリモジュールへの変換 |
| Req 2: テストカバレッジ維持 | `use super::*;` やプライベートアクセスの維持 | **問題なし**: ディレクトリモジュール化しても子モジュール関係は維持 |
| Req 3: ファイル構造の整合 | 統一パターンの選択 | **選択が必要**: オプション間のトレードオフあり |
| Req 4: プロダクション影響ゼロ | 可視性・ロジック変更なし | **問題なし**: 全オプションで実現可能 |
| Req 5: 段階的移行 | モジュール単位の独立実行 | **問題なし**: 各モジュールは独立してリファクタ可能 |

### プライベート/`pub(crate)` アクセスの詳細分析

| ファイル | アクセスが必要なアイテム | 可視性 | ディレクトリモジュール化後 |
|---------|----------------------|--------|------------------------|
| `hit_region.rs` | `point_in_polygon` | **private** | ✅ 子モジュールなのでアクセス可（Rust の可視性規則） |
| `instance_manager.rs` | `InstanceManager`, `StoryboardInstance` | `pub(crate)` | ✅ 同一クレート内の子モジュールなのでアクセス可 |
| `subscription_manager.rs` | `SubscriptionManager` | `pub(crate)` | ✅ 同上 |
| `timeline_manager.rs` | `TimelineManager` 等 | `pub(crate)` | ✅ 同上 |
| `interpolator.rs` | `ObjectInternPool` | `pub(crate)` | ✅ 同上 |
| `hit_test.rs` | `RegionHit`, `hit_test_entity_ex` | `pub(crate)` | ✅ 同上 |

**結論**: ディレクトリモジュール化により `mod tests` が子モジュールとして維持されるため、**可視性の変更は一切不要**。

### 複雑性シグナル

- **標準ケース** (5件: #1-5): ファイル移動のみ。機械的な作業。
- **プライベート関数テスト** (#6): ディレクトリモジュール化なら問題なし。
- **複数テストファイル** (#7-8): ディレクトリモジュール化で自然に解決。
- **親ディレクトリ参照** (#9): テストファイルの移動 + import 修正が必要。最も手間がかかるケース。

## 3. Implementation Approach Options

### Option A: ディレクトリモジュール化（全面採用）

**概要**: 全対象の `foo.rs` を `foo/mod.rs` に変換し、テストファイルを `foo/tests.rs` として配置。

**具体的な変更**:
```
# Before (instance_manager)
runtime/
├── instance_manager.rs
├── instance_manager_tests.rs

# After
runtime/
├── instance_manager/
│   ├── mod.rs        # 元の instance_manager.rs の内容
│   └── tests.rs      # 元の instance_manager_tests.rs の内容
```

```rust
// instance_manager/mod.rs（末尾）
#[cfg(test)]
mod tests;  // → instance_manager/tests.rs を自動解決
```

**hit_test.rs の特殊ケース**:
```
layout/
├── hit_test/
│   ├── mod.rs
│   ├── tests.rs       # 元 hit_test_tests.rs
│   └── tests_ex.rs    # 元 hit_test_ex_tests.rs
```

**graphics/mod.rs の特殊ケース**:
```
# Before
ecs/
├── graphics/
│   └── mod.rs
├── graphics_tests.rs   # 親ディレクトリに配置

# After
ecs/
├── graphics/
│   ├── mod.rs
│   └── tests.rs        # graphics/ 内に移動
```
- `graphics_tests.rs` を `graphics/tests.rs` に移動
- モジュール名を `graphics_tests` → `tests` に変更
- 内部の `use crate::ecs::graphics::*` は維持可能（or `use super::*` に変更）

**Trade-offs**:
- ✅ **`bitmap_source/` と完全に同一のパターン**（プロジェクト内の既存参考モデルに合致）
- ✅ `#[path]` を完全除去
- ✅ 可視性変更不要（子モジュール関係維持）
- ✅ 複数テストファイルも自然に対応
- ❌ **ディレクトリ数が 7 増加**（`instance_manager/`, `interpolator/`, `subscription_manager/`, `timeline_manager/`, `dispatch/`, `hit_region/`, `hit_test/`）
- ❌ `runtime/mod.rs` の宣言は変更不要だが、git diff が大きくなる
- ❌ フラットだった `runtime/` のディレクトリ構造が複雑化

### Option B: インラインテスト化（全面採用）

**概要**: 全テストコードをソースファイル末尾に `mod tests { ... }` としてインライン化。

```rust
// instance_manager.rs（末尾）
#[cfg(test)]
mod tests {
    use super::*;
    // ... 188行のテストコード ...
}
```

**Trade-offs**:
- ✅ **最もシンプル**、ファイル移動なし
- ✅ 追加ディレクトリなし
- ✅ `#[path]` 完全除去
- ✅ 1ファイル完結（ソースとテストの距離が最小）
- ❌ **ファイルサイズが大幅増加**: `hit_region.rs` 504行 + 554行 = 1058行、`hit_test.rs` 575行 + 330行 + 593行 = 1498行
- ❌ 元々「ファイルサイズを抑えるため」に分離された経緯と矛盾
- ❌ コードナビゲーションの悪化（プロダクションコードとテストコードの境界が不明瞭）
- ❌ `graphics_tests.rs` のサブモジュール構造（3つのネストされた `mod`）のインライン化は可読性を著しく低下

### Option C: ハイブリッド（規模ベース選択）

**概要**: テスト規模に応じてインラインまたはディレクトリモジュール化を選択。

**閾値例**: テスト 200 行未満 → インライン、200 行以上 → ディレクトリモジュール化

| # | テスト行数 | 方式 |
|---|----------|------|
| 1 | 188 | インライン |
| 2 | 209 | ディレクトリ |
| 3 | 284 | ディレクトリ |
| 4 | 181 | インライン |
| 5 | 163 | インライン |
| 6 | 554 | ディレクトリ |
| 7+8 | 330+593 | ディレクトリ |
| 9 | 143 | インライン |

**Trade-offs**:
- ✅ 各ケースに最適な方式を選択可能
- ✅ 小規模テストはシンプルに、大規模テストは分離を維持
- ❌ **一貫性がない**（Req 3.1「同一の分離戦略」に違反）
- ❌ 将来テストが増えた時にインライン→ディレクトリへの再移行リスク
- ❌ 判断基準が曖昧（何行から「大きい」のか）

### Option D: テストファイルリネーム（`tests.rs` 命名規約活用）

**概要**: `foo_tests.rs` → `foo/tests.rs` への変更なしに、Rust 2024 のモジュール解決で直接対応できるパターンを模索。

**検討結果**: **実現不可能**。

理由:
- フラットファイル `foo.rs` から `mod tests;` とした場合、Rust は `foo/tests.rs` を探す（`foo` がディレクトリモジュールの場合のみ）
- `foo.rs`（単一ファイルモジュール）から `mod tests;` とした場合、同一ディレクトリの `tests.rs` を探すが、これは **ディレクトリに1つしか置けない**
- 複数モジュールが同じディレクトリで `mod tests;` を使うと `tests.rs` が衝突する

→ **Option D は棄却**。ディレクトリモジュール化が不可避。

## 4. Research Needed（設計フェーズへ持ち越し）

1. **`git mv` の追跡**: `foo.rs` → `foo/mod.rs` のリネームが git の変更追跡でどう扱われるか（similarity threshold の確認）
2. **rust-analyzer 対応**: ディレクトリモジュール化後の IDE サポートの挙動確認
3. **`graphics_tests.rs` のリファクタ詳細**: 3つのネストされたサブモジュールの最適な再配置

## 5. Implementation Complexity & Risk

### Effort: **S（1-3日）**

- 既存パターン（`bitmap_source/`）を踏襲するため、設計判断は最小限
- 各ファイルの移動とモジュール宣言変更は機械的な作業
- テストコード自体の変更は不要（`use super::*;` は維持される）
- `graphics_tests.rs` のみ追加の import 修正が必要

### Risk: **Low**

- 使い慣れたパターンの適用（`bitmap_source/` で実績あり）
- テストコードのロジック変更なし
- ファイル移動はモジュール単位で独立して実行可能
- `cargo test` で即座に検証可能
- 失敗時は `git checkout` で即座にロールバック可能

## 6. Recommendations

### 推奨アプローチ: **Option A（ディレクトリモジュール化）— 確定**

**理由**:
1. プロジェクト内に既存の参考モデル（`bitmap_source/`）がある
2. 全要件（Req 1-5）を満たす唯一のオプション
3. 一貫した構造パターンを提供（Req 3.1）
4. 可視性変更が不要（Req 4.2）
5. 段階的移行が可能（Req 5）

**設計フェーズでの主要決定事項**:
1. `graphics_tests.rs` 内の3つのサブモジュールをどう再配置するか
2. `hit_test.rs` の `tests_ex` モジュールの命名を維持するか標準化するか
3. 移行順序の確定（難易度順）
4. `graphics_tests.rs` の `use crate::` パスを `use super::*` に統一するかどうか

### Requirement-to-Asset Map

| 要件 | 既存アセット | ギャップ | 状態 |
|------|------------|---------|------|
| Req 1: 慣用的構造 | `bitmap_source/` 参考モデル | フラットファイルのディレクトリ化が必要 | **対応可能** |
| Req 2: カバレッジ維持 | 全テストファイル（合計 2645 行） | なし（子モジュール関係維持） | **問題なし** |
| Req 3: ファイル構造整合 | `bitmap_source/` パターン | `graphics_tests.rs` の移動が最も複雑 | **対応可能** |
| Req 4: 影響最小化 | — | なし | **問題なし** |
| Req 5: 段階的移行 | — | なし（各モジュール独立） | **問題なし** |
