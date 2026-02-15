#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # Taffy Flexbox Demo - Tunnel/Bubbleフェーズのイベント伝播デモ + ドラッグ移動
//!
//! このサンプルは、wintfフレームワークのポインターイベントシステムにおける
//! Tunnel（親→子）とBubble（子→親）の2フェーズイベント伝播を実演します。
//! また、ウィンドウドラッグ移動機能も実装されています。
//!
//! ## イベントフェーズの概念
//!
//! wintfのイベントシステムは、WinUI3/WPF/DOMイベントモデルと同様の2フェーズを実装:
//!
//! - **Tunnelフェーズ** (親→子): イベント発生前に親が介入可能
//! - **Bubbleフェーズ** (子→親): イベント発生後に親が処理可能
//!
//! ## 他のフレームワークとの対応表
//!
//! | wintf             | WinUI3           | WPF              | DOM Level 3      |
//! |-------------------|------------------|------------------|------------------|
//! | Phase::Tunnel     | PreviewMouseDown | PreviewMouseDown | Capture Phase    |
//! | Phase::Bubble     | MouseDown        | MouseDown        | Bubble Phase     |
//! | handler return    | e.Handled = true | e.Handled = true | stopPropagation()|
//! | sender引数        | e.OriginalSource | e.OriginalSource | event.target     |
//! | entity引数        | sender引数       | sender引数       | currentTarget    |
//!
//! ## デモの操作例
//!
//! 1. **FlexDemo-Container（灰色背景）を左クリック＆ドラッグ**
//!    - 期待: ウィンドウがドラッグ移動する
//!    - ログ: `[Drag] DragStart/Drag/DragEnd` が出力される
//!
//! 2. **GreenBoxChild（黄色矩形）を左クリック**
//!    - 期待: `[Tunnel] GreenBox: Captured event` のみ出力
//!    - GreenBoxChildのログは出ない（親がTunnelでキャプチャ）
//!
//! 3. **GreenBoxChild（黄色矩形）を右クリック**
//!    - 期待: `[Tunnel] GreenBox` → `[Tunnel] GreenBoxChild` → `[Bubble] GreenBoxChild`
//!    - 両エンティティがログ出力（親がキャプチャしない）
//!
//! 4. **Ctrl+左クリックでRedBox**
//!    - 期待: `[Tunnel] FlexContainer: Event stopped` のみ出力
//!    - RedBoxのログは出ない（Containerで停止）
//!
//! ## 実装パターン
//!
//! ```rust
//! fn handler(world: &mut World, sender: Entity, entity: Entity, ev: &Phase<PointerState>) -> bool {
//!     match ev {
//!         Phase::Tunnel(state) => {
//!             if state.ctrl_down && state.left_down {
//!                 // 親で事前処理してイベントを停止
//!                 return true; // stopPropagation相当
//!             }
//!             false
//!         }
//!         Phase::Bubble(state) => {
//!             // 通常のイベント処理
//!             if state.right_down {
//!                 // 処理...
//!                 return true;
//!             }
//!             false
//!         }
//!     }
//! }
//! ```

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::time::Duration;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::core::Result;
use wintf::ecs::drag::{
    DragConfig, DragEndEvent, DragEvent, DragStartEvent, OnDrag, OnDragEnd, OnDragStart,
};
use wintf::ecs::layout::hit_region::HitRegionMap;
use wintf::ecs::layout::{BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension, Opacity};
use wintf::ecs::layout::{
    GlobalArrangement, HitTest, PhysicalPoint, hit_test, hit_test_in_window_ex,
};
use wintf::ecs::pointer::{OnPointerMoved, OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::{BitmapSource, CommandSender};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::window::find_owner_window;
use wintf::ecs::{Window, WindowPos};
use wintf::*;

#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct FlexDemoWindow;

/// イベントハンドラ用ウィンドウ識別文字列を返す
fn window_label(world: &World, entity: Entity) -> String {
    match find_owner_window(world, entity) {
        Some(win) => {
            let title = world
                .get::<Window>(win)
                .map(|w| w.title.as_str())
                .unwrap_or("?");
            format!("win={:?}({})", win, title)
        }
        None => "win=None".to_string(),
    }
}

/// Flexコンテナを識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct FlexDemoContainer;

/// 赤い矩形（固定サイズ）を識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct RedBox;

/// 緑の矩形（grow=1）を識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct GreenBox;

/// 青い矩形（grow=2）を識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct BlueBox;

/// GreenBoxの子矩形を識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct GreenBoxChild;

/// リージョンテストコンテナを識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct RegionTestContainer;

/// 矩形リージョンテスト用ボックス
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct RegionRectBox;

/// 多角形リージョンテスト用ボックス
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct RegionPolygonBox;

/// 混在（矩形+多角形）リージョンテスト用ボックス
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct RegionMixedBox;

/// カラーマップリージョンテスト用ボックス
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct RegionColorMapBox;

/// フォールバックテスト用ボックス（HitRegionMap なし）
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct RegionFallbackBox;

/// クリックスルーテストコンテナを識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct ClickThroughTestContainer;

/// クリックスルー領域（HitTest::none）を識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct ClickThroughBox;

/// 通常領域（HitTest::bounds）を識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct NormalHitBox;

fn main() -> Result<()> {
    human_panic::setup_panic!();

    // tracing-subscriber 初期化
    // RUST_LOG環境変数で制御: 例 RUST_LOG=wintf=debug,info
    // 環境変数未設定時はデフォルトでinfoレベル
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinThreadMgr::new()?;
    let world = mgr.world();

    // 非同期タスクでFlexboxデモを実行
    world.borrow().spawn(|tx| async move {
        run_demo(tx).await;
    });

    println!("\nTaffy Flexboxレイアウトのデモ:");
    println!("  [上段] イベントシステムデモ（既存）");
    println!("    - 赤い矩形 (固定200x100) / 緑の矩形 (grow=1) / 青い矩形 (grow=2)");
    println!("  [下段] 名前付きヒット領域テスト（新規）");
    println!("    - 矩形リージョン (head/body/feet)");
    println!("    - 多角形リージョン (triangle/pentagon)");
    println!("    - 混在＋重複 (banner/overlap_zone/arrow)");
    println!("    - カラーマップ (head/body/feet/hand)");
    println!("    - フォールバック (NamedRegions without HitRegionMap)");
    println!("\n5秒後にレイアウトパラメーターを変更します。");
    println!("10秒後に自動的にWindowを閉じてアプリ終了します。");

    // メッセージループを開始
    mgr.run()?;

    Ok(())
}

/// 非同期デモ実行
async fn run_demo(tx: CommandSender) {
    // === 0秒: ウィンドウ作成（2つ） ===
    println!("[Async] 0s: Creating Flexbox demo windows (multi-window)");
    let _ = tx.send(Box::new(|world: &mut World| {
        create_flexbox_window(
            world,
            "wintf - Taffy Flexbox Demo (Window 1)",
            windows::Win32::Foundation::POINT { x: 0, y: 0 },
        );
        create_flexbox_window(
            world,
            "wintf - Taffy Flexbox Demo (Window 2)",
            windows::Win32::Foundation::POINT { x: 850, y: 0 },
        );
    }));

    // === 1秒待機 ===
    async_io::Timer::after(Duration::from_secs(1)).await;

    // === 1秒: ヒットテスト検証（Window 1のみ） ===
    println!("[Async] 1s: Running hit test verification");
    let _ = tx.send(Box::new(test_hit_test_1s));

    // === 長時間待機（ポインターイベントデモ用） ===
    println!("[Async] Waiting 60 seconds for pointer event demo...");
    println!("  [上段] Left-click on RedBox, BlueBox, Right-click on Container");
    println!("  [下段] Left-click on region boxes to test hit regions");
    println!("  Hover over region boxes to see region names in debug log");
    println!("  Verify: Events in Window 1 don't affect Window 2 and vice versa");
    async_io::Timer::after(Duration::from_secs(60)).await;

    // === 61秒: ウィンドウ終了 ===
    println!("[Async] 61s: Closing windows");
    let _ = tx.send(Box::new(close_window));
}

/// Flexboxデモウィンドウを作成（パラメータ化）
fn create_flexbox_window(
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
                    height: None,
                }),
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
            Opacity(1.0),
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
            Opacity(0.5),
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
        Opacity(0.5),
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
        Opacity(0.5),
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
        Opacity(0.5),
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

