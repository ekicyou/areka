//! `domain.rs` の在中テスト。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない
//! （要件 6.2・設計 File Structure Plan）。
//!
//! 期待値は実装側の定数を参照せず**逐語の文字列**で書く。実装の表を実装の表自身と
//! 比べるだけのテストは、綴りの取り違えも並べ替えも 1 件も捕まえない（タスク 1.5 の
//! 教訓）。
//!
//! 本文の全文を釘付けするテストを 1 本置いてある。構造だけの主張（「見出しが 5 つ
//! ある」）は区切りの空白 1 個や列の並べ替えを捕まえられず、しかも新しさの検査
//! （タスク 4.4）は本文をバイト単位で比べるので、**版面そのものが契約**である
//! （タスク 1.4 の教訓）。

use super::render_domain;
use crate::ledger::{Ledger, LedgerEntry};
use crate::model::{Domain, EntryId, Link, LinkKind, PageName, Status, THEMES};

fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は 2 形のいずれかのはず")
}

/// 見本の 1 項目。欄は必要なものだけを名前で渡す。
struct Row {
    id: &'static str,
    status: Status,
    introduced: &'static str,
    alias_of: Option<&'static str>,
    values: &'static [&'static str],
    links: &'static [(LinkKind, &'static str)],
}

fn row(id: &'static str, status: Status, introduced: &'static str) -> Row {
    Row {
        id,
        status,
        introduced,
        alias_of: None,
        values: &[],
        links: &[],
    }
}

fn entry_of(row: &Row) -> LedgerEntry {
    LedgerEntry {
        id: id(row.id),
        status: row.status,
        introduced: row.introduced.to_owned(),
        alias_of: row.alias_of.map(id),
        supersedes: Vec::new(),
        owner: String::new(),
        priority: String::new(),
        values: row.values.iter().map(|value| (*value).to_owned()).collect(),
        links: row
            .links
            .iter()
            .map(|(kind, to)| Link {
                kind: *kind,
                to: id(to),
            })
            .collect(),
        note: String::new(),
    }
}

/// 見本の台帳を組む。
///
/// `file_order` には渡された順をそのまま入れる。[`render_domain`] は本文の順を読まない
/// はずなので、渡す順は答えに届かない。
fn ledger_of(domain: Domain, pages: &[&str], rows: &[Row]) -> Ledger {
    let entries: Vec<LedgerEntry> = rows.iter().map(entry_of).collect();
    let file_order = entries.iter().map(|entry| entry.id.clone()).collect();
    let map = entries
        .into_iter()
        .map(|entry| (entry.id.clone(), entry))
        .collect();
    Ledger {
        domain,
        pages: pages.iter().map(|page| PageName::new(*page)).collect(),
        entries: map,
        file_order,
    }
}

/// 見本の台帳。要件 7.1 の 5 項目すべてに中身が出るように組んである。
///
/// - **2 ページ**を id の昇順とは逆の順で渡す（並びを主張する見本は 2 要素以上・かつ
///   非整列であること・タスク 2.5 の教訓）。
/// - **未分類はページごとに違う件数**（`page_one` 2 件・`page_two` 1 件）。件数が同じ
///   だと、ページ別の表と合計 1 行の区別が付かない（要件 6.9）。
/// - `introduced` は世代の名前順が**版番号の数値順とも本文の順とも食い違う**ように
///   混ぜてある（`2.10` が `2.9` より前・空文字が最後）。
/// - `echo` は**1 つの id から 2 本の関連**を出す（タスク 3.4 からの申し送り）。
/// - `golf` の関連は**この台帳に無い id** へ伸びる（設計 D-11 の絞り込みの負の対照）。
fn sample() -> Ledger {
    ledger_of(
        Domain::Property,
        &["page_two", "page_one"],
        &[
            Row {
                values: &["気配", "更新"],
                links: &[(LinkKind::SameFeature, "ukadoc:page_one:charlie:1")],
                ..row("ukadoc:page_two:alpha:1", Status::Implemented, "2.3.53")
            },
            row("ukadoc:page_two:bravo:1", Status::Unclassified, ""),
            Row {
                links: &[(LinkKind::SameFeature, "ukadoc:other_page:zulu:1")],
                ..row("ukadoc:page_two:golf:1", Status::Absent, "")
            },
            row("ukadoc:page_one:charlie:1", Status::Unclassified, "2.9"),
            Row {
                alias_of: Some("ukadoc:page_one:echo:1"),
                ..row("ukadoc:page_one:delta:1", Status::Alias, "2.10.5")
            },
            Row {
                values: &["触れ合い"],
                links: &[
                    (LinkKind::Triggers, "ukadoc:page_two:alpha:1"),
                    (LinkKind::Queries, "ukadoc:page_one:delta:1"),
                ],
                ..row("ukadoc:page_one:echo:1", Status::Implemented, "2.3")
            },
            row("ukadoc:page_one:foxtrot:1", Status::Unclassified, ""),
        ],
    )
}

