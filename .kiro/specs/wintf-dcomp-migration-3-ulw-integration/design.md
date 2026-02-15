# 設計文書: wintf-dcomp-migration-3-ulw-integration

## 1. 概要

Phase 3 は、Phase 2 で D2D1 合成パイプラインに切り替わった描画結果を UpdateLayeredWindow（ULW）経由でウィンドウに転送する最終段階である。WS_EX_LAYERED への切替、ulw_present_system の world.rs 登録、WM_PAINT/WM_SIZE ハンドラの更新を行う。

### 前提条件
- Phase 2 完了済み: world.rs が D2D1 合成パイプラインで動作
- `WindowD3D11Compositor` が各ウィンドウエンティティに初期化済み
- `composite_render_system` が毎フレーム HBITMAP への転送を完了
- DComp API 呼び出しがゼロ（Phase 2 検証済み）

### 変更対象ファイル一覧

| ファイル | 変更種別 | 内容 |
|---------|---------|------|
| `ecs/graphics/compositor_systems.rs` | 追加 | `ulw_present_system` 関数 |
| `com/ulw.rs` | 追加 | `present_layered_window` 関数 |
| `ecs/world.rs` | 変更 | CommitComposition ステージ更新 |
| `ecs/window.rs` | 変更 | `WS_EX_LAYERED` 切替 |
| `ecs/window_proc/handlers.rs` | 変更 | WM_PAINT/WM_ERASEBKGND/WM_SIZE 更新 |
| `areka/src/main.rs` | 変更 | Shell/Balloon の `ex_style` 更新 |

---

## 2. アーキテクチャ

### 2.1 データフロー

```
composite_render_system (Phase 1-2 で実装済み)
  ↓ HBITMAP に描画済みピクセルデータ
  ↓ MemoryDC に SelectObject 済み
ulw_present_system (本 Phase で実装)
  ↓ UpdateLayeredWindow(hwnd, hdcSrc, ULW_ALPHA)
  ↓ OS が alpha 透過合成してデスクトップに表示
Desktop Window Manager
```

### 2.2 Schedule Stage 変更

Phase 3 での world.rs 変更は **CommitComposition ステージのみ**:

| Stage | Phase 2 完了時 | Phase 3 変更後 |
|-------|---------------|---------------|
| CommitComposition | `commit_composition`（残存だが実質無効）or 空 | `ulw_present_system` |

他のステージは Phase 2 の状態を維持する。

---

## 3. コンポーネント

### 3.1 既存コンポーネントの活用（変更なし）

Phase 3 では新規 ECS コンポーネントを追加しない。以下の Phase 1-2 で実装済みコンポーネントを使用する:

- **`WindowD3D11Compositor`**: `hbitmap`, `memory_dc`, `dirty` フラグ
- **`WindowSize`**: ウィンドウサイズ（ULW の `SIZE` パラメータに使用）
- **`WindowHandle`**: HWND（ULW の対象ウィンドウ）

### 3.2 WindowStyle 変更

```rust
// ecs/window.rs — WindowStyle::default() の変更
impl Default for WindowStyle {
    fn default() -> Self {
        Self {
            // Phase 3: WS_EX_NOREDIRECTIONBITMAP → WS_EX_LAYERED
            ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            // ... 既存の ws_style は維持
        }
    }
}
```

### 3.3 areka/src/main.rs 変更

```rust
// Shell ウィンドウ (L141 付近)
// Before: WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_TOPMOST
// After:  WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST

// Balloon ウィンドウ (L201 付近)  
// Before: WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_TOPMOST
// After:  WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST
```

---

## 4. システム

### 4.1 ulw_present_system

**ステージ**: CommitComposition
**クエリ**: `Query<(&WindowHandle, &WindowSize, &mut WindowD3D11Compositor)>`

