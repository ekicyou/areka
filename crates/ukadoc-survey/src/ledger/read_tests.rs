//! `read.rs` の在中テスト。
//!
//! 見本の本文はここに直に書く（実装側の定数を参照しない）。参照すると表を表自身と
//! 比べるだけになり、綴りの取り違えを 1 件も捕まえられない（タスク 1.5 の教訓）。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない
//! （要件 6.2・設計 File Structure Plan）。

use super::read;
use crate::error::SurveyError;
use crate::ledger::Ledger;
use crate::model::{Domain, EntryId, Link, LinkKind, PageName, Status};

/// 要件付録 A.1 の形をした見本の台帳。
///
/// 付録 A.1 と同じく `system.year` を先に、`balloon.scope(ID).width` を後に書く。
/// id の byte 昇順は `balloon…` が先なので、**本文の順と id の順は食い違う**。
/// 並び順の検査（設計 D-12）はこの食い違いが見えることに載っている。
///
/// 1 つ目の項目は付録 A.2 の欄をすべて（任意の 2 欄も含めて）埋め、備考は複数行、
/// 関連の相手には逆斜線を含む id を置いてある（付録 A.3 の逆斜線 2 つ重ね）。
const SAMPLE: &str = r#"# doc/ukadoc-coverage/ledger/property.toml
# 人手で記入・機械で検査する台帳。形式の正本は
# .kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md 付録 A。

[ledger]
domain = "property"
pages = ["list_propertysystem"]

[entry."ukadoc:list_propertysystem:system.year:1"]
status = "implemented"
introduced = "2.3.53"
owner = "areka-P0-property-catalog-lists"
priority = "C1"
supersedes = ["ukadoc:list_propertysystem:system.year.old:1"]
values = ["気配", "更新"]
links = [
  { kind = "queries", to = "ukadoc:list_sakura_script:\\![get,property,ID]:1" },
  { kind = "same-feature", to = "ukadoc:list_propertysystem:system.month:1" },
]
note = """
壊れ方: 値を返せないと辞書が空文字を前提に進み、黙って壊れる。
areka では sylphya の `system.*` が NotFound 縮退。
"""

[entry."ukadoc:list_propertysystem:balloon.scope(ID).width:1"]
status = "alias"
alias_of = "ukadoc:list_propertysystem:currentghost.balloon.scope(ID).width:1"
introduced = ""
owner = ""
priority = ""
values = []
links = []
note = "旧名。本文注記により currentghost.* 側が正典。"
"#;

/// 前置きだけで項目が 1 つも無い台帳。
const EMPTY: &str = r#"[ledger]
domain = "property"
pages = ["list_propertysystem"]
"#;

/// 備考の複数行文字列の中に**行頭**の見出しらしき行がある台帳。
///
/// `toml` はこれを備考の一部として読むが、行だけを見る切り分けはここで塊を割る。
/// 食い違いは `blocks::split` の較正が捕まえる（設計 D-12）。
const NOTE_WITH_HEAD: &str = r#"[ledger]
domain = "property"
pages = ["list_propertysystem"]

[entry."ukadoc:list_propertysystem:system.year:1"]
status = "unclassified"
introduced = ""
owner = ""
priority = ""
values = []
links = []
note = """
[entry."ukadoc:list_propertysystem:ghost:1"]
"""
"#;

/// 見本の 1 つ目の項目 id（本文では先に現れる）。
const YEAR: &str = "ukadoc:list_propertysystem:system.year:1";

/// 見本の 2 つ目の項目 id（byte 昇順では先に来る）。
const BALLOON: &str = "ukadoc:list_propertysystem:balloon.scope(ID).width:1";

/// 失敗の本文に載るはずのファイル名。
const FILE: &str = "doc/ukadoc-coverage/ledger/property.toml";

/// 見本の 1 か所を差し替える。
///
/// 差し替えが起きなかったら（見本を直して綴りがずれたら）その場で落とす。差し替わら
/// ないまま緑になると、摂動を当てていないのに当てたつもりになる。
fn tweak(from: &str, to: &str) -> String {
    assert!(SAMPLE.contains(from), "見本に {from} が無い");
    SAMPLE.replacen(from, to, 1)
}

/// 見本から 1 行まるごと落とす。
fn without_line(line: &str) -> String {
    let needle = format!("{line}\n");
    assert!(SAMPLE.contains(&needle), "見本に行 {line} が無い");
    SAMPLE.replacen(&needle, "", 1)
}

