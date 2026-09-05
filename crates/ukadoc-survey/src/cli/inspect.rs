//! 調べる副手続き（`check`・`evidence`・`candidates`・`diff`）。
//!
//! 生成側（[`super::generate`]）と違い、ここはファイルを 1 つも書かない。読んで、
//! 純粋層へ渡し、結果を標準出力へ並べるだけである（設計「入口 / cli」の表）。
//!
//! # 標準出力に出るのは結果だけ
//!
//! 4 つとも**結果そのもの**を標準出力へ出す（生成側は「どこへ書いたか」の 1 行だけを
//! 出す点が違う）。断りや失敗の本文は標準エラーで、出すのは呼び手の `main.rs` である
//! （タスク 6.1 の取り決め）。整合検査の本文を標準エラーへ出すと、結果を別のファイルへ
//! 流し込んだ利用者の手元に何も残らない。
//!
//! # 終了コードの決まり方
//!
//! - `check` — **所見が 0 件なら 0、1 件でもあれば 1**（[`verdict`]）。呼び手は
//!   終了コードだけで合否を判じられる。本文は標準出力にあるので、失敗の本文
//!   （標準エラー）は件数を告げる 1 行に留める。
//! - `evidence`・`candidates`・`diff` — **並べるだけで判じない**ので、読み書きに
//!   失敗しない限り 0 で終わる。証拠が 0 件でも候補が 0 件でも差分が空でも、それは
//!   「食い違い」ではない（要件 5.9 のとおり、判定は人手に委ねる）。
//!
//! # テーマ名の出どころ（要件 6.8）
//!
//! 整合検査へ渡すテーマ名は [`crate::model::THEMES`] を使う。テーマ定義の正本は
//! `doc/ukadoc-coverage/values.md`（要件 6.8）だが、**ここでその見出しを読まない**
//! のには理由が 2 つある。
//!
//! - ドメイン別報告は `report` の副手続きが `THEMES` で組み立てて書き出す
//!   （[`super::generate::report`]）。検査側だけが `values.md` を読むと、両者が
//!   食い違ったときに `DomainReportStale`（要件 7.4）が**別の理由で**赤くなる。
//!   突き合わせの両側は同じ出どころでなければならない。
//! - `values.md` の見出しと `THEMES` が順序まで一致することは、タスク 8.4 の
//!   `tests/consistency/values_md.rs` が守る（`model.rs` の注記）。定数はその写しで
//!   あって、別の出どころではない。
//!
//! `values.md` を直に読む形はタスク 8.1 が入出力層に読み取りを置いてから、そちらへ
//! 揃えるのが筋である（そのときは `report` 側も同じ値を通すこと）。

use std::collections::BTreeMap;

use crate::assignment::PageAssignment;
use crate::catalog::build::build as build_catalog;
use crate::catalog::read::read as read_catalog;
use crate::catalog::{Catalog, SnapshotMeta};
use crate::check::{CheckInput, Finding, render as render_findings, run as run_check};
use crate::diff::{CatalogDiff, diff as diff_catalogs};
use crate::error::SurveyError;
use crate::evidence::candidates::candidates as collect_candidates;
use crate::evidence::extract::extract;
use crate::evidence::resolve::resolve;
use crate::evidence::{Candidate, CandidateKind, EvidenceIndex, NameMatchFailure};
use crate::io::{files, paths, snapshot, sources};
use crate::ledger::Ledger;
use crate::ledger::read::read as read_ledger;
use crate::model::{Domain, EntryId, THEMES};

/// 台帳と正典とソースの食い違いを調べる（要件 5.5・6.3〜6.12・7.4）。
///
/// repo のカタログ・台帳 4 本・ソース・ドメイン別報告 4 本を読み、純粋層の判定へ
/// 渡す。スナップショットには触れない（設計「入口 / cli」の表・要件 6.2）。
///
/// 標準出力へ出すのは所見の本文と、id ごとの証拠のファイルパスである。終了の仕方は
/// [`verdict`] が所見の件数だけから決める。
pub fn check() -> Result<(), SurveyError> {
    let assignment = PageAssignment::canonical();
    let catalog = read_catalog(&files::read_normalized(&paths::catalog_path())?)?;
    let ledgers = load_ledgers()?;
    let evidence = collect_evidence(&catalog)?;

    let mut domain_reports: BTreeMap<Domain, String> = BTreeMap::new();
    for domain in Domain::ALL {
        let body = files::read_normalized(&paths::domain_report_path(domain))?;
        domain_reports.insert(domain, body);
    }

    let input = CheckInput {
        catalog: &catalog,
        ledgers: &ledgers,
        assignment: &assignment,
        themes: &THEMES,
        evidence: &evidence,
        domain_reports: &domain_reports,
    };
    let (body, findings) = examine(&input);
    print!("{body}");
    verdict(findings)
}

