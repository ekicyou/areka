# ギャップ分析: wintf-dcomp-migration-4-dcomp-removal

## 概要

本分析は、Phase 4「DComp コード削除・クリーンアップ」の要件と既存コードベースの現状を調査し、削除対象・影響範囲・実装戦略を評価する。

Phase 4 は Phase 1〜3 完了後の「純粋な削除作業」であり、新規機能の実装は不要。ただし、削除対象が複数ファイル・複数レイヤーに分散しているため、削除順序の計画と参照チェーンの追跡が重要となる。

---

## 1. 現状調査

### 1.1 削除対象ファイル一覧

| ファイル | 行数 | 削除方針 | 要件 |
|----------|------|----------|------|
| `com/dcomp.rs` | ~280行 | **全削除** | Req 1 |
| `ecs/graphics/visual_manager.rs` | ~129行 | **全削除** | Req 4 |
| `examples/dcomp_demo.rs` | ~583行 | **全削除** | Req 5 |
| `ecs/graphics/components.rs` | ~315行 | **部分削除** (VisualGraphics, SurfaceGraphics) | Req 2 |
| `ecs/graphics/systems.rs` | ~1303行 | **部分削除** (DComp系システム関数) | Req 3 |
| `ecs/graphics/core.rs` | ~189行 | **部分削除** (DCompフィールド・初期化) | Req 6 |
| `ecs/world.rs` | ~668行 | **部分修正** (スケジュール登録・コメント) | Req 3, 7 |
| `com/animation.rs` | ~183行 | **微修正** (IDCompositionAnimation参照) | Req 7 |
| `ecs/graphics_tests.rs` | ~103行 | **部分削除** (DCompテスト) | Req 8 |
| `com/mod.rs` | ~7行 | **1行削除** (`pub mod dcomp;`) | Req 1 |
| `ecs/graphics/mod.rs` | ~17行 | **1行削除** (`pub mod visual_manager;`) | Req 4 |

### 1.2 テスト影響ファイル

| テストファイル | 行数 | DComp依存 | 方針 |
|----------------|------|-----------|------|
| `tests/visual_hierarchy_sync_test.rs` | ~177行 | **全面依存** — `wintf::com::dcomp::*`, `VisualGraphics`, `visual_hierarchy_sync_system`, `GraphicsCore::dcomp()` を使用 | 全削除 |
| `tests/visual_graphics_auto_creation_test.rs` | ~131行 | **部分依存** — `VisualGraphics`, `SurfaceGraphics`, `visual_resource_management_system` を使用 | 全削除または大幅書き換え |
| `src/ecs/graphics_tests.rs` | ~103行 | **部分依存** — `test_create_visual`(L32), `test_create_multiple_visuals`(L65), `test_commit`(L80) が `dcomp()` API を使用 | 3テスト削除、他は保持 |

### 1.3 IDComposition 参照の詳細分布

総計: **69箇所** (`crates/wintf/src/` 配下)

| ファイル | 参照数 | 内容 |
|----------|--------|------|
| `com/dcomp.rs` | ~35 | DComp 拡張 trait 定義本体（ファイル全削除で解消） |
| `ecs/graphics/components.rs` | ~20 | `WindowGraphicsInner.target`, `VisualGraphics.inner/parent_visual`, `SurfaceGraphics.inner` |
| `ecs/graphics/core.rs` | ~5 | `GraphicsCoreInner.desktop/dcomp`, `dcomp()`/`desktop()` アクセサ |
| `ecs/graphics/systems.rs` | ~5 | `create_surface_for_visual()`, コメント内参照 |
| `ecs/graphics/visual_manager.rs` | ~1 | `create_visual_only()` の `dcomp` パラメータ |
| `ecs/world.rs` | ~4 | スケジュールラベルのドキュメントコメント内 |
| `ecs/graphics_tests.rs` | ~3 | テスト出力メッセージ内 |
| `com/animation.rs` | ~2 | `UIAnimationVariableExt::get_curve()` の `IDCompositionAnimation` パラメータ |

