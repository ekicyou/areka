//! `with_com_initialized` が自分で初期化に成功したときだけ後始末する檻。

use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

use super::with_com_initialized;

/// STA で初期化済みの糸で呼んだ後も、その糸の COM は初期化されたまま
/// （もう一度の `CoInitializeEx(STA)` が `S_FALSE`＝すでに初期化済み）。
#[test]
fn with_com_initialized_keeps_an_already_initialized_sta_thread() {
    std::thread::spawn(|| {
        let first = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        assert_eq!(first, windows::Win32::Foundation::S_OK);

        with_com_initialized(|| {});

        let again = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        assert_eq!(again, windows::Win32::Foundation::S_FALSE);
        unsafe {
            CoUninitialize();
            CoUninitialize();
        }
    })
    .join()
    .unwrap();
}
