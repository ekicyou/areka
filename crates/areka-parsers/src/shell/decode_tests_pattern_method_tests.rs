//! pattern method 忠実転記＋interval 忠実転記。

use super::{Interval, decode, lex};

// 非 overlay な pattern 行を落とさないことの確かめは `method_matrix_tests::full_method_matrix_all_rows_preserved_in_order`。

/// overlay pattern 行の method（field[1]）も verbatim 転記される。
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
/// `Interval::Other("sometimes")` へ忠実転記される。
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
    assert_eq!(shell.surfaces[0].animations[0].patterns.len(), 1);
}
