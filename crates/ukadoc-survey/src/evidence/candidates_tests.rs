//! `candidates.rs` の在中テスト。
//!
//! 守るのは 4 つ。⑴ **候補と証拠が分かれていること**（要件 5.8・5.9）——同じ 1 本の
//! 本文から、候補は種類つきで並び、証拠の索引にはその 1 件も現れないこと。否定の側
//! だけでは空の実装でも真になるので、**両側とも非空**であることを同じテストで釘付け
//! する。⑵ 4 種それぞれが実物に似た見本から逐語で拾えること（設計「候補の種類」）。
//! ⑶ 並びが決定的であること（要件 7.3）——入力の並びにも本文に現れる順にも依らない。
//! ⑷ 拾わない場所（すでに URL のある定義・テストの本文・コメント行・ファイル名）。
//!
//! **ファイルも一時ディレクトリも作らない。環境変数もスナップショットも読まない**
//! （要件 6.2）。カタログは手で書いた本文から組み立てる。
//!
//! 見本の本文には正典 URL の綴りが並ぶが、走査は `crates/ukadoc-survey/` を除く
//! （設計 D-3・`io::sources`）ので、この見本が本物の証拠として読まれることはない。

use super::*;
use crate::catalog::Catalog;
use crate::catalog::read::read;
use crate::evidence::extract::extract;
use crate::evidence::resolve::resolve;
use crate::evidence::{EvidenceIndex, NameMatchFailure};

/// 見本のファイルパス（実物の許可表の置き場と同じ綴り）。
const EVENTS_PATH: &str = "crates/areka-kanade/src/schedule/events.rs";

/// 4 種すべてを 1 本に詰めた見本。実物の綴りをなぞってある。
///
/// - 目印付きの語彙表（`MARKED_IDS`）＝**証拠**になる側。
/// - 目印の無い許可表（`ALLOWED_EVENT_IDS`）＝**候補**になる側。
/// - 要素が名前付き定数だけの表（`HOLD_FIELDS`）＝どちらにもならない。
/// - 設定キーの定数とファイル名の定数、`.get(…)` の照会。
/// - `\![move]` の消費側の登録と、その定義（実引数に文字列を持たない）。
/// - 語を含むログ行と、含まないログ行。
const FIXTURE: &str = r##"//! 見本の本文。

/// 語彙表の目印（設計 D-5）。
/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html
pub const MARKED_IDS: &[&str] = &["version", "log_path"];

/// まだ正典 URL の無い許可表。
pub const ALLOWED_EVENT_IDS: &[&str] = &["OnBoot", "OnClose"];

/// 要素が名前付き定数だけの表。
pub const HOLD_FIELDS: &[&str] = &[FIELD_ENTITY, FIELD_SCOPE];

const SHELL_DPI_KEY: &str = "seriko.dpi";
const DESCRIPT_FILE: &str = "descript.txt";

pub fn try_register(&mut self, name: &str, selector: Option<&str>) -> Result<(), Error> {
    self.table.insert(name.to_string(), selector);
    Ok(())
}

/// `\![move]` の消費側を配線する。
pub fn wire(ledger: &mut ConsumerLedger, shell_kv: &Kv) {
    ledger.try_register("move", None, CommandConsumer::MoveSink);
    let zorder = shell_kv.get("seriko.zorder").cloned();
    debug!(?zorder, "窓寸法が不明（非正）のため identity 縮退");
    info!("正常に配置した");
}
"##;

/// 見本のカタログ。`MARKED_IDS` の 2 要素に対応する見出しを持つ。
const CATALOG: &str = r##"# 見本
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
"ukadoc:list_shiori_resource:log_path:1" = { page = "list_shiori_resource", title = "log_path", category = "protocol", versions = [], hash = "0000000000000003", url = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#log_path:1" }
"ukadoc:list_shiori_resource:version:1" = { page = "list_shiori_resource", title = "version", category = "protocol", versions = [], hash = "0000000000000006", url = "https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#version:1" }
"##;

/// ソースの本文の組。
fn sources(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(path, text)| ((*path).to_owned(), (*text).to_owned()))
        .collect()
}

