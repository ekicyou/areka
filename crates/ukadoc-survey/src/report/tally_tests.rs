//! `tally.rs` の在中テスト。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない。
//!
//! 状態の呼び名は [`Status::as_japanese`] が正本なので、期待値はそこから引かず
//! **逐語の文字列**で書く。実装側の対応表を参照すると表を表自身と比べるだけになり、
//! 綴りの取り違えを 1 件も捕まえられない。

use std::collections::BTreeMap;

use super::{StatusCounts, UNKNOWN_GENERATION, generation_of, status_counts, tally};
use crate::ledger::{Ledger, LedgerEntry};
use crate::model::{Domain, EntryId, PageName, Status};

fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は 2 形のいずれかのはず")
}

fn entry(raw: &str, status: Status, introduced: &str, values: &[&str]) -> LedgerEntry {
    LedgerEntry {
        id: id(raw),
        status,
        introduced: introduced.to_owned(),
        alias_of: None,
        supersedes: Vec::new(),
        owner: String::new(),
        priority: String::new(),
        values: values.iter().map(|value| (*value).to_owned()).collect(),
        links: Vec::new(),
        note: String::new(),
    }
}

/// 項目の列から台帳を組む。
///
/// `file_order` には渡された順をそのまま入れる。ただし [`tally`] は `file_order` を
/// 読まず、項目は `BTreeMap` に入って id 順になるので、**渡す順は集計に届かない**。
/// 本文の順に引きずられる実装をここで捕まえることはできない（並びの主張を背負うのは
/// [`the_page_distribution_is_in_name_order_even_when_id_order_disagrees`] と
/// [`the_generation_distribution_is_keyed_by_generation_in_name_order`]）。
fn ledger_of(entries: Vec<LedgerEntry>) -> Ledger {
    let file_order = entries.iter().map(|entry| entry.id.clone()).collect();
    let map = entries
        .into_iter()
        .map(|entry| (entry.id.clone(), entry))
        .collect();
    Ledger {
        domain: Domain::Property,
        pages: vec![PageName::new("page_one"), PageName::new("page_two")],
        entries: map,
        file_order,
    }
}

/// 見本の台帳。
///
/// - 2 ページ（`page_one` 4 件・`page_two` 2 件）を **id の昇順とは逆の順**で渡す。
/// - 状態は 7 語彙のうち 5 つだけを使う（`absent` と `not-applicable` は 0 件）。
/// - `introduced` は 3 節・2 節・4 節・空文字を混ぜ、世代の並びが**本文の順とも
///   id の順とも食い違う**ようにしてある（id の順では `世代不明`（`page_one:charlie`）が
///   最初に現れるが、名前順では最後）。
/// - テーマは 8 つのうち 3 つだけを使う。
fn sample() -> Ledger {
    ledger_of(vec![
        entry(
            "ukadoc:page_two:alpha:1",
            Status::Implemented,
            "2.3.53",
            &["気配", "更新"],
        ),
        entry("ukadoc:page_two:bravo:1", Status::Degraded, "2.9", &[]),
        entry(
            "ukadoc:page_one:charlie:1",
            Status::Unclassified,
            "",
            &["気配"],
        ),
        entry("ukadoc:page_one:delta:1", Status::Alias, "2.10.5", &[]),
        entry(
            "ukadoc:page_one:echo:1",
            Status::Implemented,
            "2.3",
            &["触れ合い"],
        ),
        entry(
            "ukadoc:page_one:foxtrot:1",
            Status::VocabularyOnly,
            "2.3.53.9",
            &[],
        ),
    ])
}

/// 分布を「日本語の呼び名, 件数」の列に開く。並びは状態の語彙の順のはず。
fn japanese(counts: &StatusCounts) -> Vec<(&'static str, usize)> {
    counts.japanese_rows()
}

/// ページ別の表の鍵だけを並べる。
fn page_keys(map: &BTreeMap<PageName, StatusCounts>) -> Vec<&str> {
    map.keys().map(PageName::as_str).collect()
}

/// 名前を鍵にした表の鍵だけを並べる。
fn name_keys(map: &BTreeMap<String, StatusCounts>) -> Vec<&str> {
    map.keys().map(String::as_str).collect()
}

// ---- 状態の分布 ----

#[test]
fn the_overall_distribution_lists_all_seven_statuses_in_the_frozen_order() {
    let got = tally(&sample());
    // 呼び名は平易な日本語。並びは状態の語彙の順。
    // `未対応` と `対象外` は 0 件だが**欄ごと消えない**——消えると「0 件」と
    // 「そもそも数えていない」が区別できなくなる。
    assert_eq!(
        japanese(&got.overall),
        vec![
            ("実装済み", 2),
            ("語彙のみ", 1),
            ("縮退", 1),
            ("未対応", 0),
            ("別名", 1),
            ("対象外", 0),
            ("未分類", 1),
        ]
    );
    assert_eq!(got.overall.total(), 6);
}

