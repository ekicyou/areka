// fixture: 移設前の本番ファイル（Compare-RelocatedTests.ps1 の既知ケース用）
// 実ビルド対象ではない（`fixtures/` はクレート外）。

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn add_gamma_case() {
        /* ブロックコメント内の片側波括弧 { も無視されること */
        assert_eq!(add(0, 0), 0);
    }
}
