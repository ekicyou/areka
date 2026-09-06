//! 復帰文字を落として読む／改行だけで書く（設計 D-6）。
//!
//! この repo に `.gitattributes` は無く `core.autocrlf` が効くので、新しく clone した
//! 環境では作業ツリーのテキストが復帰文字付きで取り出される。読み込みでそれを落とし、
//! 書き出しでは改行だけを書けば、環境によって本文が変わらない。要件 7.4 の
//! 「報告と台帳の突き合わせ」はこの整形の後の本文どうしで行う。
//!
//! 整形そのものは [`strip_cr`] と [`lf_body`] という `&str` → `String` の純粋な関数に
//! 切り出してある。ファイルを 1 つも作らずに整形を確かめられるのはこのためで、
//! 新クレートのテストは一時ディレクトリを使わない（設計 File Structure Plan）。
//!
//! 失敗は黙って通さない。読めない・書けないときは探したパスと理由を載せた
//! [`SurveyError::Io`] を返す（要件 6.12）。

use std::path::Path;

use crate::error::SurveyError;

/// 復帰文字を 1 つ残らず落とす。
///
/// 落とすのは復帰文字だけで、改行は 1 個も増減しない。`\r\n` は `\n` になり、
/// 単独の `\r` は消える。多バイト文字の本文はそのまま残る。
pub fn strip_cr(text: &str) -> String {
    text.replace('\r', "")
}

/// ファイルへ書き出す本文の形。改行だけを含む本文にする。
///
/// 呼び出し側が復帰文字混じりの本文を渡しても、ファイルに入るのは改行だけである。
/// 整形の中身は [`strip_cr`] と同じ——読んだ本文と書いた本文が同じ形であることが
/// 要件 1.5（2 回続けて実行して 1 バイトも違わない）の前提になる。
pub fn lf_body(body: &str) -> String {
    strip_cr(body)
}

/// ファイルを読み、復帰文字を落とした本文を返す。
///
/// UTF-8 でない本文は失敗にする。置換文字で埋めて読み進めると、正典 URL の
/// 突き合わせ（要件 6.5）が黙って外れる。
pub fn read_normalized(path: &Path) -> Result<String, SurveyError> {
    let bytes = std::fs::read(path).map_err(|err| io_error(path, &err.to_string()))?;
    let text = String::from_utf8(bytes)
        .map_err(|err| io_error(path, &format!("UTF-8 として読めない: {err}")))?;
    Ok(strip_cr(&text))
}

/// 本文を改行だけの形にしてファイルへ書く。
///
/// 置き場が実在しないときは作らずに失敗する（探したパスを載せる）。生成の入口が
/// どこへ書こうとしたかを隠さないためで、置き場を用意するのは呼び出し側の仕事である。
pub fn write_lf(path: &Path, body: &str) -> Result<(), SurveyError> {
    std::fs::write(path, lf_body(body).as_bytes()).map_err(|err| io_error(path, &err.to_string()))
}

/// 探したパスと理由を載せた読み書きの失敗。
pub(crate) fn io_error(path: &Path, reason: &str) -> SurveyError {
    SurveyError::Io {
        path: path.display().to_string(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
