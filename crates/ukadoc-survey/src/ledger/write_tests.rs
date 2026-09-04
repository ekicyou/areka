//! `write.rs` の在中テスト。
//!
//! 見本も期待する本文もここに直に書く（実装側の定数を参照しない）。参照すると表を
//! 表自身と比べるだけになり、綴りの取り違えを 1 件も捕まえられない（タスク 1.5 の教訓）。
//!
//! ここは純粋層のテストなので、ファイルも一時ディレクトリも 1 つも作らない
//! （要件 6.2・設計 File Structure Plan）。

use super::{merge_initial, render_initial_entry};
use crate::error::SurveyError;
use crate::ledger::blocks;
use crate::ledger::read::read;
use crate::model::{Domain, EntryId, PageName, Status};

/// 見本のドメインが担当するページ（要件 3.1 の property の行）。
const PAGES: [&str; 1] = ["list_propertysystem"];

/// 要件 3.1 の `shiori` の行 12 ページ。**件数の多い順**（＝文字順でも逆順でもない）。
const SHIORI_PAGES: [&str; 12] = [
    "list_shiori_event",
    "list_shiori_event_ex",
    "list_shiori_resource",
    "list_plugin_event",
    "memo_shiorievent",
    "spec_shiori3",
    "spec_fmo_mutex",
    "spec_web",
    "spec_sstp",
    "spec_dll",
    "spec_plugin",
    "spec_headline",
];

/// id の byte 昇順で 1 番目。
const BALLOON: &str = "ukadoc:list_propertysystem:balloon.scope(ID).width:1";

/// id の byte 昇順で 2 番目（差し込みの試験体。両端でなく**真ん中**）。
const MONTH: &str = "ukadoc:list_propertysystem:system.month:1";

/// id の byte 昇順で 3 番目。
const YEAR: &str = "ukadoc:list_propertysystem:system.year:1";

/// 逆斜線を含む id（要件付録 A.3 の例）。
const BACKSLASH: &str = r"ukadoc:list_sakura_script:\![get,property,ID]:1";

fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は 2 形のいずれかのはず")
}

fn ids(raw: &[&str]) -> Vec<EntryId> {
    raw.iter().map(|one| id(one)).collect()
}

fn pages() -> Vec<PageName> {
    PAGES.iter().map(|name| PageName::new(*name)).collect()
}

/// 3 件そろった台帳を新規に組み立てる。
fn full() -> String {
    merge_initial(
        None,
        Domain::Property,
        &pages(),
        &ids(&[BALLOON, MONTH, YEAR]),
    )
    .expect("新規の組み立ては落ちないはず")
}

/// 本文を「id → 塊のバイト列」に開く。差し込みの前後で塊が動いていないことを見るため。
fn block_texts(text: &str) -> Vec<(String, String)> {
    let (_, blocks) = blocks::split(text).expect("見本は切り分けられるはず");
    blocks
        .iter()
        .map(|block| {
            (
                block.id.as_str().to_owned(),
                text[block.start..block.end].to_owned(),
            )
        })
        .collect()
}

/// 前置き（最初の塊より前）を取り出す。
fn prologue_of(text: &str) -> String {
    let (end, _) = blocks::split(text).expect("見本は切り分けられるはず");
    text[..end].to_owned()
}

/// 塊を 1 つだけ落とした本文を作る。落とした前後は 1 バイトも触らない。
fn without_block(text: &str, target: &EntryId) -> String {
    let (_, blocks) = blocks::split(text).expect("見本は切り分けられるはず");
    let block = blocks
        .iter()
        .find(|block| &block.id == target)
        .expect("落とす塊が本文に無い");
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..block.start]);
    out.push_str(&text[block.end..]);
    out
}

// ---- 新規の組み立て（要件 3.3・付録 A.1・A.2） ----

