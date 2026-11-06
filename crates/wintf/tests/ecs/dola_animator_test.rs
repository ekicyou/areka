//! DolaAnimator ECS Component テスト (Task 9.2)
//!
//! - spawn → tick → last_result 読み取りの一連フロー
//! - tick_dola_animators System による全エンティティ一括 tick
//! - runtime() アクセスパターン

use bevy_ecs::prelude::*;
use dola::runtime::DolaRuntime;
use dola::{
    AnimationVariableDef, DolaDocumentBuilder, StoryboardBuilder, StoryboardEntry, TransitionDef,
    TransitionRef, TransitionValue,
};
use wintf::ecs::FrameTime;
use wintf::ecs::dola::{DolaAnimator, tick_dola_animators};

/// テスト用 DolaRuntime にシンプルなアニメーションをロードするヘルパー。
fn make_runtime_with_animation() -> DolaRuntime {
    let doc = DolaDocumentBuilder::new("1.0")
        .variable(
            "opacity",
            AnimationVariableDef::Float {
                initial: 0.0,
                min: None,
                max: None,
            },
        )
        .transition(
            "fade_t",
            TransitionDef {
                from: Some(TransitionValue::Scalar(0.0)),
                to: Some(TransitionValue::Scalar(1.0)),
                duration: Some(1.0),
                ..Default::default()
            },
        )
        .storyboard(
            "fade_in",
            StoryboardBuilder::new()
                .entry(StoryboardEntry {
                    variable: Some("opacity".into()),
                    transition: Some(TransitionRef::Named("fade_t".into())),
                    ..Default::default()
                })
                .build(),
        )
        .build()
        .unwrap();

    let mut rt = DolaRuntime::new();
    rt.subscribe("opacity");
    rt.load_document(doc).unwrap();
    rt
}

// ---------------------------------------------------------------
// spawn → tick → last_result
// ---------------------------------------------------------------

#[test]
fn spawn_tick_last_result_flow() {
    // runtime_mut は pub(crate) なので、テストでは DolaRuntime を
    // 事前に start してから DolaAnimator に渡す。
    let mut rt = make_runtime_with_animation();
    rt.start("fade_in", 0.0).unwrap();
    let mut animator = DolaAnimator::with_runtime(rt);

    // tick to midpoint
    animator.tick(0.5);
    let len = animator.last_result().changes.len();
    assert!(len > 0, "Should have opacity change at t=0.5");

    // last_result is idempotent
    assert_eq!(len, animator.last_result().changes.len());
}

#[test]
fn default_animator_has_empty_result() {
    let mut animator = DolaAnimator::new();
    let result = animator.last_result();
    assert!(result.changes.is_empty());
    assert!(result.triggered.is_empty());
}

// ---------------------------------------------------------------
// tick_dola_animators System（Schedule で実行）
// ---------------------------------------------------------------

#[test]
fn tick_dola_animators_system_updates_all_entities() {
    let mut world = World::new();

    // FrameTime for the system
    world.insert_resource(FrameTime(0.5));

    // Spawn two animators
    let mut rt1 = make_runtime_with_animation();
    rt1.start("fade_in", 0.0).unwrap();
    let a1 = DolaAnimator::with_runtime(rt1);

    let mut rt2 = make_runtime_with_animation();
    rt2.start("fade_in", 0.0).unwrap();
    let a2 = DolaAnimator::with_runtime(rt2);

    let e1 = world.spawn(a1).id();
    let e2 = world.spawn(a2).id();

    // 本物の system を Schedule で回す
    let mut schedule = Schedule::default();
    schedule.add_systems(tick_dola_animators);
    schedule.run(&mut world);

    // Both should have changes
    let mut a1_ref = world.get_mut::<DolaAnimator>(e1).unwrap();
    assert!(
        !a1_ref.last_result().changes.is_empty(),
        "Entity 1 should have changes"
    );

    let mut a2_ref = world.get_mut::<DolaAnimator>(e2).unwrap();
    assert!(
        !a2_ref.last_result().changes.is_empty(),
        "Entity 2 should have changes"
    );
}
