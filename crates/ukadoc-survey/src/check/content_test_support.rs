//! `content.rs` のテーマ別テストが共有する道具（テスト専用）。
//!
//! 内容の検査は 6 種の判定を持ち、テストは 2 つのファイルに分かれている
//! （`content_tests.rs` が URL と証拠、`content_link_tests.rs` が関連・別名・版・
//! テーマ）。どちらも同じ形で主張するので、数え方と場所の綴りをここ 1 か所に集める
//! ——両方に写すと、片方だけが本文の変更に追随しなくなる。
//!
//! # 期待値は実装の定数を参照しない
//!
//! [`ledger_place`] とソース 3 本のパスは独立した文字列リテラルで書く。実装の定数を
//! 参照すると、表を表自身と比べるだけになって転記の誤りを 1 件も捕まえられない
//! （タスク 1.5 の教訓）。

use super::super::{Finding, FindingKind};
use crate::lib_test_support::World;
use crate::model::{Domain, EntryId};

/// 見本のソース 3 本のパス（実装の定数は参照しない）。
pub const EVENTS_PATH: &str = "crates/areka-kanade/src/schedule/events.rs";
pub const TAG_PATH: &str = "crates/areka-sakura/src/tag/surface.rs";
pub const VOCAB_PATH: &str = "crates/areka-sylphya/src/vocab/dotted.rs";

/// 見本の id を作る。
pub fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).expect("見本の id は要件 1.9 の 2 形のいずれかのはず")
}

/// 台帳のファイルパス（所見の「場所」に載る綴り）。実装の定数は参照しない。
pub fn ledger_place(domain: &str) -> String {
    format!("doc/ukadoc-coverage/ledger/{domain}.toml")
}

/// 出た所見を種類ごとに数える（0 件の種類は落とす）。
///
/// これを**等式**で主張すると、意図しない種類が 1 件でも出れば赤になる。件数だけの
/// 主張は中身が全部誤っていても緑になるので、この等式と id・場所・詳細の逐語の主張を
/// 必ず対で置く（タスク 1.5 の教訓）。
pub fn kinds(findings: &[Finding]) -> Vec<(FindingKind, usize)> {
    FindingKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = findings.iter().filter(|f| f.kind == kind).count();
            (count > 0).then_some((kind, count))
        })
        .collect()
}

/// その種類の所見だけを出た順に取り出す。
pub fn of_kind(findings: &[Finding], kind: FindingKind) -> Vec<&Finding> {
    findings.iter().filter(|f| f.kind == kind).collect()
}

/// その種類の所見がちょうど 1 件あることを確かめ、その 1 件を返す。
pub fn only_one(findings: &[Finding], kind: FindingKind) -> &Finding {
    let found = of_kind(findings, kind);
    assert_eq!(
        found.len(),
        1,
        "{} が 1 件でない: {:?}",
        kind.as_key(),
        found
    );
    found[0]
}

/// その種類の所見の詳細を出た順に並べる。
pub fn details(findings: &[Finding], kind: FindingKind) -> Vec<&str> {
    of_kind(findings, kind)
        .into_iter()
        .map(|finding| finding.detail.as_str())
        .collect()
}

/// その種類の所見の id を出た順に並べる。
pub fn ids(findings: &[Finding], kind: FindingKind) -> Vec<&str> {
    of_kind(findings, kind)
        .into_iter()
        .map(|finding| {
            finding
                .id
                .as_ref()
                .map(EntryId::as_str)
                .expect("項目についての所見は id を持つ")
        })
        .collect()
}

/// その種類の所見の場所を出た順に並べる。
pub fn places(findings: &[Finding], kind: FindingKind) -> Vec<&str> {
    of_kind(findings, kind)
        .into_iter()
        .map(|finding| finding.place.as_str())
        .collect()
}

/// 台帳の 1 項目を書き換えるために借りる。
pub fn entry_mut<'a>(
    world: &'a mut World,
    domain: Domain,
    raw: &str,
) -> &'a mut crate::ledger::LedgerEntry {
    world
        .ledger_mut(domain)
        .entries
        .get_mut(&id(raw))
        .expect("見本の台帳にその id が無い")
}

/// ソース文の一部をすり替え、証拠の索引を作り直す。
///
/// すり替えが**現に効いた**ことを先に確かめる。綴りを写し間違えたテストは、何も
/// 壊さないまま「壊したつもり」で緑になる。
pub fn replace_in_source(world: &mut World, path: &str, from: &str, to: &str) {
    let before = world.source_mut(path).clone();
    assert!(before.contains(from), "見本のソース文に {from} が無い");
    let after = before.replace(from, to);
    assert_ne!(before, after, "すり替えが効いていない");
    *world.source_mut(path) = after;
    world.refresh_evidence();
}
