//! 先送りしたプロパティ `currentghost.seriko.zorder` について、⑴プロパティの一覧
//! （sylphya の語彙表）に名前が現れないこと、⑵参照と書込が未提供のプロパティに
//! 対する現行どおりの応答になること、⑶その試みが窓の重なりの状態を動かさないこと、
//! の 3 つを固定する檻（要件 13.1／13.2／13.5）。
//!
//! # なぜ本機能の側に置くか
//!
//! design の要件対応表は要件 13 を「実装なし＝現行どおりの応答を維持・sylphya 非接触」
//! と定めている。守るべきものは sylphya の中の何かではなく、**本機能が sylphya へ何も
//! 足していないこと**なので、檻は本機能の側（台帳の兄弟）に置く。sylphya のファイルは
//! 1 行も編集しない。
//!
//! # 恒真を避けるための対照
//!
//! 「一覧に無い」「見つからない」の類は、一覧が空でも・読み口が壊れていても真になる。
//! そこで各テストは**同じ道具で既知の実在物を必ず 1 つ見つける**対照を同居させる:
//!
//! - 語彙表の走査（[`t_zpd10_no_vocabulary_table_carries_the_deferred_name`]）には、
//!   同じ述語が `seriko.` で必ず当たることと、表 5 本それぞれが空でないことを添える。
//! - 走査対象の名簿そのものが実物からずれないことは
//!   [`t_zpd12_every_public_vocab_const_is_scanned_or_excluded_on_purpose`] が受け持つ
//!   （名簿倒れ＝「名簿から漏れた表へ登録しても緑のまま」を機械で閉じる）。
//! - 参照の応答（[`t_zpd20_reading_the_deferred_property_is_not_found`]）には、
//!   同じ鏡像・同じ読み口で提供済みの名前が値を返すことを添える。
//! - 書込の応答（[`t_zpd30_writing_the_deferred_property_is_dropped`]）には、
//!   同じ分類器が別の名前を別の分岐へ落とすことを添える。
//! - ソース走査（[`t_zpd40_the_property_system_sources_never_mention_the_name`]）には、
//!   同じ走査が既知の語を必ず拾うことと、走査したファイル数の下限を添える。
//! - 台帳の不動（[`t_zpd50_touching_the_deferred_property_leaves_the_ledger_untouched`]）
//!   には、同じ比較が本物の重なり指定では必ず差を検出することを添える。
//!
//! # 期待値は檻の側の literal
//!
//! 応答の期待値（`NotFound`／`NotSettable`）は実装から導出せず、この檻の中に文字として
//! 書く。実装から引いた値を期待値にすると、実装が変わったときに期待値も一緒に動いて
//! 何も守らなくなる。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_sylphya::vocab::dotted::{DOTTED_ROOTS, GENERIC_PROP_NAMES, SET_EFFECTIVE};
use areka_sylphya::vocab::flat::FLAT_VOCAB;
use areka_sylphya::vocab::shiori_resource::SHIORI_RESOURCE_IDS;
use areka_sylphya::{
    AskerContext, AskerId, DottedResolution, Effect, MirrorImage, SetClass, SharedMirror,
    SylphyaCore, SylphyaMsg, SylphyaReader, classify_set,
};

use super::{ZOrderGroupLedger, parse_zorder_tokens};

// ---------------------------------------------------------------------------
// 檻の道具
// ---------------------------------------------------------------------------

/// 本リリースで提供しないプロパティの完全な名前（要件 13.1）。
const DEFERRED_PROPERTY: &str = "currentghost.seriko.zorder";

/// 先送りした名前を語彙表とソースから探すときの探し語（小文字）。
///
/// 完全名だけを探すと `seriko.zorder` の形での先行登録を見逃すので、両方に共通する
/// 部分語で探す（要件 13.5 の「名前だけの先行登録もしない」）。
const DEFERRED_NEEDLE: &str = "zorder";

/// 問い合わせ元（本檻はどのテストも同じ 1 つを使う。値そのものに意味は無い）。
const ASKER: &str = "ghost/zorder-property-deferral";

fn asker_ctx() -> AskerContext {
    AskerContext {
        asker: AskerId::new(ASKER),
    }
}

