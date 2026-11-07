#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! # Visual Tree Demo
//!
//! 描画の木（描画面を持たない節）で、重ね方・回転・切り抜きを 1 枚の描画面に描く。
//!
//! - 根: 窓の下の `Visual`（描画面の持ち主）。自分の描く中身は持たない。
//! - 節: `VisualDrawContent`（閉じた命令の列）か `VisualTransform` を持つ子の `Visual`。
//!   子の順（画家の順）で根の描画面に描かれる。
//!   1. 重ね方: 青の矩形の上に `D2D1_COMPOSITE_MODE_XOR` の赤い円（円の形に抜ける＝不透明の上の XOR は α 0）
//!   2. 回転: 中心を軸に 30 度回した緑の矩形（`VisualTransform`）
//!   3. 切り抜き: 角丸の `Visual.clip` を持つまとめ役の下に、はみ出す黄の矩形
//!
//! 中身の無いまとめ役は `VisualTransform`（恒等でよい）を付けて節にする。付けないと描画面の
//! 持ち主になり、その下は木では描かれない。
//!
//! 命令の列は `GraphicsCore` の装置の DC で記録する（別の装置の列は入れない約束）。
//!
//! ```bash
//! cargo run -p wintf --example visual_tree_demo
//! ```
//!
//! 10 秒後に窓を閉じて終わる。

use bevy_ecs::name::Name;
use bevy_ecs::prelude::*;
use std::f32::consts::PI;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::UI::WindowsAndMessaging::WS_OVERLAPPEDWINDOW;
use windows::core::Result;
use windows_numerics::Vector2;
use wintf::ecs::layout::{
    BoxInset, BoxPosition, BoxSize, BoxStyle, Dimension, LengthPercentageAuto, Rect,
};
use wintf::ecs::window::WindowStyle;
use wintf::ecs::{
    ClipShape, GraphicsCore, Point, Visual, VisualDrawContent, VisualTransform, Window, WindowPos,
};
use wintf::numerics::{Aabb, Transform2D};
use wintf::*;

#[derive(Component)]
struct DemoWindow;

fn main() -> Result<()> {
    human_panic::setup_panic!();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let mgr = WinApp::new()?;
    mgr.world().borrow().spawn(|tx| async move {
        let _ = tx.send(Box::new(create_window));

        // GraphicsCore は初回のフレームで入る。入るまで節の作成を送り直す（最大 5 秒）
        let done = Arc::new(AtomicBool::new(false));
        for _ in 0..50 {
            async_io::Timer::after(Duration::from_millis(100)).await;
            if done.load(Ordering::Acquire) {
                break;
            }
            let done = done.clone();
            let _ = tx.send(Box::new(move |world: &mut World| {
                if !done.load(Ordering::Acquire) && create_nodes(world) {
                    done.store(true, Ordering::Release);
                }
            }));
        }
        if !done.load(Ordering::Acquire) {
            warn!("[VisualTreeDemo] GraphicsCore が入らないので節を作れなかった");
        }

        async_io::Timer::after(Duration::from_secs(10)).await;
        info!("[VisualTreeDemo] Closing window");
        let _ = tx.send(Box::new(|world: &mut World| {
            let windows: Vec<Entity> = world
                .query_filtered::<Entity, With<DemoWindow>>()
                .iter(world)
                .collect();
            for e in windows {
                world.despawn(e);
            }
        }));
    });

    println!(
        "Visual Tree Demo: 重ね方（XOR）・回転（30 度）・切り抜き（角丸）。10 秒後に閉じます。"
    );
    mgr.run()
}

fn px(v: f32) -> LengthPercentageAuto {
    LengthPercentageAuto::Px(v)
}

/// 根の中の (x, y) に w×h の場所を取る
fn placed(x: f32, y: f32, w: f32, h: f32) -> BoxStyle {
    BoxStyle {
        position: Some(BoxPosition::Absolute),
        inset: Some(BoxInset(Rect {
            left: px(x),
            top: px(y),
            right: LengthPercentageAuto::Auto,
            bottom: LengthPercentageAuto::Auto,
        })),
        size: Some(BoxSize {
            width: Some(Dimension::Px(w)),
            height: Some(Dimension::Px(h)),
        }),
        ..Default::default()
    }
}

fn color(r: f32, g: f32, b: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r, g, b, a: 1.0 }
}

fn create_window(world: &mut World) {
    let window = world
        .spawn((
            Name::new("VisualTreeDemo-Window"),
            DemoWindow,
            BoxStyle {
                position: Some(BoxPosition::Absolute),
                size: Some(BoxSize {
                    width: Some(Dimension::Px(480.0)),
                    height: Some(Dimension::Px(320.0)),
                }),
                ..Default::default()
            },
            WindowPos {
                position: Some(Point { x: 200, y: 100 }),
                ..Default::default()
            },
            Window {
                title: "Visual Tree Demo".to_string(),
                ..Default::default()
            },
            WindowStyle {
                style: WS_OVERLAPPEDWINDOW,
                ..Default::default()
            },
        ))
        .id();
    // 根（描画面の持ち主）。節は後から子として足す
    world.spawn((
        Name::new("VisualTreeDemo-Root"),
        Visual::default(),
        placed(0.0, 0.0, 480.0, 320.0),
        ChildOf(window),
    ));
}

