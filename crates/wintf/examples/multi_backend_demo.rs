#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # Multi-Backend Demo
//!
//! ULW（UpdateLayeredWindow）モードと DComp（DirectComposition）モードの
//! ウィンドウを1つの ECS World に同時 spawn し、各々が独立して描画されることを検証する。
//!
//! ## 構成
//!
//! - **ULW ウィンドウ** (`CompositionMode::ULW`):
//!   透過クリックスルー + マスコット画像表示。
//!   `WS_EX_LAYERED` で半透明ウィンドウとして描画。
//!
//! - **DComp ウィンドウ** (`CompositionMode::DComp`):
//!   通常 UI ウィジェット（矩形 / ラベル）表示。
//!   `WS_EX_NOREDIRECTIONBITMAP` + DirectComposition で描画。
//!
//! ## 検証ポイント
//!
//! - 同一 World 内で ULW / DComp が共存し、独立描画される
//! - 一方のウィンドウの操作・更新が他方に影響しない
//! - 各パイプラインが正しいウィンドウにのみ適用される
//!
//! ## 使用法
//!
//! ```bash
//! cargo run -p wintf --example multi_backend_demo
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
use wintf::ecs::layout::LengthPercentageAuto;
use wintf::ecs::layout::{BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension};
use wintf::ecs::widget::bitmap_source::{BitmapSource, CommandSender};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::widget::text::label::Label;
use wintf::ecs::{CompositionMode, Point, Window, WindowPos};
use wintf::*;

/// ULW デモウィンドウマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct UlwDemoWindow;

/// DComp デモウィンドウマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct DCompDemoWindow;

fn main() -> Result<()> {
    human_panic::setup_panic!();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    world.borrow().spawn(|tx| async move {
        run_demo(tx).await;
    });

    println!("\nMulti-Backend Demo:");
    println!("  [左] ULW ウィンドウ — 透過マスコット (WS_EX_LAYERED)");
    println!("  [右] DComp ウィンドウ — 通常 UI (WS_EX_NOREDIRECTIONBITMAP)");
    println!("\n両ウィンドウが同一 ECS World 内で独立して描画されます。");
    println!("60秒後に自動的に閉じます。");

    mgr.run()?;

    Ok(())
}

/// 非同期デモ実行
async fn run_demo(tx: CommandSender) {
    info!("[MultiBackend] Creating ULW + DComp windows");
    let _ = tx.send(Box::new(|world: &mut World| {
        create_ulw_mascot_window(world);
        create_dcomp_ui_window(world);
    }));

    // 60秒後に自動終了
    async_io::Timer::after(Duration::from_secs(60)).await;

    info!("[MultiBackend] Closing windows");
    let _ = tx.send(Box::new(close_all_windows));
}

/// 全デモウィンドウを閉じる
fn close_all_windows(world: &mut World) {
    let mut query =
        world.query_filtered::<Entity, Or<(With<UlwDemoWindow>, With<DCompDemoWindow>)>>();
    let entities: Vec<Entity> = query.iter(world).collect();
    for entity in entities {
        info!("[MultiBackend] Removing entity {:?}", entity);
        world.despawn(entity);
    }
}

// ============================================================================
// ULW ウィンドウ: 透過マスコット
// ============================================================================

/// ULW モード — 透過ウィンドウにマスコット画像を表示
fn create_ulw_mascot_window(world: &mut World) {
    // ULW ウィンドウ（デフォルトは ULW なので composition_mode 指定不要だが明示する）
    let window_entity = world
        .spawn((
            Name::new("UlwMascot-Window"),
            UlwDemoWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                flex_direction: Some(taffy::FlexDirection::Column),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(200.0)),
                    height: Some(Dimension::Px(300.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(Point { x: 100, y: 100 }),
                ..Default::default()
            },
            Window {
                title: "ULW Mascot".to_string(),
                composition_mode: CompositionMode::ULW,
                ..Default::default()
            },
        ))
        .id();

    // ドラッグ対応コンテナ（ウィンドウ全体をドラッグ可能にする）
    let drag_container = world
        .spawn((
            Name::new("UlwDragContainer"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Column),
                align_items: Some(taffy::AlignItems::Center),
                flex_grow: Some(1.0),
                ..Default::default()
            },
            DragConfig::default(),
            OnDragStart(on_drag_start),
            OnDrag(on_drag),
            OnDragEnd(on_drag_end),
            ChildOf(window_entity),
        ))
        .id();

    // マスコット画像
    const SEIKATU_IMAGE_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/assets/seikatu_0_0.webp");
    world.spawn((
        Name::new("MascotImage"),
        BitmapSource::new(SEIKATU_IMAGE_PATH),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(128.0)),
                height: Some(Dimension::Px(128.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(36.0),
                right: LengthPercentageAuto::Auto,
                top: LengthPercentageAuto::Px(20.0),
                bottom: LengthPercentageAuto::Px(10.0),
            })),
            ..Default::default()
        },
        ChildOf(drag_container),
    ));

    // マスコット名ラベル
    world.spawn((
        Name::new("MascotLabel"),
        Label {
            text: "ULW マスコット".to_string(),
            font_family: "メイリオ".to_string(),
            font_size: 14.0,
            ..Default::default()
        },
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.2,
            g: 0.2,
            b: 0.2,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: None,
                height: Some(Dimension::Px(30.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(10.0),
                right: LengthPercentageAuto::Px(10.0),
                top: LengthPercentageAuto::Px(5.0),
                bottom: LengthPercentageAuto::Px(5.0),
            })),
            ..Default::default()
        },
        ChildOf(drag_container),
    ));

    info!(
        "[MultiBackend] ULW mascot window created (entity={:?})",
        window_entity
    );
}

