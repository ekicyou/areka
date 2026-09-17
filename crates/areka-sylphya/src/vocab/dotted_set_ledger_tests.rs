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

/// 新規登記の 4 項: 書込分類が運行コマンドで、書込を適用した効果列が予約済みの受理ちょうど 1 件。
#[test]
fn newly_registered_set_keys_are_runtime_command_reserved() {
    use crate::{AskerId, Effect, SetClass, SylphyaCore, SylphyaMsg, classify_set};

    let expected: &[(&str, SetClass)] = &[
        ("seriko.sticky-window", SetClass::RuntimeCommand),
        ("pause", SetClass::RuntimeCommand),
        ("playing", SetClass::RuntimeCommand),
        ("position", SetClass::RuntimeCommand),
    ];
    for (key, class) in expected {
        assert_eq!(classify_set(key), *class, "書込分類: {key}");
        let asker = AskerId::new("ghost/a");
        let effects = SylphyaCore::new().apply(&SylphyaMsg::Set {
            asker: asker.clone(),
            key: (*key).into(),
            value: "1".into(),
        });
        assert_eq!(
            effects,
            vec![Effect::RuntimeCommandReserved {
                asker,
                key: (*key).into(),
                value: "1".into(),
            }],
            "書込の効果列: {key}"
        );
    }
}

/// 記録用の 15 葉と実キー形 3 つの書込分類は変更前と同じで、18 葉と実キー形の参照はすべて見つからない。
#[test]
fn record_only_leaves_keep_their_previous_classification() {
    use crate::{
        AskerContext, AskerId, DottedResolution, MirrorImage, SetClass, SharedMirror,
        SylphyaReader, classify_set,
    };
    use std::sync::Arc;

    let expected: &[(&str, SetClass)] = &[
        // 自由な名前としての保存 13 件。
        ("duration", SetClass::StoreWrite),
        ("error", SetClass::StoreWrite),
        ("id", SetClass::StoreWrite),
        ("loop", SetClass::StoreWrite),
        ("preload", SetClass::StoreWrite),
        ("meta.album", SetClass::StoreWrite),
        ("meta.albumartist", SetClass::StoreWrite),
        ("meta.artist", SetClass::StoreWrite),
        ("meta.artwork", SetClass::StoreWrite),
        ("meta.genre", SetClass::StoreWrite),
        ("meta.title", SetClass::StoreWrite),
        ("meta.track", SetClass::StoreWrite),
        ("meta.year", SetClass::StoreWrite),
        // 設定できない正準語彙 2 件（葉が汎用名の表に載る）。
        ("name", SetClass::NotSettable),
        ("path", SetClass::NotSettable),
        // 実キー形 3 件（根 `currentghost` による）。括弧の中に区切り文字を含む要素名は使わない。
        ("currentghost.sound(bgm).pause", SetClass::NotSettable),
        ("currentghost.sound.index(0).playing", SetClass::NotSettable),
        ("currentghost.sound(bgm).meta.album", SetClass::NotSettable),
    ];
    for (key, class) in expected {
        assert_eq!(classify_set(key), *class, "書込分類: {key}");
    }

    let mut image = MirrorImage::empty();
    image
        .dotted_global
        .insert("currentghost.name".into(), "さくら".into());
    let reader = SylphyaReader::new(SharedMirror::new(Arc::new(image)));
    let ctx = AskerContext {
        asker: AskerId::new("ghost/a"),
    };

    // 対照: 値を持つ既存キーは返る（参照の経路が生きている）。
    assert_eq!(
        reader.resolve_dotted_str(&ctx, "currentghost.name"),
        DottedResolution::Value("さくら".into())
    );
    let real_keys = [
        "currentghost.sound(bgm).pause",
        "currentghost.sound.index(0).playing",
        "currentghost.sound(bgm).meta.album",
    ];
    for key in SOUND_PROP_NAMES.iter().chain(real_keys.iter()) {
        assert_eq!(
            reader.resolve_dotted_str(&ctx, key),
            DottedResolution::NotFound,
            "参照: {key}"
        );
    }
}

/// 仕分け（`classify_set`）の実装とその兄弟テスト 3 本が記録用の表を読まない（要件 3.3／3.7／7.2）。
#[test]
fn classifier_source_does_not_reference_the_record_only_table() {
    let sources: &[(&str, &str)] = &[
        ("actor.rs", include_str!("../actor.rs")),
        ("actor_tests.rs", include_str!("../actor_tests.rs")),
        (
            "actor_actor_integration_tests.rs",
            include_str!("../actor_actor_integration_tests.rs"),
        ),
        (
            "actor_actor_criteria_cage.rs",
            include_str!("../actor_actor_criteria_cage.rs"),
        ),
    ];
    let record_only = "SOUND_PROP_NAMES";
    for (file, text) in sources {
        assert_eq!(
            text.matches(record_only).count(),
            0,
            "{file} が記録用の表 {record_only} を参照している。仕分けが読む表へ変えるなら要件 3.3／7.2 の再検討と引受先 spec（areka-P0-property-query-channels）の裁定が要る"
        );
    }
    // 較正: 探し方が生きていることを、仕分けが実際に読む表の名前で示す。
    assert!(
        sources[0].1.matches("GENERIC_PROP_NAMES").count() >= 1,
        "較正失敗: actor.rs に GENERIC_PROP_NAMES が見つからない（読み込み対象を取り違えている）"
    );
}

/// 語彙表の本体に正典ページのアンカー付き URL 注記がちょうど 19 行あり、アンカー無しのページ URL 行は 0（要件 5.1／5.4）。
#[test]
fn exactly_19_anchored_ukadoc_notes_and_no_page_url_line() {
    // 探し語は行頭に置かない（証拠抽出器がこのファイルの行を拾わないように）。
    let note_prefix = concat!("// ", "ukadoc", ": ");
    let canon_page = "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#";
    let notes: Vec<&str> = include_str!("dotted.rs")
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with(note_prefix))
        .collect();
    assert_eq!(notes.len(), 19, "URL 注記の行数");
    let anchored = notes.iter().filter(|l| l.contains(canon_page)).count();
    assert_eq!(anchored, 19, "正典ページのアンカーを指す注記の行数");
    let page_only = notes.iter().filter(|l| l.ends_with(".html")).count();
    assert_eq!(page_only, 0, "アンカー無しのページ URL 注記の行数");
}
