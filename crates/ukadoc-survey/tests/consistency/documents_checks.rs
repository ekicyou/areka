//! 判定 ⑴（3 文書が引用した項目 id の実在）と判定 ⑵（引用された機械の束 id の実在）
//! （要件 11.1 ⑴ ⑵・11.3・1.2・設計「入口 / `tests/consistency`」→「判定の一覧」）。
//!
//! # ⑴ が主張すること
//!
//! `linkage.md`・`briefing.md`・`roadmap-draft.md` の 3 文書について、
//!
//! - 引用符と逆引用符で囲まれた `ukadoc:…` の綴りが**全数**カタログに実在する、
//! - 囲まずに地の文へ裸で書かれた `ukadoc:…` が **0 件**である（裸で書くと引用の網から
//!   漏れて、実在しない綴りが黙って通る・設計 D-1）、
//! - 実在しない報告のファイル名 `report.md` の綴りが**どこにも無い**（要件 1.2。実在
//!   パスは `report/summary.md` で、brief の古い綴りを新規 3 文書へ持ち込まない）。
//!
//! 後ろ 2 つは「0 件である」の主張なので、**1 件混ぜると赤になる**腕を並べて置く
//! （[`a_single_bare_id_turns_the_citation_check_red`]・
//! [`the_stale_report_spelling_turns_the_citation_check_red`]）。混ぜた 1 件で赤に
//! できない検査は、0 件を数えているのではなく何も見ていない。
//!
//! # ⑵ が主張すること
//!
//! 3 文書が引用した**機械の束 id** が全数、報告 5 本の束の一覧に在ること。引用の出どころは
//! 2 つある——`linkage.md` の囲みの `machine` 欄と、3 文書の表のうち見出しが
//! [`BUNDLE_ID_HEADER`] で終わる列である（`由来する機械の束 id`・`収まる束 id`）。
//!
//! 地の文の「`…` を束 id とする」のような言い回しは相手にしない。機械の束の id は
//! それ自体が項目 id でもあるので、地の文では束 id と構成 id を綴りで見分けられない
//! ——拾えば構成 id を束 id として名指して必ず赤になる。地の文に書かれた綴りが実在
//! すること自体は ⑴ が見ている。2026-09-12 時点で、この取りこぼしに当たるのは
//! `linkage.md`「補修した関連」「機械の束の分割」「例示の 3 連鎖」の 3 節にある
//! 言い回しで、いずれも同じ id を上の 2 つの出どころでも引用している。
//!
//! # 報告の束の一覧は「列数」ではなく「表の見出し」で見つける
//!
//! 束の一覧は 2 種類ある。`report/summary.md`「ドメインを跨いで繋がった束」は
//! **3 列**（束 id・跨ぐドメイン・構成 id）で、ドメイン別報告 4 本「ドメイン内で関連が
//! 閉じている束」は **2 列**（束 id・構成 id）である。列数を決め打ちすると片方を丸ごと
//! 取り落とし、取り落とした側の束を引用した文書が「報告に無い束 id」で偽の赤になる
//! （2026-09-12 の実測で、決め打ちが落とすのは 123 行のうち 48 行または 75 行）。
//!
//! さらに、ドメイン別報告には**別名の一覧**という別の表があり、そちらの行も
//! `| ukadoc:` で始まる。行の綴りだけで拾うと別名 27 件が束 id の一覧に紛れ込み、
//! 別名を束 id として引用した文書が偽の緑になる。
//!
//! だから拾い方は「見出しが [`BUNDLE_TABLE_HEADER`] で始まる表の 1 列目」にする。
//! 見出しは両方の束の表に共通で（`report::domain`・`report::summary` の
//! `push_bundle_list` が書く）、別名の表の見出し（`| 別名の id |`）とは違う。
//!
//! # 数えたもの（2026-09-12・段 1 の後に数え直した実測）
//!
//! - `linkage.md` の引用 id: **1,553 件**。内訳は `members` の和集合 1,552 件（＝
//!   `[tally].target`）と、地の文だけに現れる別名 1 件
//!   （`ukadoc:list_sakura_script:_5c7:1`・「過剰だった関連」の節）。
//! - `briefing.md`・`roadmap-draft.md` の引用 id: **どちらも 0 件**（まだ骨組みで、
//!   本文は段 4・段 5 が書く）。下限を置くのはその 2 段の持ち物である。
//! - 裸の id: 3 文書とも **0 件**。`report.md` の綴り: 3 文書とも **0 件**。
//! - 報告 5 本の束の一覧: **123 行・122 種**（summary 75 行・sakura-script 23 行・
//!   shiori 25 行・assets 0 行・property 0 行）。1 種だけ summary と sakura-script の
//!   両方に現れる。
//! - 3 文書が引用した機械の束 id: **47 種**（すべて `linkage.md`。`machine` 欄 47 種と
//!   表の列 3 種で、表の 3 種は `machine` 欄にも在る）。
//!
//! # 読むだけ
//!
//! 摂動は本文の**写し**（`String`）の上でだけ働かせる。repo のファイルには 1 バイトも
//! 触れない（`documents.rs` の道具・`documents_non_vacuity.rs` の見張り）。

