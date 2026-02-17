# 要件定義書: wintf-dcomp-migration-2-pipeline-switch

> **Rev 2** (2026-02-17) — Phase 1 完了を受けた改定。コンパイル整合性制約の追加、Req 5 決定確定、Req 6 スコープ明確化、Req 8 新設。

## 導入

本子仕様は親仕様 `wintf-dcomp-to-layered-migration` の Phase 2「DComp パイプライン置換」を担当する。Phase 1（`wintf-dcomp-migration-1-d2d1-composition`）で構築された D2D1 合成スタック（`WindowD3D11Compositor`, `composite_render_system`, `compositor_init_system`）を `world.rs` の ECS Schedule に登録し、既存 DComp パイプラインのシステム群を Schedule から除去して新パイプラインに切り替える。

### 前提条件

本子仕様は **Phase 1 の完了を前提** とする。Phase 1 が提供する以下の成果物が **完了済み** であることを確認した（2026-02-17 検証）:

- `ecs/graphics/compositor.rs` (265行): `WindowD3D11Compositor` コンポーネント — `composition_bitmap`, `staging_bitmap`, `hbitmap`, `memory_dc`, `dib_bits` の 4+1 リソース統合管理。`generation`, `dirty` フラグ、`resize()`, `invalidate()` メソッド実装済み
- `ecs/graphics/compositor_systems.rs` (459行): `compositor_init_system`, `composite_render_system` — D2D1 合成描画 + `transfer_to_hbitmap` による HBITMAP 転送。再帰 `render_subtree()` で opacity 手動累積。`is_window_dirty()` ヘルパーで `Changed<GraphicsCommandList>`, `Changed<GlobalArrangement>`, `Changed<Visual>`, `Added<WindowD3D11Compositor>` によるダーティ判定実装済み
- `ecs/graphics/mod.rs`: `pub mod compositor`, `pub mod compositor_systems` 宣言済み
- `com/ulw.rs` (52行): `transfer_to_hbitmap` ユーティリティ — D2D1 staging bitmap → HBITMAP ピクセル転送

**Phase 1 で未実装・Phase 3 に委譲された項目:**
- `UpdateLayeredWindow` Win32 API 呼び出し（`dirty` フラグは Phase 3 の `ulw_present_system` が消費する設計）
- `WS_EX_LAYERED` ウィンドウスタイル変更

### 本子仕様のスコープ

- `ecs/world.rs`: DComp システムの Schedule 登録解除 + Phase 1 新システムの登録
- `ecs/graphics/core.rs`: `GraphicsCoreInner` から DComp 初期化・フィールド・メソッドを除去
- `ecs/graphics/components.rs`: `on_visual_add` フックから DComp コンポーネント自動挿入を除去
- `ecs/graphics/systems.rs`: YELLOW システム（`invalidate_dependent_components`, `mark_dirty_surfaces`）を新コンポーネント型に適合
- `ecs/graphics/systems.rs`: `commit_composition` を Schedule から除去（Phase 3 `ulw_present_system` が CommitComposition ステージを引き継ぐ）
- `ecs/graphics/systems.rs`: DComp 除去に伴うコンパイルエラー解消（`dcomp()` / `desktop()` 参照を持つ旧システム関数本体の修正）
- `ecs/graphics_tests.rs`: DComp 参照を持つテストコードの更新

### コンパイル整合性制約

本 Phase の核心的課題として、**GraphicsCore から `dcomp()` / `desktop()` アクセサを除去すると、それらを参照する旧システム関数がコンパイルエラーになる** という依存関係がある。具体的に影響を受ける関数は以下の通り:

| ファイル            | 関数                                              | 参照 API         | Schedule 状態    |
| ------------------- | ------------------------------------------------- | ---------------- | ---------------- |
| `systems.rs`        | `init_window_graphics` (ヘルパー経由)             | `desktop()`      | Req 1 で除去     |
| `systems.rs`        | `commit_composition`                              | `dcomp()`        | Req 5 で除去     |
| `systems.rs`        | `deferred_surface_creation_system` (ヘルパー経由) | `dcomp()`        | Req 1 で除去     |
| `visual_manager.rs` | `visual_resource_management_system`               | `dcomp()`        | Req 1 で除去     |
| `visual_manager.rs` | `create_visual_only` (ヘルパー)                   | `dcomp()` (引数) | Req 1 で間接除去 |
| `graphics_tests.rs` | テスト関数 3 箇所                                 | `dcomp()`        | テストコード     |

