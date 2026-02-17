# Research & Design Decisions: wintf-dcomp-migration-2-pipeline-switch

## Summary
- **Feature**: `wintf-dcomp-migration-2-pipeline-switch`
- **Discovery Scope**: Extension（既存 ECS パイプラインの Schedule 切り替え）
- **Key Findings**:
  1. Schedule 登録は 13 ステージ・約 40 システムで構成され、DComp 関連は 10 システム（8 + mark_dirty_surfaces + commit_composition）
  2. `invalidate_dependent_components` は `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics`, `BitmapSourceGraphics` の 4 Query を持ち、DComp 3 Query の除去 + `WindowD3D11Compositor` 追加が必要
  3. Phase 1 新システム（`compositor_init_system`, `composite_render_system`）は既存の `GraphicsCore`, `WindowHandle`, `WindowPos`, `HasGraphicsResources` に依存し、既存パイプラインとの統合ポイントは明確

## Research Log

### Schedule ステージ構造と DComp システム分布

- **Context**: Req 1 の DComp システム除去対象を正確に特定するため
- **Sources Consulted**: `ecs/world.rs` L590-602（try_tick_world）、全 add_system 呼び出し
- **Findings**:
  - 13 ステージ: Input → Update → PreLayout → Layout → PostLayout → UISetup → GraphicsSetup → Draw → PreRenderSurface → RenderSurface → Composition → CommitComposition → FrameFinalize
  - DComp 除去対象（10 システム）:
    - PreLayout: `visual_resource_management_system`, `visual_hierarchy_sync_system`
    - GraphicsSetup: `init_window_graphics`, `window_visual_integration_system`
    - Draw: `deferred_surface_creation_system`, `cleanup_surface_on_commandlist_removed`
    - PreRenderSurface: `mark_dirty_surfaces`
    - RenderSurface: `render_surface`
    - Composition: `visual_property_sync_system`
    - CommitComposition: `commit_composition`
  - PreLayout に残る `init_graphics_core` は DComp 非依存（GraphicsCore 全体の初期化）→ 維持
- **Implications**: 除去後、PreLayout は `init_graphics_core` のみ、GraphicsSetup は `compositor_init_system` のみ、PreRenderSurface/RenderSurface/CommitComposition は空ステージとなる

### on_visual_add フック構造

- **Context**: Req 2 の DComp コンポーネント除去対象を特定するため
- **Sources Consulted**: `ecs/graphics/components.rs` L268-306
- **Findings**:
  - 挿入対象（5 コンポーネント）: `Arrangement`, `VisualGraphics`, `SurfaceGraphics`, `SurfaceGraphicsDirty`, `BrushInherit`
  - 除去対象（3 コンポーネント）: `VisualGraphics`, `SurfaceGraphics`, `SurfaceGraphicsDirty`
  - 維持対象（2 コンポーネント）: `Arrangement`, `BrushInherit`
  - すべて `if !world.entity(entity).contains::<T>()` ガード付き
- **Implications**: 除去は単純な行削除（3 つの insert ブロックを削除）

### invalidate_dependent_components の Query 構造

- **Context**: Req 3 の YELLOW システム改修内容を確定するため
- **Sources Consulted**: `ecs/graphics/systems.rs` L794-830
- **Findings**:
  - 現在の Query パラメータ: `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics`, `BitmapSourceGraphics`
  - 除去対象: `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics`（DComp 依存）
  - 追加対象: `WindowD3D11Compositor`（D2D1 パイプライン用）
  - 維持対象: `BitmapSourceGraphics`（DComp 非依存）
  - ロジック: `GraphicsCore` が `None` または generation 不一致時に全コンポーネントを `invalidate()`
- **Implications**: Query パラメータの差し替え + invalidate ループの対象変更。`WindowD3D11Compositor` は `invalidate()` メソッドを持つ（Phase 1 で実装済み）

### composite_render_system のダーティ判定

