//! `structure.rs` の在中テスト（要件 3.1・3.2・3.5・6.3・6.4・6.13）。
//!
//! # 主張はすべて入口を通す
//!
//! どのテストも [`crate::check::run`] を呼び、`structure::check` を直に呼ばない。
//! 部品を釘付けしても、入口がその部品を呼んでいるかは別に守る必要がある
//! （タスク 1.7 の教訓・タスク 4.1 からの申し送り）。判定を 1 つも呼ばない入口は、
//! 直に呼ぶテストでは永久に見つからない。
//!
//! # 壊した対と壊さない対で置く
//!
//! 「所見が 0 件」だけの主張は、判定が何も返さない実装で無条件に真になる
//! （タスク 1.6 の教訓）。そこで正常な見本で 0 件になることと、ちょうど 1 か所を
//! 壊すと該当する種類が出ることを対で置く。出た所見は種類ごとの**件数の等式**で
//! 主張するので、意図しない種類が 1 件でも混ざれば赤になる。
//!
//! # 道連れになる所見が 2 つある
//!
//! `CatalogIdInMultipleLedgers` は `LedgerIdPageMismatch` を、`PageNotAssigned` は
//! `CatalogIdMissingFromLedgers` を必ず連れてくる（1 ページ 1 ドメインが仕様の
//! 不変条件なので、どんな見本でも避けられない）。この 2 つだけは 2 種類を明示して
//! 主張する。
//!
//! # `LedgerDomainMismatch` のテストは無い
//!
//! `[ledger].domain` の食い違いは `ledger::read` の段で落ちる。[`Ledger`] は
//! `domain` を 1 つしか持たず、その値はファイル名から来るので、宣言された綴りが残る
//! のは読み取りの中だけである。[`CheckInput`] は台帳ごとのファイル名を受け取らない
//! から、検査層はこの判定を作り直せない——**書けないのが正しい**（設計 check 節の
//! 注記・タスク 2.4 からの申し送り）。下の各テストが「その種類は 0 件」を等式で
//! 主張しているので、うっかり作り物の判定を足せば赤になる。

use super::super::{CheckInput, Finding, FindingKind, run};
use crate::catalog::CatalogEntry;
use crate::ledger::{Ledger, LedgerEntry};
use crate::lib_test_support::World;
use crate::model::{Domain, EntryId, PageName, Status};
use std::collections::BTreeMap;

/// 見本の id を作る。
fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は要件 1.9 の 2 形のいずれかのはず")
}

/// 台帳のファイルパス（所見の「場所」に載る綴り）。実装の定数は参照しない。
fn ledger_place(domain: &str) -> String {
    format!("doc/ukadoc-coverage/ledger/{domain}.toml")
}

/// カタログのファイルパス。
fn catalog_place() -> String {
    "doc/ukadoc-coverage/catalog.toml".to_owned()
}

/// 出た所見を種類ごとに数える（0 件の種類は落とす）。
///
/// これを**等式**で主張すると、意図しない種類が 1 件でも出れば赤になる。件数だけの
/// 主張は中身が全部誤っていても緑になるので、この等式と id・場所・詳細の逐語の主張を
/// 必ず対で置く（タスク 1.5 の教訓）。
fn kinds(findings: &[Finding]) -> Vec<(FindingKind, usize)> {
    FindingKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = findings.iter().filter(|f| f.kind == kind).count();
            (count > 0).then_some((kind, count))
        })
        .collect()
}

/// その種類の所見だけを取り出す。
fn of_kind(findings: &[Finding], kind: FindingKind) -> Vec<&Finding> {
    findings.iter().filter(|f| f.kind == kind).collect()
}

/// その種類の所見がちょうど 1 件あることを確かめ、その 1 件を返す。
fn only_one(findings: &[Finding], kind: FindingKind) -> &Finding {
    let found = of_kind(findings, kind);
    assert_eq!(
        found.len(),
        1,
        "{} が 1 件でない: {:?}",
        kind.as_key(),
        found
    );
    found[0]
}

