//! WS_EX_TRANSPARENT クリックスルー最小検証テスト
//!
//! 3つのウィンドウを作って重ねて表示する:
//! - Window A (最背面): 通常ウィンドウ、赤背景、クリックでコンソール出力
//! - Window B (中間):   WS_EX_NOREDIRECTIONBITMAP + WS_EX_TRANSPARENT、DirectComposition描画
//! - Window C (最前面): WS_EX_TRANSPARENT + WS_EX_LAYERED のみ（NOREDIRECTIONBITMAP なし）、緑半透明
//!
//! テスト方法:
//! 1. Window C (緑半透明) 越しに Window A (赤) をクリック → C を貫通するか？
//! 2. Window B (DirectComp青帯) 越しに Window A (赤) をクリック → B を貫通するか？
//! 3. 赤だけの部分をクリック → 直接 A に届くはず
//!
//! これにより WS_EX_TRANSPARENT 単体 vs WS_EX_NOREDIRECTIONBITMAP 併用の
//! クリックスルー動作を切り分ける。

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;
use wintf::com::d2d::*;
use wintf::com::d3d11::*;
use wintf::com::dcomp::*;

const CLASS_NORMAL: PCWSTR = w!("ClickThroughTest_Normal");
const CLASS_DCOMP: PCWSTR = w!("ClickThroughTest_DComp");
const CLASS_LAYERED: PCWSTR = w!("ClickThroughTest_Layered");

fn main() -> Result<()> {
    unsafe {
        let hinstance = GetModuleHandleW(None)?;

        // ウィンドウクラス登録
        register_class(hinstance, CLASS_NORMAL, Some(wndproc_normal))?;
        register_class(hinstance, CLASS_DCOMP, Some(wndproc_dcomp))?;
        register_class(hinstance, CLASS_LAYERED, Some(wndproc_layered))?;

        // === Window A: 通常ウィンドウ（赤背景、大きめ、最背面） ===
        let hwnd_a = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS_NORMAL,
            w!("[A] Normal - click target (赤)"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            50, 50, 800, 500,
            None, None, Some(hinstance.into()), None,
        )?;
        println!("[A] Normal window: {:?}", hwnd_a);

        // === Window B: WS_EX_NOREDIRECTIONBITMAP + WS_EX_TRANSPARENT ===
        // A の左半分に重なるように配置
        let ex_style_b = WS_EX_NOREDIRECTIONBITMAP | WS_EX_TRANSPARENT | WS_EX_TOPMOST;
        let hwnd_b = CreateWindowExW(
            ex_style_b,
            CLASS_DCOMP,
            w!("[B] DComp+Transparent"),
            WS_POPUP | WS_VISIBLE,
            100, 150, 300, 300,
            None, None, Some(hinstance.into()), None,
        )?;
        println!(
            "[B] DComp+Transparent window: {:?}, ex_style=0x{:X}",
            hwnd_b,
            GetWindowLongPtrW(hwnd_b, GWL_EXSTYLE)
        );

        // DirectComposition で Window B に青い矩形を描画
        setup_dcomp(hwnd_b)?;

        // === Window C: WS_EX_TRANSPARENT + WS_EX_LAYERED (NOREDIRECTIONBITMAP なし) ===
        // A の右半分に重なるように配置
        let ex_style_c = WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_LAYERED;
        let hwnd_c = CreateWindowExW(
            ex_style_c,
            CLASS_LAYERED,
            w!("[C] Transparent+Layered"),
            WS_POPUP | WS_VISIBLE,
            450, 150, 300, 300,
            None, None, Some(hinstance.into()), None,
        )?;
        // SetLayeredWindowAttributes で半透明に
        SetLayeredWindowAttributes(hwnd_c, COLORREF(0), 128, LWA_ALPHA)?;
        println!(
            "[C] Transparent+Layered window: {:?}, ex_style=0x{:X}",
            hwnd_c,
            GetWindowLongPtrW(hwnd_c, GWL_EXSTYLE)
        );

        // SWP_FRAMECHANGED を全ウィンドウに
        for &hwnd in &[hwnd_b, hwnd_c] {
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }

        println!("\n=== クリックスルーテスト ===");
        println!("Window A (赤, 大): 最背面、800x500、クリックすると \"[A] CLICKED\" と表示");
        println!("Window B (青, 左): DComp+WS_EX_TRANSPARENT、A の左半分に重なる 300x300");
        println!("Window C (緑, 右): Layered+WS_EX_TRANSPARENT、A の右半分に重なる 300x300");
        println!();
        println!("テスト1: 右の緑の部分をクリック → [A] CLICKED が出れば C を貫通");
        println!("テスト2: 左の青の部分をクリック → [A] CLICKED が出れば B を貫通");
        println!("テスト3: 赤だけの部分をクリック → [A] CLICKED が出るはず（直接）");
        println!("\nWindow A を閉じると終了");

        // メッセージループ
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}

unsafe fn register_class(hinstance: HMODULE, name: PCWSTR, wndproc: WNDPROC) -> Result<()> {
    unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: wndproc,
            hInstance: hinstance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: name,
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            return Err(Error::from_thread());
        }
    }
    Ok(())
}

