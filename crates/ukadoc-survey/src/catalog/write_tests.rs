//! `write.rs` の在中テスト。
//!
//! 守るのは 5 つ。⑴ 設計「Data Models」の見本がバイト単位で再現されること
//! （冒頭の 2 行・`[snapshot]` の 8 欄とその並び・`[entry]` の 1 項目 1 行）。
//! ⑵ 同じカタログを 2 回書き出すと 1 バイトも違わないこと（要件 1.5）。
//! ⑶ 項目が id の byte 昇順で並び、1 項目がちょうど 1 行になること（要件 1.1・D-9）。
//! ⑷ 逃がしが設計 D-10 のとおり（逆斜線は `\\`・単引用符と日本語はそのまま）。
//! ⑸ 組み上げた本文が `toml` でも読めて値が一致すること（自前の書き出しの較正）。
//!
//! ⑸ を置くのは、⑴〜⑷ が自分の読み取りだけを相手にしていると「自分の読み手だけが
//! 許す本文」を書いていても気づけないからである。独立した読み手を 1 つ通す。
//!
//! **ファイルも一時ディレクトリも作らない。環境変数も読まない**（要件 6.2）。
//!
//! 逐語の期待値は実装の定数を参照せず、独立した文字列リテラルで書く（タスク 1.5 の
//! 教訓。定数を参照すると表を表自身と比べるだけになる）。

use std::collections::BTreeMap;

use super::*;
use crate::model::{EntryId, PageName};

/// 設計「Data Models」の `doc/ukadoc-coverage/catalog.toml` の見本そのもの。
///
/// 見出しとハッシュは見本の当て字（`...`・`0000000000000000`）なので、下の
/// [`design_sample_catalog`] もその綴りをそのまま持つ。
const DESIGN_SAMPLE: &str = r#"# 機械生成。手で編集しない。再生成: cargo run -p ukadoc-survey -- catalog
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
"ukadoc:dev_bind" = { page = "dev_bind", title = "...", category = "dev_guide", versions = [], hash = "0000000000000000", url = "https://ssp.shillest.net/ukadoc/manual/dev_bind.html" }
"ukadoc:list_propertysystem:system.year:1" = { page = "list_propertysystem", title = "system.year", category = "protocol", versions = [], hash = "0000000000000000", url = "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#system.year:1" }
"#;

/// 見本の冒頭の情報。全 2,983 件と正典 1,749 件は**別の欄**なので違う値を置く
/// （タスク 2.1 の教訓。一致する見本では 2 つの欄を区別できない）。
fn design_sample_meta() -> SnapshotMeta {
    SnapshotMeta {
        package: "ukagaka-doc-mcp".to_owned(),
        package_version: "0.2.7".to_owned(),
        snapshot_version: 1,
        generated_at: "2026-08-24T04:08:57.881Z".to_owned(),
        total_entries: 2983,
        ukadoc_entries: 1749,
        catalog_format: 1,
        hash_algorithm: "fnv1a64".to_owned(),
    }
}

/// 項目 1 件を組み立てる。ページ名は id から取る（設計 D-11）。
fn entry_of(
    id: &str,
    title: &str,
    category: &str,
    versions: &[&str],
    hash: &str,
    url: &str,
) -> CatalogEntry {
    let id = EntryId::parse(id).expect("見本の id は 2 形のはず");
    let page = id.page();
    CatalogEntry {
        id,
        page,
        title: title.to_owned(),
        category: category.to_owned(),
        versions: versions.iter().map(|v| (*v).to_string()).collect(),
        hash: hash.to_owned(),
        url: url.to_owned(),
    }
}

/// 項目の一覧をカタログに束ねる。渡す順は問わない（表が id 順に並べ替える）。
fn catalog_of(meta: SnapshotMeta, entries: Vec<CatalogEntry>) -> Catalog {
    let mut table: BTreeMap<EntryId, CatalogEntry> = BTreeMap::new();
    for entry in entries {
        let previous = table.insert(entry.id.clone(), entry);
        assert!(previous.is_none(), "見本に同じ id を 2 つ置いてはいけない");
    }
    Catalog {
        snapshot: meta,
        entries: table,
    }
}

/// 書き出す鍵は**表の鍵**であって項目の側の `id` ではない。
///
/// 鍵と中身が食い違ったカタログを渡すと、項目の側の `id` を使う書き出しは食い違いを
/// 黙って揉み消した本文を出す。現状 `read` が中身の `id` を鍵から作るのでこの食い違いは
/// 作れないが、`write.rs` の該当箇所はその防御を主張している。主張する以上は釘付けする。
#[test]
fn the_written_key_is_the_table_key_not_the_entry_id() {
    let mut mismatched = design_sample_catalog();
    let key = EntryId::parse("ukadoc:dev_bind").expect("見本の id は読めるはず");
    let other = EntryId::parse("ukadoc:dev_nar").expect("見本の id は読めるはず");
    match mismatched.entries.get_mut(&key) {
        Some(entry) => entry.id = other,
        None => panic!("見本に dev_bind が無い"),
    }

    let body = write(&mismatched);
    assert!(
        body.contains("\"ukadoc:dev_bind\" = {"),
        "表の鍵で書かれていない: {body}"
    );
    assert!(
        !body.contains("\"ukadoc:dev_nar\" = {"),
        "項目の側の id で鍵が書き換わっている: {body}"
    );
}

