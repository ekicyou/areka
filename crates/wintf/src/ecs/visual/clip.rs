use crate::numerics::*;
use bevy_ecs::prelude::*;
use core::hash::*;
use windows::Win32::Graphics::Direct2D::ID2D1Geometry;
use windows_core::*;

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
}
