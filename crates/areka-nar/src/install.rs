//! 計画のとおりに作業フォルダへ木を組み上げる（要件 4.9・5.9・6.1・6.2）。
//!
//! # なぜ宛先に直接書かないか
//!
//! 「既存を全部消してから展開する」（要件 6.2）を字義どおりに書くと、消した後で
//! 失敗したときに宛先が空のまま残る。そこで**先に作業フォルダで完成形を組み上げ、
//! 最後にフォルダ単位で入れ替える**（入れ替えはタスク 4.3）。利用者から見える結果は
//! 同じで、「全部入った」か「何も変わっていない」しか見えない（要件 5.11）。
//!
//! 本モジュールの組み上げ側は宛先を**読む**だけで、1 バイトも書かない。
//!
//! # 既存の宛先の取り込み
//!
//! 完成形は「既存の宛先の木」＋「書庫の木」の重ね合わせで、下敷きの取り方だけが
//! [`ExistingPolicy`] で変わる。`Overlay` は丸ごと、`Replace { keep }` は除外マスクに
//! 挙がったファイル名だけ（全階層・ASCII 大小無視）。
//!
//! サプリメントに `refresh,1` が書かれていても全消去にならないのは、
//! [`crate::manifest`] が `supplement` の `refresh` を読み飛ばして必ず `Overlay` を
//! 返すため（`body_existing`）。ここで種別をもう一度見ても決して効かない腕になるので
//! 見ない。兄弟テストは `refresh,1` を書いたサプリメントを**マニフェストから通して**
//! 重ね置きになることを確かめる。

use crate::error::ExistingState;
use crate::manifest::ExistingPolicy;
use crate::plan::Placement;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// 根の直下に掘る作業フォルダの棚。入れ替えが `rename` で済むよう根と同じボリュームに置く。
const WORK: &str = ".nar-work";

/// 同一プロセス内で単調増加する連番。プロセス間の一意性はプロセス識別子が担う。
static NEXT_SERIAL: AtomicU32 = AtomicU32::new(0);

/// 展開の要求。根の場所は呼び出し側が決める（要件 5.1）。
pub struct InstallRequest<'a> {
    /// ベースウェアの根（絶対パス）。
    pub root: &'a Path,
    /// `shell`／`supplement` の宛先ゴーストのフォルダ名。
    ///
    /// `accept` から宛先を求めるのは呼び出し側の仕事（要件 5.6）。
    pub target_ghost: Option<&'a str>,
}

/// どのパスで I/O に失敗したか。[`crate::NarError::Io`] の `path` と `source` になる。
///
/// 記録は公開面（タスク 4.4）が `Err` を返す直前に 1 回だけ出す。ここでは出さない
/// （二重記録を避ける＝設計「Monitoring」）。黙って握り潰す経路は 1 つも持たない。
#[derive(Debug)]
pub(crate) struct StageError {
    pub path: PathBuf,
    pub source: io::Error,
}

/// 失敗したときだけ、起きた場所のパスを添える。
fn at<T>(path: &Path, result: io::Result<T>) -> Result<T, StageError> {
    result.map_err(|source| StageError {
        path: path.to_path_buf(),
        source,
    })
}

/// 1 回の展開が使う作業フォルダ `<根>/.nar-work/<プロセス識別子>-<連番>/`。
#[derive(Debug)]
pub(crate) struct WorkArea {
    dir: PathBuf,
}

