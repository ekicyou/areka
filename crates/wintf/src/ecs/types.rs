//! 共通幾何プリミティブ型
//!
//! ECS 層で使用する幾何学・空間型の唯一の定義箇所。
//! `#[repr(C)]` によるメモリレイアウト互換と、Win32/D2D1 型との
//! ゼロコスト `From`/`Into` 変換を提供する。

use bevy_ecs::prelude::*;
use tracing::warn;
use windows::Win32::Foundation::{POINT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{D2D_RECT_F, D2D_SIZE_F};
use windows_numerics::Vector2;

// ============================================================================
// Point — 整数座標ポイント（POINT 互換）
// ============================================================================

/// 整数座標ポイント（物理ピクセル）
///
/// Win32 `POINT` とメモリレイアウト互換 (`#[repr(C)]`)。
///
/// # メモリレイアウト
/// - `POINT { x: i32, y: i32 }` と完全フィールド名一致
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// 新しい Point を作成
    #[inline]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

// ============================================================================
// PointF — 浮動小数点座標ポイント（Vector2 互換）
// ============================================================================

/// 浮動小数点座標ポイント（スクリーン座標系）
///
/// `windows_numerics::Vector2` とメモリレイアウト互換 (`#[repr(C)]`)。
/// D2D1 に `D2D_POINT_2F` は windows 0.62.2 に存在しないため、
/// `Vector2` を変換ターゲットとする。
///
/// # メモリレイアウト
/// - `Vector2 { X: f32, Y: f32 }` — PascalCase ↔ snake_case マッピング
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PointF {
    pub x: f32,
    pub y: f32,
}

impl PointF {
    /// 新しい PointF を作成
    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

// ============================================================================
// Size — 浮動小数点サイズ（D2D_SIZE_F 互換）
// ============================================================================

/// レイアウトサイズ（幅と高さ）
///
/// D2D1 `D2D_SIZE_F` とメモリレイアウト互換 (`#[repr(C)]`)。
/// レイアウト計算後の確定サイズを保持する値オブジェクト。
///
/// # フィールド
/// - `width`: 幅（ピクセル単位、物理ピクセル）
/// - `height`: 高さ（ピクセル単位、物理ピクセル）
///
/// # 使用例
/// ```
/// use wintf::ecs::Size;
///
/// let size = Size { width: 100.0, height: 50.0 };
/// assert_eq!(size.width, 100.0);
/// ```
#[repr(C)]
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    /// サイズのバリデーション（負の値チェック）
    ///
    /// 負の値が検出された場合は警告ログを出力します。
    /// これはmayレベルの推奨事項であり、実行時エラーとはしません。
    pub fn validate(&self) {
        if self.width < 0.0 {
            warn!(width = self.width, "Size.width is negative");
        }
        if self.height < 0.0 {
            warn!(height = self.height, "Size.height is negative");
        }
    }
}

// ============================================================================
// SizeI — 整数サイズ（SIZE 互換）
// ============================================================================

/// 整数版サイズ型
///
/// Win32 `SIZE` とメモリレイアウト互換 (`#[repr(C)]`)。
///
/// # メモリレイアウト
/// - `SIZE { cx: i32, cy: i32 }` — フィールド名マッピング: width↔cx, height↔cy
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SizeI {
    pub width: i32,
    pub height: i32,
}

impl SizeI {
    /// 新しい SizeI を作成
    #[inline]
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
}

// ============================================================================
// Offset — 浮動小数点オフセット（親からの相対位置）
// ============================================================================

/// オフセット（親からの相対位置）
///
/// D2D1 には直接対応型がないが、`D2D_POINT_2F` 互換のレイアウト。
///
/// # フィールド
/// - `x`: X方向オフセット（ピクセル単位）
/// - `y`: Y方向オフセット（ピクセル単位）
#[repr(C)]
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
}

// ============================================================================
// Rect — 浮動小数点矩形（D2D_RECT_F 互換）
// ============================================================================

/// バウンディングボックス用矩形
///
/// D2D1 `D2D_RECT_F` とメモリレイアウト互換 (`#[repr(C)]`)。
///
/// # メモリレイアウト
/// - `D2D_RECT_F { left, top, right, bottom: f32 }` と完全フィールド名一致
///
/// # Note
/// ボックスモデル用の `Rect<T>`（`dimension.rs`）とは別の型。
/// こちらはバウンディングボックス・描画矩形用途。
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

// ============================================================================
// From/Into 変換実装
// ============================================================================

// --- Point ↔ POINT ---

