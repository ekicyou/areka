//! Shared helpers for visual tests

use windows::UI::Composition::Visual;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::{Interface, Result};
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

/// テスト用の GraphicsCore を作成するヘルパー関数
pub fn setup_graphics() -> Result<GraphicsCore> {
    GraphicsCore::new()
}

/// COM を MTA 初期化する（WucGraphicsResource::new は DQTAT_COM_NONE を使うため
/// COM 初期化済みスレッドを要求する）。冪等で、S_FALSE / RPC_E_CHANGED_MODE は無視する。
pub fn init_com_mta() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// WUC ビジュアルファクトリ。Compositor を保持し、テストで必要な
/// 実 WUC Visual（SpriteVisual を基底 Visual へ cast）を生成する。
///
/// `visual_hierarchy_sync_system` は親を `ContainerVisual` へ cast して `.Children()` を
/// 呼ぶため、生成する Visual は Children を持つ SpriteVisual である必要がある。
pub struct WucVisualFactory {
    pub resource: WucGraphicsResource,
}

impl WucVisualFactory {
    /// GraphicsCore の D2D デバイスから WucGraphicsResource を構築する。
    /// 呼び出し前に COM を MTA 初期化しておくこと（`init_com_mta`）。
    pub fn new(graphics: &GraphicsCore) -> Result<Self> {
        init_com_mta();
        let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
        let resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource作成失敗");
        Ok(Self { resource })
    }

    /// 新しい WUC Visual（SpriteVisual を基底 Visual へ cast）を生成する。
    pub fn create_visual(&self) -> Result<Visual> {
        let compositor = self
            .resource
            .compositor()
            .expect("compositor should exist");
        let v: Visual = compositor.CreateSpriteVisual()?.cast()?;
        Ok(v)
    }
}
