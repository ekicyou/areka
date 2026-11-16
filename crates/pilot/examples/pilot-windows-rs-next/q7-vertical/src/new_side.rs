//! 新しい側: 上流 `windows-text`（`windows-core` 0.101）で縦書きの 1 列を配置し、`bind-next` の
//! 束縛（同じ 0.101）の `IDWriteTextLayout` として借りて、ハードウェアの D3D11＋D2D の画面の外の
//! 絵へ描き、premultiplied BGRA を読み戻す（問い 7・要件 8.2・8.3・15.1）。
//!
//! 条件は今の側（`old_side`）の表（README 問い 7）に揃える。値の出どころは各行の注に書く。
//! 新旧の COM の型はつながない: 借りた側（0.62）の型はここに 1 つも出ず、`windows-text` と
//! `bind-next` は同じ `windows-core` 0.101 の上なので `as_interface` で借りるだけ（生ポインタの橋は無い）。

use bind_next::Windows::Win32::*;
use windows_core::{Error, HRESULT, Interface, Result};
use windows_text::{
    FontCollection, FontFeature, FontStyle, FontWeight, LineSpacing, TextFormat, TextLayout,
    Vector2, WritingMode,
};

use crate::diff::Shot;
use crate::old_side::{FONT_HEIGHT, IMAGE, PASTA_FONT, VALIDRECT_INSET};

const E_FAIL: HRESULT = HRESULT(0x8000_4005_u32 as i32);
/// 今の側の行の箱の行内軸（折り返さない寸・`areka-emo-text/src/draw.rs` の `PROBE_MAX_EXTENT`）。
const PROBE_MAX_EXTENT: f32 = 1.0e6;
/// 今の側の書式の言語（`areka-emo-text/src/draw.rs` の `LOCALE_JA_JP`）。
const LOCALE: &str = "ja-JP";

/// areka の縦書きの機能の組（`areka-emo-text/src/draw.rs` の `VERTICAL_FEATURES` と同じ 10 個・値 1）。
pub const VERTICAL_FEATURES: [FontFeature; 10] = [
    FontFeature::new(b"ccmp", 1),
    FontFeature::new(b"locl", 1),
    FontFeature::new(b"mark", 1),
    FontFeature::new(b"mkmk", 1),
    FontFeature::new(b"rlig", 1),
    FontFeature::new(b"calt", 1),
    FontFeature::new(b"clig", 1),
    FontFeature::new(b"liga", 1),
    FontFeature::new(b"kern", 1),
    FontFeature::new(b"rclt", 1),
];

/// 字形の指定の 2 通り。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Features {
    /// 指定しない（DirectWrite の既定の組）。
    None,
    /// [`VERTICAL_FEATURES`] を全文字範囲に当てる（今の側と同じ）。
    Areka,
}

/// 面の大きさ（今の側の供給面＝validrect の内側と同じ）。
pub const SURFACE: (u32, u32) = (IMAGE.0 - 2 * VALIDRECT_INSET, IMAGE.1 - 2 * VALIDRECT_INSET);

/// 解いた顔（家族名・太さ・斜体）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Face {
    pub family: String,
    pub weight: FontWeight,
    pub style: FontStyle,
}

/// ぱすたの候補列を今の側の `FontCatalog::face_for` と同じ順で解く: 候補ごとに家族名（顔は Regular）→
/// GDI の名前。家族名は `windows-text` の `FontCollection::find` で引くが、GDI の名前で引く口が
/// `windows-text` に無いので、その枝は `bind-next` の DirectWrite（`CreateFontFromLOGFONT`）を直に呼ぶ。
/// shortcut: 全滅は Err（今の側は既定の書体へ戻す）。比べの文はぱすたの候補で解けるので要らない。
pub fn face() -> std::result::Result<Face, String> {
    let system = FontCollection::system();
    let gdi = unsafe {
        DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IDWriteFactory::IID)
            .and_then(|f| f.cast::<IDWriteFactory>())
            .and_then(|f| f.GetGdiInterop())
    }
    .map_err(|e| format!("GetGdiInterop: {e:?}"))?;
    PASTA_FONT
        .split(',')
        .find_map(|name| {
            if system
                .find(name, FontWeight::NORMAL, FontStyle::Normal)
                .is_some()
            {
                return Some(Face {
                    family: name.to_owned(),
                    weight: FontWeight::NORMAL,
                    style: FontStyle::Normal,
                });
            }
            unsafe { gdi_face(&gdi, name) }
        })
        .ok_or_else(|| format!("候補が 1 つも解けない: {PASTA_FONT}"))
}

