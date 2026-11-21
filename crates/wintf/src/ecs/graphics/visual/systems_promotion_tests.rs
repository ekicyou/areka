//! 描画の木を組む係（`update_draw_trees`）の 1 回の判断と部分の列の配線。
//! 本物の登録を通すため `EcsWorld` の `PreRenderSurface` だけを回し、フレームの時刻（`FrameTime`）を注入する。
//! 配置の伝播は回さないので `GlobalArrangement` は手で置く。画素は読まない。

use super::draw_content_test_support::solid_rect_content;
use super::promotion::Phase;
use super::*;
use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
use crate::ecs::graphics::{FrameTime, GraphicsCore, SurfaceCreationStats, SurfaceGraphics};
use crate::ecs::layout::{Arrangement, GlobalArrangement};
use crate::ecs::world::{EcsWorld, PreRenderSurface};
use crate::ecs::{Offset, Size, Window};
use bevy_ecs::hierarchy::ChildOf;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows_numerics::Matrix3x2;

const DT: f64 = 0.125;

fn rect(l: f32, t: f32, r: f32, b: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left: l,
        top: t,
        right: r,
        bottom: b,
    }
}

/// 装置と WUC の資源を入れた世界。
fn device_world() -> EcsWorld {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let mut ecs = EcsWorld::new();
    let world = ecs.world_mut();
    let core = GraphicsCore::new().expect("GraphicsCore");
    let wuc = WucGraphicsResource::new(core.d2d_device().expect("d2d_device")).expect("WUC");
    world.insert_resource(core);
    world.insert_resource(wuc);
    ecs
}

/// `(x, y)` に置いた大きさ `w`×`h` の配置と、拡大 1 の物理の外接矩形。
fn placed(x: f32, y: f32, w: f32, h: f32) -> (Arrangement, GlobalArrangement) {
    (
        Arrangement {
            offset: Offset { x, y },
            size: Size {
                width: w,
                height: h,
            },
            ..Default::default()
        },
        GlobalArrangement {
            transform: Matrix3x2::translation(x, y),
            bounds: rect(x, y, x + w, y + h).into(),
        },
    )
}

/// 窓 → 持ち主（100×100）→ [止まった節 B (50,50)・頻繁に更新する節 A (10,10)]。
fn owner_with_nodes(world: &mut World) -> (Entity, Entity) {
    let window = world.spawn(Window::default()).id();
    let owner = world
        .spawn((
            Visual::default(),
            placed(0.0, 0.0, 100.0, 100.0),
            ChildOf(window),
        ))
        .id();
    world.spawn((
        Visual::default(),
        solid_rect_content(rect(0.0, 0.0, 4.0, 4.0)),
        placed(50.0, 50.0, 4.0, 4.0),
        ChildOf(owner),
    ));
    let a = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect(0.0, 0.0, 4.0, 4.0)),
            placed(10.0, 10.0, 4.0, 4.0),
            ChildOf(owner),
        ))
        .id();
    (owner, a)
}

/// 時刻 `t` の 1 回。`bump` なら `a` の描く中身を更新する。
fn frame(world: &mut World, t: f64, bump: Option<Entity>) {
    world.insert_resource(FrameTime(t));
    if let Some(a) = bump {
        world.get_mut::<VisualDrawContent>(a).unwrap().set_changed();
    }
    world.run_schedule(PreRenderSurface);
}

fn phase(world: &World, e: Entity) -> Phase {
    world.get::<AutoPromotion>(e).unwrap().rule.phase()
}

/// 根の列に `e` を描く 1 件が在るか。
fn draws(world: &World, root: Entity, e: Entity) -> bool {
    world.get::<DrawTree>(root).is_some_and(|t| {
        t.commands
            .iter()
            .any(|c| matches!(c, DrawCommand::Draw(d) if d.entity == e))
    })
}