/// レイアウトパラメーターを変更
#[allow(dead_code)]
fn change_layout_parameters(world: &mut World) {
    // WindowエンティティのBoxStyleを変更してウィンドウを移動・リサイズ
    let mut window_query = world.query_filtered::<&mut BoxStyle, With<FlexDemoWindow>>();
    if let Some(mut style) = window_query.iter_mut(world).next() {
        style.size = Some(BoxSize {
            width: Some(Dimension::Px(600.0)),
            height: Some(Dimension::Px(400.0)),
        });
        println!("[Test] Window BoxStyle changed: size=(600,400) in DIP");
    }

    // WindowPos.position を変更してウィンドウを移動
    let mut wp_query = world.query_filtered::<&mut WindowPos, With<FlexDemoWindow>>();
    if let Some(mut wp) = wp_query.iter_mut(world).next() {
        wp.position = Some(windows::Win32::Foundation::POINT { x: -500, y: 400 });
        println!("[Test] Window position changed to (-500, 400) via WindowPos");
    }

    // FlexContainerを縦並びに変更
    let mut container_query = world.query_filtered::<&mut BoxStyle, With<FlexDemoContainer>>();
    if let Some(mut style) = container_query.iter_mut(world).next() {
        style.flex_direction = Some(taffy::FlexDirection::Column);
        style.justify_content = Some(taffy::JustifyContent::SpaceAround);
        println!("[Test] FlexContainer direction changed to Column");
    }

    // 赤い矩形のサイズを変更
    let mut red_query = world.query_filtered::<&mut BoxStyle, With<RedBox>>();
    if let Some(mut style) = red_query.iter_mut(world).next() {
        if let Some(ref mut size) = style.size {
            size.width = Some(Dimension::Px(150.0));
            size.height = Some(Dimension::Px(80.0));
        }
        println!("[Test] RedBox size changed to 150x80");
    }

    // 緑の矩形のgrowを変更
    let mut green_query = world.query_filtered::<&mut BoxStyle, With<GreenBox>>();
    if let Some(mut style) = green_query.iter_mut(world).next() {
        style.flex_grow = Some(2.0);
        println!("[Test] GreenBox grow changed to 2.0");
    }

    // 青い矩形のgrowを変更
    let mut blue_query = world.query_filtered::<&mut BoxStyle, With<BlueBox>>();
    if let Some(mut style) = blue_query.iter_mut(world).next() {
        style.flex_grow = Some(1.0);
        println!("[Test] BlueBox grow changed to 1.0");
    }

    println!("[Test] Layout parameters changed:");
    println!("  FlexContainer: Row → Column, SpaceEvenly → SpaceAround");
    println!("  RedBox: 200x100 → 150x80");
    println!("  GreenBox: grow 1.0 → 2.0");
    println!("  BlueBox: grow 2.0 → 1.0");
}

