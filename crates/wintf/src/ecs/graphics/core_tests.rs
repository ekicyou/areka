//! GraphicsCore: 開発用の旗の作り直し（7 番）・工場の同一性（8 番）・装置の失いの判定（6 番）

use super::*;
use std::cell::RefCell;
use windows::Win32::Graphics::Dxgi::{
    DXGI_ERROR_DEVICE_HUNG, DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET,
    DXGI_ERROR_SDK_COMPONENT_MISSING,
};
use windows::core::Error;

const DEBUG_FLAGS: D3D11_CREATE_DEVICE_FLAG =
    D3D11_CREATE_DEVICE_FLAG(D3D11_CREATE_DEVICE_BGRA_SUPPORT.0 | D3D11_CREATE_DEVICE_DEBUG.0);

/// 開発用の旗が SDK の部品の無さで断られたら、旗を外した 2 度目で作れる。
#[test]
fn debug_flag_rejected_for_missing_sdk_retries_without_it() {
    let calls = RefCell::new(Vec::new());
    let made = create_device_3d_with(DEBUG_FLAGS, |flags| {
        calls.borrow_mut().push(flags);
        if flags.contains(D3D11_CREATE_DEVICE_DEBUG) {
            Err(Error::from(DXGI_ERROR_SDK_COMPONENT_MISSING))
        } else {
            Ok(flags)
        }
    });
    assert_eq!(
        made.expect("2 度目で作れる"),
        D3D11_CREATE_DEVICE_BGRA_SUPPORT
    );
    assert_eq!(
        calls.into_inner(),
        vec![DEBUG_FLAGS, D3D11_CREATE_DEVICE_BGRA_SUPPORT]
    );
}

/// ほかの失敗では作り直さず、1 度目の失敗をそのまま返す。
#[test]
fn other_failures_are_returned_without_retry() {
    let calls = RefCell::new(0);
    let made: Result<()> = create_device_3d_with(DEBUG_FLAGS, |_| {
        *calls.borrow_mut() += 1;
        Err(Error::from(E_INVALIDARG))
    });
    assert_eq!(made.unwrap_err().code(), E_INVALIDARG);
    assert_eq!(calls.into_inner(), 1);
}

/// 描く文脈が属する工場は、`d2d_factory()` と同じ物（COM の同一性）。
#[test]
fn device_context_belongs_to_the_exposed_factory() {
    let core = GraphicsCore::new().expect("GraphicsCore");
    let exposed: windows::core::IUnknown = core.d2d_factory().expect("factory").cast().unwrap();
    let dc = core.device_context().expect("dc");
    let owner: ID2D1Factory = unsafe { dc.GetFactory() }.expect("GetFactory");
    let owner: windows::core::IUnknown = owner.cast().unwrap();
    assert_eq!(exposed, owner);
}

/// 失いの判定の真偽表。
#[test]
fn is_device_lost_truth_table() {
    for lost in [
        DXGI_ERROR_DEVICE_REMOVED,
        DXGI_ERROR_DEVICE_RESET,
        DXGI_ERROR_DEVICE_HUNG,
        D2DERR_RECREATE_TARGET,
    ] {
        assert!(is_device_lost(lost, S_OK), "{lost:?} は失い");
    }
    assert!(!is_device_lost(E_INVALIDARG, S_OK));
    assert!(!is_device_lost(E_FAIL, S_OK));
    // 装置が失いの理由を返すなら、描画の失敗の値に依らず失い。
    assert!(is_device_lost(E_INVALIDARG, DXGI_ERROR_DEVICE_REMOVED));
}

/// 健全な実の装置で、失いの値なら無効になり、2 度目は偽。
#[test]
fn note_draw_failure_invalidates_on_device_removed() {
    let mut core = GraphicsCore::new().expect("GraphicsCore");
    assert!(core.note_draw_failure(&Error::from(DXGI_ERROR_DEVICE_REMOVED)));
    assert!(!core.is_valid());
    assert!(!core.note_draw_failure(&Error::from(DXGI_ERROR_DEVICE_REMOVED)));
}

/// 健全な実の装置で、失いでない失敗（`E_INVALIDARG`）なら有効のまま。
#[test]
fn note_draw_failure_keeps_healthy_device_on_invalid_arg() {
    let mut core = GraphicsCore::new().expect("GraphicsCore");
    assert!(!core.note_draw_failure(&Error::from(E_INVALIDARG)));
    assert!(core.is_valid());
}
