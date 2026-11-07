//! 呼出側の判定を短く書くための補助（[`expect_one`]・[`ExpectField`]）。
//!
//! 旧 `placement/test_support.rs` から中身を変えずに移した（spec: placement-crate-split 要件 3.1）。

use crate::CapturedEvent;

/// 構造化フィールドの Debug 表現を引き、欠落を失敗として扱う（フィールド名も契約のうち）。
///
/// 正準型は同名の固有メソッド `field()` を `Option<&str>` で持つ。固有メソッドは拡張
/// トレイトより先に解決されるため、この補助は**別名**でなければ黙って無視される。
pub trait ExpectField {
    /// 構造化フィールドの Debug 表現。欠落は失敗。
    fn expect_field(&self, name: &str) -> &str;
}

impl ExpectField for CapturedEvent {
    fn expect_field(&self, name: &str) -> &str {
        self.field(name)
            .unwrap_or_else(|| panic!("ログフィールド `{name}` が無い: {:?}", self.fields_map()))
    }
}

/// メッセージに `needle` を含むイベントが**ちょうど 1 件**在ることを主張して返す。
pub fn expect_one<'a>(events: &'a [CapturedEvent], needle: &str) -> &'a CapturedEvent {
    let hits: Vec<&CapturedEvent> = events
        .iter()
        .filter(|e| e.message().contains(needle))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "`{needle}` を含むログがちょうど 1 件ではない: {events:?}"
    );
    hits[0]
}