/// ウィンドウを閉じる
fn close_window(world: &mut World) {
    let mut query = world.query_filtered::<Entity, With<FlexDemoWindow>>();
    let windows: Vec<Entity> = query.iter(world).collect();
    for window in windows {
        println!("[Test] Removing Window entity {:?}", window);
        world.despawn(window);
    }
}

/// 1秒後のヒットテスト検証
fn test_hit_test_1s(world: &mut World) {
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
fn test_region_hit_ex(
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
fn test_hit_test_6s(world: &mut World) {
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
fn dump_window_pos(world: &World, entity: Entity) {
    if let Some(window_pos) = world.get::<WindowPos>(entity) {
        if let Some(pos) = window_pos.position {
            println!("[HitTest] WindowPos.position: x={}, y={}", pos.x, pos.y);
        } else {
            println!("[HitTest] WindowPos.position: None");
        }
        if let Some(size) = window_pos.size {
            println!("[HitTest] WindowPos.size: cx={}, cy={}", size.cx, size.cy);
        } else {
            println!("[HitTest] WindowPos.size: None");
        }
    } else {
        println!("[HitTest] Window has no WindowPos");
    }
}

// ============================================================================
// ポインターイベントハンドラ
// ============================================================================

/// FlexContainer の OnPointerPressed ハンドラ（拡張版）
///
/// **Tunnelフェーズ**: Ctrl+左クリックでキャプチャ（条件付き前処理の例）
/// **Bubbleフェーズ**: 右クリックで色変更（既存）
///
/// # パラメータ
/// - `sender`: イベント発生元エンティティ（e.OriginalSource相当）
/// - `entity`: 現在処理中のエンティティ（e.currentTarget相当）
/// - `ev`: Tunnel/Bubbleフェーズを含むイベント情報
///
/// # 戻り値
/// - `true`: イベント伝播を停止（stopPropagation相当）
/// - `false`: イベント伝播を継続
fn on_container_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let wlabel = window_label(world, entity);
    match ev {
        Phase::Tunnel(state) => {
            // Ctrl+左クリックでイベントを停止
            if state.ctrl_down && state.left_down {
                info!(
                    "[Tunnel] FlexContainer: Event stopped at Container (Ctrl+Left), sender={:?}, entity={:?}, {}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    wlabel,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                // コンテナの色をピンクに変更
                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                        r: 1.0,
                        g: 0.4,
                        b: 0.8,
                        a: 1.0,
                    });
                }

                return true; // イベント停止、子に到達しない
            }

            info!(
                "[Tunnel] FlexContainer: Passing through, sender={:?}, entity={:?}, {}",
                sender, entity, wlabel,
            );
            false
        }
        Phase::Bubble(state) => {
            // 右クリック検出
            if state.right_down {
                info!(
                    "[Bubble] FlexContainer: Right-click detected! sender={:?}, entity={:?}, {}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    wlabel,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                // コンテナの色をピンクに変更
                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                        r: 1.0,
                        g: 0.7,
                        b: 0.8,
                        a: 1.0,
                    });
                }

                return true; // イベント処理済み
            }

            false
        }
    }
}

