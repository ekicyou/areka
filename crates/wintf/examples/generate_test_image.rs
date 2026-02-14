//! テスト画像生成ユーティリティ
//!
//! `cargo run --example generate_test_image` で実行

use image::{Rgba, RgbaImage};
use std::path::Path;

fn main() {
    let assets_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("assets");

    // 64x64 白(半透明)/透明 チェッカーパターン（α確認用）
    generate_checker_pattern(
        &assets_dir.join("demo_checker_64x64.png"),
        64,
        8,
        Rgba([255, 255, 255, 200]), // 白（半透明 α≈78%）
        Rgba([0, 0, 0, 0]),         // 完全透明
    );

    println!("Generated: demo_checker_64x64.png (white=semi-transparent, other=fully transparent)");

    // 64x64 カラーマップテスト画像（リージョンテスト用）
    // 4分割: 赤(top-left) / 緑(top-right) / 青(bottom-left) / 黄(bottom-right)
    generate_region_colormap(&assets_dir.join("demo_region_colormap_64x64.png"), 64);

    println!("Generated: demo_region_colormap_64x64.png (color-coded region map)");
}

/// カラーマップリージョンテスト画像を生成
///
/// 64x64 の画像で4分割:
/// - 赤 (255,0,0): "top-left" — 左上 (x: 0..32, y: 0..32)
/// - 緑 (0,255,0): "top-right" — 右上 (x: 32..64, y: 0..32)
/// - 青 (0,0,255): "bottom-left" — 左下 (x: 0..32, y: 32..64)
/// - 黄 (255,255,0): "bottom-right" — 右下 (x: 32..64, y: 32..64)
fn generate_region_colormap(path: &Path, size: u32) {
    let mut img = RgbaImage::new(size, size);
    let half = size / 2;

    for y in 0..size {
        for x in 0..size {
            let color = if x < half && y < half {
                // 左上: 赤 (top-left)
                Rgba([255, 0, 0, 255])
            } else if x >= half && y < half {
                // 右上: 緑 (top-right)
                Rgba([0, 255, 0, 255])
            } else if x < half && y >= half {
                // 左下: 青 (bottom-left)
                Rgba([0, 0, 255, 255])
            } else {
                // 右下: 黄 (bottom-right)
                Rgba([255, 255, 0, 255])
            };
            img.put_pixel(x, y, color);
        }
    }

    img.save(path)
        .expect("Failed to save region colormap image");
}

fn generate_checker_pattern(
    path: &Path,
    size: u32,
    block_size: u32,
    color1: Rgba<u8>,
    color2: Rgba<u8>,
) {
    let mut img = RgbaImage::new(size, size);

    for y in 0..size {
        for x in 0..size {
            let block_x = x / block_size;
            let block_y = y / block_size;
            let color = if (block_x + block_y) % 2 == 0 {
                color1
            } else {
                color2
            };
            img.put_pixel(x, y, color);
        }
    }

    img.save(path).expect("Failed to save image");
}
