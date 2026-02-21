use super::drag::{
    on_container_drag, on_container_drag_end, on_container_drag_start, on_container_pressed,
};
use super::handlers::{
    on_blue_box_pressed, on_green_box_moved, on_green_box_pressed, on_green_child_pressed,
    on_image_pressed, on_red_box_pressed,
};
use super::region::{on_normal_hit_box_pressed, on_region_box_moved, on_region_box_pressed};
use super::*;

/// Flexboxデモウィンドウを作成（パラメータ化）
pub(crate) fn create_flexbox_window(
    world: &mut World,
    title: &str,
    position: windows::Win32::Foundation::POINT,
) -> Entity {
    // Window Entity (ルート)
    // WindowPos.position でクライアント領域の位置を指定
    let window_entity = world
        .spawn((
            Name::new(format!("FlexDemo-Window [{}]", title)),
            FlexDemoWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                flex_direction: Some(taffy::FlexDirection::Column),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(800.0)),
                    height: Some(Dimension::Px(700.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(position),
                ..Default::default()
            },
            Window {
                title: title.to_string(),
                ..Default::default()
            },
            HitTest::none(),
        ))
        .id();

    // Flexコンテナ（横並び）- 右クリックで色変更
    let flex_container = world
        .spawn((
            Name::new("FlexDemo-Container"),
            FlexDemoContainer,
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.9,
                g: 0.9,
                b: 0.9,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Row),
                justify_content: Some(taffy::JustifyContent::SpaceEvenly),
                align_items: Some(taffy::AlignItems::Center),
                size: Some(BoxSize {
                    width: None,
                    height: Some(Dimension::Px(160.0)),
                }),
                flex_grow: Some(0.0),
                flex_shrink: Some(0.0),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    right: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    top: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    bottom: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                })),
                ..Default::default()
            },
            // イベントハンドラ: 右クリックで色変更
            OnPointerPressed(on_container_pressed),
            // ドラッグハンドラ: ウィンドウドラッグ移動
            DragConfig::default(),
            OnDragStart(on_container_drag_start),
            OnDrag(on_container_drag),
            OnDragEnd(on_container_drag_end),
            ChildOf(window_entity),
        ))
        .id();

    // Flexアイテム1（赤、固定200px幅）- 左クリックで色トグル（αマスクデモ用）
    // SeikatuImage（子）の透明部分をクリックするとRedBoxにイベントが伝播し色が変わる
    let red_box = world
        .spawn((
            Name::new("RedBox"),
            RedBox,
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(200.0)),
                    height: Some(Dimension::Px(100.0)),
                }),
                flex_grow: Some(0.0),
                flex_shrink: Some(0.0),
                flex_basis: Some(Dimension::Px(200.0)),
                ..Default::default()
            },
            // イベントハンドラ: 左クリックで色トグル（赤 ⇔ 黄）
            OnPointerPressed(on_red_box_pressed),
            ChildOf(flex_container),
        ))
        .id();

    // 赤ボックスの子として画像を追加（αマスクヒットテストデモ）
    // 透明部分クリック → 親(RedBox)に伝播して色が変わる
    // 不透明部分クリック → 画像がイベント消費して親に伝播しない
    const SEIKATU_IMAGE_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/assets/seikatu_0_0.webp");
    world.spawn((
        Name::new("SeikatuImage"),
        BitmapSource::new(SEIKATU_IMAGE_PATH),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(64.0)),
                height: Some(Dimension::Px(64.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: wintf::ecs::layout::LengthPercentageAuto::Px(68.0),
                right: wintf::ecs::layout::LengthPercentageAuto::Auto,
                top: wintf::ecs::layout::LengthPercentageAuto::Px(18.0),
                bottom: wintf::ecs::layout::LengthPercentageAuto::Px(18.0),
            })),
            ..Default::default()
        },
        // イベントハンドラ: 不透明部分クリックでイベント消費（親に伝播しない）
        OnPointerPressed(on_image_pressed),
        ChildOf(red_box),
    ));

    // Flexアイテム2（緑、growで伸縮）- マウス移動でログ、左クリックでTunnelキャプチャ
    let green_box = world
        .spawn((
            Name::new("GreenBox"),
            GreenBox,
            Visual {
                opacity: 0.5,
                ..Default::default()
            },
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Column),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(100.0)),
                    height: Some(Dimension::Px(100.0)),
                }),
                flex_grow: Some(1.0),
                flex_shrink: Some(1.0),
                flex_basis: Some(Dimension::Auto),
                ..Default::default()
            },
            // イベントハンドラ: ポインター移動でログ出力
            OnPointerMoved(on_green_box_moved),
            // イベントハンドラ: ポインター押下でTunnelキャプチャ
            OnPointerPressed(on_green_box_pressed),
            ChildOf(flex_container),
        ))
        .id();

    // GreenBoxの子エンティティ（黄色矩形、半透明）
    world.spawn((
        Name::new("GreenBoxChild"),
        GreenBoxChild,
        Visual {
            opacity: 0.5,
            ..Default::default()
        },
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(50.0)),
                height: Some(Dimension::Px(50.0)),
            }),
            ..Default::default()
        },
        // イベントハンドラ: Tunnelキャプチャ検証用
        OnPointerPressed(on_green_child_pressed),
        ChildOf(green_box),
    ));

    // Flexアイテム3（青、growで伸縮、より大きなgrow値）- 左クリックでサイズ変更
    world.spawn((
        Name::new("BlueBox"),
        BlueBox,
        Visual {
            opacity: 0.5,
            ..Default::default()
        },
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(100.0)),
                height: Some(Dimension::Px(100.0)),
            }),
            flex_grow: Some(2.0),
            flex_shrink: Some(1.0),
            flex_basis: Some(Dimension::Auto),
            ..Default::default()
        },
        // イベントハンドラ: 左クリックでサイズトグル
        OnPointerPressed(on_blue_box_pressed),
        ChildOf(flex_container),
    ));

    // =======================================================================
    // 下段: リージョンテストコンテナ（名前付きヒット領域のデモ）
    // =======================================================================
    let region_container = world
        .spawn((
            Name::new("RegionTest-Container"),
            RegionTestContainer,
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.85,
                g: 0.85,
                b: 0.95,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Row),
                justify_content: Some(taffy::JustifyContent::SpaceEvenly),
                align_items: Some(taffy::AlignItems::Center),
                size: Some(BoxSize {
                    width: None,
                    height: None,
                }),
                flex_grow: Some(1.0),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    right: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    top: wintf::ecs::layout::LengthPercentageAuto::Px(0.0),
                    bottom: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                })),
                ..Default::default()
            },
            ChildOf(window_entity),
        ))
        .id();

    // --- 1. 矩形リージョン (Rect) ---
    // 4分割: top-left / top-right / bottom-left / bottom-right
    let rect_region_map = HitRegionMap::builder()
        .rect("top-left", 0.0, 0.0, 70.0, 75.0)
        .rect("top-right", 70.0, 0.0, 70.0, 75.0)
        .rect("bottom-left", 0.0, 75.0, 70.0, 75.0)
        .rect("bottom-right", 70.0, 75.0, 70.0, 75.0)
        .build()
        .expect("Rect region map build failed");

    world.spawn((
        Name::new("RegionRectBox"),
        RegionRectBox,
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.8,
            g: 0.6,
            b: 0.6,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(140.0)),
                height: Some(Dimension::Px(150.0)),
            }),
            ..Default::default()
        },
        HitTest::named_regions(),
        rect_region_map,
        OnPointerPressed(on_region_box_pressed),
        OnPointerMoved(on_region_box_moved),
        ChildOf(region_container),
    ));

    // --- 2. 多角形リージョン (Polygon) ---
    // 4分割を三角形ポリゴンで表現（各象限を2三角形で充填）
    let polygon_region_map = HitRegionMap::builder()
        .polygon(
            "top-left",
            &[(0.0, 0.0), (70.0, 0.0), (70.0, 75.0), (0.0, 75.0)],
        )
        .polygon(
            "top-right",
            &[(70.0, 0.0), (140.0, 0.0), (140.0, 75.0), (70.0, 75.0)],
        )
        .polygon(
            "bottom-left",
            &[(0.0, 75.0), (70.0, 75.0), (70.0, 150.0), (0.0, 150.0)],
        )
        .polygon(
            "bottom-right",
            &[(70.0, 75.0), (140.0, 75.0), (140.0, 150.0), (70.0, 150.0)],
        )
        .build()
        .expect("Polygon region map build failed");

    world.spawn((
        Name::new("RegionPolygonBox"),
        RegionPolygonBox,
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.6,
            g: 0.8,
            b: 0.6,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(140.0)),
                height: Some(Dimension::Px(150.0)),
            }),
            ..Default::default()
        },
        HitTest::named_regions(),
        polygon_region_map,
        OnPointerPressed(on_region_box_pressed),
        OnPointerMoved(on_region_box_moved),
        ChildOf(region_container),
    ));

    // --- 3. 混在＋重複リージョン (Mixed + Overlap) ---
    // rect(top-left, bottom-right) + polygon(top-right, bottom-left) の混在
    let mixed_region_map = HitRegionMap::builder()
        .rect("top-left", 0.0, 0.0, 70.0, 75.0)
        .polygon(
            "top-right",
            &[(70.0, 0.0), (140.0, 0.0), (140.0, 75.0), (70.0, 75.0)],
        )
        .polygon(
            "bottom-left",
            &[(0.0, 75.0), (70.0, 75.0), (70.0, 150.0), (0.0, 150.0)],
        )
        .rect("bottom-right", 70.0, 75.0, 70.0, 75.0)
        .build()
        .expect("Mixed region map build failed");

    world.spawn((
        Name::new("RegionMixedBox"),
        RegionMixedBox,
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.6,
            g: 0.6,
            b: 0.8,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(140.0)),
                height: Some(Dimension::Px(150.0)),
            }),
            ..Default::default()
        },
        HitTest::named_regions(),
        mixed_region_map,
        OnPointerPressed(on_region_box_pressed),
        OnPointerMoved(on_region_box_moved),
        ChildOf(region_container),
    ));

    // --- 4. カラーマップリージョン (ColorMap) ---
    // demo_region_colormap_64x64.png: 赤=top-left, 緑=top-right, 青=bottom-left, 黄=bottom-right
    const COLORMAP_IMAGE_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/assets/demo_region_colormap_64x64.png"
    );
    let mut colormap_mapping = std::collections::HashMap::new();
    colormap_mapping.insert((255, 0, 0), "top-left".to_string());
    colormap_mapping.insert((0, 255, 0), "top-right".to_string());
    colormap_mapping.insert((0, 0, 255), "bottom-left".to_string());
    colormap_mapping.insert((255, 255, 0), "bottom-right".to_string());

    let colormap_region_map =
        HitRegionMap::from_color_map(std::path::Path::new(COLORMAP_IMAGE_PATH), &colormap_mapping)
            .expect("ColorMap region map load failed");

    world.spawn((
        Name::new("RegionColorMapBox"),
        RegionColorMapBox,
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.8,
            g: 0.8,
            b: 0.6,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(140.0)),
                height: Some(Dimension::Px(150.0)),
            }),
            ..Default::default()
        },
        HitTest::named_regions(),
        colormap_region_map,
        OnPointerPressed(on_region_box_pressed),
        OnPointerMoved(on_region_box_moved),
        ChildOf(region_container),
    ));

    // --- 5. フォールバックテスト (NamedRegions without HitRegionMap) ---
    // HitRegionMap なし → Bounds フォールバック（region: None）
    world.spawn((
        Name::new("RegionFallbackBox"),
        RegionFallbackBox,
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.7,
            g: 0.7,
            b: 0.7,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(140.0)),
                height: Some(Dimension::Px(150.0)),
            }),
            ..Default::default()
        },
        HitTest::named_regions(),
        // HitRegionMap は意図的に付与しない → Bounds フォールバック
        OnPointerPressed(on_region_box_pressed),
        OnPointerMoved(on_region_box_moved),
        ChildOf(region_container),
    ));

    // =======================================================================
    // 下段2: クリックスルーテストコンテナ
    // =======================================================================
    let click_through_container = world
        .spawn((
            Name::new("ClickThrough-Container"),
            ClickThroughTestContainer,
            Visual {
                opacity: 0.3,
                ..Default::default()
            },
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.2,
                g: 0.2,
                b: 0.2,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Row),
                justify_content: Some(taffy::JustifyContent::SpaceEvenly),
                align_items: Some(taffy::AlignItems::Center),
                size: Some(BoxSize {
                    width: None,
                    height: Some(Dimension::Px(120.0)),
                }),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    right: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    top: wintf::ecs::layout::LengthPercentageAuto::Px(0.0),
                    bottom: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                })),
                ..Default::default()
            },
            HitTest::none(),
            ChildOf(window_entity),
        ))
        .id();

    // --- クリックスルー領域 (HitTest::none) ---
    // 黄色半透明矩形、マウスイベントが貫通する
    world.spawn((
        Name::new("ClickThroughBox"),
        ClickThroughBox,
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 0.7,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(150.0)),
                height: Some(Dimension::Px(100.0)),
            }),
            ..Default::default()
        },
        HitTest::none(),
        ChildOf(click_through_container),
    ));

    // --- 通常領域 (HitTest::bounds) ---
    // シアン半透明矩形、マウスイベントを受け取る
    world.spawn((
        Name::new("NormalHitBox"),
        NormalHitBox,
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.0,
            g: 0.4,
            b: 1.0,
            a: 0.7,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(150.0)),
                height: Some(Dimension::Px(100.0)),
            }),
            ..Default::default()
        },
        HitTest::bounds(),
        OnPointerPressed(on_normal_hit_box_pressed),
        ChildOf(click_through_container),
    ));

    // --- α境界値テスト (Opacity=0.5, foreground.a=1.0 → 合成α=0.5 → HTCLIENT) ---
    world.spawn((
        Name::new("AlphaBoundaryBox"),
        Rectangle::new(),
        Visual {
            opacity: 0.5,
            ..Default::default()
        },
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.5,
            g: 0.0,
            b: 0.5,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(150.0)),
                height: Some(Dimension::Px(100.0)),
            }),
            ..Default::default()
        },
        HitTest::bounds(),
        OnPointerPressed(on_normal_hit_box_pressed),
        ChildOf(click_through_container),
    ));

    println!("[Test] Flexbox demo window created: {}", title);
    println!("  Window (root) - entity={:?}", window_entity);
    println!("  ├─ FlexContainer (Row, SpaceEvenly, Center) - 灰色背景");
    println!("  │  ├─ Rectangle (red, 200x100 fixed) - 左クリックで色トグル");
    println!("  │  │   └─ BitmapSource (seikatu_0_0.webp) - αマスクヒットテスト有効");
    println!("  │  ├─ Rectangle (green, 100x100, grow=1) - Tunnelキャプチャデモ");
    println!("  │  │   └─ Rectangle (yellow, 50x50)");
    println!("  │  └─ Rectangle (blue, 100x100, grow=2) - サイズトグル");
    println!("  ├─ RegionTest-Container (Row, SpaceEvenly) - 名前付きヒット領域テスト");
    println!("  │  ├─ RegionRectBox (矩形: top-left/top-right/bottom-left/bottom-right)");
    println!("  │  ├─ RegionPolygonBox (多角形: top-left/top-right/bottom-left/bottom-right)");
    println!("  │  ├─ RegionMixedBox (混在: rect+polygonで4分割)");
    println!("  │  ├─ RegionColorMapBox (カラーマップ: 4色→4リージョン)");
    println!("  │  └─ RegionFallbackBox (フォールバック: region=None)");
    println!("  └─ ClickThrough-Container (Row, SpaceEvenly) - クリックスルーテスト");
    println!("     ├─ ClickThroughBox (黄色, HitTest::none) - クリック貫通");
    println!("     ├─ NormalHitBox (シアン, HitTest::bounds) - 通常クリック");
    println!("     └─ AlphaBoundaryBox (紫, Opacity=0.5) - α境界値テスト");

    window_entity
}
