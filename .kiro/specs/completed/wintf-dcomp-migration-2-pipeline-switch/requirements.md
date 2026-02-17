# 要件定義書: wintf-dcomp-migration-2-pipeline-switch

> **Rev 6** (2026-02-17) — 議題 5: Req 3 AC 4 削除（旧実装保持戦略との整合）。`mark_dirty_surfaces` 関数本体修正要求を削除し、Schedule 除去のみに限定。

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
- `ecs/graphics/components.rs`: `on_visual_add` フックから DComp コンポーネント自動挿入を除去
- `ecs/graphics/systems.rs`: `invalidate_dependent_components` を新コンポーネント型（`WindowD3D11Compositor`）に適合

### 旧実装保持戦略

本 Phase では **Schedule 切り替えのみを実施** し、旧 DComp 実装コード（GraphicsCore の DComp フィールド、Schedule 非登録の旧システム関数、コンポーネント型定義）は **Phase 4 まで保持** する。

**保持される旧実装**:
- `GraphicsCore` の `dcomp: IDCompositionDevice3`, `desktop: IDCompositionDesktopDevice` フィールド
- `GraphicsCore::dcomp()`, `GraphicsCore::desktop()` アクセサメソッド
- Schedule 非登録の旧システム関数（`init_window_graphics`, `commit_composition`, `deferred_surface_creation_system`, `visual_resource_management_system`, `window_visual_integration_system`, `visual_hierarchy_sync_system`, `visual_property_sync_system`, `render_surface`, `cleanup_surface_on_commandlist_removed` 等）
- DComp コンポーネント型定義（`WindowGraphics`, `VisualGraphics`, `SurfaceGraphics` の struct）
- `com/dcomp.rs` モジュール
- `ecs/graphics/visual_manager.rs` モジュール
- `graphics_tests.rs` の DComp テスト関数

**Phase 2-3 の責任**: 新 D2D1+ULW パイプラインの動作検証に集中し、旧コードに一切触れない。

**Phase 4 の責任**: 新パイプライン安定後、旧実装を一括削除（GraphicsCore フィールド + 旧関数 + 型定義 + モジュール）。

### Non-Goals

- **GraphicsCore からの DComp フィールド・メソッド除去**（Phase 4 で実施）
- **Schedule 非登録の旧システム関数の修正・削除**（Phase 4 で実施）
- DComp コードモジュールの物理的削除（`com/dcomp.rs`, `ecs/graphics/visual_manager.rs` の削除は Phase 4 で実施）
- DComp コンポーネント型定義の削除（`WindowGraphics`, `VisualGraphics`, `SurfaceGraphics` の struct 定義は Phase 4 まで残存許容）
- DComp テストコードの修正・削除（`graphics_tests.rs` の DComp テストは Phase 4 まで保持）
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

4. The `world.rs` shall CommitComposition ステージから `commit_composition` を除去する

   **Note**: 本 AC は Phase 2 と Phase 3 の責任境界を明示する。CommitComposition ステージは本 Phase で空にされ、Phase 3 の `ulw_present_system` に完全に引き継がれる設計である。親仕様の段階的移行戦略（Req 2.1）における Phase 間ハンドオーバーポイントとして機能する。

### Requirement 2: on_visual_add フック更新

**Objective:** 開発者として、`Visual` コンポーネント追加時の自動コンポーネント挿入から DComp リソースコンポーネントを除去し、新パイプラインに不要な DComp コンポーネントの生成を停止したい。

_Parent: Req 6.2, 6.3_

#### Acceptance Criteria

1. The `on_visual_add` フック shall `VisualGraphics::default()` の自動挿入を除去する

2. The `on_visual_add` フック shall `SurfaceGraphics::default()` の自動挿入を除去する

3. The `on_visual_add` フック shall `SurfaceGraphicsDirty::default()` の自動挿入を除去する

4. The `on_visual_add` フック shall `Arrangement::default()` の挿入を維持する

5. The `on_visual_add` フック shall `BrushInherit` マーカーの挿入を維持する

### Requirement 3: YELLOW システム改修

