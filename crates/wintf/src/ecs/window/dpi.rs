//! DPI情報コンポーネント
//!
//! - `DPI`: ウィンドウDPI値・スケーリング変換

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{LPARAM, WPARAM};

/// DPI情報を保持するコンポーネント
///
/// Windowエンティティ専用。SparseSetストレージを使用。
/// - dpi_x, dpi_y: 通常同一値だが、将来の拡張性のため分離
/// - デフォルト値: 96 (Windows標準DPI)
///
/// # Example
/// ```
/// use wintf::ecs::DPI;
///
/// let dpi = DPI::from_dpi(120, 120);
/// assert_eq!(dpi.scale_x(), 1.25);
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[component(storage = "SparseSet")]
pub struct DPI {
    /// X方向のDPI値 (96 = 100%)
    pub dpi_x: u16,
    /// Y方向のDPI値 (96 = 100%)
    pub dpi_y: u16,
}

impl Default for DPI {
    fn default() -> Self {
        Self {
            dpi_x: 96,
            dpi_y: 96,
        }
    }
}

impl DPI {
    /// DPI値からインスタンスを作成
    pub fn from_dpi(x_dpi: u16, y_dpi: u16) -> Self {
        Self {
            dpi_x: x_dpi,
            dpi_y: y_dpi,
        }
    }

    /// WM_DPICHANGEDメッセージのwparamから作成
    ///
    /// # Arguments
    /// * `wparam` - WM_DPICHANGEDのWPARAM (LOWORD=X DPI, HIWORD=Y DPI)
    /// * `_lparam` - WM_DPICHANGEDのLPARAM (未使用だが署名の一貫性のため保持)
    #[allow(non_snake_case)]
    pub fn from_WM_DPICHANGED(wparam: WPARAM, _lparam: LPARAM) -> Self {
        let x_dpi = (wparam.0 & 0xFFFF) as u16;
        let y_dpi = ((wparam.0 >> 16) & 0xFFFF) as u16;
        Self::from_dpi(x_dpi, y_dpi)
    }

    /// X方向のスケール係数を取得 (1.0 = 96 DPI)
    pub fn scale_x(&self) -> f32 {
        self.dpi_x as f32 / 96.0
    }

    /// Y方向のスケール係数を取得 (1.0 = 96 DPI)
    pub fn scale_y(&self) -> f32 {
        self.dpi_y as f32 / 96.0
    }

    // ========================================
    // 座標変換関数（物理ピクセル ⇔ 論理座標 DIP）
    // ========================================

    /// 物理ピクセル値をX方向の論理座標（DIP）に変換
    ///
    /// # Example
    /// ```
    /// use wintf::ecs::DPI;
    /// let dpi = DPI::from_dpi(192, 192); // 200% scale
    /// assert_eq!(dpi.to_logical_x(200), 100.0); // 200px → 100dip
    /// ```
    #[inline]
    pub fn to_logical_x(&self, physical: i32) -> f32 {
        physical as f32 / self.scale_x()
    }

    /// 物理ピクセル値をY方向の論理座標（DIP）に変換
    #[inline]
    pub fn to_logical_y(&self, physical: i32) -> f32 {
        physical as f32 / self.scale_y()
    }

    /// 物理ピクセルサイズを論理座標サイズ（DIP）に変換
    #[inline]
    pub fn to_logical_size(&self, width: i32, height: i32) -> (f32, f32) {
        (self.to_logical_x(width), self.to_logical_y(height))
    }

    /// 物理ピクセル位置を論理座標位置（DIP）に変換
    #[inline]
    pub fn to_logical_point(&self, x: i32, y: i32) -> (f32, f32) {
        (self.to_logical_x(x), self.to_logical_y(y))
    }

    /// 論理座標（DIP）を物理ピクセル値に変換（X方向）
    ///
    /// # Example
    /// ```
    /// use wintf::ecs::DPI;
    /// let dpi = DPI::from_dpi(192, 192); // 200% scale
    /// assert_eq!(dpi.to_physical_x(100.0), 200); // 100dip → 200px
    /// ```
    #[inline]
    pub fn to_physical_x(&self, logical: f32) -> i32 {
        (logical * self.scale_x()).round() as i32
    }

    /// 論理座標（DIP）を物理ピクセル値に変換（Y方向）
    #[inline]
    pub fn to_physical_y(&self, logical: f32) -> i32 {
        (logical * self.scale_y()).round() as i32
    }

