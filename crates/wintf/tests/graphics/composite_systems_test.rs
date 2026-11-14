//! 入れ物を通した描画面の実 GPU のテスト（wintf-d2d-composite-rules 7.1・要件 4.2, 6.1, 6.2, 7.10）と、
//! 差分の描き直しの範囲だけの道と全部の道の一致（wintf-visual-damage-redraw 6.3・要件 2.9, 4.2, 7.4, 7.5）。
//!
//! 本物の登録（`EcsWorld` の `PreRenderSurface` と `RenderSurface`）を実の装置と WUC の資源で回す。
//! 窓は HWND の無い `Window` の部品だけ（画面に出さない）。WUC の描画面の画素は
//! `ICompositionDrawingSurfaceInterop2::CopySurface` で D2D のビットマップへ写して読む
//! （research.md §10.5・テストの確かめ方で本番には使わない）。

use bevy_ecs::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use log_capture_kit::{CapturedEvent, capture, count_levels};
use std::mem::ManuallyDrop;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_BLEND_MODE_MULTIPLY, D2D1_COLOR_F,
    D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_CPU_READ,
    D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE,
    D2D1_INTERPOLATION_MODE_LINEAR, D2D1_MAP_OPTIONS_READ, ID2D1Bitmap1, ID2D1DeviceContext,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::IDXGISurface;
use windows::Win32::System::WinRT::Composition::ICompositionDrawingSurfaceInterop2;
use windows::core::Interface;
use windows_numerics::Matrix3x2;
use wintf::ecs::layout::{Arrangement, GlobalArrangement, Offset, Size as LayoutSize};
use wintf::ecs::world::{EcsWorld, PreRenderSurface, RenderSurface};
use wintf::ecs::{
    ClipShape, GraphicsCore, PixelRead, Rect, SurfaceGraphics, SurfaceGraphicsDirty, SurfaceImage,
    SurfaceReadback, Visual, VisualDrawContent, VisualTransform, Window, WucGraphicsResource,
};
use wintf::numerics::Aabb;

const SIZE: u32 = 32;

fn rect(l: f32, t: f32, r: f32, b: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left: l,
        top: t,
        right: r,
        bottom: b,
    }
}

/// `area` を `color` で塗る中身。`blend` なら混ぜ方は MULTIPLY、そうでなければ SOURCE_OVER。
fn filled(
    core: &GraphicsCore,
    area: D2D_RECT_F,
    color: D2D1_COLOR_F,
    blend: bool,
) -> VisualDrawContent {
    let device = core.d2d_device().expect("d2d device");
    // SAFETY: Windows API 境界。記録用の DC で命令の列を 1 本記録して閉じる。
    let list = unsafe {
        let dc = device
            .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("recording dc");
        let list = dc.CreateCommandList().expect("command list");
        dc.SetTarget(&list);
        dc.BeginDraw();
        let brush = dc.CreateSolidColorBrush(&color, None).expect("brush");
        dc.FillRectangle(&area, &brush);
        dc.EndDraw(None, None).expect("EndDraw");
        list.Close().expect("close");
        list
    };
    if blend {
        VisualDrawContent::new_blend(
            Aabb(area),
            list,
            D2D1_INTERPOLATION_MODE_LINEAR,
            D2D1_BLEND_MODE_MULTIPLY,
        )
    } else {
        VisualDrawContent::new(Aabb(area), list)
    }
}

fn color(r: f32, g: f32, b: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r, g, b, a: 1.0 }
}

/// 実の装置と WUC の資源を入れ、HWND の無い窓の下に 32x32 の持ち主と 3 つの節を置く。
/// 節は 赤の全面 → 不透明度 0.5 の青 (8,8)-(24,24) → MULTIPLY の黄 (12,12)-(20,20) の順。
fn setup() -> (EcsWorld, Entity) {
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
    let red = filled(
        &core,
        rect(0.0, 0.0, 32.0, 32.0),
        color(1.0, 0.0, 0.0),
        false,
    );
    let blue = filled(
        &core,
        rect(8.0, 8.0, 24.0, 24.0),
        color(0.0, 0.0, 1.0),
        false,
    );
    let yellow = filled(
        &core,
        rect(12.0, 12.0, 20.0, 20.0),
        color(1.0, 1.0, 0.0),
        true,
    );
    world.insert_resource(core);
    world.insert_resource(wuc);
    let window = world.spawn(Window::default()).id();
    let owner = world
        .spawn((
            Visual::default(),
            Arrangement {
                size: LayoutSize {
                    width: SIZE as f32,
                    height: SIZE as f32,
                },
                ..Default::default()
            },
            ChildOf(window),
        ))
        .id();
    let mut half = Visual::default();
    half.set_opacity(0.5);
    world.spawn((Visual::default(), red, ChildOf(owner)));
    world.spawn((half, blue, ChildOf(owner)));
    world.spawn((Visual::default(), yellow, ChildOf(owner)));
    world.flush();
    // レイアウトは回さないので、描画面の大きさは配置の結果で与える
    *world.get_mut::<GlobalArrangement>(owner).unwrap() = GlobalArrangement {
        transform: Matrix3x2::identity(),
        bounds: Rect {
            left: 0.0,
            top: 0.0,
            right: SIZE as f32,
            bottom: SIZE as f32,
        },
    };
    (ecs, owner)
}