/// 台帳へ項目を差し込む。本文の順は id 順に組み直す（並び順を壊す意図が無いとき）。
fn insert_entry(ledger: &mut Ledger, entry: LedgerEntry) {
    ledger.entries.insert(entry.id.clone(), entry);
    ledger.file_order = ledger.entries.keys().cloned().collect();
}

/// 台帳から項目を抜き、本文の順も揃える。
fn remove_entry(ledger: &mut Ledger, target: &EntryId) -> LedgerEntry {
    let entry = ledger
        .entries
        .remove(target)
        .expect("見本の台帳にその id が無い");
    ledger.file_order = ledger.entries.keys().cloned().collect();
    entry
}

/// 既定の欄だけを持つ台帳の項目。
fn plain_entry(raw: &str) -> LedgerEntry {
    LedgerEntry {
        id: id(raw),
        status: Status::Unclassified,
        introduced: String::new(),
        alias_of: None,
        supersedes: Vec::new(),
        owner: String::new(),
        priority: String::new(),
        values: Vec::new(),
        links: Vec::new(),
        note: String::new(),
    }
}

/// カタログの 1 項目（ページと URL は id から導く）。
fn catalog_entry(raw: &str, title: &str) -> CatalogEntry {
    let entry_id = id(raw);
    let page = entry_id.page();
    CatalogEntry {
        url: format!(
            "https://ssp.shillest.net/ukadoc/manual/{}.html#{}",
            page.as_str(),
            title
        ),
        page,
        title: title.to_owned(),
        category: "shiori_event".to_owned(),
        versions: Vec::new(),
        hash: "dddddddddddddddd".to_owned(),
        id: entry_id,
    }
}

// ---------------------------------------------------------------------------
// 壊していない見本（対の片方）
// ---------------------------------------------------------------------------

/// 正常な見本は所見を 1 件も出さない。
///
/// これは否定の主張なので、以下の壊した側の主張と必ず対で読むこと。
#[test]
fn the_untouched_sample_world_has_no_findings() {
    let world = World::normal();
    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
    assert_eq!(outcome.findings.len(), 0);
}

/// 検査が見ている材料が空でないことを、肯定の側から数える。
///
/// 台帳も割り当ても空の入力なら「所見 0 件」は無条件に真になる。判定が実際に
/// 何かを走査していることの土台をここで固定する。
#[test]
fn the_sample_world_actually_holds_something_to_judge() {
    let world = World::normal();
    let input = world.input();
    assert_eq!(input.catalog.entries.len(), 12);
    assert_eq!(input.ledgers.len(), 4);
    let entries: usize = input.ledgers.iter().map(|l| l.entries.len()).sum();
    assert_eq!(entries, 12, "カタログの 12 件が台帳 4 本に散っているはず");
    assert_eq!(input.assignment.pages_of(Domain::Shiori).len(), 12);
    assert_eq!(input.assignment.pages_of(Domain::Assets).len(), 24);
}

// ---------------------------------------------------------------------------
// 台帳の id がカタログに実在するか（要件 6.3）
// ---------------------------------------------------------------------------

/// カタログに無い id を台帳へ 1 つ足すと `LedgerIdNotInCatalog` が 1 件だけ出る。
///
/// ページは shiori の担当なので、担当違いは道連れにならない。
#[test]
fn a_ledger_id_absent_from_the_catalog_is_a_finding() {
    let mut world = World::normal();
    insert_entry(
        world.ledger_mut(Domain::Shiori),
        plain_entry("ukadoc:list_shiori_event:OnGhostBooted:1"),
    );
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LedgerIdNotInCatalog, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::LedgerIdNotInCatalog);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_shiori_event:OnGhostBooted:1")
    );
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(finding.detail, "カタログに無い id");
}

// ---------------------------------------------------------------------------
// カタログの id がちょうど 1 つの台帳に 1 回だけ現れるか（要件 6.4・3.2）
// ---------------------------------------------------------------------------

