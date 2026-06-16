//! visual_property_sync_system のギャップテスト (W3b-T)
//!
//! IDCompositionVisual3 の SetOffsetX/Y・SetOpacity は write-only COM API で
//! 読み戻しができないため、各分岐の完走 characterization に留める
//! （クリップ・不透明度の意味論は ULW 側ピクセルテストで間接固定済み — W3a 所見5 参照）。
//! 対象: crates/wintf/src/ecs/graphics/systems/visual_sync.rs (visual_property_sync_system)

use bevy_ecs::prelude::*;
use windows::core::Result;
use wintf::com::dcomp::DCompositionDeviceExt;
use wintf::ecs::{
    DCompGraphicsResource, Visual, VisualGraphics, Window, visual_property_sync_system,
};

use super::common::setup_graphics;

fn run_sync(world: &mut World) {
    let mut schedule = Schedule::default();
    schedule.add_systems(visual_property_sync_system);
    schedule.run(world);
}

/// 実 Visual を持つテストワールドをセットアップして (world, dcomp) を返す
fn setup() -> Result<(World, DCompGraphicsResource)> {
    let graphics = setup_graphics()?;
    let d2d = graphics.d2d_device().expect("d2d device");
    let dcomp_resource = DCompGraphicsResource::new(d2d)?;
    let mut world = World::new();
    world.insert_resource(graphics);
    Ok((world, dcomp_resource))
}

/// 非 Window エンティティ: offset + opacity の両方を設定する経路が完走する
///
/// Note: Visual::on_add が Arrangement を自動挿入し、Arrangement::on_add が
/// GlobalArrangement を連鎖挿入するため、クエリの全要求コンポーネントが揃う。
#[test]
fn syncs_offset_and_opacity_for_non_window_entity() -> Result<()> {
    let (mut world, dcomp_resource) = setup()?;
    let dcomp = dcomp_resource.dcomp().expect("dcomp device");
    let visual = dcomp.create_visual()?;

    world.spawn((
        Visual {
            opacity: 0.5,
            ..Default::default()
        },
        VisualGraphics::new(visual),
    ));
    world.flush();

    run_sync(&mut world);
    Ok(())
}

/// 無効な VisualGraphics（COM Visual なし）はスキップされる
#[test]
fn skips_entity_with_invalid_visual_graphics() -> Result<()> {
    let (mut world, _dcomp_resource) = setup()?;

    world.spawn((Visual::default(), VisualGraphics::default()));
    world.flush();

    // visual() が None のため continue する経路。パニックしないこと。
    run_sync(&mut world);
    Ok(())
}

/// Window エンティティは offset 設定がスキップされ opacity のみ設定される
#[test]
fn window_entity_skips_offset_branch() -> Result<()> {
    let (mut world, dcomp_resource) = setup()?;
    let dcomp = dcomp_resource.dcomp().expect("dcomp device");
    let visual = dcomp.create_visual()?;

    world.spawn((
        Window::default(),
        Visual::default(),
        VisualGraphics::new(visual),
    ));
    world.flush();

    run_sync(&mut world);
    Ok(())
}

/// is_visible=false は opacity 0.0 の分岐を通る
#[test]
fn invisible_visual_takes_zero_opacity_branch() -> Result<()> {
    let (mut world, dcomp_resource) = setup()?;
    let dcomp = dcomp_resource.dcomp().expect("dcomp device");
    let visual = dcomp.create_visual()?;

    world.spawn((
        Visual {
            is_visible: false,
            opacity: 1.0,
            ..Default::default()
        },
        VisualGraphics::new(visual),
    ));
    world.flush();

    run_sync(&mut world);
    Ok(())
}

/// 範囲外 opacity は clamped_opacity() 経由でクランプされる分岐を通る
#[test]
fn out_of_range_opacity_is_clamped_before_set() -> Result<()> {
    let (mut world, dcomp_resource) = setup()?;
    let dcomp = dcomp_resource.dcomp().expect("dcomp device");
    let visual = dcomp.create_visual()?;

    world.spawn((
        Visual {
            opacity: 5.0, // SetOpacity(5.0) は DComp 的に不正だが clamp(1.0) で安全
            ..Default::default()
        },
        VisualGraphics::new(visual),
    ));
    world.flush();

    run_sync(&mut world);
    Ok(())
}