fn bitmap(
    dc: &ID2D1DeviceContext,
    (width, height): (u32, u32),
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
    let size = D2D_SIZE_U { width, height };
    // SAFETY: Windows API 境界。初期の画素は渡さない。
    unsafe { dc.CreateBitmap(size, None, 0, &props) }.expect("CreateBitmap")
}

/// WUC の描画面を D2D のビットマップへ写し、行を幅×4 に詰めた BGRA で返す。
fn read_wuc_surface(world: &World, owner: Entity) -> Vec<u8> {
    let surface = world
        .get::<SurfaceGraphics>(owner)
        .unwrap()
        .surface()
        .expect("WUC surface")
        .cast::<ICompositionDrawingSurfaceInterop2>()
        .expect("ICompositionDrawingSurfaceInterop2");
    let size = world.get::<SurfaceGraphics>(owner).unwrap().size;
    let device = world.resource::<GraphicsCore>().d2d_device().expect("d2d");
    let row = size.0 as usize * 4;
    let mut bytes = vec![0u8; row * size.1 as usize];
    // SAFETY: Windows API 境界。`Map(READ)` の `bits` は少なくとも `pitch * 高さ` バイトを指す。
    unsafe {
        let dc = device
            .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("dc");
        let target = bitmap(&dc, size, D2D1_BITMAP_OPTIONS_TARGET);
        let dxgi: IDXGISurface = target.GetSurface().expect("IDXGISurface");
        surface.CopySurface(&dxgi, 0, 0, None).expect("CopySurface");
        let staging = bitmap(
            &dc,
            size,
            D2D1_BITMAP_OPTIONS_CPU_READ | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        );
        staging
            .CopyFromBitmap(None, &target, None)
            .expect("CopyFromBitmap");
        let mapped = staging.Map(D2D1_MAP_OPTIONS_READ).expect("Map");
        for y in 0..size.1 as usize {
            std::ptr::copy_nonoverlapping(
                mapped.bits.add(y * mapped.pitch as usize),
                bytes.as_mut_ptr().add(y * row),
                row,
            );
        }
        staging.Unmap().expect("Unmap");
    }
    bytes
}

fn near(actual: [u8; 4], expected: [u8; 4]) -> bool {
    actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1)
}

/// ブレンドの節と不透明度 0.5 の節を含む木を描くと、`error!` が 0 で描き直しが続かず、
/// WUC の描画面の画素が入れ物の画素とバイトで一致し、1 画素の読み戻しが答えの点の値を返す
#[test]
fn composite_reaches_wuc_surface_on_real_gpu() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut ecs, owner) = setup();
        let ((), counts) = count_levels(|| {
            let world = ecs.world_mut();
            world.run_schedule(PreRenderSurface);
            world.run_schedule(RenderSurface);
        });
        assert_eq!(counts.error, 0, "描画の係が error を残した");
        let world = ecs.world_mut();
        assert_eq!(
            world.get::<SurfaceGraphics>(owner).unwrap().size,
            (SIZE, SIZE)
        );

        // 次の回に描き直しの印が立ち直らない（失敗の立て直しが続かない）
        world.increment_change_tick();
        let this_run = world.change_tick();
        let ((), counts) = count_levels(|| {
            world.run_schedule(PreRenderSurface);
            world.run_schedule(RenderSurface);
        });
        assert_eq!(counts.error, 0, "2 回目の描画が error を残した");
        let dirty = world
            .entity(owner)
            .get_change_ticks::<SurfaceGraphicsDirty>()
            .unwrap()
            .changed;
        assert!(dirty.get() < this_run.get(), "描き直しが続いている");

        let wuc = read_wuc_surface(world, owner);
        let (image, outer, half, blended) = world
            .run_system_once(move |readback: SurfaceReadback| {
                (
                    readback.read_all(owner),
                    readback.read_pixel(owner, 4, 4),
                    readback.read_pixel(owner, 10, 10),
                    readback.read_pixel(owner, 16, 16),
                )
            })
            .unwrap();
        let SurfaceImage::Pixels {
            width,
            height,
            bytes,
        } = image
        else {
            panic!("入れ物を読めない: {image:?}");
        };
        assert_eq!((width, height), (SIZE, SIZE));
        assert!(bytes == wuc, "WUC の描画面の画素が入れ物の画素と違う");

        // 赤だけ・赤に青 0.5・その上に黄の MULTIPLY（α 掛け済みの BGRA・±1）
        let expect = [
            (outer, [0, 0, 255, 255]),
            (half, [128, 0, 128, 255]),
            (blended, [0, 0, 128, 255]),
        ];
        for (read, expected) in expect {
            let PixelRead::Pixel(px) = read else {
                panic!("1 画素を読めない: {read:?}");
            };
            assert!(
                near(px, expected),
                "答えの点の値が違う: {px:?} 見本 {expected:?}"
            );
        }
        world.clear_entities();
    });
}

