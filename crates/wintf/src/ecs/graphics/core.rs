use crate::com::d2d::*;
use crate::com::d3d11::*;
use crate::com::dwrite::*;
use bevy_ecs::prelude::*;
use tracing::{debug, error, info, warn};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectWrite::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::core::{HRESULT, Interface, Result};

#[derive(Debug)]
struct GraphicsCoreInner {
    pub d3d: ID3D11Device,
    pub dxgi: IDXGIDevice4,
    pub d2d_factory: ID2D1Factory1,
    pub d2d: ID2D1Device,
    pub d2d_device_context: ID2D1DeviceContext, // グローバル共有DeviceContext
    pub dwrite_factory: IDWriteFactory2,
}

/// 描画の装置（D3D11・D2D・DirectWrite）をまとめた資源。
///
/// 下流への約束: 作り直しと装置の失いのとき、この資源の変化の印が立つ。装置に結びつく
/// 控えを持つ側は、変化を見たら控えを捨て、`is_valid()` の間だけ作り直す。
#[derive(Resource, Debug)]
pub struct GraphicsCore {
    inner: Option<GraphicsCoreInner>,
}

// SAFETY 条件: 保持する COM ポインタの跨スレッド移動（Send）・参照共有（Sync）の健全性は
// 各 API の同期特性に依拠する。d2d_factory は D2D1_FACTORY_TYPE_MULTI_THREADED で生成され、
// d2d はその工場の CreateDevice で作り、d2d_device_context はその d2d から作る。同じ工場に
// 属するオブジェクトへの COM 呼び出しは D2D が内部で直列化する。
// d3d は D3D11_CREATE_DEVICE_SINGLETHREADED を指定せず生成されるため
// ID3D11Device はスレッドセーフ（dxgi は同一オブジェクトへの別インターフェース）。
// dwrite_factory（DWRITE_FACTORY_TYPE_SHARED）もスレッドセーフ。
// よって全フィールドが内部同期を持ち、外部同期前提の DComp 系（dcomp_resource.rs）とは
// 異なり ECS スケジュール構成への追加前提を要しない。
unsafe impl Send for GraphicsCore {}
unsafe impl Sync for GraphicsCore {}

impl GraphicsCore {
    pub fn new() -> Result<Self> {
        info!("[GraphicsCore] Initialization started");

        let d3d = create_device_3d()?;
        let dxgi = d3d.cast()?;
        let d2d_factory = create_d2d_factory()?;
        // 公開する工場から装置を作る（D2D1CreateDevice は別の工場を新しく作ってしまう）。
        let d2d = unsafe { d2d_factory.CreateDevice(&dxgi)? };

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

    /// 描画の失敗を受け、装置が失われていれば自分を無効にして true を返す。
    ///
    /// 既に無効なら false（2 度目の記録を出さない）。失いでない失敗の記録は呼び出し元が残す。
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "描く係（タスク 4.5）が呼ぶまで本番の呼び出し元が無い"
        )
    )]
    pub(crate) fn note_draw_failure(&mut self, error: &windows::core::Error) -> bool {
        let Some(inner) = self.inner.as_ref() else {
            return false;
        };
        let removed_reason = match inner.d3d.get_device_removed_reason() {
            Ok(()) => S_OK,
            Err(e) => e.code(),
        };
        if !is_device_lost(error.code(), removed_reason) {
            return false;
        }
        error!(
            draw_error = ?error.code(),
            removed_reason = ?removed_reason,
            "[GraphicsCore] Device lost; invalidating"
        );
        self.invalidate();
        crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::GRAPHICS);
        true
    }

    pub fn is_valid(&self) -> bool {
        self.inner.is_some()
    }

    pub fn d2d_factory(&self) -> Option<&ID2D1Factory> {
        self.inner.as_ref().map(|i| &*i.d2d_factory)
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

/// 描画の失敗が装置の失いかを判じる（`removed_reason` は `GetDeviceRemovedReason()` の値）。
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "描く係（タスク 4.5）が呼ぶまで本番の呼び出し元が無い"
    )
)]
pub(crate) fn is_device_lost(draw_error: HRESULT, removed_reason: HRESULT) -> bool {
    removed_reason.is_err()
        || [
            DXGI_ERROR_DEVICE_REMOVED,
            DXGI_ERROR_DEVICE_RESET,
            DXGI_ERROR_DEVICE_HUNG,
            D2DERR_RECREATE_TARGET,
        ]
        .contains(&draw_error)
}

/// D2DFactoryを作成（マルチスレッド対応）
fn create_d2d_factory() -> Result<ID2D1Factory1> {
    unsafe { D2D1CreateFactory::<ID2D1Factory1>(D2D1_FACTORY_TYPE_MULTI_THREADED, None) }
}

fn create_device_3d() -> Result<ID3D11Device> {
    #[cfg(debug_assertions)]
    let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_DEBUG;

    #[cfg(not(debug_assertions))]
    let flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

    create_device_3d_with(flags, |flags| {
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
    })
}

/// 開発用の旗（D3D11 デバッグ層）が SDK の部品の無さで断られたら、旗を外して作り直す。
fn create_device_3d_with<T>(
    flags: D3D11_CREATE_DEVICE_FLAG,
    mut create: impl FnMut(D3D11_CREATE_DEVICE_FLAG) -> Result<T>,
) -> Result<T> {
    match create(flags) {
        Err(e)
            if flags.contains(D3D11_CREATE_DEVICE_DEBUG)
                && e.code() == DXGI_ERROR_SDK_COMPONENT_MISSING =>
        {
            warn!(
                "[GraphicsCore] D3D11 debug layer unavailable (Graphics Tools not installed); creating device without it"
            );
            create(D3D11_CREATE_DEVICE_FLAG(
                flags.0 & !D3D11_CREATE_DEVICE_DEBUG.0,
            ))
        }
        made => made,
    }
}

// ============================================================
// FrameTime - 高精度フレーム時刻リソース
// ============================================================

/// FrameTime - 高精度フレーム時刻リソース（f64 秒、OS起動時=0秒）
///
/// `FrameCount` と同じ設計パターン。ワールド外（`try_tick_world()` 実行前）で
/// `dola::runtime::clock::now()` を呼び出して値を更新する。
/// ワールド内の全システムは同じフレームの時刻値（`.0`）を参照する。
///
/// 利点:
/// - フレーム内でタイミングが完全に一貫
/// - テスト時に任意の時刻を注入可能
/// - システム実行中に時刻が進まない（安定性向上）
///
/// 時刻基準: `QueryPerformanceCounter / QueryPerformanceFrequency`
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct FrameTime(pub f64);

#[cfg(test)]
#[path = "core_tests.rs"]
mod core_tests;
