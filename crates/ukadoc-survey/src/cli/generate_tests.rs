//! 生成の副手続きが持つ「判断」だけを釘付けする（設計「入口 / cli」・要件 3.1・3.2・3.5）。
//!
//! 4 つの副手続き本体はスナップショットと repo の木が要るので、ここでは走らせない
//! （完了条件は実データでの通し確認の側にある）。このファイルが確かめるのは 4 つで、
//! はじめの 3 つは入出力に 1 度も触れずに決まる。
//!
//! - どの id をどの台帳へ入れるか（[`select_ids_by_domain`]）
//! - 台帳の前置きに担当ページをどの順で並べるか（[`prologue_pages`]）
//! - 既存の台帳の本文を差し込みへ渡しているか（[`plan_ledger_init`]・要件 3.3a）
//! - 既存の台帳を実際に取り寄せられるか（[`read_if_present`]・要件 3.3a）
//!
//! 2 つ目は一度書けば以後バイト列のまま写される（要件 3.3a）ので、順序の取り決めは
//! ここで逐語に固定しておく。3 つ目と 4 つ目は生成物を見比べても分からない——理由は
//! このファイルの後半に書いた。
//!
//! # 作らない。ただし repo の追跡ファイルは読む
//!
//! ファイルを 1 つも作らず、一時ディレクトリも使わない（設計 File Structure Plan。
//! 禁じてあるのは**作ること**である）。スナップショットも読まない。
//!
//! **repo に追跡されているファイルを読むことは禁じられていない。** 要件 6.2 は
//! 「スナップショットがその環境に無くても整合検査は赤にならない（repo 内のカタログを
//! 正本として検査する）」であって、repo のファイルを読むことをむしろ前提にしている。
//! 在中テストが repo の追跡ファイルを読む前例も既にある（`io/files_tests.rs:129` が
//! `Cargo.toml` を読む）。4 つ目の主張はこの読みだけを使う。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;

use super::{LedgerPlan, plan_ledger_init, prologue_pages, read_if_present, select_ids_by_domain};
use crate::assignment::PageAssignment;
use crate::catalog::{Catalog, CatalogEntry};
use crate::error::SurveyError;
use crate::io::paths;
use crate::ledger::write::render_initial_entry;
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

// ---------------------------------------------------------------------------
// 既存本文を渡している配線（要件 3.3a）
// ---------------------------------------------------------------------------
//
// 台帳が今はすべて未分類なので、既存本文を渡し忘れても**生成物は 1 バイトも
// 変わらない**。つまり出来上がったファイルを見比べても、この取り違えは 1 件も
// 見つからない。見つからないまま調査 spec が手で状態・担当・優先度を書き込むと、
// 次の `ledger-init` がそれを黙って初期値へ戻す。
//
// そこで [`plan_ledger_init`] は既存本文の取り寄せを引数で受ける。ここでは手で
// 書いた本文を渡し、その綴りが返る本文に生き残ることを確かめる。ファイルは
// 1 つも作らず、一時ディレクトリも使わない（設計 File Structure Plan）。

/// 台帳の置き場（実装の `io::paths` を引かず、独立した綴りで書く）。
const LEDGER_FILES: [&str; 4] = [
    "doc/ukadoc-coverage/ledger/shiori.toml",
    "doc/ukadoc-coverage/ledger/assets.toml",
    "doc/ukadoc-coverage/ledger/sakura-script.toml",
    "doc/ukadoc-coverage/ledger/property.toml",
];

/// property の台帳に手で書いた本文。生成器はこの綴りを 1 つも作らない
/// （欄の並び替え・余分な空白・複数行の備考・末尾の改行なし）。
///
/// 3 件のうち **真ん中** の `system.month` だけが欠けている。
const HAND_PROPERTY: &str = concat!(
    "# 手で書いた台帳。生成器が作らない綴りを混ぜてある。\n",
    "\n",
    "[ledger]\n",
    "domain   = \"property\"\n",
    "pages    = [ \"list_propertysystem\" ]\n",
    "\n",
    "[entry.\"ukadoc:list_propertysystem:currentghost.name:1\"]\n",
    "owner = \"areka-P0-ukadoc-survey-property\"\n",
    "status = \"implemented\"\n",
    "introduced = \"2.4.03\"\n",
    "priority = \"A1\"\n",
    "values = [ \"対話の手触り\" ]\n",
    "links = []\n",
    "note = \"\"\"\n",
    "手で書いた備考の 1 行目。\n",
    "2 行目。\n",
    "\"\"\"\n",
    "\n",
    "[entry.\"ukadoc:list_propertysystem:system.year:1\"]\n",
    "status=\"degraded\"\n",
    "owner=\"areka-P0-ukadoc-survey-property\"\n",
    "priority=\"C3\"\n",
    "note=\"末尾に改行を置かない書き方\"",
);

/// 差し込みの位置になる見出し行（`HAND_PROPERTY` の 2 つ目の塊の先頭）。
const PROPERTY_SECOND_HEAD: &str = "[entry.\"ukadoc:list_propertysystem:system.year:1\"]";

