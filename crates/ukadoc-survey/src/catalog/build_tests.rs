//! `build.rs` の在中テスト。
//!
//! 守るのは 6 つ。⑴ 正典以外の出典が落ち、全件数と正典の件数が別々に記録されること
//! （要件 1.4・1.6）。⑵ 版番号の抽出規則が逐語どおりで、2 つ以上あっても 1 つに
//! 絞られないこと（要件 1.2）。⑶ 本文がカタログのどこにも残らず、印はハッシュだけで
//! あること（要件 1.3・9.4）。⑷ id の 2 形が同じ形で収まること（要件 1.9）。
//! ⑸ 割り当ての無いページがページ名付きで失敗すること（要件 3.5）。⑹ 冒頭の情報が
//! スナップショットから素通しで写ること（要件 1.6）。
//!
//! **スナップショットのファイルは 1 度も読まない。環境変数も読まないし書かない。
//! ファイルも一時ディレクトリも作らない**（要件 6.2・設計 File Structure Plan）。
//! 見本は [`SnapshotDoc`] を手で組み立てて作る。

use super::*;

/// 見本の生成日時。冒頭の情報が素通しであることを確かめるために逐語で持つ。
const GENERATED_AT: &str = "2026-08-24T04:08:57.881Z";

/// 見本の entry 1 件。出典と本文だけを変えられるようにしてある。
///
/// ページは割り当て表にある名前を使う。割り当ての無いページを試すテストだけが
/// 表に無い名前を渡す。
fn raw(id: &str, source: &str, content: &str) -> RawEntry {
    RawEntry {
        id: id.to_owned(),
        title: format!("{id} の見出し"),
        source: source.to_owned(),
        category: "dev_guide".to_owned(),
        content: content.to_owned(),
        url: format!("https://ssp.shillest.net/ukadoc/manual/{id}.html"),
    }
}

/// 正典由来の entry 1 件。
fn ukadoc(id: &str, content: &str) -> RawEntry {
    raw(id, "ukadoc", content)
}

/// 見本のスナップショット 1 つ分。
fn doc_of(entries: Vec<RawEntry>) -> SnapshotDoc {
    SnapshotDoc {
        version: 1,
        generated_at: GENERATED_AT.to_owned(),
        entries,
        package: "ukagaka-doc-mcp".to_owned(),
        package_version: "0.2.7".to_owned(),
    }
}

/// 見本から組み立てたカタログ。
fn built(entries: Vec<RawEntry>) -> Catalog {
    build(&doc_of(entries), &PageAssignment::canonical()).expect("見本は組み立てられるはず")
}

/// 見本を組み立てて失敗を受け取る。
fn build_error(entries: Vec<RawEntry>) -> SurveyError {
    match build(&doc_of(entries), &PageAssignment::canonical()) {
        Ok(_) => panic!("この見本は失敗するはず"),
        Err(err) => err,
    }
}

/// カタログに入っている id の並び。
fn ids(catalog: &Catalog) -> Vec<&str> {
    catalog.entries.keys().map(EntryId::as_str).collect()
}

/// 1 項目だけのカタログから、その項目を取り出す。
fn only(catalog: &Catalog) -> &CatalogEntry {
    assert_eq!(catalog.entries.len(), 1, "見本は 1 項目のはず");
    catalog
        .entries
        .values()
        .next()
        .expect("1 項目あるので取り出せる")
}

// ---- 出典によるふるい分け（要件 1.4・1.6）----

#[test]
fn entries_from_other_sources_are_dropped() {
    // 5 件のうち正典は 3 件。全件数と正典の件数は**違う値**になる。
    let catalog = built(vec![
        ukadoc("ukadoc:dev_bind", "本文"),
        raw("yaya:foo", "yaya", "本文"),
        ukadoc("ukadoc:dev_nar", "本文"),
        raw("satori:bar", "satori", "本文"),
        ukadoc("ukadoc:memo", "本文"),
    ]);

    assert_eq!(
        ids(&catalog),
        vec!["ukadoc:dev_bind", "ukadoc:dev_nar", "ukadoc:memo"],
        "正典由来の 3 件だけが残るはず"
    );
    assert_eq!(
        catalog.snapshot.total_entries, 5,
        "全件数は出典を問わず数えるはず"
    );
    assert_eq!(
        catalog.snapshot.ukadoc_entries, 3,
        "正典の件数は残した数のはず"
    );
    assert_ne!(
        catalog.snapshot.total_entries, catalog.snapshot.ukadoc_entries,
        "2 つの欄が同じ値だと取り違えを見つけられない"
    );
}

