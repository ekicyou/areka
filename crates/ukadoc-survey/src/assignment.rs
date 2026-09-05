//! ページ→ドメインの割り当て表（要件 3.1 の機械可読な正本）。
//!
//! 調査 spec 4 本が同時に走っても互いの作業が衝突しないように、ukadoc の 38 ページを
//! 4 つの担当ドメインへページ単位で割り振る。その割り振りの正本はここ 1 か所である
//! （台帳の `[ledger].pages` はこの表から書き出し、検査でも突き合わせる。設計
//! assignment「実装上の注意」）。
//!
//! 1 ページは 1 ドメインにしか属さない（要件 3.2・設計の不変条件）。この表に無い
//! ページがカタログに現れたら、そのページ名を挙げて失敗させる（要件 3.5）。失敗の
//! 仕立てはカタログを建てる段の仕事で、ここは「割り当ての無いページ」を並べて返す
//! ところまでを受け持つ。
//!
//! 実測（スナップショットの `source` が `ukadoc` の全 1,749 件）でのドメイン別件数は
//! shiori 677・assets 542・sakura-script 342・property 188 で、合計 1,749。要件 3.1 の
//! 表と一致する。件数そのものはこの層の API では扱わない——数えるのは実データを持つ
//! 段の仕事だからである。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。

use crate::model::{Domain, PageName};
use std::collections::{BTreeMap, BTreeSet};

/// shiori 担当の 12 ページ（要件 3.1 の表・転記順）。
const SHIORI_PAGES: [&str; 12] = [
    "list_shiori_event",
    "list_shiori_event_ex",
    "list_shiori_resource",
    "list_plugin_event",
    "memo_shiorievent",
    "spec_shiori3",
    "spec_fmo_mutex",
    "spec_web",
    "spec_sstp",
    "spec_dll",
    "spec_plugin",
    "spec_headline",
];

/// assets 担当の 24 ページ（要件 3.1 の表・転記順）。
const ASSETS_PAGES: [&str; 24] = [
    "descript_balloon",
    "descript_shell_surfaces",
    "descript_shell",
    "descript_ghost",
    "descript_install",
    "descript_plugin",
    "descript_headline",
    "descript_shell_surfacetable",
    "spec_update_file",
    "manual_balloon",
    "manual_directory",
    "manual_ghost",
    "manual_install",
    "manual_owner_draw_menu",
    "manual_shell",
    "manual_translator",
    "manual_update",
    "dev_bind",
    "dev_nar",
    "dev_ownerdraw",
    "dev_shell",
    "dev_shell_error",
    "dev_update",
    "memo",
];

/// sakura-script 担当の 1 ページ（要件 3.1 の表）。
const SAKURA_SCRIPT_PAGES: [&str; 1] = ["list_sakura_script"];

/// property 担当の 1 ページ（要件 3.1 の表）。
const PROPERTY_PAGES: [&str; 1] = ["list_propertysystem"];

/// ドメインごとの担当ページ（要件 3.1 の表を転記した順のまま）。
///
/// 既定の腕を置かない。ドメインを増やすと、担当ページを書き足すまでコンパイルが
/// 通らなくなる（`model.rs` の語彙と同じ守り方）。
///
/// 並びは要件 3.1 の表の順であって名前順ではない。名前順にするのは
/// [`PageAssignment::pages_of`] の仕事で、写し違えを見つけやすいよう、ここでは
/// 表に書かれたとおりの順を保つ。
fn canonical_pages(domain: Domain) -> &'static [&'static str] {
    match domain {
        Domain::Shiori => &SHIORI_PAGES,
        Domain::Assets => &ASSETS_PAGES,
        Domain::SakuraScript => &SAKURA_SCRIPT_PAGES,
        Domain::Property => &PROPERTY_PAGES,
    }
}

/// ページ名から担当ドメインを引く表。
///
/// 中身はページ名を鍵とする 1 本の表なので、1 ページに 2 つのドメインは持てない
/// （設計の不変条件はこの形で構造的に守られる）。表の元になる並びに同じ名前を
/// 2 度書いた場合だけは畳むときに黙って上書きされるので、そちらは在中テスト
/// `source_table_has_thirty_eight_pairwise_distinct_page_names` と
/// `folding_the_table_loses_no_page` が赤にする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageAssignment {
    by_page: BTreeMap<PageName, Domain>,
}

impl PageAssignment {
    /// 要件 3.1 の割り当て（38 ページ。shiori 12・assets 24・sakura-script 1・
    /// property 1）。
    pub fn canonical() -> Self {
        let mut by_page = BTreeMap::new();
        for domain in Domain::ALL {
            for page in canonical_pages(domain) {
                by_page.insert(PageName::new(*page), domain);
            }
        }
        Self { by_page }
    }

    /// ページの担当ドメイン。表に無ければ `None`。
    ///
    /// 引き当ては名前の丸ごとの一致で行う。ページ名自身が下線を含むので、下線で
    /// 割ったり接頭辞で拾ったりはしない（`memo` と `memo_shiorievent` は別の
    /// ドメイン、`descript_shell` と `descript_shell_surfaces` は別のページ）。
    pub fn domain_of(&self, page: &PageName) -> Option<Domain> {
        self.by_page.get(page).copied()
    }

    /// そのドメインの担当ページを名前順で返す。
    ///
    /// 表はページ名を鍵とする `BTreeMap` なので、走査した順がそのまま名前順になる。
    pub fn pages_of(&self, domain: Domain) -> Vec<PageName> {
        self.by_page
            .iter()
            .filter(|(_, assigned)| **assigned == domain)
            .map(|(page, _)| page.clone())
            .collect()
    }

    /// 与えたページのうち、どのドメインにも割り当てが無いものを名前順・重複無しで
    /// 返す（要件 3.5）。
    ///
    /// 同じページ名が何度現れても 1 度だけ挙げる。呼び手はこれをそのまま失敗の
    /// 文面に載せるので、入力の並びに左右されない決まった順で返す（要件 7.3）。
    pub fn unassigned<'a>(&self, pages: impl Iterator<Item = &'a PageName>) -> Vec<PageName> {
        pages
            .filter(|page| !self.by_page.contains_key(*page))
            .cloned()
            .collect::<BTreeSet<PageName>>()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
#[path = "assignment_tests.rs"]
mod tests;
