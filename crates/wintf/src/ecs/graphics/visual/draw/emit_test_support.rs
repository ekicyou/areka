//! WARP の足場（要件 6.1・6.2・6.4）: 画面に出さずに木を絵へ描き、CPU へ読み戻す。
//! 環境変数の門なしで普通のテストとして走る。画素の答えに限って WARP を使う（実 GPU のテストは別に残す）。
//!
//! 工場と装置はプロセスで 1 度だけ作り、テストごとに作るのは DC と的のビットマップだけ。
//! 節の描く中身はこの装置の DC で記録すること（命令の列は装置に結びつく・
//! `draw_content_test_support::solid_rect_content` の WIC の中身は使えない）。

use super::super::*;
use super::builder::*;
use super::command::DrawCommand;
use super::emit::emit;
use crate::com::d3d11::d3d11_create_device;
use crate::numerics::Aabb;
use bevy_ecs::system::SystemState;
use std::mem::ManuallyDrop;
use std::sync::OnceLock;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::core::Interface;
use windows_numerics::Matrix3x2;

struct Warp {
    factory: ID2D1Factory1,
    device: ID2D1Device,
}

// SAFETY: 工場は MULTI_THREADED で、同じ工場の系列（装置・DC・資源）への呼び出しは D2D が内部で
// 直列化する。D3D11 の装置は free-threaded。テストはスレッドごとに自分の DC を作って使う。
unsafe impl Send for Warp {}
unsafe impl Sync for Warp {}

fn warp() -> &'static Warp {
    static WARP: OnceLock<Warp> = OnceLock::new();
    WARP.get_or_init(|| {
        let d3d = d3d11_create_device(
            None,
            D3D_DRIVER_TYPE_WARP,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            None,
            None,
        )
        .expect("WARP の D3D11 装置");
        let dxgi: IDXGIDevice = d3d.cast().expect("IDXGIDevice");
        let factory: ID2D1Factory1 =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None) }.expect("工場");
        let device = unsafe { factory.CreateDevice(&dxgi) }.expect("D2D の装置");
        Warp { factory, device }
    })
}

/// WARP の D2D の装置（プロセスで 1 つ）。入れ物や読み戻しのテストが自分で DC を作るときに使う。
pub(crate) fn warp_device() -> &'static ID2D1Device {
    &warp().device
}

/// WARP の装置の新しい DC（96 DPI）。
pub(crate) fn warp_dc() -> ID2D1DeviceContext {
    let dc =
        unsafe { warp_device().CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE) }.expect("DC");
    unsafe { dc.SetDpi(96.0, 96.0) };
    dc
}

/// 装置と同じ工場（切り抜きの図形はこれで作る・別の工場の図形は D2DERR_WRONG_FACTORY）。
pub(crate) fn warp_factory() -> ID2D1Factory {
    warp().factory.cast().unwrap()
}

/// BGRA（premultiplied）。
pub(crate) const RED: [u8; 4] = [0, 0, 255, 255];
pub(crate) const BLUE: [u8; 4] = [255, 0, 0, 255];
pub(crate) const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// 単色の矩形を 1 つ塗った描く中身（WARP の装置で記録）。`bgra` は不透明の色。
pub(crate) fn solid(rect: D2D_RECT_F, bgra: [u8; 4]) -> VisualDrawContent {
    let dc = warp_dc();
    let color = D2D1_COLOR_F {
        r: bgra[2] as f32 / 255.0,
        g: bgra[1] as f32 / 255.0,
        b: bgra[0] as f32 / 255.0,
        a: bgra[3] as f32 / 255.0,
    };
    unsafe {
        let list = dc.CreateCommandList().unwrap();
        dc.SetTarget(&list);
        dc.BeginDraw();
        let brush = dc.CreateSolidColorBrush(&color, None).unwrap();
        dc.FillRectangle(&rect, &brush);
        dc.EndDraw(None, None).unwrap();
        dc.SetTarget(None);
        list.Close().unwrap();
        VisualDrawContent::new(rect.into(), list)
    }
}

/// 親の中の `(x, y)` に `w`×`h` の配置。
pub(crate) fn at(x: f32, y: f32, w: f32, h: f32) -> crate::ecs::layout::Arrangement {
    crate::ecs::layout::Arrangement {
        offset: crate::ecs::Offset { x, y },
        size: crate::ecs::Size {
            width: w,
            height: h,
        },
        ..Default::default()
    }
}

pub(crate) fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

pub(crate) fn px(left: i32, top: i32, right: i32, bottom: i32) -> PxRect {
    PxRect {
        left,
        top,
        right,
        bottom,
    }
}

