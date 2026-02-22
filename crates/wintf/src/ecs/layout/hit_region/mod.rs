//! # Hit Region - 名前付きヒット領域データ
//!
//! エンティティ上の名前付きヒット領域（矩形・多角形・カラーマップ画像）を定義し、
//! マウスヒットテスト時にどの領域にヒットしたかを識別する機能を提供します。
//!
//! ## 概要
//!
//! - **矩形領域** (`Shape::Rect`): 座標ベースの矩形で領域を定義
//! - **多角形領域** (`Shape::Polygon`): 頂点リストで任意形状を定義
//! - **カラーマップ画像** (`ColorMapData`): 色分けPNG画像で領域を定義
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use wintf::ecs::layout::hit_region::{HitRegionMap, HitRegionMapBuilder};
//!
//! // 矩形＋多角形の混在定義
//! let region_map = HitRegionMap::builder()
//!     .rect("head", 20.0, 0.0, 60.0, 40.0)
//!     .polygon("hand", &[(0.0, 50.0), (30.0, 80.0), (0.0, 80.0)])
//!     .build()?;
//! ```

use std::collections::HashMap;
use std::fmt;

use bevy_ecs::prelude::*;
use tracing::debug;
use windows::core::Interface;

use super::metrics::Size;
use crate::com::wic::{
    WICBitmapDecoderExt, WICBitmapSourceExt, WICFormatConverterExt, WICImagingFactoryExt,
    wic_factory,
};

// ============================================================================
// HitRegionError - バリデーションエラー
// ============================================================================

/// ヒット領域定義に関するエラー
#[derive(Debug)]
pub enum HitRegionError {
    /// 多角形の頂点数が不足（3未満）
    InsufficientVertices { vertices: usize },
    /// 矩形のサイズが不正（幅または高さが0以下）
    InvalidRectSize { width: f32, height: f32 },
    /// カラーマップ画像の読込失敗
    ImageLoadFailed(windows::core::Error),
}

impl fmt::Display for HitRegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HitRegionError::InsufficientVertices { vertices } => {
                write!(f, "多角形の頂点数が不足しています: {} < 3", vertices)
            }
            HitRegionError::InvalidRectSize { width, height } => {
                write!(
                    f,
                    "矩形のサイズが不正です: width={}, height={}",
                    width, height
                )
            }
            HitRegionError::ImageLoadFailed(e) => {
                write!(f, "カラーマップ画像の読込に失敗しました: {}", e)
            }
        }
    }
}

impl std::error::Error for HitRegionError {}

impl From<windows::core::Error> for HitRegionError {
    fn from(e: windows::core::Error) -> Self {
        HitRegionError::ImageLoadFailed(e)
    }
}

// ============================================================================
// Shape / ShapeRegion - 形状定義
// ============================================================================

/// 形状定義（DIP単位、エンティティローカル座標系）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Shape {
    /// 矩形: 左上座標 + 幅 + 高さ（DIP単位）
    Rect {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// 多角形: 頂点リスト（DIP単位、自動的に閉じる）
    Polygon { vertices: Vec<(f32, f32)> },
}

/// 名前付き形状領域
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShapeRegion {
    /// 領域名
    pub name: String,
    /// 形状定義
    pub shape: Shape,
}

// ============================================================================
// ColorMapData - カラーマップキャッシュデータ
// ============================================================================

/// カラーマップ画像のキャッシュデータ
///
/// 画像読込時にピクセル→リージョンIDのインデックスマップを構築します。
/// serde対象外（バイナリキャッシュ）。
#[derive(Debug, Clone)]
pub struct ColorMapData {
    /// ピクセル座標 → リージョンID（0 = 無名）のインデックスマップ
    /// index_map[y * width + x] = region_id
    index_map: Vec<u8>,
    /// リージョンID → リージョン名（ID 1 から開始）
    region_names: Vec<String>,
    /// 画像幅
    width: u32,
    /// 画像高さ
    height: u32,
}

