//! `assignment.rs` の在中テスト。
//!
//! この表の危うさは 1 つしかない——**写し違え**である。ページ名を 1 文字打ち間違えても
//! 「38 ページ」「shiori は 12 ページ」といった数だけの検査は緑のまま通る。だから
//! ここでは 4 つのドメインすべてについて、**担当ページの名前を並び順ごと逐語で釘付け**
//! にする。数は「いくつ」しか言わないが、逐語の並びは「どれ」を言う。
//!
//! 守るのは 6 つ。⑴ 割り当てが 38 ページで、内訳が shiori 12・assets 24・
//! sakura-script 1・property 1 であること（要件 3.1・設計 assignment の事後条件）。
//! ⑵ 各ドメインの担当ページが逐語で一致し、名前順で返ること。⑶ 38 個の名前が互いに
//! 異なること＝1 ページは 1 ドメインにしか属さない（要件 3.2・設計の不変条件）。
//! ⑷ ページからドメインが引けること、表に無いページでは引けないこと。
//! ⑸ 割り当ての無いページを名前順・重複無しで挙げられること（要件 3.5）。
//! ⑹ ページ名は下線を含んだ丸ごとの名前として扱われること（`memo` と
//!    `memo_shiorievent` は別のドメイン、`descript_shell` と
//!    `descript_shell_surfaces` は別のページ）。
//!
//! スナップショットにもファイルにも触らない（要件 6.2）。すべて値だけで完結する。

use super::*;
use std::collections::BTreeSet;

/// 逐語比較のために、ページ名の並びを綴りの並びへ落とす。
fn spellings(pages: &[PageName]) -> Vec<&str> {
    pages.iter().map(PageName::as_str).collect()
}

/// 要件 3.1 の shiori 担当 12 ページ（名前順）。
fn expected_shiori_pages() -> Vec<&'static str> {
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
    ]
}

/// 要件 3.1 の assets 担当 24 ページ（名前順）。
fn expected_assets_pages() -> Vec<&'static str> {
    vec![
        "descript_balloon",
        "descript_ghost",
        "descript_headline",
        "descript_install",
        "descript_plugin",
        "descript_shell",
        "descript_shell_surfaces",
        "descript_shell_surfacetable",
        "dev_bind",
        "dev_nar",
        "dev_ownerdraw",
        "dev_shell",
        "dev_shell_error",
        "dev_update",
        "manual_balloon",
        "manual_directory",
        "manual_ghost",
        "manual_install",
        "manual_owner_draw_menu",
        "manual_shell",
        "manual_translator",
        "manual_update",
        "memo",
        "spec_update_file",
    ]
}

/// 要件 3.1 の sakura-script 担当 1 ページ（名前順）。
fn expected_sakura_script_pages() -> Vec<&'static str> {
    vec!["list_sakura_script"]
}

/// 要件 3.1 の property 担当 1 ページ（名前順）。
fn expected_property_pages() -> Vec<&'static str> {
    vec!["list_propertysystem"]
}

// ---- 担当ページの逐語一致（この節がこのモジュールの本体）----

#[test]
fn shiori_pages_are_the_twelve_pages_of_requirement_3_1() {
    let assignment = PageAssignment::canonical();
    let got = assignment.pages_of(Domain::Shiori);
    assert_eq!(spellings(&got), expected_shiori_pages());
}

#[test]
fn assets_pages_are_the_twenty_four_pages_of_requirement_3_1() {
    let assignment = PageAssignment::canonical();
    let got = assignment.pages_of(Domain::Assets);
    assert_eq!(spellings(&got), expected_assets_pages());
}

#[test]
fn sakura_script_pages_are_the_single_page_of_requirement_3_1() {
    let assignment = PageAssignment::canonical();
    let got = assignment.pages_of(Domain::SakuraScript);
    assert_eq!(spellings(&got), expected_sakura_script_pages());
}

#[test]
fn property_pages_are_the_single_page_of_requirement_3_1() {
    let assignment = PageAssignment::canonical();
    let got = assignment.pages_of(Domain::Property);
    assert_eq!(spellings(&got), expected_property_pages());
}

