//! pattern method 忠実転記＋interval 忠実転記（元 decode_tests.rs タスク 1.2 区画）。
//!
//! 本ファイルは `decode_tests.rs` のテーマ分割（areka-P0-file-slimming タスク 8.5・要件 1.7）で
//! 切り出したものであり、テスト本文は分割前と同一である。

use super::{Interval, decode, lex};

// --- タスク 1.2: pattern method 忠実転記＋interval 忠実転記（overlay フィルタ／fallback-Bind 撤去） ---
//
// 検証範囲（要件 4.6/7.5/8.2/8.4）:
// - 非 overlay な pattern 行（replace 等）を落とさないことは `method_matrix_tests::full_method_matrix_all_rows_preserved_in_order` が確かめる。
// - overlay 行も field[1] を verbatim 転記する（従来は空文字プレースホルダだった）。
// - `normalize_interval` の fallback-Bind 撤去: 未認識 interval キーワード（sometimes 等）を
//   `Interval::Bind` へ倒さず `Interval::Other(原文)` へ忠実転記する（要件 8.2・討議 #1 裁定）。
// 完全な転記マトリクス（overlay/replace/未知名/欠落 × interval 種）は タスク 1.3 の檻。

/// overlay pattern 行の method（field[1]）も verbatim 転記される（従来は空文字プレースホルダ・要件 4.6）。
#[test]
fn overlay_pattern_method_is_transcribed_verbatim() {
    let input = "surface0\n{\nanimation0.interval,bind\nanimation0.pattern0,overlay,100,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(
        shell.surfaces[0].animations[0].patterns[0].method.as_str(),
        "overlay"
    );
}

/// 未認識 interval キーワード（`interval,sometimes`）は `Interval::Bind` へ倒れず
/// `Interval::Other("sometimes")` へ忠実転記される（要件 8.2・fallback-Bind 撤去の証明）。
#[test]
fn unrecognized_interval_keyword_becomes_other_not_bind() {
    let input =
        "surface0\n{\nanimation0.interval,sometimes\nanimation0.pattern0,overlay,10,0,0,0\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.surfaces[0].animations.len(), 1);
    assert_eq!(
        shell.surfaces[0].animations[0].interval,
        Interval::Other("sometimes".into())
    );
    // pattern は失われない。
    assert_eq!(shell.surfaces[0].animations[0].patterns.len(), 1);
}
