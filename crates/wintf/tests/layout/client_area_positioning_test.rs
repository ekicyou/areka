//! client-area-positioning機能のテスト

use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use wintf::ecs::{Point, SizeI};

/// WS_OVERLAPPEDWINDOWスタイルでの座標変換テスト
#[test]
fn test_to_window_coords_overlapped_window() {
    // 見えない実際のウィンドウで試す（読む API は非表示でも動き、前面を奪わない）
    let hwnd = create_test_window(WS_OVERLAPPEDWINDOW, WS_EX_LAYERED);
    assert!(!hwnd.is_invalid(), "テストウィンドウの作成に失敗");

    let window_pos = wintf::ecs::window::WindowPos::new()
        .with_position(Point { x: 100, y: 100 })
        .with_size(SizeI {
            width: 800,
            height: 600,
        });

    let window_handle = wintf::ecs::window::WindowHandle { hwnd };
    let result = window_pos.to_window_coords(&window_handle);

    unsafe {
        let _ = DestroyWindow(hwnd);
    }

    let (x, y, width, height) = result.expect("座標変換が成功すること");

    // WS_OVERLAPPEDWINDOWスタイルでは、タイトルバーとボーダーが付くため:
    // - x座標はクライアント領域より左にオフセット
    // - y座標はクライアント領域より上にオフセット（タイトルバー分）
    // - width/heightはボーダー分だけ大きくなる

    assert!(
        x < 100,
        "x座標がクライアント領域より左であること: got {}",
        x
    );
    assert!(
        y < 100,
        "y座標がクライアント領域より上であること (タイトルバー分): got {}",
        y
    );

    assert!(
        width > 800,
        "幅がクライアント領域より大きいこと: got {}",
        width
    );
    assert!(
        height > 600,
        "高さがクライアント領域より大きいこと: got {}",
        height
    );
}

/// WS_POPUPスタイルでの座標変換テスト
#[test]
fn test_to_window_coords_popup_window() {
    let hwnd = create_test_window(WS_POPUP, WINDOW_EX_STYLE(0));
    assert!(!hwnd.is_invalid(), "テストウィンドウの作成に失敗");

    let window_pos = wintf::ecs::window::WindowPos::new()
        .with_position(Point { x: 100, y: 100 })
        .with_size(SizeI {
            width: 800,
            height: 600,
        });

    let window_handle = wintf::ecs::window::WindowHandle { hwnd };
    let result = window_pos.to_window_coords(&window_handle);

    unsafe {
        let _ = DestroyWindow(hwnd);
    }

    let (x, y, width, height) = result.expect("座標変換が成功すること");

    // WS_POPUPスタイルでは装飾なしのため、入力座標と同一であること
    assert_eq!(x, 100, "x座標がクライアント領域と同一であること");
    assert_eq!(y, 100, "y座標がクライアント領域と同一であること");
    assert_eq!(width, 800, "幅がクライアント領域と同一であること");
    assert_eq!(height, 600, "高さがクライアント領域と同一であること");
}

/// エラーハンドリングテスト
/// 無効なHWND(HWND(0))を渡した場合のテスト
#[test]
fn test_to_window_coords_invalid_hwnd() {
    let invalid_hwnd = HWND(std::ptr::null_mut());

    let window_pos = wintf::ecs::window::WindowPos::new()
        .with_position(Point { x: 100, y: 100 })
        .with_size(SizeI {
            width: 800,
            height: 600,
        });

    let window_handle = wintf::ecs::window::WindowHandle { hwnd: invalid_hwnd };
    let result = window_pos.to_window_coords(&window_handle);

    assert!(result.is_err(), "無効なHWNDでErrが返されること");

    let error_msg = result.unwrap_err();
    assert!(!error_msg.is_empty(), "エラーメッセージが空でないこと");
}

fn create_test_window(style: WINDOW_STYLE, ex_style: WINDOW_EX_STYLE) -> HWND {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;

    let class_name: Vec<u16> = OsStr::new("ClientAreaTestWindow")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let hinstance = GetModuleHandleW(None).unwrap_or_default();

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(test_window_proc),
            hInstance: hinstance.into(),
            lpszClassName: windows::core::PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        RegisterClassExW(&wc);

        CreateWindowExW(
            ex_style,
            windows::core::PCWSTR(class_name.as_ptr()),
            windows::core::PCWSTR(class_name.as_ptr()),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            400,
            300,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .unwrap_or_default()
    }
}

unsafe extern "system" fn test_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
