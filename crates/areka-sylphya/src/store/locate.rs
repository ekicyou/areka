//! 置き場から、記憶のファイルの名前と SQLite へ渡す `file:` の URI を作る。

use super::Access;
use std::path::Path;

/// 層の記憶のファイルの名前。
pub(crate) const DB_FILE: &str = "sylphya.sqlite";
/// 取り込む前の古い記憶のファイルの名前。
pub(crate) const LEGACY_FILE: &str = "sylphya.toml";

/// 取り込んだ古いファイルを残す名前（`n` が 0 なら番号なし）。
pub(crate) fn retired_name(n: u32) -> String {
    if n == 0 {
        format!("{LEGACY_FILE}.imported")
    } else {
        format!("{LEGACY_FILE}.imported.{n}")
    }
}

/// `path` を、絶対パスに `\\?\`（UNC は `\\?\UNC\`）を付けた `file:` の URI にする。
///
/// 素のパスでは SQLite は 260 文字を超える置き場を開けず、既定の VFS は `\\?\` を付けても
/// 約 1,040 バイトまでなので、VFS はいつも `win32-longpath` にする。SQLite はパスを UTF-8 で
/// しか受けないので、UTF-16 として正しくない名前は `InvalidFilename` で返す。
pub(crate) fn file_uri(path: &Path, access: Access) -> std::io::Result<String> {
    let absolute = std::path::absolute(path)?;
    let Some(text) = absolute.to_str() else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidFilename,
            format!(
                "置き場のパスに Unicode として正しくない名前が含まれるので、記憶のファイル {DB_FILE} を開けません: {}",
                absolute.display()
            ),
        ));
    };
    let verbatim = if text.starts_with(r"\\?\") {
        text.to_owned()
    } else if let Some(unc) = text.strip_prefix(r"\\") {
        format!(r"\\?\UNC\{unc}")
    } else {
        format!(r"\\?\{text}")
    };
    let mut uri = String::from("file:");
    for c in verbatim.chars() {
        match c {
            '%' => uri.push_str("%25"),
            '?' => uri.push_str("%3F"),
            '#' => uri.push_str("%23"),
            '\\' => uri.push_str("%5C"),
            _ => uri.push(c),
        }
    }
    uri.push_str("?vfs=win32-longpath&mode=");
    uri.push_str(mode(access));
    Ok(uri)
}

/// 名前つきのメモリーの DB（memdb）の URI。同じ名前の接続どうしで中身を分け合う。
pub(crate) fn memdb_uri(name: &str, access: Access) -> String {
    format!("file:/{name}?vfs=memdb&mode={}", mode(access))
}

fn mode(access: Access) -> &'static str {
    match access {
        Access::ReadOnly => "ro",
        Access::ReadWrite => "rw",
        Access::Create => "rwc",
    }
}

#[cfg(test)]
#[path = "locate_tests.rs"]
mod tests;
