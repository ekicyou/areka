//! 1 回の判断の GPU を使わない ECS のテスト（要件 1.9〜1.11, 2.1, 2.5, 2.6, 3.1, 4.5, 8.7）。
//! 走査の列を手で組み（`promotion_shape_tests` の部品）、項ごとに実体を生んで部品の様子から判断させる。
//! 回すたびに走査の列の相を部品から読み直す（`fill_world_visuals` と同じ）。描画面ができた様子だけは
//! WUC の描画面が要るので、WARP（CPU の描き手）の D2D の装置の上に作る。

use std::collections::HashSet;

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use windows::Foundation::Size;
use windows::Graphics::DirectX::{DirectXAlphaMode, DirectXPixelFormat};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows_numerics::Matrix3x2;

use super::emit_test_support::warp_device;
use super::placement::{PlacementClip, PlacementKind, SurfacePlacement};
use super::promotion::*;
use super::promotion_round::*;
use super::promotion_shape::Ineligible;
use super::promotion_shape_tests::{node, owner, phased, scan};
use super::world_visual::{ClipClass, ReportedIssues, ScanEntry, ScanKind, TreeScan};
use super::{OwnSurface, VisualTransform};
use crate::ecs::graphics::GraphicsCommandList;
use crate::ecs::graphics::components::SurfaceGraphics;
use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
use crate::numerics::Aabb;

const OK: fn() -> Result<(), Ineligible> = || Ok(());

/// 相 `phase` の規則の状態。用意中は 0.875 秒に上げた（0〜0.875 秒に 8 Hz で 8 回の更新）、
/// 上げたは 1.0 秒に切り替えた（落ち着きの戻しは 4.0 秒から・最後の更新は 0.875 秒）。
fn rule(phase: Phase) -> RuleState {
    let mut s = RuleState::default();
    if phase == Phase::Node {
        return s;
    }
    for k in 0..PROMOTE_COUNT {
        s.step(k as f64 * 0.125, true, OK, SurfaceReadiness::Pending);
    }
    if phase == Phase::Active {
        s.step(1.0, false, OK, SurfaceReadiness::Ready);
    }
    assert_eq!(s.phase(), phase);
    s
}

/// 手で組んだ項ごとに実体を生む（節は項の相の状態の部品を持ち、手動の持ち主は持たない）。
pub(super) fn world_scan(world: &mut World, entries: Vec<ScanEntry>) -> TreeScan {
    let mut s = scan(entries);
    for e in &mut s.entries {
        e.entity = match e.kind {
            ScanKind::Node { phase } => world
                .spawn(AutoPromotion {
                    rule: rule(phase),
                    ..Default::default()
                })
                .id(),
            ScanKind::Owner { .. } => world.spawn_empty().id(),
        };
    }
    s
}

pub(super) fn ctx(now: f64) -> RoundContext {
    RoundContext {
        now: Some(now),
        device_ready: true,
        scale: 1.0,
        host_px: (100, 100),
    }
}

/// 走査の列の相を部品から読み直し、1 回の判断を回して命令を反映する。
pub(super) fn round(
    world: &mut World,
    s: &mut TreeScan,
    c: &RoundContext,
) -> (RoundPlan, HashSet<Entity>) {
    round_with(world, s, c, &mut ReportedIssues::default())
}

/// [`round`] の、記録の覚えを回をまたいで持つ形（回の終わりに `end_round` を呼ぶ）。
pub(super) fn round_with(
    world: &mut World,
    s: &mut TreeScan,
    c: &RoundContext,
    reported: &mut ReportedIssues,
) -> (RoundPlan, HashSet<Entity>) {
    for e in &mut s.entries {
        if let ScanKind::Node { phase } = &mut e.kind {
            *phase = world.get::<AutoPromotion>(e.entity).unwrap().rule.phase();
        }
    }
    let mut state: SystemState<(Commands, Query<PromotionData>)> = SystemState::new(world);
    let (mut commands, mut states) = state.get_mut(world).unwrap();
    let mut written = HashSet::new();
    let plan = run_round(c, s, &mut states, &mut commands, &mut written, reported);
    reported.end_round();
    state.apply(world);
    (plan, written)
}