// ---- 差分の描き直しの実 GPU の確かめ（wintf-visual-damage-redraw 6.3・要件 2.9, 4.2, 7.4, 7.5） ----

/// 範囲だけの道を通す持ち主の大きさ（DIP）。拡大 1.25 でもにじみの余白で全面にならない大きさ（6.1 と同じ）。
const SIDE: f32 = 40.0;

fn at(x: f32, y: f32, width: f32, height: f32) -> Arrangement {
    Arrangement {
        offset: Offset { x, y },
        size: LayoutSize { width, height },
        ..Default::default()
    }
}

/// `parent` の下に `(x, y)` の 8×8 の単色の件（混ぜ方 SOURCE_OVER・補間 LINEAR）。
fn item(world: &mut World, parent: Entity, x: f32, y: f32, c: D2D1_COLOR_F) -> Entity {
    let content = filled(
        world.resource::<GraphicsCore>(),
        rect(0.0, 0.0, 8.0, 8.0),
        c,
        false,
    );
    world
        .spawn((
            Visual::default(),
            at(x, y, 8.0, 8.0),
            content,
            ChildOf(parent),
        ))
        .id()
}

/// 中身なしの節（変形を持つので節になる）。
fn group(world: &mut World, parent: Entity, visual: Visual, area: Arrangement) -> Entity {
    let parts = (visual, area, VisualTransform::default(), ChildOf(parent));
    world.spawn(parts).id()
}

/// 持ち主の下に木を組む（返した節が `Change` の `n`）。
type Build = fn(&mut World, Entity) -> Vec<Entity>;
/// 1 フレームの変え方。
type Change = fn(&mut World, &[Entity]);

/// 自分の窓の下に、`name` の SIDE 四方・拡大 `s` の持ち主を置き、`build` で木を組む（根の命令の列は
/// 持たない＝範囲だけの道に乗る・R1 の規則 2）。窓を分けるのは、後から足す持ち主が先の持ち主の窓の子を
/// 変えないため。
fn owner(world: &mut World, name: &'static str, s: f32, build: Build) -> (Entity, Vec<Entity>) {
    let window = world.spawn(Window::default()).id();
    let owner = world
        .spawn((
            Visual::default(),
            Name::new(name),
            at(0.0, 0.0, SIDE, SIDE),
            ChildOf(window),
        ))
        .id();
    let nodes = build(world, owner);
    world.flush();
    // レイアウトは回さないので、描画面の大きさと拡大は配置の結果で与える
    *world.get_mut::<GlobalArrangement>(owner).unwrap() = GlobalArrangement {
        transform: Matrix3x2 {
            M11: s,
            M12: 0.0,
            M21: 0.0,
            M22: s,
            M31: 0.0,
            M32: 0.0,
        },
        bounds: Rect {
            left: 0.0,
            top: 0.0,
            right: SIDE * s,
            bottom: SIDE * s,
        },
    };
    (owner, nodes)
}

