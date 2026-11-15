use super::resolve_test_support::input;
use super::test_support::work_area;
use super::*;
use crate::config::{Alignment, build_placement_config};
use crate::shared_test_support::MEASURE_DPI;
use crate::test_support::kv;
use log_capture_kit::capture;

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
/// （`Seam` は descript に書かれたままの値を持つので、大文字小文字はここでそろえる）。
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

/// descript の `alignmenttodesktop`（別綴 `alignmentondesktop`）は大文字小文字を区別しない。
/// 受け付ける 5 語はどの書き方でも同じ向きになり、未知アンカーの警告を出さない。
/// 知らない語は今までどおり bottom 相当で警告 1 行、`value` には書かれたままの値を載せる。
#[test]
fn descript_alignment_words_ignore_case() {
    let wa = work_area(96);
    let inputs = [input(0, 400, 600)];
    let resolve = |key: &str, raw: &str| {
        let cfg = build_placement_config(&kv(&[(key, raw)]), &kv(&[]));
        let anchor = Anchor::from_alignment(&cfg.scopes[&0].alignment);
        let (_, events) = capture(|| resolve_placement(&cfg, wa, &inputs, MEASURE_DPI));
        let warns: Vec<_> = events
            .into_iter()
            .filter(|e| e.level == tracing::Level::WARN)
            .collect();
        (anchor, warns)
    };
    let words = [
        ("bottom", Anchor::Bottom),
        ("free", Anchor::Free),
        ("top", Anchor::Top),
        ("left", Anchor::Left),
        ("right", Anchor::Right),
    ];
    for key in ["seriko.alignmenttodesktop", "seriko.alignmentondesktop"] {
        for (word, expected) in words {
            let mut capitalized = word.to_owned();
            capitalized[..1].make_ascii_uppercase();
            for raw in [word.to_owned(), word.to_ascii_uppercase(), capitalized] {
                let (anchor, warns) = resolve(key, &raw);
                assert_eq!(anchor, expected, "{key},{raw}");
                assert!(warns.is_empty(), "{key},{raw}: {warns:?}");
            }
        }
    }

    let (anchor, warns) = resolve("seriko.alignmenttodesktop", "Middle");
    assert_eq!(anchor, Anchor::Bottom, "知らない語は bottom 相当");
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert_eq!(warns[0].field("value"), Some("Middle"), "書かれたままの値");
}
