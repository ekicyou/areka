//! `read.rs` の在中テスト。
//!
//! 守るのは 4 つ。⑴ 手で書いた本文を読んで書き戻すと 1 バイトも違わないこと
//! （設計 catalog の事後条件 `write(read(t)) == t`・丸めが無いこと）。⑵ 欄の値が
//! 逐語で読めること（逃がした逆斜線が 1 つに戻る・全件数と正典の件数が別々に読める）。
//! ⑶ 形が違う本文を**繕わずに落とす**こと。落ちる場所を本文が名指しすること。
//! ⑷ 3 つの読み出し（[`Catalog::by_url`]・[`Catalog::page_urls`]・
//! [`Catalog::titles_of_page`]）が具体の値で決まること。
//!
//! 読み戻し一致だけに頼らない（タスク 1.4 の教訓）。往復は「単射であること」しか
//! 言えず、欄を並べ替えても余計に逃がしても素通りする。逐語の主張は `write_tests.rs`
//! が持ち、ここでは値そのものを 1 つずつ釘付けする。
//!
//! **ファイルも一時ディレクトリも作らない。環境変数も読まない**（要件 6.2）。

use super::*;
use crate::catalog::write::write;
use crate::model::{EntryId, PageName};

/// 手で書いた見本の本文。
///
/// 逆斜線を含む見出し（設計 D-10 の実測 316 件の形）・単引用符・日本語・版番号 2 つ・
/// 版番号 0 個・アンカー無しの id・アンカー付きの id をすべて 1 つの本文に入れてある。
const SAMPLE: &str = r##"# 機械生成。手で編集しない。再生成: cargo run -p ukadoc-survey -- catalog
# 形式の正本: .kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/design.md

[snapshot]
package = "ukagaka-doc-mcp"
package_version = "0.2.7"
snapshot_version = 1
generated_at = "2026-08-24T04:08:57.881Z"
total_entries = 2983
ukadoc_entries = 1749
catalog_format = 1
hash_algorithm = "fnv1a64"

[entry]
"ukadoc:dev_bind" = { page = "dev_bind", title = "着せ替えの仕組み", category = "dev_guide", versions = [], hash = "1a2b3c4d5e6f7081", url = "https://ssp.shillest.net/ukadoc/manual/dev_bind.html" }
"ukadoc:list_propertysystem:system.year:1" = { page = "list_propertysystem", title = "system.year", category = "protocol", versions = ["2.3.53", "2.5.60"], hash = "0f1e2d3c4b5a6978", url = "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#system.year:1" }
"ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2cID_5d:1" = { page = "list_sakura_script", title = "\\![get,property,ID]", category = "sakurascript", versions = ["2.4.00"], hash = "00000000000000ff", url = "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bget_2cproperty_2cID_5d:1" }
"ukadoc:list_sakura_script:_5c_5f_71:1" = { page = "list_sakura_script", title = "選択肢の'既定'を消す", category = "sakurascript", versions = [], hash = "fedcba9876543210", url = "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_5f_71:1" }
"##;

/// 見本を読む。読めなければテストを落とす。
fn sample() -> Catalog {
    match read(SAMPLE) {
        Ok(catalog) => catalog,
        Err(err) => panic!("見本は読めるはず: {err}"),
    }
}

/// 見本の 1 か所を差し替える。差し替えが実際に起きたことも確かめる
/// （置き換わらないまま緑になると、その摂動は何も試していない）。
fn mutated(from: &str, to: &str) -> String {
    assert!(SAMPLE.contains(from), "見本に {from} が無い");
    let body = SAMPLE.replace(from, to);
    assert_ne!(body, SAMPLE, "差し替えが起きていない");
    body
}

/// 読み取りが落ちることを確かめ、失敗を返す。
fn read_error(text: &str) -> SurveyError {
    match read(text) {
        Ok(_) => panic!("この本文は落ちるはず"),
        Err(err) => err,
    }
}

/// 見本の 1 項目を取り出す。
fn entry_of<'a>(catalog: &'a Catalog, id: &str) -> &'a CatalogEntry {
    let id = EntryId::parse(id).expect("見本の id は 2 形のはず");
    catalog.entries.get(&id).expect("見本にある id のはず")
}