### 1.4 Cargo.toml / メタデータ影響

| 箇所 | 内容 | 方針 |
|------|------|------|
| `Cargo.toml` (workspace root) L57 | `"Win32_Graphics_DirectComposition"` feature | **削除候補** — animation.rs の `IDCompositionAnimation` が使用する場合は保持要 |
| `crates/wintf/Cargo.toml` L10 | `keywords = ["directcomposition", ...]` | **更新** — `"directcomposition"` を除去し `"layered-window"` 等に変更 |

---

## 2. 要件–資産マッピングと識別されたギャップ

### Req 1: com/dcomp.rs モジュール完全削除

| 項目 | 状態 | 詳細 |
|------|------|------|
| ファイル削除 | ✅ 実行可能 | `com/dcomp.rs` (~280行) を `rm` で削除 |
| mod宣言除去 | ✅ 実行可能 | `com/mod.rs` L4: `pub mod dcomp;` を除去 |
| use文波及 | ⚠️ 要追跡 | `systems.rs` L3-5, `visual_manager.rs` L1, `core.rs` L3, `graphics_tests.rs` が `use crate::com::dcomp::*` をインポート |
| **ギャップ: animation.rs の IDCompositionAnimation** | 🔍 Research Needed | `com/animation.rs` L166,178 で `IDCompositionAnimation` を使用。これは `IDCompositionDevice3::CreateAnimation()` で作成される DComp 型。Phase 1 の新アニメーション方式で代替されるか要確認 |

### Req 2: DComp ECS コンポーネント削除

| 項目 | 状態 | 詳細 |
|------|------|------|
| VisualGraphics 定義削除 | ✅ 実行可能 | `components.rs` L80-167 (~87行) |
| SurfaceGraphics 定義削除 | ✅ 実行可能 | `components.rs` L169-228 (~59行) |
| on_visual_graphics_remove フック | ✅ 実行可能 | `components.rs` L92-99 — `parent.remove_visual(visual)` DComp依存 |
| Visual.on_add フック修正 | ⚠️ 要注意 | `components.rs` L265-307 — `VisualGraphics::default()`, `SurfaceGraphics::default()`, `SurfaceGraphicsDirty::default()` を自動挿入。Phase 1 の新コンポーネントに差し替え済みか確認要 |
| SurfaceGraphicsDirty | ⚠️ 条件付き | `components.rs` L231-250 — DComp 固有ではない汎用ダーティマーカー。Phase 1 で再利用されている可能性あり |
| SurfaceCreationStats | ⚠️ 条件付き | `components.rs` L340-364 — `Resource` 型の統計リソース。DComp 非依存だが DComp Surface 用途。Phase 1 で再利用されるか確認要 |
| WindowGraphics の IDCompositionTarget | ✅ 実行可能 | `components.rs` L28 — `target: IDCompositionTarget` フィールド。Phase 1 で合成ビットマップに置換済みのはず |

### Req 3: DComp ECS システム関数削除

