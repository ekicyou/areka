//! [`SurveyError`] の本文が「探した絶対パス」「読めない理由」「形が違う場所」を
//! 載せていることを逐語で確かめる（要件 1.8・6.10・6.12・設計 Error Handling）。

use super::SurveyError;

#[test]
fn snapshot_unreadable_carries_the_absolute_path_and_the_reason() {
    let err = SurveyError::SnapshotUnreadable {
        path: r"C:\Users\maz\AppData\Roaming\npm\node_modules\ukagaka-doc-mcp\data\index.json"
            .to_string(),
        reason: "指定されたパスが見つかりません。 (os error 3)".to_string(),
    };
    let body = err.to_string();
    assert!(
        body.contains(r"C:\Users\maz\AppData\Roaming\npm\node_modules\ukagaka-doc-mcp\data\index.json"),
        "探した絶対パスが本文に無い: {body}"
    );
    assert!(
        body.contains("指定されたパスが見つかりません。 (os error 3)"),
        "読めない理由が本文に無い: {body}"
    );
}

#[test]
fn io_and_toml_parse_carry_the_path_and_the_reason() {
    let io = SurveyError::Io {
        path: "doc/ukadoc-coverage/catalog.toml".to_string(),
        reason: "アクセスが拒否されました。".to_string(),
    };
    let io_body = io.to_string();
    assert!(io_body.contains("doc/ukadoc-coverage/catalog.toml"), "パスが無い: {io_body}");
    assert!(io_body.contains("アクセスが拒否されました。"), "理由が無い: {io_body}");

    let parse = SurveyError::TomlParse {
        path: "doc/ukadoc-coverage/ledger/property.toml".to_string(),
        reason: "expected `=`".to_string(),
    };
    let parse_body = parse.to_string();
    assert!(
        parse_body.contains("doc/ukadoc-coverage/ledger/property.toml"),
        "パスが無い: {parse_body}"
    );
    assert!(parse_body.contains("expected `=`"), "理由が無い: {parse_body}");
}

#[test]
fn missing_env_names_the_variable_it_looked_for() {
    let err = SurveyError::MissingEnv { name: "APPDATA" };
    let body = err.to_string();
    assert!(body.contains("APPDATA"), "環境変数の名前が本文に無い: {body}");
}

#[test]
fn bad_vocabulary_points_at_the_file_the_id_the_field_and_the_value() {
    let err = SurveyError::BadVocabulary {
        file: "doc/ukadoc-coverage/ledger/shiori.toml".to_string(),
        id: "ukadoc:list_shiori_event:OnBoot:1".to_string(),
        field: "status",
        value: "implmented".to_string(),
    };
    let body = err.to_string();
    for needle in [
        "doc/ukadoc-coverage/ledger/shiori.toml",
        "ukadoc:list_shiori_event:OnBoot:1",
        "status",
        "implmented",
    ] {
        assert!(body.contains(needle), "形が違う場所の手掛かり {needle} が本文に無い: {body}");
    }
}

#[test]
fn structural_failures_point_at_the_place_that_broke() {
    let out_of_order = SurveyError::LedgerOutOfOrder {
        file: "doc/ukadoc-coverage/ledger/property.toml".to_string(),
        id: "ukadoc:list_propertysystem:system.year:1".to_string(),
    };
    let body = out_of_order.to_string();
    assert!(
        body.contains("doc/ukadoc-coverage/ledger/property.toml"),
        "ファイルが本文に無い: {body}"
    );
    assert!(
        body.contains("ukadoc:list_propertysystem:system.year:1"),
        "順序を破る id が本文に無い: {body}"
    );

    let unassigned = SurveyError::PageNotAssigned {
        pages: "list_newpage, list_anotherpage".to_string(),
    };
    let unassigned_body = unassigned.to_string();
    assert!(unassigned_body.contains("list_newpage"), "ページ名が本文に無い: {unassigned_body}");

    let split = SurveyError::LedgerSplitMismatch {
        detail: "切り分けの鍵 676 件・toml の鍵 677 件".to_string(),
    };
    assert!(
        split.to_string().contains("切り分けの鍵 676 件・toml の鍵 677 件"),
        "食い違いの内訳が本文に無い: {split}"
    );

    let shape = SurveyError::SnapshotShape {
        detail: "entries が配列ではない".to_string(),
    };
    assert!(
        shape.to_string().contains("entries が配列ではない"),
        "形の食い違いが本文に無い: {shape}"
    );

    let bad_id = SurveyError::BadEntryId { raw: "ukadoc".to_string() };
    assert!(bad_id.to_string().contains("ukadoc"), "元の綴りが本文に無い: {bad_id}");
}
