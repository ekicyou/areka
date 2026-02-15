# ギャップ分析レポート: wintf-dcomp-to-layered-migration

## 分析概要

DirectComposition（DComp）描画パイプラインからUpdateLayeredWindow（ULW）方式への全面移行について、既存コードベースとの実装ギャップを包括的に分析した。

### 主要発見

- **再利用可能資産: ~70%** — ウィジェット描画、レイアウト、入力、D2D/DWrite/WIC のCOMラッパー全てがDComp非依存
- **廃止/置換対象: ~15%** — `ecs/graphics/` 内の合成パイプライン（10システム関数）と `com/dcomp.rs`
- **改修対象: ~15%** — GraphicsCore初期化、コンポーネント定義、スケジュール登録、ウィンドウスタイル
- **最大の新規実装**: D2D1合成描画エンジンおよびD2D→HBITMAP→ULW変換パス（現コードベースにゼロ）

---

## 1. 現状調査（Current State Investigation）

### 1.1 ディレクトリ構成とアーキテクチャ

```
crates/wintf/src/
├── com/                    # COMラッパー層（LOW LEVEL）
│   ├── dcomp.rs           # ★ DComp API (315行) — 廃止対象
│   ├── d3d11.rs           # D3D11 (76行) — 維持
│   ├── d2d/               # D2D API — 完全維持
│   │   ├── mod.rs, command.rs, ...
│   ├── dwrite.rs          # DirectWrite — 維持
│   └── wic.rs             # WIC — 維持（HBITMAP変換は新規追加）
├── ecs/                    # ECSコンポーネント・システム層
│   ├── graphics/          # ★ 描画パイプライン — 主要変更範囲
│   │   ├── core.rs        # GraphicsCore (147行) — DComp初期化除去
│   │   ├── components.rs  # コンポーネント定義 (370行) — 大幅置換
│   │   ├── systems.rs     # 全描画システム (1419行) — 核心変更
│   │   ├── visual_manager.rs # Visual階層管理 (170行) — 全廃止
│   │   ├── command_list.rs   # CommandList (36行) — 完全維持
│   │   └── mod.rs
│   ├── layout/            # Taffyレイアウト — 完全維持
│   ├── widget/            # ウィジェット描画 — 完全維持
│   ├── pointer/           # ポインター入力 — 完全維持
│   ├── drag/              # ドラッグ — 完全維持
│   ├── window.rs          # ウィンドウ管理 — WS_EXスタイル変更
│   ├── world.rs           # ScheduleとSystem登録 (668行) — 登録更新
│   └── window_proc/       # メッセージハンドラ — コメント更新
└── ...                     # その他（api.rs, winproc.rs等）— 維持
```

### 1.2 デバイス初期化チェーン

**現行（DComp方式）** — `GraphicsCore::new()` (core.rs L37-L55):

```
D3D11CreateDevice → ID3D11Device
    ↓ cast
IDXGIDevice4
    ↓ D2D1CreateFactory
ID2D1Factory
    ↓ d2d_create_device(dxgi)
ID2D1Device → ID2D1DeviceContext (共有)
    ↓ DWriteCreateFactory
IDWriteFactory2
    ↓ dcomp_create_desktop_device(d2d)     ← ★廃止
IDCompositionDesktopDevice → IDCompositionDevice3  ← ★廃止
```

**目標（ULW方式）**:

```
D3D11CreateDevice → ID3D11Device
    ↓ cast
IDXGIDevice4
    ↓ D2D1CreateFactory
ID2D1Factory
    ↓ d2d_create_device(dxgi)
ID2D1Device → ID2D1DeviceContext (共有)
    ↓ DWriteCreateFactory
IDWriteFactory2
    （DComp初期化なし — 2ステップ削減）
```

### 1.3 GraphicsCoreInner フィールド (core.rs L18-L27)

| フィールド | 型 | 維持/廃止 |
|---|---|---|
| `d3d` | `ID3D11Device` | ✅ 維持 |
| `dxgi` | `IDXGIDevice4` | ✅ 維持 |
| `d2d_factory` | `ID2D1Factory` | ✅ 維持 |
| `d2d` | `ID2D1Device` | ✅ 維持 |
| `d2d_device_context` | `ID2D1DeviceContext` | ✅ 維持 |
| `dwrite_factory` | `IDWriteFactory2` | ✅ 維持 |
| `desktop` | `IDCompositionDesktopDevice` | ❌ 廃止 |
| `dcomp` | `IDCompositionDevice3` | ❌ 廃止 |

### 1.4 GraphicsCore メソッド (core.rs L36-L98)

