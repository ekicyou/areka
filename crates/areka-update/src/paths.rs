//! 定義の相対パスとローカルパスの橋渡し（要件 1.13・5.6・6.3）。
//! 読み手・差分・確定・削除・作業場所が借りる。`manifest`・`md5` は見ない。

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// 作業場所（`<対象>/.update-work/`）の名前。
pub(crate) const WORK_DIR: &str = ".update-work";

/// `\` を `/` に揃える（1.13・`..\` を `..` の要素として捕まえるため）。
pub(crate) fn normalize_separators(s: &str) -> String {
    s.replace('\\', "/")
}

/// `/` 区切りの相対パスを対象フォルダ配下のパスにする（`\` は既に `/` に正規化済み）。
/// 部品ごとに組み直さず、そのまま継ぐ（`UpdateError::file()` の綴りと揃える）。
pub(crate) fn local_path(target: &Path, rel: &str) -> PathBuf {
    target.join(rel)
}

/// 対象フォルダの配下で、かつ作業場所の外に実パスが解決されるか。
/// `candidate` の最も深い実在する祖先を `canonicalize` して判定する。ジャンクションや
/// シンボリックリンクで外へ解決されるパスを捕まえる（5.6・6.3）。作業場所の除外も実パスで見る:
/// 8.3 短縮名（`UPDATE~1`）や作業場所を指すジャンクションは綴りの検査（`is_in_work_area`）を抜けるため。
/// 在るのに実パスを確かめられない祖先（行き先の消えたリンク）は外扱い（安全側）。
pub(crate) fn resolves_under(target_real: &Path, candidate: &Path) -> std::io::Result<bool> {
    for a in candidate.ancestors() {
        match std::fs::symlink_metadata(a) {
            Ok(_) => {
                return match std::fs::canonicalize(a) {
                    Ok(real) => Ok(real.strip_prefix(target_real).is_ok_and(|rest| {
                        rest.components()
                            .next()
                            .is_none_or(|first| !first.as_os_str().eq_ignore_ascii_case(WORK_DIR))
                    })),
                    Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
                    Err(e) => Err(e),
                };
            }
            Err(e) if e.kind() == ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(false)
}

/// `/` 区切りの相対パスに、Windows が綴りと違う物を開く区切り要素があるか。
/// 空・`.`・末尾が `.` か空白（Win32 が落とすので `. ` は `.`、`.update-work.` は `.update-work`
/// になる）・`:` を含む（NTFS のストリーム指定）。`..` そのものは数えない（呼び手が `DotDot` で先に拒む）。
pub(crate) fn unsafe_component(rel: &str) -> bool {
    rel.split('/').any(|c| {
        c.is_empty() || c.contains(':') || (c != ".." && (c.ends_with('.') || c.ends_with(' ')))
    })
}

/// 先頭の区切り要素が `WORK_DIR`（大小無視）か。`\` も区切りとして見る。
pub(crate) fn is_in_work_area(rel: &str) -> bool {
    rel.split(['/', '\\'])
        .next()
        .is_some_and(|first| first.eq_ignore_ascii_case(WORK_DIR))
}

#[cfg(test)]
#[path = "paths_tests.rs"]
mod tests;
