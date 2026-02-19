# 子仕様統合指針 — wintf-dcomp-to-layered-migration

> 本文書は親仕様 `wintf-dcomp-to-layered-migration` の design.md に基づき、4つの子仕様が共通参照する統合指針を定義する。子仕様の要件定義・設計・タスク生成時にはこの文書を必ず参照すること。

> **方針変更 (2026-02-19)**: Phase 4 は「DComp完全除去」から「切り替え式バックエンド実装」に変更。DCompコードは削除せず、CompositionMode enumによりULW/DCompをウィンドウ単位で切り替え可能にする。将来的にDCompをWinRT Compositorへ移行することも見据える。

---

## 1. DComp依存分類マップ

既存コードの全コンポーネント・システム・ファイルを **RED（廃止）/ YELLOW（改修）/ GREEN（再利用）** の3カテゴリに分類し、各子仕様の担当を明確にする。

### 1.1 コンポーネント分類

| コンポーネント                                | 分類     | 所属子仕様        | 対応                                                           | 要件カバー |
| --------------------------------------------- | -------- | ----------------- | -------------------------------------------------------------- | ---------- |
| WindowGraphics                                | 🔴 RED    | Phase 1 → Phase 4 | WindowD3D11Compositor に置換（ULWモード）、DCompモードでは維持 | Req 6.1    |
| VisualGraphics                                | 🟡 YELLOW | Phase 4           | DCompモード用に維持（CompositionMode::DComp時のみ使用）        | Req 6.3    |
| SurfaceGraphics                               | 🟡 YELLOW | Phase 4           | DCompモード用に維持（CompositionMode::DComp時のみ使用）        | Req 6.3    |
| SurfaceGraphicsDirty                          | 🟡 YELLOW | Phase 2           | ウィンドウ単位ダーティ管理に変更                               | Req 3.3    |
| SurfaceCreationStats                          | 🟡 YELLOW | Phase 4           | 名称・メトリクス変更                                           | —          |
| Visual                                        | 🟢 GREEN  | —                 | 変更なし（on_visual_addフックのみPhase 2で改修）               | Req 6.2    |
| GraphicsCommandList                           | 🟢 GREEN  | —                 | 完全再利用                                                     | Req 3.4    |
| Arrangement                                   | 🟢 GREEN  | —                 | 変更なし                                                       | —          |
| GlobalArrangement                             | � GREEN  | —                 | 変更なし（座標変換専用、Opacityは保持しない）                  | —          |
| FrameTime                                     | 🟢 GREEN  | —                 | 変更なし                                                       | —          |
| TaffyStyle / TaffyComputedLayout              | 🟢 GREEN  | —                 | 変更なし                                                       | —          |
| Label / Rectangle / BitmapSource / Typewriter | 🟢 GREEN  | —                 | 変更なし                                                       | —          |
| HasGraphicsResources                          | 🟢 GREEN  | —                 | マーカーとして維持                                             | —          |

### 1.2 システム関数分類（全19関数 + visual_manager 5関数）

| #   | 関数名                                           | 分類     | 所属子仕様 | 移行後                                   |
| --- | ------------------------------------------------ | -------- | ---------- | ---------------------------------------- |
| 1   | `format_entity_name`                             | 🟢 GREEN  | —          | 維持                                     |
| 2   | `calculate_surface_size_from_global_arrangement` | 🟢 GREEN  | —          | 維持                                     |
| 3   | `create_window_graphics_for_hwnd`                | 🔴 RED    | Phase 1    | compositor_init_system に置換            |
| 4   | `create_surface_for_visual`                      | � YELLOW | Phase 4    | CompositionMode::DComp時のみ条件付き実行 |
| 5   | `draw_recursive` (dead_code)                     | 🔴 RED    | Phase 4    | 削除（dead_codeのため）                  |
| 6   | `render_surface`                                 | 🔴 RED    | Phase 1    | composite_render_system に置換           |
| 7   | `commit_composition`                             | 🔴 RED    | Phase 3    | ulw_present_system に置換                |
| 8   | `init_graphics_core`                             | 🟡 YELLOW | Phase 2    | DComp部分除去                            |
| 9   | `init_window_graphics`                           | 🔴 RED    | Phase 1    | compositor_init_system に置換            |
| 10  | `init_window_visual` (deprecated)                | � YELLOW | Phase 4    | CompositionMode::DComp時のみ条件付き実行 |
| 11  | `sync_surface_from_arrangement` (deprecated)     | 🟡 YELLOW | Phase 4    | CompositionMode::DComp時のみ条件付き実行 |
| 12  | `apply_window_pos_changes`                       | 🟢 GREEN  | —          | 維持                                     |
| 13  | `invalidate_dependent_components`                | 🟡 YELLOW | Phase 2    | コンポーネント型変更に追従               |
| 14  | `mark_dirty_surfaces`                            | 🟡 YELLOW | Phase 2    | ダーティ検出条件変更                     |
| 15  | `visual_hierarchy_sync_system`                   | 🔴 RED    | Phase 2    | 削除（Visual階層同期不要）               |
| 16  | `visual_property_sync_system`                    | 🔴 RED    | Phase 2    | 削除（合成描画でtransform適用）          |
| 17  | `deferred_surface_creation_system`               | 🔴 RED    | Phase 2    | 削除（個別Surface不要）                  |
| 18  | `cleanup_surface_on_commandlist_removed`         | 🔴 RED    | Phase 2    | 削除                                     |
| 19  | `resolve_inherited_brushes`                      | 🟢 GREEN  | —          | 維持                                     |
| VM1 | `insert_visual()`                                | 🟢 GREEN  | —          | 維持                                     |
| VM2 | `insert_visual_with()`                           | 🟢 GREEN  | —          | 維持                                     |
| VM3 | `create_visual_only()`                           | � YELLOW | Phase 4    | CompositionMode::DComp時のみ条件付き実行 |
| VM4 | `visual_resource_management_system`              | 🔴 RED    | Phase 2    | 削除                                     |
| VM5 | `window_visual_integration_system`               | 🔴 RED    | Phase 2    | 削除                                     |

