//! UI スレッドの COM の初期化（7）。別の部屋で初期化済みのときだけ `warn!` が 1 行残る。
//!
//! どちらのテストも新しいスレッドで回す（テストのスレッドの COM の状態に左右されない）。

use super::*;
use log_capture_kit::count_levels;

#[test]
fn already_sta_thread_is_ok_with_one_warn() {
    std::thread::spawn(|| {
        // SAFETY: このスレッドの COM を STA で初期化し、最後に対で閉じる。
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .expect("STA で初期化できるはず");
        let (result, counts) = count_levels(init_ui_com);
        // SAFETY: 上の STA の初期化と対。
        unsafe { CoUninitialize() };
        assert!(
            result.is_ok(),
            "別の部屋で初期化済みでも Ok のはず: {result:?}"
        );
        assert_eq!(counts.warn, 1, "別の部屋の食い違いを warn 1 行で残すはず");
    })
    .join()
    .expect("テストのスレッドが panic した");
}

#[test]
fn uninitialized_thread_is_ok_without_warn() {
    std::thread::spawn(|| {
        let (result, counts) = count_levels(init_ui_com);
        assert!(
            result.is_ok(),
            "初期化していないスレッドは Ok のはず: {result:?}"
        );
        // SAFETY: init_ui_com の MTA の初期化（S_OK）と対。
        unsafe { CoUninitialize() };
        assert_eq!(counts.warn, 0, "初めての初期化では warn を残さないはず");
    })
    .join()
    .expect("テストのスレッドが panic した");
}
