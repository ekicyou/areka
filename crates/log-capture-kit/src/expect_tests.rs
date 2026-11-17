//! `expect.rs` の自己テスト（`expect_one`・`expect_field` の「ちょうど 1 件」「欠落は失敗」の確かめ）。
//!
//! 捕捉した記録は手で組む（捕捉窓を通さない）。形は捕捉窓が組むものに合わせる——本文は
//! `record_debug` 経路で載るので `debug` は引用符の無い本文そのもの・`str_raw` は `None`。

use super::{ExpectField, expect_one};
use crate::{CapturedEvent, FieldValue};

fn event(message: &str, extra: &[(&str, &str)]) -> CapturedEvent {
    let mut fields = vec![(
        "message".to_string(),
        FieldValue {
            debug: message.to_string(),
            str_raw: None,
        },
    )];
    for (name, debug) in extra {
        fields.push((
            (*name).to_string(),
            FieldValue {
                debug: (*debug).to_string(),
                str_raw: None,
            },
        ));
    }
    CapturedEvent {
        level: tracing::Level::INFO,
        target: "t".to_string(),
        fields,
    }
}

#[test]
fn ちょうど_1_件なら返し_項目も引ける() {
    let events = vec![event("alpha", &[("n", "3")]), event("beta", &[])];
    let hit = expect_one(&events, "alpha");
    assert_eq!(hit.message(), "alpha");
    assert_eq!(hit.expect_field("n"), "3");
}

#[test]
#[should_panic(expected = "ちょうど 1 件ではない")]
fn 該当が_0_件なら失敗する() {
    let events = vec![event("alpha", &[])];
    expect_one(&events, "gamma");
}

#[test]
#[should_panic(expected = "ちょうど 1 件ではない")]
fn 該当が_2_件なら失敗する() {
    let events = vec![event("alpha 1", &[]), event("alpha 2", &[])];
    expect_one(&events, "alpha");
}

#[test]
#[should_panic(expected = "ログフィールド `n` が無い")]
fn 項目が欠けていれば失敗する() {
    let events = vec![event("alpha", &[])];
    expect_one(&events, "alpha").expect_field("n");
}
