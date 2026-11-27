//! まとめの節（子孫を透明から始まる 1 枚に描き、閉じるとき重ね方つきで親へ重ねる）。

use super::{MixMode, VisualVersion, WorldVisual};
use bevy_ecs::prelude::*;

/// まとめの節。節と子孫を透明から始まる 1 枚に描き、閉じるとき節の変換を当てて `mix` で親の画素へ重ねる。
///
/// - 中の重ね方とブレンドは、同じまとめの中で先に描いた画素にだけ効く。
/// - 中は節の局所の座標（節の変換の前・1 DIP＝1 画素）の解像度で描く。
/// - 同じ節では外から、不透明度の層 → まとめ → 切り抜き。
/// - `mix` が `SOURCE_OVER` でない重ね方なら、1 枚は今の軸に沿う切り抜きまで透明で広がって重なる
///   （描く中身の件と同じ）。
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[require(WorldVisual, VisualVersion)]
pub struct VisualGroup {
    pub mix: MixMode,
}
