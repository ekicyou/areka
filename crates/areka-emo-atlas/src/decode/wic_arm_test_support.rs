//! `WicDecoderArm` の兄弟テストが共有する手伝い（spec: emo-atlas-palette-alpha 要件 5.6）。

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use super::WicDecoderArm;

/// COM を張った下で本番の読み手を組んで渡す（`new` は COM が要る）。
pub(super) fn with_arm<F: FnOnce(&WicDecoderArm)>(f: F) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    {
        let arm = WicDecoderArm::new().expect("WIC factory creates under COM init");
        f(&arm);
    }
    unsafe {
        CoUninitialize();
    }
}
