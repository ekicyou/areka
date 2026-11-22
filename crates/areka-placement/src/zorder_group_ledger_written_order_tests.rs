//! 明示の記法を**書かれた順のまま**受け取る解釈のテスト。
//!
//! - 並びは書かれた順のまま。片方だけ名指しのスコープにだけ相棒を 1 枚補う
//!   （`sN` だけならすぐ手前に `bN`、`bN` だけならすぐ奥に `sN`）。
//! - 2 枚とも書かれたスコープの並びの形を 4 通り（隣り合い・逆転・非隣接・逆転かつ非隣接）に
//!   分けて記録する。補ったスコープは常に隣り合い。
//! - 判定の順序（解釈不能 → 記法の混在 → 要素 2 個未満 → 数字だけの記法の展開 → 同じ窓の重複）
//!   と拒否の 5 種は固定されている。
//!
//! 純関数のテストなので、単独で回しても一括で回しても結果は同じである。

use super::*;
use crate::zorder_test_support::{b, expect_accept, expect_reject, implied, s, written};

// ---------------------------------------------------------------------------
// 書かれた順のまま採る
// ---------------------------------------------------------------------------

/// ぱすたと同じ指定 `s0,b0,s1`: 並びは書かれた順のまま、`s1` の手前に `b1` が補われる。
/// スコープ 0 は逆転、スコープ 1 は補った隣り合い。
#[test]
fn pasta_inversion_keeps_written_order_and_fills_b1_before_s1() {
    let (elements, records) = expect_accept(&["s0", "b0", "s1"]);

    assert_eq!(
        elements,
        vec![s(0), b(0), b(1), s(1)],
        "書かれた順が組み替えられたか、b1 が s1 のすぐ手前に補われていない"
    );
    assert_eq!(
        records,
        vec![
            written(0, WrittenOrder::Inverted),
            implied(1, GroupWindowKind::Balloon),
        ]
    );
}

/// 非隣接 `b1,s0,s1,b0`: 4 枚とも書かれた順のまま。スコープ 1 は非隣接、
/// スコープ 0 は逆転かつ非隣接。記録はスコープの初出順。
#[test]
fn split_order_is_kept_verbatim_and_classified() {
    let (elements, records) = expect_accept(&["b1", "s0", "s1", "b0"]);

    assert_eq!(elements, vec![b(1), s(0), s(1), b(0)], "書かれた順のまま");
    assert_eq!(
        records,
        vec![
            written(1, WrittenOrder::Split),
            written(0, WrittenOrder::InvertedSplit),
        ]
    );
}

/// 隣り合った逆転 `s1,b1` はそのまま採る。
#[test]
fn adjacent_inversion_is_kept() {
    let (elements, records) = expect_accept(&["s1", "b1"]);

    assert_eq!(elements, vec![s(1), b(1)]);
    assert_eq!(records, vec![written(1, WrittenOrder::Inverted)]);
}

/// バルーンのすぐ奥にキャラクターを書いたスコープは隣り合い。
#[test]
fn balloon_then_char_stays_adjacent() {
    let (elements, records) = expect_accept(&["b0", "s0", "b1", "s1"]);

    assert_eq!(elements, vec![b(0), s(0), b(1), s(1)]);
    assert_eq!(
        records,
        vec![
            written(0, WrittenOrder::Adjacent),
            written(1, WrittenOrder::Adjacent),
        ]
    );
}

/// 記録の語は 4 通り。隣り合いは既存の読み手のために `false` のまま。
#[test]
fn written_order_words() {
    assert_eq!(WrittenOrder::Adjacent.as_str(), "false");
    assert_eq!(WrittenOrder::Inverted.as_str(), "inverted");
    assert_eq!(WrittenOrder::Split.as_str(), "split");
    assert_eq!(WrittenOrder::InvertedSplit.as_str(), "inverted-split");
}

// ---------------------------------------------------------------------------
// 片方だけの名指しの補い
// ---------------------------------------------------------------------------

