# ギャップ分析: namespace-refactoring

## 1. 現状調査

### 1.1 クレート構成概要

| クレート | src/ ファイル数 | tests/ ファイル数 | 共通ヘルパー | `#[path]` テスト |
|---------|---------------|-----------------|-------------|----------------|
| **dola** | 9 (ルート) + 3 (compile/) + 12 (runtime/) + 2 (validate/) = 26 | 21 + 2モジュール | `compile_common/`, `trigger_common/` | 4件 (runtime内) |
| **wintf** | 7 (ルート) + 60+ (ecs/ 再帰的) + 8 (com/) = 75+ | 41 | なし | 5件 (ecs内) |
| **areka** | 1 (main.rs) | 0 | — | — |

### 1.2 dola テスト現状

**テストファイル 21件** がフラットに `tests/` 直下に配置:

| ドメイン | ファイル数 | 共通モジュール利用 |
|---------|-----------|-----------------|
| compile | 6 | `compile_common` (4/6利用、1件は重複ローカルヘルパー) |
| runtime | 3 | なし |
| trigger | 4 | `trigger_common` (3/4利用) |
| validation | 3 | なし（`minimal_valid_doc()` が3ファイルに重複定義） |
| cross-domain | 5 | なし |

**重複ヘルパー関数**:
- `minimal_valid_doc()` — validation 3ファイルに重複
- `make_doc()` — `compile_integration_test.rs` にローカル定義（`compile_common::make_doc_with_storyboard` と同等）
- `simple_float_doc()` / `loop_doc()` — 類似パターン

### 1.3 wintf テスト現状

**テストファイル 41件** がフラットに `tests/` 直下に配置:

| ドメイン | ファイル数 | `wintf::com::` 直接使用 |
|---------|-----------|----------------------|
| layout | 12 | 0 |
| graphics | 10 | 3 (`com::ulw`, `com::dcomp`) |
| visual | 8 | 4 (`com::dcomp`) |
| widget | 2 | 0 |
| window | 4 | 0 |
| other | 5 | 0 |

**重複ヘルパー関数**:
- `setup_graphics()` — visual 系 5ファイルに重複定義

**モック専用テスト** (wintf 未使用): `component_state_pattern_test`, `lazy_reinit_pattern_test`, `resource_removal_detection_test` は bevy_ecs のみ使用。

**テストアセット**: `tests/assets/` に13ファイル（`bitmap_source_integration_test.rs` のみ参照）

### 1.4 プロダクションコード構造

#### dola

```
src/
├── builder.rs          ← ルート直下（定義系）
├── document.rs         ← ルート直下（定義系）
├── easing.rs           ← ルート直下（定義系）
├── error.rs            ← ルート直下（共通）
├── playback.rs         ← ルート直下（再生制御）
├── storyboard.rs       ← ルート直下（定義系）
├── transition.rs       ← ルート直下（定義系）
├── value.rs            ← ルート直下（定義系）
├── variable.rs         ← ルート直下（定義系）
├── compile/            ← 3ファイル ✓ 適正
│   ├── mod.rs
│   ├── resolve.rs
│   └── types.rs
├── runtime/            ← 12ファイル（+ テスト4件）⚠ 過大
│   ├── mod.rs
│   ├── clock.rs
│   ├── conflict_resolver.rs
│   ├── document_store.rs
│   ├── facade.rs
│   ├── instance_manager.rs (+tests)
│   ├── instance_state.rs
│   ├── interpolator.rs (+tests)
│   ├── loop_controller.rs
│   ├── subscription_manager.rs (+tests)
│   ├── timeline_manager.rs (+tests)
│   └── types.rs
└── validate/           ← 2ファイル ✓ 適正
    ├── mod.rs
    └── rules.rs
```

**分析**: ルート直下9ファイルのうち7ファイルが「宣言的定義」ドメイン（document, storyboard, transition, easing, value, variable, builder）。概念的には同一ドメインだが、Rust の `pub use` で `dola::*` としてフラットにエクスポートする設計意図があるため、サブモジュール化のメリットは限定的。

`runtime/` の12ファイルは論理的に以下のサブグループに分けられる:
- **インスタンス管理**: instance_manager, instance_state, loop_controller
- **補間・タイムライン**: interpolator, timeline_manager
- **競合解決**: conflict_resolver
- **外部インターフェース**: facade, subscription_manager, document_store, clock, types

#### wintf ecs/