/// 既存本文が無ければ、付録 A.1 の形で全項目を書き出す。
///
/// 期待する本文を丸ごと逐語で置く。読み戻し一致だけでは区切りの空白 1 個の違いを
/// 捕まえられないが、1 バイトの安定はそこに載っている（タスク 1.4・2.2 の教訓）。
#[test]
fn a_missing_body_is_rendered_in_the_appendix_a_shape() {
    let text = merge_initial(None, Domain::Property, &pages(), &ids(&[BALLOON, MONTH]))
        .expect("新規の組み立ては落ちないはず");

    assert_eq!(
        text,
        r#"# doc/ukadoc-coverage/ledger/property.toml
# 人手で記入・機械で検査する台帳。形式の正本は
# .kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md 付録 A。

[ledger]
domain = "property"
pages = ["list_propertysystem"]

[entry."ukadoc:list_propertysystem:balloon.scope(ID).width:1"]
status = "unclassified"
introduced = ""
owner = ""
priority = ""
values = []
links = []
note = ""

[entry."ukadoc:list_propertysystem:system.month:1"]
status = "unclassified"
introduced = ""
owner = ""
priority = ""
values = []
links = []
note = ""

"#
    );
}

/// 項目が 1 件も無くても、前置きだけは付録 A.1 の形で出る。
#[test]
fn an_empty_id_list_renders_the_prologue_alone() {
    let text =
        merge_initial(None, Domain::Property, &pages(), &[]).expect("新規の組み立ては落ちないはず");

    assert_eq!(
        text,
        r#"# doc/ukadoc-coverage/ledger/property.toml
# 人手で記入・機械で検査する台帳。形式の正本は
# .kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md 付録 A。

[ledger]
domain = "property"
pages = ["list_propertysystem"]

"#
    );
}

/// 前置きは読んでいるドメインの綴りを持つ（要件 3.4 の独立性はここから始まる）。
#[test]
fn the_prologue_names_the_domain_and_its_pages() {
    let text = merge_initial(
        None,
        Domain::SakuraScript,
        &[PageName::new("list_sakura_script")],
        &[],
    )
    .expect("新規の組み立ては落ちないはず");

    assert_eq!(
        text,
        r#"# doc/ukadoc-coverage/ledger/sakura-script.toml
# 人手で記入・機械で検査する台帳。形式の正本は
# .kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md 付録 A。

[ledger]
domain = "sakura-script"
pages = ["list_sakura_script"]

"#
    );
}

/// 担当ページが複数あるときも、**渡された順のまま**書く。
///
/// 見本は要件 3.1 の `shiori` の行 12 ページを、表と同じ件数の多い順に置いてある——
/// 文字順でも逆順でもない並びであることが要点である。文字順に整った一覧を渡すと、
/// 一覧を並べ替える間違いがそのまま通ってしまう。
///
/// ここを釘付けしておく理由は 3 つある。⑴ 4 本の台帳のうち `shiori`（12 ページ）と
/// `assets`（24 ページ）は複数ページであり、1 ページきりの見本ではこの経路を 1 度も
/// 通らない。⑵ 整合検査の担当ページの突き合わせは集合で比べるので、並びの違いは
/// 下流でも見えない。⑶ 要件 3.3a により前置きは初回の生成以降そのまま写され続けるので、
/// 並びが狂ったまま焼き付く。
#[test]
fn a_multi_page_prologue_keeps_the_given_page_order() {
    let pages: Vec<PageName> = SHIORI_PAGES
        .iter()
        .map(|name| PageName::new(*name))
        .collect();

    let text =
        merge_initial(None, Domain::Shiori, &pages, &[]).expect("新規の組み立ては落ちないはず");

    assert_eq!(
        text,
        r#"# doc/ukadoc-coverage/ledger/shiori.toml
# 人手で記入・機械で検査する台帳。形式の正本は
# .kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md 付録 A。

[ledger]
domain = "shiori"
pages = ["list_shiori_event", "list_shiori_event_ex", "list_shiori_resource", "list_plugin_event", "memo_shiorievent", "spec_shiori3", "spec_fmo_mutex", "spec_web", "spec_sstp", "spec_dll", "spec_plugin", "spec_headline"]

"#
    );
}

/// 1 項目の初期値は付録 A.2 の 1 文のとおり。
#[test]
fn an_initial_entry_is_pinned_verbatim() {
    assert_eq!(
        render_initial_entry(&id(YEAR)),
        r#"[entry."ukadoc:list_propertysystem:system.year:1"]
status = "unclassified"
introduced = ""
owner = ""
priority = ""
values = []
links = []
note = ""

"#
    );
}

