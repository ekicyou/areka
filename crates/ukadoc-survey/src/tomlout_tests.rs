//! `tomlout.rs` の在中テスト。
//!
//! ここが自前の書き出しの**較正**である（設計「境界の要点」の 1 つ目・`tomlout` 節の
//! 不変条件）。書き出しライブラリを使わずに本文を組み立てる以上、「組み立てた本文を
//! `toml` で読み戻すと元の値に一致する」ことを機械で押さえないと、逃がし漏れが黙って通る。
//!
//! ただし読み戻し一致だけでは足りない。**必要以上に逃がす**書き出し（たとえば日本語を
//! `\uXXXX` に潰す、インラインテーブルの欄を名前順に並べ替える）は読み戻し一致を完璧に
//! 満たしながら、要件付録 A.3 が凍結した書き方から 1 バイトずれ、設計 D-1 の実測
//! （1 行最大 579 文字）も破る。だから**組み上がった本文そのものの逐語一致**を主要な形
//! すべてに置く。読み戻しは「読める・値が戻る」を、逐語一致は「どの書き方か」を
//! それぞれ受け持つ。
//!
//! 守るのは 6 つ。
//!
//! 1. 逐語一致 — 素の ASCII・逆斜線・二重引用符・単引用符・日本語・空文字列・制御文字、
//!    空配列と 2 要素の配列、3 対のインラインテーブル、キー付きテーブルの見出し。
//! 2. 読み戻し一致 — 上を `toml` で読み戻すと元の Rust 文字列に戻る（設計の不変条件）。
//! 3. 1 行に収まること — 改行を含む値でも出力に生の改行が現れない（`tomlout` 節の事後条件）。
//! 4. 与えた順のまま — インラインテーブルも文字列の配列も並べ替えない（設計 D-9）。
//! 5. 決定性 — 2 回呼べば 1 バイト一致（要件 1.5）。
//! 6. 設計の見本との一致 — 設計「Data Models」のカタログ 1 行と要件付録 A.1 の
//!    テーブル見出しを、そのまま組み上げられること。
//!
//! スナップショットにもファイルにも触らない（要件 6.2）。すべて文字列だけで完結する。

use super::*;

// ---- 読み戻しの道具 ----

/// 組み上げた本文を `toml` で読み戻す。読めなければ本文ごと晒して落ちる。
fn parse_table(text: &str) -> toml::Table {
    match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(err) => panic!("読み戻せなかった: {err}\n--- 本文 ---\n{text}\n---"),
    }
}

/// `basic_string` で組んだ本文を読み戻して、戻ってきた文字列を返す。
fn round_trip_string(original: &str) -> String {
    let text = format!("v = {}", basic_string(original));
    let table = parse_table(&text);
    let value = table.get("v").expect("鍵 v が読み戻せていない");
    value
        .as_str()
        .unwrap_or_else(|| panic!("文字列として読み戻せていない: {value:?}"))
        .to_owned()
}

