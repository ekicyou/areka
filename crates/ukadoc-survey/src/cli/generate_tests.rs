//! 生成の副手続きが持つ「判断」だけを釘付けする（設計「入口 / cli」・要件 3.1・3.2・3.5）。
//!
//! 4 つの副手続き本体はスナップショットと repo の木が要るので、ここでは走らせない
//! （完了条件は実データでの通し確認の側にある）。このファイルが確かめるのは、
//! 入出力に触れずに決まる 2 つの判断だけである。
//!
//! - どの id をどの台帳へ入れるか（[`select_ids_by_domain`]）
//! - 台帳の前置きに担当ページをどの順で並べるか（[`prologue_pages`]）
//!
//! 後者は一度書けば以後バイト列のまま写される（要件 3.3a）ので、順序の取り決めは
//! ここで逐語に固定しておく。
//!
//! このファイルはファイルを 1 つも作らず、一時ディレクトリも使わず、スナップショットも
//! 読まない（要件 6.2・設計 File Structure Plan）。

use std::collections::BTreeMap;

use super::{prologue_pages, select_ids_by_domain};
use crate::assignment::PageAssignment;
use crate::catalog::{Catalog, CatalogEntry};
use crate::error::SurveyError;
use crate::lib_test_support;
use crate::model::{Domain, EntryId, PageName};

/// 仕分けの結果を「ドメインの綴り → id の綴りの一覧」に均す小道具。
fn spelled(buckets: &BTreeMap<Domain, Vec<EntryId>>) -> BTreeMap<&'static str, Vec<&str>> {
    buckets
        .iter()
        .map(|(domain, ids)| {
            (
                domain.as_key(),
                ids.iter().map(EntryId::as_str).collect::<Vec<&str>>(),
            )
        })
        .collect()
}

#[test]
fn each_sample_id_lands_in_the_ledger_that_owns_its_page() {
    // 件数だけを確かめると、12 件が全部よそのドメインへ入っていても緑になる。
    // どの id がどの台帳へ行くかを綴りで書く。
    let catalog = lib_test_support::catalog();
    let buckets = select_ids_by_domain(&catalog, &PageAssignment::canonical())
        .expect("見本のカタログはすべて割り当て済みのページのはず");

    let expected: BTreeMap<&str, Vec<&str>> = BTreeMap::from([
        (
            "shiori",
            vec![
                "ukadoc:list_shiori_event:OnBoot:1",
                "ukadoc:list_shiori_event:OnClose:1",
                "ukadoc:spec_shiori3",
            ],
        ),
        (
            "assets",
            vec![
                "ukadoc:descript_ghost:charset:1",
                "ukadoc:descript_ghost:name:1",
                "ukadoc:manual_shell",
            ],
        ),
        (
            "sakura-script",
            vec![
                "ukadoc:list_sakura_script:_5c_5f_71:1",
                "ukadoc:list_sakura_script:_5c_65:1",
                "ukadoc:list_sakura_script:_5c_73_5bID_5d:1",
            ],
        ),
        (
            "property",
            vec![
                "ukadoc:list_propertysystem:currentghost.name:1",
                "ukadoc:list_propertysystem:system.month:1",
                "ukadoc:list_propertysystem:system.year:1",
            ],
        ),
    ]);
    assert_eq!(spelled(&buckets), expected, "id の行き先が割り当て表と違う");
}

#[test]
fn no_id_is_dropped_and_no_id_is_placed_twice() {
    // 要件 3.2（同じ id を 2 つ以上の台帳に置かない）と、黙って 1 件落とす壊れ方の対。
    let catalog = lib_test_support::catalog();
    let buckets = select_ids_by_domain(&catalog, &PageAssignment::canonical())
        .expect("見本のカタログはすべて割り当て済みのページのはず");

    let mut placed: Vec<&str> = buckets
        .values()
        .flat_map(|ids| ids.iter().map(EntryId::as_str))
        .collect();
    let before = placed.len();
    placed.sort_unstable();
    placed.dedup();
    assert_eq!(placed.len(), before, "同じ id が 2 つの台帳へ入っている");

    let all: Vec<&str> = catalog.entries.keys().map(EntryId::as_str).collect();
    assert_eq!(placed, all, "カタログの id と台帳へ入った id が一致しない");
}