// ---- 往復 ----

/// 読んで書き戻すと 1 バイトも違わない（設計 catalog の事後条件・要件 1.5）。
#[test]
fn round_trip_of_a_hand_written_body_is_byte_identical() {
    assert_eq!(write(&sample()), SAMPLE);
}

/// 項目が 1 つも無い本文でも往復する。
#[test]
fn round_trip_of_an_empty_entry_table() {
    let body = SAMPLE
        .split_once("[entry]\n")
        .map(|(head, _)| format!("{head}[entry]\n"))
        .expect("見本に [entry] があるはず");
    let catalog = read(&body).expect("空の表も読めるはず");
    assert!(catalog.entries.is_empty());
    assert_eq!(write(&catalog), body);
}

// ---- 欄の値 ----

/// 冒頭の情報が丸められずに読める。全件数と正典の件数は**別の欄**（タスク 2.1 の教訓）。
#[test]
fn snapshot_fields_are_read_without_rounding() {
    let meta = sample().snapshot;
    assert_eq!(meta.package, "ukagaka-doc-mcp");
    assert_eq!(meta.package_version, "0.2.7");
    assert_eq!(meta.snapshot_version, 1);
    assert_eq!(meta.generated_at, "2026-08-24T04:08:57.881Z");
    assert_eq!(meta.total_entries, 2983);
    assert_eq!(meta.ukadoc_entries, 1749);
    assert_eq!(meta.catalog_format, 1);
    assert_eq!(meta.hash_algorithm, "fnv1a64");
}

/// 逃がした逆斜線が 1 つに戻り、版番号は書かれた順のまま読める。
#[test]
fn entry_columns_are_read_verbatim() {
    let catalog = sample();
    let entry = entry_of(
        &catalog,
        "ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2cID_5d:1",
    );
    assert_eq!(entry.page, PageName::new("list_sakura_script"));
    assert_eq!(entry.title, r"\![get,property,ID]");
    assert_eq!(entry.category, "sakurascript");
    assert_eq!(entry.versions, vec!["2.4.00".to_owned()]);
    assert_eq!(entry.hash, "00000000000000ff");
    assert_eq!(
        entry.url,
        "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bget_2cproperty_2cID_5d:1"
    );

    let two = entry_of(&catalog, "ukadoc:list_propertysystem:system.year:1");
    assert_eq!(
        two.versions,
        vec!["2.3.53".to_owned(), "2.5.60".to_owned()],
        "版番号は 1 つに絞らない（要件 1.2）"
    );

    let none = entry_of(&catalog, "ukadoc:dev_bind");
    assert!(none.versions.is_empty());
    assert_eq!(none.title, "着せ替えの仕組み");
    assert_eq!(
        entry_of(&catalog, "ukadoc:list_sakura_script:_5c_5f_71:1").title,
        "選択肢の'既定'を消す"
    );
}

/// 項目は id の byte 昇順で並ぶ。
#[test]
fn entries_are_ordered_by_id() {
    let catalog = sample();
    let ids: Vec<&str> = catalog.entries.keys().map(EntryId::as_str).collect();
    assert_eq!(
        ids,
        vec![
            "ukadoc:dev_bind",
            "ukadoc:list_propertysystem:system.year:1",
            "ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2cID_5d:1",
            "ukadoc:list_sakura_script:_5c_5f_71:1",
        ]
    );
}

// ---- 繕わずに落ちる ----

/// TOML として読めない本文は、場所を名指しして落ちる。
#[test]
fn not_toml_is_rejected() {
    let message = read_error("@@@ これは TOML ではない").to_string();
    assert!(message.contains("catalog.toml"), "{message}");
}

/// 冒頭の情報の欄が欠けていたら落ちる。
#[test]
fn missing_snapshot_field_is_rejected() {
    let body = mutated("hash_algorithm = \"fnv1a64\"\n", "");
    let message = read_error(&body).to_string();
    assert!(message.contains("hash_algorithm"), "{message}");
    assert!(message.contains("snapshot"), "{message}");
}

/// `[snapshot]` そのものが無ければ落ちる。
#[test]
fn missing_snapshot_table_is_rejected() {
    let body = mutated("[snapshot]\n", "");
    let message = read_error(&body).to_string();
    assert!(message.contains("snapshot"), "{message}");
}

