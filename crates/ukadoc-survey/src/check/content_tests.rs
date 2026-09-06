//! `content.rs` の在中テスト——**ソースの正典 URL と証拠**（要件 6.5・6.6・6.10・6.13）。
//!
//! 関連・別名・版・テーマの側は姉妹モジュール `content_link_tests` が持つ
//! （要件 6.7・6.8）。共用の道具は `content_test_support` にある。
//!
//! # 主張はすべて入口を通す
//!
//! どのテストも [`crate::check::run`] を呼び、`content::check` を直に呼ばない。部品を
//! 釘付けしても、入口がその部品を呼んでいるかは別に守る必要がある（タスク 4.1 からの
//! 必須の申し送り。判定を 1 つも呼ばない `run` は直に呼ぶテストでは永久に見つからない）。
//!
//! # 壊した対と壊さない対で置く
//!
//! 「所見が 0 件」だけの主張は、判定が何も返さない実装で無条件に真になる（タスク 1.6 の
//! 教訓）。そこで正常な見本で 0 件になることと、ちょうど 1 か所を壊すと該当する種類が
//! 出ることを対で置く。出た所見は**種類ごとの件数の等式**で主張するので、意図しない
//! 種類が 1 件でも混ざれば赤になる（タスク 4.1 からの申し送り）。
//!
//! # 「全部を挙げる」は違反 2 件でしか守れない
//!
//! 違反を 1 件だけ置いた見本では「全部挙げる」と「最初の 1 件だけ挙げる」を区別
//! できない（タスク 4.2 からの申し送り）。`SourceUrlNotInCatalog` と
//! `ImplementedWithoutEvidence` には、違反 2 件の**間に正しい項目を挟んだ**見本を
//! 置いてある（「最初の違反で止まる」と「最初の正しい項目で止まる」の両方を捕まえる）。
//!
//! # 証拠のファイルパスは所見にしない
//!
//! 要件 2.3・5.5 は「証拠は台帳に書かず、検査の出力に id ごとのファイルパスとして
//! 列挙する」と言う。その列挙を作るのは**入口の実行ファイル**（タスク 6.3。「所見の
//! 本文と、id ごとの証拠のファイルパスを並べて出す」）であって、この判定ではない。
//! ここが作るのは食い違いだけで、証拠のある項目については 1 件も所見を出さない
//! ——出すと「所見が空なら緑」（設計 check 節の事後条件）が成り立たなくなる。

use super::super::{FindingKind, run};
use super::test_support::{
    EVENTS_PATH, TAG_PATH, VOCAB_PATH, details, entry_mut, id, ids, kinds, ledger_place, only_one,
    places, replace_in_source,
};
use crate::lib_test_support::World;
use crate::model::{Domain, EntryId, Status};

// ---------------------------------------------------------------------------
// 壊していない見本（対の片方）
// ---------------------------------------------------------------------------

/// 正常な見本は内容の所見を 1 件も出さない。
///
/// これは否定の主張なので、以下の壊した側の主張と必ず対で読むこと。
#[test]
fn the_untouched_sample_world_has_no_content_findings() {
    let world = World::normal();
    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
}

/// 内容の 6 判定それぞれに、見るべき材料が見本にあることを肯定の側から数える。
///
/// 台帳が空・証拠が空・関連が空の入力なら「所見 0 件」は無条件に真になる。判定が
/// 実際に何かを走査している土台をここで固定する（要件 6.13 の「検査対象が 0 件で
/// ない」側）。
#[test]
fn the_sample_world_holds_material_for_every_content_judgement() {
    let world = World::normal();
    let input = world.input();

    // 証拠の側（要件 6.5・6.6）。
    assert!(
        input.evidence.unresolved.is_empty(),
        "正常な見本に解決できない URL があってはならない"
    );
    assert_eq!(input.evidence.by_id.len(), 4, "証拠の付いた id の数");
    let implemented = input
        .ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.values())
        .filter(|entry| entry.status == Status::Implemented)
        .count();
    assert_eq!(implemented, 3, "実装済みの項目が無ければ 6.6 は空回りする");

    // 関連の側（要件 6.7）。
    let links: usize = input
        .ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.values())
        .map(|entry| entry.links.len())
        .sum();
    let aliases = input
        .ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.values())
        .filter(|entry| entry.alias_of.is_some())
        .count();
    let supersedes: usize = input
        .ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.values())
        .map(|entry| entry.supersedes.len())
        .sum();
    assert_eq!(links, 2, "関連の対");
    assert_eq!(aliases, 1, "別名の行");
    assert_eq!(supersedes, 1, "後継の参照");

    // 版の側（要件 6.7）。
    let introduced = input
        .ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.values())
        .filter(|entry| !entry.introduced.is_empty())
        .count();
    let versioned = input
        .catalog
        .entries
        .values()
        .filter(|entry| !entry.versions.is_empty())
        .count();
    assert_eq!(introduced, 6, "登場版の書かれた台帳の項目");
    assert_eq!(versioned, 7, "版番号を持つカタログの項目");

    // テーマの側（要件 6.8）。
    let values: usize = input
        .ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.values())
        .map(|entry| entry.values.len())
        .sum();
    assert_eq!(values, 7, "テーマの書かれた欄");
    assert_eq!(input.themes.len(), 8, "突き合わせ相手のテーマ");
}

