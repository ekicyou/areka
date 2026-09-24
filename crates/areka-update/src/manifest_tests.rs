use super::*;
use crate::testkit::{dau, sjis, txt};

const M1: &str = "0123456789abcdef0123456789abcdef";
const M2: &str = "fedcba9876543210fedcba9876543210";

/// (行番号, ローカルパス, MD5)。
fn rows(m: &Manifest) -> Vec<(usize, &str, &str)> {
    m.entries
        .iter()
        .map(|e| (e.line, e.local.as_str(), e.md5.as_str()))
        .collect()
}

fn unknown_charsets(w: &[UpdateWarning]) -> Vec<&str> {
    w.iter()
        .filter_map(|w| match w {
            UpdateWarning::UnknownCharset { name } => Some(name.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn dau_extension_fields_are_skipped() {
    let with = dau(
        &[
            &[
                "ghost/master/a.dic",
                M1,
                "size=10",
                "date=2020-01-01T00:00:00",
            ],
            &["shell/master/s0.png", M2, "x=y"],
        ],
        true,
    );
    let without = dau(
        &[&["ghost/master/a.dic", M1], &["shell/master/s0.png", M2]],
        true,
    );
    let (a, wa) = parse(ManifestName::Updates2Dau, &with);
    let (b, wb) = parse(ManifestName::Updates2Dau, &without);
    assert_eq!(
        rows(&a),
        vec![
            (1, "ghost/master/a.dic", M1),
            (2, "shell/master/s0.png", M2)
        ]
    );
    assert_eq!(rows(&a), rows(&b));
    assert!(wa.is_empty() && wb.is_empty());
    assert_eq!(a.name, ManifestName::Updates2Dau);
}

#[test]
fn dau_shift_jis_path_same_with_or_without_charset() {
    let mut with = sjis("ゴースト/辞書.dic\x01");
    with.extend_from_slice(M1.as_bytes());
    with.extend_from_slice(b"\x01size=3\x01charset=Shift_JIS\r\n");
    with.extend(sjis("シェル/表.png\x01"));
    with.extend_from_slice(M2.as_bytes());
    with.extend_from_slice(b"\r\n");

    let mut without = sjis("ゴースト/辞書.dic\x01");
    without.extend_from_slice(M1.as_bytes());
    without.extend_from_slice(b"\r\n");
    without.extend(sjis("シェル/表.png\x01"));
    without.extend_from_slice(M2.as_bytes());
    without.extend_from_slice(b"\r\n");

    let (a, wa) = parse(ManifestName::Updates2Dau, &with);
    let (b, wb) = parse(ManifestName::Updates2Dau, &without);
    assert_eq!(
        rows(&a),
        vec![(1, "ゴースト/辞書.dic", M1), (2, "シェル/表.png", M2)]
    );
    assert_eq!(rows(&a), rows(&b));
    assert_eq!(a.charset, encoding_rs::SHIFT_JIS);
    assert_eq!(b.charset, DEFAULT_CHARSET);
    assert!(wa.is_empty() && wb.is_empty());
}

#[test]
fn dau_charset_utf8_decodes_whole_file() {
    let bytes = dau(
        &[
            &["ゴースト/辞書.dic", M1, "charset=UTF-8"],
            &["シェル/表.png", M2],
        ],
        true,
    );
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(m.charset, encoding_rs::UTF_8);
    assert_eq!(
        rows(&m),
        vec![(1, "ゴースト/辞書.dic", M1), (2, "シェル/表.png", M2)]
    );
    assert!(w.is_empty());
}

#[test]
fn dau_charset_only_honoured_as_last_field_of_first_entry() {
    // 先頭エントリの途中の charset= は拡張欄（読み飛ばす）
    let mid = dau(&[&["あ.txt", M1, "charset=UTF-8", "size=1"]], true);
    let (m, _) = parse(ManifestName::Updates2Dau, &mid);
    assert_eq!(m.charset, DEFAULT_CHARSET);
    // 2 番目のエントリの charset= も無視
    let second = dau(&[&["a.txt", M1], &["b.txt", M2, "charset=UTF-8"]], true);
    let (m, w) = parse(ManifestName::Updates2Dau, &second);
    assert_eq!(m.charset, DEFAULT_CHARSET);
    assert_eq!(rows(&m), vec![(1, "a.txt", M1), (2, "b.txt", M2)]);
    assert!(w.is_empty());
}

#[test]
fn crlf_and_lf_read_the_same() {
    let lines: &[&[&str]] = &[&["a.txt", M1], &["b/c.txt", M2, "size=2"]];
    let (a, _) = parse(ManifestName::Updates2Dau, &dau(lines, true));
    let (b, _) = parse(ManifestName::Updates2Dau, &dau(lines, false));
    assert_eq!(rows(&a), vec![(1, "a.txt", M1), (2, "b/c.txt", M2)]);
    assert_eq!(rows(&a), rows(&b));
}

#[test]
fn updates_txt_three_line_kinds() {
    let bytes = txt(&[
        "charset,UTF-8",
        &format!("file,ゴースト/辞書.dic\x01{M1}\x01size=3"),
        "# 作者のメモ",
        "",
        &format!("file,b.txt\x01{M2}"),
    ]);
    let (m, w) = parse(ManifestName::UpdatesTxt, &bytes);
    assert_eq!(m.name, ManifestName::UpdatesTxt);
    assert_eq!(m.charset, encoding_rs::UTF_8);
    assert_eq!(
        rows(&m),
        vec![(2, "ゴースト/辞書.dic", M1), (5, "b.txt", M2)]
    );
    assert!(w.is_empty());
}

#[test]
fn updates_txt_without_charset_line_uses_default() {
    let mut bytes = b"file,".to_vec();
    bytes.extend(sjis("ゴースト.txt\x01"));
    bytes.extend_from_slice(M1.as_bytes());
    bytes.extend_from_slice(b"\r\n");
    let (m, w) = parse(ManifestName::UpdatesTxt, &bytes);
    assert_eq!(m.charset, DEFAULT_CHARSET);
    assert_eq!(rows(&m), vec![(1, "ゴースト.txt", M1)]);
    assert!(w.is_empty());
}

#[test]
fn unknown_charset_warns_once_and_falls_back() {
    let bytes = dau(&[&["a.txt", M1, "charset=no-such-charset"]], true);
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(m.charset, DEFAULT_CHARSET);
    assert_eq!(w.len(), 1);
    assert_eq!(unknown_charsets(&w), vec!["no-such-charset"]);
    assert_eq!(rows(&m), vec![(1, "a.txt", M1)]);

    let bytes = txt(&["charset,bogus", &format!("file,a.txt\x01{M1}")]);
    let (m, w) = parse(ManifestName::UpdatesTxt, &bytes);
    assert_eq!(m.charset, DEFAULT_CHARSET);
    assert_eq!(unknown_charsets(&w), vec!["bogus"]);
    assert_eq!(w.len(), 1);
}

#[test]
fn empty_file_has_no_entries() {
    for name in [ManifestName::Updates2Dau, ManifestName::UpdatesTxt] {
        let (m, w) = parse(name, b"");
        assert!(m.entries.is_empty());
        assert!(w.is_empty());
        assert_eq!(m.charset, DEFAULT_CHARSET);
    }
    // 改行だけ・末尾の改行の後の空も数えない
    let (m, _) = parse(ManifestName::Updates2Dau, b"\r\n\n");
    assert!(m.entries.is_empty());
}

#[test]
fn bom_wins_over_resolved_charset() {
    let mut bytes = b"\xEF\xBB\xBF".to_vec();
    bytes.extend(dau(&[&["あ.txt", M1]], true));
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(rows(&m), vec![(1, "あ.txt", M1)]);
    assert_eq!(m.charset, encoding_rs::UTF_8);
    assert!(w.is_empty());
}

/// 無効の警告（行番号・理由）を出た順に。
fn invalids(w: &[UpdateWarning]) -> Vec<(usize, InvalidWhy)> {
    w.iter()
        .filter_map(|w| match w {
            UpdateWarning::InvalidEntry { line, why } => Some((*line, *why)),
            _ => None,
        })
        .collect()
}

fn duplicates(w: &[UpdateWarning]) -> Vec<(usize, &str)> {
    w.iter()
        .filter_map(|w| match w {
            UpdateWarning::DuplicateEntry { line, path } => Some((*line, path.as_str())),
            _ => None,
        })
        .collect()
}

/// (行番号, ローカルパス, URL パス)。
fn urls(m: &Manifest) -> Vec<(usize, &str, &str)> {
    m.entries
        .iter()
        .map(|e| (e.line, e.local.as_str(), e.url_path.as_str()))
        .collect()
}

#[test]
fn nine_invalid_kinds_each_warn_once() {
    use InvalidWhy::*;
    let g32 = "g".repeat(32);
    let bytes = dau(
        &[
            &["ok.txt", M1],                   // 1 有効
            &["no-md5.txt"],                   // 2
            &["empty-md5.txt", ""],            // 3
            &["bad-md5.txt", "0123"],          // 4
            &["bad-md5-2.txt", &g32],          // 5
            &["nul\0.txt", M1],                // 6
            &["/abs.txt", M1],                 // 7
            &["\\abs.txt", M1],                // 8
            &["C:/abs.txt", M1],               // 9
            &["\\\\server\\share\\x.txt", M1], // 10
            &["dir/", M1],                     // 11
            &["dir\\", M1],                    // 12
            &["", M1],                         // 13
            &["a//b.txt", M1],                 // 14
            &["a/./b.txt", M1],                // 15
            &["../x.txt", M1],                 // 16
            &["a\\..\\x.txt", M1],             // 17 `..\` を区切りとして捕まえる
            &["UPDATES2.DAU", M1],             // 18
            &[".update-work/x.txt", M1],       // 19
            &[".Update-Work\\x.txt", M1],      // 20
            &["updates.txt", M2],              // 21 もう一方の定義ファイル名は通常のファイル
        ],
        true,
    );
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(
        invalids(&w),
        vec![
            (2, NoMd5),
            (3, NoMd5),
            (4, BadMd5),
            (5, BadMd5),
            (6, Nul),
            (7, Absolute),
            (8, Absolute),
            (9, Absolute),
            (10, Absolute),
            (11, FolderEntry),
            (12, FolderEntry),
            (13, EmptyComponent),
            (14, EmptyComponent),
            (15, EmptyComponent),
            (16, DotDot),
            (17, DotDot),
            (18, SelfReference),
            (19, InsideWorkArea),
            (20, InsideWorkArea),
        ]
    );
    assert_eq!(w.len(), 19, "無効 1 件につき警告 1 件");
    assert_eq!(rows(&m), vec![(1, "ok.txt", M1), (21, "updates.txt", M2)]);
}

#[test]
fn first_matching_reason_wins() {
    use InvalidWhy::*;
    let bytes = txt(&[
        "file,/../x/\x01bad", // BadMd5 が Absolute・DotDot・FolderEntry より先
        &format!("file,/a/../\x01{M1}"), // Absolute が FolderEntry・DotDot より先
        &format!("file,a/../\x01{M1}"), // FolderEntry が DotDot より先
        &format!("file,updates.txt/..\x01{M1}"), // DotDot
        &format!("file,Updates.txt\x01{M1}"), // 読んでいるのは updates.txt
        &format!("file,updates2.dau\x01{M1}"), // もう一方は有効
        "file,",              // 空の file, 行は NoMd5（黙って捨てない）
    ]);
    let (m, w) = parse(ManifestName::UpdatesTxt, &bytes);
    assert_eq!(
        invalids(&w),
        vec![
            (1, BadMd5),
            (2, Absolute),
            (3, FolderEntry),
            (4, DotDot),
            (5, SelfReference),
            (7, NoMd5),
        ]
    );
    assert_eq!(w.len(), 6);
    assert_eq!(rows(&m), vec![(6, "updates2.dau", M1)]);
}

#[test]
fn duplicates_later_wins_case_and_separator_insensitive() {
    let bytes = dau(
        &[
            &["A.txt", M1],
            &["b.txt", M2],
            &["a.TXT", M2],
            &["Dir\\X.txt", M1],
            &["dir/x.TXT", M2],
        ],
        true,
    );
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(
        rows(&m),
        vec![(2, "b.txt", M2), (3, "a.TXT", M2), (5, "dir/x.TXT", M2)]
    );
    assert_eq!(duplicates(&w), vec![(1, "A.txt"), (4, "Dir/X.txt")]);
    assert_eq!(w.len(), 2);
}

#[test]
fn md5_is_lowercased() {
    let upper = M1.to_uppercase();
    let bytes = dau(&[&["a.txt", &upper]], true);
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(rows(&m), vec![(1, "a.txt", M1)]);
    assert!(w.is_empty());
}

#[test]
fn all_encoded_utf8_bytes_decode_to_local() {
    // ゴ＝E3 82 B4（UTF-8）
    let bytes = dau(&[&["%E3%82%B4/a%20b.txt", M1], &["plain.txt", M2]], true);
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(
        urls(&m),
        vec![
            (1, "ゴ/a b.txt", "%E3%82%B4/a%20b.txt"),
            (2, "plain.txt", "plain.txt")
        ]
    );
    assert!(w.is_empty());
}

#[test]
fn all_encoded_shift_jis_bytes_fall_back_to_manifest_charset() {
    // UTF-8 として読めないので定義ファイルの文字コード（既定 Shift_JIS）で読む。
    let encoded: String = sjis("ゴースト")
        .iter()
        .map(|b| format!("%{b:02X}"))
        .collect();
    let path = format!("{encoded}/a.txt");
    let bytes = dau(&[&[&path, M1]], true);
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(urls(&m), vec![(1, "ゴースト/a.txt", path.as_str())]);
    assert!(w.is_empty());
}

#[test]
fn not_all_encoded_encodes_url_side_only() {
    let bytes = dau(
        &[
            &["ゴースト/a b.txt", M1, "charset=UTF-8"],
            &["a%20b.txt", M2],
            &["d\\e.txt", M1],
        ],
        true,
    );
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(
        urls(&m),
        vec![
            (
                1,
                "ゴースト/a b.txt",
                "%E3%82%B4%E3%83%BC%E3%82%B9%E3%83%88/a%20b.txt"
            ),
            (2, "a%20b.txt", "a%2520b.txt"),
            (3, "d/e.txt", "d/e.txt"),
        ]
    );
    assert!(w.is_empty());
}

#[test]
fn decoded_local_is_checked_again() {
    use InvalidWhy::*;
    let bytes = dau(
        &[
            &["%2E%2E/x.txt", M1],
            &["a%5C..%5Cx.txt", M1],
            &["n%00.txt", M1],
            &["ok%2Fy.txt", M2],
        ],
        true,
    );
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(invalids(&w), vec![(1, DotDot), (2, DotDot), (3, Nul)]);
    assert_eq!(w.len(), 3);
    assert_eq!(urls(&m), vec![(4, "ok/y.txt", "ok%2Fy.txt")]);
}

/// Windows は区切り要素の末尾の `.`・空白を落とし、`:` は NTFS のストリーム指定になる。
/// 末尾の `.`・空白は `EmptyComponent`、`:` は `Absolute`（`..` そのものは `DotDot` のまま）。
#[test]
fn windows_trailing_dot_space_and_colon_are_invalid() {
    use InvalidWhy::*;
    let bytes = dau(
        &[
            &["updates2.dau.", M1],
            &[".update-work./x", M1],
            &["a. /b", M1],
            &["a /b", M1],
            &["ab:c", M1],
            &["..", M1],
            &["ok.txt", M2],
        ],
        true,
    );
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(
        invalids(&w),
        vec![
            (1, EmptyComponent),
            (2, EmptyComponent),
            (3, EmptyComponent),
            (4, EmptyComponent),
            (5, Absolute),
            (6, DotDot),
        ]
    );
    assert_eq!(w.len(), 6);
    assert_eq!(rows(&m), vec![(7, "ok.txt", M2)]);
}

/// 復号して初めて見える末尾の空白・`.`・`:` も復号後の検査で捕まえる。
#[test]
fn decoded_trailing_dot_space_and_colon_are_invalid() {
    use InvalidWhy::*;
    let bytes = dau(
        &[
            &["a%20/b.txt", M1],
            &["updates2.dau%2E", M1],
            &["%2Eupdate-work%2E/x", M1],
            &["ab%3Ac", M1],
            &["ok.txt", M2],
        ],
        true,
    );
    let (m, w) = parse(ManifestName::Updates2Dau, &bytes);
    assert_eq!(
        invalids(&w),
        vec![
            (1, EmptyComponent),
            (2, EmptyComponent),
            (3, EmptyComponent),
            (4, Absolute),
        ]
    );
    assert_eq!(w.len(), 4);
    assert_eq!(rows(&m), vec![(5, "ok.txt", M2)]);
}
