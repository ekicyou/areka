//! 本機能（サウンドプロパティ名の記録用の表と設定可能語彙への登記）が足した項目の判定。
//!
//! 期待値は実装や表から導出せず、テストの中に文字として書く。

use super::*;
use std::collections::BTreeSet;

/// 記録用の表の 18 要素（literal）。
const EXPECTED_SOUND_PROP_NAMES: &[&str] = &[
    "duration",
    "error",
    "id",
    "loop",
    "name",
    "path",
    "preload",
    "pause",
    "playing",
    "position",
    "meta.album",
    "meta.albumartist",
    "meta.artist",
    "meta.artwork",
    "meta.genre",
    "meta.title",
    "meta.track",
    "meta.year",
];

/// 記録用の表: 件数 18・literal と過不足なく一致・重複 0・区切りなし 10・`meta.` 始まり 8。
#[test]
fn sound_prop_names_has_18_entries_10_bare_and_8_meta() {
    assert_eq!(SOUND_PROP_NAMES.len(), 18);

    let actual: BTreeSet<&str> = SOUND_PROP_NAMES.iter().copied().collect();
    assert_eq!(
        actual.len(),
        SOUND_PROP_NAMES.len(),
        "SOUND_PROP_NAMES に重複あり"
    );
    let expected: BTreeSet<&str> = EXPECTED_SOUND_PROP_NAMES.iter().copied().collect();
    assert_eq!(
        actual, expected,
        "SOUND_PROP_NAMES の集合が literal と不一致"
    );

    let bare = SOUND_PROP_NAMES.iter().filter(|n| !n.contains('.')).count();
    assert_eq!(bare, 10, "区切りを含まない要素の数");
    let meta = SOUND_PROP_NAMES
        .iter()
        .filter(|n| n.starts_with("meta."))
        .count();
    assert_eq!(meta, 8, "`meta.` で始まる要素の数");
}

/// `pause`・`playing`・`position` の 3 葉だけが設定可能語彙にも載り、残り 15 葉は載らない。
#[test]
fn sound_set_leaves_appear_in_both_tables() {
    let set_keys: BTreeSet<&str> = SET_EFFECTIVE.iter().map(|(k, _)| *k).collect();
    let in_both: BTreeSet<&str> = SOUND_PROP_NAMES
        .iter()
        .copied()
        .filter(|n| set_keys.contains(n))
        .collect();
    let expected: BTreeSet<&str> = ["pause", "playing", "position"].into_iter().collect();
    assert_eq!(in_both, expected, "設定可能語彙にも載る葉の集合");
    assert_eq!(in_both.len(), 3, "設定可能語彙にも載る葉の数");
}

/// 本機能より前からある設定可能語彙のキーが、先頭で順序込みで変わっていない（要件 1.6）。
#[test]
fn set_effective_entries_before_this_spec_are_unchanged_in_order() {
    let before: &[&str] = &[
        "surface.num",
        "animation.num",
        "seriko.defaultsurface",
        "mousecursor",
        "mousecursor.text",
        "mousecursor.wait",
        "mousecursor.hand",
        "mousecursor.grip",
        "mousecursor.arrow",
        "balloon.mousecursor",
        "balloon.mousecursor.text",
        "balloon.mousecursor.wait",
        "balloon.mousecursor.arrow",
        "seriko.cursor.path",
        "seriko.cursor.name",
        "seriko.tooltip.text",
        "seriko.tooltip.name",
        "menu",
        "sakura.bind.menu",
        "kero.bind.menu",
        "char*.bind.menu",
    ];
    assert!(
        SET_EFFECTIVE.len() >= before.len(),
        "設定可能語彙が変更前より短い"
    );
    let head: Vec<&str> = SET_EFFECTIVE[..before.len()]
        .iter()
        .map(|(k, _)| *k)
        .collect();
    assert_eq!(head, before, "設定可能語彙の既存キーの順序が変わった");
}