/// 項 `idx` だけを「この回の更新」ありにする。
pub(super) fn only_updated(s: &mut TreeScan, idx: &[usize]) {
    for (i, e) in s.entries.iter_mut().enumerate() {
        e.updated = idx.contains(&i);
    }
}

/// 項 `idx` を更新した回を、`start` 秒から 8 Hz で `n` 回まわし、最後の回の分け方を返す。
pub(super) fn feed(
    world: &mut World,
    s: &mut TreeScan,
    idx: &[usize],
    start: f64,
    n: usize,
) -> RoundPlan {
    only_updated(s, idx);
    let mut last = None;
    for k in 0..n {
        last = Some(round(world, s, &ctx(start + k as f64 * 0.125)).0);
    }
    last.unwrap()
}

pub(super) fn ent(s: &TreeScan, i: usize) -> Entity {
    s.entries[i].entity
}

fn ents(s: &TreeScan, idx: &[usize]) -> Vec<(Entity, usize)> {
    idx.iter()
        .map(|&i| (s.entries[i].entity, s.entries[i].depth))
        .collect()
}

pub(super) fn ph(world: &World, e: Entity) -> Phase {
    world.get::<AutoPromotion>(e).unwrap().rule.phase()
}

fn untouched(world: &World, e: Entity) -> bool {
    world.get::<AutoPromotion>(e).unwrap().rule == RuleState::default()
}

fn placement(world: &World, e: Entity) -> Option<PlacementKind> {
    world.get::<SurfacePlacement>(e).map(|p| p.0.clone())
}

/// 部分の列ごとの（持ち主, 見える節）。
fn segs(plan: &RoundPlan) -> Vec<(Entity, Vec<(Entity, usize)>)> {
    plan.segments
        .iter()
        .map(|g| (g.owner, g.visited.clone()))
        .collect()
}

/// WARP の D2D の装置の上の、できた（有効な）描画面。資源は描画面より長く持つ。
pub(super) fn ready_surface() -> (WucGraphicsResource, SurfaceGraphics) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let resource = WucGraphicsResource::new(warp_device()).expect("WUC の資源");
    let surface = resource
        .graphics_device()
        .expect("graphics_device")
        .CreateDrawingSurface(
            Size {
                Width: 4.0,
                Height: 4.0,
            },
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            DirectXAlphaMode::Premultiplied,
        )
        .expect("CreateDrawingSurface");
    let brush = resource
        .compositor()
        .expect("compositor")
        .CreateSurfaceBrushWithSurface(&surface)
        .expect("CreateSurfaceBrushWithSurface");
    (resource, SurfaceGraphics::new(surface, brush, (4, 4)))
}

// ---- 帰属と判断の対象 ----

#[test]
fn updates_under_an_auto_owner_count_for_the_owner() {
    // A は 1.0 秒に上げた。子 B の 3.9 秒の更新が A の更新になれば、4.5 秒でも落ち着きで戻らない。
    for (b_updates, expect) in [(true, Phase::Active), (false, Phase::Node)] {
        let mut world = World::new();
        let mut s = world_scan(
            &mut world,
            vec![
                phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Active),
                node(2, 12.0, 12.0, 16.0, 16.0),
            ],
        );
        only_updated(&mut s, if b_updates { &[1] } else { &[] });
        round(&mut world, &mut s, &ctx(3.9));
        only_updated(&mut s, &[]);
        let (plan, _) = round(&mut world, &mut s, &ctx(4.5));
        assert_eq!(ph(&world, ent(&s, 0)), expect);
        assert!(untouched(&world, ent(&s, 1)), "帰属した子の規則は呼ばない");
        if expect == Phase::Node {
            // 戻った回は親の列に在り、部分の列に無い。
            assert_eq!(plan.host_visited, ents(&s, &[0, 1]));
            assert!(plan.segments.is_empty());
        }
    }
}

