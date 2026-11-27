//! 計画を描画の木の節へ写す部品と、重ね方から D2D の合成モードへの写し。

use areka_emo_atlas::ElementId;
use areka_emo_compose::{DrawPlan, IRect, Mix, PlanItem, Transform};
use bevy_ecs::component::{Component, Mutable};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_COMPOSITE_MODE, D2D1_COMPOSITE_MODE_DESTINATION_IN, D2D1_COMPOSITE_MODE_DESTINATION_OVER,
    D2D1_COMPOSITE_MODE_SOURCE_ATOP, D2D1_COMPOSITE_MODE_SOURCE_COPY,
    D2D1_COMPOSITE_MODE_SOURCE_OVER,
};
use windows::Win32::Graphics::Direct2D::D2D1_INTERPOLATION_MODE_LINEAR;
use wintf::ecs::{
    Arrangement, ClipShape, HitTest, LayoutScale, MixMode, Offset, Size, Visual, VisualDrawContent,
    VisualGroup, VisualTransform,
};
use wintf::numerics::Aabb;

use crate::command::PresentError;
use crate::gpu_atlas::GpuAtlas;

/// 重ね方 → D2D の合成モード（写しはこの 1 か所）。
pub(crate) fn mix_mode_of(mix: Mix) -> D2D1_COMPOSITE_MODE {
    match mix {
        Mix::SourceOver => D2D1_COMPOSITE_MODE_SOURCE_OVER,
        Mix::SourceAtop => D2D1_COMPOSITE_MODE_SOURCE_ATOP,
        Mix::DestinationOver => D2D1_COMPOSITE_MODE_DESTINATION_OVER,
        Mix::Copy => D2D1_COMPOSITE_MODE_SOURCE_COPY,
        Mix::DestinationIn => D2D1_COMPOSITE_MODE_DESTINATION_IN,
    }
}

/// 計画を根の下の節へ写す。画面と画面の外が同じ写しを使う。
pub(crate) struct DrawNodes {
    root: Entity,
    slots: Vec<Slot>,
}

/// 計画の 1 項と、それを写した節（`clip` は範囲の切り抜きの節・`node` はその中の節）。
struct Slot {
    item: PlanItem,
    clip: Option<Entity>,
    node: Option<Entity>,
}

impl DrawNodes {
    pub(crate) fn new(root: Entity) -> Self {
        Self {
            root,
            slots: Vec::new(),
        }
    }

