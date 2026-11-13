//! 相手の欄の読みの檻（seriko-pattern-calls 要件 2.1〜2.3・9.2・8 語の見分けの檻は areka-parsers の
//! `shell/pattern_word_tests.rs`）。

use super::{PatternWord, UnreadableTarget, call_targets};

const LISTS: [PatternWord; 4] = [
    PatternWord::AlternativeStart,
    PatternWord::AlternativeStop,
    PatternWord::ParallelStart,
    PatternWord::ParallelStop,
];

fn ok(ids: &[u32]) -> Result<Box<[u32]>, UnreadableTarget> {
    Ok(ids.into())
}

/// start・stop・insert は番号 1 つを読む（要件 2.1）。
#[test]
fn single_words_read_one_number() {
    for word in [PatternWord::Start, PatternWord::Stop, PatternWord::Insert] {
        assert_eq!(call_targets(word, "3"), ok(&[3]), "{word:?}");
        assert_eq!(call_targets(word, " 12 "), ok(&[12]), "{word:?}");
        assert_eq!(
            call_targets(word, "4294967295"),
            ok(&[u32::MAX]),
            "{word:?}"
        );
    }
}

/// 番号 1 つの語で、空・名前・符号つきの数・並び・u32 を越える数は読めない（要件 2.5）。
#[test]
fn single_words_reject_unreadable_columns() {
    for word in [PatternWord::Start, PatternWord::Stop, PatternWord::Insert] {
        for target in [
            "",
            "  ",
            "abc",
            "1a",
            "-1",
            "+1",
            "(0,1)",
            "1.5",
            "4294967296",
        ] {
            assert_eq!(
                call_targets(word, target),
                Err(UnreadableTarget),
                "{word:?} {target:?}"
            );
        }
    }
}

/// 並びの 4 語は `( )`・`[ ]`・`,`・`.`・括弧なしを同じ並びとして読み、順と重複を保つ（要件 2.2・2.3）。
#[test]
fn list_words_read_every_list_form() {
    for word in LISTS {
        for target in [
            "(0,1)",
            "( 0,1 )",
            "( 0 , 1 )",
            "(0.1)",
            "[0,1]",
            "[0.1]",
            "0.1",
        ] {
            assert_eq!(
                call_targets(word, target),
                ok(&[0, 1]),
                "{word:?} {target:?}"
            );
        }
        assert_eq!(call_targets(word, "(2,0,2)"), ok(&[2, 0, 2]), "{word:?}");
        assert_eq!(call_targets(word, "(5)"), ok(&[5]), "{word:?}");
        assert_eq!(call_targets(word, "7"), ok(&[7]), "{word:?}");
    }
}

/// 並びの 4 語で、空・空の並び・空の項・名前・符号つきの数・閉じない括弧・u32 を越える数は読めない（要件 2.5）。
#[test]
fn list_words_reject_unreadable_columns() {
    for word in LISTS {
        for target in [
            "",
            "()",
            "[]",
            "(0,,1)",
            "(0,a)",
            "(0,-1)",
            "(+1)",
            "(0,1",
            "[0,1)",
            "abc",
            "(0,4294967296)",
        ] {
            assert_eq!(
                call_targets(word, target),
                Err(UnreadableTarget),
                "{word:?} {target:?}"
            );
        }
    }
}

/// Draw と Move は相手の欄を持たない。
#[test]
fn draw_and_move_have_no_targets() {
    assert_eq!(call_targets(PatternWord::Draw, "1"), Err(UnreadableTarget));
    assert_eq!(call_targets(PatternWord::Move, "1"), Err(UnreadableTarget));
}
