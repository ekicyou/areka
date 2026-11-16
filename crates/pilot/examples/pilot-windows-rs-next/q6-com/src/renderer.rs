// 下線の位置を集める描き手（6.2）。元の viewbox_draw_underline.rs は写さず新しく書いた。
// `_Impl` のメソッド名は COM の PascalCase のまま。
#![allow(non_snake_case)]

use std::cell::RefCell;

use bind_next::Windows::Win32::*;
use windows_core::{AsImpl, BOOL, HRESULT, IUnknown, Interface, Ref, Result, implement};

/// 集めた下線 1 本（`DrawUnderline` の引数から）。
#[derive(Debug, Clone, Copy)]
pub struct Underline {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub thickness: f32,
    /// `IDWriteTextRenderer1` の向きの角（`DWRITE_GLYPH_ORIENTATION_ANGLE`）。基の版から来たら 0。
    pub orientation: i32,
    /// 角つきの `IDWriteTextRenderer1::DrawUnderline` から来たか。
    pub renderer1: bool,
}

#[implement(IDWriteTextRenderer1)]
struct Collector {
    lines: RefCell<Vec<Underline>>,
}

impl Collector {
    fn push(
        &self,
        x: f32,
        y: f32,
        u: *const DWRITE_UNDERLINE,
        orientation: i32,
        renderer1: bool,
    ) -> HRESULT {
        let Some(u) = (unsafe { u.as_ref() }) else {
            return HRESULT(0x8000_4003u32 as i32); // E_POINTER
        };
        self.lines.borrow_mut().push(Underline {
            x,
            y: y + u.offset,
            width: u.width,
            thickness: u.thickness,
            orientation,
            renderer1,
        });
        HRESULT(0)
    }
}

impl IDWritePixelSnapping_Impl for Collector_Impl {
    fn IsPixelSnappingDisabled(&self, _: *const core::ffi::c_void, out: *mut BOOL) -> HRESULT {
        unsafe { *out = false.into() };
        HRESULT(0)
    }
    fn GetCurrentTransform(&self, _: *const core::ffi::c_void, out: *mut DWRITE_MATRIX) -> HRESULT {
        unsafe {
            *out = DWRITE_MATRIX {
                m11: 1.0,
                m12: 0.0,
                m21: 0.0,
                m22: 1.0,
                dx: 0.0,
                dy: 0.0,
            }
        };
        HRESULT(0)
    }
    fn GetPixelsPerDip(&self, _: *const core::ffi::c_void, out: *mut f32) -> HRESULT {
        unsafe { *out = 1.0 };
        HRESULT(0)
    }
}

// 基の版の 4 つ。DirectWrite が `IDWriteTextRenderer1` を問い合わせて見つけると角つきの版を呼ぶ。
impl IDWriteTextRenderer_Impl for Collector_Impl {
    fn DrawGlyphRun(
        &self,
        _: *const core::ffi::c_void,
        _: f32,
        _: f32,
        _: DWRITE_MEASURING_MODE,
        _: *const DWRITE_GLYPH_RUN,
        _: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        _: Ref<IUnknown>,
    ) -> HRESULT {
        HRESULT(0)
    }
    fn DrawUnderline(
        &self,
        _: *const core::ffi::c_void,
        x: f32,
        y: f32,
        u: *const DWRITE_UNDERLINE,
        _: Ref<IUnknown>,
    ) -> HRESULT {
        self.push(x, y, u, 0, false)
    }
    fn DrawStrikethrough(
        &self,
        _: *const core::ffi::c_void,
        _: f32,
        _: f32,
        _: *const DWRITE_STRIKETHROUGH,
        _: Ref<IUnknown>,
    ) -> HRESULT {
        HRESULT(0)
    }
    fn DrawInlineObject(
        &self,
        _: *const core::ffi::c_void,
        _: f32,
        _: f32,
        _: Ref<IDWriteInlineObject>,
        _: BOOL,
        _: BOOL,
        _: Ref<IUnknown>,
    ) -> HRESULT {
        HRESULT(0)
    }
}

impl IDWriteTextRenderer1_Impl for Collector_Impl {
    fn DrawGlyphRun(
        &self,
        _: *const core::ffi::c_void,
        _: f32,
        _: f32,
        _: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _: DWRITE_MEASURING_MODE,
        _: *const DWRITE_GLYPH_RUN,
        _: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        _: Ref<IUnknown>,
    ) -> HRESULT {
        HRESULT(0)
    }
    fn DrawUnderline(
        &self,
        _: *const core::ffi::c_void,
        x: f32,
        y: f32,
        angle: DWRITE_GLYPH_ORIENTATION_ANGLE,
        u: *const DWRITE_UNDERLINE,
        _: Ref<IUnknown>,
    ) -> HRESULT {
        self.push(x, y, u, angle, true)
    }
    fn DrawStrikethrough(
        &self,
        _: *const core::ffi::c_void,
        _: f32,
        _: f32,
        _: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _: *const DWRITE_STRIKETHROUGH,
        _: Ref<IUnknown>,
    ) -> HRESULT {
        HRESULT(0)
    }
    fn DrawInlineObject(
        &self,
        _: *const core::ffi::c_void,
        _: f32,
        _: f32,
        _: DWRITE_GLYPH_ORIENTATION_ANGLE,
        _: Ref<IDWriteInlineObject>,
        _: BOOL,
        _: BOOL,
        _: Ref<IUnknown>,
    ) -> HRESULT {
        HRESULT(0)
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// 「下線 underline」の全体に下線を付けた文字の配置を、集める描き手で描く。
pub fn collect_underlines() -> Result<Vec<Underline>> {
    unsafe {
        let factory: IDWriteFactory =
            DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IDWriteFactory::IID)?.cast()?;
        let format = factory.CreateTextFormat(
            wide("Segoe UI").as_ptr(),
            None,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            24.0,
            wide("ja-jp").as_ptr(),
        )?;
        let text: Vec<u16> = "下線 underline".encode_utf16().collect();
        let layout =
            factory.CreateTextLayout(text.as_ptr(), text.len() as u32, &format, 400.0, 100.0)?;
        layout
            .SetUnderline(
                true,
                DWRITE_TEXT_RANGE {
                    startPosition: 0,
                    length: text.len() as u32,
                },
            )
            .ok()?;
        let renderer: IDWriteTextRenderer1 = Collector {
            lines: RefCell::new(Vec::new()),
        }
        .into();
        layout.Draw(None, &renderer, 0.0, 0.0).ok()?;
        Ok(renderer.as_impl().lines.take())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn collects_at_least_one_underline() {
        let lines = super::collect_underlines().unwrap();
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|u| u.width > 0.0 && u.thickness > 0.0));
    }
}
