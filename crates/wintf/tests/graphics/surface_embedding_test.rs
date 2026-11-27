//! 節の下の持ち主と持ち主の変形が WUC に写ることの実 GPU のテスト。
//!
//! 本物の登録（`EcsWorld` の `PreRenderSurface`・`RenderSurface`・`Composition`）を実の装置と WUC の資源で
//! 回す。窓は HWND の無い `Window` の部品だけで、画面の画素は読まない。WUC の Visual の親・位置・
//! 変形の行列・不透明度・子の並び・切り抜きを読み戻し、描画面の画素は読み戻しの口（入れ物の写し）と
//! WUC の描画面そのもの（`CopySurface`）の両方で読む。
//! 配置の伝播は回さないので、持ち主の `GlobalArrangement`（DPI の拡大と描画面の大きさ）は手で置く。

use bevy_ecs::prelude::*;
use bevy_ecs::system::RunSystemOnce;
use dola::TimePoint;
use dola::time::testing::{at_ms, ms};
use windows::UI::Composition::{
    CompositionGeometricClip, CompositionRoundedRectangleGeometry, ContainerVisual,
    Visual as WucVisual,
};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D1_COLOR_F};
use windows::core::{IUnknown, Interface};
use windows_numerics::{Matrix3x2, Matrix4x4, Vector2};
use wintf::ecs::layout::{Arrangement, GlobalArrangement, Offset, Size as LayoutSize};
use wintf::ecs::world::{Composition, EcsWorld, PreRenderSurface, RenderSurface};
use wintf::ecs::{
    ClipShape, DrawCommand, DrawTree, FrameTime, GraphicsCore, OwnSurface, Rect, SurfaceGraphics,
    SurfaceImage, SurfaceReadback, Visual, VisualDrawContent, VisualGraphics, VisualTransform,
    VisualVersion, Window, WucGraphicsResource,
};
use wintf::numerics::{Aabb, Transform2D};

const EPS: f32 = 1e-4;
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

/// 実の装置と WUC の資源を入れた世界と、HWND の無い窓。
fn rig() -> (EcsWorld, Entity) {
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d")).expect("WUC");
    world.insert_resource(core);
    world.insert_resource(wuc);
    let window = world.spawn(Window::default()).id();
    (ecs, window)
}

/// `(x, y)` に置いた大きさ `w`×`h` の配置（DIP）。
fn at(x: f32, y: f32, w: f32, h: f32) -> Arrangement {
    Arrangement {
        offset: Offset { x, y },
        size: LayoutSize {
            width: w,
            height: h,
        },
        ..Default::default()
    }
}

/// 局所の `rect` を色 `rgba` で塗る命令の列を持つ描く中身（実の装置の DC で記録する）。
fn solid(core: &GraphicsCore, rect: D2D_RECT_F, rgba: [f32; 4]) -> VisualDrawContent {
    let dc = core.device_context().expect("device context");
    let list = unsafe { dc.CreateCommandList() }.expect("command list");
    unsafe {
        dc.SetTarget(&list);
        dc.BeginDraw();
        let brush = dc
            .CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: rgba[0],
                    g: rgba[1],
                    b: rgba[2],
                    a: rgba[3],
                },
                None,
            )
            .expect("brush");
        dc.FillRectangle(&rect, &brush);
        dc.EndDraw(None, None).expect("EndDraw");
        dc.SetTarget(None);
        list.Close().expect("close");
    }
    VisualDrawContent::new(Aabb(rect), list)
}

fn r(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}

/// 持ち主の DPI の拡大 `s` と描画面の物理の大きさ（大きさ `w`×`h` の DIP から）を置く。
fn owner_global(world: &mut World, e: Entity, s: f32, w: f32, h: f32) {
    *world.get_mut::<GlobalArrangement>(e).unwrap() = GlobalArrangement {
        transform: Matrix3x2::scale(s, s),
        bounds: Rect {
            left: 0.0,
            top: 0.0,
            right: w * s,
            bottom: h * s,
        },
    };
}

/// 1 回ぶん（描画面の部品と木 → 描く → WUC の木と性質と切り抜き）。
fn frame(world: &mut World) {
    world.run_schedule(PreRenderSurface);
    world.run_schedule(RenderSurface);
    world.run_schedule(Composition);
}