#[test]
fn only_updated_nodes_and_auto_owners_are_decided() {
    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![
            node(1, 0.0, 0.0, 10.0, 10.0),
            node(1, 20.0, 20.0, 30.0, 30.0),
        ],
    );
    only_updated(&mut s, &[1]);
    round(&mut world, &mut s, &ctx(0.0));
    assert!(
        untouched(&world, ent(&s, 0)),
        "更新の無い節の規則は呼ばない"
    );
    assert!(!untouched(&world, ent(&s, 1)));
}

#[test]
fn no_decision_without_frame_time() {
    // 時刻が無い回は、頻繁に更新しても上げず、上げた持ち主も落ち着きで戻さない。分け方と置き場は出す。
    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![
            phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Active),
            node(1, 40.0, 40.0, 50.0, 50.0),
        ],
    );
    only_updated(&mut s, &[1]);
    let none = RoundContext {
        now: None,
        ..ctx(0.0)
    };
    for _ in 0..PROMOTE_COUNT {
        round(&mut world, &mut s, &none);
    }
    let (plan, written) = round(&mut world, &mut s, &none);
    assert_eq!(ph(&world, ent(&s, 0)), Phase::Active);
    assert!(untouched(&world, ent(&s, 1)));
    assert_eq!(plan.host_visited, ents(&s, &[1]));
    assert_eq!(segs(&plan), vec![(ent(&s, 0), ents(&s, &[0]))]);
    assert_eq!(written, HashSet::from([ent(&s, 0)]));
}

// ---- 祖先の優先と吸い込み ----

#[test]
fn ancestor_wins_when_both_would_promote_on_the_same_frame() {
    // 子 B だけなら上がる（上げると答える形）ことを先に確かめる。
    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![
            node(1, 10.0, 10.0, 30.0, 30.0),
            node(2, 12.0, 12.0, 16.0, 16.0),
        ],
    );
    feed(&mut world, &mut s, &[1], 0.0, PROMOTE_COUNT);
    assert_eq!(ph(&world, ent(&s, 1)), Phase::Preparing);

    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![
            node(1, 10.0, 10.0, 30.0, 30.0),
            node(2, 12.0, 12.0, 16.0, 16.0),
        ],
    );
    let plan = feed(&mut world, &mut s, &[0, 1], 0.0, PROMOTE_COUNT);
    assert_eq!(ph(&world, ent(&s, 0)), Phase::Preparing);
    assert_eq!(
        ph(&world, ent(&s, 1)),
        Phase::Node,
        "祖先も上がる回は上げない"
    );
    // 判断の回は 3 部品がまだ無いので部分の列を描かず、親の列にだけ残す。
    assert!(plan.segments.is_empty());
    assert_eq!(plan.host_visited, ents(&s, &[0, 1]));
}

#[test]
fn auto_owners_inside_a_new_promotion_are_absorbed() {
    // B は 1.0 秒に上げた（4.0 秒まで落ち着きでは戻らない）。祖先 A が 1.875 秒に上がると、同じ回に戻る。
    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![
            node(1, 10.0, 10.0, 30.0, 30.0),
            phased(node(2, 12.0, 12.0, 16.0, 16.0), Phase::Active),
        ],
    );
    let b = ent(&s, 1);
    let plan = feed(&mut world, &mut s, &[0], 1.0, PROMOTE_COUNT - 1);
    assert_eq!(ph(&world, b), Phase::Active);
    assert_eq!(plan.host_visited, ents(&s, &[0]));
    world.clear_trackers();
    let plan = feed(&mut world, &mut s, &[0], 1.875, 1);
    assert_eq!(ph(&world, ent(&s, 0)), Phase::Preparing);
    assert_eq!(ph(&world, b), Phase::Node);
    assert!(
        world
            .entity(b)
            .get_ref::<AutoPromotion>()
            .unwrap()
            .is_changed(),
        "相が変わった回は変化の印を立てる"
    );
    // 判断の回は 3 部品がまだ無いので部分の列を描かず、親の列にだけ残す。
    assert!(plan.segments.is_empty());
    assert_eq!(plan.host_visited, ents(&s, &[0, 1]));
}

// ---- 用意中の描画面の様子 ----