/// 見本の本文の逐語の期待値。
///
/// テストファイルの改行が復帰文字に化けても期待値が変わらないよう、行の列を `\n` で
/// 繋いで組む（本文は改行だけ・設計 D-6）。
fn expected_body() -> String {
    let lines = [
        "# property の網羅状況",
        "",
        "この本文は台帳から機械で作ります。手で書き換えず、食い違いは作り直して直します。",
        "",
        "## 状態の分布",
        "",
        "| 状態 | 件数 |",
        "| --- | ---: |",
        "| 実装済み | 2 |",
        "| 語彙のみ | 0 |",
        "| 縮退 | 0 |",
        "| 未対応 | 1 |",
        "| 別名 | 1 |",
        "| 対象外 | 0 |",
        "| 未分類 | 3 |",
        "| 合計 | 7 |",
        "",
        "## ページ別の状態の分布",
        "",
        "未分類の残りがどのページに何件あるかは、この表の「未分類」の列が正です。",
        "",
        "| ページ | 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        "| page_one | 1 | 0 | 0 | 0 | 1 | 0 | 2 | 4 |",
        "| page_two | 1 | 0 | 0 | 1 | 0 | 0 | 1 | 3 |",
        "",
        "## SSP 世代別の対応表",
        "",
        "| 世代 | 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        "| 2.10 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 1 |",
        "| 2.3 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |",
        "| 2.9 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 1 |",
        "| 世代不明 | 0 | 0 | 0 | 1 | 0 | 0 | 2 | 3 |",
        "",
        "## 別名の一覧",
        "",
        "| 別名の id | 指す先の id |",
        "| --- | --- |",
        "| ukadoc:page_one:delta:1 | ukadoc:page_one:echo:1 |",
        "",
        "## テーマ別の状態分布",
        "",
        "| テーマ | 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        "| 交わり | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |",
        "| 掛け合い | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |",
        "| 更新 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |",
        "| 気配 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |",
        "| 気配り | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |",
        "| 装い | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |",
        "| 触れ合い | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |",
        "| 記憶 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |",
        "",
        "## ドメイン内で関連が閉じている束",
        "",
        "| 束 id | 構成 id |",
        "| --- | --- |",
        "| ukadoc:page_one:charlie:1 | ukadoc:page_one:charlie:1, ukadoc:page_one:delta:1, ukadoc:page_one:echo:1, ukadoc:page_two:alpha:1 |",
        "",
    ];
    lines.join("\n")
}

/// 見出しの行を丸ごと探す（節の頭出し）。
fn section_at(body: &str, heading: &str) -> usize {
    let needle = format!("\n{heading}\n");
    body.find(&needle)
        .unwrap_or_else(|| panic!("見出し {heading} が本文に無い"))
}

/// ある見出しの節の本文（次の見出しまで）。
fn section_body(body: &str, heading: &str) -> String {
    let start = section_at(body, heading);
    let rest = &body[start + 1..];
    match rest[heading.len()..].find("\n## ") {
        Some(end) => rest[..heading.len() + end].to_owned(),
        None => rest.to_owned(),
    }
}