/// 見本の台帳を読む（読めるはず）。
fn sample() -> Ledger {
    read(SAMPLE, Domain::Property).expect("見本は読めるはず")
}

/// 読めないはずの本文を読んで、失敗を取り出す。
fn err_of(text: &str) -> SurveyError {
    match read(text, Domain::Property) {
        Ok(_) => panic!("読めてはいけない本文が読めた"),
        Err(err) => err,
    }
}

/// 語彙の失敗の中身を取り出す。別の失敗なら落とす。
fn vocabulary_of(err: SurveyError) -> (String, String, &'static str, String) {
    match err {
        SurveyError::BadVocabulary {
            file,
            id,
            field,
            value,
        } => (file, id, field, value),
        other => panic!("語彙の失敗ではない: {other}"),
    }
}

/// 失敗の本文に語がすべて載っていることを確かめる。
fn assert_mentions(err: &SurveyError, words: &[&str]) {
    let text = err.to_string();
    for word in words {
        assert!(text.contains(word), "失敗の本文に {word} が無い: {text}");
    }
}

/// 項目 id を作る（見本の綴りは 2 形のいずれかであるはず）。
fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は 2 形のいずれかのはず")
}

// ---- 欄を型のある値として読む（要件 2.1・付録 A.2） ----

/// 付録 A.2 の欄がすべて、それぞれの型の値として読まれる。
///
/// 件数ではなく逐語で釘付けする（タスク 1.5 の教訓）。
#[test]
fn reads_every_column_of_the_appendix_sample() {
    let ledger = sample();

    assert_eq!(ledger.domain, Domain::Property);
    assert_eq!(ledger.pages, vec![PageName::new("list_propertysystem")]);
    assert_eq!(ledger.entries.len(), 2, "見本の項目は 2 つ");

    let year = ledger.entries.get(&id(YEAR)).expect("1 つ目の項目が無い");
    assert_eq!(year.id, id(YEAR));
    assert_eq!(year.status, Status::Implemented);
    assert_eq!(year.introduced, "2.3.53");
    assert_eq!(year.alias_of, None, "別名でない行に写像先は無い");
    assert_eq!(
        year.supersedes,
        vec![id("ukadoc:list_propertysystem:system.year.old:1")]
    );
    assert_eq!(year.owner, "areka-P0-property-catalog-lists");
    assert_eq!(year.priority, "C1");
    assert_eq!(year.values, vec!["気配".to_owned(), "更新".to_owned()]);
    assert_eq!(
        year.links,
        vec![
            Link {
                kind: LinkKind::Queries,
                // 台帳には逆斜線 2 つ重ねで書かれている（付録 A.3）。読み終えた値は
                // 逆斜線 1 つでなければならない。
                to: id("ukadoc:list_sakura_script:\\![get,property,ID]:1"),
            },
            Link {
                kind: LinkKind::SameFeature,
                to: id("ukadoc:list_propertysystem:system.month:1"),
            },
        ]
    );
    assert_eq!(
        year.note,
        "壊れ方: 値を返せないと辞書が空文字を前提に進み、黙って壊れる。\n\
         areka では sylphya の `system.*` が NotFound 縮退。\n",
        "複数行の備考は改行ごと持つ"
    );

    let balloon = ledger
        .entries
        .get(&id(BALLOON))
        .expect("2 つ目の項目が無い");
    assert_eq!(balloon.status, Status::Alias);
    assert_eq!(
        balloon.alias_of,
        Some(id(
            "ukadoc:list_propertysystem:currentghost.balloon.scope(ID).width:1"
        )),
        "別名の行は写像先を持つ（要件 2.4）"
    );
    assert_eq!(balloon.introduced, "");
    assert_eq!(balloon.owner, "");
    assert_eq!(balloon.priority, "");
    assert!(balloon.supersedes.is_empty(), "書かれていない任意の欄は空");
    assert!(balloon.values.is_empty());
    assert!(balloon.links.is_empty());
    assert_eq!(
        balloon.note,
        "旧名。本文注記により currentghost.* 側が正典。"
    );
}

