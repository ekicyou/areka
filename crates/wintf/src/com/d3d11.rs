use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::core::*;

/// D3D11CreateDevice
pub fn d3d11_create_device<P0>(
    padapter: P0,
    drivertype: D3D_DRIVER_TYPE,
    software: HMODULE,
    flags: D3D11_CREATE_DEVICE_FLAG,
    pfeaturelevels: Option<&[D3D_FEATURE_LEVEL]>,
    sdkversion: u32,
    featurelevel: Option<&mut D3D_FEATURE_LEVEL>,
    immediatecontext: Option<&mut Option<ID3D11DeviceContext>>,
) -> Result<ID3D11Device>
where
    P0: Param<IDXGIAdapter>,
{
    let featurelevel = featurelevel.map(|v| v as *mut _);
    let immediatecontext = immediatecontext.map(|v| v as *mut _);
    let mut device = None;
    unsafe {
        D3D11CreateDevice(
            padapter,
            drivertype,
            software,
            flags,
            pfeaturelevels,
            sdkversion,
            Some(&mut device),
            featurelevel,
            immediatecontext,
        )
        // NOTE: この unwrap は成功経路でのみ到達する。ppDevice（Some(&mut device)）を
        // 非 null で渡しているため、D3D11CreateDevice の契約上 HRESULT 成功時には
        // out パラメータが必ず書き込まれる（失敗時は map が実行されず unwrap に
        // 到達しない）。
        .map(|()| device.unwrap())
    }
}

pub trait D3D11DeviceExt {
    /// GetDeviceRemovedReason
    fn get_device_removed_reason(&self) -> Result<()>;
    /// CreateTexture2D
    fn create_texture2d(
        &self,
        pdesc: &D3D11_TEXTURE2D_DESC,
        pinitialdata: Option<&D3D11_SUBRESOURCE_DATA>,
    ) -> Result<ID3D11Texture2D>;
}

impl D3D11DeviceExt for ID3D11Device {
    #[inline(always)]
    fn get_device_removed_reason(&self) -> Result<()> {
        unsafe { self.GetDeviceRemovedReason() }
    }

    #[inline(always)]
    fn create_texture2d(
        &self,
        pdesc: &D3D11_TEXTURE2D_DESC,
        pinitialdata: Option<&D3D11_SUBRESOURCE_DATA>,
    ) -> Result<ID3D11Texture2D> {
        let pdesc = pdesc as *const _;
        let pinitialdata = pinitialdata.map(|data| data as *const _);
        let mut texture2d: Option<ID3D11Texture2D> = None;
        let pptexture2d = Some(&mut texture2d as *mut _);
        unsafe { self.CreateTexture2D(pdesc, pinitialdata, pptexture2d)? }
        // NOTE: `?` が失敗時に先行リターンするため、この unwrap は成功時のみ到達。
        // ppTexture2D を非 null で渡しており、成功時の out パラメータ書き込みは
        // API 契約で保証される（tests/com/d3d11_test.rs の幅 0 エラーテストで
        // 失敗経路が unwrap に到達しないことを特性化済み）。
        Ok(texture2d.unwrap())
    }
}