/// 逆斜線を含む id は 2 つ重ねて書く（要件付録 A.3）。
#[test]
fn a_backslash_in_the_id_is_doubled() {
    assert_eq!(
        render_initial_entry(&id(BACKSLASH)),
        r#"[entry."ukadoc:list_sakura_script:\\![get,property,ID]:1"]
status = "unclassified"
introduced = ""
owner = ""
priority = ""
values = []
links = []
note = ""

"#
    );
}

/// 書き出した本文は台帳として読み戻せる（付録 A.2 が必須とする欄が全部ある）。
///
/// 逐語テストと対で置く。読み戻し一致は「単射」しか言わないが、逐語は「欄が足りない」
/// を言えない（タスク 1.4 の教訓）。
#[test]
fn the_rendered_body_reads_back_as_a_ledger() {
    let text = full();

    let ledger = read(&text, Domain::Property).expect("書き出した本文は読めるはず");

    assert_eq!(ledger.domain, Domain::Property);
    assert_eq!(ledger.pages, vec![PageName::new("list_propertysystem")]);
    assert_eq!(ledger.file_order, ids(&[BALLOON, MONTH, YEAR]));
    for entry in ledger.entries.values() {
        assert_eq!(entry.status, Status::Unclassified, "{}", entry.id.as_str());
        assert_eq!(entry.introduced, "");
        assert_eq!(entry.owner, "");
        assert_eq!(entry.priority, "");
        assert!(entry.values.is_empty());
        assert!(entry.links.is_empty());
        assert_eq!(entry.note, "");
        assert_eq!(entry.alias_of, None);
        assert!(entry.supersedes.is_empty());
    }
}

// ---- 塊を 1 つ落として差し込み直す（タスク 2.5 の完了条件） ----

/// 落とした塊だけが戻り、他の塊は 1 バイトも動かない。
///
/// 両端と真ん中を別々に確かめる。差し込む位置は `blocks::split` が返すバイト位置から
/// 決まるので、端で 1 つずれる間違いが本物の危険である。
fn one_block_comes_back(target: &str) {
    let text = full();
    let dropped = without_block(&text, &id(target));
    let before = block_texts(&dropped);

    let spliced = merge_initial(
        Some(&dropped),
        Domain::Property,
        &pages(),
        &ids(&[BALLOON, MONTH, YEAR]),
    )
    .expect("差し込みは落ちないはず");

    assert_eq!(spliced, text, "落とす前の本文に 1 バイトも違わず戻る");

    let after = block_texts(&spliced);
    assert_eq!(after.len(), before.len() + 1, "増えるのは 1 つだけ");
    for (id_before, body_before) in &before {
        let (_, body_after) = after
            .iter()
            .find(|(id_after, _)| id_after == id_before)
            .unwrap_or_else(|| panic!("落としていない塊が消えた: {id_before}"));
        assert_eq!(body_after, body_before, "他の塊が動いた: {id_before}");
    }
    let added: Vec<&String> = after
        .iter()
        .filter(|(id_after, _)| !before.iter().any(|(id_before, _)| id_before == id_after))
        .map(|(id_after, _)| id_after)
        .collect();
    assert_eq!(
        added,
        vec![&target.to_owned()],
        "戻ったのは落とした 1 つだけ"
    );
}

#[test]
fn dropping_the_first_block_puts_back_only_that_block() {
    one_block_comes_back(BALLOON);
}

#[test]
fn dropping_the_middle_block_puts_back_only_that_block() {
    one_block_comes_back(MONTH);
}

#[test]
fn dropping_the_last_block_puts_back_only_that_block() {
    one_block_comes_back(YEAR);
}

// ---- 手で書いた本文をそのまま写す（要件 3.3a・設計 D-12） ----

/// 手で書いた台帳。区切りの空白も空行も持ち主の書き方のまま置いてある。
///
/// 1 つ目と 2 つ目の間に**空行が無い**のも意図してある。差し込みが余計な空行を
/// 足していないかは、ここが赤くなることで分かる。
const HAND_WRITTEN: &str = r#"# doc/ukadoc-coverage/ledger/property.toml
# 手で書いた台帳。