/// sakura-script の台帳に手で書いた本文。塊と塊の間に空行を置かず、末尾にも
/// 改行を置かない。3 件のうち **最後** の `_5c_73_5bID_5d` だけが欠けている。
const HAND_SAKURA: &str = concat!(
    "# 手で書いた台帳（空行を 1 つも置かない書き方）。\n",
    "[ledger]\n",
    "domain = \"sakura-script\"\n",
    "pages = [\"list_sakura_script\"]\n",
    "[entry.\"ukadoc:list_sakura_script:_5c_5f_71:1\"]\n",
    "status = \"absent\"\n",
    "owner = \"areka-P0-ukadoc-survey-sakura-script\"\n",
    "note = \"手書き 1\"\n",
    "[entry.\"ukadoc:list_sakura_script:_5c_65:1\"]\n",
    "status = \"vocabulary-only\"\n",
    "owner = \"areka-P0-ukadoc-survey-sakura-script\"\n",
    "note = \"手書き 2\"",
);

/// パスを「区切りが `/` の綴り」に均す。どの計算機でも同じ比較になる。
fn spelled_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// `doc/ukadoc-coverage/` から先だけを取り出す。
///
/// 場所はワークスペース根からの絶対パスで来るので、計算機ごとに違う頭を落とす。
/// その綴りが 1 度も現れなければ、落とさずに全部を返す——別の場所を指す壊し方を
/// 「合っている」と誤魔化さないためである。
fn under_coverage(path: &Path) -> String {
    let spelled = spelled_path(path);
    match spelled.find("doc/ukadoc-coverage/") {
        Some(at) => spelled[at..].to_owned(),
        None => spelled,
    }
}

/// 4 本ぶんの計画を組み立てる。渡した本文と、取り寄せに聞かれた場所を返す。
fn plan_with(hand: &[(&str, &str)]) -> (Vec<LedgerPlan>, Vec<String>) {
    let asked: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let bodies: Vec<(String, String)> = hand
        .iter()
        .map(|(file, body)| ((*file).to_owned(), (*body).to_owned()))
        .collect();

    let read = |path: &Path| -> Result<Option<String>, SurveyError> {
        let spelled = under_coverage(path);
        asked.borrow_mut().push(spelled.clone());
        Ok(bodies
            .iter()
            .find(|(file, _)| spelled.ends_with(file.as_str()))
            .map(|(_, body)| body.clone()))
    };

    let catalog = lib_test_support::catalog();
    let plans = plan_ledger_init(&catalog, &PageAssignment::canonical(), &read)
        .expect("見本のカタログと手書きの本文なら計画は立つはず");
    let asked = asked.into_inner();
    (plans, asked)
}

#[test]
fn the_plan_asks_for_each_ledger_at_its_own_place_and_writes_back_there() {
    // 取り寄せに渡す場所を間違える壊し方（カタログの場所を渡す・4 本を同じ場所で
    // 読む）は、本文の比較だけでは見えないことがある。聞かれた場所そのものを見る。
    let (plans, asked) = plan_with(&[]);

    assert_eq!(
        asked, LEDGER_FILES,
        "既存本文を取り寄せた場所が台帳 4 本と違う"
    );
    let targets: Vec<String> = plans
        .iter()
        .map(|plan| under_coverage(&plan.target))
        .collect();
    assert_eq!(targets, LEDGER_FILES, "書き出す先が台帳 4 本と違う");
    let counts: Vec<usize> = plans.iter().map(|plan| plan.count).collect();
    assert_eq!(
        counts,
        vec![3, 3, 3, 3],
        "見本のカタログはドメインごとに 3 件のはず"
    );
}

#[test]
fn a_hand_written_ledger_survives_the_plan_byte_for_byte() {
    // 要件 3.3a の本体。既存本文を渡し忘れると、この主張だけが赤くなる
    // （生成物の比較も件数の比較も、今の台帳では 1 件も捕まえない）。
    let (plans, _) = plan_with(&[
        (LEDGER_FILES[3], HAND_PROPERTY),
        (LEDGER_FILES[2], HAND_SAKURA),
    ]);

    // property: 真ん中の 1 件が、手書きの塊を 1 バイトも動かさずに差し込まれる。
    let month = EntryId::parse("ukadoc:list_propertysystem:system.month:1").expect("2 形のはず");
    let inserted = render_initial_entry(&month);
    let expected = HAND_PROPERTY.replace(
        PROPERTY_SECOND_HEAD,
        &format!("{inserted}{PROPERTY_SECOND_HEAD}"),
    );
    assert_eq!(
        plans[3].body, expected,
        "property の手書き本文が写されていない"
    );

    // sakura-script: 最後の 1 件。末尾に改行が無い本文へ改行 1 つだけを足して繋ぐ。
    let last = EntryId::parse("ukadoc:list_sakura_script:_5c_73_5bID_5d:1").expect("2 形のはず");
    let expected = format!("{HAND_SAKURA}\n{}", render_initial_entry(&last));
    assert_eq!(
        plans[2].body, expected,
        "sakura-script の手書き本文が写されていない"
    );
}

