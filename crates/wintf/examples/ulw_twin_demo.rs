#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # ULW Twin Window Demo - 同一レイアウト2ウィンドウ比較
//!
//! taffy_flex_demo から機能を削ぎ落とした最小デモ。
//! 同一構造の2つのウィンドウを並べて表示し、描画結果が一致するか確認する。
//!
//! - ドラッグなし
//! - クリックハンドラなし
//! - ヒットリージョンなし
//! - イベントハンドラなし
//!
//! ## 確認項目
//! - 左右のウィンドウが同一レイアウトで描画されるか
//! - 赤い矩形(200x100固定), 緑の矩形(grow=1), 青の矩形(grow=2) が同じ配置か
//! - デバッグ赤枠がウィンドウ端に一致しているか

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::core::Result;
use wintf::ecs::layout::{BoxMargin, BoxPosition, BoxSize, BoxStyle, Dimension, GlobalArrangement};
use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::window::WindowHandle;
use wintf::ecs::{Window, WindowPos};
use wintf::*;

fn main() -> Result<()> {
    human_panic::setup_panic!();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("wintf=debug,info")),
        )
        .init();

    info!("=== ULW Twin Window Demo START ===");

    let mgr = WinThreadMgr::new()?;
    let world = mgr.world();

    world.borrow().spawn(|tx| async move {
        run_demo(tx).await;
    });

    println!("\n=== ULW Twin Window Demo ===");
    println!("同一構造の2ウィンドウを表示。レイアウトが一致するか確認。");
    println!("  Window 1: (100, 100)");
    println!("  Window 2: (1200, 100)");
    println!("  構成: 灰色コンテナ(Row) → 赤(固定200x100) / 緑(grow=1) / 青(grow=2)");
    println!("60秒後に自動終了。\n");

    mgr.run()?;
    Ok(())
}

async fn run_demo(tx: CommandSender) {
    info!("[Demo] Creating twin windows...");

    let _ = tx.send(Box::new(|world: &mut World| {
        create_simple_window(
            world,
            "Twin Demo (Window 1)",
            windows::Win32::Foundation::POINT { x: 100, y: 100 },
        );
        create_simple_window(
            world,
            "Twin Demo (Window 2)",
            windows::Win32::Foundation::POINT { x: 1200, y: 100 },
        );
    }));

    // 3秒後にレイアウト情報をダンプ → 即終了
    async_io::Timer::after(Duration::from_secs(3)).await;
    info!("[Demo] Dumping layout info...");
    let _ = tx.send(Box::new(dump_layout_info));

    // ダンプ後すぐ終了（PostMessage で非同期クローズ → RefCell再借用回避）
    async_io::Timer::after(Duration::from_millis(500)).await;
    info!("[Demo] Closing...");
    let _ = tx.send(Box::new(|world: &mut World| {
        let mut q = world.query::<&WindowHandle>();
        let handles: Vec<_> = q.iter(world).map(|h| h.hwnd).collect();
        for hwnd in handles {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(hwnd),
                    windows::Win32::UI::WindowsAndMessaging::WM_CLOSE,
                    windows::Win32::Foundation::WPARAM(0),
                    windows::Win32::Foundation::LPARAM(0),
                );
            }
        }
    }));
}

/// レイアウト情報を全エンティティからダンプ
fn dump_layout_info(world: &mut World) {
    let mut q = world.query::<(
        Entity,
        Option<&Name>,
        Option<&GlobalArrangement>,
        Option<&WindowPos>,
        Option<&WindowHandle>,
    )>();

    info!("===== Layout Dump =====");
    for (entity, name, ga, wp, wh) in q.iter(world) {
        let name_str = name.map(|n| n.as_str()).unwrap_or("(unnamed)");

        if let Some(ga) = ga {
            info!(
                "  Entity {:?} [{}]: GA bounds=({:.1},{:.1},{:.1},{:.1})",
                entity, name_str, ga.bounds.left, ga.bounds.top, ga.bounds.right, ga.bounds.bottom,
            );
        }

        if let Some(wp) = wp {
            info!(
                "  Entity {:?} [{}]: WindowPos pos={:?} size={:?}",
                entity, name_str, wp.position, wp.size,
            );
        }

        if let Some(wh) = wh {
            // GetWindowRect で実際のフレーム座標を取得
            let hwnd = wh.hwnd;
            unsafe {
                let mut rect = windows::Win32::Foundation::RECT::default();
                let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
                info!(
                    "  Entity {:?} [{}]: HWND={:?} GetWindowRect=({},{},{},{}) frame_size={}x{}",
                    entity,
                    name_str,
                    hwnd.0,
                    rect.left,
                    rect.top,
                    rect.right,
                    rect.bottom,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                );

                // クライアント座標も取得
                let mut client_rect = windows::Win32::Foundation::RECT::default();
                let _ =
                    windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut client_rect);
                let mut pt = windows::Win32::Foundation::POINT { x: 0, y: 0 };
                let _ = windows::Win32::Graphics::Gdi::ClientToScreen(hwnd, &mut pt);
                info!(
                    "  Entity {:?} [{}]: ClientRect=({},{},{},{}) ClientOrigin=({},{})",
                    entity,
                    name_str,
                    client_rect.left,
                    client_rect.top,
                    client_rect.right,
                    client_rect.bottom,
                    pt.x,
                    pt.y,
                );
            }
        }
    }
    info!("===== End Layout Dump =====");
}

/// 同一構造のシンプルなウィンドウを作成
fn create_simple_window(
    world: &mut World,
    title: &str,
    position: windows::Win32::Foundation::POINT,
) -> Entity {
    info!(
        "[Demo] Creating window: {} at ({},{})",
        title, position.x, position.y
    );

    // ウィンドウ: 600x300, Column レイアウト
    let window_entity = world
        .spawn((
            Name::new(format!("TwinWindow [{}]", title)),
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                flex_direction: Some(taffy::FlexDirection::Column),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(600.0)),
                    height: Some(Dimension::Px(300.0)),
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
        ))
        .id();

    // Flexコンテナ（横並び、灰色背景）
    let flex_container = world
        .spawn((
            Name::new("Container"),
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
                margin: Some(BoxMargin(wintf::ecs::layout::Rect {
                    left: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    right: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    top: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                    bottom: wintf::ecs::layout::LengthPercentageAuto::Px(10.0),
                })),
                ..Default::default()
            },
            ChildOf(window_entity),
        ))
        .id();

    // 赤い矩形（固定 200x100）
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

    // 緑の矩形（grow=1）
    world.spawn((
        Name::new("GreenBox"),
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        }),
        BoxStyle {
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
    ));

    // 青い矩形（grow=2）
    world.spawn((
        Name::new("BlueBox"),
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
        ChildOf(flex_container),
    ));

    info!("[Demo] Window entity: {:?}", window_entity);
    window_entity
}