```
ecs/
├── mod.rs              ← 大量の pub use（70行超のフラットエクスポート）
├── app.rs              ← ecs直下 → 妥当（アプリ初期化）
├── monitor.rs          ← ecs直下 ⚠ → window/ に移動候補
├── nchittest_cache.rs  ← ecs直下 → pointer/ に移動候補
├── window_system.rs    ← ecs直下 ⚠ → window/ に移動候補
├── common/             ← 3ファイル ✓
├── drag/               ← 6ファイル ✓
├── graphics/           ← 11ファイル + compositor_systems/3 = 14 ✓
├── layout/             ← 12ファイル + systems/4 = 16 ⚠ 大きめ
├── pointer/            ← 6ファイル ✓
├── transform/          ← 2ファイル ✓（非推奨）
├── widget/             ← text/7 + bitmap_source/8 + shapes/2 + mod.rs + brushes.rs = 19 ✓
├── window/             ← 5ファイル ✓
├── window_proc/        ← 6ファイル ✓
└── world/              ← 3ファイル ✓
```

### 1.5 公開 API パス

#### dola lib.rs エクスポート (16 pub use)

- `DolaDocumentBuilder`, `StoryboardBuilder` ← `builder`
- `CompiledSegment`, `CompiledStoryboard`, `CompiledTrigger`, `CompiledVariableTimeline`, `VariableTypeHint`, `compile_storyboard` ← `compile`
- `DolaDocument` ← `document`
- `EasingFunction`, `EasingName`, `ParametricEasing` ← `easing`
- `DolaError` ← `error`
- `PlaybackState`, `ScheduleRequest` ← `playback`
- 多数の storyboard/transition/value/variable 型
- `pub mod runtime` — サブモジュールとして公開

#### wintf ecs/mod.rs エクスポート (70行超)

`pub use` で多数の型をフラットに再エクスポート。`graphics::*`, `layout::*`, `transform::*` は glob 再エクスポート。

#### 外部クレートからのインポートパス

**areka main.rs** が使用するパス:
- `wintf::ecs::drag::*`
- `wintf::ecs::layout::*`
- `wintf::ecs::pointer::*`
- `wintf::ecs::widget::bitmap_source::*`
- `wintf::ecs::widget::brushes::*`
- `wintf::ecs::widget::shapes::*`
- `wintf::ecs::widget::text::*`
- `wintf::ecs::*` (フラットエクスポート)
- `wintf::*`

**examples** が使用するパス: 上記と同等のパターン。

---

## 2. 要件別ギャップ分析

### Req 1: dola テストのサブディレクトリ整理

| 項目 | 状態 | ギャップ |
|------|------|---------|
| ドメイン別分類 | 可能 | ✓ 分類ルールは明確 |
| `mod compile_common;` パス解決 | **問題あり** | サブディレクトリ移動で壊れる |
| Cargo integration test 規約 | **要調査** | サブディレクトリ方式の選定が必要 |
| ヘルパー重複統合 | 未整備 | `validation_common` が必要 |

**最大のギャップ**: Rust の integration test ディレクトリ構造の制約。`tests/` 直下の `.rs` ファイルのみが独立テストバイナリとして認識される。

### Req 2: wintf テストのサブディレクトリ整理

| 項目 | 状態 | ギャップ |
|------|------|---------|
| ドメイン別分類 | 可能 | 分類ルールは明確（一部クロスドメイン） |
| 共有ヘルパー | **未整備** | `setup_graphics()` 等の共通化が必要 |
| アセットパス | **要調整** | `tests/assets/` への相対パス変更 |
| `[[test]]` 設定 | 未定義 | サブディレクトリ方式に応じて必要 |

### Req 3: `#[path]` パターン一貫性

| クレート | 件数 | 命名 | 状態 |
|---------|------|------|------|
| dola runtime/ | 4 | `{module}_tests.rs` | ✓ 一貫 |
| wintf layout/ | 3 | `hit_region_tests.rs`, `hit_test_tests.rs`, `hit_test_ex_tests.rs` | ⚠ `hit_test_ex_tests.rs` が例外的 |
| wintf pointer/ | 1 | `dispatch_tests.rs` | ✓ |
| wintf graphics/ | 1 | `graphics_tests.rs` (パス `../graphics_tests.rs`) | ⚠ 親ディレクトリ参照 |

**ギャップ**: `graphics_tests.rs` のみ `#[path = "../graphics_tests.rs"]` と親ディレクトリを参照しており、他のパターンと不一致。

### Req 4: dola プロダクションコード検証

