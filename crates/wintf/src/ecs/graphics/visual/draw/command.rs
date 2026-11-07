//! 平らにした描く列（画家の順）。矩形と行列はすべて根からの DIP。
//! 欄は `wintf-visual-damage-redraw` が読むので公開にする。

use crate::numerics::*;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::*;

/// 描く列の 1 命令。push と pop は必ず対で、入れ子になる。
#[derive(Clone, Debug)]
pub enum DrawCommand {
    Draw(DrawItem),
    PushClipRect(ClipRect),
    PopClipRect,
    /// 単位矩形・角丸の path・使い手の図形のどれも「図形＋局所→世界の変換」で同じ形。
    PushClipGeometry(ClipGeometry),
    PopClipGeometry,
}

/// 描く 1 件（節の描く中身を世界の行列で置いたもの）。
#[derive(Clone, Debug)]
pub struct DrawItem {
    /// 同一性。COM の番地に依らない値から取る（4.5）。
    pub hash: u64,
    pub entity: Entity,
    /// 根からの DIP。
    pub world_mat: Matrix3x2,
    /// DIP。
    pub world_aabb: Aabb,
    /// `VisualDrawContent` の命令の列の複製（AddRef だけ）。中身を差し替えても次の組み直しまで生きる。
    pub command_list: ID2D1CommandList,
    pub interpolation_mode: D2D1_INTERPOLATION_MODE,
    pub composite_mode: D2D1_COMPOSITE_MODE,
}

/// 軸に沿う矩形の切り抜き。
#[derive(Clone, Debug)]
pub struct ClipRect {
    /// DIP。
    pub world_aabb: Aabb,
}

/// 図形の切り抜き。
#[derive(Clone, Debug)]
pub struct ClipGeometry {
    /// 図形の局所→世界（根からの DIP）の変換。
    pub world_mat: Matrix3x2,
    pub geometry: ID2D1Geometry,
    /// 図形を `world_mat` で置いた外接矩形（DIP）。
    pub world_aabb: Aabb,
}
