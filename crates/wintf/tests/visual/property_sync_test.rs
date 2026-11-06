//! visual_property_sync_system のテスト (W3b-T)
//!
//! WUC の `Visual` は `Opacity()`・`Offset()` を読み戻せるので、同期の後に読んで期待の値と比べる
//! （読み出しも GPU の持ち主のスレッドの閉包の中で行う）。
//! 対象: crates/wintf/src/ecs/graphics/systems/visual_sync.rs (visual_property_sync_system)

use bevy_ecs::prelude::*;
use windows::core::Result;
use windows_numerics::Vector3;
use wintf::ecs::layout::{Arrangement, Offset};
use wintf::ecs::{Visual, VisualGraphics, Window, visual_property_sync_system};

use super::common::{WucVisualFactory, setup_graphics};

fn run_sync(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_property_sync_system);
    schedule.run(world);
}

/// 実 Visual を持つテストワールドをセットアップして (world, factory) を返す
fn setup() -> Result<(World, WucVisualFactory)> {
    let graphics = setup_graphics()?;
    let factory = WucVisualFactory::new(&graphics)?;
    let mut world = World::new();
    world.insert_resource(graphics);
    Ok((world, factory))
}

/// 配置の位置（論理座標・累積の倍率は既定の 1）
fn arrangement_at(x: f32, y: f32) -> Arrangement {
    Arrangement {
        offset: Offset { x, y },
        ..Default::default()
    }
}

/// 非 Window エンティティ: offset と opacity の両方が WUC の Visual へ写る
///
/// Note: Arrangement::on_add が GlobalArrangement を連鎖挿入するため、
/// クエリの全要求コンポーネントが揃う。
#[test]
fn syncs_offset_and_opacity_for_non_window_entity() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let (mut world, factory) = setup()?;
        let visual = factory.create_visual()?;

        world.spawn((
            Visual {
                opacity: 0.5,
                ..Default::default()
            },
            arrangement_at(10.0, 20.0),
            VisualGraphics::new(visual.clone()),
        ));
        world.flush();

        run_sync(&mut world);
        assert_eq!(visual.Opacity()?, 0.5);
        assert_eq!(
            visual.Offset()?,
            Vector3 {
                X: 10.0,
                Y: 20.0,
                Z: 0.0
            }
        );
        Ok(())
    })
}

/// 無効な VisualGraphics（COM Visual なし）はスキップされる
#[test]
fn skips_entity_with_invalid_visual_graphics() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let (mut world, _factory) = setup()?;

        world.spawn((Visual::default(), VisualGraphics::default()));
        world.flush();

        // visual() が None のため continue する経路。パニックしないこと。
        run_sync(&mut world);
        Ok(())
    })
}

/// Window エンティティは offset 設定がスキップされ opacity のみ設定される
#[test]
fn window_entity_skips_offset_branch() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let (mut world, factory) = setup()?;
        let visual = factory.create_visual()?;

        world.spawn((
            Window::default(),
            Visual {
                opacity: 0.5,
                ..Default::default()
            },
            arrangement_at(10.0, 20.0),
            VisualGraphics::new(visual.clone()),
        ));
        world.flush();

        run_sync(&mut world);
        assert_eq!(visual.Opacity()?, 0.5);
        // 窓の位置は Win32 が持つので、WUC の Offset は既定の 0 のまま
        assert_eq!(visual.Offset()?, Vector3::default());
        Ok(())
    })
}

/// is_visible=false は opacity 0.0 を写す
#[test]
fn invisible_visual_takes_zero_opacity_branch() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let (mut world, factory) = setup()?;
        let visual = factory.create_visual()?;

        world.spawn((
            Visual {
                is_visible: false,
                opacity: 1.0,
                ..Default::default()
            },
            VisualGraphics::new(visual.clone()),
        ));
        world.flush();

        run_sync(&mut world);
        assert_eq!(visual.Opacity()?, 0.0);
        Ok(())
    })
}

/// 範囲外 opacity は clamped_opacity() でクランプされて写る
#[test]
fn out_of_range_opacity_is_clamped_before_set() -> Result<()> {
    crate::gpu::on_gpu_owner_thread(|| -> Result<()> {
        let (mut world, factory) = setup()?;
        let visual = factory.create_visual()?;
        // 既定の 1.0 と見分けるため、同期の前の値をずらしておく
        visual.SetOpacity(0.25)?;

        world.spawn((
            Visual {
                opacity: 5.0,
                ..Default::default()
            },
            VisualGraphics::new(visual.clone()),
        ));
        world.flush();

        run_sync(&mut world);
        assert_eq!(visual.Opacity()?, 1.0);
        Ok(())
    })
}