/// 走査対象＝**プロパティ名を要素として載せる表**の名簿（const 名だけ）。
///
/// [`vocabulary_tables`] が実際に読む表と一致していることは
/// [`t_zpd12_every_public_vocab_const_is_scanned_or_excluded_on_purpose`] が見張る。
const SCANNED_VOCAB_TABLES: [&str; 5] = [
    "FLAT_VOCAB",
    "SHIORI_RESOURCE_IDS",
    "DOTTED_ROOTS",
    "GENERIC_PROP_NAMES",
    "SET_EFFECTIVE",
];

/// 走査対象外＝`vocab/` の公開 const のうち、**プロパティ名を載せない**もの（と外す理由）。
///
/// 要件 13.5 が言う「プロパティ一覧」は、プロパティの名前そのものを要素として持つ表を
/// 指す。ここに挙げた 3 本はいずれもそれに当たらないので走査対象へ入れない——入れると
/// 「一覧」の定義が、記法の記録やイベント名の予約まで含む別物へ広がってしまう。
const NON_PROPERTY_VOCAB_CONSTS: [(&str, &str); 3] = [
    (
        "SYNTAX_RECORDS",
        "解決対象外の記法（`*`・`property[...]`）の記録であって、プロパティの名前ではない",
    ),
    (
        "EXT_EVENT_GET",
        "ext 亜枝の取得で発火するイベント名の予約であって、プロパティの名前ではない",
    ),
    (
        "EXT_EVENT_SET",
        "ext 亜枝の設定で発火するイベント名の予約であって、プロパティの名前ではない",
    ),
];

/// `vocab::dotted::SET_EFFECTIVE` のような修飾名から末尾の const 名だけを取り出す。
fn const_ident(qualified: &str) -> &str {
    qualified.rsplit("::").next().unwrap_or(qualified)
}

/// プロパティ名を載せる語彙表 5 本を `(表の名前, 載っている名前の列)` で返す。
///
/// # この 5 本は `vocab/` の公開 const 全数ではない
///
/// `crates/areka-sylphya/src/vocab/` の公開 const は現物 8 本ある。ここが読むのは
/// そのうち**プロパティ名を要素として持つ 5 本**だけで、残る 3 本は
/// [`NON_PROPERTY_VOCAB_CONSTS`] へ除外理由つきで登記してある。
///
/// 名簿は実物からずれても「載っていない」の主張が緑のままになる（守りが静かに狭まる）。
/// そこで 2 段で閉じている:
///
/// ⑴名簿と実物のずれは
///   [`t_zpd12_every_public_vocab_const_is_scanned_or_excluded_on_purpose`] が
///   公開 const をソースから抜き出して両方向で見張る。
/// ⑵名簿外の表へ名前が先行登録されること自体は
///   [`t_zpd40_the_property_system_sources_never_mention_the_name`] の全ソース走査が
///   受け持つ（走査対象を 5 本へ絞ったことで守りに穴は開かない）。
fn vocabulary_tables() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        (
            "vocab::flat::FLAT_VOCAB",
            FLAT_VOCAB.iter().map(|entry| entry.token).collect(),
        ),
        (
            "vocab::shiori_resource::SHIORI_RESOURCE_IDS",
            SHIORI_RESOURCE_IDS.to_vec(),
        ),
        ("vocab::dotted::DOTTED_ROOTS", DOTTED_ROOTS.to_vec()),
        (
            "vocab::dotted::GENERIC_PROP_NAMES",
            GENERIC_PROP_NAMES.to_vec(),
        ),
        (
            "vocab::dotted::SET_EFFECTIVE",
            SET_EFFECTIVE.iter().map(|(key, _)| *key).collect(),
        ),
    ]
}

/// 語彙表 5 本を横断して、名前に `needle` を含むものを `(表の名前, 名前)` で拾う。
fn vocabulary_entries_containing(needle: &str) -> Vec<(&'static str, &'static str)> {
    let mut hits = Vec::new();
    for (table, names) in vocabulary_tables() {
        for name in names {
            if name.to_ascii_lowercase().contains(needle) {
                hits.push((table, name));
            }
        }
    }
    hits
}

/// 統一プロパティシステム（`areka-sylphya`）のソース木の根。
fn sylphya_src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("areka-sylphya")
        .join("src")
}

/// `crates/areka-sylphya/src` 配下の Rust ソースを全て集める（本番・テストの別なく）。
fn sylphya_source_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_rust_sources(&sylphya_src_dir(), &mut out);
    out.sort();
    out
}

