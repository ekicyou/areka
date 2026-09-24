//! 作業場所（要件 4.1・4.7・4.8・5.5）。
//!
//! `<対象>/.update-work/<プロセス識別子>-<連番>/` に、落とした物（`new/<相対パス>`）と
//! 退避した元の内容（`old/<相対パス>`）を置く。`areka-nar` の `.nar-work` と同じ置き方。
//! 記録はしない（残骸はデータで返し、`lib.rs` が `warn!` に写す）。

use crate::paths::WORK_DIR;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// 同一プロセス内で単調増加する連番。プロセス間の一意性はプロセス識別子が担う。
static NEXT_SERIAL: AtomicU32 = AtomicU32::new(0);

/// 失敗は `(触ったパス, io::Error)`。
type Failed = (PathBuf, io::Error);

fn at<T>(path: &Path, result: io::Result<T>) -> Result<T, Failed> {
    result.map_err(|e| (path.to_path_buf(), e))
}

pub(crate) struct WorkArea {
    dir: PathBuf,
    /// 棚に残っていて消さなかった・消せなかった他の走行の物。
    residue: Vec<PathBuf>,
}

impl WorkArea {
    /// 棚の他の走行の残骸を先に消し（消せなければ `residue`）、自分のフォルダを作る。
    /// `old/` に中身が残るフォルダ（戻せなかった走行の元の内容＝5.5）は消さずに `residue` へ。
    /// 自分の名前のフォルダが既に在れば（プロセス識別子の再利用）作らずに失敗する。
    pub(crate) fn create(target: &Path) -> Result<WorkArea, Failed> {
        let shelf = target.join(WORK_DIR);
        let dir = shelf.join(format!(
            "{}-{}",
            std::process::id(),
            NEXT_SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let residue = sweep(&shelf)?;
        at(&shelf, fs::create_dir_all(&shelf))?;
        at(&dir, fs::create_dir(&dir))?;
        Ok(WorkArea { dir, residue })
    }

    #[cfg(test)]
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    /// 落とした物の置き場 `new/<rel>`。
    pub(crate) fn fresh(&self, rel: &str) -> PathBuf {
        self.dir.join("new").join(rel)
    }

    /// 退避した元の内容の置き場 `old/<rel>`。
    pub(crate) fn retired(&self, rel: &str) -> PathBuf {
        self.dir.join("old").join(rel)
    }

    /// `new/<rel>` の親を作って書く。バイト列は変換しない（4.7）。
    pub(crate) fn put(&self, rel: &str, bytes: &[u8]) -> Result<(), Failed> {
        let path = self.fresh(rel);
        if let Some(parent) = path.parent() {
            at(parent, fs::create_dir_all(parent))?;
        }
        at(&path, fs::write(&path, bytes))
    }

    /// 自分のフォルダを消し、空なら棚も消す。消せなかった物と `residue` を返す（4.8）。
    pub(crate) fn cleanup(self) -> Vec<PathBuf> {
        let mut leftovers = self.residue;
        if let Err(e) = fs::remove_dir_all(&self.dir)
            && e.kind() != ErrorKind::NotFound
        {
            leftovers.push(self.dir.clone());
        }
        let shelf = self.dir.parent().expect("作業場所は棚の下");
        match fs::remove_dir(shelf) {
            Ok(()) => {}
            Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty) => {}
            Err(_) => leftovers.push(shelf.to_path_buf()),
        }
        leftovers
    }

    /// 戻せなかったとき: 片付けずに自分のフォルダと `residue` を返す（5.5）。
    pub(crate) fn keep(self) -> (PathBuf, Vec<PathBuf>) {
        (self.dir, self.residue)
    }
}

/// 棚の中身を消す。消せなかった物と、`old/` に中身が残るので消さなかった物を返す。
fn sweep(shelf: &Path) -> Result<Vec<PathBuf>, Failed> {
    let entries = match fs::read_dir(shelf) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err((shelf.to_path_buf(), e)),
    };
    let mut residue = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            // 何が居るのかすら読めなかった。棚ごと人の目に出す。
            residue.push(shelf.to_path_buf());
            continue;
        };
        let path = entry.path();
        let removed = match entry.file_type() {
            Ok(kind) if kind.is_dir() => {
                if has_content(&path.join("old")) {
                    residue.push(path);
                    continue;
                }
                fs::remove_dir_all(&path)
            }
            Ok(_) => fs::remove_file(&path),
            Err(e) => Err(e),
        };
        if removed.is_err_and(|e| e.kind() != ErrorKind::NotFound) {
            residue.push(path);
        }
    }
    Ok(residue)
}

fn has_content(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
}

#[cfg(test)]
#[path = "work_tests.rs"]
mod tests;