**Note**: `window_visual_integration_system`（visual_manager.rs）および `visual_hierarchy_sync_system`, `visual_property_sync_system`, `render_surface`, `cleanup_surface_on_commandlist_removed`（systems.rs）は DComp COM オブジェクトをコンポーネント経由で使用するが、`GraphicsCore.dcomp()` / `desktop()` を直接呼ばないため、アクセサ除去によるコンパイルエラーは発生しない。

**方針**: Schedule から除去されるシステム関数は、関数本体から `dcomp()` / `desktop()` 呼び出しを除去する（空実装化、引数除去、または関数削除）。`com/dcomp.rs` モジュールおよびコンポーネント型定義（`WindowGraphics`, `VisualGraphics`, `SurfaceGraphics` の struct）の物理的削除は Phase 4 のスコープとする。

### Non-Goals

- DComp コードモジュールの物理的削除（`com/dcomp.rs`, `ecs/graphics/visual_manager.rs` の削除は Phase 4 で実施）
- DComp コンポーネント型定義の削除（`WindowGraphics`, `VisualGraphics`, `SurfaceGraphics` の struct 定義は Phase 4 まで残存許容）
- `UpdateLayeredWindow` 呼び出し（Phase 3 で実施）
- `WS_EX_LAYERED` ウィンドウスタイル変更（Phase 3 で実施）
- Phase 1 新モジュール（`compositor.rs`, `compositor_systems.rs`）の新規実装

---

## Requirements

### Requirement 1: ECS Schedule 切り替え

**Objective:** 開発者として、`world.rs` の描画パイプラインを DComp システムから D2D1 合成システムに切り替え、全既存 example が新パイプラインで動作するようにしたい。

_Parent: Req 2.3, 3.3_

#### Acceptance Criteria

1. The `world.rs` shall 以下の DComp システムを各 Schedule ステージから除去する:
   - PreLayout: `visual_resource_management_system`, `visual_hierarchy_sync_system`
   - GraphicsSetup: `init_window_graphics`, `window_visual_integration_system`
   - Draw: `deferred_surface_creation_system`, `cleanup_surface_on_commandlist_removed`
   - RenderSurface: `render_surface`
   - Composition: `visual_property_sync_system`

2. The `world.rs` shall Phase 1 で構築された以下の新システムを Schedule に登録する:
   - GraphicsSetup ステージ: `compositor_init_system`
   - Composition ステージ: `composite_render_system`

   **Note**: RenderSurface ステージは Phase 2 完了後に空になる。DComp では RenderSurface で CommandList を IDCompositionSurface へ焼き付けていたが、D2D1 パイプラインでは WPF 的遅延戦略を採用し、Composition ステージまで Window Bitmap への焼き付けを遅延する。これにより、最終描画タイミングでの最適化が可能となる。

3. The `world.rs` shall PreRenderSurface ステージから `mark_dirty_surfaces` を除去する（`composite_render_system` 内の `is_window_dirty()` ヘルパーが `Changed<GraphicsCommandList>`, `Changed<GlobalArrangement>`, `Changed<Visual>` でダーティ判定を代替する）

4. The `world.rs` shall CommitComposition ステージから `commit_composition` を除去する（Phase 3 で `ulw_present_system` が当該ステージを引き継ぐ）

5. When Schedule 切り替えが完了した時, the wintf crate shall 全既存 example（`taffy_flex_demo`, `typewriter_demo`, `multi_window_test`, `split_image`）が D2D1 合成パイプラインで正常動作する

### Requirement 2: GraphicsCore DComp 除去

**Objective:** 開発者として、`GraphicsCore` から DComp 初期化コードとフィールドを除去し、D2D1 デバイス中心のシンプルな初期化フローにしたい。

_Parent: Req 5.1, 5.2, 5.3, 5.4_

#### Acceptance Criteria

1. The `GraphicsCoreInner` shall `desktop: IDCompositionDesktopDevice` フィールドおよび `dcomp: IDCompositionDevice3` フィールドを削除する

