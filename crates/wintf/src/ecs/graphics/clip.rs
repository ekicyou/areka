//! クリップ形状の型定義
//!
//! Visual コンポーネントのクリッピングに使用する形状 enum。
//! 矩形クリップ（角張り・全角統一角丸・各角個別角丸）の3バリアントを提供する。

use tracing::warn;

/// クリップ形状を表現する enum
///
/// # バリアント
/// - `Rectangle` — 角張った矩形クリップ（追加パラメーターなし）
/// - `RoundedRectangle` — 全角統一の角丸矩形クリップ
/// - `RoundedRectangleIndividual` — 各角個別設定の角丸矩形クリップ
///
/// # 使用例
/// ```
/// use wintf::ecs::ClipShape;
///
/// let rect = ClipShape::Rectangle;
/// let rounded = ClipShape::rounded_rectangle(10.0);
/// let individual = ClipShape::rounded_rectangle_individual(5.0, 10.0, 15.0, 20.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum ClipShape {
    /// 角張った矩形クリップ
    Rectangle,
    /// 全角統一の角丸矩形クリップ
    RoundedRectangle { radius: f32 },
    /// 各角個別設定の角丸矩形クリップ
    RoundedRectangleIndividual {
        top_left: f32,
        top_right: f32,
        bottom_left: f32,
        bottom_right: f32,
    },
}

impl ClipShape {
    /// 全角統一の角丸矩形クリップを作成する。
    ///
    /// 負の `radius` は 0.0 にクランプされ、`warn!` ログが出力される。
    pub fn rounded_rectangle(radius: f32) -> Self {
        if radius < 0.0 {
            warn!(
                value = radius,
                "ClipShape::RoundedRectangle radius is negative, clamping to 0.0"
            );
        }
        ClipShape::RoundedRectangle {
            radius: radius.max(0.0),
        }
    }

    /// 各角個別設定の角丸矩形クリップを作成する。
    ///
    /// 負の値は各フィールドごとに 0.0 にクランプされ、`warn!` ログが出力される。
    pub fn rounded_rectangle_individual(
        top_left: f32,
        top_right: f32,
        bottom_left: f32,
        bottom_right: f32,
    ) -> Self {
        if top_left < 0.0 || top_right < 0.0 || bottom_left < 0.0 || bottom_right < 0.0 {
            warn!(
                top_left = top_left,
                top_right = top_right,
                bottom_left = bottom_left,
                bottom_right = bottom_right,
                "ClipShape::RoundedRectangleIndividual has negative radius values, clamping to 0.0"
            );
        }
        ClipShape::RoundedRectangleIndividual {
            top_left: top_left.max(0.0),
            top_right: top_right.max(0.0),
            bottom_left: bottom_left.max(0.0),
            bottom_right: bottom_right.max(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_variant_creation() {
        let clip = ClipShape::Rectangle;
        assert_eq!(clip, ClipShape::Rectangle);
    }

    #[test]
    fn rounded_rectangle_creation() {
        let clip = ClipShape::rounded_rectangle(10.0);
        assert_eq!(clip, ClipShape::RoundedRectangle { radius: 10.0 });
    }

    #[test]
    fn rounded_rectangle_zero_radius() {
        let clip = ClipShape::rounded_rectangle(0.0);
        assert_eq!(clip, ClipShape::RoundedRectangle { radius: 0.0 });
    }

    #[test]
    fn rounded_rectangle_negative_clamped_to_zero() {
        let clip = ClipShape::rounded_rectangle(-5.0);
        assert_eq!(clip, ClipShape::RoundedRectangle { radius: 0.0 });
    }

    #[test]
    fn rounded_rectangle_individual_creation() {
        let clip = ClipShape::rounded_rectangle_individual(5.0, 10.0, 15.0, 20.0);
        assert_eq!(
            clip,
            ClipShape::RoundedRectangleIndividual {
                top_left: 5.0,
                top_right: 10.0,
                bottom_left: 15.0,
                bottom_right: 20.0,
            }
        );
    }

    #[test]
    fn rounded_rectangle_individual_negative_clamped() {
        let clip = ClipShape::rounded_rectangle_individual(-1.0, -2.0, -3.0, -4.0);
        assert_eq!(
            clip,
            ClipShape::RoundedRectangleIndividual {
                top_left: 0.0,
                top_right: 0.0,
                bottom_left: 0.0,
                bottom_right: 0.0,
            }
        );
    }

    #[test]
    fn rounded_rectangle_individual_partial_negative_clamped() {
        let clip = ClipShape::rounded_rectangle_individual(-1.0, 5.0, -3.0, 10.0);
        assert_eq!(
            clip,
            ClipShape::RoundedRectangleIndividual {
                top_left: 0.0,
                top_right: 5.0,
                bottom_left: 0.0,
                bottom_right: 10.0,
            }
        );
    }

    #[test]
    fn clip_shape_clone() {
        let original = ClipShape::rounded_rectangle(8.0);
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn clip_shape_debug() {
        let clip = ClipShape::Rectangle;
        let debug_str = format!("{:?}", clip);
        assert!(debug_str.contains("Rectangle"));
    }

    #[test]
    fn clip_shape_partial_eq_different_variants() {
        assert_ne!(ClipShape::Rectangle, ClipShape::rounded_rectangle(0.0));
        assert_ne!(
            ClipShape::rounded_rectangle(5.0),
            ClipShape::rounded_rectangle_individual(5.0, 5.0, 5.0, 5.0)
        );
    }

    #[test]
    fn visual_default_clip_is_none() {
        let visual = super::super::Visual::default();
        assert_eq!(visual.clip, None);
    }

    #[test]
    fn visual_set_clip() {
        let mut visual = super::super::Visual::default();
        visual.set_clip(Some(ClipShape::Rectangle));
        assert_eq!(visual.clip, Some(ClipShape::Rectangle));
        visual.set_clip(None);
        assert_eq!(visual.clip, None);
    }
}