/// カタログにある id を台帳から抜くと `CatalogIdMissingFromLedgers` が 1 件だけ出る。
#[test]
fn a_catalog_id_missing_from_every_ledger_is_a_finding() {
    let mut world = World::normal();
    remove_entry(
        world.ledger_mut(Domain::Shiori),
        &id("ukadoc:list_shiori_event:OnClose:1"),
    );
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::CatalogIdMissingFromLedgers, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::CatalogIdMissingFromLedgers);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_shiori_event:OnClose:1")
    );
    assert_eq!(finding.place, catalog_place());
    assert_eq!(finding.detail, "どの台帳にも現れない id");
}

/// 同じ id を 2 本目の台帳へ写すと `CatalogIdInMultipleLedgers` が出る。
///
/// **`LedgerIdPageMismatch` が必ず道連れになる**。1 ページは 1 ドメインにしか
/// 属さない（要件 3.2 の不変条件）ので、2 本目の台帳はその id のページを担当して
/// いない。どんな見本を選んでも避けられないから、2 種類を明示して主張する
/// （タスク 4.1 からの申し送り）。
#[test]
fn a_catalog_id_in_two_ledgers_is_a_finding_together_with_the_page_mismatch() {
    let mut world = World::normal();
    let copied = world
        .ledger_mut(Domain::Shiori)
        .entries
        .get(&id("ukadoc:list_shiori_event:OnBoot:1"))
        .cloned()
        .expect("見本の shiori 台帳に OnBoot があるはず");
    insert_entry(world.ledger_mut(Domain::Property), copied);
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![
            (FindingKind::CatalogIdInMultipleLedgers, 1),
            (FindingKind::LedgerIdPageMismatch, 1),
        ]
    );

    let doubled = only_one(&outcome.findings, FindingKind::CatalogIdInMultipleLedgers);
    assert_eq!(
        doubled.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_shiori_event:OnBoot:1")
    );
    assert_eq!(doubled.place, catalog_place());
    assert_eq!(doubled.detail, "2 つ以上の台帳にある id: shiori・property");

    let page = only_one(&outcome.findings, FindingKind::LedgerIdPageMismatch);
    assert_eq!(page.place, ledger_place("property"));
    assert_eq!(
        page.detail,
        "ページ list_shiori_event の担当は shiori で、この台帳（property）ではない"
    );
}

// ---------------------------------------------------------------------------
// id のページがその台帳の担当か（要件 3.1・3.2）
// ---------------------------------------------------------------------------

/// 担当違いの台帳へ id を移すと `LedgerIdPageMismatch` が 1 件だけ出る。
///
/// 抜いて差し込むので、カタログ側の「ちょうど 1 回」は保たれる。
#[test]
fn an_entry_filed_under_the_wrong_ledger_is_a_finding() {
    let mut world = World::normal();
    let moved = remove_entry(world.ledger_mut(Domain::Assets), &id("ukadoc:manual_shell"));
    insert_entry(world.ledger_mut(Domain::Shiori), moved);
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LedgerIdPageMismatch, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::LedgerIdPageMismatch);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:manual_shell")
    );
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(
        finding.detail,
        "ページ manual_shell の担当は assets で、この台帳（shiori）ではない"
    );
}

/// 誤った前置きは項目の担当違いを**隠さない**。
///
/// 担当違いの項目を移したうえで、その台帳の前置きにもそのページを書き足す。前置きを
/// 担当の正本に採る実装（「前置きに書いてあるなら自分の担当」）はここで担当違いを
/// 見落として赤になる。誤りが誤りを隠す形を塞ぐのはこのテストだけである。
#[test]
fn a_wrong_preamble_does_not_excuse_a_misfiled_entry() {
    let mut world = World::normal();
    let moved = remove_entry(world.ledger_mut(Domain::Assets), &id("ukadoc:manual_shell"));
    let shiori = world.ledger_mut(Domain::Shiori);
    insert_entry(shiori, moved);
    shiori.pages.push(PageName::new("manual_shell"));
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![
            (FindingKind::LedgerIdPageMismatch, 1),
            (FindingKind::LedgerPagesMismatch, 1),
        ]
    );
    let finding = only_one(&outcome.findings, FindingKind::LedgerIdPageMismatch);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:manual_shell")
    );
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(
        finding.detail,
        "ページ manual_shell の担当は assets で、この台帳（shiori）ではない"
    );
    assert_eq!(
        only_one(&outcome.findings, FindingKind::LedgerPagesMismatch).detail,
        "前置きの担当ページが割り当て表と違う（足りない: なし / 余分: manual_shell）"
    );
}