2. The `GraphicsCore` shall `dcomp()` アクセサメソッドおよび `desktop()` アクセサメソッドを削除する

3. The `GraphicsCore::new()` shall `dcomp_create_desktop_device()` 呼び出しおよび `desktop.cast::<IDCompositionDevice3>()` 呼び出しを除去する

4. The `GraphicsCore` shall 以下のデバイスチェーンを変更なく維持する:
   - `D3D11CreateDevice` → `ID3D11Device` → `IDXGIDevice4`
   - `D2D1CreateFactory` → `ID2D1Factory` → `ID2D1Device` → `ID2D1DeviceContext`
   - `DWriteCreateFactory` → `IDWriteFactory2`

5. When デバイスロストが発生した時, the `GraphicsCore` shall `invalidate()` → 再初期化フローを DComp 再初期化ステップなしで正常に完了する

6. The `graphics_tests.rs` のテストコード shall `dcomp()` / `desktop()` への参照を除去し、DComp 非依存のテストとして更新する

### Requirement 3: on_visual_add フック更新

**Objective:** 開発者として、`Visual` コンポーネント追加時の自動コンポーネント挿入から DComp リソースコンポーネントを除去し、新パイプラインに不要な DComp コンポーネントの生成を停止したい。

_Parent: Req 6.2, 6.3_

#### Acceptance Criteria

1. The `on_visual_add` フック shall `VisualGraphics::default()` の自動挿入を除去する

2. The `on_visual_add` フック shall `SurfaceGraphics::default()` の自動挿入を除去する

3. The `on_visual_add` フック shall `SurfaceGraphicsDirty::default()` の自動挿入を除去する

4. The `on_visual_add` フック shall `Arrangement::default()` の挿入を維持する

5. The `on_visual_add` フック shall `BrushInherit` マーカーの挿入を維持する

### Requirement 4: YELLOW システム改修

**Objective:** 開発者として、DComp コンポーネント型への参照を持つ YELLOW 分類システムを、Phase 1 の新コンポーネント型（`WindowD3D11Compositor`）に追従させたい。

_Parent: Req 3.3_

#### Acceptance Criteria

1. The `invalidate_dependent_components` shall `WindowGraphics` への Query パラメータおよび `VisualGraphics`, `SurfaceGraphics` への Query パラメータを除去する

2. The `invalidate_dependent_components` shall `WindowD3D11Compositor` への Query パラメータを追加し、デバイスロスト時に `WindowD3D11Compositor` を invalidate する

3. The `invalidate_dependent_components` shall `BitmapSourceGraphics` への Query パラメータを維持する（DComp 非依存のコンポーネントであるため）

4. The `mark_dirty_surfaces` shall per-entity `SurfaceGraphicsDirty` ベースのダーティ検出を廃止する

5. The `mark_dirty_surfaces` の機能 shall `composite_render_system` 内の `is_window_dirty()` ヘルパー（`Changed<GraphicsCommandList>`, `Changed<GlobalArrangement>`, `Changed<Visual>` ベース）で代替されるため、システム自体を Schedule から除去する（Req 1.3 と連動）

6. The `init_graphics_core` shall Req 2 で `GraphicsCore::new()` から DComp 初期化ステップが消滅するため、DComp デバイスの有効性に関する暗黙の依存がなくなる（Req 2 の効果として自動達成）

### Requirement 5: commit_composition の除去

**Objective:** 開発者として、`commit_composition` システムを Schedule から除去し、CommitComposition ステージを Phase 3 の `ulw_present_system` に引き渡したい。

_Parent: Req 2.3_

**Note**: 本要件は Phase 2 と Phase 3 の責任境界を明示する。CommitComposition ステージは本 Phase で空にされ、Phase 3 の `ulw_present_system` に完全に引き継がれる設計である。親仕様の段階的移行戦略（Req 2.1）における Phase 間ハンドオーバーポイントとして機能する。

#### Acceptance Criteria

1. The `world.rs` shall `commit_composition` を CommitComposition ステージから除去する（Req 1.4 と連動）

2. The `commit_composition` 関数本体 shall `GraphicsCore.dcomp()` 呼び出し（`IDCompositionDevice3::Commit()`）を除去する（コンパイル整合性のため。関数の空実装化または削除のいずれかを選択。Phase 3 `ulw_present_system` が CommitComposition ステージの新たな担い手となる）