| 関数 | 行範囲 | 状態 | 詳細 |
|------|--------|------|------|
| `visual_resource_management_system` | visual_manager.rs L76-115 | ✅ | ファイル全削除で解消 |
| `visual_hierarchy_sync_system` | systems.rs L881-1003 | ✅ | ~122行。DComp Visual 親子同期 |
| `init_window_graphics` | systems.rs L447-538 | ✅ | ~91行。DComp Target/DC 作成 |
| `window_visual_integration_system` | visual_manager.rs L117-140 | ✅ | ファイル全削除で解消。`SetRoot(visual)` DComp依存 |
| `deferred_surface_creation_system` | systems.rs L1116-1272 | ✅ | ~156行。DComp Surface 遅延作成 |
| `cleanup_surface_on_commandlist_removed` | systems.rs L1274-1327 | ✅ | ~53行。Surface クリーンアップ |
| `render_surface` | systems.rs L197-310 | ✅ | ~113行。DComp Surface BeginDraw/EndDraw |
| `visual_property_sync_system` | systems.rs L1005-1114 | ✅ | ~109行。SetOffset/SetOpacity DComp依存 |
| `commit_composition` | systems.rs L312-364 | ✅ | ~52行。`IDCompositionDevice3::Commit()` |
| **ヘルパー関数** | | | |
| `create_window_graphics_for_hwnd` | systems.rs L74-98 | ✅ | `desktop.create_target_for_hwnd()` DComp依存 |
| `create_surface_for_visual` | systems.rs L100-128 | ✅ | `dcomp.create_surface()` DComp依存 |
| `draw_recursive` | systems.rs L130-195 | ✅ | `#[allow(dead_code)]` 旧描画方式、既にデッドコード |
| `init_window_visual` | systems.rs L539-566 | ✅ | Deprecated、空実装 |
| `sync_surface_from_arrangement` | systems.rs L573-700 | ✅ | `#[deprecated]` `#[allow(dead_code)]` |

**保持すべき関数** (DComp 非依存):
- `format_entity_name` (L30-37)
- `calculate_surface_size_from_global_arrangement` (L50-72)
- `init_graphics_core` (L366-445)
- `apply_window_pos_changes` (L702-792)
- `invalidate_dependent_components` (L794-833)
- `mark_dirty_surfaces` (L835-879)
- `resolve_inherited_brushes` (L1329-1369)
- `find_parent_brushes` (L1371-1393)
- `resolve_brush_fields` (L1395-1419)

### Req 4: visual_manager.rs モジュール完全削除

| 項目 | 状態 | 詳細 |
|------|------|------|
| ファイル削除 | ✅ 実行可能 | `visual_manager.rs` (~129行) |
| mod宣言除去 | ✅ 実行可能 | `ecs/graphics/mod.rs` L5: `pub mod visual_manager;` |
| re-export除去 | ✅ 実行可能 | `ecs/graphics/mod.rs` L11: `pub use visual_manager::*;` |
| **注意点** | ⚠️ | `insert_visual()` / `insert_visual_with()` がウィジェット層から使用されていないか要確認 |

### Req 5: dcomp_demo.rs サンプル削除

| 項目 | 状態 | 詳細 |
|------|------|------|
| ファイル削除 | ✅ 実行可能 | `examples/dcomp_demo.rs` (~583行) |
| Cargo.toml | ✅ 確認済み | 明示的な `[[example]]` エントリなし（自動検出方式）— ファイル削除のみで十分 |

### Req 6: GraphicsCore の DComp フィールド除去

| 項目 | 状態 | 詳細 |
|------|------|------|
| フィールド削除 | ✅ 実行可能 | `core.rs` L24-25: `desktop: IDCompositionDesktopDevice`, `dcomp: IDCompositionDevice3` |
| 初期化コード除去 | ✅ 実行可能 | `core.rs` L50-51: `dcomp_create_desktop_device()`, `desktop.cast()` |
| アクセサ除去 | ✅ 実行可能 | `core.rs` L85-92: `dcomp()`, `desktop()` メソッド |
| use文除去 | ✅ 実行可能 | `core.rs` L3: `use crate::com::dcomp::*;`, L12: `use windows::...::DirectComposition::*;` |
| invalidate() | ✅ 確認済み | `inner = None` 方式のため DComp 固有コードなし |

### Req 7: use 文・参照の網羅的クリーンアップ

