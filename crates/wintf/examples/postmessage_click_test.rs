#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # PostMessage Click Debug Test
//!
//! PostMessageW で WM_LBUTTONDOWN / WM_LBUTTONUP をプログラム的に投入し、
//! クリック後の描画パイプライン（Brushes → draw_rectangles → GPU 合成（WUC）present）
//! が正しく動作するかを検証する。
//!
//! ## 検証ポイント
//! 1. 初回表示が正常（赤い矩形が見える）
//! 2. PostMessage によるクリック後に Brushes が変更される
//! 3. draw_rectangles が Changed<Brushes> を検出して GraphicsCommandList を再作成する
//! 4. GPU 合成（WUC）パスが再描画・present する
//! 5. 色が赤→黄にトグルされる
//!
//! ## 使い方
//! ```
//! RUST_LOG=debug cargo run --example postmessage_click_test
//! ```

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::EnvFilter;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_LBUTTONDOWN, WM_LBUTTONUP};
use windows::core::Result;
use wintf::ecs::layout::{BoxPosition, BoxSize, BoxStyle, Dimension, GlobalArrangement};
use wintf::ecs::pointer::{OnPointerPressed, Phase, PointerState};
use wintf::ecs::widget::brushes::Brushes;
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::window::WindowHandle;
use wintf::ecs::{Point, Window, WindowPos};
use wintf::*;

/// ウィンドウマーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct TestWindow;

/// クリック対象矩形マーカー
#[derive(Debug, Clone, Copy, Component, PartialEq, Hash)]
pub struct ClickTarget;

fn main() -> Result<()> {
    human_panic::setup_panic!();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    world.borrow().spawn(|tx| async move {
        run_test(tx).await;
    });

    println!("PostMessage Click Test:");
    println!("  - 赤い矩形が表示される");
    println!("  - 2秒後に PostMessage で左クリックを注入");
    println!("  - 矩形が赤→黄にトグルされるはず");
    println!("  - さらに2秒後に再度クリック注入（黄→赤に戻る）");
    println!("  - 10秒後に自動終了");

    mgr.run()?;
    Ok(())
}

async fn run_test(tx: wintf::ecs::widget::bitmap_source::CommandSender) {
    // 0秒: ウィンドウ作成
    info!("[Test] 0s: Creating test window");
    let _ = tx.send(Box::new(create_test_window));

    // 2秒: 初回表示を確認してからクリック注入
    async_io::Timer::after(Duration::from_secs(2)).await;

    info!("[Test] 2s: Injecting PostMessage WM_LBUTTONDOWN at (50, 50)");
    let _ = tx.send(Box::new(inject_click));

    // 4秒: もう一度クリック注入（黄→赤に戻るはず）
    async_io::Timer::after(Duration::from_secs(2)).await;

    info!("[Test] 4s: Injecting second PostMessage WM_LBUTTONDOWN at (50, 50)");
    let _ = tx.send(Box::new(inject_click));

    // 10秒: 終了
    async_io::Timer::after(Duration::from_secs(6)).await;
    info!("[Test] 10s: Closing window");
    let _ = tx.send(Box::new(close_window));
}

/// テストウィンドウ作成
fn create_test_window(world: &mut World) {
    let window_entity = world
        .spawn((
            Name::new("PostMsg-TestWindow"),
            TestWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(300.0)),
                    height: Some(Dimension::Px(300.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(Point { x: 200, y: 200 }),
                ..Default::default()
            },
            Window {
                title: "PostMessage Click Test".to_string(),
                ..Default::default()
            },
        ))
        .id();

    // クリック対象: 赤い矩形（100x100、offset (20,20)）
    world.spawn((
        Name::new("ClickTarget-Red"),
        ClickTarget,
        Rectangle::new(),
        Brushes::with_foreground(D2D1_COLOR_F {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }),
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(100.0)),
                height: Some(Dimension::Px(100.0)),
            }),
            ..Default::default()
        },
        OnPointerPressed(on_click_target_pressed),
        ChildOf(window_entity),
    ));

    info!(
        "[Test] Window {:?} created with ClickTarget child",
        window_entity
    );
}