/// 担当の正本は割り当て表であって、台帳の前置きではない。
///
/// 前置きから `list_shiori_event` を落としても、割り当て表がそのページを shiori に
/// 割り当てている限り、その台帳の項目は担当違いにならない。**前置きを正本に取り違えた
/// 実装はここで赤になる**（そのとき 2 件の担当違いが出る）。
#[test]
fn the_page_owner_comes_from_the_assignment_not_from_the_preamble() {
    let mut world = World::normal();
    let ledger = world.ledger_mut(Domain::Shiori);
    let dropped = PageName::new("list_shiori_event");
    assert!(
        ledger.pages.contains(&dropped),
        "前置きにそのページが無ければ落とす意味が無い"
    );
    ledger.pages.retain(|page| page != &dropped);
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LedgerPagesMismatch, 1)],
        "前置きを担当の正本に取り違えると LedgerIdPageMismatch も出てしまう"
    );
    let finding = only_one(&outcome.findings, FindingKind::LedgerPagesMismatch);
    assert_eq!(finding.id, None);
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(
        finding.detail,
        "前置きの担当ページが割り当て表と違う（足りない: list_shiori_event / 余分: なし）"
    );
}

// ---------------------------------------------------------------------------
// 前置きの担当ページ（要件 3.1）
// ---------------------------------------------------------------------------

/// 前置きの 1 ページを別のページに**すり替える**と `LedgerPagesMismatch` が出る。
///
/// ページ数は変わらないので、件数だけを比べる実装はここで赤になる。足りない側と
/// 余分な側の両方が本文に出ることも同時に釘付けする。
#[test]
fn swapping_one_preamble_page_for_another_is_a_finding() {
    let mut world = World::normal();
    let ledger = world.ledger_mut(Domain::Property);
    let before = ledger.pages.len();
    ledger.pages = vec![PageName::new("descript_balloon")];
    assert_eq!(before, ledger.pages.len(), "件数を変えては試験にならない");
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LedgerPagesMismatch, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::LedgerPagesMismatch);
    assert_eq!(finding.id, None);
    assert_eq!(finding.place, ledger_place("property"));
    assert_eq!(
        finding.detail,
        "前置きの担当ページが割り当て表と違う（足りない: list_propertysystem / 余分: descript_balloon）"
    );
}

/// 前置きに余分なページを足すと `LedgerPagesMismatch` が 1 件だけ出る。
#[test]
fn an_extra_page_in_the_preamble_is_a_finding() {
    let mut world = World::normal();
    world
        .ledger_mut(Domain::Property)
        .pages
        .push(PageName::new("descript_balloon"));
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LedgerPagesMismatch, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::LedgerPagesMismatch);
    assert_eq!(finding.id, None);
    assert_eq!(finding.place, ledger_place("property"));
    assert_eq!(
        finding.detail,
        "前置きの担当ページが割り当て表と違う（足りない: なし / 余分: descript_balloon）"
    );
}

/// 前置きの比較は**集合**である（設計の判定表）。
///
/// 並べ替えただけでは所見にならない。並びの正しさは初期台帳の生成が受け持ち、
/// 要件 3.3a により前置きは以後バイト列のまま写される（タスク 2.5 の申し送り）。
#[test]
fn reordering_the_preamble_pages_is_not_a_finding() {
    let mut world = World::normal();
    let ledger = world.ledger_mut(Domain::Assets);
    assert!(
        ledger.pages.len() >= 2,
        "1 要素の見本では並べ替えを試したことにならない"
    );
    ledger.pages.reverse();
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
}