| 項目 | 状態 | 詳細 |
|------|------|------|
| ecs/graphics/ IDComposition参照 | ✅ | Req 1-6 の削除により自動解消 |
| com::dcomp 参照 | ✅ | `dcomp.rs` 削除 + use文除去で解消 |
| **com/animation.rs IDCompositionAnimation** | 🔍 Research Needed | `UIAnimationVariableExt::get_curve()` の `P0: Param<IDCompositionAnimation>` パラメータ。Windows UI Animation API が DComp Animation オブジェクトを受け取るインターフェース。Phase 1 で使用しない場合は trait メソッドごと削除可能。使用する場合は `windows` クレートの `Win32_Graphics_DirectComposition` feature が依存的に必要 |
| world.rs コメント | ✅ 実行可能 | L141, 147, 156, 164 のドキュメントコメントを新パイプラインの記述に更新 |
| Cargo.toml feature | ⚠️ 条件付き | `Win32_Graphics_DirectComposition` — animation.rs 依存次第 |
| Cargo.toml keywords | ✅ 実行可能 | `"directcomposition"` を除去 |

### Req 8: テストコードの修正

| テスト | 方針 | 詳細 |
|--------|------|------|
| `graphics_tests.rs::test_create_visual` | **削除** | `GraphicsCore::dcomp()` + `DCompositionDeviceExt::create_visual()` テスト。DComp 固有 |
| `graphics_tests.rs::test_create_multiple_visuals` | **削除** | 同上 |
| `graphics_tests.rs::test_commit` | **削除** | `dcomp.commit()` テスト。DComp 固有 |
| `graphics_tests.rs::test_graphics_core_creation` | **修正** | `GraphicsCore::new()` テスト自体は保持。ただし DComp フィールド初期化が除去されるため成功条件が変わる |
| `graphics_tests.rs::test_create_device_context` | **保持** | D2D DeviceContext テスト。DComp 非依存 |
| `tests/visual_hierarchy_sync_test.rs` | **全削除** | 全4テスト（~177行）が DComp Visual 階層同期テスト |
| `tests/visual_graphics_auto_creation_test.rs` | **全削除または大幅書き換え** | `VisualGraphics` / `SurfaceGraphics` / `visual_resource_management_system` を使用。Phase 1 の新コンポーネントに対応するテストが別途必要 |

### Req 9: Phase 4 最終検証基準

| 検証項目 | 状態 | 備考 |
|----------|------|------|
| `grep -r "IDComposition"` ゼロ | ⚠️ | animation.rs 依存次第。animation.rs の `IDCompositionAnimation` が残る場合はゼロにならない |
| `grep -r "dcomp"` ゼロ | ⚠️ | コメント許容だが `com/animation.rs` L2 の `use windows::...::DirectComposition::*` が残る可能性 |
| `cargo build` 成功 | ✅ | 削除順序を正しく計画すれば達成可能 |
| `cargo test` パス | ✅ | テスト修正/削除で達成可能 |
| `cargo build --examples` 成功 | ✅ | `dcomp_demo.rs` 削除のみ |
| `cargo clippy` ゼロ | ✅ | 未使用 import 等の警告が発生しうるが修正可能 |

---

## 3. 実装アプローチオプション

### Option A: ボトムアップ削除（COM層 → ECS層 → テスト）

**方針**: 依存の根元（`com/dcomp.rs`）から削除し、コンパイルエラーを手がかりに参照を網羅的に除去する。

**手順**:
1. `com/dcomp.rs` 削除 + `com/mod.rs` 修正
2. コンパイルエラーをたどり `use crate::com::dcomp::*` を全除去
3. `VisualGraphics`, `SurfaceGraphics` 削除（コンパイルエラーでuse/参照を検出）
4. `visual_manager.rs` 削除
5. `core.rs` の DComp フィールド除去
6. `systems.rs` の DComp システム関数削除
7. `world.rs` のスケジュール登録除去
8. テスト修正
9. `dcomp_demo.rs` 削除
10. `Cargo.toml` 更新

**トレードオフ**:
- ✅ コンパイラが参照チェーンを教えてくれるため漏れにくい
- ✅ 各ステップで `cargo check` で進捗確認可能
- ❌ 中間状態でコンパイルが通らないため、複数ステップをアトミックに行う必要あり
- ❌ COM層削除が最初なので、コンパイルエラーが大量に発生して見通しが悪い

### Option B: トップダウン削除（world.rs → systems → components → COM）

