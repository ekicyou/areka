//! kero.surface.alias 写像（元 decode_tests.rs タスク 4.5 区画）。
//!
//! 本ファイルは `decode_tests.rs` のテーマ分割（areka-P0-file-slimming タスク 8.5・要件 1.7）で
//! 切り出したものであり、テスト本文は分割前と同一である。

use super::{AliasKey, SurfaceAlias, decode, lex};

// --- タスク 4.5: kero.surface.alias 写像（不透明キー・順序付き ID・重複保持） ---
//
// 検証範囲（要件 8.1/8.2/8.3/8.4）:
// - `kero.surface.alias` ブロックの各 `KEY,[ID,...]` 行を、不透明 alias キーと
//   順序付き数値 ID リストの写像として `shell.aliases` に保持する（要件 8.1）。
// - alias キー（数値・日本語文字列いずれも）を意味解釈せず不透明文字列で保持する（要件 8.2）。
// - alias 値 `[id,id,...]` を数値 ID の順序付きリストとして保持する（要件 8.3）。
// - 同一 alias キーが複数回出現しても潰さず全出現を保持する（衝突解決は下流・要件 8.4）。
// 期待値はリテラル直書き（sakura 規律）。

/// 数値キー alias（`6,[2106,2206]`）→ キー "6"（不透明）＋ ids [2106,2206]（要件 8.1/8.2/8.3）。
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
    // キーは数値解釈せず不透明文字列で保持する（要件 8.2）。
    assert_eq!(shell.aliases[0].key.as_str(), "6");
    // surface/append は alias 解析で汚染されない。
    assert!(shell.surfaces.is_empty());
    assert!(shell.appends.is_empty());
}

/// 日本語キー alias（`静観,[2106,2206]`）→ キー "静観" を不透明に保持（要件 8.2）。
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

/// 複数 ID の順序を保持する（要件 8.3）。
#[test]
fn alias_ids_preserve_order() {
    let input = "kero.surface.alias\n{\n0,[30,10,20,40]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.aliases.len(), 1);
    assert_eq!(shell.aliases[0].ids, vec![30, 10, 20, 40]);
}

/// 単一 ID alias（`5,[0]`）→ ids [0]（要件 8.3）。
#[test]
fn alias_single_id_is_mapped() {
    let input = "kero.surface.alias\n{\n5,[0]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.aliases.len(), 1);
    assert_eq!(shell.aliases[0].key.as_str(), "5");
    assert_eq!(shell.aliases[0].ids, vec![0]);
}

/// 同一キーの複数出現は潰さず全出現を保持する（出現順・各自の ID リスト・要件 8.4）。
#[test]
fn duplicate_keys_are_all_retained() {
    let input = "kero.surface.alias\n{\n100,[2100,2101]\n100,[2200,2201]\n}\n";
    let shell = decode(lex(input));
    // 2 エントリとも残る（衝突解決は下流）。
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

/// 複数 alias エントリは出現順で保持する（要件 8.1）。
#[test]
fn multiple_alias_entries_keep_appearance_order() {
    let input = "kero.surface.alias\n{\n6,[2106,2206]\n静観,[2106]\n7,[2107]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.aliases.len(), 3);
    assert_eq!(shell.aliases[0].key.as_str(), "6");
    assert_eq!(shell.aliases[1].key.as_str(), "静観");
    assert_eq!(shell.aliases[2].key.as_str(), "7");
}

/// 値が空/欠落のキーは空 ids で寛容に保持する（パニックしない・要件 8.4 系寛容）。
#[test]
fn alias_with_empty_value_yields_empty_ids() {
    let input = "kero.surface.alias\n{\n8,[]\n}\n";
    let shell = decode(lex(input));
    assert_eq!(shell.aliases.len(), 1);
    assert_eq!(shell.aliases[0].key.as_str(), "8");
    assert!(shell.aliases[0].ids.is_empty());
}
