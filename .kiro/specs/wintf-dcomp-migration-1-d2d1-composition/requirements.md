# 要件定義書: wintf-dcomp-migration-1-d2d1-composition

## 導入

本子仕様は親仕様 `wintf-dcomp-to-layered-migration` の Phase 1「D2D1合成スタック構築」を担当する。現行の DComp パイプライン（`IDCompositionDevice3::Commit` による合成）を温存したまま、新しい D2D1 合成描画スタックを独立モジュールとして構築し、既存のウィジェット描画システムが出力する `GraphicsCommandList`（`ID2D1CommandList`）を per-window 合成ビットマップに統合描画する能力を確立する。

### コンテキスト

現行パイプラインでは各ウィジェットの描画結果は `IDCompositionSurface` を介して DComp Visual ツリーに送られ、`IDCompositionDevice3::Commit()` でハードウェア合成される。本仕様では、DComp 合成に代わる D2D1 ソフトウェア合成パスを新規に構築し、`ID2D1Bitmap1` 上に全ウィジェットを1枚の合成結果として描画する。この合成結果は後続 Phase で `UpdateLayeredWindow` に渡されるが、Phase 1 では合成完了と HBITMAP 転送までをスコープとする。

### 前提条件の充足状況

以下の子仕様が実装完了しており、Phase 1 の前提条件は全て充足済みである:

| 子仕様 | 状態 | Phase 1 への貢献 |
|--------|------|-----------------|
| `wintf-dcomp-migration-0-visual-opacity-dataflow` | **完了** ✅ | `Visual.opacity` / `Visual.is_visible` データフロー確立。Widget 層から `Visual.set_opacity()` / `Visual.set_visible()` で書き込み可能。`visual_property_sync_system` は `Changed<Visual>` で opacity 変更を検出し DComp に同期。`Opacity` コンポーネントは `#[deprecated]` 済み |
| `wintf-taffy-child-order-fix` | **完了** ✅ | `sync_taffy_tree_system` と `visual_hierarchy_sync_system` が `Children` コンポーネントを権威的ソースとして参照。アーキタイプ反復順序への依存を排除し、`composite_render_system` が `Children` を depth-first pre-order 走査する際の正しい兄弟順序を保証 |

### 本子仕様のスコープ

- `ecs/graphics/compositor.rs` 新規作成: WindowD3D11Compositor コンポーネント
- `ecs/graphics/compositor_systems.rs` 新規作成: compositor_init_system, composite_render_system
- `com/ulw.rs` 新規作成（部分）: transfer_to_hbitmap ユーティリティ

### Non-Goals

- `world.rs` への新システム登録（Phase 2 で実施）
- DComp パイプラインの変更・無効化（Phase 2 で実施）
- `UpdateLayeredWindow` 呼び出し（Phase 3 で実施）
- `WS_EX_LAYERED` ウィンドウスタイル変更（Phase 3 で実施）
- 旧 DComp コード削除（Phase 4 で実施）
- `GraphicsCore` からの DComp 初期化除去（Phase 2 以降）
- Layout 層への透明度追加（GlobalArrangement に opacity は不要。`CompositeContext` 手動累積方式を採用）
- `Opacity` コンポーネントの削除（Phase 0 で `#[deprecated]` 済み。完全削除は Phase 4 で DComp 依存コード削除時に実施）

---

## Requirements

### Requirement 1: WindowD3D11Compositor コンポーネント

**Objective:** 開発者として、ウィンドウごとの D2D1 合成描画リソース群を統合管理する ECS コンポーネントが欲しい。これにより DComp Visual ツリーに依存しない per-window 合成描画基盤が得られる。

_Parent: Req 3.1, 6.1_

#### Acceptance Criteria

