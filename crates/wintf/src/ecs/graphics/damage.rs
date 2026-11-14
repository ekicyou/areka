//! 描画面 1 枚の合成の計画（`wintf-visual-damage-redraw`）。この回に入れ物をどう描くかと、平行移動の写しを
//! 使ってよいかを決める。記録は target `wintf::damage` の debug の 1 行。

use super::visual::PxRect;

/// 合成の記録の target（履歴の層の info 以上には入らない）。
pub(crate) const DAMAGE_TARGET: &str = "wintf::damage";

/// 全部の合成の理由（記録の `reason` の綴り・3.4）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FullReason {
    First,
    Recreated,
    RootList,
    Scale,
    NoPrev,
    Undecidable,
    CoversSurface,
}

impl FullReason {
    pub fn as_str(self) -> &'static str {
        match self {
            FullReason::First => "first",
            FullReason::Recreated => "recreated",
            FullReason::RootList => "root_list",
            FullReason::Scale => "scale",
            FullReason::NoPrev => "no_prev",
            FullReason::Undecidable => "undecidable",
            FullReason::CoversSurface => "covers_surface",
        }
    }
}

/// 入れ物の中の写し（`src` を左上 `dst` へ）。
#[expect(
    dead_code,
    reason = "計画を立てる plan（タスク 3.1）が使い始めたら外す"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegionShift {
    pub src: PxRect,
    pub dst: (u32, u32),
}

/// 範囲だけの合成の計画。
#[expect(
    dead_code,
    reason = "計画を立てる plan（タスク 3.1）が使い始めたら外す"
)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PartialPlan {
    /// 描き直す矩形（描画面の中・空でない）。
    pub rects: Vec<PxRect>,
    pub shifts: Vec<RegionShift>,
    pub items_redrawn: u32,
    /// `rects` の面積の和（重なりは重ねて数える）。
    pub area: u64,
    /// 写した面積。
    pub shift_area: u64,
    /// 写しの切り抜きの中で描き直した面積。
    pub exposed_area: u64,
    /// 描画面へ写す矩形＝`rects` と `shifts` の先の外接矩形（2.6）。
    pub present_rect: PxRect,
}

/// 描画面 1 枚の、この回の合成の計画。
#[expect(
    dead_code,
    reason = "計画を立てる plan（タスク 3.1）が使い始めたら外す"
)]
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ComposePlan {
    Skip,
    PresentOnly,
    Full(FullReason),
    Partial(PartialPlan),
}