[ledger]
domain =   "property"
pages = [ "list_propertysystem" ]

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
status  =  "implemented"
introduced = ""
owner = "areka-P0-property-catalog-lists"
priority = "C1"
values = ["気配", "更新"]
links = [
  { kind = "queries", to = "ukadoc:list_sakura_script:\\![get,property,ID]:1" },
]
note = """
壊れ方: 値を返せないと辞書が空文字を前提に進み、黙って壊れる。

上の空行ごと写されること。
"""
"#;

/// 手で書いた塊も前置きも 1 バイトも変わらず、欠けた id だけが順の位置へ入る。
#[test]
fn hand_written_blocks_and_prologue_survive_verbatim() {
    let before = block_texts(HAND_WRITTEN);

    let spliced = merge_initial(
        Some(HAND_WRITTEN),
        Domain::Property,
        &pages(),
        &ids(&[BALLOON, MONTH, YEAR]),
    )
    .expect("差し込みは落ちないはず");

    assert_eq!(
        prologue_of(&spliced),
        prologue_of(HAND_WRITTEN),
        "前置きは組み立て直さず写す"
    );

    let after = block_texts(&spliced);
    let order: Vec<&str> = after.iter().map(|(one, _)| one.as_str()).collect();
    assert_eq!(order, [BALLOON, MONTH, YEAR], "欠けた id は文字順の位置へ");

    for (id_before, body_before) in &before {
        let (_, body_after) = after
            .iter()
            .find(|(id_after, _)| id_after == id_before)
            .unwrap_or_else(|| panic!("手書きの塊が消えた: {id_before}"));
        assert_eq!(
            body_after, body_before,
            "手書きの塊が書き換わった: {id_before}"
        );
    }
    assert!(
        spliced.contains("上の空行ごと写されること。"),
        "複数行の備考が写っていない"
    );
    assert_eq!(
        after[1].1,
        render_initial_entry(&id(MONTH)),
        "差し込んだ塊は付録 A の初期値"
    );
}

/// 台帳にあってカタログの id 一覧に無い項目は、落とさずそのまま残す。
#[test]
fn entries_outside_the_given_id_list_are_left_alone() {
    let spliced = merge_initial(
        Some(HAND_WRITTEN),
        Domain::Property,
        &pages(),
        &ids(&[MONTH]),
    )
    .expect("差し込みは落ちないはず");

    let order: Vec<String> = block_texts(&spliced)
        .into_iter()
        .map(|(one, _)| one)
        .collect();
    assert_eq!(order, [BALLOON, MONTH, YEAR], "一覧に無い id も残る");
}

/// 欠けている id が 1 つも無ければ、本文は 1 バイトも動かない（要件 3.3a）。
#[test]
fn nothing_missing_leaves_the_hand_written_body_untouched() {
    let spliced = merge_initial(
        Some(HAND_WRITTEN),
        Domain::Property,
        &pages(),
        &ids(&[BALLOON, YEAR]),
    )
    .expect("差し込みは落ちないはず");

    assert_eq!(spliced, HAND_WRITTEN);
}

/// 同じ本文をもう一度通しても変わらない（タスク 6.2 の完了条件が載っている）。
#[test]
fn splicing_the_same_body_again_changes_nothing() {
    let once = full();

    let twice = merge_initial(
        Some(&once),
        Domain::Property,
        &pages(),
        &ids(&[BALLOON, MONTH, YEAR]),
    )
    .expect("差し込みは落ちないはず");

    assert_eq!(twice, once);
}

/// 末尾に改行の無い本文の後ろへ差し込んでも、見出しは行頭から始まる。
#[test]
fn a_body_without_a_trailing_newline_still_gets_a_line_start() {
    let trimmed = full().trim_end().to_owned();
    let last = "ukadoc:list_propertysystem:system.zzz:1";

    let spliced = merge_initial(
        Some(&trimmed),
        Domain::Property,
        &pages(),
        &ids(&[BALLOON, MONTH, YEAR, last]),
    )
    .expect("差し込みは落ちないはず");

    assert!(
        spliced.contains("note = \"\"\n[entry.\"ukadoc:list_propertysystem:system.zzz:1\"]"),
        "行頭でない位置に見出しが置かれた: {spliced}"
    );
    let order: Vec<String> = block_texts(&spliced)
        .into_iter()
        .map(|(one, _)| one)
        .collect();
    assert_eq!(order, [BALLOON, MONTH, YEAR, last]);
}

// ---- 並び順は厳密な昇順（設計 D-12） ----

