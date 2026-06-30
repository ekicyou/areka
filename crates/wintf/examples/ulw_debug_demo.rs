#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # ULW Debug Demo - UpdateLayeredWindow 最小検証
//!
//! Phase 3 ULW パイプラインの問題を切り分けるための最小デモ。
//! 1つのウィンドウに赤い矩形1つだけ配置し、描画パイプラインの各段階をロギング。

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::core::Result;
use wintf::ecs::layout::{BoxPosition, BoxSize, BoxStyle, Dimension};
use wintf::ecs::pointer::{OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::bitmap_source::CommandSender;
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::{Window, WindowPos};
use wintf::*;

fn main() -> Result<()> {
    human_panic::setup_panic!();

    // tracing 初期化: デフォルトで trace レベル
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("wintf=debug,info")),
        )
        .init();

    info!("=== ULW Debug Demo START ===");

    let mgr = WinApp::new()?;
    let world = mgr.world();

    world.borrow().spawn(|tx| async move {
        run_demo(tx).await;
    });

    println!("\n=== ULW Debug Demo ===");
    println!("赤い矩形1つだけのウィンドウを表示します。");
    println!("30秒後に自動終了。クリックすると色が赤⇔緑に変わります。\n");

    mgr.run()?;
    Ok(())
}

async fn run_demo(tx: CommandSender) {
    info!("[Demo] Creating window...");
    let _ = tx.send(Box::new(|world: &mut World| {
        create_debug_window(world);
    }));

    // 30秒待機して終了
    async_io::Timer::after(Duration::from_secs(30)).await;

    info!("[Demo] 30s elapsed, closing...");
    let _ = tx.send(Box::new(|world: &mut World| {
        use wintf::ecs::window::WindowHandle;
        let mut q = world.query::<&WindowHandle>();
        let handles: Vec<_> = q.iter(world).map(|h| h.hwnd).collect();
        for hwnd in handles {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            }
        }
    }));
}

fn create_debug_window(world: &mut World) {
    info!("[Demo] Spawning window entity...");

    // ウィンドウ (300x200) — ウィンドウ自体に背景矩形を付ける
    let window_entity = world
        .spawn((
            Name::new("DebugWindow"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 0.2,
                g: 0.2,
                b: 0.8,
                a: 1.0, // 青背景、完全不透明
            }),
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(300.0)),
                    height: Some(Dimension::Px(200.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(wintf::ecs::Point { x: 100, y: 100 }),
                ..Default::default()
            },
            Window {
                title: "ULW Debug".to_string(),
                ..Default::default()
            },
        ))
        .id();

    info!("[Demo] Window entity: {:?}", window_entity);

    // 赤い矩形 (150x80) - ウィンドウの子として配置
    let box_entity = world
        .spawn((
            Name::new("RedRect"),
            Rectangle::new(),
            Brushes::with_foreground(D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0, // 赤、完全不透明
            }),
            BoxStyle {
                size: Some(BoxSize {
                    width: Some(Dimension::Px(150.0)),
                    height: Some(Dimension::Px(80.0)),
                }),
                ..Default::default()
            },
            OnPointerPressed(on_rect_pressed),
            ChildOf(window_entity),
        ))
        .id();

    info!("[Demo] Box entity: {:?}", box_entity);
}

/// クリックハンドラ: Bubble フェーズで色トグル
fn on_rect_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    match ev {
        Phase::Tunnel(state) => {
            info!(
                "[Click-Tunnel] entity={:?} sender={:?} L={} R={} pos=({:.0},{:.0})",
                entity,
                sender,
                state.left_down,
                state.right_down,
                state.client_point.x,
                state.client_point.y,
            );
            false
        }
        Phase::Bubble(state) => {
            info!(
                "[Click-Bubble] entity={:?} sender={:?} L={} R={} pos=({:.0},{:.0})",
                entity,
                sender,
                state.left_down,
                state.right_down,
                state.client_point.x,
                state.client_point.y,
            );

            if state.left_down {
                // 色トグル: 赤 ⇔ 緑
                if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
                    let is_red = match &brushes.foreground {
                        wintf::ecs::widget::brushes::Brush::Solid(c) => c.r > 0.5,
                        _ => false,
                    };
                    if is_red {
                        info!("[Click] Toggling RED -> GREEN");
                        brushes.foreground =
                            wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                                r: 0.0,
                                g: 1.0,
                                b: 0.0,
                                a: 1.0,
                            });
                    } else {
                        info!("[Click] Toggling GREEN -> RED");
                        brushes.foreground =
                            wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                                r: 1.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            });
                    }
                }
                true // handled
            } else {
                false
            }
        }
    }
}
