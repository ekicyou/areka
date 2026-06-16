//! W3a-T: clip_sync_system（DComp モードのクリップ同期）のヘッドレステスト
//!
//! 実 DComp デバイスを用いて clip_sync_system を Schedule 経由で実行する。
//! IDCompositionRectangleClip は write-only COM オブジェクトで適用結果を
//! 読み戻せないため、各分岐（3 バリアント適用・クリップ解除・リソース未初期化
//! スキップ）がエラーなく完走することの characterization に留める
//! （詳細は W3a-T 断片の R2.8 所見を参照）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ExecutorKind;
use wintf::com::dcomp::DCompositionDeviceExt;
use wintf::ecs::layout::Arrangement;
use wintf::ecs::{
    ClipShape, DCompGraphicsResource, GraphicsCore, Size, Visual, VisualGraphics, clip_sync_system,
};

fn setup_world() -> World {
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let d2d = core.d2d_device().expect("d2d device");
    let dcomp_resource = DCompGraphicsResource::new(d2d).expect("DCompGraphicsResource 作成失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(dcomp_resource);
    world
}

fn clip_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(clip_sync_system);
    schedule
}

/// Visual + 実 DComp ビジュアル + サイズ付き Arrangement のエンティティを生成
fn spawn_clip_entity(world: &mut World, visual: Visual, width: f32, height: f32) -> Entity {
    let dcomp_visual = {
        let resource = world.resource::<DCompGraphicsResource>();
        let dcomp = resource.dcomp().expect("dcomp device");
        dcomp.create_visual().expect("create_visual")
    };

    // Visual の on_add フックが Arrangement → GlobalArrangement を連鎖挿入する
    let entity = world.spawn(visual).id();
    world.flush();

    world.get_mut::<Arrangement>(entity).unwrap().size = Size { width, height };
    world
        .entity_mut(entity)
        .insert(VisualGraphics::new(dcomp_visual));
    entity
}

#[test]
fn clip_sync_applies_all_clip_shape_variants() {
    let mut world = setup_world();
    let mut schedule = clip_schedule();

    spawn_clip_entity(
        &mut world,
        Visual {
            clip: Some(ClipShape::Rectangle),
            ..Default::default()
        },
        100.0,
        50.0,
    );
    spawn_clip_entity(
        &mut world,
        Visual {
            clip: Some(ClipShape::RoundedRectangle { radius: 8.0 }),
            ..Default::default()
        },
        100.0,
        50.0,
    );
    spawn_clip_entity(
        &mut world,
        Visual {
            clip: Some(ClipShape::RoundedRectangleIndividual {
                top_left: 4.0,
                top_right: 8.0,
                bottom_left: 12.0,
                bottom_right: 16.0,
            }),
            ..Default::default()
        },
        100.0,
        50.0,
    );

    // 3 バリアントすべてが SetLeft/SetRight/角丸 8 パラメーター設定 → SetClip まで
    // エラーなく完走する（COM エラー時は error! ログ + continue のため panic しない設計だが、
    // 正常系では cast().expect が通ることを実行で確認する）
    schedule.run(&mut world);
}

#[test]
fn clip_sync_clears_clip_when_clip_is_none() {
    let mut world = setup_world();
    let mut schedule = clip_schedule();

    let entity = spawn_clip_entity(
        &mut world,
        Visual {
            clip: Some(ClipShape::Rectangle),
            ..Default::default()
        },
        100.0,
        50.0,
    );

    // 1回目: クリップ適用
    schedule.run(&mut world);

    // clip = None に変更 → クリップ解除（clear_clip）経路
    world.get_mut::<Visual>(entity).unwrap().clip = None;
    schedule.run(&mut world);
}

#[test]
fn clip_sync_clears_clip_when_size_is_zero() {
    let mut world = setup_world();
    let mut schedule = clip_schedule();

    // clip はあるがサイズ (0, 0) → クリップ解除経路に分岐する
    spawn_clip_entity(
        &mut world,
        Visual {
            clip: Some(ClipShape::Rectangle),
            ..Default::default()
        },
        0.0,
        0.0,
    );

    schedule.run(&mut world);
}

#[test]
fn clip_sync_skips_when_dcomp_resource_is_absent() {
    // DCompGraphicsResource なし（ULW モード相当）→ 早期リターン
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let mut world = World::new();
    world.insert_resource(core);

    // VisualGraphics は default（COM なし）でクエリ成立だけさせる
    let entity = world
        .spawn(Visual {
            clip: Some(ClipShape::Rectangle),
            ..Default::default()
        })
        .id();
    world.flush();
    world.get_mut::<Arrangement>(entity).unwrap().size = Size {
        width: 100.0,
        height: 50.0,
    };
    world
        .entity_mut(entity)
        .insert(VisualGraphics::default());
    // GlobalArrangement はフック挿入済みの default のまま

    let mut schedule = clip_schedule();
    schedule.run(&mut world);
    // パニックなしで完走すれば early return が機能している
}
