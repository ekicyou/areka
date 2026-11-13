use crate::numerics::*;
use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;

/// 描く中身 1 つの混ぜ方。重ね方とブレンドを同時には持てない（1.5）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixMode {
    /// `DrawImage` の重ね方（13 種）。既定は `SOURCE_OVER`。
    Composite(D2D1_COMPOSITE_MODE),
    /// `ID2D1DeviceContext6::BlendImage` のブレンド（26 種・`MULTIPLY`〜`DIVISION`）。
    Blend(D2D1_BLEND_MODE),
}

impl Default for MixMode {
    fn default() -> Self {
        Self::Composite(D2D1_COMPOSITE_MODE_SOURCE_OVER)
    }
}

/// 描画面を持たない節の描く中身（閉じた命令の列・局所の外接矩形〔96 DPI の DIP〕・補間・混ぜ方）。
///
/// 持てば節になり、世界の行列（`WorldVisual`）と見た目の版（`VisualVersion`）を必ず連れてくる。
///
/// 重ね方（`MixMode::Composite`）が `SOURCE_OVER` でないとき、D2D の `DrawImage` は絵を今の軸に沿う
/// 切り抜きまで透明で広げてから合成する（`DESTINATION_IN` などは絵の外の画素にも効く）。
/// wintf は D2D の値をそのまま通す。
#[derive(Component, Clone, Debug, PartialEq)]
#[require(super::WorldVisual, super::VisualVersion)]
pub struct VisualDrawContent {
    pub local_aabb: Aabb,
    pub command_list: ID2D1CommandList,
    pub interpolation_mode: D2D1_INTERPOLATION_MODE,
    pub mix_mode: MixMode,
}

// SAFETY 条件: windows-rs の COM スマートポインタは自動では Send/Sync にならない。
// `ID2D1CommandList` は GraphicsCore の装置から作り、その装置は D2D1_FACTORY_TYPE_MULTI_THREADED の
// 工場（GraphicsCore::new → create_d2d_factory）の CreateDevice で作られる。同じ工場の系列への COM
// 呼び出しは D2D が内部で直列化するため、跨スレッド移動・参照共有とも安全。別の装置で作った命令の列を
// 入れる道は型では閉じられないので、「GraphicsCore の装置で作り、閉じてから入れる」を使い手の約束とする
// （command_list.rs と同じ根拠）。
unsafe impl Send for VisualDrawContent {}
unsafe impl Sync for VisualDrawContent {}

impl VisualDrawContent {
    /// 事前に計算済みの `local_aabb` を用いて `VisualDrawContent` を構築する。
    ///
    /// # 引数
    /// - `local_aabb`: コンテンツがローカル座標で占める AABB。
    ///   **論理単位（96DPI）** で与えること。DPI に依存した値を
    ///   入れると、モニタ跨ぎや DPI 変更時に内在プロパティとしての不変性が
    ///   崩れるため注意。
    /// - `command_list`: このコンテンツの描画コマンド列。呼び出し側で
    ///   [`ID2D1CommandList::Close`] 済みであることを想定する。
    /// - `interpolation_mode`: 描画時の補間モード。
    /// - `composite_mode`: 描画時の合成モード（`MixMode::Composite` に包む）。
    #[inline]
    pub fn new_ex(
        local_aabb: Aabb,
        command_list: ID2D1CommandList,
        interpolation_mode: D2D1_INTERPOLATION_MODE,
        composite_mode: D2D1_COMPOSITE_MODE,
    ) -> Self {
        Self {
            local_aabb,
            command_list,
            interpolation_mode,
            mix_mode: MixMode::Composite(composite_mode),
        }
    }

    /// 混ぜ方をブレンド `blend_mode` にして構築する（引数は `new_ex` と同じ約束）。
    #[inline]
    pub fn new_blend(
        local_aabb: Aabb,
        command_list: ID2D1CommandList,
        interpolation_mode: D2D1_INTERPOLATION_MODE,
        blend_mode: D2D1_BLEND_MODE,
    ) -> Self {
        Self {
            local_aabb,
            command_list,
            interpolation_mode,
            mix_mode: MixMode::Blend(blend_mode),
        }
    }

    #[inline]
    pub fn new(local_aabb: Aabb, command_list: ID2D1CommandList) -> Self {
        Self::new_ex(
            local_aabb,
            command_list,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_COMPOSITE_MODE_SOURCE_OVER,
        )
    }
}