#[test]
fn the_hand_written_columns_are_still_spelled_the_way_the_owner_wrote_them() {
    // 上の逐語比較とは別に、「手で書いた値が生き残る」ことだけを名指しで書く。
    // 期待値の組み立て方をどう変えても、この主張は独立に立っている。
    let (plans, _) = plan_with(&[(LEDGER_FILES[3], HAND_PROPERTY)]);
    let body = &plans[3].body;

    for needle in [
        "status = \"implemented\"",
        "introduced = \"2.4.03\"",
        "owner = \"areka-P0-ukadoc-survey-property\"",
        "priority = \"A1\"",
        "values = [ \"対話の手触り\" ]",
        "手で書いた備考の 1 行目。",
        "status=\"degraded\"",
        "priority=\"C3\"",
    ] {
        assert!(
            body.contains(needle),
            "手で書いた {needle} が消えている（既存本文が渡っていない）"
        );
    }
    // 巻き戻ったときに現れる形そのもの——手書きの id の塊が初期値で書かれている——が
    // 1 つも無いこと。既存本文を渡し忘れると、これがそのまま本文に現れる。
    for id in [
        "ukadoc:list_propertysystem:currentghost.name:1",
        "ukadoc:list_propertysystem:system.year:1",
    ] {
        let rolled_back = render_initial_entry(&EntryId::parse(id).expect("2 形のはず"));
        assert!(
            !body.contains(&rolled_back),
            "手書きの塊 {id} が初期値へ巻き戻っている"
        );
    }
}

#[test]
fn a_ledger_with_no_existing_body_gets_the_generated_prologue() {
    // 既存が無いドメインは新規に組み立てる（要件 3.3）。前置きの担当ページを
    // 渡し忘れる壊し方をここで捕まえる。
    let (plans, _) = plan_with(&[(LEDGER_FILES[3], HAND_PROPERTY)]);

    assert!(
        plans[0]
            .body
            .starts_with("# doc/ukadoc-coverage/ledger/shiori.toml\n"),
        "shiori の前置きが生成されていない: {}",
        &plans[0].body[..plans[0].body.len().min(80)]
    );
    assert!(
        plans[0].body.contains("\"list_plugin_event\""),
        "shiori の前置きに担当ページが入っていない"
    );
    assert!(
        !plans[3]
            .body
            .starts_with("# doc/ukadoc-coverage/ledger/property.toml\n"),
        "手書きの前置きが生成された前置きへ置き換わっている"
    );
}

// ---------------------------------------------------------------------------
// 取り寄せそのもの（要件 3.3a）
// ---------------------------------------------------------------------------
//
// 上の一群は「呼び手が渡した取り寄せの結果を差し込みへ回しているか」を見る。渡す中身
// ——実在すれば本文を返し、無ければ `None` を返す——は [`read_if_present`] が持つ。
//
// ここは判断の分岐なので釘付けする。`Ok(true)` の枝を `Ok(None)` へ潰すと、実在する
// 台帳に「無い」と答えるようになり、次の `ledger-init` が既存の記入をすべて初期値へ
// 戻す（要件 3.3a が禁じる形そのもの）。生成物の比較では 1 件も捕まらない。
//
// repo に追跡されている台帳を **読むだけ** である。ファイルは作らず、一時ディレクトリも
// 使わない。

#[test]
fn read_if_present_returns_the_committed_ledger_and_none_for_a_missing_sibling() {
    let path = paths::ledger_path(Domain::Property);
    let body = read_if_present(&path)
        .expect("repo に追跡されている台帳は読めるはず")
        .unwrap_or_else(|| panic!("実在する台帳に None が返った: {}", path.display()));

    // 空の本文やよそのファイルで誤魔化されないよう、その台帳自身の 1 行目を見る。
    assert!(
        body.starts_with(
            "# doc/ukadoc-coverage/ledger/property.toml
"
        ),
        "返った本文が property の台帳のものではない: {}",
        body.lines().next().unwrap_or_default()
    );
    assert!(
        body.contains("domain = \"property\""),
        "返った本文に台帳の前置きが無い"
    );

    // 1 つの台帳だけを見ると、置き場の実在だけ確かめて本文はいつも property から読む
    // 取り違えを通してしまう。読む先が引数で決まっていることを 2 本目で確かめる。
    let other = paths::ledger_path(Domain::Shiori);
    let other_body = read_if_present(&other)
        .expect("repo に追跡されている台帳は読めるはず")
        .unwrap_or_else(|| panic!("実在する台帳に None が返った: {}", other.display()));
    assert!(
        other_body.starts_with(
            "# doc/ukadoc-coverage/ledger/shiori.toml
"
        ),
        "返った本文が shiori の台帳のものではない: {}",
        other_body.lines().next().unwrap_or_default()
    );

    // 実在しない兄弟は `None`。ここが `Some` になると、無いはずの本文を写す。
    let missing = path.with_file_name("property.toml.not-a-file");
    assert!(
        !missing.exists(),
        "この置き場は実在してはならない: {}",
        missing.display()
    );
    assert_eq!(
        read_if_present(&missing).expect("実在しない置き場でも失敗しない"),
        None,
        "実在しない置き場に本文が返った"
    );
}
