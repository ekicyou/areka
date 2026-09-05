//! `resolve.rs` の在中テスト。
//!
//! 守るのは 4 つ。⑴ URL の 3 段解決（設計 D-4）——完全一致で項目の証拠になること、
//! 同じ id が複数のファイルに現れたら重複を除いた名前順に並ぶこと、カタログに無い
//! URL は証拠に混ざらず別の一覧へ回ること。⑵ 語彙表の取り出し規則（設計 D-5・
//! Testing Strategy 5a）——3 形のいずれからも「要素ごとの最初の文字列リテラル」だけが
//! 拾われ、2 番目以降の文字列もスライスの外の文字列も拾われないこと。⑶ 名前の
//! 突き合わせ——正規化した完全一致で 1 件に定まるときだけ証拠にし、0 件も 2 件以上も
//! 別の一覧へ回すこと（赤にしない）。⑷ 入口の配線と決定性。
//!
//! **ファイルも一時ディレクトリも作らない。環境変数もスナップショットも読まない**
//! （要件 6.2）。カタログは手で書いた本文から組み立てる。

use super::*;
use crate::catalog::read::read;
use crate::model::EntryId;

/// `list_shiori_resource` のページ URL（フラグメント無し）。
const LIST_RESOURCE: &str = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html";
/// `list_sakura_script` のページ URL（フラグメント無し）。
const LIST_SAKURA: &str = "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html";
/// アンカーを持たない項目の URL。ページ URL と綴りが同じになる（要件 1.9 の 19 件の形）。
const DEV_BIND: &str = "https://ssp.shillest.net/ukadoc/manual/dev_bind.html";
/// アンカー付きの項目 URL 1 本。
const SAKURA_CHOICE: &str =
    "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_5f_71:1";

