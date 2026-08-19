use crate::numerics::*;
use bevy_ecs::prelude::*;
use core::ops::*;

#[derive(Component, Clone, Copy, Debug, PartialEq, Default)]
#[repr(transparent)]
pub struct LocalTransform(pub Transform2D);

impl LocalTransform {
    #[inline]
    pub const fn new(t: Transform2D) -> Self {
        Self(t)
    }
}

impl Deref for LocalTransform {
    type Target = Transform2D;
    #[inline]
    fn deref(&self) -> &Transform2D {
        &self.0
    }
}
impl DerefMut for LocalTransform {
    #[inline]
    fn deref_mut(&mut self) -> &mut Transform2D {
        &mut self.0
    }
}

impl From<Transform2D> for LocalTransform {
    #[inline]
    fn from(t: Transform2D) -> Self {
        Self(t)
    }
}
impl From<LocalTransform> for Transform2D {
    #[inline]
    fn from(l: LocalTransform) -> Self {
        l.0
    }
}
