//! `io/snapshot.rs` の在中テスト。
//!
//! 守るのは 4 つ。⑴ 既定の場所の組み立てが逐語で正しいこと（設計 D-7）。⑵ JSON を
//! `SnapshotDoc` へ写す規則が逐語で正しいこと——鍵の綴り（`generatedAt`）と欄の値を
//! 実際の文字列で固定する（設計 D-2・付録 B 手順 1）。⑶ 形が違うとき・読めないときに
//! 黙って通らず、どこがどう違うかと探した絶対パスが本文に載ること（要件 1.8）。
//! ⑷ 提供パッケージの版の読み取り（要件 1.6）が package.json の欄から来ること。
//!
//! **ここではスナップショットを 1 度も読まない。環境変数も 1 つも読まないし書かない**
//! （要件 6.2・設計 Testing Strategy 19）。場所の組み立ても JSON の写しも
//! 「値を引数で受け取る純粋な関数」に切り出してあるので、文字列だけで確かめられる。
//! ファイルも一時ディレクトリも作らない（設計 File Structure Plan）。

use std::path::Path;

use super::*;

/// 失敗経路のテストが本文に載っていることを確かめる、探した絶対パスの見本。
const SHOWN: &str =
    r"C:\Users\someone\AppData\Roaming\npm\node_modules\ukagaka-doc-mcp\data\index.json";

/// 形の整った最小のスナップショット。1 項目だけを持つ。
const MINIMAL: &str = r#"{
  "version": 1,
  "generatedAt": "2026-08-24T04:08:57.881Z",
  "entries": [
    {
      "id": "ukadoc:list_propertysystem",
      "title": "プロパティシステム",
      "source": "ukadoc",
      "category": "リファレンス",
      "content": "本文。2.8.83 で追加。",
      "url": "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html"
    }
  ]
}"#;

/// 見本を 1 か所だけ差し替える（形の違いを 1 つだけ作るため）。
fn minimal_with(from: &str, to: &str) -> String {
    assert!(MINIMAL.contains(from), "見本に {from} が無い");
    MINIMAL.replacen(from, to, 1)
}

fn parse_minimal(text: &str) -> Result<SnapshotDoc, SurveyError> {
    parse(text, SHOWN, "ukagaka-doc-mcp", "0.2.7")
}

// ---- 既定の場所の組み立て（設計 D-7）----

#[test]
fn default_path_from_appdata_spells_the_whole_tail() {
    let got = default_path_from_appdata(Path::new(r"C:\Users\someone\AppData\Roaming"));
    assert_eq!(
        got.display().to_string(),
        r"C:\Users\someone\AppData\Roaming\npm\node_modules\ukagaka-doc-mcp\data\index.json"
    );
}

#[test]
fn default_path_from_appdata_keeps_the_given_root() {
    let got = default_path_from_appdata(Path::new(r"D:\roaming"));
    assert_eq!(
        got.display().to_string(),
        r"D:\roaming\npm\node_modules\ukagaka-doc-mcp\data\index.json"
    );
}

// ---- 探した絶対パスの組み立て（要件 1.8）----

#[test]
fn absolutize_leaves_an_absolute_path_alone() {
    let got = absolutize(Path::new(SHOWN), Some(Path::new(r"C:\work")));
    assert_eq!(got, SHOWN);
}

#[test]
fn absolutize_joins_a_relative_path_onto_the_working_directory() {
    let got = absolutize(Path::new("index.json"), Some(Path::new(r"C:\work")));
    assert_eq!(got, r"C:\work\index.json");
}

/// 作業ディレクトリが取れないときでも、綴りを隠さずそのまま載せる。
#[test]
fn absolutize_falls_back_to_the_given_spelling() {
    let got = absolutize(Path::new("index.json"), None);
    assert_eq!(got, "index.json");
}

