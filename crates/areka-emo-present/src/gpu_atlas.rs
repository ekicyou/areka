//! 装置ごとのアトラスの頁の GPU の絵と、絵ごとの葉の命令の列の控え（`GpuAtlas`）。
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "表示の段から呼ばれるまで本番の呼び手が無い")
)]

use std::mem::ManuallyDrop;

use areka_emo_atlas::{AtlasTable, ElementId};
use areka_emo_compose::IRect;

use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_NONE, D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE,
    D2D1_INTERPOLATION_MODE_LINEAR, ID2D1Bitmap1, ID2D1ColorContext, ID2D1CommandList, ID2D1Device,
    ID2D1DeviceContext,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows_numerics::Matrix3x2;

use wintf::com::d2d::{D2D1CommandListExt, D2D1DeviceContextExt, D2D1DeviceExt};

use crate::command::{PresentError, device_err};

/// 装置ごとのアトラスの頁の GPU の絵と、絵ごとの葉の命令の列。
///
/// 頁は作るときに 1 度だけ上げ、葉は初めて引かれたときに 1 度だけ記録する。装置が替わったら
/// 捨てて作り直す。
pub(crate) struct GpuAtlas {
    device: ID2D1Device,
    dc: ID2D1DeviceContext,
    atlas: AtlasTable,
    pages: Vec<ID2D1Bitmap1>,
    leaves: Vec<Option<ID2D1CommandList>>,
    #[cfg(test)]
    uploads: u32,
    #[cfg(test)]
    recordings: u32,
}

impl GpuAtlas {
    /// `device` の上に `atlas` の頁を全部上げる。失敗は `error!` を残して `Err`。
    pub(crate) fn new(device: &ID2D1Device, atlas: &AtlasTable) -> Result<Self, PresentError> {
        let dc = device
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .map_err(device_err("CreateDeviceContext(atlas)", None))?;
        // 頁の絵と葉の座標を画素のまま扱うため、DC も頁も 96 にそろえる。
        // SAFETY: Windows API 境界。作ったばかりの専有の DC の設定。
        unsafe { dc.SetDpi(96.0, 96.0) };
        let pages = atlas
            .pages()
            .iter()
            .map(|page| upload_page(&dc, page))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            device: device.clone(),
            dc,
            atlas: atlas.clone(),
            #[cfg(test)]
            uploads: pages.len() as u32,
            pages,
            leaves: vec![None; atlas.len()],
            #[cfg(test)]
            recordings: 0,
        })
    }

    /// 絵 `element` を、頁の絵の区画から縁の位置・原寸へ描く閉じた命令の列。
    pub(crate) fn leaf(&mut self, element: ElementId) -> Result<&ID2D1CommandList, PresentError> {
        let slot = element.0 as usize;
        if self.leaves[slot].is_none() {
            let placement = self.atlas.entry(element).placement.clone();
            let list = self.record(|dc| {
                let Some(p) = placement else {
                    return Ok(());
                };
                let uv = p.uv_rect;
                let src = D2D_RECT_F {
                    left: uv.x as f32,
                    top: uv.y as f32,
                    right: (uv.x + uv.w) as f32,
                    bottom: (uv.y + uv.h) as f32,
                };
                let (x, y) = (p.trim_offset.x as f32, p.trim_offset.y as f32);
                let dest = D2D_RECT_F {
                    left: x,
                    top: y,
                    right: x + uv.w as f32,
                    bottom: y + uv.h as f32,
                };
                dc.draw_bitmap(
                    &self.pages[p.page as usize],
                    Some(&dest),
                    1.0,
                    D2D1_INTERPOLATION_MODE_LINEAR,
                    Some(&src),
                    None,
                );
                Ok(())
            })?;
            #[cfg(test)]
            {
                self.recordings += 1;
            }
            self.leaves[slot] = Some(list);
        }
        Ok(self.leaves[slot].as_ref().expect("直前に控えた"))
    }

    /// `rect` を黒の不透明で塗る閉じた命令の列。
    pub(crate) fn black_fill(&mut self, rect: IRect) -> Result<ID2D1CommandList, PresentError> {
        self.record(|dc| {
            let brush = dc
                .create_solid_color_brush(
                    &D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    },
                    None,
                )
                .map_err(device_err("CreateSolidColorBrush(black fill)", None))?;
            let (x, y) = (rect.x as f32, rect.y as f32);
            dc.fill_rectangle(
                &D2D_RECT_F {
                    left: x,
                    top: y,
                    right: x + rect.w as f32,
                    bottom: y + rect.h as f32,
                },
                &brush,
            );
            Ok(())
        })
    }

    /// 頁と葉を作った装置。
    pub(crate) fn device(&self) -> &ID2D1Device {
        &self.device
    }

    #[cfg(test)]
    pub(crate) fn uploads(&self) -> u32 {
        self.uploads
    }

    #[cfg(test)]
    pub(crate) fn recordings(&self) -> u32 {
        self.recordings
    }

    fn record(
        &self,
        draw: impl FnOnce(&ID2D1DeviceContext) -> Result<(), PresentError>,
    ) -> Result<ID2D1CommandList, PresentError> {
        let dc = &self.dc;
        // SAFETY: Windows API 境界。専有の DC への COM 呼び出し。
        let list = unsafe { dc.CreateCommandList() }
            .map_err(device_err("CreateCommandList(atlas)", None))?;
        // SAFETY: Windows API 境界。的の差し替えと記録。`BeginDraw`／`EndDraw` は本関数の中で対になる。
        unsafe {
            dc.SetTarget(&list);
            dc.BeginDraw();
        }
        dc.set_transform(&Matrix3x2::identity());
        let drawn = draw(dc);
        // SAFETY: Windows API 境界。直前の `BeginDraw` を閉じ、的を外す。
        let ended = unsafe {
            let ended = dc.EndDraw(None, None);
            dc.SetTarget(None);
            ended
        };
        drawn?;
        ended.map_err(device_err("EndDraw(atlas)", None))?;
        list.close()
            .map_err(device_err("CommandList::Close(atlas)", None))?;
        Ok(list)
    }
}

fn upload_page(
    dc: &ID2D1DeviceContext,
    page: &areka_emo_atlas::AtlasPage,
) -> Result<ID2D1Bitmap1, PresentError> {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
        colorContext: ManuallyDrop::new(None::<ID2D1ColorContext>),
    };
    let short = (page.stride as usize) < page.width as usize * 4
        || page.bytes.len() < page.stride as usize * page.height as usize;
    if short {
        let e = windows::core::Error::from(windows::Win32::Foundation::E_INVALIDARG);
        return Err(device_err("CreateBitmap(atlas page: short bytes)", None)(e));
    }
    // SAFETY: Windows API 境界。`bytes` は本呼び出しの間だけ読まれ（D2D が中身を写す）、長さが
    // `stride * height` 以上で `stride` が 1 行に足りることを直前で確かめた。
    unsafe {
        dc.CreateBitmap(
            D2D_SIZE_U {
                width: page.width,
                height: page.height,
            },
            Some(page.bytes.as_ptr() as *const _),
            page.stride,
            &props,
        )
    }
    .map_err(device_err("CreateBitmap(atlas page)", None))
}

#[cfg(test)]
#[path = "gpu_atlas_tests.rs"]
mod tests;
