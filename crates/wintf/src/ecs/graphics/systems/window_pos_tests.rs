//! 装置の失いで描画の木の単位矩形（古い工場の図形）が空になること（外のテストからは見えない欄）。

use super::*;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

#[test]
fn invalidate_clears_draw_tree_unit_rect() {
    let graphics = GraphicsCore::new().expect("GraphicsCore creation");
    let unit_rect = unsafe {
        graphics
            .d2d_factory()
            .expect("factory")
            .CreateRectangleGeometry(&D2D_RECT_F {
                left: 0.0,
                top: 0.0,
                right: 1.0,
                bottom: 1.0,
            })
    }
    .expect("geometry");
    let mut world = World::new();
    world.insert_resource(graphics);
    let root = world
        .spawn(DrawTree {
            commands: Vec::new(),
            unit_rect: Some(unit_rect.into()),
        })
        .id();

    world.resource_mut::<GraphicsCore>().invalidate();
    let mut schedule = Schedule::default();
    schedule.add_systems(invalidate_dependent_components);
    schedule.run(&mut world);

    assert!(world.get::<DrawTree>(root).unwrap().unit_rect.is_none());
}
