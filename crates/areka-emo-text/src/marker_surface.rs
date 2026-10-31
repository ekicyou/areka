//! # marker_surface — 印の面（スクロールの矢印）の装着・描画・当たりのマスク・読み戻し・片付け
//!
//! `actor.rs` の子（`#[path]`・balloon-scroll-arrows の MarkerSurface）。場所ごとに 1 枚、窓の直接の子
//! として文字の面の**直前**（wintf の `Children` は先頭ほど手前＝文字より手前・バルーンの絵より手前）に挿す。
//! 供給面は文字の面（[`TextSurface::attach_window_child`]）と同じ作りを共有し、矢印の絵を拡大率どおりの
//! 位置と大きさへ、バルーンの絵と同じ `D2D1_INTERPOLATION_MODE_LINEAR` で描く（要件 3.4）。
//!
//! 当たりは出ている矢印の絵の α ≥ 0.5 の画素だけ（[`marker_hit_mask`]）。マスクは面と同じ物理寸で、
//! 矢印が無くても面の大きさの空のマスクを入れる（空のままだと wintf は矩形全体の判定へ縮退する）。
//!
//! 押下の届き方（6.2・6.3 の受け口が読む事実）: wintf の当たり判定は窓の `Children` を**末尾から**
//! 走査する（`DepthFirstReversePostOrder`）。印の面は文字の面と絵より前に並ぶので、矢印の下の絵が
//! α ≥ 0.5 ならその絵の entity が、透けていれば印の面の entity が押下の送り手になる。どちらでも
//! 泡立ちの道は「送り手 → 窓」で、押下は窓（普通のバルーンは**バルーンの窓**、箱は**シェル（キャラクター）
//! の窓**）の `OnPointerPressed` が受ける。矢印かどうかは送り手の entity でなく、点の座標で決める
//! （`marker_at_point`）。

use std::mem::ManuallyDrop;
use std::sync::Arc;

use areka_emo_present::balloon_marker::MarkerImage;
use bevy_ecs::entity::Entity;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows::UI::Composition::Compositor;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_NONE, D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE,
    D2D1_INTERPOLATION_MODE_LINEAR, ID2D1Bitmap1, ID2D1ColorContext, ID2D1DeviceContext,
    ID2D1Image,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use wintf::com::d2d::{D2D1DeviceContextExt, D2D1DeviceExt};
use wintf::ecs::widget::bitmap_source::AlphaMask;
use wintf::ecs::{AlphaMaskResource, GraphicsCore};

use crate::TextLayerError;
use crate::draw::create_d2d_target_bitmap;
use crate::region::{ImagePx, ScaleContract};
use crate::scroll_marker::MarkerKind;
use crate::surface::TextSurface;

/// 面に描く 1 つの矢印: 種類・左上（面の左上を原点とする image px）・絵（原寸・乗算済み BGRA）。
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedMarker {
    pub kind: MarkerKind,
    pub at: (i32, i32),
    pub image: Arc<MarkerImage>,
}

/// 場所ごとに 1 枚の印の面（窓の子の [`TextSurface`] と、矢印を描く専用の D2D DC）。
pub struct MarkerSurface {
    surface: TextSurface,
    dc: ID2D1DeviceContext,
    /// 窓のクライアント原点から面の左上までの物理 px（当たりの写しの原点）。
    offset: (f32, f32),
    /// 最後に描けた矢印（当たりの写しの絵と置き場・次のフレームの「前と同じか」の比べ）。
    drawn: Vec<PlacedMarker>,
    /// 最後に描けた拡大率。
    scale: f32,
    /// 最後に描けた当たりのマスク（wintf へ渡したのと同じもの・面の物理寸）。
    mask: Option<AlphaMask>,
}

