#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # DComp Taffy Flexbox Demo
//!
//! `CompositionMode::DComp` を使用した DirectComposition ベースの描画デモ。
//! 既存の `taffy_flex_demo` と同等の Flexbox レイアウトを DComp パイプラインで描画する。
//!
//! ## 検証ポイント
//!
//! - `Window { composition_mode: CompositionMode::DComp }` 指定で DComp パイプラインが起動
//! - `WindowGraphics` → `VisualGraphics` → `SurfaceGraphics` の描画チェーン
//! - Rectangle / Label / BitmapSource ウィジェットの DComp Surface 上への描画
//! - Flexbox レイアウト（taffy）が DComp モードでも正常動作
//!
//! ## 使用法
//!
//! ```bash
//! cargo run -p wintf --example dcomp_taffy_demo
//! ```

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::core::Result;
use wintf::ecs::layout::LengthPercentageAuto;
use wintf::ecs::layout::{BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension};
use wintf::ecs::widget::bitmap_source::{BitmapSource, CommandSender};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::widget::text::label::Label;
use wintf::ecs::{CompositionMode, Window, WindowPos};
use wintf::*;

/// DComp デモウィンドウを識別するマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct DCompDemoWindow;

fn main() -> Result<()> {
    human_panic::setup_panic!();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinThreadMgr::new()?;
    let world = mgr.world();

    world.borrow().spawn(|tx| async move {
        run_demo(tx).await;
    });

    println!("\nDComp Taffy Flexbox Demo:");
    println!("  CompositionMode::DComp を使用した DirectComposition 描画");
    println!("  - 赤い矩形 (固定200x100)");
    println!("  - 緑の矩形 (grow=1) + ラベル子要素");
    println!("  - 青い矩形 (grow=2) + 画像子要素");
    println!("\n60秒後に自動的にWindowを閉じてアプリ終了します。");

    mgr.run()?;

    Ok(())
}

/// 非同期デモ実行
async fn run_demo(tx: CommandSender) {
    info!("[DCompDemo] Creating DComp window");
    let _ = tx.send(Box::new(|world: &mut World| {
        create_dcomp_demo_window(world);
    }));

    // 60秒後に自動終了
    async_io::Timer::after(Duration::from_secs(60)).await;

    info!("[DCompDemo] Closing window");
    let _ = tx.send(Box::new(close_window));
}

/// ウィンドウを閉じる
fn close_window(world: &mut World) {
    let mut query = world.query_filtered::<Entity, With<DCompDemoWindow>>();
    if let Some(window) = query.iter(world).next() {
        info!("[DCompDemo] Removing Window entity {:?}", window);
        world.despawn(window);
    }
}

/// DComp モードの Flexbox デモウィンドウを作成
fn create_dcomp_demo_window(world: &mut World) {
    // Window Entity (ルート) — CompositionMode::DComp を指定
    let window_entity = world
        .spawn((
            Name::new("DCompDemo-Window"),
            DCompDemoWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                flex_direction: Some(taffy::FlexDirection::Column),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(800.0)),
                    height: Some(Dimension::Px(500.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(POINT { x: 100, y: 100 }),
                ..Default::default()
            },
            Window {
                title: "wintf - DComp Taffy Demo".to_string(),
                composition_mode: CompositionMode::DComp,
                ..Default::default()
            },
        ))
        .id();

    // タイトルラベル
    world.spawn((
        Name::new("TitleLabel"),
        Label {
            text: "DComp Flexbox Demo — DirectComposition 描画".to_string(),
            font_family: "メイリオ".to_string(),
            font_size: 20.0,
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
                height: Some(Dimension::Px(40.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(10.0),
                right: LengthPercentageAuto::Px(10.0),
                top: LengthPercentageAuto::Px(10.0),
                bottom: LengthPercentageAuto::Px(5.0),
            })),
            ..Default::default()
        },
        ChildOf(window_entity),
    ));

    // Flex コンテナ（横並び）
    let flex_container = world
        .spawn((
            Name::new("DCompDemo-FlexContainer"),
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
                    height: Some(Dimension::Px(200.0)),
                }),
                flex_grow: Some(0.0),
                flex_shrink: Some(0.0),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: LengthPercentageAuto::Px(10.0),
                    right: LengthPercentageAuto::Px(10.0),
                    top: LengthPercentageAuto::Px(5.0),
                    bottom: LengthPercentageAuto::Px(10.0),
                })),
                ..Default::default()
            },
            ChildOf(window_entity),
        ))
        .id();

    // Flex アイテム1: 赤い矩形（固定サイズ）
    world.spawn((
        Name::new("RedBox"),
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
        ChildOf(flex_container),
    ));

    // Flex アイテム2: 緑の矩形（grow=1）+ ラベル子要素
    let green_box = world
        .spawn((
            Name::new("GreenBox"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 0.8,
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
            ChildOf(flex_container),
        ))
        .id();

    // GreenBox の子ラベル
    world.spawn((
        Name::new("GreenBoxLabel"),
        Label {
            text: "DComp Surface".to_string(),
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
                height: Some(Dimension::Px(30.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(5.0),
                right: LengthPercentageAuto::Px(5.0),
                top: LengthPercentageAuto::Px(5.0),
                bottom: LengthPercentageAuto::Px(5.0),
            })),
            ..Default::default()
        },
        ChildOf(green_box),
    ));

    // Flex アイテム3: 青い矩形（grow=2）+ BitmapSource 子要素
    let blue_box = world
        .spawn((
            Name::new("BlueBox"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Column),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(100.0)),
                    height: Some(Dimension::Px(120.0)),
                }),
                flex_grow: Some(2.0),
                flex_shrink: Some(1.0),
                flex_basis: Some(Dimension::Auto),
                ..Default::default()
            },
            ChildOf(flex_container),
        ))
        .id();

    // BlueBox の子画像（BitmapSource）
    const SEIKATU_IMAGE_PATH: &str =
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/assets/seikatu_0_0.webp");
    world.spawn((
        Name::new("DemoImage"),
        BitmapSource::new(SEIKATU_IMAGE_PATH),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(64.0)),
                height: Some(Dimension::Px(64.0)),
            }),
            margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                left: LengthPercentageAuto::Px(10.0),
                right: LengthPercentageAuto::Auto,
                top: LengthPercentageAuto::Px(10.0),
                bottom: LengthPercentageAuto::Px(10.0),
            })),
            ..Default::default()
        },
        ChildOf(blue_box),
    ));

    // 下段: 情報ラベル
    world.spawn((
        Name::new("InfoLabel"),
        Label {
            text:
                "このウィンドウは WS_EX_NOREDIRECTIONBITMAP + DirectComposition で描画されています"
                    .to_string(),
            font_family: "メイリオ".to_string(),
            font_size: 12.0,
            ..Default::default()
        },
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.4,
            g: 0.4,
            b: 0.4,
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
                bottom: LengthPercentageAuto::Px(10.0),
            })),
            ..Default::default()
        },
        ChildOf(window_entity),
    ));

    info!(
        "[DCompDemo] DComp Flexbox window created (entity={:?})",
        window_entity
    );
}
