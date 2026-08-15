//! # Monitor階層システムの統合テスト
//!
//! 仮想デスクトップ・モニター階層システムの動作を検証する。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use windows::Win32::Foundation::RECT;
use wintf::ecs::*;

/// テスト用の合成 Monitor（実 HMONITOR 不要）。bounds から physical_size/top_left が導出される。
fn make_test_monitor(handle: isize, left: i32, top: i32, right: i32, bottom: i32) -> Monitor {
    Monitor {
        handle,
        bounds: RECT {
            left,
            top,
            right,
            bottom,
        },
        work_area: RECT {
            left,
            top,
            right,
            bottom,
        },
        dpi: 96,
        is_primary: true,
    }
}

// ===== Task 11.1: LayoutRoot Singleton生成とMonitor列挙テスト =====

#[test]
fn test_layout_root_singleton_creation() {
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());

    // initialize_layout_rootを実行
    initialize_layout_root(&mut world);

    // LayoutRootが1つだけ生成されることを検証
    let layout_roots: Vec<Entity> = world
        .query_filtered::<Entity, With<LayoutRoot>>()
        .iter(&world)
        .collect();
    assert_eq!(
        layout_roots.len(),
        1,
        "LayoutRoot should be created exactly once"
    );

    // 2回目の実行でもLayoutRootが1つだけであることを検証（既に存在する場合はスキップ）
    initialize_layout_root(&mut world);
    let layout_roots: Vec<Entity> = world
        .query_filtered::<Entity, With<LayoutRoot>>()
        .iter(&world)
        .collect();
    assert_eq!(
        layout_roots.len(),
        1,
        "LayoutRoot should not be duplicated on subsequent runs"
    );
}

#[test]
fn test_monitor_enumeration() {
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());

    // initialize_layout_rootを実行
    initialize_layout_root(&mut world);

    // Monitorエンティティが生成されることを検証
    let monitor_count = world.query::<&Monitor>().iter(&world).count();

    // システムに少なくとも1つのモニターが存在することを検証
    assert!(
        monitor_count >= 1,
        "At least one monitor should be enumerated"
    );

    // LayoutRootエンティティを取得
    let mut layout_root_query = world.query_filtered::<Entity, With<LayoutRoot>>();
    let layout_root_entity = layout_root_query
        .iter(&world)
        .next()
        .expect("LayoutRoot should exist");

    // 各MonitorエンティティがLayoutRootの子であることを検証
    for (entity, monitor) in world.query::<(Entity, &Monitor)>().iter(&world) {
        let child_of = world
            .get::<ChildOf>(entity)
            .expect("Monitor should have ChildOf component");
        assert_eq!(
            child_of.parent(),
            layout_root_entity,
            "Monitor should be a child of LayoutRoot"
        );

        // Monitor情報の妥当性を検証
        assert!(monitor.dpi > 0, "Monitor DPI should be positive");
        assert!(
            monitor.bounds.right > monitor.bounds.left,
            "Monitor bounds width should be positive"
        );
        assert!(
            monitor.bounds.bottom > monitor.bounds.top,
            "Monitor bounds height should be positive"
        );
    }
}

// ===== Task 11.2: LayoutRoot → {Monitor, Window} → Widget階層構築テスト =====