/// `crates/areka-sylphya/src/vocab/` に宣言された公開 const の名前をソースから抜き出す。
///
/// 抜き出す対象は、字下げを除いた行頭が `pub const ` で始まる行の `:` までの識別子。
/// doc コメント（`///`）や本文中の言及には当たらない。**名簿ではなく実物を数える**のが
/// 要点で、名簿だけを見ていると新しい表が生えたときに誰も赤くならない。
fn vocab_public_consts() -> Vec<String> {
    let mut files = Vec::new();
    collect_rust_sources(&sylphya_src_dir().join("vocab"), &mut files);
    files.sort();

    let mut out = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("語彙表のソースを読めない: {} ({e})", path.display()));
        for line in text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("pub const ") else {
                continue;
            };
            let Some(name) = rest.split(':').next() else {
                continue;
            };
            let name = name.trim();
            if !name.is_empty() {
                out.push(name.to_owned());
            }
        }
    }
    out.sort();
    out
}

fn collect_rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("sylphya のソース木を読めない: {} ({e})", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("ディレクトリ項目を読めない: {e}"));
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// 何も値の入っていない鏡像に、提供済みの点付きプロパティを 1 つだけ載せた読み口。
///
/// 載せる名前（`currentghost.name`）は対照専用である——同じ鏡像・同じ読み口で
/// 「引ける名前は引ける」ことを示し、先送りした名前の `NotFound` が
/// 「読み口が壊れているから何も引けない」ではないことを保証する。
fn reader_with_one_provided_property() -> SylphyaReader {
    let mut image = MirrorImage::empty();
    image
        .dotted_per_asker
        .entry(AskerId::new(ASKER))
        .or_default()
        .insert("currentghost.name".to_owned(), "Alice".to_owned());
    SylphyaReader::new(SharedMirror::new(Arc::new(image)))
}

/// 先送りしたプロパティへの書込を 1 回試み、返る効果列を得る。
fn set_effects(key: &str, value: &str) -> Vec<Effect> {
    SylphyaCore::new().apply(&SylphyaMsg::Set {
        asker: AskerId::new(ASKER),
        key: key.to_owned(),
        value: value.to_owned(),
    })
}

// ---------------------------------------------------------------------------
// ⑴ 一覧に名前が現れない（要件 13.5）
// ---------------------------------------------------------------------------

/// 語彙表 5 本のどれにも先送りした名前が載っていない（要件 13.1／13.5）。
#[test]
fn t_zpd10_no_vocabulary_table_carries_the_deferred_name() {
    let hits = vocabulary_entries_containing(DEFERRED_NEEDLE);
    assert!(
        hits.is_empty(),
        "先送りしたプロパティの名前が sylphya の語彙表に先行登録されている\
         （要件 13.5 は名前だけの登録も禁じる）: {hits:?}"
    );

    // 完全名・短縮形の双方を名指しでも確かめる（部分語の走査が将来ゆるんでも
    // この 2 行は名前そのものを見張る）。
    for forbidden in [DEFERRED_PROPERTY, "seriko.zorder"] {
        let listed: Vec<(&str, &str)> = vocabulary_tables()
            .into_iter()
            .flat_map(|(table, names)| {
                names
                    .into_iter()
                    .filter(move |name| *name == forbidden)
                    .map(move |name| (table, name))
            })
            .collect();
        assert!(
            listed.is_empty(),
            "語彙表に {forbidden} が載っている: {listed:?}"
        );
    }
}