/// FlexContainer の OnDragStart ハンドラ
///
/// ドラッグ開始時に初期inset値を記録する。
fn on_container_drag_start(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragStartEvent>,
) -> bool {
    match ev {
        wintf::ecs::pointer::Phase::Tunnel(_) => false,
        wintf::ecs::pointer::Phase::Bubble(event) => {
            let sender_name = world
                .get::<Name>(sender)
                .map(|n| n.as_str())
                .unwrap_or("unknown");
            let entity_name = world
                .get::<Name>(entity)
                .map(|n| n.as_str())
                .unwrap_or("unknown");

            info!(
                "[Drag] DragStart: sender={}, entity={}, pos=({},{})",
                sender_name, entity_name, event.position.x, event.position.y
            );

            // ウィンドウエンティティを探索してドラッグ開始位置を記録
            // これはDraggingStateとして保存される（DraggingStateには既にdrag_start_posがある）

            false
        }
    }
}

/// FlexContainer の OnDrag ハンドラ
///
/// ドラッグ中のログ出力を行う。
/// ウィンドウ位置の更新はフレームワークのWndProcレベル直接SetWindowPosが
/// 自動的に処理する（DragConfig.move_window = true）。
fn on_container_drag(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragEvent>,
) -> bool {
    match ev {
        wintf::ecs::pointer::Phase::Tunnel(_) => false,
        wintf::ecs::pointer::Phase::Bubble(event) => {
            let sender_name = world
                .get::<Name>(sender)
                .map(|n| n.as_str())
                .unwrap_or("unknown");
            let entity_name = world
                .get::<Name>(entity)
                .map(|n| n.as_str())
                .unwrap_or("unknown");

            // start_positionとpositionから移動量を計算（ログ出力用）
            let delta_x = event.position.x - event.start_position.x;
            let delta_y = event.position.y - event.start_position.y;

            debug!(
                "[Drag] Drag: sender={}, entity={}, pos=({},{}), delta=({},{})",
                sender_name, entity_name, event.position.x, event.position.y, delta_x, delta_y
            );

            false
        }
    }
}