    /// 論理座標サイズ（DIP）を物理ピクセルサイズに変換
    #[inline]
    pub fn to_physical_size(&self, width: f32, height: f32) -> (i32, i32) {
        (self.to_physical_x(width), self.to_physical_y(height))
    }

    /// 論理座標位置（DIP）を物理ピクセル位置に変換
    #[inline]
    pub fn to_physical_point(&self, x: f32, y: f32) -> (i32, i32) {
        (self.to_physical_x(x), self.to_physical_y(y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_96_dpi() {
        let dpi = DPI::default();
        assert_eq!(dpi.dpi_x, 96);
        assert_eq!(dpi.dpi_y, 96);
        // 96 DPI = 100% スケール
        assert_eq!(dpi.scale_x(), 1.0);
        assert_eq!(dpi.scale_y(), 1.0);
    }

    #[test]
    fn test_from_dpi_stores_axes_independently() {
        // X/Y は将来の拡張性のため独立保持される
        let dpi = DPI::from_dpi(120, 144);
        assert_eq!(dpi.dpi_x, 120);
        assert_eq!(dpi.dpi_y, 144);
        assert_eq!(dpi.scale_x(), 120.0 / 96.0); // 1.25
        assert_eq!(dpi.scale_y(), 144.0 / 96.0); // 1.5
    }

    #[test]
    fn test_from_wm_dpichanged_parses_loword_and_hiword() {
        // WM_DPICHANGED の WPARAM は LOWORD=X DPI / HIWORD=Y DPI
        // 0x00C00078 → X=0x0078=120, Y=0x00C0=192
        let wparam = WPARAM(0x00C0_0078);
        let dpi = DPI::from_WM_DPICHANGED(wparam, LPARAM(0));
        assert_eq!(dpi.dpi_x, 120);
        assert_eq!(dpi.dpi_y, 192);
    }

    #[test]
    fn test_from_wm_dpichanged_masks_upper_bits() {
        // 上位 32bit や 16bit を超える値はマスクされ下位16bitのみ採用される
        // 下位 = 0x6060_6060 → X=0x6060, Y=0x6060
        let wparam = WPARAM(0xFFFF_FFFF_6060_6060);
        let dpi = DPI::from_WM_DPICHANGED(wparam, LPARAM(0));
        assert_eq!(dpi.dpi_x, 0x6060);
        assert_eq!(dpi.dpi_y, 0x6060);
    }

    #[test]
    fn test_scale_at_192_dpi_is_2x() {
        let dpi = DPI::from_dpi(192, 192);
        assert_eq!(dpi.scale_x(), 2.0);
        assert_eq!(dpi.scale_y(), 2.0);
    }

    #[test]
    fn test_to_logical_y_and_size_and_point() {
        let dpi = DPI::from_dpi(192, 192); // 200%
        assert_eq!(dpi.to_logical_y(200), 100.0);
        assert_eq!(dpi.to_logical_size(200, 400), (100.0, 200.0));
        assert_eq!(dpi.to_logical_point(50, 80), (25.0, 40.0));
    }

    #[test]
    fn test_to_physical_y_and_size_and_point() {
        let dpi = DPI::from_dpi(192, 192); // 200%
        assert_eq!(dpi.to_physical_y(100.0), 200);
        assert_eq!(dpi.to_physical_size(100.0, 50.0), (200, 100));
        assert_eq!(dpi.to_physical_point(25.0, 40.0), (50, 80));
    }

    #[test]
    fn test_to_physical_rounds_half_away_from_zero() {
        // 150% スケール: 1.0 dip → 1.5 px → round() = 2 (half away from zero)
        let dpi = DPI::from_dpi(144, 144);
        assert_eq!(dpi.scale_x(), 1.5);
        assert_eq!(dpi.to_physical_x(1.0), 2); // 1.5 を最近接整数へ（.round() の偶数丸めではない）
        assert_eq!(dpi.to_physical_x(3.0), 5); // 4.5 → 5（偶数丸めなら 4 になる）
    }

    #[test]
    fn test_logical_physical_at_100_percent_is_identity() {
        let dpi = DPI::default(); // 96 DPI
        assert_eq!(dpi.to_logical_x(123), 123.0);
        assert_eq!(dpi.to_physical_x(123.0), 123);
    }
}
