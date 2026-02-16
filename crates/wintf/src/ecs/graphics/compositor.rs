//! WindowD3D11Compositor — per-window D2D1 合成描画リソース管理コンポーネント
//!
//! DComp Visual ツリーに依存しない D2D1 ソフトウェア合成描画基盤を提供する。
//! 合成描画先ビットマップ、CPU ステージングビットマップ、GDI HBITMAP、MemoryDC の
//! 4リソースを統合管理し、常に同一サイズ・同一ピクセルフォーマット（PBGRA32）を維持する。

use bevy_ecs::prelude::*;
use std::mem;
use tracing::debug;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Gdi::*;

/// リソースの内部構造体（COM + GDI リソース保持）
struct WindowD3D11CompositorInner {
    /// 合成描画先ビットマップ（D2D1_BITMAP_OPTIONS_TARGET）
    composition_bitmap: ID2D1Bitmap1,
    /// CPU ステージングビットマップ（D2D1_BITMAP_OPTIONS_CPU_READ | CANNOT_DRAW）
    staging_bitmap: ID2D1Bitmap1,
    /// GDI HBITMAP（CreateDIBSection PBGRA32 top-down DIB）
    hbitmap: HBITMAP,
    /// GDI メモリデバイスコンテキスト
    memory_dc: HDC,
    /// DIBSection ピクセルデータポインタ
    dib_bits: *mut u8,
}

impl Drop for WindowD3D11CompositorInner {
    fn drop(&mut self) {
        unsafe {
            // GDI リソースを手動解放（COM リソースはスマートポインタで自動解放）
            if !self.memory_dc.is_invalid() {
                let _ = DeleteDC(self.memory_dc);
            }
            if !self.hbitmap.is_invalid() {
                let _ = DeleteObject(self.hbitmap.into());
            }
        }
    }
}

/// ウィンドウごとの D2D1 合成描画リソースを統合管理する ECS コンポーネント。
///
/// 4リソース（composition_bitmap, staging_bitmap, hbitmap, memory_dc）の
/// ライフサイクルを一貫して管理し、`Drop` で GDI リソースを確実に解放する。
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct WindowD3D11Compositor {
    inner: Option<WindowD3D11CompositorInner>,
    generation: u32,
    dirty: bool,
    cached_size: (u32, u32),
}

unsafe impl Send for WindowD3D11Compositor {}
unsafe impl Sync for WindowD3D11Compositor {}

impl std::fmt::Debug for WindowD3D11Compositor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowD3D11Compositor")
            .field("is_valid", &self.is_valid())
            .field("generation", &self.generation)
            .field("dirty", &self.dirty)
            .field("cached_size", &self.cached_size)
            .finish()
    }
}

/// D2D1 Bitmap 作成パラメータ定数
fn composition_bitmap_props() -> D2D1_BITMAP_PROPERTIES1 {
    D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET,
        dpiX: 96.0,
        dpiY: 96.0,
        ..Default::default()
    }
}

fn staging_bitmap_props() -> D2D1_BITMAP_PROPERTIES1 {
    D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        bitmapOptions: D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        dpiX: 96.0,
        dpiY: 96.0,
        ..Default::default()
    }
}

/// GDI DIBSection（HBITMAP + MemoryDC）を作成する。
///
/// # Safety
/// GDI API を直接呼び出す。
unsafe fn create_dib_section(
    width: u32,
    height: u32,
) -> windows::core::Result<(HBITMAP, HDC, *mut u8)> {
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut dib_bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let hbitmap = unsafe { CreateDIBSection(None, &bmi, DIB_RGB_COLORS, &mut dib_bits, None, 0)? };
    if hbitmap.is_invalid() || dib_bits.is_null() {
        return Err(windows::core::Error::from(E_FAIL));
    }

    let memory_dc = unsafe { CreateCompatibleDC(None) };
    if memory_dc.is_invalid() {
        let _ = unsafe { DeleteObject(hbitmap.into()) };
        return Err(windows::core::Error::from(E_FAIL));
    }

    // HBITMAP を DC に関連付け
    let _old = unsafe { SelectObject(memory_dc, hbitmap.into()) };

    Ok((hbitmap, memory_dc, dib_bits as *mut u8))
}