// ============================================================================
// DComp ウィンドウ: 通常 UI
// ============================================================================

/// DComp モード — 通常 UI ウィジェット表示
fn create_dcomp_ui_window(world: &mut World) {
    let window_entity = world
        .spawn((
            Name::new("DCompUI-Window"),
            DCompDemoWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                flex_direction: Some(taffy::FlexDirection::Column),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(500.0)),
                    height: Some(Dimension::Px(400.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(Point { x: 400, y: 100 }),
                ..Default::default()
            },
            Window {
                title: "DComp UI Panel".to_string(),
                composition_mode: CompositionMode::DComp,
                ..Default::default()
            },
        ))
        .id();

    // ドラッグ対応コンテナ（ウィンドウ全体をドラッグ可能にする）
    let drag_container = world
        .spawn((
            Name::new("DCompDragContainer"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Column),
                flex_grow: Some(1.0),
                ..Default::default()
            },
            DragConfig::default(),
            OnDragStart(on_drag_start),
            OnDrag(on_drag),
            OnDragEnd(on_drag_end),
            ChildOf(window_entity),
        ))
        .id();

    // ヘッダーラベル
    world.spawn((
        Name::new("HeaderLabel"),
        Label {
            text: "DComp UI Panel — DirectComposition".to_string(),
            font_family: "Segoe UI".to_string(),
            font_size: 18.0,
            ..Default::default()
        },
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.1,
            g: 0.1,
            b: 0.3,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: None,
                height: Some(Dimension::Px(40.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(15.0),
                right: LengthPercentageAuto::Px(15.0),
                top: LengthPercentageAuto::Px(15.0),
                bottom: LengthPercentageAuto::Px(5.0),
            })),
            ..Default::default()
        },
        ChildOf(drag_container),
    ));

    // UI パネル（横並び 3 カラム）
    let panel = world
        .spawn((
            Name::new("UIPanel"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.95,
                g: 0.95,
                b: 0.97,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Row),
                justify_content: Some(taffy::JustifyContent::SpaceEvenly),
                align_items: Some(taffy::AlignItems::Center),
                flex_grow: Some(1.0),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: LengthPercentageAuto::Px(15.0),
                    right: LengthPercentageAuto::Px(15.0),
                    top: LengthPercentageAuto::Px(5.0),
                    bottom: LengthPercentageAuto::Px(5.0),
                })),
                ..Default::default()
            },
            ChildOf(drag_container),
        ))
        .id();

    // カラム 1: 赤ボタン風
    let col1 = world
        .spawn((
            Name::new("ButtonRed"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.9,
                g: 0.2,
                b: 0.2,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Column),
                justify_content: Some(taffy::JustifyContent::Center),
                align_items: Some(taffy::AlignItems::Center),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(120.0)),
                    height: Some(Dimension::Px(80.0)),
                }),
                ..Default::default()
            },
            ChildOf(panel),
        ))
        .id();

    world.spawn((
        Name::new("ButtonRedLabel"),
        Label {
            text: "Button A".to_string(),
            font_family: "Segoe UI".to_string(),
            font_size: 14.0,
            ..Default::default()
        },
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: None,
                height: Some(Dimension::Px(24.0)),
            }),
            ..Default::default()
        },
        ChildOf(col1),
    ));

    // カラム 2: 緑ボタン風
    let col2 = world
        .spawn((
            Name::new("ButtonGreen"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.2,
                g: 0.8,
                b: 0.3,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Column),
                justify_content: Some(taffy::JustifyContent::Center),
                align_items: Some(taffy::AlignItems::Center),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(120.0)),
                    height: Some(Dimension::Px(80.0)),
                }),
                ..Default::default()
            },
            ChildOf(panel),
        ))
        .id();

    world.spawn((
        Name::new("ButtonGreenLabel"),
        Label {
            text: "Button B".to_string(),
            font_family: "Segoe UI".to_string(),
            font_size: 14.0,
            ..Default::default()
        },
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: None,
                height: Some(Dimension::Px(24.0)),
            }),
            ..Default::default()
        },
        ChildOf(col2),
    ));

    // カラム 3: 青ボタン風
    let col3 = world
        .spawn((
            Name::new("ButtonBlue"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.2,
                g: 0.3,
                b: 0.9,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Column),
                justify_content: Some(taffy::JustifyContent::Center),
                align_items: Some(taffy::AlignItems::Center),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(120.0)),
                    height: Some(Dimension::Px(80.0)),
                }),
                ..Default::default()
            },
            ChildOf(panel),
        ))
        .id();

    world.spawn((
        Name::new("ButtonBlueLabel"),
        Label {
            text: "Button C".to_string(),
            font_family: "Segoe UI".to_string(),
            font_size: 14.0,
            ..Default::default()
        },
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: None,
                height: Some(Dimension::Px(24.0)),
            }),
            ..Default::default()
        },
        ChildOf(col3),
    ));

    // フッターラベル
    world.spawn((
        Name::new("FooterLabel"),
        Label {
            text: "ULW と DComp が同一 World 内で独立描画されています".to_string(),
            font_family: "メイリオ".to_string(),
            font_size: 11.0,
            ..Default::default()
        },
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.5,
            g: 0.5,
            b: 0.5,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: None,
                height: Some(Dimension::Px(25.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(15.0),
                right: LengthPercentageAuto::Px(15.0),
                top: LengthPercentageAuto::Px(5.0),
                bottom: LengthPercentageAuto::Px(10.0),
            })),
            ..Default::default()
        },
        ChildOf(drag_container),
    ));

    info!(
        "[MultiBackend] DComp UI window created (entity={:?})",
        window_entity
    );
}