| メソッド | DComp依存 | 対応 |
|---|---|---|
| `new()` | YELLOW | DComp初期化ステップを除去 |
| `invalidate()`, `is_valid()` | GREEN | そのまま維持 |
| `d2d_factory()`, `d2d_device()` | GREEN | 維持 |
| `dcomp()`, `desktop()` | RED | 廃止（メソッド削除） |
| `dwrite_factory()`, `device_context()` | GREEN | 維持 |
| `d3d()`, `dxgi()` | GREEN | 維持 |

---

## 2. ECSコンポーネント・カタログ

### 2.1 廃止対象コンポーネント（RED）

#### WindowGraphics (components.rs L28-L72)
| フィールド | 型 | 対応 |
|---|---|---|
| `target` | `IDCompositionTarget` | ❌ → ID2D1Bitmap1（合成ターゲット）+ MemDC/HBITMAP |
| `device_context` | `ID2D1DeviceContext` | ⚠ per-window DC — 維持だが用途変更 |
| `generation` | `u32` | ✅ 維持 |

#### VisualGraphics (components.rs L75-L151)
| フィールド | 型 | 対応 |
|---|---|---|
| `inner` | `Option<IDCompositionVisual3>` | ❌ 全廃止 |
| `parent_visual` | `Option<IDCompositionVisual3>` | ❌ 全廃止 |

- `on_visual_graphics_remove` フック内の `parent.remove_visual(visual)` も廃止

#### SurfaceGraphics (components.rs L153-L220)
| フィールド | 型 | 対応 |
|---|---|---|
| `inner` | `Option<IDCompositionSurface>` | ❌ 全廃止 |
| `size` | `(u32, u32)` | ⚠ 概念は維持可能だが、per-entity Surface → ウィンドウ単位に統合 |

### 2.2 改修対象コンポーネント（YELLOW）

| コンポーネント | 改修内容 |
|---|---|
| `SurfaceGraphicsDirty` | per-entity → ウィンドウ単位のダーティ管理に変更 |
| `Visual` | `on_visual_add` フック内のVisualGraphics/SurfaceGraphics配置を改修 |
| `SurfaceCreationStats` | 名称・メトリクスをComposition Bitmap基準に変更 |
| `HasGraphicsResources` | マーカーとして維持（変更なし） |

### 2.3 再利用可能コンポーネント（GREEN）

| コンポーネント | 根拠 |
|---|---|
| `GraphicsCommandList` (command_list.rs) | 純粋 `ID2D1CommandList` — DComp非依存 |
| `FrameTime` (core.rs L102-L147) | システムタイマー — DComp非依存 |
| Layout全体（`Arrangement`, `GlobalArrangement`, `TaffyStyle`, `TaffyComputedLayout`） | Taffyベース — DComp非依存 |
| Widget全体（`Label`, `Rectangle`, `BitmapSource`, `Typewriter`） | D2D CommandList生成のみ — DComp非依存 |

---

## 3. ECSシステム全カタログ（systems.rs 1419行）

### 3.1 全19システム関数の分類

| # | 関数名 | 行範囲 | DComp API | 分類 | 移行後 |
|---|---|---|---|---|---|
| 1 | `format_entity_name` | L28-L33 | なし | 🟢 GREEN | 維持 |
| 2 | `calculate_surface_size_from_global_arrangement` | L46-L68 | なし | 🟢 GREEN | 維持 |
| 3 | `create_window_graphics_for_hwnd` | L71-L91 | `desktop.create_target_for_hwnd()` | 🔴 RED | ULW版WindowGraphics初期化に置換 |
| 4 | `create_surface_for_visual` | L94-L118 | `dcomp.create_surface()`, `visual.set_content()` | 🔴 RED | 廃止 |
| 5 | `draw_recursive` (dead_code) | L124-L173 | なし | 🔴 RED | 削除 |
| 6 | **`render_surface`** | L184-L291 | `surface.begin_draw()`, `surface.end_draw()` | 🔴 RED | 合成描画システムに置換 |
| 7 | **`commit_composition`** | L294-L338 | `dcomp.commit()` | 🔴 RED | `UpdateLayeredWindow()` に置換 |
| 8 | `init_graphics_core` | L344-L411 | 内部で`GraphicsCore::new()`(DComp含む) | 🟡 YELLOW | DComp部分除去 |
| 9 | **`init_window_graphics`** | L416-L535 | `create_window_graphics_for_hwnd()` | 🔴 RED | ULW版window初期化に置換 |
| 10 | `init_window_visual` (deprecated) | L538-L553 | — | 🔴 RED | 削除（空関数） |
| 11 | `sync_surface_from_arrangement` (deprecated) | L564-L672 | `create_surface_for_visual()` | 🔴 RED | 削除（非使用） |
| 12 | `apply_window_pos_changes` | L677-L743 | なし (Win32 `SetWindowPos`) | 🟢 GREEN | 維持 |
| 13 | `invalidate_dependent_components` | L746-L777 | なし | 🟡 YELLOW | コンポーネント型変更に追従 |
| 14 | `mark_dirty_surfaces` | L793-L824 | なし | 🟡 YELLOW | ダーティ検出トリガー条件変更 |
| 15 | **`visual_hierarchy_sync_system`** | L841-L971 | `parent_visual.add_visual()` | 🔴 RED | 廃止（Visual階層不要） |
| 16 | **`visual_property_sync_system`** | L1000-L1091 | `visual.set_offset_x/y()`, `visual.set_opacity()` | 🔴 RED | 廃止（合成描画時にtransform適用） |
| 17 | **`deferred_surface_creation_system`** | L1110-L1241 | `dcomp.create_surface()`, `visual.SetContent()` | 🔴 RED | 廃止（個別Surface不要） |
| 18 | `cleanup_surface_on_commandlist_removed` | L1255-L1299 | `visual.SetContent(None)` | 🔴 RED | 廃止 |
| 19 | `resolve_inherited_brushes` | L1310-L1350 | なし | 🟢 GREEN | 維持 |