**Objective:** 開発者として、DComp コンポーネント型への参照を持つ YELLOW 分類システムを、Phase 1 の新コンポーネント型（`WindowD3D11Compositor`）に追従させたい。

_Parent: Req 3.3_

#### Acceptance Criteria

1. The `invalidate_dependent_components` shall `WindowGraphics` への Query パラメータおよび `VisualGraphics`, `SurfaceGraphics` への Query パラメータを除去する

2. The `invalidate_dependent_components` shall `WindowD3D11Compositor` への Query パラメータを追加し、デバイスロスト時に `WindowD3D11Compositor` を invalidate する

3. The `invalidate_dependent_components` shall `BitmapSourceGraphics` への Query パラメータを維持する（DComp 非依存のコンポーネントであるため）

4. The `mark_dirty_surfaces` の機能 shall `composite_render_system` 内の `is_window_dirty()` ヘルパー（`Changed<GraphicsCommandList>`, `Changed<GlobalArrangement>`, `Changed<Visual>` ベース）で代替されるため、システム自体を Schedule から除去する（Req 1.3 と連動）

### Requirement 4: Schedule 登録済みシステムの DComp 参照除去検証

**Objective:** 開発者として、ECS Schedule に登録済みのシステムが DComp API を参照していないことを検証し、新パイプラインへの完全切り替えを確認したい。

_Parent: Req 2.3, 10.1_

**Note**: Schedule 非登録の旧システム関数および GraphicsCore の DComp フィールドは Phase 4 まで保持されるため、本検証のスコープ外とする。

#### Acceptance Criteria

1. When Phase 2 が完了した時, the `ecs/world.rs` の Schedule 登録システム shall DComp システム（Req 1.1 の 8 システム + `mark_dirty_surfaces` + `commit_composition`）を含まない

2. When Phase 2 が完了した時, the `ecs/world.rs` の Schedule 登録システム shall Phase 1 新システム（`compositor_init_system`, `composite_render_system`）を含む

### Requirement 5: Phase 2 完了検証基準

**Objective:** 開発者として、Phase 2 の完了を客観的に判定できる包括的な検証基準が欲しい。

_Parent: Req 10.1, 10.2_

**Note**: Phase 2 は Schedule 切り替えによる **構造的整合性** のみを検証する。`composite_render_system` はビットマップへの合成描画を完了するが、`UpdateLayeredWindow` 呼び出しは Phase 3 の `ulw_present_system` が担当するため、**Phase 2 単体では画面に何も表示されない**。Example の視覚的動作確認は Phase 3 完了まで不可能である。

#### Acceptance Criteria

1. The `world.rs` の Schedule shall DComp システム（Req 1.1 の 8 システム + `mark_dirty_surfaces` + `commit_composition`）を含まない

2. The `world.rs` の Schedule shall Phase 1 新システム（`compositor_init_system`, `composite_render_system`）を含む

3. The RenderSurface ステージ shall システム登録を含まない（WPF 的遅延戦略により、焼き付けは Composition ステージで実行）

4. The `on_visual_add` フック shall DComp コンポーネント（`VisualGraphics`, `SurfaceGraphics`, `SurfaceGraphicsDirty`）の挿入を含まない

5. The `cargo test` shall 全テストがパスする

6. The `cargo build --examples` shall 全 example がビルド成功する

---

## 改定履歴

### Rev 6 (2026-02-17) — 議題 5: Req 3 AC 4 削除（旧実装保持戦略との整合）

**改定動機**: Req 3 AC 4 が `mark_dirty_surfaces` 関数本体の修正（`SurfaceGraphicsDirty` ベースのダーティ検出廃止）を要求していたが、これは Rev 3 の旧実装保持戦略（「Schedule 非登録の旧システム関数は Phase 4 まで保持し、Phase 2-3 では触らない」）と矛盾する。`mark_dirty_surfaces` は Req 1.3 で Schedule から除去されるため、Schedule 非登録関数となり、関数本体の修正は Phase 4 の責任である。

**主な変更点**:
- **Req 3 AC 4 削除**: `mark_dirty_surfaces` 関数本体の修正要求を削除（旧実装保持戦略違反）
- **Req 3 AC 5 → AC 4**: Schedule 除去のみを要求する AC を繰り上げ（Req 1.3 と連動）