/// 手で書いた見本のカタログ。
///
/// 名前の突き合わせに要るものを 1 つずつ入れてある。⑴ 素直に一致する見出し
/// （`version`・`log_path`）⑵ 全角空白を含む見出し（実測でここだけが正規化を要する）
/// ⑶ 全角チルダを含む見出し ⑷ **同じページに 2 つある見出し**（`name`。実測 5 組の形）
/// ⑸ アンカー無しの項目（`ukadoc:dev_bind`）⑹ 逆斜線と引用符を含む見出し。
const CATALOG: &str = r##"# 見本
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
"ukadoc:list_sakura_script:_5c_21_5bget_5d:1" = { page = "list_sakura_script", title = "\\![get]", category = "sakurascript", versions = [], hash = "00000000000000ff", url = "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bget_5d:1" }
"ukadoc:list_sakura_script:_5c_5f_71:1" = { page = "list_sakura_script", title = "選択肢", category = "sakurascript", versions = [], hash = "fedcba9876543210", url = "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_5f_71:1" }
"ukadoc:list_sakura_script:quote:1" = { page = "list_sakura_script", title = "引用符\"入り", category = "sakurascript", versions = [], hash = "abcdefabcdefabcd", url = "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#quote:1" }
"ukadoc:list_shiori_resource:Reference1:1" = { page = "list_shiori_resource", title = "Reference1～", category = "protocol", versions = [], hash = "0000000000000001", url = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#Reference1:1" }
"ukadoc:list_shiori_resource:kigou:1" = { page = "list_shiori_resource", title = "(入力ボックス種類).defaultleft　(入力ボックス種類).defaulttop", category = "protocol", versions = [], hash = "0000000000000002", url = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#kigou:1" }
"ukadoc:list_shiori_resource:log_path:1" = { page = "list_shiori_resource", title = "log_path", category = "protocol", versions = [], hash = "0000000000000003", url = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#log_path:1" }
"ukadoc:list_shiori_resource:name:1" = { page = "list_shiori_resource", title = "name", category = "protocol", versions = [], hash = "0000000000000004", url = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#name:1" }
"ukadoc:list_shiori_resource:name:2" = { page = "list_shiori_resource", title = "name", category = "protocol", versions = [], hash = "0000000000000005", url = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#name:2" }
"ukadoc:list_shiori_resource:version:1" = { page = "list_shiori_resource", title = "version", category = "protocol", versions = [], hash = "0000000000000006", url = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#version:1" }
"##;

/// 見本のカタログを組み立てる。組み立てられなければテストを落とす。
fn catalog() -> Catalog {
    match read(CATALOG) {
        Ok(catalog) => catalog,
        Err(err) => panic!("見本のカタログは読めるはず: {err}"),
    }
}

/// 項目 id を作る。綴りが 2 形でなければテストを落とす。
fn id(raw: &str) -> EntryId {
    match EntryId::parse(raw) {
        Ok(id) => id,
        Err(err) => panic!("見本の id は読めるはず: {err}"),
    }
}

/// 証拠 1 件分の取り出し。
fn hit(path: &str, url: &str) -> UrlHit {
    UrlHit {
        path: path.to_owned(),
        url: url.to_owned(),
    }
}

/// ソースの本文の組。
fn sources(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

/// 索引から 1 項目の証拠を取り出す。無ければ空。
fn evidence_of(index: &EvidenceIndex, raw_id: &str) -> Vec<String> {
    index.by_id.get(&id(raw_id)).cloned().unwrap_or_default()
}

/// 見本の語彙表を 1 本だけ持つソース本文を組み立てる。
///
/// 目印の行は取り出しの段（[`super::extract`]）が拾う形そのままで書く。
fn vocab_source(page_url: &str, body: &str) -> String {
    format!("/// ukadoc: {page_url}\n{body}")
}

// ---------------------------------------------------------------------------
// ⑴ URL の 3 段解決（設計 D-4）
// ---------------------------------------------------------------------------

#[test]
fn the_same_id_in_two_files_collapses_to_a_deduplicated_name_ordered_list() {
    // 入力の並びは名前順と逆。同じファイルの重複も混ぜる。
    let hits = [
        hit("crates/zzz/src/lib.rs", SAKURA_CHOICE),
        hit("crates/aaa/src/lib.rs", SAKURA_CHOICE),
        hit("crates/zzz/src/lib.rs", SAKURA_CHOICE),
    ];
    let index = resolve(&hits, &sources(&[]), &catalog());

    assert_eq!(
        evidence_of(&index, "ukadoc:list_sakura_script:_5c_5f_71:1"),
        vec![
            "crates/aaa/src/lib.rs".to_owned(),
            "crates/zzz/src/lib.rs".to_owned()
        ],
        "重複を除いた名前順に並ぶ（設計 D-4）"
    );
    assert!(
        index.unresolved.is_empty(),
        "カタログにある URL は赤にしない"
    );
}

#[test]
fn a_url_outside_the_catalog_yields_no_evidence_and_one_unresolved() {
    let stray = "http://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_5f_71:1";
    let hits = [hit("crates/a/src/lib.rs", stray)];
    let index = resolve(&hits, &sources(&[]), &catalog());

    assert!(index.by_id.is_empty(), "証拠は 0 件");
    assert_eq!(
        index.unresolved,
        vec![UnresolvedUrl {
            path: "crates/a/src/lib.rs".to_owned(),
            url: stray.to_owned(),
        }],
        "綴りをそのまま別の一覧へ回す（設計 D-4 の 3 段目）"
    );
    assert!(index.unmatched_names.is_empty(), "語彙表の話ではない");
}

#[test]
fn a_url_that_merely_extends_a_catalog_url_is_not_resolved() {
    // 設計 D-4 の「末尾の余計な文字」。項目 URL とページ URL のどちらの段でも、
    // 前方一致に緩めると証拠になってしまう組を 1 つずつ置く。
    let longer_entry = format!("{SAKURA_CHOICE}0");
    let longer_page = format!("{LIST_RESOURCE}#unknown");
    let hits = [
        hit("crates/a/src/lib.rs", &longer_entry),
        hit("crates/a/src/lib.rs", &longer_page),
    ];
    let index = resolve(&hits, &sources(&[("crates/a/src/lib.rs", "")]), &catalog());

    assert!(index.by_id.is_empty(), "完全一致だけ（設計 D-4 の 1 段目）");
    assert!(
        index.unmatched_names.is_empty(),
        "語彙表の目印にもならない（設計 D-4 の 2 段目）"
    );
    assert_eq!(
        index.unresolved,
        vec![
            UnresolvedUrl {
                path: "crates/a/src/lib.rs".to_owned(),
                url: longer_entry,
            },
            UnresolvedUrl {
                path: "crates/a/src/lib.rs".to_owned(),
                url: longer_page,
            },
        ],
        "2 件とも 3 段目へ落ちる（並びは綴り順）"
    );
}

#[test]
fn an_anchorless_entry_url_wins_over_the_page_url() {
    // `ukadoc:dev_bind` の URL はページ URL と綴りが同じ。1 段目が勝つ（設計 D-4）。
    let hits = [hit("crates/a/src/bind.rs", DEV_BIND)];
    let index = resolve(&hits, &sources(&[("crates/a/src/bind.rs", "")]), &catalog());

    assert_eq!(
        evidence_of(&index, "ukadoc:dev_bind"),
        vec!["crates/a/src/bind.rs".to_owned()]
    );
    assert!(
        index.unmatched_names.is_empty(),
        "語彙表の目印としては扱わない"
    );
}

// ---------------------------------------------------------------------------
// ⑵ 語彙表の取り出し規則（設計 D-5・Testing Strategy 5a）
// ---------------------------------------------------------------------------

#[test]
fn a_slice_of_plain_strings_yields_its_element_names() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"version\",\n    \"log_path\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/ids.rs".to_owned()]
    );
    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:log_path:1"),
        vec!["crates/v/src/ids.rs".to_owned()]
    );
    assert!(index.unmatched_names.is_empty(), "2 件とも 1 件に定まる");
}

#[test]
fn a_slice_of_tuples_yields_only_the_first_string_of_each_element() {
    // 2 番目の文字列（`log_path`）は拾われてはならない。
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const SET: &[(&str, Kind)] = &[\n    (\"version\", Kind::A),\n    (\"log_path\", Kind::B),\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/set.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/set.rs", &text)]),
        &catalog(),
    );

    assert_eq!(index.by_id.len(), 2, "2 要素それぞれの最初の文字列だけ");
    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/set.rs".to_owned()]
    );
    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:log_path:1"),
        vec!["crates/v/src/set.rs".to_owned()]
    );
}

