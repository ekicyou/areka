use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::core::*;
use windows_numerics::*;

/// D2D1CreateDevice
pub fn d2d_create_device(dxgi: &IDXGIDevice4) -> Result<ID2D1Device> {
    unsafe { D2D1CreateDevice(dxgi, None) }
}

pub trait D2D1DeviceExt {
    /// CreateDeviceContext
    fn create_device_context(
        &self,
        options: D2D1_DEVICE_CONTEXT_OPTIONS,
    ) -> Result<ID2D1DeviceContext>;
}

impl D2D1DeviceExt for ID2D1Device {
    #[inline(always)]
    fn create_device_context(
        &self,
        options: D2D1_DEVICE_CONTEXT_OPTIONS,
    ) -> Result<ID2D1DeviceContext> {
        unsafe { self.CreateDeviceContext(options) }
    }
}

pub trait D2D1CommandListExt {
    /// CommandListを閉じる
    fn close(&self) -> Result<()>;
}

impl D2D1CommandListExt for ID2D1CommandList {
    #[inline(always)]
    fn close(&self) -> Result<()> {
        unsafe { self.Close() }
    }
}

pub trait D2D1DeviceContextExt {
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
}

impl D2D1DeviceContextExt for ID2D1DeviceContext {
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
}