// ---------------------------------------------------------------------------
// 台帳の並び順（要件 3.3a・付録 A・設計 D-12）
// ---------------------------------------------------------------------------

/// 本文の順が降る箇所があると `LedgerOutOfOrder` が 1 件だけ出る。
///
/// 入れ替えるのは**本文の順だけ**で、id を鍵にした表は触らない。表の鍵は作りからして
/// 昇順なので、そちらを見る実装はこの見本を素通りする（設計 D-12 が `file_order` を
/// 別に持たせている理由そのもの）。
#[test]
fn a_descending_pair_in_the_file_order_is_a_finding() {
    let mut world = World::normal();
    let ledger = world.ledger_mut(Domain::Shiori);
    assert_eq!(
        ledger
            .file_order
            .iter()
            .map(EntryId::as_str)
            .collect::<Vec<_>>(),
        vec![
            "ukadoc:list_shiori_event:OnBoot:1",
            "ukadoc:list_shiori_event:OnClose:1",
            "ukadoc:spec_shiori3",
        ],
        "壊す前の並びを固定しておく"
    );
    ledger.file_order.swap(0, 1);
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LedgerOutOfOrder, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::LedgerOutOfOrder);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_shiori_event:OnBoot:1"),
        "順序を破っているのは後ろに置かれた側"
    );
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(
        finding.detail,
        "直前の id ukadoc:list_shiori_event:OnClose:1 より後ろに並んでいない"
    );
}

/// 同じ id が 2 回続いても `LedgerOutOfOrder` が出る（**厳密な**昇順・設計 D-12）。
///
/// 備考の複数行文字列の中に既存 id の見出し行が隠れていると、塊の切り分けは鍵の集合が
/// 一致するので見抜けない。厳密昇順だけがこの穴を閉じる。判定を `<` から `<=` へ
/// 緩めるとこのテストだけが赤になる。
#[test]
fn an_equal_pair_in_the_file_order_is_a_finding_too() {
    let mut world = World::normal();
    let ledger = world.ledger_mut(Domain::Shiori);
    ledger
        .file_order
        .insert(1, id("ukadoc:list_shiori_event:OnBoot:1"));

    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LedgerOutOfOrder, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::LedgerOutOfOrder);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_shiori_event:OnBoot:1")
    );
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(
        finding.detail,
        "直前の id ukadoc:list_shiori_event:OnBoot:1 より後ろに並んでいない"
    );
}

// ---------------------------------------------------------------------------
// カタログのページに割り当てが無い（要件 3.5）
// ---------------------------------------------------------------------------

/// 割り当ての無いページの項目をカタログへ足すと `PageNotAssigned` が出る。
///
/// **`CatalogIdMissingFromLedgers` が必ず道連れになる**——足した id はどの台帳にも
/// 無いからである（タスク 4.1 からの申し送り）。台帳へ入れると今度は担当違いが
/// 道連れになる（次のテスト）。どちらの形でも 1 種類だけにはできない。
#[test]
fn a_catalog_page_outside_the_assignment_is_a_finding() {
    let mut world = World::normal();
    let entry = catalog_entry("ukadoc:list_unknown_page:Foo:1", "Foo");
    world.catalog.entries.insert(entry.id.clone(), entry);

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![
            (FindingKind::CatalogIdMissingFromLedgers, 1),
            (FindingKind::PageNotAssigned, 1),
        ]
    );
    let finding = only_one(&outcome.findings, FindingKind::PageNotAssigned);
    assert_eq!(finding.id, None, "主語はページであって項目ではない");
    assert_eq!(finding.place, catalog_place());
    assert_eq!(finding.detail, "割り当ての無いページ: list_unknown_page");
}

