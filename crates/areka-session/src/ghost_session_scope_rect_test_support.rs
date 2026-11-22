//! 窓の矩形の読み手のテストが分け合う小道具（`#[test]` を持たない）。

use super::*;
use areka_placement::follow::OffsetBase;
use areka_placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};

/// 主の画面より左上の画面の矩形（SSP の段 1 の値）。
pub const LEFT_SCREEN: ScreenRect = ScreenRect {
    left: -782,
    top: 201,
    right: 49,
    bottom: 1227,
};

pub fn placement() -> ScopePlacement {
    ScopePlacement {
        scope: 0,
        char_pos: PointPx { x: 1483, y: 733 },
        char_size: SizePx { w: 434, h: 687 },
        balloon_pos: PointPx { x: 1071, y: 708 },
        balloon_size: SizePx { w: 223, h: 158 },
        balloon_offset: PointPx { x: -412, y: -25 },
        balloon_offset_base: OffsetBase::unpinned(PointPx { x: -412, y: -25 }),
        balloon_limit: true,
        anchor: Anchor::Bottom,
        balloon_keyword_base: None,
    }
}