/// 今の側の `gdi_face`＋`face_of`（太さ 400・DEFAULT_CHARSET の LOGFONT・家族名は `GetFamilyNames` の先頭）。
/// 失敗はどれも「無い」（今の側は想定外の失敗を記録してから「無い」）。
unsafe fn gdi_face(gdi: &IDWriteGdiInterop, name: &str) -> Option<Face> {
    let units: Vec<u16> = name.encode_utf16().collect();
    if units.len() > 31 {
        return None;
    }
    let mut logfont = LOGFONTW {
        lfWeight: 400,
        lfCharSet: DEFAULT_CHARSET as u8,
        ..Default::default()
    };
    logfont.lfFaceName[..units.len()].copy_from_slice(&units);
    unsafe {
        let font = gdi.CreateFontFromLOGFONT(&logfont).ok()?;
        // windows-text の書式は幅を NORMAL に固定する（幅の setter が無い）。違う顔は揃えられないので断る。
        if font.GetStretch() != DWRITE_FONT_STRETCH_NORMAL {
            return None;
        }
        let names = font.GetFontFamily().ok()?.GetFamilyNames().ok()?;
        let mut len = 0;
        names.GetStringLength(0, &mut len).ok().ok()?;
        let mut buffer = vec![0u16; len as usize + 1];
        names
            .GetString(0, buffer.as_mut_ptr(), buffer.len() as u32)
            .ok()
            .ok()?;
        let style = match font.GetStyle() {
            DWRITE_FONT_STYLE_OBLIQUE => FontStyle::Oblique,
            DWRITE_FONT_STYLE_ITALIC => FontStyle::Italic,
            _ => FontStyle::Normal,
        };
        Some(Face {
            family: String::from_utf16_lossy(&buffer[..len as usize]),
            weight: FontWeight(font.GetWeight() as u16),
            style,
        })
    }
}

/// 今の側の `create_text_format`＋`create_line_layout`（縦書き）と同じ配置を `windows-text` で組む。
/// 8.4 は返った配置を借りて範囲の色を付けてから [`render`] へ渡す。
pub fn layout(text: &str, features: Features) -> std::result::Result<TextLayout, String> {
    let em = FONT_HEIGHT as f32;
    let face = face()?;
    let format = TextFormat::new(&face.family, em)
        .with_weight(face.weight)
        .with_style(face.style)
        .with_locale(LOCALE)
        .with_writing_mode(WritingMode::VerticalRightToLeft)
        // 列の幅＝em・基線はその真ん中（今の側の `SetLineSpacing(UNIFORM, block_extent, block_extent/2)`）。
        .with_line_spacing(LineSpacing::Uniform {
            height: em,
            baseline: em / 2.0,
        });
    let mut layout = TextLayout::new(text, &format, em, PROBE_MAX_EXTENT);
    if features == Features::Areka {
        layout.set_font_features(.., &VERTICAL_FEATURES);
    }
    Ok(layout)
}

/// `text` を新しい側で描いて読み戻す（`vt-compare` が呼ぶ口）。
pub fn draw(text: &str, features: Features) -> std::result::Result<Shot, String> {
    render(&layout(text, features)?)
}

/// 配置を画面の外の絵へ 1 回描き、読み戻す。列は面の右端に接し、先頭の字の枠の上端が y=0
/// （今の側の行の原点＝validrect の内側の右端から 1 列分の左・上端）。
pub fn render(layout: &TextLayout) -> std::result::Result<Shot, String> {
    // 今の側と同じ MTA。既に初期化済み（S_FALSE）などの戻り値は今の側と同じく見ない。
    // SAFETY: 予約の引数は None・COINIT_MULTITHREADED（0）。
    let _ = unsafe { CoInitializeEx(None, 0) };
    unsafe { render_mta(layout) }.map_err(|e| format!("新しい側の描画: {e:?}"))
}

unsafe fn render_mta(layout: &TextLayout) -> Result<Shot> {
    let (width, height) = SURFACE;
    let (d3d, context) = unsafe { create_d3d() }?;
    let dc = unsafe { create_dc(&d3d) }?;

    // 描画面のテクスチャ（今の側の `create_transparent_source_tex`: B8G8R8A8・DEFAULT・RENDER_TARGET・全 0）。
    let mut desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_RENDER_TARGET as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let zeros = vec![0u8; (width * height * 4) as usize];
    let init = D3D11_SUBRESOURCE_DATA {
        pSysMem: zeros.as_ptr() as *const _,
        SysMemPitch: width * 4,
        SysMemSlicePitch: 0,
    };
    let tex = unsafe { texture(&d3d, &desc, Some(&init)) }?;

    // 描き先の bitmap（今の側の `create_d2d_target_bitmap` と同じ props）。
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        colorContext: core::mem::ManuallyDrop::new(None),
    };
    let surface: IDXGISurface = tex.cast()?;
    let target = unsafe { dc.CreateBitmapFromDxgiSurface(&surface, Some(&props)) }?;
    // 黒の単色の筆（今の側は font.color の 0,0,0 を ÷255・a=1）。
    let black = D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let brush = unsafe { dc.CreateSolidColorBrush(&black, None) }?;
    // 借りる: windows-text の IDWriteTextLayout2 を bind-next の IDWriteTextLayout として（QI なし）。
    let raw: &IDWriteTextLayout = layout.as_interface();
    let origin = Vector2::new((width - FONT_HEIGHT) as f32, 0.0);
    unsafe {
        dc.SetTarget(&target);
        dc.BeginDraw();
        dc.Clear(None);
        dc.DrawTextLayout(
            origin,
            raw,
            &brush,
            D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
        );
        dc.EndDraw(None, None).ok()?;
        dc.SetTarget(None::<&ID2D1Image>);
    }

    // 読み戻し（今の側の `read_back`: staging へ CopyResource → Map(READ) → 密の BGRA）。
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ as u32;
    let staging = unsafe { texture(&d3d, &desc, None) }?;
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    let stride = (width * 4) as usize;
    let mut bgra = Vec::with_capacity(stride * height as usize);
    unsafe {
        context.CopyResource(&staging, &tex);
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .ok()?;
        for y in 0..height as usize {
            let row = (mapped.pData as *const u8).add(y * mapped.RowPitch as usize);
            bgra.extend_from_slice(std::slice::from_raw_parts(row, stride));
        }
        context.Unmap(&staging, 0);
    }
    Ok(Shot {
        width,
        height,
        bgra,
    })
}