| 領域 | 状態 | 判定 |
|------|------|------|
| ルート直下 9ファイル | 7ファイルが定義ドメイン | ⚠ サブモジュール化は利点薄い（`pub use` フラットエクスポート設計） |
| compile/ (3ファイル) | 適正 | ✓ |
| runtime/ (12ファイル) | 過大だが論理的まとまりあり | ⚠ 分割は可能だがリスク増 |
| validate/ (2ファイル) | 適正 | ✓ |

### Req 5: wintf ecs/ 検証

| 領域 | 状態 | 判定 |
|------|------|------|
| `monitor.rs` | ecs直下 | ⚠ `window/` に移動候補 |
| `window_system.rs` | ecs直下 | ⚠ `window/` に移動候補 |
| `nchittest_cache.rs` | ecs直下 | ⚠ `pointer/` に移動候補 |
| `app.rs` | ecs直下 | ✓ 妥当（アプリ初期化は全体横断） |
| `graphics_tests.rs` | ecs直下 | ⚠ `graphics/` 内に移動すべき |
| 各サブモジュール | 構造化済み | ✓ ドメイン対応は概ね良好 |

### Req 6: 公開 API 整合性

| 項目 | リスク | 対策 |
|------|-------|------|
| dola `pub use` | 低 | ルート直下ファイルを移動しなければ影響なし |
| wintf `ecs/mod.rs` glob re-export | 中 | `monitor.rs` 等の移動時に `pub use` 更新が必要 |
| areka main.rs インポート | 中 | `wintf::ecs::*` 経由なら影響最小、サブモジュール直接参照は要更新 |
| examples インポート | 中 | 同上 |

### Req 7: テスト命名規約

| パターン | 現状 | ギャップ |
|---------|------|---------|
| integration test | `{対象}_test.rs` | ✓ ほぼ統一 |
| unit test (`#[path]`) | `{module}_tests.rs` | ✓ 概ね統一 |
| structure.md | 命名規約未記載 | **Missing** — テスト命名規約セクションの追記が必要 |

---

## 3. 実装アプローチ選択肢

### テストサブディレクトリ化の方式

#### Option A: エントリポイント方式（`tests/compile.rs` + `tests/compile/`）

```
tests/
├── compile.rs          ← エントリポイント: mod error_test; mod integration_test; ...
├── compile/
│   ├── error_test.rs
│   ├── integration_test.rs
│   ├── common/mod.rs
│   └── ...
├── runtime.rs
├── runtime/
│   └── ...
├── builder_test.rs     ← cross-domain はルート直下に残す
└── ...
```

**トレードオフ**:
- ✅ Cargo 標準の仕組みで動作（`[[test]]` 不要）
- ✅ 共通モジュールのパス解決が自然（`mod common;` がサブディレクトリから解決される）
- ❌ ドメイン内の全テストが単一バイナリにまとまり、並列コンパイルの粒度が粗くなる
- ❌ テスト失敗時の出力がドメイン単位になり、個別テストの特定がやや手間

#### Option B: `[[test]]` 明示定義方式

```toml
# Cargo.toml
[[test]]
name = "compile_error_test"
path = "tests/compile/error_test.rs"

[[test]]
name = "compile_integration_test"
path = "tests/compile/integration_test.rs"
# ... 各テストファイルを個別定義
```

**トレードオフ**:
- ✅ 各テストが独立バイナリを維持（並列性維持）
- ✅ ファイル命名の自由度が高い
- ❌ Cargo.toml が肥大化（dola 21行 + wintf 41行）
- ❌ テストファイル追加のたびに `[[test]]` を追加する運用コスト
- ❌ 共通モジュール参照に `#[path]` が必要になりうる

#### Option C: ハイブリッド方式（推奨候補）

ドメイン内テスト数が多い場合は Option A（エントリポイント方式）、少ない場合はルート直下に残す。

```
tests/
├── compile.rs          ← 6テスト → エントリポイント方式
├── compile/
│   ├── common/mod.rs
│   ├── error_test.rs
│   └── ...
├── trigger.rs          ← 4テスト → エントリポイント方式
├── trigger/
│   ├── common/mod.rs
│   └── ...
├── validation.rs       ← 3テスト → エントリポイント方式
├── validation/
│   ├── common/mod.rs
│   └── ...
├── runtime.rs          ← 3テスト → エントリポイント方式
├── runtime/
│   └── ...
├── builder_test.rs     ← ルート直下に残す
├── core_types_test.rs
├── integration_test.rs
├── loop_integration_test.rs
└── loop_offset_test.rs
```

