// Phase 0: DirectComposition + SetWindowRgn 互換性検証プロトタイプ
//
// 検証項目:
// 1. WS_EX_NOREDIRECTIONBITMAP ウィンドウに SetWindowRgn を適用して API が成功すること
// 2. DirectComposition Visual の描画が維持されること（目視確認）
// 3. リージョン外のクリックが他プロセスに貫通すること（目視確認）
//
// 使用方法:
//   cargo run --example setwindowrgn_compat_test
//
// 期待動作:
//   - ウィンドウ中央に青い矩形が表示される
//   - リージョン外（矩形の外側周辺）をクリックするとデスクトップアイコン等にクリックが貫通する
//   - ESC キーで終了

use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;
use wintf::com::{d2d::*, d3d11::*, dcomp::*};

const TIMER_ID_APPLY_RGN: usize = 1;
static REGION_APPLIED: AtomicBool = AtomicBool::new(false);

const WINDOW_WIDTH: i32 = 400;
const WINDOW_HEIGHT: i32 = 400;
const REGION_MARGIN: i32 = 80;

fn main() -> Result<()> {
    // ウィンドウクラス登録
    let class_name = w!("SetWindowRgnCompatTest");
    let hinstance: HINSTANCE =
        unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None)? }.into();
    let wc = WNDCLASSEXW {
        cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc),
        hInstance: hinstance,
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW)? },
        lpszClassName: class_name,
        ..Default::default()
    };
    unsafe { RegisterClassExW(&wc) };

    // WS_EX_NOREDIRECTIONBITMAP + WS_POPUP (DirectComposition 用の透過ウィンドウ)
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST,
            class_name,
            w!("SetWindowRgn + DirectComposition Test"),
            WS_POPUP | WS_VISIBLE,
            200,
            200,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            None,
            None,
            Some(hinstance),
            None,
        )?
    };

    println!("[Phase 0] Window created: hwnd={:?}", hwnd);

    // === DirectComposition セットアップ ===
    let d3d_device = create_d3d_device()?;
    let dxgi: windows::Win32::Graphics::Dxgi::IDXGIDevice4 = d3d_device.cast()?;
    let d2d = d2d_create_device(&dxgi)?;

    let desktop = dcomp_create_desktop_device(&d2d)?;
    let dcomp: IDCompositionDevice3 = desktop.cast()?;

    let target = desktop.create_target_for_hwnd(hwnd, true)?;

    let root_visual = dcomp.create_visual()?;
    unsafe { target.SetRoot(&root_visual)? };

    // Surface 作成（DirectComposition Surface）
    let surface = dcomp.create_surface(
        WINDOW_WIDTH as u32,
        WINDOW_HEIGHT as u32,
        DXGI_FORMAT_B8G8R8A8_UNORM,
        DXGI_ALPHA_MODE_PREMULTIPLIED,
    )?;

    // Surface に描画
    {
        let mut offset = POINT::default();
        let dc: ID2D1DeviceContext =
            unsafe { surface.BeginDraw::<ID2D1DeviceContext>(None, &mut offset as *mut _)? };

        let ox = offset.x as f32;
        let oy = offset.y as f32;
        println!("[DEBUG] BeginDraw offset: ({}, {})", offset.x, offset.y);

        // 背景クリア（完全透明）
        unsafe {
            dc.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
        }

        // マージン領域（リージョン外）を明るいマゼンタで塗る
        // SetWindowRgn 適用後にこれが見えるかどうかで DComp vs Region クリッピングを検証
        let margin_brush = unsafe {
            dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 0.0,
                    b: 1.0,
                    a: 0.6,
                },
                None,
            )?
        };
        // 上マージン
        unsafe {
            dc.FillRectangle(
                &D2D_RECT_F {
                    left: ox,
                    top: oy,
                    right: ox + WINDOW_WIDTH as f32,
                    bottom: oy + REGION_MARGIN as f32,
                },
                &margin_brush,
            );
        }
        // 下マージン
        unsafe {
            dc.FillRectangle(
                &D2D_RECT_F {
                    left: ox,
                    top: oy + (WINDOW_HEIGHT - REGION_MARGIN) as f32,
                    right: ox + WINDOW_WIDTH as f32,
                    bottom: oy + WINDOW_HEIGHT as f32,
                },
                &margin_brush,
            );
        }
        // 左マージン
        unsafe {
            dc.FillRectangle(
                &D2D_RECT_F {
                    left: ox,
                    top: oy + REGION_MARGIN as f32,
                    right: ox + REGION_MARGIN as f32,
                    bottom: oy + (WINDOW_HEIGHT - REGION_MARGIN) as f32,
                },
                &margin_brush,
            );
        }
        // 右マージン
        unsafe {
            dc.FillRectangle(
                &D2D_RECT_F {
                    left: ox + (WINDOW_WIDTH - REGION_MARGIN) as f32,
                    top: oy + REGION_MARGIN as f32,
                    right: ox + WINDOW_WIDTH as f32,
                    bottom: oy + (WINDOW_HEIGHT - REGION_MARGIN) as f32,
                },
                &margin_brush,
            );
        }

        // 赤い枠線を描画（ウィンドウ全体の境界を示す）
        let red_brush = unsafe {
            dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.8,
                },
                None,
            )?
        };
        unsafe {
            dc.DrawRectangle(
                &D2D_RECT_F {
                    left: ox + 1.0,
                    top: oy + 1.0,
                    right: ox + WINDOW_WIDTH as f32 - 1.0,
                    bottom: oy + WINDOW_HEIGHT as f32 - 1.0,
                },
                &red_brush,
                2.0,
                None,
            );
        }

        // 中央に青い矩形を描画（リージョン = ヒット領域）
        let blue_brush = unsafe {
            dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.2,
                    g: 0.4,
                    b: 0.9,
                    a: 1.0,
                },
                None,
            )?
        };
        unsafe {
            dc.FillRectangle(
                &D2D_RECT_F {
                    left: ox + REGION_MARGIN as f32,
                    top: oy + REGION_MARGIN as f32,
                    right: ox + (WINDOW_WIDTH - REGION_MARGIN) as f32,
                    bottom: oy + (WINDOW_HEIGHT - REGION_MARGIN) as f32,
                },
                &blue_brush,
            );
        }

        // リージョン境界を緑の枠線で示す
        let green_brush = unsafe {
            dc.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                },
                None,
            )?
        };
        unsafe {
            dc.DrawRectangle(
                &D2D_RECT_F {
                    left: ox + REGION_MARGIN as f32,
                    top: oy + REGION_MARGIN as f32,
                    right: ox + (WINDOW_WIDTH - REGION_MARGIN) as f32,
                    bottom: oy + (WINDOW_HEIGHT - REGION_MARGIN) as f32,
                },
                &green_brush,
                3.0,
                None,
            );
        }

        unsafe { surface.EndDraw()? };
    }

    // Visual に Surface を設定
    unsafe { root_visual.SetContent(&surface)? };

    // コミット
    dcomp.commit()?;

    println!("[Phase 0] DirectComposition Visual rendering complete");

    // === SetWindowRgn は 3 秒後にタイマーで適用 ===
    // まずリージョンなしで全描画が見えることを確認してから適用する
    unsafe { SetTimer(Some(hwnd), TIMER_ID_APPLY_RGN, 3000, None) };

    println!("[Phase 0] DirectComposition rendering complete (no region yet)");
    println!();
    println!("=== Verification Steps ===");
    println!("1. マゼンタの枠が青矩形の周囲に表示されていることを確認（DComp描画OK）");
    println!("2. 3秒後に SetWindowRgn が自動適用される");
    println!("   → マゼンタが消えるかどうかで DWM の Region クリッピング挙動を確認");
    println!("3. 'R' キーで SetWindowRgn ON/OFF をトグル");
    println!("4. マゼンタ領域クリック: クリックスルーなら OK");
    println!("5. 青矩形クリック: このウィンドウが受け取れば OK");
    println!("6. ESC で終了");
    println!();
    println!("All pass -> GO");
    println!("Any fail -> NO-GO");

    // メッセージループ
    let mut msg = MSG::default();
    loop {
        let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if !ret.as_bool() {
            break;
        }
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}