// ---------------------------------------------------------------------------
// ソースの正典 URL がカタログに実在するか（要件 6.5・6.10・設計 D-4）
// ---------------------------------------------------------------------------

/// ソースの URL を 1 文字変えると `SourceUrlNotInCatalog` が出る。
///
/// **`ImplementedWithoutEvidence` が道連れになる**。すり替えた URL は実装済みの項目
/// `\s[ID]` の唯一の証拠なので、綴りが変われば同時にその項目の証拠が消える（要件 6.5
/// と 6.6 が同じ 1 行に載っている見本ではどうやっても分けられない）。2 種類を明示して
/// 主張し、次のテストで URL の側だけを 1 種類に切り出す。
#[test]
fn a_one_character_change_in_a_source_url_is_a_finding() {
    let mut world = World::normal();
    replace_in_source(
        &mut world,
        TAG_PATH,
        "list_sakura_script.html#_5c_73_5bID_5d:1",
        "list_sakura_script.html#_5c_73_5bID_5d:2",
    );

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![
            (FindingKind::SourceUrlNotInCatalog, 1),
            (FindingKind::ImplementedWithoutEvidence, 1),
        ]
    );

    let url = only_one(&outcome.findings, FindingKind::SourceUrlNotInCatalog);
    assert_eq!(url.id, None, "主語は URL であって項目ではない");
    assert_eq!(url.place, TAG_PATH, "場所は書かれていたソースのファイル");
    assert_eq!(
        url.detail,
        "カタログに無い正典 URL: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_73_5bID_5d:2"
    );

    let lost = only_one(&outcome.findings, FindingKind::ImplementedWithoutEvidence);
    assert_eq!(
        lost.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_sakura_script:_5c_73_5bID_5d:1")
    );
    assert_eq!(lost.place, ledger_place("sakura-script"));
}

/// 実装済みでない項目の行に誤った URL を置くと、URL の所見だけが 1 件出る。
///
/// `OnClose` の状態は `absent` なので証拠を求められない（要件 5.7）。ここだけが
/// `SourceUrlNotInCatalog` を単独で見せられる形である。
#[test]
fn a_bad_url_beside_an_unimplemented_item_is_only_a_url_finding() {
    let mut world = World::normal();
    replace_in_source(
        &mut world,
        EVENTS_PATH,
        "pub const ON_CLOSE",
        "/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnClose:2\npub const ON_CLOSE",
    );

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::SourceUrlNotInCatalog, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::SourceUrlNotInCatalog);
    assert_eq!(finding.id, None);
    assert_eq!(finding.place, EVENTS_PATH);
    assert_eq!(
        finding.detail,
        "カタログに無い正典 URL: https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnClose:2"
    );
}

/// 1 本のソースに誤った URL が 2 つあれば、**2 件とも**所見になる。
///
/// 要件 6.5 は「ソース中の正典 URL が**すべて**カタログに実在すること」を求める。
/// 2 つの誤りの**間に正しい URL を 1 本挟んである**ので、「最初の誤りで打ち切る」も
/// 「最初の正しい URL で打ち切る」も、どちらもここで赤になる。正しい URL を残して
/// あるから実装済みの項目の証拠は消えず、種類は 1 つに保たれる。
#[test]
fn every_unresolved_url_is_reported_not_just_the_first() {
    let mut world = World::normal();
    replace_in_source(
        &mut world,
        TAG_PATH,
        "/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_73_5bID_5d:1",
        concat!(
            "/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_61:9\n",
            "/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_73_5bID_5d:1\n",
            "/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_7a:9",
        ),
    );

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::SourceUrlNotInCatalog, 2)]
    );
    assert_eq!(
        details(&outcome.findings, FindingKind::SourceUrlNotInCatalog),
        vec![
            "カタログに無い正典 URL: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_61:9",
            "カタログに無い正典 URL: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_7a:9",
        ]
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::SourceUrlNotInCatalog),
        vec![TAG_PATH, TAG_PATH]
    );
}

