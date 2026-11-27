//! `asis` で参照される絵の集め方と、`asis` 用の絵（乗算せず・透過色を抜かず・縁を切らず・α＝255）の作り方。

use std::collections::{BTreeMap, BTreeSet};

use areka_parsers::shell::same_canonical;

use crate::decode::DecodedImage;
use crate::manifest::{SurfaceSet, surface_ids};
use crate::table::{Point, Size};
use crate::trim::{TrimResult, Trimmed};

const ASIS: &str = "asis";

/// `asis` 用の絵を作る (セットの番号, 相対パス) の集合。
///
/// 入るのは、語が `asis` の element定義の行の絵と、語が `asis` の pattern（通常のアニメーション・
/// 着せ替えのどちらでも）が指す面のうち絵を 1 枚だけ持つ面の絵。
pub(crate) fn asis_paths(sets: &[SurfaceSet<'_>]) -> BTreeSet<(u32, String)> {
    let mut out = BTreeSet::new();
    for (set_index, set) in sets.iter().enumerate() {
        let set_id = set_index as u32;
        for element in set.surfaces.iter().flat_map(|s| &s.elements) {
            if same_canonical(element.method.as_str(), ASIS) {
                out.insert((set_id, element.path.as_str().to_string()));
            }
        }
        let targets: BTreeSet<i64> = set
            .surfaces
            .iter()
            .flat_map(|s| &s.animations)
            .flat_map(|a| &a.patterns)
            .filter(|p| same_canonical(p.method.as_str(), ASIS))
            .map(|p| p.surface_id)
            .collect();
        if targets.is_empty() {
            continue;
        }
        let mut pictures: BTreeMap<i64, Vec<&str>> = BTreeMap::new();
        for surface in set.surfaces {
            for id in surface_ids(surface) {
                if targets.contains(&id) {
                    let paths = pictures.entry(id).or_default();
                    paths.extend(surface.elements.iter().map(|e| e.path.as_str()));
                }
            }
        }
        // ファイル名の慣習だけの面は読み手が絵 1 枚の面として渡すので、ここでは element定義の面と
        // 見分けられない。絵 1 枚の面は全部焼き、どちらとして描くかは合成が決める。
        out.extend(
            pictures
                .into_values()
                .filter_map(|paths| match paths.as_slice() {
                    [only] => Some((set_id, only.to_string())),
                    _ => None,
                }),
        );
    }
    out
}

/// 乗算しない絵の α を 255 にし、原寸のまま頁に載せる形にする（α＝255 なので乗算済みと同じ色）。
pub(crate) fn picture(img: DecodedImage) -> TrimResult {
    let original = Size {
        w: img.width,
        h: img.height,
    };
    if img.width == 0 || img.height == 0 {
        return TrimResult {
            original,
            placement: None,
        };
    }
    let row_bytes = img.width as usize * 4;
    let mut pbgra = Vec::with_capacity(row_bytes * img.height as usize);
    for y in 0..img.height as usize {
        let start = y * img.stride as usize;
        pbgra.extend_from_slice(&img.bgra[start..start + row_bytes]);
    }
    for px in pbgra.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    TrimResult {
        original,
        placement: Some(Trimmed {
            trim_offset: Point { x: 0, y: 0 },
            size: original,
            pbgra,
            stride: row_bytes as u32,
        }),
    }
}

#[cfg(test)]
#[path = "asis_tests.rs"]
mod asis_tests;
