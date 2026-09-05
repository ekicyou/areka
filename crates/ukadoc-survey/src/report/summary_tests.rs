//! `summary.rs` の在中テスト。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない
//! （要件 6.2・設計 File Structure Plan）。
//!
//! 期待値は実装側の定数を参照せず**逐語の文字列**で書く。実装の表を実装の表自身と
//! 比べるだけのテストは、綴りの取り違えも並べ替えも 1 件も捕まえない（タスク 1.5 の
//! 教訓）。本文の全文を釘付けするテストを 1 本置いてある（タスク 3.5 の教訓——版面
//! そのものが契約である）。
//!
//! # 時刻の主張は 2 回描いても守れない（タスク 3.5 の教訓）
//!
//! 2 回続けて描くテストは、秒単位の壁時計を焼き込んでも素通りする（2 回の描画は
//! マイクロ秒差なので同じ値になる）。全体報告には**カタログの生成日時 1 つだけ**が
//! 載るので、ドメイン別報告の「4 桁続く数字が無い」はそのままでは使えない。ここでは
//! 「時刻を含む行を数え、1 行だけであること・その 1 行が生成日時であること」を主張し、
//! 数える道具自身も既知の例で較正する。

use std::collections::BTreeMap;

use super::render_summary;
use crate::catalog::{Catalog, SnapshotMeta};
use crate::evidence::{EvidenceIndex, NameMatchFailure, UnmatchedName, UnresolvedUrl};
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
/// 時刻を数えるテストが赤くなる。
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

/// 見本の台帳 4 本。**渡す順は名前順でも要件 3.1 の並びでもない**（タスク 2.5 の
/// 教訓——整列済みの見本は並び順を 1 つも守らない）。
///
/// - 項目数と未分類件数は**4 本とも違う値**（3/2・5/3・2/0・4/1）。同じ値だと台帳
///   ごとの表と合計 1 行の区別が付かない。
/// - `s:1` は**1 つの id から 2 本の関連**を出す（タスク 3.4・3.5 からの申し送り）。
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

/// 見本の証拠の索引。
///
/// ドメインごとの件数を **0/1/1/2 と散らして**ある（全部同じだと定数を書いただけの
/// 実装と区別が付かない）。`ghost:1` はどの台帳にも無い id なので、`by_id` の鍵は
/// 5 つでも台帳ごとの合計は 4 件になる。
fn sample_evidence() -> EvidenceIndex {
    let mut by_id: BTreeMap<EntryId, Vec<String>> = BTreeMap::new();
    by_id.insert(
        id("ukadoc:list_shell:alpha:1"),
        vec![
            "crates/areka-emo/src/atlas.rs".to_owned(),
            "crates/areka-seriko/src/shell.rs".to_owned(),
        ],
    );
    by_id.insert(
        id("ukadoc:list_shell:bravo:1"),
        vec!["crates/areka-seriko/src/bravo.rs".to_owned()],
    );
    by_id.insert(
        id("ukadoc:list_shiori_event:OnBoot:1"),
        vec!["crates/areka-shiori/src/boot.rs".to_owned()],
    );
    by_id.insert(
        id("ukadoc:list_sakura_script:s:1"),
        vec!["crates/areka-sakura/src/script.rs".to_owned()],
    );
    by_id.insert(
        id("ukadoc:list_shell:ghost:1"),
        vec!["crates/areka-seriko/src/ghost.rs".to_owned()],
    );
    EvidenceIndex {
        by_id,
        unresolved: vec![UnresolvedUrl {
            path: "crates/areka-emo/src/unresolved.rs".to_owned(),
            url: "https://ssp.shillest.net/ukadoc/manual/typo.html".to_owned(),
        }],
        unmatched_names: vec![UnmatchedName {
            path: "crates/sylphya/src/unmatched.rs".to_owned(),
            page_url: "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html".to_owned(),
            reason: NameMatchFailure::TableMissing,
        }],
    }
}

/// 見本の証拠に現れるファイルパスのすべて（本文に 1 つも出てはいけない・要件 2.3）。
const EVIDENCE_PATHS: [&str; 8] = [
    "crates/areka-emo/src/unresolved.rs",
    "crates/areka-emo/src/atlas.rs",
    "crates/areka-seriko/src/shell.rs",
    "crates/areka-seriko/src/bravo.rs",
    "crates/areka-shiori/src/boot.rs",
    "crates/areka-sakura/src/script.rs",
    "crates/areka-seriko/src/ghost.rs",
    "crates/sylphya/src/unmatched.rs",
];