// ---- 完了条件の前半: 2 回作ると同じ本文（要件 7.3） ----

#[test]
fn rendering_the_same_ledger_twice_gives_the_same_body() {
    let ledger = sample();
    let first = render_domain(&ledger, &THEMES);
    let second = render_domain(&ledger, &THEMES);
    assert_eq!(first, second);
    // 空文字どうしを比べて満足しないこと（否定・同一の主張は対象が空だと恒真）。
    assert!(!first.is_empty());
}

#[test]
fn the_body_is_pinned_verbatim() {
    // 版面そのものが契約である（新しさの検査はバイト単位で比べる・要件 7.4）。
    assert_eq!(render_domain(&sample(), &THEMES), expected_body());
}

#[test]
fn the_body_writes_only_line_feeds() {
    let body = render_domain(&sample(), &THEMES);
    // バイトを直に数える（`grep -c` の類は空パターンに退化して嘘をつく）。
    assert_eq!(
        body.as_bytes()
            .iter()
            .filter(|byte| **byte == b'\r')
            .count(),
        0
    );
    assert!(
        body.as_bytes()
            .iter()
            .filter(|byte| **byte == b'\n')
            .count()
            > 20
    );
}

#[test]
fn the_body_carries_no_wall_clock() {
    // 生成日時を書かない（要件 7.3・設計 Data Models）。この見本に現れる数字は
    // 件数（1 桁）と版番号（`2.10` など）だけなので、4 桁続く数字があれば年号など
    // 時刻由来のものが混ざったことになる。
    let body = render_domain(&sample(), &THEMES);
    let mut run = 0usize;
    for ch in body.chars() {
        run = if ch.is_ascii_digit() { run + 1 } else { 0 };
        assert!(run < 4, "本文に 4 桁続く数字がある: {body}");
    }
}

// ---- 完了条件の後半: 未分類がどのページに何件あるか（要件 6.9） ----

#[test]
fn the_page_table_says_how_many_unclassified_entries_remain_on_each_page() {
    let body = render_domain(&sample(), &THEMES);
    let section = section_body(&body, "## ページ別の状態の分布");

    // 見本の未分類は page_one 2 件・page_two 1 件（**件数が違う**ので、ページ別の表と
    // 合計 1 行を区別できる）。
    assert!(
        section.contains("| page_one | 1 | 0 | 0 | 0 | 1 | 0 | 2 | 4 |"),
        "{section}"
    );
    assert!(
        section.contains("| page_two | 1 | 0 | 0 | 1 | 0 | 0 | 1 | 3 |"),
        "{section}"
    );
    // 合計だけを載せた表に落ちていないこと。合計 3 件はこの節に現れない。
    assert!(!section.contains("| 合計 | 3 |"), "{section}");
}

#[test]
fn the_page_table_is_in_name_order_even_when_id_order_disagrees() {
    // 数字（0x30）は区切りのコロン（0x3A）より小さいので、`a0` を持つ id は `a` を
    // 持つ id より前に来る。ページ名としては `a` が先である（タスク 3.4 の実測）。
    let ledger = ledger_of(
        Domain::Shiori,
        &["a", "a0"],
        &[
            row("ukadoc:a0:x:1", Status::Implemented, ""),
            row("ukadoc:a:y:1", Status::Absent, ""),
        ],
    );
    // 前提の確認——この見本が本当に 2 つの並びを食い違わせている。
    let in_id_order: Vec<&str> = ledger
        .entries
        .values()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(in_id_order, vec!["ukadoc:a0:x:1", "ukadoc:a:y:1"]);

    let section = section_body(&render_domain(&ledger, &THEMES), "## ページ別の状態の分布");
    let a = section.find("| a |").expect("ページ a の行が無い");
    let a0 = section.find("| a0 |").expect("ページ a0 の行が無い");
    assert!(a < a0, "{section}");
}