#[test]
fn preparing_waits_for_the_drawn_surface_then_activates() {
    // 係の順（3 部品を入れる → 1 回の判断 → 部分の列を書いて built → 描画面を作る）を回ごとに写す。
    for created in [true, false] {
        let mut world = World::new();
        let mut s = world_scan(
            &mut world,
            vec![
                phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Preparing),
                node(2, 12.0, 12.0, 14.0, 14.0),
            ],
        );
        let a = ent(&s, 0);
        world.entity_mut(a).insert(VisualTransform::default());
        // 回 N: 3 部品がまだ無い。待つ。部分は親の列にだけ在り、部分の列は描かない。
        world.clear_trackers();
        let (plan, written) = round(&mut world, &mut s, &ctx(1.0));
        assert_eq!(ph(&world, a), Phase::Preparing);
        assert!(
            !world
                .entity(a)
                .get_ref::<AutoPromotion>()
                .unwrap()
                .is_changed(),
            "履歴だけの回は変化の印を立てない"
        );
        assert_eq!(plan.host_visited, ents(&s, &[0, 1]));
        assert!(
            plan.segments.is_empty(),
            "3 部品の無い用意中は部分の列を描かない"
        );
        assert_eq!(written, HashSet::from([a]));
        // 回 N+1: 3 部品が入った（描画面はまだ作られていない＝無効）。部分の列をまだ描いていないので待つ。
        world.entity_mut(a).insert(SurfaceGraphics::default());
        let (plan, _) = round(&mut world, &mut s, &ctx(1.1));
        assert_eq!(
            ph(&world, a),
            Phase::Preparing,
            "描く前の無効な描画面は失敗でない"
        );
        assert_eq!(plan.host_visited, ents(&s, &[0, 1]));
        assert_eq!(segs(&plan), vec![(a, ents(&s, &[0, 1]))]);
        // 同じ回の後: 部分の列を書いて built を立て、作成の係が描画面を作る（作れなかった）。
        world.get_mut::<AutoPromotion>(a).unwrap().built = true;
        let _resource = if created {
            let (resource, surface) = ready_surface();
            world.entity_mut(a).insert(surface);
            Some(resource)
        } else {
            None
        };
        // 回 N+2: 描いた次の回に描画面が有効なら上げた（親の列から部分が外れる）。無効なら節へ戻す。
        let (plan, _) = round(&mut world, &mut s, &ctx(1.2));
        assert!(!world.get::<AutoPromotion>(a).unwrap().built);
        if created {
            assert_eq!(ph(&world, a), Phase::Active);
            assert!(plan.host_visited.is_empty());
            assert_eq!(segs(&plan), vec![(a, ents(&s, &[0, 1]))]);
        } else {
            assert_eq!(ph(&world, a), Phase::Node);
            assert_eq!(plan.host_visited, ents(&s, &[0, 1]));
            assert!(plan.segments.is_empty());
        }
        // 使い手の部品には触らない。
        assert!(world.get::<VisualTransform>(a).is_some());
        assert!(world.get::<OwnSurface>(a).is_none());
        assert!(world.get::<GraphicsCommandList>(a).is_none());
    }
}

#[test]
fn preparing_demotes_when_the_surface_failed_or_the_device_is_gone() {
    for device_ready in [true, false] {
        let mut world = World::new();
        let mut s = world_scan(
            &mut world,
            vec![phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Preparing)],
        );
        let a = ent(&s, 0);
        world.entity_mut(a).insert(SurfaceGraphics::default());
        world.get_mut::<AutoPromotion>(a).unwrap().built = true;
        let c = RoundContext {
            device_ready,
            ..ctx(1.0)
        };
        let (plan, written) = round(&mut world, &mut s, &c);
        assert_eq!(ph(&world, a), Phase::Node);
        assert_eq!(plan.host_visited, ents(&s, &[0]));
        assert!(plan.segments.is_empty());
        assert!(written.is_empty());
    }
}

