//! 描画の木を組む係（`update_draw_trees`）の 1 回の判断と部分の列の配線（要件 1.9, 3.1, 3.3, 5.1, 8.6）。
//! 本物の登録を通すため `EcsWorld` の `PreRenderSurface` だけを回し、フレームの時刻（`FrameTime`）を注入する。
//! 配置の伝播は回さないので `GlobalArrangement` は手で置く。画素は読まない。

use super::draw_content_test_support::solid_rect_content;
use super::promotion::Phase;
use super::*;
use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
use crate::ecs::graphics::{FrameTime, GraphicsCore};
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
/// 落ち着くと親の列へ戻る（要件 1.9, 3.1, 3.3）。
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
/// 内の根の回で外されず、回をまたいで付いたり外れたりしない（要件 5.1）。
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