### Requirement 6: ECS コードからの DComp 実行パス除去検証

**Objective:** 開発者として、ECS パイプラインの実行パス（Schedule 登録済みシステム、GraphicsCore、テストコード）から DComp API 参照を完全に除去したことを静的に検証したい。

_Parent: Req 2.3, 10.1_

#### Acceptance Criteria

1. When Phase 2 が完了した時, the `ecs/` ディレクトリ内の **Schedule 登録済みシステム関数**, **GraphicsCore**, **テストコード** shall `IDComposition` 型への参照を含まない

2. When Phase 2 が完了した時, the `ecs/` ディレクトリ内の Schedule 登録済みコード shall `dcomp()` や `desktop()` メソッド呼び出しを含まない

3. The `ecs/` ディレクトリ内の DComp コンポーネント型定義（`WindowGraphics`, `VisualGraphics`, `SurfaceGraphics` の struct 定義）における `IDComposition` 型フィールド shall Phase 4 まで残存を許容する（型定義は Schedule 実行パスに含まれないため）

4. The `ecs/` ディレクトリ内の Schedule 非登録関数（`init_window_graphics`, `render_surface`, `deferred_surface_creation_system` 等の旧関数本体）における `dcomp()` / `desktop()` 参照 shall 除去する（空実装化・引数除去・関数削除のいずれか）

5. The wintf crate shall `cargo test` の全テストがパスする

6. The wintf crate shall `cargo build --examples` で全 example がビルド成功する

### Requirement 7: Phase 2 完了検証基準

**Objective:** 開発者として、Phase 2 の完了を客観的に判定できる包括的な検証基準が欲しい。

_Parent: Req 10.1, 10.2_

#### Acceptance Criteria

1. The 全既存 example（`taffy_flex_demo`, `typewriter_demo`, `multi_window_test`, `split_image`）shall D2D1 合成パイプラインで正常に描画される

2. The `GraphicsCoreInner` shall DComp 関連フィールド（`desktop`, `dcomp`）を含まない

3. The `GraphicsCore` shall DComp 関連アクセサメソッド（`dcomp()`, `desktop()`）を含まない

4. The `world.rs` の Schedule shall DComp システム（Req 1.1 の 8 システム + `mark_dirty_surfaces` + `commit_composition`）を含まない

5. The `world.rs` の Schedule shall Phase 1 新システム（`compositor_init_system`, `composite_render_system`）を含む

6. The RenderSurface ステージ shall システム登録を含まない（WPF 的遅延戦略により、焼き付けは Composition ステージで実行）

7. The `on_visual_add` フック shall DComp コンポーネント（`VisualGraphics`, `SurfaceGraphics`, `SurfaceGraphicsDirty`）の挿入を含まない

8. The `cargo test` shall 全テストがパスする

9. The `cargo build --examples` shall 全 example がビルド成功する

10. The `ecs/` ディレクトリ内の Schedule 登録済みシステム関数 shall `dcomp()` / `desktop()` メソッド呼び出しを含まない

### Requirement 8: コンパイル整合性

**Objective:** 開発者として、Phase 2 の全変更適用後に wintf crate 全体がコンパイル可能であることを保証したい。DComp アクセサ除去と旧システム関数の共存を安全に管理する。

_Parent: Req 2.3, 10.1_

#### Acceptance Criteria

1. The wintf crate shall Phase 2 の全変更を適用後、`cargo build` が成功する

2. The `systems.rs` 内の Schedule 非登録旧システム関数（`init_window_graphics`, `render_surface`, `deferred_surface_creation_system`, `cleanup_surface_on_commandlist_removed`, `visual_property_sync_system`）shall `GraphicsCore.dcomp()` / `GraphicsCore.desktop()` への参照を含まない状態でコンパイル可能とする

3. The `visual_manager.rs` 内の関数 shall `GraphicsCore.dcomp()` への参照を含まない状態でコンパイル可能とする（関数本体の修正、引数変更、または関数の `#[allow(dead_code)]` + 空実装化のいずれか）

4. The `graphics_tests.rs` shall `GraphicsCore.dcomp()` / `GraphicsCore.desktop()` への参照を含まない状態でコンパイル可能とする