/// 1 フレーム描き（`PreRenderSurface` と `RenderSurface`）、`error!` が 0 であることを確かめて記録を返す。
fn frame(world: &mut World) -> Vec<CapturedEvent> {
    world.increment_change_tick();
    let ((), events) = capture(|| {
        world.run_schedule(PreRenderSurface);
        world.run_schedule(RenderSurface);
    });
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert!(errors.is_empty(), "描画の係が error を残した: {errors:?}");
    events
}

/// `name` の描画面の、この回の描き直しの量の記録（1 行）。
fn damage<'a>(events: &'a [CapturedEvent], name: &str) -> &'a CapturedEvent {
    let mut lines = events
        .iter()
        .filter(|e| e.message() == "[render_surface] damage" && e.field("entity") == Some(name));
    let line = lines
        .next()
        .unwrap_or_else(|| panic!("{name} の描き直しの記録が無い"));
    assert!(lines.next().is_none(), "{name} の記録が 2 行ある");
    line
}

fn number(line: &CapturedEvent, field: &str) -> u64 {
    line.field(field)
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("記録の欄 {field} が数でない: {line:?}"))
}

/// 入れ物の画素（行を幅×4 に詰めた BGRA）。
fn composite_bytes(world: &mut World, owner: Entity) -> Vec<u8> {
    let image = world
        .run_system_once(move |readback: SurfaceReadback| readback.read_all(owner))
        .unwrap();
    let SurfaceImage::Pixels { bytes, .. } = image else {
        panic!("入れ物を読めない: {image:?}");
    };
    bytes
}

/// 2 つの絵がバイトで同じ（違えば最初の画素の座標と値を出す）。
fn assert_same(got: &[u8], want: &[u8], width: u32, what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: 大きさが違う");
    if let Some(i) = got.iter().zip(want).position(|(g, w)| g != w) {
        let px = i / 4;
        let (x, y) = (px % width as usize, px / width as usize);
        let at = px * 4;
        panic!(
            "{what}: ({x},{y}) が違う: {:?} 見本 {:?}",
            &got[at..at + 4],
            &want[at..at + 4]
        );
    }
}

/// 同じ装置の描画面 2 枚で、範囲だけの道と初めから全部の道の答えをバイトで比べる（要件 2.9・4.2・7.4・7.5）。
///
/// 1. 持ち主 A を `build` の形で全部で描いて写す。
/// 2. A を `change` で変え、同じ回に B（`build` → `change` の形・作りたて）を足して描く。A は範囲だけの道
///    （`want_shift` なら平行移動の写しも）で、描画面へ写したのは全面より狭い範囲。B は全部。A と B の
///    入れ物と WUC の描画面の画素がバイトで一致する（A の写した範囲の外が前の写しのまま残らなければここで
///    違う）。A の画面は 1 の画面から変わっている。
/// 3. A に描き直しの印だけを立てて描く。何もしない回で、入れ物と描画面の画素は前のまま。
fn check_partial_matches_full(s: f32, build: Build, change: Change, want_shift: bool) {
    crate::common::on_gpu_owner_thread(move || {
        let mut ecs = EcsWorld::new();
        let world = ecs.world_mut();
        let core = GraphicsCore::new().expect("GraphicsCore");
        let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
        world.insert_resource(core);
        world.insert_resource(wuc);

        let (a, a_nodes) = owner(world, "A", s, build);
        let events = frame(world);
        assert_eq!(damage(&events, "A").field_str("mode"), Some("full"));
        let width = world.get::<SurfaceGraphics>(a).unwrap().size.0;
        let before = read_wuc_surface(world, a);

        world.increment_change_tick();
        change(world, &a_nodes);
        let (b, b_nodes) = owner(world, "B", s, build);
        change(world, &b_nodes);
        let events = frame(world);
        let line = damage(&events, "A");
        assert_eq!(line.field_str("mode"), Some("partial"), "A: {line:?}");
        assert!(
            number(line, "present_area") < number(line, "surface_area"),
            "A は描画面の一部だけを写すはず: {line:?}"
        );
        assert_eq!(
            number(line, "shift_area") > 0,
            want_shift,
            "A の写しの有無: {line:?}"
        );
        assert_eq!(damage(&events, "B").field_str("mode"), Some("full"));
        let a_wuc = read_wuc_surface(world, a);
        let a_bytes = composite_bytes(world, a);
        assert_same(&a_bytes, &composite_bytes(world, b), width, "入れ物");
        assert_same(&a_wuc, &read_wuc_surface(world, b), width, "WUC の描画面");
        assert_same(&a_wuc, &a_bytes, width, "A の WUC の描画面と入れ物");
        assert!(a_wuc != before, "動かした節が画面に出ていない");

        world.increment_change_tick();
        world
            .get_mut::<SurfaceGraphicsDirty>(a)
            .unwrap()
            .set_changed();
        let events = frame(world);
        assert_eq!(damage(&events, "A").field_str("mode"), Some("skip"));
        assert_same(
            &composite_bytes(world, a),
            &a_bytes,
            width,
            "何もしない回の入れ物",
        );
        assert_same(
            &read_wuc_surface(world, a),
            &a_wuc,
            width,
            "何もしない回の WUC",
        );
        world.clear_entities();
    });
}

