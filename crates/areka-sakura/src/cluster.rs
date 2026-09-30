//! 書記素クラスタの切り方（cluster）— 台詞の文字列を「人が 1 文字と見る単位」へ切る、
//! areka で唯一の定義点。
//!
//! 規則は Unicode の拡張書記素クラスタ（UAX #29）だけで、`unicode-segmentation` の
//! `graphemes(true)` を包むだけにする。ZWJ 列・国旗・肌色・異体字セレクタ・キーキャップ・
//! 結合文字を個別に扱う分岐は持たない（形ごとの特例を足すと、ここを通る再生時間と
//! バルーンの段数・幅・折り返しの数が食い違うため）。
//!
//! 1 スカラー値の文字（日本語・英字・記号・😀）は 1 クラスタで、`chars()` と同じ列になる。
//! クラスタは渡された文字列の中だけで切る（呼び出し側が cue ごとに渡す・cue をまたいで
//! 繋がない）。失敗せず、記録も出さない純関数。

use unicode_segmentation::UnicodeSegmentation;

/// 拡張書記素クラスタ（UAX #29）の列。空文字列は空の列。
///
/// 事後条件: 各要素は非空・連結は入力と一致（無損失）・同じ入力に同じ列（決定的）。
pub fn clusters(text: &str) -> impl Iterator<Item = &str> {
    text.graphemes(true)
}

/// `clusters(text).count()`。
pub fn cluster_count(text: &str) -> usize {
    clusters(text).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 要件 2.2 の 6 形は、それぞれ 1 クラスタ。
    #[test]
    fn six_multi_scalar_forms_are_one_cluster_each() {
        for text in [
            "👨\u{200D}👩\u{200D}👧", // ZWJ 列
            "🇯🇵",                     // 地域表示記号の対
            "👍🏻",                     // 肌色の修飾つき
            "\u{2764}\u{FE0F}",       // 異体字セレクタつき ❤️
            "1\u{FE0F}\u{20E3}",      // キーキャップ 1️⃣
            "\u{304B}\u{309A}",       // 結合文字つき か゚
        ] {
            assert!(text.chars().count() > 1, "{text:?} は複数のスカラー値");
            assert_eq!(clusters(text).collect::<Vec<_>>(), [text], "{text:?}");
            assert_eq!(cluster_count(text), 1, "{text:?}");
        }
    }

    /// 1 スカラー値の文字は今日の `chars()` と同じ列（要件 2.3）。
    #[test]
    fn single_scalar_chars_split_like_chars() {
        assert_eq!(clusters("aあ🦆").collect::<Vec<_>>(), ["a", "あ", "🦆"]);
        assert_eq!(cluster_count("aあ🦆"), 3);
    }

    /// 空文字列は空の列。
    #[test]
    fn empty_text_has_no_clusters() {
        assert_eq!(clusters("").count(), 0);
        assert_eq!(cluster_count(""), 0);
    }

    /// 連結は入力と一致し、各要素は非空（無損失）。
    #[test]
    fn concatenation_reproduces_input() {
        let text = "こんにちは👨\u{200D}👩\u{200D}👧🇯🇵 a\u{304B}\u{309A}1\u{FE0F}\u{20E3}\n";
        assert!(clusters(text).all(|c| !c.is_empty()));
        assert_eq!(clusters(text).collect::<String>(), text);
    }
}