/// 本文に現れた順と id の順は別に持つ（設計 D-12）。
///
/// 見本は付録 A.1 と同じ並びで、本文の順（`system.year` が先）と id の byte 昇順
/// （`balloon…` が先）が食い違う。両方を逐語で釘付けし、食い違い自体も主張する——
/// `file_order` が表の鍵の写しになっていたら、この 3 つ目で赤になる。
#[test]
fn file_order_is_the_text_order_and_entries_are_id_order() {
    let ledger = sample();

    assert_eq!(
        ledger.file_order,
        vec![id(YEAR), id(BALLOON)],
        "本文に現れた順"
    );
    let by_id: Vec<EntryId> = ledger.entries.keys().cloned().collect();
    assert_eq!(by_id, vec![id(BALLOON), id(YEAR)], "id の byte 昇順");
    assert_ne!(
        ledger.file_order, by_id,
        "見本は本文の順と id の順が食い違っているはず（食い違わない見本では並び順の検査が何も言えない）"
    );
}

/// 項目が 1 つも無い台帳も読める。前置きだけが残る。
#[test]
fn an_empty_ledger_reads_to_no_entries() {
    let ledger = read(EMPTY, Domain::Property).expect("前置きだけの台帳は読めるはず");

    assert_eq!(ledger.domain, Domain::Property);
    assert_eq!(ledger.pages, vec![PageName::new("list_propertysystem")]);
    assert!(ledger.entries.is_empty());
    assert!(ledger.file_order.is_empty());
}

/// 読み取りは塊への切り分けを必ず通る（タスク 1.7 の教訓＝入口の配線は別に守る）。
///
/// 備考の中の行頭見出しは `toml` の鍵集合と食い違うので、切り分けの較正が落とす。
/// `toml` だけで読んで本文の順を鍵から作る実装は、ここで緑になってしまう。
#[test]
fn read_goes_through_the_block_split() {
    let err = err_of(NOTE_WITH_HEAD);

    match err {
        SurveyError::LedgerSplitMismatch { detail } => {
            assert!(
                detail.contains("ukadoc:list_propertysystem:ghost:1"),
                "食い違った id が本文に無い: {detail}"
            );
        }
        other => panic!("切り分けの食い違いではない: {other}"),
    }
}

// ---- 語彙（要件 2.2・4.3・4.4・6.10） ----