#[test]
fn a_well_formed_id_from_another_source_is_still_dropped() {
    // 出典違いの entry が正典と同じ形の id を持ち、ページも割り当て済みでも落とす。
    // ふるい分けを外しても id の形では気付けない見本なので、件数と id の並びだけが
    // 食い違いを告げる（要件 1.4・1.6）。
    let catalog = built(vec![
        ukadoc("ukadoc:dev_bind", "本文"),
        raw("ukadoc:dev_shell", "yaya", "本文"),
    ]);

    assert_eq!(ids(&catalog), vec!["ukadoc:dev_bind"], "出典で選ぶはず");
    assert_eq!(catalog.snapshot.total_entries, 2);
    assert_eq!(catalog.snapshot.ukadoc_entries, 1);
}

#[test]
fn other_sources_are_dropped_before_the_assignment_is_checked() {
    // 出典違いの entry は id の形も割り当ても問われない。ふるい分けが先に来る
    // ことの証拠になる（要件 1.4）。
    let catalog = built(vec![
        ukadoc("ukadoc:dev_bind", "本文"),
        raw("これは id の形をしていない", "yaya", "本文"),
        raw("ukadoc:no_such_page:x:1", "satori", "本文"),
    ]);

    assert_eq!(ids(&catalog), vec!["ukadoc:dev_bind"]);
    assert_eq!(catalog.snapshot.total_entries, 3);
    assert_eq!(catalog.snapshot.ukadoc_entries, 1);
}

// ---- 版番号の抽出（要件 1.2）----

#[test]
fn several_versions_are_all_kept() {
    // 課題の完了条件そのもの。2 つ以上あっても 1 つに絞らない。
    // 見本の本文は順が入れ替わっていて重複もある。
    let catalog = built(vec![ukadoc(
        "ukadoc:dev_bind",
        "2.8.80 で変更。2.3.53 以降で使える。2.8.80 も参照。2.4.09 で追加。",
    )]);

    assert_eq!(
        only(&catalog).versions,
        vec!["2.3.53", "2.4.09", "2.8.80"],
        "重複を除き昇順に並べ、1 つに絞らないはず"
    );
}

#[test]
fn versions_are_sorted_as_strings_not_as_numbers() {
    // 「文字列として昇順」（設計 版番号の抽出規則）。数として並べると
    // 2.9.0 が先に来るので、ここで規則の取り違えが露見する。
    let catalog = built(vec![ukadoc("ukadoc:dev_bind", "2.9.0 と 2.10.0 の話。")]);

    assert_eq!(only(&catalog).versions, vec!["2.10.0", "2.9.0"]);
}

#[test]
fn content_without_a_version_yields_an_empty_list() {
    let catalog = built(vec![ukadoc("ukadoc:dev_bind", "版番号を含まない本文。")]);

    assert!(only(&catalog).versions.is_empty(), "1 つも無ければ空のはず");
}