#[test]
fn the_overall_distribution_can_be_read_by_status_value() {
    let got = tally(&sample());
    assert_eq!(got.overall.get(Status::Implemented), 2);
    assert_eq!(got.overall.get(Status::Absent), 0);
    assert_eq!(got.overall.get(Status::Unclassified), 1);
}

#[test]
fn every_status_has_a_row_of_its_own() {
    // `Status::ALL` の 7 語彙それぞれを 1 つずつ数えると、どの行も 1 になる。
    // どれか 1 語彙に行が無ければ合計が 7 に届かない。
    let mut counts = StatusCounts::new();
    for status in Status::ALL {
        counts.add(status);
    }
    assert_eq!(counts.total(), 7);
    assert_eq!(counts.rows().len(), 7);
    for (_, count) in counts.rows() {
        assert_eq!(*count, 1);
    }
}

// ---- ページ別の分布 ----

#[test]
fn the_page_distribution_is_in_name_order_even_when_id_order_disagrees() {
    // ページ名は id の 2 番目の区切りだが、**id 順とページ名順は一致しない**。
    // 区切りのコロン（0x3A）より数字（0x30）の方が小さいので、`a0` を持つ id は
    // `a` を持つ id より前に来る。ページ名としては `a` が先である。
    //
    // 実データに今この形の対は無いが、`spec_shiori3` は既にあるので、`spec_shiori` の
    // ページにアンカー付きの項目（`ukadoc:spec_shiori:x:1` の形）が入ればその日に生じる
    // （アンカー無しの `ukadoc:spec_shiori` なら id 順でも前に並び、ページ名の順と一致する）。
    let ledger = ledger_of(vec![
        entry("ukadoc:a0:x:1", Status::Implemented, "", &[]),
        entry("ukadoc:a:y:1", Status::Absent, "", &[]),
    ]);

    // 前提の確認——この見本が本当に 2 つの並びを食い違わせていること。
    let in_id_order: Vec<&str> = ledger
        .entries
        .values()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(in_id_order, vec!["ukadoc:a0:x:1", "ukadoc:a:y:1"]);

    let got = tally(&ledger);
    assert_eq!(page_keys(&got.by_page), vec!["a", "a0"]);
}

#[test]
fn each_page_carries_the_full_seven_status_rows() {
    let got = tally(&sample());
    let page_one = got
        .by_page
        .get(&PageName::new("page_one"))
        .expect("見本には page_one がある");
    assert_eq!(
        japanese(page_one),
        vec![
            ("実装済み", 1),
            ("語彙のみ", 1),
            ("縮退", 0),
            ("未対応", 0),
            ("別名", 1),
            ("対象外", 0),
            ("未分類", 1),
        ]
    );

    let page_two = got
        .by_page
        .get(&PageName::new("page_two"))
        .expect("見本には page_two がある");
    assert_eq!(
        japanese(page_two),
        vec![
            ("実装済み", 1),
            ("語彙のみ", 0),
            ("縮退", 1),
            ("未対応", 0),
            ("別名", 0),
            ("対象外", 0),
            ("未分類", 0),
        ]
    );
}

#[test]
fn the_page_distribution_says_where_the_unclassified_entries_are() {
    // 未分類の件数はここが正。台帳側の宣言値は持たない。
    let got = tally(&sample());
    let unclassified: Vec<(String, usize)> = got
        .by_page
        .iter()
        .map(|(page, counts)| (page.as_str().to_owned(), counts.get(Status::Unclassified)))
        .collect();
    assert_eq!(
        unclassified,
        vec![("page_one".to_owned(), 1), ("page_two".to_owned(), 0)]
    );
}

#[test]
fn a_page_with_no_entry_does_not_appear() {
    // ページは id の 2 番目の区切りから取る。前置きの `pages` は
    // 見ない——前置きと id の食い違いは整合検査（`LedgerPagesMismatch`）の担当で、
    // 集計が黙って埋めてしまうと食い違いが見えなくなる。
    let mut ledger = ledger_of(vec![entry(
        "ukadoc:page_one:charlie:1",
        Status::Unclassified,
        "",
        &[],
    )]);
    ledger.pages = vec![PageName::new("page_one"), PageName::new("page_two")];
    let got = tally(&ledger);
    assert_eq!(page_keys(&got.by_page), vec!["page_one"]);
}

