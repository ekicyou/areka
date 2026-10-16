//! ベースウェアの根の並び（design.md「CatalogRoots」）。
//!
//! 並びは呼び手（bin）が決めた順のまま持つ（上ほど先に探す）。重複を除くのは作り手。
//! 見つからない根も並びに残り、在否（`is_dir`）は呼ぶたびに見直す（要件 9.3）。
//! フォルダ名の同じ・違う（要件 2.8）と同じ根（要件 1.9）の規則を 1 つずつ持つ。

use std::path::{Path, PathBuf};

use crate::catalog::BasewareRoot;

/// 根の順の付いた並び（上ほど先に探す）。重複を除くのは作り手。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasewareRoots {
    roots: Vec<BasewareRoot>,
}

impl BasewareRoots {
    pub fn new(roots: Vec<BasewareRoot>) -> Self {
        Self { roots }
    }

    /// 根 1 つの並び（テストと単一の根の呼び手のための口）。
    pub fn single(dir: PathBuf) -> Self {
        Self::new(vec![BasewareRoot::new(dir)])
    }

    pub fn roots(&self) -> &[BasewareRoot] {
        &self.roots
    }

    /// 今見つかる（`is_dir`）根を上から。呼ぶたびに見直す。
    pub fn present(&self) -> impl Iterator<Item = &BasewareRoot> {
        self.roots.iter().filter(|r| r.dir().is_dir())
    }

    /// 見つかる根の `ghost\`（上から）。
    pub fn ghost_stores(&self) -> Vec<PathBuf> {
        self.present().map(BasewareRoot::ghost_store).collect()
    }

    /// 見つかる根の `balloon\`（上から）。
    pub fn balloon_stores(&self) -> Vec<PathBuf> {
        self.present().map(BasewareRoot::balloon_store).collect()
    }
}

/// フォルダ名の同じ・違う（要件 2.8 の 1 つの規則）。
pub fn same_folder_name(a: &str, b: &str) -> bool {
    fold_key(a) == fold_key(b)
}

/// 比べるための鍵: 1 字ずつ Unicode の単純大文字へ（1 字 → 1 字のときだけ写す）。
///
/// ponytail: Rust の Unicode の版が NTFS の大文字表（ボリュームを作った時点の表）と違う字では
/// OS と判断が分かれうる。食い違いが出たら OS の比較（`CompareStringOrdinal`）へ替える。
pub fn fold_key(name: &str) -> String {
    name.chars()
        .map(|c| {
            let mut up = c.to_uppercase();
            match (up.next(), up.next()) {
                (Some(u), None) => u,
                _ => c,
            }
        })
        .collect()
}

/// 同じ根か: 両方を絶対化し、末尾の区切りを落とした綴りを `fold_key` で比べる（実体は見ない）。
pub fn same_root(a: &Path, b: &Path) -> bool {
    root_key(a) == root_key(b)
}

fn root_key(p: &Path) -> String {
    // 絶対化に失敗する（空のパス等）ときはそのままの綴りで比べる。
    let abs = std::path::absolute(p).unwrap_or_else(|e| {
        tracing::warn!(
            event = "root_absolute_failed",
            path = %p.display(),
            error = %e,
            "根を絶対化できないのでそのままの綴りで比べる"
        );
        p.to_path_buf()
    });
    let s = abs.to_string_lossy();
    let trimmed = s.trim_end_matches(['\\', '/']);
    // `C:\` のような根だけのパスは区切りを落とすと意味が変わるので残す。
    if trimmed.ends_with(':') {
        return fold_key(&format!("{trimmed}\\"));
    }
    fold_key(trimmed)
}

#[cfg(test)]
#[path = "catalog_roots_tests.rs"]
mod tests;
