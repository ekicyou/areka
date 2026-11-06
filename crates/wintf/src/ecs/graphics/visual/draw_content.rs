use crate::numerics::*;
use bevy_ecs::prelude::*;
use core::hash::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::core::Result;
use windows_core::*;

/// 描画面を持たない節の描く中身（閉じた命令の列・局所の外接矩形〔96 DPI の DIP〕・補間・重ね方）。
///
/// 持てば節になり、世界の行列（`WorldVisual`）と見た目の版（`VisualVersion`）を必ず連れてくる。
///
/// 重ね方（`composite_mode`）が `SOURCE_OVER` でないとき、D2D の `DrawImage` は絵を今の軸に沿う
/// 切り抜きまで透明で広げてから合成する（`DESTINATION_IN` などは絵の外の画素にも効く）。
/// wintf は D2D の値をそのまま通す。
#[derive(Component, Clone, Debug, PartialEq)]
#[require(super::WorldVisual, super::VisualVersion)]
pub struct VisualDrawContent {
    pub local_aabb: Aabb,
    pub command_list: ID2D1CommandList,
    pub interpolation_mode: D2D1_INTERPOLATION_MODE,
    pub composite_mode: D2D1_COMPOSITE_MODE,
}

// SAFETY 条件: windows-rs の COM スマートポインタは自動では Send/Sync にならない。
// `ID2D1CommandList` は GraphicsCore の装置から作り、その装置は D2D1_FACTORY_TYPE_MULTI_THREADED の
// 工場（GraphicsCore::new → create_d2d_factory）の CreateDevice で作られる。同じ工場の系列への COM
// 呼び出しは D2D が内部で直列化するため、跨スレッド移動・参照共有とも安全。別の装置で作った命令の列を
// 入れる道は型では閉じられないので、「GraphicsCore の装置で作り、閉じてから入れる」を使い手の約束とする
// （command_list.rs と同じ根拠）。
unsafe impl Send for VisualDrawContent {}
unsafe impl Sync for VisualDrawContent {}

impl Hash for VisualDrawContent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.local_aabb.hash(state);
        (self.command_list.as_raw() as usize).hash(state);
        self.interpolation_mode.0.hash(state);
        self.composite_mode.0.hash(state);
    }
}

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
    /// - `composite_mode`: 描画時の合成モード。
    ///
    /// バウンズが既知の場合はこちらを使う。未知で DC から求めたい場合は
    /// [`Self::new_calc_aabb`] を使用する。
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
            composite_mode,
        }
    }

    /// `command_list` のローカル AABB を DC から算出して `VisualDrawContent` を構築する。
    ///
    /// [`ID2D1DeviceContext::GetImageLocalBounds`] は world transform を含まない
    /// ローカル空間のバウンズを返すが、その値は **DC の現在の DPI・UnitMode・
    /// InterpolationMode を反映する**。したがって `local_aabb` を論理単位
    /// （96DPI）として保持したい場合、渡す `dc` は
    /// **DPI=96 / `D2D1_UNIT_MODE_DIPS`** に設定されていなければならない。
    /// 画面 DPI や UnitMode=Pixels の DC を渡すと、得られる AABB がその DPI に
    /// スケールされてしまい、モニタ跨ぎで再計算が必要になる。
    ///
    /// 実運用では描画本番 DC とは別に、DPI=96/DIPs へ固定した
    /// 「バウンズ計算専用 DC」を渡すことを推奨する。
    ///
    /// # 引数
    /// - `command_list`: 対象の描画コマンド列。**[`ID2D1CommandList::Close`]
    ///   済みであること**。未クローズだと正しいバウンズが得られない。
    /// - `interpolation_mode`: 描画時の補間モード。
    /// - `composite_mode`: 描画時の合成モード。
    /// - `dc`: バウンズ算出に用いるデバイスコンテキスト。上記のとおり
    ///   DPI=96 / UnitMode=DIPs を満たすこと。`command_list` と同一
    ///   `ID2D1Device` から生成された DC である必要がある。
    ///
    /// # 戻り値
    /// 算出した論理単位の `local_aabb` を持つ `VisualDrawContent`。
    /// `GetImageLocalBounds` が失敗した場合はその [`windows::core::Error`] を返す。
    ///
    /// # Safety / 実装メモ
    /// `&ID2D1CommandList` から `Param<ID2D1Image>` への変換は `QUERY = false`
    /// （ポインタ再解釈のみ）で解決され、`QueryInterface` は発生しない。
    /// `unsafe` は COM 呼び出しのためで、`dc` が有効な参照であり `command_list`
    /// が Close 済みであることが前提。
    #[inline]
    pub fn new_calc_aabb_ex(
        command_list: ID2D1CommandList,
        interpolation_mode: D2D1_INTERPOLATION_MODE,
        composite_mode: D2D1_COMPOSITE_MODE,
        dc: &ID2D1DeviceContext,
    ) -> Result<Self> {
        let local_aabb = unsafe { dc.GetImageLocalBounds(&command_list) }?.into();
        Ok(Self::new_ex(
            local_aabb,
            command_list,
            interpolation_mode,
            composite_mode,
        ))
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

    #[inline]
    pub fn new_calc_aabb(command_list: ID2D1CommandList, dc: &ID2D1DeviceContext) -> Result<Self> {
        Self::new_calc_aabb_ex(
            command_list,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_COMPOSITE_MODE_SOURCE_OVER,
            dc,
        )
    }
}