/// 見本のカタログを組み立てる。組み立てられなければテストを落とす。
fn catalog() -> Catalog {
    match read(CATALOG) {
        Ok(catalog) => catalog,
        Err(err) => panic!("見本のカタログは読めるはず: {err}"),
    }
}

/// 同じ本文から証拠の索引を作る。
fn index(list: &[(&str, &str)]) -> EvidenceIndex {
    let sources = sources(list);
    let mut hits = Vec::new();
    for (path, text) in &sources {
        hits.extend(extract(path, text));
    }
    resolve(&hits, &sources, &catalog())
}

/// 候補を「(種類, 文字列)」の並びへ畳む（パスは別に確かめる）。
fn kinds_and_texts(found: &[Candidate]) -> Vec<(CandidateKind, &str)> {
    found
        .iter()
        .map(|candidate| (candidate.kind, candidate.text.as_str()))
        .collect()
}

/// 索引に現れる文字列をすべて集める（証拠に混ざっていないことを見るため）。
fn strings_in_index(index: &EvidenceIndex) -> Vec<String> {
    let mut out = Vec::new();
    for (id, paths) in &index.by_id {
        out.push(id.as_str().to_owned());
        out.extend(paths.iter().cloned());
    }
    for unresolved in &index.unresolved {
        out.push(unresolved.path.clone());
        out.push(unresolved.url.clone());
    }
    for unmatched in &index.unmatched_names {
        out.push(unmatched.path.clone());
        out.push(unmatched.page_url.clone());
        match &unmatched.reason {
            NameMatchFailure::NoMatch(name) | NameMatchFailure::Ambiguous(name) => {
                out.push(name.clone());
            }
            NameMatchFailure::TableMissing => {}
        }
    }
    out
}

/// 課題の完了条件（要件 5.8・5.9）。見本 1 本から候補が種類つきで並び、**同じ本文から
/// 作った証拠の索引には 1 件も混ざらない**。
///
/// 否定の側（混ざらない）は対象が空でも真になるので、⑴ 候補が非空でその中身が逐語で
/// 定まること ⑵ 証拠の索引も非空であること を同じテストで釘付けする。
#[test]
fn 候補は種類つきで並び同じ本文の証拠には混ざらない() {
    let list = [(EVENTS_PATH, FIXTURE)];
    let found = candidates(&sources(&list));

    // ⑴ 候補は非空で、中身は逐語で定まる。
    assert_eq!(
        kinds_and_texts(&found),
        vec![
            (CandidateKind::AllowListElement, "OnBoot"),
            (CandidateKind::AllowListElement, "OnClose"),
            (CandidateKind::BangCommandConsumer, "move"),
            (CandidateKind::ConfigKey, "seriko.dpi"),
            (CandidateKind::ConfigKey, "seriko.zorder"),
            (
                CandidateKind::LogLine,
                "窓寸法が不明（非正）のため identity 縮退"
            ),
        ],
        "見本 1 本から 4 種の候補が種類つきで並ぶ"
    );
    for candidate in &found {
        assert_eq!(candidate.path, EVENTS_PATH, "候補はファイルパスを持つ");
    }

    // ⑵ 同じ本文の証拠も非空（目印付きの表が 2 項目の証拠になる）。
    let index = index(&list);
    assert_eq!(
        index.by_id.len(),
        2,
        "同じ本文から証拠も 2 件立つ（否定の側が空虚にならないこと）"
    );

    // ⑶ 候補は証拠の索引のどこにも現れない。
    let in_index = strings_in_index(&index);
    for candidate in &found {
        assert!(
            !in_index.contains(&candidate.text),
            "候補 {:?} が証拠の索引に混ざっている",
            candidate.text
        );
    }

    // ⑷ 逆向き——証拠になった要素名は候補に現れない。
    let texts: Vec<&str> = found.iter().map(|c| c.text.as_str()).collect();
    for name in ["version", "log_path"] {
        assert!(
            !texts.contains(&name),
            "証拠になった要素名 {name} が候補にも現れている"
        );
    }
}