1. The `WindowD3D11Compositor` shall 以下の4リソースを統合管理する:
   - 合成描画先ビットマップ（`ID2D1Bitmap1`, `D2D1_BITMAP_OPTIONS_TARGET` フラグ付き）
   - CPU ステージングビットマップ（`ID2D1Bitmap1`, `D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW` フラグ付き）
   - HBITMAP（`CreateDIBSection` による PBGRA32 形式 top-down DIB）
   - MemoryDC（`CreateCompatibleDC` によるメモリデバイスコンテキスト）

2. The `WindowD3D11Compositor` shall リソースライフサイクル管理 API として以下を提供する:
   - 全リソースの一括作成（コンストラクタ `new()`）
   - 新サイズでの全リソース再作成（`resize()`）
   - 全リソースの無効化（`invalidate()`、リソース解放は Drop に委譲）
   - リソース有効性の判定（`is_valid()`）
   - リソース世代の追跡（generation カウンタ）
   - 合成完了フラグ管理（`is_dirty()` / `set_dirty()`、Phase 3 `ulw_present_system` が消費）

3. The `WindowD3D11Compositor` shall 管理する全4リソースが常に同一サイズ・同一ピクセルフォーマット（PBGRA32）を維持することを保証する

4. The `WindowD3D11Compositor` shall bevy_ecs の SparseSet ストレージ戦略を使用する（ウィンドウエンティティ数が少ないため）

5. The `WindowD3D11Compositor` shall 既存の `WindowGraphics` と同じ `Option<Inner>` パターン（`WindowD3D11CompositorInner` 内部構造体による COM 安全管理）を採用し、`Drop` で GDI リソース（`DeleteObject(hbitmap)`, `DeleteDC(memory_dc)`）を適切に解放する

6. The `WindowD3D11Compositor` shall `ecs/graphics/compositor.rs` に配置される

### Requirement 2: 合成描画システム（composite_render_system）

**Objective:** 開発者として、全エンティティの `GraphicsCommandList` を z-order + transform + opacity で per-window 合成ビットマップに描画するシステムが欲しい。これにより DComp Visual ツリーの階層合成を D2D1 ソフトウェア合成で代替できる。

_Parent: Req 3.1, 3.2, 3.3, 3.4, 3.6_

#### Acceptance Criteria

1. The `composite_render_system` shall ウィンドウに属する全エンティティを `Children` 関係の depth-first pre-order で走査し、z-order に従って合成ビットマップに描画する（`wintf-taffy-child-order-fix` により `Children` の兄弟順序が権威的ソースとして保証済み）

2. The `composite_render_system` shall 各エンティティの `GlobalArrangement.transform` をデバイスコンテキストに `SetTransform` で適用した上で、対応する `GraphicsCommandList` を合成ビットマップに描画する

3. When `Visual.is_visible` が `false` のエンティティを処理する時, the `composite_render_system` shall そのエンティティとその children の描画を完全にスキップする（Phase 0 で確立された `Visual.is_visible` データフローを利用）

4. The `composite_render_system` shall `CompositeContext` ローカル構造体により再帰的な階層走査（`render_subtree()`）中に `accumulated_opacity * Visual.opacity` で各エンティティの最終 opacity を累積計算し、親から子へ引き継ぐ（Phase 0 で確立された `Visual.opacity` データフローを利用。Widget は `Visual.set_opacity()` で設定済み）

5. When 累積 opacity が 1.0 未満のエンティティを処理する時, the `composite_render_system` shall D2D Effect または pre-multiplied alpha 操作で累積 opacity を適用してから描画する（PushLayer は中間サーフェス確保の負荷のため不使用）

6. When 累積 opacity が 0.0 のエンティティを処理する時, the `composite_render_system` shall そのサブツリーの描画を完全にスキップする（完全透明の場合の早期脱出）

7. The `composite_render_system` shall 合成描画完了後、`CopyFromBitmap` で合成ビットマップからステージングビットマップへピクセルデータをコピーし、`WindowD3D11Compositor.set_dirty(true)` を設定する