#[test]
fn test_monitor_hierarchy_construction() {
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());

    // initialize_layout_rootを実行
    initialize_layout_root(&mut world);

    // LayoutRootを取得
    let mut layout_root_query = world.query_filtered::<Entity, With<LayoutRoot>>();
    let layout_root = layout_root_query
        .iter(&world)
        .next()
        .expect("LayoutRoot should exist");

    // Monitorエンティティを取得
    let monitors: Vec<Entity> = world
        .query_filtered::<Entity, With<Monitor>>()
        .iter(&world)
        .collect();

    // 各MonitorがLayoutRootの子であることを再検証
    for monitor_entity in monitors.iter() {
        let child_of = world
            .get::<ChildOf>(*monitor_entity)
            .expect("Monitor should have ChildOf");
        assert_eq!(child_of.parent(), layout_root);
    }

    // Monitorに必要なレイアウトコンポーネントが存在することを検証
    for monitor_entity in monitors.iter() {
        // BoxStyleコンポーネントが存在し、position, size, insetが設定されていることを検証
        let box_style = world
            .get::<BoxStyle>(*monitor_entity)
            .expect("Monitor should have BoxStyle component");
        assert!(
            box_style.position.is_some(),
            "Monitor BoxStyle should have position"
        );
        assert!(
            box_style.size.is_some(),
            "Monitor BoxStyle should have size"
        );
        assert!(
            box_style.inset.is_some(),
            "Monitor BoxStyle should have inset"
        );
        assert!(
            world.get::<Arrangement>(*monitor_entity).is_some(),
            "Monitor should have Arrangement component"
        );
        assert!(
            world.get::<GlobalArrangement>(*monitor_entity).is_some(),
            "Monitor should have GlobalArrangement component"
        );
    }
}

// ===== Task 12.1: Monitor.boundsからTaffyStyle変換テスト =====

#[test]
fn test_monitor_to_taffy_style_conversion() {
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());

    // initialize_layout_rootを実行
    initialize_layout_root(&mut world);

    // build_taffy_styles_systemを実行してTaffyStyleを生成
    let mut schedule2 = Schedule::default();
    schedule2.add_systems(build_taffy_styles_system);
    schedule2.run(&mut world);

    // 各MonitorのTaffyStyleを検証
    let mut has_monitors = false;
    for (_monitor, box_style, _taffy_style) in world
        .query::<(&Monitor, &BoxStyle, &TaffyStyle)>()
        .iter(&world)
    {
        has_monitors = true;

        // BoxPosition::Absoluteが設定されていることを検証
        assert_eq!(box_style.position, Some(BoxPosition::Absolute));
        println!("Monitor has TaffyStyle");
    }

    assert!(has_monitors, "At least one monitor should have TaffyStyle");
}

// ===== Task 13.1: Taffyツリー同期とレイアウト計算テスト =====

#[test]
fn test_taffy_tree_sync_and_layout_computation() {
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());

    // LayoutRootとMonitorを初期化
    initialize_layout_root(&mut world);

    // TaffyStyleを構築
    let mut schedule2 = Schedule::default();
    schedule2.add_systems(build_taffy_styles_system);
    schedule2.run(&mut world);

    // Taffyツリーを同期
    let mut schedule3 = Schedule::default();
    schedule3.add_systems(sync_taffy_tree_system);
    schedule3.run(&mut world);

    // LayoutRootとMonitorのEntityを先に取得
    let layout_root = world
        .query_filtered::<Entity, With<LayoutRoot>>()
        .iter(&world)
        .next()
        .expect("LayoutRoot should exist");

    let monitors: Vec<Entity> = world
        .query_filtered::<Entity, With<Monitor>>()
        .iter(&world)
        .collect();

    // Entity↔NodeIdマッピングの検証
    {
        let taffy_res = world.resource::<TaffyLayoutResource>();

        // LayoutRootのマッピング検証
        assert!(
            taffy_res.get_node(layout_root).is_some(),
            "LayoutRoot should have a Taffy node"
        );

        // Monitorのマッピング検証
        for monitor_entity in monitors.iter() {
            assert!(
                taffy_res.get_node(*monitor_entity).is_some(),
                "Monitor should have a Taffy node"
            );
        }
    }

    // レイアウト計算を実行
    let mut schedule4 = Schedule::default();
    schedule4.add_systems(compute_taffy_layout_system);
    schedule4.run(&mut world);

    // TaffyComputedLayoutが配布されていることを検証
    let _layout_root_computed = world
        .get::<TaffyComputedLayout>(layout_root)
        .expect("LayoutRoot should have TaffyComputedLayout");

    for monitor_entity in monitors.iter() {
        let _monitor_computed = world
            .get::<TaffyComputedLayout>(*monitor_entity)
            .expect("Monitor should have TaffyComputedLayout");

        // レイアウトが計算されていることを検証（内部構造にアクセスできないため存在確認のみ）
        println!("Monitor {:?} has computed layout", monitor_entity);
    }
}