/// `[entry]` そのものが無ければ落ちる（項目 0 件と区別する）。
#[test]
fn missing_entry_table_is_rejected() {
    let body = SAMPLE
        .split_once("[entry]\n")
        .map(|(head, _)| head.to_owned())
        .expect("見本に [entry] があるはず");
    let message = read_error(&body).to_string();
    assert!(message.contains("entry"), "{message}");
}

/// 欄の型が違えば落ちる。数を文字列で書いた場合。
#[test]
fn wrong_typed_count_is_rejected() {
    let body = mutated("total_entries = 2983", "total_entries = \"2983\"");
    let message = read_error(&body).to_string();
    assert!(message.contains("total_entries"), "{message}");
}

/// 欄の型が違えば落ちる。文字列を数で書いた場合。
#[test]
fn wrong_typed_string_is_rejected() {
    let body = mutated("package = \"ukagaka-doc-mcp\"", "package = 7");
    let message = read_error(&body).to_string();
    assert!(message.contains("package"), "{message}");
}

/// 件数に負の数は入らない。
#[test]
fn negative_count_is_rejected() {
    let body = mutated("ukadoc_entries = 1749", "ukadoc_entries = -1");
    let message = read_error(&body).to_string();
    assert!(message.contains("ukadoc_entries"), "{message}");
}

/// 項目 id が 2 形のどちらでもなければ落ちる（要件 1.9）。
#[test]
fn malformed_entry_id_is_rejected() {
    let body = mutated("\"ukadoc:dev_bind\" = {", "\"ukadoc\" = {");
    let message = read_error(&body).to_string();
    assert!(message.contains("ukadoc"), "{message}");
    assert!(
        matches!(read_error(&body), SurveyError::BadEntryId { .. }),
        "id の失敗は BadEntryId で表す"
    );
}

/// `page` の欄が id のページと食い違えば落ちる。**書かれた値を捨てて id から
/// 作り直したりはしない**（食い違いを繕うと、壊れたカタログが黙って通る）。
#[test]
fn page_column_disagreeing_with_the_id_is_rejected() {
    let body = mutated(
        "\"ukadoc:dev_bind\" = { page = \"dev_bind\"",
        "\"ukadoc:dev_bind\" = { page = \"dev_guide\"",
    );
    let message = read_error(&body).to_string();
    assert!(message.contains("ukadoc:dev_bind"), "{message}");
    assert!(message.contains("dev_guide"), "{message}");
    assert!(message.contains("page"), "{message}");
}

/// 項目の値がインラインテーブルでなければ落ちる。
#[test]
fn entry_value_that_is_not_a_table_is_rejected() {
    let body = mutated(
        "\"ukadoc:dev_bind\" = { page = \"dev_bind\", title = \"着せ替えの仕組み\", category = \"dev_guide\", versions = [], hash = \"1a2b3c4d5e6f7081\", url = \"https://ssp.shillest.net/ukadoc/manual/dev_bind.html\" }",
        "\"ukadoc:dev_bind\" = \"ただの文字列\"",
    );
    let message = read_error(&body).to_string();
    assert!(message.contains("ukadoc:dev_bind"), "{message}");
}

/// 知らない欄があれば落ちる。形の版が違うカタログを黙って読み飛ばさない。
#[test]
fn unknown_entry_column_is_rejected() {
    let body = mutated(
        "hash = \"1a2b3c4d5e6f7081\"",
        "hash = \"1a2b3c4d5e6f7081\", memo = \"余計な欄\"",
    );
    let message = read_error(&body).to_string();
    assert!(message.contains("memo"), "{message}");
}

/// 冒頭の情報に知らない欄があれば落ちる。
#[test]
fn unknown_snapshot_field_is_rejected() {
    let body = mutated(
        "catalog_format = 1\n",
        "catalog_format = 1\nfuture_field = 9\n",
    );
    let message = read_error(&body).to_string();
    assert!(message.contains("future_field"), "{message}");
}

/// `versions` の要素が文字列でなければ落ちる。
#[test]
fn non_string_version_is_rejected() {
    let body = mutated("versions = [\"2.4.00\"]", "versions = [4]");
    let message = read_error(&body).to_string();
    assert!(message.contains("versions"), "{message}");
}