**要約**: 19関数中 **12関数がRED（廃止/置換）**、3関数がYELLOW（改修）、4関数がGREEN（維持）

### 3.2 visual_manager.rs 全関数

| 関数名 | DComp API | 分類 |
|---|---|---|
| `insert_visual()` | なし | 🟢 GREEN（Visualコンポーネント挿入ヘルパー） |
| `insert_visual_with()` | なし | 🟢 GREEN |
| `create_visual_only()` (private) | `dcomp.create_visual()` | 🔴 RED |
| `visual_resource_management_system` | `dcomp.create_visual()` 経由 | 🔴 RED |
| `window_visual_integration_system` | `target.SetRoot(visual)` | 🔴 RED |

---

## 4. Scheduleステージマッピング

### 4.1 実行順序（world.rs L604-L616）

```
Input → Update → PreLayout → Layout → PostLayout → UISetup
→ GraphicsSetup → Draw → PreRenderSurface → RenderSurface
→ Composition → CommitComposition → FrameFinalize
```

### 4.2 ステージ別DComp依存度

| Stage | DComp依存システム | 非依存システム | 総合判定 |
|---|---|---|---|
| Input | — | drain_task_pool_commands, dispatch_pointer_events, dispatch_drag_events | 🟢 完全GREEN |
| Update | invalidate_dependent_components (YELLOW) | detect_display_change, update_monitor_layout, update_typewriters | 🟡 軽微YELLOW |
| **PreLayout** | **visual_resource_management** (RED), **visual_hierarchy_sync** (RED) | init_graphics_core (YELLOW) | 🔴 主要RED |
| Layout | — | build_taffy_styles, sync_taffy_tree, compute_taffy_layout, update_arrangements | 🟢 完全GREEN |
| PostLayout | — | sync_window_arrangement, sync_simple_arrangements, propagate_global_arrangements, window_pos_sync | 🟢 完全GREEN |
| UISetup | — | create_windows, apply_window_pos_changes | 🟢 完全GREEN |
| **GraphicsSetup** | **init_window_graphics** (RED), **window_visual_integration** (RED) | — | 🔴 全RED |
| **Draw** | **deferred_surface_creation** (RED), **cleanup_surface** (RED) | resolve_inherited_brushes (G), draw_rectangles (G), draw_labels (G), draw_bitmap_sources (G), generate_alpha_mask (G) | 🟡 部分RED |
| **PreRenderSurface** | — | mark_dirty_surfaces (YELLOW) | 🟡 軽微YELLOW |
| **RenderSurface** | **render_surface** (RED) | — | 🔴 全RED |
| **Composition** | **visual_property_sync** (RED) | — | 🔴 全RED |
| **CommitComposition** | **commit_composition** (RED) | — | 🔴 全RED |
| FrameFinalize | — | clear_transient_pointer_state | 🟢 完全GREEN |

### 4.3 ULW方式でのステージ再構成案

| 現行Stage | 変更 | ULW方式での内容 |
|---|---|---|
| PreLayout | 🔴 RED削除 | `init_graphics_core` のみ（DComp除去版） |
| GraphicsSetup | 🔴 置換 | ULW版 `init_window_graphics`（合成Bitmap確保） |
| Draw | ⚠ RED削除 | ウィジェット描画システムのみ（deferred_surface/cleanup削除） |
| PreRenderSurface | 🟡 改修 | ウィンドウ単位のダーティ検出 |
| RenderSurface | 🔴 置換 | **新: 合成描画システム**（全CommandListをウィンドウBitmapに合成） |
| Composition | 🔴 廃止 | 不要（合成描画で座標/opacity適用済み） |
| CommitComposition | 🔴 置換 | **新: UpdateLayeredWindow呼出** |