// ===== Task 14.1: DisplayConfigurationChangedフラグテスト =====

#[test]
fn test_display_configuration_changed_flag() {
    let mut app = App::new();

    // 初期状態ではフラグがfalse
    assert!(
        !app.display_configuration_changed(),
        "Flag should be false initially"
    );

    // mark_display_change()でフラグがtrueになる
    app.mark_display_change();
    assert!(
        app.display_configuration_changed(),
        "Flag should be true after mark_display_change"
    );

    // reset_display_change()でフラグがfalseになる
    app.reset_display_change();
    assert!(
        !app.display_configuration_changed(),
        "Flag should be false after reset_display_change"
    );
}

// ===== Task 14.2: モニター追加・削除・更新テスト =====

#[test]
fn test_monitor_update_on_change() {
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());
    world.insert_resource(App::new());
    // 遷移観測の刻印（D1）: `detect_display_change_system` は World 資源から
    // 刻印を組むため、`EcsWorld::new` が入れる 2 資源を素の World にも用意する。
    world.insert_resource(wintf::ecs::world::FrameCount::default());
    world.insert_resource(wintf::ecs::window::transition_diag::TickStart(
        std::time::Instant::now(),
    ));

    // LayoutRootとMonitorを初期化
    initialize_layout_root(&mut world);

    // 初期のMonitor数を取得
    let initial_count = world.query::<&Monitor>().iter(&world).count();

    // ディスプレイ構成変更をシミュレート
    {
        let mut app = world.resource_mut::<App>();
        app.mark_display_change();
    }

    // detect_display_change_systemを実行
    let mut schedule2 = Schedule::default();
    schedule2.add_systems(detect_display_change_system);
    schedule2.run(&mut world);

    // フラグがリセットされていることを検証
    let app = world.resource::<App>();
    assert!(
        !app.display_configuration_changed(),
        "Flag should be reset after detect_display_change_system"
    );

    // Monitor数が維持されていることを検証（実際の環境では変化しない想定）
    let current_count = world.query::<&Monitor>().iter(&world).count();
    assert_eq!(
        initial_count, current_count,
        "Monitor count should remain the same in stable environment"
    );
}

// ===== Task 15.1: 既存システム互換性テスト =====

#[test]
fn test_backward_compatibility_without_layout_root() {
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());

    // LayoutRootなしでWidgetエンティティを作成
    let widget = world
        .spawn((
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(100.0)),
                    height: Some(Dimension::Px(50.0)),
                }),
                ..Default::default()
            },
            Arrangement::default(),
            GlobalArrangement::default(),
        ))
        .id();

    // build_taffy_styles_systemを実行
    let mut schedule = Schedule::default();
    schedule.add_systems(build_taffy_styles_system);
    schedule.run(&mut world);

    // TaffyStyleが自動生成されることを検証
    assert!(
        world.get::<TaffyStyle>(widget).is_some(),
        "TaffyStyle should be auto-generated even without LayoutRoot"
    );

    // sync_taffy_tree_systemを実行
    let mut schedule2 = Schedule::default();
    schedule2.add_systems(sync_taffy_tree_system);
    schedule2.run(&mut world);

    // Taffyノードが作成されることを検証
    let taffy_res = world.resource::<TaffyLayoutResource>();
    assert!(
        taffy_res.get_node(widget).is_some(),
        "Taffy node should be created even without LayoutRoot"
    );
}

