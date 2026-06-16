//! 合成描画パイプラインの RAII ガードとクリップ補助関数
//!
//! - `DcTargetGuard`: ID2D1DeviceContext のターゲット切替を RAII で管理
//! - `ClipGuard` / `ClipType`: D2D クリップ Push/Pop を RAII で管理
//! - `geometric_mask_layer_params`: geometricMask 付き PushLayer 用パラメータ構築

use crate::com::d2d::D2D1FactoryExt;
use crate::ecs::graphics::clip::ClipShape;
use windows::core::Interface;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Direct2D::*;
use windows_numerics::{Matrix3x2, Vector2};

/// ID2D1DeviceContext のターゲット切替を RAII パターンで管理し、
/// スコープ終了時に自動復元する。
pub(super) struct DcTargetGuard<'a> {
    dc: &'a ID2D1DeviceContext,
    prev_target: Option<ID2D1Image>,
}

impl<'a> DcTargetGuard<'a> {
    /// DC のターゲットを new_target に切り替え、RAII ガードを返す。
    pub(super) unsafe fn new(dc: &'a ID2D1DeviceContext, new_target: &ID2D1Bitmap1) -> Self {
        let prev_target = unsafe { dc.GetTarget().ok() };
        unsafe { dc.SetTarget(new_target) };
        Self { dc, prev_target }
    }
}

impl Drop for DcTargetGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            self.dc.SetTarget(self.prev_target.as_ref());
        }
    }
}

/// D2D クリップ Pop 方式の識別子。
enum ClipType {
    /// PopAxisAlignedClip で解除（Rectangle 用）
    AxisAligned,
    /// PopLayer で解除（RoundedRectangle / RoundedRectangleIndividual 用）
    Layer,
}

/// D2D クリップの RAII ガード。Drop 時に自動で Pop を実行。
///
/// `DcTargetGuard` と同じパターンで Push/Pop ペアの確実な実行を保証する。
pub(super) struct ClipGuard<'a> {
    dc: &'a ID2D1DeviceContext,
    clip_type: ClipType,
}

/// geometricMask 付き PushLayer 用の `D2D1_LAYER_PARAMETERS1` を構築する。
///
/// Note: `geometricMask` への `transmute`（owned move）は既存挙動の維持であり、
/// COM 参照リークを含む（P38 で修正提案済み。本関数は重複排除のみで挙動不変）。
fn geometric_mask_layer_params(
    geo_mask: ID2D1Geometry,
    width: f32,
    height: f32,
) -> D2D1_LAYER_PARAMETERS1 {
    D2D1_LAYER_PARAMETERS1 {
        contentBounds: D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: width,
            bottom: height,
        },
        geometricMask: unsafe { std::mem::transmute(Some(geo_mask)) },
        maskAntialiasMode: D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
        maskTransform: Matrix3x2::identity(),
        opacity: 1.0,
        opacityBrush: unsafe { std::mem::zeroed() },
        layerOptions: D2D1_LAYER_OPTIONS1_NONE,
    }
}