/// `load` の入口が `absolutize` に繋がっていること（要件 1.8）。
///
/// `absolutize` 単体は上の 2 本で固定してあるが、`load` がそれを呼ばずに渡された綴りを
/// そのまま載せても、単体テストは全部緑のままになる。実在しない相対パスを渡し、本文に
/// 出るのが絶対パスであることをここで確かめる。読むファイルは無く（存在しないので）、
/// スナップショットにも環境変数にも触れない。
#[test]
fn load_reports_an_absolute_path_even_when_given_a_relative_one() {
    let relative = Path::new("no-such-snapshot-for-ukadoc-survey-tests.json");
    let here = std::env::current_dir().expect("作業ディレクトリが取れない");
    let expected = absolutize(relative, Some(&here));
    assert!(
        Path::new(&expected).is_absolute(),
        "見込みの綴りが絶対パスでない: {expected}"
    );

    let body = load(relative)
        .expect_err("実在しないパスなのに成功した")
        .to_string();
    assert!(
        body.contains(&expected),
        "本文に絶対パスが載っていない: {body}"
    );
}

// ---- 形の整った JSON の写し（設計 D-2・付録 B 手順 1）----

#[test]
fn parse_reads_every_field_of_a_well_formed_document() {
    let doc = parse_minimal(MINIMAL).expect("形の整った見本は読めるはず");
    assert_eq!(doc.version, 1);
    assert_eq!(doc.generated_at, "2026-08-24T04:08:57.881Z");
    assert_eq!(doc.package, "ukagaka-doc-mcp");
    assert_eq!(doc.package_version, "0.2.7");
    assert_eq!(doc.entries.len(), 1);
    let entry = &doc.entries[0];
    assert_eq!(entry.id, "ukadoc:list_propertysystem");
    assert_eq!(entry.title, "プロパティシステム");
    assert_eq!(entry.source, "ukadoc");
    assert_eq!(entry.category, "リファレンス");
    assert_eq!(entry.content, "本文。2.8.83 で追加。");
    assert_eq!(
        entry.url,
        "https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html"
    );
}

/// 生成日時の鍵は JSON では `generatedAt`。綴りを取り違えると読めない。
#[test]
fn parse_maps_generated_at_from_the_camel_case_key() {
    let text = minimal_with("\"generatedAt\"", "\"generated_at\"");
    let err = parse_minimal(&text).expect_err("鍵の綴りが違えば失敗するはず");
    let body = err.to_string();
    assert!(
        matches!(err, SurveyError::SnapshotShape { .. }),
        "形の違いとして返るはず: {body}"
    );
    assert!(
        body.contains("generatedAt"),
        "どの鍵が無いかが載るはず: {body}"
    );
}

/// 順序も件数も写しのまま。`source` が `ukadoc` 以外の entry もここでは落とさない
/// （落とす／落とさないの判断はカタログ側の仕事＝要件 1.4）。
#[test]
fn parse_carries_every_entry_through_in_order() {
    let text = r#"{
  "version": 2,
  "generatedAt": "2026-01-02T03:04:05.000Z",
  "entries": [
    {"id": "ukadoc:a", "title": "あ", "source": "ukadoc",
     "category": "c1", "content": "b1", "url": "https://ssp.shillest.net/ukadoc/manual/a.html"},
    {"id": "yaya:b", "title": "い", "source": "yaya_wiki",
     "category": "c2", "content": "b2", "url": "https://example.invalid/b"},
    {"id": "ukadoc:c", "title": "う", "source": "ukadoc",
     "category": "c3", "content": "b3", "url": "https://ssp.shillest.net/ukadoc/manual/c.html"}
  ]
}"#;
    let doc = parse(text, SHOWN, "ukagaka-doc-mcp", "0.2.7").expect("読めるはず");
    assert_eq!(doc.version, 2);
    let ids: Vec<&str> = doc.entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["ukadoc:a", "yaya:b", "ukadoc:c"]);
    let sources: Vec<&str> = doc.entries.iter().map(|e| e.source.as_str()).collect();
    assert_eq!(sources, vec!["ukadoc", "yaya_wiki", "ukadoc"]);
    assert_eq!(doc.entries[1].url, "https://example.invalid/b");
}