/// PostMessage でクリックを注入
fn inject_click(world: &mut World) {
    // WindowHandle を持つ TestWindow エンティティを検索
    let mut query = world.query_filtered::<&WindowHandle, With<TestWindow>>();
    let Some(window_handle) = query.iter(world).next() else {
        info!("[Test] Window not found, skipping click injection");
        return;
    };
    let hwnd = window_handle.hwnd;

    // クリック前の状態をログ
    dump_click_target_state(world, "BEFORE click");

    // クライアント座標 (50, 50) にクリックを投入
    // MAKELPARAM(50, 50) = (50 | (50 << 16))
    let lparam = LPARAM((50i32 | (50i32 << 16)) as isize);
    let wparam = WPARAM(0x0001); // MK_LBUTTON

    info!(
        "[Test] PostMessageW WM_LBUTTONDOWN to {:?} at (50,50)",
        hwnd
    );
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_LBUTTONDOWN, wparam, lparam);
    }

    // WM_LBUTTONUP も投入（次フレーム処理用）
    let wparam_up = WPARAM(0);
    info!("[Test] PostMessageW WM_LBUTTONUP to {:?} at (50,50)", hwnd);
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_LBUTTONUP, wparam_up, lparam);
    }
}

/// クリック対象の現在状態をダンプ
fn dump_click_target_state(world: &mut World, label: &str) {
    let mut query =
        world.query_filtered::<(Entity, &Brushes, Option<&GlobalArrangement>), With<ClickTarget>>();
    for (entity, brushes, ga_opt) in query.iter(world) {
        let color = brushes
            .foreground
            .as_color()
            .map(|c| format!("({:.2},{:.2},{:.2},{:.2})", c.r, c.g, c.b, c.a))
            .unwrap_or_else(|| "inherit".to_string());

        let bounds = ga_opt
            .map(|ga| {
                format!(
                    "({:.0},{:.0})-({:.0},{:.0})",
                    ga.bounds.left, ga.bounds.top, ga.bounds.right, ga.bounds.bottom
                )
            })
            .unwrap_or_else(|| "no bounds".to_string());

        info!(
            "[Test] {} entity={:?} color={} bounds={}",
            label, entity, color, bounds
        );
    }
}

/// クリックハンドラ: 赤⇔黄トグル
fn on_click_target_pressed(
    world: &mut World,
    sender: Entity,
    entity: Entity,
    ev: &Phase<PointerState>,
) -> bool {
    if !ev.is_bubble() {
        info!(
            "[Test] Tunnel phase: sender={:?}, entity={:?}",
            sender, entity
        );
        return false;
    }

    let state = ev.value();
    if !state.left_down {
        return false;
    }

    info!(
        "[Test] === CLICK HANDLER FIRED === sender={:?}, entity={:?}",
        sender, entity
    );

    if let Some(mut brushes) = world.get_mut::<Brushes>(entity) {
        let is_red = match brushes.foreground.as_color() {
            Some(c) => c.r > 0.9 && c.g < 0.1,
            None => false,
        };

        if is_red {
            brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                r: 1.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            });
            info!("[Test] Color toggled: RED -> YELLOW");
        } else {
            brushes.foreground = wintf::ecs::widget::brushes::Brush::Solid(D2D1_COLOR_F {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            });
            info!("[Test] Color toggled: YELLOW -> RED");
        }
    } else {
        info!("[Test] WARNING: Brushes not found on entity {:?}", entity);
    }

    true
}

/// ウィンドウ終了
fn close_window(world: &mut World) {
    let mut query = world.query_filtered::<Entity, With<TestWindow>>();
    let windows: Vec<Entity> = query.iter(world).collect();
    for window in windows {
        info!("[Test] Removing Window entity {:?}", window);
        world.despawn(window);
    }
}