/// 割り当ての無いページの項目を台帳へ入れると、担当違いの側も出る。
///
/// 「どのドメインの担当でもない」ページを担当と見なす実装はここで赤になる。
#[test]
fn an_entry_on_an_unassigned_page_is_never_the_ledgers_own() {
    let mut world = World::normal();
    let entry = catalog_entry("ukadoc:list_unknown_page:Foo:1", "Foo");
    world.catalog.entries.insert(entry.id.clone(), entry);
    insert_entry(
        world.ledger_mut(Domain::Shiori),
        plain_entry("ukadoc:list_unknown_page:Foo:1"),
    );
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![
            (FindingKind::LedgerIdPageMismatch, 1),
            (FindingKind::PageNotAssigned, 1),
        ]
    );
    let finding = only_one(&outcome.findings, FindingKind::LedgerIdPageMismatch);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_unknown_page:Foo:1")
    );
    assert_eq!(finding.place, ledger_place("shiori"));
    assert_eq!(
        finding.detail,
        "ページ list_unknown_page に割り当てが無く、この台帳（shiori）の担当ではない"
    );
}

// ---------------------------------------------------------------------------
// 1 件目で止めない・同じ入力なら同じ結果（設計 Error Handling・要件 7.3）
// ---------------------------------------------------------------------------

/// 2 か所を同時に壊すと 2 件とも出る（1 件目で止めない）。
///
/// 壊す 2 か所は別のドメイン・別の種類にしてある。1 件目で返す実装はここで赤になる。
#[test]
fn two_breakages_produce_two_findings() {
    let mut world = World::normal();
    world.ledger_mut(Domain::Shiori).file_order.swap(0, 1);
    world
        .ledger_mut(Domain::Assets)
        .pages
        .push(PageName::new("list_plugin_event"));
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![
            (FindingKind::LedgerPagesMismatch, 1),
            (FindingKind::LedgerOutOfOrder, 1),
        ]
    );
    assert_eq!(
        only_one(&outcome.findings, FindingKind::LedgerPagesMismatch).place,
        ledger_place("assets")
    );
    assert_eq!(
        only_one(&outcome.findings, FindingKind::LedgerOutOfOrder).place,
        ledger_place("shiori")
    );
}

/// 同じ入力を 2 度通すと所見は 1 件ずつ同じ並びで返る（要件 7.3 の決まり方）。
#[test]
fn the_same_input_yields_the_same_findings_in_the_same_order() {
    let mut world = World::normal();
    world.ledger_mut(Domain::Shiori).file_order.swap(0, 1);
    let entry = catalog_entry("ukadoc:list_unknown_page:Foo:1", "Foo");
    world.catalog.entries.insert(entry.id.clone(), entry);
    world.refresh_reports();

    let first = run(&world.input()).findings;
    let second = run(&world.input()).findings;
    assert_eq!(first, second);
    assert!(first.len() >= 3, "所見が少なすぎて並びを見比べられない");
}

/// 台帳を 1 本も渡さなければ、カタログの 12 件すべてが行き先を失う。
///
/// 「どの台帳にも無い」の判定がカタログ側を走査していることを、件数の側から
/// 裏取りする（1 件だけ抜く見本では 1 対 1 の対応しか見えない）。報告も一緒に
/// 空にするのは、台帳の無い世界の報告が古いかどうかを別の判定に問わせないため。
#[test]
fn with_no_ledgers_every_catalog_id_is_missing() {
    let world = World::normal();
    let empty: Vec<Ledger> = Vec::new();
    let no_reports: BTreeMap<Domain, String> = BTreeMap::new();
    let input = CheckInput {
        ledgers: &empty,
        domain_reports: &no_reports,
        ..world.input()
    };

    let outcome = run(&input);
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::CatalogIdMissingFromLedgers, 12)]
    );
}

// ---------------------------------------------------------------------------
// 「全 id」を見る・台帳の並びに左右されない（要件 6.3・7.3）
// ---------------------------------------------------------------------------

/// その種類の所見の詳細を出た順に並べる。
fn details(findings: &[Finding], kind: FindingKind) -> Vec<&str> {
    of_kind(findings, kind)
        .into_iter()
        .map(|finding| finding.detail.as_str())
        .collect()
}

