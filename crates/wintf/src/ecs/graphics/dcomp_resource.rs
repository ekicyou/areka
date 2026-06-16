//! DCompGraphicsResource — DComp COM デバイスの遅延初期化管理
//!
//! GraphicsCore とは独立した ECS Resource として DComp デバイスを管理する。
//! DComp モードのウィンドウが初めて必要になった時点で初期化（遅延初期化）。

use bevy_ecs::prelude::*;
use tracing::{debug, info};
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::core::Result;

use crate::com::dcomp::dcomp_create_desktop_device;

/// DComp COM デバイスの内部構造体
struct DCompGraphicsResourceInner {
    desktop: IDCompositionDesktopDevice,
    dcomp: IDCompositionDevice3,
}

/// DComp COM デバイス（IDCompositionDesktopDevice, IDCompositionDevice3）を
/// 遅延初期化し、DComp パイプライン専用リソースとして管理する ECS Resource。
///
/// `Option<Inner>` パターンにより、`invalidate()` でリソースを無効化し、
/// 次フレームで再初期化可能。
#[derive(Resource)]
pub struct DCompGraphicsResource {
    inner: Option<DCompGraphicsResourceInner>,
}

// SAFETY 条件: 保持する COM ポインタの跨スレッド移動（Send）・参照共有（Sync）は
// 「同一オブジェクトへの COM 呼び出しが同時に実行されない」ことを前提とする。
// DComp デバイス（desktop / dcomp）は D2D（MULTI_THREADED ファクトリ系列）と異なり
// 内部同期を持たない外部同期前提の API であり、ECS スケジュール構成（Res/ResMut の
// 借用規則と、同一リソースへ並行アクセスしないシステム配置）が安全性条件を担う
// （components.rs の DComp 系コンポーネント 3 種と同一条件）。
unsafe impl Send for DCompGraphicsResource {}
unsafe impl Sync for DCompGraphicsResource {}

impl std::fmt::Debug for DCompGraphicsResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DCompGraphicsResource")
            .field("is_valid", &self.is_valid())
            .finish()
    }
}

impl DCompGraphicsResource {
    /// DComp デバイスを初期化する。GraphicsCore の D2D デバイスに依存。
    ///
    /// # Arguments
    /// * `d2d_device` - D2D デバイス参照（GraphicsCore から取得）
    ///
    /// # Errors
    /// COM デバイス初期化に失敗した場合
    pub fn new(d2d_device: &ID2D1Device) -> Result<Self> {
        info!("[DCompGraphicsResource] Initialization started");

        let desktop = dcomp_create_desktop_device(d2d_device)?;
        let dcomp: IDCompositionDevice3 = windows::core::Interface::cast(&desktop)?;

        info!("[DCompGraphicsResource] Initialization completed");

        Ok(Self {
            inner: Some(DCompGraphicsResourceInner { desktop, dcomp }),
        })
    }

    /// 未初期化状態の DCompGraphicsResource を作成する。
    ///
    /// テスト用。`is_valid()` は `false` を返す。
    pub fn new_empty() -> Self {
        Self { inner: None }
    }

    /// 全 DComp COM リソースを無効化する。
    ///
    /// デバイスロスト時に `invalidate_dependent_components` から呼ばれる。
    /// 次フレームの `init_window_graphics` で再初期化される。
    pub fn invalidate(&mut self) {
        if self.inner.is_some() {
            debug!("[DCompGraphicsResource] Invalidated");
        }
        self.inner = None;
    }

    /// DComp デバイスが有効かどうかを返す。
    pub fn is_valid(&self) -> bool {
        self.inner.is_some()
    }

    /// IDCompositionDevice3 への Option アクセサ
    pub fn dcomp(&self) -> Option<&IDCompositionDevice3> {
        self.inner.as_ref().map(|i| &i.dcomp)
    }

    /// IDCompositionDesktopDevice への Option アクセサ
    pub fn desktop(&self) -> Option<&IDCompositionDesktopDevice> {
        self.inner.as_ref().map(|i| &i.desktop)
    }
}
