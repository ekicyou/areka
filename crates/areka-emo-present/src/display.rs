//! 表示の段の装置の失敗の注入点（テストビルドでだけ実体を持つ）。

use crate::command::PresentError;
#[cfg(test)]
use crate::command::device_err;

/// 表示の段の GPU の呼び出し（`GpuAtlas` の頁の上げと葉の記録）が失敗し得る 4 点。
///
/// テストビルドでのみ実体を持つ注入点 [`fault_point`] の引数であり、通常ビルドでは「何もしない
/// 空の処理」の識別子としてのみ現れる（実行時の分岐・確保・呼出は増えない）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DisplayFault {
    /// `CreateBitmap`（アトラスの頁の GPU への上げ）の直前。
    CreateBitmap,
    /// `CreateCommandList` の直前。
    CreateCommandList,
    /// `EndDraw`（描画終了）が失敗した場合（注入は実呼び出しの直後）。
    EndDraw,
    /// `Close`（コマンドリストを閉じる）が失敗した場合（注入は実呼び出しの直後）。
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
pub(crate) fn fault_point(
    at: DisplayFault,
    repeat: Option<(&'static str, i32)>,
) -> Result<(), PresentError> {
    if ARMED_DISPLAY_FAULT.with(|armed| armed.get()) != Some(at) {
        return Ok(());
    }
    ARMED_DISPLAY_FAULT.with(|armed| armed.set(None));
    let e = windows::core::Error::from_hresult(windows::core::HRESULT(0x8000_4005u32 as i32));
    Err(device_err(injected_context(at), repeat)(e))
}

/// 失敗の注入点（通常ビルド）。何もしない空の処理＝常に `Ok(())`。
#[cfg(not(test))]
#[inline(always)]
pub(crate) fn fault_point(
    _at: DisplayFault,
    _repeat: Option<(&'static str, i32)>,
) -> Result<(), PresentError> {
    Ok(())
}

/// テスト専用: 次の一致点で 1 回だけ失敗させる（同一スレッド）。
///
/// 返す守りを落とすと（正常終了・panic による巻き戻しの双方で）武装を解く。`fault_point` は
/// 一致した点でしか武装を解かないので、届かなかった注入が後続の記録で発火しないようにする。
#[cfg(test)]
pub(crate) fn arm_display_fault(at: DisplayFault) -> DisplayFaultGuard {
    ARMED_DISPLAY_FAULT.with(|armed| armed.set(Some(at)));
    DisplayFaultGuard
}

/// テスト専用: [`arm_display_fault`] の守り（落とすと武装を解く）。
#[cfg(test)]
#[must_use = "守りをすぐ落とすと武装もすぐ解ける"]
pub(crate) struct DisplayFaultGuard;

#[cfg(test)]
impl Drop for DisplayFaultGuard {
    fn drop(&mut self) {
        ARMED_DISPLAY_FAULT.with(|armed| armed.set(None));
    }
}

#[cfg(test)]
#[path = "display_gpu_tests.rs"]
pub(crate) mod gpu_tests;