**要約**: 24関数中 **11関数がRED（廃止/置換）**、6関数がYELLOW（改修/条件付き維持）、7関数がGREEN（維持）

### 1.3 ファイル分類

| ファイル                                 | 分類                | 所属子仕様 | 対応                                                                                          |
| ---------------------------------------- | ------------------- | ---------- | --------------------------------------------------------------------------------------------- |
| `com/dcomp.rs` (315行)                   | � GREEN             | —          | **維持**（DCompモード用）                                                                     |
| `ecs/graphics/visual_manager.rs` (170行) | 🟢 GREEN             | —          | **維持**（DCompモード用）                                                                     |
| `ecs/graphics/core.rs`                   | 🟡 YELLOW            | Phase 2    | DComp初期化除去                                                                               |
| `ecs/graphics/components.rs`             | 🔴 RED / 🟢 GREEN混在 | Phase 1-4  | WindowGraphics→WindowD3D11Compositor置換、VisualGraphics/SurfaceGraphicsはDCompモード用に維持 |
| `ecs/graphics/systems.rs` (1419行)       | 🔴 RED / 🟢 GREEN混在 | Phase 1-4  | REDシステム置換・削除                                                                         |
| `ecs/world.rs`                           | 🟡 YELLOW            | Phase 2    | スケジュール定義更新                                                                          |
| `ecs/window.rs`                          | 🟡 YELLOW            | Phase 3    | WS_EX_LAYERED切替                                                                             |
| `ecs/window_proc/handlers.rs`            | 🟡 YELLOW            | Phase 3    | WM_PAINT/WM_SIZE更新                                                                          |
| `areka/src/main.rs`                      | 🟡 YELLOW            | Phase 3    | WS_EX_NOREDIRECTIONBITMAP→WS_EX_LAYERED                                                       |
| `examples/dcomp_demo.rs`                 | � GREEN             | —          | **維持**（DCompバックエンド検証用）                                                           |
| `com/d2d/` 全体                          | 🟢 GREEN             | —          | 完全再利用                                                                                    |
| `com/dwrite.rs`                          | 🟢 GREEN             | —          | 完全再利用                                                                                    |
| `com/wic.rs`                             | 🟢 GREEN             | —          | 完全再利用                                                                                    |
| `ecs/layout/` 全体                       | 🟢 GREEN             | —          | 完全再利用                                                                                    |
| `ecs/widget/` 全体                       | 🟢 GREEN             | —          | 完全再利用                                                                                    |
| `ecs/pointer/`, `ecs/drag/`              | 🟢 GREEN             | —          | 完全再利用                                                                                    |

---

## 2. コンポーネント・システム所属マップ

各新規コンポーネント・システムがどの子仕様に所属するかを明確にする。

### 2.1 新規コンポーネント

| コンポーネント        | 所属子仕様 | Phase | 要件カバー             | 備考                           |
| --------------------- | ---------- | ----- | ---------------------- | ------------------------------ |
| WindowD3D11Compositor | Phase 1    | 1     | Req 3.1, 3.5, 4.1, 6.1 | per-window合成リソース統合管理 |

### 2.2 新規システム

| システム                | 所属子仕様 | Phase | Stage             | 要件カバー        | 備考                    |
| ----------------------- | ---------- | ----- | ----------------- | ----------------- | ----------------------- |
| compositor_init_system  | Phase 1    | 1     | GraphicsSetup     | Req 3.1, 6.1      | Phase 2でworld.rsに登録 |
| composite_render_system | Phase 1    | 1     | RenderSurface     | Req 3.1-3.4       | Phase 2でworld.rsに登録 |
| ulw_present_system      | Phase 3    | 3     | CommitComposition | Req 4.1, 4.4, 4.5 | ULW統合時に登録         |