#[test]
fn parse_accepts_an_empty_entry_list() {
    let text = r#"{"version": 1, "generatedAt": "t", "entries": []}"#;
    let doc = parse(text, SHOWN, "p", "v").expect("読めるはず");
    assert!(doc.entries.is_empty());
    assert_eq!(doc.generated_at, "t");
}

// ---- 形が違うとき（要件 1.8）----

#[test]
fn parse_rejects_a_missing_top_level_version() {
    let text = r#"{"generatedAt": "t", "entries": []}"#;
    let err = parse(text, SHOWN, "p", "v").expect_err("version が無ければ失敗するはず");
    let body = err.to_string();
    assert!(matches!(err, SurveyError::SnapshotShape { .. }), "{body}");
    assert!(body.contains("version"), "どの鍵が無いかが載るはず: {body}");
}

#[test]
fn parse_rejects_a_version_that_is_not_a_number() {
    let text = minimal_with("\"version\": 1", "\"version\": \"1\"");
    let err = parse_minimal(&text).expect_err("version が文字列なら失敗するはず");
    let body = err.to_string();
    assert!(matches!(err, SurveyError::SnapshotShape { .. }), "{body}");
    assert!(body.contains("version"), "どの鍵かが載るはず: {body}");
    assert!(body.contains("整数"), "何が違うかが載るはず: {body}");
}

#[test]
fn parse_rejects_a_missing_entries_key() {
    let text = r#"{"version": 1, "generatedAt": "t"}"#;
    let err = parse(text, SHOWN, "p", "v").expect_err("entries が無ければ失敗するはず");
    let body = err.to_string();
    assert!(matches!(err, SurveyError::SnapshotShape { .. }), "{body}");
    assert!(body.contains("entries"), "どの鍵が無いかが載るはず: {body}");
}

#[test]
fn parse_rejects_entries_that_are_not_a_list() {
    let text = r#"{"version": 1, "generatedAt": "t", "entries": {}}"#;
    let err = parse(text, SHOWN, "p", "v").expect_err("entries が表なら失敗するはず");
    let body = err.to_string();
    assert!(matches!(err, SurveyError::SnapshotShape { .. }), "{body}");
    assert!(body.contains("entries"), "どの鍵かが載るはず: {body}");
    assert!(body.contains("配列"), "何が違うかが載るはず: {body}");
}

#[test]
fn parse_rejects_a_top_level_that_is_not_a_table() {
    let err = parse("[]", SHOWN, "p", "v").expect_err("最上位が配列なら失敗するはず");
    let body = err.to_string();
    assert!(matches!(err, SurveyError::SnapshotShape { .. }), "{body}");
}

#[test]
fn parse_rejects_an_entry_missing_a_field() {
    let text = minimal_with(
        "\"url\": \"https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html\"",
        "\"link\": \"https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html\"",
    );
    let err = parse_minimal(&text).expect_err("url が無ければ失敗するはず");
    let body = err.to_string();
    assert!(matches!(err, SurveyError::SnapshotShape { .. }), "{body}");
    assert!(body.contains("entries[0]"), "何番目かが載るはず: {body}");
    assert!(body.contains("url"), "どの欄かが載るはず: {body}");
}

#[test]
fn parse_names_the_offending_entry_by_its_position() {
    let text = r#"{
  "version": 1, "generatedAt": "t",
  "entries": [
    {"id": "a", "title": "t", "source": "ukadoc", "category": "c", "content": "b", "url": "u"},
    {"id": "b", "title": "t", "source": "ukadoc", "category": "c", "content": "b"}
  ]
}"#;
    let err = parse(text, SHOWN, "p", "v").expect_err("2 件目に url が無いので失敗するはず");
    let body = err.to_string();
    assert!(body.contains("entries[1]"), "何番目かが載るはず: {body}");
    assert!(body.contains("url"), "どの欄かが載るはず: {body}");
}

