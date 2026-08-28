use crate::numerics::*;
use bevy_ecs::prelude::*;
use core::hash::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_core::Result;
use windows_core::*;
use windows_numerics::*;

#[derive(Component, Clone, Debug, PartialEq)]
pub enum VisualClip {
    Rect(Aabb),
    Geometry(ID2D1Geometry),
}

unsafe impl Send for VisualClip {}
unsafe impl Sync for VisualClip {}

impl Hash for VisualClip {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            VisualClip::Rect(r) => {
                0u8.hash(state);
                r.hash(state);
            }
            VisualClip::Geometry(g) => {
                1u8.hash(state);
                (g.as_raw() as usize).hash(state);
            }
        }
    }
}

impl VisualClip {
    pub fn new_rect(aabb: Aabb) -> Self {
        VisualClip::Rect(aabb)
    }

    pub fn new_geometry(geometry: ID2D1Geometry) -> Self {
        VisualClip::Geometry(geometry)
    }

    #[inline]
    pub fn rect(&self) -> Option<Aabb> {
        match self {
            VisualClip::Rect(r) => Some(*r),
            _ => None,
        }
    }

    #[inline]
    pub fn geometry(&self) -> Option<ID2D1Geometry> {
        match self {
            VisualClip::Geometry(g) => Some(g.clone()),
            _ => None,
        }
    }

    pub fn world_aabb(&self, world_transform: Matrix3x2) -> Result<Aabb> {
        let bounds = match self {
            VisualClip::Rect(r) => *r * world_transform,
            VisualClip::Geometry(g) => {
                let bounds = unsafe { g.GetBounds(Some(&world_transform)) }?;
                bounds.into()
            }
        };
        Ok(bounds)
    }
}