/// FlexContainer の OnDragEnd ハンドラ
fn on_container_drag_end(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragEndEvent>,
) -> bool {
    match ev {
        wintf::ecs::pointer::Phase::Tunnel(_) => false,
        wintf::ecs::pointer::Phase::Bubble(event) => {
            let sender_name = world
                .get::<Name>(sender)
                .map(|n| n.as_str())
                .unwrap_or("unknown");
            let entity_name = world
                .get::<Name>(entity)
                .map(|n| n.as_str())
                .unwrap_or("unknown");

            info!(
                "[Drag] DragEnd: sender={}, entity={}, pos=({},{}), cancelled={}",
                sender_name, entity_name, event.position.x, event.position.y, event.cancelled
            );
            false
        }
    }
}

/// RedBox の OnPointerPressed ハンドラ
///
/// 左クリックで色をトグル（赤 ⇔ 黄）する。
/// αマスクヒットテストのデモ: 画像の透明部分をクリックすると
/// イベントが親(RedBox)に伝播してこのハンドラが呼ばれる。
fn on_red_box_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let wlabel = window_label(world, entity);
    // Bubble フェーズでのみ処理
    if !ev.is_bubble() {
        info!(
            "[Tunnel] RedBox: Passing through, sender={:?}, entity={:?}, {}",
            sender, entity, wlabel,
        );
        return false;
    }

    let state = ev.value();

    // 左クリック検出
    if state.left_down {
        info!(
            "[Bubble] RedBox: Left-click, sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1}), L={}, R={}, Ctrl={}",
            sender,
            entity,
            state.client_point.x,
            state.client_point.y,
            state.local_point.x,
            state.local_point.y,
            state.left_down,
            state.right_down,
            state.ctrl_down,
        );

        // 色をトグル（赤 ⇔ 黄）
        if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
            let is_red = match brushes.foreground.as_color() {
                Some(c) => c.r > 0.9 && c.g < 0.1,
                None => false,
            };
            if is_red {
                // 黄色に変更
                brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 0.0,
                    a: 1.0,
                });
                info!(
                    "[AlphaMask Demo] BACKGROUND clicked (transparent area) - color: RED -> YELLOW"
                );
            } else {
                // 赤に戻す
                brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                });
                info!(
                    "[AlphaMask Demo] BACKGROUND clicked (transparent area) - color: YELLOW -> RED"
                );
            }
        }

        return true; // イベント処理済み、親に伝播しない
    }

    false
}

/// SeikatuImage の OnPointerPressed ハンドラ
///
/// αマスクヒットテストのデモ用。
/// 不透明部分がクリックされた場合のみこのハンドラが呼ばれる。
/// イベントを消費して親(RedBox)に伝播させない。
fn on_image_pressed(
    _world: &mut World,
    _sender: Entity,
    _entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble フェーズでのみ処理
    if !ev.is_bubble() {
        return false;
    }

    let state = ev.value();

    // 左クリック検出
    if state.left_down {
        info!(
            "[AlphaMask Demo] IMAGE clicked (opaque area) - event consumed, background unchanged"
        );
        return true; // イベント処理済み、親(RedBox)に伝播しない
    }

    false
}

