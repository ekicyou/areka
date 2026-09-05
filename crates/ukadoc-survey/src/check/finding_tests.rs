//! `finding.rs` の在中テストと、検査の入口・見本データの在中テスト。
//!
//! # なぜ入口と見本のテストまでここに置くか
//!
//! 設計の Directory Structure は `check/` に 4 つの本体（`mod.rs`・`finding.rs`・
//! `structure.rs`・`content.rs`・`freshness.rs`）を置きながら、`_tests.rs` の兄弟を
//! `mod.rs` には与えていない。残る 3 本の兄弟はタスク 4.2〜4.4 の持ち物なので、入口
//! （[`crate::check::run`]）と共用の見本データを確かめる場所はここしかない。
//! `catalog::read` に `impl Catalog` を置いたとき（タスク 2.2）と同じ扱いである。
//!
//! # 期待値は実装の定数を参照しない
//!
//! 描かれる本文そのものが契約なので（タスク 3.5 の教訓）、逐語の期待値は独立した
//! 文字列リテラルで書く。実装の定数を引くと、表を表自身と比べるだけになる。
//!
//! # 「所見 0 件」は否定の主張である
//!
//! 見本が 3 判定すべてで所見 0 件になることは、判定が空の実装である今は無条件に真で
//! ある（タスク 1.6 の教訓）。そこで「見本が何を持っているか」を肯定の側から数え、
//! 各判定が見るはずの材料が実際に非空であることを別に主張する。壊せば赤になることを
//! 確かめるのはタスク 4.2〜4.4 の仕事である。

use super::super::{CheckInput, JUDGEMENTS, Judgement, ScanStats, run, run_with};
use super::{Finding, FindingKind, render};
use crate::evidence::UrlHit;
use crate::lib_test_support::World;
use crate::model::{Domain, EntryId, Status, THEMES};
use std::collections::{BTreeMap, BTreeSet};

fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は 2 形のいずれかのはず")
}

/// 台帳のファイルパス（所見の「場所」に載る綴り）。
fn ledger_place(domain: &str) -> String {
    format!("doc/ukadoc-coverage/ledger/{domain}.toml")
}

// ---------------------------------------------------------------------------
// 出力の整形（要件 6.10・6.12）
// ---------------------------------------------------------------------------

#[test]
fn no_findings_render_to_an_empty_body() {
    assert_eq!(render(&[]), "");
}

/// 設計「Data Models」→「検査の出力」の版面を逐語で釘付けする。
///
/// 件数だけの主張では区切りの空白 1 個も並びも守れず、この本文はテストの失敗
/// メッセージそのものになる（要件 6.12）。
#[test]
fn the_rendered_body_is_pinned_verbatim() {
    let findings = vec![
        Finding::new(
            FindingKind::LedgerIdNotInCatalog,
            Some(id("ukadoc:list_propertysystem:balloon.scope(ID).width:1")),
            ledger_place("property"),
            "カタログに無い id",
        ),
        Finding::new(
            FindingKind::LedgerIdNotInCatalog,
            Some(id("ukadoc:list_propertysystem:system.zzz:1")),
            ledger_place("property"),
            "カタログに無い id",
        ),
        Finding::new(
            FindingKind::ImplementedWithoutEvidence,
            Some(id("ukadoc:list_shiori_event:OnBoot:1")),
            ledger_place("shiori"),
            "正典 URL がソースに 1 件も無い",
        ),
    ];

    let expected = "\
食い違い 3 件

[LedgerIdNotInCatalog] 2 件
  doc/ukadoc-coverage/ledger/property.toml
    ukadoc:list_propertysystem:balloon.scope(ID).width:1  カタログに無い id
    ukadoc:list_propertysystem:system.zzz:1  カタログに無い id
[ImplementedWithoutEvidence] 1 件
  doc/ukadoc-coverage/ledger/shiori.toml
    ukadoc:list_shiori_event:OnBoot:1  正典 URL がソースに 1 件も無い
";
    assert_eq!(render(&findings), expected);
}