### 2.3 新規COMモジュール

| モジュール   | 所属子仕様                          | Phase | 要件カバー        | 備考                                                              |
| ------------ | ----------------------------------- | ----- | ----------------- | ----------------------------------------------------------------- |
| `com/ulw.rs` | Phase 1（構造）→ Phase 3（ULW実装） | 1, 3  | Req 4.1, 4.2, 4.4 | Phase 1で`transfer_to_hbitmap`、Phase 3で`present_layered_window` |

---

## 3. インターフェース契約

### 3.1 子仕様間の依存方向

```
Phase 1 ← Phase 2 ← Phase 3 ← Phase 4
(D2D1合成)  (DComp置換)  (ULW統合)  (切替式バックエンド)
```

- **矢印の意味**: `A ← B` は「B が A の完了を前提とする」
- **Phase 1 → Phase 2**: Phase 2 は Phase 1 で作成した新コンポーネント・システム（WindowD3D11Compositor, composite_render_system）を world.rs に登録する
- **Phase 2 → Phase 3**: Phase 3 は DComp パイプラインが無効化済み（Phase 2 完了）を前提に ULW 統合する
- **Phase 3 → Phase 4**: Phase 4 は ULW パイプラインが完全稼働（Phase 3 完了）を前提に、CompositionMode enumによるULW/DComp切り替えを実装する

### 3.2 Phase 1 が提供する契約（Phase 1 → Phase 2 境界）

Phase 2 は Phase 1 から以下の成果物を消費する:

| 成果物                                  | 用途                                          | 消費者                                                              |
| --------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------- |
| `WindowD3D11Compositor` コンポーネント  | per-window合成リソース                        | compositor_init_system, composite_render_system, ulw_present_system |
| `compositor_init_system` 関数           | ウィンドウ初期化                              | world.rs GraphicsSetup Stage                                        |
| `composite_render_system` 関数          | 合成描画                                      | world.rs RenderSurface Stage                                        |
| `composite_render_system` 関数          | 合成描画（CompositeContextでopacity手動累積） | world.rs RenderSurface Stage                                        |
| `com/ulw.rs` の `transfer_to_hbitmap()` | ステージング→HBITMAP転送                      | ulw_present_system（Phase 3で使用）                                 |
| `ecs/graphics/compositor.rs`            | コンポーネント定義モジュール                  | Phase 2以降の全子仕様                                               |
| `ecs/graphics/compositor_systems.rs`    | システム定義モジュール                        | Phase 2以降の全子仕様                                               |

### 3.3 Phase 2 が提供する契約（Phase 2 → Phase 3 境界）

Phase 3 は Phase 2 から以下の状態を前提とする:

| 状態                         | 意味                                                  | 検証方法                                                              |
| ---------------------------- | ----------------------------------------------------- | --------------------------------------------------------------------- |
| DComp API 呼び出しゼロ       | ECSシステムからDComp APIが一切呼ばれない              | `grep -r "IDComposition" ecs/` がゼロ件（※`com/dcomp.rs` 自体は残存） |
| 新パイプライン稼働           | compositor_init → composite_render の完全パイプライン | 全example動作確認                                                     |
| GraphicsCore DComp除去       | GraphicsCoreInnerからDCompフィールド削除済み          | コンパイル検証                                                        |
| on_visual_add フック更新済み | VisualGraphics/SurfaceGraphics挿入除去                | コード検査                                                            |

### 3.4 Phase 3 が提供する契約（Phase 3 → Phase 4 境界）

Phase 4 は Phase 3 から以下の状態を前提とする:

| 状態                     | 意味                                                                                      | 検証方法                                  |
| ------------------------ | ----------------------------------------------------------------------------------------- | ----------------------------------------- |
| ULW パイプライン完全稼働 | UpdateLayeredWindow による描画が全ウィンドウで動作                                        | alpha透過 + クリックスルー動作確認        |
| WS_EX_LAYERED 適用済み   | 全ウィンドウのex_styleが更新済み                                                          | コード検査                                |
| DComp コード未使用       | `com/dcomp.rs` は存在するが ECS スケジュールからは参照されない（Phase 4で条件付き再登録） | `use` 文がゼロ（`com/dcomp.rs` 自体以外） |

### 3.5 公開 API への影響

wintf クレートの公開 API (`api.rs`) への影響:

- **変更なし**: ECS World 構築、ウィンドウ作成、ウィジェット挿入、入力処理の公開インターフェース
- **内部変更のみ**: 描画パイプラインの切り替え（DComp→D2D1合成→ULW）は全て内部実装の変更
- **WindowStyle 変更**: `WS_EX_NOREDIRECTIONBITMAP` → `WS_EX_LAYERED`（デフォルト値変更は破壊的だが、外部クレートへの影響は areka のみ）

---

## 4. 共有リソースカタログ

複数の子仕様にまたがって参照されるコンポーネント・リソースを整理する。

### 4.1 Phase 1 定義の共有リソース

| リソース                               | 定義元          | 参照元        | 役割                                                              |
| -------------------------------------- | --------------- | ------------- | ----------------------------------------------------------------- |
| `WindowD3D11Compositor`                | Phase 1         | Phase 2, 3, 4 | per-window合成リソース統合コンポーネント                          |
| `compositor.rs` モジュール             | Phase 1         | Phase 2, 3, 4 | コンポーネント定義                                                |
| `compositor_systems.rs` モジュール     | Phase 1         | Phase 2, 3    | システム定義                                                      |
| `com/ulw.rs` モジュール                | Phase 1（部分） | Phase 3       | ULW ユーティリティ                                                |
| `CompositeContext.accumulated_opacity` | Phase 1         | Phase 2, 3, 4 | 合成描画時のOpacity手動累積（ローカル変数、コンポーネント非保存） |

### 4.2 既存 wintf リソース（全子仕様が参照可能）

| リソース               | 定義元モジュール             | 参照する子仕様 | 役割                    |
| ---------------------- | ---------------------------- | -------------- | ----------------------- |
| `GraphicsCore`         | ecs/graphics/core.rs         | Phase 1, 2     | D2D デバイス管理        |
| `GraphicsCommandList`  | ecs/graphics/command_list.rs | Phase 1        | ウィジェット描画データ  |
| `GlobalArrangement`    | ecs/layout/                  | Phase 1, 2     | 累積座標変換            |
| `Visual`               | ecs/graphics/components.rs   | Phase 1, 2     | 可視性・ローカルOpacity |
| `WindowHandle`         | ecs/window.rs                | Phase 1, 3     | HWND保持                |
| `WindowStyle`          | ecs/window.rs                | Phase 3        | WS_EX_LAYERED設定       |
| `Children` / `ChildOf` | bevy_ecs                     | Phase 1        | z-orderツリー走査       |

### 4.3 COM層リソース（全子仕様が参照可能）

| リソース                                  | 定義元        | 参照する子仕様 | 役割                            |
| ----------------------------------------- | ------------- | -------------- | ------------------------------- |
| `ID2D1DeviceContext`                      | com/d2d/      | Phase 1        | 合成描画用DC                    |
| `ID2D1Bitmap1`                            | com/d2d/      | Phase 1        | 合成ビットマップ + ステージング |
| `ID2D1CommandList`                        | com/d2d/      | Phase 1        | ウィジェット描画コマンド        |
| `BLENDFUNCTION` / `ULW_ALPHA`             | windows crate | Phase 3        | ULW パラメータ                  |
| `CreateDIBSection` / `CreateCompatibleDC` | windows crate | Phase 1        | GDI リソース                    |

---

## 5. 依存グラフと実装順序

### 5.1 Phase 構成

```
Phase 1 (基盤構築 — 新モジュール並行追加)
└── wintf-dcomp-migration-1-d2d1-composition  ← 依存なし

Phase 2 (パイプライン切替)
└── wintf-dcomp-migration-2-pipeline-switch   ← Phase 1 に依存

Phase 3 (ULW統合)
└── wintf-dcomp-migration-3-ulw-integration   ← Phase 2 に依存

Phase 4 (切替式バックエンド実装)
└── wintf-dcomp-migration-4-switchable-backend     ← Phase 3 に依存
```

### 5.2 各 Phase 完了時の検証可能状態

| Phase        | 完了後に検証可能な機能                                                                            | DComp状態                                 |
| ------------ | ------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| Phase 1 完了 | WindowD3D11Compositor リソース作成、合成描画（独立テスト）、CompositeContextによるopacity手動累積 | **稼働中** — 変更なし                     |
| Phase 2 完了 | 全exampleがD2D1合成パイプラインで動作、DComp API呼出ゼロ                                          | **無効化** — コード残存だがSchedule未登録 |
| Phase 3 完了 | UpdateLayeredWindow透過描画、alpha=0クリックスルー、WM_SIZEリサイズ                               | **無効化** — ULWで全描画                  |
| Phase 4 完了 | CompositionMode切替完全動作、両パイプラインでexample動作確認                                      | **切替可能** — CompositionMode enumで選択 |

### 5.3 Phase 1-2 の並行稼働戦略

Phase 1-2 では新旧パイプラインが**並行して存在**する（ハイブリッド段階アプローチ）:

- **Phase 1**: 新モジュール（`compositor.rs`, `compositor_systems.rs`, `com/ulw.rs`）を並行追加。world.rs には**登録しない**。独立テスト環境で検証
- **Phase 2**: world.rs のシステム登録を新パイプラインに切り替え。旧DCompシステムの登録を解除するが、**コード自体は残存**（参照用）
- **ロールバック**: Phase 2 で問題発生時、world.rs の登録を旧システムに戻すだけで即座にロールバック可能

---

## 6. 技術リファレンス

### 6.1 D2D1 Bitmap Options

**合成描画先ビットマップ（composition_bitmap）**:
```
D2D1_BITMAP_PROPERTIES1:
  pixelFormat: { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED }
  bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET
```

**CPUステージングビットマップ（staging_bitmap）**:
```
D2D1_BITMAP_PROPERTIES1:
  pixelFormat: { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED }
  bitmapOptions: D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW
```

### 6.2 BLENDFUNCTION for UpdateLayeredWindow

```
BLENDFUNCTION:
  BlendOp: AC_SRC_OVER
  BlendFlags: 0
  SourceConstantAlpha: 255
  AlphaFormat: AC_SRC_ALPHA
```

### 6.3 DIBSection 作成パラメータ

```
BITMAPINFOHEADER:
  biSize: sizeof(BITMAPINFOHEADER)
  biWidth: window_width
  biHeight: -(window_height as i32)   // top-down DIB (negative height)
  biPlanes: 1
  biBitCount: 32
  biCompression: BI_RGB
```

> **注意**: `biHeight` を負にすることでtop-down DIBになり、D2D1のピクセルレイアウト（top-down）とメモリレイアウトが一致する。これにより行の反転コピーが不要になる。

### 6.4 D2D → HBITMAP 転送パス（設計決定: Option B — GPU Render + CPU Map）

```
ID2D1Bitmap1 (RenderTarget)
  → CopyFromBitmap → ID2D1Bitmap1 (Staging, CPU_READ)
    → Map() → memcpy row-by-row → HBITMAP (DIBSection, PBGRA32)
      → SelectObject(MemoryDC) → UpdateLayeredWindow(hwnd, hdcSrc, ULW_ALPHA)
```

**stride/pitch 注意**: D2D1 `Map()` の pitch と DIBSection の stride（`width * 4`）が異なる場合は行単位コピーが必要。一致する場合は単一 `memcpy` で最適化可能。

### 6.5 Opacity階層累積（CompositeContext 手動累積方式）

```
accumulated_opacity = parent_accumulated_opacity * child.Visual.opacity
```

- `accumulated_opacity` ∈ `[0.0, 1.0]`（clamp）
- `Visual.is_visible == false` の場合: サブツリーごとスキップ（描画スキップ最適化）
- `composite_render_system` の `render_subtree()` 再帰走査中に動的に計算（ECSコンポーネントには保存しない）
- **PushLayer は不使用**（中間サーフェス確保による負荷が大きいため）

### 6.6 z-order ソートアルゴリズム

合成描画の描画順序は **depth-first pre-order 走査**（画家のアルゴリズム）:

```
Root → Child1 → Child1.Child1 → Child1.Child2 → Child2 → ...
```

- 先に描いたものが背景、後に描いたものが前景
- `Children` コンポーネントから再帰的に走査
- 各エンティティの `GlobalArrangement.transform` で `SetTransform` → `DrawImage(CommandList)` with `CompositeContext.accumulated_opacity`

---

## 7. Schedule Stage 再構成指針

### 7.1 ステージ別変更マップ

| Stage                 | 現行システム                                                                               | 新パイプライン                    | Phase | 変更種別     |
| --------------------- | ------------------------------------------------------------------------------------------ | --------------------------------- | ----- | ------------ |
| Input                 | 変更なし                                                                                   | 変更なし                          | —     | —            |
| Update                | invalidate_dependent_components                                                            | コンポーネント型変更に追従        | 2     | 軽微改修     |
| **PreLayout**         | visual_resource_management (RED), visual_hierarchy_sync (RED), init_graphics_core (YELLOW) | init_graphics_core（DComp除去版） | 2     | RED削除      |
| Layout                | 変更なし（taffy系4システム）                                                               | 変更なし                          | —     | —            |
| PostLayout            | 変更なし（arrangement伝播系5システム）                                                     | 変更なし                          | —     | —            |
| UISetup               | 変更なし                                                                                   | 変更なし                          | —     | —            |
| **GraphicsSetup**     | init_window_graphics (RED), window_visual_integration (RED)                                | **compositor_init_system**        | 2     | 全面置換     |
| **Draw**              | deferred_surface_creation (RED), cleanup_surface (RED), ウィジェット描画 (GREEN)           | ウィジェット描画のみ              | 2     | RED削除      |
| PreRenderSurface      | mark_dirty_surfaces (YELLOW)                                                               | ウィンドウ単位ダーティ検出        | 2     | 改修         |
| **RenderSurface**     | render_surface (RED)                                                                       | **composite_render_system**       | 2     | 全面置換     |
| **Composition**       | visual_property_sync (RED)                                                                 | **空化**                          | 2     | ステージ空化 |
| **CommitComposition** | commit_composition (RED)                                                                   | **ulw_present_system**            | 3     | 全面置換     |
| FrameFinalize         | 変更なし                                                                                   | 変更なし                          | —     | —            |