/// 読み込みの済んだ入力から、標準出力へ出す本文と所見の件数を作る。
///
/// `check` の本体からファイルを読む段だけを外した形である。**この形にしてあるのは
/// 檻のため**——`check` を丸ごと走らせるには repo の木（カタログ・台帳・報告）が
/// 要るので、`tests/cli_streams.rs` の決まりでは走らせられない。判定を呼ぶ・本文を
/// 組む・件数を数える、の 3 つをここへ集めれば、在中テストが手で組んだ入力で
/// 配線ごと確かめられる（部品を釘付けしても入口がその部品を呼んでいるかは別に
/// 守る必要がある・タスク 1.7 の教訓）。
///
/// 証拠の索引は [`CheckInput::evidence`] から引く。`check` が判定へ渡した索引と
/// 本文へ並べた索引が同じものであることが、こうすると構造で決まる。
pub(crate) fn examine(input: &CheckInput) -> (String, usize) {
    let outcome = run_check(input);
    (
        render_check(&outcome.findings, input.evidence),
        outcome.findings.len(),
    )
}

/// 項目ごとの証拠を並べる（要件 5.5）。
///
/// 検査は走らせない。読むのはカタログとソースだけで、台帳も報告も要らない。
pub fn evidence() -> Result<(), SurveyError> {
    let catalog = read_catalog(&files::read_normalized(&paths::catalog_path())?)?;
    let index = collect_evidence(&catalog)?;
    print!("{}", render_evidence(&index));
    Ok(())
}

/// 手掛かりの候補を並べる（要件 5.8・5.9）。
///
/// 読むのはソースだけである。カタログも台帳も要らないので、`doc/ukadoc-coverage/` が
/// まだ無い作業ツリーでも走る。
pub fn candidates() -> Result<(), SurveyError> {
    let found = collect_candidates(&sources::walk(&paths::workspace_root())?);
    print!("{}", render_candidates(&found));
    Ok(())
}

/// 今のカタログと新しいスナップショットの差を並べる（要件 8.1〜8.3）。
///
/// 4 つの副手続きの中でこれだけがスナップショットを要る（設計「入口 / cli」の表・D-7）。
/// 比べる前に 2 つのカタログが同じ形かを見て、違えば注意を先に出す（[`comparability_notice`]）。
pub fn diff() -> Result<(), SurveyError> {
    let current = read_catalog(&files::read_normalized(&paths::catalog_path())?)?;
    let source = snapshot::default_path()?;
    let doc = snapshot::load(&source)?;
    let next = build_catalog(&doc, &PageAssignment::canonical())?;
    let ledgers = load_ledgers()?;

    print!("{}", compare(&current, &next, &ledgers));
    Ok(())
}

/// 読み込みの済んだ 2 つのカタログと台帳から、標準出力へ出す本文を作る。
///
/// `diff` の本体から読む段だけを外した形で、[`examine`] と同じ役割である。**この形に
/// してあるのは檻のため**——`diff` を丸ごと走らせるにはスナップショットが要るので、
/// 常時走るテストからは走らせられない（要件 8.4）。比べる向き・注意を出すかどうか・
/// 版面の 3 つをここへ集めれば、在中テストが手で組んだカタログで配線ごと確かめられる
/// （部品を釘付けしても入口がその部品を呼んでいるかは別に守る必要がある・タスク 1.7
/// の教訓）。
///
/// 守っているのは 3 つ。
///
/// - `current` と `next` を**この向きで**渡すこと（取り違えると増減が裏返り、同じ
///   カタログを 2 度渡すと差分は永久に空になって、きれいに見えたまま間違う）
/// - 比べられる形かどうかを毎回見て、違えば注意を**捨てずに**本文へ載せること
/// - 注意を一覧より先に置くこと（[`render_diff`]）
pub(crate) fn compare(current: &Catalog, next: &Catalog, ledgers: &[Ledger]) -> String {
    let delta = diff_catalogs(current, next, ledgers);
    let notice = comparability_notice(&current.snapshot, &next.snapshot);
    render_diff(&delta, notice.as_deref())
}

/// 所見の件数から終了の仕方を決める（完了条件・設計「Error Handling / 見張り」）。
///
/// 0 件なら成功（終了コード 0）、1 件でもあれば失敗（終了コード 1）。**本文はここに
/// 載せない**——所見の本文は既に標準出力へ出ているので、標準エラーへ同じものを
/// 二度書くと読み手が同じ一覧を 2 度読むことになる。
pub(crate) fn verdict(findings: usize) -> Result<(), SurveyError> {
    if findings == 0 {
        Ok(())
    } else {
        Err(SurveyError::CheckFindings { count: findings })
    }
}

