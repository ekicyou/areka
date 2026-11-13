//! 描画面ごとの入れ物（要件 4.1, 4.2, 4.4, 4.8）。描画面 1 枚の合成の結果を持ち、合成と写しの
//! 手順を持つ（WUC と ECS を知らない）。本番の係（`render_surface`）と WARP のテストが同じ手順を呼ぶ。

// 本番の係が使い始めるまで（6.2）は、テストの外で使い手が無い。
#![cfg_attr(not(test), allow(dead_code))]

use super::visual::{DrawCommand, emit};
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::sync::atomic::AtomicU64;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::core::Interface;
use windows_numerics::{Matrix3x2, Vector2};

/// 1 画素の読み戻しの写し先と控え（同じ錠の中に置く）。
#[allow(dead_code)] // 読む手順（read_pixel）が入るまで（5.2）。
struct PixelReadState {
    /// 1×1 の `CPU_READ | CANNOT_DRAW` の写し先。
    staging: ID2D1Bitmap1,
    /// 最後に GPU から読んだ `(x, y, generation, 値)`。
    last: Option<(u32, u32, u64, [u8; 4])>,
}

/// 描画面 1 枚の入れ物（描画面と同じ物理ピクセルの大きさ・8 ビット BGRA・α 掛け済み・96 DPI）。
pub(crate) struct SurfaceComposite {
    bitmap: ID2D1Bitmap1,
    size: (u32, u32),
    /// 合成し終えた回数（0＝まだ）。
    generation: u64,
    /// 1×1 の写し先と控えを 1 つの錠に入れる（写し・Map・Unmap・控えの更新を錠を持ったまま行う）。
    #[allow(dead_code)] // 読む手順が入るまで（5.2）。
    pixel: Mutex<PixelReadState>,
    /// GPU から読んだ回数（5.4 のテストの的）。
    #[allow(dead_code)] // 読む手順が入るまで（5.2）。
    gpu_reads: AtomicU64,
}

// SAFETY: 入れ物と写し先は D2D1_FACTORY_TYPE_MULTI_THREADED の工場の系列（GraphicsCore の装置）の
// 資源で、同じ工場の系列への COM 呼び出しは D2D が内部で直列化する。写し先と控えは `Mutex` の中に置き、
// 世代と読んだ回数は `&mut self` か原子の値でだけ変わる。
unsafe impl Send for SurfaceComposite {}
unsafe impl Sync for SurfaceComposite {}

impl std::fmt::Debug for SurfaceComposite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SurfaceComposite")
            .field("size", &self.size)
            .field("generation", &self.generation)
            .finish()
    }
}

fn create_bitmap(
    dc: &ID2D1DeviceContext,
    (width, height): (u32, u32),
    options: D2D1_BITMAP_OPTIONS,
) -> windows::core::Result<ID2D1Bitmap1> {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: options,
        colorContext: ManuallyDrop::new(None),
    };
    // SAFETY: Windows API 境界。初期の画素は渡さない。
    unsafe { dc.CreateBitmap(D2D_SIZE_U { width, height }, None, 0, &props) }
}

impl SurfaceComposite {
    /// `dc` で入れ物と 1×1 の写し先を作る。
    pub(crate) fn new(dc: &ID2D1DeviceContext, size: (u32, u32)) -> windows::core::Result<Self> {
        let bitmap = create_bitmap(dc, size, D2D1_BITMAP_OPTIONS_TARGET)?;
        let staging = create_bitmap(
            dc,
            (1, 1),
            D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        )?;
        Ok(Self {
            bitmap,
            size,
            generation: 0,
            pixel: Mutex::new(PixelReadState {
                staging,
                last: None,
            }),
            gpu_reads: AtomicU64::new(0),
        })
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        self.size
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// 入れ物を透明に塗り、根の命令の列を `SOURCE_OVER` で、木の列を `emit` で描く（4.1・4.8）。
    /// `scale` は描画面の拡大（`GlobalArrangement` の拡大・offset なし）。成功したら世代を 1 上げる。
    /// 戻るとき `dc` は結果に依らず的なし・変換は恒等。
    pub(crate) fn compose(
        &mut self,
        dc: &ID2D1DeviceContext,
        scale: Matrix3x2,
        root_list: Option<&ID2D1CommandList>,
        tree: &[DrawCommand],
    ) -> windows::core::Result<()> {
        // SAFETY: Windows API 境界。`dc` は UI スレッドの係だけが使う共有の DC（呼び手の約束）。
        let result = unsafe {
            dc.SetTarget(&self.bitmap);
            dc.SetDpi(96.0, 96.0);
            dc.BeginDraw();
            dc.SetTransform(&scale);
            dc.SetPrimitiveBlend(D2D1_PRIMITIVE_BLEND_SOURCE_OVER);
            dc.Clear(Some(&D2D1_COLOR_F::default()));
            if let Some(list) = root_list {
                dc.DrawImage(
                    list,
                    None,
                    None,
                    D2D1_INTERPOLATION_MODE_LINEAR,
                    D2D1_COMPOSITE_MODE_SOURCE_OVER,
                );
            }
            if !tree.is_empty() {
                let dc6 = dc.cast::<ID2D1DeviceContext6>().ok();
                emit(dc, dc6.as_ref(), scale, tree);
            }
            let result = dc.EndDraw(None, None);
            dc.SetTransform(&Matrix3x2::identity());
            dc.SetTarget(None);
            result
        };
        if result.is_ok() {
            self.generation += 1;
        }
        result
    }

    /// `dst`（`BeginDraw` の中の DC）を透明に塗り、入れ物を整数の `offset` に `NEAREST_NEIGHBOR`・
    /// `SOURCE_OVER` で 1 度描く（4.2）。入れ物は読むだけ（4.4）。失敗は `dst` の `EndDraw` に出る。
    pub(crate) fn copy_to(&self, dst: &ID2D1DeviceContext, offset: POINT) {
        // SAFETY: Windows API 境界。
        unsafe {
            dst.SetTransform(&Matrix3x2::identity());
            dst.Clear(Some(&D2D1_COLOR_F::default()));
            dst.DrawImage(
                &self.bitmap,
                Some(&Vector2 {
                    X: offset.x as f32,
                    Y: offset.y as f32,
                }),
                None,
                D2D1_INTERPOLATION_MODE_NEAREST_NEIGHBOR,
                D2D1_COMPOSITE_MODE_SOURCE_OVER,
            );
        }
    }
}
