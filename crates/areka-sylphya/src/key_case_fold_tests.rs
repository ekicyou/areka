//! 名前の畳みの解釈の段の檻（spec areka-P0-property-name-case-fold タスク 1.1・要件 5.1）。
//!
//! `parse_dotted` は各部分の名前と括弧の中の名前の半角の英字だけを小文字へ畳む。
//! 全角の英字・かな・空白は変えず、書式の誤りは小文字の同じ名前と同じ `Err` になる。

use super::*;
use crate::vocab::dotted::{DOTTED_ROOTS, GENERIC_PROP_NAMES, SET_EFFECTIVE};

fn canon(input: &str) -> String {
    parse_dotted(input).unwrap().to_canonical_string()
}

#[test]
fn segment_names_fold_ascii_letters() {
    // 1. 各部分の名前の大小違い（要件 1.1）。
    assert_eq!(canon("BASEWARE.NAME"), "baseware.name");
    assert_eq!(canon("Baseware.Name"), "baseware.name");
    assert_eq!(parse_dotted("BASEWARE.NAME"), parse_dotted("baseware.name"));
}

#[test]
fn bracket_names_fold_ascii_letters() {
    // 2. 括弧の中の名前（要件 1.2）。
    assert_eq!(canon("test(ABC).key"), "test(abc).key");
    assert_eq!(
        canon("ghostlist(えも2DEBUG).path"),
        "ghostlist(えも2debug).path"
    );
    assert_eq!(canon("SCOPE(0).X"), "scope(0).x");
}

#[test]
fn non_ascii_letters_are_not_folded() {
    // 3. 全角の英字とかなは畳まない（`to_lowercase` との取り違えを捕まえる・要件 1.3）。
    assert_eq!(canon("ＴＥＳＴ.ｋｅｙ"), "ＴＥＳＴ.ｋｅｙ");
    assert_eq!(canon("test.ア"), "test.ア");
}

#[test]
fn surrounding_spaces_are_kept() {
    // 4. 前後の空白は削らない（要件 1.4）。
    assert_eq!(canon(" BASEWARE.NAME"), " baseware.name");
    assert_eq!(canon("BASEWARE.NAME "), "baseware.name ");
}

#[test]
fn malformed_keys_give_same_err_as_lowercase() {
    // 5. 書式の誤りは大小にかかわらず小文字と同じ Err（要件 1.6）。
    assert_eq!(
        parse_dotted("index(abc)"),
        Err(KeyParseError::BadIndex { at: 0 })
    );
    assert_eq!(parse_dotted("INDEX(abc)"), parse_dotted("index(abc)"));
    assert_eq!(parse_dotted("Index(abc)"), parse_dotted("index(abc)"));
    for (upper, lower) in [("A..B", "a..b"), ("FOO(BAR", "foo(bar"), (".A", ".a")] {
        let got = parse_dotted(upper);
        assert!(got.is_err(), "{upper} must be Err");
        assert_eq!(got, parse_dotted(lower), "{upper} vs {lower}");
    }
}

#[test]
fn vocab_tables_are_lowercase_only() {
    // 6. 語彙の表 3 つは小文字だけ（畳んだ名前と表が当たる前提）。
    let all = DOTTED_ROOTS
        .iter()
        .chain(GENERIC_PROP_NAMES)
        .copied()
        .chain(SET_EFFECTIVE.iter().map(|(k, _)| *k));
    for s in all {
        assert_eq!(
            s,
            s.to_ascii_lowercase(),
            "vocab entry {s:?} must be lowercase"
        );
    }
}