/// 該当 id は必ず本文に載る（要件 6.10）。
#[test]
fn the_line_of_a_finding_carries_its_id() {
    let findings = vec![Finding::new(
        FindingKind::UnknownTheme,
        Some(id("ukadoc:list_shiori_event:OnBoot:1")),
        ledger_place("shiori"),
        "テーマ定義に無い名前: きはい",
    )];
    let body = render(&findings);
    assert!(
        body.contains("ukadoc:list_shiori_event:OnBoot:1"),
        "id が本文に無い:\n{body}"
    );
    assert!(
        body.contains("doc/ukadoc-coverage/ledger/shiori.toml"),
        "場所が本文に無い:\n{body}"
    );
    assert!(
        body.contains("テーマ定義に無い名前: きはい"),
        "詳細が本文に無い:\n{body}"
    );
}

/// id を持たない所見（ページの割り当てなど）は場所と詳細だけを並べる。
#[test]
fn a_finding_without_an_id_shows_its_place_and_detail() {
    let findings = vec![Finding::new(
        FindingKind::PageNotAssigned,
        None,
        "doc/ukadoc-coverage/catalog.toml",
        "どの台帳にも割り当てが無いページ: new_page",
    )];
    let expected = "\
食い違い 1 件

[PageNotAssigned] 1 件
  doc/ukadoc-coverage/catalog.toml
    どの台帳にも割り当てが無いページ: new_page
";
    assert_eq!(render(&findings), expected);
}

/// 同じ種類・同じ場所の所見は場所の行を 1 度しか書かない。
#[test]
fn two_findings_in_the_same_place_share_one_place_line() {
    let findings = vec![
        Finding::new(
            FindingKind::LedgerIdNotInCatalog,
            Some(id("ukadoc:spec_web:a:1")),
            ledger_place("shiori"),
            "カタログに無い id",
        ),
        Finding::new(
            FindingKind::LedgerIdNotInCatalog,
            Some(id("ukadoc:spec_web:b:1")),
            ledger_place("shiori"),
            "カタログに無い id",
        ),
    ];
    let body = render(&findings);
    assert_eq!(
        body.matches("  doc/ukadoc-coverage/ledger/shiori.toml\n")
            .count(),
        1,
        "場所の行が 1 度でない:\n{body}"
    );
}

/// 同じ種類でも場所が違えば場所の行が分かれ、場所の名前順に並ぶ。
#[test]
fn findings_of_one_kind_in_two_places_get_two_place_lines_in_name_order() {
    let findings = vec![
        Finding::new(
            FindingKind::LedgerIdNotInCatalog,
            Some(id("ukadoc:spec_web:a:1")),
            ledger_place("shiori"),
            "カタログに無い id",
        ),
        Finding::new(
            FindingKind::LedgerIdNotInCatalog,
            Some(id("ukadoc:manual_shell")),
            ledger_place("assets"),
            "カタログに無い id",
        ),
    ];
    let expected = "\
食い違い 2 件

[LedgerIdNotInCatalog] 2 件
  doc/ukadoc-coverage/ledger/assets.toml
    ukadoc:manual_shell  カタログに無い id
  doc/ukadoc-coverage/ledger/shiori.toml
    ukadoc:spec_web:a:1  カタログに無い id
";
    assert_eq!(render(&findings), expected);
}

/// 同じ種類・同じ場所の中は id の昇順に並ぶ。入力の順には従わない。
#[test]
fn findings_in_one_place_are_ordered_by_id_not_by_input_order() {
    let findings = vec![
        Finding::new(
            FindingKind::AliasChain,
            Some(id("ukadoc:spec_web:z:1")),
            ledger_place("shiori"),
            "指す先が別名",
        ),
        Finding::new(
            FindingKind::AliasChain,
            Some(id("ukadoc:spec_web:a:1")),
            ledger_place("shiori"),
            "指す先が別名",
        ),
    ];
    let body = render(&findings);
    let a = body.find("ukadoc:spec_web:a:1").expect("a が無い");
    let z = body.find("ukadoc:spec_web:z:1").expect("z が無い");
    assert!(a < z, "id の昇順で並んでいない:\n{body}");
}