- **Context**: `mark_dirty_surfaces` の代替確認
- **Sources Consulted**: `ecs/graphics/compositor_systems.rs` L350-460
- **Findings**:
  - `is_window_dirty()` ヘルパー: `Changed<GraphicsCommandList>`, `Changed<GlobalArrangement>`, `Changed<Visual>`, `Added<WindowD3D11Compositor>` でダーティ判定
  - `mark_dirty_surfaces` の対象: `Changed<GraphicsCommandList>`, `Changed<SurfaceGraphics>`, `Added<SurfaceGraphics>`, `Changed<GlobalArrangement>` — per-entity `SurfaceGraphicsDirty` マーカー
  - 代替関係: `is_window_dirty()` はウィンドウ単位のダーティ検出であり、`SurfaceGraphicsDirty` の per-entity 粒度は不要
- **Implications**: `mark_dirty_surfaces` は Schedule 除去のみ（関数本体は旧実装保持戦略で Phase 4 まで保持）

### Phase 2 完了時の空ステージ

- **Context**: 設計時の Schedule 全体像把握
- **Sources Consulted**: world.rs 全体
- **Findings**:
  - Phase 2 完了後の空ステージ: PreRenderSurface, RenderSurface, CommitComposition
  - PreRenderSurface: `mark_dirty_surfaces` 除去 → 空
  - RenderSurface: `render_surface` 除去 → 空（WPF 的遅延戦略: Composition で焼き付け）
  - CommitComposition: `commit_composition` 除去 → 空（Phase 3 `ulw_present_system` が引き継ぐ）
- **Implications**: 空ステージは Phase 3-4 で再利用・削除されるため、Phase 2 では空のまま保持

## Design Decisions

### Decision: PreLayout ステージのシステム除去後の chain 構造

- **Context**: PreLayout には `init_graphics_core`, `visual_resource_management_system`, `visual_hierarchy_sync_system` が chain で接続されている。後者 2 つを除去すると chain が崩れる
- **Alternatives Considered**:
  1. `init_graphics_core` を単独システムとして再登録
  2. chain 全体を再構築
- **Selected Approach**: `init_graphics_core` を単独 add_systems で登録
- **Rationale**: 依存関係のない単独システムに chain は不要
- **Trade-offs**: chain から単独登録への変更は微小な差異
- **Follow-up**: world.rs の PreLayout セクション書き換え

### Decision: GraphicsSetup ステージの新システム配置

- **Context**: 旧 `init_window_graphics` + `window_visual_integration_system` の chain を `compositor_init_system` 単独に置換
- **Selected Approach**: `compositor_init_system` を GraphicsSetup に単独登録
- **Rationale**: compositor_init_system は WindowD3D11Compositor の作成・再作成・リサイズを一括処理し、旧 2 システムの責任を統合している

### Decision: Composition ステージの新システム配置

- **Context**: `composite_render_system` をどのステージに配置するか
- **Selected Approach**: Composition ステージに配置
- **Rationale**: D2D1 パイプラインでは WPF 的遅延戦略を採用し、CommandList の焼き付けを Composition ステージまで遅延。RenderSurface ステージは空になる

## Risks & Mitigations

- **Draw ステージの DComp 後段システム除去**: `deferred_surface_creation_system` と `cleanup_surface_on_commandlist_removed` は Draw の chain 末尾にあるため、chain から除去後の依存関係を確認 → Draw の chain は `resolve_inherited_brushes` から `generate_alpha_mask_system` まで維持し、後段 2 システムを除去
- **既存テストへの影響**: DComp テスト関数は GraphicsCore.dcomp()/desktop() に依存するが、旧実装保持戦略により Phase 4 まで保持 → `cargo test` は通過する（Schedule 未登録のため実行されないが、コンパイルは通る）
- **Phase 2 単体での画面非表示**: `composite_render_system` はビットマップ合成のみで `UpdateLayeredWindow` は呼ばない → Phase 3 完了まで視覚的確認不可。構造検証（Schedule 構造、コンパイル、テスト）で Phase 2 完了を判定
