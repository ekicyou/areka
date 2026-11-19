//! `summary.rs` の在中テスト。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない
//! （要件 6.2・設計 File Structure Plan）。
//!
//! 期待値は実装側の定数を参照せず**逐語の文字列**で書く。実装の表を実装の表自身と
//! 比べるだけのテストは、綴りの取り違えも並べ替えも 1 件も捕まえない。本文の全文を
//! 釘付けするテストを 1 本置いてある（版面そのものが契約である）。
//!
//! # 時刻はカタログの値だけ
//!
//! 全体報告に載る時刻は**カタログの生成日時 1 つだけ**で、2 回続けて描いても壁時計は
//! 見つからない（マイクロ秒差で同じ値になる）。そこで時刻の行は逐語の本文の一部として
//! 固定し、別の日時を渡すと本文がそれに変わることを
//! [`the_snapshot_time_comes_from_the_catalog_and_is_not_baked_in`] が見る。

use std::collections::BTreeMap;

use super::render_summary;
use crate::catalog::{Catalog, SnapshotMeta};
use crate::ledger::{Ledger, LedgerEntry};
use crate::model::{Domain, EntryId, Link, LinkKind, PageName, Status, THEMES};

fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は 2 形のいずれかのはず")
}

/// 見本の 1 項目。欄は必要なものだけを名前で渡す。
struct Row {
    id: &'static str,
    status: Status,
    values: &'static [&'static str],
    links: &'static [(LinkKind, &'static str)],
}

fn row(id: &'static str, status: Status) -> Row {
    Row {
        id,
        status,
        values: &[],
        links: &[],
    }
}

