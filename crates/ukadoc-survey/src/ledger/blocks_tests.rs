//! `blocks.rs` の在中テスト。
//!
//! 見本の本文はここに直に書く（実装側の定数を参照しない）。参照すると表を表自身と
//! 比べるだけになり、綴りの取り違えを 1 件も捕まえられない（タスク 1.5 の教訓）。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない
//! （要件 6.2・設計 File Structure Plan）。

use super::{Block, split};
use crate::error::SurveyError;

/// 付録 A.1 の形をした見本の台帳。
///
/// 前置き（コメント 2 行＋`[ledger]` の表）と項目 2 つを持ち、2 つ目の備考は複数行の
/// 文字列である。備考の中には**字下げした**見出しらしき行を 1 本埋めてある。行頭では
/// ないので塊を始めてはならない。
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
links = [
  { kind = "queries", to = "ukadoc:list_sakura_script:\\![get,property,ID]:1" },
]
note = """
壊れ方: 値を返せないと辞書が空文字を前提に進み、黙って壊れる。
  [entry."ukadoc:fake:indented:1"] は行頭でないので塊を始めない。
"""
"#;

/// 見本の 1 つ目の塊の本文（見出し行から次の見出し行の直前まで・末尾の空行を含む）。
const FIRST_BLOCK: &str = r#"[entry."ukadoc:list_propertysystem:balloon.scope(ID).width:1"]
status = "alias"
alias_of = "ukadoc:list_propertysystem:currentghost.balloon.scope(ID).width:1"
introduced = "2.3.53"
owner = ""
priority = ""
values = []
links = []
note = "旧名。本文注記により currentghost.* 側が正典。"

"#;

/// 塊の id を並び順のまま取り出す。
fn ids(blocks: &[Block]) -> Vec<&str> {
    blocks.iter().map(|block| block.id.as_str()).collect()
}

/// `toml` が読んだ `entry` の鍵を取り出す（較正の相手側をテストが独立に組み立てる）。
fn toml_keys(text: &str) -> Vec<String> {
    let root: toml::Table = text.parse().expect("見本は TOML として読めるはず");
    root.get("entry")
        .and_then(toml::Value::as_table)
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default()
}

// ---- 切り分けの形 ----

/// 前置きの終端は最初の見出し行の始まりで、塊は本文に現れた順に並ぶ。
#[test]
fn prologue_ends_where_the_first_block_begins() {
    let (prologue, blocks) = split(SAMPLE).expect("見本は切り分けられるはず");

    let head = SAMPLE.find("[entry.").expect("見本に塊があるはず");
    assert_eq!(prologue, head);
    assert_eq!(
        ids(&blocks),
        [
            "ukadoc:list_propertysystem:balloon.scope(ID).width:1",
            "ukadoc:list_propertysystem:system.year:1",
        ]
    );
    assert!(SAMPLE[..prologue].contains("[ledger]"), "{prologue}");
}

/// 前置きと塊を順に繋ぐと、元の本文に 1 バイトも違わず戻る。
///
/// 差し込み（タスク 2.5）は塊のバイト列をそのまま写すので、ここが 1 バイトでもずれると
/// 手書きの台帳が黙って壊れる。
#[test]
fn prologue_and_blocks_reconstruct_the_input_byte_for_byte() {
    let (prologue, blocks) = split(SAMPLE).expect("見本は切り分けられるはず");
    assert!(prologue > 0, "見本には前置きがある");
    assert!(!blocks.is_empty(), "見本には塊がある");

    let mut rebuilt = String::from(&SAMPLE[..prologue]);
    for block in &blocks {
        rebuilt.push_str(&SAMPLE[block.start..block.end]);
    }
    assert_eq!(rebuilt, SAMPLE);
}

/// 塊は見出し行を含み、欄の綴りをそのまま持つ。
#[test]
fn a_block_spans_its_header_line_and_every_field() {
    let (_, blocks) = split(SAMPLE).expect("見本は切り分けられるはず");
    let first = blocks.first().expect("見本に塊があるはず");
    assert_eq!(&SAMPLE[first.start..first.end], FIRST_BLOCK);

    let last = blocks.last().expect("見本に塊があるはず");
    assert_eq!(last.end, SAMPLE.len(), "最後の塊は本文の終端まで");
    assert!(
        SAMPLE[last.start..last.end].contains("壊れ方:"),
        "最後の塊は複数行の備考を丸ごと含む"
    );
}

