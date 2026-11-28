//! element定義の描画メソッドの語の表の単体テスト。

use super::{BLEND_ELEMENT_WORDS, BoxDefinition, parse, parse_boxes, parse_undrawn_elements};

const COMPOSE_WORDS: [&str; 8] = [
    "overlay",
    "overlay-fast",
    "overlayfast",
    "interpolate",
    "replace",
    "asis",
    "reduce",
    "base",
];

const UNDRAWABLE_WORDS: [&str; 14] = [
    "",
    "add",
    "bind",
    "auto",
    "zoom",
    "Overlay",
    "REPLACE",
    "Blend-Multiply",
    "blend-Multiply",
    "blend-multiply-Fast",
    "blend-dither-fast",
    "blend-frobnicate",
    "blend-",
    "overlay_fast",
];

/// 1 行の element定義を持つ面の文面を、画像・箱・描けない行の 3 つの転記の件数に分ける。
fn partition(word: &str) -> (usize, usize, usize) {
    let text = format!("surface0\n{{\nelement0,{word},x.png,0,0\n}}\n");
    let images = parse(&text).surfaces.iter().map(|s| s.elements.len()).sum();
    let boxes = parse_boxes(&text)
        .iter()
        .map(|d| match d {
            BoxDefinition::Surface(s) | BoxDefinition::Append(s) => s.elements.len(),
            BoxDefinition::Brace(_) => 0,
        })
        .sum();
    (images, boxes, parse_undrawn_elements(&text).len())
}

#[test]
fn blend_element_words_are_the_ukadoc_spellings() {
    assert_eq!(BLEND_ELEMENT_WORDS.len(), 27 * 2 + 3);
    let mut sorted = BLEND_ELEMENT_WORDS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        BLEND_ELEMENT_WORDS.len(),
        "綴りが重なっている"
    );
    for word in [
        "blend-add",
        "blend-add-glow-fast",
        "blend-vivid-light",
        "blend-vivid-light-fast",
        "blend-dither",
        "overlaymultiply",
        "overlayscreen",
    ] {
        assert!(BLEND_ELEMENT_WORDS.contains(&word), "{word}");
    }
    assert!(!BLEND_ELEMENT_WORDS.contains(&"blend-dither-fast"));
}

#[test]
fn every_table_word_is_an_image_element() {
    for word in COMPOSE_WORDS.iter().chain(BLEND_ELEMENT_WORDS) {
        assert_eq!(partition(word), (1, 0, 0), "{word}");
    }
}

#[test]
fn uppercase_unknown_add_bind_auto_stay_undrawn() {
    for word in UNDRAWABLE_WORDS {
        assert_eq!(partition(word), (0, 0, 1), "{word:?}");
    }
}

#[test]
fn balloon_goes_only_to_boxes() {
    assert_eq!(partition("balloon"), (0, 1, 0));
}