/// Window A: 通常ウィンドウ（赤背景）
unsafe extern "system" fn wndproc_normal(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
                let hdc = windows::Win32::Graphics::Gdi::BeginPaint(hwnd, &mut ps);
                let brush = windows::Win32::Graphics::Gdi::CreateSolidBrush(COLORREF(0x000000FF));
                windows::Win32::Graphics::Gdi::FillRect(hdc, &ps.rcPaint, brush);
                windows::Win32::Graphics::Gdi::DeleteObject(brush.into());
                windows::Win32::Graphics::Gdi::EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_LBUTTONDOWN => {
                println!(
                    ">>> [A] CLICKED at ({}, {}) <<<",
                    (lparam.0 as i32) & 0xFFFF,
                    ((lparam.0 as i32) >> 16) & 0xFFFF
                );
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Window B: DComp ウィンドウ（WM_NCHITTEST で HTTRANSPARENT を返す）
unsafe extern "system" fn wndproc_dcomp(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_NCHITTEST => {
                LRESULT(-1) // HTTRANSPARENT
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Window C: Layered ウィンドウ（WM_NCHITTEST で HTTRANSPARENT を返す）
unsafe extern "system" fn wndproc_layered(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
                let hdc = windows::Win32::Graphics::Gdi::BeginPaint(hwnd, &mut ps);
                let brush =
                    windows::Win32::Graphics::Gdi::CreateSolidBrush(COLORREF(0x0000FF00));
                windows::Win32::Graphics::Gdi::FillRect(hdc, &ps.rcPaint, brush);
                windows::Win32::Graphics::Gdi::DeleteObject(brush.into());
                windows::Win32::Graphics::Gdi::EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_NCHITTEST => {
                LRESULT(-1) // HTTRANSPARENT
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Window B に DirectComposition で青い帯を描画
fn setup_dcomp(hwnd: HWND) -> Result<()> {
    // D3D11 → DXGI → D2D → DComp
    let d3d = d3d11_create_device(
        None,
        D3D_DRIVER_TYPE_HARDWARE,
        HMODULE::default(),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
        None,
        D3D11_SDK_VERSION,
        None,
        None,
    )?;
    let dxgi: IDXGIDevice4 = d3d.cast()?;
    let d2d_device = d2d_create_device(&dxgi)?;
    let desktop = dcomp_create_desktop_device(&d2d_device)?;
    let dcomp: IDCompositionDevice3 = desktop.cast()?;

    let target = desktop.create_target_for_hwnd(hwnd, true)?;
    let root_visual = dcomp.create_visual()?;
    target.set_root(&root_visual)?;

    // サーフェス（青い矩形）
    let surface = dcomp.create_surface(300, 300, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_ALPHA_MODE_PREMULTIPLIED)?;

    // サーフェスに青を描画
    {
        let mut offset = POINT::default();
        let d2d_dc: ID2D1DeviceContext =
            unsafe { surface.BeginDraw(None, &mut offset)? };

        let color = D2D1_COLOR_F {
            r: 0.2,
            g: 0.3,
            b: 0.9,
            a: 0.7,
        };
        unsafe { d2d_dc.Clear(Some(&color)) };
        unsafe { surface.EndDraw()? };
    }

    root_visual.set_content(&surface)?;
    dcomp.commit()?;

    // デバイスとターゲットをリークさせて生存させる（テスト用）
    std::mem::forget(dcomp);
    std::mem::forget(target);
    std::mem::forget(root_visual);
    std::mem::forget(surface);

    println!("[B] DirectComposition setup complete (blue surface)");
    Ok(())
}
