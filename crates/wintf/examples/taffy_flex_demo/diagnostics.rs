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
            let (w, h) = match &bs.size {
                Some(size) => {
                    let w = size
                        .width
                        .map(|d| match d {
                            Dimension::Px(v) => v,
                            _ => 0.0,
                        })
                        .unwrap_or(0.0);
                    let h = size
                        .height
                        .map(|d| match d {
                            Dimension::Px(v) => v,
                            _ => 0.0,
                        })
                        .unwrap_or(0.0);
                    (w, h)
                }
                None => (0.0, 0.0),
            };
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
            let (w, h) = match &bs.size {
                Some(size) => {
                    let w = size
                        .width
                        .map(|d| match d {
                            Dimension::Px(v) => v,
                            _ => 0.0,
                        })
                        .unwrap_or(0.0);
                    let h = size
                        .height
                        .map(|d| match d {
                            Dimension::Px(v) => v,
                            _ => 0.0,
                        })
                        .unwrap_or(0.0);
                    (w, h)
                }
                None => (0.0, 0.0),
            };
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

/// 1秒後のヒットテスト検証
#[allow(dead_code)]
pub(crate) fn test_hit_test_1s(world: &mut World) {
    println!("[HitTest @1s] === Running hit test verification ===");

    // ウィンドウエンティティを取得
    let mut window_query = world.query_filtered::<Entity, With<FlexDemoWindow>>();
    let Some(window_entity) = window_query.iter(world).next() else {
        println!("[HitTest @1s] Window entity not found");
        return;
    };

    // ウィンドウの GlobalArrangement からスケールと原点を取得
    let Some(window_global) = world.get::<GlobalArrangement>(window_entity) else {
        println!("[HitTest @1s] Window has no GlobalArrangement");
        return;
    };
    let (scale_x, scale_y) = window_global.scale();
    let origin_x = window_global.bounds.left;
    let origin_y = window_global.bounds.top;

    println!(
        "[HitTest @1s] Window scale: ({:.2}, {:.2}), origin: ({:.0}, {:.0})",
        scale_x, scale_y, origin_x, origin_y
    );

    // DIP座標からスクリーン座標（物理ピクセル）に変換するヘルパー
    let to_screen = |dip_x: f32, dip_y: f32| -> PhysicalPoint {
        PhysicalPoint::new(origin_x + dip_x * scale_x, origin_y + dip_y * scale_y)
    };

    // ウィンドウの WindowPos をログ出力（基準座標）
    println!("[HitTest @1s] --- Window reference coordinates ---");
    dump_window_pos(world, window_entity);

    // 各エンティティの GlobalArrangement.bounds をログ出力
    println!("[HitTest @1s] --- Entity bounds (GlobalArrangement) ---");
    dump_entity_bounds(world, "FlexDemo-Window", window_entity);

    // FlexContainerを検索
    let mut container_query = world.query_filtered::<Entity, With<FlexDemoContainer>>();
    if let Some(container) = container_query.iter(world).next() {
        dump_entity_bounds(world, "FlexDemo-Container", container);
    }

    // 各Boxを検索
    let mut red_query = world.query_filtered::<Entity, With<RedBox>>();
    if let Some(red) = red_query.iter(world).next() {
        dump_entity_bounds(world, "RedBox", red);
    }

    let mut green_query = world.query_filtered::<Entity, With<GreenBox>>();
    if let Some(green) = green_query.iter(world).next() {
        dump_entity_bounds(world, "GreenBox", green);
    }

    let mut blue_query = world.query_filtered::<Entity, With<BlueBox>>();
    if let Some(blue) = blue_query.iter(world).next() {
        dump_entity_bounds(world, "BlueBox", blue);
    }
    println!("[HitTest @1s] --- End of entity bounds ---");

    // テストポイント（DIP座標で指定、to_screen で物理ピクセルに変換）
    // 実際のレイアウト結果（物理ピクセル、スケール1.25、原点125,125）:
    // - GreenBox: (135,375)→(260,500) → DIP (8,200)→(108,300)
    // - RedBox: (235,375)→(485,500) → DIP (88,200)→(288,300)
    //   - RedBox内に子要素 SeikatuImage があり、中心テストでは子がヒット
    // - BlueBox: (435,375)→(560,500) → DIP (248,200)→(348,300)
    let test_points = [
        (
            to_screen(50.0, 250.0),
            "GreenBox center (DIP 50,250)",
            "GreenBox",
        ),
        (
            to_screen(150.0, 250.0),
            "RedBox child (SeikatuImage) (DIP 150,250)",
            "SeikatuImage",
        ),
        (
            to_screen(320.0, 250.0),
            "BlueBox center (DIP 320,250)",
            "BlueBox",
        ),
        (
            to_screen(15.0, 15.0),
            "Container area (DIP 15,15)",
            "FlexDemo-Container",
        ),
        (
            to_screen(700.0, 300.0),
            "Outside Container (DIP 700,300)",
            "FlexDemo-Window",
        ),
    ];

    println!("[HitTest @1s] --- Hit test results ---");
    for (point, description, expected) in test_points {
        match hit_test(world, window_entity, point) {
            Some(entity) => {
                if let Some(name) = world.get::<Name>(entity) {
                    println!(
                        "[HitTest @1s] {} at DIP->Screen ({:.0}, {:.0}): Hit {:?} (expected: {})",
                        description,
                        point.x,
                        point.y,
                        name.as_str(),
                        expected
                    );
                } else {
                    println!(
                        "[HitTest @1s] {} at ({:.0}, {:.0}): Hit Entity {:?} (no name)",
                        description, point.x, point.y, entity
                    );
                }
            }
            None => {
                println!(
                    "[HitTest @1s] {} at ({:.0}, {:.0}): No hit (expected: {})",
                    description, point.x, point.y, expected
                );
            }
        }
    }

    // === リージョンテスト用ヒットテスト検証 ===
    println!("[HitTest @1s] --- Region hit test (hit_test_in_window_ex) ---");

    // リージョンテストボックスの bounds をダンプ
    let mut rect_query = world.query_filtered::<Entity, With<RegionRectBox>>();
    if let Some(e) = rect_query.iter(world).next() {
        dump_entity_bounds(world, "RegionRectBox", e);
    }
    let mut polygon_query = world.query_filtered::<Entity, With<RegionPolygonBox>>();
    if let Some(e) = polygon_query.iter(world).next() {
        dump_entity_bounds(world, "RegionPolygonBox", e);
    }
    let mut mixed_query = world.query_filtered::<Entity, With<RegionMixedBox>>();
    if let Some(e) = mixed_query.iter(world).next() {
        dump_entity_bounds(world, "RegionMixedBox", e);
    }
    let mut colormap_query = world.query_filtered::<Entity, With<RegionColorMapBox>>();
    if let Some(e) = colormap_query.iter(world).next() {
        dump_entity_bounds(world, "RegionColorMapBox", e);
    }
    let mut fallback_query = world.query_filtered::<Entity, With<RegionFallbackBox>>();
    if let Some(e) = fallback_query.iter(world).next() {
        dump_entity_bounds(world, "RegionFallbackBox", e);
    }

    // hit_test_in_window_ex でリージョン付きヒットテスト
    // 各ボックスの4象限をテスト（DIP座標はウィンドウサイズに相対）
    test_region_hit_ex(world, window_entity, "RectBox top-left?", 40.0, 420.0);
    test_region_hit_ex(world, window_entity, "RectBox top-right?", 110.0, 420.0);
    test_region_hit_ex(world, window_entity, "RectBox bottom-left?", 40.0, 490.0);
    test_region_hit_ex(world, window_entity, "RectBox bottom-right?", 110.0, 490.0);
    test_region_hit_ex(world, window_entity, "FallbackBox center", 720.0, 460.0);

    println!("[HitTest @1s] --- End of region hit test ---");
}

