//! ファイル単位の確定と逆順の戻し（要件 2.3・5.2・5.4〜5.7）。
//!
//! 作業場所の `new/<rel>` から対象フォルダへ、1 件ずつ 配下の検査 → 親フォルダの作成 →
//! 既存の退避 → 置く、の順に進め、各手の取り消しを積む。途中で失敗したら積んだ手を
//! 逆順に全部試みる。歩くのは `files` だけ＝定義に無いローカルのファイルには触らない（2.3）。
//! 記録はしない（失敗はデータで返し、`lib.rs` が記録する）。
// ponytail: 本番の呼び手（run）が付くまでの間だけ。run から呼んだら外す。
#![cfg_attr(not(test), allow(dead_code))]

use crate::error::Stuck;
use crate::paths::{local_path, resolves_under};
use crate::work::WorkArea;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

/// 確定の失敗。`path` は全て `target.join(rel)` で組む（`UpdateError::file()` が相対名を取り戻せる）。
#[derive(Debug)]
pub(crate) enum CommitFailure {
    /// 置けなかったが全部戻せた（対象フォルダは開始前と同一）。
    Write { path: PathBuf, source: io::Error },
    /// 実パスが配下に無かった（外には何も作っていない。それまでに置いた分は戻した）。
    Escapes { path: PathBuf },
    /// 戻しにも失敗した。`path`／`source` は確定を止めた失敗。
    RollbackFailed {
        path: PathBuf,
        source: io::Error,
        restored: Vec<String>,
        stuck: Vec<Stuck>,
    },
}

/// 積んだ手の取り消し。どれも `/` 区切りの相対名で持ち、パスは戻すときに組む。
enum Undo {
    /// 確定で作った親フォルダ（空なら消す）。
    RemoveDir(String),
    /// 置いた宛先。
    Remove(String),
    /// 退避した元の内容 `old/<rel>` を宛先へ戻す。
    Restore(String),
}

/// 確定を止めた理由。
enum Stop {
    Io(PathBuf, io::Error),
    Escapes(PathBuf),
}

/// `files` は `/` 区切りの相対パス（定義の順・最後は定義ファイル名）。成功で置いた一覧を返す。
pub(crate) fn commit(
    target: &Path,
    target_real: &Path,
    area: &WorkArea,
    files: &[String],
) -> Result<Vec<String>, CommitFailure> {
    let mut undo = Vec::new();
    for rel in files {
        let Err(stop) = place(target, target_real, area, rel, &mut undo) else {
            continue;
        };
        let (restored, stuck) = unwind(target, area, undo);
        return Err(match (stop, stuck.is_empty()) {
            (Stop::Io(path, source), true) => CommitFailure::Write { path, source },
            (Stop::Escapes(path), true) => CommitFailure::Escapes { path },
            (Stop::Io(path, source), false) => CommitFailure::RollbackFailed {
                path,
                source,
                restored,
                stuck,
            },
            (Stop::Escapes(path), false) => CommitFailure::RollbackFailed {
                path,
                source: io::Error::other("実パスが対象フォルダの外へ出る"),
                restored,
                stuck,
            },
        });
    }
    Ok(files.to_vec())
}

/// 1 件を最大 3 手（親を作る・退避・置く）で置き、各手の取り消しを `undo` に積む。
fn place(
    target: &Path,
    target_real: &Path,
    area: &WorkArea,
    rel: &str,
    undo: &mut Vec<Undo>,
) -> Result<(), Stop> {
    let dest = local_path(target, rel);
    let io = |e| Stop::Io(dest.clone(), e);
    // 作る前に検査する（5.6＝外へ解決されるパスを決して作らない）。
    match resolves_under(target_real, &dest) {
        Ok(true) => {}
        Ok(false) => return Err(Stop::Escapes(dest)),
        Err(e) => return Err(io(e)),
    }
    // 無い親フォルダを外側から積んでから作る（途中で失敗しても作れた分は消せる）。
    let missing: Vec<&str> = rel
        .match_indices('/')
        .map(|(i, _)| &rel[..i])
        // 無いと分かった段だけ（他の読めなさは在る扱い＝作った物と取り違えて消さない）。
        .filter(|dir| {
            fs::symlink_metadata(local_path(target, dir))
                .is_err_and(|e| e.kind() == ErrorKind::NotFound)
        })
        .collect();
    if let Some(parent) = dest.parent().filter(|_| !missing.is_empty()) {
        undo.extend(missing.iter().map(|dir| Undo::RemoveDir(dir.to_string())));
        fs::create_dir_all(parent).map_err(io)?;
    }
    // `rename` はファイル相手だと既存を置き換えるので、先に退避しないと元の内容が消える。
    if fs::symlink_metadata(&dest).is_ok() {
        let old = area.retired(rel);
        if let Some(parent) = old.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        fs::rename(&dest, &old).map_err(io)?;
        undo.push(Undo::Restore(rel.to_owned()));
    }
    fs::rename(area.fresh(rel), &dest).map_err(io)?;
    undo.push(Undo::Remove(rel.to_owned()));
    Ok(())
}

/// 積んだ手を逆順に全部試みる（途中で失敗しても続ける）。戻せたファイルと戻せなかった物を返す。
fn unwind(target: &Path, area: &WorkArea, undo: Vec<Undo>) -> (Vec<String>, Vec<Stuck>) {
    let mut touched: Vec<String> = Vec::new();
    let mut stuck: Vec<Stuck> = Vec::new();
    for step in undo.into_iter().rev() {
        let (rel, result) = match step {
            Undo::RemoveDir(dir) => {
                let r = match fs::remove_dir(local_path(target, &dir)) {
                    Err(e)
                        if matches!(
                            e.kind(),
                            ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty
                        ) =>
                    {
                        Ok(())
                    }
                    r => r,
                };
                // フォルダは戻せた一覧に数えない（戻せなければ戻せなかった一覧には載せる）。
                if let Err(source) = r {
                    stuck.push(Stuck { file: dir, source });
                }
                continue;
            }
            Undo::Remove(rel) => {
                let r = match fs::remove_file(local_path(target, &rel)) {
                    Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
                    r => r,
                };
                (rel, r)
            }
            Undo::Restore(rel) => {
                let r = fs::rename(area.retired(&rel), local_path(target, &rel));
                (rel, r)
            }
        };
        // 1 ファイル 1 件（削除と復元の両方が失敗しても、最初の失敗だけを載せる）。
        if let Err(source) = result
            && !stuck.iter().any(|s| s.file == rel)
        {
            stuck.push(Stuck {
                file: rel.clone(),
                source,
            });
        }
        if !touched.contains(&rel) {
            touched.push(rel);
        }
    }
    let restored = touched
        .into_iter()
        .filter(|rel| !stuck.iter().any(|s| &s.file == rel))
        .collect();
    (restored, stuck)
}

#[cfg(test)]
#[path = "commit_tests.rs"]
mod tests;
