//! 合成の計画の純粋なテスト（COM 無し・要件 7.2, 7.4）。ここでは全部の合成の理由の綴り（3.4）。

use super::damage::*;

#[test]
fn full_reason_spellings_are_the_seven_record_words() {
    let all = [
        (FullReason::First, "first"),
        (FullReason::Recreated, "recreated"),
        (FullReason::RootList, "root_list"),
        (FullReason::Scale, "scale"),
        (FullReason::NoPrev, "no_prev"),
        (FullReason::Undecidable, "undecidable"),
        (FullReason::CoversSurface, "covers_surface"),
    ];
    for (reason, word) in all {
        assert_eq!(reason.as_str(), word);
    }
}

#[test]
fn damage_record_target() {
    assert_eq!(DAMAGE_TARGET, "wintf::damage");
}
