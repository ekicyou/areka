//! 描画面ごとの入れ物（要件 4.1, 4.2, 4.4, 4.8）。描画面 1 枚の合成の結果を持ち、合成と写しの
//! 手順を持つ（WUC と ECS を知らない）。本番の係（`render_surface`）と WARP のテストが同じ手順を呼ぶ。

use super::visual::{DamageEntry, DrawCommand, emit};
use std::mem::ManuallyDrop;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::core::Interface;
use windows_numerics::{Matrix3x2, Vector2};

/// 1 画素の読み戻しの写し先と控え（同じ錠の中に置く）。
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
    pixel: Mutex<PixelReadState>,
    /// GPU から読んだ回数（5.4 のテストの的）。
    gpu_reads: AtomicU64,
    /// 前の合成の項の列（無い＝比べられない）。入れ物と同じ寿命（捨てれば控えも消える）。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    prev_entries: Option<Vec<DamageEntry>>,
    /// 前の合成の根の命令の列（参照を持つので番地の使い回しで取り違えない）。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    prev_root: Option<ID2D1CommandList>,
    /// 前の合成の拡大 (x, y)。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    prev_scale: (f32, f32),
    /// 前の合成の入れ物を描画面へ写し終えた。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    presented: bool,
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

/// 置き場の入れ物への描き 1 回（`draw`）を走らせる（置き場が空なら何もしない）。失敗したら入れ物を
/// 控えごと捨てる（途中まで描いた絵を読ませない＝読みは `Unreadable`・5.5。次の回に作り直す・3.5）。
pub(crate) fn compose_or_drop(
    slot: &mut Option<SurfaceComposite>,
    draw: impl FnOnce(&mut SurfaceComposite) -> windows::core::Result<()>,
) -> windows::core::Result<()> {
    let Some(composite) = slot.as_mut() else {
        return Ok(());
    };
    let result = draw(composite);
    if result.is_err() {
        *slot = None;
    }
    result
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
            prev_entries: None,
            prev_root: None,
            prev_scale: (0.0, 0.0),
            presented: false,
        })
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        self.size
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// 前の合成の項の列（新しい入れ物は無い）。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    pub(crate) fn prev_entries(&self) -> Option<&[DamageEntry]> {
        self.prev_entries.as_deref()
    }

    /// 根の命令の列が前の合成と同じ参照か（COM の参照の同一性・両方無いも同じ）。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    pub(crate) fn same_root(&self, root_list: Option<&ID2D1CommandList>) -> bool {
        self.prev_root.as_ref() == root_list
    }

    /// 拡大が前の合成とビットで同じか。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    pub(crate) fn same_scale(&self, (x, y): (f32, f32)) -> bool {
        let (px, py) = self.prev_scale;
        (px.to_bits(), py.to_bits()) == (x.to_bits(), y.to_bits())
    }

    /// 前の合成の入れ物を描画面へ写し終えたか（新しい入れ物は偽）。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    pub(crate) fn presented(&self) -> bool {
        self.presented
    }

    /// 合成か Skip・PresentOnly を終えた回に控えを置き換える。`composed` なら `presented` を偽にする
    /// （1.1・2.8）。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    pub(crate) fn remember(
        &mut self,
        entries: Option<Vec<DamageEntry>>,
        root_list: Option<&ID2D1CommandList>,
        scale: (f32, f32),
        composed: bool,
    ) {
        self.prev_entries = entries;
        self.prev_root = root_list.cloned();
        self.prev_scale = scale;
        if composed {
            self.presented = false;
        }
    }

    /// 入れ物を描画面へ写し終えた印を付ける（2.8）。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "4.3 の compose_planned が使い始める")
    )]
    pub(crate) fn mark_presented(&mut self) {
        self.presented = true;
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

    /// 1 画素（座標は呼び手が範囲の内と確かめ済み・世代は 1 以上）。`(x, y, 世代)` が控えと同じなら
    /// GPU から読まない（5.4）。`Map(READ)` は GPU が合成を終えるまで待つ（5.2）。
    pub(crate) fn read_pixel(&self, x: u32, y: u32) -> windows::core::Result<[u8; 4]> {
        // 錠を持ったまま写し・Map・Unmap・控えの更新を行う（2 つの読みが写し先を取り合わない）。
        let mut state = self.pixel.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((lx, ly, lg, value)) = state.last
            && (lx, ly, lg) == (x, y, self.generation)
        {
            return Ok(value);
        }
        let src = D2D_RECT_U {
            left: x,
            top: y,
            right: x + 1,
            bottom: y + 1,
        };
        // SAFETY: Windows API 境界。`Map(READ)` の `bits` は少なくとも 4 バイトを指す。
        let value = unsafe {
            state
                .staging
                .CopyFromBitmap(None, &self.bitmap, Some(&src))?;
            let mapped = state.staging.Map(D2D1_MAP_OPTIONS_READ)?;
            let value = *(mapped.bits as *const [u8; 4]);
            state.staging.Unmap()?;
            value
        };
        self.gpu_reads.fetch_add(1, Ordering::Relaxed);
        state.last = Some((x, y, self.generation, value));
        Ok(value)
    }

    /// 全体（1 行＝幅×4 バイトに詰めた BGRA・5.9）。写し先は `device` の新しい DC で作って捨てる
    /// （共有の DC と 1 画素の写し先に触らないので錠を要らない）。
    pub(crate) fn read_all(&self, device: &ID2D1Device) -> windows::core::Result<Vec<u8>> {
        let (width, height) = self.size;
        let row = width as usize * 4;
        let mut bytes = vec![0u8; row * height as usize];
        // SAFETY: Windows API 境界。`Map(READ)` の `bits` は少なくとも `pitch * height` バイトを指す。
        unsafe {
            let dc = device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
            let staging = create_bitmap(
                &dc,
                self.size,
                D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
            )?;
            staging.CopyFromBitmap(None, &self.bitmap, None)?;
            let mapped = staging.Map(D2D1_MAP_OPTIONS_READ)?;
            for y in 0..height as usize {
                std::ptr::copy_nonoverlapping(
                    mapped.bits.add(y * mapped.pitch as usize),
                    bytes.as_mut_ptr().add(y * row),
                    row,
                );
            }
            staging.Unmap()?;
        }
        Ok(bytes)
    }

    /// GPU から読んだ回数（5.4 のテストの的）。
    #[cfg(test)]
    pub(crate) fn gpu_reads(&self) -> u64 {
        self.gpu_reads.load(Ordering::Relaxed)
    }
}