/// 走査した語彙表 5 本が実在し・空でなく・同じ述語が既知の名前を必ず拾う（恒真回避）。
#[test]
fn t_zpd11_the_scanned_vocabulary_tables_are_live() {
    let tables = vocabulary_tables();
    assert_eq!(
        tables.len(),
        5,
        "この檻が読む語彙表の本数が変わっている\
         （見ているのは檻の側の名簿であって、sylphya 側の表の増減は t_zpd12 の担当）"
    );
    for (table, names) in &tables {
        assert!(!names.is_empty(), "語彙表 {table} が空（走査が空振りする）");
    }

    // 表ごとに既知の実在名を 1 つずつ名指しで確かめる（どの表も本物を運んでいる）。
    let expected_members: [(&str, &str); 5] = [
        ("vocab::flat::FLAT_VOCAB", "username"),
        ("vocab::shiori_resource::SHIORI_RESOURCE_IDS", "homeurl"),
        ("vocab::dotted::DOTTED_ROOTS", "currentghost"),
        ("vocab::dotted::GENERIC_PROP_NAMES", "keroname"),
        ("vocab::dotted::SET_EFFECTIVE", "seriko.defaultsurface"),
    ];
    for (table, member) in expected_members {
        let found = tables
            .iter()
            .find(|(name, _)| *name == table)
            .map(|(_, names)| names.contains(&member))
            .unwrap_or(false);
        assert!(found, "語彙表 {table} に既知の名前 {member} が無い");
    }

    // 「無い」を主張したのと**同じ述語**が、別の探し語では必ず当たる。
    let calibration = vocabulary_entries_containing("seriko.");
    assert!(
        !calibration.is_empty(),
        "走査そのものが効いていない（既知の部分語 seriko. すら 1 件も拾えない）"
    );
    assert_eq!(
        vocabulary_entries_containing("defaultsurface"),
        vec![("vocab::dotted::SET_EFFECTIVE", "seriko.defaultsurface")],
        "走査が表の名前と項目を正しく組にして返していない"
    );
}

/// 走査対象の名簿が実物からずれない（名簿倒れを機械で閉じる）。
///
/// `t_zpd10` は**名簿に載っている表だけ**を読む。名簿から漏れた表へ名前を足しても
/// `t_zpd10` は緑のままなので、名簿が実物へ追随していることを別に主張する必要がある
/// （task 7.2 の
/// `spawn_zorder_pair_deferred_tests::the_scanned_roster_covers_every_zorder_production_source_in_this_crate`
/// と同じ形）。
///
/// # なぜ名簿を 2 本にするか
///
/// 素朴に「公開 const 全数が走査対象名簿に載っていること」を主張すると、記法の記録や
/// イベント名の予約まで「プロパティ一覧」に数えることになり、要件 13.5 の対象が歪む。
/// そこで走査対象名簿（[`SCANNED_VOCAB_TABLES`]）と除外名簿
/// （[`NON_PROPERTY_VOCAB_CONSTS`]）の 2 本を置き、抜き出した公開 const がどちらかに
/// **ちょうど 1 回**現れることを主張する——新しい表が生えたら「どちらへ入れるか決めろ」
/// と赤くなり、除外するなら理由を書かせる形になる。
#[test]
fn t_zpd12_every_public_vocab_const_is_scanned_or_excluded_on_purpose() {
    let declared = vocab_public_consts();

    // ⓪ 抜き出しの較正: 件数の下限と、両名簿の側から既知の正例が 1 つずつ挙がること。
    assert!(
        declared.len() >= 8,
        "語彙表ソースからの公開 const の抜き出しが空振りしている（抜き出せた {} 件）: {declared:?}",
        declared.len()
    );
    for known in ["DOTTED_ROOTS", "SYNTAX_RECORDS"] {
        assert!(
            declared.iter().any(|name| name == known),
            "抜き出しが効いていない（既知の公開 const {known} を拾えない）: {declared:?}"
        );
    }
    let unique: BTreeSet<&String> = declared.iter().collect();
    assert_eq!(
        unique.len(),
        declared.len(),
        "同じ公開 const を二重に数えている: {declared:?}"
    );

    // ① 走査対象名簿が、`vocabulary_tables` の**実際に読む表**と一致する
    //    （名簿と実装のどちらか一方だけを削ると、ここで赤くなる）。
    let scanned: Vec<&str> = vocabulary_tables()
        .iter()
        .map(|(qualified, _)| const_ident(qualified))
        .collect();
    assert_eq!(
        scanned,
        SCANNED_VOCAB_TABLES.to_vec(),
        "走査対象の名簿と、実際に読んでいる表がずれている"
    );

    // ② 2 つの名簿は重ならない（同じ表を走査対象にも除外にも置かない）。
    for excluded in NON_PROPERTY_VOCAB_CONSTS.map(|(name, _)| name) {
        assert!(
            !scanned.contains(&excluded),
            "{excluded} が走査対象名簿と除外名簿の両方に載っている"
        );
    }

    // ③ 実物 → 名簿: 抜き出した公開 const は、どちらかの名簿にちょうど 1 回現れる。
    for name in &declared {
        let hits = scanned.iter().filter(|entry| *entry == name).count()
            + NON_PROPERTY_VOCAB_CONSTS
                .iter()
                .filter(|(entry, _)| entry == name)
                .count();
        assert_eq!(
            hits, 1,
            "語彙表の公開 const {name} がどちらの名簿にも載っていない（または両方に載っている）\
             ——プロパティ名を載せる表なら走査対象名簿へ、そうでないなら除外名簿へ理由つきで登記すること"
        );
    }

    // ④ 名簿 → 実物: 名簿の項目が実在する（改名・撤去で名簿だけが取り残されない）。
    for name in scanned
        .iter()
        .copied()
        .chain(NON_PROPERTY_VOCAB_CONSTS.iter().map(|(name, _)| *name))
    {
        assert!(
            declared.iter().any(|entry| entry == name),
            "名簿に実在しない公開 const が載っている: {name}"
        );
    }

    // ⑤ 除外には必ず理由が書いてある（理由なしの除外を作れないようにする）。
    for (name, reason) in NON_PROPERTY_VOCAB_CONSTS {
        assert!(
            !reason.trim().is_empty(),
            "除外名簿の {name} に理由が書かれていない"
        );
    }
}

