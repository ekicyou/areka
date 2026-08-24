use bevy_ecs::prelude::*;
use windows::Win32::Graphics::Direct2D::{ID2D1CommandList, ID2D1DeviceContext};
use windows::core::Result;

use crate::numerics::*;

#[derive(Component, Clone, Debug, PartialEq)]
pub struct VisualDrawContent {
    pub local_aabb: Aabb,
    pub command_list: ID2D1CommandList,
}

unsafe impl Send for VisualDrawContent {}
unsafe impl Sync for VisualDrawContent {}

impl VisualDrawContent {
    /// 事前に計算済みの `local_aabb` を用いて `VisualDrawContent` を構築する。
    ///
    /// # 引数
    /// - `local_aabb`: コンテンツがローカル座標で占める AABB。
    ///   **純粋な論理単位（DPI 非依存）** で与えること。DPI に依存した値を
    ///   入れると、モニタ跨ぎや DPI 変更時に内在プロパティとしての不変性が
    ///   崩れるため注意。
    /// - `command_list`: このコンテンツの描画コマンド列。呼び出し側で
    ///   [`ID2D1CommandList::Close`] 済みであることを想定する。
    ///
    /// バウンズが既知の場合はこちらを使う。未知で DC から求めたい場合は
    /// [`Self::new_calc_aabb`] を使用する。
    #[inline]
    pub fn new(local_aabb: Aabb, command_list: ID2D1CommandList) -> Self {
        Self {
            local_aabb,
            command_list,
        }
    }

    /// `command_list` のローカル AABB を DC から算出して `VisualDrawContent` を構築する。
    ///
    /// [`ID2D1DeviceContext::GetImageLocalBounds`] は world transform を含まない
    /// ローカル空間のバウンズを返すが、その値は **DC の現在の DPI・UnitMode・
    /// InterpolationMode を反映する**。したがって `local_aabb` を純粋な論理単位
    /// （DPI 非依存の内在プロパティ）として保持したい場合、渡す `dc` は
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
    pub fn new_calc_aabb(command_list: ID2D1CommandList, dc: &ID2D1DeviceContext) -> Result<Self> {
        let aabb = unsafe { dc.GetImageLocalBounds(&command_list) }?;
        Ok(Self {
            local_aabb: aabb.into(),
            command_list,
        })
    }
}