/// すでに正典 URL のある表は候補にならず、無い表は候補になる（要件 5.8「まだ置かれて
/// いない既存コード」）。同じ 1 本の中で対にして確かめる。
#[test]
fn 目印のある表は候補にならず無い表は候補になる() {
    let text = r##"/// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html
pub const MARKED: &[&str] = &["version"];

/// 目印の無い表。
pub const PLAIN: &[&str] = &["log_path"];
"##;
    let found = candidates(&sources(&[(EVENTS_PATH, text)]));
    assert_eq!(
        kinds_and_texts(&found),
        vec![(CandidateKind::AllowListElement, "log_path")],
        "目印の無い表だけが候補になる"
    );
}

/// 並びはパス → 種類 → 文字列の昇順。入力の並びにも本文に現れる順にも依らない
/// （要件 7.3）。**本文の中の自然な順と出力の順が違う**見本で確かめる。
#[test]
fn 並びは決定的でパスと種類と文字列の昇順() {
    let later = r##"pub const T: &[&str] = &["yankee", "bravo"];
"##;
    let earlier = r##"pub const T: &[&str] = &["zeta", "alpha"];
debug!("未知の綴りは無視");
"##;
    // 入力はパスの降順、表の要素も降順に並べてある。
    let found = candidates(&sources(&[
        ("crates/zz/src/later.rs", later),
        ("crates/aa/src/earlier.rs", earlier),
    ]));
    let seen: Vec<(&str, CandidateKind, &str)> = found
        .iter()
        .map(|c| (c.path.as_str(), c.kind, c.text.as_str()))
        .collect();
    assert_eq!(
        seen,
        vec![
            (
                "crates/aa/src/earlier.rs",
                CandidateKind::AllowListElement,
                "alpha"
            ),
            (
                "crates/aa/src/earlier.rs",
                CandidateKind::AllowListElement,
                "zeta"
            ),
            (
                "crates/aa/src/earlier.rs",
                CandidateKind::LogLine,
                "未知の綴りは無視"
            ),
            (
                "crates/zz/src/later.rs",
                CandidateKind::AllowListElement,
                "bravo"
            ),
            (
                "crates/zz/src/later.rs",
                CandidateKind::AllowListElement,
                "yankee"
            ),
        ],
        "パス → 種類 → 文字列の昇順に並ぶ"
    );
}

/// 4 つの形を持たない本文からは 1 件も返らない。空虚にならないよう、同じテストで
/// 非空になる本文も確かめる。
#[test]
fn 形の無い本文からは何も返らず形のある本文からは返る() {
    let plain = r##"pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
"##;
    assert_eq!(
        candidates(&sources(&[(EVENTS_PATH, plain)])),
        Vec::new(),
        "4 つの形を持たない本文からは 1 件も返らない"
    );

    let shaped = r##"pub const ALLOWED_EVENT_IDS: &[&str] = &["OnBoot"];
"##;
    assert_eq!(
        kinds_and_texts(&candidates(&sources(&[(EVENTS_PATH, shaped)]))),
        vec![(CandidateKind::AllowListElement, "OnBoot")],
        "形のある本文からは返る（否定の側が空虚でないこと）"
    );
}

/// テストの本文は拾わない。`#[cfg(test)] mod … { … }` の中とテストのファイルの 2 形を
/// 同じ本文で対にして確かめる。
#[test]
fn テストの本文は拾わない() {
    let text = r##"pub const ALLOWED_EVENT_IDS: &[&str] = &["OnBoot"];

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_IDS: &[&str] = &["OnTestOnly"];

    #[test]
    fn t() {
        let ledger = ConsumerLedger::new();
        ledger.try_register("resize", None, CommandConsumer::Seriko);
    }
}
"##;
    assert_eq!(
        kinds_and_texts(&candidates(&sources(&[(EVENTS_PATH, text)]))),
        vec![(CandidateKind::AllowListElement, "OnBoot")],
        "`#[cfg(test)] mod` の中は拾わない"
    );
    assert_eq!(
        candidates(&sources(&[(
            "crates/areka/src/schedule/events_tests.rs",
            text
        )])),
        Vec::new(),
        "テストのファイルは 1 件も拾わない"
    );
}