// ---------------------------------------------------------------------------
// ⑵ 参照・書込が現行どおりの応答（要件 13.1／13.2）
// ---------------------------------------------------------------------------

/// 参照は未提供のプロパティに対する現行どおりの応答＝`NotFound` を返す（要件 13.2）。
#[test]
fn t_zpd20_reading_the_deferred_property_is_not_found() {
    let reader = reader_with_one_provided_property();
    assert_eq!(
        reader.resolve_dotted_str(&asker_ctx(), DEFERRED_PROPERTY),
        DottedResolution::NotFound,
        "先送りしたプロパティが値を返している（要件 13.1 は読み取りを提供しない）"
    );
    // 短縮形でも同じ（`currentghost` を省いた形で提供され始めていない）。
    assert_eq!(
        reader.resolve_dotted_str(&asker_ctx(), "seriko.zorder"),
        DottedResolution::NotFound
    );
}

/// 対照: 同じ鏡像・同じ読み口で、提供済みのプロパティは値を返す（読み口が生きている）。
#[test]
fn t_zpd21_a_provided_property_still_resolves_to_its_value() {
    let reader = reader_with_one_provided_property();
    assert_eq!(
        reader.resolve_dotted_str(&asker_ctx(), "currentghost.name"),
        DottedResolution::Value("Alice".to_owned()),
        "読み口が壊れている（提供済みのプロパティすら引けない）"
    );
}

/// 書込は現行どおり「SET 無効な正準語彙」として落ち、書込の効果を 1 つも生まない
/// （要件 13.1／13.2）。
#[test]
fn t_zpd30_writing_the_deferred_property_is_dropped() {
    assert_eq!(
        classify_set(DEFERRED_PROPERTY),
        SetClass::NotSettable,
        "先送りしたプロパティの書込分類が変わっている"
    );
    assert_eq!(
        set_effects(DEFERRED_PROPERTY, "1,0"),
        vec![Effect::NotSettable {
            asker: AskerId::new(ASKER),
            key: DEFERRED_PROPERTY.to_owned(),
            value: "1,0".to_owned(),
        }],
        "書込の試みが現行どおりの応答（書込なし）になっていない"
    );
}

/// 対照: 同じ分類器・同じ入口が、別の名前は別の分岐へ落とす（分類が生きている）。
///
/// `NotSettable` が「何を入れてもこうなる既定」ではなく、名前に応じた判断であることを
/// 示す。ここが崩れると `t_zpd30` は「分類器が常に NotSettable を返す」だけで緑になる。
#[test]
fn t_zpd31_other_property_names_take_different_write_paths() {
    assert_eq!(
        classify_set("surface.num"),
        SetClass::RuntimeCommand,
        "SET 有効群の分類が変わっている（対照が対照にならない）"
    );
    assert_eq!(
        set_effects("surface.num", "0"),
        vec![Effect::RuntimeCommandReserved {
            asker: AskerId::new(ASKER),
            key: "surface.num".to_owned(),
            value: "0".to_owned(),
        }]
    );

    assert_eq!(
        classify_set("mystuff.custom"),
        SetClass::StoreWrite,
        "正準語彙外の自由 key の分類が変わっている（対照が対照にならない）"
    );
    assert_eq!(
        set_effects("mystuff.custom", "v"),
        vec![Effect::SetDottedPerAsker {
            asker: AskerId::new(ASKER),
            key: "mystuff.custom".to_owned(),
            value: "v".to_owned(),
        }]
    );
}

