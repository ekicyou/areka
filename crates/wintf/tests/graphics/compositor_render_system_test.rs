//! W3a-T: composite_render_system / ulw_present_system の ECS 駆動ヘッドレステスト
//!
//! 実 GraphicsCore を用い、合成描画パイプラインを Schedule 経由で実行して
//! DIB ピクセル（BGRA）を直接読み出して検証する。
//! - 子 CommandList の合成位置（GlobalArrangement.bounds ベース + ULW ウィンドウオフセット補正）
//! - is_visible / opacity==0 のサブツリースキップ
//! - opacity 累積の alpha 反映
//! - ClipShape 3 バリアントの ULW クリップ（ClipGuard の Push/Pop 経路）
//! - ダーティ判定（初回 is_added / Changed 検出 / 変更なしスキップ）
//! - ulw_present_system のスキップ・失敗リトライ挙動
//!
//! Note: composite_render_system は現状 DIB 外周に 2px の赤デバッグ枠を無条件描画する
//! （切り分け用デバッグコードの残置）。本テストはサンプル座標を枠から離して採取し、
//! 枠自体は characterization として 1 テストで固定する。

use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ExecutorKind;
use wintf::com::d2d::{D2D1CommandListExt, D2D1DeviceContextExt, D2D1DeviceExt};
use wintf::ecs::compositor::WindowD3D11Compositor;
use wintf::ecs::compositor_systems::{composite_render_system, ulw_present_system};
use wintf::ecs::layout::{Arrangement, GlobalArrangement};
use wintf::ecs::window::{Window, WindowHandle};
use wintf::ecs::{ClipShape, GraphicsCommandList, GraphicsCore, Rect, Size, Visual};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::D2D1_DEVICE_CONTEXT_OPTIONS_NONE;
use windows_numerics::Matrix3x2;

const BLUE: D2D1_COLOR_F = D2D1_COLOR_F {
    r: 0.0,
    g: 0.0,
    b: 1.0,
    a: 1.0,
};

/// (width, height) を塗りつぶす CommandList を記録する
fn make_filled_command_list(core: &GraphicsCore, w: f32, h: f32) -> GraphicsCommandList {
    let device = core.d2d_device().expect("d2d device");
    // 記録専用の DC を別途作成（core.device_context() のターゲットを汚さない）
    let dc = device
        .create_device_context(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
        .expect("recording dc");
    let cl = device.create_command_list().expect("command list");
    unsafe {
        dc.SetTarget(&cl);
        dc.BeginDraw();
    }
    let brush = dc
        .create_solid_color_brush(&BLUE, None)
        .expect("solid brush");
    dc.fill_rectangle(
        &D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: w,
            bottom: h,
        },
        &brush,
    );
    unsafe {
        dc.EndDraw(None, None).expect("EndDraw");
    }
    cl.close().expect("close");
    GraphicsCommandList::new(cl)
}

/// DIB から 1 ピクセルを [B, G, R, A] で読み出す
fn read_pixel(compositor: &WindowD3D11Compositor, x: u32, y: u32) -> [u8; 4] {
    let (w, h) = compositor.cached_size();
    assert!(x < w && y < h, "サンプル座標がビットマップ範囲内");
    let bits = compositor.dib_bits().expect("dib_bits");
    let stride = w as usize * 4;
    let off = y as usize * stride + x as usize * 4;
    let buf = unsafe { std::slice::from_raw_parts(bits, stride * h as usize) };
    [buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]
}

struct TestStage {
    world: World,
    schedule: Schedule,
    window: Entity,
}

/// 100x100 の ULW ウィンドウ（compositor 付き）を持つワールドを構築する
///
/// window_origin: GlobalArrangement.bounds の左上（スクリーン座標）。
/// ULW オフセット補正の検証用に任意指定できる。
fn setup_stage(window_origin: (f32, f32)) -> TestStage {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let dc = core.device_context().expect("device context").clone();

    let mut world = World::new();
    world.insert_resource(core);

    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(composite_render_system);

    // Window スポーン（on_add フックで Visual → Arrangement → GlobalArrangement が連鎖挿入）
    let window = world.spawn(Window::default()).id();
    world.flush();

    // フック挿入後に GlobalArrangement を上書き（スポーン同梱だとフックの default で潰されるため）
    let (ox, oy) = window_origin;
    *world.get_mut::<GlobalArrangement>(window).unwrap() = GlobalArrangement {
        transform: Matrix3x2::identity(),
        bounds: Rect {
            left: ox,
            top: oy,
            right: ox + 100.0,
            bottom: oy + 100.0,
        },
    };

    let compositor = WindowD3D11Compositor::new(&dc, 100, 100).expect("compositor");
    world.entity_mut(window).insert(compositor);

    TestStage {
        world,
        schedule,
        window,
    }
}