/// 状態の綴りを 1 文字変えると読み取りが失敗し、**どの id のどの欄か**が本文に載る
/// （タスク 2.4 の完了条件・要件 2.2・6.10）。
#[test]
fn a_one_character_status_typo_names_the_id_and_the_field() {
    let text = tweak(r#"status = "implemented""#, r#"status = "implementad""#);

    let err = err_of(&text);
    assert_mentions(&err, &[YEAR, "status"]);
    let (file, entry, field, value) = vocabulary_of(err);
    assert_eq!(file, FILE);
    assert_eq!(entry, YEAR);
    assert_eq!(field, "status");
    assert_eq!(value, "implementad");
}

/// テーマの綴りを 1 文字変えると失敗し、id と欄が本文に載る（要件 4.4・6.10）。
#[test]
fn a_theme_typo_names_the_id_and_the_field() {
    let text = tweak(
        r#"values = ["気配", "更新"]"#,
        r#"values = ["気配", "更心"]"#,
    );

    let err = err_of(&text);
    assert_mentions(&err, &[YEAR, "values"]);
    let (file, entry, field, value) = vocabulary_of(err);
    assert_eq!(file, FILE);
    assert_eq!(entry, YEAR);
    assert_eq!(field, "values");
    assert_eq!(value, "更心");
}

/// 関連の種別が 6 種に無ければ失敗し、id と欄が本文に載る（要件 4.3・6.10）。
#[test]
fn a_link_kind_typo_names_the_id_and_the_field() {
    let text = tweak(r#"kind = "same-feature""#, r#"kind = "same_feature""#);

    let err = err_of(&text);
    assert_mentions(&err, &[YEAR, "links.kind"]);
    let (file, entry, field, value) = vocabulary_of(err);
    assert_eq!(file, FILE);
    assert_eq!(entry, YEAR);
    assert_eq!(field, "links.kind");
    assert_eq!(value, "same_feature");
}

/// 前置きのドメイン名が 4 つに無ければ失敗し、場所と欄が本文に載る。
#[test]
fn a_domain_typo_names_the_prologue_and_the_field() {
    let text = tweak(r#"domain = "property""#, r#"domain = "propertx""#);

    let err = err_of(&text);
    assert_mentions(&err, &["[ledger]", "domain"]);
    let (file, entry, field, value) = vocabulary_of(err);
    assert_eq!(file, FILE);
    assert_eq!(entry, "[ledger]", "前置きには項目 id が無いので場所を書く");
    assert_eq!(field, "domain");
    assert_eq!(value, "propertx");
}

/// 前置きのドメインが読んでいるファイルのドメインと食い違えば失敗し、**両方**が
/// 本文に載る。
#[test]
fn the_declared_domain_must_match_the_file() {
    let err = match read(SAMPLE, Domain::Shiori) {
        Ok(_) => panic!("別ドメインの台帳として読めてはいけない"),
        Err(err) => err,
    };

    assert_mentions(&err, &["property", "shiori"]);
}

// ---- 別名の写像（要件 2.4・付録 A.2） ----

/// 別名でない行に写像先を書いたら失敗する（付録 A.2「それ以外は書かない」）。
#[test]
fn alias_of_outside_an_alias_row_is_rejected() {
    let text = tweak(
        r#"status = "implemented""#,
        "status = \"implemented\"\nalias_of = \"ukadoc:list_propertysystem:system.month:1\"",
    );

    let err = err_of(&text);
    assert_mentions(&err, &[YEAR, "alias_of"]);
}

/// 別名の行に写像先が無ければ失敗する（付録 A.2「`status = \"alias\"` のとき必須」）。
#[test]
fn an_alias_row_without_alias_of_is_rejected() {
    let text = without_line(
        r#"alias_of = "ukadoc:list_propertysystem:currentghost.balloon.scope(ID).width:1""#,
    );

    let err = err_of(&text);
    assert_mentions(&err, &[BALLOON, "alias_of"]);
}

// ---- 台帳に「無い」欄（要件 2.3・6.9） ----

/// 証拠の欄を書いたら失敗する（要件 2.3。台帳に証拠の欄は無い）。
///
/// 黙って読み飛ばすと、手書きの台帳が「証拠を書いたつもり」のまま検査を素通りする。
#[test]
fn an_evidence_column_is_rejected() {
    let text = tweak(
        r#"status = "implemented""#,
        "status = \"implemented\"\nevidence = [\"crates/areka-sylphya/src/lib.rs\"]",
    );

    let err = err_of(&text);
    assert_mentions(&err, &[YEAR, "evidence"]);
}

/// 未分類件数を宣言する欄を書いたら失敗する（要件 6.9。件数は報告側の分布が正）。
#[test]
fn an_unclassified_count_field_is_rejected() {
    let text = tweak(
        r#"pages = ["list_propertysystem"]"#,
        "pages = [\"list_propertysystem\"]\nunclassified_count = 42",
    );

    let err = err_of(&text);
    assert_mentions(&err, &["[ledger]", "unclassified_count"]);
}

/// 知らない欄は落とす（形の版が違う台帳を黙って読み飛ばさない）。
#[test]
fn an_unknown_entry_column_is_rejected() {
    let text = tweak(
        r#"priority = "C1""#,
        "priority = \"C1\"\nconfidence = \"high\"",
    );

    let err = err_of(&text);
    assert_mentions(&err, &[YEAR, "confidence"]);
}

/// 関連の要素に知らない欄があれば落とす。
#[test]
fn an_unknown_link_field_is_rejected() {
    let text = tweak(
        r#"{ kind = "same-feature", to = "ukadoc:list_propertysystem:system.month:1" }"#,
        r#"{ kind = "same-feature", to = "ukadoc:list_propertysystem:system.month:1", why = "同じ面" }"#,
    );

    let err = err_of(&text);
    assert_mentions(&err, &[YEAR, "why"]);
}

// ---- 必須の欄（付録 A.2） ----

/// 付録 A.2 が必須とする 7 欄は、1 つでも欠けたら失敗し、id と欄名が本文に載る。
#[test]
fn every_required_column_is_required() {
    // 2 つ目の項目（別名の行）は全欄が 1 行なので、行ごと落とせる。
    let required = [
        (r#"status = "alias""#, "status"),
        (r#"introduced = """#, "introduced"),
        (r#"owner = """#, "owner"),
        (r#"priority = """#, "priority"),
        ("values = []", "values"),
        ("links = []", "links"),
        (
            r#"note = "旧名。本文注記により currentghost.* 側が正典。""#,
            "note",
        ),
    ];
    assert_eq!(required.len(), 7, "付録 A.2 の必須欄は 7 つ");

    for (line, key) in required {
        let text = without_line(line);
        let err = err_of(&text);
        assert_mentions(&err, &[BALLOON, key]);
    }
}

/// 任意の 2 欄（`alias_of`・`supersedes`）は書かれていなくてもよい。
#[test]
fn the_two_optional_columns_may_be_absent() {
    let text = without_line(r#"supersedes = ["ukadoc:list_propertysystem:system.year.old:1"]"#);

    let ledger = read(&text, Domain::Property).expect("任意の欄は無くても読めるはず");
    let year = ledger.entries.get(&id(YEAR)).expect("1 つ目の項目が無い");
    assert!(year.supersedes.is_empty());
    assert!(year.alias_of.is_none());
}

/// 関連の要素に種別か相手 id が無ければ失敗する。
#[test]
fn a_link_without_kind_or_to_is_rejected() {
    let missing_to = tweak(
        r#"{ kind = "same-feature", to = "ukadoc:list_propertysystem:system.month:1" }"#,
        r#"{ kind = "same-feature" }"#,
    );
    assert_mentions(&err_of(&missing_to), &[YEAR, "to"]);

    let missing_kind = tweak(
        r#"{ kind = "same-feature", to = "ukadoc:list_propertysystem:system.month:1" }"#,
        r#"{ to = "ukadoc:list_propertysystem:system.month:1" }"#,
    );
    assert_mentions(&err_of(&missing_kind), &[YEAR, "kind"]);
}

// ---- 項目 id の形（要件 1.9） ----

/// 表の鍵が 2 形のどちらでもなければ失敗する。
#[test]
fn a_malformed_table_key_is_rejected() {
    let text = tweak(
        r#"[entry."ukadoc:list_propertysystem:system.year:1"]"#,
        r#"[entry."ukadoc"]"#,
    );

    let err = err_of(&text);
    match err {
        SurveyError::BadEntryId { raw } => assert_eq!(raw, "ukadoc"),
        other => panic!("項目 id の形の失敗ではない: {other}"),
    }
}

/// 欄に書かれた相手 id が 2 形のどちらでもなければ、**どの項目の**どの欄かを添えて失敗する。
#[test]
fn a_malformed_reference_id_names_the_entry() {
    let in_alias_of = tweak(
        r#"alias_of = "ukadoc:list_propertysystem:currentghost.balloon.scope(ID).width:1""#,
        r#"alias_of = "ukadoc::empty:1""#,
    );
    assert_mentions(&err_of(&in_alias_of), &[BALLOON, "alias_of"]);

    let in_supersedes = tweak(
        r#"supersedes = ["ukadoc:list_propertysystem:system.year.old:1"]"#,
        r#"supersedes = ["list_propertysystem:system.year.old:1"]"#,
    );
    assert_mentions(&err_of(&in_supersedes), &[YEAR, "supersedes"]);

    let in_link = tweak(
        r#"to = "ukadoc:list_propertysystem:system.month:1""#,
        r#"to = "ukadoc:a:b""#,
    );
    assert_mentions(&err_of(&in_link), &[YEAR, "to"]);
}

// ---- 型（付録 A.2 の「型」の列） ----

/// 欄の型が違えば失敗する。黙って別の値に読み替えない。
#[test]
fn a_column_of_the_wrong_type_is_rejected() {
    let values_not_array = tweak(r#"values = ["気配", "更新"]"#, r#"values = "気配""#);
    assert_mentions(&err_of(&values_not_array), &[YEAR, "values"]);

    let note_not_string =
        without_line(r#"note = "旧名。本文注記により currentghost.* 側が正典。""#);
    let note_not_string = note_not_string.replace("links = []", "links = []\nnote = 3");
    assert_mentions(&err_of(&note_not_string), &[BALLOON, "note"]);

    let link_not_table = tweak(
        r#"{ kind = "same-feature", to = "ukadoc:list_propertysystem:system.month:1" }"#,
        r#""same-feature""#,
    );
    assert_mentions(&err_of(&link_not_table), &[YEAR, "links"]);

    let pages_not_array = tweak(
        r#"pages = ["list_propertysystem"]"#,
        r#"pages = "list_propertysystem""#,
    );
    assert_mentions(&err_of(&pages_not_array), &["[ledger]", "pages"]);
}

/// 前置きが無い台帳は落とす（どのドメインの台帳かを名乗らない本文を読み進めない）。
#[test]
fn a_ledger_without_a_prologue_is_rejected() {
    let text = tweak(
        "[ledger]\ndomain = \"property\"\npages = [\"list_propertysystem\"]\n",
        "",
    );

    let err = err_of(&text);
    assert_mentions(&err, &["[ledger]"]);
}

/// TOML として読めない本文は、置き場を添えて落とす（要件 6.12）。
#[test]
fn an_unreadable_body_names_the_file() {
    let text = tweak(r#"owner = "areka-P0-property-catalog-lists""#, "owner = ");

    let err = err_of(&text);
    assert_mentions(&err, &[FILE]);
}
