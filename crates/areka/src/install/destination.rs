//! 並びでインストールの宛先の根に答える部品（design.md「InstallDestination」・要件 5.1〜5.3・5.6）。
//!
//! 答えは `catalog_roots::install_root` から取る（新しいものは見つかるいちばん上の根・すでにあるものは
//! 見えている根）。どの答えも並びの写しの綴りのまま返す（`areka-nar` は作業フォルダを根の綴りの
//! 完全一致でまとめるため・tasks.md Implementation Notes 1.5）。行き先を尋ねる画面は出さない（5.6）。

use std::path::{Path, PathBuf};

use areka_ghost::BasewareRoots;
use areka_ghost::catalog_roots::{StoreKind, install_root};

/// 並びの写しで `InstallRoots` に答える。見つかる根が 1 つも無ければ作れない。
// 呼び手（インストールの窓口と背景スレッド）は tasks.md 4.6 でつなぐ。
#[allow(dead_code)]
pub(crate) struct CatalogDestinations {
    roots: BasewareRoots,
    /// 作ったときの見つかるいちばん上の根（答えが出ないときに答える）。
    first: PathBuf,
}

#[allow(dead_code)] // 呼び手は tasks.md 4.6 で来る
impl CatalogDestinations {
    /// 見つかる根が無ければ `None`（呼び手が error `install_no_root` を残して今の失敗の経路へ倒す）。
    pub(crate) fn new(roots: BasewareRoots) -> Option<Self> {
        let first = roots.present().next()?.dir().to_path_buf();
        Some(Self { roots, first })
    }

    fn answer(&self, kind: StoreKind, folder: &str) -> PathBuf {
        // 答えが出ない（尋ねた間に全部の根が消えた）ときは作ったときの根を答える。
        // `areka-nar` が作業フォルダを掘れずに失敗として返し、記録は `areka-nar` の今の `error!`。
        let Some(dest) = install_root(&self.roots, kind, folder) else {
            return self.first.clone();
        };
        if let Some(head) = &dest.head_skipped {
            log_head_skipped(head, &dest.root);
        }
        dest.root
    }
}

impl areka_nar::InstallRoots for CatalogDestinations {
    fn ghost_root(&self, folder: &str) -> PathBuf {
        self.answer(StoreKind::Ghost, folder)
    }

    fn balloon_root(&self, folder: &str) -> PathBuf {
        self.answer(StoreKind::Balloon, folder)
    }
}

fn log_head_skipped(head: &Path, root: &Path) {
    tracing::warn!(
        event = "install_head_root_skipped",
        head_root = %head.display(),
        root = %root.display(),
        "先頭の根が見つからないので、見つかるいちばん上の根へ入れる"
    );
}

#[cfg(test)]
#[path = "destination_tests.rs"]
mod tests;