impl ColorMapData {
    /// WIC画像読込 + インデックスマップ構築
    ///
    /// PNG画像を読み込み、各ピクセルの色値からリージョンIDへのインデックスマップを構築する。
    /// マッピング外の色はID 0（無名）として扱う。
    pub fn from_image(
        image_path: &std::path::Path,
        mapping: &HashMap<(u8, u8, u8), String>,
    ) -> windows::core::Result<Self> {
        use windows::Win32::Foundation::GENERIC_READ;
        use windows::Win32::Graphics::Imaging::*;
        use windows::core::HSTRING;

        // リージョン名リストとカラー→ID マッピングを構築
        let mut region_names = Vec::new();
        let mut color_to_id: HashMap<(u8, u8, u8), u8> = HashMap::new();

        for (color, name) in mapping {
            let id = (region_names.len() + 1) as u8; // ID 1 から開始
            region_names.push(name.clone());
            color_to_id.insert(*color, id);
        }

        // WIC で PNG を読み込む
        let factory = wic_factory()?;
        let path_str = image_path.to_string_lossy();
        let path_hstring = HSTRING::from(path_str.as_ref());
        let decoder = factory.create_decoder_from_filename(
            &path_hstring,
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )?;
        let frame = decoder.frame(0)?;

        // PBGRA32 に変換
        let converter = factory.create_format_converter()?;
        converter.init(
            &frame.cast::<IWICBitmapSource>()?,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
        let converted: IWICBitmapSource = converter.cast()?;

        // サイズ取得
        let (width, height) = converted.get_size()?;
        let stride = width * 4; // PBGRA32: 4 bytes per pixel
        let buffer_size = (stride * height) as usize;
        let mut buffer = vec![0u8; buffer_size];

        // ピクセルデータをコピー
        converted.copy_pixels(None, stride, &mut buffer)?;

        // インデックスマップ構築（RGB→リージョンID変換）
        let pixel_count = (width * height) as usize;
        let mut index_map = vec![0u8; pixel_count];

        for i in 0..pixel_count {
            let offset = i * 4;
            // PBGRA32: B, G, R, A の順
            let b = buffer[offset];
            let g = buffer[offset + 1];
            let r = buffer[offset + 2];
            // アルファは無視

            if let Some(&id) = color_to_id.get(&(r, g, b)) {
                index_map[i] = id;
            }
            // マッピング外は 0（無名）のまま
        }

        debug!(
            width = width,
            height = height,
            regions = region_names.len(),
            "カラーマップ画像読込完了"
        );

        Ok(Self {
            index_map,
            region_names,
            width,
            height,
        })
    }

    /// ピクセル座標から領域名を返す
    pub fn hit_test(&self, pixel_x: u32, pixel_y: u32) -> Option<&str> {
        if pixel_x >= self.width || pixel_y >= self.height {
            return None;
        }
        let index = (pixel_y * self.width + pixel_x) as usize;
        let region_id = self.index_map[index];
        if region_id == 0 {
            None
        } else {
            self.region_names
                .get((region_id - 1) as usize)
                .map(|s| s.as_str())
        }
    }

    /// 画像幅
    pub fn width(&self) -> u32 {
        self.width
    }

    /// 画像高さ
    pub fn height(&self) -> u32 {
        self.height
    }
}

// ============================================================================
// ColorMapDef / ColorMapping - シリアライズ対象の定義構造
// ============================================================================

/// カラーマップ定義（シリアライズ対象）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorMapDef {
    /// 画像ファイルパス
    pub image_path: String,
    /// RGB → 領域名マッピング
    pub mapping: Vec<ColorMapping>,
}

/// 色→領域名マッピングエントリ
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorMapping {
    /// RGB値 [R, G, B]
    pub color: [u8; 3],
    /// 領域名
    pub name: String,
}

// ============================================================================
// RegionKind - 排他的方式 enum
// ============================================================================

/// 領域定義の方式（排他的）
///
/// `Shapes` と `ColorMap` は同時に使用できません。
#[derive(Debug, Clone)]
enum RegionKind {
    /// 矩形/多角形の形状リスト（混在可能）
    Shapes(Vec<ShapeRegion>),
    /// カラーマップ画像ベースの領域定義
    ColorMap(ColorMapData),
}

// ============================================================================
// HitRegionMap - ECSコンポーネント
// ============================================================================

/// 名前付きヒット領域マップコンポーネント
///
/// エンティティに紐づくヒット領域データを保持します。
/// 矩形/多角形方式またはカラーマップ方式のいずれか一方を使用します。
///
/// # 使用例
/// ```rust,ignore
/// use wintf::ecs::layout::hit_region::HitRegionMap;
///
/// // ビルダーで矩形/多角形方式を構築
/// let map = HitRegionMap::builder()
///     .rect("head", 20.0, 0.0, 60.0, 40.0)
///     .build()?;
///
/// // カラーマップ方式で構築
/// let map = HitRegionMap::from_color_map(path, &mapping)?;
/// ```
#[derive(Component, Debug, Clone)]
pub struct HitRegionMap {
    kind: RegionKind,
}

impl HitRegionMap {
    /// ビルダー開始（矩形/多角形方式）
    pub fn builder() -> HitRegionMapBuilder {
        HitRegionMapBuilder::new()
    }

    /// カラーマップ方式で構築
    ///
    /// # Arguments
    /// * `image_path` - カラーマップ PNG 画像パス
    /// * `mapping` - RGB → リージョン名のマッピング
    pub fn from_color_map(
        image_path: &std::path::Path,
        mapping: &HashMap<(u8, u8, u8), String>,
    ) -> Result<Self, HitRegionError> {
        let data = ColorMapData::from_image(image_path, mapping)?;
        Ok(Self {
            kind: RegionKind::ColorMap(data),
        })
    }