fn wuc(world: &World, e: Entity) -> WucVisual {
    world
        .get::<VisualGraphics>(e)
        .and_then(|g| g.visual())
        .cloned()
        .unwrap_or_else(|| panic!("{e:?} に WUC の Visual が無い"))
}

/// COM の同一性（IUnknown の番地）。
fn id(v: &impl Interface) -> usize {
    v.cast::<IUnknown>().unwrap().as_raw() as usize
}

/// WUC の親（無ければ `None`）。
fn wuc_parent(v: &WucVisual) -> Option<usize> {
    v.Parent().ok().map(|p| id(&p))
}

/// WUC の子の並び（下から上）。
fn wuc_children(v: &WucVisual) -> Vec<usize> {
    let it = v
        .cast::<ContainerVisual>()
        .unwrap()
        .Children()
        .unwrap()
        .First()
        .unwrap();
    let mut out = Vec::new();
    while it.HasCurrent().unwrap() {
        out.push(id(&it.Current().unwrap()));
        it.MoveNext().unwrap();
    }
    out
}

fn ids(world: &World, es: &[Entity]) -> Vec<usize> {
    es.iter().map(|&e| id(&wuc(world, e))).collect()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < EPS
}

fn assert_offset(v: &WucVisual, x: f32, y: f32, what: &str) {
    let o = v.Offset().unwrap();
    assert!(
        near(o.X, x) && near(o.Y, y) && o.Z == 0.0,
        "{what}: Offset {o:?}, expected ({x}, {y})"
    );
}

fn assert_matrix(v: &WucVisual, expected: Matrix4x4, what: &str) {
    let m = v.TransformMatrix().unwrap();
    let got = [m.M11, m.M12, m.M21, m.M22, m.M41, m.M42, m.M33, m.M44];
    let want = [
        expected.M11,
        expected.M12,
        expected.M21,
        expected.M22,
        expected.M41,
        expected.M42,
        expected.M33,
        expected.M44,
    ];
    assert!(
        got.iter().zip(want).all(|(&a, b)| near(a, b)),
        "{what}: TransformMatrix {got:?}, expected {want:?}"
    );
}

fn identity4() -> Matrix4x4 {
    Matrix4x4::translation(0.0, 0.0, 0.0)
}

/// DIP の行列 `m` を拡大 `s` の WUC の行列へ（線形はそのまま・平行移動だけ拡大）。
fn wuc_matrix(m: Matrix3x2, s: f32) -> Matrix4x4 {
    Matrix4x4 {
        M11: m.M11,
        M12: m.M12,
        M21: m.M21,
        M22: m.M22,
        M33: 1.0,
        M41: m.M31 * s,
        M42: m.M32 * s,
        M44: 1.0,
        ..Default::default()
    }
}

/// 切り抜きの図形の位置と大きさと、切り抜きの変形。
fn wuc_clip(v: &WucVisual) -> (Vector2, Vector2, Matrix3x2) {
    let clip = v.Clip().expect("切り抜きが無い");
    let geometry = clip
        .cast::<CompositionGeometricClip>()
        .unwrap()
        .Geometry()
        .unwrap()
        .cast::<CompositionRoundedRectangleGeometry>()
        .unwrap();
    (
        geometry.Offset().unwrap(),
        geometry.Size().unwrap(),
        clip.TransformMatrix().unwrap(),
    )
}

fn invert(m: Matrix3x2) -> Matrix3x2 {
    let det = m.M11 * m.M22 - m.M12 * m.M21;
    Matrix3x2 {
        M11: m.M22 / det,
        M12: -m.M12 / det,
        M21: -m.M21 / det,
        M22: m.M11 / det,
        M31: (m.M21 * m.M32 - m.M22 * m.M31) / det,
        M32: (m.M12 * m.M31 - m.M11 * m.M32) / det,
    }
}

