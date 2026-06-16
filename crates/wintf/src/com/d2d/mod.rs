pub mod command_sink;
pub mod command_types;
pub use command_sink::*;
pub use command_types::*;

use ambassador::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Imaging::*;
use windows::core::*;
use windows_numerics::*;

/// D2D1CreateDevice
pub fn d2d_create_device(dxgi: &IDXGIDevice4) -> Result<ID2D1Device> {
    unsafe { D2D1CreateDevice(dxgi, None) }
}

pub trait D2D1FactoryExt {
    /// CreatePathGeometry
    fn create_path_geometry(&self) -> Result<ID2D1PathGeometry>;
    /// CreateRoundedRectangleGeometry
    fn create_rounded_rectangle_geometry(
        &self,
        rounded_rect: &D2D1_ROUNDED_RECT,
    ) -> Result<ID2D1RoundedRectangleGeometry>;
}

impl D2D1FactoryExt for ID2D1Factory {
    #[inline(always)]
    fn create_path_geometry(&self) -> Result<ID2D1PathGeometry> {
        unsafe { self.CreatePathGeometry() }
    }

    #[inline(always)]
    fn create_rounded_rectangle_geometry(
        &self,
        rounded_rect: &D2D1_ROUNDED_RECT,
    ) -> Result<ID2D1RoundedRectangleGeometry> {
        unsafe { self.CreateRoundedRectangleGeometry(rounded_rect) }
    }
}

pub trait D2D1DeviceExt {
    /// CreateDeviceContext
    fn create_device_context(
        &self,
        options: D2D1_DEVICE_CONTEXT_OPTIONS,
    ) -> Result<ID2D1DeviceContext>;
    /// CreateCommandList
    fn create_command_list(&self) -> Result<ID2D1CommandList>;
}

impl D2D1DeviceExt for ID2D1Device {
    #[inline(always)]
    fn create_device_context(
        &self,
        options: D2D1_DEVICE_CONTEXT_OPTIONS,
    ) -> Result<ID2D1DeviceContext> {
        unsafe { self.CreateDeviceContext(options) }
    }

    #[inline(always)]
    fn create_command_list(&self) -> Result<ID2D1CommandList> {
        unsafe {
            let dc = self.create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            dc.CreateCommandList()
        }
    }
}

pub trait D2D1CommandListExt {
    /// CommandListを開いてDeviceContextを取得
    fn open(&self) -> Result<ID2D1DeviceContext>;
    /// CommandListを閉じる
    fn close(&self) -> Result<()>;
}

impl D2D1CommandListExt for ID2D1CommandList {
    #[inline(always)]
    fn open(&self) -> Result<ID2D1DeviceContext> {
        // ID2D1CommandListは直接Openできない
        // 代わりにキャストでID2D1Imageとして扱う
        Err(Error::from_hresult(windows::Win32::Foundation::E_NOTIMPL))
    }

    #[inline(always)]
    fn close(&self) -> Result<()> {
        unsafe { self.Close() }
    }
}

#[delegatable_trait]
pub trait D2D1DeviceContextExt {
    /// CreateBitmapFromWicBitmap
    fn create_bitmap_from_wic_bitmap<P0>(&self, wicbitmapsource: P0) -> Result<ID2D1Bitmap1>
    where
        P0: Param<IWICBitmapSource>;
    /// SetTransform
    fn set_transform(&self, transform: &Matrix3x2);
    /// Clear
    fn clear(&self, color: Option<&D2D1_COLOR_F>);
    /// CreateSolidColorBrush
    fn create_solid_color_brush(
        &self,
        color: &D2D1_COLOR_F,
        brush_properties: Option<&D2D1_BRUSH_PROPERTIES>,
    ) -> Result<ID2D1SolidColorBrush>;
    /// DrawText
    fn draw_text<P0, P1, P2>(
        &self,
        string: P0,
        text_format: P1,
        layout_rect: &D2D_RECT_F,
        default_fill_brush: P2,
        options: D2D1_DRAW_TEXT_OPTIONS,
        measuring_mode: DWRITE_MEASURING_MODE,
    ) where
        P0: Param<HSTRING>,
        P1: Param<IDWriteTextFormat>,
        P2: Param<ID2D1Brush>;

    /// DrawTextLayout
    fn draw_text_layout<P0, P1>(
        &self,
        origin: Vector2,
        text_layout: P0,
        default_fill_brush: P1,
        options: D2D1_DRAW_TEXT_OPTIONS,
    ) where
        P0: Param<IDWriteTextLayout>,
        P1: Param<ID2D1Brush>;

    fn draw_bitmap<P0>(
        &self,
        bitmap: P0,
        destinationrectangle: Option<&D2D_RECT_F>,
        opacity: f32,
        interpolationmode: D2D1_INTERPOLATION_MODE,
        sourcerectangle: Option<&D2D_RECT_F>,
        perspectivetransform: Option<&Matrix4x4>,
    ) where
        P0: Param<ID2D1Bitmap>;

    /// FillEllipse
    fn fill_ellipse<P0>(&self, ellipse: &D2D1_ELLIPSE, brush: P0)
    where
        P0: Param<ID2D1Brush>;

    /// FillRectangle
    fn fill_rectangle<P0>(&self, rect: &D2D_RECT_F, brush: P0)
    where
        P0: Param<ID2D1Brush>;

    /// FillGeometry
    fn fill_geometry<P0, P1>(&self, geometry: P0, brush: P1)
    where
        P0: Param<ID2D1Geometry>,
        P1: Param<ID2D1Brush>;