#[test]
fn a_slice_of_struct_literals_yields_only_the_first_string_of_each_element() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const FLAT: &[Entry] = &[\n    Entry { token: \"version\", note: \"log_path\" },\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/flat.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/flat.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/flat.rs".to_owned()],
        "要素の最初の文字列は拾う"
    );
    assert!(
        evidence_of(&index, "ukadoc:list_shiori_resource:log_path:1").is_empty(),
        "2 番目の文字列は拾わない（Testing Strategy 5a）"
    );
    assert_eq!(index.by_id.len(), 1);
}

#[test]
fn strings_outside_the_slice_are_not_taken() {
    let text = vocab_source(
        LIST_RESOURCE,
        "const BEFORE: &str = \"name\";\npub const IDS: &[&str] = &[\n    \"version\",\n];\nconst AFTER: &str = \"log_path\";\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/ids.rs".to_owned()],
        "表の中は拾う（非空の主張）"
    );
    assert!(
        evidence_of(&index, "ukadoc:list_shiori_resource:log_path:1").is_empty(),
        "`];` の後の文字列は拾わない（Testing Strategy 5a）"
    );
    assert!(
        index.unmatched_names.is_empty(),
        "表の前の `name` も拾われない（拾えば 2 件以上で一覧へ回るはず）"
    );
    assert_eq!(index.by_id.len(), 1);
}

#[test]
fn elements_split_only_on_commas_at_the_slices_own_depth() {
    // 入れ子の括弧・波括弧・角括弧の中のコンマで割ってはならない。割ると
    // `Kind::B` 側の断片が新しい要素になり、`log_path` が拾われてしまう。
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const SET: &[Entry] = &[\n    Entry { token: \"version\", tags: [\"a\", \"b\"], kind: f(1, \"log_path\") },\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/set.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/set.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/set.rs".to_owned()]
    );
    assert_eq!(index.by_id.len(), 1, "要素は 1 つだけ");
    assert!(index.unmatched_names.is_empty());
}