/// 行頭でない `[entry.` は塊の始まりにならない。
#[test]
fn an_indented_header_inside_a_note_starts_no_block() {
    assert!(
        SAMPLE.contains("  [entry.\"ukadoc:fake:indented:1\"]"),
        "見本に字下げした見出しらしき行があること（主張が空振りしないための足場）"
    );

    let (_, blocks) = split(SAMPLE).expect("見本は切り分けられるはず");
    assert!(!ids(&blocks).contains(&"ukadoc:fake:indented:1"));
    assert_eq!(blocks.len(), 2);
}

// ---- 較正（設計 `ledger` 節の事後条件・テスト 8） ----

/// 切り分けが返す id の集合は、同じ本文を `toml` で読んだ鍵の集合に一致する。
#[test]
fn split_ids_agree_with_the_toml_entry_keys() {
    let (_, blocks) = split(SAMPLE).expect("見本は切り分けられるはず");
    let mut cut: Vec<String> = ids(&blocks).iter().map(|id| (*id).to_owned()).collect();
    cut.sort();

    let keys = toml_keys(SAMPLE);
    assert_eq!(keys.len(), 2, "較正の相手が空でないこと");
    assert_eq!(cut, keys);
}

/// 備考の複数行文字列の中に、**行頭**の見出しらしき行がある本文。
///
/// 行だけを見る素朴な走査は塊を 2 つに割ってしまうが、`toml` は 1 件しか読まない。
const NOTE_WITH_HEADER: &str = r#"[ledger]
domain = "property"
pages = ["list_propertysystem"]

[entry."ukadoc:list_propertysystem:system.year:1"]
status = "unclassified"
note = """
次の行は備考の一部であって、項目の見出しではない。
[entry."ukadoc:fake:thing:1"]
"""
"#;

/// 備考の中の行頭の見出しに引っかかった切り分けは、較正が食い違いとして落とす。
#[test]
fn a_header_line_inside_a_multiline_note_is_caught_by_the_calibration() {
    assert_eq!(
        toml_keys(NOTE_WITH_HEADER),
        ["ukadoc:list_propertysystem:system.year:1"],
        "toml は備考の中の行を項目として読まない（較正の相手が正しいこと）"
    );

    match split(NOTE_WITH_HEADER).expect_err("較正が食い違いを落とすはず") {
        SurveyError::LedgerSplitMismatch { detail } => {
            assert!(detail.contains("ukadoc:fake:thing:1"), "{detail}");
        }
        other => panic!("切り分けと読み取りの食い違いとして落ちること: {other}"),
    }
}

/// 切り分けた id の集合と `toml` の鍵の集合が、**同じ件数のまま中身だけ食い違う**本文。
///
/// 備考の中の行頭の見出し（自前の走査だけが拾う）と、字下げした見出し（`toml` だけが
/// 項目として読む）を 1 本ずつ置いてある。両側とも 2 件なので、件数を数えるだけの較正は
/// これを取り逃がす。
const SAME_SIZE_DIFFERENT_MEMBERS: &str = r#"[entry."ukadoc:dev_bind"]
note = """
[entry."ukadoc:fake:one:1"]
"""

  [entry."ukadoc:dev_menu"]
status = "unclassified"
"#;

/// 較正は件数ではなく**中身**を比べ、食い違いの両側を挙げる。
///
/// 件数だけを比べる較正（`cut.len() == read.len()`）はこの本文を素通りする。つまり
/// このテストが赤になることが、比べているのが集合の中身であることの証跡である
/// （タスク 1.5 の教訓——件数だけのテストは綴り違いを 1 件も捕まえない）。
///
/// 本文に**両側**を挙げることも併せて釘付けする。片側しか挙げないと、台帳を直す人に
/// 食い違いの半分が見えない（要件 6.12）。
#[test]
fn the_calibration_compares_members_not_counts() {
    let keys = toml_keys(SAME_SIZE_DIFFERENT_MEMBERS);
    assert_eq!(
        keys,
        ["ukadoc:dev_bind", "ukadoc:dev_menu"],
        "toml は字下げした見出しを項目として読む（較正の相手も 2 件）"
    );

    match split(SAME_SIZE_DIFFERENT_MEMBERS).expect_err("中身が食い違うので落ちるはず")
    {
        SurveyError::LedgerSplitMismatch { detail } => {
            assert!(
                detail.contains("ukadoc:fake:one:1"),
                "切り分けだけが拾った id を挙げる: {detail}"
            );
            assert!(
                detail.contains("ukadoc:dev_menu"),
                "読み取りだけが拾った id を挙げる: {detail}"
            );
        }
        other => panic!("切り分けと読み取りの食い違いとして落ちること: {other}"),
    }
}