/// 設計の見本と同じ 2 項目のカタログ。
fn design_sample_catalog() -> Catalog {
    catalog_of(
        design_sample_meta(),
        vec![
            entry_of(
                "ukadoc:dev_bind",
                "...",
                "dev_guide",
                &[],
                "0000000000000000",
                "https://ssp.shillest.net/ukadoc/manual/dev_bind.html",
            ),
            entry_of(
                "ukadoc:list_propertysystem:system.year:1",
                "system.year",
                "protocol",
                &[],
                "0000000000000000",
                "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#system.year:1",
            ),
        ],
    )
}

/// 逃がしと並びを確かめるための 4 項目。渡す順はわざと id の順と違える。
fn rich_catalog() -> Catalog {
    catalog_of(
        design_sample_meta(),
        vec![
            entry_of(
                "ukadoc:list_sakura_script:_5c_5f_71:1",
                "選択肢の'既定'を消す",
                "sakurascript",
                &[],
                "fedcba9876543210",
                "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_5f_71:1",
            ),
            entry_of(
                "ukadoc:dev_bind",
                "着せ替えの仕組み",
                "dev_guide",
                &[],
                "1a2b3c4d5e6f7081",
                "https://ssp.shillest.net/ukadoc/manual/dev_bind.html",
            ),
            entry_of(
                "ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2cID_5d:1",
                r"\![get,property,ID]",
                "sakurascript",
                &["2.4.00"],
                "00000000000000ff",
                "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bget_2cproperty_2cID_5d:1",
            ),
            entry_of(
                "ukadoc:list_propertysystem:system.year:1",
                "system.year",
                "protocol",
                &["2.3.53", "2.5.60"],
                "0f1e2d3c4b5a6978",
                "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#system.year:1",
            ),
        ],
    )
}

/// 本文を行に割る（末尾の改行で空の行を作らない）。
fn lines(body: &str) -> Vec<&str> {
    body.strip_suffix('\n')
        .expect("本文は改行で終わるはず")
        .split('\n')
        .collect()
}

/// 設計「Data Models」の見本を 1 バイトも違わずに組み上げる。
///
/// 冒頭の 2 行・`[snapshot]` の 8 欄の並び・空行の入り方・`[entry]` の 1 行の形の
/// すべてがここに載っている。読み戻し一致では区切りの空白 1 個の違いも欄の並び替えも
/// 捕まらないので、この逐語の主張が要件 1.5 の土台になる（タスク 1.4 の教訓）。
#[test]
fn design_sample_body_is_reproduced_byte_for_byte() {
    assert_eq!(write(&design_sample_catalog()), DESIGN_SAMPLE);
}

/// 同じカタログを 2 回書き出しても 1 バイトも違わない（要件 1.5）。
#[test]
fn writing_twice_is_byte_identical() {
    let catalog = rich_catalog();
    let first = write(&catalog);
    let second = write(&catalog);
    assert_eq!(first, second);
    // 空の主張にならないよう、実際に項目が載っていることも確かめる。
    assert!(first.contains("[entry]\n\"ukadoc:dev_bind\" = {"));
}

/// 項目は id の byte 昇順に並び、渡した順には従わない（設計 D-9）。
#[test]
fn entries_are_written_in_id_byte_order() {
    let body = write(&rich_catalog());
    let keys: Vec<&str> = lines(&body)
        .into_iter()
        .filter(|line| line.starts_with('"'))
        .map(|line| line.split(" = ").next().unwrap_or(line))
        .collect();
    assert_eq!(
        keys,
        vec![
            "\"ukadoc:dev_bind\"",
            "\"ukadoc:list_propertysystem:system.year:1\"",
            "\"ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2cID_5d:1\"",
            "\"ukadoc:list_sakura_script:_5c_5f_71:1\"",
        ]
    );
}

/// 1 項目はちょうど 1 行（要件 1.1）。行数は冒頭 13 行＋項目数で決まる。
#[test]
fn each_entry_occupies_exactly_one_line() {
    let body = write(&rich_catalog());
    let all = lines(&body);
    // 注意書き 2・空行 1・`[snapshot]` 1・欄 8・空行 1・`[entry]` 1 ＝ 14 行。
    assert_eq!(all.len(), 14 + 4);
    assert_eq!(all[14 - 1], "[entry]");
    for line in &all[14..] {
        assert!(line.starts_with('"'), "項目の行のはず: {line}");
        assert!(
            line.ends_with(" }"),
            "1 行のインラインテーブルのはず: {line}"
        );
    }
}