---

## 5. Layout → Graphics データフロー

### 5.1 データフロー解析

```
TaffyStyle (ユーザー定義)
    ↓ compute_taffy_layout
TaffyComputedLayout (taffyの計算結果)
    ↓ update_arrangements
Arrangement (ローカル: offset, scale, size)
    ↓ propagate_global_arrangements
GlobalArrangement (グローバル: transform(Matrix3x2), bounds(D2D_RECT_F))
    ↓ ★ここから先がDComp依存
    ├── [DComp] deferred_surface_creation → Surface size計算
    ├── [DComp] visual_property_sync → SetOffsetX/Y, SetOpacity
    └── [DComp] render_surface → BeginDraw時Transform
```

**ULW方式でのデータフロー変更**:
```
GlobalArrangement (変更なし)
    ↓ ★新しいフロー
    ├── [新] 合成描画: bounds → ウィジェット描画位置の決定
    ├── [新] 合成描画: GraphicsCommandList → DrawImage(transform, opacity)
    └── [新] ULW転送: 合成Bitmap → HBITMAP → UpdateLayeredWindow
```

### 5.2 重要な既存構造体

- **Arrangement**: `offset: Offset`, `scale: LayoutScale`, `size: Size` — 親からの相対位置
- **GlobalArrangement**: `transform: Matrix3x2`, `bounds: D2DRect` — スクリーン絶対座標
  - `scale_x()`, `scale_y()`: 累積DPIスケール
  - `offset_x()`, `offset_y()`: 累積平行移動
  - `width()`, `height()`: 矩形幅・高さ

**判定: GREEN** — Layout→Arrangementのデータフローは完全にDComp非依存。合成描画でも同じ `GlobalArrangement` データをそのまま使用可能。

---

## 6. ウィジェット描画システム評価

### 6.1 ウィジェット描画パターン（全て同一パターン）

```rust
// 例: draw_rectangles (rectangle.rs L125-L280)
fn draw_rectangles(
    core: Res<GraphicsCore>,  // 共有D2D DeviceContext取得
    mut query: Query<(&Rectangle, &mut GraphicsCommandList, ...), ...>
) {
    let dc = core.device_context().unwrap();
    for (rect, mut cmd, ..) in query.iter_mut() {
        let command_list = dc.CreateCommandList().unwrap();
        dc.SetTarget(&command_list);
        dc.BeginDraw();
        // D2D描画コマンド（FillRectangle, DrawText等）
        dc.EndDraw();
        command_list.Close();
        cmd.set(command_list);  // GraphicsCommandListに格納
    }
}
```

**DComp API呼び出し: ゼロ** — 全ウィジェットシステムは `ID2D1DeviceContext` + `ID2D1CommandList` のみ使用。

### 6.2 確認済みウィジェットシステム

| ウィジェット | DComp依存 | 判定 |
|---|---|---|
| `draw_rectangles` | なし | 🟢 完全再利用 |
| `draw_labels` | なし | 🟢 完全再利用 |
| `draw_typewriters` + 関連 | なし | 🟢 完全再利用 |
| `draw_bitmap_sources` | なし | 🟢 完全再利用 |
| `generate_alpha_mask` | なし | 🟢 完全再利用 |
| `resolve_inherited_brushes` | なし | 🟢 完全再利用 |

---

## 7. COM層分析

### 7.1 DComp API呼び出し一覧（廃止対象）

| API | ラッパー関数 | 呼び出し元 |
|---|---|---|
| `DCompositionCreateDevice3` | `dcomp_create_desktop_device()` | `GraphicsCore::new()` |
| `CreateVisual()` | `DCompositionDeviceExt::create_visual()` | `visual_resource_management_system` |
| `Commit()` | `DCompositionDeviceExt::commit()` | `commit_composition` |
| `CreateSurface()` | `DCompositionDeviceExt::create_surface()` | `deferred_surface_creation` |
| `CreateTargetForHwnd()` | `DCompositionDesktopDeviceExt` | `create_window_graphics_for_hwnd` |
| `SetRoot()` | `DCompositionTargetExt` | `window_visual_integration` |
| `AddVisual/RemoveVisual()` | `DCompositionVisualExt` | `visual_hierarchy_sync` |
| `SetOffsetX/Y/Opacity()` | `DCompositionVisualExt` | `visual_property_sync` |
| `SetContent()` | `DCompositionVisualExt` | `deferred_surface_creation` |
| `BeginDraw/EndDraw()` | `DCompositionSurfaceExt` | `render_surface` |

### 7.2 既存のD2D API（全て再利用可能）

