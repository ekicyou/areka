//! 利用条件の本文のテスト（design「Testing Strategy / 判断（find_terms）」・要件 4.1〜4.3・4.9〜4.11・12.15）。
//!
//! 書庫は `sample_ghost_kit` の `nar_writer` で組み、ワークツリーの `target\` の下の
//! 作業フォルダ（`WorkDir`）に置いて本物の `NarArchive::open` で開く。OS の一時フォルダは
//! 使わない（`areka-P0-install-companion-reading` 要件 8.5）。

use sample_ghost_kit::{NarBuilder, WorkDir, install_txt};

use super::*;

/// 「利用条件です」の Shift_JIS。
const SJIS_TERMS: &[u8] = &[
    0x97, 0x98, 0x97, 0x70, 0x8f, 0xf0, 0x8c, 0x8f, 0x82, 0xc5, 0x82, 0xb7,
];

/// ゴースト 1 本（同梱バルーン `kaku` 付き）の骨組み。利用条件のファイルは足す側が選ぶ。
fn ghost() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "type,ghost",
                "name,テスト",
                "directory,tester",
                "balloon.directory,kaku",
                "balloon.source.directory,kaku",
            ]),
        )
        .done()
        .file("ghost/master/descript.txt", b"charset,Shift_JIS\r\n")
        .done()
        .file("kaku/descript.txt", b"charset,Shift_JIS\r\n")
        .done()
}

fn open(builder: NarBuilder) -> NarArchive {
    let work = WorkDir::new().expect("作業フォルダを取れる");
    let path = work.path().join("ghost.nar");
    std::fs::write(&path, builder.bytes()).expect("固定入力を置ける");
    NarArchive::open(&path).expect("無傷の書庫は開ける")
}

fn with(files: &[(&str, &[u8])]) -> NarArchive {
    let mut builder = ghost();
    for (name, data) in files {
        builder = builder.file(*name, data).done();
    }
    open(builder)
}

fn body(file: &'static str, body: &str) -> Option<TermsNotice> {
    Some(TermsNotice {
        file,
        body: body.to_owned(),
        clipped: false,
    })
}

#[test]
fn no_terms_file_is_none() {
    let archive = with(&[]);
    assert_eq!(find_terms(&archive), None, "無ければ画面を出さない（4.11）");
    assert!(nested_terms(&archive).is_empty());
}

#[test]
fn terms_txt_only() {
    let archive = with(&[("terms.txt", b"txt terms\r\nline 2\r\n")]);
    assert_eq!(find_terms(&archive), body("terms.txt", "txt terms\nline 2"));
}

#[test]
fn terms_md_only_is_shown_without_interpreting_markdown() {
    let archive = with(&[("terms.md", b"# Title\r\n**bold**\r\n")]);
    assert_eq!(
        find_terms(&archive),
        body("terms.md", "# Title\n**bold**"),
        "Markdown の装飾は本文のまま（4.3）"
    );
}

#[test]
fn both_files_prefer_terms_txt() {
    let archive = with(&[("terms.md", b"md\r\n"), ("terms.txt", b"txt\r\n")]);
    assert_eq!(find_terms(&archive), body("terms.txt", "txt"), "4.2");
}

#[test]
fn upper_case_name_is_found() {
    let archive = with(&[("TERMS.TXT", b"upper\r\n")]);
    assert_eq!(find_terms(&archive), body("terms.txt", "upper"));
}

#[test]
fn without_charset_line_reads_shift_jis() {
    let archive = with(&[("terms.txt", SJIS_TERMS)]);
    assert_eq!(find_terms(&archive), body("terms.txt", "利用条件です"));
}

#[test]
fn charset_first_line_is_followed_and_not_shown() {
    let archive = with(&[("terms.txt", "charset,UTF-8\r\n利用条件です\r\n".as_bytes())]);
    assert_eq!(find_terms(&archive), body("terms.txt", "利用条件です"));
}

#[test]
fn utf8_bom_is_followed() {
    let bytes = [&[0xEF, 0xBB, 0xBF][..], "利用条件です\r\n".as_bytes()].concat();
    let archive = with(&[("terms.txt", &bytes)]);
    assert_eq!(find_terms(&archive), body("terms.txt", "利用条件です"));
}

#[test]
fn twenty_five_lines_are_not_clipped() {
    let text: String = (1..=TERMS_MAX_LINES).map(|n| format!("{n}\r\n")).collect();
    let archive = with(&[("terms.txt", text.as_bytes())]);
    let notice = find_terms(&archive).expect("在る");
    assert!(!notice.clipped);
    assert_eq!(notice.body.lines().count(), TERMS_MAX_LINES);
}

