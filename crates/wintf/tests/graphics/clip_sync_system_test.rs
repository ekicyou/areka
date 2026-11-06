//! W3a-T: clip_sync_system（WUC モードのクリップ同期）のヘッドレステスト
//!
//! 実 WUC デバイスを用いて clip_sync_system を Schedule 経由で実行する。
//! 切り抜きの中身（図形）は読み戻さず、各分岐（3 バリアント適用・クリップ解除）で
//! `error` のログが 0 件であることと、切り抜きが当たっているか・外れたか・同じ物か
//! （`Visual::Clip()`）を確かめる。本番は COM の失敗を `error!` して続けるので、ログの数で失敗を見る。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SingleThreadedExecutor;
use log_capture_kit::count_levels;
use windows::UI::Composition::Visual as WucVisual;
use windows::core::Interface;
use windows_numerics::Matrix3x2;
use wintf::ecs::layout::{Arrangement, GlobalArrangement};
use wintf::ecs::{
    ClipShape, GraphicsCore, Size, Visual, VisualGraphics, WucGraphicsResource, clip_sync_system,
};

fn setup_world() -> World {
    // WucGraphicsResource::new が求める MTA の初期化は GPU の持ち主のスレッドが済ませている。
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let d2d = core.d2d_device().expect("d2d device");
    let wuc_resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource 作成失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc_resource);
    world
}

/// 1 回回し、`error` のログが 0 件であることを確かめる（ログの購読は回したスレッドに付くので、
/// GPU の持ち主のスレッドの閉包の中で呼ぶ）
fn run_without_error(schedule: &mut Schedule, world: &mut World) {
    let ((), counts) = count_levels(|| schedule.run(world));
    assert_eq!(counts.error, 0, "clip_sync_system が error を残した");
}

/// 実 WUC ビジュアルに切り抜きが当たっているか
fn has_clip(world: &World, entity: Entity) -> bool {
    world
        .get::<VisualGraphics>(entity)
        .and_then(|vg| vg.visual().cloned())
        .expect("見た目が無い")
        .Clip()
        .is_ok()
}

fn clip_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(clip_sync_system);
    schedule
}

/// Visual + 実 WUC ビジュアル + サイズ付き Arrangement のエンティティを生成
fn spawn_clip_entity(world: &mut World, visual: Visual, width: f32, height: f32) -> Entity {
    let wuc_visual: WucVisual = {
        let resource = world.resource::<WucGraphicsResource>();
        let compositor = resource.compositor().expect("compositor");
        compositor
            .CreateSpriteVisual()
            .expect("CreateSpriteVisual")
            .cast()
            .expect("cast to Visual")
    };

    // Visual の on_add フックが Arrangement → GlobalArrangement を連鎖挿入する
    let entity = world.spawn(visual).id();
    world.flush();

    world.get_mut::<Arrangement>(entity).unwrap().size = Size { width, height };
    world
        .entity_mut(entity)
        .insert(VisualGraphics::new(wuc_visual));
    entity
}

#[test]
fn clip_sync_applies_all_clip_shape_variants() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let mut schedule = clip_schedule();

        let rect = spawn_clip_entity(
            &mut world,
            Visual {
                clip: Some(ClipShape::Rectangle),
                ..Default::default()
            },
            100.0,
            50.0,
        );
        let rounded = spawn_clip_entity(
            &mut world,
            Visual {
                clip: Some(ClipShape::RoundedRectangle { radius: 8.0 }),
                ..Default::default()
            },
            100.0,
            50.0,
        );
        let individual = spawn_clip_entity(
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

        // 3 バリアントすべてが角丸の図形の切り抜き → SetClip まで失敗なく通る
        run_without_error(&mut schedule, &mut world);
        for entity in [rect, rounded, individual] {
            assert!(has_clip(&world, entity), "切り抜きが当たっていない");
        }
    });
}