/// GreenBox の OnPointerPressed ハンドラ
///
/// **Tunnelフェーズ**: 左クリックでキャプチャし、子（GreenBoxChild）に到達させない
/// **Bubbleフェーズ**: 右クリックで色を変更
/// **ダブルクリック**: サイズを変更（100x100 ⇔ 150x150）
///
/// # stopPropagation使用例
/// Tunnelフェーズでtrueを返すことで、親エンティティが子のイベント処理前に
/// 介入できます。これはWinUI3/WPFの`PreviewMouseDown`やDOMの`Capture Phase`と
/// 同じ動作です。
///
/// # sender vs entity
/// - `sender`: 常にイベント発生元（例: GreenBoxChild）
/// - `entity`: 現在処理中のエンティティ（この場合はGreenBox）
fn on_green_box_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    match ev {
        Phase::Tunnel(state) => {
            // 左クリックでキャプチャ
            if state.left_down {
                tracing::info!(
                    double_click = ?state.double_click,
                    left_down = state.left_down,
                    "[Tunnel] GreenBox: Button pressed, checking double-click"
                );

                // ダブルクリック判定
                if state.double_click == wintf::ecs::pointer::DoubleClick::Left {
                    info!(
                        "[Tunnel] GreenBox: DOUBLE-CLICK detected, toggling size, sender={:?}, entity={:?}",
                        sender, entity,
                    );

                    // サイズをトグル（100x100 ⇔ 150x150）
                    if let Some(mut box_style) =
                        world.get_mut::<wintf::ecs::layout::BoxStyle>(entity)
                    {
                        let current_width = box_style
                            .size
                            .and_then(|s| s.width)
                            .and_then(|w| {
                                if let Dimension::Px(px) = w {
                                    Some(px)
                                } else {
                                    None
                                }
                            })
                            .unwrap_or(100.0);

                        let new_size = if current_width < 125.0 { 150.0 } else { 100.0 };
                        box_style.size = Some(wintf::ecs::layout::BoxSize {
                            width: Some(Dimension::Px(new_size)),
                            height: Some(Dimension::Px(new_size)),
                        });
                        info!(
                            "[Tunnel] GreenBox: Size changed {} -> {}",
                            current_width, new_size
                        );
                    }

                    return true;
                }

                // 通常の左クリック：色をトグル（緑 ⇔ 黄緑）
                info!(
                    "[Tunnel] GreenBox: Captured event, stopping propagation (Left), sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    let is_green = match brushes.foreground.as_color() {
                        Some(c) => c.r < 0.1 && c.g > 0.9,
                        None => false,
                    };
                    if is_green {
                        // 黄緑に変更
                        brushes.foreground =
                            wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                                r: 0.5,
                                g: 1.0,
                                b: 0.0,
                                a: 1.0,
                            });
                        info!("[Tunnel] GreenBox: Color changed GREEN -> YELLOW-GREEN");
                    } else {
                        // 緑に戻す
                        brushes.foreground =
                            wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                                r: 0.0,
                                g: 1.0,
                                b: 0.0,
                                a: 1.0,
                            });
                        info!("[Tunnel] GreenBox: Color changed YELLOW-GREEN -> GREEN");
                    }
                }

                return true; // イベント停止、子に到達しない
            }

            info!(
                "[Tunnel] GreenBox: Passing through, sender={:?}, entity={:?}",
                sender, entity,
            );
            false
        }
        Phase::Bubble(state) => {
            // 右クリック処理
            if state.right_down {
                info!(
                    "[Bubble] GreenBox: Right-click, sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                // 色を変更
                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                        r: 0.0,
                        g: 0.8,
                        b: 0.8,
                        a: 1.0,
                    });
                }

                return true;
            }

            false
        }
    }
}