impl<'a> ClipGuard<'a> {
    /// クリップを Push し、RAII ガードを返す。
    ///
    /// サイズは Arrangement のローカル座標系 (0, 0)-(w, h)。
    /// SetTransform 後に呼び出すため、transform により物理座標に自動変換される。
    pub(super) unsafe fn push(
        dc: &'a ID2D1DeviceContext,
        clip_shape: &ClipShape,
        width: f32,
        height: f32,
    ) -> windows::core::Result<Self> {
        match clip_shape {
            ClipShape::Rectangle => {
                // Task 4.2: PushAxisAlignedClip
                let clip_rect = D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: width,
                    bottom: height,
                };
                unsafe {
                    dc.PushAxisAlignedClip(&clip_rect, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                }
                Ok(Self {
                    dc,
                    clip_type: ClipType::AxisAligned,
                })
            }
            ClipShape::RoundedRectangle { radius } => {
                // Task 4.3: PushLayer + RoundedRectangleGeometry
                let factory: ID2D1Factory = unsafe { dc.GetFactory()? };
                let rounded_rect = D2D1_ROUNDED_RECT {
                    rect: D2D_RECT_F {
                        left: 0.0,
                        top: 0.0,
                        right: width,
                        bottom: height,
                    },
                    radiusX: *radius,
                    radiusY: *radius,
                };
                let geometry = factory.create_rounded_rectangle_geometry(&rounded_rect)?;
                let geo_mask: ID2D1Geometry = geometry.cast()?;
                let layer_params = geometric_mask_layer_params(geo_mask, width, height);
                unsafe { dc.PushLayer(&layer_params, None) };
                Ok(Self {
                    dc,
                    clip_type: ClipType::Layer,
                })
            }
            ClipShape::RoundedRectangleIndividual {
                top_left,
                top_right,
                bottom_left,
                bottom_right,
            } => {
                // Task 4.4: PushLayer + PathGeometry（各角に個別半径の円弧）
                let factory: ID2D1Factory = unsafe { dc.GetFactory()? };
                let path_geo = factory.create_path_geometry()?;
                let sink = unsafe { path_geo.Open()? };

                // 4角を個別円弧で構築
                // 開始点: 左上角の円弧開始位置 (top_left, 0)
                let tl = *top_left;
                let tr = *top_right;
                let bl = *bottom_left;
                let br = *bottom_right;
                let w = width;
                let h = height;

                unsafe {
                    sink.BeginFigure(
                        Vector2 { X: tl, Y: 0.0 },
                        D2D1_FIGURE_BEGIN_FILLED,
                    );

                    // 上辺: top_left → top_right
                    sink.AddLine(Vector2 { X: w - tr, Y: 0.0 });

                    // 右上角の円弧
                    if tr > 0.0 {
                        sink.AddArc(&D2D1_ARC_SEGMENT {
                            point: Vector2 { X: w, Y: tr },
                            size: D2D_SIZE_F { width: tr, height: tr },
                            rotationAngle: 0.0,
                            sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                            arcSize: D2D1_ARC_SIZE_SMALL,
                        });
                    } else {
                        sink.AddLine(Vector2 { X: w, Y: 0.0 });
                    }

                    // 右辺: top_right → bottom_right
                    sink.AddLine(Vector2 { X: w, Y: h - br });

                    // 右下角の円弧
                    if br > 0.0 {
                        sink.AddArc(&D2D1_ARC_SEGMENT {
                            point: Vector2 { X: w - br, Y: h },
                            size: D2D_SIZE_F { width: br, height: br },
                            rotationAngle: 0.0,
                            sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                            arcSize: D2D1_ARC_SIZE_SMALL,
                        });
                    } else {
                        sink.AddLine(Vector2 { X: w, Y: h });
                    }

                    // 下辺: bottom_right → bottom_left
                    sink.AddLine(Vector2 { X: bl, Y: h });

                    // 左下角の円弧
                    if bl > 0.0 {
                        sink.AddArc(&D2D1_ARC_SEGMENT {
                            point: Vector2 { X: 0.0, Y: h - bl },
                            size: D2D_SIZE_F { width: bl, height: bl },
                            rotationAngle: 0.0,
                            sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                            arcSize: D2D1_ARC_SIZE_SMALL,
                        });
                    } else {
                        sink.AddLine(Vector2 { X: 0.0, Y: h });
                    }

                    // 左辺: bottom_left → top_left
                    sink.AddLine(Vector2 { X: 0.0, Y: tl });

                    // 左上角の円弧
                    if tl > 0.0 {
                        sink.AddArc(&D2D1_ARC_SEGMENT {
                            point: Vector2 { X: tl, Y: 0.0 },
                            size: D2D_SIZE_F { width: tl, height: tl },
                            rotationAngle: 0.0,
                            sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                            arcSize: D2D1_ARC_SIZE_SMALL,
                        });
                    }
                    // tl == 0 の場合、BeginFigure の始点に自動接続

                    sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                    sink.Close()?;
                }

                let geo_mask: ID2D1Geometry = path_geo.cast()?;
                let layer_params = geometric_mask_layer_params(geo_mask, width, height);
                unsafe { dc.PushLayer(&layer_params, None) };
                Ok(Self {
                    dc,
                    clip_type: ClipType::Layer,
                })
            }
        }
    }
}

impl Drop for ClipGuard<'_> {
    fn drop(&mut self) {
        unsafe {
            match self.clip_type {
                ClipType::AxisAligned => self.dc.PopAxisAlignedClip(),
                ClipType::Layer => self.dc.PopLayer(),
            }
        }
    }
}
