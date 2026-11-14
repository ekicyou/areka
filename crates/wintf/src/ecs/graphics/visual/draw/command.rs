//! 平らにした描く列（画家の順）。矩形と行列はすべて根からの DIP。
//! 欄は `wintf-visual-damage-redraw` が読むので公開にする。

use super::super::MixMode;
use crate::numerics::*;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::*;

/// 描く列の 1 命令。push と pop は必ず対で、入れ子になる。同じ節では外から不透明度の層 → 切り抜き。
///
/// 切り抜きの内側の重ね方とブレンドは、矩形・角丸・回転した矩形・任意の図形のどの切り抜きでも、
/// 切り抜きが無いときと同じく下の画素に効き、切り抜きの外側の画素は変わらない（3.1〜3.3）。
/// 不透明度の層の内側では、同じ層の中で先に描いた画素に効き、層の外の下の画素は層を重ねるときの
/// 不透明度でだけ変わる（3.4）。
#[derive(Clone, Debug)]
pub enum DrawCommand {
    Draw(DrawItem),
    PushClipRect(ClipRect),
    PopClipRect,
    /// 単位矩形・角丸の path・使い手の図形のどれも「図形＋局所→世界の変換」で同じ形。
    /// 層は背景の写しから始める（中の重ね方とブレンドが下の画素に効く）。
    PushClipGeometry(ClipGeometry),
    PopClipGeometry,
    /// 節と子孫をまとめた 1 枚に不透明度を掛けて重ねる層（透明から始める）。
    PushOpacity(OpacityLayer),
    PopOpacity,
}

/// 描く 1 件（節の描く中身を世界の行列で置いたもの）。
#[derive(Clone, Debug)]
pub struct DrawItem {
    /// 同一性。COM の番地に依らない値から取る（4.5）。
    pub hash: u64,
    pub entity: Entity,
    /// 節の `VisualVersion`（無ければ 0）。平行移動を除いた鍵に使う。
    pub version: u64,
    /// 根からの DIP。
    pub world_mat: Matrix3x2,
    /// DIP。
    pub world_aabb: Aabb,
    /// `VisualDrawContent` の命令の列の複製（AddRef だけ）。中身を差し替えても次の組み直しまで生きる。
    pub command_list: ID2D1CommandList,
    pub interpolation_mode: D2D1_INTERPOLATION_MODE,
    pub mix_mode: MixMode,
}

/// 軸に沿う矩形の切り抜き。
#[derive(Clone, Debug)]
pub struct ClipRect {
    /// 層を積んだ節。
    pub entity: Entity,
    /// その節の `VisualVersion`（無ければ 0）。
    pub version: u64,
    /// DIP。
    pub world_aabb: Aabb,
}

/// 図形の切り抜き。
#[derive(Clone, Debug)]
pub struct ClipGeometry {
    /// 層を積んだ節。
    pub entity: Entity,
    /// その節の `VisualVersion`（無ければ 0）。
    pub version: u64,
    /// 図形の局所→世界（根からの DIP）の変換。
    pub world_mat: Matrix3x2,
    pub geometry: ID2D1Geometry,
    /// 図形を `world_mat` で置いた外接矩形（DIP）。
    pub world_aabb: Aabb,
}

/// 不透明度の層。
#[derive(Clone, Debug)]
pub struct OpacityLayer {
    /// 層を積んだ節。
    pub entity: Entity,
    /// その節の `VisualVersion`（無ければ 0）。
    pub version: u64,
    /// `Visual::clamped_opacity` の値（0 より大きく 1.0 未満）。
    pub opacity: f32,
    /// 層の中身の範囲（根からの DIP）。節の部分木の世界の外接矩形と、外側の有効な矩形の交わり。
    pub world_aabb: Aabb,
}
