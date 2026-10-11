//! 引き金の 3 語（`talk,数値`・`runonce`・`periodic,数値`）の読みの檻
//! （areka-P0-seriko-trigger-intervals タスク 1.1・要件 1.1〜1.6・9.1）。
//!
//! 見るのは `normalize_interval` の判断の分岐だけ: 小文字の完全一致で 3 語を見分けること、
//! 数値を落とさないこと、数値が 1 以上の整数として読めないときに第 2 欄以降を `,` で繋いだ
//! 原文を `Interval::Other` へ運ぶこと。既存の語（`bind`・`random,数値`・`bind+random,数値`・
//! `always`・`sometimes`・`rarely`）の読みは兄弟の `decode_tests_*_tests.rs` が固定している
//! ので、ここでは繰り返さない。期待値はリテラル直書き。

use super::{Interval, decode};
use crate::shell::lexer::lex;

/// `animation0.interval,<rest>` の 1 行だけを持つ面を読み、その interval を返す。
fn interval_of(rest: &str) -> Interval {
    let input = format!("surface0\n{{\nanimation0.interval,{rest}\n}}\n");
    let shell = decode(lex(&input));
    assert_eq!(shell.surfaces.len(), 1);
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    shell.surfaces[0].animations[0].interval.clone()
}

/// `talk,数値`・`periodic,数値` は語と数値の両方を運ぶ（要件 1.1・1.2）。
#[test]
fn talk_and_periodic_carry_word_and_number() {
    assert_eq!(interval_of("talk,3"), Interval::Talk { n: 3 });
    assert_eq!(interval_of("periodic,5"), Interval::Periodic { secs: 5 });
}

/// `runonce` は語を運ぶ。余分な欄（`runonce,3`）は読まない（要件 1.3）。
#[test]
fn runonce_carries_word_and_ignores_extra_fields() {
    assert_eq!(interval_of("runonce"), Interval::Runonce);
    assert_eq!(interval_of("runonce,3"), Interval::Runonce);
}

/// 数値が無い `talk`／`periodic` は、第 2 欄以降の原文（＝語だけ）を「その他の語」へ運ぶ
/// （要件 1.5 の読み手の側・読み手は失敗しない）。
#[test]
fn talk_and_periodic_without_number_become_other_verbatim() {
    assert_eq!(interval_of("talk"), Interval::Other("talk".into()));
    assert_eq!(interval_of("periodic"), Interval::Other("periodic".into()));
}

/// 数値が 0 の `talk`／`periodic` は、第 2 欄以降を `,` で繋いだ原文を運ぶ（要件 1.5）。
#[test]
fn talk_and_periodic_with_zero_become_other_verbatim() {
    assert_eq!(interval_of("talk,0"), Interval::Other("talk,0".into()));
    assert_eq!(
        interval_of("periodic,0"),
        Interval::Other("periodic,0".into())
    );
}

/// 数値が非数値の `talk`／`periodic` は、第 2 欄以降を `,` で繋いだ原文を運ぶ
/// （第 4 欄以降も落とさない・要件 1.5）。
#[test]
fn talk_and_periodic_with_non_number_become_other_verbatim() {
    assert_eq!(interval_of("talk,abc"), Interval::Other("talk,abc".into()));
    assert_eq!(
        interval_of("periodic,x,7"),
        Interval::Other("periodic,x,7".into())
    );
}

/// 大文字混じりの綴りは 3 語として読まず、今までどおり語だけを運ぶ（要件 1.4）。
#[test]
fn mixed_case_spellings_stay_other_keyword_only() {
    assert_eq!(interval_of("Talk,3"), Interval::Other("Talk".into()));
    assert_eq!(interval_of("RunOnce"), Interval::Other("RunOnce".into()));
    assert_eq!(
        interval_of("PERIODIC,5"),
        Interval::Other("PERIODIC".into())
    );
}

/// `+` を含む綴りは 3 語として読まず、今までどおり語だけを運ぶ（要件 1.4）。
#[test]
fn plus_combinations_stay_other_keyword_only() {
    assert_eq!(
        interval_of("bind+runonce"),
        Interval::Other("bind+runonce".into())
    );
    assert_eq!(
        interval_of("talk+bind,2"),
        Interval::Other("talk+bind".into())
    );
    assert_eq!(
        interval_of("bind+periodic,5"),
        Interval::Other("bind+periodic".into())
    );
}