**トレードオフ**:
- ✅ 実用的なバランス
- ✅ 共通ヘルパーの自然な配置
- ✅ Cargo.toml 変更不要
- ❌ ドメイン内テストは単一バイナリになる（コンパイル粒度はドメイン単位）
- ❌ Option A と同じ並列性の制限

### プロダクションコードの方式

#### Option A: 現状維持 + 最小限の移動

- `ecs/monitor.rs` → `ecs/window/monitor.rs`
- `ecs/window_system.rs` → `ecs/window/window_system.rs`
- `ecs/nchittest_cache.rs` → `ecs/pointer/nchittest_cache.rs`
- `ecs/graphics_tests.rs` → `ecs/graphics/graphics_tests.rs`
- dola ルート直下は変更なし
- dola runtime/ は分割しない

**トレードオフ**:
- ✅ 最小限のリスク
- ✅ `pub use` 更新箇所が少ない
- ✅ 外部 API への影響最小
- ❌ runtime/ の12ファイル問題は解決しない

#### Option B: 積極的なリファクタリング

上記 Option A に加え:
- dola ルート直下の定義系7ファイルを `definition/` サブモジュールへ
- dola `runtime/` をサブグループに分割

**トレードオフ**:
- ✅ より論理的な構造
- ❌ `pub use` の大幅な書き換え
- ❌ 外部クレート（areka）への波及
- ❌ リスクと工数が大幅に増加

#### Option C: 段階的リファクタリング（推奨候補）

Phase 1: テストのサブディレクトリ化のみ実行
Phase 2: wintf ecs/ の3ファイル移動（monitor, window_system, nchittest_cache）
Phase 3: （オプション）dola runtime/ の分割検討

**トレードオフ**:
- ✅ 段階的にリスクを管理
- ✅ Phase 1 完了時点で最大の成果（テスト62ファイルの整理）
- ❌ 計画・追跡のオーバーヘッド

---

## 4. 複雑性・リスク評価

| 要件 | 工数 | リスク | 根拠 |
|------|------|-------|------|
| Req 1: dola テスト整理 | **S** (1-2日) | **低** | 既存パターンの移動、API変更なし |
| Req 2: wintf テスト整理 | **M** (3-5日) | **低** | 41ファイルの移動、アセットパス調整 |
| Req 3: `#[path]` 一貫性 | **S** (0.5日) | **低** | 命名確認と最小限のリネーム |
| Req 4: dola プロダクション | **S** (0.5日) | **低** | 検証のみ（変更は最小限推奨） |
| Req 5: wintf プロダクション | **S-M** (1-2日) | **中** | 3ファイル移動 + `pub use` 更新 + 外部参照修正 |
| Req 6: 公開API整合性 | — (他タスクに包含) | **中** | Req 5 の移動に伴う影響管理 |
| Req 7: テスト命名規約 | **S** (0.5日) | **低** | ドキュメント追記と最小限のリネーム |

**全体工数**: **M** (5-8日)
**全体リスク**: **中** — テスト移動自体は低リスクだが、プロダクションコード移動と pub use 更新に注意が必要

---

## 5. 設計フェーズへの推奨事項

### 推奨アプローチ

1. **テスト整理**: Option C (ハイブリッド方式) — エントリポイント方式を基本とし、少数ドメインはルート直下に残す
2. **プロダクションコード**: Option C (段階的) — まずテスト整理を完了し、その後 ecs/ ルート直下ファイルの移動を検討

### Research Needed（設計フェーズで要調査）

1. **エントリポイント方式のコンパイル時間影響**: ドメイン内テストが単一バイナリにまとまることで `cargo test` の所要時間がどう変化するか
2. **`graphics_tests.rs` の `#[path = "../graphics_tests.rs"]` 解消方法**: ファイル移動 vs パス更新のどちらが適切か
3. **wintf `pub use` の glob re-export 整理**: `pub use graphics::*` のような glob をより明示的な再エクスポートに変更すべきか
4. **モック専用テスト（3件）の扱い**: wintf テストに含めるべきか、別の場所に移すべきか

### 優先実装順序（推奨）

1. dola テスト整理 (Req 1) — 最もファイル数/複雑度比が良い
2. wintf テスト整理 (Req 2) — 最大の成果
3. テスト命名規約 (Req 7) — ドキュメント整備
4. `#[path]` 一貫性 (Req 3) — 小規模修正
5. wintf ecs/ プロダクション (Req 5) — 依存する外部参照の更新が必要
6. dola プロダクション (Req 4) — 検証のみ（変更最小限）
