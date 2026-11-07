//! 描く中身の道具（`draw_content_test_support`）で作った節を `World` に置く（要件 6.7）。

use super::draw_content_test_support::solid_rect_content;
use super::*;
use crate::numerics::Aabb;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

#[test]
fn solid_rect_content_node_sits_in_world() {
    let rect = D2D_RECT_F {
        left: 2.0,
        top: 3.0,
        right: 12.0,
        bottom: 8.0,
    };
    let mut world = World::new();
    let node = world
        .spawn((Visual::default(), solid_rect_content(rect)))
        .id();
    let content = world.get::<VisualDrawContent>(node).unwrap();
    // 閉じた命令の列の外接矩形が描いた矩形そのもの（GetImageLocalBounds は閉じていないと失敗する）。
    assert_eq!(content.local_aabb, Aabb(rect));
}