    /// 正規化座標から領域名を返す（AlphaMaskパターン踏襲）
    ///
    /// # Arguments
    /// * `rel_x`, `rel_y` - 正規化座標（0.0〜1.0）
    /// * `entity_size` - エンティティの論理サイズ（DIP単位、Shapes方式で使用）
    ///
    /// # Returns
    /// 領域内→`Some("name")`、領域外→`None`
    pub fn hit_test_region(&self, rel_x: f32, rel_y: f32, entity_size: &Size) -> Option<&str> {
        match &self.kind {
            RegionKind::Shapes(regions) => {
                // 正規化座標 → DIPローカル座標に変換
                let local_x = rel_x * entity_size.width;
                let local_y = rel_y * entity_size.height;

                // 定義順の先勝ちルール: 前から順に評価
                for region in regions {
                    let hit = match &region.shape {
                        Shape::Rect {
                            x,
                            y,
                            width,
                            height,
                        } => {
                            local_x >= *x
                                && local_x <= *x + *width
                                && local_y >= *y
                                && local_y <= *y + *height
                        }
                        Shape::Polygon { vertices } => point_in_polygon(local_x, local_y, vertices),
                    };
                    if hit {
                        return Some(&region.name);
                    }
                }
                None
            }
            RegionKind::ColorMap(data) => {
                // 正規化座標 → 画像ピクセル座標に変換
                let pixel_x = (rel_x * data.width() as f32) as u32;
                let pixel_y = (rel_y * data.height() as f32) as u32;
                data.hit_test(pixel_x, pixel_y)
            }
        }
    }
}

// ============================================================================
// HitRegionMapBuilder - ビルダーパターン
// ============================================================================

/// 矩形/多角形方式のヒット領域ビルダー
pub struct HitRegionMapBuilder {
    regions: Vec<ShapeRegion>,
}

impl HitRegionMapBuilder {
    /// 新しいビルダーを作成
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    /// 矩形領域を追加
    pub fn rect(mut self, name: &str, x: f32, y: f32, width: f32, height: f32) -> Self {
        self.regions.push(ShapeRegion {
            name: name.to_string(),
            shape: Shape::Rect {
                x,
                y,
                width,
                height,
            },
        });
        self
    }

    /// 多角形領域を追加
    pub fn polygon(mut self, name: &str, vertices: &[(f32, f32)]) -> Self {
        self.regions.push(ShapeRegion {
            name: name.to_string(),
            shape: Shape::Polygon {
                vertices: vertices.to_vec(),
            },
        });
        self
    }

    /// HitRegionMap を構築（バリデーション付き）
    ///
    /// # Errors
    /// - 矩形: width <= 0 または height <= 0 の場合 `InvalidRectSize`
    /// - 多角形: 頂点数 < 3 の場合 `InsufficientVertices`
    pub fn build(self) -> Result<HitRegionMap, HitRegionError> {
        // バリデーション
        for region in &self.regions {
            match &region.shape {
                Shape::Rect { width, height, .. } => {
                    if *width <= 0.0 || *height <= 0.0 {
                        return Err(HitRegionError::InvalidRectSize {
                            width: *width,
                            height: *height,
                        });
                    }
                }
                Shape::Polygon { vertices } => {
                    if vertices.len() < 3 {
                        return Err(HitRegionError::InsufficientVertices {
                            vertices: vertices.len(),
                        });
                    }
                }
            }
        }

        Ok(HitRegionMap {
            kind: RegionKind::Shapes(self.regions),
        })
    }
}

impl Default for HitRegionMapBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// point_in_polygon - Ray Casting 法
// ============================================================================

/// Ray Casting法による点の多角形内外判定
///
/// 判定点から右方向に半直線を伸ばし、多角形の辺との交差回数が奇数なら内部。
/// O(n) — n: 頂点数
fn point_in_polygon(x: f32, y: f32, vertices: &[(f32, f32)]) -> bool {
    let n = vertices.len();
    if n < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = n - 1;

    for i in 0..n {
        let (xi, yi) = vertices[i];
        let (xj, yj) = vertices[j];

        // 辺が判定点のY座標をまたぐかチェック
        if (yi > y) != (yj > y) {
            // 交点のX座標を計算
            let intersect_x = xj + (y - yj) / (yi - yj) * (xi - xj);
            if x < intersect_x {
                inside = !inside;
            }
        }

        j = i;
    }

    inside
}

// ============================================================================
// テスト

// ============================================================================
// テスト
// ============================================================================

#[cfg(test)]
mod tests;
