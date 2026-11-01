//! 表示成立点の倍率の記録（`shell-scale-linkage` 要件 3.3・3.4・4.2・4.4・5.6・5.7）。
//!
//! 表示成立点（`apply_show` の `applied` を書く唯一の点）で、装着ごとの起点
//! （`PresentTarget::scale_baseline`）と（k・その k を出した窓の DPI）の組が違うときだけ
//! [`ScaleChange`] を積む。取り出すのは呼び手（毎フレームの相）で、表示層は kanade・sylphya・
//! スコープを知らない（[`TargetId`] のまま渡す）。

use super::{EmoPresenter, ScaleRatio, TargetId};

/// 絵にかかった倍率と、それを出した窓の DPI（`DPI` が無ければ `None`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedScale {
    pub ratio: ScaleRatio,
    pub dpi_x: Option<u16>,
}

/// 表示成立点で（倍率・DPI）が装着ごとの起点と違った記録。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleChange {
    pub target: TargetId,
    /// 起点（`None`＝この装着で初めて決まった）。
    pub before: Option<AppliedScale>,
    pub after: AppliedScale,
}

impl EmoPresenter {
    /// 積まれた記録を積んだ順に全部取り出す（無ければ空・確保しない）。
    pub fn take_scale_changes(&mut self) -> Vec<ScaleChange> {
        std::mem::take(&mut self.scale_changes)
    }
}