/// GreenBoxChild の OnPointerPressed ハンドラ
///
/// 親（GreenBox）がTunnelでキャプチャした場合、このハンドラは呼ばれない。
/// 右クリック時は親がキャプチャしないため、Tunnel/Bubble両方で実行される。
///
/// # ev.value()の使用例
/// `Phase::Tunnel(state)` や `Phase::Bubble(state)` でパターンマッチする代わりに、
/// `ev.value()` で `PointerState` を直接取得できます。
fn on_green_child_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    let state = ev.value();

    match ev {
        Phase::Tunnel(_) => {
            info!(
                "[Tunnel] GreenBoxChild: This should NOT be called if parent captured (Left), sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1}), L={}, R={}, Ctrl={}",
                sender,
                entity,
                state.client_point.x,
                state.client_point.y,
                state.local_point.x,
                state.local_point.y,
                state.left_down,
                state.right_down,
                state.ctrl_down,
            );
            false
        }
        Phase::Bubble(_) => {
            // 右クリック処理
            if state.right_down {
                info!(
                    "[Bubble] GreenBoxChild: Right-click detected, changing to orange, sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1})",
                    sender,
                    entity,
                    state.client_point.x,
                    state.client_point.y,
                    state.local_point.x,
                    state.local_point.y,
                );

                // 色をオレンジに変更
                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                        r: 1.0,
                        g: 0.5,
                        b: 0.0,
                        a: 1.0,
                    });
                }

                return true;
            }

            false
        }
    }
}

/// GreenBox の OnPointerMoved ハンドラ
///
/// マウス移動時にログを出力する（デバッグ用）。
fn on_green_box_moved(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble フェーズでのみ処理（Tunnel でログ出力すると冗長）
    if !ev.is_bubble() {
        return false;
    }

    let wlabel = window_label(world, entity);
    let state = ev.value();

    // 10フレームに1回程度ログ出力（頻繁すぎないように）
    static MOVE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let count = MOVE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if count % 30 == 0 {
        info!(
            sender = ?sender,
            entity = ?entity,
            window = %wlabel,
            x = state.client_point.x,
            y = state.client_point.y,
            "[Bubble] GreenBox: Pointer moved"
        );
    }

    false // 伝播続行（親にも通知）
}

/// BlueBox の OnPointerPressed ハンドラ
///
/// 左クリックでサイズをトグル（100 ⇔ 150）する。
fn on_blue_box_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble フェーズでのみ処理
    if !ev.is_bubble() {
        info!(
            "[Tunnel] BlueBox: Passing through, sender={:?}, entity={:?}",
            sender, entity,
        );
        return false;
    }

    let state = ev.value();

    // 左クリック検出
    if state.left_down {
        info!(
            "[Bubble] BlueBox: Left-click detected! Toggling size, sender={:?}, entity={:?}, screen=({:.1},{:.1}), local=({:.1},{:.1}), L={}, R={}, Ctrl={}",
            sender,
            entity,
            state.client_point.x,
            state.client_point.y,
            state.local_point.x,
            state.local_point.y,
            state.left_down,
            state.right_down,
            state.ctrl_down,
        );

        // サイズをトグル
        if let Some(mut style) = world.get_mut::<BoxStyle>(entity) {
            if let Some(ref mut size) = style.size {
                let new_size = if size.width == Some(Dimension::Px(100.0)) {
                    150.0
                } else {
                    100.0
                };
                size.width = Some(Dimension::Px(new_size));
                size.height = Some(Dimension::Px(new_size));
                info!(new_size = new_size, "[PointerEvent] BlueBox: New size");
            }
        }

        return true; // イベント処理済み
    }

    false
}

