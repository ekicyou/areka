#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # Clip Demo
//!
//! visual-clip 機能の検証デモ。3 種類の ClipShape バリアントを
//! GPU 合成（WUC / DirectComposition）パイプラインで表示し、クリッピングの動作を目視確認する。
//!
//! ## 構成
//!
//! - **GPU 合成ウィンドウ** — DirectComposition パイプラインでのクリップ
//!
//! ウィンドウに3つのクリップ領域を配置:
//! 1. `ClipShape::Rectangle` — 矩形クリップ
//! 2. `ClipShape::RoundedRectangle { radius: 20.0 }` — 均一角丸クリップ
//! 3. `ClipShape::RoundedRectangleIndividual` — 個別角丸クリップ
//!
//! ## 使用法
//!
//! ```bash
//! cargo run -p wintf --example clip_demo
//! ```

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::WindowsAndMessaging::{WS_EX_NOREDIRECTIONBITMAP, WS_OVERLAPPEDWINDOW};
use windows::core::Result;
use wintf::ecs::layout::{
    BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension, LengthPercentageAuto,
};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::widget::text::label::Label;
use wintf::ecs::window::WindowStyle;
use wintf::ecs::{ClipShape, Point, Visual, Window, WindowPos};
use wintf::*;

/// DComp クリップデモウィンドウマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct DCompClipWindow;

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

    println!("\nClip Demo:");
    println!(
        "  GPU 合成ウィンドウ — DirectComposition クリップ（タイトルバー有、リサイズ可能）"
    );
    println!("\nウィンドウに3種類のクリップが表示されます:");
    println!("  上段: Rectangle（矩形クリップ）");
    println!("  中段: RoundedRectangle（均一角丸 radius=20）");
    println!("  下段: RoundedRectangleIndividual（個別角丸）");
    println!("\nウィンドウをリサイズしてクリップ領域が動的に追従することを確認できます。");
    println!("60秒後に自動的に閉じます。");

    mgr.run()?;

    Ok(())
}

/// 非同期デモ実行
async fn run_demo(tx: wintf::ecs::widget::bitmap_source::CommandSender) {
    info!("[ClipDemo] Creating DComp clip window");
    let _ = tx.send(Box::new(|world: &mut World| {
        create_dcomp_clip_window(world);
    }));

    // 60秒後に自動終了
    async_io::Timer::after(Duration::from_secs(60)).await;

    info!("[ClipDemo] Closing windows");
    let _ = tx.send(Box::new(close_all_windows));
}

/// 全デモウィンドウを閉じる
fn close_all_windows(world: &mut World) {
    let mut query = world.query_filtered::<Entity, With<DCompClipWindow>>();
    let entities: Vec<Entity> = query.iter(world).collect();
    for entity in entities {
        info!("[ClipDemo] Removing entity {:?}", entity);
        world.despawn(entity);
    }
}

// ============================================================================
// 共通: クリップデモ子ウィジェット構築
// ============================================================================

/// クリップ付き要素を3つ（Rectangle, RoundedRectangle, RoundedRectangleIndividual）
/// 縦に並べた構成を親コンテナに追加する。
fn create_clip_demo_children(world: &mut World, parent: Entity) {
    // セクション 1: Rectangle クリップ
    create_clip_section(
        world,
        parent,
        "Rectangle Clip",
        ClipShape::Rectangle,
        D2D1_COLOR_F {
            r: 0.2,
            g: 0.4,
            b: 0.8,
            a: 1.0,
        }, // 青系の親
        D2D1_COLOR_F {
            r: 1.0,
            g: 0.3,
            b: 0.3,
            a: 1.0,
        }, // 赤系の子（はみ出し用）
    );

    // セクション 2: RoundedRectangle クリップ（radius=20）
    create_clip_section(
        world,
        parent,
        "RoundedRect (r=20)",
        ClipShape::rounded_rectangle(20.0),
        D2D1_COLOR_F {
            r: 0.2,
            g: 0.7,
            b: 0.3,
            a: 1.0,
        }, // 緑系の親
        D2D1_COLOR_F {
            r: 0.9,
            g: 0.8,
            b: 0.1,
            a: 1.0,
        }, // 黄色系の子
    );

    // セクション 3: RoundedRectangleIndividual クリップ
    create_clip_section(
        world,
        parent,
        "Individual (10,20,30,40)",
        ClipShape::rounded_rectangle_individual(10.0, 20.0, 30.0, 40.0),
        D2D1_COLOR_F {
            r: 0.7,
            g: 0.2,
            b: 0.7,
            a: 1.0,
        }, // 紫系の親
        D2D1_COLOR_F {
            r: 0.1,
            g: 0.8,
            b: 0.8,
            a: 1.0,
        }, // シアン系の子
    );
}