- `D2D1CreateFactory`, `d2d_create_device`, `D2D1DeviceExt` — デバイス作成
- `D2D1DeviceContextExt` — SetTransform, Clear, CreateSolidColorBrush, FillRectangle, DrawText, DrawTextLayout, DrawBitmap, DrawImage, FillEllipse, FillGeometry, CreateBitmapFromWicBitmap
- `DrawCommand` enum + `RecCommandSink` (ID2D1CommandSink5実装) — D2Dコマンド記録
- `GraphicsCommandList` — ID2D1CommandList保持

### 7.3 HBITMAP変換パスの不在（新規実装必要）

**⚠ クリティカルギャップ**: 現在のコードベースに `ID2D1Bitmap1` → `HBITMAP` (PARGB32) 変換ユーティリティが**存在しない**。

ULW方式の核心である `UpdateLayeredWindow()` は `HDC`（MemoryDC + HBITMAP）を受け取るため、以下のいずれかの変換パスが新規必要:

**方式A**: D2D → WIC → HBITMAP
```
ID2D1Bitmap1 → IWICBitmap (CopyFromRenderTarget) → CreateDIBSection() → memcpy
```

**方式B**: D2D → CPU Map → HBITMAP
```
ID2D1Bitmap1 (D2D1_BITMAP_OPTIONS_CPU_READ) → Map() → CreateDIBSection() → memcpy
```

**方式C**: WICBitmap を直接 D2D RenderTarget にする
```
IWICBitmap → ID2D1RenderTarget (CreateWicBitmapRenderTarget) → 直接描画
→ IWICBitmap::Lock → CreateDIBSection() → memcpy
```

`com/wic.rs` に `CopyPixels` メソッドは存在するが、D2D→WIC→HBITMAP の完全パスは未構築。

---

## 8. アプリケーション層依存

### 8.1 areka main.rs

| 箇所 | 内容 | 対応 |
|---|---|---|
| L141 | Shell: `WS_EX_NOREDIRECTIONBITMAP \| WS_EX_TOOLWINDOW \| WS_EX_TOPMOST` | → `WS_EX_LAYERED \| WS_EX_TOOLWINDOW \| WS_EX_TOPMOST` |
| L201 | Balloon: `WS_EX_NOREDIRECTIONBITMAP \| WS_EX_TOOLWINDOW \| WS_EX_TOPMOST` | → `WS_EX_LAYERED \| WS_EX_TOOLWINDOW \| WS_EX_TOPMOST` |

その他のアプリケーションコード（ECSコンポーネント挿入、ポインター、ドラッグ、Typewriter等）は全てDComp非依存。

### 8.2 examples/

| ファイル | DComp依存 | 対応 |
|---|---|---|
| `dcomp_demo.rs` | ✅ 全面DComp（ECS不使用、独立デモ） | レガシーとして残存 or ULW版デモに置換 |
| `taffy_flex_demo.rs` | ⚠ ECS経由でDComp間接依存 | フェーズ2で自動的にULW化 |
| その他example | ECS経由で間接依存 | フェーズ2で自動的にULW化 |

---

## 9. 関連仕様への影響評価

### 9.1 wintf-P0-click-through-rgn（設計済み・未承認）

**影響度: 高** — ULW移行により要件の大部分が不要化

| click-through-rgnの要件 | ULW移行後の状態 |
|---|---|
| SetWindowRgn ベースのクリックスルー | ❌ ULW_ALPHAで自動実現 → **不要** |
| GlobalArrangement.bounds → HRGN構築 | ❌ alpha=0が自動クリックスルー → **不要** |
| グリッドスナップサイズ（4x4px） | ❌ ピクセル精度のalpha判定 → **不要** |
| タイマーベースのリージョン更新 | ❌ フレームごとのULW更新で十分 → **不要** |
| WS_EX_NOREDIRECTIONBITMAP互換性検証 | ❌ WS_EX_NOREDIRECTIONBITMAPが廃止 → **不要** |
| HitTestMode::NamedRegions精密制御 | ⚠ **残存可能性** — 特定領域の非透過クリック制御 |
| NCHITTEST二層アーキテクチャ | ⚠ **縮小** — ULW alpha=0がfirst-pass、NCHITTESTがsecond-pass |
| ドラッグ時SetWindowRgn(NULL)リセット | ❌ **不要** |

**推奨**: wintf-P0-click-through-rgn仕様を大幅にスコープ縮小するか、ULW移行完了後に再評価。

### 9.2 wintf-P0-animation-system（要件生成済み・未承認）

**影響度: 低** — dola駆動のスケジュールベースアニメーションはDComp Animation API非依存。

- DComp Animation API（`CreateAnimation()`等）は `dcomp_demo.rs` でのみ使用、ECSシステムでは未使用
- dolaクレートのアニメーション値→ECSコンポーネント（Opacity, Offset等）→描画システム のフローは維持
- **影響なし**: dolaアニメーションの出力先がDComp Visual PropertiesからD2D合成描画パラメータに変わるだけ