use std::collections::{BTreeMap, BTreeSet};

use ukadoc_survey::catalog::Catalog;
use ukadoc_survey::documents::Linkage;
use ukadoc_survey::documents::parse::{bare_id_tokens, read_linkage};
use ukadoc_survey::model::{Domain, EntryId};

use super::RepoData;
use super::documents::{Documents, cited_ids, twist_id, twisted_id};

/// 3 文書のワークスペース根からの相対パス（失敗の本文はこの綴りで名指す）。
const LINKAGE_MD: &str = "doc/ukadoc-coverage/linkage.md";
const BRIEFING_MD: &str = "doc/ukadoc-coverage/briefing.md";
const ROADMAP_MD: &str = "doc/ukadoc-coverage/roadmap-draft.md";

/// brief が書いていた実在しない報告のファイル名（要件 1.2）。
///
/// 実在パスは `doc/ukadoc-coverage/report/summary.md` である。この綴りが 3 文書の
/// どこかにあれば、読み手は存在しないファイルを探しに行く。
const STALE_REPORT_SPELLING: &str = "report.md";

/// 報告の束の一覧の表の見出しの始まり（列数に依らない目印）。
const BUNDLE_TABLE_HEADER: &str = "| 束 id |";

/// 3 文書の表のうち、機械の束 id を並べた列の見出しの語尾。
const BUNDLE_ID_HEADER: &str = "束 id";

// ---------------------------------------------------------------------------
// 取り出し
// ---------------------------------------------------------------------------

/// 判定の相手になる 3 文書（相対パスと本文）。
fn three_documents(documents: &Documents) -> [(&'static str, &str); 3] {
    [
        (LINKAGE_MD, documents.linkage_text.as_str()),
        (BRIEFING_MD, documents.briefing_text.as_str()),
        (ROADMAP_MD, documents.roadmap_text.as_str()),
    ]
}

/// 報告 1 本の束の一覧に並ぶ束 id（現れた順・重複は畳まない）。
///
/// 見つけ方は [`BUNDLE_TABLE_HEADER`] で始まる行の次の次から、`|` で始まる行が続く
/// 限り 1 列目を読む（間に挟まる `| --- |` の区切り行は読み飛ばす）。列数は見ない。
pub(super) fn bundle_ids_in_report(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut ids = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        if !lines[at].starts_with(BUNDLE_TABLE_HEADER) {
            at += 1;
            continue;
        }
        at += 1;
        while at < lines.len() && lines[at].starts_with('|') {
            let cell = first_cell(lines[at]);
            if !is_rule_cell(cell) {
                ids.push(cell.to_owned());
            }
            at += 1;
        }
    }
    ids
}

/// 表の行の 1 列目（前後の空白を落とす）。
fn first_cell(line: &str) -> &str {
    line.trim_start_matches('|')
        .split('|')
        .next()
        .unwrap_or("")
        .trim()
}

/// 表の区切り行（`---` や `---:` だけの升目）か。
fn is_rule_cell(cell: &str) -> bool {
    !cell.is_empty() && cell.chars().all(|c| matches!(c, '-' | ':'))
}

/// 報告 5 本の束の一覧に在る束 id の全種（設計 判定 ⑵）。
pub(super) fn report_bundle_ids(repo: &RepoData, summary_text: &str) -> BTreeSet<String> {
    let mut ids: BTreeSet<String> = bundle_ids_in_report(summary_text).into_iter().collect();
    for domain in Domain::ALL {
        let report = repo
            .domain_reports
            .get(&domain)
            .unwrap_or_else(|| panic!("{domain:?} のドメイン別報告が読み込まれていない"));
        ids.extend(bundle_ids_in_report(report));
    }
    ids
}