/// 整合検査の標準出力（所見の本文＋証拠の一覧）。
///
/// [`render_findings`] は所見が 0 件なら空の本文を返す（合否を本文の有無で判じる
/// 呼び手のため）。こちらは**人が読む出力**なので、0 件でもそう書く——何も出ないと
/// 「緑だった」と「走らなかった」を見分けられない。合否を決めるのは [`verdict`] で
/// あって本文の有無ではないので、ここで 1 行足しても判定は動かない。
pub(crate) fn render_check(findings: &[Finding], evidence: &EvidenceIndex) -> String {
    let mut out = String::new();
    let body = render_findings(findings);
    if body.is_empty() {
        out.push_str("食い違い 0 件\n");
    } else {
        out.push_str(&body);
    }
    out.push('\n');
    out.push_str(&render_evidence_by_id(&evidence.by_id));
    out
}

/// id ごとの証拠のファイルパス（要件 5.5・設計 D-4）。
///
/// 並びは `BTreeMap` の鍵の順＝id の byte 昇順、その中はファイルパスの名前順
/// （[`EvidenceIndex::by_id`] が既にそう並べている）。
pub(crate) fn render_evidence_by_id(by_id: &BTreeMap<EntryId, Vec<String>>) -> String {
    let mut out = format!("証拠のある項目 {} 件\n", by_id.len());
    for (id, files) in by_id {
        out.push_str(&format!("  {}\n", id.as_str()));
        for path in files {
            out.push_str(&format!("    {path}\n"));
        }
    }
    out
}

/// 証拠の報告（`evidence` の副手続きの標準出力）。
///
/// [`EvidenceIndex`] の 3 欄をそれぞれ別の塊にする。証拠は [`EvidenceIndex::by_id`]
/// だけで、残る 2 つは「証拠にできなかったもの」の置き場である（要件 5.9 のとおり
/// どちらへ寄せるかは人が決める）。手掛かりの候補（[`Candidate`]）はここに 1 件も
/// 入らない——あれは別の副手続きの持ち物である。
pub(crate) fn render_evidence(index: &EvidenceIndex) -> String {
    let mut out = render_evidence_by_id(&index.by_id);

    let mut unresolved: Vec<&crate::evidence::UnresolvedUrl> = index.unresolved.iter().collect();
    unresolved.sort();
    out.push('\n');
    out.push_str(&format!("解決できなかった URL {} 件\n", unresolved.len()));
    for hit in unresolved {
        out.push_str(&format!("  {}\n    {}\n", hit.path, hit.url));
    }

    let mut unmatched: Vec<&crate::evidence::UnmatchedName> =
        index.unmatched_names.iter().collect();
    unmatched.sort();
    out.push('\n');
    out.push_str(&format!("対応が付かなかった名前 {} 件\n", unmatched.len()));
    for name in unmatched {
        out.push_str(&format!(
            "  {}\n    {}  {}\n",
            name.path,
            name.page_url,
            reason_text(&name.reason)
        ));
    }
    out
}

/// 名前が証拠にならなかった理由の綴り（設計 D-5 の 3 つ）。
///
/// 既定の腕を置かない。理由を増やしたら綴りを書き足すまでコンパイルが通らない。
fn reason_text(reason: &NameMatchFailure) -> String {
    match reason {
        NameMatchFailure::NoMatch(name) => format!("同じ見出しが 1 つも無い: {name}"),
        NameMatchFailure::Ambiguous(name) => format!("同じ見出しが 2 つ以上ある: {name}"),
        NameMatchFailure::TableMissing => "ページ URL の後に表が続かない".to_owned(),
    }
}

/// 手掛かりの候補（要件 5.8）。**証拠ではない**（要件 5.9）ので、証拠の本文とは
/// 別の副手続きの、別の塊として出す。
///
/// 塊は種類ごとで、並びは [`CandidateKind`] の宣言順（`BTreeMap` の鍵の順）。
/// 種類の一覧を手書きの配列で持たないのは、種類を増やしたときに配列へ書き足し
/// 忘れると、その種類の候補が黙って落ちるからである。
pub(crate) fn render_candidates(found: &[Candidate]) -> String {
    let mut out = format!("手掛かりの候補 {} 件\n", found.len());
    if found.is_empty() {
        return out;
    }

    let mut by_kind: BTreeMap<CandidateKind, Vec<&Candidate>> = BTreeMap::new();
    for candidate in found {
        by_kind.entry(candidate.kind).or_default().push(candidate);
    }

    out.push('\n');
    for (kind, mut group) in by_kind {
        group.sort_by(|left, right| (&left.path, &left.text).cmp(&(&right.path, &right.text)));
        out.push_str(&format!("[{}] {} 件\n", kind_key(kind), group.len()));
        let mut written_path: Option<&str> = None;
        for candidate in group {
            if written_path != Some(candidate.path.as_str()) {
                out.push_str(&format!("  {}\n", candidate.path));
                written_path = Some(&candidate.path);
            }
            out.push_str(&format!("    {}\n", candidate.text));
        }
    }
    out
}

