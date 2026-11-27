//! 画面の外の描き手（`OffscreenRenderer`）。使い捨ての描画の木に計画を写し、原寸の絵に描いて読む。

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{
    BindSet, ComposeError, ComposedSurface, Composer, DrawPlan, EmoWorld, PatternState,
};
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::ID2D1Device;
use windows_numerics::Matrix3x2;
use wintf::ecs::{GraphicsCore, Visual, render_tree_offscreen, warp_d2d_device};

use crate::command::{PresentError, device_err};
use crate::draw_nodes::{DrawNodes, place};
use crate::gpu_atlas::GpuAtlas;

/// 画面の外の描き手。`--offline-dump`・WARP のテスト・書き出しの共通の入口。
pub struct OffscreenRenderer {
    device: ID2D1Device,
    composer: Composer,
    gpu: Option<GpuAtlas>,
}

impl OffscreenRenderer {
    /// 画面と同じ hardware の装置で作る。
    pub fn hardware() -> Result<Self, PresentError> {
        Ok(Self::on(hardware_device()?))
    }

    /// WARP の装置（プロセスで 1 つ）で作る。
    pub fn warp() -> Result<Self, PresentError> {
        let device = warp_d2d_device().map_err(device_err("warp_d2d_device(offscreen)", None))?;
        Ok(Self::on(device))
    }

    fn on(device: ID2D1Device) -> Self {
        Self {
            device,
            composer: Composer::new(),
            gpu: None,
        }
    }

    /// 面 `surface_id` の計画を組み、原寸に描いた乗算済み BGRA を返す。
    pub fn render(
        &mut self,
        world: &EmoWorld,
        atlas: &AtlasTable,
        surface_id: u32,
        binds: &BindSet,
        pattern: &PatternState,
    ) -> Result<ComposedSurface, PresentError> {
        let plan = self
            .composer
            .plan(world, atlas, surface_id, binds, pattern)?;
        if !self.gpu.as_ref().is_some_and(|g| g.is_for(atlas)) {
            self.gpu = Some(GpuAtlas::new(&self.device, atlas)?);
        }
        let gpu = self.gpu.as_mut().expect("直前に用意した");
        Self::render_plan(gpu, &plan)
    }

    /// `gpu` の装置で計画を原寸に描く。描けなければ `ComposeError::Render`（`error!` は描く口が残す）。
    pub(crate) fn render_plan(
        gpu: &mut GpuAtlas,
        plan: &DrawPlan,
    ) -> Result<ComposedSurface, PresentError> {
        let (w, h) = (plan.extent.w, plan.extent.h);
        let mut world = World::new();
        let root = world.spawn((Visual::default(), place(0, 0, w, h))).id();
        DrawNodes::new(root).sync(&mut world, gpu, plan)?;
        let device = gpu.device().clone();
        let image = render_tree_offscreen(&device, &mut world, root, (w, h), Matrix3x2::identity())
            .map_err(|e| ComposeError::Render {
                surface: plan.surface_id,
                reason: e.to_string(),
            })?;
        Ok(ComposedSurface::from_bytes(
            image.width,
            image.height,
            image.bytes,
        ))
    }
}

/// 画面と同じ hardware の装置を新しく作る。
pub(crate) fn hardware_device() -> Result<ID2D1Device, PresentError> {
    let core = GraphicsCore::new().map_err(device_err("GraphicsCore::new(offscreen)", None))?;
    Ok(core
        .d2d_device()
        .cloned()
        .expect("作ったばかりの GraphicsCore は装置を持つ"))
}

#[cfg(test)]
#[path = "offscreen_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "draw_methods_warp_tests.rs"]
mod draw_methods_warp_tests;

#[cfg(test)]
#[path = "offscreen_emo2_tests.rs"]
mod emo2_tests;