/// LINEAR の赤 (3.5,3.5)・青 (15.25,4.5) と、離れた黄 (26,26)（写した範囲の外に残る絵）。返すのは [赤]。
fn linear_items(world: &mut World, root: Entity) -> Vec<Entity> {
    let red = item(world, root, 3.5, 3.5, color(1.0, 0.0, 0.0));
    item(world, root, 15.25, 4.5, color(0.0, 0.0, 1.0));
    item(world, root, 26.0, 26.0, color(1.0, 1.0, 0.0));
    vec![red]
}

/// 節を 1 つ（LINEAR の件を小数の位置へ）動かした回の画面が今の答えになり、描き直しの印だけの回は前のまま。
#[test]
fn moved_linear_item_at_fractional_position_partial_matches_full_on_real_gpu() {
    for s in [1.0, 1.25] {
        check_partial_matches_full(
            s,
            linear_items,
            |w, n| {
                w.entity_mut(n[0]).insert(at(4.75, 2.25, 8.0, 8.0));
            },
            false,
        );
    }
}

/// 不透明度 0.5 の層の組 (2,2)-(34,12) の中の赤（左端）と青（右端）・離れた黄 (26,26)。返すのは [青]。
/// 青を動かすと描く矩形は青の周りだけで、層は描く矩形の縁をまたぐ。
fn layer_across_rect(world: &mut World, root: Entity) -> Vec<Entity> {
    let mut half = Visual::default();
    half.set_opacity(0.5);
    let layer = group(world, root, half, at(2.0, 2.0, 32.0, 10.0));
    item(world, layer, 0.0, 0.0, color(1.0, 0.0, 0.0));
    let blue = item(world, layer, 22.0, 0.0, color(0.0, 0.0, 1.0));
    item(world, root, 26.0, 26.0, color(1.0, 1.0, 0.0));
    vec![blue]
}

#[test]
fn opacity_layer_across_draw_rect_edge_partial_matches_full_on_real_gpu() {
    for s in [1.0, 1.25] {
        check_partial_matches_full(
            s,
            layer_across_rect,
            |w, n| {
                w.entity_mut(n[0]).insert(at(23.0, 1.0, 8.0, 8.0));
            },
            false,
        );
    }
}

/// 根の下の矩形の切り抜き (8,8)-(32,32) の中に動かす中身（24×40）。中身には赤 (2,2)・小数の位置の青・
/// 下の外の緑（入る）・上へはみ出す黄（出る）。6.2 の WARP で上へ 4 の写しが効く形（余白込みで件の範囲が
/// 重ならない・規則 (1) が帯を写しの先へ広げない）。返すのは [中身]。
fn scroll_box(world: &mut World, root: Entity) -> Vec<Entity> {
    let clip = Visual {
        clip: Some(ClipShape::Rectangle),
        ..Default::default()
    };
    let clip = group(world, root, clip, at(8.0, 8.0, 24.0, 24.0));
    let content = group(world, clip, Visual::default(), at(0.0, 0.0, 24.0, 40.0));
    item(world, content, 2.0, 2.0, color(1.0, 0.0, 0.0));
    item(world, content, 13.5, 11.25, color(0.0, 0.0, 1.0));
    item(world, content, 2.0, 27.0, color(0.0, 1.0, 0.0));
    item(world, content, 14.0, -6.5, color(1.0, 1.0, 0.0));
    vec![content]
}

/// 拡大 1.25 で中身を上へ 4 DIP（5 px の整数）動かすと平行移動の写しを使い、全部の答えとバイトで一致する。
#[test]
fn integer_shift_copy_at_scale_1_25_matches_full_on_real_gpu() {
    check_partial_matches_full(
        1.25,
        scroll_box,
        |w, n| {
            w.entity_mut(n[0]).insert(at(0.0, -4.0, 24.0, 40.0));
        },
        true,
    );
}