// ---------------------------------------------------------------------------
// 実装済みには証拠が要る（要件 6.6・2.3・5.5）
// ---------------------------------------------------------------------------

/// 実装済みの項目から証拠の行を消すと `ImplementedWithoutEvidence` が 1 件だけ出る。
///
/// 行を丸ごと落とすので、誤った URL は 1 件も残らない——`SourceUrlNotInCatalog` を
/// 道連れにしない形はこれである。
#[test]
fn removing_the_evidence_line_of_an_implemented_entry_is_a_finding() {
    let mut world = World::normal();
    replace_in_source(
        &mut world,
        TAG_PATH,
        "/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_73_5bID_5d:1\n",
        "",
    );

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::ImplementedWithoutEvidence, 1)]
    );
    let finding = only_one(&outcome.findings, FindingKind::ImplementedWithoutEvidence);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_sakura_script:_5c_73_5bID_5d:1")
    );
    assert_eq!(finding.place, ledger_place("sakura-script"));
    assert_eq!(finding.detail, "正典 URL がソースに 1 件も無い");
}

/// 語彙表の目印の URL を 1 文字変えると、その表から生えていた証拠がまとめて消える。
///
/// `system.year` の証拠は URL の直書きではなく**名前の突き合わせ**で付いている
/// （設計 D-5）。目印だけを壊せば、直書きの証拠を 1 つも触らずに 6.6 の経路を試せる。
/// 同じ表から証拠を得ていた `currentghost.name` は `vocabulary-only` なので所見に
/// ならない（要件 5.7）。
#[test]
fn breaking_the_vocabulary_marker_url_costs_the_implemented_entry_its_evidence() {
    let mut world = World::normal();
    replace_in_source(
        &mut world,
        VOCAB_PATH,
        "manual/list_propertysystem.html",
        "manual/list_propertysysten.html",
    );

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![
            (FindingKind::SourceUrlNotInCatalog, 1),
            (FindingKind::ImplementedWithoutEvidence, 1),
        ]
    );
    let finding = only_one(&outcome.findings, FindingKind::ImplementedWithoutEvidence);
    assert_eq!(
        finding.id.as_ref().map(EntryId::as_str),
        Some("ukadoc:list_propertysystem:system.year:1")
    );
    assert_eq!(finding.place, ledger_place("property"));
    assert_eq!(finding.detail, "正典 URL がソースに 1 件も無い");
}

/// 証拠の無い実装済みが 2 つあれば、**2 件とも**所見になる。
///
/// 2 つの違反（shiori の `OnClose`・property の `system.month`）の**間に、証拠を持つ
/// 実装済みの項目**（sakura-script の `\s[ID]`）が挟まっている。「最初の違反で
/// 打ち切る」実装も「最初の正しい項目で打ち切る」実装も、ここで赤になる。
#[test]
fn every_implemented_entry_without_evidence_is_reported_not_just_the_first() {
    let mut world = World::normal();
    entry_mut(
        &mut world,
        Domain::Shiori,
        "ukadoc:list_shiori_event:OnClose:1",
    )
    .status = Status::Implemented;
    entry_mut(
        &mut world,
        Domain::Property,
        "ukadoc:list_propertysystem:system.month:1",
    )
    .status = Status::Implemented;
    world.refresh_reports();

    // 間に挟まる「正しい実装済み」が現にあることを先に確かめる。
    let between = world
        .evidence
        .by_id
        .get(&id("ukadoc:list_sakura_script:_5c_73_5bID_5d:1"));
    assert!(
        between.is_some_and(|paths| !paths.is_empty()),
        "違反 2 件の間に証拠つきの実装済みが無ければ試験が弱くなる"
    );

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::ImplementedWithoutEvidence, 2)]
    );
    assert_eq!(
        ids(&outcome.findings, FindingKind::ImplementedWithoutEvidence),
        vec![
            "ukadoc:list_shiori_event:OnClose:1",
            "ukadoc:list_propertysystem:system.month:1",
        ]
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::ImplementedWithoutEvidence),
        vec![ledger_place("shiori"), ledger_place("property")]
    );
}

/// 実装済みでない項目に証拠が無くても所見にならない（要件 5.7）。
///
/// 「証拠の無い項目」をすべて赤にする実装はここで赤になる。見本には証拠の無い項目が
/// 8 つある。
#[test]
fn an_entry_that_is_not_implemented_needs_no_evidence() {
    let world = World::normal();
    let without: usize = world
        .ledgers
        .iter()
        .flat_map(|ledger| ledger.entries.values())
        .filter(|entry| !world.evidence.by_id.contains_key(&entry.id))
        .count();
    assert_eq!(without, 8, "証拠の無い項目が無ければ試験にならない");

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
}
