//! `patch.rs` の在中テスト。
//!
//! 見本の本文はここに直に書く（実装側の定数を参照しない）。参照すると表を表自身と
//! 比べるだけになり、欄の綴りの取り違えを 1 件も捕まえられない。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない
//! （要件 6.2・設計 File Structure Plan）。
//!
//! # 「触らない」の主張には必ず対の腕を付ける
//!
//! 「備考の中の字下げ行を触らない」「`wanted` に無い id を触らない」は、何もしない
//! 実装でも緑になる。どのテストにも「触るべき行は現に変わった」を並べて置き、母数 0 の
//! 恒真で通り抜けられないようにしてある。

use std::collections::BTreeMap;

use super::replace_priority;
use crate::error::SurveyError;
use crate::ledger::read::read;
use crate::model::{Domain, EntryId};

/// 付録 A.1 の形をした見本の台帳。
///
/// 前置きと項目 2 つを持つ。1 つ目は別名（`priority` は空文字）、2 つ目は実装済みで
/// 複数行の備考を持ち、その備考には**字下げした** `priority = ` の行が 1 本ある。
/// 行頭ではないので置き換えの相手ではない。
const SAMPLE: &str = r#"# doc/ukadoc-coverage/ledger/property.toml
# 人手で記入・機械で検査する台帳。

[ledger]
domain = "property"
pages = ["list_propertysystem"]

[entry."ukadoc:list_propertysystem:balloon.scope(ID).width:1"]
status = "alias"
alias_of = "ukadoc:list_propertysystem:currentghost.balloon.scope(ID).width:1"
introduced = "2.3.53"
owner = ""
priority = ""
values = []
links = []
note = "旧名。本文注記により currentghost.* 側が正典。"

[entry."ukadoc:list_propertysystem:system.year:1"]
status = "implemented"
introduced = ""
owner = "areka-P0-property-catalog-lists"
priority = "C1"
values = []
links = []
note = """
壊れ方: 値を返せないと辞書が空文字を前提に進み、黙って壊れる。
  priority = "Z9" は字下げしてあるので置き換えの相手ではない。
"""
"#;

/// 見本の 1 つ目の項目（別名・`priority` は空文字）。
const ALIAS_ID: &str = "ukadoc:list_propertysystem:balloon.scope(ID).width:1";
/// 見本の 2 つ目の項目（実装済み・`priority` は `C1`）。
const YEAR_ID: &str = "ukadoc:list_propertysystem:system.year:1";

/// 見本の綴りから項目 id を作る。
fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は 2 形のはず")
}

/// 求める値の表を組み立てる。
fn wanted(pairs: &[(&str, &str)]) -> BTreeMap<EntryId, String> {
    pairs
        .iter()
        .map(|(raw, value)| (id(raw), (*value).to_owned()))
        .collect()
}

/// 置き換えが通ることを前提に本文を取り出す。
fn patched(text: &str, pairs: &[(&str, &str)]) -> String {
    replace_priority(text, &wanted(pairs)).expect("見本は置き換えられるはず")
}

// ---- 置き換わる行と、動かない行 ----

/// `priority` の行だけが入れ替わり、残りのバイトは 1 つも動かない。
///
/// 期待する本文をテストの側で独立に組み立てる（見本の `priority = "C1"` は 1 か所
/// だけなので、置き換えは一意に決まる）。実装の出力どうしを比べるのではないので、
/// 前置き・他の欄・空行・備考のいずれかが動けばここで赤くなる。
#[test]
fn only_the_priority_line_changes() {
    assert_eq!(
        SAMPLE.matches("\npriority = \"C1\"").count(),
        1,
        "見本の行頭の priority = \"C1\" は 1 か所だけ（期待値の組み立てが一意であること）"
    );

    let out = patched(SAMPLE, &[(YEAR_ID, "A2")]);

    assert_eq!(
        out,
        SAMPLE.replace("priority = \"C1\"", "priority = \"A2\"")
    );
    assert_ne!(out, SAMPLE, "置き換えるべき行は現に変わっていること");
}

