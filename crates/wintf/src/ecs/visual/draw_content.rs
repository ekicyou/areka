use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::ID2D1CommandList;

use crate::numerics::*;

#[derive(Component, Clone, Debug, PartialEq)]
pub struct DrawContent {
    pub local_aabb: Aabb,
    pub command_list: ID2D1CommandList,
}

impl DrawContent {
    #[inline]
    pub fn new(local_aabb: Aabb, command_list: ID2D1CommandList) -> Self {
        Self {
            local_aabb,
            command_list,
        }
    }
}