// ---- 世代別の分布 ----

#[test]
fn the_generation_is_the_first_two_segments_of_the_introduced_version() {
    assert_eq!(generation_of("2.3.53"), "2.3");
    assert_eq!(generation_of("2.9"), "2.9");
    assert_eq!(generation_of("2.3.53.9"), "2.3");
    assert_eq!(generation_of("2"), "2");
    assert_eq!(generation_of(""), "世代不明");
    assert_eq!(UNKNOWN_GENERATION, "世代不明");
}

#[test]
fn the_generation_distribution_is_keyed_by_generation_in_name_order() {
    let got = tally(&sample());
    // id の順では `世代不明`（`page_one:charlie`）が最初に現れるが、名前順では最後に来る。
    assert_eq!(
        name_keys(&got.by_generation),
        vec!["2.10", "2.3", "2.9", "世代不明"]
    );
}

#[test]
fn each_generation_carries_the_status_distribution_of_its_entries() {
    let got = tally(&sample());
    let generation = |name: &str| {
        got.by_generation
            .get(name)
            .map(japanese)
            .expect("見本にある世代のはず")
    };

    // 2.3 は 3 節（実装済み）・2 節（実装済み）・4 節（語彙のみ）の 3 件。
    assert_eq!(
        generation("2.3"),
        vec![
            ("実装済み", 2),
            ("語彙のみ", 1),
            ("縮退", 0),
            ("未対応", 0),
            ("別名", 0),
            ("対象外", 0),
            ("未分類", 0),
        ]
    );
    assert_eq!(generation("2.9")[2], ("縮退", 1));
    assert_eq!(generation("2.10")[4], ("別名", 1));
    assert_eq!(generation("世代不明")[6], ("未分類", 1));
}

// ---- テーマ別の状態分布 ----

#[test]
fn only_the_themes_an_entry_wears_appear() {
    let got = tally(&sample());
    // 並びは名前順。
    // 1 件も使われていないテーマの 0 件の行は報告の側が並べる（`report::domain`）。
    assert_eq!(name_keys(&got.by_theme), vec!["更新", "気配", "触れ合い"]);
}

#[test]
fn a_theme_carries_the_status_distribution_of_the_entries_that_wear_it() {
    let got = tally(&sample());
    let theme = |name: &str| {
        got.by_theme
            .get(name)
            .map(japanese)
            .expect("見本が使ったテーマは欄を持つ")
    };

    // 気配 は実装済み 1 件（alpha）と未分類 1 件（charlie）。
    assert_eq!(
        theme("気配"),
        vec![
            ("実装済み", 1),
            ("語彙のみ", 0),
            ("縮退", 0),
            ("未対応", 0),
            ("別名", 0),
            ("対象外", 0),
            ("未分類", 1),
        ]
    );
    assert_eq!(theme("更新")[0], ("実装済み", 1));
    assert_eq!(theme("触れ合い")[0], ("実装済み", 1));
}

#[test]
fn an_entry_with_two_themes_is_counted_under_both() {
    let got = tally(&sample());
    // alpha は 気配 と 更新 の 2 つを持つ。テーマ別の合計は項目数を超えてよい。
    let total: usize = got.by_theme.values().map(StatusCounts::total).sum();
    assert_eq!(total, 4);
    assert_eq!(got.overall.total(), 6);
}

#[test]
fn a_theme_name_outside_the_frozen_eight_gets_a_row_of_its_own() {
    // `ledger::read` は 8 テーマ以外の綴りを落とすので、この形はそこを通っては
    // 現れない。それでも黙って捨てない——捨てると手で組んだ台帳で件数だけが
    // 合わなくなる。
    let ledger = ledger_of(vec![entry(
        "ukadoc:page_one:charlie:1",
        Status::Absent,
        "",
        &["知らないテーマ"],
    )]);
    let got = tally(&ledger);
    assert_eq!(name_keys(&got.by_theme), vec!["知らないテーマ"]);
    assert_eq!(
        got.by_theme
            .get("知らないテーマ")
            .map(|counts| counts.get(Status::Absent))
            .expect("知らない綴りも欄を持つ"),
        1
    );
}

// ---- 分布どうしの足し合わせ ----

#[test]
fn merging_adds_the_counts_status_by_status() {
    let mut total = status_counts(sample().entries.values());
    total.merge(&status_counts(sample().entries.values()));
    assert_eq!(total.get(Status::Implemented), 4);
    assert_eq!(total.get(Status::Alias), 2);
    assert_eq!(total.get(Status::Absent), 0);
    assert_eq!(total.total(), 12);
}
