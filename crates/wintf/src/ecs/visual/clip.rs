use crate::numerics::*;
use bevy_ecs::prelude::*;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum VisualClip {
    Rect(Aabb),
}

impl VisualClip {
    #[inline]
    pub fn local_aabb(&self) -> Aabb {
        match self {
            VisualClip::Rect(r) => *r,
        }
    }
}