/// 逃がしは設計 D-10 のとおり。逆斜線は `\\`、単引用符と日本語はそのまま。
#[test]
fn escaping_follows_design_d10() {
    let body = write(&rich_catalog());
    assert!(
        body.contains(r#"title = "\\![get,property,ID]""#),
        "逆斜線は 2 つ重ねる: {body}"
    );
    assert!(
        body.contains(r#"title = "選択肢の'既定'を消す""#),
        "単引用符と日本語は逃がさない: {body}"
    );
    assert!(
        body.contains(r#"versions = ["2.3.53", "2.5.60"]"#),
        "版番号は与えた順のまま並べる: {body}"
    );
    assert!(body.contains("versions = []"), "版番号が無ければ空: {body}");
}

/// 冒頭の情報の 8 欄が逐語で並ぶ。全件数と正典の件数は別の欄なので値を違えてある。
#[test]
fn snapshot_block_is_verbatim() {
    let body = write(&rich_catalog());
    let head: Vec<&str> = lines(&body).into_iter().take(13).collect();
    assert_eq!(
        head,
        vec![
            "# 機械生成。手で編集しない。再生成: cargo run -p ukadoc-survey -- catalog",
            "# 形式の正本: .kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/design.md",
            "",
            "[snapshot]",
            "package = \"ukagaka-doc-mcp\"",
            "package_version = \"0.2.7\"",
            "snapshot_version = 1",
            "generated_at = \"2026-08-24T04:08:57.881Z\"",
            "total_entries = 2983",
            "ukadoc_entries = 1749",
            "catalog_format = 1",
            "hash_algorithm = \"fnv1a64\"",
            "",
        ]
    );
}

/// 項目が 1 つも無くても表の見出しは書く（読み戻しが空の表として成り立つように）。
#[test]
fn empty_catalog_still_writes_both_table_headers() {
    let body = write(&catalog_of(design_sample_meta(), Vec::new()));
    assert!(body.ends_with("[entry]\n"), "{body}");
    assert!(body.contains("\n[snapshot]\n"), "{body}");
}

/// 組み上げた本文を **`toml` で** 読み戻して値が一致する（自前の書き出しの較正）。
///
/// 自分の読み取りだけを相手にしていると、自分の読み手だけが許す本文を書いていても
/// 気づけない。独立した読み手を 1 つ通して、逃がした文字列が元の値に戻ることまで見る。
#[test]
fn rendered_body_is_readable_by_the_toml_crate_with_matching_values() {
    let body = write(&rich_catalog());
    let root: toml::Table = match body.parse() {
        Ok(table) => table,
        Err(err) => panic!("組み上げた本文が toml で読めない: {err}\n{body}"),
    };

    let snapshot = root["snapshot"].as_table().expect("[snapshot] は表のはず");
    assert_eq!(snapshot["package"].as_str(), Some("ukagaka-doc-mcp"));
    assert_eq!(snapshot["snapshot_version"].as_integer(), Some(1));
    assert_eq!(snapshot["total_entries"].as_integer(), Some(2983));
    assert_eq!(snapshot["ukadoc_entries"].as_integer(), Some(1749));
    assert_eq!(snapshot["catalog_format"].as_integer(), Some(1));
    assert_eq!(snapshot["hash_algorithm"].as_str(), Some("fnv1a64"));

    let entry = root["entry"]
        .as_table()
        .expect("[entry] は表のはず")
        ["ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2cID_5d:1"]
        .as_table()
        .expect("項目は表のはず");
    assert_eq!(entry["page"].as_str(), Some("list_sakura_script"));
    // 逃がした逆斜線が 1 つに戻る。ここが落ちたら書き出しが過剰に逃がしている。
    assert_eq!(entry["title"].as_str(), Some(r"\![get,property,ID]"));
    assert_eq!(entry["category"].as_str(), Some("sakurascript"));
    assert_eq!(entry["hash"].as_str(), Some("00000000000000ff"));
    assert_eq!(
        entry["url"].as_str(),
        Some(
            "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bget_2cproperty_2cID_5d:1"
        )
    );
    let versions: Vec<&str> = entry["versions"]
        .as_array()
        .expect("versions は配列のはず")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect();
    assert_eq!(versions, vec!["2.4.00"]);
}

/// 表の鍵には表そのものの id が使われ、値の側の欄と食い違わない。
#[test]
fn table_key_and_page_column_agree_with_the_id() {
    let body = write(&rich_catalog());
    assert!(
        body.contains(
            "\"ukadoc:list_propertysystem:system.year:1\" = { page = \"list_propertysystem\", "
        ),
        "{body}"
    );
}

/// ページ名は id から取る型なので、書き出しにも id と同じページ名が出る。
#[test]
fn page_column_comes_from_the_id() {
    let entry = entry_of(
        "ukadoc:dev_bind",
        "着せ替えの仕組み",
        "dev_guide",
        &[],
        "1a2b3c4d5e6f7081",
        "https://ssp.shillest.net/ukadoc/manual/dev_bind.html",
    );
    assert_eq!(entry.page, PageName::new("dev_bind"));
}