---

## 改定履歴

### Rev 2 (2026-02-17) — Phase 1 完了に伴う改定

**改定動機**: Phase 1 (`wintf-dcomp-migration-1-d2d1-composition`) が完了し、成果物の実態を検証した結果、以下の課題が判明した:

1. **コンパイル整合性問題**: Req 2 で `dcomp()` / `desktop()` を除去すると、Schedule 非登録の旧システム関数 6 箇所 + テスト 3 箇所がコンパイルエラーになる。元要件はこの連鎖的影響を明示していなかった
2. **Req 5 の判断保留**: `commit_composition` の Schedule 維持/除去が未決定だったが、Phase 3 仕様で `ulw_present_system` が CommitComposition ステージを引き継ぐことが確定しており、Phase 2 で除去が妥当
3. **Req 6 のスコープ不整合**: "ecs/ の IDComposition 参照ゼロ" と Non-Goals "DComp コード物理削除は Phase 4" が矛盾。コンポーネント型定義（16 件の IDComposition 参照残存）の許容を明確化する必要あり
4. **Req 4 の WindowGraphics 欠落**: `invalidate_dependent_components` が `WindowGraphics` も無効化しているが、元 AC 4.1 には記載がなかった
5. **Phase 1 成果物の具体性**: `composite_render_system` 内の `is_window_dirty()` ヘルパーの存在が確認され、`mark_dirty_surfaces` の完全除去判断が確定

**主な変更点**:
- **Req 1**: AC 1.3 を確定（mark_dirty_surfaces 除去）、AC 1.4 を新設（commit_composition 除去）、旧 AC 1.4 → AC 1.5 に繰り下げ。AC 1.2 に WPF 的遅延戦略 Note 追加、AC 7.6 新設（RenderSurface 空ステージ検証）
- **Req 2**: AC 2.6 新設（テストコード更新）
- **Req 4**: AC 4.1 に `WindowGraphics` 除去を追加、AC 4.3 を BitmapSourceGraphics 維持として新設、旧 AC を繰り下げ、AC 4.6 を Req 2 効果の自動達成に修正
- **Req 5**: 全面改訂 — 判断保留の条件分岐を除去し、Schedule 除去 + 関数本体修正に確定。Phase 2-3 責任境界明示のため Note 追加（Phase 間ハンドオーバーポイントとしての位置づけを明確化）
- **Req 6**: タイトル変更（「参照ゼロ検証」→「実行パス除去検証」）、AC 6.1 のスコープを Schedule 登録済みコードに限定、AC 6.3 に型定義残存許容を明記、AC 6.4 に旧関数本体の修正義務を追加
- **Req 7**: AC 7.4 に mark_dirty_surfaces + commit_composition を追加、AC 7.6 新設（RenderSurface 空ステージ検証）、AC 7.8-7.9 新設
- **Req 8**: 新設 — コンパイル整合性保証（旧システム関数・visual_manager・テストコードの修正義務）
- **導入**: Phase 1 成果物の詳細確認結果を記載、コンパイル整合性制約セクション新設

## 要件カバレッジサマリー

| 子仕様要件 | 親要件     | 概要                                                                     |
| ---------- | ---------- | ------------------------------------------------------------------------ |
| Req 1      | 2.3, 3.3   | ECS Schedule 切り替え（DComp 除去 + D2D1 登録 + mark_dirty/commit 除去） |
| Req 2      | 5.1–5.4    | GraphicsCore DComp フィールド・初期化除去 + テスト更新                   |
| Req 3      | 6.2, 6.3   | on_visual_add フックから DComp コンポーネント除去                        |
| Req 4      | 3.3        | YELLOW システム（invalidate / mark_dirty / init）改修                    |
| Req 5      | 2.3        | commit_composition の Schedule 除去 + 関数本体修正                       |
| Req 6      | 2.3, 10.1  | ECS 実行パスからの DComp 参照除去検証（型定義残存許容）                  |
| Req 7      | 10.1, 10.2 | Phase 2 完了検証基準（E2E + 構造 + コンパイル検証）                      |
| Req 8      | 2.3, 10.1  | コンパイル整合性保証（旧関数・テスト修正）                               |