/// 種類ごとの塊は [`FindingKind::ALL`] の並び（＝設計の判定表の並び）で出る。
///
/// 入力を逆順に与えても本文の並びは変わらない。15 種すべてを 1 件ずつ与えて、
/// 見出しの並びを逐語で釘付けする。
#[test]
fn the_kind_blocks_come_out_in_the_declared_order() {
    let mut findings: Vec<Finding> = FindingKind::ALL
        .iter()
        .map(|kind| {
            Finding::new(
                *kind,
                Some(id("ukadoc:spec_web:a:1")),
                ledger_place("shiori"),
                "見本",
            )
        })
        .collect();
    findings.reverse();

    let body = render(&findings);
    let headings: Vec<String> = body
        .lines()
        .filter(|line| line.starts_with('['))
        .map(|line| {
            line.trim_start_matches('[')
                .split(']')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect();

    let expected: Vec<String> = vec![
        "LedgerIdNotInCatalog",
        "CatalogIdMissingFromLedgers",
        "CatalogIdInMultipleLedgers",
        "LedgerIdPageMismatch",
        "LedgerDomainMismatch",
        "LedgerPagesMismatch",
        "LedgerOutOfOrder",
        "PageNotAssigned",
        "SourceUrlNotInCatalog",
        "ImplementedWithoutEvidence",
        "LinkEndpointMissing",
        "AliasChain",
        "IntroducedNotInCatalogVersions",
        "UnknownTheme",
        "DomainReportStale",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(headings, expected);
}

/// 種類の綴りは設計の判定表そのままである。往復では守れないので逐語で並べる
/// （タスク 1.2 の教訓）。
#[test]
fn the_kind_keys_are_pinned_verbatim() {
    let keys: Vec<&'static str> = FindingKind::ALL.iter().map(FindingKind::as_key).collect();
    assert_eq!(
        keys,
        vec![
            "LedgerIdNotInCatalog",
            "CatalogIdMissingFromLedgers",
            "CatalogIdInMultipleLedgers",
            "LedgerIdPageMismatch",
            "LedgerDomainMismatch",
            "LedgerPagesMismatch",
            "LedgerOutOfOrder",
            "PageNotAssigned",
            "SourceUrlNotInCatalog",
            "ImplementedWithoutEvidence",
            "LinkEndpointMissing",
            "AliasChain",
            "IntroducedNotInCatalogVersions",
            "UnknownTheme",
            "DomainReportStale",
        ]
    );
}

/// 設計の判定表は 15 種。数え落としも重複も落とす。
#[test]
fn all_fifteen_kinds_are_declared_once_each() {
    assert_eq!(FindingKind::ALL.len(), 15);
    let distinct: BTreeSet<&'static str> =
        FindingKind::ALL.iter().map(FindingKind::as_key).collect();
    assert_eq!(distinct.len(), 15, "同じ綴りが 2 度出ている");
}

/// 本文は入力の並びに依らない（要件 7.3 の決定論を検査の出力にも通す）。
#[test]
fn the_body_does_not_depend_on_the_input_order() {
    let mut findings = vec![
        Finding::new(
            FindingKind::DomainReportStale,
            None,
            "doc/ukadoc-coverage/report/assets.md",
            "台帳から作り直した本文と違う",
        ),
        Finding::new(
            FindingKind::LedgerOutOfOrder,
            Some(id("ukadoc:manual_shell")),
            ledger_place("assets"),
            "id の順に並んでいない",
        ),
        Finding::new(
            FindingKind::UnknownTheme,
            Some(id("ukadoc:spec_web:a:1")),
            ledger_place("shiori"),
            "テーマ定義に無い名前",
        ),
    ];
    let first = render(&findings);
    findings.reverse();
    assert_eq!(render(&findings), first);
}

// ---------------------------------------------------------------------------
// 入口の配線（3 つの判定を必ず呼ぶ）
// ---------------------------------------------------------------------------

/// 入口が 3 つの判定を**この順で**走らせたことが、走った判定の名前として残る。
///
/// 部品を釘付けしても入口がその部品を呼んでいるかは別に守る必要がある
/// （タスク 1.7 の教訓）。いま 3 つの判定はどれも所見を返さないので、所見の側からは
/// 呼び忘れを見分けられない。走った判定の名前を数えることでだけ見分けられる。
#[test]
fn run_reports_the_three_judgements_it_ran_in_order() {
    let world = World::normal();
    let outcome = run(&world.input());
    assert_eq!(
        outcome.stats.judgements_run,
        vec!["structure", "content", "freshness"]
    );
}

/// 判定の表には本物の 3 つの関数が入っている。
#[test]
fn the_judgement_table_holds_the_three_real_functions() {
    let names: Vec<&'static str> = JUDGEMENTS.iter().map(|j| j.name).collect();
    assert_eq!(names, vec!["structure", "content", "freshness"]);

    let wired: Vec<usize> = JUDGEMENTS
        .iter()
        .map(|j| j.run as *const () as usize)
        .collect();
    let real: Vec<usize> = vec![
        super::super::structure::check as *const () as usize,
        super::super::content::check as *const () as usize,
        super::super::freshness::check as *const () as usize,
    ];
    assert_eq!(wired, real, "表に載っている関数が本物と違う");
}

/// 走らせる側は表の項目を 1 つも落とさず、並びも保つ。
///
/// 本物の 3 つはいま所見を返さないので、代役の判定 3 つを与えて確かめる。代役が
/// 返した所見が全部そのまま出れば、たたみ込みは表を全部通っている。
#[test]
fn the_entry_point_keeps_every_judgement_in_the_table() {
    fn one(kind: FindingKind, detail: &'static str) -> Vec<Finding> {
        vec![Finding::new(kind, None, "見本", detail)]
    }
    fn first(_input: &CheckInput) -> Vec<Finding> {
        one(FindingKind::LedgerIdNotInCatalog, "1 番目")
    }
    fn second(_input: &CheckInput) -> Vec<Finding> {
        one(FindingKind::CatalogIdMissingFromLedgers, "2 番目")
    }
    fn third(_input: &CheckInput) -> Vec<Finding> {
        one(FindingKind::CatalogIdInMultipleLedgers, "3 番目")
    }

    let stand_ins = [
        Judgement {
            name: "一",
            run: first,
        },
        Judgement {
            name: "二",
            run: second,
        },
        Judgement {
            name: "三",
            run: third,
        },
    ];

    let world = World::normal();
    let outcome = run_with(&stand_ins, &world.input());

    let details: Vec<&str> = outcome
        .findings
        .iter()
        .map(|finding| finding.detail.as_str())
        .collect();
    assert_eq!(details, vec!["1 番目", "2 番目", "3 番目"]);
    assert_eq!(outcome.stats.judgements_run, vec!["一", "二", "三"]);
}

// ---------------------------------------------------------------------------
// 見本データ——まず「何を持っているか」を数える（否定の主張だけにしない）
// ---------------------------------------------------------------------------

/// 同じドメインの台帳が 2 本来たら件数を足し合わせる（片方を黙って落とさない）。
///
/// 見本は 1 ドメイン 1 本なので、足すか上書きするかを見分けられない。ここだけ台帳を
/// 複製した入力を作って、`ScanStats` の doc が主張する防御そのものを釘付けする。
#[test]
fn the_stats_add_up_two_ledgers_of_the_same_domain() {
    let world = World::normal();
    let base = world.input();
    let single = base.ledgers[0].entries.len();
    assert!(single > 0, "見本の台帳が空では足し算を見分けられない");

    let doubled: Vec<crate::ledger::Ledger> =
        vec![base.ledgers[0].clone(), base.ledgers[0].clone()];
    let input = CheckInput {
        ledgers: &doubled,
        ..base
    };

    let stats = run(&input).stats;
    assert_eq!(
        stats.ledger_entries.get(&doubled[0].domain).copied(),
        Some(single * 2),
        "2 本分が足されていない: {:?}",
        stats.ledger_entries
    );
}

/// 走査の件数は見本の中身をそのまま写す。数字は独立したリテラルで書く。
#[test]
fn the_stats_of_the_sample_world_are_pinned() {
    let world = World::normal();
    let stats: ScanStats = run(&world.input()).stats;

    assert_eq!(stats.catalog_entries, 12);
    assert_eq!(stats.catalog_pages, 6);
    assert_eq!(stats.catalog_categories, 6);
    assert_eq!(stats.ledgers, 4);
    assert_eq!(
        stats.ledger_entries,
        BTreeMap::from([
            (Domain::Shiori, 3),
            (Domain::Assets, 3),
            (Domain::SakuraScript, 3),
            (Domain::Property, 3),
        ])
    );
    assert_eq!(stats.assigned_pages, 38);
    assert_eq!(
        stats.assigned_pages_by_domain,
        BTreeMap::from([
            (Domain::Shiori, 12),
            (Domain::Assets, 24),
            (Domain::SakuraScript, 1),
            (Domain::Property, 1),
        ])
    );
    assert_eq!(stats.themes, 8);
    assert_eq!(stats.evidence_ids, 4);
    assert_eq!(stats.evidence_paths, 3);
    assert_eq!(stats.domain_reports, 4);
    assert_eq!(stats.non_empty_domain_reports, 4);
}

/// 台帳の全 id がカタログに実在する。**照合した件数も数える**——空の台帳でも
/// 「1 件も違反が無い」は真になるからである（タスク 1.6 の教訓）。
#[test]
fn every_ledger_id_is_in_the_catalog_and_there_are_twelve_of_them() {
    let world = World::normal();
    let mut checked = 0;
    for ledger in &world.ledgers {
        for entry_id in ledger.entries.keys() {
            assert!(
                world.catalog.entries.contains_key(entry_id),
                "カタログに無い id: {}",
                entry_id.as_str()
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 12);
}

/// カタログの全 id がちょうど 1 つの台帳に 1 回だけ現れる。
#[test]
fn every_catalog_id_appears_in_exactly_one_ledger_once() {
    let world = World::normal();
    let mut seen: BTreeMap<&EntryId, usize> = BTreeMap::new();
    for ledger in &world.ledgers {
        for entry_id in ledger.entries.keys() {
            *seen.entry(entry_id).or_default() += 1;
        }
    }
    assert_eq!(seen.len(), world.catalog.entries.len());
    for (entry_id, count) in &seen {
        assert_eq!(*count, 1, "2 度現れる id: {}", entry_id.as_str());
        assert!(world.catalog.entries.contains_key(*entry_id));
    }
}

/// 前置きの担当ページが割り当て表と集合として一致し、id のページも担当の中にある。
#[test]
fn each_ledger_declares_the_assigned_pages_and_holds_only_its_own() {
    let world = World::normal();
    for ledger in &world.ledgers {
        let declared: BTreeSet<_> = ledger.pages.iter().cloned().collect();
        let assigned: BTreeSet<_> = world
            .assignment
            .pages_of(ledger.domain)
            .into_iter()
            .collect();
        assert_eq!(
            declared, assigned,
            "{:?} の前置きが割り当てと違う",
            ledger.domain
        );
        assert!(!declared.is_empty());
        for entry_id in ledger.entries.keys() {
            assert_eq!(
                world.assignment.domain_of(&entry_id.page()),
                Some(ledger.domain),
                "担当外のページの id: {}",
                entry_id.as_str()
            );
        }
    }
}

/// カタログのページはすべて割り当てがある（要件 3.5）。
#[test]
fn every_catalog_page_is_assigned() {
    let world = World::normal();
    let pages: Vec<_> = world
        .catalog
        .entries
        .values()
        .map(|entry| entry.page.clone())
        .collect();
    assert!(!pages.is_empty());
    assert_eq!(world.assignment.unassigned(pages.iter()), Vec::new());
}

/// 台帳の並びは id の byte 厳密昇順で、本文の順と鍵の順が一致する。
///
/// 見本の台帳はどれも 3 項目ある。1 項目の台帳は並び順を 1 つも守らない
/// （タスク 2.5 の教訓）。
#[test]
fn every_ledger_is_in_strictly_ascending_id_order_with_three_entries() {
    let world = World::normal();
    for ledger in &world.ledgers {
        assert_eq!(ledger.file_order.len(), 3, "{:?} の項目数", ledger.domain);
        for pair in ledger.file_order.windows(2) {
            assert!(
                pair[0] < pair[1],
                "{:?} の並びが昇順でない: {} → {}",
                ledger.domain,
                pair[0].as_str(),
                pair[1].as_str()
            );
        }
        let keys: Vec<&EntryId> = ledger.entries.keys().collect();
        let order: Vec<&EntryId> = ledger.file_order.iter().collect();
        assert_eq!(keys, order);
    }
}

/// 実装済みの行にはすべて証拠があり、その行は 3 件ある。
///
/// 3 件のうち 1 件（`system.year`）は語彙表の経路で付く（設計 D-5）。URL の直書き
/// だけの見本にすると名前の突き合わせが 1 度も走らない。
#[test]
fn every_implemented_row_has_evidence_and_one_of_them_comes_from_a_vocabulary_table() {
    let world = World::normal();
    let mut implemented = 0;
    for ledger in &world.ledgers {
        for entry in ledger.entries.values() {
            if entry.status != Status::Implemented {
                continue;
            }
            implemented += 1;
            let paths = world.evidence.by_id.get(&entry.id);
            assert!(
                paths.is_some_and(|paths| !paths.is_empty()),
                "証拠の無い実装済み: {}",
                entry.id.as_str()
            );
        }
    }
    assert_eq!(implemented, 3);

    assert_eq!(
        world
            .evidence
            .by_id
            .get(&id("ukadoc:list_propertysystem:system.year:1"))
            .map(Vec::as_slice),
        Some(["crates/areka-sylphya/src/vocab/dotted.rs".to_owned()].as_slice())
    );
}

/// ソースの URL はすべてカタログに解決し、名前の突き合わせも全部 1 件に定まる。
#[test]
fn no_source_url_is_left_unresolved_and_no_name_is_left_unmatched() {
    let world = World::normal();
    assert_eq!(world.evidence.unresolved, Vec::new());
    assert_eq!(world.evidence.unmatched_names, Vec::new());
    assert_eq!(world.evidence.by_id.len(), 4);
}

/// 取り出しは URL 付きの行だけを拾う（要件 5.6）。見本のソース文にはその対照が
/// 入っている。
#[test]
fn the_sample_sources_carry_three_evidence_lines_and_one_url_less_mention() {
    let world = World::normal();
    let hits: Vec<UrlHit> = world
        .sources
        .iter()
        .flat_map(|(path, text)| extract_of(path, text))
        .collect();
    assert_eq!(hits.len(), 3, "取り出した URL の件数");
    let mentions = world
        .sources
        .iter()
        .flat_map(|(_, text)| text.lines())
        .filter(|line| line.contains("ukadoc"))
        .count();
    assert_eq!(
        mentions, 4,
        "ukadoc に触れる行の件数（うち 1 行は URL 無し）"
    );
}

fn extract_of(path: &str, text: &str) -> Vec<UrlHit> {
    crate::evidence::extract::extract(path, text)
}

/// 関連・別名・置き換えの相手はすべてカタログに実在し、対は 3 組ある。
#[test]
fn every_link_endpoint_exists_in_the_catalog() {
    let world = World::normal();
    let mut endpoints = 0;
    for ledger in &world.ledgers {
        for entry in ledger.entries.values() {
            let mut targets: Vec<&EntryId> = entry.links.iter().map(|link| &link.to).collect();
            targets.extend(entry.alias_of.as_ref());
            targets.extend(entry.supersedes.iter());
            for target in targets {
                assert!(
                    world.catalog.entries.contains_key(target),
                    "カタログに無い相手: {}",
                    target.as_str()
                );
                endpoints += 1;
            }
        }
    }
    assert_eq!(endpoints, 4);
}

/// 別名の連鎖は無く、別名の行は 1 件ある（要件 2.4・6.7）。
#[test]
fn the_alias_row_points_at_a_row_that_is_not_an_alias() {
    let world = World::normal();
    let by_id: BTreeMap<&EntryId, Status> = world
        .ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.values())
        .map(|entry| (&entry.id, entry.status))
        .collect();
    let mut aliases = 0;
    for (entry_id, status) in &by_id {
        if *status != Status::Alias {
            continue;
        }
        aliases += 1;
        let target = world
            .ledgers
            .iter()
            .flat_map(|ledger| ledger.entries.values())
            .find(|entry| entry.id == **entry_id)
            .and_then(|entry| entry.alias_of.clone())
            .expect("別名の行には指す先がある");
        assert_ne!(by_id.get(&target).copied(), Some(Status::Alias));
    }
    assert_eq!(aliases, 1);
}

/// 記録された登場版はカタログの版番号の中にあり、そういう行が 6 件ある（要件 6.7）。
#[test]
fn every_recorded_version_is_one_of_the_catalog_versions() {
    let world = World::normal();
    let mut recorded = 0;
    for ledger in &world.ledgers {
        for entry in ledger.entries.values() {
            if entry.introduced.is_empty() {
                continue;
            }
            let versions = &world
                .catalog
                .entries
                .get(&entry.id)
                .expect("台帳の id はカタログにある")
                .versions;
            assert!(
                versions.contains(&entry.introduced),
                "カタログの版に無い登場版: {} の {}",
                entry.id.as_str(),
                entry.introduced
            );
            recorded += 1;
        }
    }
    assert_eq!(recorded, 6);
}

/// 台帳が書いたテーマ名はすべて 8 テーマにあり、使われた名前は 5 種ある（要件 6.8）。
#[test]
fn every_theme_written_in_the_ledgers_is_one_of_the_eight() {
    let world = World::normal();
    let used: BTreeSet<&str> = world
        .ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.values())
        .flat_map(|entry| entry.values.iter().map(String::as_str))
        .collect();
    assert!(!used.is_empty());
    for name in &used {
        assert!(THEMES.contains(name), "テーマ定義に無い名前: {name}");
    }
    assert_eq!(used.len(), 5);
}

/// ドメイン別報告 4 本は台帳から作り直した本文と一致し、いずれも空でない（要件 7.4）。
#[test]
fn the_four_domain_reports_match_what_the_ledgers_produce() {
    let world = World::normal();
    assert_eq!(world.domain_reports.len(), 4);
    for ledger in &world.ledgers {
        let body = world
            .domain_reports
            .get(&ledger.domain)
            .expect("4 ドメインすべての報告がある");
        assert!(!body.is_empty(), "{:?} の報告が空", ledger.domain);
        assert!(!body.contains('\r'), "{:?} の報告に復帰文字", ledger.domain);
        assert_eq!(
            *body,
            crate::report::domain::render_domain(ledger, &THEMES),
            "{:?} の報告が台帳と食い違う",
            ledger.domain
        );
    }
}

/// 見本の世界に食い違いは 1 件も無い（この課題の完了条件・否定の主張）。
///
/// これだけでは何も守らない。上のいくつかの主張が「判定が見るはずの材料が実際に
/// 揃っている」ことを肯定の側から数えている。
#[test]
fn the_sample_world_has_no_findings() {
    let world = World::normal();
    let outcome = run(&world.input());
    assert_eq!(render(&outcome.findings), "");
    assert_eq!(outcome.findings.len(), 0);
}

// ---------------------------------------------------------------------------
// 見本の作り直しが本当に効く（4.2〜4.4 が壊し方を組み立てる土台）
// ---------------------------------------------------------------------------

/// 台帳を変えて報告を作り直すと、報告の本文が実際に変わる。
///
/// 作り直しが何もしない実装だと、タスク 4.2・4.3 の「1 件だけ出る」が静かに崩れる。
#[test]
fn refreshing_the_reports_follows_the_ledgers() {
    let mut world = World::normal();
    let before = world
        .domain_reports
        .get(&Domain::Shiori)
        .expect("shiori の報告")
        .clone();

    let ledger = world.ledger_mut(Domain::Shiori);
    let target = id("ukadoc:spec_shiori3");
    ledger
        .entries
        .get_mut(&target)
        .expect("見本にある id")
        .status = Status::Degraded;
    world.refresh_reports();

    let after = world
        .domain_reports
        .get(&Domain::Shiori)
        .expect("shiori の報告");
    assert_ne!(*after, before, "台帳を変えたのに報告が変わらない");
}

/// ソース文を変えて索引を作り直すと、解決できない URL が実際に現れる。
///
/// 証拠の索引が本当にソース文から作られていることの確かめでもある。
#[test]
fn refreshing_the_evidence_follows_the_sources() {
    let mut world = World::normal();
    assert_eq!(world.evidence.unresolved, Vec::new());

    let text = world.source_mut("crates/areka-kanade/src/schedule/events.rs");
    *text = text.replace("#OnBoot:1", "#OnBoot:2");
    world.refresh_evidence();

    assert_eq!(world.evidence.unresolved.len(), 1);
    assert!(
        !world
            .evidence
            .by_id
            .contains_key(&id("ukadoc:list_shiori_event:OnBoot:1")),
        "URL を壊したのに証拠が残っている"
    );
}

/// 報告だけを書き換えても台帳は動かない（タスク 4.4 の壊し方の土台）。
#[test]
fn the_report_can_be_edited_without_touching_the_ledger() {
    let mut world = World::normal();
    let body = world.report_mut(Domain::Assets);
    body.push_str("手で足した 1 行\n");

    let ledger = world
        .ledgers
        .iter()
        .find(|ledger| ledger.domain == Domain::Assets)
        .expect("assets の台帳");
    assert_ne!(
        *world.domain_reports.get(&Domain::Assets).expect("報告"),
        crate::report::domain::render_domain(ledger, &THEMES)
    );
}