fn create_d3d_device() -> Result<ID3D11Device> {
    let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
    let feature_levels = [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0];
    let mut device = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            flags,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )?;
    }
    device.ok_or_else(|| Error::from(E_FAIL))
}

fn apply_region(hwnd: HWND) {
    let rgn = unsafe {
        CreateRectRgn(
            REGION_MARGIN,
            REGION_MARGIN,
            WINDOW_WIDTH - REGION_MARGIN,
            WINDOW_HEIGHT - REGION_MARGIN,
        )
    };
    let result = unsafe { SetWindowRgn(hwnd, Some(rgn), true) };
    if result != 0 {
        REGION_APPLIED.store(true, Ordering::SeqCst);
        println!("[Phase 0] SetWindowRgn APPLIED (region ON)");
    } else {
        let err = unsafe { GetLastError() };
        println!("[Phase 0] SetWindowRgn FAILED: {:?}", err);
        unsafe {
            let _ = DeleteObject(rgn.into());
        };
    }
}

fn remove_region(hwnd: HWND) {
    let result = unsafe { SetWindowRgn(hwnd, None, true) };
    if result != 0 {
        REGION_APPLIED.store(false, Ordering::SeqCst);
        println!("[Phase 0] SetWindowRgn REMOVED (region OFF)");
    } else {
        println!("[Phase 0] SetWindowRgn remove FAILED");
    }
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_DESTROY => {
            unsafe {
                KillTimer(Some(hwnd), TIMER_ID_APPLY_RGN).ok();
                PostQuitMessage(0);
            };
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == TIMER_ID_APPLY_RGN {
                unsafe {
                    KillTimer(Some(hwnd), TIMER_ID_APPLY_RGN).ok();
                }
                apply_region(hwnd);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            match wparam.0 {
                0x1B => {
                    // ESC to exit
                    let _ = unsafe { DestroyWindow(hwnd) };
                }
                0x52 => {
                    // 'R' to toggle region
                    if REGION_APPLIED.load(Ordering::SeqCst) {
                        remove_region(hwnd);
                    } else {
                        apply_region(hwnd);
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            println!("[Phase 0] Click detected: region hit OK");
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}
