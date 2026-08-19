use bevy_ecs::prelude::*;
use bitflags::*;

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct VisualFlagBits: u32 {
        /// 非表示なら描画／カリングでスキップ。
        const VISIBLE = 1 << 0;
        // 将来の見た目系（OPACITY_GROUP / CACHE_AS_BITMAP 等）の余地を残す。
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct VisualFlags(pub VisualFlagBits);

impl VisualFlags {
    #[inline]
    pub fn is_visible(&self) -> bool {
        self.0.contains(VisualFlagBits::VISIBLE)
    }

    #[inline]
    pub fn set_is_visible(&mut self, value: bool) {
        self.0.set(VisualFlagBits::VISIBLE, value);
    }
}

impl Default for VisualFlags {
    /// 既定は可視。
    #[inline]
    fn default() -> Self {
        Self(VisualFlagBits::VISIBLE)
    }
}