### 9.3 wintf-P0-balloon-system（要件ドラフト）

**影響度: 中** — バルーンウィンドウもULW方式に移行

- バルーンもウィンドウ単位の描画であり、描画パイプライン変更の影響を受ける
- ただしウィジェット描画（テキスト、背景矩形等）はGREENなので、パイプライン変更が完了すればそのまま動作
- **推奨**: ULW移行（本仕様）を先に完了させ、その上でバルーン仕様を実装

---

## 10. 要件 → コード資産マッピング

### 10.1 Requirement 1: 影響範囲の特定と分類

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| DComp依存の3カテゴリ分類 | ✅ 本分析で完了 | なし — 分析結果を実装指針に反映 |
| 廃止対象ファイル識別 | ✅ 7ファイル特定済み | なし |
| 再利用可能資産保証 | ✅ 20+ファイル確認済み | なし |

### 10.2 Requirement 2: 段階的移行戦略

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| 3フェーズ移行定義 | — | **設計フェーズで策定** |
| DComp並行稼働 | ✅ feature flag or module-level切り替え可能 | `cfg` 属性 or 新モジュール追加で対応 |
| 段階的検証 | ✅ 既存examples（taffy_flex_demo等）がベンチマーク | 新ULW版exampleが必要 |

### 10.3 Requirement 3: 新描画パイプライン

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| ウィンドウ単位合成Bitmap | — | **新規実装**: Bitmap確保・管理ロジック |
| CommandList合成描画 | ✅ `GraphicsCommandList` + `DrawImage` 既存 | **新規実装**: z-order走査 + transform適用の合成ループ |
| Visual階層廃止 | ✅ Children/ChildOfでECS階層は既に管理 | GlobalArrangementからtransformを取得する合成描画ロジック |
| ウィジェットシステム再利用 | ✅ 完全再利用可能（確認済み） | なし |
| リサイズ対応 | ✅ WindowPos変更検知は既存 | **新規実装**: 合成Bitmapのリサイズ処理 |

### 10.4 Requirement 4: UpdateLayeredWindow統合

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| PARGB32→HBITMAP変換 | ⚠ `com/wic.rs` にCopyPixels存在 | **新規実装(CRITICAL)**: D2D→HBITMAP完全パス |
| WS_EX_LAYERED設定 | ✅ `win_style.rs` にメソッド既存 | 適用箇所の変更のみ |
| UpdateLayeredWindow呼出 | — | **新規実装**: `windows` クレートにAPIバインディング存在確認が必要 |
| エラーリカバリ | ✅ tracingインフラ既存 | エラーハンドリングパターンの適用 |

### 10.5 Requirement 5: GraphicsCore簡素化

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| DComp初期化除去 | ✅ 該当2行を削除するだけ | 軽微 |
| デバイスチェーン維持 | ✅ 全てそのまま | なし |
| フィールド除去 | ✅ 2フィールド＋2メソッド削除 | 軽微 |
| デバイスロスト対応 | ✅ invalidate()パターン維持 | DComp再初期化ステップの省略（簡素化） |

### 10.6 Requirement 6: ECSコンポーネント再設計

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| WindowGraphics置換 | ✅ 既存struct定義あり | **新規設計**: ID2D1Bitmap1 + MemDC/HBITMAP フィールド |
| VisualGraphics廃止 | ✅ 削除のみ | 参照箇所の除去 |
| SurfaceGraphics廃止 | ✅ 削除のみ | 参照箇所の除去 |
| visual_manager廃止 | ✅ 削除のみ | world.rsからの登録除去 |

### 10.7 Requirement 7: メッセージハンドリング

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| WM_ERASEBKGND更新 | ✅ 既存ハンドラ（背景消去スキップ） | コメント更新のみ（動作は同じ） |
| WM_SIZE→リサイズ | ✅ WM_SIZE検出は既存 | **新規**: 合成Bitmapリサイズトリガーの追加 |
| WM_PAINT最小化 | ✅ 既存ValidateRect | WS_EX_LAYEREDでWM_PAINT未発火の可能性→検証必要 |

**Research Needed**: `WS_EX_LAYERED` ウィンドウでの `WM_PAINT` 発火動作の確認

### 10.8 Requirement 8: 既存仕様影響

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| click-through-rgnへの影響 | ✅ 仕様分析完了（セクション9.1） | 仕様スコープ再定義が必要 |
| animation-systemへの影響 | ✅ 分析完了（影響なし） | なし |
| balloon-systemへの影響 | ✅ 分析完了（影響中） | ULW移行後に再評価 |
| dcomp_demo.rsの扱い | ✅ 分析完了 | 判断を設計フェーズに持ち越し |