#[test]
fn a_version_is_taken_only_when_neither_side_is_a_digit_or_a_dot() {
    // 規則は「前後が数字でも小数点でもない `数字+.数字+.数字+`」（設計 版番号の
    // 抽出規則）。境界を見ない取り出し方も、語の境界で切る取り出し方も、ここで
    // 赤くなる。
    let cases: [(&str, Vec<&str>); 10] = [
        // 前後が空白なら拾う。
        ("SSP 2.3.53 以降", vec!["2.3.53"]),
        // 直前が英字でも拾う。実データの `SSP2.3.00以降` がこの形で、語の境界で
        // 切ると本文 4 件から版番号が落ちる（dev_nar・dev_update・OnMouseWheel・
        // OnMouseGesture）。
        ("SSP2.3.00以降の場合", vec!["2.3.00"]),
        // 4 つ目の欄が続くものは拾わない（直後が小数点）。
        ("1.2.3.4", vec![]),
        // 直前が小数点でも拾わない。
        (".1.2.3", vec![]),
        // 3 つ目の欄が何桁でも、それで 1 つの版番号になる。
        ("2.3.531", vec!["2.3.531"]),
        // 半角の点で終わる文は拾わない（直後が小数点になる）。
        ("2.3.53.", vec![]),
        // 欄が 2 つでは足りない。
        ("2.3 と 10.20", vec![]),
        // どの欄も 2 桁以上でよい。
        ("12.34.56", vec!["12.34.56"]),
        // 1 つ目の欄が 2 桁でも、それで 1 つの版番号になる。
        ("11.2.3", vec!["11.2.3"]),
        // 全角の数字は数字として扱わない。
        ("１.２.３", vec![]),
    ];

    for (content, expected) in cases {
        let catalog = built(vec![ukadoc("ukadoc:dev_bind", content)]);
        assert_eq!(
            only(&catalog).versions,
            expected,
            "本文 {content:?} からの版番号"
        );
    }
}

// ---- 本文は残さない（要件 1.3・9.4）----

#[test]
fn the_content_is_hashed_and_then_discarded() {
    let content = "この本文の綴りはカタログのどこにも残ってはならない。2.8.80。";
    let catalog = built(vec![ukadoc("ukadoc:dev_bind", content)]);
    let entry = only(&catalog);

    assert_eq!(
        entry.hash,
        crate::hash::content_hash(content),
        "印は本文のハッシュそのもののはず"
    );
    assert_eq!(entry.hash.len(), 16, "16 桁の 16 進小文字のはず");

    // 本文の綴りがどの欄にも現れない。欄を増やしたときにここが赤くなる。
    let mut columns: Vec<&str> = vec![
        entry.id.as_str(),
        entry.page.as_str(),
        &entry.title,
        &entry.category,
        &entry.hash,
        &entry.url,
    ];
    columns.extend(entry.versions.iter().map(String::as_str));
    for text in columns {
        assert!(
            !text.contains(content),
            "本文が {text:?} に残っている（要件 1.3・9.4）"
        );
    }
}

// ---- id の 2 形（要件 1.9）----

#[test]
fn both_id_forms_are_held_the_same_way() {
    let catalog = built(vec![
        ukadoc("ukadoc:list_propertysystem:system.year:1", "本文"),
        ukadoc("ukadoc:dev_bind", "本文"),
    ]);

    let page_wide = catalog
        .entries
        .get(&EntryId::parse("ukadoc:dev_bind").expect("形は正しい"))
        .expect("ページ全体の id も入るはず");
    assert_eq!(page_wide.page.as_str(), "dev_bind");
    assert!(!page_wide.id.has_anchor());

    let anchored = catalog
        .entries
        .get(&EntryId::parse("ukadoc:list_propertysystem:system.year:1").expect("形は正しい"))
        .expect("アンカー付きの id も入るはず");
    assert_eq!(anchored.page.as_str(), "list_propertysystem");
    assert!(anchored.id.has_anchor());

    // 2 形は同じ表に、id の byte 昇順で並ぶ（設計 D-9）。
    assert_eq!(
        ids(&catalog),
        vec![
            "ukadoc:dev_bind",
            "ukadoc:list_propertysystem:system.year:1"
        ]
    );
}

#[test]
fn a_malformed_id_fails_and_names_the_spelling() {
    let err = build_error(vec![ukadoc("ukadoc:dev_bind:anchor_only", "本文")]);

    match &err {
        SurveyError::BadEntryId { raw } => assert_eq!(raw, "ukadoc:dev_bind:anchor_only"),
        other => panic!("id の形の誤りとして返るはず: {other:?}"),
    }
    assert!(
        err.to_string().contains("ukadoc:dev_bind:anchor_only"),
        "本文に綴りが載るはず: {err}"
    );
}