#[test]
fn a_domain_that_owns_no_entry_still_gets_a_bucket() {
    // 台帳は 4 本と決まっている（要件 3.1）。項目 0 件のドメインの鍵が消えると、
    // その台帳だけが書き出されずに黙って欠ける。
    let mut catalog = lib_test_support::catalog();
    catalog
        .entries
        .retain(|id, _| id.page() == PageName::new("list_propertysystem"));
    let buckets = select_ids_by_domain(&catalog, &PageAssignment::canonical())
        .expect("見本のカタログはすべて割り当て済みのページのはず");

    let keys: Vec<&str> = buckets.keys().map(Domain::as_key).collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 4, "台帳 4 本ぶんの鍵が揃っていない: {keys:?}");
    for domain in Domain::ALL {
        let ids = buckets
            .get(&domain)
            .unwrap_or_else(|| panic!("ドメイン {} の鍵が無い", domain.as_key()));
        let expected = if domain == Domain::Property { 3 } else { 0 };
        assert_eq!(
            ids.len(),
            expected,
            "ドメイン {} の件数が違う",
            domain.as_key()
        );
    }
}

#[test]
fn an_entry_whose_page_belongs_to_no_ledger_is_named_and_refused() {
    // 割り当ての無いページの id を黙って落とすと、件数が減るだけで何も言わない
    // （要件 3.5・設計 Error Handling）。
    let mut catalog: Catalog = lib_test_support::catalog();
    let stray = EntryId::parse("ukadoc:list_no_such_page:thing:1").expect("2 形のはず");
    catalog.entries.insert(
        stray.clone(),
        CatalogEntry {
            page: stray.page(),
            title: "thing".to_owned(),
            category: "misc".to_owned(),
            versions: Vec::new(),
            hash: "9999999999999999".to_owned(),
            url: "https://ssp.shillest.net/ukadoc/manual/list_no_such_page.html#thing:1".to_owned(),
            id: stray,
        },
    );

    let err = select_ids_by_domain(&catalog, &PageAssignment::canonical())
        .expect_err("割り当ての無いページが黙って落とされた");
    assert!(
        matches!(err, SurveyError::PageNotAssigned { .. }),
        "別の失敗になっている: {err}"
    );
    let body = err.to_string();
    assert!(
        body.contains("list_no_such_page"),
        "どのページが割り当て無しなのかが本文に無い: {body}"
    );
}

#[test]
fn the_prologue_lists_the_pages_in_name_order() {
    // 前置きは一度書いたら以後バイト列のまま写される（要件 3.3a）ので、順序の
    // 取り決めはここで凍る。期待値は実装の表を引かず、独立した綴りで書く。
    let pages: Vec<String> = prologue_pages(&PageAssignment::canonical(), Domain::Shiori)
        .iter()
        .map(|page| page.as_str().to_owned())
        .collect();
    assert_eq!(
        pages,
        vec![
            "list_plugin_event",
            "list_shiori_event",
            "list_shiori_event_ex",
            "list_shiori_resource",
            "memo_shiorievent",
            "spec_dll",
            "spec_fmo_mutex",
            "spec_headline",
            "spec_plugin",
            "spec_shiori3",
            "spec_sstp",
            "spec_web",
        ],
        "前置きの担当ページが名前順でない"
    );
}

#[test]
fn the_prologue_order_is_not_the_transcription_order_of_the_requirement_table() {
    // 名前順を選んだことの対照。要件 3.1 の表の転記順（件数の多い順に近い並び）を
    // 渡す実装に差し替えると、上のテストと合わせてここが赤になる。
    let pages = prologue_pages(&PageAssignment::canonical(), Domain::Shiori);
    let first = pages.first().expect("shiori は 12 ページ持つはず");
    assert_eq!(
        first.as_str(),
        "list_plugin_event",
        "名前順なら先頭は list_plugin_event（要件 3.1 の転記順なら list_shiori_event）"
    );
    assert_eq!(pages.len(), 12, "shiori の担当ページが 12 ページでない");
}

#[test]
fn a_one_page_domain_still_declares_its_page() {
    // 1 ページのドメインは並び順を 1 つも守らない（タスク 2.5 の教訓）ので、
    // 上の 2 本は 12 ページの shiori で書いた。こちらは「空にならない」だけを見る。
    for (domain, page) in [
        (Domain::SakuraScript, "list_sakura_script"),
        (Domain::Property, "list_propertysystem"),
    ] {
        let pages = prologue_pages(&PageAssignment::canonical(), domain);
        let spelled: Vec<&str> = pages.iter().map(PageName::as_str).collect();
        assert_eq!(
            spelled,
            vec![page],
            "ドメイン {} の担当ページが違う",
            domain.as_key()
        );
    }
}