// ---- 見出しの構成と並び（設計 report 節の 5 項目） ----

#[test]
fn the_five_items_appear_in_the_order_the_design_gives() {
    let body = render_domain(&sample(), &THEMES);
    let order = [
        section_at(&body, "## 状態の分布"),
        section_at(&body, "## ページ別の状態の分布"),
        section_at(&body, "## SSP 世代別の対応表"),
        section_at(&body, "## 別名の一覧"),
        section_at(&body, "## テーマ別の状態分布"),
        section_at(&body, "## ドメイン内で関連が閉じている束"),
    ];
    let mut sorted = order;
    sorted.sort_unstable();
    assert_eq!(order.to_vec(), sorted.to_vec());
}

#[test]
fn the_heading_names_this_ledgers_domain() {
    // 綴りを焼き付けていないこと（別のドメインを渡せば見出しが変わる）。
    let property = render_domain(&sample(), &THEMES);
    assert!(
        property.starts_with("# property の網羅状況\n"),
        "{property}"
    );

    let shiori = ledger_of(
        Domain::Shiori,
        &["page_one"],
        &[row("ukadoc:page_one:x:1", Status::Implemented, "")],
    );
    let body = render_domain(&shiori, &THEMES);
    assert!(body.starts_with("# shiori の網羅状況\n"), "{body}");
}

// ---- ⑴ 状態の分布（要件 7.8） ----

#[test]
fn the_overall_distribution_uses_the_plain_japanese_names_in_the_frozen_order() {
    let section = section_body(&render_domain(&sample(), &THEMES), "## 状態の分布");
    let labels: Vec<&str> = section
        .lines()
        .filter_map(|line| line.strip_prefix("| "))
        .filter_map(|line| line.split(" | ").next())
        .filter(|label| *label != "状態" && !label.starts_with("---"))
        .collect();
    assert_eq!(
        labels,
        vec![
            "実装済み",
            "語彙のみ",
            "縮退",
            "未対応",
            "別名",
            "対象外",
            "未分類",
            "合計",
        ]
    );
}

#[test]
fn the_wide_tables_head_their_columns_with_the_plain_japanese_names() {
    // ページ・世代・テーマの 3 表は列に 7 語彙を並べる。呼び名は要件 7.8 の平易な
    // 日本語（英字の綴りではない）で、並びは要件 2.2 の順。
    let body = render_domain(&sample(), &THEMES);
    let expected = "| 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |";
    for (heading, first) in [
        ("## ページ別の状態の分布", "ページ"),
        ("## SSP 世代別の対応表", "世代"),
        ("## テーマ別の状態分布", "テーマ"),
    ] {
        let section = section_body(&body, heading);
        assert!(
            section.contains(&format!("| {first} {expected}")),
            "{section}"
        );
        // 英字の綴りが漏れていないこと（`implemented` などは台帳側の綴り）。
        assert!(!section.contains("implemented"), "{section}");
        assert!(!section.contains("not-applicable"), "{section}");
    }
}

#[test]
fn the_overall_distribution_keeps_the_zero_rows() {
    // 0 件の欄を消すと「0 件だった」と「そもそも数えていない」が区別できない。
    let section = section_body(&render_domain(&sample(), &THEMES), "## 状態の分布");
    assert!(section.contains("| 語彙のみ | 0 |"), "{section}");
    assert!(section.contains("| 対象外 | 0 |"), "{section}");
}

// ---- ⑵ 世代別の対応表 ----

#[test]
fn the_generation_table_names_the_unknown_generation_and_orders_by_name() {
    let section = section_body(&render_domain(&sample(), &THEMES), "## SSP 世代別の対応表");
    let names: Vec<&str> = section
        .lines()
        .filter_map(|line| line.strip_prefix("| "))
        .filter_map(|line| line.split(" | ").next())
        .filter(|name| *name != "世代" && !name.starts_with("---"))
        .collect();
    // 名前順であって数値順ではない（`2.10` が `2.9` より前）。空文字は「世代不明」で、
    // 最古の世代へ混ぜない（要件 4.2）。
    assert_eq!(names, vec!["2.10", "2.3", "2.9", "世代不明"]);
    assert!(
        section.contains("| 世代不明 | 0 | 0 | 0 | 1 | 0 | 0 | 2 | 3 |"),
        "{section}"
    );
}