/// リージョン付きヒットテスト結果を表示するヘルパー
#[allow(dead_code)]
pub(crate) fn test_region_hit_ex(
    world: &World,
    window_entity: Entity,
    description: &str,
    client_x: f32,
    client_y: f32,
) {
    let client_point = PhysicalPoint::new(client_x, client_y);
    match hit_test_in_window_ex(world, window_entity, client_point) {
        Some(result) => {
            let name = world
                .get::<Name>(result.entity)
                .map(|n| n.as_str().to_string())
                .unwrap_or_else(|| format!("{:?}", result.entity));
            println!(
                "[HitTest @1s] {} at client({:.0},{:.0}): entity={}, region={:?}",
                description, client_x, client_y, name, result.region
            );
        }
        None => {
            println!(
                "[HitTest @1s] {} at client({:.0},{:.0}): No hit",
                description, client_x, client_y
            );
        }
    }
}

#[allow(dead_code)]
pub(crate) fn test_hit_test_6s(world: &mut World) {
    println!("[HitTest @6s] === Running hit test after layout change ===");

    // ウィンドウエンティティを取得
    let mut window_query = world.query_filtered::<Entity, With<FlexDemoWindow>>();
    let Some(window_entity) = window_query.iter(world).next() else {
        println!("[HitTest @6s] Window entity not found");
        return;
    };

    // ウィンドウの GlobalArrangement からスケールと原点を取得
    let Some(window_global) = world.get::<GlobalArrangement>(window_entity) else {
        println!("[HitTest @6s] Window has no GlobalArrangement");
        return;
    };
    let (scale_x, scale_y) = window_global.scale();
    let origin_x = window_global.bounds.left;
    let origin_y = window_global.bounds.top;

    println!(
        "[HitTest @6s] Window scale: ({:.2}, {:.2}), origin: ({:.0}, {:.0})",
        scale_x, scale_y, origin_x, origin_y
    );

    // DIP座標からスクリーン座標（物理ピクセル）に変換するヘルパー
    let to_screen = |dip_x: f32, dip_y: f32| -> PhysicalPoint {
        PhysicalPoint::new(origin_x + dip_x * scale_x, origin_y + dip_y * scale_y)
    };

    // 各エンティティの GlobalArrangement.bounds をログ出力（デバッグ用）
    println!("[HitTest @6s] --- Entity bounds (GlobalArrangement) ---");
    dump_entity_bounds(world, "FlexDemo-Window", window_entity);

    let mut container_query = world.query_filtered::<Entity, With<FlexDemoContainer>>();
    if let Some(container) = container_query.iter(world).next() {
        dump_entity_bounds(world, "FlexDemo-Container", container);
    }

    let mut red_query = world.query_filtered::<Entity, With<RedBox>>();
    if let Some(red) = red_query.iter(world).next() {
        dump_entity_bounds(world, "RedBox", red);
    }

    let mut green_query = world.query_filtered::<Entity, With<GreenBox>>();
    if let Some(green) = green_query.iter(world).next() {
        dump_entity_bounds(world, "GreenBox", green);
    }

    let mut blue_query = world.query_filtered::<Entity, With<BlueBox>>();
    if let Some(blue) = blue_query.iter(world).next() {
        dump_entity_bounds(world, "BlueBox", blue);
    }
    println!("[HitTest @6s] --- End of entity bounds ---");

    // テストポイント（DIP座標で指定）
    // 6秒時点: ウィンドウサイズ 600x400 DIP、Column レイアウト
    // 実際のレイアウト結果に基づく（Containerは幅150DIP程度、左寄せ）
    // GreenBox, RedBox, BlueBox は Container内で縦並び
    let test_points = [
        (
            to_screen(20.0, 50.0),
            "GreenBox area (DIP 20,50)",
            "GreenBox",
        ),
        (to_screen(20.0, 150.0), "RedBox area (DIP 20,150)", "RedBox"),
        (
            to_screen(20.0, 200.0),
            "BlueBox area (DIP 20,200)",
            "BlueBox",
        ),
        (
            to_screen(5.0, 5.0),
            "Top-left corner (DIP 5,5)",
            "FlexDemo-Container",
        ),
        (
            to_screen(400.0, 200.0),
            "Right side - outside Container (DIP 400,200)",
            "FlexDemo-Window",
        ),
    ];

    println!("[HitTest @6s] --- Hit test results ---");
    for (point, description, expected) in test_points {
        match hit_test(world, window_entity, point) {
            Some(entity) => {
                if let Some(name) = world.get::<Name>(entity) {
                    let result = if name.as_str() == expected {
                        "✓"
                    } else {
                        "✗"
                    };
                    println!(
                        "[HitTest @6s] {} {} -> ({:.0}, {:.0}): Hit {:?} (expected: {})",
                        result,
                        description,
                        point.x,
                        point.y,
                        name.as_str(),
                        expected
                    );
                } else {
                    println!(
                        "[HitTest @6s] ✗ {} -> ({:.0}, {:.0}): Hit Entity {:?} (no name, expected: {})",
                        description, point.x, point.y, entity, expected
                    );
                }
            }
            None => {
                println!(
                    "[HitTest @6s] ✗ {} -> ({:.0}, {:.0}): No hit (expected: {})",
                    description, point.x, point.y, expected
                );
            }
        }
    }
}