#[test]
fn clip_sync_clears_clip_when_clip_is_none() {
    crate::common::on_gpu_owner_thread(move || {
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
        run_without_error(&mut schedule, &mut world);
        assert!(has_clip(&world, entity), "切り抜きが当たっていない");

        // clip = None に変更 → クリップ解除（SetClip(None)）経路
        world.get_mut::<Visual>(entity).unwrap().clip = None;
        run_without_error(&mut schedule, &mut world);
        assert!(
            !has_clip(&world, entity),
            "clip = None で切り抜きが外れない"
        );
    });
}

#[test]
fn clip_sync_clears_clip_when_size_is_zero() {
    crate::common::on_gpu_owner_thread(move || {
        let mut world = setup_world();
        let mut schedule = clip_schedule();

        // いったん切り抜きを当ててから、clip はあるがサイズ (0, 0) → クリップ解除経路に分岐する
        let entity = spawn_clip_entity(
            &mut world,
            Visual {
                clip: Some(ClipShape::Rectangle),
                ..Default::default()
            },
            100.0,
            50.0,
        );
        run_without_error(&mut schedule, &mut world);
        assert!(has_clip(&world, entity), "切り抜きが当たっていない");

        world.get_mut::<Arrangement>(entity).unwrap().size = Size {
            width: 0.0,
            height: 0.0,
        };
        run_without_error(&mut schedule, &mut world);
        assert!(!has_clip(&world, entity), "大きさ 0 で切り抜きが外れない");
    });
}

/// 位置だけ変えても切り抜きは同じ物のまま・大きさを変えると作り直す・
/// 見た目を作り直した部品には新しい見た目へ当て直す（11 番・6 番）
#[test]
fn clip_sync_keeps_clip_on_move_and_reapplies_to_recreated_visual() {
    crate::common::on_gpu_owner_thread(move || {
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
        let visual = |world: &World| {
            world
                .get::<VisualGraphics>(entity)
                .unwrap()
                .visual()
                .unwrap()
                .clone()
        };
        let vg_changed = |world: &World| {
            world
                .entity(entity)
                .get_change_ticks::<VisualGraphics>()
                .unwrap()
                .changed
        };
        schedule.run(&mut world);
        let first = visual(&world).Clip().expect("切り抜きが当たっていない");

        // 位置だけ変える
        world
            .get_mut::<GlobalArrangement>(entity)
            .unwrap()
            .transform = Matrix3x2 {
            M11: 1.0,
            M12: 0.0,
            M21: 0.0,
            M22: 1.0,
            M31: 30.0,
            M32: 30.0,
        };
        schedule.run(&mut world);
        assert_eq!(
            visual(&world).Clip().unwrap(),
            first,
            "位置だけで切り抜きを作り直した"
        );

        // 大きさを変えると作り直す。作り直して鍵を書いても部品の変化の印は動かない
        let vg_tick = vg_changed(&world);
        world.get_mut::<Arrangement>(entity).unwrap().size = Size {
            width: 200.0,
            height: 50.0,
        };
        schedule.run(&mut world);
        assert_ne!(
            visual(&world).Clip().unwrap(),
            first,
            "大きさが変わったのに切り抜きが古いまま"
        );
        assert_eq!(
            vg_changed(&world),
            vg_tick,
            "鍵の書き込みが見た目の部品の変化の印を立てた"
        );

        // 見た目を作り直す（本番の visual_manager と同じく中身を差し替える）
        let second: WucVisual = {
            let resource = world.resource::<WucGraphicsResource>();
            let compositor = resource.compositor().expect("compositor");
            compositor.CreateSpriteVisual().unwrap().cast().unwrap()
        };
        *world.get_mut::<VisualGraphics>(entity).unwrap() = VisualGraphics::new(second.clone());
        schedule.run(&mut world);
        assert!(
            second.Clip().is_ok(),
            "作り直した見た目に切り抜きが当たっていない"
        );
    });
}