/// ウィンドウ配下に 40x40 の子 Visual を bounds (10,10)-(50,50) で生成する
fn spawn_child(stage: &mut TestStage, visual: Visual, cmd: Option<GraphicsCommandList>) -> Entity {
    let window_ga = stage
        .world
        .get::<GlobalArrangement>(stage.window)
        .unwrap()
        .clone();
    let (ox, oy) = (window_ga.bounds.left, window_ga.bounds.top);

    let child = stage
        .world
        .spawn((visual, ChildOf(stage.window)))
        .id();
    stage.world.flush();

    stage.world.get_mut::<Arrangement>(child).unwrap().size = Size {
        width: 40.0,
        height: 40.0,
    };
    *stage.world.get_mut::<GlobalArrangement>(child).unwrap() = GlobalArrangement {
        transform: Matrix3x2::identity(),
        bounds: Rect {
            left: ox + 10.0,
            top: oy + 10.0,
            right: ox + 50.0,
            bottom: oy + 50.0,
        },
    };
    if let Some(cmd) = cmd {
        stage.world.entity_mut(child).insert(cmd);
    }
    child
}

fn compositor_of<'a>(stage: &'a TestStage) -> &'a WindowD3D11Compositor {
    stage.world.get::<WindowD3D11Compositor>(stage.window).unwrap()
}

// ==========================================================================
// 基本合成 + dirty フラグ
// ==========================================================================

#[test]
fn composite_render_sets_dirty_and_draws_debug_border() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    spawn_child(&mut stage, Visual::default(), Some(cmd));

    stage.schedule.run(&mut stage.world);

    let compositor = compositor_of(&stage);
    assert!(compositor.is_dirty(), "合成完了後 dirty == true");

    // characterization: 現実装は外周 2px に赤デバッグ枠を無条件描画する
    let border = read_pixel(compositor, 0, 0);
    assert_eq!(
        border,
        [0, 0, 255, 255],
        "外周ピクセルは赤デバッグ枠 [B,G,R,A]（デバッグコード残置の固定）"
    );
}

#[test]
fn child_command_list_is_composited_at_global_bounds() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    spawn_child(&mut stage, Visual::default(), Some(cmd));

    stage.schedule.run(&mut stage.world);

    let compositor = compositor_of(&stage);
    // 子 bounds (10,10)-(50,50) の内側 → 青
    assert_eq!(
        read_pixel(compositor, 20, 20),
        [255, 0, 0, 255],
        "子 CommandList が bounds 位置に合成される（青 BGRA）"
    );
    // 子の外側（枠からも離れた位置）→ 透明
    assert_eq!(
        read_pixel(compositor, 60, 60),
        [0, 0, 0, 0],
        "子の範囲外は透明クリアのまま"
    );
}

#[test]
fn window_offset_is_compensated_for_ulw_bitmap() {
    // ウィンドウがスクリーン (200,100) にある場合でも DIB は (0,0) 起点
    let mut stage = setup_stage((200.0, 100.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    spawn_child(&mut stage, Visual::default(), Some(cmd));

    stage.schedule.run(&mut stage.world);

    let compositor = compositor_of(&stage);
    assert_eq!(
        read_pixel(compositor, 20, 20),
        [255, 0, 0, 255],
        "スクリーン座標からウィンドウ原点を差し引いた位置に描画される"
    );
    assert_eq!(read_pixel(compositor, 60, 60), [0, 0, 0, 0]);
}

// ==========================================================================
// 可視性 / opacity スキップ
// ==========================================================================

#[test]
fn invisible_child_subtree_is_skipped() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    spawn_child(
        &mut stage,
        Visual {
            is_visible: false,
            ..Default::default()
        },
        Some(cmd),
    );

    stage.schedule.run(&mut stage.world);

    let compositor = compositor_of(&stage);
    assert!(compositor.is_dirty(), "合成自体は実行される");
    assert_eq!(
        read_pixel(compositor, 20, 20),
        [0, 0, 0, 0],
        "is_visible == false のサブツリーは描画されない"
    );
}

#[test]
fn zero_opacity_child_is_skipped() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    spawn_child(
        &mut stage,
        Visual {
            opacity: 0.0,
            ..Default::default()
        },
        Some(cmd),
    );

    stage.schedule.run(&mut stage.world);

    assert_eq!(
        read_pixel(compositor_of(&stage), 20, 20),
        [0, 0, 0, 0],
        "opacity == 0.0 のサブツリーは描画されない"
    );
}

