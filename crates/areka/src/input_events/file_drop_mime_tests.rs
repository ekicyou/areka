// =============================================================================
// MIME の表の決定論テスト（areka-P0-file-drop・設計のテスト 12・要件 4.5・9.8・10.5）
// =============================================================================
//
// 表の全項目を小文字・大文字・混在で引き、表に無い拡張子・拡張子なし・空の拡張子が空文字に
// なることを判定する。表の要素数は直書き（行を足したら数も直す）。

use std::collections::HashSet;
use std::path::Path;

use super::{MIME_TABLE, mime_for};

/// 表の拡張子の数（設計「MIME の表」）。
const TABLE_LEN: usize = 38;
/// 表の MIME の種類の数。
const MIME_KINDS: usize = 33;

#[test]
fn table_has_38_extensions_and_33_mime_kinds() {
    assert_eq!(MIME_TABLE.len(), TABLE_LEN);
    let kinds: HashSet<&str> = MIME_TABLE.iter().map(|(_, m)| *m).collect();
    assert_eq!(kinds.len(), MIME_KINDS);
}

#[test]
fn table_extensions_are_lowercase_non_empty_and_unique() {
    let mut seen = HashSet::new();
    for (ext, mime) in MIME_TABLE {
        assert!(!ext.is_empty() && !mime.is_empty());
        assert_eq!(
            *ext,
            ext.to_ascii_lowercase(),
            "拡張子は小文字で持つ: {ext}"
        );
        assert!(seen.insert(*ext), "拡張子が重複: {ext}");
    }
}

/// 表の全項目を小文字・大文字・先頭だけ大文字で引いて同じ MIME。
#[test]
fn every_table_entry_resolves_case_insensitively() {
    for (ext, mime) in MIME_TABLE {
        let mut mixed = ext.to_string();
        mixed[..1].make_ascii_uppercase();
        for spelling in [ext.to_string(), ext.to_ascii_uppercase(), mixed] {
            let file = format!(r"C:\d\file.{spelling}");
            assert_eq!(mime_for(Path::new(&file)), *mime, "{file}");
        }
    }
}

#[test]
fn nar_and_zip_are_application_zip() {
    for f in ["a.nar", "a.zip", "a.NAR", "a.Zip"] {
        assert_eq!(mime_for(Path::new(f)), "application/zip", "{f}");
    }
}

#[test]
fn spot_checks_against_the_design_table() {
    for (f, m) in [
        ("a.PNG", "image/png"),
        ("a.Png", "image/png"),
        ("a.jpeg", "image/jpeg"),
        ("a.ico", "image/vnd.microsoft.icon"),
        ("a.midi", "audio/midi"),
        ("a.mkv", "video/x-matroska"),
        ("a.htm", "text/html"),
        ("a.7z", "application/x-7z-compressed"),
        ("a.dll", "application/vnd.microsoft.portable-executable"),
    ] {
        assert_eq!(mime_for(Path::new(f)), m, "{f}");
    }
}

/// 表に無い・拡張子なし・空の拡張子（`foo.`）・先頭が `.` だけの名前は空文字。
#[test]
fn unknown_missing_or_empty_extension_is_empty() {
    for f in [
        r"C:\d\a.xyz",
        r"C:\d\README",
        r"C:\d\foo.",
        r"C:\d\.png",
        "",
    ] {
        assert_eq!(mime_for(Path::new(f)), "", "{f:?}");
    }
}
