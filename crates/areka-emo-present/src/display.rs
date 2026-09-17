//! 表示の記録（display.rs）: 原寸 bitmap × 論理 px 宛先矩形 × LINEAR。
//!
//! 合成結果 [`ComposedSurface`]（**native 原寸**・premultiplied BGRA）を 1 度だけ GPU へ上げ、
//! 論理 px の宛先矩形 `(0, 0, w, h)` で描く命令を、閉じた [`GraphicsCommandList`] として返す。
//! 拡大縮小は wintf の描画経路（`render_surface` の `SetTransform`）が担うため、本モジュールは
//! **拡大率 k を引数に持たず**、記録内容も k に依らない（要件 1.1／1.2／1.3）。
//!
//! 手順は wintf の `BitmapSource`（`ecs/widget/bitmap_source/systems.rs` の `draw_bitmap_sources`／
//! `create_d2d_bitmap`）の逐語 lift であり、差分は「WIC 由来 bitmap → メモリからの
//! `ID2D1DeviceContext::CreateBitmap`」の 1 点だけである。補間は bilinear 相当の
//! `D2D1_INTERPOLATION_MODE_LINEAR`（要件 2.6）。
//!
//! 失敗し得る 4 点（bitmap 生成・コマンドリスト生成・描画終了・閉じる）はすべて
//! [`crate::command::device_err`] を通し、`error!` を残してから [`PresentError::Device`] を返す
//! （パニックしない・要件 7.1／7.2／7.4）。

use std::mem::ManuallyDrop;

use areka_emo_compose::ComposedSurface;

use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_NONE, D2D1_BITMAP_PROPERTIES1, D2D1_INTERPOLATION_MODE,
    D2D1_INTERPOLATION_MODE_LINEAR, ID2D1ColorContext, ID2D1DeviceContext,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows_numerics::Matrix3x2;

use wintf::com::d2d::{D2D1CommandListExt, D2D1DeviceContextExt};
use wintf::ecs::GraphicsCommandList;

use crate::command::{PresentError, device_err};

/// 記録の引数を値で持つ純関数の出力（k を含まない）。
///
/// [`record_display`] はこのレシピの値だけを見て記録する。同一の [`ComposedSurface`] からは
/// 常に同一のレシピが導かれる（決定的）。
pub(crate) struct DisplayRecipe {
    /// D2D bitmap の寸（画素）＝ native 原寸。
    pub(crate) bitmap_size: (u32, u32),
    /// 元バイト列の行ストライド（バイト）＝ [`ComposedSurface::stride`]。
    pub(crate) pitch: u32,
    /// 宛先矩形 `(left, top, right, bottom)`＝ `(0, 0, w, h)` 論理 px。
    pub(crate) dest: (f32, f32, f32, f32),
    /// 補間モード（bilinear 相当以上＝`LINEAR`）。
    pub(crate) interpolation: D2D1_INTERPOLATION_MODE,
}

impl DisplayRecipe {
    /// 原寸の合成結果から記録の引数を導く（GPU に触れない純関数）。
    pub(crate) fn for_surface(s: &ComposedSurface) -> Self {
        let (w, h) = (s.width(), s.height());
        Self {
            bitmap_size: (w, h),
            pitch: s.stride(),
            dest: (0.0, 0.0, w as f32, h as f32),
            interpolation: D2D1_INTERPOLATION_MODE_LINEAR,
        }
    }
}

/// `record_display` が失敗し得る 4 点。
///
/// テストビルドでのみ実体を持つ注入点 [`fault_point`] の引数であり、通常ビルドでは「何もしない
/// 空の処理」の識別子としてのみ現れる（実行時の分岐・確保・呼出は増えない）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayFault {
    /// `CreateBitmap`（原寸バイト列の GPU 転送）の直前。
    CreateBitmap,
    /// `CreateCommandList` の直前。
    CreateCommandList,
    /// `EndDraw`（描画終了）が失敗した場合（注入は実呼び出しの直後・下記註釈）。
    EndDraw,
    /// `Close`（コマンドリストを閉じる）が失敗した場合（注入は実呼び出しの直後・下記註釈）。
    Close,
}

// 次に踏む一致点で 1 回だけ失敗させる旗（スレッド局所・テストビルド限定）。
#[cfg(test)]
thread_local! {
    static ARMED_DISPLAY_FAULT: std::cell::Cell<Option<DisplayFault>> =
        const { std::cell::Cell::new(None) };
}

/// 注入された失敗の文脈文字列（`device_err` は `&'static str` を要するため変位ごとに定数を持つ。
/// 字面は `<injected:{at:?}>`＝変位名の Debug 表現と一致する）。
#[cfg(test)]
fn injected_context(at: DisplayFault) -> &'static str {
    match at {
        DisplayFault::CreateBitmap => "<injected:CreateBitmap>",
        DisplayFault::CreateCommandList => "<injected:CreateCommandList>",
        DisplayFault::EndDraw => "<injected:EndDraw>",
        DisplayFault::Close => "<injected:Close>",
    }
}

