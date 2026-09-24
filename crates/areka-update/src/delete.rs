//! `delete.txt`／`delete[数字].txt` の安全な適用（要件 6.1〜6.7・10.7）。
//!
//! 確定の後始末なので一周を失敗にしない。拒否・種別の食い違い・読めないファイルは警告、
//! 取り除けなかった物は `undeletable` に列挙して返す（記録は `lib.rs`）。

use crate::error::{DeleteWhy, UpdateWarning};
use crate::outcome::Undeletable;
use crate::paths::{
    is_in_work_area, local_path, normalize_separators, resolves_under, unsafe_component,
};
use encoding_rs::Encoding;
use std::fs;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

/// 適用の結果。パスは全て `target.join(rel)` で組む。
#[derive(Debug)]
pub(crate) struct DeleteReport {
    pub removed: Vec<PathBuf>,
    pub undeletable: Vec<Undeletable>,
    pub warnings: Vec<UpdateWarning>,
}

/// `delete.txt`／`delete<N>.txt` の N（`delete.txt` は `None`）。当てはまらない名前は外す。
fn delete_number(name: &str) -> Option<Option<&str>> {
    let lower = name.to_ascii_lowercase();
    let digits = lower.strip_prefix("delete")?.strip_suffix(".txt")?;
    if digits.is_empty() {
        return Some(None);
    }
    digits
        .bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| Some(&name[6..6 + digits.len()]))
}

/// 対象フォルダ直下の `delete.txt` と `delete<N>.txt`（N は 10 進 1 桁以上）を、
/// `delete.txt` → N の昇順に並べる。N は桁数を問わず数値で比べる（先頭の 0 を除いた長さ → 綴り）。
pub(crate) fn delete_files(target: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(target)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(n) = delete_number(&name) else {
            continue;
        };
        let key = n.map(|d| {
            let d = d.trim_start_matches('0');
            (d.len(), d.to_owned())
        });
        found.push((key, name, entry.path()));
    }
    found.sort();
    Ok(found.into_iter().map(|(_, _, p)| p).collect())
}

/// 全ファイルを順に適用する。読めないファイルは `DeleteFileUnreadable` を警告して飛ばす
/// （削除は一周を失敗にしない＝6.6）。並べられなければ `file` を `delete*.txt` として同じ警告にする。
// ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_install.html
pub(crate) fn apply(target: &Path, target_real: &Path, charset: &'static Encoding) -> DeleteReport {
    let mut report = DeleteReport {
        removed: Vec::new(),
        undeletable: Vec::new(),
        warnings: Vec::new(),
    };
    let files = match delete_files(target) {
        Ok(files) => files,
        Err(source) => {
            report.warnings.push(UpdateWarning::DeleteFileUnreadable {
                file: "delete*.txt".to_owned(),
                source,
            });
            return report;
        }
    };
    for path in files {
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(source) => {
                report
                    .warnings
                    .push(UpdateWarning::DeleteFileUnreadable { file, source });
                continue;
            }
        };
        let (text, _, _) = charset.decode(&bytes);
        for (i, raw) in text.split('\n').enumerate() {
            let raw = raw.strip_suffix('\r').unwrap_or(raw);
            if raw.trim().is_empty() {
                continue;
            }
            apply_line(target, target_real, &file, i + 1, raw, &mut report);
        }
    }
    report
}

/// 1 行を拒否の順（絶対 → `..` → 危ない綴り → 作業場所の綴り → 実パスが外か作業場所）に掛け、通れば実体の種別を見て取り除く。
fn apply_line(
    target: &Path,
    target_real: &Path,
    file: &str,
    line: usize,
    raw: &str,
    report: &mut DeleteReport,
) {
    let norm = normalize_separators(raw);
    let is_dir_line = norm.ends_with('/');
    let rel = norm.trim_end_matches('/');
    let path = local_path(target, rel);
    let ignore = |why| UpdateWarning::DeleteLineIgnored {
        file: file.to_owned(),
        line,
        why,
    };
    if norm.starts_with('/') || norm.as_bytes().get(1) == Some(&b':') {
        return report.warnings.push(ignore(DeleteWhy::Absolute));
    }
    if rel.split('/').any(|c| c == "..") {
        return report.warnings.push(ignore(DeleteWhy::DotDot));
    }
    // Windows が綴りと違う物を開く要素（空・`.`・末尾の `.`／空白・`:`）は、対象フォルダ
    // そのもの（`. /`）や作業場所（`.update-work./`）へ化けうるので外扱い。
    if unsafe_component(rel) {
        return report.warnings.push(ignore(DeleteWhy::EscapesTarget));
    }
    // 綴りどおりの作業場所は理由の分かる `InsideWorkArea`。実パスの検査も作業場所を外扱いに
    // するので、先に見ないとこの理由に届かない。別名（8.3 短縮名・ジャンクション）は下で捕まる。
    if is_in_work_area(rel) {
        return report.warnings.push(ignore(DeleteWhy::InsideWorkArea));
    }
    match resolves_under(target_real, &path) {
        Ok(true) => {}
        Ok(false) => return report.warnings.push(ignore(DeleteWhy::EscapesTarget)),
        Err(source) => return report.undeletable.push(Undeletable { path, source }),
    }
    let is_dir = match fs::symlink_metadata(&path) {
        Ok(meta) => meta.is_dir(),
        Err(e) if e.kind() == ErrorKind::NotFound => return,
        Err(source) => return report.undeletable.push(Undeletable { path, source }),
    };
    if is_dir != is_dir_line {
        return report.warnings.push(UpdateWarning::DeleteKindMismatch {
            file: file.to_owned(),
            line,
            path,
        });
    }
    let done = if is_dir {
        fs::remove_dir_all(&path)
    } else {
        fs::remove_file(&path)
    };
    match done {
        Ok(()) => report.removed.push(path),
        Err(source) => report.undeletable.push(Undeletable { path, source }),
    }
}

#[cfg(test)]
#[path = "delete_tests.rs"]
mod tests;