impl MarkerSurface {
    /// 窓の `Children` の `child_index` 番目（文字の面の位置＝文字の面を 1 つ後ろへ送る）へ、物理寸
    /// `physical_size`・窓のクライアント原点からの物理 px `physical_offset` の印の面を挿す。
    /// 面は透明で始まり、当たりは面の大きさの空のマスク（何も受けない）。
    pub fn attach(
        world: &mut World,
        window: Entity,
        child_index: usize,
        compositor: &Compositor,
        core: &GraphicsCore,
        physical_size: (u32, u32),
        physical_offset: (f32, f32),
    ) -> Result<MarkerSurface, TextLayerError> {
        let Some(d2d) = core.d2d_device() else {
            tracing::error!("印の面: GraphicsCore::d2d_device が無い（矢印を描けない）");
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "GraphicsCore::d2d_device",
            });
        };
        let dc = d2d
            .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .map_err(device_err("CreateDeviceContext(MarkerSurface)"))?;
        let surface = TextSurface::attach_window_child(
            world,
            window,
            child_index,
            compositor,
            core,
            physical_size,
            physical_offset,
        )?;
        if let Some(entity) = surface.window_child() {
            world
                .entity_mut(entity)
                .insert(Name::new("emo-text-marker-surface"));
        }
        Ok(MarkerSurface {
            surface,
            dc,
            offset: physical_offset,
            drawn: Vec::new(),
            scale: 0.0,
            mask: None,
        })
    }

    /// 全面を透明にして `shown` の矢印を `(at × k, 大きさ × k)` へ描き、提示し、当たりのマスクを
    /// 出ている矢印の α ≥ 0.5 の画素へ差し替える。失敗は `error!` を残して `Err`（面と当たりは前のまま）。
    pub fn draw(
        &mut self,
        world: &mut World,
        shown: &[PlacedMarker],
        scale: &ScaleContract,
    ) -> Result<(), TextLayerError> {
        let k = scale.scale;
        // 可謬な準備（絵の GPU への転送）は BeginDraw の前に済ませる。
        let bitmaps = shown
            .iter()
            .map(|m| marker_bitmap(&self.dc, &m.image).map(|b| (m, b)))
            .collect::<Result<Vec<_>, _>>()?;
        let target = create_d2d_target_bitmap(&self.dc, self.surface.back_tex())?;
        unsafe { self.dc.SetTarget(&target) };
        unsafe { self.dc.BeginDraw() };
        self.dc.clear(None);
        for (m, bitmap) in &bitmaps {
            let px = |v: i32| scale.to_physical(ImagePx(v as f32)).0;
            let dest = D2D_RECT_F {
                left: px(m.at.0),
                top: px(m.at.1),
                right: px(m.at.0 + m.image.width as i32),
                bottom: px(m.at.1 + m.image.height as i32),
            };
            self.dc.draw_bitmap(
                bitmap,
                Some(&dest),
                1.0,
                D2D1_INTERPOLATION_MODE_LINEAR,
                None,
                None,
            );
        }
        let end = unsafe { self.dc.EndDraw(None, None) };
        unsafe { self.dc.SetTarget(None::<&ID2D1Image>) };
        end.map_err(device_err("EndDraw(MarkerSurface)"))?;
        self.surface.flip();
        self.surface.present()?;

        let mask = marker_hit_mask(self.surface.size(), k, shown);
        let entity = self.surface.window_child();
        match entity.and_then(|e| world.get_mut::<AlphaMaskResource>(e)) {
            Some(mut res) => res.set(mask.clone()),
            None => tracing::warn!(
                ?entity,
                "印の面の entity に当たりのマスクが無い——矢印でクリックを受けられない"
            ),
        }
        self.drawn = shown.to_vec();
        self.scale = k;
        self.mask = Some(mask);
        Ok(())
    }

    /// 最後に描けた矢印と拡大率が `shown`・`k` と同じか（同じなら描き直さない・要件 3.8）。
    pub fn is_drawn(&self, shown: &[PlacedMarker], k: f32) -> bool {
        self.mask.is_some() && self.scale == k && self.drawn == shown
    }

    /// 点（窓のクライアント物理 px）が最後に描けた矢印のどれかの矩形の中で、その画素が当たりの
    /// マスク（矢印の絵の α ≥ 0.5）に入っていれば、その矢印の種類（要件 5.7）。
    pub fn hit(&self, client_px: (f32, f32)) -> Option<MarkerKind> {
        let mask = self.mask.as_ref()?;
        let (x, y) = (client_px.0 - self.offset.0, client_px.1 - self.offset.1);
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let (px, py) = (x.floor() as u32, y.floor() as u32);
        if !mask.is_hit(px, py) {
            return None;
        }
        let k = self.scale;
        // 当たった画素を描いた矢印（マスクと同じ矩形の取り方）。重なりは先の矢印を採る。
        self.drawn
            .iter()
            .find(|m| {
                let inside = |p: u32, at: i32, len: u32| {
                    (at as f32 * k).floor() <= p as f32
                        && (p as f32) < ((at as i64 + len as i64) as f32 * k).ceil()
                };
                inside(px, m.at.0, m.image.width) && inside(py, m.at.1, m.image.height)
            })
            .map(|m| m.kind)
    }

    /// 文字の面と同じ型の供給面（`dump_balloon` の読み戻し）。
    pub fn surface(&self) -> &TextSurface {
        &self.surface
    }

    /// 最後に描いた面の読み戻し（乗算済み BGRA・物理寸・`stride = width × 4`）。
    pub fn read_back(&self) -> Result<Vec<u8>, TextLayerError> {
        self.surface.read_back()
    }

    /// 面の物理寸。
    pub fn size(&self) -> (u32, u32) {
        self.surface.size()
    }

    /// 窓の子の entity。
    pub fn window_child(&self) -> Option<Entity> {
        self.surface.window_child()
    }

    /// 片付け: 窓の子の entity を消す（窓の `Children` からも外れる）。
    pub fn despawn(self, world: &mut World) {
        self.surface.despawn_window_child(world);
    }
}

