//! 窓の組の帳簿の決定論の檻（要件 3.1〜3.5・3.7〜3.9・6.5・design「StickyGroupLedger」）。
//!
//! World も実機も使わない純関数の檻である。

use std::collections::BTreeSet;

use super::*;

fn set(scopes: &[u32]) -> BTreeSet<u32> {
    scopes.iter().copied().collect()
}

fn script(scopes: &[u32]) -> StickyGroup {
    StickyGroup {
        members: set(scopes),
        source: StickySource::Script,
    }
}

fn descript(scopes: &[u32]) -> StickyGroup {
    StickyGroup {
        members: set(scopes),
        source: StickySource::Descript,
    }
}

fn ledger_with_script(groups: &[&[u32]]) -> StickyGroupLedger {
    let mut ledger = StickyGroupLedger::default();
    for scopes in groups {
        ledger.try_add_script_group(set(scopes)).unwrap();
    }
    ledger
}

// ── 読み（要件 3.5） ──

#[test]
fn parse_accepts_two_or_more_distinct_ids() {
    assert_eq!(parse_sticky_tokens(&["0", "1"]), Ok(set(&[0, 1])));
    assert_eq!(parse_sticky_tokens(&["3", " 2 ", "7"]), Ok(set(&[2, 3, 7])));
}

#[test]
fn parse_order_has_no_meaning() {
    assert_eq!(
        parse_sticky_tokens(&["1", "0"]),
        parse_sticky_tokens(&["0", "1"])
    );
}

#[test]
fn parse_rejects_fewer_than_two() {
    let empty: [&str; 0] = [];
    assert_eq!(
        parse_sticky_tokens(&empty),
        Err(StickyReject::TooFewScopes { count: 0 })
    );
    assert_eq!(
        parse_sticky_tokens(&["0"]),
        Err(StickyReject::TooFewScopes { count: 1 })
    );
}

#[test]
fn parse_rejects_duplicate_before_counting() {
    // `0,0` は異なる ID が 1 つだが、重ね書きの検査が先に当たる。
    assert_eq!(
        parse_sticky_tokens(&["0", "0"]),
        Err(StickyReject::DuplicateScope { scope: 0 })
    );
    assert_eq!(
        parse_sticky_tokens(&["0", "1", " 1"]),
        Err(StickyReject::DuplicateScope { scope: 1 })
    );
}

#[test]
fn parse_rejects_non_numbers_first() {
    for token in ["a", "", " ", "+1", "-1", "1.0", "１"] {
        assert_eq!(
            parse_sticky_tokens(&["0", token]),
            Err(StickyReject::UnparsableToken {
                token: token.to_string()
            }),
            "token={token:?}"
        );
    }
    // 数でないことは重ね書きより先に検査する。
    assert_eq!(
        parse_sticky_tokens(&["0", "0", "x"]),
        Err(StickyReject::UnparsableToken {
            token: "x".to_string()
        })
    );
}

#[test]
fn parse_rejects_overflow_but_accepts_u32_max() {
    assert_eq!(
        parse_sticky_tokens(&["0", "4294967296"]),
        Err(StickyReject::UnparsableToken {
            token: "4294967296".to_string()
        })
    );
    assert_eq!(
        parse_sticky_tokens(&["0", "4294967295"]),
        Ok(set(&[0, u32::MAX]))
    );
}

// ── 台本の組（要件 3.1〜3.4） ──

#[test]
fn script_groups_can_be_many() {
    let ledger = ledger_with_script(&[&[1, 0], &[3, 2]]);
    assert_eq!(ledger.groups(), &[script(&[0, 1]), script(&[2, 3])]);
}

#[test]
fn overlap_with_script_group_changes_nothing() {
    let mut ledger = ledger_with_script(&[&[0, 1]]);
    let before = ledger.clone();
    assert_eq!(
        ledger.try_add_script_group(set(&[1, 2])),
        Err(StickyReject::AlreadyGrouped { scopes: vec![1] })
    );
    assert_eq!(ledger, before);
}

#[test]
fn overlap_reports_every_overlapping_id_across_groups() {
    let mut ledger = ledger_with_script(&[&[0, 1], &[2, 3]]);
    ledger.replace_descript(Some("5,6")).unwrap();
    let before = ledger.clone();
    assert_eq!(
        ledger.try_add_script_group(set(&[3, 4, 0, 6])),
        Err(StickyReject::AlreadyGrouped {
            scopes: vec![0, 3, 6]
        })
    );
    assert_eq!(ledger, before);
}

