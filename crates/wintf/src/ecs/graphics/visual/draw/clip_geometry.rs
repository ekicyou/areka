//! 節の切り抜きの図形（2.4）。矩形は `build` が単位矩形を共有し、任意の図形はそのまま使うので、
//! ここで作るのは角丸の path geometry だけ。

use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::core::{Interface, Result};
use windows_numerics::Vector2;

/// (0,0)-(width,height) の角丸矩形の path geometry（局所の DIP）。均一も個別も同じ 1 本の道。
/// 半径は D2D の角丸矩形と同じく X は幅の半分・Y は高さの半分で別々に頭を打つ（角は楕円になりうる）。
pub(crate) fn rounded_rect(
    factory: &ID2D1Factory,
    width: f32,
    height: f32,
    top_left: f32,
    top_right: f32,
    bottom_right: f32,
    bottom_left: f32,
) -> Result<ID2D1Geometry> {
    let (cap_x, cap_y) = ((width * 0.5).max(0.0), (height * 0.5).max(0.0));
    let [tl, tr, br, bl] = [top_left, top_right, bottom_right, bottom_left]
        .map(|r| (r.clamp(0.0, cap_x), r.clamp(0.0, cap_y)));
    let pt = |x: f32, y: f32| Vector2 { X: x, Y: y };
    unsafe {
        let path = factory.CreatePathGeometry()?;
        let sink = path.Open()?;
        sink.BeginFigure(pt(tl.0, 0.0), D2D1_FIGURE_BEGIN_FILLED);
        let corner = |to: Vector2, (rx, ry): (f32, f32)| {
            sink.AddArc(&D2D1_ARC_SEGMENT {
                point: to,
                size: D2D_SIZE_F {
                    width: rx,
                    height: ry,
                },
                rotationAngle: 0.0,
                sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                arcSize: D2D1_ARC_SIZE_SMALL,
            })
        };
        sink.AddLine(pt(width - tr.0, 0.0));
        corner(pt(width, tr.1), tr);
        sink.AddLine(pt(width, height - br.1));
        corner(pt(width - br.0, height), br);
        sink.AddLine(pt(bl.0, height));
        corner(pt(0.0, height - bl.1), bl);
        sink.AddLine(pt(0.0, tl.1));
        corner(pt(tl.0, 0.0), tl);
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        sink.Close()?;
        path.cast()
    }
}
