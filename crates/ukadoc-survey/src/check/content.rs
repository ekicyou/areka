//! 内容の検査（要件 6.5・6.6・6.7・6.8・6.10・6.11）。
//!
//! 台帳の中身と、ソースに置かれた正典 URL を判定する。設計 check 節の「判定の内訳」
//! から、この段が受け持つのは次の 6 種である。
//!
//! - `SourceUrlNotInCatalog`（6.5・6.10）——ソースの URL がカタログに無い
//! - `ImplementedWithoutEvidence`（6.6）——実装済みなのに証拠が 1 件も無い
//! - `LinkEndpointMissing` / `AliasChain`（6.7・2.4）——関連の相手と別名の連鎖
//! - `IntroducedNotInCatalogVersions`（6.7）——登場版がカタログの版番号と矛盾
//! - `UnknownTheme`（6.8）——テーマ名がテーマ定義に無い
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（要件 6.2）。失敗も
//! しない——食い違いは [`Finding`] として全部集めて返し、1 件目で止めない（設計
//! Error Handling）。手掛かり候補（要件 5.8）は見ない。候補は証拠ではない（要件 5.9）。
//!
//! # 行番号はどこにも出ない
//!
//! 所見の場所はファイルパスだけである（要件 5.1・6.11）。証拠が行番号を持たないのと
//! 同じ理由で、整理や作り替えで行が上下しても同じ食い違いは同じ本文になる。
//!
//! # ソースの本文はもう見ない
//!
//! `SourceUrlNotInCatalog` は [`EvidenceIndex::unresolved`] をそのまま所見に変える。
//! カタログの項目 URL とページ URL の 2 段の突き合わせ（設計 D-4 の 1・2 段目）は
//! すでに `evidence::resolve` が済ませてあり、`unresolved` に残っているのは
//! 「どちらでもなかった」もの——つまり D-4 の 3 段目そのものである。[`CheckInput`] は
//! ソースの本文を持たないので、ここで走査をやり直すことはできないし、やり直せば
//! 2 か所で同じ規則を持つことになる。
//!
//! # 証拠のファイルパスの一覧はここでは作らない
//!
//! 要件 2.3・5.5 は「証拠は台帳に書かず、検査の出力に id ごとのファイルパスとして
//! 列挙する」と言う。その列挙を作るのは入口の実行ファイル（タスク 6.3。「所見の本文と、
//! id ごとの証拠のファイルパスを並べて出す」）であって、この判定ではない。ここが作る
//! のは食い違いだけで、証拠のある項目については所見を 1 件も出さない——出すと
//! 「所見が空なら緑」（設計 check 節の事後条件）が成り立たなくなる。
//!
//! # 別名の指す先は 4 本の台帳ぜんぶから引く
//!
//! `AliasChain` は指す先の**状態**を要る。状態は相手の載っている台帳にあり、それが
//! 同じ台帳とは限らない（旧名と正典名がドメインをまたぐことはある）。だから
//! [`CheckInput::ledgers`] の全部を見て「どれか 1 本でも `alias` として持っていれば
//! 連鎖」と判じる。「どれか」で判じるので渡された台帳の並びに結果は左右されない
//! （要件 7.3）。

use super::{CheckInput, Finding, FindingKind};
use crate::evidence::EvidenceIndex;
use crate::ledger::{Ledger, LedgerEntry};
use crate::model::{Domain, EntryId, Status};

/// その台帳の場所（所見の「場所」に載る綴り）。
fn ledger_file(domain: Domain) -> String {
    format!("doc/ukadoc-coverage/ledger/{}.toml", domain.as_key())
}

/// 内容の食い違いを集める。
///
/// 並びは決まっている——まずソースの URL を（`unresolved` の並びのまま）、続いて
/// 台帳を渡された順に見て、1 本ごとに項目を id の byte 昇順で、1 項目ごとに
/// 証拠 → 関連 → 別名の連鎖 → 登場版 → テーマの順に見る。同じ入力なら同じ並びで
/// 返る（要件 7.3 の決まり方を検査の出力にも通す）。
pub fn check(input: &CheckInput) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_source_urls(input.evidence, &mut findings);
    for ledger in input.ledgers {
        for entry in ledger.entries.values() {
            check_evidence(input, ledger, entry, &mut findings);
            check_link_endpoints(input, ledger, entry, &mut findings);
            check_alias_chain(input, ledger, entry, &mut findings);
            check_introduced(input, ledger, entry, &mut findings);
            check_themes(input, ledger, entry, &mut findings);
        }
    }
    findings
}

