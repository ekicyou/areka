use crate::numerics::*;
use bevy_ecs::prelude::*;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum Clip {
    Rect(Aabb),
}

impl Clip {
    #[inline]
    pub fn local_aabb(&self) -> Aabb {
        match self {
            Clip::Rect(r) => *r,
        }
    }
}
