//! W3a-T: clip_sync_system（WUC モードのクリップ同期）のヘッドレステスト
//!
//! 実 WUC デバイスを用いて clip_sync_system を Schedule 経由で実行する。
//! WUC の InsetClip は write-only 相当で適用結果を読み戻せないため、
//! 各分岐（3 バリアント適用・クリップ解除・リソース未初期化スキップ）が
//! エラーなく完走することの characterization に留める
//! （詳細は W3a-T 断片の R2.8 所見を参照）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SingleThreadedExecutor;
use windows::UI::Composition::Visual as WucVisual;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::Interface;
use wintf::ecs::layout::Arrangement;
use wintf::ecs::{
    ClipShape, GraphicsCore, Size, Visual, VisualGraphics, WucGraphicsResource, clip_sync_system,
};

fn setup_world() -> World {
    // WucGraphicsResource::new は DQTAT_COM_NONE を使うため COM 初期化済みスレッドを要求する。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let core = GraphicsCore::new().expect("GraphicsCore 作成失敗");
    let d2d = core.d2d_device().expect("d2d device");
    let wuc_resource = WucGraphicsResource::new(d2d).expect("WucGraphicsResource 作成失敗");

    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc_resource);
    world
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
        schedule.run(&mut world);

        // clip = None に変更 → クリップ解除（clear_clip）経路
        world.get_mut::<Visual>(entity).unwrap().clip = None;
        schedule.run(&mut world);
    });
}

#[test]
fn clip_sync_clears_clip_when_size_is_zero() {
    crate::common::on_gpu_owner_thread(move || {
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
    });
}

#[test]
fn clip_sync_skips_when_wuc_resource_is_absent() {
    crate::common::on_gpu_owner_thread(move || {
        // WucGraphicsResource なし（ULW モード相当）→ 早期リターン
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
        world.entity_mut(entity).insert(VisualGraphics::default());
        // GlobalArrangement はフック挿入済みの default のまま

        let mut schedule = clip_schedule();
        schedule.run(&mut world);
        // パニックなしで完走すれば early return が機能している
    });
}