/// ハードウェアの D3D11（BGRA）と直の文脈（今の側の `create_device_3d`・debug 組みの DEBUG の旗は付けない）。
unsafe fn create_d3d() -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    let (mut d3d, mut context) = (None, None);
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            core::ptr::null_mut(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT as u32,
            None,
            0,
            D3D11_SDK_VERSION as u32,
            Some(&mut d3d),
            None,
            Some(&mut context),
        )
        .ok()?
    };
    let none = || Error::new(E_FAIL, "D3D11CreateDevice returned no device");
    Ok((d3d.ok_or_else(none)?, context.ok_or_else(none)?))
}

/// multi-threaded の D2D の工場 → D2D の装置 → 描く文脈（今の側: `core.rs` の工場・`viewbox_draw.rs` の文脈）。
unsafe fn create_dc(d3d: &ID3D11Device) -> Result<ID2D1DeviceContext> {
    let dxgi: IDXGIDevice = d3d.cast()?;
    let mut raw = core::ptr::null_mut();
    let factory = unsafe {
        D2D1CreateFactory(
            D2D1_FACTORY_TYPE_MULTI_THREADED,
            &ID2D1Factory1::IID,
            None,
            &mut raw,
        )
        .ok()?;
        ID2D1Factory1::from_raw(raw)
    };
    let device = unsafe { factory.CreateDevice(&dxgi) }?;
    unsafe { device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE) }
}

unsafe fn texture(
    d3d: &ID3D11Device,
    desc: &D3D11_TEXTURE2D_DESC,
    init: Option<&D3D11_SUBRESOURCE_DATA>,
) -> Result<ID3D11Texture2D> {
    let mut tex = None;
    unsafe {
        d3d.CreateTexture2D(desc, init.map(|i| i as *const _), Some(&mut tex))
            .ok()?
    };
    tex.ok_or_else(|| Error::new(E_FAIL, "CreateTexture2D returned no texture"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 実 GPU で 2 通りとも描き、面が今の側と同じ大きさで、墨が面の右端の 1 列に収まり、
    /// 先頭の字が上端から始まり、絵文字が色つきで出ることを確かめる（机は要らない・窓は出さない）。
    #[test]
    fn new_side_draws_the_vertical_column_both_ways() {
        eprintln!("[q7] new face={:?}", face().expect("書体"));
        for features in [Features::None, Features::Areka] {
            let shot = draw(crate::old_side::TEXT, features).expect("新しい側で描く");
            assert_eq!((shot.width, shot.height), SURFACE);
            assert_eq!(shot.bgra.len(), (shot.width * shot.height * 4) as usize);
            let px = shot.bgra.as_chunks::<4>().0;
            let ink: Vec<(u32, u32)> = (0..shot.height)
                .flat_map(|y| (0..shot.width).map(move |x| (x, y)))
                .filter(|&(x, y)| px[(y * shot.width + x) as usize][3] != 0)
                .collect();
            assert!(!ink.is_empty(), "{features:?}: 墨が無い");
            let x0 = ink.iter().map(|p| p.0).min().unwrap();
            let x1 = ink.iter().map(|p| p.0).max().unwrap();
            let (y0, y1) = (ink[0].1, ink[ink.len() - 1].1);
            let tinted = px
                .iter()
                .filter(|p| p[3] != 0 && (p[0] != p[1] || p[1] != p[2]))
                .count();
            eprintln!(
                "[q7] new {features:?} ink={} x={x0}..{x1} y={y0}..{y1} tinted={tinted}",
                ink.len()
            );
            assert!(
                x1 - x0 < FONT_HEIGHT * 3 / 2,
                "{features:?}: 墨が 1 列に収まらない: x={x0}..{x1}"
            );
            assert!(
                x0 >= SURFACE.0 - FONT_HEIGHT * 3 / 2,
                "{features:?}: 列が右端に無い"
            );
            assert!(
                y0 < FONT_HEIGHT,
                "{features:?}: 先頭の字が上端から始まらない"
            );
            assert!(tinted > 0, "{features:?}: 絵文字が色つきで出ていない");
        }
    }
}