/// 1 本の台帳にカタログに無い id が 2 つあれば、**2 件とも**所見になる。
///
/// 要件 6.3 は「台帳に現れる**全** id がカタログに実在すること」を求める。1 つだけ
/// 壊した見本では「全部を挙げる」と「最初の 1 件で打ち切る」を見分けられない
/// ——ドメインごとに 1 件だけ報告する実装はここでだけ赤になる。2 つの間に**実在する
/// id を 1 つ挟んである**ので、走査が正しい id で止まらないことも同時に守る。
///
/// カタログ側の対（`CatalogIdMissingFromLedgers`）には
/// `with_no_ledgers_every_catalog_id_is_missing` という同じ形の檻が既にあり、これは
/// その台帳側の相方である。
#[test]
fn every_catalog_absent_id_in_a_ledger_is_reported_not_just_the_first() {
    let mut world = World::normal();
    let ledger = world.ledger_mut(Domain::Shiori);
    insert_entry(
        ledger,
        plain_entry("ukadoc:list_shiori_event:OnGhostBooted:1"),
    );
    insert_entry(ledger, plain_entry("ukadoc:spec_web"));
    assert_eq!(
        ledger
            .file_order
            .iter()
            .map(EntryId::as_str)
            .collect::<Vec<_>>(),
        vec![
            "ukadoc:list_shiori_event:OnBoot:1",
            "ukadoc:list_shiori_event:OnClose:1",
            "ukadoc:list_shiori_event:OnGhostBooted:1",
            "ukadoc:spec_shiori3",
            "ukadoc:spec_web",
        ],
        "2 つの偽の id の間に実在する id が挟まっていなければ試験が弱くなる"
    );
    world.refresh_reports();

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::LedgerIdNotInCatalog, 2)]
    );

    let found = of_kind(&outcome.findings, FindingKind::LedgerIdNotInCatalog);
    let ids: Vec<&str> = found
        .iter()
        .map(|finding| {
            finding
                .id
                .as_ref()
                .map(EntryId::as_str)
                .expect("台帳の id についての所見は id を持つ")
        })
        .collect();
    assert_eq!(
        ids,
        vec![
            "ukadoc:list_shiori_event:OnGhostBooted:1",
            "ukadoc:spec_web"
        ]
    );
    for finding in &found {
        assert_eq!(finding.place, ledger_place("shiori"));
        assert_eq!(finding.detail, "カタログに無い id");
    }
}

/// 台帳を渡す並びを変えても、所見は同じことを言う（要件 7.3）。
///
/// `owning_domains` は集めたドメインを並べ直してから本文にする。見本の台帳は
/// [`Domain::ALL`] の順に並んでおり、その並びは `Domain` の宣言順と同じなので、
/// **並べ直しを消しても普段の見本では何も変わらない**。逆順で 1 度通すことだけが
/// その守りを釘付けする（同じ入力を 2 度通す主張では原理的に見えない）。
#[test]
fn the_order_of_the_ledgers_does_not_change_the_findings() {
    let mut world = World::normal();
    let copied = world
        .ledger_mut(Domain::Shiori)
        .entries
        .get(&id("ukadoc:list_shiori_event:OnBoot:1"))
        .cloned()
        .expect("見本の shiori 台帳に OnBoot があるはず");
    insert_entry(world.ledger_mut(Domain::Property), copied);
    world.refresh_reports();

    let reversed: Vec<Ledger> = world.ledgers.iter().rev().cloned().collect();
    assert_ne!(
        reversed[0].domain, world.ledgers[0].domain,
        "並びを変えていなければ試験にならない"
    );

    let forward = run(&world.input()).findings;
    let backward = run(&CheckInput {
        ledgers: &reversed,
        ..world.input()
    })
    .findings;

    let expected = vec!["2 つ以上の台帳にある id: shiori・property"];
    assert_eq!(
        details(&forward, FindingKind::CatalogIdInMultipleLedgers),
        expected
    );
    assert_eq!(
        details(&backward, FindingKind::CatalogIdInMultipleLedgers),
        expected,
        "台帳を渡す並びが本文に漏れている"
    );
    assert_eq!(kinds(&forward), kinds(&backward));
}
