//! `WicDecoderArm` の兄弟テストが共有する手伝い（spec: emo-atlas-palette-alpha 要件 5.6）。

use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use super::WicDecoderArm;

/// COM を張った下で本番の読み手を組んで渡す（`new` は COM が要る）。
///
/// 初期化に失敗したら落とす。別の型の apartment に入っているスレッド（`RPC_E_CHANGED_MODE`）は
/// そのまま使い、入っていない apartment を `CoUninitialize` で壊さない。
pub(super) fn with_arm<F: FnOnce(&WicDecoderArm)>(f: F) {
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let entered = hr.is_ok();
    assert!(
        entered || hr == RPC_E_CHANGED_MODE,
        "CoInitializeEx が失敗した: {hr:?}"
    );
    {
        let arm = WicDecoderArm::new().expect("WIC factory creates under COM init");
        f(&arm);
    }
    if entered {
        unsafe { CoUninitialize() };
    }
}