// ============================================================================
// ドラッグハンドラ（共通）
// ============================================================================

/// ドラッグ開始ハンドラ
fn on_drag_start(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragStartEvent>,
) -> bool {
    match ev {
        wintf::ecs::pointer::Phase::Tunnel(_) => false,
        wintf::ecs::pointer::Phase::Bubble(event) => {
            let entity_name = world
                .get::<Name>(entity)
                .map(|n| n.as_str().to_string())
                .unwrap_or_else(|| format!("{:?}", entity));
            info!(
                "[Drag] DragStart: entity={}, pos=({},{})",
                entity_name, event.position.x, event.position.y
            );
            false
        }
    }
}

/// ドラッグ中ハンドラ（ウィンドウ移動は DragConfig.move_window=true で自動処理）
fn on_drag(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragEvent>,
) -> bool {
    match ev {
        wintf::ecs::pointer::Phase::Tunnel(_) => false,
        wintf::ecs::pointer::Phase::Bubble(event) => {
            let entity_name = world
                .get::<Name>(entity)
                .map(|n| n.as_str().to_string())
                .unwrap_or_else(|| format!("{:?}", entity));
            let delta_x = event.position.x - event.start_position.x;
            let delta_y = event.position.y - event.start_position.y;
            debug!(
                "[Drag] Drag: entity={}, delta=({},{})",
                entity_name, delta_x, delta_y
            );
            false
        }
    }
}

/// ドラッグ終了ハンドラ
fn on_drag_end(
    world: &mut World,
    _sender: Entity,
    entity: Entity,
    ev: &wintf::ecs::pointer::Phase<DragEndEvent>,
) -> bool {
    match ev {
        wintf::ecs::pointer::Phase::Tunnel(_) => false,
        wintf::ecs::pointer::Phase::Bubble(event) => {
            let entity_name = world
                .get::<Name>(entity)
                .map(|n| n.as_str().to_string())
                .unwrap_or_else(|| format!("{:?}", entity));
            info!(
                "[Drag] DragEnd: entity={}, pos=({},{}), cancelled={}",
                entity_name, event.position.x, event.position.y, event.cancelled
            );
            false
        }
    }
}
