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
