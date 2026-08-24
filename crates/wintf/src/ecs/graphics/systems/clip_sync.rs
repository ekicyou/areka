//! WUC モードのクリップ同期システム
//!
//! Visual.clip の変更を検知し、`ClipShape` の 3 変種を Windows.UI.Composition の
//! clip 型（`InsetClip` / `CompositionGeometricClip` + geometry）へ等価写像して
//! WUC `Visual` に適用する。DComp 版（IDCompositionRectangleClip）の意味論を等価に写す。

use crate::ecs::graphics::Visual;
use crate::ecs::graphics::clip::ClipShape;
use crate::ecs::graphics::components::VisualGraphics;
use crate::ecs::graphics::wuc_resource::WucGraphicsResource;
use crate::ecs::layout::{Arrangement, GlobalArrangement};
use bevy_ecs::prelude::*;
use tracing::{debug, error, trace, warn};
use windows::UI::Composition::{CompositionClip, Compositor};
use windows::core::Interface;

/// WUC モードのクリップ同期システム
///
/// Arrangement, GlobalArrangement, Visual のいずれかが変更されたエンティティについて、
/// Visual.clip に基づいて WUC clip を作成・適用する。
///
/// # DPI スケーリング
/// - クリップ矩形のサイズに GlobalArrangement の累積スケール値を適用して物理ピクセル座標に変換
/// - 角丸の半径にも同一のスケール値を適用（DComp 版 clip_sync_system と同一計算）
///
/// # エラーハンドリング
/// COM API 失敗時は `error!` ログを出力して処理を継続する。
///
/// WucGraphicsResource が未初期化の場合はスキップする。
pub fn clip_sync_system(
    wuc_resource: Option<Res<WucGraphicsResource>>,
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
    let Some(wuc_resource) = wuc_resource else {
        return;
    };
    let Some(compositor) = wuc_resource.compositor() else {
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
                let physical_width = size.width * scale_x;
                let physical_height = size.height * scale_y;

                let clip_res = build_clip(
                    compositor,
                    clip_shape,
                    physical_width,
                    physical_height,
                    scale_x,
                );

                let clip = match clip_res {
                    Ok(c) => c,
                    Err(e) => {
                        error!(
                            entity = ?entity,
                            error = ?e,
                            "[clip_sync] Failed to build WUC clip"
                        );
                        continue;
                    }
                };

                if let Err(e) = visual_com.SetClip(&clip) {
                    error!(
                        entity = ?entity,
                        error = ?e,
                        "[clip_sync] SetClip failed"
                    );
                } else {
                    debug!(
                        entity = ?entity,
                        clip_shape = ?clip_shape,
                        physical_width,
                        physical_height,
                        "[clip_sync] Clip applied"
                    );
                }
            }
            _ => {
                // clip == None または size が (0, 0) → クリップ解除
                // WUC: SetClip(None) で解除（DComp の SetClip(None) と等価）。
                if let Err(e) = visual_com.SetClip(None::<&CompositionClip>) {
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

/// `ClipShape` の 3 変種を WUC `CompositionClip` へ等価写像する。
///
/// - `Rectangle` → `CreateInsetClip()`（全 inset 0・矩形範囲そのまま）。
/// - `RoundedRectangle{radius}` → `CreateRoundedRectangleGeometry()`
///   （`SetSize` + `SetCornerRadius`）→ `CreateGeometricClipWithGeometry`。
/// - `RoundedRectangleIndividual{4角}` → 角ごとの半径が異なる clip は WUC の
///   `CompositionPathGeometry` + `CompositionPath` を要するが、`CompositionPath::Create`
///   は `IGeometrySource2D`（Win2D 由来）を必須とし、`windows` 0.62.2 単体では
///   構築経路が存在しない（registry 確認済み・`Graphics` feature でも Win2D なしでは不可）。
///   areka 本体は個別半径を使わず（design.md「例外は example/ULW guard のみ」）、ビルド・
///   挙動等価目的の全変種対応として、**4 角の最大半径による均一角丸矩形へ縮約**して
///   `RoundedRectangleGeometry` で写す（均一角丸ケース＝全角同値では厳密一致）。非均一時の
///   角差は WUC clip 単体 API の制約による近似であることを警告ログで明示する。
fn build_clip(
    compositor: &Compositor,
    clip_shape: &ClipShape,
    physical_width: f32,
    physical_height: f32,
    scale_x: f32,
) -> windows::core::Result<CompositionClip> {
    match clip_shape {
        ClipShape::Rectangle => {
            // 角張った矩形クリップ。DComp 版は RectangleClip に明示的な矩形
            // (0,0,physical_width,physical_height) を設定していた（＝Visual 自身のサイズに
            // 依存しない絶対矩形）。WUC の InsetClip は Visual 自身のサイズを基準に inset する
            // ため、Visual に SetSize していない本パイプラインでは等価にならない
            // （visual_property_sync は offset/opacity のみ設定）。よって半径 0 の
            // RoundedRectangleGeometry（明示サイズ）→ GeometricClip で「絶対サイズの矩形クリップ」
            // として写像し、DComp の矩形範囲と等価にする。
            build_rounded_geometric_clip(compositor, physical_width, physical_height, 0.0)
        }
        ClipShape::RoundedRectangle { radius } => {
            let physical_radius = radius * scale_x;
            build_rounded_geometric_clip(
                compositor,
                physical_width,
                physical_height,
                physical_radius,
            )
        }
        ClipShape::RoundedRectangleIndividual {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        } => {
            let tl = top_left * scale_x;
            let tr = top_right * scale_x;
            let bl = bottom_left * scale_x;
            let br = bottom_right * scale_x;
            // 全角同値なら均一角丸として厳密一致。異なる場合は最大値へ縮約（近似）。
            let uniform = tl.max(tr).max(bl).max(br);
            let all_equal = tl == tr && tr == bl && bl == br;
            if !all_equal {
                warn!(
                    top_left = tl,
                    top_right = tr,
                    bottom_left = bl,
                    bottom_right = br,
                    uniform,
                    "[clip_sync] RoundedRectangleIndividual with non-uniform radii approximated by \
                     uniform max radius (WUC lacks per-corner clip geometry without Win2D)"
                );
            }
            build_rounded_geometric_clip(compositor, physical_width, physical_height, uniform)
        }
    }
}

/// 角丸矩形 geometry から `CompositionGeometricClip` を組む。
fn build_rounded_geometric_clip(
    compositor: &Compositor,
    physical_width: f32,
    physical_height: f32,
    physical_radius: f32,
) -> windows::core::Result<CompositionClip> {
    let geometry = compositor.CreateRoundedRectangleGeometry()?;
    geometry.SetSize(windows_numerics::Vector2 {
        X: physical_width,
        Y: physical_height,
    })?;
    geometry.SetCornerRadius(windows_numerics::Vector2 {
        X: physical_radius,
        Y: physical_radius,
    })?;
    let clip = compositor.CreateGeometricClipWithGeometry(&geometry)?;
    clip.cast::<CompositionClip>()
}