/// 3 文書の表のうち、見出しが [`BUNDLE_ID_HEADER`] で終わる列に並ぶ id。
///
/// 囲みの中は見ない（あちらは `machine` 欄の持ち場である）。
pub(super) fn bundle_ids_in_tables(markdown: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let mut columns: Option<Vec<usize>> = None;
    let mut inside_fence = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            inside_fence = !inside_fence;
            columns = None;
            continue;
        }
        if inside_fence {
            continue;
        }
        if !line.starts_with('|') {
            columns = None;
            continue;
        }
        let cells = row_cells(line);
        match &columns {
            // 表の 1 行目＝見出し。どの列が束 id の列かをここで決める。
            None => {
                columns = Some(
                    cells
                        .iter()
                        .enumerate()
                        .filter(|(_, cell)| cell.ends_with(BUNDLE_ID_HEADER))
                        .map(|(index, _)| index)
                        .collect(),
                );
            }
            Some(indexes) => {
                for &index in indexes {
                    if let Some(cell) = cells.get(index) {
                        ids.extend(ids_in_cell(cell));
                    }
                }
            }
        }
    }
    ids
}

/// 表の 1 行を升目に割る（前後の空白を落とす）。
fn row_cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

/// 1 つの升目に現れる `ukadoc:…` の綴り（逆引用符の有無を問わない）。
fn ids_in_cell(cell: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = cell;
    while let Some(at) = rest.find("ukadoc:") {
        let tail = &rest[at..];
        let end = tail.find(|c: char| !is_id_char(c)).unwrap_or(tail.len());
        ids.push(tail[..end].to_owned());
        rest = &tail[end..];
    }
    ids
}

/// id の綴りに現れてよい文字（`documents::parse` の同名の規則と同じ）。
fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-' | '.' | '(' | ')')
}

/// `linkage.md` が引用した機械の束 id（`machine` 欄＋表の列）。
pub(super) fn linkage_bundle_ids(linkage: &Linkage, linkage_text: &str) -> BTreeSet<String> {
    let mut ids: BTreeSet<String> = linkage
        .bundles
        .values()
        .flat_map(|bundle| bundle.machine.iter())
        .map(|id| id.as_str().to_owned())
        .collect();
    ids.extend(bundle_ids_in_tables(linkage_text));
    ids
}

/// 3 文書が引用した機械の束 id（ファイルごと）。
///
/// `machine` 欄を持つのは `linkage.md` だけなので、残る 2 本は表の列だけを見る。
fn cited_bundle_ids(
    linkage: &Linkage,
    documents: &Documents,
) -> Vec<(&'static str, BTreeSet<String>)> {
    vec![
        (
            LINKAGE_MD,
            linkage_bundle_ids(linkage, &documents.linkage_text),
        ),
        (BRIEFING_MD, bundle_ids_in_tables(&documents.briefing_text)),
        (ROADMAP_MD, bundle_ids_in_tables(&documents.roadmap_text)),
    ]
}

// ---------------------------------------------------------------------------
// 判定そのもの（所見を返す。摂動の腕も同じ関数を通す）
// ---------------------------------------------------------------------------

