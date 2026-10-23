//! SSP と areka がそれぞれ `--offline-dump` で書き出した 2 つのフォルダの、同じ名前の PNG を
//! 画素で比べる道具（spec: areka-P0-offline-surface-dump 要件 13）。
//!
//! 両方のフォルダの直下の `*.png` を名前（大文字小文字を区別しない）で組にし、前もって乗算して
//! いない RGBA 8 ビットで読んで、組ごとの [`Verdict`] を名前の昇順で返す。合否は決めない
//! （許す違いの大きさは使う試験が決める・13.4）。フォルダは読むだけで、SSP は起こさない（13.3）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// どちらのフォルダか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Ssp,
    Areka,
}

/// 1 組の比べた答え。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Same,
    SizeDiffers {
        ssp: (u32, u32),
        areka: (u32, u32),
    },
    /// `max_color` は R・G・B の差の最大、`max_alpha` は α の差の最大。
    PixelsDiffer {
        count: u64,
        max_color: u8,
        max_alpha: u8,
    },
    OnlyIn(Side),
    /// 読めないファイルは黙って落とさない。
    Unreadable {
        side: Side,
        message: String,
    },
}

/// ファイル 1 つ分の答え。`name` は SSP 側の綴り（SSP 側に無ければ areka 側）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileVerdict {
    pub name: String,
    pub verdict: Verdict,
}

/// 2 つのフォルダの PNG を比べる。答えは名前の昇順。フォルダが読めなければ Err。
pub fn compare_dirs(ssp_dir: &Path, areka_dir: &Path) -> std::io::Result<Vec<FileVerdict>> {
    let ssp = list_pngs(ssp_dir)?;
    let mut areka = list_pngs(areka_dir)?;
    let mut out = Vec::new();
    for (key, (name, ssp_path)) in ssp {
        let verdict = match areka.remove(&key) {
            Some((_, areka_path)) => compare_files(&ssp_path, &areka_path),
            None => Verdict::OnlyIn(Side::Ssp),
        };
        out.push((key, FileVerdict { name, verdict }));
    }
    out.extend(areka.into_iter().map(|(key, (name, _))| {
        let verdict = Verdict::OnlyIn(Side::Areka);
        (key, FileVerdict { name, verdict })
    }));
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out.into_iter().map(|(_, v)| v).collect())
}

/// 直下の `*.png` を、小文字にした名前 → （元の綴り・パス）で並べる。
fn list_pngs(dir: &Path) -> std::io::Result<BTreeMap<String, (String, PathBuf)>> {
    let mut map = BTreeMap::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let is_png = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"));
        if !is_png || !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        map.insert(name.to_lowercase(), (name, path));
    }
    Ok(map)
}

fn compare_files(ssp_path: &Path, areka_path: &Path) -> Verdict {
    let read = |path: &Path, side: Side| {
        image::open(path)
            .map(|img| img.to_rgba8())
            .map_err(|err| Verdict::Unreadable {
                side,
                message: err.to_string(),
            })
    };
    let (ssp, areka) = match (read(ssp_path, Side::Ssp), read(areka_path, Side::Areka)) {
        (Ok(s), Ok(a)) => (s, a),
        (Err(v), _) | (_, Err(v)) => return v,
    };
    if ssp.dimensions() != areka.dimensions() {
        return Verdict::SizeDiffers {
            ssp: ssp.dimensions(),
            areka: areka.dimensions(),
        };
    }
    let (mut count, mut max_color, mut max_alpha) = (0u64, 0u8, 0u8);
    for (s, a) in ssp.pixels().zip(areka.pixels()) {
        if s == a {
            continue;
        }
        count += 1;
        for c in 0..3 {
            max_color = max_color.max(s[c].abs_diff(a[c]));
        }
        max_alpha = max_alpha.max(s[3].abs_diff(a[3]));
    }
    if count == 0 {
        Verdict::Same
    } else {
        Verdict::PixelsDiffer {
            count,
            max_color,
            max_alpha,
        }
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