/// 頻繁に更新した節が用意中を経て上げたへ進み、親の列から外れて自分の列で描かれ、
/// 落ち着くと親の列へ戻る。
#[test]
fn frequent_node_promotes_leaves_parent_and_returns_when_quiet() {
    let mut ecs = device_world();
    let world = ecs.world_mut();
    let (owner, a) = owner_with_nodes(world);
    let mut t = 0.0;
    let mut seen = Vec::new();
    for _ in 0..12 {
        frame(world, t, Some(a));
        let p = phase(world, a);
        if seen.last() != Some(&p) {
            seen.push(p);
        }
        // どの回も部分木は親の列か自分の列のどちらか 1 か所（用意中は親の列で画面に出る）。
        let on_parent = draws(world, owner, a);
        assert_eq!(on_parent, p != Phase::Active, "t={t} {p:?}");
        if p == Phase::Active {
            assert!(draws(world, a, a), "上げた節は自分の列で描く");
            assert!(
                world.get::<DrawTree>(a).unwrap().device_px,
                "部分の列は恒等の根の変換で再生する"
            );
            assert!(
                !world.get::<DrawTree>(owner).unwrap().device_px,
                "根の木は拡大で再生する"
            );
            assert!(world.get::<SurfacePlacement>(a).is_some(), "置き場が在る");
        }
        t += DT;
    }
    assert_eq!(seen, vec![Phase::Node, Phase::Preparing, Phase::Active]);

    // 更新を止めると、履歴と落ち着きの後に戻り、同じ回に親の列へ戻る。
    let mut back = None;
    for _ in 0..60 {
        frame(world, t, None);
        if phase(world, a) == Phase::Node {
            back = Some(t);
            break;
        }
        assert!(!draws(world, owner, a));
        t += DT;
    }
    assert!(back.is_some(), "落ち着けば戻る");
    assert!(draws(world, owner, a), "戻した回に親の列へ戻る");
    assert!(world.get::<SurfacePlacement>(a).is_none(), "置き場を外す");
    frame(world, t + DT, None);
    assert!(world.get::<DrawTree>(a).is_none(), "次の回に自分の木を外す");
    assert!(draws(world, owner, a));
}

/// 上げた節がどの走査の列にも現れなくなった回（持ち主の下から外れた）に見えないとして戻す。
#[test]
fn auto_owner_missing_from_every_scan_demotes() {
    let mut ecs = device_world();
    let world = ecs.world_mut();
    let (owner, a) = owner_with_nodes(world);
    let mut t = 0.0;
    while phase(world, a) != Phase::Active {
        assert!(t < 3.0, "上がらない");
        frame(world, t, Some(a));
        t += DT;
    }
    let window = world.get::<ChildOf>(owner).unwrap().parent();
    world.entity_mut(a).insert(ChildOf(window));
    frame(world, t, None);
    assert_eq!(phase(world, a), Phase::Node, "見えないとして戻す");
    assert!(world.get::<SurfacePlacement>(a).is_none());
}

/// 根が 2 つの木（持ち主 → 節 → 持ち主 → 節）で、内の持ち主の置き場は外の根の回で書かれ、
/// 内の根の回で外されず、回をまたいで付いたり外れたりしない。
#[test]
fn nested_owner_placement_does_not_flap_with_two_roots() {
    let mut ecs = device_world();
    let world = ecs.world_mut();
    let window = world.spawn(Window::default()).id();
    let outer = world
        .spawn((
            Visual::default(),
            placed(0.0, 0.0, 100.0, 100.0),
            ChildOf(window),
        ))
        .id();
    let node = world
        .spawn((
            Visual::default(),
            solid_rect_content(rect(0.0, 0.0, 40.0, 40.0)),
            placed(5.0, 5.0, 40.0, 40.0),
            ChildOf(outer),
        ))
        .id();
    let inner = world
        .spawn((
            Visual::default(),
            placed(3.0, 3.0, 20.0, 20.0),
            ChildOf(node),
        ))
        .id();
    world.spawn((
        Visual::default(),
        solid_rect_content(rect(0.0, 0.0, 4.0, 4.0)),
        placed(1.0, 1.0, 4.0, 4.0),
        ChildOf(inner),
    ));
    frame(world, 0.0, None);
    let tick = |w: &World| {
        w.entity(inner)
            .get_ref::<SurfacePlacement>()
            .expect("節の下の持ち主の置き場")
            .last_changed()
            .get()
    };
    frame(world, DT, None);
    let first = tick(world);
    for k in 2..6 {
        frame(world, k as f64 * DT, None);
        assert_eq!(tick(world), first, "置き場を毎回外して付け直さない");
    }
    assert!(world.get::<DrawTree>(inner).is_some(), "内の根も組む");
}