#[test]
fn commas_and_brackets_inside_a_string_do_not_split() {
    // 見出し `\![get]` は角括弧を含む。文字列の中を構造として読むと表がそこで
    // 終わったことになり、以降の要素が消える。
    let text = vocab_source(
        LIST_SAKURA,
        "pub const TAGS: &[&str] = &[\n    \"\\\\![get]\",\n    \"選択肢\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/tags.rs", LIST_SAKURA)],
        &sources(&[("crates/v/src/tags.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_sakura_script:_5c_21_5bget_5d:1"),
        vec!["crates/v/src/tags.rs".to_owned()],
        "逃がした逆斜線は 1 つに戻して突き合わせる"
    );
    assert_eq!(
        evidence_of(&index, "ukadoc:list_sakura_script:_5c_5f_71:1"),
        vec!["crates/v/src/tags.rs".to_owned()],
        "角括弧を含む文字列の後の要素も読める"
    );
}

#[test]
fn an_escaped_quote_does_not_end_the_string() {
    let text = vocab_source(
        LIST_SAKURA,
        "pub const TAGS: &[&str] = &[\n    \"引用符\\\"入り\",\n    \"選択肢\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/tags.rs", LIST_SAKURA)],
        &sources(&[("crates/v/src/tags.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_sakura_script:quote:1"),
        vec!["crates/v/src/tags.rs".to_owned()]
    );
    assert_eq!(
        evidence_of(&index, "ukadoc:list_sakura_script:_5c_5f_71:1"),
        vec!["crates/v/src/tags.rs".to_owned()],
        "続く要素も読める"
    );
}

#[test]
fn a_slice_before_the_marker_is_not_read() {
    let text = format!(
        "pub const BEFORE: &[&str] = &[\n    \"log_path\",\n];\n/// ukadoc: {LIST_RESOURCE}\npub const AFTER: &[&str] = &[\n    \"version\",\n];\n"
    );
    let index = resolve(
        &[hit("crates/v/src/two.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/two.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/two.rs".to_owned()],
        "目印の直後のスライスを見る"
    );
    assert!(
        evidence_of(&index, "ukadoc:list_shiori_resource:log_path:1").is_empty(),
        "目印より前のスライスは見ない"
    );
}

#[test]
fn only_the_first_slice_after_the_marker_is_read() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const FIRST: &[&str] = &[\n    \"version\",\n];\npub const SECOND: &[&str] = &[\n    \"log_path\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/two.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/two.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/two.rs".to_owned()],
        "最初のスライスは見る（非空の主張）"
    );
    assert!(
        evidence_of(&index, "ukadoc:list_shiori_resource:log_path:1").is_empty(),
        "「直後に始まる最初のスライス定数」だけ（設計 D-5）"
    );
}

#[test]
fn two_markers_in_one_file_each_get_their_own_table() {
    let text = format!(
        "/// ukadoc: {LIST_RESOURCE}\npub const A: &[&str] = &[\n    \"version\",\n];\n/// ukadoc: {LIST_SAKURA}\npub const B: &[&str] = &[\n    \"選択肢\",\n];\n"
    );
    let index = resolve(
        &[
            hit("crates/v/src/two.rs", LIST_RESOURCE),
            hit("crates/v/src/two.rs", LIST_SAKURA),
        ],
        &sources(&[("crates/v/src/two.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/two.rs".to_owned()]
    );
    assert_eq!(
        evidence_of(&index, "ukadoc:list_sakura_script:_5c_5f_71:1"),
        vec!["crates/v/src/two.rs".to_owned()]
    );
}

#[test]
fn comments_inside_the_table_are_not_read_as_structure() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    // 群 [SET] の 3 件, ここに \" がある\n    \"version\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/ids.rs".to_owned()]
    );
    assert!(index.unmatched_names.is_empty());
}

// ---------------------------------------------------------------------------
// ⑶ 名前の突き合わせ（設計 D-5）
// ---------------------------------------------------------------------------