    /// 計画を根の下の節へ写す。同じ位置の項が等しければ節を変えない（版を上げない）。
    pub(crate) fn sync(
        &mut self,
        world: &mut World,
        gpu: &mut GpuAtlas,
        plan: &DrawPlan,
    ) -> Result<(), PresentError> {
        let keep = self
            .slots
            .iter()
            .zip(&plan.items)
            .take_while(|(slot, item)| shape(&slot.item) == shape(item))
            .count();
        // 後ろから消すので、まとめの子は親より先に消える。
        for slot in self.slots.drain(keep..).rev() {
            for e in [slot.node, slot.clip].into_iter().flatten() {
                let _ = world.try_despawn(e);
            }
        }
        let mut parents = vec![self.root];
        for (i, item) in plan.items.iter().enumerate() {
            if i < keep {
                if self.slots[i].item != *item {
                    update(world, gpu, &mut self.slots[i], item)?;
                }
            } else {
                let parent = *parents.last().expect("根は消えない");
                self.slots.push(spawn(world, gpu, parent, item)?);
            }
            match item {
                PlanItem::GroupBegin { .. } => {
                    parents.push(self.slots[i].node.expect("まとめは節を持つ"));
                }
                PlanItem::GroupEnd if parents.len() > 1 => {
                    parents.pop();
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// 節の形（種類と切り抜きの節の有無）。違えばそこから後ろを作り直す。
fn shape(item: &PlanItem) -> (u8, bool) {
    match item {
        PlanItem::Image { range, .. } => (0, range.is_some()),
        PlanItem::Fill { .. } => (1, false),
        PlanItem::GroupBegin { range, .. } => (2, range.is_some()),
        PlanItem::GroupEnd => (3, false),
    }
}

fn spawn(
    world: &mut World,
    gpu: &mut GpuAtlas,
    parent: Entity,
    item: &PlanItem,
) -> Result<Slot, PresentError> {
    let (clip, node) = match *item {
        PlanItem::Image {
            element,
            transform,
            mix,
            range,
        } => {
            let (content, place) = image_parts(gpu, element, transform, mix, range)?;
            let clip = range.map(|r| spawn_clip(world, parent, r));
            let node = world
                .spawn((
                    Visual::default(),
                    content,
                    place,
                    HitTest::none(),
                    ChildOf(clip.unwrap_or(parent)),
                ))
                .id();
            (clip, Some(node))
        }
        PlanItem::Fill { rect } => {
            let content = fill_content(gpu, rect)?;
            let node = world
                .spawn((
                    Visual::default(),
                    content,
                    rect_place(rect),
                    HitTest::none(),
                    ChildOf(parent),
                ))
                .id();
            (None, Some(node))
        }
        PlanItem::GroupBegin {
            transform,
            mix,
            range,
        } => {
            let clip = range.map(|r| spawn_clip(world, parent, r));
            let node = world
                .spawn((
                    Visual::default(),
                    group_of(mix),
                    group_place(transform, range),
                    HitTest::none(),
                    ChildOf(clip.unwrap_or(parent)),
                ))
                .id();
            (clip, Some(node))
        }
        PlanItem::GroupEnd => (None, None),
    };
    Ok(Slot {
        item: item.clone(),
        clip,
        node,
    })
}

/// 形の同じ項の中身だけを差し替える（等しい部品は書かないので版は上がらない）。
fn update(
    world: &mut World,
    gpu: &mut GpuAtlas,
    slot: &mut Slot,
    item: &PlanItem,
) -> Result<(), PresentError> {
    let node = slot.node;
    match *item {
        PlanItem::Image {
            element,
            transform,
            mix,
            range,
        } => {
            let (content, place) = image_parts(gpu, element, transform, mix, range)?;
            set(world, node, content);
            set(world, node, place);
        }
        PlanItem::Fill { rect } => {
            let same_size = matches!(slot.item, PlanItem::Fill { rect: old } if (old.w, old.h) == (rect.w, rect.h));
            if !same_size {
                let content = fill_content(gpu, rect)?;
                set(world, node, content);
            }
            set(world, node, rect_place(rect));
        }
        PlanItem::GroupBegin {
            transform,
            mix,
            range,
        } => {
            set(world, node, group_of(mix));
            set(world, node, group_place(transform, range));
        }
        PlanItem::GroupEnd => {}
    }
    let range = match *item {
        PlanItem::Image { range, .. } | PlanItem::GroupBegin { range, .. } => range,
        _ => None,
    };
    if let Some(r) = range {
        set(world, slot.clip, rect_place(r));
    }
    slot.item = item.clone();
    Ok(())
}

fn set<T: Component<Mutability = Mutable> + PartialEq>(
    world: &mut World,
    e: Option<Entity>,
    value: T,
) {
    if let Some(mut c) = e.and_then(|e| world.get_mut::<T>(e)) {
        c.set_if_neq(value);
    }
}

fn spawn_clip(world: &mut World, parent: Entity, range: IRect) -> Entity {
    world
        .spawn((
            Visual {
                clip: Some(ClipShape::Rectangle),
                ..Visual::default()
            },
            // 中身を持たない切り抜きを描画面の持ち主でなく節にするため。
            VisualTransform::default(),
            rect_place(range),
            HitTest::none(),
            ChildOf(parent),
        ))
        .id()
}

/// 絵の葉と置き場所。範囲が在れば位置は切り抜きの節の左上からの値。
fn image_parts(
    gpu: &mut GpuAtlas,
    element: ElementId,
    transform: Transform,
    mix: Mix,
    range: Option<IRect>,
) -> Result<(VisualDrawContent, Arrangement), PresentError> {
    let leaf = gpu.leaf(element)?.clone();
    let size = gpu.original(element);
    let (x, y) = local(transform, range);
    let content = VisualDrawContent::new_ex(
        Aabb::new(0.0, 0.0, size.w as f32, size.h as f32),
        leaf,
        D2D1_INTERPOLATION_MODE_LINEAR,
        mix_mode_of(mix),
    );
    Ok((content, place(x, y, size.w, size.h)))
}

fn fill_content(gpu: &mut GpuAtlas, rect: IRect) -> Result<VisualDrawContent, PresentError> {
    let list = gpu.black_fill(IRect { x: 0, y: 0, ..rect })?;
    Ok(VisualDrawContent::new_ex(
        Aabb::new(0.0, 0.0, rect.w as f32, rect.h as f32),
        list,
        D2D1_INTERPOLATION_MODE_LINEAR,
        D2D1_COMPOSITE_MODE_SOURCE_OVER,
    ))
}

fn group_of(mix: Mix) -> VisualGroup {
    VisualGroup {
        mix: MixMode::Composite(mix_mode_of(mix)),
    }
}

fn group_place(transform: Transform, range: Option<IRect>) -> Arrangement {
    let (x, y) = local(transform, range);
    let (w, h) = range.map_or((0, 0), |r| (r.w, r.h));
    place(x, y, w, h)
}

fn local(transform: Transform, range: Option<IRect>) -> (i64, i64) {
    let (x, y) = transform.offset();
    let (ox, oy) = range.map_or((0, 0), |r| (r.x, r.y));
    (x - ox, y - oy)
}

fn rect_place(r: IRect) -> Arrangement {
    place(r.x, r.y, r.w, r.h)
}

pub(crate) fn place(x: i64, y: i64, w: u32, h: u32) -> Arrangement {
    Arrangement {
        offset: Offset {
            x: x as f32,
            y: y as f32,
        },
        scale: LayoutScale::default(),
        size: Size {
            width: w as f32,
            height: h as f32,
        },
    }
}

#[cfg(test)]
#[path = "draw_nodes_tests.rs"]
mod tests;
