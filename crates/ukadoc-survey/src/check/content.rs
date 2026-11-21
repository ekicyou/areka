//! 内容の検査。
//!
//! 台帳の中身と、ソースに置かれた正典 URL を判定する。この段が受け持つのは次の 4 種である。
//!
//! - `SourceUrlNotInCatalog`——ソースの URL がカタログに無い
//! - `ImplementedWithoutEvidence`——実装済みなのに証拠が 1 件も無い
//! - `LinkEndpointMissing` / `AliasChain`——関連の相手と別名の連鎖
//! - `IntroducedNotInCatalogVersions`——登場版がカタログの版番号と矛盾
//!
//! テーマ名はここでは見ない。綴り違いは `ledger::read` が読み込みの段で落とすので、
//! 検査に届く台帳はもう定義の外のテーマ名を持たない。
//!
//! 純粋層で、ファイルにもスナップショットにも触らない。失敗もしない。食い違いは
//! [`Finding`] として全部集めて返し、1 件目で止めない。手掛かり候補は見ない（候補は
//! 証拠ではない）。
//!
//! # 行番号はどこにも出ない
//!
//! 所見の場所はファイルパスだけである。証拠が行番号を持たないのと同じ理由で、整理や
//! 作り替えで行が上下しても同じ食い違いは同じ本文になる。
//!
//! # ソースの本文はもう見ない
//!
//! `SourceUrlNotInCatalog` は [`EvidenceIndex::unresolved`] をそのまま所見に変える。
//! カタログの項目 URL とページ URL の 2 段の突き合わせはすでに `evidence::resolve` が
//! 済ませてあり、`unresolved` に残っているのは「どちらでもなかった」ものである。
//! [`CheckInput`] はソースの本文を持たないので、ここで走査をやり直すことはできないし、
//! やり直せば 2 か所で同じ規則を持つことになる。
//!
//! # 証拠のファイルパスの一覧はここでは作らない
//!
//! 証拠は台帳に書かず、検査の出力に id ごとのファイルパスとして列挙する。その列挙を
//! 作るのは入口の実行ファイル（所見の本文と id ごとの証拠のファイルパスを並べて出す）で
//! あって、この判定ではない。ここが作るのは食い違いだけで、証拠のある項目については
//! 所見を 1 件も出さない。出すと「所見が空なら緑」が成り立たなくなる。
//!
//! # 別名の指す先は 4 本の台帳ぜんぶから引く
//!
//! `AliasChain` は指す先の**状態**を要る。状態は相手の載っている台帳にあり、それが
//! 同じ台帳とは限らない（旧名と正典名がドメインをまたぐことはある）。だから
//! [`CheckInput::ledgers`] の全部を見て「どれか 1 本でも `alias` として持っていれば
//! 連鎖」と判じる。「どれか」で判じるので渡された台帳の並びに結果は左右されない。

use super::{CheckInput, Finding, FindingKind};
use crate::evidence::EvidenceIndex;
use crate::ledger::{Ledger, LedgerEntry, ledger_file};
use crate::model::{EntryId, Status};

/// 内容の食い違いを集める。
///
/// 並びは決まっている。まずソースの URL を（`unresolved` の並びのまま）、続いて台帳を
/// 渡された順に見て、1 本ごとに項目を id の byte 昇順で、1 項目ごとに
/// 証拠 → 関連 → 別名の連鎖 → 登場版の順に見る。同じ入力なら同じ並びで返る。
pub fn check(input: &CheckInput) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_source_urls(input.evidence, &mut findings);
    for ledger in input.ledgers {
        for entry in ledger.entries.values() {
            check_evidence(input, ledger, entry, &mut findings);
            check_link_endpoints(input, ledger, entry, &mut findings);
            check_alias_chain(input, ledger, entry, &mut findings);
            check_introduced(input, ledger, entry, &mut findings);
        }
    }
    findings
}

/// ソースの正典 URL がカタログに実在するか。
///
/// 主語は項目ではなく URL なので id は付かない（解決できなかった URL に対応する id は
/// 無い）。場所は URL の書かれていたファイルである。同じ URL が複数のファイルに現れる
/// ことは赤にしない。それは `resolve` の側で証拠として畳まれ、ここには来ない。
fn check_source_urls(evidence: &EvidenceIndex, findings: &mut Vec<Finding>) {
    for unresolved in &evidence.unresolved {
        findings.push(Finding::new(
            FindingKind::SourceUrlNotInCatalog,
            None,
            unresolved.path.clone(),
            format!("カタログに無い正典 URL: {}", unresolved.url),
        ));
    }
}

/// 実装済みの項目に証拠が 1 件でもあるか。
///
/// 実装済み以外は見ない。未実装の項目についてはソース側に何も書かせないので、
/// 証拠が無いことはそれ自体正しい状態である。
fn check_evidence(
    input: &CheckInput,
    ledger: &Ledger,
    entry: &LedgerEntry,
    findings: &mut Vec<Finding>,
) {
    if entry.status != Status::Implemented {
        return;
    }
    let has_evidence = match input.evidence.by_id.get(&entry.id) {
        Some(paths) => !paths.is_empty(),
        None => false,
    };
    if has_evidence {
        return;
    }
    findings.push(Finding::new(
        FindingKind::ImplementedWithoutEvidence,
        Some(entry.id.clone()),
        ledger_file(ledger.domain),
        "正典 URL がソースに 1 件も無い",
    ));
}