/// コメント行の中の綴りは拾わない。実物の doc コメントには `debug!("…")` の綴りが
/// 書かれている（`crates/areka/src/input_events/balloon.rs`）。
#[test]
fn コメント行の中のログの綴りは拾わない() {
    let text = r##"/// **正常縮退**＝`debug!("恒等へ縮退")`＋no-op。
pub fn follow() {
    debug!("空 snapshot のため identity 縮退");
}
"##;
    assert_eq!(
        kinds_and_texts(&candidates(&sources(&[(EVENTS_PATH, text)]))),
        vec![(CandidateKind::LogLine, "空 snapshot のため identity 縮退")],
        "コメント行の綴りは拾わず、実際の呼び出しだけを拾う"
    );
}

/// ログ行は[語](LOG_WORDS)を含むものだけ。含まない行と対にして確かめる。
///
/// 構造化フィールドの後ろに本文が来る形（`debug!(?size, "…")`）と、`target` が先に
/// 来る形（`warn!(target: "…", "…")`）の**どちらでも本文の側**を拾う。
#[test]
fn ログ行は語を含む文字列だけを拾う() {
    let text = r##"pub fn f() {
    info!("正常に配置した");
    debug!(event = "choice_pressed_no_emo2");
    warn!(target: "areka::emo", "未知の合成メソッド名: Unknown シームへ吸収");
}
"##;
    assert_eq!(
        kinds_and_texts(&candidates(&sources(&[(EVENTS_PATH, text)]))),
        vec![(
            CandidateKind::LogLine,
            "未知の合成メソッド名: Unknown シームへ吸収"
        )],
        "語を含む本文だけを拾い、target の綴りは拾わない"
    );
}

/// 設定キーはファイル名と区別する。実物で最も多い綴りが `descript.txt`・`*.png` で、
/// 区別しないと ⑶ はファイル名の一覧になる。
#[test]
fn 設定キーはファイル名と区別する() {
    let text = r##"const SHELL_DPI_KEY: &str = "seriko.dpi";
const DESCRIPT_FILE: &str = "descript.txt";
const BASE_IMAGE: &str = "surface0.png";
const NOT_DOTTED: &str = "username";
"##;
    assert_eq!(
        kinds_and_texts(&candidates(&sources(&[(EVENTS_PATH, text)]))),
        vec![(CandidateKind::ConfigKey, "seriko.dpi")],
        "点付きの設定キーだけを拾い、ファイル名と点無しの綴りは拾わない"
    );
}

/// 表からも登録からも**最初の文字列だけ**を拾う（設計 D-5 と同じ規則）。
///
/// 要素が組になっている表（実物の `&[(&str, SetSemantics)]`）では 2 番目の文字列が
/// 名前ではなく注記であり、登録では 2 番目の文字列が選別子である。どちらも拾うと
/// 候補の一覧に名前でないものが混ざる。
#[test]
fn 表と登録からは最初の文字列だけを拾う() {
    let text = r##"pub const VOCAB: &[(&str, &str)] = &[
    ("OnBoot", "起動"),
    ("OnClose", "終了"),
];

pub fn wire(ledger: &mut ConsumerLedger) {
    ledger.try_register("set", Some("zorder"), CommandConsumer::ZOrderSink);
}
"##;
    assert_eq!(
        kinds_and_texts(&candidates(&sources(&[(EVENTS_PATH, text)]))),
        vec![
            (CandidateKind::AllowListElement, "OnBoot"),
            (CandidateKind::AllowListElement, "OnClose"),
            (CandidateKind::BangCommandConsumer, "set"),
        ],
        "要素の 2 番目の文字列（注記）も登録の選別子も拾わない"
    );
}

/// 同じ場所の同じ手掛かりは 1 件に畳む。
#[test]
fn 同じ組は一件に畳む() {
    let text = r##"pub fn wire(ledger: &mut ConsumerLedger) {
    ledger.try_register("move", None, CommandConsumer::MoveSink);
    ledger.try_register("move", None, CommandConsumer::MoveSink);
}
"##;
    assert_eq!(
        kinds_and_texts(&candidates(&sources(&[(EVENTS_PATH, text)]))),
        vec![(CandidateKind::BangCommandConsumer, "move")],
        "同じ (パス, 種類, 文字列) は 1 件"
    );
}