/// ソースの正典 URL がカタログに実在するか（要件 6.5・6.10・設計 D-4 の 3 段目）。
///
/// 主語は項目ではなく URL なので id は付かない（付けようにも、解決できなかった URL に
/// 対応する id は無い）。場所は URL の書かれていたファイルである。同じ URL が複数の
/// ファイルに現れることは赤にしない（D-4）——それは `resolve` の側で証拠として畳まれ、
/// ここには来ない。
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

/// 実装済みの項目に証拠が 1 件でもあるか（要件 6.6）。
///
/// 実装済み以外は見ない。未実装の項目についてはソース側に何も書かせないので
/// （要件 5.7）、証拠が無いことはそれ自体正しい状態である。
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

/// 関連の相手・別名の参照先・後継の参照先がカタログに実在するか（要件 6.7）。
///
/// 見る順は付録 A.2 の欄の並び（`alias_of` → `supersedes` → `links`）。**1 件目で
/// 止めない**——1 つの項目が無い相手を 2 つ持つことはあり、片方だけ直しても残りが
/// 見えないと直す人は 2 度往復することになる。
///
/// 主語は相手ではなく、その関連を書いた側の項目である。直すべき行がそこにあるから
/// で、相手の id は本文に綴りとして載せる（要件 6.12）。
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

/// 別名の指す先が別名でないか（要件 6.7・2.4）。
///
/// 別名の行が持つのは「正典側の id への写像があるか否か」だけで、実装状態の判定は
/// 写像先の正典行に委ねる（要件 2.4）。写像先がまた別名だと、その委譲の先が正典行に
/// 辿り着かない。
///
/// **1 段だけを見る**。連鎖を何段も辿ると、輪になった別名で止まらなくなるうえ、
/// 直す人に「どこを直せばよいか」を 1 段目より的確に示せない。指す先が別名なら
/// その先も別名の行として独立に判定される。
///
/// 相手がどの台帳にも無い場合は何も言わない——その申し立ては
/// [`FindingKind::LinkEndpointMissing`] が受け持つ（状態を引けない相手を「別名だ」と
/// 決めつけない）。
fn check_alias_chain(
    input: &CheckInput,
    ledger: &Ledger,
    entry: &LedgerEntry,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &entry.alias_of else {
        return;
    };
    // 「どれか 1 本でも」で判じるので、渡された台帳の並びに結果は左右されない
    // （要件 7.3）。
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

/// 台帳の登場版がカタログの版番号と矛盾しないか（要件 6.7）。
///
/// 番人が 2 つある（設計の判定表）。どちらも「まだ判らない」を赤にしないためのもの
/// である。
///
/// - **カタログ側の版番号が空なら見ない**。正典の本文に版番号が 1 つも現れない項目は
///   実測で多数あり、そこに書かれた登場版を否定する根拠はカタログに無い。
/// - **台帳の登場版が空なら見ない**。空文字は「世代不明」であって最古ではない
///   （要件 4.2）。まだ版を調べていない行を赤にすると、調べる前に嘘を書く力が働く。
///
/// カタログに id が無い場合も見ない——その申し立ては
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

/// テーマ名がテーマ定義に実在するか（要件 6.8）。
///
/// 突き合わせは**完全一致**である。8 つのうち「気配」と「気配り」は片方が他方の
/// 接頭辞なので、部分一致で拾うと別のテーマを取り違える。
///
/// 照らす相手は [`CheckInput::themes`] であって定数ではない。定義そのものが凍結の
/// 対象（要件 4.4・2.6）で、その正本は `doc/ukadoc-coverage/values.md` の側にある。
///
/// **1 件目で止めない**。1 つの項目が 0 個以上のテーマを持つので、綴り違いが 2 つ
/// 並ぶことはある。
fn check_themes(
    input: &CheckInput,
    ledger: &Ledger,
    entry: &LedgerEntry,
    findings: &mut Vec<Finding>,
) {
    for value in &entry.values {
        if input.themes.contains(&value.as_str()) {
            continue;
        }
        findings.push(Finding::new(
            FindingKind::UnknownTheme,
            Some(entry.id.clone()),
            ledger_file(ledger.domain),
            format!("テーマ定義に無いテーマ名: {value}"),
        ));
    }
}

// 在中テストは 3 本に分かれている。1 本にまとめると 1,000 行を超えて要件 9.6 に
// 反するので、判定の主題で割った（URL と証拠／関連・別名・版・テーマ）。数え方と
// 場所の綴りは `content_test_support` に集めてある——両方に写すと、片方だけが
// 本文の変更に追随しなくなる。
#[cfg(test)]
#[path = "content_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "content_link_tests.rs"]
mod link_tests;