/// 関連の相手・別名の参照先・後継の参照先がカタログに実在するか。
///
/// 見る順は欄の並び（`alias_of` → `supersedes` → `links`）。**1 件目で止めない**。
/// 1 つの項目が無い相手を 2 つ持つことはあり、片方だけ直しても残りが見えないと直す人は
/// 2 度往復することになる。
///
/// 主語は相手ではなく、その関連を書いた側の項目である。直すべき行がそこにあるから
/// で、相手の id は本文に綴りとして載せる。
fn check_link_endpoints(
    input: &CheckInput,
    ledger: &Ledger,
    entry: &LedgerEntry,
    findings: &mut Vec<Finding>,
) {
    let mut endpoints: Vec<(String, &EntryId)> = Vec::new();
    if let Some(target) = &entry.alias_of {
        endpoints.push(("alias_of".to_owned(), target));
    }
    for target in &entry.supersedes {
        endpoints.push(("supersedes".to_owned(), target));
    }
    for link in &entry.links {
        endpoints.push((format!("関連 {}", link.kind.as_key()), &link.to));
    }

    for (field, target) in endpoints {
        if input.catalog.entries.contains_key(target) {
            continue;
        }
        findings.push(Finding::new(
            FindingKind::LinkEndpointMissing,
            Some(entry.id.clone()),
            ledger_file(ledger.domain),
            format!("{field} の相手がカタログに無い: {}", target.as_str()),
        ));
    }
}

/// 別名の指す先が別名でないか。
///
/// 別名の行が持つのは「正典側の id への写像があるか否か」だけで、実装状態の判定は
/// 写像先の正典行に委ねる。写像先がまた別名だと、その委譲の先が正典行に
/// 辿り着かない。
///
/// **1 段だけを見る**。連鎖を何段も辿ると、輪になった別名で止まらなくなるうえ、
/// 直す人に「どこを直せばよいか」を 1 段目より的確に示せない。指す先が別名なら
/// その先も別名の行として独立に判定される。
///
/// 相手がどの台帳にも無い場合は何も言わない（状態を引けない相手を「別名だ」と
/// 決めつけない）。カタログに無ければ [`FindingKind::LinkEndpointMissing`] が、
/// カタログにあってどの台帳にも無ければ構造の検査の
/// [`FindingKind::CatalogIdMissingFromLedgers`] が申し立てる。
fn check_alias_chain(
    input: &CheckInput,
    ledger: &Ledger,
    entry: &LedgerEntry,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &entry.alias_of else {
        return;
    };
    // 「どれか 1 本でも」で判じるので、渡された台帳の並びに結果は左右されない。
    let points_at_alias = input
        .ledgers
        .iter()
        .filter_map(|held| held.entries.get(target))
        .any(|found| found.status == Status::Alias);
    if !points_at_alias {
        return;
    }
    findings.push(Finding::new(
        FindingKind::AliasChain,
        Some(entry.id.clone()),
        ledger_file(ledger.domain),
        format!("alias_of の指す先 {} も別名である", target.as_str()),
    ));
}

/// 台帳の登場版がカタログの版番号と矛盾しないか。
///
/// 番人が 2 つある。どちらも「まだ判らない」を赤にしないためのものである。
///
/// - **カタログ側の版番号が空なら見ない**。正典の本文に版番号が 1 つも現れない項目は
///   現に多数あり、そこに書かれた登場版を否定する根拠はカタログに無い。
/// - **台帳の登場版が空なら見ない**。空文字は「世代不明」であって最古ではない。
///   まだ版を調べていない行を赤にすると、調べる前に嘘を書く力が働く。
///
/// カタログに id が無い場合も見ない。その申し立ては
/// [`FindingKind::LedgerIdNotInCatalog`] が受け持つ（`check::structure`）。
fn check_introduced(
    input: &CheckInput,
    ledger: &Ledger,
    entry: &LedgerEntry,
    findings: &mut Vec<Finding>,
) {
    if entry.introduced.is_empty() {
        return;
    }
    let Some(canonical) = input.catalog.entries.get(&entry.id) else {
        return;
    };
    if canonical.versions.is_empty() {
        return;
    }
    if canonical
        .versions
        .iter()
        .any(|version| version == &entry.introduced)
    {
        return;
    }
    findings.push(Finding::new(
        FindingKind::IntroducedNotInCatalogVersions,
        Some(entry.id.clone()),
        ledger_file(ledger.domain),
        format!(
            "登場版 {} がカタログの版番号に無い: {}",
            entry.introduced,
            canonical.versions.join("・")
        ),
    ));
}

// 在中テストは判定の主題で 2 本に分けてある（URL と証拠／関連・別名・版）。数え方と場所の
// 綴りは `check::test_support` に集めてある。
#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "content_link_tests.rs"]
mod link_tests;