// ---- ⑶ 別名の一覧 ----

#[test]
fn the_alias_list_shows_the_alias_row_and_the_id_it_points_at() {
    let section = section_body(&render_domain(&sample(), &THEMES), "## 別名の一覧");
    assert!(
        section.contains("| ukadoc:page_one:delta:1 | ukadoc:page_one:echo:1 |"),
        "{section}"
    );
    // 別名でない行は混ざらない。
    assert!(!section.contains("ukadoc:page_one:foxtrot:1"), "{section}");
}

#[test]
fn an_alias_row_without_a_target_says_so_instead_of_going_missing() {
    let ledger = ledger_of(
        Domain::Property,
        &["page_one"],
        &[row("ukadoc:page_one:x:1", Status::Alias, "")],
    );
    let section = section_body(&render_domain(&ledger, &THEMES), "## 別名の一覧");
    assert!(
        section.contains("| ukadoc:page_one:x:1 | （未設定） |"),
        "{section}"
    );
}

#[test]
fn an_empty_alias_list_says_there_is_none() {
    // 否定の主張（別名が混ざらない）と対で、空のときの版面も釘付けする。
    let ledger = ledger_of(
        Domain::Property,
        &["page_one"],
        &[row("ukadoc:page_one:x:1", Status::Implemented, "")],
    );
    let section = section_body(&render_domain(&ledger, &THEMES), "## 別名の一覧");
    assert!(section.contains("別名の項目はありません。"), "{section}");
    assert!(!section.contains("| --- |"), "{section}");
}

// ---- ⑷ テーマ別の状態分布（要件 4.4 の 8 テーマ） ----

#[test]
fn the_theme_table_lists_all_eight_themes_in_name_order_including_the_empty_ones() {
    let section = section_body(&render_domain(&sample(), &THEMES), "## テーマ別の状態分布");
    let names: Vec<&str> = section
        .lines()
        .filter_map(|line| line.strip_prefix("| "))
        .filter_map(|line| line.split(" | ").next())
        .filter(|name| *name != "テーマ" && !name.starts_with("---"))
        .collect();
    // 名前順は UTF-8 のバイト順であって五十音順ではない（タスク 3.4 の確定）。
    assert_eq!(
        names,
        vec![
            "交わり",
            "掛け合い",
            "更新",
            "気配",
            "気配り",
            "装い",
            "触れ合い",
            "記憶",
        ]
    );
    // 0 件のテーマも欄が残る（見本で使ったのは 3 つだけ）。
    assert!(
        section.contains("| 記憶 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |"),
        "{section}"
    );
    assert!(
        section.contains("| 触れ合い | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |"),
        "{section}"
    );
}

#[test]
fn a_theme_name_that_only_the_argument_carries_still_gets_a_row() {
    // テーマ名は引数から来る（設計 D-11「入力はその台帳 1 本とテーマ名だけ」）。
    // 引数を無視して定数だけを見る実装をここで捕まえる。
    let ledger = ledger_of(
        Domain::Property,
        &["page_one"],
        &[row("ukadoc:page_one:x:1", Status::Implemented, "")],
    );
    let mut themes: Vec<&str> = THEMES.to_vec();
    themes.push("あいうえお");
    let section = section_body(&render_domain(&ledger, &themes), "## テーマ別の状態分布");
    assert!(
        section.contains("| あいうえお | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |"),
        "{section}"
    );
}

// ---- ⑸ ドメイン内で関連が閉じている束（設計 D-11） ----

