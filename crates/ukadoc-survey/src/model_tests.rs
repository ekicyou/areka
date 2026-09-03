//! `model.rs` の在中テスト。
//!
//! 守るのは 4 つ。⑴ 項目 id の 2 形（要件 1.9）が通り、ページ名の下線で割られないこと。
//! ⑵ 形が違う id が誤りとして返ること。⑶ 4 つの語彙（状態 7・関連 6・ドメイン 4・
//! テーマ 8）の綴りが要件どおりで、綴り違いが誤りとして返ること（要件 2.2・4.3・4.4）。
//! ⑷ 報告に出す平易な呼び名が要件 7.8 のとおりであること。
//!
//! スナップショットには一切触らない（要件 6.2）。すべて文字列だけで完結する。

use super::*;

// ---- 項目 id の 2 形（要件 1.9）----

#[test]
fn page_wide_id_parses() {
    let id = EntryId::parse("ukadoc:dev_bind").expect("ページ全体の形は通るはず");
    assert_eq!(id.as_str(), "ukadoc:dev_bind");
    assert_eq!(id.page().as_str(), "dev_bind");
    assert!(!id.has_anchor());
}

#[test]
fn anchored_id_parses() {
    let raw = "ukadoc:list_propertysystem:system.year:1";
    let id = EntryId::parse(raw).expect("アンカー付きの形は通るはず");
    assert_eq!(id.as_str(), raw);
    assert_eq!(id.page().as_str(), "list_propertysystem");
    assert!(id.has_anchor());
}

#[test]
fn anchored_id_with_encoded_anchor_parses() {
    // 実データのアンカーは日本語や記号が `_5c` のように符号化されて入る（設計 D-10）。
    let raw = "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.width:1";
    let id = EntryId::parse(raw).expect("符号化されたアンカーも通るはず");
    assert_eq!(id.page().as_str(), "list_propertysystem");
    assert!(id.has_anchor());
    assert_eq!(id.as_str(), raw);
}

#[test]
fn page_name_containing_underscore_is_not_split() {
    // 区切りはコロンだけ。下線で割ると "list" になってしまう（要件付録 B 手順 2）。
    let raw = "ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2cID_5d:1";
    let id = EntryId::parse(raw).expect("下線を含むページ名も通るはず");
    assert_eq!(id.page().as_str(), "list_sakura_script");
    assert!(id.has_anchor());

    let page_wide = EntryId::parse("ukadoc:descript_shell_surfacetable").expect("通るはず");
    assert_eq!(page_wide.page().as_str(), "descript_shell_surfacetable");
    assert!(!page_wide.has_anchor());
}

// ---- 形が違う id（要件 1.9 の裏）----

#[test]
fn malformed_ids_are_rejected() {
    let bad = [
        "",                                       // 空
        "ukadoc",                                 // 区切りが 0
        "ukadoc_dev_bind",                        // コロン以外で割った形
        "ukadoc:list_propertysystem:system.year", // 区切りが 2（連番が無い）
        "ukadoc:a:b:1:2",                         // 区切りが 4
        "ukadoc:",                                // ページ名が空
        "ukadoc::b:1",                            // ページ名が空（アンカー付き）
        "ukadoc:page::1",                         // アンカーが空
        "ukadoc:page:anchor:",                    // 連番が空
        "satori:dev_bind",                        // 接頭辞が ukadoc でない
        "UKADOC:dev_bind",                        // 接頭辞の大小が違う
        " ukadoc:dev_bind",                       // 前後の空白は直さない
    ];
    for raw in bad {
        let got = EntryId::parse(raw);
        assert!(got.is_err(), "{raw:?} は誤りとして返るはず");
        let message = format!("{}", got.expect_err("上で誤りを確かめた"));
        assert!(
            message.contains(raw),
            "誤りの本文に元の綴りが載るはず: {message}"
        );
    }
}

// ---- 並び（設計 D-9: id の byte 昇順）----