/// 塊が id の順に並んでいない本文。`system.year` の後ろに `balloon…` が来る。
const DECREASING: &str = r#"[ledger]
domain = "property"
pages = ["list_propertysystem"]

[entry."ukadoc:list_propertysystem:system.year:1"]
status = "unclassified"
introduced = ""
owner = ""
priority = ""
values = []
links = []
note = ""

[entry."ukadoc:list_propertysystem:balloon.scope(ID).width:1"]
status = "unclassified"
introduced = ""
owner = ""
priority = ""
values = []
links = []
note = ""
"#;

/// 並べ替えず、順序を破る id を告げて失敗する（設計 D-12）。
#[test]
fn a_decreasing_body_fails_naming_the_offending_id() {
    let failure = merge_initial(
        Some(DECREASING),
        Domain::Property,
        &pages(),
        &ids(&[BALLOON, MONTH, YEAR]),
    )
    .expect_err("並びが逆なら落ちるはず");

    match failure {
        SurveyError::LedgerOutOfOrder { file, id: named } => {
            assert_eq!(file, "doc/ukadoc-coverage/ledger/property.toml");
            assert_eq!(named, BALLOON, "順序を破っている側の id を告げる");
        }
        other => panic!("並び順の失敗として落ちること: {other}"),
    }
}

/// 備考の中の見出しが**既にある id と同じ綴り**である本文（設計 D-12 の盲点）。
///
/// `toml` の鍵の集合は 1 件なので `blocks::split` の較正は素通りする。塞ぐのは
/// 厳密な昇順の判定だけである。
const DUPLICATE_IN_NOTE: &str = r#"[ledger]
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
[entry."ukadoc:list_propertysystem:system.year:1"]
"""
"#;

/// 同じ id が 2 度現れたら、重複として落ちる（設計 D-12・タスク 2.3 からの申し送り）。
///
/// 判定が「非減少」だとここは通ってしまい、盲点は永久に開いたままになる。
/// `ledger-init` は整合検査より先に走るので、塞ぐ場所はここしかない。
#[test]
fn a_duplicate_id_hidden_in_a_note_fails_as_out_of_order() {
    let (_, blocks) = blocks::split(DUPLICATE_IN_NOTE).expect("較正は素通りするはず");
    assert_eq!(blocks.len(), 2, "切り分けは同じ id を 2 つ拾う");

    let failure = merge_initial(
        Some(DUPLICATE_IN_NOTE),
        Domain::Property,
        &pages(),
        &ids(&[YEAR]),
    )
    .expect_err("同じ id が 2 度あれば落ちるはず");

    match failure {
        SurveyError::LedgerOutOfOrder { file, id: named } => {
            assert_eq!(file, "doc/ukadoc-coverage/ledger/property.toml");
            assert_eq!(named, YEAR);
        }
        other => panic!("並び順の失敗として落ちること: {other}"),
    }
}

/// 塊が 1 つだけの本文は、比べる隣が無いので並び順で落ちない。
///
/// 「順序を破る隣が無い」ことを言う対照。全称の主張だけだと対象が空でも真になる
/// （タスク 1.6 の教訓）ので、落ちる側と対で置く。
#[test]
fn a_single_block_body_passes_the_order_check() {
    let one = merge_initial(None, Domain::Property, &pages(), &ids(&[YEAR]))
        .expect("新規の組み立ては落ちないはず");

    let spliced = merge_initial(
        Some(&one),
        Domain::Property,
        &pages(),
        &ids(&[BALLOON, YEAR]),
    )
    .expect("塊が 1 つなら並び順では落ちないはず");

    let order: Vec<String> = block_texts(&spliced)
        .into_iter()
        .map(|(id_after, _)| id_after)
        .collect();
    assert_eq!(order, [BALLOON, YEAR]);
}

/// 切り分けが失敗する本文は、そのまま失敗が上がる（繕わない）。
#[test]
fn an_unreadable_body_is_not_repaired() {
    let failure = merge_initial(Some("[entry.\n"), Domain::Property, &pages(), &[])
        .expect_err("読めない本文は落ちるはず");

    match failure {
        SurveyError::TomlParse { .. } => {}
        other => panic!("読み取りの失敗として落ちること: {other}"),
    }
}
