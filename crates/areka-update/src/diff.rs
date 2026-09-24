//! 要取得の選別（要件 2.1・2.3・2.4・2.6）。定義の側だけを定義の順に歩く。

use crate::manifest::Manifest;
use crate::md5::md5_hex;
use crate::paths::{local_path, resolves_under};
use std::fs;
use std::io::{self, ErrorKind, Read};
use std::path::{Path, PathBuf};

/// ローカルの読みの上限（`md5_hex` の前提 `bytes.len() <= u32::MAX`）。
pub(crate) const MAX_LOCAL_BYTES: u64 = u32::MAX as u64;

#[derive(Debug)]
pub(crate) enum DiffFailure {
    /// 在るのに読めない（権限・同名のフォルダ・4 GiB 超）。
    Unreadable { path: PathBuf, source: io::Error },
    /// 実パスが対象フォルダの外へ解決される（ジャンクション・リンク）。
    Escapes { path: PathBuf },
}

/// 各エントリを定義の順に見て、無い／MD5 が違う → 要取得（`entries` の添字）、同じ → 対象外。
/// 定義に無いローカルのファイルは走査しない（2.3＝定義の側だけを歩く）。
/// 失敗のパスは `target.join(rel)` で組む（`UpdateError::file()` の綴り）。
pub(crate) fn plan(
    target: &Path,
    target_real: &Path,
    m: &Manifest,
) -> Result<Vec<usize>, DiffFailure> {
    let mut need = Vec::new();
    for (i, entry) in m.entries.iter().enumerate() {
        let path = local_path(target, &entry.local);
        let unreadable = |source| DiffFailure::Unreadable {
            path: path.clone(),
            source,
        };
        if !resolves_under(target_real, &path).map_err(unreadable)? {
            return Err(DiffFailure::Escapes { path: path.clone() });
        }
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound => need.push(i),
            Err(e) => return Err(unreadable(e)),
            Ok(_) => {
                if md5_hex(&read_limited(&path).map_err(unreadable)?) != entry.md5 {
                    need.push(i);
                }
            }
        }
    }
    Ok(need)
}

/// 1 つのハンドルで大きさを確かめてから読む（大きさと中身の食い違いを作らない）。
fn read_limited(path: &Path) -> io::Result<Vec<u8>> {
    let file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    check_size(len, MAX_LOCAL_BYTES)?;
    let mut bytes = Vec::with_capacity(len as usize);
    // 読んでいる間に伸びても上限を越えては読まない。
    file.take(MAX_LOCAL_BYTES + 1).read_to_end(&mut bytes)?;
    check_size(bytes.len() as u64, MAX_LOCAL_BYTES)?;
    Ok(bytes)
}

/// `len` が `limit` 以下なら通す。超えたら `FileTooLarge`（ハッシュの前に失敗にする）。
fn check_size(len: u64, limit: u64) -> io::Result<()> {
    if len > limit {
        return Err(io::Error::new(
            ErrorKind::FileTooLarge,
            format!("{len} バイトは上限 {limit} バイトを超える"),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
