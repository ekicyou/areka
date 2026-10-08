//! 独自のツールの表と 11 本の登録の決定論テスト（実ソケットなし・spec: areka-P0-mcp-author-tools）。

use std::collections::BTreeSet;

use super::*;

fn specs(registry: &ToolRegistry) -> Vec<ToolSpec> {
    registry.entries().iter().map(|(s, _)| s.clone()).collect()
}

/// (要件 1.1・1.3・1.4) 11 本・先頭 10 本は `entrances` と同じ並びと定義・11 本目は `check_script`。
#[test]
fn all_entrances_is_ssp_ten_then_own_rows() {
    let (all, _rx) = all_entrances(REPLY_WAIT);
    let (ssp, _rx) = entrances(REPLY_WAIT);
    let all = specs(&all);
    assert_eq!(all.len(), 11);
    assert_eq!(&all[..10], specs(&ssp).as_slice());
    assert_eq!(all[10].name, "check_script");
}

/// (要件 1.1・4.3) 名前の重複 0（登録口は後勝ちで畳むので、表の全行の名前で見る）・独自の表の各行が登録表に在る。
#[test]
fn own_rows_are_registered_without_duplicate_names() {
    let names: Vec<String> = TABLE
        .iter()
        .map(|(def, _)| *def)
        .chain(OWN_TABLE.iter().map(|(def, _, _)| *def))
        .map(|def| spec_from_definition(def).unwrap().name)
        .collect();
    let unique: BTreeSet<&String> = names.iter().collect();
    assert_eq!(unique.len(), names.len(), "名前が重なっている: {names:?}");

    let (all, _rx) = all_entrances(REPLY_WAIT);
    let all = specs(&all);
    for (def, _, _) in OWN_TABLE {
        let own = spec_from_definition(def).unwrap();
        assert!(all.contains(&own), "{} が登録表に無い", own.name);
    }
}

/// (要件 1.4) `entrances` は 10 本のまま。
#[test]
fn entrances_stays_ten() {
    let (ssp, _rx) = entrances(REPLY_WAIT);
    assert_eq!(ssp.len(), 10);
}
