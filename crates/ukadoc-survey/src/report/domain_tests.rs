//! `domain.rs` の在中テスト。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない。
//!
//! 期待値は実装側の定数を参照せず**逐語の文字列**で書く。実装の表を実装の表自身と
//! 比べるだけのテストは、綴りの取り違えも並べ替えも 1 件も捕まえない。
//!
//! 本文の全文を釘付けするテストを 1 本置いてある。構造だけの主張（「見出しが 5 つ
//! ある」）は区切りの空白 1 個や列の並べ替えを捕まえられず、しかも新しさの検査
//! （`check::freshness`）は本文をバイト単位で比べるので、**版面そのものが契約**である。

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

/// 見本の台帳。報告の 5 項目すべてに中身が出るように組んである。
///
/// - **2 ページ**を id の昇順とは逆の順で渡す（並びを主張する見本は 2 要素以上・かつ
///   非整列であること）。
/// - **未分類はページごとに違う件数**（`page_one` 2 件・`page_two` 1 件）。件数が同じ
///   だと、ページ別の表と合計 1 行の区別が付かない。
/// - `introduced` は世代の名前順が**版番号の数値順とも本文の順とも食い違う**ように
///   混ぜてある（`2.10` が `2.9` より前・空文字が最後）。
/// - `echo` は**1 つの id から 2 本の関連**を出す（元 id ごとに 1 本の入れ物で集めると
///   2 本目以降が黙って落ちる）。
/// - `golf` の関連は**この台帳に無い id** へ伸びる（絞り込みの負の対照）。
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
/// 繋いで組む（本文は改行だけ）。
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

// ---- 版面の逐語の釘 ----

#[test]
fn the_body_is_pinned_verbatim() {
    // 版面そのものが契約である（新しさの検査はバイト単位で比べる）。
    assert_eq!(render_domain(&sample(), &THEMES), expected_body());
}

// ---- 完了条件の後半: 未分類がどのページに何件あるか ----

#[test]
fn the_page_table_is_in_name_order_even_when_id_order_disagrees() {
    // 数字（0x30）は区切りのコロン（0x3A）より小さいので、`a0` を持つ id は `a` を
    // 持つ id より前に来る。ページ名としては `a` が先である（バイト順の比べ）。
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

// ---- 見出し ----

#[test]
fn the_heading_names_this_ledgers_domain() {
    // 綴りを焼き付けていないこと（逐語の本文は property だけなので、別のドメインを
    // 渡せば見出しが変わることをここで見る）。
    let shiori = ledger_of(
        Domain::Shiori,
        &["page_one"],
        &[row("ukadoc:page_one:x:1", Status::Implemented, "")],
    );
    let body = render_domain(&shiori, &THEMES);
    assert!(body.starts_with("# shiori の網羅状況\n"), "{body}");
}

// ---- ⑶ 別名の一覧 ----

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

// ---- ⑷ テーマ別の状態分布（8 テーマ） ----

#[test]
fn a_theme_name_that_only_the_argument_carries_still_gets_a_row() {
    // テーマ名は引数から来る。
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

// ---- ⑸ ドメイン内で関連が閉じている束 ----

#[test]
fn several_links_leaving_the_same_id_all_become_edges() {
    // 関連を「元 id ごとに 1 本」の入れ物で集めると、
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
    // 束は `links` だけで作る。`alias_of` は ⑶ の一覧が受け持つ。
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