### 7.2 Phase 別 Schedule 変更タイムライン

**Phase 1**: world.rs 変更なし。PostLayout の `propagate_global_arrangements` にOpacity累積ロジックのみ追加。新システムは**登録しない**（独立テスト）

**Phase 2**: world.rs の以下を変更:
- PreLayout: `visual_resource_management`, `visual_hierarchy_sync` を削除
- GraphicsSetup: `init_window_graphics`, `window_visual_integration` → `compositor_init_system`
- Draw: `deferred_surface_creation`, `cleanup_surface` を削除
- RenderSurface: `render_surface` → `composite_render_system`
- Composition: `visual_property_sync` を削除（ステージ空化）

**Phase 3**: world.rs の以下を変更:
- CommitComposition: `commit_composition` → `ulw_present_system`

**Phase 4**: DCompシステムのCompositionMode条件付き再登録、ULW/DComp切替ロジック実装

---

## 8. エラーハンドリング共通指針

### 8.1 エラー戦略マトリクス

| エラー                                     | 発生元                          | 担当子仕様 | レスポンス                                             | リカバリ              |
| ------------------------------------------ | ------------------------------- | ---------- | ------------------------------------------------------ | --------------------- |
| D2D Bitmap作成失敗                         | compositor_init_system          | Phase 1    | `tracing::error` + invalidate()                        | 次フレーム再作成      |
| BeginDraw/EndDraw失敗                      | composite_render_system         | Phase 1    | `tracing::error` + フレームスキップ                    | 次フレーム再描画      |
| CopyFromBitmap失敗                         | composite_render_system         | Phase 1    | `tracing::error` + フレームスキップ                    | 次フレーム再試行      |
| Map失敗                                    | ulw_present_system              | Phase 3    | `tracing::error` + フレームスキップ                    | 次フレーム再試行      |
| UpdateLayeredWindow失敗                    | ulw_present_system              | Phase 3    | `tracing::warn` + 次フレーム再試行                     | Req 4.5: 自動リトライ |
| デバイスロスト (DXGI_ERROR_DEVICE_REMOVED) | 任意のD2D操作                   | Phase 1, 2 | `GraphicsCore::invalidate()` → 全Compositor invalidate | 既存フロー維持        |
| リサイズ時Bitmap作成失敗                   | WindowD3D11Compositor::resize() | Phase 1    | `tracing::error` + 旧サイズ維持                        | 次回リサイズで再試行  |

### 8.2 デバイスロスト対応指針

全子仕様共通のデバイスロスト対応パターン:

1. `GraphicsCore.is_valid()` を監視（既存 `init_graphics_core` システム）
2. デバイスロスト検出時: `GraphicsCore::invalidate()` → `HasGraphicsResources.set_changed()` → 全GPUリソースコンポーネントの再初期化トリガー
3. `WindowD3D11Compositor` は `generation` カウンタでリソース世代を追跡。`compositor_init_system` で世代不一致を検出して再作成
4. Phase 2 完了後: DComp再初期化ステップが省略されるため、リカバリフローが**簡素化**される

---

## 9. テスト責務マトリクス

### 9.1 単体テスト（各子仕様の lib tests / unit tests）

| テスト対象                           | 担当子仕様 | テスト概要                                                   |
| ------------------------------------ | ---------- | ------------------------------------------------------------ |
| CompositeContext opacity 手動累積    | Phase 1    | parent 0.8 × child 0.5 = 0.4, is_visible=false → skip, clamp |
| WindowD3D11Compositor ライフサイクル | Phase 1    | new() / resize() / invalidate() / is_valid()                 |
| transfer_to_hbitmap stride処理       | Phase 1    | pitch≠stride時の行単位コピー検証                             |
| present_layered_window BLENDFUNCTION | Phase 3    | BLENDFUNCTION構成テスト                                      |
| GraphicsCore DComp除去               | Phase 2    | DCompフィールド・メソッドの不在確認                          |

### 9.2 統合テスト