#[test]
fn the_same_id_twice_fails_instead_of_being_folded() {
    // 表に畳むと 1 件が黙って消え、正典の件数だけが減る（要件 1.8 の「黙って
    // 失敗しない」）。
    let err = build_error(vec![
        ukadoc("ukadoc:dev_bind", "本文"),
        ukadoc("ukadoc:dev_bind", "別の本文"),
    ]);

    match &err {
        SurveyError::SnapshotShape { detail } => assert!(
            detail.contains("ukadoc:dev_bind"),
            "どの id かが載るはず: {detail}"
        ),
        other => panic!("形の違いとして返るはず: {other:?}"),
    }
}

// ---- 割り当ての無いページ（要件 3.5）----

#[test]
fn an_unassigned_page_fails_and_names_the_page() {
    let err = build_error(vec![
        ukadoc("ukadoc:dev_bind", "本文"),
        ukadoc("ukadoc:spec_unknown:foo:1", "本文"),
    ]);

    match &err {
        SurveyError::PageNotAssigned { pages } => {
            assert_eq!(
                pages, "spec_unknown",
                "割り当ての無いページ名だけが載るはず"
            );
        }
        other => panic!("割り当ての無いページとして返るはず: {other:?}"),
    }
    assert!(
        err.to_string().contains("spec_unknown"),
        "本文にページ名が載るはず: {err}"
    );
}

#[test]
fn every_unassigned_page_is_named_once_in_name_order() {
    let err = build_error(vec![
        ukadoc("ukadoc:zzz_page:a:1", "本文"),
        ukadoc("ukadoc:aaa_page:b:1", "本文"),
        ukadoc("ukadoc:zzz_page:c:1", "本文"),
        ukadoc("ukadoc:dev_bind", "本文"),
    ]);

    match &err {
        SurveyError::PageNotAssigned { pages } => {
            assert_eq!(pages, "aaa_page・zzz_page", "重複を除いて名前順に並ぶはず");
        }
        other => panic!("割り当ての無いページとして返るはず: {other:?}"),
    }
}

// ---- 冒頭のスナップショット情報（要件 1.6）----

#[test]
fn the_snapshot_information_is_carried_through_unchanged() {
    let catalog = built(vec![ukadoc("ukadoc:dev_bind", "本文")]);

    assert_eq!(
        catalog.snapshot,
        SnapshotMeta {
            package: "ukagaka-doc-mcp".to_owned(),
            package_version: "0.2.7".to_owned(),
            snapshot_version: 1,
            generated_at: GENERATED_AT.to_owned(),
            total_entries: 1,
            ukadoc_entries: 1,
            catalog_format: CATALOG_FORMAT,
            hash_algorithm: crate::hash::HASH_ALGORITHM.to_owned(),
        }
    );
    assert_eq!(CATALOG_FORMAT, 1, "形の版は 1 から始まる（設計 D-9）");
    assert_eq!(catalog.snapshot.hash_algorithm, "fnv1a64");
}

// ---- 列の内容（要件 1.2）----

#[test]
fn each_column_comes_from_the_matching_field() {
    let entries = vec![RawEntry {
        id: "ukadoc:list_propertysystem:system.year:1".to_owned(),
        title: "system.year".to_owned(),
        source: "ukadoc".to_owned(),
        category: "protocol".to_owned(),
        content: "現在の年。2.5.02 で追加。".to_owned(),
        url: "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#system.year:1"
            .to_owned(),
    }];
    let catalog = built(entries);
    let entry = only(&catalog);

    assert_eq!(
        entry.id.as_str(),
        "ukadoc:list_propertysystem:system.year:1"
    );
    assert_eq!(entry.page.as_str(), "list_propertysystem");
    assert_eq!(entry.title, "system.year");
    assert_eq!(entry.category, "protocol");
    assert_eq!(entry.versions, vec!["2.5.02"]);
    assert_eq!(
        entry.url,
        "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#system.year:1"
    );
}
