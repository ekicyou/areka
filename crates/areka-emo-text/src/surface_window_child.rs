//! # surface_window_child — 箱の文字の面（シェルの窓の直接の子）の装着・片付け・当たりのマスク
//!
//! `surface.rs` の子（`#[path]`）。供給面の生成は親の `TextSurface::create` を共有し、ここは
//! 「窓の子として挿す・消す」と「表示されている字の矩形で当たりのマスクを焼く」（要件 9.4）だけを持つ。

use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use windows::UI::Composition::Compositor;
use wintf::ecs::widget::bitmap_source::AlphaMask;
use wintf::ecs::{AlphaMaskResource, GraphicsCore, HitTest, Visual, VisualGraphics};

use super::{TextSurface, physical_arrangement};
use crate::TextLayerError;
use crate::choice::HitRectPx;

/// 字の矩形の集まり（面の左上を原点とする物理 px）を、面の大きさ `size` の当たりのマスクへ焼く
/// （要件 9.4）。矩形 `[left, right) × [top, bottom)` を外側へ丸めた画素が内で、面の外は切る。
/// マスクは面と同じ物理寸なので、wintf の bounds 相対の比例写像は 1:1 になる。
pub(crate) fn hit_cells_mask(size: (u32, u32), cells: &[HitRectPx]) -> AlphaMask {
    let (w, h) = size;
    let mut px = vec![0u8; w as usize * h as usize * 4];
    let span = |lo: f32, hi: f32, len: u32| {
        let clip = |v: f32| v.clamp(0.0, len as f32) as usize;
        clip(lo.floor())..clip(hi.ceil())
    };
    for c in cells {
        for y in span(c.top, c.bottom, h) {
            for x in span(c.left, c.right, w) {
                px[(y * w as usize + x) * 4 + 3] = 255;
            }
        }
    }
    AlphaMask::from_pbgra32(&px, w, h, w * 4)
}

/// 箱の文字の面の entity が装着の時に持つ当たり判定: マスク判定で、字の矩形は 0 個（何も受けない）。
/// マスクを空のまま（`AlphaMaskResource::new()`）にすると wintf は矩形全体の判定へ縮退するので、
/// 必ず面の大きさの空のマスクを入れる。
pub(crate) fn box_hit_components(size: (u32, u32)) -> (HitTest, AlphaMaskResource) {
    let mut mask = AlphaMaskResource::new();
    mask.set(hit_cells_mask(size, &[]));
    (HitTest::alpha_mask(), mask)
}

/// 箱の文字の面の entity の当たりのマスクを、字の矩形の集まりへ差し替える（要件 9.4）。
/// クリック透過の切り替えと窓のメッセージの振り分けは、どちらもこのマスクを読む。
pub(crate) fn set_child_hit_cells(
    world: &mut World,
    entity: Entity,
    size: (u32, u32),
    cells: &[HitRectPx],
) {
    match world.get_mut::<AlphaMaskResource>(entity) {
        Some(mut res) => res.set(hit_cells_mask(size, cells)),
        None => tracing::warn!(
            ?entity,
            "箱の文字の面の entity に当たりのマスクが無い——字の矩形でクリックを受けられない"
        ),
    }
}

impl TextSurface {
    /// 箱の文字の面の装着（UI スレッド・`&mut World`）: 供給面を [`Self::attach`] と同じ手順で作り、
    /// **シェルの窓の直接の子**の entity（`Visual`＋`VisualGraphics`＋物理 px の `Arrangement`＋
    /// 字の矩形のマスクの当たり判定）として窓の `Children` の `index` 番目へ挿す（`index` が子の数を
    /// 超えれば末尾）。
    ///
    /// 当たり判定は表示されている字の矩形の集まりで、装着の時は 0 個（何も受けない）。提示が
    /// [`Self::set_hit_cells`] で入れ替える。届くかどうかはシェルの絵とこの矩形で決まる（要件 9.4）。
    /// 面は透明で始まり、背景の絵を描かない（要件 1.6）。作った entity は面が持ち、
    /// [`Self::despawn_window_child`] で消す。
    pub fn attach_window_child(
        world: &mut World,
        window: Entity,
        index: usize,
        compositor: &Compositor,
        core: &GraphicsCore,
        physical_size: (u32, u32),
        physical_offset: (f32, f32),
    ) -> Result<TextSurface, TextLayerError> {
        if world.get_entity(window).is_err() {
            tracing::error!(
                ?window,
                "シェルの窓の entity が World に存在しない（箱の文字の面を装着できない）"
            );
            return Err(TextLayerError::Device {
                hresult: 0,
                context: "shell window entity not found",
            });
        }
        let (mut surface, wuc_visual) = Self::create(compositor, core, physical_size)?;
        // 既に子である entity の並べ替え（bevy の `place`）は範囲外の位置で panic するので、
        // 挿す前の子の数で切り詰める（超えれば末尾）。
        let index = index.min(world.get::<Children>(window).map_or(0, |c| c.len()));
        let child = world
            .spawn((
                Name::new("emo-text-box-surface"),
                Visual::default(),
                VisualGraphics::new(wuc_visual),
                physical_arrangement(physical_size, physical_offset),
                box_hit_components(physical_size),
                ChildOf(window),
            ))
            .id();
        world.entity_mut(window).insert_child(index, child);
        world.flush();
        surface.window_child = Some(child);
        Ok(surface)
    }

    /// 箱の文字の面の片付け: 窓の子の entity を消す（`ChildOf` により窓の `Children` からも外れる）。
    /// 差し込み口へ装着した面（窓の子を持たない）に呼ぶのは呼び手の誤りで、`warn!` して何も消さない
    /// （差し込み口の寿命は emo-present の領分）。
    pub fn despawn_window_child(self, world: &mut World) {
        let Some(entity) = self.window_child else {
            tracing::warn!("despawn_window_child: 窓の子を持たない面（差し込み口の面）は消さない");
            return;
        };
        if !world.despawn(entity) {
            tracing::warn!(
                ?entity,
                "despawn_window_child: 箱の文字の面の entity が既に居ない"
            );
        }
    }

    /// 箱の文字の面の当たり判定を、表示されている字の矩形の集まり（面の左上を原点とする物理 px）へ
    /// 合わせる（要件 9.4）。前に焼いた集まりと同じなら何もしない。差し込み口へ装着した面（窓の子を
    /// 持たない）は何もしない（普通のバルーンの窓の当たり判定は emo-present の領分）。
    pub fn set_hit_cells(&mut self, world: &mut World, cells: Vec<HitRectPx>) {
        let Some(entity) = self.window_child else {
            return;
        };
        if self.hit_cells == cells {
            return;
        }
        set_child_hit_cells(world, entity, self.size, &cells);
        self.hit_cells = cells;
    }

    /// 窓の子として装着した面の entity（差し込み口へ装着した面は `None`）。
    pub fn window_child(&self) -> Option<Entity> {
        self.window_child
    }
}

#[cfg(test)]
#[path = "surface_window_child_tests.rs"]
mod window_child_tests;

#[cfg(test)]
#[path = "surface_hit_cells_tests.rs"]
mod hit_cells_tests;
