#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy_ecs::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing_subscriber::EnvFilter;
use windows::core::Result;
use wintf::ecs::Window;
use wintf::ecs::layout::{BoxPosition, BoxSize, BoxStyle, Dimension};
use wintf::ecs::widget::brushes::{Brush, Brushes};
use wintf::ecs::widget::shapes::Rectangle;
use wintf::ecs::{
    GraphicsCore, Point, SurfaceGraphics, VisualGraphics, WindowGraphics, WindowHandle, WindowPos,
};
use wintf::*;

/// 4 秒の確かめが通ったら立つ（立たずに終わったら終了コード 1）。
static PASSED: AtomicBool = AtomicBool::new(false);

/// GraphicsCore再初期化システムの統合テスト
///
/// このテストは以下を検証します:
/// - GraphicsCore初期化とコンポーネントの自動初期化
/// - GraphicsCore無効化による依存コンポーネントの自動無効化
/// - GraphicsCore再初期化と全依存コンポーネントの再初期化
/// - 複数ウィンドウでの同時再初期化
fn main() -> Result<()> {
    println!("\n========== GraphicsCore Reinitialization Test ==========\n");

    human_panic::setup_panic!();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    let world = mgr.world();

    // テストシナリオ
    world.borrow().spawn(|tx| async move {
        // 0秒: ウィンドウを2つ作成
        println!("[Timer] 0s: Creating two windows");
        let _ = tx.send(Box::new(|world: &mut World| {
            world.spawn((
                Window {
                    title: "Test Window 1 (Red)".to_string(),
                    ..Default::default()
                },
                // 窓の大きさはレイアウトから決まる（BoxStyle が無いと 0 で描画面が作られない）
                BoxStyle {
                    position: Some(BoxPosition::Absolute),
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(600.0)),
                        height: Some(Dimension::Px(400.0)),
                    }),
                    ..Default::default()
                },
                WindowPos {
                    position: Some(Point { x: 100, y: 100 }),
                    ..Default::default()
                },
                Rectangle::new(),
                Brushes::with_foreground(Brush::RED.as_color().unwrap()),
            ));

            world.spawn((
                Window {
                    title: "Test Window 2 (Blue)".to_string(),
                    ..Default::default()
                },
                // 窓の大きさはレイアウトから決まる（BoxStyle が無いと 0 で描画面が作られない）
                BoxStyle {
                    position: Some(BoxPosition::Absolute),
                    size: Some(BoxSize {
                        width: Some(Dimension::Px(600.0)),
                        height: Some(Dimension::Px(400.0)),
                    }),
                    ..Default::default()
                },
                WindowPos {
                    position: Some(Point { x: 750, y: 100 }),
                    ..Default::default()
                },
                Rectangle::new(),
                Brushes::with_foreground(Brush::BLUE.as_color().unwrap()),
            ));

            println!("[Test] Two windows spawned");
        }));

        // 3秒: GraphicsCoreを無効化（デバイスロストをシミュレート）
        async_io::Timer::after(Duration::from_secs(3)).await;
        println!("\n[Timer] 3s: Simulating device loss (invalidating GraphicsCore)");
        let _ = tx.send(Box::new(|world: &mut World| {
            if let Some(mut graphics) = world.get_resource_mut::<GraphicsCore>() {
                println!("\n========================================");
                println!("[Test] ===== デバイスロスト シミュレーション開始 =====");
                println!("[Test] GraphicsCore.invalidate() を呼び出します");
                graphics.invalidate();
                println!("[Test] GraphicsCore無効化完了。次フレームで自動再初期化されます。");
                println!("========================================\n");
            } else {
                println!("  ❌ [FAIL] GraphicsCore resource not found");
            }
        }));

        // 4秒: 再初期化状態の確認
        async_io::Timer::after(Duration::from_secs(1)).await;
        println!("\n[Timer] 4s: Verifying reinitialization");
        let _ = tx.send(Box::new(|world: &mut World| {
            let graphics_valid = world
                .get_resource::<GraphicsCore>()
                .map(|g| g.is_valid())
                .unwrap_or(false);

            let mut query = world.query::<(
                Entity,
                &WindowHandle,
                &WindowGraphics,
                &VisualGraphics,
                &SurfaceGraphics,
            )>();

            println!("\n========================================");
            println!("[Test] ===== 再初期化検証 =====");
            println!("[Test] GraphicsCore.is_valid() = {}", graphics_valid);

            let mut all_success = true;
            let mut count = 0;
            for (entity, handle, wg, v, s) in query.iter(world) {
                count += 1;
                let wg_valid = wg.is_valid();
                let v_valid = v.is_valid();
                let s_valid = s.is_valid();
                let generation = wg.generation();

                println!("[Test] Entity {:?} (HWND {:?}):", entity, handle.hwnd);
                println!(
                    "  - WindowGraphics: valid={}, generation={}",
                    wg_valid, generation
                );
                println!("  - Visual.is_valid() = {}", v_valid);
                println!("  - Surface.is_valid() = {}", s_valid);

                if generation > 0 && wg_valid && v_valid && s_valid {
                    println!(
                        "  ✅ [SUCCESS] 再初期化されました！（generation={} > 0）",
                        generation
                    );
                } else if generation == 0 && wg_valid && v_valid && s_valid {
                    println!("  ❌ [FAIL] 作り直されていない（generation=0）");
                    all_success = false;
                } else {
                    println!("  ❌ [FAIL] コンポーネントが無効または未初期化");
                    all_success = false;
                }
            }

            if count == 0 {
                println!("  ❌ [FAIL] 確かめる窓が 1 つも無い");
                all_success = false;
            }
            if all_success && graphics_valid {
                PASSED.store(true, Ordering::SeqCst);
                println!(
                    "\n  🎉🎉🎉 [TEST SUCCESS] {} 窓とも正常に再初期化されました！",
                    count
                );
            } else {
                println!("\n  ❌ [TEST FAIL] 作り直しの確かめに失敗しました");
            }
            println!("========================================\n");
        }));

        // 7秒: 1つ目のウィンドウからRectangle削除（視覚効果のため）
        async_io::Timer::after(Duration::from_secs(3)).await;
        println!("\n[Timer] 7s: Removing Rectangle from first window (visual effect)");
        let _ = tx.send(Box::new(|world: &mut World| {
            let mut query = world.query::<(Entity, &WindowHandle)>();
            if let Some((entity, handle)) = query.iter(world).next() {
                println!(
                    "[Test] Removing Rectangle from entity {:?} (hwnd {:?})",
                    entity, handle.hwnd
                );
                println!("       赤い四角形が消えます...");
                world.entity_mut(entity).remove::<Rectangle>();
            }
        }));

        // 10秒: ウィンドウを1つ閉じる
        async_io::Timer::after(Duration::from_secs(3)).await;
        println!("\n[Timer] 10s: Closing one window");
        let _ = tx.send(Box::new(|world: &mut World| {
            let mut query = world.query::<(Entity, &WindowHandle)>();
            let entities: Vec<_> = query.iter(world).map(|(e, h)| (e, h.hwnd)).collect();

            if let Some((entity, hwnd)) = entities.first() {
                println!(
                    "[Test] Closing window: Entity {:?}, HWND {:?}",
                    entity, hwnd
                );
                world.despawn(*entity);
            }
        }));

        // 13秒: 最後のウィンドウを閉じる
        async_io::Timer::after(Duration::from_secs(3)).await;
        println!("\n[Timer] 13s: Closing last window");
        let _ = tx.send(Box::new(|world: &mut World| {
            let mut query = world.query::<(Entity, &WindowHandle)>();
            if let Some((entity, handle)) = query.iter(world).next() {
                println!(
                    "[Test] Closing last window: Entity {:?}, HWND {:?}",
                    entity, handle.hwnd
                );
                world.despawn(entity);
            }
        }));
    });

    println!("[Test] Test scenario started");
    println!("\nTest Phases:");
    println!("  Phase 1 (0s):  Create 2 windows with red & blue rectangles");
    println!("  Phase 2 (3s):  Simulate device loss (invalidate GraphicsCore)");
    println!("  Phase 3 (4s):  Verify automatic reinitialization (generation++)");
    println!("  Phase 4 (7s):  Remove Rectangle for visual effect");
    println!("  Phase 5 (10s): Close one window");
    println!("  Phase 6 (13s): Close last window and exit");
    println!("\n========================================\n");

    mgr.run()?;

    if !PASSED.load(Ordering::SeqCst) {
        eprintln!("[TEST FAIL] 作り直しの確かめが通らなかった");
        std::process::exit(1);
    }
    Ok(())
}