**方針**: スケジュール登録を先に外し、デッドコード化してから削除する。

**手順**:
1. `world.rs` から DComp システム関数のスケジュール登録を除去
2. DComp スケジュールラベル（`PreRenderSurface`, `RenderSurface`, `Composition`, `CommitComposition`）を評価 — Phase 1 で再利用されている場合はラベルは保持
3. `systems.rs` の DComp システム関数を削除
4. `visual_manager.rs` 全削除
5. `VisualGraphics`, `SurfaceGraphics` 削除
6. `core.rs` の DComp フィールド除去
7. `com/dcomp.rs` 削除
8. テスト修正
9. `dcomp_demo.rs` 削除
10. `Cargo.toml` 更新

**トレードオフ**:
- ✅ スケジュール登録を外した時点で実行時の影響がゼロになる安全性
- ✅ 各ステップでコンパイル可能な状態を維持しやすい
- ❌ `#[allow(dead_code)]` 警告が中間状態で発生
- ❌ スケジュールラベルの Phase 1 再利用判定が必要

### Option C: 一括削除（推奨）

**方針**: Phase 4 は Phase 1-3 完了後の純粋な削除作業であるため、全削除対象を同時に処理し、一度の `cargo check` で完了を確認する。

**手順**:
1. ファイル全削除: `com/dcomp.rs`, `visual_manager.rs`, `dcomp_demo.rs`
2. モジュール宣言除去: `com/mod.rs`, `ecs/graphics/mod.rs`
3. `components.rs`: `VisualGraphics`, `SurfaceGraphics` 定義 + `Visual.on_add` フック修正
4. `core.rs`: DComp フィールド + 初期化 + アクセサ除去
5. `systems.rs`: DComp システム関数 + ヘルパー関数削除
6. `world.rs`: スケジュール登録除去 + コメント更新
7. 全ファイル use 文クリーンアップ
8. テスト修正（`graphics_tests.rs`, `visual_hierarchy_sync_test.rs` 削除, `visual_graphics_auto_creation_test.rs` 削除）
9. `Cargo.toml` 更新
10. `cargo check` → `cargo test` → `cargo clippy`

**トレードオフ**:
- ✅ 最も効率的（中間状態の broken ビルドを回避）
- ✅ Phase 1-3 完了後なので参照は全てデッドコード — 削除で壊れるものがない
- ✅ `git diff --stat` で削除規模を一覧確認しやすい
- ❌ 変更量が大きいため、コミットの分割を慎重に行う必要あり
- ❌ 万一 Phase 1-3 で移行漏れがあった場合、発見が遅れる

---

## 4. 「Research Needed」項目

### RN-1: com/animation.rs の IDCompositionAnimation 依存

**問題**: `UIAnimationVariableExt::get_curve()` が `IDCompositionAnimation` を受け取る。この型は `Win32_Graphics_DirectComposition` feature に依存する。

**調査事項**:
- Phase 1 の新パイプラインで `IUIAnimationVariable2::GetCurve()` は使用されるか？
- 使用されない場合: `get_curve()` trait メソッドごと削除し、`DirectComposition` feature を除去可能
- 使用される場合: `IDCompositionAnimation` 型は DComp デバイスなしでも存在可能か？（`IDCompositionDevice3::CreateAnimation()` で作成する必要がある）

**暫定結論**: `get_curve()` は `dcomp_demo.rs` でのみ使用されている可能性が高い。Phase 1 の新アニメーション方式（dola クレート or Windows UI Animation のみ）では不要と推定。削除が安全な選択肢。

### RN-2: SurfaceGraphicsDirty / SurfaceCreationStats の Phase 1 再利用状況

**問題**: これらは DComp 固有型を含まないが、DComp Surface 処理と密結合。Phase 1 で再利用されるか否かにより削除判断が分かれる。

