use super::*;

// banner: 共有ヘルパ（テーマ分割時は test_support へ移る）
pub(super) const LIMIT: i32 = 7;

pub(super) fn make_pair() -> (i32, i32) {
    // 文字列内の波括弧とダミー属性がトークナイザを惑わせないこと: "}"
    let s = "#[cfg(test)] mod fake {";
    assert!(!s.is_empty());
    (3, 4)
}