/// 備考の中の見出しが、**既にある id と同じ綴り**である本文（設計 D-12 の盲点）。
const NOTE_WITH_DUPLICATE: &str = r#"[entry."ukadoc:list_propertysystem:system.year:1"]
status = "unclassified"
note = """
[entry."ukadoc:list_propertysystem:system.year:1"]
"""
"#;

/// 較正は同じ綴りの重複を見抜けない（設計 D-12 が明記する盲点）。
///
/// 集合として比べる以上ここは通る。捕まえるのは台帳の並びを**厳密な昇順**で確かめる側
/// （[`SurveyError::LedgerOutOfOrder`]）である。この形を釘付けしておくと、較正の主張が
/// 実際より強く読まれることがなくなる。
#[test]
fn the_calibration_cannot_see_a_duplicate_of_an_existing_id() {
    let (prologue, blocks) = split(NOTE_WITH_DUPLICATE).expect("集合は一致するので通るはず");

    assert_eq!(prologue, 0);
    assert_eq!(
        ids(&blocks),
        [
            "ukadoc:list_propertysystem:system.year:1",
            "ukadoc:list_propertysystem:system.year:1",
        ]
    );
    assert_eq!(
        toml_keys(NOTE_WITH_DUPLICATE).len(),
        1,
        "toml の鍵は 1 件きり。集合にすると一致してしまう"
    );
}

// ---- 端の場合 ----

/// 項目が 1 つも無い本文は、全体が前置きになる。
#[test]
fn a_body_without_entries_is_all_prologue() {
    let text = "# 見出しだけの台帳\n\n[ledger]\ndomain = \"property\"\npages = []\n";
    let (prologue, blocks) = split(text).expect("項目が無くても切り分けられるはず");

    assert_eq!(prologue, text.len());
    assert!(blocks.is_empty());
    assert!(toml_keys(text).is_empty(), "toml も 0 件（較正の相手も空）");
}

/// 空の本文でも落ちない。
#[test]
fn an_empty_body_has_neither_prologue_nor_blocks() {
    let (prologue, blocks) = split("").expect("空の本文でも落ちないはず");
    assert_eq!(prologue, 0);
    assert!(blocks.is_empty());
}

/// 本文が塊で始まれば前置きは空になる。
#[test]
fn a_body_starting_with_a_header_has_an_empty_prologue() {
    let text = "[entry.\"ukadoc:dev_bind\"]\nstatus = \"unclassified\"\n";
    let (prologue, blocks) = split(text).expect("塊で始まる本文も切り分けられるはず");

    assert_eq!(prologue, 0);
    assert_eq!(ids(&blocks), ["ukadoc:dev_bind"]);
    assert_eq!(blocks[0].start, 0);
    assert_eq!(&text[blocks[0].start..blocks[0].end], text);
}

/// 末尾の改行があってもなくても、塊は本文の終端までを指す。
#[test]
fn a_missing_trailing_newline_does_not_shift_the_positions() {
    let with = "[entry.\"ukadoc:dev_bind\"]\nstatus = \"unclassified\"\n";
    let without = "[entry.\"ukadoc:dev_bind\"]\nstatus = \"unclassified\"";
    assert_ne!(with.len(), without.len(), "2 つは実際に別の本文");

    for text in [with, without] {
        let (prologue, blocks) = split(text).expect("どちらも切り分けられるはず");
        assert_eq!(prologue, 0, "{text:?}");
        assert_eq!(blocks.len(), 1, "{text:?}");
        assert_eq!(blocks[0].end, text.len(), "{text:?}");
        assert_eq!(&text[blocks[0].start..blocks[0].end], text);
    }
}

