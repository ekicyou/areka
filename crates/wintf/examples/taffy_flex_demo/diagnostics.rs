use super::*;

/// 全ウィンドウの DPI / GlobalArrangement / Arrangement / BoxStyle をダンプ
pub(crate) fn dump_all_windows_dpi(world: &mut World) {
    use wintf::ecs::DPI;
    use wintf::ecs::layout::Arrangement;

    info!("[DPIDump] ========== DPI Layout Dump for All Windows ==========");

    let mut window_query =
        world.query_filtered::<(Entity, Option<&bevy_ecs::name::Name>), With<FlexDemoWindow>>();
    let windows: Vec<(Entity, String)> = window_query
        .iter(world)
        .map(|(e, n)| {
            (
                e,
                n.map(|n| n.to_string())
                    .unwrap_or_else(|| format!("{:?}", e)),
            )
        })
        .collect();

    for (window_entity, window_name) in &windows {
        let window_entity = *window_entity;

        // DPI
        if let Some(dpi) = world.get::<DPI>(window_entity) {
            info!(
                window = %window_name,
                dpi_x = dpi.dpi_x,
                dpi_y = dpi.dpi_y,
                scale_x = format_args!("{:.3}", dpi.scale_x()),
                scale_y = format_args!("{:.3}", dpi.scale_y()),
                "[DPIDump] DPI"
            );
        } else {
            info!(window = %window_name, "[DPIDump] DPI: None");
        }

        // BoxStyle (論理 px)
        if let Some(bs) = world.get::<BoxStyle>(window_entity) {
            let (w, h) = px_size(bs);
            info!(
                window = %window_name,
                logical_width = format_args!("{:.1}", w),
                logical_height = format_args!("{:.1}", h),
                "[DPIDump] BoxStyle.size (logical px)"
            );
        }

        // WindowPos
        if let Some(wp) = world.get::<wintf::ecs::window::WindowPos>(window_entity) {
            info!(
                window = %window_name,
                pos = ?wp.position,
                size = ?wp.size,
                "[DPIDump] WindowPos"
            );
        }

        // Arrangement (local)
        if let Some(arr) = world.get::<Arrangement>(window_entity) {
            info!(
                window = %window_name,
                offset_x = format_args!("{:.1}", arr.offset.x),
                offset_y = format_args!("{:.1}", arr.offset.y),
                scale_x = format_args!("{:.3}", arr.scale.x),
                scale_y = format_args!("{:.3}", arr.scale.y),
                size_w = format_args!("{:.1}", arr.size.width),
                size_h = format_args!("{:.1}", arr.size.height),
                "[DPIDump] Arrangement"
            );
        }

        // GlobalArrangement
        if let Some(ga) = world.get::<GlobalArrangement>(window_entity) {
            let ga_width = ga.bounds.right - ga.bounds.left;
            let ga_height = ga.bounds.bottom - ga.bounds.top;
            info!(
                window = %window_name,
                bounds_left = format_args!("{:.1}", ga.bounds.left),
                bounds_top = format_args!("{:.1}", ga.bounds.top),
                bounds_right = format_args!("{:.1}", ga.bounds.right),
                bounds_bottom = format_args!("{:.1}", ga.bounds.bottom),
                physical_width = format_args!("{:.1}", ga_width),
                physical_height = format_args!("{:.1}", ga_height),
                scale_x = format_args!("{:.3}", ga.scale_x()),
                scale_y = format_args!("{:.3}", ga.scale_y()),
                transform_M11 = format_args!("{:.3}", ga.transform.M11),
                transform_M22 = format_args!("{:.3}", ga.transform.M22),
                "[DPIDump] GlobalArrangement"
            );
        }

        // 子エンティティをダンプ
        dump_children_dpi(world, window_entity, 1);
    }

    info!("[DPIDump] ========== End of DPI Layout Dump ==========");
}

/// 子エンティティの GA/Arrangement/BoxStyle を再帰的にダンプ
pub(crate) fn dump_children_dpi(world: &mut World, entity: Entity, depth: usize) {
    use wintf::ecs::layout::Arrangement;

    let children: Vec<Entity> = world
        .get::<bevy_ecs::hierarchy::Children>(entity)
        .map(|c| c.iter().collect())
        .unwrap_or_default();

    let indent = "  ".repeat(depth + 1);
    for child in children {
        let name = world
            .get::<bevy_ecs::name::Name>(child)
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("{:?}", child));

        // BoxStyle (論理 px)
        let bs_str = if let Some(bs) = world.get::<BoxStyle>(child) {
            let (w, h) = px_size(bs);
            format!("box_style=({:.1}x{:.1})", w, h)
        } else {
            "no BoxStyle".to_string()
        };

        let arr_str = if let Some(arr) = world.get::<Arrangement>(child) {
            format!(
                "offset=({:.1},{:.1}) scale=({:.3},{:.3}) size=({:.1}x{:.1})",
                arr.offset.x,
                arr.offset.y,
                arr.scale.x,
                arr.scale.y,
                arr.size.width,
                arr.size.height
            )
        } else {
            "no Arrangement".to_string()
        };
        let ga_str = if let Some(ga) = world.get::<GlobalArrangement>(child) {
            format!(
                "bounds=({:.1},{:.1})-({:.1},{:.1}) scale=({:.3},{:.3})",
                ga.bounds.left,
                ga.bounds.top,
                ga.bounds.right,
                ga.bounds.bottom,
                ga.scale_x(),
                ga.scale_y()
            )
        } else {
            "no GA".to_string()
        };
        info!(
            "[DPIDump]{}{}: {} arr=[{}] ga=[{}]",
            indent, name, bs_str, arr_str, ga_str
        );
        // 再帰（2階層まで）
        if depth < 2 {
            dump_children_dpi(world, child, depth + 1);
        }
    }
}

/// `BoxStyle.size` の px の幅と高さ（px でない・無い次元は 0）。
fn px_size(bs: &BoxStyle) -> (f32, f32) {
    let px = |d: Option<Dimension>| match d {
        Some(Dimension::Px(v)) => v,
        _ => 0.0,
    };
    bs.size
        .as_ref()
        .map_or((0.0, 0.0), |size| (px(size.width), px(size.height)))
}
