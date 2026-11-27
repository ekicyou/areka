//! 装置ごとのアトラスの頁の GPU の絵と、絵ごとの葉の命令の列の控え（`GpuAtlas`）。

use std::mem::ManuallyDrop;

use areka_emo_atlas::{AtlasTable, ElementId};
use areka_emo_compose::{DrawPlan, IRect, PlanItem};

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
use crate::display::{DisplayFault, fault_point};

/// 前に記録した同じ対象の装置の失敗の原因（`device_err` へそのまま渡す）。
type Repeat = Option<(&'static str, i32)>;

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
    fills: Vec<((u32, u32), ID2D1CommandList)>,
    #[cfg(test)]
    uploads: u32,
    #[cfg(test)]
    recordings: u32,
}

impl GpuAtlas {
    /// `device` の上に `atlas` の頁を全部上げる。失敗は `error!` を残して `Err`。
    pub(crate) fn new(device: &ID2D1Device, atlas: &AtlasTable) -> Result<Self, PresentError> {
        Self::new_with(device, atlas, None)
    }

    /// [`Self::new`] の、同じ原因の続きなら失敗の行を `debug!` にする形。
    pub(crate) fn new_with(
        device: &ID2D1Device,
        atlas: &AtlasTable,
        repeat: Repeat,
    ) -> Result<Self, PresentError> {
        let dc = device
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .map_err(device_err("CreateDeviceContext(atlas)", repeat))?;
        // 頁の絵と葉の座標を画素のまま扱うため、DC も頁も 96 にそろえる。
        // SAFETY: Windows API 境界。作ったばかりの専有の DC の設定。
        unsafe { dc.SetDpi(96.0, 96.0) };
        let pages = atlas
            .pages()
            .iter()
            .map(|page| upload_page(&dc, page, repeat))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            device: device.clone(),
            dc,
            atlas: atlas.clone(),
            #[cfg(test)]
            uploads: pages.len() as u32,
            pages,
            leaves: vec![None; atlas.len()],
            fills: Vec::new(),
            #[cfg(test)]
            recordings: 0,
        })
    }

    /// 絵 `element` を、頁の絵の区画から縁の位置・原寸へ描く閉じた命令の列。
    pub(crate) fn leaf(&mut self, element: ElementId) -> Result<&ID2D1CommandList, PresentError> {
        self.leaf_with(element, None)
    }

    /// 計画の絵と黒の塗りの葉を前もって全部記録する（節を触る前に、失敗し得る記録を済ませる）。
    pub(crate) fn prepare(&mut self, plan: &DrawPlan, repeat: Repeat) -> Result<(), PresentError> {
        for item in &plan.items {
            match *item {
                PlanItem::Image { element, .. } => {
                    self.leaf_with(element, repeat)?;
                }
                PlanItem::Fill { rect } => {
                    self.fill_with((rect.w, rect.h), repeat)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// 原点から `size` の矩形を黒の不透明で塗る葉（大きさごとに 1 度だけ記録する）。
    pub(crate) fn fill(&mut self, size: (u32, u32)) -> Result<&ID2D1CommandList, PresentError> {
        self.fill_with(size, None)
    }

    fn fill_with(
        &mut self,
        size: (u32, u32),
        repeat: Repeat,
    ) -> Result<&ID2D1CommandList, PresentError> {
        let i = match self.fills.iter().position(|(s, _)| *s == size) {
            Some(i) => i,
            None => {
                let rect = IRect {
                    x: 0,
                    y: 0,
                    w: size.0,
                    h: size.1,
                };
                let list = self.black_fill_with(rect, repeat)?;
                self.fills.push((size, list));
                self.fills.len() - 1
            }
        };
        Ok(&self.fills[i].1)
    }

    fn leaf_with(
        &mut self,
        element: ElementId,
        repeat: Repeat,
    ) -> Result<&ID2D1CommandList, PresentError> {
        let slot = element.0 as usize;
        if self.leaves[slot].is_none() {
            let placement = self
                .atlas
                .entry(element)
                .placement
                .clone()
                .filter(|p| self.page_holds(element, p));
            let list = self.record(repeat, |dc| {
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
    #[cfg(test)]
    pub(crate) fn black_fill(&mut self, rect: IRect) -> Result<ID2D1CommandList, PresentError> {
        self.black_fill_with(rect, None)
    }

    fn black_fill_with(
        &mut self,
        rect: IRect,
        repeat: Repeat,
    ) -> Result<ID2D1CommandList, PresentError> {
        self.record(repeat, |dc| {
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
                .map_err(device_err("CreateSolidColorBrush(black fill)", repeat))?;
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

    /// 絵 `element` の原寸（葉が描く局所の範囲）。
    pub(crate) fn original(&self, element: ElementId) -> areka_emo_atlas::Size {
        self.atlas.entry(element).original
    }

    /// `atlas` がこの控えを作った表（またはその写し）か。
    pub(crate) fn is_for(&self, atlas: &AtlasTable) -> bool {
        std::ptr::eq(self.atlas.pages(), atlas.pages())
    }

    /// 頁と葉を作った装置。
    pub(crate) fn device(&self) -> &ID2D1Device {
        &self.device
    }

    /// 置き場 `p` の頁が在り、uv がその頁に収まるか。収まらなければ `warn!` を残す（その絵は描かない）。
    /// 表（`AtlasTable::new`）は uv と頁を照らさないので、葉を記録する手前で信じずに確かめる。
    fn page_holds(&self, element: ElementId, p: &areka_emo_atlas::Placement) -> bool {
        let Some(page) = self.atlas.page(p.page) else {
            tracing::warn!(
                element = element.0,
                page = p.page,
                "アトラスの頁が存在しない: 絵を描かない"
            );
            return false;
        };
        let uv = &p.uv_rect;
        let fits = uv.x as u64 + uv.w as u64 <= page.width as u64
            && uv.y as u64 + uv.h as u64 <= page.height as u64;
        if !fits {
            tracing::warn!(
                element = element.0,
                page = p.page,
                uv = ?uv,
                "uv が頁に収まらない: 絵を描かない"
            );
        }
        fits
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
        repeat: Repeat,
        draw: impl FnOnce(&ID2D1DeviceContext) -> Result<(), PresentError>,
    ) -> Result<ID2D1CommandList, PresentError> {
        let dc = &self.dc;
        fault_point(DisplayFault::CreateCommandList, repeat)?;
        // SAFETY: Windows API 境界。専有の DC への COM 呼び出し。
        let list = unsafe { dc.CreateCommandList() }
            .map_err(device_err("CreateCommandList(atlas)", repeat))?;
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
        ended.map_err(device_err("EndDraw(atlas)", repeat))?;
        // 注入点は実呼び出しの後（前に置くと DC が `BeginDraw` のまま残る）。
        fault_point(DisplayFault::EndDraw, repeat)?;
        list.close()
            .map_err(device_err("CommandList::Close(atlas)", repeat))?;
        fault_point(DisplayFault::Close, repeat)?;
        Ok(list)
    }
}

fn upload_page(
    dc: &ID2D1DeviceContext,
    page: &areka_emo_atlas::AtlasPage,
    repeat: Repeat,
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
        return Err(device_err("CreateBitmap(atlas page: short bytes)", repeat)(
            e,
        ));
    }
    fault_point(DisplayFault::CreateBitmap, repeat)?;
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
    .map_err(device_err("CreateBitmap(atlas page)", repeat))
}

#[cfg(test)]
#[path = "gpu_atlas_tests.rs"]
pub(crate) mod tests;