### 10.9 Requirement 9: 子仕様構成

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| 4子仕様の構成定義 | — | **設計フェーズで策定** |
| 依存関係と実装順序 | ✅ 本分析で情報収集完了 | 設計フェーズで確定 |

### 10.10 Requirement 10: テスト・検証戦略

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| フェーズ別検証基準 | ✅ 既存examples、cargo testインフラ | **新規**: ULW版テスト・デモの追加 |
| 完了基準（DoD） | — | 設計フェーズで定義 |
| 描画品質比較 | — | **Research Needed**: DComp vs ULW の描画品質差異 |

---

## 11. 実装アプローチオプション

### Option A: モジュール並行置換（推奨）

**概要**: 新しいULW方式のモジュールを `ecs/graphics/` 内に並行追加し、feature flagまたはcfg属性で切り替え。

**戦略**:
1. `ecs/graphics/ulw_systems.rs`（新規）に合成描画システムを実装
2. `ecs/graphics/ulw_components.rs`（新規）にULW版WindowGraphicsを定義
3. `com/ulw.rs`（新規）にUpdateLayeredWindow呼出＋D2D→HBITMAP変換を実装
4. `ecs/world.rs` でcfgまたはfeature flagにより新旧システムを切り替え
5. 検証完了後に旧モジュールを削除

**Trade-offs**:
- ✅ 旧コード参照しながら新実装を進められる（開発者の要求に合致）
- ✅ 段階的にシステムを切り替え可能
- ✅ ロールバックが容易
- ❌ 一時的にコードが重複する
- ❌ feature flagの管理が必要

### Option B: インプレース置換

**概要**: 既存の `systems.rs`, `components.rs` を直接編集し、DCompコードをULW版に置き換え。

**Trade-offs**:
- ✅ コード重複なし
- ✅ 最終状態が即座に見える
- ❌ 旧実装が参照不能（git logのみ）
- ❌ 中間状態でビルドが壊れる期間が長い
- ❌ ロールバックが困難

### Option C: ハイブリッド段階アプローチ

**概要**: 子仕様1-2は並行追加（Option A）、子仕様3-4でインプレース統合。

**戦略**:
1. **子仕様1**: 新モジュール（`ulw_*`）として並行追加 → 独立テスト可能
2. **子仕様2**: world.rsのシステム登録を切り替え → 旧モジュールは残存
3. **子仕様3**: ULW統合をインプレースで実装（この時点で旧は参照のみ）
4. **子仕様4**: 旧モジュール削除＋クリーンアップ

**Trade-offs**:
- ✅ 各段階で検証可能
- ✅ 並行期間を最適化（必要な期間のみ重複）
- ✅ 段階的なリスク低減
- ❌ 計画の複雑度が最も高い

### 推奨: **Option C（ハイブリッド段階アプローチ）**

開発者の要求（「旧実装を参照しつつ、新しい実装を検討し、最後にまとめて削除」）に最も合致。要件定義のフェーズ1-4構成とも自然に整合する。

---

## 12. 技術リスク評価

### Risk 1: D2D1Bitmap → HBITMAP 変換パス（HIGH）

**状況**: 現コードベースに完全パスが**ゼロ**。ULW方式の核心。
**影響**: フェーズ3（ULW統合）のブロッカー
**緩和策**: 
- `windows` クレートの `UpdateLayeredWindow` バインディング存在確認
- 方式A/B/Cの技術検証をフェーズ1の早期に実施
- `com/wic.rs` の `CopyPixels` 既存実装がベースになる可能性

**Research Needed**: 最適なD2D→HBITMAP変換方式の選定（方式A: WIC経由、方式B: CPU Map、方式C: WICBitmapRenderTarget）

### Risk 2: 合成描画パフォーマンス（MEDIUM）

**状況**: DComp方式はGPU側で並列合成。ULW方式はCPU側で全ウィジェットを順次合成。
**影響**: ウィジェット数が多い場合に描画遅延の可能性
**緩和策**: デスクトップマスコットのウィジェット数は限定的（数十個程度）。パフォーマンス問題は発生しにくい。

### Risk 3: Opacity階層累積（MEDIUM）

**状況**: DComp方式では `Visual.SetOpacity()` でDCompが階層的にOpacityを処理。
ULW方式では自前で親→子のOpacity累積を計算する必要がある可能性。
**影響**: 透過表示の品質に関わる
**Research Needed**: `GlobalArrangement` にOpacity累積を追加するか、合成描画時に動的計算するかの設計判断

### Risk 4: ダーティ検出の粒度変更（LOW-MEDIUM）