#[test]
fn entry_ids_order_by_bytes_of_the_id() {
    let mut ids = [
        EntryId::parse("ukadoc:list_propertysystem:system.year:1").expect("通るはず"),
        EntryId::parse("ukadoc:dev_bind").expect("通るはず"),
        EntryId::parse("ukadoc:descript_balloon:type:1").expect("通るはず"),
    ];
    ids.sort();
    let sorted: Vec<&str> = ids.iter().map(EntryId::as_str).collect();
    let mut expected = [
        "ukadoc:list_propertysystem:system.year:1",
        "ukadoc:dev_bind",
        "ukadoc:descript_balloon:type:1",
    ];
    expected.sort_unstable();
    assert_eq!(sorted, expected);
}

// ---- 状態の語彙（要件 2.2・7.8）----

#[test]
fn every_status_key_round_trips() {
    let all = [
        Status::Implemented,
        Status::VocabularyOnly,
        Status::Degraded,
        Status::Absent,
        Status::Alias,
        Status::NotApplicable,
        Status::Unclassified,
    ];
    for status in all {
        let key = status.as_key();
        assert_eq!(
            Status::parse(key).expect("台帳の綴りは通るはず"),
            status,
            "{key} が往復しない"
        );
    }
}

#[test]
fn status_keys_are_the_seven_spellings_of_requirement_2_2() {
    assert_eq!(Status::Implemented.as_key(), "implemented");
    assert_eq!(Status::VocabularyOnly.as_key(), "vocabulary-only");
    assert_eq!(Status::Degraded.as_key(), "degraded");
    assert_eq!(Status::Absent.as_key(), "absent");
    assert_eq!(Status::Alias.as_key(), "alias");
    assert_eq!(Status::NotApplicable.as_key(), "not-applicable");
    assert_eq!(Status::Unclassified.as_key(), "unclassified");
}

#[test]
fn status_japanese_names_are_those_of_requirement_7_8() {
    assert_eq!(Status::Implemented.as_japanese(), "実装済み");
    assert_eq!(Status::VocabularyOnly.as_japanese(), "語彙のみ");
    assert_eq!(Status::Degraded.as_japanese(), "縮退");
    assert_eq!(Status::Absent.as_japanese(), "未対応");
    assert_eq!(Status::Alias.as_japanese(), "別名");
    assert_eq!(Status::NotApplicable.as_japanese(), "対象外");
    assert_eq!(Status::Unclassified.as_japanese(), "未分類");
}

#[test]
fn status_outside_the_vocabulary_is_rejected() {
    // 1 文字違い・下線と横棒の取り違え・大文字・空・平易な日本語の呼び名。
    let bad = [
        "implemente",
        "implementedd",
        "vocabulary_only",
        "not_applicable",
        "Implemented",
        "",
        "実装済み",
    ];
    for raw in bad {
        let got = Status::parse(raw);
        assert!(got.is_err(), "{raw:?} は誤りとして返るはず");
        let err = got.expect_err("上で誤りを確かめた");
        assert_eq!(err.field, "status");
        assert_eq!(err.value, raw);
    }
}

#[test]
fn unknown_vocabulary_can_be_given_the_file_and_id() {
    // 語彙の失敗はこの層では綴りしか持てない。file と id は台帳を読む段が添える。
    let err = Status::parse("nope").expect_err("誤りとして返るはず");
    let survey = err.at(
        "doc/ukadoc-coverage/ledger/property.toml",
        "ukadoc:dev_bind",
    );
    let message = format!("{survey}");
    assert!(
        message.contains("doc/ukadoc-coverage/ledger/property.toml"),
        "{message}"
    );
    assert!(message.contains("ukadoc:dev_bind"), "{message}");
    assert!(message.contains("status"), "{message}");
    assert!(message.contains("nope"), "{message}");
}

// ---- 関連の種別（要件 4.3）----

#[test]
fn every_link_kind_key_round_trips() {
    let all = [
        LinkKind::AliasOf,
        LinkKind::Supersedes,
        LinkKind::Triggers,
        LinkKind::Configures,
        LinkKind::Queries,
        LinkKind::SameFeature,
    ];
    for kind in all {
        let key = kind.as_key();
        assert_eq!(
            LinkKind::parse(key).expect("台帳の綴りは通るはず"),
            kind,
            "{key} が往復しない"
        );
    }
}