/// 判定 ⑴ の所見。空なら成り立っている。
fn citation_findings(files: &[(&'static str, &str)], catalog: &Catalog) -> Vec<String> {
    let mut findings = Vec::new();
    for (name, text) in files {
        for id in cited_ids(text) {
            let known =
                EntryId::parse(&id).is_ok_and(|parsed| catalog.entries.contains_key(&parsed));
            if !known {
                findings.push(format!(
                    "{name}: 引用した id {id} がカタログに実在しない（catalog.toml から写すこと）"
                ));
            }
        }
        for token in bare_id_tokens(text) {
            findings.push(format!(
                "{name}: id {token} が地の文に裸で書かれている（引用符か逆引用符で囲むこと・設計 D-1）"
            ));
        }
        if text.contains(STALE_REPORT_SPELLING) {
            findings.push(format!(
                "{name}: 実在しない報告のファイル名 {STALE_REPORT_SPELLING} が書かれている（実在パスは doc/ukadoc-coverage/report/summary.md・要件 1.2）"
            ));
        }
    }
    findings
}

/// 判定 ⑵ の所見。空なら成り立っている。
fn bundle_findings(
    cited: &[(&'static str, BTreeSet<String>)],
    known: &BTreeSet<String>,
) -> Vec<String> {
    let mut findings = Vec::new();
    for (name, ids) in cited {
        for id in ids {
            if !known.contains(id) {
                findings.push(format!(
                    "{name}: 機械の束 id {id} が報告 5 本の束の一覧に無い（報告を作り直すか引用を直すこと）"
                ));
            }
        }
    }
    findings
}

/// 所見が 1 つでもあれば、その全部を名指して止まる。
fn assert_no_findings(judgement: &str, findings: &[String]) {
    assert!(
        findings.is_empty(),
        "{judgement}（{} 件）:\n{}",
        findings.len(),
        findings.join("\n")
    );
}

// ---------------------------------------------------------------------------
// 実データで緑（要件 11.1 ⑴ ⑵）
// ---------------------------------------------------------------------------

/// ⑴ 3 文書が引用した id が全数カタログに実在し、裸の id も古い報告名も 0 件である。
#[test]
fn every_id_cited_by_the_three_documents_exists_in_the_catalog() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let files = three_documents(&documents);

    let findings = citation_findings(&files, &repo.catalog);
    assert_no_findings("3 文書の引用 id が台帳の正典と食い違う", &findings);
}

/// ⑵ 3 文書が引用した機械の束 id が全数、報告 5 本の束の一覧に在る。
#[test]
fn every_machine_bundle_id_cited_by_the_three_documents_is_listed_in_the_reports() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let known = report_bundle_ids(&repo, &documents.summary_text);
    let cited = cited_bundle_ids(&documents.linkage, &documents);

    let findings = bundle_findings(&cited, &known);
    assert_no_findings("3 文書の機械の束 id が報告と食い違う", &findings);
}

// ---------------------------------------------------------------------------
// 写しを 1 か所だけ壊すと赤（要件 11.3）
// ---------------------------------------------------------------------------

/// ⑴ 写しの id を 1 文字変えると、**ちょうどその 1 件**が名指しで赤になる。
#[test]
fn twisting_one_cited_id_turns_the_citation_check_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let target = a_twistable_id(&cited_ids(&documents.linkage_text));
    let broken = twist_id(&documents.linkage_text, &target);

    let files = [
        (LINKAGE_MD, broken.as_str()),
        (BRIEFING_MD, documents.briefing_text.as_str()),
        (ROADMAP_MD, documents.roadmap_text.as_str()),
    ];
    let findings = citation_findings(&files, &repo.catalog);

    let twisted = twisted_id(&target);
    assert_eq!(
        findings.len(),
        1,
        "1 か所だけ壊したのに赤が 1 件でない（{target} を {twisted} へ）:\n{}",
        findings.join("\n")
    );
    assert!(
        findings[0].contains(LINKAGE_MD) && findings[0].contains(&twisted),
        "失敗の本文がファイル名と id を名指していない: {}",
        findings[0]
    );

    // 壊す前は緑であること（赤の原因が摂動だと言い切るために要る）。
    let untouched = three_documents(&documents);
    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &citation_findings(&untouched, &repo.catalog),
    );
}

/// ⑵ 写しの由来の id（`machine` 欄）を 1 文字変えると、**ちょうどその 1 件**が
/// 名指しで赤になる。
#[test]
fn twisting_one_machine_bundle_id_turns_the_bundle_check_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let known = report_bundle_ids(&repo, &documents.summary_text);

    let cited = linkage_bundle_ids(&documents.linkage, &documents.linkage_text);
    let target = a_twistable_id(&cited);
    let broken = twist_id(&documents.linkage_text, &target);
    let broken_linkage = read_linkage(&broken)
        .unwrap_or_else(|err| panic!("1 文字変えた写しを linkage.md として読めない: {err}"));

    let findings = bundle_findings(
        &[(LINKAGE_MD, linkage_bundle_ids(&broken_linkage, &broken))],
        &known,
    );

    let twisted = twisted_id(&target);
    assert_eq!(
        findings.len(),
        1,
        "1 か所だけ壊したのに赤が 1 件でない（{target} を {twisted} へ）:\n{}",
        findings.join("\n")
    );
    assert!(
        findings[0].contains(LINKAGE_MD) && findings[0].contains(&twisted),
        "失敗の本文がファイル名と束 id を名指していない: {}",
        findings[0]
    );

    // 壊す前は緑であること。
    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &bundle_findings(&cited_bundle_ids(&documents.linkage, &documents), &known),
    );
}