#[test]
fn a_bundle_whose_members_all_belong_to_this_ledger_is_listed() {
    let section = section_body(
        &render_domain(&sample(), &THEMES),
        "## ドメイン内で関連が閉じている束",
    );
    assert!(
        section.contains(
            "| ukadoc:page_one:charlie:1 | ukadoc:page_one:charlie:1, \
             ukadoc:page_one:delta:1, ukadoc:page_one:echo:1, ukadoc:page_two:alpha:1 |"
        ),
        "{section}"
    );
}

#[test]
fn a_bundle_reaching_an_id_outside_this_ledger_is_not_listed() {
    // 負の主張は必ず正の主張と対で置く（対象が空だと恒真になる・タスク 1.6 の教訓）。
    // 上のテストが「載る束が実在する」ことを言い、こちらが「伸びた束は載らない」を言う。
    let section = section_body(
        &render_domain(&sample(), &THEMES),
        "## ドメイン内で関連が閉じている束",
    );
    assert!(!section.contains("ukadoc:other_page:zulu:1"), "{section}");
    assert!(!section.contains("ukadoc:page_two:golf:1"), "{section}");
}

#[test]
fn several_links_leaving_the_same_id_all_become_edges() {
    // タスク 3.4 からの申し送り: 関連を「元 id ごとに 1 本」の入れ物で集めると、
    // 2 本目以降が黙って落ちる。`hub` は互いに繋がらない 2 つの id へ関連を出すので、
    // 辺が 1 本でも落ちれば束は 3 件にまとまらない。
    let ledger = ledger_of(
        Domain::Property,
        &["page_one"],
        &[
            Row {
                links: &[
                    (LinkKind::Triggers, "ukadoc:page_one:leaf_a:1"),
                    (LinkKind::Queries, "ukadoc:page_one:leaf_b:1"),
                ],
                ..row("ukadoc:page_one:hub:1", Status::Implemented, "")
            },
            row("ukadoc:page_one:leaf_a:1", Status::Absent, ""),
            row("ukadoc:page_one:leaf_b:1", Status::Absent, ""),
        ],
    );
    let section = section_body(
        &render_domain(&ledger, &THEMES),
        "## ドメイン内で関連が閉じている束",
    );
    assert!(
        section.contains(
            "| ukadoc:page_one:hub:1 | ukadoc:page_one:hub:1, \
             ukadoc:page_one:leaf_a:1, ukadoc:page_one:leaf_b:1 |"
        ),
        "{section}"
    );
    // 束は 1 つだけ（2 つに割れていないこと）。表の本文の行を数える。
    let rows = section
        .lines()
        .filter(|line| line.starts_with("| ukadoc:"))
        .count();
    assert_eq!(rows, 1, "{section}");
}

#[test]
fn an_empty_bundle_list_says_there_is_none() {
    let ledger = ledger_of(
        Domain::Property,
        &["page_one"],
        &[row("ukadoc:page_one:x:1", Status::Implemented, "")],
    );
    let section = section_body(
        &render_domain(&ledger, &THEMES),
        "## ドメイン内で関連が閉じている束",
    );
    assert!(
        section.contains("ドメイン内で閉じている束はありません。"),
        "{section}"
    );
}

#[test]
fn alias_of_alone_does_not_make_a_bundle() {
    // 束は `links` だけで作る（設計 D-11）。`alias_of` は ⑶ の一覧が受け持つ。
    let ledger = ledger_of(
        Domain::Property,
        &["page_one"],
        &[
            Row {
                alias_of: Some("ukadoc:page_one:new:1"),
                ..row("ukadoc:page_one:old:1", Status::Alias, "")
            },
            row("ukadoc:page_one:new:1", Status::Implemented, ""),
        ],
    );
    let body = render_domain(&ledger, &THEMES);
    // 正の対照——別名の一覧にはこの対が出ている（見本が空でないこと）。
    assert!(
        section_body(&body, "## 別名の一覧")
            .contains("| ukadoc:page_one:old:1 | ukadoc:page_one:new:1 |")
    );
    assert!(
        section_body(&body, "## ドメイン内で関連が閉じている束")
            .contains("ドメイン内で閉じている束はありません。")
    );
}