/// 返る並びが名前順であること自体も釘付けにする。上の 4 本の期待値は名前順で
/// 書いてあるので、実装が転記順のまま返すと assets と shiori が赤くなる。
#[test]
fn pages_of_returns_names_in_sorted_order() {
    let assignment = PageAssignment::canonical();
    for domain in Domain::ALL {
        let got = assignment.pages_of(domain);
        let mut sorted = got.clone();
        sorted.sort();
        assert_eq!(got, sorted, "{} の並びが名前順でない", domain.as_key());
    }
}

// ---- 件数と内訳（要件 3.1・設計 assignment の事後条件）----

#[test]
fn canonical_assigns_exactly_thirty_eight_pages() {
    let assignment = PageAssignment::canonical();
    let total: usize = Domain::ALL
        .into_iter()
        .map(|domain| assignment.pages_of(domain).len())
        .sum();
    assert_eq!(total, 38);
}

#[test]
fn canonical_split_is_twelve_twenty_four_one_and_one() {
    let assignment = PageAssignment::canonical();
    assert_eq!(assignment.pages_of(Domain::Shiori).len(), 12);
    assert_eq!(assignment.pages_of(Domain::Assets).len(), 24);
    assert_eq!(assignment.pages_of(Domain::SakuraScript).len(), 1);
    assert_eq!(assignment.pages_of(Domain::Property).len(), 1);
}

// ---- 1 ページは 1 ドメインにしか属さない（要件 3.2・設計の不変条件）----

/// 元の表（ドメインごとの並び）を平らにした 38 個の綴りが互いに異なること。
///
/// 割り当ては綴りを鍵とする表に畳まれるので、同じ名前を 2 つのドメインに書くと
/// 後の 1 つが前の 1 つを黙って上書きし、件数だけが 1 減る。ここは畳む前の並びを
/// 直に数えるので、その取り違えが赤になる。
#[test]
fn source_table_has_thirty_eight_pairwise_distinct_page_names() {
    let flat: Vec<&str> = Domain::ALL
        .into_iter()
        .flat_map(|domain| canonical_pages(domain).iter().copied())
        .collect();
    assert_eq!(flat.len(), 38, "表に並んだ綴りが 38 個でない");
    let distinct: BTreeSet<&str> = flat.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        38,
        "同じページ名が 2 度書かれている: {flat:?}"
    );
}

/// 畳んだ表の大きさが、畳む前の並びの長さと等しいこと（＝上書きが起きていない）。
#[test]
fn folding_the_table_loses_no_page() {
    let assignment = PageAssignment::canonical();
    let flat_len = Domain::ALL
        .into_iter()
        .map(|domain| canonical_pages(domain).len())
        .sum::<usize>();
    assert_eq!(assignment.by_page.len(), flat_len);
}

/// ドメインの担当ページ同士が交わらないこと。
#[test]
fn domains_share_no_page() {
    let assignment = PageAssignment::canonical();
    for (i, left) in Domain::ALL.into_iter().enumerate() {
        for right in Domain::ALL.into_iter().skip(i + 1) {
            let a: BTreeSet<PageName> = assignment.pages_of(left).into_iter().collect();
            let b: BTreeSet<PageName> = assignment.pages_of(right).into_iter().collect();
            let shared: Vec<&PageName> = a.intersection(&b).collect();
            assert!(
                shared.is_empty(),
                "{} と {} が同じページを持つ: {shared:?}",
                left.as_key(),
                right.as_key()
            );
        }
    }
}

// ---- ページからドメインを引く ----

#[test]
fn domain_of_resolves_a_representative_page_of_each_domain() {
    let assignment = PageAssignment::canonical();
    let cases = [
        ("list_shiori_event", Domain::Shiori),
        ("descript_balloon", Domain::Assets),
        ("list_sakura_script", Domain::SakuraScript),
        ("list_propertysystem", Domain::Property),
    ];
    for (page, expected) in cases {
        assert_eq!(
            assignment.domain_of(&PageName::new(page)),
            Some(expected),
            "{page} のドメインが違う"
        );
    }
}