/// 1セクション分のクリップデモ要素を構築。
/// 親（クリップ付き）の中に子（親より大きいサイズ）を配置し、はみ出しが切り取られることを検証。
fn create_clip_section(
    world: &mut World,
    parent: Entity,
    label_text: &str,
    clip_shape: ClipShape,
    parent_color: D2D1_COLOR_F,
    child_color: D2D1_COLOR_F,
) {
    // ラベル
    world.spawn((
        Name::new(format!("Label-{}", label_text)),
        Label {
            text: label_text.to_string(),
            font_family: "Segoe UI".to_string(),
            font_size: 13.0,
            ..Default::default()
        },
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.1,
            g: 0.1,
            b: 0.1,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: None,
                height: Some(Dimension::Px(20.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(5.0),
                right: LengthPercentageAuto::Auto,
                top: LengthPercentageAuto::Px(5.0),
                bottom: LengthPercentageAuto::Px(2.0),
            })),
            ..Default::default()
        },
        ChildOf(parent),
    ));

    // クリップ親要素（flex_grow で可変幅、クリップ適用）
    let clip_parent = world
        .spawn((
            Name::new(format!("ClipParent-{}", label_text)),
            Rectangle::new(),
            Brushes::with_foreground(parent_color),
            Visual {
                clip: Some(clip_shape),
                ..Default::default()
            },
            BoxStyle {
                flex_grow: Some(1.0),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: LengthPercentageAuto::Px(10.0),
                    right: LengthPercentageAuto::Px(10.0),
                    top: LengthPercentageAuto::Px(0.0),
                    bottom: LengthPercentageAuto::Px(5.0),
                })),
                ..Default::default()
            },
            ChildOf(parent),
        ))
        .id();

    // 子要素（親よりはるかに大きい固定サイズ → クリップではみ出しが切り取られる）
    world.spawn((
        Name::new(format!("OverflowChild-{}", label_text)),
        Rectangle::new(),
        Brushes::with_foreground(child_color),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(600.0)),
                height: Some(Dimension::Px(200.0)),
            }),
            ..Default::default()
        },
        ChildOf(clip_parent),
    ));
}

// ============================================================================
// GPU 合成（DComp）ウィンドウ
// ============================================================================

fn create_dcomp_clip_window(world: &mut World) {
    let window_entity = world
        .spawn((
            Name::new("ClipDemo-DComp-Window"),
            DCompClipWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                flex_direction: Some(taffy::FlexDirection::Column),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(400.0)),
                    height: Some(Dimension::Px(500.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(Point { x: 500, y: 50 }),
                ..Default::default()
            },
            Window {
                title: "Clip Demo (DComp)".to_string(),
                ..Default::default()
            },
            WindowStyle {
                style: WS_OVERLAPPEDWINDOW,
                ex_style: WS_EX_NOREDIRECTIONBITMAP,
            },
        ))
        .id();

    // メインコンテナ（白背景）
    let container = world
        .spawn((
            Name::new("DComp-Container"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.95,
                g: 0.95,
                b: 0.95,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Column),
                flex_grow: Some(1.0),
                ..Default::default()
            },
            ChildOf(window_entity),
        ))
        .id();

    // ヘッダー
    world.spawn((
        Name::new("DComp-Header"),
        Label {
            text: "DComp Clip Demo (DirectComposition)".to_string(),
            font_family: "Segoe UI".to_string(),
            font_size: 16.0,
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
                height: Some(Dimension::Px(30.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(10.0),
                right: LengthPercentageAuto::Px(10.0),
                top: LengthPercentageAuto::Px(10.0),
                bottom: LengthPercentageAuto::Px(5.0),
            })),
            ..Default::default()
        },
        ChildOf(container),
    ));

    create_clip_demo_children(world, container);

    info!(
        "[ClipDemo] DComp clip window created (entity={:?})",
        window_entity
    );
}
