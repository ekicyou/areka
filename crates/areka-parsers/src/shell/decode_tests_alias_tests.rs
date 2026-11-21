//! kero.surface.alias 写像。

use super::{AliasKey, SurfaceAlias, decode, lex};

/// 数値キー alias（`6,[2106,2206]`）→ キー "6"（不透明）＋ ids [2106,2206]。
#[test]
fn numeric_key_alias_is_mapped_with_ordered_ids() {
    let input = "kero.surface.alias\n{\n6,[2106,2206]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(
        shell.aliases,
        vec![SurfaceAlias {
            key: AliasKey::new("6".to_string()),
            ids: vec![2106, 2206],
        }]
    );
    assert_eq!(shell.aliases[0].key.as_str(), "6");
    assert!(shell.surfaces.is_empty());
    assert!(shell.appends.is_empty());
}

/// 日本語キー alias（`静観,[2106,2206]`）→ キー "静観" を不透明に保持。
#[test]
fn japanese_key_alias_is_preserved_opaque() {
    let input = "kero.surface.alias\n{\n静観,[2106,2206]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(
        shell.aliases,
        vec![SurfaceAlias {
            key: AliasKey::new("静観".to_string()),
            ids: vec![2106, 2206],
        }]
    );
    assert_eq!(shell.aliases[0].key.as_str(), "静観");
}

/// 複数 ID の順序を保持する。
#[test]
fn alias_ids_preserve_order() {
    let input = "kero.surface.alias\n{\n0,[30,10,20,40]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.aliases.len(), 1);
    assert_eq!(shell.aliases[0].ids, vec![30, 10, 20, 40]);
}

/// 単一 ID alias（`5,[0]`）→ ids [0]。
#[test]
fn alias_single_id_is_mapped() {
    let input = "kero.surface.alias\n{\n5,[0]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.aliases.len(), 1);
    assert_eq!(shell.aliases[0].key.as_str(), "5");
    assert_eq!(shell.aliases[0].ids, vec![0]);
}

/// 同一キーの複数出現は潰さず全出現を保持する（出現順・各自の ID リスト）。
#[test]
fn duplicate_keys_are_all_retained() {
    let input = "kero.surface.alias\n{\n100,[2100,2101]\n100,[2200,2201]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(
        shell.aliases,
        vec![
            SurfaceAlias {
                key: AliasKey::new("100".to_string()),
                ids: vec![2100, 2101],
            },
            SurfaceAlias {
                key: AliasKey::new("100".to_string()),
                ids: vec![2200, 2201],
            },
        ]
    );
}

/// 複数 alias エントリは出現順で保持する。
#[test]
fn multiple_alias_entries_keep_appearance_order() {
    let input = "kero.surface.alias\n{\n6,[2106,2206]\n静観,[2106]\n7,[2107]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.aliases.len(), 3);
    assert_eq!(shell.aliases[0].key.as_str(), "6");
    assert_eq!(shell.aliases[1].key.as_str(), "静観");
    assert_eq!(shell.aliases[2].key.as_str(), "7");
}

/// 値が空/欠落のキーは空 ids で寛容に保持する（パニックしない）。
#[test]
fn alias_with_empty_value_yields_empty_ids() {
    let input = "kero.surface.alias\n{\n8,[]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.aliases.len(), 1);
    assert_eq!(shell.aliases[0].key.as_str(), "8");
    assert!(shell.aliases[0].ids.is_empty());
}