/// 窓 → 持ち主 R (100×100) → [節 N (10,20) 50×40・不透明度 0.5・矩形の切り抜き → 手動の持ち主 M (5,6) 16×14,
/// 手動の持ち主 P (70,70) 8×8]。返すのは (R, N, M, P)。
fn embedded_tree(world: &mut World, window: Entity, s: f32) -> (Entity, Entity, Entity, Entity) {
    let core = world.resource::<GraphicsCore>();
    let (n_content, m_content, p_content) = (
        solid(core, r(0.0, 0.0, 3.0, 3.0), BLUE),
        solid(core, r(0.0, 0.0, 16.0, 14.0), RED),
        solid(core, r(0.0, 0.0, 8.0, 8.0), RED),
    );
    let root = world
        .spawn((
            Visual::default(),
            at(0.0, 0.0, 100.0, 100.0),
            ChildOf(window),
        ))
        .id();
    let n = world
        .spawn((
            Visual {
                opacity: 0.5,
                clip: Some(ClipShape::Rectangle),
                ..Default::default()
            },
            at(10.0, 20.0, 50.0, 40.0),
            n_content,
            ChildOf(root),
        ))
        .id();
    let m = world
        .spawn((
            Visual::default(),
            OwnSurface,
            at(5.0, 6.0, 16.0, 14.0),
            m_content,
            ChildOf(n),
        ))
        .id();
    let p = world
        .spawn((
            Visual::default(),
            OwnSurface,
            at(70.0, 70.0, 8.0, 8.0),
            p_content,
            ChildOf(root),
        ))
        .id();
    world.flush();
    owner_global(world, root, s, 100.0, 100.0);
    owner_global(world, m, s, 16.0, 14.0);
    owner_global(world, p, s, 8.0, 8.0);
    (root, n, m, p)
}

/// 節の下の手動の持ち主は祖先の持ち主の WUC の子になり、位置（DPI の拡大込み）・祖先の不透明度の積・
/// 祖先の矩形の切り抜きを写す。祖先の節が見えない回は不透明度 0、祖先の節が動いた回は同じ回に位置を
/// 合わせ直す。
#[test]
fn embedded_owner_follows_ancestor_node_at_dpi_scales() {
    crate::common::on_gpu_owner_thread(move || {
        for s in [1.0f32, 1.25] {
            let (mut ecs, window) = rig();
            let world = ecs.world_mut();
            let (root, n, m, p) = embedded_tree(world, window, s);
            frame(world);
            frame(world);

            let (rv, mv) = (wuc(world, root), wuc(world, m));
            assert_eq!(wuc_parent(&mv), Some(id(&rv)), "s={s}: M の親が R でない");
            // 前順の持ち主 [M, P] の最初が最上（下から上は P, M）
            assert_eq!(
                wuc_children(&rv),
                ids(world, &[p, m]),
                "s={s}: R の子の並び"
            );
            assert_offset(&mv, 15.0 * s, 26.0 * s, &format!("s={s}: M"));
            assert_matrix(&mv, identity4(), &format!("s={s}: M"));
            assert_eq!(mv.Opacity().unwrap(), 0.5, "s={s}: 祖先の不透明度の積");
            // 祖先の切り抜き (10,20)-(60,60) を M の局所へ（軸に沿うので逆行列は図形に入る）
            let (offset, size, transform) = wuc_clip(&mv);
            assert!(
                near(offset.X, -5.0 * s)
                    && near(offset.Y, -6.0 * s)
                    && near(size.X, 50.0 * s)
                    && near(size.Y, 40.0 * s),
                "s={s}: 切り抜きの図形 {offset:?} {size:?}"
            );
            assert_eq!(transform, Matrix3x2::identity(), "s={s}: 切り抜きの変形");

            world.get_mut::<Visual>(n).unwrap().is_visible = false;
            frame(world);
            assert_eq!(mv.Opacity().unwrap(), 0.0, "s={s}: 祖先が見えない回");
            world.get_mut::<Visual>(n).unwrap().is_visible = true;
            frame(world);
            assert_eq!(mv.Opacity().unwrap(), 0.5, "s={s}: 祖先が見えた回");

            world.get_mut::<Arrangement>(n).unwrap().offset = Offset { x: 12.0, y: 20.0 };
            frame(world);
            assert_offset(&mv, 17.0 * s, 26.0 * s, &format!("s={s}: 祖先が動いた回"));
            world.clear_entities();
        }
    });
}

