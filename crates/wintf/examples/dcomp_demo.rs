#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # DComp Demo
//!
//! GPU 合成（WUC）による DirectComposition ベースの宣言的描画デモ。
//!
//! ## 移行に関する注記（task 4.4）
//!
//! 旧 `dcomp_demo` は `WinThreadMgr::create_window` の命令的経路で `WinMessageHandler`
//! を実装した `DemoWindow`（DirectComposition の 3D カードフリップによる神経衰弱ゲーム）を
//! 生成していた。`WinApp` 新 facade は宣言的ウィンドウ生成（`Window` + `BoxStyle` を
//! 持つ Entity を spawn）へ一本化され、命令的 `create_window` は consumer から撤去された。
//!
//! ECS 描画層には 3D カードフリップ相当のアニメーションウィジェットが存在しないため、
//! 旧カードゲームのインタラクションをそのまま忠実に移植することはできない。本デモは
//! その意図（DComp パイプライン上にカード状のグリッドを描画する）を、宣言的な
//! `Rectangle` + `Label` ウィジェットによるカードグリッドとして等価に再構成する。
//! `spawn_ui_local`（旧 `spawn_normal` 相当・UI スレッド単一 async）の利用例も併せて示す。
//!
//! ## 使用法
//!
//! ```bash
//! cargo run -p wintf --example dcomp_demo
//! ```

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::core::Result;
use wintf::ecs::layout::LengthPercentageAuto;
use wintf::ecs::layout::{BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::widget::text::label::Label;
use wintf::ecs::{ChildOf, Point, Window, WindowPos};
use wintf::*;

const CARD_ROWS: usize = 3;
const CARD_COLUMNS: usize = 6;

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

    let mgr = WinApp::new()?;
    let world = mgr.world();

    // 宣言的にカードグリッドウィンドウを生成（背景プール経路 world.spawn は温存）。
    world.borrow().spawn(|tx| async move {
        info!("[DCompDemo] Creating DComp card-grid window");
        let _ = tx.send(Box::new(|world: &mut World| {
            create_dcomp_demo_window(world);
        }));

        // 60秒後に自動終了
        async_io::Timer::after(Duration::from_secs(60)).await;

        info!("[DCompDemo] Closing window");
        let _ = tx.send(Box::new(close_window));
    });

    // 旧 `spawn_normal` 相当の UI スレッド単一 async（`!Send` 可）の利用例。
    // fire-and-forget のため JoinHandle はそのまま drop する（投入＝実行確定）。
    let _ = mgr.spawn_ui_local(async {
        info!("[DCompDemo] spawn_ui_local: UI async task running");
    });

    println!("\nDComp Demo:");
    println!("  GPU 合成（WUC）による DirectComposition 描画");
    println!("  {CARD_ROWS}x{CARD_COLUMNS} のカードグリッドを宣言的に描画します。");
    println!("\n60秒後に自動的にWindowを閉じてアプリ終了します。");

    mgr.run()
}

/// ウィンドウを閉じる
fn close_window(world: &mut World) {
    let mut query = world.query_filtered::<Entity, With<DCompDemoWindow>>();
    if let Some(window) = query.iter(world).next() {
        info!("[DCompDemo] Removing Window entity {:?}", window);
        world.despawn(window);
    }
}

/// DComp モードのカードグリッドウィンドウを宣言的に生成する。
fn create_dcomp_demo_window(world: &mut World) {
    // Window Entity (ルート) — GPU 合成（WUC）で描画
    let window_entity = world
        .spawn((
            Name::new("DCompDemo-Window"),
            DCompDemoWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                flex_direction: Some(taffy::FlexDirection::Column),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(980.0)),
                    height: Some(Dimension::Px(690.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(Point { x: 100, y: 100 }),
                ..Default::default()
            },
            Window {
                title: "wintf - DComp Demo".to_string(),
                ..Default::default()
            },
        ))
        .id();

    // カードグリッド（行ごとに横並び）
    for row in 0..CARD_ROWS {
        let row_entity = world
            .spawn((
                Name::new(format!("CardRow-{row}")),
                Rectangle::new(),
                Brushes::with_foreground(D2D1_COLOR_F {
                    r: 0.12,
                    g: 0.12,
                    b: 0.16,
                    a: 1.0,
                }),
                BoxStyle {
                    flex_direction: Some(taffy::FlexDirection::Row),
                    flex_grow: Some(1.0),
                    ..Default::default()
                },
                ChildOf(window_entity),
            ))
            .id();

        for column in 0..CARD_COLUMNS {
            create_card(world, row_entity, row, column);
        }
    }

    info!(
        "[DCompDemo] DComp card-grid window created (entity={:?})",
        window_entity
    );
}

/// 1 枚のカード（裏面想定の "?" ラベル付き矩形）を生成する。
fn create_card(world: &mut World, parent: Entity, row: usize, column: usize) {
    // カード色は行・列で軽く変化させる（装飾目的）。
    let shade = 0.55 + 0.12 * ((row + column) % 3) as f32;
    let card = world
        .spawn((
            Name::new(format!("Card-{row}-{column}")),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: shade,
                g: 0.45,
                b: 0.75,
                a: 1.0,
            }),
            BoxStyle {
                flex_direction: Some(taffy::FlexDirection::Column),
                justify_content: Some(taffy::JustifyContent::Center),
                align_items: Some(taffy::AlignItems::Center),
                flex_grow: Some(1.0),
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: LengthPercentageAuto::Px(15.0),
                    right: LengthPercentageAuto::Px(15.0),
                    top: LengthPercentageAuto::Px(15.0),
                    bottom: LengthPercentageAuto::Px(15.0),
                })),
                ..Default::default()
            },
            ChildOf(parent),
        ))
        .id();

    world.spawn((
        Name::new(format!("CardLabel-{row}-{column}")),
        Label {
            text: "?".to_string(),
            font_family: "Candara".to_string(),
            font_size: 64.0,
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
                height: Some(Dimension::Px(80.0)),
            }),
            ..Default::default()
        },
        ChildOf(card),
    ));
}