#[test]
fn promotion_is_skipped_without_device_then_taken_with_it() {
    let mut world = World::new();
    let mut s = world_scan(&mut world, vec![node(1, 10.0, 10.0, 20.0, 20.0)]);
    only_updated(&mut s, &[0]);
    let no_device = |now| RoundContext {
        device_ready: false,
        ..ctx(now)
    };
    for k in 0..PROMOTE_COUNT {
        round(&mut world, &mut s, &no_device(k as f64 * 0.125));
    }
    assert_eq!(ph(&world, ent(&s, 0)), Phase::Node);
    round(&mut world, &mut s, &ctx(1.0));
    assert_eq!(ph(&world, ent(&s, 0)), Phase::Preparing);
}

#[test]
fn hidden_auto_owner_demotes_and_empty_subtree_is_not_promoted() {
    let mut world = World::new();
    let mut hidden = phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Active);
    hidden.visible = false;
    let mut s = world_scan(&mut world, vec![hidden]);
    let (plan, _) = round(&mut world, &mut s, &ctx(1.5));
    assert_eq!(
        ph(&world, ent(&s, 0)),
        Phase::Node,
        "見えない持ち主はその回に戻る"
    );
    assert!(plan.segments.is_empty());

    // 描く中身の無い部分木は頻繁でも上げない（0×0 の描画面を作らない）。
    let mut world = World::new();
    let mut s = world_scan(&mut world, vec![node(1, 0.0, 0.0, 0.0, 0.0)]);
    feed(&mut world, &mut s, &[0], 0.0, PROMOTE_COUNT);
    assert_eq!(ph(&world, ent(&s, 0)), Phase::Node);
}

// ---- 置き場 ----

#[test]
fn placements_are_written_on_change_and_recorded() {
    let mut n = node(1, 0.0, 0.0, 50.0, 50.0);
    n.own_clip = ClipClass::Rect;
    let mut o = owner(2, 5.0, 5.0, 10.0, 10.0);
    o.kind = ScanKind::Owner {
        parent_world: Matrix3x2::translation(3.0, 4.0),
    };
    o.ancestors.opacity = 0.5;
    o.ancestors.rect_clip = Some(Aabb::new(0.0, 0.0, 50.0, 50.0));
    let mut world = World::new();
    let mut s = world_scan(&mut world, vec![n, o, owner(1, 60.0, 60.0, 70.0, 70.0)]);
    let (o, top) = (ent(&s, 1), ent(&s, 2));
    let embedded = |opacity, clip| PlacementKind::Embedded {
        parent_world: Matrix3x2::translation(3.0, 4.0),
        opacity,
        visible: true,
        clip,
    };
    let rect = PlacementClip::Rect(Aabb::new(0.0, 0.0, 50.0, 50.0));
    let (_, written) = round(&mut world, &mut s, &ctx(1.0));
    assert_eq!(placement(&world, o), Some(embedded(0.5, rect)));
    assert_eq!(placement(&world, top), None, "根の直下の持ち主には書かない");
    assert_eq!(written, HashSet::from([o]));

    world.clear_trackers();
    let (_, written) = round(&mut world, &mut s, &ctx(1.1));
    assert!(
        !world
            .entity(o)
            .get_ref::<SurfacePlacement>()
            .unwrap()
            .is_changed(),
        "同じ値なら書かない"
    );
    assert_eq!(written, HashSet::from([o]), "書かなくても控えには記す");

    // 祖先の変化はその回に書き直す。
    s.entries[1].ancestors.opacity = 0.25;
    round(&mut world, &mut s, &ctx(1.2));
    assert!(
        world
            .entity(o)
            .get_ref::<SurfacePlacement>()
            .unwrap()
            .is_changed()
    );
    assert_eq!(placement(&world, o), Some(embedded(0.25, rect)));

    // 祖先の角丸の切り抜きは 1 つの矩形に合わせられない。見えない祖先は可視を偽にする。
    s.entries[0].own_clip = ClipClass::Rounded;
    s.entries[1].visible = false;
    round(&mut world, &mut s, &ctx(1.3));
    assert_eq!(
        placement(&world, o),
        Some(PlacementKind::Embedded {
            parent_world: Matrix3x2::translation(3.0, 4.0),
            opacity: 0.25,
            visible: false,
            clip: PlacementClip::Unsupported,
        })
    );
}

