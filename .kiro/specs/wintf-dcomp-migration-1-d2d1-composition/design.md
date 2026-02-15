# 設計書: wintf-dcomp-migration-1-d2d1-composition

## Overview

DComp パイプラインを温存したまま、D2D1 合成描画スタックを新規モジュールとして並行構築する。本設計は親仕様 design.md と統合指針（migration-guide.md）から Phase 1 担当範囲を抽出・詳細化したものである。

### Goals

- `WindowD3D11Compositor` コンポーネントの実装（per-window 合成リソース管理）
- `compositor_init_system` / `composite_render_system` の実装（合成描画パイプライン）
- `com/ulw.rs` の `transfer_to_hbitmap()` 実装（D2D→HBITMAP 転送基盤）
- `GlobalArrangement.global_opacity` 拡張（Opacity 階層累積）
- 全新規コードは **独立テスト可能** — world.rs に登録しない（Phase 2 で登録）

### Non-Goals

- world.rs へのシステム登録
- DComp パイプラインの変更
- UpdateLayeredWindow 呼び出し（`present_layered_window` は Phase 3）
- 旧コード削除

---

## Architecture

### 新規モジュール配置

```
crates/wintf/src/
├── com/
│   ├── ulw.rs              ← NEW: D2D→HBITMAP転送ユーティリティ
│   └── (既存: d2d/, dwrite.rs, wic.rs, dcomp.rs)
├── ecs/
│   ├── graphics/
│   │   ├── compositor.rs           ← NEW: WindowD3D11Compositor コンポーネント
│   │   ├── compositor_systems.rs   ← NEW: compositor_init_system, composite_render_system
│   │   └── (既存: core.rs, components.rs, systems.rs, visual_manager.rs)
│   └── layout/
│       ├── (既存: arrangement.rs, systems.rs)
│       └── (拡張: GlobalArrangement.global_opacity)
```

### コンポーネント間依存関係

```mermaid
graph TB
    GC[GraphicsCore Res] --> CIS[compositor_init_system]
    CIS --> WDC[WindowD3D11Compositor per-window]

    WDC --> CRS[composite_render_system]
    GA[GlobalArrangement per-entity] --> CRS
    GCL[GraphicsCommandList per-entity] --> CRS
    V[Visual per-entity] --> CRS
    CH[Children per-entity] --> CRS

    WDC --> TTH[transfer_to_hbitmap com/ulw.rs]
```

---

## Components

### WindowD3D11Compositor

**ファイル**: `ecs/graphics/compositor.rs`

```rust
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct WindowD3D11Compositor {
    inner: Option<WindowD3D11CompositorInner>,
    generation: u32,
}

struct WindowD3D11CompositorInner {
    composition_bitmap: ID2D1Bitmap1,  // D2D1_BITMAP_OPTIONS_TARGET
    staging_bitmap: ID2D1Bitmap1,      // D2D1_BITMAP_OPTIONS_CPU_READ | CANNOT_DRAW
    hbitmap: HBITMAP,                  // CreateDIBSection PBGRA32 top-down
    memory_dc: CreatedHDC,             // CreateCompatibleDC
    dib_bits: *mut u8,                 // DIBSection pixel pointer
    size: (u32, u32),                  // (width, height)
}
```

**Service Interface**:

| メソッド | 引数 | 返却 | 前提条件 | 事後条件 |
|---------|------|------|---------|---------|
| `new()` | `dc: &ID2D1DeviceContext, w: u32, h: u32` | `Result<Self>` | w>0, h>0, DC有効 | 全4リソース有効 |
| `resize()` | `dc: &ID2D1DeviceContext, w: u32, h: u32` | `Result<()>` | is_valid()==true | 全リソース新サイズ |
| `invalidate()` | — | `()` | — | inner=None |
| `is_valid()` | — | `bool` | — | — |
| `composition_bitmap()` | — | `Option<&ID2D1Bitmap1>` | — | — |
| `staging_bitmap()` | — | `Option<&ID2D1Bitmap1>` | — | — |
| `hbitmap()` | — | `Option<HBITMAP>` | — | — |
| `memory_dc()` | — | `Option<HDC>` | — | — |
| `dib_bits()` | — | `Option<*mut u8>` | — | — |
| `generation()` | — | `u32` | — | — |