### Rev 5 (2026-02-17) — 議題 4: Req 4 冗長性解消

**改定動機**: Req 4 「commit_composition の除去」の唯一の AC が Req 1 AC 4 と完全に同一であり、実装者の混乱を招く。Req 5 は「検証の独立性」により独立維持が妥当だったが、Req 4 は「実装の重複」により統合が適切。Phase 間ハンドオーバー情報は Req 1 AC 4 の Note として保持。

**主な変更点**:
- **Req 4 削除**: 「commit_composition の除去」要件全体を削除
- **Req 1 AC 4**: Req 4 の Phase 間ハンドオーバー Note を統合
- **Req 5-6 繰り上げ**: 旧 Req 5 → Req 4、旧 Req 6 → Req 5
- **要件カバレッジサマリー**: 6 要件 → 5 要件に更新

### Rev 4 (2026-02-17) — 議題 3: Phase 2 完了基準の現実化

**改定動機**: Phase 1 実装確認の結果、`composite_render_system` がビットマップ合成完了後に `dirty` フラグを設定するのみで、`UpdateLayeredWindow` 呼び出しは Phase 3 の `ulw_present_system` に委譲されることが判明。Phase 2 単体では画面表示が不可能であるため、完了検証基準を構造的整合性に限定する。

**主な変更点**:
- **Req 1**: AC 5 削除（「全既存 example が正常動作する」— Phase 2 単体では画面表示不可能）
- **Req 6**: Note 新設（Phase 2 は構造的整合性のみ検証、画面表示は Phase 3 待ち）、AC 1 削除（「example が正常に描画される」— Phase 2 単体では実現不可能）、AC 番号繰り上げ（旧 AC 2-7 → 新 AC 1-6）

### Rev 3 (2026-02-17) — 議題 2: 旧関数保持戦略への変更

**改定動機**: 旧 DComp 実装を Phase 2-3 で保持し、Phase 4 で一括削除する戦略に変更。「新実装を別名で作り、削除＆リネームは Phase 4」という開発者の意図に沿う。旧実装を積極的にいじるモチベーションはなく、Phase 2-3 は Schedule 切り替えと ULW 統合に集中する。

**主な変更点**:
- **Req 2 削除**: GraphicsCore DComp 除去を Phase 4 に延期。GraphicsCore の `dcomp`, `desktop` フィールドおよびアクセサメソッドは Phase 4 まで保持
- **Req 3-8 繰り上げ**: 旧 Req 3 → Req 2, 旧 Req 4 → Req 3, 旧 Req 5 → Req 4, 旧 Req 6 → Req 5, 旧 Req 7 → Req 6、旧 Req 8 削除（コンパイル整合性問題は旧関数保持で解消）
- **Req 3 (YELLOW システム)**: AC 3.6 削除（Req 2 効果への言及）
- **Req 4 (commit_composition)**: AC 4.2 削除（関数本体修正を削除、Schedule 除去のみ）
- **Req 5 (検証)**: タイトル変更（「ECS コードからの DComp 実行パス除去検証」→「Schedule 登録済みシステムの DComp 参照除去検証」）、AC 簡素化（Schedule 登録済みシステムのみ検証）、Note 追加（旧関数保持明記）
- **Req 6 (完了検証)**: AC 6.2-6.3 削除（GraphicsCore 検証）、AC 番号調整
- **スコープセクション**: GraphicsCore 関連項目削除、テストコード更新項目削除
- **旧実装保持戦略セクション**: 新設—Phase 2-3 で保持される旧実装の一覧と Phase 間責任分担を明記
- **Non-Goals**: GraphicsCore 除去、旧関数修正・削除、DComp テスト保持を明記

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
| Req 2      | 6.2, 6.3   | on_visual_add フックから DComp コンポーネント除去                        |
| Req 3      | 3.3        | YELLOW システム（invalidate / mark_dirty）改修                           |
| Req 4      | 2.3, 10.1  | Schedule 登録済みシステムの DComp 参照除去検証                           |
| Req 5      | 10.1, 10.2 | Phase 2 完了検証基準（構造検証のみ、画面表示は Phase 3 待ち）            |