/// 4リソース（composition_bitmap, staging_bitmap, hbitmap, memory_dc）を一括作成する。
unsafe fn create_inner(
    dc: &ID2D1DeviceContext,
    width: u32,
    height: u32,
) -> windows::core::Result<WindowD3D11CompositorInner> {
    let size = D2D_SIZE_U { width, height };

    // D2D1 Bitmap 作成
    let composition_bitmap =
        unsafe { dc.CreateBitmap(size, None, 0, &composition_bitmap_props())? };
    let staging_bitmap = unsafe { dc.CreateBitmap(size, None, 0, &staging_bitmap_props())? };

    // GDI DIBSection + MemoryDC 作成
    let (hbitmap, memory_dc, dib_bits) = unsafe { create_dib_section(width, height)? };

    Ok(WindowD3D11CompositorInner {
        composition_bitmap,
        staging_bitmap,
        hbitmap,
        memory_dc,
        dib_bits,
    })
}

impl WindowD3D11Compositor {
    /// 全4リソースを一括作成する。
    ///
    /// # Safety
    /// `dc` は有効な `ID2D1DeviceContext` であること。`width > 0`, `height > 0` であること。
    pub fn new(dc: &ID2D1DeviceContext, width: u32, height: u32) -> windows::core::Result<Self> {
        let inner = unsafe { create_inner(dc, width, height)? };
        debug!(
            width = width,
            height = height,
            "[WindowD3D11Compositor::new] Resources created"
        );
        Ok(Self {
            inner: Some(inner),
            generation: 0,
            dirty: false,
            cached_size: (width, height),
        })
    }

    /// 新サイズで全リソースを再作成する。generation をインクリメントする。
    pub fn resize(
        &mut self,
        dc: &ID2D1DeviceContext,
        width: u32,
        height: u32,
    ) -> windows::core::Result<()> {
        // 旧リソースを Drop（inner を置き換え）
        let new_inner = unsafe { create_inner(dc, width, height)? };
        self.inner = Some(new_inner);
        self.generation = self.generation.wrapping_add(1);
        self.dirty = false;
        self.cached_size = (width, height);
        debug!(
            width = width,
            height = height,
            generation = self.generation,
            "[WindowD3D11Compositor::resize] Resources recreated"
        );
        Ok(())
    }

    /// 全リソースを無効化する（リソース解放は Drop に委譲）。
    pub fn invalidate(&mut self) {
        self.inner = None;
        self.dirty = false;
    }

    /// リソースが有効かどうかを返す。
    pub fn is_valid(&self) -> bool {
        self.inner.is_some()
    }

    /// 合成描画先ビットマップへの参照を返す。
    pub fn composition_bitmap(&self) -> Option<&ID2D1Bitmap1> {
        self.inner.as_ref().map(|i| &i.composition_bitmap)
    }

    /// CPU ステージングビットマップへの参照を返す。
    pub fn staging_bitmap(&self) -> Option<&ID2D1Bitmap1> {
        self.inner.as_ref().map(|i| &i.staging_bitmap)
    }

    /// GDI HBITMAP を返す。
    pub fn hbitmap(&self) -> Option<HBITMAP> {
        self.inner.as_ref().map(|i| i.hbitmap)
    }

    /// GDI メモリデバイスコンテキストを返す。
    pub fn memory_dc(&self) -> Option<HDC> {
        self.inner.as_ref().map(|i| i.memory_dc)
    }

    /// DIBSection ピクセルデータポインタを返す。
    pub fn dib_bits(&self) -> Option<*mut u8> {
        self.inner.as_ref().map(|i| i.dib_bits)
    }

    /// キャッシュされたリソースサイズ（幅, 高さ）を返す。
    pub fn cached_size(&self) -> (u32, u32) {
        self.cached_size
    }

    /// リソース世代カウンタを返す。
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// generation を手動インクリメントする（再作成時に旧世代を引き継ぐ用）。
    pub fn increment_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// 合成完了フラグを返す（Phase 3 `ulw_present_system` が消費）。
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// 合成完了フラグを設定する。
    pub fn set_dirty(&mut self, v: bool) {
        self.dirty = v;
    }
}