| テストシナリオ                               | 担当子仕様 | 対応要件     |
| -------------------------------------------- | ---------- | ------------ |
| composite_render_system z-order合成          | Phase 1    | Req 3.1, 3.2 |
| 完全パイプライン統合（init→render→transfer） | Phase 1    | Req 3.1-3.4  |
| デバイスロスト後の自動再初期化               | Phase 1    | Req 5.4      |
| ウィンドウリサイズ後の合成Bitmap再作成       | Phase 1    | Req 3.5      |
| ULW透過描画＋クリックスルー                  | Phase 3    | Req 4.1, 4.3 |
| ULW失敗時のリトライ動作                      | Phase 3    | Req 4.5      |

### 9.3 E2E テスト（Phase-specific 検証基準）

| Phase   | 検証基準                                                                                                | 具体的確認手段                       | 対応要件 |
| ------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------ | -------- |
| Phase 1 | `taffy_flex_demo` 相当の描画が新パイプラインで動作                                                      | 独立テスト環境での目視確認           | Req 10.1 |
| Phase 2 | 全既存example（taffy_flex_demo, typewriter_demo, multi_window_test, split_image）が新パイプラインで動作 | `cargo run --example` 全example実行  | Req 10.1 |
| Phase 3 | ULW透過表示 + alpha=0クリックスルー                                                                     | 実機での透過ウィンドウ操作確認       | Req 10.1 |
| Phase 4 | CompositionMode切替動作確認（ULW/DComp両モード）                                                        | `cargo test` + 各モードでexample実行 | Req 10.1 |

### 9.4 描画品質基準（Req 10.3）

- DComp方式とULW方式で最終的なピクセル出力が同一であることは**保証しない**
- 許容基準: 人間の目視で差異が認識できないレベル（サブピクセルアンチエイリアシングの微差は許容）
- GPU→CPU転送時のフォーマットはPBGRA32で同一のため、理論上の品質劣化はない
- D2D DeviceContext（ハードウェアアクセラレーション）を合成描画に使用するため、品質はDComp方式と同等

---

## 10. 既存仕様影響評価

### 10.1 wintf-P0-click-through-rgn（影響度: 高）

| click-through-rgnの要件             | ULW移行後の状態                                              |
| ----------------------------------- | ------------------------------------------------------------ |
| SetWindowRgn ベースのクリックスルー | ❌ ULW_ALPHAで自動実現 → **不要**                             |
| GlobalArrangement.bounds → HRGN構築 | ❌ alpha=0が自動クリックスルー → **不要**                     |
| グリッドスナップサイズ（4x4px）     | ❌ ピクセル精度のalpha判定 → **不要**                         |
| タイマーベースのリージョン更新      | ❌ フレームごとのULW更新で十分 → **不要**                     |
| WS_EX_NOREDIRECTIONBITMAP互換性検証 | ❌ WS_EX_NOREDIRECTIONBITMAPが廃止 → **不要**                 |
| HitTestMode::NamedRegions精密制御   | ⚠ **残存可能性** — 特定領域の非透過クリック制御              |
| NCHITTEST二層アーキテクチャ         | ⚠ **縮小** — ULW alpha=0がfirst-pass、NCHITTESTがsecond-pass |

**対応方針**: 両仕様は**競争的並走**として独立進行（Req 8.1）。CTRが十分な性能を示す場合は本仕様凍結の可能性あり。逆に本仕様完了時はCTRの大部分が不要化する。

### 10.2 wintf-P0-animation-system（影響度: 低）

- DComp Animation API（`CreateAnimation()`等）は `dcomp_demo.rs` でのみ使用、ECSシステムでは未使用
- dolaクレートのアニメーション値→ECSコンポーネント（`Visual.opacity`, Offset等）→描画システム のフローは維持
- **影響なし**: dolaアニメーションの出力先がDComp Visual PropertiesからD2D合成描画パラメータに変わるのみ

### 10.3 wintf-P0-balloon-system（影響度: 中）

- バルーンウィンドウもULW方式に移行（ウィンドウ単位の描画）
- ウィジェット描画（テキスト、背景矩形等）はGREENなので、パイプライン変更完了後はそのまま動作
- **推奨**: ULW移行（本仕様）を先に完了させ、その上でバルーン仕様を実装

---

## 11. 技術リスクと緩和策

### 11.1 D2D1Bitmap → HBITMAP 変換パス（HIGH）

- **状況**: 現コードベースに完全な変換パスがゼロ。ULW方式の核心
- **影響**: Phase 1 の主要実装項目
- **緩和策**: Option B（GPU Render + CPU Map）を採用済み。`com/wic.rs` の `CopyPixels` 既存実装がベースになる可能性
- **Phase 1 早期検証**: 最小構成での D2D →Map→ memcpy →DIBSection→ULW 技術検証を実施

### 11.2 合成描画パフォーマンス（MEDIUM）

