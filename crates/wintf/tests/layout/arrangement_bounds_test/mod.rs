// tests/layout/arrangement_bounds_test/mod.rs — arrangement bounds test entry point
//
// Behavior-preserving split of the original `arrangement_bounds_test.rs` into two
// cohesive sub-modules:
//   - `rect_ext`      : Rect / D2DRectExt geometry + transform_rect_axis_aligned
//   - `arrangement`   : Size / Arrangement / GlobalArrangement / Matrix3x2 conversions
//
// Shared imports are hoisted here; sub-files pull them in via `use super::*`.

use windows_numerics::Matrix3x2;
use wintf::ecs::{
    Arrangement, D2DRectExt, GlobalArrangement, LayoutScale, Offset, PointF, Rect, Size,
    transform_rect_axis_aligned,
};

mod arrangement;
mod rect_ext;