#[test]
fn overlap_with_descript_group_is_rejected() {
    let mut ledger = StickyGroupLedger::default();
    ledger.replace_descript(Some("0,1")).unwrap();
    let before = ledger.clone();
    assert_eq!(
        ledger.try_add_script_group(set(&[0, 2])),
        Err(StickyReject::AlreadyGrouped { scopes: vec![0] })
    );
    assert_eq!(ledger, before);
}

// ── 解除（要件 3.7） ──

#[test]
fn reset_removes_only_script_groups() {
    let mut ledger = StickyGroupLedger::default();
    ledger.replace_descript(Some("0,1")).unwrap();
    ledger.try_add_script_group(set(&[2, 3])).unwrap();
    ledger.try_add_script_group(set(&[4, 5])).unwrap();
    assert_eq!(ledger.reset_to_descript(), 2);
    assert_eq!(ledger.groups(), &[descript(&[0, 1])]);
    assert_eq!(ledger.reset_to_descript(), 0);
}

// ── descript の組（要件 3.8・3.9・6.5） ──

#[test]
fn descript_none_or_empty_means_no_group() {
    for raw in [None, Some(""), Some("  ")] {
        let mut ledger = StickyGroupLedger::default();
        assert_eq!(ledger.replace_descript(raw), Ok(()), "raw={raw:?}");
        assert!(ledger.groups().is_empty());
    }
}

#[test]
fn descript_uses_same_rules_as_script() {
    let mut ledger = StickyGroupLedger::default();
    assert_eq!(ledger.replace_descript(Some(" 1 , 0 ")), Ok(()));
    assert_eq!(ledger.groups(), &[descript(&[0, 1])]);

    for (raw, reject) in [
        ("0", StickyReject::TooFewScopes { count: 1 }),
        ("0,0", StickyReject::DuplicateScope { scope: 0 }),
        (
            "0,x",
            StickyReject::UnparsableToken {
                token: "x".to_string(),
            },
        ),
        (
            "0,",
            StickyReject::UnparsableToken {
                token: String::new(),
            },
        ),
    ] {
        let mut ledger = StickyGroupLedger::default();
        assert_eq!(
            ledger.replace_descript(Some(raw)),
            Err(reject),
            "raw={raw:?}"
        );
        assert!(ledger.groups().is_empty());
    }
}

#[test]
fn replace_keeps_script_groups_and_swaps_descript_group() {
    let mut ledger = StickyGroupLedger::default();
    ledger.replace_descript(Some("0,1")).unwrap();
    ledger.try_add_script_group(set(&[2, 3])).unwrap();

    assert_eq!(ledger.replace_descript(Some("4,5")), Ok(()));
    assert_eq!(ledger.groups(), &[script(&[2, 3]), descript(&[4, 5])]);

    assert_eq!(ledger.replace_descript(None), Ok(()));
    assert_eq!(ledger.groups(), &[script(&[2, 3])]);
}

#[test]
fn replace_rejects_descript_group_overlapping_script_group() {
    let mut ledger = StickyGroupLedger::default();
    ledger.replace_descript(Some("0,1")).unwrap();
    ledger.try_add_script_group(set(&[2, 3])).unwrap();

    assert_eq!(
        ledger.replace_descript(Some("3,0")),
        Err(StickyReject::AlreadyGrouped { scopes: vec![3] })
    );
    // 台本の組は保ち、前の descript の組は新しい値に取り替えた（作らなかった）ので無い。
    assert_eq!(ledger.groups(), &[script(&[2, 3])]);
}

#[test]
fn replace_does_not_reject_against_previous_descript_group() {
    // 前の descript の組は取り替える相手なので、重なっても拒否しない。
    let mut ledger = StickyGroupLedger::default();
    ledger.replace_descript(Some("0,1")).unwrap();
    assert_eq!(ledger.replace_descript(Some("1,0,2")), Ok(()));
    assert_eq!(ledger.groups(), &[descript(&[0, 1, 2])]);
}

// ── 読む口 ──

#[test]
fn group_of_and_partners_of() {
    let ledger = ledger_with_script(&[&[2, 0, 1], &[5, 4]]);
    assert_eq!(ledger.group_of(1), Some(&script(&[0, 1, 2])));
    assert_eq!(ledger.group_of(4), Some(&script(&[4, 5])));
    assert_eq!(ledger.group_of(3), None);
    assert_eq!(ledger.partners_of(1), vec![0, 2]);
    assert_eq!(ledger.partners_of(0), vec![1, 2]);
    assert_eq!(ledger.partners_of(5), vec![4]);
    assert!(ledger.partners_of(3).is_empty());
}
