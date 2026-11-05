//! Shared helpers for visual tests
//!
//! Compositor を作るテストの本体は `crate::gpu::on_gpu_owner_thread` の中で回す
//! （COM の MTA の初期化と常駐の DispatcherQueue は持ち主のスレッドが済ませている）。

use windows::UI::Composition::{ContainerVisual, Visual};
use windows::core::{Interface, Result};
use wintf::ecs::{GraphicsCore, WucGraphicsResource};

/// テスト用の GraphicsCore を作成するヘルパー関数
pub fn setup_graphics() -> Result<GraphicsCore> {
    GraphicsCore::new()
}

/// WUC: ContainerVisual.Children().InsertAtBottom で子をつなぐ。
pub fn add_child(parent: &Visual, child: &Visual) -> Result<()> {
    parent
        .cast::<ContainerVisual>()?
        .Children()?
        .InsertAtBottom(child)?;
    Ok(())
}

/// WUC ビジュアルファクトリ。テストごとに新規 `WucGraphicsResource` を保持し、
/// そこから実 WUC `Visual`（`SpriteVisual` を基底 `Visual` へ cast）を供給する。
pub struct WucVisualFactory {
    resource: WucGraphicsResource,
}

impl WucVisualFactory {
    /// 引数の `GraphicsCore` の D2D の装置から `WucGraphicsResource` を構築する。
    pub fn new(graphics: &GraphicsCore) -> Result<Self> {
        let d2d = graphics.d2d_device().expect("D2Dデバイスが無効");
        let resource = WucGraphicsResource::new(d2d)?;
        Ok(Self { resource })
    }

    /// 新しい WUC Visual（`SpriteVisual` を基底 `Visual` へ cast）を生成する。
    pub fn create_visual(&self) -> Result<Visual> {
        let compositor = self.resource.compositor().expect("compositor should exist");
        let v: Visual = compositor.CreateSpriteVisual()?.cast()?;
        Ok(v)
    }
}