#[test]
fn a_half_width_space_matches_a_full_width_space_after_normalization() {
    // 設計 D-5 の「素のままでは 1 件だけ食い違い（全角空白と半角空白の差）」。実データの
    // `SHIORI_RESOURCE_IDS` 159 要素のうち、正規化を要するのはこの 1 件だけである。
    // 効いているのは**空白の畳み込み**の側で（全角空白は空白として畳まれる）、全角形の
    // 写しの側ではない。写しの側は下の全角チルダのテストが受け持つ。
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"(入力ボックス種類).defaultleft (入力ボックス種類).defaulttop\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:kigou:1"),
        vec!["crates/v/src/ids.rs".to_owned()],
        "全角空白と半角空白の差は正規化で吸収する"
    );
    assert!(index.unmatched_names.is_empty());
}

#[test]
fn leading_trailing_and_repeated_spaces_are_folded_away() {
    // 設計 D-5 の「連続空白の畳み込み ＋ 前後の空白落とし」。見出しの側は空白 1 個
    // （全角）で、要素の側は前後と途中に余分な空白を持つ。
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"  (入力ボックス種類).defaultleft   (入力ボックス種類).defaulttop  \",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:kigou:1"),
        vec!["crates/v/src/ids.rs".to_owned()]
    );
    assert!(index.unmatched_names.is_empty());
}

#[test]
fn a_half_width_tilde_matches_a_full_width_tilde_after_normalization() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"Reference1~\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:Reference1:1"),
        vec!["crates/v/src/ids.rs".to_owned()],
        "全角チルダは NFKC で半角チルダになる（コーパス実測 2 文字のうちの 1 つ）"
    );
}

#[test]
fn normalization_does_not_fold_letter_case() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"Version\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert!(index.by_id.is_empty(), "`Version` は `version` と別の名前");
    assert_eq!(
        index.unmatched_names,
        vec![UnmatchedName {
            path: "crates/v/src/ids.rs".to_owned(),
            page_url: LIST_RESOURCE.to_owned(),
            reason: NameMatchFailure::NoMatch("Version".to_owned()),
        }]
    );
}

#[test]
fn a_name_with_no_matching_title_goes_to_unmatched_names() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"version\",\n    \"存在しない名前\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert_eq!(
        evidence_of(&index, "ukadoc:list_shiori_resource:version:1"),
        vec!["crates/v/src/ids.rs".to_owned()],
        "一致した方は証拠になる（非空の主張）"
    );
    assert_eq!(
        index.unmatched_names,
        vec![UnmatchedName {
            path: "crates/v/src/ids.rs".to_owned(),
            page_url: LIST_RESOURCE.to_owned(),
            reason: NameMatchFailure::NoMatch("存在しない名前".to_owned()),
        }],
        "0 件は赤にせず一覧へ回す（設計 D-5 規則 3）"
    );
    assert!(index.unresolved.is_empty(), "URL の話ではない");
}

#[test]
fn a_name_matching_two_titles_is_not_evidence() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"name\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert!(
        index.by_id.is_empty(),
        "1 件に定まらないものは証拠にしない（設計 D-5 規則 2）"
    );
    assert_eq!(
        index.unmatched_names,
        vec![UnmatchedName {
            path: "crates/v/src/ids.rs".to_owned(),
            page_url: LIST_RESOURCE.to_owned(),
            reason: NameMatchFailure::Ambiguous("name".to_owned()),
        }]
    );
}

#[test]
fn names_are_never_matched_by_substring() {
    // `versio` は `version` の先頭部分。部分一致を使うと証拠になってしまう。
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"versio\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert!(index.by_id.is_empty(), "完全一致だけ（設計 D-5 規則 1）");
    assert_eq!(index.unmatched_names.len(), 1);
}

#[test]
fn name_matching_looks_only_at_the_titles_of_that_page() {
    // `選択肢` は `list_sakura_script` の見出し。`list_shiori_resource` の目印から
    // 引き当ててはならない。
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"選択肢\",\n];\n",
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert!(index.by_id.is_empty(), "他のページの見出しは引かない");
    assert_eq!(
        index.unmatched_names,
        vec![UnmatchedName {
            path: "crates/v/src/ids.rs".to_owned(),
            page_url: LIST_RESOURCE.to_owned(),
            reason: NameMatchFailure::NoMatch("選択肢".to_owned()),
        }]
    );
}