**Bitmap 作成パラメータ**:

composition_bitmap:
```
D2D1_BITMAP_PROPERTIES1 {
    pixelFormat: D2D1_PIXEL_FORMAT {
        format: DXGI_FORMAT_B8G8R8A8_UNORM,
        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
    },
    bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET,
    dpiX: 96.0, dpiY: 96.0,
}
```

staging_bitmap:
```
D2D1_BITMAP_PROPERTIES1 {
    pixelFormat: D2D1_PIXEL_FORMAT {
        format: DXGI_FORMAT_B8G8R8A8_UNORM,
        alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
    },
    bitmapOptions: D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
    dpiX: 96.0, dpiY: 96.0,
}
```

DIBSection:
```
BITMAPINFOHEADER {
    biSize: size_of::<BITMAPINFOHEADER>() as u32,
    biWidth: width as i32,
    biHeight: -(height as i32),  // top-down DIB
    biPlanes: 1,
    biBitCount: 32,
    biCompression: BI_RGB,
    ..Default::default()
}
```

**Drop実装**: `WindowD3D11CompositorInner` の Drop で `DeleteObject(hbitmap)`, `DeleteDC(memory_dc)` を呼び出す。D2D リソースは COM Drop で自動解放。

---

### GlobalArrangement 拡張

**ファイル**: `ecs/layout/arrangement.rs`（既存ファイル拡張）

**変更内容**:
```rust
// 追加フィールド
pub struct GlobalArrangement {
    pub transform: Matrix3x2,    // 既存
    pub bounds: D2DRect,          // 既存
    pub global_opacity: f32,      // NEW: 初期値 1.0
}
```

**PropagateTransform 拡張**: `propagate_global_arrangements` で Opacity 累積ロジックを追加。

```rust
// 擬似コード — propagate時のOpacity計算
fn propagate_opacity(
    parent_global_opacity: f32,
    child_visual: &Visual,
) -> f32 {
    if !child_visual.is_visible {
        return 0.0;
    }
    (parent_global_opacity * child_visual.opacity).clamp(0.0, 1.0)
}
```

**制約**:
- `global_opacity` ∈ `[0.0, 1.0]`（clamp）
- `Default::default()` で `global_opacity: 1.0`
- `is_visible == false` → `global_opacity = 0.0`（全子孫に伝播）

---

## Systems

### compositor_init_system

**ファイル**: `ecs/graphics/compositor_systems.rs`

**Stage**: GraphicsSetup（Phase 2 で world.rs に登録予定）

**Query**:
```rust
fn compositor_init_system(
    core: Res<GraphicsCore>,
    mut commands: Commands,
    query: Query<(Entity, &WindowHandle), (Added<WindowHandle>, Without<WindowD3D11Compositor>)>,
    mut existing: Query<(&WindowHandle, &mut WindowD3D11Compositor)>,
) {
    // 1. 新規ウィンドウ: WindowD3D11Compositor 作成・挿入
    // 2. 既存ウィンドウ: generation不一致検出→再作成
    // 3. リサイズ検出: サイズ変更→resize()
}
```

**エラーハンドリング**:
- リソース作成失敗: `tracing::error!` + エンティティに `WindowD3D11Compositor` を挿入しない（次フレームで再試行）
- GraphicsCore 無効: システム全体をスキップ

---

### composite_render_system

**ファイル**: `ecs/graphics/compositor_systems.rs`

**Stage**: RenderSurface（Phase 2 で world.rs に登録予定）

**合成描画フロー**:

```rust
fn composite_render_system(
    core: Res<GraphicsCore>,
    mut compositor_query: Query<(Entity, &mut WindowD3D11Compositor, &Children)>,
    entity_query: Query<(&GlobalArrangement, Option<&GraphicsCommandList>, &Visual)>,
) {
    let dc = core.device_context();

    for (window_entity, mut compositor, children) in compositor_query.iter_mut() {
        // 1. ダーティ判定（Changed<GraphicsCommandList> || Changed<GlobalArrangement> || Changed<Visual>）
        // → ダーティでなければスキップ

        // 2. composition_bitmap を SetTarget
        let comp_bmp = compositor.composition_bitmap().unwrap();
        dc.SetTarget(comp_bmp);

        // 3. BeginDraw → Clear(transparent)
        dc.BeginDraw();
        dc.Clear(Some(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }));

        // 4. depth-first pre-order 走査で z-order 描画
        for entity in depth_first_preorder(window_entity, children, &entity_query) {
            let (ga, cmd_opt, visual) = entity_query.get(entity).unwrap();

            // 4a. global_opacity == 0.0 → スキップ
            if ga.global_opacity == 0.0 { continue; }

            // 4b. GraphicsCommandList が無い → スキップ
            let Some(cmd) = cmd_opt else { continue; };
            let Some(command_list) = cmd.get() else { continue; };

            // 4c. SetTransform(GlobalArrangement.transform)
            dc.SetTransform(&ga.transform);

            // 4d. Opacity < 1.0 の場合 PushLayer
            if ga.global_opacity < 1.0 {
                // PushLayer with opacity parameter
                dc.PushLayer(&layer_params_with_opacity(ga.global_opacity), None);
            }

            // 4e. DrawImage(command_list)
            dc.DrawImage(command_list, None, None, D2D1_INTERPOLATION_MODE_LINEAR, D2D1_COMPOSITE_MODE_SOURCE_OVER);

            // 4f. PopLayer (if pushed)
            if ga.global_opacity < 1.0 {
                dc.PopLayer();
            }
        }

        // 5. EndDraw
        dc.EndDraw(None, None);

        // 6. CopyFromBitmap(composition → staging)
        let staging = compositor.staging_bitmap().unwrap();
        staging.CopyFromBitmap(None, comp_bmp, None);
    }
}
```

**z-order走査関数**:
```rust
fn depth_first_preorder(
    root: Entity,
    root_children: &Children,
    query: &Query<...>,
) -> Vec<Entity> {
    // Children コンポーネントから再帰的に走査
    // 描画順: 親 → 子1 → 子1の子 → 子2 → ...
    // 先に描いたものが背景（画家のアルゴリズム）
}
```

**Opacity 適用方式**: `ID2D1DeviceContext::PushLayer()` で `D2D1_LAYER_PARAMETERS1` の `opacity` フィールドを使用。CommandList 全体に均一な opacity を適用するため、PushLayer/PopLayer が最も自然な API。

**ダーティ判定**: bevy_ecs の `Changed<T>` フィルタを使用。ウィンドウに属するいずれかのエンティティが変更された場合にウィンドウ全体を再合成。初回フレーム（`Added<WindowD3D11Compositor>`）は必ず描画。

---

### com/ulw.rs — transfer_to_hbitmap

**ファイル**: `com/ulw.rs`

```rust
/// ステージング ID2D1Bitmap1 のピクセルデータを DIBSection HBITMAP にコピーする。
///
/// # Safety
/// `dib_bits` は `width * height * 4` バイト以上のメモリを指すこと。
pub unsafe fn transfer_to_hbitmap(
    staging: &ID2D1Bitmap1,
    dib_bits: *mut u8,
    width: u32,
    height: u32,
) -> windows::core::Result<()> {
    // 1. Map staging bitmap (D2D1_MAP_OPTIONS_READ)
    let mapped = staging.Map(D2D1_MAP_OPTIONS_READ)?;

    // 2. stride計算
    let src_pitch = mapped.pitch;          // D2D の行ストライド
    let dst_stride = width as usize * 4;   // DIBSection の行ストライド

    // 3. コピー
    if src_pitch as usize == dst_stride {
        // 一括コピー最適化
        std::ptr::copy_nonoverlapping(
            mapped.bits,
            dib_bits,
            dst_stride * height as usize,
        );
    } else {
        // 行単位コピー（pitch != stride の場合）
        for y in 0..height as usize {
            let src = mapped.bits.add(y * src_pitch as usize);
            let dst = dib_bits.add(y * dst_stride);
            std::ptr::copy_nonoverlapping(src, dst, dst_stride);
        }
    }

    // 4. Unmap
    staging.Unmap()?;

    Ok(())
}
```