**暫定結論**: `SurfaceGraphicsDirty` は描画ダーティ管理の汎用マーカーとして Phase 1 でも有用。`SurfaceCreationStats` はデバッグリソースとして Phase 1 でも再利用可能。ただし Phase 1 の設計によっては別のダーティ管理方式を採用している可能性あり。

### RN-3: スケジュールラベルの Phase 1 再利用

**問題**: `PreRenderSurface`, `RenderSurface`, `Composition`, `CommitComposition` の各スケジュールラベルは DComp パイプライン用に命名されたもの。Phase 1 で新しいスケジュールラベルが導入されている場合、これらのラベル自体も削除対象となる。

**暫定結論**: Phase 2 でパイプライン切り替えが行われた際、既存ラベルを再利用（中身だけ新パイプラインに差し替え）か、新ラベルに移行したかにより判断が異なる。設計フェーズで確認要。

### RN-4: insert_visual / insert_visual_with のウィジェット層からの使用

**問題**: `visual_manager.rs` の `insert_visual()` / `insert_visual_with()` がウィジェット on_add フックから呼ばれている可能性。

**暫定結論**: 現在の実装では `Visual.on_add` フック内で直接 `Arrangement::default()` 等を挿入しており、`insert_visual()` を経由していない模様。grep で確認要。

---

## 5. 実装複雑度・リスク評価

### 工数: **S（1-3日）**

**根拠**:
- 新規実装なし、純粋な削除作業
- 削除対象が調査済みで明確
- Phase 1-3 完了により全 DComp コードがデッドコード化済み（前提）
- コンパイラとgrepで参照追跡が容易

### リスク: **Low**

**根拠**:
- 既存パターン（ファイル削除 + use文除去）の繰り返しで完了
- Phase 1-3 が正しく完了していれば、実行時の影響はゼロ
- 万一の漏れも `cargo check` / `cargo test` で即座に検出可能

**注意点**:
- `com/animation.rs` の `IDCompositionAnimation` 依存が唯一の不確実性
- Phase 1-3 の移行漏れがある場合、コンパイル時に発見される可能性あり

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ: Option C（一括削除）

Phase 4 は「削除のみ」の作業であり、設計の複雑さは低い。Option C の一括削除が最も効率的。

### 設計フェーズで確定すべき事項

1. **RN-1**: `com/animation.rs` の `get_curve()` / `IDCompositionAnimation` を削除するか保持するか
2. **RN-2**: `SurfaceGraphicsDirty` / `SurfaceCreationStats` の Phase 1 再利用状況
3. **RN-3**: DComp スケジュールラベルの Phase 1 再利用状況
4. **RN-4**: `insert_visual()` / `insert_visual_with()` の使用状況（削除前確認）
5. **コミット分割方針**: 1コミットで全削除 vs. ファイル種別ごとに分割
6. **Visual.on_add フック**: Phase 1 新コンポーネントの自動挿入に差し替え済みか確認

### 削除順序の推奨（Option C 内の実行順）

```
Phase 4 削除実行フロー:

[Step 1] ファイル全削除
   com/dcomp.rs, visual_manager.rs, dcomp_demo.rs
   + mod 宣言除去 (com/mod.rs, ecs/graphics/mod.rs)

[Step 2] コンポーネント修正
   components.rs: VisualGraphics, SurfaceGraphics 削除
   Visual.on_add フック修正

[Step 3] GraphicsCore 修正
   core.rs: DComp フィールド + 初期化 + アクセサ除去

[Step 4] システム関数削除
   systems.rs: DComp システム + ヘルパー削除

[Step 5] スケジュール更新
   world.rs: 登録除去 + コメント更新

[Step 6] 参照クリーンアップ
   全ファイル use 文除去
   com/animation.rs 修正 (RN-1 次第)
   Cargo.toml 更新

[Step 7] テスト修正
   graphics_tests.rs, test files 修正/削除

[Step 8] 最終検証
   cargo check → cargo test → cargo build --examples → cargo clippy
   grep -r "IDComposition" / "dcomp" 確認
```