#[test]
fn a_marker_with_no_slice_after_it_is_table_missing() {
    let text = vocab_source(LIST_RESOURCE, "pub fn f() -> u32 {\n    1\n}\n");
    let index = resolve(
        &[hit("crates/v/src/none.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/none.rs", &text)]),
        &catalog(),
    );

    assert!(index.by_id.is_empty());
    assert!(
        index.unresolved.is_empty(),
        "`SourceUrlNotInCatalog` にはしない（設計 D-5）"
    );
    assert_eq!(
        index.unmatched_names,
        vec![UnmatchedName {
            path: "crates/v/src/none.rs".to_owned(),
            page_url: LIST_RESOURCE.to_owned(),
            reason: NameMatchFailure::TableMissing,
        }]
    );
}

#[test]
fn an_unclosed_slice_is_table_missing() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"version\",\n",
    );
    let index = resolve(
        &[hit("crates/v/src/open.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/open.rs", &text)]),
        &catalog(),
    );

    assert!(index.by_id.is_empty(), "読み切れない表から証拠は採らない");
    assert_eq!(
        index.unmatched_names,
        vec![UnmatchedName {
            path: "crates/v/src/open.rs".to_owned(),
            page_url: LIST_RESOURCE.to_owned(),
            reason: NameMatchFailure::TableMissing,
        }]
    );
}

#[test]
fn a_marker_whose_text_was_not_supplied_is_table_missing() {
    let index = resolve(
        &[hit("crates/v/src/missing.rs", LIST_RESOURCE)],
        &sources(&[]),
        &catalog(),
    );

    assert!(index.by_id.is_empty());
    assert_eq!(
        index.unmatched_names,
        vec![UnmatchedName {
            path: "crates/v/src/missing.rs".to_owned(),
            page_url: LIST_RESOURCE.to_owned(),
            reason: NameMatchFailure::TableMissing,
        }]
    );
}

// ---------------------------------------------------------------------------
// ⑷ 入口の配線と決定性
// ---------------------------------------------------------------------------

#[test]
fn the_marker_line_is_recognized_only_in_the_extract_stages_shape() {
    // 説明文が続く行は取り出しの段が拾わない（要件 5.3）。同じ規則をここでも使う
    // ので、この行はスライスの起点にならない。
    let text = format!(
        "// ukadoc: {LIST_RESOURCE} 参照\npub const IDS: &[&str] = &[\n    \"version\",\n];\n"
    );
    let index = resolve(
        &[hit("crates/v/src/ids.rs", LIST_RESOURCE)],
        &sources(&[("crates/v/src/ids.rs", &text)]),
        &catalog(),
    );

    assert!(
        index.by_id.is_empty(),
        "目印の行が見つからなければ表は読まない"
    );
    assert_eq!(
        index.unmatched_names,
        vec![UnmatchedName {
            path: "crates/v/src/ids.rs".to_owned(),
            page_url: LIST_RESOURCE.to_owned(),
            reason: NameMatchFailure::TableMissing,
        }]
    );
}

#[test]
fn the_index_does_not_depend_on_the_order_of_the_hits() {
    let text = vocab_source(
        LIST_RESOURCE,
        "pub const IDS: &[&str] = &[\n    \"version\",\n    \"存在しない名前\",\n];\n",
    );
    let sources = sources(&[("crates/v/src/ids.rs", &text)]);
    let stray = "https://example.invalid/x";
    let forward = [
        hit("crates/v/src/ids.rs", LIST_RESOURCE),
        hit("crates/b/src/lib.rs", SAKURA_CHOICE),
        hit("crates/a/src/lib.rs", stray),
    ];
    let mut backward = forward.clone();
    backward.reverse();

    assert_eq!(
        resolve(&forward, &sources, &catalog()),
        resolve(&backward, &sources, &catalog()),
        "並べ替えても同じ索引になる"
    );
    let index = resolve(&forward, &sources, &catalog());
    assert_eq!(index.by_id.len(), 2, "見本は空ではない");
    assert_eq!(index.unresolved.len(), 1);
    assert_eq!(index.unmatched_names.len(), 1);
}

#[test]
fn no_hits_yields_an_empty_index() {
    let index = resolve(
        &[],
        &sources(&[("crates/a/src/lib.rs", "fn f() {}")]),
        &catalog(),
    );
    assert_eq!(index, EvidenceIndex::default());
}