```
fn ulw_present_system(
    query: Query<(&WindowHandle, &WindowSize, &mut WindowD3D11Compositor)>,
) {
    for (window_handle, window_size, mut compositor) in &mut query {
        // ダーティでなければスキップ
        if !compositor.dirty {
            continue;
        }

        let hwnd = window_handle.hwnd();
        let size = SIZE {
            cx: window_size.width as i32,
            cy: window_size.height as i32,
        };

        // present_layered_window 呼び出し
        match present_layered_window(hwnd, compositor.memory_dc, &size) {
            Ok(()) => {
                compositor.dirty = false;
            }
            Err(e) => {
                tracing::warn!("UpdateLayeredWindow failed: {e:?}, retrying next frame");
                // dirty フラグは true のまま → 次フレームで再試行
            }
        }
    }
}
```

### 4.2 present_layered_window（com/ulw.rs）

> **親 design.md からの設計変更**: 親仕様では `present_layered_window(hwnd, memory_dc, width, height, window_pos: Option<(i32, i32)>)` の5引数シグネチャだったが、Phase 3 詳細設計で以下の簡素化を行った:
> - `width, height` → `size: &SIZE`（Win32 API 直結型に統一）
> - `window_pos: Option<(i32, i32)>` → 削除（`pptDst=None` 固定。ウィンドウ位置管理は既存 `SetWindowPos` フローに委譲）

```rust
use windows::Win32::UI::WindowsAndMessaging::UpdateLayeredWindow;
use windows::Win32::UI::WindowsAndMessaging::ULW_ALPHA;
use windows::Win32::Graphics::Gdi::{AC_SRC_OVER, AC_SRC_ALPHA, BLENDFUNCTION, HDC};
use windows::Win32::Foundation::{HWND, POINT, SIZE};

pub fn present_layered_window(
    hwnd: HWND,
    hdc_src: HDC,
    size: &SIZE,
) -> windows::core::Result<()> {
    let pt_dst = POINT { x: 0, y: 0 }; // ウィンドウ位置は OS が管理
    let pt_src = POINT { x: 0, y: 0 };
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };

    unsafe {
        // ptDst = None → ウィンドウ位置を変更しない
        UpdateLayeredWindow(
            hwnd,
            None,       // hdcDst (screen DC, None = default)
            None,       // pptDst (None = 位置変更なし)
            Some(size),
            Some(hdc_src),
            Some(&pt_src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        )?;
    }
    Ok(())
}
```

**注意**: `pptDst` を `None` にすることでウィンドウ位置を変更しない。ウィンドウの移動は ECS の既存 `SetWindowPos` フローで管理する。ウィンドウ位置が必要な場合は `GetWindowRect` で取得して `pptDst` に渡す設計に変更する（Phase 3 実装時に検証）。

### 4.3 WM_PAINT / WM_ERASEBKGND ハンドラ更新

```rust
// handlers.rs — WM_PAINT
WM_PAINT => {
    // WS_EX_LAYERED ウィンドウでは WM_PAINT がほぼ発火しないが、
    // 安全のため BeginPaint/EndPaint 最小ペアを維持
    let mut ps = PAINTSTRUCT::default();
    unsafe {
        BeginPaint(hwnd, &mut ps);
        EndPaint(hwnd, &ps);
    }
    LRESULT(0)
}

// handlers.rs — WM_ERASEBKGND
WM_ERASEBKGND => {
    // 背景消去をスキップ（ULW が全画面を管理）
    LRESULT(1)
}
```

### 4.4 WM_SIZE ハンドラ更新

WM_SIZE の処理は Phase 1 で実装済みの `WindowD3D11Compositor::resize()` を活用する。既存の WM_SIZE ハンドラがウィンドウサイズコンポーネントを更新し、`compositor_init_system`（Phase 1-2）がサイズ変更を検出して `resize()` を呼び出すフローが既に構築されている。

追加のハンドラ変更は不要（ECS のリアクティブフローで処理）。ただし、WM_SIZE 受信後の即時 ULW 更新が必要な場合は `InvalidateRect` をトリガーする。

---

## 5. WS_EX_LAYERED 前提検証

### 5.1 検証項目（Phase 3 実装開始前に実施）