/// ⑴ の「裸の id は 0 件」に、1 件混ぜると赤になる腕を付ける。
#[test]
fn a_single_bare_id_turns_the_citation_check_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    // 実在する id を、囲まずに地の文へ 1 行足す。綴りは実在するので「カタログに無い」
    // では赤くならない——赤くなるとしたら「裸で書いた」だけである。
    let real = a_twistable_id(&cited_ids(&documents.linkage_text));
    let smuggled = format!("{}\n\n裸で書いた {real} の行。\n", documents.roadmap_text);

    let files = [
        (LINKAGE_MD, documents.linkage_text.as_str()),
        (BRIEFING_MD, documents.briefing_text.as_str()),
        (ROADMAP_MD, smuggled.as_str()),
    ];
    let findings = citation_findings(&files, &repo.catalog);

    assert_eq!(
        findings.len(),
        1,
        "裸の id を 1 件混ぜたのに赤が 1 件でない:\n{}",
        findings.join("\n")
    );
    assert!(
        findings[0].contains(ROADMAP_MD) && findings[0].contains(&real),
        "失敗の本文がファイル名と id を名指していない: {}",
        findings[0]
    );
}

/// ⑴ の「`report.md` の綴りは 0 件」に、1 件混ぜると赤になる腕を付ける（要件 1.2）。
#[test]
fn the_stale_report_spelling_turns_the_citation_check_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let smuggled = format!(
        "{}\n\n根拠は `doc/ukadoc-coverage/{STALE_REPORT_SPELLING}` にある。\n",
        documents.briefing_text
    );

    let files = [
        (LINKAGE_MD, documents.linkage_text.as_str()),
        (BRIEFING_MD, smuggled.as_str()),
        (ROADMAP_MD, documents.roadmap_text.as_str()),
    ];
    let findings = citation_findings(&files, &repo.catalog);

    assert_eq!(
        findings.len(),
        1,
        "古い報告名を 1 件混ぜたのに赤が 1 件でない:\n{}",
        findings.join("\n")
    );
    assert!(
        findings[0].contains(BRIEFING_MD) && findings[0].contains(STALE_REPORT_SPELLING),
        "失敗の本文がファイル名と綴りを名指していない: {}",
        findings[0]
    );
}

// ---------------------------------------------------------------------------
// 取り出しの較正
// ---------------------------------------------------------------------------

/// 報告の束の一覧の拾い方が、列数の違う 2 種類の表を**どちらも**拾い、別名の表を
/// 拾わないこと。
///
/// 列数を決め打ちした拾い方はここで赤くなる（3 列だけ見れば 2 列の表を、2 列だけ
/// 見れば 3 列の表を落とす）。行の綴り（`| ukadoc:`）だけで拾う書き方も、別名の行を
/// 束 id として拾うのでここで赤くなる。
#[test]
fn the_bundle_list_scan_reads_both_shapes_and_skips_the_alias_table() {
    let three_columns = "\
## ドメインを跨いで繋がった束

| 束 id | 跨ぐドメイン | 構成 id |
| --- | --- | --- |
| ukadoc:a:X:1 | assets, shiori | ukadoc:a:X:1, ukadoc:b:Y:1 |

## テーマ別の状態分布
";
    let two_columns = "\
## 別名の一覧

| 別名の id | 指す先の id |
| --- | --- |
| ukadoc:alias:Z:1 | ukadoc:a:X:1 |

## ドメイン内で関連が閉じている束

| 束 id | 構成 id |
| --- | --- |
| ukadoc:c:W:1 | ukadoc:c:W:1, ukadoc:d:V:1 |
";
    assert_eq!(
        bundle_ids_in_report(three_columns),
        vec!["ukadoc:a:X:1".to_owned()],
        "3 列の束の表を拾えていない"
    );
    assert_eq!(
        bundle_ids_in_report(two_columns),
        vec!["ukadoc:c:W:1".to_owned()],
        "2 列の束の表だけを拾えていない（別名の表を拾っている疑い）"
    );
    assert!(
        bundle_ids_in_report("束の表はありません。\n").is_empty(),
        "表の無い報告から束 id を拾っている"
    );
}