/// 表に載っている 38 ページすべてが、それを載せたドメインへ戻ること。
#[test]
fn domain_of_round_trips_every_assigned_page() {
    let assignment = PageAssignment::canonical();
    for domain in Domain::ALL {
        for page in assignment.pages_of(domain) {
            assert_eq!(
                assignment.domain_of(&page),
                Some(domain),
                "{} が {} へ戻らない",
                page.as_str(),
                domain.as_key()
            );
        }
    }
}

#[test]
fn domain_of_returns_none_for_a_page_outside_the_table() {
    let assignment = PageAssignment::canonical();
    for page in [
        "list_new_page",
        "",
        "ukadoc",
        "descript_shell_surfacetables",
    ] {
        assert_eq!(
            assignment.domain_of(&PageName::new(page)),
            None,
            "{page:?} は表に無い"
        );
    }
}

// ---- 割り当ての無いページを挙げる（要件 3.5）----

#[test]
fn unassigned_is_empty_when_every_page_is_assigned() {
    let assignment = PageAssignment::canonical();
    let pages: Vec<PageName> = Domain::ALL
        .into_iter()
        .flat_map(|domain| assignment.pages_of(domain))
        .collect();
    assert_eq!(assignment.unassigned(pages.iter()), Vec::<PageName>::new());
}

#[test]
fn unassigned_is_empty_for_no_pages_at_all() {
    let assignment = PageAssignment::canonical();
    let pages: Vec<PageName> = Vec::new();
    assert_eq!(assignment.unassigned(pages.iter()), Vec::<PageName>::new());
}

#[test]
fn unassigned_names_the_pages_that_have_no_domain() {
    let assignment = PageAssignment::canonical();
    let pages: Vec<PageName> = ["list_new_page", "descript_balloon", "spec_unknown"]
        .into_iter()
        .map(PageName::new)
        .collect();
    let got = assignment.unassigned(pages.iter());
    assert_eq!(spellings(&got), vec!["list_new_page", "spec_unknown"]);
}

/// 同じ名前が何度現れても 1 度だけ、並びは名前順（要件 7.3 の決定論）。
#[test]
fn unassigned_is_sorted_and_deduplicated() {
    let assignment = PageAssignment::canonical();
    let pages: Vec<PageName> = [
        "spec_unknown",
        "list_new_page",
        "spec_unknown",
        "list_shiori_event",
        "aaa_new_page",
        "list_new_page",
    ]
    .into_iter()
    .map(PageName::new)
    .collect();
    let got = assignment.unassigned(pages.iter());
    assert_eq!(
        spellings(&got),
        vec!["aaa_new_page", "list_new_page", "spec_unknown"]
    );
}

// ---- 下線を含んだ丸ごとの名前として扱う ----

/// `memo` は assets、`memo_shiorievent` は shiori。下線で割ったり接頭辞で拾ったり
/// すると、この 2 つが取り違わる。
#[test]
fn underscore_names_are_whole_names_not_prefixes() {
    let assignment = PageAssignment::canonical();
    assert_eq!(
        assignment.domain_of(&PageName::new("memo")),
        Some(Domain::Assets)
    );
    assert_eq!(
        assignment.domain_of(&PageName::new("memo_shiorievent")),
        Some(Domain::Shiori)
    );
    // 一方が他方の接頭辞になっている組も、それぞれ別のページとして載っている。
    for page in [
        "descript_shell",
        "descript_shell_surfaces",
        "descript_shell_surfacetable",
        "dev_shell",
        "dev_shell_error",
        "list_shiori_event",
        "list_shiori_event_ex",
    ] {
        assert!(
            assignment.domain_of(&PageName::new(page)).is_some(),
            "{page} が表に無い"
        );
    }
    // 下線の手前で切った名前は、どれも表に無い（`memo` だけは実在するので挙げない）。
    for truncated in [
        "descript",
        "dev",
        "list",
        "manual",
        "spec",
        "memo_shiori",
        "descript_shell_surface",
        "list_shiori",
    ] {
        assert_eq!(
            assignment.domain_of(&PageName::new(truncated)),
            None,
            "{truncated} は丸ごとのページ名ではない"
        );
    }
}