/// `Children` の並びだけを変えた回・別の持ち主の下の節への付け替え・節へ戻った回・消えた回に、
/// WUC の子の並びと親を同じ回に合わせ直し、古い親の下に残さない。
#[test]
fn embedded_owner_reorders_reattaches_and_detaches() {
    crate::common::on_gpu_owner_thread(move || {
        let (mut ecs, window) = rig();
        let world = ecs.world_mut();
        let (root, _n, m, p) = embedded_tree(world, window, 1.0);
        let c = solid(
            world.resource::<GraphicsCore>(),
            r(0.0, 0.0, 2.0, 2.0),
            BLUE,
        );
        let root2 = world
            .spawn((Visual::default(), at(0.0, 0.0, 50.0, 50.0), ChildOf(window)))
            .id();
        let n2 = world
            .spawn((
                Visual::default(),
                at(4.0, 3.0, 20.0, 20.0),
                c,
                ChildOf(root2),
            ))
            .id();
        world.flush();
        owner_global(world, root2, 1.0, 50.0, 50.0);
        frame(world);
        frame(world);
        let (rv, r2v, mv) = (wuc(world, root), wuc(world, root2), wuc(world, m));
        assert_eq!(wuc_children(&rv), ids(world, &[p, m]));

        world.get_mut::<Children>(root).unwrap().swap(0, 1);
        frame(world);
        assert_eq!(
            wuc_children(&rv),
            ids(world, &[m, p]),
            "並びだけの変化で並べ直さない"
        );

        world.entity_mut(m).insert(ChildOf(n2));
        frame(world);
        assert_eq!(wuc_parent(&mv), Some(id(&r2v)), "付け替えた先の持ち主の子");
        assert_eq!(wuc_children(&rv), ids(world, &[p]), "古い親の下に残った");
        assert_eq!(wuc_children(&r2v), ids(world, &[m]));
        assert_offset(&mv, 9.0, 9.0, "付け替えた先の節の下の位置");
        assert_eq!(mv.Opacity().unwrap(), 1.0, "付け替えた先の祖先は不透明");
        assert!(mv.Clip().is_err(), "付け替えた先の祖先は切り抜かない");

        world.entity_mut(m).remove::<OwnSurface>();
        frame(world);
        frame(world);
        assert!(
            world.get::<VisualGraphics>(m).is_none(),
            "節へ戻った M が見た目を持つ"
        );
        assert!(
            wuc_children(&r2v).is_empty(),
            "節へ戻った M が古い親に残った"
        );

        world.despawn(p);
        frame(world);
        assert!(wuc_children(&rv).is_empty(), "消えた P が古い親に残った");
        world.clear_entities();
    });
}