/// 出ている矢印の α ≥ 0.5 の画素だけを、面の大きさ `size`（物理寸）の当たりのマスクへ焼く。
/// 物理 px の中心を `1 / k` で絵の画素へ戻して読む（面の外・絵の外は切る）。
pub(crate) fn marker_hit_mask(size: (u32, u32), k: f32, shown: &[PlacedMarker]) -> AlphaMask {
    let (w, h) = size;
    let mut px = vec![0u8; w as usize * h as usize * 4];
    for m in shown {
        let (iw, ih) = (m.image.width as i64, m.image.height as i64);
        let span = |at: i32, len: i64, limit: u32| {
            let lo = (at as f32 * k).floor().clamp(0.0, limit as f32) as u32;
            let hi = ((at as i64 + len) as f32 * k)
                .ceil()
                .clamp(0.0, limit as f32) as u32;
            lo..hi
        };
        for y in span(m.at.1, ih, h) {
            let iy = ((y as f32 + 0.5) / k).floor() as i64 - m.at.1 as i64;
            if !(0..ih).contains(&iy) {
                continue;
            }
            for x in span(m.at.0, iw, w) {
                let ix = ((x as f32 + 0.5) / k).floor() as i64 - m.at.0 as i64;
                if !(0..iw).contains(&ix) {
                    continue;
                }
                let alpha = m.image.bgra[((iy * iw + ix) * 4 + 3) as usize];
                // 矢印が重なっても、後の絵の透けた画素で前の絵の当たりを消さない。
                let cell = &mut px[(y as usize * w as usize + x as usize) * 4 + 3];
                *cell = (*cell).max(alpha);
            }
        }
    }
    // α ≥ 128（≈ 0.5）で当たる 2 値化は AlphaMask の固定の閾値が行う。
    AlphaMask::from_pbgra32(&px, w, h, w * 4)
}

/// 印の絵（矢印・行の中の印）を D2D のビットマップへ上げる（DC の DPI を与える・emo-present `display.rs` と同じ）。
/// 失敗は `error!` を残して `Err`。
pub(crate) fn marker_bitmap(
    dc: &ID2D1DeviceContext,
    image: &MarkerImage,
) -> Result<ID2D1Bitmap1, TextLayerError> {
    let (w, h) = (image.width, image.height);
    if w == 0 || h == 0 || image.bgra.len() < w as usize * h as usize * 4 {
        tracing::error!(
            w,
            h,
            len = image.bgra.len(),
            "印の絵の寸とバイト列が合わない（描けない）"
        );
        return Err(TextLayerError::Device {
            hresult: 0,
            context: "marker image size mismatch",
        });
    }
    let (mut dpi_x, mut dpi_y) = (0.0f32, 0.0f32);
    unsafe { dc.GetDpi(&mut dpi_x, &mut dpi_y) };
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: dpi_x,
        dpiY: dpi_y,
        bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
        colorContext: ManuallyDrop::new(None::<ID2D1ColorContext>),
    };
    // SAFETY: バイト列は直前で `w × h × 4` 以上と確かめた（D2D は内容を複製する）。
    unsafe {
        dc.CreateBitmap(
            D2D_SIZE_U {
                width: w,
                height: h,
            },
            Some(image.bgra.as_ptr() as *const _),
            w * 4,
            &props,
        )
    }
    .map_err(device_err("CreateBitmap(marker)"))
}

/// `windows_core::Error` を [`TextLayerError::Device`] へ写す（log-first・surface.rs と同型）。
fn device_err(context: &'static str) -> impl FnOnce(windows::core::Error) -> TextLayerError {
    move |e| {
        let hresult = e.code().0;
        tracing::error!(hresult, context, "印の面の D2D 呼び出しが失敗");
        TextLayerError::Device { hresult, context }
    }
}

#[cfg(test)]
#[path = "marker_surface_tests.rs"]
mod tests;