/// `wanted` に無い id の塊は 1 バイトも変えない。
#[test]
fn a_block_absent_from_wanted_keeps_every_byte() {
    let out = patched(SAMPLE, &[(ALIAS_ID, "B1")]);

    assert!(
        out.contains("\npriority = \"C1\"\n"),
        "wanted に無い項目の priority は元のまま: {out}"
    );
    // 対の腕。wanted に載せた側は現に変わっている（何もしない実装で緑にならない）。
    assert!(out.contains("\npriority = \"B1\"\n"), "{out}");
    assert_eq!(out, SAMPLE.replace("priority = \"\"", "priority = \"B1\""));
}

/// 備考の中の**字下げ**された `priority = ` の行は置き換えの相手ではない。
#[test]
fn an_indented_priority_line_inside_a_note_is_left_alone() {
    assert!(
        SAMPLE.contains("  priority = \"Z9\" は字下げしてある"),
        "見本に字下げした priority の行があること（主張が空振りしないための足場）"
    );

    let out = patched(SAMPLE, &[(YEAR_ID, "A1")]);

    assert!(
        out.contains("  priority = \"Z9\" は字下げしてある"),
        "備考の字下げ行は写されること: {out}"
    );
    // 対の腕。行頭の側は現に変わっている。
    assert!(out.contains("\npriority = \"A1\"\n"), "{out}");
    assert!(!out.contains("\npriority = \"C1\"\n"), "{out}");
}

// ---- 冪等 ----

/// 求める値が 1 つも無ければ、返る本文は入力と 1 バイトも違わない。
#[test]
fn an_empty_request_returns_the_input_unchanged() {
    assert_eq!(patched(SAMPLE, &[]), SAMPLE);
}

/// 既に入っている値を求めても、返る本文は入力と 1 バイトも違わない。
#[test]
fn requesting_the_value_already_written_changes_nothing() {
    assert_eq!(patched(SAMPLE, &[(YEAR_ID, "C1"), (ALIAS_ID, "")]), SAMPLE);
}

/// 2 度目の置き換えは 1 度目の本文を動かさない。
#[test]
fn replacing_twice_lands_on_the_same_text() {
    let pairs = [(YEAR_ID, "A2"), (ALIAS_ID, "")];
    let once = patched(SAMPLE, &pairs);
    let twice = patched(&once, &pairs);

    assert_eq!(twice, once);
    // 対の腕。1 度目は現に本文を動かしている（冪等の主張が空振りでないこと）。
    assert_ne!(once, SAMPLE);
}

// ---- 置き換える行を決められない塊 ----

/// `priority` の行が無い項目を持つ本文。
const NO_PRIORITY: &str = r#"[ledger]
domain = "property"
pages = ["list_propertysystem"]

[entry."ukadoc:list_propertysystem:system.year:1"]
status = "implemented"
owner = ""
values = []
links = []
note = ""
"#;

/// `priority` の行が無い塊は、id を挙げて落ちる。
#[test]
fn a_block_without_a_priority_line_fails_naming_the_id() {
    match replace_priority(NO_PRIORITY, &wanted(&[(YEAR_ID, "A1")]))
        .expect_err("priority の行が無ければ落ちるはず")
    {
        SurveyError::DeriveMismatch { reason, .. } => {
            assert!(reason.contains(YEAR_ID), "{reason}");
            assert!(reason.contains("priority"), "{reason}");
        }
        other => panic!("置き換える行を決められない失敗として落ちること: {other}"),
    }
}

/// 備考の複数行文字列の**行頭**に `priority = ` がある本文。
///
/// `toml` は備考の一部としてしか読まないので、鍵は 1 つのままである（`blocks::split`
/// の較正も素通りする）。行だけを見る走査には 2 本に見えるので、どちらを直すべきかを
/// 機械が決めることはできない。
const TWO_PRIORITY_LINES: &str = r#"[ledger]
domain = "property"
pages = ["list_propertysystem"]