/// 市松の `side`×`side` の絵を 8×8 の矩形へ `DrawBitmap` で描く中身（補間が絵の縁の外ににじみうる・
/// `side` が 8 より大きければ中身の中で縮める）。
pub(crate) fn checker(
    bgra: [u8; 4],
    mode: D2D1_INTERPOLATION_MODE,
    side: u32,
) -> VisualDrawContent {
    let dc = warp_dc();
    let data: Vec<u8> = (0..side * side)
        .flat_map(|i| {
            if (i % side + i / side).is_multiple_of(2) {
                bgra
            } else {
                CLEAR
            }
        })
        .collect();
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
        colorContext: ManuallyDrop::new(None),
    };
    unsafe {
        let bitmap = dc
            .CreateBitmap(
                D2D_SIZE_U {
                    width: side,
                    height: side,
                },
                Some(data.as_ptr() as *const _),
                side * 4,
                &props,
            )
            .unwrap();
        let list = dc.CreateCommandList().unwrap();
        dc.SetTarget(&list);
        dc.BeginDraw();
        let dest = rect(0.0, 0.0, 8.0, 8.0);
        dc.DrawBitmap(&bitmap, Some(&dest), 1.0, mode, None, None);
        dc.EndDraw(None, None).unwrap();
        dc.SetTarget(None);
        list.Close().unwrap();
        VisualDrawContent {
            interpolation_mode: mode,
            ..VisualDrawContent::new(Aabb::new(0.0, 0.0, 8.0, 8.0), list)
        }
    }
}

/// 市松の絵の件 A・B（同じ変形 `t`・補間 `mode`・絵の大きさ `side`・B は A の動く範囲にかかる）と、
/// B の上の小さい件。返すのは [A, B, 小さい件]。
pub(crate) fn checker_pair(
    w: &mut World,
    root: Entity,
    t: crate::numerics::Transform2D,
    mode: D2D1_INTERPOLATION_MODE,
    side: u32,
) -> Vec<Entity> {
    let mut spawn = |x, y, c: VisualDrawContent| {
        let parts = (Visual::default(), at(x, y, 8.0, 8.0), c, ChildOf(root));
        w.spawn((parts, VisualTransform(t))).id()
    };
    let a = spawn(4.5, 4.25, checker(RED, mode, side));
    let b = spawn(15.0, 13.0, checker(BLUE, mode, side));
    let small = (
        at(18.0, 16.0, 3.0, 3.0),
        solid(rect(0.0, 0.0, 3.0, 3.0), RED),
    );
    vec![
        a,
        b,
        w.spawn((Visual::default(), small, ChildOf(root))).id(),
    ]
}

/// 読み戻した絵（行を詰めた BGRA）。
pub(crate) struct Pixels {
    pub width: u32,
    pub bytes: Vec<u8>,
}

impl Pixels {
    pub(crate) fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.width + x) * 4) as usize;
        self.bytes[i..i + 4].try_into().unwrap()
    }
}

/// `(x, y)` の画素が `expected` と 1 チャネル ±1 で一致するか（外れたら座標と値つきで panic）。
pub(crate) fn pixel_near(actual: &Pixels, expected: [u8; 4], x: u32, y: u32) {
    let got = actual.at(x, y);
    let near = got
        .iter()
        .zip(expected)
        .all(|(&a, e)| (a as i16 - e as i16).abs() <= 1);
    assert!(near, "({x}, {y}): got {got:?}, expected {expected:?}");
}

fn make_bitmap(
    dc: &ID2D1DeviceContext,
    w: u32,
    h: u32,
    options: D2D1_BITMAP_OPTIONS,
) -> ID2D1Bitmap1 {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: options,
        colorContext: ManuallyDrop::new(None),
    };
    unsafe {
        dc.CreateBitmap(
            D2D_SIZE_U {
                width: w,
                height: h,
            },
            None,
            0,
            &props,
        )
    }
    .expect("ビットマップ")
}

/// 根 `root` の下の木を本番と同じ順（`fill_world_visuals`→`build`→`emit`）で `w×h` の透明な
/// 的へ描き、読み戻す。viewport は本番と同じく根の `Arrangement.size` の矩形
/// （大きさ 0 の根は全部間引かれる）。`root_transform` は DIP→物理。
pub(crate) fn render_tree_to_pixels(
    world: &mut World,
    root: Entity,
    w: u32,
    h: u32,
    root_transform: Matrix3x2,
) -> Pixels {
    let commands = tree_commands(world, root, None);
    replay_to_pixels(&commands, w, h, root_transform)
}

/// 根 `root` の下の木を組んだ描く列。`viewport` が `None` なら本番と同じ根の `Arrangement.size`。
/// 呼ぶたびに変化の覚えを新しくするので、全部の節の版が上がる（続くフレームは [`TreeFrames`]）。
pub(crate) fn tree_commands(
    world: &mut World,
    root: Entity,
    viewport: Option<Aabb>,
) -> Vec<DrawCommand> {
    TreeFrames::new(world).commands(world, root, viewport)
}

/// 続くフレームの描く列を組む。変化の覚えを持ち続けるので、変えない節の版は上がらない（本番の係と同じ）。
pub(crate) struct TreeFrames(SystemState<Query<'static, 'static, FillData>>);

impl TreeFrames {
    pub(crate) fn new(world: &mut World) -> Self {
        Self(SystemState::new(world))
    }

