use super::*;

// banner: 共有ヘルパ（テーマ分割時は test_support へ移る）
const LIMIT: i32 = 7;

fn make_pair() -> (i32, i32) {
    // 文字列内の波括弧とダミー属性がトークナイザを惑わせないこと: "}"
    let s = "#[cfg(test)] mod fake {";
    assert!(!s.is_empty());
    (3, 4)
}

#[test]
fn add_alpha_case() {
    let (a, b) = make_pair();
    assert_eq!(add(a, b), 7);
}

#[test]
fn add_beta_case() {
    assert_eq!(add(LIMIT, 1), 8);
}