// ============================================================================
// リージョンテスト用イベントハンドラ
// ============================================================================

/// リージョンテスト共通: hit_test_in_window_ex でリージョン名を取得するヘルパー
fn resolve_region_name(world: &World, entity: Entity, state: &PointerState) -> Option<String> {
    // ウィンドウエンティティを探索
    let window = find_owner_window(world, entity)?;
    // hit_test_in_window_ex でリージョン名を含む結果を取得
    let result = hit_test_in_window_ex(
        world,
        window,
        PhysicalPoint::new(state.client_point.x as f32, state.client_point.y as f32),
    )?;
    result.region
}

/// リージョンに基づく色を返す（視覚フィードバック用）
fn region_color(region: Option<&str>) -> D2D1_COLOR_F {
    match region {
        Some("top-left") => D2D1_COLOR_F {
            r: 1.0,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        }, // 赤
        Some("top-right") => D2D1_COLOR_F {
            r: 0.2,
            g: 0.8,
            b: 0.2,
            a: 1.0,
        }, // 緑
        Some("bottom-left") => D2D1_COLOR_F {
            r: 0.2,
            g: 0.2,
            b: 1.0,
            a: 1.0,
        }, // 青
        Some("bottom-right") => D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 0.2,
            a: 1.0,
        }, // 黄
        Some(other) => {
            println!("[Region] 不明なリージョン: {}", other);
            D2D1_COLOR_F {
                r: 0.9,
                g: 0.9,
                b: 0.9,
                a: 1.0,
            }
        }
        None => D2D1_COLOR_F {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        }, // 無名（フォールバック）
    }
}

/// リージョンテストボックス共通の OnPointerPressed ハンドラ
///
/// クリック時に hit_test_in_window_ex でリージョン名を取得し、色を変更＋ログ出力
fn on_region_box_pressed(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    // Bubble フェーズでのみ処理
    if !ev.is_bubble() {
        return false;
    }

    let state = ev.value();
    if !state.left_down {
        return false;
    }

    let entity_name = world
        .get::<Name>(entity)
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| format!("{:?}", entity));

    // リージョン名を取得
    let region = resolve_region_name(world, entity, state);

    info!(
        "[Region] {} pressed: region={:?}, client=({:.1},{:.1}), local=({:.1},{:.1})",
        entity_name,
        region,
        state.client_point.x,
        state.client_point.y,
        state.local_point.x,
        state.local_point.y,
    );

    // リージョンに応じた色に変更
    let color = region_color(region.as_deref());
    if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
        brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(color);
    }

    true
}

/// リージョンテストボックス共通の OnPointerMoved ハンドラ
///
/// ホバー時にリージョン名をログ出力（30フレームに1回）
fn on_region_box_moved(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    if !ev.is_bubble() {
        return false;
    }

    let state = ev.value();

    // 頻度制限: 30回に1回ログ出力
    static REGION_MOVE_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let count = REGION_MOVE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if count % 30 != 0 {
        return false;
    }

    let entity_name = world
        .get::<Name>(entity)
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| format!("{:?}", entity));

    let region = resolve_region_name(world, entity, state);

    debug!(
        "[Region] {} hover: region={:?}, client=({:.1},{:.1})",
        entity_name, region, state.client_point.x, state.client_point.y,
    );

    false // 伝播続行
}

/// 通常ヒットテスト領域の OnPointerPressed ハンドラ
///
/// クリックスルーテストの通常領域がクリックされたことを確認するためのログ出力
fn on_normal_hit_box_pressed(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    if !ev.is_bubble() {
        return false;
    }

    let state = ev.value();
    if !state.left_down {
        return false;
    }

    let entity_name = world
        .get::<Name>(entity)
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| format!("{:?}", entity));

    info!(
        "[ClickThrough] Normal region clicked: {} at ({:.1},{:.1})",
        entity_name, state.client_point.x, state.client_point.y,
    );

    false
}