#[test]
fn test_existing_tests_still_pass() {
    // 既存のレイアウトシステムが正常に動作することを検証
    let mut world = World::new();
    world.insert_resource(TaffyLayoutResource::default());

    // LayoutRootを作成（新システム）
    world.insert_resource(App::new());
    initialize_layout_root(&mut world);

    // 既存のWindow/Widgetエンティティを作成
    let window = world
        .spawn((
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(800.0)),
                    height: Some(Dimension::Px(600.0)),
                }),
                ..Default::default()
            },
            Arrangement::default(),
            GlobalArrangement::default(),
        ))
        .id();

    let widget = world
        .spawn((
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(200.0)),
                    height: Some(Dimension::Px(100.0)),
                }),
                ..Default::default()
            },
            ChildOf(window),
            Arrangement::default(),
            GlobalArrangement::default(),
        ))
        .id();

    // レイアウトシステムを実行
    let mut schedule2 = Schedule::default();
    schedule2.add_systems((
        build_taffy_styles_system,
        sync_taffy_tree_system,
        compute_taffy_layout_system,
    ));
    schedule2.run(&mut world);

    // Window/Widgetのレイアウトが正しく計算されることを検証
    assert!(
        world.get::<TaffyComputedLayout>(window).is_some(),
        "Window should have computed layout"
    );
    assert!(
        world.get::<TaffyComputedLayout>(widget).is_some(),
        "Widget should have computed layout"
    );
}

// ===== W4b-T: update_monitor_layout_system 単独テスト =====
// 既存テストは initialize_layout_root / detect_display_change_system を固定していたが、
// Monitor 変更時に BoxStyle.size/inset を再計算する update_monitor_layout_system は
// 直接検証されていなかった。実 HMONITOR 不要のデバイス非依存ロジックのため特性化する。

/// Changed<Monitor> 検出時に BoxStyle.size と inset が bounds から再計算される
#[test]
fn test_update_monitor_layout_recomputes_box_style() {
    let mut world = World::new();

    // Monitor + 空の BoxStyle を spawn（spawn 時の Added は Changed として扱われる）
    let monitor = make_test_monitor(1, 100, 200, 1380, 1000); // 幅1280 x 高さ800, 左上(100,200)
    let entity = world.spawn((monitor, BoxStyle::default())).id();

    let mut schedule = Schedule::default();
    schedule.add_systems(update_monitor_layout_system);
    schedule.run(&mut world);

    let box_style = world.get::<BoxStyle>(entity).unwrap();

    // size = physical_size() = (1280, 800)
    let size = box_style.size.as_ref().expect("size should be set");
    assert_eq!(size.width, Some(Dimension::Px(1280.0)));
    assert_eq!(size.height, Some(Dimension::Px(800.0)));

    // inset = top_left() を Px、right/bottom は Auto
    let inset = box_style.inset.as_ref().expect("inset should be set");
    assert_eq!(inset.0.left, LengthPercentageAuto::Px(100.0));
    assert_eq!(inset.0.top, LengthPercentageAuto::Px(200.0));
    assert_eq!(inset.0.right, LengthPercentageAuto::Auto);
    assert_eq!(inset.0.bottom, LengthPercentageAuto::Auto);
}

/// Monitor 未変更時は何も起きない（Changed フィルタにより BoxStyle は再計算されない）
#[test]
fn test_update_monitor_layout_skips_unchanged() {
    let mut world = World::new();

    let monitor = make_test_monitor(1, 0, 0, 800, 600);
    let entity = world.spawn((monitor, BoxStyle::default())).id();

    let mut schedule = Schedule::default();
    schedule.add_systems(update_monitor_layout_system);

    // 1回目: Added → 処理されて size/inset が設定される
    schedule.run(&mut world);
    assert!(world.get::<BoxStyle>(entity).unwrap().size.is_some());

    // BoxStyle を None に戻し、Monitor を変更せず再実行
    world.entity_mut(entity).get_mut::<BoxStyle>().unwrap().size = None;

    // 2回目: Monitor は未変更（Changed なし）→ システムは対象外でスキップ
    schedule.run(&mut world);
    assert!(
        world.get::<BoxStyle>(entity).unwrap().size.is_none(),
        "未変更 Monitor は再計算されず、手動でリセットした size は None のまま"
    );
}