#[test]
fn parse_rejects_an_entry_field_of_the_wrong_type() {
    let text = minimal_with("\"title\": \"プロパティシステム\"", "\"title\": 12");
    let err = parse_minimal(&text).expect_err("title が数なら失敗するはず");
    let body = err.to_string();
    assert!(matches!(err, SurveyError::SnapshotShape { .. }), "{body}");
    assert!(body.contains("entries[0]"), "何番目かが載るはず: {body}");
    assert!(body.contains("title"), "どの欄かが載るはず: {body}");
    assert!(body.contains("文字列"), "何が違うかが載るはず: {body}");
}

#[test]
fn parse_rejects_an_entry_that_is_not_a_table() {
    let text = r#"{"version": 1, "generatedAt": "t", "entries": ["a"]}"#;
    let err = parse(text, SHOWN, "p", "v").expect_err("entry が文字列なら失敗するはず");
    let body = err.to_string();
    assert!(matches!(err, SurveyError::SnapshotShape { .. }), "{body}");
    assert!(body.contains("entries[0]"), "何番目かが載るはず: {body}");
}

// ---- 読めないとき（要件 1.8）----

#[test]
fn parse_reports_broken_json_with_the_absolute_path() {
    let err = parse("{\"version\": 1,", SHOWN, "p", "v").expect_err("壊れた JSON は失敗するはず");
    let body = err.to_string();
    assert!(
        matches!(err, SurveyError::SnapshotUnreadable { .. }),
        "読めない側として返るはず: {body}"
    );
    assert!(body.contains(SHOWN), "探した絶対パスが載るはず: {body}");
    assert!(body.contains("JSON"), "理由が載るはず: {body}");
}

#[test]
fn unreadable_carries_both_the_path_and_the_reason() {
    let err = unreadable(SHOWN, "指定されたファイルが見つかりません。 (os error 2)");
    let body = err.to_string();
    assert!(body.contains(SHOWN), "探した絶対パスが載るはず: {body}");
    assert!(body.contains("os error 2"), "理由が載るはず: {body}");
}

// ---- 提供パッケージの版（要件 1.6・設計 D-7）----

#[test]
fn package_json_path_is_two_directories_above_the_snapshot() {
    let snapshot = Path::new(
        r"C:\Users\someone\AppData\Roaming\npm\node_modules\ukagaka-doc-mcp\data\index.json",
    );
    let got = package_json_path(snapshot).expect("2 つ上が取れるはず");
    assert_eq!(
        got.display().to_string(),
        r"C:\Users\someone\AppData\Roaming\npm\node_modules\ukagaka-doc-mcp\package.json"
    );
}

#[test]
fn package_json_path_is_none_when_there_is_no_grandparent() {
    assert!(package_json_path(Path::new("index.json")).is_none());
}

#[test]
fn package_fields_reads_the_name_and_the_version() {
    let text = r#"{"name": "ukagaka-doc-mcp", "version": "0.2.7", "bin": {}}"#;
    let got = package_fields(text).expect("両方あるので読めるはず");
    assert_eq!(got, ("ukagaka-doc-mcp".to_owned(), "0.2.7".to_owned()));
}

#[test]
fn package_fields_falls_back_to_unknown_for_a_missing_name() {
    let text = r#"{"version": "0.2.7"}"#;
    let got = package_fields(text).expect("版があるので読めるはず");
    assert_eq!(got, (UNKNOWN.to_owned(), "0.2.7".to_owned()));
}

#[test]
fn package_fields_gives_up_without_a_version() {
    assert!(package_fields(r#"{"name": "ukagaka-doc-mcp"}"#).is_none());
}

#[test]
fn package_fields_gives_up_on_a_version_that_is_not_a_string() {
    assert!(package_fields(r#"{"name": "p", "version": 2}"#).is_none());
}

#[test]
fn package_fields_gives_up_on_broken_json() {
    assert!(package_fields("{\"name\":").is_none());
}

#[test]
fn unknown_is_spelled_exactly_once() {
    assert_eq!(UNKNOWN, "unknown");
}
