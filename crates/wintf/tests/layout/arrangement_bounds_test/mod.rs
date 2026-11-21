// 配置の境界のテスト（子モジュールの入口）
// - `rect_ext`    : Rect / D2DRectExt の幾何と transform_rect_axis_aligned
// - `arrangement`: Size / Arrangement / GlobalArrangement / Matrix3x2 の変換
// 共通の use はここに集め、子のファイルは `use super::*` で取り込む。

use windows_numerics::Matrix3x2;
use wintf::ecs::{
    Arrangement, D2DRectExt, GlobalArrangement, LayoutScale, Offset, PointF, Rect, Size,
    transform_rect_axis_aligned,
};

mod arrangement;
mod rect_ext;