/// 失敗の注入点（テストビルド）。
///
/// 武装中の失敗点と一致したときだけ旗を降ろし、**既存の失敗経路と同じ形**——`device_err` を通す
/// ＝`error!` で記録を残してから [`PresentError::Device`] を返す——で失敗させる。一致しなければ
/// 旗はそのまま（後続の一致点まで武装が残る）。
#[cfg(test)]
fn fault_point(at: DisplayFault) -> Result<(), PresentError> {
    if ARMED_DISPLAY_FAULT.with(|armed| armed.get()) != Some(at) {
        return Ok(());
    }
    ARMED_DISPLAY_FAULT.with(|armed| armed.set(None));
    let e = windows::core::Error::from_hresult(windows::core::HRESULT(0x8000_4005u32 as i32));
    Err(device_err(injected_context(at))(e))
}

/// 失敗の注入点（通常ビルド）。何もしない空の処理＝常に `Ok(())`。
#[cfg(not(test))]
#[inline(always)]
fn fault_point(_at: DisplayFault) -> Result<(), PresentError> {
    Ok(())
}

/// テスト専用: 次の一致点で 1 回だけ失敗させる（同一スレッド）。
#[cfg(test)]
pub(crate) fn arm_display_fault(at: DisplayFault) {
    ARMED_DISPLAY_FAULT.with(|armed| armed.set(Some(at)));
}

/// テスト専用: 武装を解除する（未消費のまま残った旗を次のテストへ持ち越さない）。
#[cfg(test)]
pub(crate) fn clear_display_fault() {
    ARMED_DISPLAY_FAULT.with(|armed| armed.set(None));
}

/// 原寸の合成結果 → 閉じたコマンドリスト（GPU 呼び出しは本関数の内側だけ）。
///
/// `dc` は `GraphicsCore` の共有 DC（UI スレッド直列・`BitmapSource` と同条件）。`SetTarget` は
/// 戻さない（`draw_bitmap_sources` の踏襲）。生成した bitmap は閉じたリストが参照を保持するため
/// 戻り値には含めない。
///
/// 事前条件: `surface` の外形は 1×1 以上（0 外形は上流 `EmptyComposition` が先に遮断する）。
/// 事後条件: `Ok` なら返したリストは `Close` 済みで再利用できる。`Err` なら生成途中の GPU 資源も
/// 含めて drop され、呼び手から見える副作用は無い。
pub(crate) fn record_display(
    dc: &ID2D1DeviceContext,
    surface: &ComposedSurface,
) -> Result<GraphicsCommandList, PresentError> {
    let recipe = DisplayRecipe::for_surface(surface);

    // DC の実際の DPI を bitmap に与える（96.0 固定だと DC の DPI スケールとズレ、宛先矩形を
    // 指定しても内部スケール計算が狂う——`create_d2d_bitmap` の踏襲）。
    // SAFETY: Windows API 境界。`dc` は生存中の COM 参照であり、出力先は自スタック上の f32。
    let (dpi_x, dpi_y) = unsafe {
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        dc.GetDpi(&mut x, &mut y);
        (x, y)
    };

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

    let (w, h) = recipe.bitmap_size;
    fault_point(DisplayFault::CreateBitmap)?;
    // SAFETY: Windows API 境界。`bytes` は本呼び出しの間だけ読まれ（D2D は内容を複製する）、
    // 長さは `pitch * h` 以上であることを `ComposedSurface` が不変条件として保証する。
    let bitmap = unsafe {
        dc.CreateBitmap(
            D2D_SIZE_U {
                width: w,
                height: h,
            },
            Some(surface.bytes().as_ptr() as *const _),
            recipe.pitch,
            &props,
        )
    }
    .map_err(device_err("CreateBitmap(composed native)"))?;

    fault_point(DisplayFault::CreateCommandList)?;
    // SAFETY: Windows API 境界。共有 DC への COM 呼び出し（UI スレッド直列）。
    let command_list =
        unsafe { dc.CreateCommandList() }.map_err(device_err("CreateCommandList"))?;

    // SAFETY: Windows API 境界。ターゲット差し替えと描画命令の記録（UI スレッド直列）。
    unsafe { dc.SetTarget(&command_list) };

    let (left, top, right, bottom) = recipe.dest;
    let dest = D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    };
    // 共有 DC のワールド変換をリセットする（前の記録の残留変換を持ち込まない）。
    dc.set_transform(&Matrix3x2::identity());
    // SAFETY: Windows API 境界。`BeginDraw`／`EndDraw` は同一関数内で対になる。
    unsafe { dc.BeginDraw() };
    dc.draw_bitmap(&bitmap, Some(&dest), 1.0, recipe.interpolation, None, None);
    // SAFETY: Windows API 境界。直前の `BeginDraw` を閉じる。
    unsafe { dc.EndDraw(None, None) }.map_err(device_err("EndDraw(display)"))?;
    // 注入点は実呼び出しの**後**に置く（前に置くと共有 DC が `BeginDraw` 開きっぱなしで残り、
    // 後続の記録を巻き添えにする）。観測される結果——`error!`＋`Err`・リストを返さない——は
    // 呼び出しが失敗した場合と同一である。
    fault_point(DisplayFault::EndDraw)?;

    command_list
        .close()
        .map_err(device_err("CommandList::Close"))?;
    fault_point(DisplayFault::Close)?;

    Ok(GraphicsCommandList::new(command_list))
}

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "display_gpu_tests.rs"]
mod gpu_tests;

#[cfg(test)]
#[path = "display_fault_tests.rs"]
mod fault_tests;