/// 根の下に節を足す。GraphicsCore がまだ無ければ何もせず false
fn create_nodes(world: &mut World) -> bool {
    let Some(root) = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find(|(_, n)| n.as_str() == "VisualTreeDemo-Root")
        .map(|(e, _)| e)
    else {
        return false;
    };
    let Some(dc) = world
        .get_resource::<GraphicsCore>()
        .and_then(|c| c.device_context().cloned())
    else {
        return false;
    };
    let mut node = |name: &str, content: VisualDrawContent, extra: (Visual, BoxStyle), parent| {
        world
            .spawn((Name::new(name.to_string()), content, extra, ChildOf(parent)))
            .id()
    };
    let src_over = D2D1_COMPOSITE_MODE_SOURCE_OVER;

    // 背景（薄い灰色）
    node(
        "Background",
        record(&dc, 480.0, 320.0, color(0.95, 0.95, 0.95), false, src_over),
        (Visual::default(), placed(0.0, 0.0, 480.0, 320.0)),
        root,
    );

    // 1. 重ね方: 青の矩形の上に XOR の赤い円
    node(
        "Composite-Base",
        record(&dc, 100.0, 100.0, color(0.2, 0.4, 0.9), false, src_over),
        (Visual::default(), placed(30.0, 60.0, 100.0, 100.0)),
        root,
    );
    node(
        "Composite-Xor",
        record(
            &dc,
            100.0,
            100.0,
            color(0.9, 0.2, 0.2),
            true,
            D2D1_COMPOSITE_MODE_XOR,
        ),
        (Visual::default(), placed(80.0, 110.0, 100.0, 100.0)),
        root,
    );

    // 2. 回転: 中心を軸に 30 度
    let rotated = node(
        "Rotated",
        record(&dc, 100.0, 60.0, color(0.2, 0.7, 0.3), false, src_over),
        (Visual::default(), placed(210.0, 110.0, 100.0, 60.0)),
        root,
    );
    world
        .entity_mut(rotated)
        .insert(VisualTransform(Transform2D {
            anchor: Vector2 { X: 50.0, Y: 30.0 },
            rotation: 30.0 * PI / 180.0,
            ..Default::default()
        }));

    // 3. 切り抜き: 角丸のまとめ役（中身なし＝恒等の VisualTransform で節にする）の下に、はみ出す矩形
    let clip_group = world
        .spawn((
            Name::new("Clip-Group"),
            Visual {
                clip: Some(ClipShape::rounded_rectangle(24.0)),
                ..Default::default()
            },
            VisualTransform::default(),
            placed(340.0, 80.0, 110.0, 160.0),
            ChildOf(root),
        ))
        .id();
    world.spawn((
        Name::new("Clip-Overflow"),
        record(&dc, 200.0, 200.0, color(0.95, 0.8, 0.1), false, src_over),
        Visual::default(),
        placed(-40.0, -20.0, 200.0, 200.0),
        ChildOf(clip_group),
    ));

    info!("[VisualTreeDemo] nodes created under {:?}", root);
    true
}

/// w×h の矩形か楕円を 1 色で描く命令の列を記録し、描く中身にする
fn record(
    dc: &ID2D1DeviceContext,
    w: f32,
    h: f32,
    c: D2D1_COLOR_F,
    ellipse: bool,
    composite: D2D1_COMPOSITE_MODE,
) -> VisualDrawContent {
    let rect = D2D_RECT_F {
        left: 0.0,
        top: 0.0,
        right: w,
        bottom: h,
    };
    // shortcut: 例なので記録の失敗は panic（本番の描き手は error! を残して飛ばす）
    let list = unsafe {
        let list = dc.CreateCommandList().expect("CreateCommandList");
        dc.SetTarget(&list);
        dc.SetTransform(&windows_numerics::Matrix3x2::identity());
        dc.BeginDraw();
        let brush = dc.CreateSolidColorBrush(&c, None).expect("brush");
        if ellipse {
            let e = D2D1_ELLIPSE {
                point: Vector2 {
                    X: w / 2.0,
                    Y: h / 2.0,
                },
                radiusX: w / 2.0,
                radiusY: h / 2.0,
            };
            dc.FillEllipse(&e, &brush);
        } else {
            dc.FillRectangle(&rect, &brush);
        }
        dc.EndDraw(None, None).expect("EndDraw");
        dc.SetTarget(None);
        list.Close().expect("Close");
        list
    };
    VisualDrawContent::new_ex(Aabb(rect), list, D2D1_INTERPOLATION_MODE_LINEAR, composite)
}