- **状況**: DComp方式はGPU並列合成。ULW方式はCPU側で全ウィジェットを順次合成
- **緩和策**: デスクトップマスコットのウィジェット数は限定的（数十個）。合成描画自体はD2D DeviceContext（GPU）で実行しGPU→CPUコピーのみがボトルネック候補
- **デスクトップマスコット規模（~500x500px）ではオーバーヘッド無視可能**

### 11.3 WS_EX_LAYERED での WM_PAINT 動作（LOW）

- **状況**: `WS_EX_LAYERED` ウィンドウは一般にWM_PAINTを受信しない
- **Phase 3 前提検証**: 最小構成の WS_EX_LAYERED ウィンドウで WM_PAINT ハンドラの発火動作を確認。描画は基本的にULW駆動を想定
- **設計への影響**: 未発火確認の場合、handlers.rsのWM_PAINTハンドラが不要化（簡素化）

---

## 12. 子仕様作成ガイドライン

### 12.1 仕様サイクルの進め方

各子仕様は以下の工程で作成する:

1. **init**: `.kiro/specs/{child-spec-name}/spec.json` を作成
2. **requirements**: 親仕様の該当要件を子仕様の粒度に詳細化（番号体系は子仕様で独立）
3. **design**: 親仕様の design.md と本統合指針から該当セクションを抽出・詳細化
4. **tasks**: 子仕様の実装タスクを生成（ここで初めてコード実装レベルの粒度）

### 12.2 子仕様での要件番号体系

- 親仕様の要件番号（Req 1〜10）は各子仕様の requirements.md 内でトレーサビリティとして参照する
- 子仕様独自の要件番号体系を採用する（例: Phase 1 の Req 1, 2, 3...）
- 子仕様 requirements.md の各要件に `_Parent: Req X.Y_` 形式で親要件への逆参照を記載する

### 12.3 design.md からの抽出範囲

| 子仕様                       | 抽出するセクション                                                                                                                                                                                |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase 1 (D2D1合成)           | WindowD3D11Compositor コンポーネント, composite_render_system, compositor_init_system, CompositeContext opacity手動累積, com/ulw.rs (transfer_to_hbitmap), D2D→HBITMAP転送パス, Opacity累積フロー |
| Phase 2 (DComp置換)          | GraphicsCore改修, Schedule Stage再構成, on_visual_addフック, DCompシステム登録解除, invalidate_dependent_components改修, mark_dirty_surfaces改修                                                  |
| Phase 3 (ULW統合)            | UlwTransfer (present_layered_window), ulw_present_system, WS_EX_LAYERED切替, WM_PAINT/WM_SIZE ハンドラ更新, areka/src/main.rs更新                                                                 |
| Phase 4 (切替式バックエンド) | CompositionMode enum定義, DCompシステム条件付き再登録, ULW/DComp切替ロジック, WinRT Compositor拡張計画, テスト修正                                                                                |

### 12.4 子仕様名の命名規則

| Phase | 子仕様名                                     | 説明                    |
| ----- | -------------------------------------------- | ----------------------- |
| 1     | `wintf-dcomp-migration-1-d2d1-composition`   | D2D1合成スタック構築    |
| 2     | `wintf-dcomp-migration-2-pipeline-switch`    | DCompパイプライン置換   |
| 3     | `wintf-dcomp-migration-3-ulw-integration`    | UpdateLayeredWindow統合 |
| 4     | `wintf-dcomp-migration-4-switchable-backend` | 切替式バックエンド実装  |

---

## 13. 要件カバレッジトレーサビリティ

| 親要件 | 概要                       | 担当子仕様                | 本文書の該当セクション                 |
| ------ | -------------------------- | ------------------------- | -------------------------------------- |
| Req 1  | 影響範囲の特定と分類       | 全Phase（分類は§1で完了） | §1 DComp依存分類マップ                 |
| Req 2  | 段階的移行戦略             | Phase 1-4                 | §5 依存グラフ, §7 Schedule再構成       |
| Req 3  | 新描画パイプライン         | Phase 1, 2                | §2 所属マップ, §6 技術リファレンス     |
| Req 4  | ULW統合                    | Phase 3                   | §6.2 BLENDFUNCTION, §6.4 転送パス      |
| Req 5  | GraphicsCore合成モード対応 | Phase 2, 4                | §3.3 Phase 2契約                       |
| Req 6  | ECSコンポーネント再設計    | Phase 1, 2, 4             | §1.1 コンポーネント分類, §2 所属マップ |
| Req 7  | メッセージハンドリング     | Phase 3                   | §11.3 WM_PAINTリスク                   |
| Req 8  | 既存仕様影響               | —                         | §10 既存仕様影響評価                   |
| Req 9  | 子仕様構成                 | —                         | §5 依存グラフ, §12 作成ガイドライン    |
| Req 10 | テスト・検証戦略           | Phase 1-4                 | §9 テスト責務マトリクス                |