**状況**: per-entity `SurfaceGraphicsDirty` → ウィンドウ全体の再合成
**影響**: 不要な再描画が増える可能性
**緩和策**: ウィンドウ内の**いずれかの** `GraphicsCommandList` が変更された場合のみ合成ビットマップを再生成。判定ロジックは `mark_dirty_surfaces` の改修で実現可能。

### Risk 5: WS_EX_LAYERED での WM_PAINT 動作（LOW）

**状況**: `WS_EX_LAYERED` ウィンドウは通常 `WM_PAINT` を受信しない
**影響**: 既存の `WM_PAINT` ハンドラが発火しなくなる（問題化しないが確認が必要）
**Research Needed**: Win32仕様確認

### Risk 6: テスト基盤の修正量（MEDIUM）

**状況**: DComp直接依存テストが約8ファイル
**影響**: テスト修正の作業量
**緩和策**: widget系テスト（`GraphicsCommandList`ベース）は影響なし。要修正はグラフィックスパイプラインテストに限定。

---

## 13. 工数・リスク総合評価

### 工数見積

| 子仕様 | 工数 | 根拠 |
|---|---|---|
| 子仕様1: D2D1合成スタック構築 | **L (1-2週)** | D2D→HBITMAP変換パス新規、合成描画エンジン新規、コンポーネント設計 |
| 子仕様2: DCompパイプライン置換 | **M (3-7日)** | world.rs切替、旧システム無効化、既存example検証 |
| 子仕様3: UpdateLayeredWindow統合 | **M (3-7日)** | ULW呼出実装、WS_EX_LAYERED適用、クリックスルー検証 |
| 子仕様4: DCompコード削除・クリーンアップ | **S (1-3日)** | 旧コード削除、テスト修正、dcomp_demo対応 |
| **合計** | **XL (3-5週)** | アーキテクチャ変更、広範な影響 |

### リスク総合

**全体リスク: Medium-High**

- HIGH: D2D→HBITMAP変換パスの技術的不確実性
- MEDIUM: 合成描画パフォーマンス、Opacity階層処理
- LOW: ウィンドウメッセージ動作、デバイスロスト処理の簡素化

---

## 14. 設計フェーズへの推奨事項

### 確定済み方針
1. **アプローチ**: Option C（ハイブリッド段階アプローチ）を推奨
2. **再利用可能資産**: ウィジェット描画・レイアウト・入力系の全モジュール（~70%）
3. **廃止対象**: DComp Visual/Surface/Target関連のコンポーネント・システム

### 設計フェーズで検討が必要な事項 — 解決済み

> 以下の事項は design.md にて確定済み。

1. **D2D→HBITMAP変換方式の選定**: ✅ **Option B（GPU Render + CPU Map）を採用**。ハードウェアアクセラレーションD2D描画を維持しつつ、staging bitmap (CPU_READ) → Map → DIBSection memcpy で転送。WICBitmapRenderTarget（Option C）のソフトウェアレンダリング制約を回避。
2. **合成描画エンジンの設計**: ✅ **composite_render_system** として定義。Children関係のBFS走査でz-order確定 → SetTransform(GlobalArrangement.transform) → DrawImage(CommandList) with opacity。per-window ID2D1Bitmap1に合成描画。
3. **Opacity階層累積の設計**: ✅ **GlobalArrangement拡張方式を採用**。`global_opacity: f32` フィールドを追加し、既存の `propagate_global_arrangements` で `parent_global_opacity * child_local_opacity` を伝播。
4. **feature flag or cfg切り替えの設計**: ✅ **ハイブリッド段階アプローチ（Option C）を採用**。Phase 1-2で新モジュール並行追加（cfg不要）、Phase 3でインプレースULW統合、Phase 4で旧コード削除。
5. **wintf-P0-click-through-rgnの処遇**: ✅ **競争的並走**として独立進行（要件C2で確定済み）。
6. **dcomp_demo.rsの処遇**: ✅ **Phase 4で削除**（要件C3で確定済み）。

### Research Needed（設計フェーズで調査）— 解決状況
- [x] `UpdateLayeredWindow` の `windows` クレートバインディング確認 — windows 0.62.2 に `UpdateLayeredWindow`, `BLENDFUNCTION`, `ULW_ALPHA` が存在
- [x] D2D→HBITMAP最適変換方式の技術検証 — **Option B採用**（design.md §System Flows）
- [ ] `WS_EX_LAYERED` ウィンドウでの `WM_PAINT` 発火動作確認 — WS_EX_LAYEREDウィンドウはWM_PAINT未発火（Win32仕様）。子仕様3で実証
- [x] DComp vs ULW の描画品質差異検証 — 理論上同一（PBGRA32フォーマット維持、ハードウェアD2D使用）。視覚差異なしと判定
- [x] Opacity階層累積の最適アプローチ — **GlobalArrangement.global_opacity拡張**（design.md §System Flows）