/// プロパティシステムのソースが先送りした名前を 1 度も書いていない（要件 13.1／13.5）。
///
/// 語彙表の走査（`t_zpd10`）は**表に載る名前**しか見ないので、表を経ない実装
/// （読み口や分類器の中の名前ごとの特別扱い）を拾えない。ソース全走査がその裏を取る。
#[test]
fn t_zpd40_the_property_system_sources_never_mention_the_name() {
    let files = sylphya_source_files();
    assert!(
        files.len() >= 15,
        "sylphya のソース走査が空振りしている（見つけたファイル {} 本）",
        files.len()
    );

    let mut hits: Vec<String> = Vec::new();
    let mut calibration = 0usize;
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("ソースを読めない: {} ({e})", path.display()));
        let lower = text.to_ascii_lowercase();
        if lower.contains(DEFERRED_NEEDLE) {
            hits.push(path.display().to_string());
        }
        if lower.contains("defaultsurface") {
            calibration += 1;
        }
    }

    assert!(
        hits.is_empty(),
        "統一プロパティシステムのソースが先送りした名前に触れている\
         （要件 13 は本リリースでの提供を行わないと定める）: {hits:?}"
    );
    assert!(
        calibration >= 1,
        "走査そのものが効いていない（既知の語 defaultsurface を 1 度も拾えない）"
    );
}

// ---------------------------------------------------------------------------
// ⑶ 試みても窓の重なりの状態が動かない（要件 13.2 後段）
// ---------------------------------------------------------------------------

/// 参照と書込を試みても、重なり順のグループ台帳が 1 ビットも動かない（要件 13.2）。
///
/// 対照として、同じ比較に本物の重なり指定を通すと必ず差が出ることを同じ 1 本で示す
/// ——差を検出できない比較で「動かない」と言っても何も守れない。
#[test]
fn t_zpd50_touching_the_deferred_property_leaves_the_ledger_untouched() {
    let mut ledger = ZOrderGroupLedger::default();
    let members = match parse_zorder_tokens(&["0", "1"]) {
        Ok((elements, _)) => elements,
        Err(reject) => panic!("受理されるべき指定が解釈で拒否された: {reject:?}"),
    };
    ledger
        .try_add_tag_group(members)
        .expect("受理されるべき指定が台帳に拒否された");

    let before = ledger.clone();
    let before_version = ledger.version();

    // 参照と書込を試みる。
    let reader = reader_with_one_provided_property();
    assert_eq!(
        reader.resolve_dotted_str(&asker_ctx(), DEFERRED_PROPERTY),
        DottedResolution::NotFound
    );
    assert_eq!(
        set_effects(DEFERRED_PROPERTY, "1,0"),
        vec![Effect::NotSettable {
            asker: AskerId::new(ASKER),
            key: DEFERRED_PROPERTY.to_owned(),
            value: "1,0".to_owned(),
        }]
    );

    assert_eq!(
        ledger, before,
        "先送りしたプロパティへの参照・書込がグループ台帳を動かした（要件 13.2）"
    );
    assert_eq!(
        ledger.version(),
        before_version,
        "先送りしたプロパティへの参照・書込が台帳の版を進めた（要件 13.2）"
    );

    // 対照: 本物の重なり指定は同じ比較で必ず差として現れる。
    let more = match parse_zorder_tokens(&["2", "3"]) {
        Ok((elements, _)) => elements,
        Err(reject) => panic!("受理されるべき指定が解釈で拒否された: {reject:?}"),
    };
    ledger
        .try_add_tag_group(more)
        .expect("受理されるべき指定が台帳に拒否された");
    assert_ne!(
        ledger, before,
        "台帳の比較が変化を検出できていない（不動の主張が恒真になる）"
    );
    assert!(
        ledger.version() > before_version,
        "版の比較が変化を検出できていない（不動の主張が恒真になる）"
    );
}