/// 置き場の大きさ（自動の持ち主のとき）。
fn auto_size(world: &World, e: Entity) -> Option<(u32, u32)> {
    world.get::<SurfacePlacement>(e)?.auto_size_px()
}

/// 自動の持ち主の描画面は配置の外接矩形でなく置き場の大きさで作り、範囲がはみ出した回は
/// 作り直す。
#[test]
fn auto_owner_surface_takes_placement_size_and_regrows() {
    let mut ecs = device_world();
    let world = ecs.world_mut();
    let (_, a) = owner_with_nodes(world);
    let mut t = 0.0;
    let mut saw_preparing = false;
    while phase(world, a) != Phase::Active {
        assert!(t < 3.0, "上がらない");
        frame(world, t, Some(a));
        // 用意中の最初の回は 3 部品がまだ無い（次の回に付く）。
        if phase(world, a) == Phase::Preparing
            && let Some(size) = auto_size(world, a)
            && let Some(s) = world.get::<SurfaceGraphics>(a).filter(|s| s.is_valid())
        {
            saw_preparing = true;
            assert_eq!(s.size, size, "用意中も置き場の大きさ");
        }
        t += DT;
    }
    assert!(saw_preparing);
    let before = auto_size(world, a).expect("置き場");
    assert_ne!(
        before,
        (4, 4),
        "配置の外接矩形（4×4）とは違う大きさ（余白を含む）"
    );
    assert_eq!(world.get::<SurfaceGraphics>(a).unwrap().size, before);

    // 描く範囲が今の描画面からはみ出すと、置き場の大きさを決め直して作り直す。
    let resized = world.resource::<SurfaceCreationStats>().resize_count;
    world
        .entity_mut(a)
        .insert(solid_rect_content(rect(0.0, 0.0, 30.0, 30.0)));
    frame(world, t, None);
    let after = auto_size(world, a).expect("上げたまま");
    assert!(
        after.0 > before.0 && after.1 > before.1,
        "{before:?} → {after:?}"
    );
    assert_eq!(world.get::<SurfaceGraphics>(a).unwrap().size, after);
    assert_eq!(
        world.resource::<SurfaceCreationStats>().resize_count,
        resized + 1,
        "作り直しの道に乗る"
    );
}

/// 戻した回は置き場が先に外れ 3 部品は次の回に外れる。その回に配置が変わっても、もう描画面を
/// 欲しがらない節の描画面を外接矩形の大きさで作り直さない。
#[test]
fn demotion_frame_does_not_recreate_the_discarded_surface() {
    let mut ecs = device_world();
    let world = ecs.world_mut();
    let (_, a) = owner_with_nodes(world);
    let mut t = 0.0;
    while phase(world, a) != Phase::Active {
        assert!(t < 3.0, "上がらない");
        frame(world, t, Some(a));
        t += DT;
    }
    let stats = |w: &World| {
        let s = w.resource::<SurfaceCreationStats>();
        (s.created_count, s.resize_count)
    };
    let before = stats(world);
    for _ in 0..80 {
        // 配置の変化の印を毎回立てる（戻した回にも作る係の対象に入る）
        world.get_mut::<GlobalArrangement>(a).unwrap().set_changed();
        frame(world, t, None);
        t += DT;
        if phase(world, a) == Phase::Node {
            break;
        }
    }
    assert_eq!(phase(world, a), Phase::Node, "落ち着けば戻る");
    assert_eq!(stats(world), before, "捨てる描画面を作り直さない");
}
