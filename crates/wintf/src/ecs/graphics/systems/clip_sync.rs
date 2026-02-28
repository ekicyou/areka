//! DComp モードのクリップ同期システム
//!
//! Visual.clip の変更を検知し、IDCompositionRectangleClip を作成して
//! IDCompositionVisual3 に適用する。

use crate::com::dcomp::{DCompositionDeviceExt, DCompositionVisualExt};
use crate::ecs::graphics::Visual;
use crate::ecs::graphics::clip::ClipShape;
use crate::ecs::graphics::components::VisualGraphics;
use crate::ecs::graphics::dcomp_resource::DCompGraphicsResource;
use crate::ecs::layout::{Arrangement, GlobalArrangement};
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace};
use windows::core::Interface;

/// DComp モードのクリップ同期システム
///
/// Arrangement, GlobalArrangement, Visual のいずれかが変更されたエンティティについて、
/// Visual.clip に基づいて IDCompositionRectangleClip を作成・適用する。
///
/// # DPI スケーリング
/// - クリップ矩形のサイズに GlobalArrangement の累積スケール値を適用して物理ピクセル座標に変換
/// - 角丸の半径にも同一のスケール値を適用
///
/// # エラーハンドリング
/// COM API 失敗時は `error!` ログを出力して処理を継続する。
///
/// ULW モードなど DCompGraphicsResource が未初期化の場合はスキップする。
pub fn clip_sync_system(
    dcomp_resource: Option<Res<DCompGraphicsResource>>,
    query: Query<
        (
            Entity,
            &Arrangement,
            &GlobalArrangement,
            &Visual,
            &VisualGraphics,
        ),
        Or<(
            Changed<Arrangement>,
            Changed<GlobalArrangement>,
            Changed<Visual>,
        )>,
    >,
) {
    let Some(dcomp_resource) = dcomp_resource else {
        return;
    };
    let Some(dcomp) = dcomp_resource.dcomp() else {
        return;
    };

    for (entity, arrangement, ga, visual, vg) in query.iter() {
        let Some(visual_com) = vg.visual() else {
            continue;
        };

        let size = &arrangement.size;

        match &visual.clip {
            Some(clip_shape) if size.width > 0.0 && size.height > 0.0 => {
                // 物理ピクセル座標に変換
                let scale_x = ga.scale_x();
                let scale_y = ga.scale_y();
                let physical_right = size.width * scale_x;
                let physical_bottom = size.height * scale_y;

                // IDCompositionRectangleClip を作成
                let rectangle_clip = match dcomp.create_rectangle_clip() {
                    Ok(clip) => clip,
                    Err(e) => {
                        error!(
                            entity = ?entity,
                            error = ?e,
                            "[clip_sync] CreateRectangleClip failed"
                        );
                        continue;
                    }
                };

                // 矩形範囲を設定
                unsafe {
                    if let Err(e) = rectangle_clip.SetLeft2(0.0) {
                        error!(entity = ?entity, error = ?e, "[clip_sync] SetLeft failed");
                        continue;
                    }
                    if let Err(e) = rectangle_clip.SetTop2(0.0) {
                        error!(entity = ?entity, error = ?e, "[clip_sync] SetTop failed");
                        continue;
                    }
                    if let Err(e) = rectangle_clip.SetRight2(physical_right) {
                        error!(entity = ?entity, error = ?e, "[clip_sync] SetRight failed");
                        continue;
                    }
                    if let Err(e) = rectangle_clip.SetBottom2(physical_bottom) {
                        error!(entity = ?entity, error = ?e, "[clip_sync] SetBottom failed");
                        continue;
                    }
                }

                // ClipShape バリアント別の corner radius 設定
                let radius_result = match clip_shape {
                    ClipShape::Rectangle => {
                        // すべてのラジアス = 0.0
                        unsafe { set_all_corner_radii(&rectangle_clip, 0.0) }
                    }
                    ClipShape::RoundedRectangle { radius } => {
                        let physical_radius = radius * scale_x;
                        unsafe { set_all_corner_radii(&rectangle_clip, physical_radius) }
                    }
                    ClipShape::RoundedRectangleIndividual {
                        top_left,
                        top_right,
                        bottom_left,
                        bottom_right,
                    } => unsafe {
                        set_individual_corner_radii(
                            &rectangle_clip,
                            *top_left * scale_x,
                            *top_right * scale_x,
                            *bottom_left * scale_x,
                            *bottom_right * scale_x,
                        )
                    },
                };

                if let Err(e) = radius_result {
                    error!(
                        entity = ?entity,
                        error = ?e,
                        "[clip_sync] Set corner radii failed"
                    );
                    continue;
                }

                // クリップを適用
                let clip_interface: windows::Win32::Graphics::DirectComposition::IDCompositionClip =
                    rectangle_clip
                        .cast()
                        .expect("RectangleClip should implement IDCompositionClip");
                if let Err(e) = visual_com.set_clip_object(&clip_interface) {
                    error!(
                        entity = ?entity,
                        error = ?e,
                        "[clip_sync] SetClip failed"
                    );
                } else {
                    debug!(
                        entity = ?entity,
                        clip_shape = ?clip_shape,
                        physical_right,
                        physical_bottom,
                        "[clip_sync] Clip applied"
                    );
                }
            }
            _ => {
                // clip == None または size が (0, 0) → クリップ解除
                if let Err(e) = visual_com.clear_clip() {
                    error!(
                        entity = ?entity,
                        error = ?e,
                        "[clip_sync] SetClip(None) failed"
                    );
                } else {
                    trace!(
                        entity = ?entity,
                        "[clip_sync] Clip cleared"
                    );
                }
            }
        }
    }
}

/// 全8つの角丸パラメーターに同一の半径を設定する
unsafe fn set_all_corner_radii(
    clip: &windows::Win32::Graphics::DirectComposition::IDCompositionRectangleClip,
    radius: f32,
) -> windows::core::Result<()> {
    unsafe {
        clip.SetTopLeftRadiusX2(radius)?;
        clip.SetTopLeftRadiusY2(radius)?;
        clip.SetTopRightRadiusX2(radius)?;
        clip.SetTopRightRadiusY2(radius)?;
        clip.SetBottomLeftRadiusX2(radius)?;
        clip.SetBottomLeftRadiusY2(radius)?;
        clip.SetBottomRightRadiusX2(radius)?;
        clip.SetBottomRightRadiusY2(radius)?;
    }
    Ok(())
}

/// 各角に個別の半径を設定する（RadiusX = RadiusY）
unsafe fn set_individual_corner_radii(
    clip: &windows::Win32::Graphics::DirectComposition::IDCompositionRectangleClip,
    top_left: f32,
    top_right: f32,
    bottom_left: f32,
    bottom_right: f32,
) -> windows::core::Result<()> {
    unsafe {
        clip.SetTopLeftRadiusX2(top_left)?;
        clip.SetTopLeftRadiusY2(top_left)?;
        clip.SetTopRightRadiusX2(top_right)?;
        clip.SetTopRightRadiusY2(top_right)?;
        clip.SetBottomLeftRadiusX2(bottom_left)?;
        clip.SetBottomLeftRadiusY2(bottom_left)?;
        clip.SetBottomRightRadiusX2(bottom_right)?;
        clip.SetBottomRightRadiusY2(bottom_right)?;
    }
    Ok(())
}