/// `VisualTransform` を持つ持ち主は局所の行列を WUC の行列へ写し（平行移動だけ拡大）、節の下で回した
/// 持ち主は祖先の矩形の切り抜きを親の持ち主の物理ピクセルの図形と逆行列の切り抜きの変形で写す
/// （WARP の写しの仮定＝切り抜きは Visual の局所・`CompositionClip.TransformMatrix`
/// は図形に掛かる、の値の側）。
#[test]
fn owner_transform_and_rotated_clip_reach_wuc() {
    crate::common::on_gpu_owner_thread(move || {
        let s = 1.25f32;
        let (mut ecs, window) = rig();
        let world = ecs.world_mut();
        let (root, _n, m, p) = embedded_tree(world, window, s);
        let rotate = Transform2D {
            rotation: 0.5,
            anchor: Vector2 { X: 8.0, Y: 7.0 },
            ..Default::default()
        };
        let scale = Transform2D {
            scale: Vector2 { X: 2.0, Y: 1.5 },
            ..Default::default()
        };
        world.entity_mut(m).insert(VisualTransform(rotate));
        world.entity_mut(p).insert(VisualTransform(scale));
        frame(world);
        frame(world);
        let (rv, mv, pv) = (wuc(world, root), wuc(world, m), wuc(world, p));

        // 直下の持ち主: 局所 = VisualTransform × 平行移動(配置)
        let p_local = scale.to_affine() * Matrix3x2::translation(70.0, 70.0);
        assert_matrix(&pv, wuc_matrix(p_local, s), "拡大した P");
        assert_offset(&pv, 0.0, 0.0, "行列を写す持ち主の Offset");

        // 節の下: 局所 × 親の節の世界の行列
        let to_host = rotate.to_affine()
            * Matrix3x2::translation(5.0, 6.0)
            * Matrix3x2::translation(10.0, 20.0);
        assert_eq!(wuc_parent(&mv), Some(id(&rv)));
        assert_matrix(&mv, wuc_matrix(to_host, s), "回した M");
        assert_offset(&mv, 0.0, 0.0, "行列を写す持ち主の Offset");
        assert_eq!(mv.Opacity().unwrap(), 0.5);

        let (offset, size, transform) = wuc_clip(&mv);
        assert!(
            near(offset.X, 10.0 * s)
                && near(offset.Y, 20.0 * s)
                && near(size.X, 50.0 * s)
                && near(size.Y, 40.0 * s),
            "切り抜きの図形は親の持ち主の物理ピクセル: {offset:?} {size:?}"
        );
        let inv = invert(to_host);
        let want = [inv.M11, inv.M12, inv.M21, inv.M22, inv.M31 * s, inv.M32 * s];
        let got = [
            transform.M11,
            transform.M12,
            transform.M21,
            transform.M22,
            transform.M31,
            transform.M32,
        ];
        assert!(
            got.iter().zip(want).all(|(&a, b)| near(a, b)),
            "切り抜きの変形は親の物理 → 局所の物理: {got:?}, expected {want:?}"
        );
        // 局所の物理ピクセルの点を親へ送り、切り抜きの変形で戻すと元の点（図形に掛かる向きの確かめ）
        let local = Vector2 { X: 3.0, Y: 4.0 };
        let host = Vector2 {
            X: local.X * to_host.M11 + local.Y * to_host.M21 + to_host.M31 * s,
            Y: local.X * to_host.M12 + local.Y * to_host.M22 + to_host.M32 * s,
        };
        let back = Vector2 {
            X: host.X * transform.M11 + host.Y * transform.M21 + transform.M31,
            Y: host.X * transform.M12 + host.Y * transform.M22 + transform.M32,
        };
        assert!(near(back.X, local.X) && near(back.Y, local.Y), "{back:?}");

        let version = world.get::<VisualVersion>(m).map(|v| v.0);
        assert!(version.is_some(), "中身を持つ M は版を持つ");
        world.entity_mut(m).remove::<VisualTransform>();
        frame(world);
        // 持ち主の変形は WUC の Visual が受け持つので、見た目の版を上げない
        assert_eq!(world.get::<VisualVersion>(m).map(|v| v.0), version);
        assert_matrix(&mv, identity4(), "変形を外した M");
        assert_offset(&mv, 15.0 * s, 26.0 * s, "変形を外した M");
        world.clear_entities();
    });
}

/// 根の木の列に `e` を描く 1 件が在るか。
fn draws(world: &World, root: Entity, e: Entity) -> bool {
    world.get::<DrawTree>(root).is_some_and(|t| {
        t.commands
            .iter()
            .any(|c| matches!(c, DrawCommand::Draw(d) if d.entity == e))
    })
}

fn image(world: &mut World, e: Entity) -> (u32, u32, Vec<u8>) {
    match world
        .run_system_once(move |rb: SurfaceReadback| rb.read_all(e))
        .unwrap()
    {
        SurfaceImage::Pixels {
            width,
            height,
            bytes,
        } => (width, height, bytes),
        SurfaceImage::Unreadable => panic!("{e:?} の描画面が読めない"),
    }
}

/// `e` の WUC の描画面そのものを `CopySurface` で読む。
fn wuc_image(world: &World, e: Entity) -> (u32, u32, Vec<u8>) {
    let (w, h) = world.get::<SurfaceGraphics>(e).unwrap().size;
    let bytes = crate::composite_systems_test::read_wuc_surface(world, e);
    (w, h, bytes)
}

fn pixel(img: &(u32, u32, Vec<u8>), x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.0 + x) * 4) as usize;
    img.2[i..i + 4].try_into().unwrap()
}

