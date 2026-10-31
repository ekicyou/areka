//! 8 語の見分けと相手の欄の読みの檻（seriko-pattern-calls 要件 1.2・2.1〜2.3・9.2）。

use super::{PatternWord, UnreadableTarget, call_targets, pattern_word};
use crate::method::is_move_name;
use crate::world::targets_animation_id;

const LISTS: [PatternWord; 4] = [
    PatternWord::AlternativeStart,
    PatternWord::AlternativeStop,
    PatternWord::ParallelStart,
    PatternWord::ParallelStop,
];

fn ok(ids: &[u32]) -> Result<Box<[u32]>, UnreadableTarget> {
    Ok(ids.into())
}

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

/// 今の 2 つの判定は照会への薄い呼び出しで、答えは 7 語と move に揃う。
#[test]
fn legacy_checks_follow_the_query() {
    for method in [
        "start",
        "Stop",
        "alternative-start",
        "alternativestop",
        "parallel_start",
        "parallelstop",
        "INSERT",
    ] {
        assert!(targets_animation_id(method), "{method}");
        assert!(!is_move_name(method), "{method}");
    }
    assert!(!targets_animation_id(" Move "));
    assert!(is_move_name(" Move "));
    assert!(!targets_animation_id("overlay"));
    assert!(!is_move_name("overlay"));
}

/// start・stop・insert は番号 1 つを読む（要件 2.1）。
#[test]
fn single_words_read_one_number() {
    for word in [PatternWord::Start, PatternWord::Stop, PatternWord::Insert] {
        assert_eq!(call_targets(word, "3"), ok(&[3]), "{word:?}");
        assert_eq!(call_targets(word, " 12 "), ok(&[12]), "{word:?}");
    }
}

/// 番号 1 つの語で、空・名前・負の数・並びは読めない（要件 2.5）。
#[test]
fn single_words_reject_unreadable_columns() {
    for word in [PatternWord::Start, PatternWord::Stop, PatternWord::Insert] {
        for target in ["", "  ", "abc", "1a", "-1", "(0,1)", "1.5"] {
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

/// 並びの 4 語で、空・空の並び・空の項・名前・負の数・閉じない括弧は読めない（要件 2.5）。
#[test]
fn list_words_reject_unreadable_columns() {
    for word in LISTS {
        for target in [
            "", "()", "[]", "(0,,1)", "(0,a)", "(0,-1)", "(0,1", "[0,1)", "abc",
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
