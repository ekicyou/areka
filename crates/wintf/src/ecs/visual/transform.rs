use crate::numerics::*;
use bevy_ecs::prelude::*;
use core::ops::*;

#[derive(Component, Clone, Copy, Debug, PartialEq, Default)]
#[repr(transparent)]
pub struct VisualTransform(pub Transform2D);

impl VisualTransform {
    #[inline]
    pub const fn new(t: Transform2D) -> Self {
        Self(t)
    }
}

impl Deref for VisualTransform {
    type Target = Transform2D;
    #[inline]
    fn deref(&self) -> &Transform2D {
        &self.0
    }
}
impl DerefMut for VisualTransform {
    #[inline]
    fn deref_mut(&mut self) -> &mut Transform2D {
        &mut self.0
    }
}

impl From<Transform2D> for VisualTransform {
    #[inline]
    fn from(t: Transform2D) -> Self {
        Self(t)
    }
}
impl From<VisualTransform> for Transform2D {
    #[inline]
    fn from(l: VisualTransform) -> Self {
        l.0
    }
}