8. The `composite_render_system` shall ウィンドウ内のいずれかのエンティティでコンポーネント変更（`GraphicsCommandList`、`GlobalArrangement`、`Visual` のいずれか）が検出された場合のみ、ウィンドウ全体の再合成を実行する

9. The `composite_render_system` shall 既存のウィジェット描画システム群（`draw_rectangles`, `draw_labels`, `draw_typewriters`, `draw_bitmap_sources`）を一切変更せず、それらが出力した `GraphicsCommandList` を合成入力として消費する

10. The `composite_render_system` shall `ecs/graphics/compositor_systems.rs` に配置される

### Requirement 3: 合成リソース初期化システム（compositor_init_system）

**Objective:** 開発者として、HWND を持つウィンドウエンティティに自動的に `WindowD3D11Compositor` を作成・アタッチするシステムが欲しい。これにより合成リソースのライフサイクルが ECS フレームワーク内で自動管理される。

_Parent: Req 3.1, 3.5, 5.4, 6.1_

#### Acceptance Criteria

1. When `WindowHandle` が新たにアタッチされたエンティティが検出された時, the `compositor_init_system` shall そのエンティティに `WindowD3D11Compositor` を作成・挿入する

2. The `compositor_init_system` shall `GraphicsCore` リソースから `ID2D1DeviceContext` を取得し、合成リソースの作成に使用する

3. When ウィンドウサイズが前フレームから変更されたことを検出した時, the `compositor_init_system` shall `WindowD3D11Compositor` のリサイズ処理を呼び出す（検出方式: `WindowD3D11Compositor` 内部の `cached_size` と `WindowPos` の比較）

4. When `HasGraphicsResources` の変更が検出されリソースが無効（`!is_valid()`）な時, the `compositor_init_system` shall `WindowD3D11Compositor` を再作成する（既存の `init_window_graphics` と同じ `Or<(Without<WindowD3D11Compositor>, Changed<HasGraphicsResources>)>` + `!is_valid()` パターン。`GraphicsCore` に generation フィールドは存在しない）

5. If リソース作成が失敗した場合, the `compositor_init_system` shall リソースを無効化（invalidate）し、`tracing::error` でエラー詳細をログ出力する（次フレームで再試行）

6. The `compositor_init_system` shall 0×0 サイズのウィンドウに対してリソース作成を試行しない

7. The `compositor_init_system` shall `ecs/graphics/compositor_systems.rs` に配置される

### Requirement 4: D2D → HBITMAP 転送ユーティリティ

**Objective:** 開発者として、D2D1 ステージングビットマップから GDI HBITMAP への高速ピクセル転送関数が欲しい。これにより Phase 3 での `UpdateLayeredWindow` 呼び出しの前提条件が整う。

_Parent: Req 3.1（合成パイプラインの一部として）_

#### Acceptance Criteria

1. The `transfer_to_hbitmap()` shall ステージング `ID2D1Bitmap1` のピクセルデータを `Map(D2D1_MAP_OPTIONS_READ)` でマップし、DIBSection メモリへコピーし、`Unmap()` する

2. When D2D1 Map の pitch と DIBSection の stride（`width * 4`）が異なる時, the `transfer_to_hbitmap()` shall 行単位のコピーを行う

3. When pitch と stride が一致する時, the `transfer_to_hbitmap()` shall `std::ptr::copy_nonoverlapping` による単一の連続メモリコピーで転送を最適化する

4. The `transfer_to_hbitmap()` shall `com/ulw.rs` モジュールに配置され、ECS 非依存の純粋ユーティリティ関数として実装する

5. If Map 操作が失敗した場合, the `transfer_to_hbitmap()` shall `windows::core::Result` でエラーを返却する

### Requirement 5: Phase 1 検証基準

**Objective:** 開発者として、Phase 1 の完了を客観的に判定できる検証基準が欲しい。

_Parent: Req 10.1, 10.2_

#### Acceptance Criteria

1. The `WindowD3D11Compositor::new()` shall 全4リソースを正しく作成できること（unit test で検証）

