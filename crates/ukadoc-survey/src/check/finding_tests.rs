//! `finding.rs` の在中テスト——検査の出力の整形（要件 6.10・6.12）。
//!
//! # 期待値は実装の定数を参照しない
//!
//! 描かれる本文そのものが契約なので、逐語の期待値は独立した文字列リテラルで書く。
//! 実装の定数を引くと、表を表自身と比べるだけになる。

use super::super::test_support::{id, ledger_place};
use super::{Finding, FindingKind, render};

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

/// 種類ごとの塊は [`FindingKind::ALL`] の並びで出る。
///
/// 入力を逆順に与えても本文の並びは変わらない。13 種すべてを 1 件ずつ与えて、
/// 見出しの並びを逐語で釘付けする。見出しの綴りは [`FindingKind::as_key`] で、README の
/// 所見の表と同じである。往復では守れないので、ここで逐語に並べる。
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
        "LedgerPagesMismatch",
        "LedgerOutOfOrder",
        "PageNotAssigned",
        "SourceUrlNotInCatalog",
        "ImplementedWithoutEvidence",
        "LinkEndpointMissing",
        "AliasChain",
        "IntroducedNotInCatalogVersions",
        "DomainReportStale",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(headings, expected);
}