---

## Data Models

### WindowD3D11Compositor 状態遷移

```
[未作成] --new()--> [有効] --invalidate()--> [無効]
   ^                  |                        |
   |                  +--resize()-->[有効]      |
   +--new()-----------------------------------+
```

- **未作成**: エンティティに挿入されていない
- **有効**: `is_valid() == true`、全リソースが使用可能
- **無効**: `is_valid() == false`、`inner == None`

### GlobalArrangement 拡張（互換性）

- `global_opacity` フィールド追加は **後方互換** — `Default::default()` で `1.0`
- 既存の `GlobalArrangement` を使用するコードは `global_opacity` を無視しても動作する
- `propagate_global_arrangements` の変更は **前方互換** — Opacity 累積は追加ロジックのみ

---

## Requirements Traceability

| Requirement | Design Component | Test Coverage |
|-------------|-----------------|---------------|
| Req 1.1-1.4 | WindowD3D11Compositor struct + methods | Unit: new/resize/invalidate lifecycle |
| Req 2.1-2.7 | composite_render_system | Integration: z-order, transform, opacity |
| Req 3.1-3.4 | compositor_init_system | Unit: creation, resize detection, generation |
| Req 4.1-4.5 | GlobalArrangement.global_opacity + propagation | Unit: opacity accumulation |
| Req 5.1-5.4 | com/ulw.rs transfer_to_hbitmap | Unit: pitch/stride copy |
| Req 6.1-6.3 | WindowD3D11Compositor::resize() | Integration: resize → redraw |
| Req 7.1-7.3 | compositor_init_system generation check | Integration: device lost → reinit |
| Req 8.1-8.5 | All above | E2E: taffy_flex_demo equivalent |

---

## Error Handling

| エラー | 発生元 | レスポンス | リカバリ |
|--------|--------|----------|---------|
| Bitmap作成失敗 | `WindowD3D11Compositor::new()` | `tracing::error` | 次フレームで再作成 |
| BeginDraw失敗 | `composite_render_system` | `tracing::error` + フレームスキップ | 次フレーム再描画 |
| CopyFromBitmap失敗 | `composite_render_system` | `tracing::error` + フレームスキップ | 次フレーム再試行 |
| Map失敗 | `transfer_to_hbitmap` | `Err` 返却 | 呼出元で判断 |
| リサイズ失敗 | `WindowD3D11Compositor::resize()` | `tracing::error` + 旧サイズ維持 | 次回リサイズ再試行 |
| デバイスロスト | D2D操作全般 | `GraphicsCore::invalidate()` → Compositor invalidate | 既存フロー |

---

## Testing Strategy

### Unit Tests

- `WindowD3D11Compositor::new()` — 全4リソース正常作成
- `WindowD3D11Compositor::resize()` — リソース再作成＋サイズ整合
- `WindowD3D11Compositor::invalidate()` — `is_valid() == false`
- `GlobalArrangement::global_opacity` — Default で 1.0
- Opacity 累積計算 — parent 0.8 × child 0.5 = 0.4
- Opacity is_visible=false — global_opacity = 0.0
- Opacity clamp — 範囲外値のクランプ
- `transfer_to_hbitmap` — pitch==stride 一括コピー
- `transfer_to_hbitmap` — pitch!=stride 行単位コピー

### Integration Tests

- `composite_render_system` — 複数エンティティ z-order 合成
- `compositor_init_system` + `composite_render_system` 統合
- デバイスロスト → Compositor 再初期化
- ウィンドウリサイズ → Bitmap 再作成 → 再描画

### E2E Tests

- `taffy_flex_demo` 相当の描画が新パイプライン（独立テスト環境）で動作
- 既存テストへの回帰なし: `cargo test` 全パス