fn sample_body() -> String {
    render_summary(
        &catalog_of(GENERATED_AT),
        &sample_ledgers(),
        &sample_evidence(),
        &THEMES,
    )
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
        "4 本の台帳を跨ぐ報告なので、新しさは常時検査の合否に入れません。作り直すのは統合担当です。",
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
        "## ドメインごとの証拠あり件数",
        "",
        "載せるのは件数だけです。どのファイルに書かれているかは検査の出力が示します。",
        "",
        "| ドメイン | 証拠あり |",
        "| --- | ---: |",
        "| assets | 2 |",
        "| property | 0 |",
        "| sakura-script | 1 |",
        "| shiori | 1 |",
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

/// 1 行が時刻を含むか。年（4 桁続く数字）か時計（数字・コロン・数字）を探す。
///
/// 道具そのものが壊れていないことは [`the_time_detector_notices_a_wall_clock`] が
/// 両向きで確かめる（緑は道具が壊れていても出る）。
fn looks_like_a_time(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut run = 0usize;
    for (index, byte) in bytes.iter().enumerate() {
        if byte.is_ascii_digit() {
            run += 1;
            if run >= 4 {
                return true;
            }
            if index + 2 < bytes.len()
                && bytes[index + 1] == b':'
                && bytes[index + 2].is_ascii_digit()
            {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

// ---- 完了条件の前半: 2 回作ると同じ本文（要件 7.3） ----

#[test]
fn rendering_the_same_input_twice_gives_the_same_body() {
    let catalog = catalog_of(GENERATED_AT);
    let ledgers = sample_ledgers();
    let evidence = sample_evidence();
    let first = render_summary(&catalog, &ledgers, &evidence, &THEMES);
    let second = render_summary(&catalog, &ledgers, &evidence, &THEMES);
    assert_eq!(first, second);
    // 空文字どうしを比べて満足しないこと（同一の主張は対象が空だと恒真）。
    assert!(!first.is_empty());
}

#[test]
fn the_body_is_pinned_verbatim() {
    // 版面そのものが契約である（タスク 3.5 の教訓）。
    assert_eq!(sample_body(), expected_body());
}

#[test]
fn the_body_writes_only_line_feeds() {
    let body = sample_body();
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

// ---- 完了条件の後半: 時刻を含む行は生成日時の 1 行だけ（要件 7.3） ----

#[test]
fn the_time_detector_notices_a_wall_clock() {
    // 道具の較正。既知の例で両向きに確かめる（[[subagent-tooling-can-be-wrong-calibrate-it]]）。
    assert!(looks_like_a_time("生成日時: 2026-09-04T12:00:00Z"));
    assert!(looks_like_a_time("12:34 に作りました"));
    assert!(!looks_like_a_time("| assets | 3 | 2 |"));
    assert!(!looks_like_a_time("| ukadoc:list_shell:alpha:1 | 1 |"));
    assert!(!looks_like_a_time("4 本の台帳を跨ぐ報告なので"));
}

#[test]
fn the_only_line_carrying_a_time_is_the_snapshot_generated_at() {
    let body = sample_body();
    let timed: Vec<&str> = body
        .lines()
        .filter(|line| looks_like_a_time(line))
        .collect();
    // 壁時計の行を 1 行足せばここが 2 になる（2 回描画の一致では守れない・タスク 3.5）。
    assert_eq!(timed.len(), 1, "{body}");
    assert!(timed[0].contains(GENERATED_AT), "{}", timed[0]);
    assert!(
        timed[0].starts_with("スナップショットの生成日時は"),
        "{}",
        timed[0]
    );
}

#[test]
fn the_snapshot_time_comes_from_the_catalog_and_is_not_baked_in() {
    let body = render_summary(
        &catalog_of("2019-01-02T03:04:05.000Z"),
        &sample_ledgers(),
        &sample_evidence(),
        &THEMES,
    );
    assert!(body.contains("2019-01-02T03:04:05.000Z"), "{body}");
    assert!(!body.contains(GENERATED_AT), "{body}");
}

// ---- 冒頭: 各台帳の項目数と未分類件数（要件 7.2） ----

#[test]
fn the_head_gives_each_ledgers_item_count_and_unclassified_count() {
    let section = section_body(&sample_body(), "## 台帳ごとの項目数と未分類件数");
    // 4 本とも値が違うので、台帳ごとの表と合計 1 行を区別できる。
    assert!(section.contains("| assets | 3 | 2 |"), "{section}");
    assert!(section.contains("| property | 5 | 3 |"), "{section}");
    assert!(section.contains("| sakura-script | 2 | 0 |"), "{section}");
    assert!(section.contains("| shiori | 4 | 1 |"), "{section}");
    // 合計 1 行に畳んでいないこと（14 件・未分類 6 件）。
    assert!(!section.contains("| 14 |"), "{section}");
    assert!(!section.contains("合計"), "{section}");
}

#[test]
fn the_sections_appear_in_the_order_the_design_gives() {
    let body = sample_body();
    let order = [
        section_at(&body, "## 元にしたカタログ"),
        section_at(&body, "## 台帳ごとの項目数と未分類件数"),
        section_at(&body, "## 状態の分布"),
        section_at(&body, "## ドメイン別の状態の分布"),
        section_at(&body, "## ドメインを跨いで繋がった束"),
        section_at(&body, "## テーマ別の状態分布"),
        section_at(&body, "## ドメインごとの証拠あり件数"),
    ];
    let mut sorted = order;
    sorted.sort_unstable();
    assert_eq!(order.to_vec(), sorted.to_vec());
}

// ---- 状態の分布（全体・ドメイン別・要件 7.2・7.8） ----

#[test]
fn the_overall_distribution_uses_the_plain_japanese_names_in_the_frozen_order() {
    let section = section_body(&sample_body(), "## 状態の分布");
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
    // 0 件の欄も残る（「0 件だった」と「数えていない」を区別できるように）。
    assert!(section.contains("| 語彙のみ | 0 |"), "{section}");
    assert!(section.contains("| 別名 | 0 |"), "{section}");
    // 英字の綴りは台帳側のもので、報告には出ない。
    assert!(!section.contains("vocabulary-only"), "{section}");
}

#[test]
fn the_domain_tables_are_in_name_order_even_though_the_ledgers_arrive_in_another_order() {
    // 前提の確認——見本は名前順でも要件 3.1 の並び（shiori・assets・sakura-script・
    // property）でもない順で渡している。
    let arrived: Vec<&str> = sample_ledgers()
        .iter()
        .map(|ledger| ledger.domain.as_key())
        .collect();
    assert_eq!(
        arrived,
        vec!["property", "shiori", "sakura-script", "assets"]
    );

    let body = sample_body();
    for heading in [
        "## 台帳ごとの項目数と未分類件数",
        "## ドメイン別の状態の分布",
        "## ドメインごとの証拠あり件数",
    ] {
        let section = section_body(&body, heading);
        let names: Vec<&str> = section
            .lines()
            .filter_map(|line| line.strip_prefix("| "))
            .filter_map(|line| line.split(" | ").next())
            .filter(|name| !name.starts_with("---") && name.is_ascii())
            .collect();
        // 名前順は UTF-8 のバイト順（`sakura-script` が `shiori` より前）。
        assert_eq!(
            names,
            vec!["assets", "property", "sakura-script", "shiori"],
            "{section}"
        );
    }
}

#[test]
fn the_per_domain_distribution_shows_each_ledgers_own_numbers() {
    let section = section_body(&sample_body(), "## ドメイン別の状態の分布");
    assert!(
        section.contains(
            "| ドメイン | 実装済み | 語彙のみ | 縮退 | 未対応 | 別名 | 対象外 | 未分類 | 合計 |"
        ),
        "{section}"
    );
    assert!(
        section.contains("| assets | 1 | 0 | 0 | 0 | 0 | 0 | 2 | 3 |"),
        "{section}"
    );
    assert!(
        section.contains("| shiori | 1 | 0 | 1 | 1 | 0 | 0 | 1 | 4 |"),
        "{section}"
    );
    // 4 本を足した 1 行に畳んでいないこと（全体の分布は別の節が持つ）。
    assert!(!section.contains(" | 14 |"), "{section}");
}

// ---- ドメインを跨いで繋がった束（設計 D-11） ----

#[test]
fn a_bundle_spanning_several_domains_is_listed_with_the_domains_it_crosses() {
    let section = section_body(&sample_body(), "## ドメインを跨いで繋がった束");
    assert!(
        section.contains(
            "| ukadoc:list_sakura_script:s:1 | assets, sakura-script, shiori | \
             ukadoc:list_sakura_script:s:1, ukadoc:list_sakura_script:t:1, \
             ukadoc:list_shell:alpha:1, ukadoc:list_shiori_event:OnBoot:1 |"
        ),
        "{section}"
    );
}

#[test]
fn a_bundle_closed_inside_one_domain_is_not_listed() {
    // 負の主張は必ず正の主張と対で置く（対象が空だと恒真になる・タスク 1.6 の教訓）。
    let section = section_body(&sample_body(), "## ドメインを跨いで繋がった束");
    assert!(
        section.contains("| ukadoc:list_sakura_script:s:1 |"),
        "{section}"
    );
    // property の中で閉じた束（x → y）は property の報告の担当。
    assert!(
        !section.contains("ukadoc:list_propertysystem:x:1"),
        "{section}"
    );
    assert!(
        !section.contains("ukadoc:list_propertysystem:y:1"),
        "{section}"
    );
}

#[test]
fn a_bundle_member_missing_from_every_ledger_is_marked_instead_of_going_missing() {
    // どの台帳にも無い id へ伸びた束は、ドメイン別報告からも落ちる。ここが落とすと
    // どの報告にも現れなくなるので、載せたうえで所属が無いことを書く。
    let section = section_body(&sample_body(), "## ドメインを跨いで繋がった束");
    assert!(
        section.contains(
            "| ukadoc:list_shell:charlie:1 | assets, （台帳に無い） | \
             ukadoc:list_shell:charlie:1, ukadoc:list_shell:ghost:1 |"
        ),
        "{section}"
    );
}

#[test]
fn several_links_leaving_the_same_id_all_become_edges() {
    // タスク 3.4・3.5 からの申し送り: 関連を「元 id ごとに 1 本」の入れ物で集めると
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
    let body = render_summary(
        &catalog_of(GENERATED_AT),
        &ledgers,
        &EvidenceIndex::default(),
        &THEMES,
    );
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
    let body = render_summary(
        &catalog_of(GENERATED_AT),
        &ledgers,
        &EvidenceIndex::default(),
        &THEMES,
    );
    let section = section_body(&body, "## ドメインを跨いで繋がった束");
    assert!(
        section.contains("ドメインを跨いだ束はありません。"),
        "{section}"
    );
    assert!(!section.contains("| 束 id |"), "{section}");
}

// ---- テーマ別の状態分布（全体・要件 4.4 の 8 テーマ） ----

#[test]
fn the_theme_table_lists_all_eight_themes_in_name_order_including_the_empty_ones() {
    let section = section_body(&sample_body(), "## テーマ別の状態分布");
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
    // 0 件のテーマも欄が残る。
    assert!(
        section.contains("| 更新 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |"),
        "{section}"
    );
    // 台帳を跨いで足し合わせた数（`気配` は shiori と property に 1 件ずつ）。
    assert!(
        section.contains("| 気配 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |"),
        "{section}"
    );
    assert!(
        section.contains("| 装い | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |"),
        "{section}"
    );
}

#[test]
fn a_theme_name_that_only_the_argument_carries_still_gets_a_row() {
    // テーマ名は引数から来る。引数を無視して定数だけを見る実装をここで捕まえる。
    let mut themes: Vec<&str> = THEMES.to_vec();
    themes.push("あいうえお");
    let body = render_summary(
        &catalog_of(GENERATED_AT),
        &sample_ledgers(),
        &sample_evidence(),
        &themes,
    );
    let section = section_body(&body, "## テーマ別の状態分布");
    assert!(
        section.contains("| あいうえお | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |"),
        "{section}"
    );
}

// ---- ドメインごとの証拠あり件数（要件 2.3・設計 D-11） ----

#[test]
fn the_evidence_section_gives_counts_per_domain() {
    let section = section_body(&sample_body(), "## ドメインごとの証拠あり件数");
    // 件数は 0/1/1/2 と散らしてある（定数を書いただけの実装と区別が付く）。
    assert!(section.contains("| assets | 2 |"), "{section}");
    assert!(section.contains("| property | 0 |"), "{section}");
    assert!(section.contains("| sakura-script | 1 |"), "{section}");
    assert!(section.contains("| shiori | 1 |"), "{section}");
    // `by_id` の鍵は 5 つだが、`ghost:1` はどの台帳にも無いので合計は 4 件。
    // 索引の大きさをそのまま載せる実装をここで捕まえる。
    assert!(!section.contains("| 5 |"), "{section}");
}

#[test]
fn the_report_never_shows_where_the_evidence_is_written() {
    // 要件 2.3 の上限——報告に載せるのは有無（件数）だけで、ファイルパスは載せない。
    // 場所を示すのは検査の出力の役目（要件 5.5）。
    let body = sample_body();
    for path in EVIDENCE_PATHS {
        assert!(!body.contains(path), "本文に証拠のパスが出た: {path}");
    }
    assert!(!body.contains("https://"), "{body}");
    // 負の主張の対照——証拠そのものは 4 件あって、件数として本文に出ている。
    assert!(body.contains("| assets | 2 |"), "{body}");
}