/// 補いの 2 つの位置: `bN` だけならすぐ奥に `sN`、`sN` だけならすぐ手前に `bN`。補いは他の窓の順に触れない。
#[test]
fn partner_is_filled_right_behind_a_balloon_and_right_before_a_char() {
    let (elements, records) = expect_accept(&["b1", "s0"]);
    assert_eq!(elements, vec![b(1), s(1), b(0), s(0)]);
    assert_eq!(
        records,
        vec![
            implied(1, GroupWindowKind::Char),
            implied(0, GroupWindowKind::Balloon),
        ]
    );

    // 補った窓の挟まりで離れたスコープは、補った後の並びで数えて非隣接になる。
    let (elements, records) = expect_accept(&["s3", "b0", "b2", "s0"]);
    assert_eq!(elements, vec![b(3), s(3), b(0), b(2), s(2), s(0)]);
    assert_eq!(
        records,
        vec![
            implied(3, GroupWindowKind::Balloon),
            written(0, WrittenOrder::Split),
            implied(2, GroupWindowKind::Char),
        ]
    );
}

/// 2 枚とも書けば補わない。窓の枚数は書いた数のまま。
#[test]
fn both_windows_written_means_no_fill() {
    for tokens in [
        ["s0", "b0"].as_slice(),
        ["b0", "s0"].as_slice(),
        ["b0", "s1", "s0", "b1"].as_slice(),
    ] {
        let (elements, records) = expect_accept(tokens);
        assert_eq!(elements.len(), tokens.len(), "補いすぎ: {tokens:?}");
        assert!(
            records.iter().all(|n| n.implied_partner.is_none()),
            "2 枚とも書いたのに補いの記録が出た: {tokens:?} {records:?}"
        );
    }
}

/// 補いの記録は常に隣り合い（`Some(_)` なら `Adjacent`）。記録はスコープごとに初出順で 1 件。
#[test]
fn every_explicit_scope_is_recorded_once_and_fills_are_adjacent() {
    for tokens in [
        ["s0", "b0", "s1"].as_slice(),
        ["b1", "s0", "s1", "b0"].as_slice(),
        ["s3", "b0", "b2", "s0"].as_slice(),
        ["s0", "s1", "s2"].as_slice(),
        ["b0", "b1", "b2"].as_slice(),
    ] {
        let (elements, records) = expect_accept(tokens);
        for record in &records {
            if record.implied_partner.is_some() {
                assert_eq!(
                    record.order,
                    WrittenOrder::Adjacent,
                    "{tokens:?} {record:?}"
                );
            }
        }
        let mut scopes: Vec<u32> = Vec::new();
        for element in &elements {
            if !scopes.contains(&element.scope) {
                scopes.push(element.scope);
            }
        }
        let recorded: Vec<u32> = records.iter().map(|n| n.scope).collect();
        assert_eq!(recorded, scopes, "{tokens:?}");
    }
}

// ---------------------------------------------------------------------------
// 数字だけの記法・拒否（完全形と省略形の同義は兄弟の `zorder_group_ledger_tests.rs` の `explicit_short_form_equals_long_form_and_numeric_expansion` が受け持つ）
// ---------------------------------------------------------------------------

/// 数字だけの記法は `[バルーン, キャラ]` へ展開し、記録は空。
#[test]
fn numeric_mode_expands_as_before_with_no_record() {
    let (elements, records) = expect_accept(&["1", "0"]);
    assert_eq!(elements, vec![b(1), s(1), b(0), s(0)]);
    assert!(records.is_empty(), "{records:?}");
}

/// 拒否 5 種。解釈の 4 種に加え、台帳の再指定の拒否も
/// 逆転の並びで変わらない。
#[test]
fn the_five_rejections_are_unchanged() {
    assert_eq!(
        expect_reject(&["s0", "xyz"]),
        ZOrderReject::UnparsableToken {
            token: "xyz".to_owned()
        }
    );
    assert_eq!(expect_reject(&["s0", "1"]), ZOrderReject::ModeMixed);
    assert_eq!(
        expect_reject(&["s0"]),
        ZOrderReject::TooFewElements { count: 1 }
    );
    assert_eq!(
        expect_reject(&["s0", "b0", "s0"]),
        ZOrderReject::DuplicateElement { element: s(0) }
    );

    let mut ledger = ZOrderGroupLedger::default();
    let (first, _) = expect_accept(&["s0", "b0", "s1"]);
    ledger.try_add_tag_group(first).expect("1 本目は受理");
    let (second, _) = expect_accept(&["s1", "b2"]);
    assert_eq!(
        ledger.try_add_tag_group(second),
        Err(ZOrderReject::CrossGroupRedesignation { scopes: vec![1] })
    );
    assert_eq!(ledger.groups().len(), 1, "拒否で台帳が動いた");
}