const OPAQUE_RED: [u8; 4] = [0, 0, 255, 255];
const OPAQUE_BLUE: [u8; 4] = [255, 0, 0, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];

/// 自動で上げる場面: 窓 → 持ち主 R (64×64) → [止まった節 B (2,2), 頻繁に更新する節 A (20,10),
/// 手動の持ち主 P (50,50)]。A は前順で P より前（手動の持ち主になれば P の上に並ぶ）。
struct Promoted {
    ecs: EcsWorld,
    root: Entity,
    a: Entity,
    p: Entity,
    /// 次の回の時刻。
    t: TimePoint,
    /// A の中の点・B の中の点（R の描画面の物理ピクセル）。
    a_px: (u32, u32),
    b_px: (u32, u32),
}

/// フレームの時刻を注入して A を毎回更新し、上げた回まで回す。用意中の回は親の描画面で描き、
/// 親の持ち主の WUC の子に居ないことを確かめる。
fn promoted(s: f32) -> Promoted {
    let (mut ecs, window) = rig();
    let world = ecs.world_mut();
    let core = world.resource::<GraphicsCore>();
    let (b_content, a_content, p_content) = (
        solid(core, r(0.0, 0.0, 8.0, 8.0), BLUE),
        solid(core, r(0.0, 0.0, 8.0, 8.0), RED),
        solid(core, r(0.0, 0.0, 4.0, 4.0), BLUE),
    );
    let root = world
        .spawn((Visual::default(), at(0.0, 0.0, 64.0, 64.0), ChildOf(window)))
        .id();
    world.spawn((
        Visual::default(),
        at(2.0, 2.0, 8.0, 8.0),
        b_content,
        ChildOf(root),
    ));
    let a = world
        .spawn((
            Visual::default(),
            at(20.0, 10.0, 8.0, 8.0),
            a_content,
            ChildOf(root),
        ))
        .id();
    let p = world
        .spawn((
            Visual::default(),
            OwnSurface,
            at(50.0, 50.0, 4.0, 4.0),
            p_content,
            ChildOf(root),
        ))
        .id();
    world.flush();
    owner_global(world, root, s, 64.0, 64.0);
    owner_global(world, p, s, 4.0, 4.0);

    let mut t = at_ms(0);
    let mut stages = Vec::new();
    for _ in 0..40 {
        world.insert_resource(FrameTime(t));
        world.get_mut::<VisualDrawContent>(a).unwrap().set_changed();
        frame(world);
        t = t + ms(125);
        let on_parent = draws(world, root, a);
        if world
            .get::<VisualGraphics>(a)
            .is_some_and(|g| g.visual().is_some())
        {
            if !on_parent {
                stages.push("active");
                break;
            }
            let (rv, av) = (wuc(world, root), wuc(world, a));
            assert!(
                !wuc_children(&rv).contains(&id(&av)),
                "s={s} t={t}: 用意中の A が親の子に居る"
            );
            stages.push("preparing");
        } else {
            assert!(on_parent && !draws(world, a, a), "s={s} t={t}");
        }
    }
    assert_eq!(
        stages.last(),
        Some(&"active"),
        "s={s}: 上がらない {stages:?}"
    );
    assert!(stages.contains(&"preparing"), "s={s}: 用意中を経ない");
    Promoted {
        ecs,
        root,
        a,
        p,
        t,
        a_px: ((24.0 * s) as u32, (14.0 * s) as u32),
        b_px: ((6.0 * s) as u32, (6.0 * s) as u32),
    }
}