2. The `composite_render_system` shall 複数の `GraphicsCommandList` を z-order + transform で正しく合成描画できること（integration test で検証）

3. The `composite_render_system` shall 階層構造で opacity 累積を正確に実行できること（integration test: parent `Visual { opacity: 0.8, .. }` × child `Visual { opacity: 0.5, .. }` = final 0.4）

4. The `transfer_to_hbitmap()` shall pitch/stride 不一致パターンを含む転送を正しく実行できること（unit test で検証）

5. The 新モジュール群 shall `cargo test` で全テストがパスし、既存テストへの回帰がないこと

6. The 新パイプライン shall DComp パイプラインと共存状態で `cargo build` が成功すること

---

## 要件カバレッジサマリー

| 子仕様要件 | 親要件 | 概要 |
|-----------|--------|------|
| Req 1 | 3.1, 6.1 | WindowD3D11Compositor コンポーネント定義 |
| Req 2 | 3.1-3.4, 3.6 | composite_render_system（合成描画 + opacity 累積） |
| Req 3 | 3.1, 3.5, 5.4, 6.1 | compositor_init_system（初期化・サイズ変更・デバイスロスト） |
| Req 4 | 3.1 | D2D → HBITMAP 転送ユーティリティ |
| Req 5 | 10.1, 10.2 | Phase 1 検証基準 |

## 変更履歴

### v2.1 (2026-02-16): レビュー指摘修正

**事実誤認の修正**:
- Req 3 AC4: 「GraphicsCore の generation カウンタ」→ `Changed<HasGraphicsResources>` + `!is_valid()` パターンに修正。`GraphicsCore` に generation フィールドは存在しない（research.md v2 で確認済み）

### v2 (2026-02-16): 子仕様完了に伴う洗練

**Phase 0 完了反映**:
- 導入セクションに「前提条件の充足状況」テーブルを追加し、Phase 0 / child-order-fix の完了を明示
- Non-Goals から「Widget → Visual.opacity データフロー確立」の未来形表現を削除。`#[deprecated]` 済みであること、完全削除は Phase 4 で行うことを明記
- Req 2 AC3/AC4: Phase 0 で確立された `Visual.is_visible` / `Visual.opacity` データフローの利用を明示
- Req 5 AC3: `Visual.opacity` がデフォルト 1.0 という注記を削除（Phase 0 でデータフロー確立済み）。テストは `Visual { opacity: 値, .. }` で直接設定

**taffy-child-order-fix 完了反映**:
- Req 2 AC1: `Children` 兄弟順序が権威的ソースとして保証済みであることを明記

**要件の精緻化**:
- Req 1 AC2: `dirty` フラグ管理 API（`is_dirty()` / `set_dirty()`）を追加。Phase 3 インターフェースとして設計に存在していたが要件に未反映だった
- Req 1 AC5: `Option<Inner>` パターン、`Drop` 実装要件を新規追加（design.md から要件レベルに昇格）
- Req 2 AC6: 累積 opacity == 0.0 時のサブツリースキップを別 AC として分離（旧 AC4 から独立。design.md の `render_subtree()` ロジックに対応）
- Req 2 AC7: `CopyFromBitmap` と `set_dirty(true)` の呼び出しを明確化（旧 AC6 から精緻化）
- Req 3 AC3: リサイズ検出方式（`cached_size` vs `WindowPos` 比較）を明記（research.md Option A 推奨に対応）
- Req 3 AC4: デバイスロスト復旧パターンの参照元（`init_window_graphics`）を明記
- Req 3 親要件: 3.5, 5.4 を追加（リサイズ・デバイスロスト復旧に対応する親要件）
- Req 4 AC1/AC3: D2D API 詳細（`D2D1_MAP_OPTIONS_READ`, `std::ptr::copy_nonoverlapping`）を明記
- 旧 Req 2 を 10 AC に拡張（旧 AC は 9 個）、AC 番号の整合性を修正