#[test]
fn link_kind_keys_are_the_six_spellings_of_requirement_4_3() {
    assert_eq!(LinkKind::AliasOf.as_key(), "alias_of");
    assert_eq!(LinkKind::Supersedes.as_key(), "supersedes");
    assert_eq!(LinkKind::Triggers.as_key(), "triggers");
    assert_eq!(LinkKind::Configures.as_key(), "configures");
    assert_eq!(LinkKind::Queries.as_key(), "queries");
    assert_eq!(LinkKind::SameFeature.as_key(), "same-feature");
}

#[test]
fn link_kind_outside_the_vocabulary_is_rejected() {
    // `alias_of` は下線・`same-feature` は横棒。取り違えは誤りにする。
    for raw in ["alias-of", "same_feature", "trigger", "", "Queries"] {
        let got = LinkKind::parse(raw);
        assert!(got.is_err(), "{raw:?} は誤りとして返るはず");
        assert_eq!(got.expect_err("上で誤りを確かめた").field, "links.kind");
    }
}

#[test]
fn link_holds_a_kind_and_a_parsed_id() {
    let link = Link {
        kind: LinkKind::Queries,
        to: EntryId::parse("ukadoc:list_propertysystem:system.year:1").expect("通るはず"),
    };
    assert_eq!(link.kind, LinkKind::Queries);
    assert_eq!(link.to.page().as_str(), "list_propertysystem");
}

// ---- ドメイン（要件 3.1）----

#[test]
fn every_domain_key_round_trips() {
    assert_eq!(Domain::ALL.len(), 4);
    for domain in Domain::ALL {
        let key = domain.as_key();
        assert_eq!(
            Domain::parse(key).expect("ドメイン名は通るはず"),
            domain,
            "{key} が往復しない"
        );
    }
}

#[test]
fn domain_keys_are_the_four_ledger_file_names() {
    assert_eq!(Domain::Shiori.as_key(), "shiori");
    assert_eq!(Domain::Assets.as_key(), "assets");
    assert_eq!(Domain::SakuraScript.as_key(), "sakura-script");
    assert_eq!(Domain::Property.as_key(), "property");
    assert_eq!(
        Domain::ALL,
        [
            Domain::Shiori,
            Domain::Assets,
            Domain::SakuraScript,
            Domain::Property
        ]
    );
}

#[test]
fn domain_outside_the_vocabulary_is_rejected() {
    for raw in ["sakura_script", "sakura", "", "Shiori", "balloon"] {
        let got = Domain::parse(raw);
        assert!(got.is_err(), "{raw:?} は誤りとして返るはず");
        assert_eq!(got.expect_err("上で誤りを確かめた").field, "domain");
    }
}

// ---- テーマ（要件 4.4）----

#[test]
fn themes_are_the_eight_of_requirement_4_4_in_order() {
    assert_eq!(THEMES.len(), 8);
    assert_eq!(
        THEMES,
        [
            "気配",
            "触れ合い",
            "掛け合い",
            "装い",
            "記憶",
            "交わり",
            "気配り",
            "更新"
        ]
    );
}

#[test]
fn every_theme_round_trips() {
    for theme in THEMES {
        assert_eq!(parse_theme(theme).expect("テーマ名は通るはず"), theme);
    }
}

#[test]
fn theme_outside_the_vocabulary_is_rejected() {
    // 「気配」と「気配り」は片方が他方の接頭辞なので、部分一致で通してはいけない。
    for raw in ["気配 ", "きくばり", "", "装", "更新する"] {
        let got = parse_theme(raw);
        assert!(got.is_err(), "{raw:?} は誤りとして返るはず");
        assert_eq!(got.expect_err("上で誤りを確かめた").field, "values");
    }
    assert_eq!(parse_theme("気配").expect("通るはず"), "気配");
    assert_eq!(parse_theme("気配り").expect("通るはず"), "気配り");
}