/// 逆斜線とコロンを含む id が、逃がしを解いた綴りで返る（付録 A.3）。
#[test]
fn ids_with_backslashes_and_colons_are_unescaped() {
    let text = concat!(
        "[entry.\"ukadoc:list_sakura_script:\\\\![get,property,ID]:1\"]\n",
        "status = \"unclassified\"\n",
        "\n",
        "[entry.\"ukadoc:list_sakura_script:_5c_21_5b_67_65_74:1\"]\n",
        "status = \"unclassified\"\n",
    );
    let (_, blocks) = split(text).expect("逆斜線を含む見出しも切り分けられるはず");

    assert_eq!(
        ids(&blocks),
        [
            "ukadoc:list_sakura_script:\\![get,property,ID]:1",
            "ukadoc:list_sakura_script:_5c_21_5b_67_65_74:1",
        ]
    );
    assert_eq!(toml_keys(text), ids(&blocks), "較正が通ること");
}

/// 項目の後ろに `[ledger]` の表が来ても塊は切れず、直前の塊に含まれる。
#[test]
fn a_ledger_table_after_the_entries_stays_inside_the_preceding_block() {
    let text = concat!(
        "[entry.\"ukadoc:dev_bind\"]\n",
        "status = \"unclassified\"\n",
        "\n",
        "[ledger]\n",
        "domain = \"assets\"\n",
    );
    let (prologue, blocks) = split(text).expect("TOML としては読めるので通るはず");

    assert_eq!(prologue, 0);
    assert_eq!(blocks.len(), 1);
    assert!(text[blocks[0].start..blocks[0].end].contains("[ledger]"));
    assert_eq!(&text[blocks[0].start..blocks[0].end], text);
}

// ---- 繕わずに落ちる ----

/// 引用符で囲まれていない見出しは、項目 id の形の誤りとして落ちる。
#[test]
fn an_unquoted_header_key_is_rejected_as_a_bad_id() {
    let text = "[entry.notquoted]\nstatus = \"unclassified\"\n";
    match split(text).expect_err("項目 id の形でないので落ちるはず") {
        SurveyError::BadEntryId { raw } => assert_eq!(raw, "notquoted"),
        other => panic!("項目 id の形の誤りとして落ちること: {other}"),
    }
}

/// 引用符が閉じていない見出しは、読めなかった行を挙げて落ちる。
#[test]
fn an_unterminated_header_quote_is_rejected() {
    let text = "[entry.\"unterminated]\nstatus = \"unclassified\"\n";
    match split(text).expect_err("見出しが読めないので落ちるはず") {
        SurveyError::TomlParse { path, reason } => {
            assert!(path.contains("ledger"), "{path}");
            assert!(reason.contains("[entry.\"unterminated]"), "{reason}");
        }
        other => panic!("読み取りの失敗として落ちること: {other}"),
    }
}

/// 見出しは読めても本文全体が TOML にならなければ、較正できないので落ちる。
#[test]
fn a_body_that_is_not_toml_is_rejected_because_it_cannot_be_calibrated() {
    let text = "[entry.\"ukadoc:dev_bind\"]\nstatus =\n";
    match split(text).expect_err("較正の相手が作れないので落ちるはず") {
        SurveyError::TomlParse { path, reason } => {
            assert!(path.contains("ledger"), "{path}");
            assert!(!reason.is_empty());
        }
        other => panic!("読み取りの失敗として落ちること: {other}"),
    }
}

/// 復帰文字を落としていない本文は、黙って直さずに落ちる。
///
/// 事前条件は「復帰文字を落とした本文」（設計 `ledger` 節・D-6）。ここで復帰文字を
/// 落として読み進めると、返すバイト位置が呼び出し側の持つ本文と食い違う。
#[test]
fn a_body_still_carrying_carriage_returns_is_rejected() {
    let text = "[entry.\"ukadoc:dev_bind\"]\r\nstatus = \"unclassified\"\r\n";
    match split(text).expect_err("復帰文字付きの見出しは読めないはず") {
        SurveyError::TomlParse { reason, .. } => {
            assert!(reason.contains("[entry."), "{reason}");
        }
        other => panic!("読み取りの失敗として落ちること: {other}"),
    }
}
