//! DolaAnimator ECS Component テスト (Task 9.2)
//!
//! - spawn → tick → last_result 読み取りの一連フロー
//! - tick_dola_animators System による全エンティティ一括 tick
//! - runtime() アクセスパターン

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
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
    let result = animator.last_result();
    assert!(
        !result.changes.is_empty(),
        "Should have opacity change at t=0.5"
    );

    // last_result is idempotent
    let result2 = animator.last_result();
    assert_eq!(result.changes.len(), result2.changes.len());
}

#[test]
fn default_animator_has_empty_result() {
    let animator = DolaAnimator::new();
    let result = animator.last_result();
    assert!(result.changes.is_empty());
    assert!(result.triggered.is_empty());
}

// ---------------------------------------------------------------
// tick_dola_animators System（手動 Query 実行）
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

    // Run the system via SystemState
    let mut system_state =
        SystemState::<(Query<&mut DolaAnimator>, Res<FrameTime>)>::new(&mut world);
    {
        let (mut query, frame_time) = system_state.get_mut(&mut world);
        for mut animator in query.iter_mut() {
            animator.tick(frame_time.0);
        }
    }

    // Both should have changes
    let a1_ref = world.get::<DolaAnimator>(e1).unwrap();
    assert!(
        !a1_ref.last_result().changes.is_empty(),
        "Entity 1 should have changes"
    );

    let a2_ref = world.get::<DolaAnimator>(e2).unwrap();
    assert!(
        !a2_ref.last_result().changes.is_empty(),
        "Entity 2 should have changes"
    );
}

// ---------------------------------------------------------------
// runtime() 読み取りアクセス
// ---------------------------------------------------------------

#[test]
fn runtime_accessor_returns_inner_runtime() {
    let rt = make_runtime_with_animation();
    let animator = DolaAnimator::with_runtime(rt);

    // runtime() は読み取り可能
    let _runtime = animator.runtime();
    // Debug trait
    let debug = format!("{:?}", animator);
    assert!(debug.contains("DolaAnimator"));
}

// ---------------------------------------------------------------
// tick_dola_animators 関数のシグネチャ検証
// ---------------------------------------------------------------

#[test]
fn tick_dola_animators_is_valid_system() {
    // tick_dola_animators の型が System として有効であることをコンパイル時に確認
    let _ = tick_dola_animators as fn(Query<&mut DolaAnimator>, Res<FrameTime>);
}