    /// DrawImage
    fn draw_image<P0>(&self, image: P0)
    where
        P0: Param<ID2D1Image>;
}

impl D2D1DeviceContextExt for ID2D1DeviceContext {
    #[inline(always)]
    fn create_bitmap_from_wic_bitmap<P0>(&self, wicbitmapsource: P0) -> Result<ID2D1Bitmap1>
    where
        P0: Param<IWICBitmapSource>,
    {
        unsafe { self.CreateBitmapFromWicBitmap(wicbitmapsource, None) }
    }
    #[inline(always)]
    fn set_transform(&self, transform: &Matrix3x2) {
        unsafe { self.SetTransform(transform) }
    }
    #[inline(always)]
    fn clear(&self, color: Option<&D2D1_COLOR_F>) {
        let color_ptr = color.map(|c| c as *const D2D1_COLOR_F);
        unsafe { self.Clear(color_ptr) }
    }

    #[inline(always)]
    fn create_solid_color_brush(
        &self,
        color: &D2D1_COLOR_F,
        brush_properties: Option<&D2D1_BRUSH_PROPERTIES>,
    ) -> Result<ID2D1SolidColorBrush> {
        let brush_properties_ptr = brush_properties.map(|p| p as *const D2D1_BRUSH_PROPERTIES);
        unsafe { self.CreateSolidColorBrush(color, brush_properties_ptr) }
    }

    #[inline(always)]
    fn draw_text<P0, P1, P2>(
        &self,
        string: P0,
        text_format: P1,
        layout_rect: &D2D_RECT_F,
        default_fill_brush: P2,
        options: D2D1_DRAW_TEXT_OPTIONS,
        measuring_mode: DWRITE_MEASURING_MODE,
    ) where
        P0: Param<HSTRING>,
        P1: Param<IDWriteTextFormat>,
        P2: Param<ID2D1Brush>,
    {
        unsafe {
            let hstring = string.param();
            let hstring_borrow = hstring.borrow();
            // SAFETY 根拠: この unwrap は失敗し得ない。`Ref<HSTRING>::as_ref()` は
            // `Type<HSTRING>::is_null()` が None 判定の唯一の経路だが、HSTRING は
            // CloneType（windows-core 0.62.2 windows.rs: `impl TypeKind for HSTRING`）
            // であり `Type<T, CloneType>::is_null()` は常に false を返す
            // （同 type.rs）。空 HSTRING（内部表現 null）でも Some(&HSTRING) が
            // 返り、Deref で空スライス &[] として DrawText へ渡るため panic 経路は
            // 到達不能（tests/com/d2d_ext_test.rs の空 HSTRING 特性化テストで保護）。
            let string = hstring_borrow.as_ref().unwrap();
            self.DrawText(
                &string,
                text_format,
                layout_rect,
                default_fill_brush,
                options,
                measuring_mode,
            )
        }
    }

    #[inline(always)]
    fn draw_text_layout<P0, P1>(
        &self,
        origin: Vector2,
        text_layout: P0,
        default_fill_brush: P1,
        options: D2D1_DRAW_TEXT_OPTIONS,
    ) where
        P0: Param<IDWriteTextLayout>,
        P1: Param<ID2D1Brush>,
    {
        unsafe { self.DrawTextLayout(origin, text_layout, default_fill_brush, options) }
    }

    #[inline(always)]
    fn draw_bitmap<P0>(
        &self,
        bitmap: P0,
        destinationrectangle: Option<&D2D_RECT_F>,
        opacity: f32,
        interpolationmode: D2D1_INTERPOLATION_MODE,
        sourcerectangle: Option<&D2D_RECT_F>,
        perspectivetransform: Option<&Matrix4x4>,
    ) where
        P0: Param<ID2D1Bitmap>,
    {
        let destinationrectangle = destinationrectangle.map(|r| r as *const _);
        let sourcerectangle = sourcerectangle.map(|r| r as *const _);
        let perspectivetransform = perspectivetransform.map(|r| r as *const _);
        unsafe {
            self.DrawBitmap(
                bitmap,
                destinationrectangle,
                opacity,
                interpolationmode,
                sourcerectangle,
                perspectivetransform,
            )
        }
    }

    #[inline(always)]
    fn fill_ellipse<P0>(&self, ellipse: &D2D1_ELLIPSE, brush: P0)
    where
        P0: Param<ID2D1Brush>,
    {
        unsafe { self.FillEllipse(ellipse, brush) }
    }

    #[inline(always)]
    fn fill_rectangle<P0>(&self, rect: &D2D_RECT_F, brush: P0)
    where
        P0: Param<ID2D1Brush>,
    {
        unsafe { self.FillRectangle(rect, brush) }
    }

    #[inline(always)]
    fn fill_geometry<P0, P1>(&self, geometry: P0, brush: P1)
    where
        P0: Param<ID2D1Geometry>,
        P1: Param<ID2D1Brush>,
    {
        unsafe { self.FillGeometry(geometry, brush, None) }
    }

    #[inline(always)]
    fn draw_image<P0>(&self, image: P0)
    where
        P0: Param<ID2D1Image>,
    {
        unsafe {
            self.DrawImage(
                image,
                None,
                None,
                D2D1_INTERPOLATION_MODE_LINEAR,
                D2D1_COMPOSITE_MODE_SOURCE_OVER,
            )
        }
    }
}