/// 上げた回の自動の持ち主 A は手動の持ち主 P の下に繋がり、整数の位置に置かれ、自分の描画面に
/// 部分木を描き、親の描画面からは消える。落ち着いて戻した回は親の子から外れ、親の描画面に戻る。
#[test]
fn auto_promoted_node_moves_between_surfaces_and_wuc_children() {
    crate::common::on_gpu_owner_thread(move || {
        for s in [1.0f32, 1.25] {
            let Promoted {
                mut ecs,
                root,
                a,
                p,
                mut t,
                a_px: (ax, ay),
                b_px: (bx, by),
            } = promoted(s);
            let world = ecs.world_mut();
            let (rv, av) = (wuc(world, root), wuc(world, a));
            assert_eq!(wuc_parent(&av), Some(id(&rv)), "s={s}");
            assert_eq!(
                wuc_children(&rv),
                ids(world, &[a, p]),
                "s={s}: 自動の持ち主は手動の持ち主の下"
            );
            let o = av.Offset().unwrap();
            assert!(
                o.X.fract() == 0.0 && o.Y.fract() == 0.0,
                "s={s}: 整数の位置 {o:?}"
            );
            assert_matrix(&av, identity4(), &format!("s={s}: 自動の持ち主"));
            assert_eq!(av.Opacity().unwrap(), 1.0);
            let own = image(world, a);
            assert_eq!(
                av.Size().unwrap(),
                Vector2 {
                    X: own.0 as f32,
                    Y: own.1 as f32
                },
                "s={s}: 描画面の大きさ"
            );
            let (lx, ly) = (ax - o.X as u32, ay - o.Y as u32);
            assert_eq!(pixel(&own, lx, ly), OPAQUE_RED, "s={s}: 自分の描画面の A");
            let parent = image(world, root);
            assert_eq!(
                pixel(&parent, ax, ay),
                CLEAR,
                "s={s}: 親の描画面に A が残った"
            );
            assert_eq!(pixel(&parent, bx, by), OPAQUE_BLUE, "s={s}: 親の描画面の B");
            // 入れ物の写しでなく WUC の描画面そのものでも同じ（CopySurface）
            let own_wuc = wuc_image(world, a);
            assert_eq!(
                pixel(&own_wuc, lx, ly),
                OPAQUE_RED,
                "s={s}: A の WUC 描画面"
            );
            let parent_wuc = wuc_image(world, root);
            assert_eq!(
                pixel(&parent_wuc, ax, ay),
                CLEAR,
                "s={s}: 親の WUC 描画面に A が残った"
            );
            assert_eq!(
                pixel(&parent_wuc, bx, by),
                OPAQUE_BLUE,
                "s={s}: 親の WUC 描画面の B"
            );

            // 落ち着くと戻る: 戻した回に親の子から外れ、親の描画面へ戻る
            let mut demoted = false;
            for _ in 0..40 {
                world.insert_resource(FrameTime(t));
                frame(world);
                t = t + ms(125);
                if draws(world, root, a) {
                    assert_eq!(
                        wuc_children(&rv),
                        ids(world, &[p]),
                        "s={s}: 戻した回に親の子に残った"
                    );
                    demoted = true;
                    break;
                }
            }
            assert!(demoted, "s={s}: 戻らない");
            let parent = image(world, root);
            assert_eq!(
                pixel(&parent, ax, ay),
                OPAQUE_RED,
                "s={s}: 親の描画面に戻らない"
            );
            assert_eq!(
                pixel(&wuc_image(world, root), ax, ay),
                OPAQUE_RED,
                "s={s}: 親の WUC 描画面に戻らない"
            );
            frame(world);
            assert_eq!(wuc_children(&rv), ids(world, &[p]), "s={s}: 次の回");
            world.clear_entities();
        }
    });
}

/// 上げた A に手動の目印を入れた回は、親も `Children` も変わらず相だけが変わる。その回のうちに
/// 自動の持ち主の位置（一番下）から手動の持ち主の前順の位置（P の上）へ並べ直す。
#[test]
fn manual_mark_on_promoted_node_reorders_on_phase_change_only() {
    crate::common::on_gpu_owner_thread(move || {
        let Promoted {
            mut ecs,
            root,
            a,
            p,
            ..
        } = promoted(1.0);
        let world = ecs.world_mut();
        let (rv, av) = (wuc(world, root), wuc(world, a));
        assert_eq!(wuc_children(&rv), ids(world, &[a, p]));

        world.entity_mut(a).insert(OwnSurface);
        frame(world);
        assert_eq!(wuc_parent(&av), Some(id(&rv)), "親は同じまま");
        assert_eq!(
            wuc_children(&rv),
            ids(world, &[p, a]),
            "手動の持ち主になった A が前順の位置に並ばない"
        );
        world.clear_entities();
    });
}