/// 実データの報告 5 本から、両方の形の束の表が実際に拾えていること。
///
/// 上の較正は作った本文の上の話なので、実データで両方の形が現れることを別に見る
/// ——片方が 0 行に落ちても作った本文の較正は緑のままである。
#[test]
fn both_report_shapes_contribute_bundle_ids_in_the_real_data() {
    let repo = RepoData::load();
    let documents = Documents::load();

    let crossing = bundle_ids_in_report(&documents.summary_text);
    assert!(
        !crossing.is_empty(),
        "report/summary.md の「ドメインを跨いで繋がった束」から束 id を 1 件も拾えていない"
    );

    let within: usize = Domain::ALL
        .iter()
        .map(|domain| {
            bundle_ids_in_report(
                repo.domain_reports
                    .get(domain)
                    .unwrap_or_else(|| panic!("{domain:?} のドメイン別報告が読み込まれていない")),
            )
            .len()
        })
        .sum();
    assert!(
        within > 0,
        "ドメイン別報告 4 本の「ドメイン内で関連が閉じている束」から束 id を 1 件も拾えていない"
    );
}

/// 表の列の拾い方が、見出しの語尾で列を選ぶこと。
#[test]
fn the_table_column_scan_picks_columns_by_their_heading() {
    let markdown = "\
| ページ単位の id | 正典の題 | 由来する機械の束 id |
| --- | --- | --- |
| `ukadoc:manual_update` | ネットワーク | `ukadoc:m:A:1` |

地の文の `ukadoc:not:a:1` は拾わない。

| 連鎖 | 収まる束 id |
| --- | --- |
| 時刻の刻み | `ukadoc:t:B:1` |
";
    assert_eq!(
        bundle_ids_in_tables(markdown),
        BTreeSet::from(["ukadoc:m:A:1".to_owned(), "ukadoc:t:B:1".to_owned()]),
        "見出しの語尾で束 id の列を選べていない"
    );

    let fenced = "```toml\n| 由来する機械の束 id |\n| --- |\n| `ukadoc:fenced:C:1` |\n```\n";
    assert!(
        bundle_ids_in_tables(fenced).is_empty(),
        "囲みの中の表まで拾っている"
    );
}

// ---------------------------------------------------------------------------
// 摂動の的
// ---------------------------------------------------------------------------

/// 摂動に使える id を 1 つ選ぶ。
///
/// 選ぶのは**ほかのどの引用 id の接頭辞でもない**綴りである。ページ単位の id
/// （`ukadoc:manual_update` のような 2 節の綴り）はアンカー付きの id の接頭辞に
/// なりうるので、これを選ぶと本文の差し替えが巻き添えで別の id まで壊し、
/// 「1 か所だけ壊した」が成り立たなくなる。
fn a_twistable_id(ids: &BTreeSet<String>) -> String {
    for id in ids {
        if !ids.iter().any(|other| other != id && other.starts_with(id)) {
            return id.clone();
        }
    }
    panic!(
        "摂動に使える id が 1 つも無い（引用 id が {} 件）",
        ids.len()
    );
}

/// 摂動の的の選び方が、ほかの id の接頭辞になる綴りを避けること。
#[test]
fn the_perturbation_target_is_never_a_prefix_of_another_id() {
    let ids = BTreeSet::from([
        "ukadoc:a".to_owned(),
        "ukadoc:a:X:1".to_owned(),
        "ukadoc:b:Y:1".to_owned(),
    ]);
    assert_eq!(
        a_twistable_id(&ids),
        "ukadoc:a:X:1",
        "ほかの id の接頭辞になる綴りを選んでいる"
    );
}

/// 実データで選んだ的が、実際に 3 文書のどこにも巻き添えを作らないこと。
#[test]
fn the_real_perturbation_target_appears_only_as_itself() {
    let documents = Documents::load();
    let ids = cited_ids(&documents.linkage_text);
    let target = a_twistable_id(&ids);
    let hit: BTreeMap<&str, usize> = ids
        .iter()
        .filter(|id| id.starts_with(&target))
        .map(|id| (id.as_str(), 1))
        .collect();
    assert_eq!(
        hit.len(),
        1,
        "摂動の的 {target} を接頭辞に持つ引用 id がほかにある: {:?}",
        hit.keys().collect::<Vec<_>>()
    );
}
