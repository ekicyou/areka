// .minimal() の束縛と、それが出さない口の手書きの包み。
// .minimal() は自由関数を生の `link!`（ポインタと HRESULT）でしか出さないので、
// 既定の形（q3-default）と同じ呼び方になる包みをここに置く。ここに書いた名前は
// 束縛の glob より強いので、呼ぶ側は `use bindings::*` のまま包みに届く。
#![allow(non_snake_case, clippy::too_many_arguments)]

#[allow(warnings)]
mod raw {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
pub use raw::*;

use core::ffi::c_void;
use windows_core::*;

/// CoInitializeEx（既定の形と同じく `Option` の予約の引数）
pub unsafe fn CoInitializeEx(pvreserved: Option<*const c_void>, dwcoinit: u32) -> HRESULT {
    unsafe { raw::CoInitializeEx(pvreserved.unwrap_or(core::ptr::null()) as _, dwcoinit) }
}

/// D3D11CreateDevice（out の装置を `Option<ID3D11Device>` で受ける）
pub unsafe fn D3D11CreateDevice(
    padapter: Option<*mut c_void>,
    drivertype: D3D_DRIVER_TYPE,
    software: HMODULE,
    flags: u32,
    pfeaturelevels: Option<*const D3D_FEATURE_LEVEL>,
    featurelevels: u32,
    sdkversion: u32,
    ppdevice: Option<&mut Option<ID3D11Device>>,
    pfeaturelevel: Option<*mut D3D_FEATURE_LEVEL>,
    ppimmediatecontext: Option<*mut *mut c_void>,
) -> HRESULT {
    unsafe {
        raw::D3D11CreateDevice(
            padapter.unwrap_or(core::ptr::null_mut()),
            drivertype,
            software,
            flags,
            pfeaturelevels.unwrap_or(core::ptr::null()),
            featurelevels,
            sdkversion,
            // Option<ID3D11Device> は 1 つのポインタと同じ形（None＝null）
            ppdevice.map_or(core::ptr::null_mut(), |p| p as *mut _ as *mut *mut c_void),
            pfeaturelevel.unwrap_or(core::ptr::null_mut()),
            ppimmediatecontext.unwrap_or(core::ptr::null_mut()),
        )
    }
}

/// D2D1CreateFactory（既定の形と同じく options を `Option` で受ける）
pub unsafe fn D2D1CreateFactory(
    factorytype: D2D1_FACTORY_TYPE,
    riid: *const GUID,
    pfactoryoptions: Option<*const D2D1_FACTORY_OPTIONS>,
    ppifactory: *mut *mut c_void,
) -> HRESULT {
    unsafe {
        raw::D2D1CreateFactory(
            factorytype,
            riid,
            pfactoryoptions.unwrap_or(core::ptr::null()),
            ppifactory,
        )
    }
}

/// DWriteCreateFactory（既定の形と同じく `Result<IUnknown>` で返す）
pub unsafe fn DWriteCreateFactory(
    factorytype: DWRITE_FACTORY_TYPE,
    iid: *const GUID,
) -> Result<IUnknown> {
    let mut p = core::ptr::null_mut();
    unsafe {
        raw::DWriteCreateFactory(factorytype, iid, &mut p).ok()?;
        Ok(IUnknown::from_raw(p))
    }
}
