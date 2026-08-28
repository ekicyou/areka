use crate::numerics::*;
use bevy_ecs::prelude::*;
use core::hash::*;
#[derive(Component, Clone, Copy, Debug)]
pub struct WorldVisual {
    /// local → world の累積アフィン。
    pub world_mat: Matrix3x2,

    /// world_mat が軸平行か（M12 == 0 && M21 == 0）。
    /// `PushAxisAlignedClip` 可否 / `Aabb::transform` fast path のキャッシュ。
    pub axis_aligned: bool,

    /// 自コンテンツの world AABB（`DrawContent.local_aabb * world_mat`）。
    /// Draw 時の実効クリップ `top_clip & content_world_aabb` に使用。
    pub content_world_aabb: Aabb,

    /// 自 + 子孫の world AABB union（サブツリー・カリング／スクロール集計用）。
    pub subtree_world_aabb: Aabb,
}

impl Hash for WorldVisual {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.world_mat.calc_hash(state);
        self.axis_aligned.hash(state);
        self.content_world_aabb.hash(state);
        self.subtree_world_aabb.hash(state);
    }
}

impl WorldVisual {
    /// 未計算状態の初期値。DFS 前のプレースホルダとして使う。
    pub const PENDING: WorldVisual = WorldVisual {
        world_mat: Matrix3x2 {
            M11: 1.0,
            M12: 0.0,
            M21: 0.0,
            M22: 1.0,
            M31: 0.0,
            M32: 0.0,
        },
        axis_aligned: true,
        content_world_aabb: Aabb::EMPTY,
        subtree_world_aabb: Aabb::EMPTY,
    };

    /// world_mat から軸平行フラグを判定して構築する。
    /// content / subtree AABB は呼び出し側（DFS）で確定させる。
    #[inline]
    pub fn from_world_mat(world_mat: Matrix3x2) -> Self {
        Self {
            world_mat,
            axis_aligned: world_mat.axis_aligned(),
            content_world_aabb: Aabb::EMPTY,
            subtree_world_aabb: Aabb::EMPTY,
        }
    }
}

impl Default for WorldVisual {
    #[inline]
    fn default() -> Self {
        Self::PENDING
    }
}