    /// [`tree_commands`] と同じ列（この覚えの前の呼び出しからの変化だけ版を上げる）。
    pub(crate) fn commands(
        &mut self,
        world: &mut World,
        root: Entity,
        viewport: Option<Aabb>,
    ) -> Vec<DrawCommand> {
        build_commands(world, root, viewport, &mut self.0)
    }
}

fn build_commands(
    world: &mut World,
    root: Entity,
    viewport: Option<Aabb>,
    fill_state: &mut SystemState<Query<FillData>>,
) -> Vec<DrawCommand> {
    let top = world
        .get::<Children>(root)
        .map(|c| c.to_vec())
        .unwrap_or_default();
    let mut visited = Vec::new();
    let mut fill_nodes = fill_state.get_mut(world).unwrap();
    fill_world_visuals(
        root,
        None,
        &mut fill_nodes,
        false,
        &top,
        false,
        &mut visited,
        &mut Default::default(),
    );

    let viewport = viewport.unwrap_or_else(|| {
        let size = world
            .get::<crate::ecs::layout::Arrangement>(root)
            .map(|a| a.size)
            .unwrap_or_default();
        Aabb::new(0.0, 0.0, size.width, size.height)
    });
    let factory = warp_factory();
    let unit: ID2D1Geometry = unsafe {
        factory.CreateRectangleGeometry(&D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: 1.0,
            bottom: 1.0,
        })
    }
    .unwrap()
    .cast()
    .unwrap();
    let mut commands: Vec<DrawCommand> = Vec::new();
    let mut build_state: SystemState<Query<NodeData>> = SystemState::new(world);
    let nodes = build_state.get(world).unwrap();
    build(
        &mut commands,
        &factory,
        &unit,
        viewport,
        &nodes,
        root,
        None,
        &visited,
    );
    commands
}

/// 列 `commands` を `emit` で `w×h` の透明な的へ再生し、読み戻す。本番と同じく DC6 を cast して渡す。
pub(crate) fn replay_to_pixels(
    commands: &[DrawCommand],
    w: u32,
    h: u32,
    root_transform: Matrix3x2,
) -> Pixels {
    replay(commands, w, h, root_transform, true)
}

/// `replay_to_pixels` と同じだが DC6 を渡さない（ブレンドの描き方が無い Windows の道・1.7）。
pub(crate) fn replay_to_pixels_without_blend(
    commands: &[DrawCommand],
    w: u32,
    h: u32,
    root_transform: Matrix3x2,
) -> Pixels {
    replay(commands, w, h, root_transform, false)
}

fn replay(
    commands: &[DrawCommand],
    w: u32,
    h: u32,
    root_transform: Matrix3x2,
    with_blend: bool,
) -> Pixels {
    draw_to_pixels(w, h, root_transform, |dc| {
        let dc6 = if with_blend { dc.cast().ok() } else { None };
        emit(dc, dc6.as_ref(), root_transform, commands);
        let mut after = Matrix3x2::default();
        // SAFETY: Windows API 境界。
        unsafe { dc.GetTransform(&mut after) };
        assert_eq!(after, root_transform, "emit は DC の変換を根に戻す");
    })
}

/// 木を持たない描画面の今の道（`render_surface`）と同じ形: 根の変換のまま命令の列を 1 度
/// `DrawImage`（`LINEAR`・`SOURCE_OVER`）する。
pub(crate) fn render_list_to_pixels(
    list: &ID2D1CommandList,
    w: u32,
    h: u32,
    root_transform: Matrix3x2,
) -> Pixels {
    draw_to_pixels(w, h, root_transform, |dc| unsafe {
        dc.DrawImage(
            list,
            None,
            None,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_COMPOSITE_MODE_SOURCE_OVER,
        )
    })
}

/// `w×h` の透明な的に `root_transform` を置いて `draw` で描き、読み戻す。
pub(crate) fn draw_to_pixels(
    w: u32,
    h: u32,
    root_transform: Matrix3x2,
    draw: impl FnOnce(&ID2D1DeviceContext),
) -> Pixels {
    let dc = warp_dc();
    let target = make_bitmap(&dc, w, h, D2D1_BITMAP_OPTIONS_TARGET);
    let staging = make_bitmap(
        &dc,
        w,
        h,
        D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
    );
    let row = (w as usize) * 4;
    let mut bytes = vec![0u8; row * h as usize];
    // SAFETY: Windows API 境界。`Map(READ)` の `bits` は少なくとも `pitch * h` バイトを指す。
    unsafe {
        dc.SetTarget(&target);
        dc.BeginDraw();
        dc.SetTransform(&root_transform);
        dc.Clear(Some(&D2D1_COLOR_F::default()));
        draw(&dc);
        dc.EndDraw(None, None).expect("EndDraw");
        dc.SetTarget(None);
        staging.CopyFromBitmap(None, &target, None).unwrap();
        let mapped = staging.Map(D2D1_MAP_OPTIONS_READ).unwrap();
        for y in 0..h as usize {
            std::ptr::copy_nonoverlapping(
                mapped.bits.add(y * mapped.pitch as usize),
                bytes.as_mut_ptr().add(y * row),
                row,
            );
        }
        staging.Unmap().unwrap();
    }
    Pixels { width: w, bytes }
}