/// エンティティの GlobalArrangement.bounds をログ出力
fn dump_entity_bounds(world: &World, name: &str, entity: Entity) {
    if let Some(global) = world.get::<GlobalArrangement>(entity) {
        let b = &global.bounds;
        println!(
            "[HitTest] {} bounds: left={:.1}, top={:.1}, right={:.1}, bottom={:.1} (size: {:.1}x{:.1})",
            name,
            b.left,
            b.top,
            b.right,
            b.bottom,
            b.right - b.left,
            b.bottom - b.top
        );
    } else {
        println!("[HitTest] {} has no GlobalArrangement", name);
    }
}

/// ウィンドウの WindowPos をログ出力
#[allow(dead_code)]
fn dump_window_pos(world: &World, entity: Entity) {
    if let Some(window_pos) = world.get::<WindowPos>(entity) {
        if let Some(pos) = window_pos.position {
            println!("[HitTest] WindowPos.position: x={}, y={}", pos.x, pos.y);
        } else {
            println!("[HitTest] WindowPos.position: None");
        }
        if let Some(size) = window_pos.size {
            println!(
                "[HitTest] WindowPos.size: width={}, height={}",
                size.width, size.height
            );
        } else {
            println!("[HitTest] WindowPos.size: None");
        }
    } else {
        println!("[HitTest] Window has no WindowPos");
    }
}