[entry."ukadoc:list_propertysystem:system.year:1"]
status = "implemented"
priority = "C1"
values = []
links = []
note = """
次の行は備考の一部であって、この項目の欄ではない。
priority = "Z9"
"""
"#;

/// 塊の中に行頭の `priority = ` が 2 本あれば、id を挙げて落ちる。
#[test]
fn two_priority_lines_in_one_block_fail_naming_the_id() {
    assert_eq!(
        toml_priority(TWO_PRIORITY_LINES),
        "C1",
        "toml は備考の中の行を欄として読まない（見本が狙いどおりであること）"
    );

    match replace_priority(TWO_PRIORITY_LINES, &wanted(&[(YEAR_ID, "A1")]))
        .expect_err("priority の行が 2 本あれば落ちるはず")
    {
        SurveyError::DeriveMismatch { reason, .. } => {
            assert!(reason.contains(YEAR_ID), "{reason}");
            assert!(reason.contains("2"), "本数を告げること: {reason}");
        }
        other => panic!("置き換える行を決められない失敗として落ちること: {other}"),
    }

    // 較正。同じ本文の備考の行を 1 文字字下げしただけで通る＝落ちた理由は行頭にある。
    let indented = TWO_PRIORITY_LINES.replace("\npriority = \"Z9\"", "\n priority = \"Z9\"");
    let out = replace_priority(&indented, &wanted(&[(YEAR_ID, "A1")]))
        .expect("字下げすれば置き換えられるはず");
    assert!(out.contains(" priority = \"Z9\""), "{out}");
    assert!(out.contains("\npriority = \"A1\"\n"), "{out}");
}

/// `toml` が読んだ項目の `priority`（見本が狙いどおりであることの相手側）。
fn toml_priority(text: &str) -> String {
    let root: toml::Table = text.parse().expect("見本は TOML として読めるはず");
    root["entry"][YEAR_ID]["priority"]
        .as_str()
        .expect("priority は文字列")
        .to_owned()
}

// ---- 置き換えた本文を読み直す ----

/// 置き換えの後も台帳として読め、`priority` だけが求めた値に変わっている。
#[test]
fn the_patched_text_still_reads_as_a_ledger() {
    let before = read(SAMPLE, Domain::Property).expect("見本は台帳として読めるはず");
    let out = patched(SAMPLE, &[(ALIAS_ID, ""), (YEAR_ID, "A2")]);
    let after = read(&out, Domain::Property).expect("置き換えた本文も読めるはず");

    // 期待する台帳を「元の台帳の priority だけを差し替えたもの」として組み立てる。
    // 一致すれば、他の欄も前置きも本文の並びも読み直して変わっていない。
    let mut expected = before.clone();
    expected
        .entries
        .get_mut(&id(YEAR_ID))
        .expect("見本に項目があるはず")
        .priority = "A2".to_owned();
    assert_eq!(after, expected);
    // 対の腕。読み直した値は現に動いている。
    assert_eq!(before.entries[&id(YEAR_ID)].priority, "C1");
    assert_eq!(after.entries[&id(YEAR_ID)].priority, "A2");
}

/// 値は `tomlout::basic_string` の綴りで書くので、逆斜線と二重引用符を含んでも読み戻せる。
///
/// `priority` に入る確定値は「段階 1 文字＋数値」だけだが、素朴な組み立て（値をそのまま
/// 引用符で挟む）は逃がしを落として本文を壊す。読み戻して同じ値になることで押さえる。
#[test]
fn a_value_needing_escapes_is_written_so_it_reads_back() {
    let odd = r#"A"1\"#;
    let out = patched(SAMPLE, &[(YEAR_ID, odd)]);
    let after = read(&out, Domain::Property).expect("逃がした本文も読めるはず");

    assert_eq!(after.entries[&id(YEAR_ID)].priority, odd);
    assert!(out.contains(r#"priority = "A\"1\\""#), "{out}");
}
