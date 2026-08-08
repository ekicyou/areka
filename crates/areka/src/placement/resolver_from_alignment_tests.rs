use super::*;
use crate::placement::config::Alignment;

// ------------------------------------------------------------------
// Anchor::from_alignment（4.2・design Testing Strategy Unit #4）
// ------------------------------------------------------------------

/// 4.2: cascade 解決済み `Alignment` の 5 値アンカーへの解釈写像を全分岐固定する。
/// `Bottom`→`Bottom`・`Free`→`Free`・`Seam("top"/"left"/"right")`→対応値・
/// 未知 `Seam` →`Bottom`（フォールバック・DD9「未知は bottom 相当」を継承）。
#[test]
fn from_alignment_maps_all_branches() {
    assert_eq!(
        Anchor::from_alignment(&Alignment::Bottom),
        Anchor::Bottom,
        "Bottom → Bottom"
    );
    assert_eq!(
        Anchor::from_alignment(&Alignment::Free),
        Anchor::Free,
        "Free → Free"
    );
    assert_eq!(
        Anchor::from_alignment(&Alignment::Seam("top".to_owned())),
        Anchor::Top,
        "Seam(top) → Top"
    );
    assert_eq!(
        Anchor::from_alignment(&Alignment::Seam("left".to_owned())),
        Anchor::Left,
        "Seam(left) → Left"
    );
    assert_eq!(
        Anchor::from_alignment(&Alignment::Seam("right".to_owned())),
        Anchor::Right,
        "Seam(right) → Right"
    );
    assert_eq!(
        Anchor::from_alignment(&Alignment::Seam("unknown-value".to_owned())),
        Anchor::Bottom,
        "Seam(未知) → Bottom（フォールバック）"
    );
}

/// 4.2 補: `Seam` 値は `trim().to_ascii_lowercase()` で正規化してから解釈する
/// （parsers 側で正規化済み前提だが防御・design Implementation Notes Risks）。
#[test]
fn from_alignment_normalizes_case_and_whitespace() {
    assert_eq!(
        Anchor::from_alignment(&Alignment::Seam(" TOP ".to_owned())),
        Anchor::Top,
        "前後空白＋大文字も Top へ"
    );
    assert_eq!(
        Anchor::from_alignment(&Alignment::Seam("Right".to_owned())),
        Anchor::Right,
        "大文字混じりも Right へ"
    );
}