1. **WM_PAINT 発火テスト**: 最小構成の `WS_EX_LAYERED` ウィンドウを作成し、`WM_PAINT` ハンドラでログ出力
2. **ULW 基本動作テスト**: DIBSection → SelectObject → UpdateLayeredWindow の最小構成での透過描画
3. **alpha=0 クリックスルーテスト**: alpha=0 ピクセル上のクリックが背後のウィンドウに到達

### 5.2 検証結果に基づく設計分岐

| 検証結果 | 設計への影響 |
|---------|------------|
| WM_PAINT 発火なし（想定通り） | WM_PAINT ハンドラは BeginPaint/EndPaint 最小ペアのみ。描画は ulw_present_system 一元管理 |
| WM_PAINT 発火あり | WM_PAINT ハンドラ内で追加の ValidateRect() を実行して無限ループを防止 |
| pptDst=None で位置維持される | present_layered_window は pptDst=None のまま |
| pptDst=None で位置がリセットされる | GetWindowRect で現在位置を取得して pptDst に渡す |

---

## 6. 要件トレーサビリティ

| 子仕様要件 | 設計セクション |
|-----------|---------------|
| Req 1 (ulw_present_system) | §4.1 ulw_present_system |
| Req 2 (present_layered_window) | §4.2 present_layered_window |
| Req 3 (WS_EX_LAYERED) | §3.2 WindowStyle, §3.3 main.rs |
| Req 4 (WM_PAINT/ERASEBKGND) | §4.3 ハンドラ更新, §5 前提検証 |
| Req 5 (WM_SIZE) | §4.4 WM_SIZE |
| Req 6 (ULW失敗) | §4.1 ulw_present_system エラーハンドリング |
| Req 7 (クリックスルー) | §5.1 前提検証 |
| Req 8 (Phase 3検証) | §7 テスト戦略 |

---

## 7. テスト戦略

### 7.1 単体テスト

| テスト | 対象 | 検証内容 |
|-------|------|---------|
| `present_layered_window` BLENDFUNCTION 構成 | com/ulw.rs | AC_SRC_OVER, AC_SRC_ALPHA, SourceConstantAlpha=255 |
| ulw_present_system ダーティスキップ | compositor_systems.rs | dirty=false → ULW 呼び出しなし |

### 7.2 統合テスト

| テスト | 検証内容 |
|-------|---------|
| ULW 透過描画 | 合成ビットマップ → ULW → 透過表示が動作 |
| ULW 失敗リトライ | ULW 失敗 → warn ログ → 次フレームで再試行成功 |
| リサイズ後 ULW | WM_SIZE → resize() → 次フレームで正しいサイズの ULW |

### 7.3 E2E テスト

| テスト | 検証方法 |
|-------|---------|
| alpha=0 クリックスルー | 実機操作: 透過領域クリック → 背後ウィンドウにフォーカス移動 |
| 全 example 動作 | `cargo run --example {name}` 全 example の目視確認 |
| `cargo test` 全パス | CI 確認 |

### 7.4 前提検証テスト（Phase 3 開始前）

| テスト | 方法 | 結果の扱い |
|-------|------|-----------|
| WM_PAINT 発火 | 最小 WS_EX_LAYERED ウィンドウ + tracing | §5.2 の設計分岐に反映 |
| pptDst=None 動作 | ULW 呼び出し + ウィンドウ位置確認 | present_layered_window の引数に反映 |

---

## 8. エラーハンドリング

| エラー | 発生元 | レスポンス | リカバリ |
|--------|--------|----------|---------|
| UpdateLayeredWindow 失敗 | present_layered_window | `tracing::warn!` + フレームスキップ | 次フレーム再試行（dirty=true 維持） |
| GetWindowRect 失敗 | ulw_present_system (pptDst 取得時) | `tracing::error!` + フレームスキップ | 次フレーム再試行 |
| HDC 無効 | ulw_present_system | `tracing::error!` + compositor.invalidate() | compositor_init_system で再初期化 |