fn entry_of(row: &Row) -> LedgerEntry {
    LedgerEntry {
        id: id(row.id),
        status: row.status,
        introduced: String::new(),
        alias_of: None,
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

/// 見本のカタログ。全体報告が読むのは [`SnapshotMeta::generated_at`] だけだが、
/// 他の欄には**わざと 4 桁の数字**（`2983`・`1749`）を入れてある。本文に紛れ込めば
/// 逐語の比べが赤くなる。
fn catalog_of(generated_at: &str) -> Catalog {
    Catalog {
        snapshot: SnapshotMeta {
            package: "ukagaka-doc-mcp".to_owned(),
            package_version: "0.2.7".to_owned(),
            snapshot_version: 1,
            generated_at: generated_at.to_owned(),
            total_entries: 2983,
            ukadoc_entries: 1749,
            catalog_format: 1,
            hash_algorithm: "fnv1a-64".to_owned(),
        },
        entries: BTreeMap::new(),
    }
}

const GENERATED_AT: &str = "2026-08-24T04:08:57.881Z";

/// 見本の台帳 4 本。**渡す順は名前順でも要件 3.1 の並びでもない**（整列済みの見本は
/// 並び順を 1 つも守らない）。
///
/// - 項目数と未分類件数は**4 本とも違う値**（3/2・5/3・2/0・4/1）。同じ値だと台帳
///   ごとの表と合計 1 行の区別が付かない。
/// - `s:1` は**1 つの id から 2 本の関連**を出す（元 id ごとに 1 本の入れ物で集めると
///   2 本目以降が黙って落ちる）。
/// - `OnBoot:1` → `alpha:1` と `s:1` → `OnBoot:1` で 3 ドメインに跨る束が 1 つできる。
/// - `x:1` → `y:1` は property の中で閉じる（跨がない束の負の対照）。
/// - `charlie:1` → `ghost:1` の相手は**どの台帳にも無い**（所属不明の負の対照）。
fn sample_ledgers() -> Vec<Ledger> {
    vec![
        ledger_of(
            Domain::Property,
            &["list_propertysystem"],
            &[
                Row {
                    values: &["気配", "記憶"],
                    links: &[(LinkKind::SameFeature, "ukadoc:list_propertysystem:y:1")],
                    ..row("ukadoc:list_propertysystem:x:1", Status::Implemented)
                },
                row("ukadoc:list_propertysystem:y:1", Status::Unclassified),
                row("ukadoc:list_propertysystem:z:1", Status::Unclassified),
                row("ukadoc:list_propertysystem:w:1", Status::Unclassified),
                row("ukadoc:list_propertysystem:v:1", Status::NotApplicable),
            ],
        ),
        ledger_of(
            Domain::Shiori,
            &["list_shiori_event"],
            &[
                Row {
                    values: &["気配"],
                    links: &[(LinkKind::SameFeature, "ukadoc:list_shell:alpha:1")],
                    ..row("ukadoc:list_shiori_event:OnBoot:1", Status::Implemented)
                },
                row("ukadoc:list_shiori_event:OnClose:1", Status::Unclassified),
                row("ukadoc:list_shiori_event:OnTalk:1", Status::Absent),
                row("ukadoc:list_shiori_event:OnMenuExec:1", Status::Degraded),
            ],
        ),
        ledger_of(
            Domain::SakuraScript,
            &["list_sakura_script"],
            &[
                Row {
                    values: &["掛け合い"],
                    links: &[
                        (LinkKind::Triggers, "ukadoc:list_shiori_event:OnBoot:1"),
                        (LinkKind::SameFeature, "ukadoc:list_sakura_script:t:1"),
                    ],
                    ..row("ukadoc:list_sakura_script:s:1", Status::Implemented)
                },
                row("ukadoc:list_sakura_script:t:1", Status::Degraded),
            ],
        ),
        ledger_of(
            Domain::Assets,
            &["list_shell"],
            &[
                Row {
                    values: &["装い"],
                    ..row("ukadoc:list_shell:alpha:1", Status::Implemented)
                },
                row("ukadoc:list_shell:bravo:1", Status::Unclassified),
                Row {
                    links: &[(LinkKind::SameFeature, "ukadoc:list_shell:ghost:1")],
                    ..row("ukadoc:list_shell:charlie:1", Status::Unclassified)
                },
            ],
        ),
    ]
}

fn sample_body() -> String {
    render_summary(&catalog_of(GENERATED_AT), &sample_ledgers(), &THEMES)
}

/// 見本の本文の逐語の期待値。
///
/// テストファイルの改行が復帰文字に化けても期待値が変わらないよう、行の列を `\n` で
/// 繋いで組む（本文は改行だけ・設計 D-6）。
fn expected_body() -> String {
    let lines = [
        "# ukadoc 網羅状況の全体報告",
        "",
        "この本文はカタログと台帳から機械で作ります。手で書き換えず、食い違いは作り直して直します。",
        "この報告はカタログと台帳から決まり、常時検査が新しさを判定します。証拠の件数は `evidence` 副手続きで読みます。",
        "",
        "## 元にしたカタログ",
        "",
        "スナップショットの生成日時は 2026-08-24T04:08:57.881Z です。この報告がどれだけ古いかは、この日時で読みます。",
        "",
        "## 台帳ごとの項目数と未分類件数",
        "",
        "| ドメイン | 項目数 | 未分類 |",
        "| --- | ---: | ---: |",
        "| assets | 3 | 2 |",
        "| property | 5 | 3 |",
        "| sakura-script | 2 | 0 |",
        "| shiori | 4 | 1 |",
        "",
        "## 状態の分布",
        "",
        "| 状態 | 件数 |",
        "| --- | ---: |",
        "| 実装済み | 4 |",
        "| 語彙のみ | 0 |",
        "| 縮退 | 2 |",
        "| 未対応 | 1 |",
        "| 別名 | 0 |",
        "| 対象外 | 1 |",
        "| 未分類 | 6 |",
        "| 合計 | 14 |",
        "",
        "## ドメイン別の状態の分布",
        "",
        "| ドメイン | 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        "| assets | 1 | 0 | 0 | 0 | 0 | 0 | 2 | 3 |",
        "| property | 1 | 0 | 0 | 0 | 0 | 1 | 3 | 5 |",
        "| sakura-script | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 2 |",
        "| shiori | 1 | 0 | 1 | 1 | 0 | 0 | 1 | 4 |",
        "",
        "## ドメインを跨いで繋がった束",
        "",
        "ドメインの中で閉じている束は、それぞれのドメインの報告に載ります。ここに並ぶのは跨いだものだけです。",
        "",
        "| 束 id | 跨ぐドメイン | 構成 id |",
        "| --- | --- | --- |",
        "| ukadoc:list_sakura_script:s:1 | assets, sakura-script, shiori | \
         ukadoc:list_sakura_script:s:1, ukadoc:list_sakura_script:t:1, \
         ukadoc:list_shell:alpha:1, ukadoc:list_shiori_event:OnBoot:1 |",
        "| ukadoc:list_shell:charlie:1 | assets, （台帳に無い） | \
         ukadoc:list_shell:charlie:1, ukadoc:list_shell:ghost:1 |",
        "",
        "## テーマ別の状態分布",
        "",
        "| テーマ | 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        "| 交わり | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |",
        "| 掛け合い | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |",
        "| 更新 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |",
        "| 気配 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |",
        "| 気配り | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |",
        "| 装い | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |",
        "| 触れ合い | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |",
        "| 記憶 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |",
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

// ---- 版面の逐語の釘（要件 7.3・11.2） ----

#[test]
fn the_body_is_pinned_verbatim() {
    // 前提の確認——見本の台帳は名前順でも要件 3.1 の並び（shiori・assets・
    // sakura-script・property）でもない順で渡している。本文が名前順に並ぶのは
    // 渡された順に頼らないからで、この前提が崩れると逐語の比べが並べ替えを守らない。
    let arrived: Vec<&str> = sample_ledgers()
        .iter()
        .map(|ledger| ledger.domain.as_key())
        .collect();
    assert_eq!(
        arrived,
        vec!["property", "shiori", "sakura-script", "assets"]
    );
    // 版面そのものが契約である。
    assert_eq!(sample_body(), expected_body());
}

// ---- 時刻はカタログから来る（要件 7.3） ----

#[test]
fn the_snapshot_time_comes_from_the_catalog_and_is_not_baked_in() {
    let body = render_summary(
        &catalog_of("2019-01-02T03:04:05.000Z"),
        &sample_ledgers(),
        &THEMES,
    );
    assert!(body.contains("2019-01-02T03:04:05.000Z"), "{body}");
    assert!(!body.contains(GENERATED_AT), "{body}");
}

// ---- ドメインを跨いで繋がった束（設計 D-11） ----

#[test]
fn several_links_leaving_the_same_id_all_become_edges() {
    // 関連を「元 id ごとに 1 本」の入れ物で集めると
    // 2 本目以降が黙って落ちる。`hub` は別々のドメインの 2 つの id へ関連を出すので、
    // 辺が 1 本でも落ちれば束は 3 件にまとまらない。
    let ledgers = vec![
        ledger_of(
            Domain::SakuraScript,
            &["list_sakura_script"],
            &[Row {
                links: &[
                    (LinkKind::Triggers, "ukadoc:list_shiori_event:hub_a:1"),
                    (LinkKind::Queries, "ukadoc:list_propertysystem:hub_b:1"),
                ],
                ..row("ukadoc:list_sakura_script:hub:1", Status::Implemented)
            }],
        ),
        ledger_of(
            Domain::Shiori,
            &["list_shiori_event"],
            &[row("ukadoc:list_shiori_event:hub_a:1", Status::Absent)],
        ),
        ledger_of(
            Domain::Property,
            &["list_propertysystem"],
            &[row("ukadoc:list_propertysystem:hub_b:1", Status::Absent)],
        ),
    ];
    let body = render_summary(&catalog_of(GENERATED_AT), &ledgers, &THEMES);
    let section = section_body(&body, "## ドメインを跨いで繋がった束");
    assert!(
        section.contains(
            "| ukadoc:list_propertysystem:hub_b:1 | property, sakura-script, shiori | \
             ukadoc:list_propertysystem:hub_b:1, ukadoc:list_sakura_script:hub:1, \
             ukadoc:list_shiori_event:hub_a:1 |"
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
fn an_empty_cross_domain_bundle_list_says_there_is_none() {
    let ledgers = vec![ledger_of(
        Domain::Shiori,
        &["list_shiori_event"],
        &[
            Row {
                links: &[(LinkKind::SameFeature, "ukadoc:list_shiori_event:b:1")],
                ..row("ukadoc:list_shiori_event:a:1", Status::Implemented)
            },
            row("ukadoc:list_shiori_event:b:1", Status::Absent),
        ],
    )];
    let body = render_summary(&catalog_of(GENERATED_AT), &ledgers, &THEMES);
    let section = section_body(&body, "## ドメインを跨いで繋がった束");
    assert!(
        section.contains("ドメインを跨いだ束はありません。"),
        "{section}"
    );
    assert!(!section.contains("| 束 id |"), "{section}");
}

// ---- テーマ別の状態分布（全体・要件 4.4 の 8 テーマ） ----

#[test]
fn a_theme_name_that_only_the_argument_carries_still_gets_a_row() {
    // テーマ名は引数から来る。引数を無視して定数だけを見る実装をここで捕まえる。
    let mut themes: Vec<&str> = THEMES.to_vec();
    themes.push("あいうえお");
    let body = render_summary(&catalog_of(GENERATED_AT), &sample_ledgers(), &themes);
    let section = section_body(&body, "## テーマ別の状態分布");
    assert!(
        section.contains("| あいうえお | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |"),
        "{section}"
    );
}