#[test]
fn fractional_opacity_reduces_alpha() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    spawn_child(
        &mut stage,
        Visual {
            opacity: 0.5,
            ..Default::default()
        },
        Some(cmd),
    );

    stage.schedule.run(&mut stage.world);

    let [_b, _g, _r, a] = read_pixel(compositor_of(&stage), 20, 20);
    assert!(
        a > 0 && a < 255,
        "opacity 0.5 は ColorMatrix Effect で alpha が減衰する（actual alpha = {a}）"
    );
}

// ==========================================================================
// ClipShape（ClipGuard の ULW クリップ経路）
// ==========================================================================

#[test]
fn rectangle_clip_constrains_drawing_to_arrangement_size() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    // 40x40 を塗る CommandList を 20x20 の Arrangement + Rectangle クリップで制限
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    let child = spawn_child(
        &mut stage,
        Visual {
            clip: Some(ClipShape::Rectangle),
            ..Default::default()
        },
        Some(cmd),
    );
    stage.world.get_mut::<Arrangement>(child).unwrap().size = Size {
        width: 20.0,
        height: 20.0,
    };

    stage.schedule.run(&mut stage.world);

    let compositor = compositor_of(&stage);
    // クリップ内（ローカル (10,10) → DIB (20,20)）→ 青
    assert_eq!(read_pixel(compositor, 20, 20), [255, 0, 0, 255], "クリップ内は描画");
    // クリップ外だが塗り範囲内（ローカル (25,25) → DIB (35,35)）→ 透明
    assert_eq!(
        read_pixel(compositor, 35, 35),
        [0, 0, 0, 0],
        "PushAxisAlignedClip がローカル (0,0)-(20,20) で描画を制限"
    );
}

#[test]
fn rounded_rectangle_clip_clears_corners() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    spawn_child(
        &mut stage,
        Visual {
            clip: Some(ClipShape::RoundedRectangle { radius: 15.0 }),
            ..Default::default()
        },
        Some(cmd),
    );

    stage.schedule.run(&mut stage.world);

    let compositor = compositor_of(&stage);
    // 中央（ローカル (20,20) → DIB (30,30)）→ 青
    assert_eq!(read_pixel(compositor, 30, 30), [255, 0, 0, 255], "角丸内部は描画");
    // 左上角（ローカル (1,1) → DIB (11,11)、円弧中心 (15,15) から約 19.8px > 15px）→ 透明
    assert_eq!(
        read_pixel(compositor, 11, 11),
        [0, 0, 0, 0],
        "PushLayer + RoundedRectangleGeometry が角をクリップ"
    );
}

#[test]
fn individual_corner_clip_applies_per_corner_radii() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    spawn_child(
        &mut stage,
        Visual {
            clip: Some(ClipShape::RoundedRectangleIndividual {
                top_left: 20.0,
                top_right: 0.0,
                bottom_left: 0.0,
                bottom_right: 0.0,
            }),
            ..Default::default()
        },
        Some(cmd),
    );

    stage.schedule.run(&mut stage.world);

    let compositor = compositor_of(&stage);
    // 左上角（ローカル (2,2) → DIB (12,12)、円弧中心 (20,20) から約 25.5px > 20px）→ 透明
    assert_eq!(
        read_pixel(compositor, 12, 12),
        [0, 0, 0, 0],
        "top_left = 20 の角はクリップされる"
    );
    // 右上角（ローカル (37,3) → DIB (47,13)、top_right = 0）→ 青
    assert_eq!(
        read_pixel(compositor, 47, 13),
        [255, 0, 0, 255],
        "radius 0 の角は PathGeometry の直線辺で描画が残る"
    );
}

