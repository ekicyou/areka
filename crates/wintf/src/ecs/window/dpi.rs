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
