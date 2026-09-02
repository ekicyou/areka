#![allow(dead_code)]

use crate::numerics::*;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::*;

#[derive(Clone, Debug)]
pub enum DrawCommand {
    Draw(DrawItem),
    PushClipRect(ClipRect),
    PopClipRect,
    PushClipGeometryEntity(ClipGeometryEntity),
    PushClipGeometryRect(ClipGeometryRect),
    PopClipGeometry,
}

#[derive(Clone, Debug)]
pub struct DrawItem {
    pub(crate) hash: u64,
    pub(crate) world_mat: Matrix3x2,
    pub(crate) entity: Entity,
    pub(crate) world_aabb: Aabb,
}

#[derive(Clone, Debug)]
pub struct ClipRect {
    pub(crate) world_aabb: Aabb,
}

#[derive(Clone, Debug)]
pub struct ClipGeometryEntity {
    pub(crate) world_mat: Matrix3x2,
    pub(crate) entity: Entity,
    pub(crate) world_aabb: Aabb,
}

#[derive(Clone, Debug)]
pub struct ClipGeometryRect {
    pub(crate) world_mat: Matrix3x2,
    pub(crate) geometry: ID2D1Geometry,
    pub(crate) world_aabb: Aabb,
}
