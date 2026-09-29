//! 利用条件の本文を用意する（design「areka / install / terms」）。
//!
//! 純粋: 書庫が持つ伸長済みの中身を引いて復号し、画面に収まる上限で切るだけ。
//! 何も書かず、記録も出さない（出すのは呼び手）。

use areka_nar::NarArchive;
use areka_parsers::charset::{DefaultEncoding, decode};

/// 正典の利用条件のファイル名（並びが優先順・要件 4.2）。
const TERMS_FILES: [&str; 2] = ["terms.txt", "terms.md"];

/// 本文の上限の行数（設計で決めたこと 12）。
pub(crate) const TERMS_MAX_LINES: usize = 25;
/// 本文の上限の文字数（設計で決めたこと 12）。
pub(crate) const TERMS_MAX_CHARS: usize = 1200;

/// 出す利用条件（ファイル名と、復号して上限で切った本文）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TermsNotice {
    pub file: &'static str,
    pub body: String,
    pub clipped: bool,
}

/// 書庫の最上位の利用条件（`terms.txt` が先・無ければ `terms.md`）。無ければ None。
pub(crate) fn find_terms(archive: &NarArchive) -> Option<TermsNotice> {
    let (file, bytes) = TERMS_FILES
        .into_iter()
        .find_map(|file| archive.entry_bytes(file).map(|bytes| (file, bytes)))?;
    // `install.txt` と同じ読み方（`charset` の行か BOM に従い、無ければ Shift_JIS・要件 4.3）。
    let text = decode(bytes, DefaultEncoding::Ansi);
    let mut lines: Vec<&str> = text.lines().collect();
    // 1 行目の `charset,` の指定は本文ではない（読み方の鍵は `decode` の先読みと同じ）。
    if lines.first().is_some_and(|first| {
        first
            .split_once(',')
            .is_some_and(|(key, _)| key.trim().eq_ignore_ascii_case("charset"))
    }) {
        lines.remove(0);
    }
    let clipped_lines = lines.len() > TERMS_MAX_LINES;
    lines.truncate(TERMS_MAX_LINES);
    let mut body = lines.join("\n");
    let clipped_chars = body.chars().count() > TERMS_MAX_CHARS;
    if clipped_chars {
        body = body.chars().take(TERMS_MAX_CHARS).collect();
    }
    let clipped = clipped_lines || clipped_chars;
    if clipped {
        body.push_str(&format!("\n\n続きは書庫の中の {file} にあります"));
    }
    Some(TermsNotice {
        file,
        body,
        clipped,
    })
}

/// 同梱バルーンの取り出し元フォルダの中に在る利用条件のパス（出さずに記録するため）。
/// 取り出し元フォルダの直下だけを見る（書庫の中の一覧を引く口は `areka-nar` に無い）。
pub(crate) fn nested_terms(archive: &NarArchive) -> Vec<String> {
    archive
        .manifest()
        .companions
        .iter()
        .flat_map(|companion| {
            TERMS_FILES.map(|file| format!("{}/{file}", companion.source_directory))
        })
        .filter(|path| archive.entry_bytes(path).is_some())
        .collect()
}

#[cfg(test)]
#[path = "terms_tests.rs"]
mod terms_tests;
