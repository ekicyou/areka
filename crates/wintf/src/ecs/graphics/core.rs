use crate::com::d2d::*;
use crate::com::d3d11::*;
use crate::com::dwrite::*;
use bevy_ecs::prelude::*;
use tracing::{debug, info};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::core::{Interface, Result};

#[derive(Debug)]
struct GraphicsCoreInner {
    pub d3d: ID3D11Device,
    pub dxgi: IDXGIDevice4,
    pub d2d_factory: ID2D1Factory,
    pub d2d: ID2D1Device,
    pub d2d_device_context: ID2D1DeviceContext, // グローバル共有DeviceContext
    pub dwrite_factory: IDWriteFactory2,
}

#[derive(Resource, Debug)]
pub struct GraphicsCore {
    inner: Option<GraphicsCoreInner>,
}

unsafe impl Send for GraphicsCore {}
unsafe impl Sync for GraphicsCore {}

impl GraphicsCore {
    pub fn new() -> Result<Self> {
        info!("[GraphicsCore] Initialization started");

        let d3d = create_device_3d()?;
        let dxgi = d3d.cast()?;
        let d2d_factory = create_d2d_factory()?;
        let d2d = d2d_create_device(&dxgi)?;

        // グローバル共有DeviceContextを作成
        let d2d_device_context = d2d.create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;
        debug!("[GraphicsCore] Global DeviceContext created");

        let dwrite_factory = dwrite_create_factory(DWRITE_FACTORY_TYPE_SHARED)?;

        info!("[GraphicsCore] Initialization completed");

        Ok(Self {
            inner: Some(GraphicsCoreInner {
                d3d,
                dxgi,
                d2d_factory,
                d2d,
                d2d_device_context,
                dwrite_factory,
            }),
        })
    }

    pub fn invalidate(&mut self) {
        self.inner = None;
    }

    pub fn is_valid(&self) -> bool {
        self.inner.is_some()
    }

    pub fn d2d_factory(&self) -> Option<&ID2D1Factory> {
        self.inner.as_ref().map(|i| &i.d2d_factory)
    }

    pub fn d2d_device(&self) -> Option<&ID2D1Device> {
        self.inner.as_ref().map(|i| &i.d2d)
    }

    pub fn dwrite_factory(&self) -> Option<&IDWriteFactory2> {
        self.inner.as_ref().map(|i| &i.dwrite_factory)
    }

    /// グローバル共有DeviceContextへの参照を取得
    pub fn device_context(&self) -> Option<&ID2D1DeviceContext> {
        self.inner.as_ref().map(|i| &i.d2d_device_context)
    }

    pub fn d3d(&self) -> Option<&ID3D11Device> {
        self.inner.as_ref().map(|i| &i.d3d)
    }

    pub fn dxgi(&self) -> Option<&IDXGIDevice4> {
        self.inner.as_ref().map(|i| &i.dxgi)
    }
}

/// D2DFactoryを作成（マルチスレッド対応）
fn create_d2d_factory() -> Result<ID2D1Factory> {
    #[allow(unused_imports)]
    use windows::Win32::Graphics::Direct2D::Common::*;

    unsafe { D2D1CreateFactory::<ID2D1Factory>(D2D1_FACTORY_TYPE_MULTI_THREADED, None) }
}

fn create_device_3d() -> Result<ID3D11Device> {
    #[cfg(debug_assertions)]
    let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_DEBUG;

    #[cfg(not(debug_assertions))]
    let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

    d3d11_create_device(
        None,
        D3D_DRIVER_TYPE_HARDWARE,
        HMODULE::default(),
        flags,
        None,
        D3D11_SDK_VERSION,
        None,
        None,
    )
}

// ============================================================
// FrameTime - 高精度フレーム時刻リソース
// ============================================================

/// FrameTime - 高精度フレーム時刻リソース
///
/// GetSystemTimePreciseAsFileTime（100ナノ秒単位）を使用。
/// Windows 8以降で利用可能な最高精度のシステム時刻API。
/// OS起動時からの経過時刻を提供するリソース。
///
/// dola の `clock::now()` と同じ時刻基準（OS起動時=0秒、QueryPerformanceCounter ベース）を使用。
/// スレッドセーフ、どのスケジュールからでもアクセス可能。
#[derive(Resource, Debug)]
pub struct FrameTime;

impl FrameTime {
    /// リソース作成
    pub fn new() -> Self {
        Self
    }

    /// 現在時刻取得 (f64秒、OS起動時からの経過時間)
    ///
    /// dola の `clock::now()` と同じ時刻基準。
    /// `QueryPerformanceCounter / QueryPerformanceFrequency` ベース。
    pub fn elapsed_secs(&self) -> f64 {
        Self::query_performance_time()
    }

    /// 高精度システム時刻を取得（OS起動時からの秒数）
    fn query_performance_time() -> f64 {
        use windows::Win32::System::Performance::{
            QueryPerformanceCounter, QueryPerformanceFrequency,
        };

        let mut counter: i64 = 0;
        let mut frequency: i64 = 0;
        unsafe {
            let _ = QueryPerformanceCounter(&mut counter);
            let _ = QueryPerformanceFrequency(&mut frequency);
        }
        (counter as f64) / (frequency as f64)
    }
}

impl Default for FrameTime {
    fn default() -> Self {
        Self::new()
    }
}