#[test]
fn twenty_six_lines_are_clipped_with_a_pointer_to_the_file() {
    let text: String = (1..=TERMS_MAX_LINES + 1)
        .map(|n| format!("{n}\r\n"))
        .collect();
    let archive = with(&[("terms.txt", text.as_bytes())]);
    let notice = find_terms(&archive).expect("在る");
    assert!(notice.clipped, "4.9");
    let kept: Vec<String> = (1..=TERMS_MAX_LINES).map(|n| n.to_string()).collect();
    assert_eq!(
        notice.body,
        format!(
            "{}\n\n続きは書庫の中の terms.txt にあります",
            kept.join("\n")
        )
    );
}

#[test]
fn over_1200_chars_are_clipped_with_a_pointer_to_the_file() {
    let text = "あ".repeat(TERMS_MAX_CHARS + 1);
    let archive = with(&[("terms.md", format!("charset,UTF-8\r\n{text}").as_bytes())]);
    let notice = find_terms(&archive).expect("在る");
    assert!(notice.clipped, "4.9");
    assert_eq!(
        notice.body,
        format!(
            "{}\n\n続きは書庫の中の terms.md にあります",
            "あ".repeat(TERMS_MAX_CHARS)
        )
    );
}

#[test]
fn exactly_1200_chars_are_not_clipped() {
    let text = "あ".repeat(TERMS_MAX_CHARS);
    let archive = with(&[("terms.txt", format!("charset,UTF-8\r\n{text}").as_bytes())]);
    assert_eq!(find_terms(&archive), body("terms.txt", &text));
}

#[test]
fn clipping_never_splits_an_emoji_cluster() {
    let family = "👨\u{200d}👩\u{200d}👧";
    let text = format!("{}{family}{family}", "あ".repeat(TERMS_MAX_CHARS - 1));
    let archive = with(&[("terms.txt", format!("charset,UTF-8\r\n{text}").as_bytes())]);
    let notice = find_terms(&archive).expect("在る");
    assert!(notice.clipped);
    assert_eq!(
        notice.body,
        format!(
            "{}{family}\n\n続きは書庫の中の terms.txt にあります",
            "あ".repeat(TERMS_MAX_CHARS - 1)
        )
    );
}

#[test]
fn terms_inside_the_companion_balloon_only_are_not_shown_but_listed() {
    let archive = with(&[
        ("kaku/terms.txt", b"balloon\r\n"),
        ("kaku/terms.md", b"md\r\n"),
    ]);
    assert_eq!(
        find_terms(&archive),
        None,
        "同梱バルーンの中は出さない（4.10）"
    );
    assert_eq!(
        nested_terms(&archive),
        vec!["kaku/terms.txt".to_owned(), "kaku/terms.md".to_owned()]
    );
}

#[test]
fn top_level_terms_are_shown_and_nested_ones_are_listed() {
    let archive = with(&[("terms.txt", b"top\r\n"), ("kaku/terms.md", b"md\r\n")]);
    assert_eq!(find_terms(&archive), body("terms.txt", "top"));
    assert_eq!(nested_terms(&archive), vec!["kaku/terms.md".to_owned()]);
}

#[test]
fn terms_directly_under_a_nested_source_are_listed_but_not_its_parent() {
    // `\` で書いた階層付きの取り出し元は `/` 区切りで読まれ、その直下だけを見る
    // （`areka-P0-install-companion-reading` 要件 2.7・8.4）。
    let archive = open(
        NarBuilder::new()
            .file(
                "install.txt",
                &install_txt(&[
                    "type,ghost",
                    "name,テスト",
                    "directory,tester",
                    "balloon.directory,kaku",
                    "balloon.source.directory,extra\\bal1",
                ]),
            )
            .done()
            .file("ghost/master/descript.txt", b"charset,Shift_JIS\r\n")
            .done()
            .file("extra/bal1/descript.txt", b"charset,Shift_JIS\r\n")
            .done()
            .file("extra/bal1/terms.txt", b"balloon\r\n")
            .done()
            .file("extra/terms.txt", b"parent\r\n")
            .done(),
    );
    assert_eq!(find_terms(&archive), None, "最上位には無い");
    assert_eq!(
        nested_terms(&archive),
        vec!["extra/bal1/terms.txt".to_owned()],
        "取り出し元の直下だけ・途中のフォルダは見ない"
    );
}