impl From<POINT> for Point {
    #[inline]
    fn from(p: POINT) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<Point> for POINT {
    #[inline]
    fn from(p: Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

// --- PointF ↔ Vector2 ---

impl From<Vector2> for PointF {
    #[inline]
    fn from(v: Vector2) -> Self {
        Self { x: v.X, y: v.Y }
    }
}

impl From<PointF> for Vector2 {
    #[inline]
    fn from(p: PointF) -> Self {
        Self { X: p.x, Y: p.y }
    }
}

// --- Size ↔ D2D_SIZE_F ---

impl From<D2D_SIZE_F> for Size {
    #[inline]
    fn from(s: D2D_SIZE_F) -> Self {
        Self {
            width: s.width,
            height: s.height,
        }
    }
}

impl From<Size> for D2D_SIZE_F {
    #[inline]
    fn from(s: Size) -> Self {
        Self {
            width: s.width,
            height: s.height,
        }
    }
}

// --- SizeI ↔ SIZE ---

impl From<SIZE> for SizeI {
    #[inline]
    fn from(s: SIZE) -> Self {
        Self {
            width: s.cx,
            height: s.cy,
        }
    }
}

impl From<SizeI> for SIZE {
    #[inline]
    fn from(s: SizeI) -> Self {
        Self {
            cx: s.width,
            cy: s.height,
        }
    }
}

// --- Rect ↔ D2D_RECT_F ---

impl From<D2D_RECT_F> for Rect {
    #[inline]
    fn from(r: D2D_RECT_F) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

impl From<Rect> for D2D_RECT_F {
    #[inline]
    fn from(r: Rect) -> Self {
        Self {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }
    }
}

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    // --- メモリレイアウト互換性テスト ---

    #[test]
    fn test_point_memory_layout() {
        assert_eq!(mem::size_of::<Point>(), mem::size_of::<POINT>());
        assert_eq!(mem::align_of::<Point>(), mem::align_of::<POINT>());
    }

    #[test]
    fn test_pointf_memory_layout() {
        assert_eq!(mem::size_of::<PointF>(), mem::size_of::<Vector2>());
        assert_eq!(mem::align_of::<PointF>(), mem::align_of::<Vector2>());
    }

    #[test]
    fn test_size_memory_layout() {
        assert_eq!(mem::size_of::<Size>(), mem::size_of::<D2D_SIZE_F>());
        assert_eq!(mem::align_of::<Size>(), mem::align_of::<D2D_SIZE_F>());
    }

    #[test]
    fn test_sizei_memory_layout() {
        assert_eq!(mem::size_of::<SizeI>(), mem::size_of::<SIZE>());
        assert_eq!(mem::align_of::<SizeI>(), mem::align_of::<SIZE>());
    }

    #[test]
    fn test_rect_memory_layout() {
        assert_eq!(mem::size_of::<Rect>(), mem::size_of::<D2D_RECT_F>());
        assert_eq!(mem::align_of::<Rect>(), mem::align_of::<D2D_RECT_F>());
    }

    // --- From/Into ラウンドトリップテスト ---

    #[test]
    fn test_point_roundtrip() {
        let original = Point { x: 42, y: -17 };
        let win32: POINT = original.into();
        let back: Point = win32.into();
        assert_eq!(original, back);
    }

    #[test]
    fn test_pointf_roundtrip() {
        let original = PointF { x: 3.14, y: -2.71 };
        let vec2: Vector2 = original.into();
        let back: PointF = vec2.into();
        assert_eq!(original, back);
    }

    #[test]
    fn test_size_roundtrip() {
        let original = Size {
            width: 1920.0,
            height: 1080.0,
        };
        let d2d: D2D_SIZE_F = original.into();
        let back: Size = d2d.into();
        assert_eq!(original, back);
    }

    #[test]
    fn test_sizei_roundtrip() {
        let original = SizeI {
            width: 800,
            height: 600,
        };
        let win32: SIZE = original.into();
        let back: SizeI = win32.into();
        assert_eq!(original, back);
    }

    #[test]
    fn test_rect_roundtrip() {
        let original = Rect {
            left: 10.0,
            top: 20.0,
            right: 110.0,
            bottom: 70.0,
        };
        let d2d: D2D_RECT_F = original.into();
        let back: Rect = d2d.into();
        assert_eq!(original, back);
    }

    // --- Default 値テスト ---

    #[test]
    fn test_point_default() {
        let p = Point::default();
        assert_eq!(p.x, 0);
        assert_eq!(p.y, 0);
    }

    #[test]
    fn test_pointf_default() {
        let p = PointF::default();
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
    }

    #[test]
    fn test_size_default() {
        let s = Size::default();
        assert_eq!(s.width, 0.0);
        assert_eq!(s.height, 0.0);
    }

    #[test]
    fn test_sizei_default() {
        let s = SizeI::default();
        assert_eq!(s.width, 0);
        assert_eq!(s.height, 0);
    }

    #[test]
    fn test_offset_default() {
        let o = Offset::default();
        assert_eq!(o.x, 0.0);
        assert_eq!(o.y, 0.0);
    }

    #[test]
    fn test_rect_default() {
        let r = Rect::default();
        assert_eq!(r.left, 0.0);
        assert_eq!(r.top, 0.0);
        assert_eq!(r.right, 0.0);
        assert_eq!(r.bottom, 0.0);
    }

    // --- PartialEq テスト ---

    #[test]
    fn test_point_equality() {
        let a = Point { x: 1, y: 2 };
        let b = Point { x: 1, y: 2 };
        let c = Point { x: 3, y: 4 };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_size_equality() {
        let a = Size {
            width: 100.0,
            height: 50.0,
        };
        let b = Size {
            width: 100.0,
            height: 50.0,
        };
        let c = Size {
            width: 200.0,
            height: 50.0,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // --- コンストラクタテスト ---

    #[test]
    fn test_point_new() {
        let p = Point::new(100, 200);
        assert_eq!(p.x, 100);
        assert_eq!(p.y, 200);
    }

    #[test]
    fn test_pointf_new() {
        let p = PointF::new(3.14, 2.71);
        assert_eq!(p.x, 3.14);
        assert_eq!(p.y, 2.71);
    }

    #[test]
    fn test_sizei_new() {
        let s = SizeI::new(800, 600);
        assert_eq!(s.width, 800);
        assert_eq!(s.height, 600);
    }
}