// ---- 3 つの読み出し ----

/// URL から項目 id を引く逆引き（設計 D-4 の 1 段目）。
#[test]
fn by_url_resolves_each_entry_exactly() {
    let catalog = sample();
    let index = catalog.by_url();
    assert_eq!(index.len(), 4);
    assert_eq!(
        index
            .get("https://ssp.shillest.net/ukadoc/manual/dev_bind.html")
            .map(|id| id.as_str()),
        Some("ukadoc:dev_bind")
    );
    assert_eq!(
        index
            .get("https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_5f_71:1")
            .map(|id| id.as_str()),
        Some("ukadoc:list_sakura_script:_5c_5f_71:1")
    );
    // フラグメントを外した綴りは 1 段目では引けない（完全一致だけ）。
    assert!(!index.contains_key("https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html"));
}

/// フラグメントを外したページ URL の一覧。同じページの複数の項目は 1 つに畳まれる。
#[test]
fn page_urls_strip_the_fragment_and_collapse_per_page() {
    let pages = sample().page_urls();
    let listed: Vec<(&str, &str)> = pages
        .iter()
        .map(|(url, page)| (url.as_str(), page.as_str()))
        .collect();
    assert_eq!(
        listed,
        vec![
            (
                "https://ssp.shillest.net/ukadoc/manual/dev_bind.html",
                "dev_bind"
            ),
            (
                "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html",
                "list_propertysystem"
            ),
            (
                "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html",
                "list_sakura_script"
            ),
        ],
        "4 項目・3 ページ。アンカー付き 2 件は 1 つの URL に畳まれる"
    );
}

/// 切るのは**最初の** `#`。後ろの `#` で切ると別の綴りに落ちる。
///
/// 実データの URL に `#` は 1 つしか無いので現物では見分けが付かないが、doc コメントが
/// 「最初の `#` より前を採る」と言う以上、その規則そのものを釘付けする。
#[test]
fn page_urls_cut_at_the_first_hash() {
    let mut catalog = sample();
    let key = EntryId::parse("ukadoc:dev_bind").expect("見本の id は読めるはず");
    match catalog.entries.get_mut(&key) {
        Some(entry) => entry.url = "https://example.test/a.html#one#two".to_owned(),
        None => panic!("見本に dev_bind が無い"),
    }

    let pages = catalog.page_urls();
    assert!(
        pages.contains_key("https://example.test/a.html"),
        "最初の # で切れていない: {:?}",
        pages.keys().collect::<Vec<_>>()
    );
    assert!(
        !pages.contains_key("https://example.test/a.html#one"),
        "最後の # で切っている: {:?}",
        pages.keys().collect::<Vec<_>>()
    );
}

/// ページごとの見出し一覧は id の昇順で返る。
#[test]
fn titles_of_page_lists_in_id_order() {
    let catalog = sample();
    let listed: Vec<(&str, &str)> = catalog
        .titles_of_page(&PageName::new("list_sakura_script"))
        .into_iter()
        .map(|(id, title)| (id.as_str(), title))
        .collect();
    assert_eq!(
        listed,
        vec![
            (
                "ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2cID_5d:1",
                r"\![get,property,ID]"
            ),
            (
                "ukadoc:list_sakura_script:_5c_5f_71:1",
                "選択肢の'既定'を消す"
            ),
        ]
    );
}

/// アンカーを持たないページも同じ形で引ける（要件 1.9）。
#[test]
fn titles_of_page_covers_a_page_without_anchors() {
    let catalog = sample();
    let listed: Vec<(&str, &str)> = catalog
        .titles_of_page(&PageName::new("dev_bind"))
        .into_iter()
        .map(|(id, title)| (id.as_str(), title))
        .collect();
    assert_eq!(listed, vec![("ukadoc:dev_bind", "着せ替えの仕組み")]);
}

/// 知らないページ名では空が返る（黙って全件を返したりしない）。
#[test]
fn titles_of_page_of_an_unknown_page_is_empty() {
    assert!(
        sample()
            .titles_of_page(&PageName::new("no_such_page"))
            .is_empty()
    );
}
