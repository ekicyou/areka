use crate::com::d2d::D2D1DeviceContextExt;
use crate::ecs::Visual;
use crate::ecs::graphics::{GraphicsCommandList, GraphicsCore};
use crate::ecs::layout::Arrangement;
use crate::ecs::widget::brushes::Brushes;
use crate::ecs::widget::record::{record_command_list, store_command_list};
use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;
use tracing::{trace, warn};

/// 四角形ウィジット
///
/// サイズは`Arrangement`コンポーネントで管理されます。
/// レイアウトシステム（`BoxStyle` → Taffy → `Arrangement`）との統合により、
/// 動的なサイズ変更が自動的に反映されます。
///
/// 色は`Brushes`コンポーネントで指定します。
/// ```ignore
/// world.spawn((
///     Rectangle::new(),
///     Brushes::with_foreground(Brush::RED.as_color().unwrap()),
/// ));
/// ```
#[derive(Component, Debug, Clone, Default)]
#[component(on_add = on_rectangle_add, on_remove = on_rectangle_remove)]
pub struct Rectangle;

impl Rectangle {
    /// 新しいRectangleを作成
    pub fn new() -> Self {
        Self
    }
}

/// Rectangle追加時のフック: Visualコンポーネントを自動挿入 (R4)
fn on_rectangle_add(mut world: DeferredWorld, hook: HookContext) {
    // 既にVisualを持っている場合はスキップ
    if world.get::<Visual>(hook.entity).is_some() {
        return;
    }
    world
        .commands()
        .entity(hook.entity)
        .insert(Visual::default());
}

/// Rectangleコンポーネントが削除される時に呼ばれるフック
fn on_rectangle_remove(mut world: DeferredWorld, hook: HookContext) {
    let entity = hook.entity;
    // GraphicsCommandListを取得して中身をクリア(Changed検出のため)
    if let Some(mut cmd_list) = world.get_mut::<GraphicsCommandList>(entity) {
        cmd_list.set_if_neq(GraphicsCommandList::empty());
    }
}

/// RectangleコンポーネントからGraphicsCommandListを生成
///
/// Rectangleのサイズは`Arrangement`コンポーネントから取得されます。
/// 色は`Brushes::foreground_color`から取得されます。
pub fn draw_rectangles(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &Rectangle,
            &Arrangement,
            &Brushes,
            Option<&GraphicsCommandList>,
            Option<&Name>,
        ),
        // 命令の列が無い（立ち直りで外された）ときも描き直す
        Or<(
            Changed<Rectangle>,
            Changed<Arrangement>,
            Changed<Brushes>,
            Without<GraphicsCommandList>,
        )>,
    >,
    graphics_core: Option<Res<GraphicsCore>>,
) {
    if query.is_empty() {
        return;
    }
    let Some(graphics_core) = graphics_core else {
        warn!("GraphicsCore not available, skipping draw_rectangles");
        return;
    };
    // グローバル共有DeviceContextを取得（無ければ部品ごとでなく 1 回だけ記録）
    let Some(dc) = graphics_core.device_context() else {
        warn!("DeviceContext not available, skipping draw_rectangles");
        return;
    };

    for (entity, _rectangle, arrangement, brushes, cmd_list_opt, name) in query.iter() {
        let color = brushes.foreground_color();
        trace!(
            ?entity,
            width = arrangement.size.width,
            height = arrangement.size.height,
            "Drawing rectangle"
        );

        // 原点 (0,0) から部品の大きさで塗る
        let rect = crate::ecs::Rect {
            left: 0.0,
            top: 0.0,
            right: arrangement.size.width,
            bottom: arrangement.size.height,
        };
        let recorded = record_command_list(dc, |dc| {
            let brush = dc.create_solid_color_brush(&color, None)?;
            dc.fill_rectangle(&rect.into(), &brush);
            Ok(())
        });
        store_command_list(
            &mut commands,
            entity,
            name,
            cmd_list_opt.is_some(),
            recorded,
            "rectangle",
        );
    }
}