/// 手掛かりの種類の綴り（`FindingKind::as_key` と同じ流儀の英字）。
///
/// 既定の腕を置かない。種類を増やしたら綴りを書き足すまでコンパイルが通らない。
fn kind_key(kind: CandidateKind) -> &'static str {
    match kind {
        CandidateKind::AllowListElement => "AllowListElement",
        CandidateKind::BangCommandConsumer => "BangCommandConsumer",
        CandidateKind::ConfigKey => "ConfigKey",
        CandidateKind::LogLine => "LogLine",
    }
}

/// 2 つのカタログが比べられる形かを見る（設計 diff 節の申し送り）。
///
/// [`CatalogDiff`] は 4 つの一覧しか持たない形に凍結してあり、[`diff_catalogs`] は
/// 失敗もしない。だから「算法が違うので比べられない」を告げるのは**呼び出し側の
/// 役目**である（`diff.rs` の冒頭の注記）。告げずに一覧を出すと、本文が 1 文字も
/// 変わっていない項目までほぼ全件が「本文が変わった項目」に並び、読み手はそれを
/// 本物の改訂だと受け取る。
///
/// 見るのは 2 欄。本文ハッシュの算法（[`SnapshotMeta::hash_algorithm`]）と、
/// カタログの形の版（[`SnapshotMeta::catalog_format`]）である。どちらも揃っていれば
/// `None`。
pub(crate) fn comparability_notice(current: &SnapshotMeta, next: &SnapshotMeta) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    if current.hash_algorithm != next.hash_algorithm {
        lines.push(format!(
            "  本文ハッシュの算法: 現行 {} / 新しい方 {}",
            current.hash_algorithm, next.hash_algorithm
        ));
    }
    if current.catalog_format != next.catalog_format {
        lines.push(format!(
            "  カタログの形の版: 現行 {} / 新しい方 {}",
            current.catalog_format, next.catalog_format
        ));
    }
    if lines.is_empty() {
        return None;
    }
    Some(format!(
        "注意: 2 つのカタログは同じ形で作られていない。\n{}\n\
         「本文が変わった項目」はほぼ全件に挙がるので、そのまま改訂とは読まないこと。\n",
        lines.join("\n")
    ))
}

/// 差分の標準出力（要件 8.1・8.3）。
///
/// 4 つの一覧はいずれも件数の見出しを持ち、**0 件でも見出しを落とさない**。落とすと
/// 読み手は「無かった」と「見ていない」を見分けられない。
///
/// `notice` があれば一覧より**先**に出す（後ろに置くと、読む価値の無い一覧を全部
/// 読んでからそれと知ることになる）。
pub(crate) fn render_diff(delta: &CatalogDiff, notice: Option<&str>) -> String {
    let mut out = String::new();
    if let Some(body) = notice {
        out.push_str(body);
        out.push('\n');
    }
    for (heading, ids) in [
        ("増えた項目", &delta.added),
        ("消えた項目", &delta.removed),
        ("本文が変わった項目", &delta.changed),
        ("台帳の見直しが要る項目", &delta.removed_in_ledger),
    ] {
        out.push_str(&format!("{heading} {} 件\n", ids.len()));
        for id in ids {
            out.push_str(&format!("  {}\n", id.as_str()));
        }
    }
    out
}

/// 台帳 4 本を読む（並びは [`Domain::ALL`]）。
///
/// 無ければ探したパスを添えて失敗する（先に `ledger-init`）。
fn load_ledgers() -> Result<Vec<Ledger>, SurveyError> {
    let mut ledgers = Vec::with_capacity(Domain::ALL.len());
    for domain in Domain::ALL {
        let path = paths::ledger_path(domain);
        ledgers.push(read_ledger(&files::read_normalized(&path)?, domain)?);
    }
    Ok(ledgers)
}

/// ソースを走査して証拠の索引を組む（取り出し → 解決）。
///
/// `check` と `evidence` が同じ経路を通るための 1 か所。片方だけが別の集め方を
/// すると、検査が見た証拠と `evidence` が並べた証拠が食い違う。
fn collect_evidence(catalog: &Catalog) -> Result<EvidenceIndex, SurveyError> {
    let sources = sources::walk(&paths::workspace_root())?;
    let hits: Vec<_> = sources
        .iter()
        .flat_map(|(path, text)| extract(path, text))
        .collect();
    Ok(resolve(&hits, &sources, catalog))
}

#[cfg(test)]
#[path = "inspect_tests.rs"]
mod tests;
