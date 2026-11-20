//! `region` の画像のマスク（GPU に依らない）: 色の規則と、ファイルごとに 1 度だけの読み。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use areka_emo_atlas::{DecodedImage, ElementDecoder};
use areka_parsers::shell::CollisionRegion;

/// 画素ごとに「当たる／当たらない」を持つ、画像と同じ大きさのマスク。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionMask {
    width: u32,
    height: u32,
    bits: Vec<bool>,
}

impl RegionMask {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// 画像の外は `false`。
    pub fn is_set(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height && self.bits[(y * self.width + x) as usize]
    }
}

/// 乗算しない BGRA の絵から、指定色（反転つき）のマスクを作る。
///
/// α＝0 の画素は蓄えた R・G・B に依らず (0,0,0) として、α≥1 の画素は蓄えた R・G・B で、
/// 3 成分の完全一致で比べる。全部の画素が指定色でも、そのまま全部を立てる。
pub fn region_mask(image: &DecodedImage, rgb: [u8; 3], invert: bool) -> RegionMask {
    let (width, height) = (image.width, image.height);
    let mut bits = Vec::with_capacity(width as usize * height as usize);
    for y in 0..height as usize {
        let row = &image.bgra[y * image.stride as usize..];
        for px in row.as_chunks::<4>().0.iter().take(width as usize) {
            let color = if px[3] == 0 {
                [0, 0, 0]
            } else {
                [px[2], px[1], px[0]]
            };
            bits.push((color == rgb) != invert);
        }
    }
    RegionMask {
        width,
        height,
        bits,
    }
}

/// 同じファイルの見分け（区切りを揃え ASCII の大小文字を畳む）。
fn file_key(file: &str) -> String {
    file.replace('/', "\\").to_ascii_lowercase()
}

/// `region` の画像をファイルごとに 1 度読み、（ファイル・色・反転）ごとに持つマスクの集まり。
#[derive(Debug, Default)]
pub struct RegionMasks {
    masks: HashMap<(String, [u8; 3], bool), Arc<RegionMask>>,
    unreadable: Vec<(String, String)>,
}

impl RegionMasks {
    /// 並びの中の `region` を全部たどり、ファイルごとに 1 度だけ `base_dir.join(書いた名前)` を
    /// 乗算しない読みで読む。抜き色と透過の宣言は当てない。
    pub fn load<'a>(
        regions: impl Iterator<Item = &'a CollisionRegion>,
        base_dir: &Path,
        decoder: &impl ElementDecoder,
    ) -> Self {
        let mut out = RegionMasks::default();
        let mut images: HashMap<String, Option<DecodedImage>> = HashMap::new();
        for region in regions {
            let file = file_key(&region.file);
            let image = images.entry(file.clone()).or_insert_with(|| {
                match decoder.decode_unmultiplied(&base_dir.join(&region.file)) {
                    Ok(image) => Some(image),
                    Err(e) => {
                        out.unreadable.push((region.file.clone(), e.to_string()));
                        None
                    }
                }
            });
            let Some(image) = image else { continue };
            let rgb = [region.red, region.green, region.blue];
            out.masks
                .entry((file, rgb, region.invert))
                .or_insert_with(|| Arc::new(region_mask(image, rgb, region.invert)));
        }
        out
    }

    /// 読めなかったファイルの `region` には `None`。
    pub fn get(&self, region: &CollisionRegion) -> Option<&Arc<RegionMask>> {
        self.masks.get(&(
            file_key(&region.file),
            [region.red, region.green, region.blue],
            region.invert,
        ))
    }

    /// 読めなかったファイル（書いた名前と理由）。ファイルごとに 1 件。
    pub fn unreadable(&self) -> &[(String, String)] {
        &self.unreadable
    }
}

#[cfg(test)]
#[path = "hit_mask_tests.rs"]
mod tests;