#[test]
fn auto_owner_placement_follows_the_extent_and_regrows_on_overflow() {
    // 中身 [10, 20) は にじみの余白 2 で物理 [8, 22)。
    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Active)],
    );
    let a = ent(&s, 0);
    let auto = |size_px| PlacementKind::Auto {
        origin_px: (8, 8),
        size_px,
    };
    let (_, written) = round(&mut world, &mut s, &ctx(1.5));
    assert_eq!(placement(&world, a), Some(auto((14, 14))));
    assert_eq!(written, HashSet::from([a]));
    // はみ出したら決め直す。
    s.entries[0].world.content_world_aabb = Aabb::new(10.0, 10.0, 30.0, 20.0);
    round(&mut world, &mut s, &ctx(1.6));
    assert_eq!(placement(&world, a), Some(auto((24, 14))));
    // 縮むほうは追わない。
    s.entries[0].world.content_world_aabb = Aabb::new(10.0, 10.0, 12.0, 12.0);
    let (plan, _) = round(&mut world, &mut s, &ctx(1.7));
    assert_eq!(placement(&world, a), Some(auto((24, 14))));
    assert_eq!(plan.segments[0].origin.px, (8, 8));
    assert_eq!(plan.segments[0].origin.size_px, (24, 14));
}

// ---- 描く列の分け方 ----

#[test]
fn split_keeps_active_out_of_the_host_list_and_preparing_in_both() {
    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![
            phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Active),
            node(2, 12.0, 12.0, 14.0, 14.0),
            phased(node(1, 40.0, 40.0, 50.0, 50.0), Phase::Preparing),
            node(1, 70.0, 70.0, 80.0, 80.0),
        ],
    );
    // 用意中の部分の列は 3 部品が入ってから描く。
    world
        .entity_mut(ent(&s, 2))
        .insert(SurfaceGraphics::default());
    let (plan, written) = round(&mut world, &mut s, &ctx(1.5));
    assert_eq!(plan.host_visited, ents(&s, &[2, 3]));
    assert_eq!(
        segs(&plan),
        vec![
            (ent(&s, 0), ents(&s, &[0, 1])),
            (ent(&s, 2), ents(&s, &[2])),
        ]
    );
    assert_eq!(written, HashSet::from([ent(&s, 0), ent(&s, 2)]));
}

// ---- 切り抜きの合わせの見分けが係の答えと合う ----

/// `clip_combo`（warn! の見分け）は `merged_clip` が `Unsupported` を返す条件の写し。
/// 置き場の矩形の切り抜きがある場合（自分の切り抜き・変形・包むか）で両者の答えが合う。
#[test]
fn clip_combo_matches_merged_clip_unsupported() {
    use super::placement::{ClipPlan, merged_clip};
    use crate::ecs::graphics::ClipShape;
    const SIZE: (f32, f32) = (100.0, 50.0);
    let mats = [
        Matrix3x2::translation(10.0, 10.0),
        Matrix3x2::rotation(30.0) * Matrix3x2::translation(10.0, 20.0),
        Matrix3x2::rotation(90.0) * Matrix3x2::translation(60.0, 0.0),
    ];
    let hosts = [
        Aabb::new(0.0, 0.0, 300.0, 300.0), // 包む
        Aabb::new(0.0, 0.0, 40.0, 30.0),   // 切る
    ];
    let owns = [
        (None, ClipClass::None),
        (Some(ClipShape::Rectangle), ClipClass::Rect),
        (Some(ClipShape::rounded_rectangle(4.0)), ClipClass::Rounded),
    ];
    for m in mats {
        for host in hosts {
            for (own, class) in &owns {
                let mut e = owner(0, 0.0, 0.0, SIZE.0, SIZE.1);
                e.world = super::world_visual::WorldVisual::from_world_mat(m);
                e.world.content_world_aabb = Aabb::new(0.0, 0.0, SIZE.0, SIZE.1).transform(m);
                e.own_clip = *class;
                let clip = PlacementClip::Rect(host);
                assert_eq!(
                    clip_combo(&e, &clip),
                    merged_clip(own.as_ref(), &clip, m, SIZE) == ClipPlan::Unsupported,
                    "{m:?} {host:?} {class:?}"
                );
            }
        }
    }
}
