//! 8 語の見分けと綴りの比べ方の檻（seriko-pattern-calls 要件 1.2・1.5）。

use super::pattern_word::{PatternWord, canonical_method_name, pattern_word, same_canonical};

/// 8 語を大文字・`-`・`_`・前後の空白を問わずに見分ける（要件 1.2）。
#[test]
fn words_are_compared_like_compose_methods() {
    let cases = [
        ("start", PatternWord::Start),
        (" STOP ", PatternWord::Stop),
        ("Alternative-Start", PatternWord::AlternativeStart),
        ("alternative_stop", PatternWord::AlternativeStop),
        ("PARALLEL-START", PatternWord::ParallelStart),
        ("parallel_Stop", PatternWord::ParallelStop),
        ("\tInsert", PatternWord::Insert),
        ("Mo-ve", PatternWord::Move),
    ];
    for (method, want) in cases {
        assert_eq!(pattern_word(method), want, "{method:?}");
    }
}

/// 8 語以外（重ね方の語・未知の語・空）は Draw（要件 1.5）。
#[test]
fn other_words_are_draw() {
    for method in [
        "overlay", "bind", "add", "base", "reduce", "starts", "", "blink",
    ] {
        assert_eq!(pattern_word(method), PatternWord::Draw, "{method:?}");
    }
}

/// 確保しない `same_canonical(n, c)` は `canonical_method_name(n) == c` と同じ答え（2 つの比べ方は同じ規則）。
#[test]
fn same_canonical_matches_canonical_method_name() {
    let names = [
        "overlay",
        "OVERLAY",
        " Over-Lay ",
        "o_v-e_rlay",
        "\tStart\n",
        "Alternative-Start",
        "parallel_Stop",
        "overla",
        "overlayy",
        "",
        "  ",
        "-_-",
        "ＯＶＥＲＬＡＹ",
        "bïnd",
        "BÏND",
    ];
    let canons = [
        "overlay",
        "start",
        "alternativestart",
        "parallelstop",
        "",
        "ＯＶＥＲＬＡＹ",
        "bïnd",
        "bÏnd",
    ];
    for name in names {
        for canon in canons {
            assert_eq!(
                same_canonical(name, canon),
                canonical_method_name(name) == canon,
                "{name:?} {canon:?}"
            );
        }
    }
}
