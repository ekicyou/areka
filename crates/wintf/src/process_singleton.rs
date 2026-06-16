use std::sync::*;
use tracing::{debug, info};
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

use crate::winproc::*;

const WINTF_CLASS_NAME: &str = "wintf_window_class";
const WINTF_ECS_CLASS_NAME: &str = "wintf_ecs_window_class";

static WIN_PROCESS_SINGLETON: OnceLock<WinProcessSingleton> = OnceLock::new();

#[derive(Debug)]
pub struct WinProcessSingleton {
    instance: HINSTANCE,
    window_class_name: HSTRING,
    ecs_window_class_name: HSTRING,
}

// SAFETY: HINSTANCE は本プロセス実行イメージのモジュールハンドルで、プロセス生存中
// 不変かつ解放されない（GetModuleHandleW(None) の戻り値）。HSTRING はアトミック参照
// カウントの不変 UTF-16 文字列。本構造体は OnceLock 初期化後に一切変更されない
// （全フィールド読み取り専用アクセスのみ）ため、スレッド間の共有（Sync）と
// 所有権移動（Send）はいずれも安全。
unsafe impl Send for WinProcessSingleton {}
unsafe impl Sync for WinProcessSingleton {}

impl WinProcessSingleton {
    pub(crate) fn instance(&self) -> HINSTANCE {
        self.instance
    }

    pub(crate) fn window_class_name(&self) -> &HSTRING {
        &self.window_class_name
    }

    pub(crate) fn ecs_window_class_name(&self) -> &HSTRING {
        &self.ecs_window_class_name
    }

    // NOTE(W1-V): 本初期化クロージャは非冪等（プロセスグローバルな RegisterClassExW を
    // 2 回呼ぶ）。1つ目のクラス登録成功後に2つ目が失敗して panic した場合、OnceLock は
    // 未初期化のまま残り、次の get_or_init はクロージャを再実行して1つ目の
    // RegisterClassExW が ERROR_CLASS_ALREADY_EXISTS で 0 を返し、誤解を招くメッセージで
    // 再 panic する（部分失敗からの回復不能）。RegisterClassExW の失敗は実用上リソース
    // 枯渇時に限られるため現行挙動を維持し、冪等化（既登録の許容）・GetLastError を含む
    // panic メッセージ改善は挙動変更を伴うため P31 として記録。
    pub(crate) fn get_or_init() -> &'static Self {
        WIN_PROCESS_SINGLETON.get_or_init(|| {
            debug!("Window class creation starting...");
            let instance = unsafe { GetModuleHandleW(None).unwrap().into() };
            let window_class_name = HSTRING::from(WINTF_CLASS_NAME);
            let ecs_window_class_name = HSTRING::from(WINTF_ECS_CLASS_NAME);

            // 既存のウィンドウクラスを登録（dcomp_demo用）
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wndproc),
                hInstance: instance,
                hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
                lpszClassName: PCWSTR(window_class_name.as_ptr()),
                ..Default::default()
            };
            unsafe {
                let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                if RegisterClassExW(&wc) == 0 {
                    panic!("Failed to register window class");
                }
            }

            // ECS用のウィンドウクラスを登録
            // CS_DBLCLKS: ダブルクリックメッセージ（WM_*DBLCLK）を受信
            let ecs_wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
                lpfnWndProc: Some(crate::ecs::ecs_wndproc),
                hInstance: instance,
                hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
                lpszClassName: PCWSTR(ecs_window_class_name.as_ptr()),
                ..Default::default()
            };
            unsafe {
                if RegisterClassExW(&ecs_wc) == 0 {
                    panic!("Failed to register ECS window class");
                }
            }

            info!("Window classes created");
            Self {
                instance,
                window_class_name,
                ecs_window_class_name,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意: get_or_init はウィンドウクラス登録（RegisterClassExW）と
    // プロセス DPI 設定というプロセスグローバルな副作用を持つが、
    // ウィンドウ生成・メッセージループは伴わないためヘッドレスで実行可能。
    // OnceLock により同一テストバイナリ内で初期化は 1 回に限定される。

    #[test]
    fn get_or_init_returns_same_static_instance() {
        let a = WinProcessSingleton::get_or_init();
        let b = WinProcessSingleton::get_or_init();
        assert!(std::ptr::eq(a, b));
    }

    #[test]
    fn get_or_init_exposes_expected_class_names_and_instance() {
        let s = WinProcessSingleton::get_or_init();
        assert_eq!(s.window_class_name().to_string(), "wintf_window_class");
        assert_eq!(
            s.ecs_window_class_name().to_string(),
            "wintf_ecs_window_class"
        );
        assert!(!s.instance().is_invalid());
    }
}
