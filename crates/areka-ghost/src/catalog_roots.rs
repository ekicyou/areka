//! ベースウェアの根の並び（design.md「CatalogRoots」）。
//!
//! 並びは呼び手（bin）が決めた順のまま持つ（上ほど先に探す）。重複を除くのは作り手。
//! 見つからない根も並びに残り、在否（`is_dir`）は呼ぶたびに見直す（要件 9.3）。
//! フォルダ名の同じ・違う（要件 2.8）と同じ根（要件 1.9）の規則を 1 つずつ持つ。
//! 並び全体の和・隠れ・引き当て・入れる根は、どれも同じ和（`union`）の上で決める（要件 2.6）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::catalog::{self, BalloonEntry, BasewareRoot, GhostEntry};

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

    /// 印の付いた項目（書ける根）。無ければ None。
    pub fn user_root(&self) -> Option<&BasewareRoot> {
        self.roots
            .iter()
            .find(|r| matches!(r.mark(), catalog::RootMark::UserRoot { .. }))
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

/// 隠れの種類（ゴーストとバルーンは別々に判断する＝要件 2.4）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreKind {
    Ghost,
    Balloon,
}

/// 隠れた 1 件。`hidden` は隠れたフォルダ、`by` は隠したフォルダ（どちらも絶対）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hidden {
    pub kind: StoreKind,
    pub folder: String,
    pub hidden: PathBuf,
    pub by: PathBuf,
}

/// 和と隠れ（起動と並びの変更のときの記録用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Survey {
    pub ghosts: Vec<GhostEntry>,
    pub balloons: Vec<BalloonEntry>,
    pub hidden: Vec<Hidden>,
}

/// 見つかる根を上から走査して和を組み、隠れを控える（要件 2.2〜2.4・2.7・3.1〜3.3）。
pub fn survey(roots: &BasewareRoots) -> Survey {
    let mut hidden = Vec::new();
    let ghosts = union(
        roots,
        StoreKind::Ghost,
        catalog::scan_ghosts,
        |e: &GhostEntry| (&e.identity.folder, &e.dir),
        &mut hidden,
    );
    let balloons = union(
        roots,
        StoreKind::Balloon,
        catalog::scan_balloons,
        |e: &BalloonEntry| (&e.identity.folder, &e.dir),
        &mut hidden,
    );
    Survey {
        ghosts,
        balloons,
        hidden,
    }
}

/// 隠れを除いたゴーストの和（フォルダ名のバイト順）。
pub fn list_ghosts(roots: &BasewareRoots) -> Vec<GhostEntry> {
    union(
        roots,
        StoreKind::Ghost,
        catalog::scan_ghosts,
        |e: &GhostEntry| (&e.identity.folder, &e.dir),
        &mut Vec::new(),
    )
}

/// 隠れを除いたバルーンの和（フォルダ名のバイト順）。
pub fn list_balloons(roots: &BasewareRoots) -> Vec<BalloonEntry> {
    union(
        roots,
        StoreKind::Balloon,
        catalog::scan_balloons,
        |e: &BalloonEntry| (&e.identity.folder, &e.dir),
        &mut Vec::new(),
    )
}

/// 和からフォルダ名で 1 つ（`same_folder_name`・要件 2.1・2.6）。
pub fn find_ghost(roots: &BasewareRoots, folder: &str) -> Option<GhostEntry> {
    list_ghosts(roots)
        .into_iter()
        .find(|e| same_folder_name(&e.identity.folder, folder))
}

/// 和からフォルダ名で 1 つ（`same_folder_name`・要件 2.1・2.6）。
pub fn find_balloon(roots: &BasewareRoots, folder: &str) -> Option<BalloonEntry> {
    list_balloons(roots)
        .into_iter()
        .find(|e| same_folder_name(&e.identity.folder, folder))
}

/// 入れる根。見えているものがあればその根、無ければ見つかるいちばん上の根。
/// `head_skipped` は先頭の根が見つからず飛ばしたときのその根（要件 5.2 の記録用）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallRoot {
    pub root: PathBuf,
    pub head_skipped: Option<PathBuf>,
}

/// 入れる根を決める（要件 5.1〜5.3）。見つかる根が 1 つも無ければ `None`。
pub fn install_root(roots: &BasewareRoots, kind: StoreKind, folder: &str) -> Option<InstallRoot> {
    let visible = match kind {
        StoreKind::Ghost => find_ghost(roots, folder).map(|e| e.root),
        StoreKind::Balloon => find_balloon(roots, folder).map(|e| e.root),
    };
    if let Some(root) = visible {
        return Some(InstallRoot {
            root,
            head_skipped: None,
        });
    }
    let top = roots.present().next()?.dir().to_path_buf();
    let head_skipped = roots
        .roots()
        .first()
        .filter(|head| !head.dir().is_dir())
        .map(|head| head.dir().to_path_buf());
    Some(InstallRoot {
        root: top,
        head_skipped,
    })
}

/// 見つかる根を上から 1 根ずつ走査して足す。同じフォルダ名がすでにあれば隠れとして控える。
/// 最後にフォルダ名のバイト順に並べ直す。
fn union<E>(
    roots: &BasewareRoots,
    kind: StoreKind,
    scan: fn(&BasewareRoot) -> Vec<E>,
    key: impl Fn(&E) -> (&String, &PathBuf),
    hidden: &mut Vec<Hidden>,
) -> Vec<E> {
    let mut seen: HashMap<String, PathBuf> = HashMap::new();
    let mut out = Vec::new();
    for root in roots.present() {
        for entry in scan(root) {
            let (folder, dir) = key(&entry);
            match seen.get(&fold_key(folder)) {
                Some(by) => hidden.push(Hidden {
                    kind,
                    folder: folder.clone(),
                    hidden: dir.clone(),
                    by: by.clone(),
                }),
                None => {
                    seen.insert(fold_key(folder), dir.clone());
                    out.push(entry);
                }
            }
        }
    }
    out.sort_by(|a, b| key(a).0.cmp(key(b).0));
    out
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