impl WorkArea {
    /// 棚の残骸を片付けてから、この展開のための作業フォルダを 1 つ作る。
    ///
    /// 残骸は前回の異常終了が置いていったもの。同じ根への同時インストールは起きない
    /// 前提（設計「Risks」）なので、棚に在る物は全て前回の置き土産として消す。
    ///
    /// # Errors
    ///
    /// 根が実在しないとき（根を勝手に作らない＝設計の事前条件）、棚の残骸を消せない
    /// とき、作業フォルダを作れないとき [`StageError`]。残骸を消せないまま続けると
    /// 前回の木が完成形に混ざるので、黙って続ける道は持たない。
    pub(crate) fn create(root: &Path) -> Result<WorkArea, StageError> {
        if !root.is_dir() {
            return Err(StageError {
                path: root.to_path_buf(),
                source: io::Error::new(io::ErrorKind::NotFound, "ベースウェアの根が実在しない"),
            });
        }
        let shelf = root.join(WORK);
        if let Err(source) = std::fs::remove_dir_all(&shelf) {
            // 棚がそもそも無いのは片付けの目的が達成された状態。
            if source.kind() != io::ErrorKind::NotFound {
                return Err(StageError {
                    path: shelf,
                    source,
                });
            }
        }
        let dir = shelf.join(format!(
            "{}-{}",
            std::process::id(),
            NEXT_SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        at(&dir, std::fs::create_dir_all(&dir))?;
        Ok(WorkArea { dir })
    }

    /// 作業フォルダそのもの。タスク 4.3 が退避先 `old-<k>` をこの下に作る。
    pub(crate) fn path(&self) -> &Path {
        &self.dir
    }

    /// `k` 番目の配置を組み上げる場所。
    pub(crate) fn stage(&self, k: usize) -> PathBuf {
        self.dir.join(k.to_string())
    }
}

/// 配置 1 つぶんの完成形を `stage` に組み上げ、宛先が確定前にどうだったかを返す。
///
/// 下敷き（既存の宛先）を敷いてから書庫の内容を上書きする。`contents` はエントリ番号で
/// 引ける伸長済みの中身で、**そのまま**書く（要件 5.9）。宛先は読むだけで触らない。
///
/// # Errors
///
/// 既存の複写・フォルダの作成・書き出しのいずれかが失敗したとき [`StageError`]。
pub(crate) fn stage_placement(
    stage: &Path,
    placement: &Placement,
    contents: &[Vec<u8>],
) -> Result<ExistingState, StageError> {
    at(stage, std::fs::create_dir_all(stage))?;

    let existing = if placement.destination.is_dir() {
        match &placement.existing {
            ExistingPolicy::Overlay => {
                copy_existing(&placement.destination, stage, None)?;
                ExistingState::Overlaid
            }
            ExistingPolicy::Replace { keep } => {
                copy_existing(&placement.destination, stage, Some(keep))?;
                ExistingState::Refreshed
            }
        }
    } else {
        ExistingState::New
    };

    // 先にフォルダを全部作る。フォルダのエントリ（要件 4.9 前段）とファイルの親
    // （同後段）が `dirs` で 1 つに揃っているので、ここは区別せずに作れる。
    for relative in &placement.dirs {
        let folder = stage.join(relative);
        at(&folder, std::fs::create_dir_all(&folder))?;
    }
    for (index, relative) in &placement.files {
        let file = stage.join(relative);
        at(&file, std::fs::write(&file, &contents[*index]))?;
    }
    Ok(existing)
}

/// 既存の宛先の木を作業フォルダへ複写する。
///
/// `keep` が `None` なら空フォルダも含めて丸ごと（要件 6.1）。`Some` なら、ファイル名が
/// 挙がっているファイルだけを**同じ相対位置**へ（要件 6.2）。後者では残すファイルを
/// 1 つも含まないフォルダは作らない——「全消去してから mask の名前だけ戻す」と同じ形。
fn copy_existing(from: &Path, to: &Path, keep: Option<&[String]>) -> Result<(), StageError> {
    for child in at(from, std::fs::read_dir(from))? {
        let child = at(from, child)?;
        let source = child.path();
        let target = to.join(child.file_name());
        if at(&source, child.file_type())?.is_dir() {
            if keep.is_none() {
                at(&target, std::fs::create_dir_all(&target))?;
            }
            copy_existing(&source, &target, keep)?;
        } else if kept(keep, &child.file_name()) {
            // 残す物が出てきて初めて親を作る（`keep` の側で空フォルダが生えない）。
            at(to, std::fs::create_dir_all(to))?;
            at(&target, std::fs::copy(&source, &target))?;
        }
    }
    Ok(())
}

/// このファイル名を下敷きに残すか。
///
/// 除外マスクは**ファイル名**の一覧で、階層を問わず同名に当たる（要件 6.2・正典
/// `descript_install#refreshundeletemask`）。突き合わせは ASCII の大小を無視する
/// （Windows では同じファイルを指す綴り）。
fn kept(keep: Option<&[String]>, name: &OsStr) -> bool {
    match keep {
        None => true,
        Some(keep) => {
            let name = name.to_string_lossy();
            keep.iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(&name))
        }
    }
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