// ==========================================================================
// ダーティ判定（is_window_dirty）
// ==========================================================================

#[test]
fn unchanged_window_skips_recomposition() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    let child = spawn_child(&mut stage, Visual::default(), Some(cmd));

    // 初回フレーム（is_added）→ 合成され dirty == true
    stage.schedule.run(&mut stage.world);
    assert!(compositor_of(&stage).is_dirty());

    // dirty を消費した状態を再現し、変更なしで再実行 → 再合成されない
    stage
        .world
        .get_mut::<WindowD3D11Compositor>(stage.window)
        .unwrap()
        .set_dirty(false);
    stage.schedule.run(&mut stage.world);
    assert!(
        !compositor_of(&stage).is_dirty(),
        "Changed なし → is_window_dirty == false で合成スキップ"
    );

    // 子 Visual の変更 → 再合成される
    stage.world.get_mut::<Visual>(child).unwrap().set_changed();
    stage.schedule.run(&mut stage.world);
    assert!(
        compositor_of(&stage).is_dirty(),
        "Changed<Visual> がサブツリー走査で検出され再合成"
    );
}

#[test]
fn invalid_compositor_is_skipped_without_panic() {
    let mut stage = setup_stage((0.0, 0.0));
    let core = stage.world.resource::<GraphicsCore>();
    let cmd = make_filled_command_list(core, 40.0, 40.0);
    spawn_child(&mut stage, Visual::default(), Some(cmd));

    stage
        .world
        .get_mut::<WindowD3D11Compositor>(stage.window)
        .unwrap()
        .invalidate();

    stage.schedule.run(&mut stage.world);

    let compositor = compositor_of(&stage);
    assert!(!compositor.is_valid());
    assert!(!compositor.is_dirty(), "無効 compositor は合成されない");
}

// ==========================================================================
// ulw_present_system
// ==========================================================================

fn present_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(ulw_present_system);
    schedule
}

fn null_handle() -> WindowHandle {
    WindowHandle {
        hwnd: HWND(std::ptr::null_mut()),
        instance: HINSTANCE(std::ptr::null_mut()),
    }
}

#[test]
fn ulw_present_skips_clean_compositor() {
    let core = GraphicsCore::new().expect("GraphicsCore");
    let dc = core.device_context().unwrap().clone();
    let mut world = World::new();

    let compositor = WindowD3D11Compositor::new(&dc, 64, 64).expect("compositor");
    assert!(!compositor.is_dirty());
    let entity = world.spawn((null_handle(), compositor)).id();
    world.flush();

    let mut schedule = present_schedule();
    schedule.run(&mut world);

    assert!(
        !world.get::<WindowD3D11Compositor>(entity).unwrap().is_dirty(),
        "dirty == false はスキップ（パニックなし）"
    );
}

#[test]
fn ulw_present_skips_invalid_compositor() {
    let core = GraphicsCore::new().expect("GraphicsCore");
    let dc = core.device_context().unwrap().clone();
    let mut world = World::new();

    let mut compositor = WindowD3D11Compositor::new(&dc, 64, 64).expect("compositor");
    compositor.set_dirty(true);
    compositor.invalidate(); // invalidate は dirty も false に戻す
    let entity = world.spawn((null_handle(), compositor)).id();
    world.flush();

    let mut schedule = present_schedule();
    schedule.run(&mut world);

    let c = world.get::<WindowD3D11Compositor>(entity).unwrap();
    assert!(!c.is_valid());
    assert!(!c.is_dirty(), "無効リソースはスキップされる");
}

#[test]
fn ulw_present_failure_keeps_dirty_for_retry() {
    let core = GraphicsCore::new().expect("GraphicsCore");
    let dc = core.device_context().unwrap().clone();
    let mut world = World::new();

    let mut compositor = WindowD3D11Compositor::new(&dc, 64, 64).expect("compositor");
    compositor.set_dirty(true);
    // 無効 HWND → UpdateLayeredWindow は必ず失敗する
    let entity = world.spawn((null_handle(), compositor)).id();
    world.flush();

    let mut schedule = present_schedule();
    schedule.run(&mut world);

    assert!(
        world.get::<WindowD3D11Compositor>(entity).unwrap().is_dirty(),
        "ULW 失敗時は dirty == true のまま（次フレーム再試行のセマンティクス）"
    );
}