/// 読み戻し一致を確かめる見本（名前・値）。逆斜線と改行を必ず含む。
fn round_trip_samples() -> Vec<(&'static str, String)> {
    vec![
        ("空文字列", String::new()),
        ("素の ASCII", "dev_bind".to_owned()),
        (
            "逆斜線（さくらスクリプトのタグ・設計 D-10 の実測 316 件の形）",
            r"\![get,property,ID]".to_owned(),
        ),
        ("逆斜線 2 連", r"a\\b".to_owned()),
        ("末尾が逆斜線", r"tail\".to_owned()),
        ("二重引用符", "say \"hi\"".to_owned()),
        (
            "単引用符（逃がさない・設計 D-10 の実測 3 件）",
            "it's".to_owned(),
        ),
        ("改行", "1 行目\n2 行目".to_owned()),
        ("復帰改行", "1 行目\r\n2 行目".to_owned()),
        ("水平タブ", "左\t右".to_owned()),
        ("垂直タブ", "左\u{0b}右".to_owned()),
        ("ヌル文字", "左\u{0}右".to_owned()),
        ("制御文字の上端 U+001F", "左\u{1f}右".to_owned()),
        ("削除文字 U+007F", "左\u{7f}右".to_owned()),
        ("日本語", "ゴーストの表示位置を指定する。".to_owned()),
        (
            "日本語と逆斜線と引用符の混在",
            "見出し \"\\![raise,OnTest]\" の説明".to_owned(),
        ),
        (
            "正典 URL",
            "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#system.year:1"
                .to_owned(),
        ),
        (
            "実測の最長級の見出し（105 文字・設計 D-10）",
            "あ".repeat(105),
        ),
    ]
}

// ---- 1. 逐語一致（どの書き方かを釘付けにする）----

#[test]
fn basic_string_renders_plain_ascii_verbatim() {
    assert_eq!(basic_string("dev_bind"), r#""dev_bind""#);
}

#[test]
fn basic_string_renders_empty_as_two_quotes() {
    assert_eq!(basic_string(""), r#""""#);
}

/// 要件付録 A.3 が凍結した形。逆斜線は 2 つ重ねて書く。
#[test]
fn basic_string_renders_backslash_as_double_backslash() {
    assert_eq!(
        basic_string(r"\![get,property,ID]"),
        r#""\\![get,property,ID]""#
    );
}

#[test]
fn basic_string_renders_double_quote_escaped() {
    assert_eq!(basic_string("say \"hi\""), r#""say \"hi\"""#);
}

/// 単引用符は二重引用符の文字列の中では逃がす必要が無い（設計 D-10・実測 3 件）。
/// 逃がすと付録 A.3 の形から 1 バイトずれる。
#[test]
fn basic_string_leaves_single_quote_alone() {
    assert_eq!(basic_string("it's"), r#""it's""#);
}

/// 日本語は生のまま書く。`\uXXXX` に潰すと読み戻しは通るがカタログが読めなくなり、
/// 設計 D-1 の実測（1 行最大 579 文字）も破る。
#[test]
fn basic_string_leaves_non_ascii_alone() {
    assert_eq!(
        basic_string("ゴーストの表示位置を指定する。"),
        r#""ゴーストの表示位置を指定する。""#
    );
}

/// 制御文字は `\u00XX` に逃がす（設計 D-10）。改行も含めて一様に扱う。
#[test]
fn basic_string_renders_control_characters_as_u00xx() {
    assert_eq!(basic_string("a\nb"), r#""a\u000Ab""#);
    assert_eq!(basic_string("a\rb"), r#""a\u000Db""#);
    assert_eq!(basic_string("a\tb"), r#""a\u0009b""#);
    assert_eq!(basic_string("a\u{0}b"), r#""a\u0000b""#);
    assert_eq!(basic_string("a\u{8}b"), r#""a\u0008b""#);
    assert_eq!(basic_string("a\u{b}b"), r#""a\u000Bb""#);
    assert_eq!(basic_string("a\u{1f}b"), r#""a\u001Fb""#);
    assert_eq!(basic_string("a\u{7f}b"), r#""a\u007Fb""#);
}

#[test]
fn string_array_renders_empty_as_brackets() {
    assert_eq!(string_array(&[]), "[]");
}

#[test]
fn string_array_renders_two_elements_with_comma_and_space() {
    let values = vec!["2.3.53".to_owned(), "2.5.60".to_owned()];
    assert_eq!(string_array(&values), r#"["2.3.53", "2.5.60"]"#);
}

#[test]
fn string_array_escapes_each_element() {
    let values = vec![r"\![raise]".to_owned(), "say \"hi\"".to_owned()];
    assert_eq!(string_array(&values), r#"["\\![raise]", "say \"hi\""]"#);
}

#[test]
fn inline_table_renders_empty_as_braces() {
    assert_eq!(inline_table(&[]), "{}");
}

#[test]
fn inline_table_renders_three_pairs_verbatim() {
    let pairs = [
        ("page", basic_string("dev_bind")),
        ("versions", string_array(&[])),
        ("hash", basic_string("0000000000000000")),
    ];
    assert_eq!(
        inline_table(&pairs),
        r#"{ page = "dev_bind", versions = [], hash = "0000000000000000" }"#
    );
}

/// 要件付録 A.1 のテーブル見出しの形。
#[test]
fn keyed_table_header_renders_appendix_a_form() {
    assert_eq!(
        keyed_table_header("entry", "ukadoc:list_propertysystem:system.year:1"),
        r#"[entry."ukadoc:list_propertysystem:system.year:1"]"#
    );
}

/// 鍵に逆斜線が入っても付録 A.3 の形で書く。
#[test]
fn keyed_table_header_escapes_backslash_in_key() {
    assert_eq!(
        keyed_table_header("entry", r"ukadoc:list_sakura_script:\![get,property,ID]:1"),
        r#"[entry."ukadoc:list_sakura_script:\\![get,property,ID]:1"]"#
    );
}

// ---- 6. 設計の見本をそのまま組み上げられること ----

/// 設計「Data Models」のカタログ 1 行（`ukadoc:dev_bind`）を逐語で組み上げる。
/// これが崩れると要件 1.5 の「2 回続けて 1 バイト一致」が守る対象そのものが変わる。
#[test]
fn catalog_line_matches_the_design_sample() {
    let pairs = [
        ("page", basic_string("dev_bind")),
        ("title", basic_string("...")),
        ("category", basic_string("dev_guide")),
        ("versions", string_array(&[])),
        ("hash", basic_string("0000000000000000")),
        (
            "url",
            basic_string("https://ssp.shillest.net/ukadoc/manual/dev_bind.html"),
        ),
    ];
    let line = format!(
        "{} = {}",
        basic_string("ukadoc:dev_bind"),
        inline_table(&pairs)
    );
    assert_eq!(
        line,
        r#""ukadoc:dev_bind" = { page = "dev_bind", title = "...", category = "dev_guide", versions = [], hash = "0000000000000000", url = "https://ssp.shillest.net/ukadoc/manual/dev_bind.html" }"#
    );
}

// ---- 2. 読み戻し一致（設計「tomlout」節の不変条件）----

#[test]
fn every_sample_round_trips_through_the_toml_reader() {
    for (label, original) in round_trip_samples() {
        let got = round_trip_string(&original);
        assert_eq!(got, original, "読み戻しが元の値と違う: {label}");
    }
}

#[test]
fn string_array_round_trips_through_the_toml_reader() {
    for values in [
        Vec::new(),
        vec!["2.3.53".to_owned()],
        vec![
            r"\![get,property,ID]".to_owned(),
            "1 行目\n2 行目".to_owned(),
            "say \"hi\"".to_owned(),
            "ゴースト".to_owned(),
        ],
    ] {
        let text = format!("v = {}", string_array(&values));
        let table = parse_table(&text);
        let array = table
            .get("v")
            .and_then(|v| v.as_array())
            .expect("配列として読み戻せていない");
        let got: Vec<String> = array
            .iter()
            .map(|v| v.as_str().expect("要素が文字列でない").to_owned())
            .collect();
        assert_eq!(got, values);
    }
}

#[test]
fn inline_table_round_trips_every_value() {
    let title = "\\![get,property,ID] と \"引用\" の説明";
    let note = "1 行目\n2 行目";
    let pairs = [
        ("page", basic_string("list_sakura_script")),
        ("title", basic_string(title)),
        ("versions", string_array(&["2.3.53".to_owned()])),
        ("note", basic_string(note)),
    ];
    let text = format!("v = {}", inline_table(&pairs));
    let table = parse_table(&text);
    let inline = table
        .get("v")
        .and_then(|v| v.as_table())
        .expect("インラインテーブルとして読み戻せていない");
    assert_eq!(
        inline.get("page").and_then(|v| v.as_str()),
        Some("list_sakura_script")
    );
    assert_eq!(inline.get("title").and_then(|v| v.as_str()), Some(title));
    assert_eq!(inline.get("note").and_then(|v| v.as_str()), Some(note));
    let versions = inline
        .get("versions")
        .and_then(|v| v.as_array())
        .expect("versions が配列でない");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].as_str(), Some("2.3.53"));
}

/// 逆斜線を含む id を鍵にしても、`toml` が同じ綴りの鍵として読み戻す。
#[test]
fn keyed_table_header_round_trips_as_a_table_key() {
    let id = r"ukadoc:list_sakura_script:\![get,property,ID]:1";
    let text = format!(
        "{}\nstatus = {}\n",
        keyed_table_header("entry", id),
        basic_string("unclassified")
    );
    let table = parse_table(&text);
    let entry = table
        .get("entry")
        .and_then(|v| v.as_table())
        .expect("entry テーブルが無い");
    let item = entry.get(id).and_then(|v| v.as_table()).unwrap_or_else(|| {
        let keys: Vec<&String> = entry.keys().collect();
        panic!("鍵 {id:?} が読み戻せていない: {keys:?}")
    });
    assert_eq!(
        item.get("status").and_then(|v| v.as_str()),
        Some("unclassified")
    );
}

// ---- 3. 1 行に収まること（設計「tomlout」節の事後条件）----

#[test]
fn output_never_contains_a_raw_line_break() {
    for (label, original) in round_trip_samples() {
        let rendered = basic_string(&original);
        assert!(
            !rendered.contains('\n'),
            "生の改行が出力に残っている: {label} → {rendered:?}"
        );
        assert!(
            !rendered.contains('\r'),
            "生の復帰が出力に残っている: {label} → {rendered:?}"
        );
    }
    let values = vec!["1 行目\n2 行目".to_owned()];
    assert!(!string_array(&values).contains('\n'));
    let pairs = [("note", basic_string("1 行目\n2 行目"))];
    assert!(!inline_table(&pairs).contains('\n'));
    assert!(!keyed_table_header("entry", "a\nb").contains('\n'));
}

// ---- 4. 与えた順のまま（設計 D-9）----

/// 欄の並びはカタログの列順で凍結されている。名前順に並べ替えても読み戻すと同じ表に
/// なるので、逐語一致でしか捕まえられない。
#[test]
fn inline_table_keeps_the_given_order_and_does_not_sort() {
    let pairs = [
        ("page", basic_string("p")),
        ("title", basic_string("t")),
        ("category", basic_string("c")),
        ("hash", basic_string("h")),
    ];
    assert_eq!(
        inline_table(&pairs),
        r#"{ page = "p", title = "t", category = "c", hash = "h" }"#
    );
}

/// 配列も並べ替えない（昇順に整えるのは呼ぶ側の仕事）。
#[test]
fn string_array_keeps_the_given_order_and_does_not_sort() {
    let values = vec!["2.5.60".to_owned(), "2.3.53".to_owned()];
    assert_eq!(string_array(&values), r#"["2.5.60", "2.3.53"]"#);
}

// ---- 5. 決定性（要件 1.5）----

#[test]
fn every_function_is_byte_identical_on_a_second_call() {
    for (label, original) in round_trip_samples() {
        assert_eq!(
            basic_string(&original),
            basic_string(&original),
            "basic_string が 2 回で違う: {label}"
        );
    }
    let values = vec![r"\a".to_owned(), "b".to_owned()];
    assert_eq!(string_array(&values), string_array(&values));
    let pairs = [
        ("page", basic_string("p")),
        ("versions", string_array(&values)),
    ];
    assert_eq!(inline_table(&pairs), inline_table(&pairs));
    assert_eq!(
        keyed_table_header("entry", r"ukadoc:a\b"),
        keyed_table_header("entry", r"ukadoc:a\b")
    );
}
