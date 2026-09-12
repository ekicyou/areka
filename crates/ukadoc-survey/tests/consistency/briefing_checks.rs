//! 判定 ⑷（段階と順位・`briefing.md`）が実データで緑であることと、1 か所壊すと
//! 赤になることの主張（要件 11.1 ⑷・11.3・設計「判定の一覧」）。
//!
//! 腕そのもの（a〜g）と数え直しの道具は兄弟の `briefing_arms.rs` にあり、そちらには
//! テストの本体を 1 つも置かない。2 つに割ってあるのは 1 ファイル 1,000 行の目安を
//! 守るためである（`structure.md:176`・設計「1,000 行の番人」）。
//!
//! # 摂動 3 本と、残る腕の較正
//!
//! タスクが名指す摂動は「束名を 1 つ消す」「件数を 1 ずらす」「優先度を 1 件書き換える」
//! の 3 本で、それぞれ腕 a・b・c を赤にする。残る腕 d・e・f・g にも 1 つずつ較正を
//! 置く——赤にできない腕は判定として数えない（要件 11.3）。
//!
//! # 読むだけ
//!
//! 摂動は本文の**写し**（`String`）か、読み終えた骨組みの**写し**（[`Briefing`]・
//! [`Linkage`]・[`Ledger`]）の上でだけ働かせる。repo のファイルには 1 バイトも触れない。

use std::collections::BTreeSet;

use ukadoc_survey::documents::parse::read_briefing;
use ukadoc_survey::documents::{Briefing, Linkage, OverrideKind, RankTarget, Stage};
use ukadoc_survey::ledger::Ledger;
use ukadoc_survey::model::{Domain, EntryId};

use super::RepoData;
use super::briefing_arms::{
    BRIEFING_MD, Carryover, LedgerFacts, STAGE_RULE_REF, SYSTEM_MARK, UPDATE_THEME,
    distribution_findings, ledger_file, override_finding, priority_findings,
    rank_coverage_findings, rank_derived_findings, rank_order_findings, row_names, row_place,
    rows_at, stage_and_rank_findings, stage_count_findings, stage_rule_findings,
};
use super::documents::{Documents, shift_count};

// ---------------------------------------------------------------------------
// 摂動と較正の共通の道具
// ---------------------------------------------------------------------------

/// 所見が 1 つでもあれば、その全部を名指して止まる。
fn assert_no_findings(judgement: &str, findings: &[String]) {
    assert!(
        findings.is_empty(),
        "{judgement}（{} 件）:\n{}",
        findings.len(),
        findings.join("\n")
    );
}

/// 摂動の後の所見がちょうど 1 件で、その本文が指定の綴りを全部含むこと。
fn assert_single_finding(findings: &[String], what: &str, must_name: &[&str]) {
    assert_eq!(
        findings.len(),
        1,
        "{what} で赤が 1 件でない:\n{}",
        findings.join("\n")
    );
    for needle in must_name {
        assert!(
            findings[0].contains(needle),
            "失敗の本文が {needle} を名指していない: {}",
            findings[0]
        );
    }
}

// ---------------------------------------------------------------------------
// 実データで緑（要件 11.1 ⑷）
// ---------------------------------------------------------------------------

/// ⑷ 段階と順位が帰属・台帳・導出と揃う。
#[test]
fn every_bundle_carries_one_stage_and_one_rank_that_the_ledgers_agree_with() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);

    let findings = stage_and_rank_findings(
        &documents.linkage,
        &documents.briefing,
        &repo.ledgers,
        &facts,
        &Carryover::new(),
    );
    assert_no_findings("briefing.md の段階と順位が台帳と食い違う", &findings);
}

// ---------------------------------------------------------------------------
// 摂動 3 本（要件 11.3）
// ---------------------------------------------------------------------------

/// 摂動 1: 束名を 1 つ消すと、腕 a が**その束名**を名指して赤になる。
///
/// 消すのは骨組みの写しの上である。本文の写しから `bundle = "…"` の行だけを落とすと
/// `[[rank]]` が `bundle` も `singles` も持たない行になり、読み手が**判定より前に**
/// 落ちる（`documents.rs` の `drop_bundle_name` はそのために使えない）。
#[test]
fn dropping_one_bundle_name_turns_the_coverage_arm_red() {
    let documents = Documents::load();

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &rank_coverage_findings(&documents.linkage, &documents.briefing),
    );

    let name = a_ranked_bundle(&documents.briefing);
    let mut broken = documents.briefing.clone();
    broken
        .ranks
        .retain(|row| !row_names(row).contains(&name.as_str()));

    let findings = rank_coverage_findings(&documents.linkage, &broken);
    assert_single_finding(
        &findings,
        &format!("順位表から束 {name} の行を消した"),
        &[BRIEFING_MD, &name],
    );
}

/// 摂動 2: 件数を 1 ずらすと、腕 b が**その段階の鍵**を名指して赤になる。
#[test]
fn shifting_one_stage_count_turns_the_stage_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &stage_count_findings(&documents.briefing, &facts),
    );

    let shifted = shift_count(&documents.briefing_text, "[stage.A]", "items");
    let broken = read_briefing(&shifted)
        .unwrap_or_else(|err| panic!("件数を 1 ずらした写しを briefing.md として読めない: {err}"));

    let findings = stage_count_findings(&broken, &facts);
    assert_single_finding(
        &findings,
        "[stage.A] の items を 1 ずらした",
        &[BRIEFING_MD, "[stage.A] の items"],
    );
}

/// 摂動 3: 優先度を 1 件書き換えると、腕 c が**その台帳と id** を名指して赤になる。
///
/// 書き換えるのは段階の 1 文字を保ったまま順位の数だけである。段階まで動かすと
/// 腕 b（`[stage.X]` の `items`）も同時に赤くなり、「腕 c が働いた」と言えなくなる。
#[test]
fn rewriting_one_priority_turns_the_priority_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &priority_findings(&documents.linkage, &documents.briefing, &repo.ledgers),
    );

    let mut broken = repo.ledgers.clone();
    let (domain, id) = a_rewritable_priority(&broken);
    let key = EntryId::parse(&id).unwrap_or_else(|err| panic!("{id} を id として読めない: {err}"));
    let entry = broken
        .iter_mut()
        .find(|ledger| ledger.domain == domain)
        .and_then(|ledger| ledger.entries.get_mut(&key))
        .unwrap_or_else(|| panic!("{id} が写しの台帳に無い"));
    entry.priority = format!("{}999", &entry.priority[..1]);

    let findings = priority_findings(&documents.linkage, &documents.briefing, &broken);
    assert_single_finding(
        &findings,
        &format!("{id} の priority を書き換えた"),
        &[&ledger_file(domain), &id],
    );
}

// ---------------------------------------------------------------------------
// 残る腕の較正（1 か所壊すと赤・要件 11.3）
// ---------------------------------------------------------------------------

/// 腕 d: `assets` を 1 ずらすと、その行と束名を名指して赤になる。
#[test]
fn shifting_one_assets_turns_the_derived_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &rank_derived_findings(&documents.linkage, &documents.briefing, &repo.ledgers),
    );

    let mut broken = documents.briefing.clone();
    let row = broken.ranks.first_mut().expect("[[rank]] が 1 行も無い");
    let name = row_names(row)
        .first()
        .map(|name| (*name).to_owned())
        .expect("先頭の行が束を指していない");
    let place = row_place(row);
    row.assets += 1;

    let findings = rank_derived_findings(&documents.linkage, &broken, &repo.ledgers);
    assert_single_finding(
        &findings,
        &format!("{place} の assets を 1 ずらした"),
        &[BRIEFING_MD, &name, "assets"],
    );
}

/// 腕 e: 密な順位を 1 つ飛ばすと、その行を名指して赤になる。
#[test]
fn skipping_one_rank_turns_the_order_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let carryover = Carryover::new();

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &rank_order_findings(
            &documents.linkage,
            &documents.briefing,
            &repo.ledgers,
            &carryover,
        ),
    );

    let mut broken = documents.briefing.clone();
    let at = broken
        .ranks
        .iter()
        .position(|row| row.stage == Stage::A && row.rank == 2)
        .expect("段階 A に rank 2 の行が無い");
    broken.ranks[at].rank = 3;
    let place = row_place(&broken.ranks[at]);

    let findings = rank_order_findings(&documents.linkage, &broken, &repo.ledgers, &carryover);
    assert!(
        findings.iter().any(|line| line.contains(&place)),
        "順位を 1 つ飛ばしたのにその行を名指していない:\n{}",
        findings.join("\n")
    );
    assert!(
        findings.iter().all(|line| line.contains(BRIEFING_MD)),
        "失敗の本文がファイル名を名指していない:\n{}",
        findings.join("\n")
    );
}

/// 腕 e: `override` の受け付け形が母数 0 の恒真でないこと（`second-stage` は実データ 0 行）。
#[test]
fn an_unacceptable_override_reference_turns_the_order_arm_red() {
    let documents = Documents::load();
    let carryover = Carryover::new();

    assert!(
        documents.briefing.ranks.iter().all(|row| {
            row.exception
                .as_ref()
                .is_none_or(|exception| exception.kind != OverrideKind::SecondStage)
        }),
        "実データに second-stage の override がある——この較正の前提（母数 0）が変わった"
    );

    let at = documents
        .briefing
        .ranks
        .iter()
        .position(|row| row.exception.is_some())
        .expect("override を持つ行が 1 つも無い");
    let carried = carryover
        .headings()
        .first()
        .expect("持ち越し行の見出しが無い")
        .clone();

    for (kind, reference, wanted) in [
        (OverrideKind::StageRule, "要件 5.4".to_owned(), false),
        (OverrideKind::SecondStage, "項目 21".to_owned(), false),
        (
            OverrideKind::SecondStage,
            "記録 §13.1 行 99".to_owned(),
            false,
        ),
        (OverrideKind::StageRule, STAGE_RULE_REF.to_owned(), true),
        (OverrideKind::SecondStage, "項目 1".to_owned(), true),
        (OverrideKind::SecondStage, carried, true),
    ] {
        let mut copy = documents.briefing.clone();
        let exception = copy.ranks[at]
            .exception
            .as_mut()
            .expect("override が写しから消えた");
        exception.kind = kind;
        exception.reference = reference.clone();

        let seen = format!("ref \"{reference}\"（{}）", kind.as_key());
        match (wanted, override_finding(&copy.ranks[at], &carryover)) {
            (true, Some(line)) => panic!("受け付ける {seen} で赤になった: {line}"),
            (false, None) => panic!("受け付けない {seen} なのに赤にならない"),
            (false, Some(line)) => assert!(
                line.contains(BRIEFING_MD) && line.contains(&reference),
                "失敗の本文がファイル名と ref を名指していない: {line}"
            ),
            (true, None) => {}
        }
    }
}

/// 腕 e: `insufficient` の行の位置の主張が母数 0 の恒真でないこと（実データ 0 行）。
#[test]
fn an_insufficient_row_placed_before_the_targets_turns_the_order_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let carryover = Carryover::new();

    assert!(
        documents.briefing.ranks.iter().all(|row| !row.insufficient),
        "実データに insufficient の行がある——この較正の前提（母数 0）が変わった"
    );

    // 段階 A の先頭の行に印を付ける。対象行から外れるので順位の主張は動かないが、
    // 「対象行より後」だけが破れる。
    let mut broken = documents.briefing.clone();
    let at = broken
        .ranks
        .iter()
        .position(|row| row.stage == Stage::A)
        .expect("段階 A の行が無い");
    broken.ranks[at].insufficient = true;
    let place = row_place(&broken.ranks[at]);

    let findings: Vec<String> =
        rank_order_findings(&documents.linkage, &broken, &repo.ledgers, &carryover)
            .into_iter()
            .filter(|line| line.contains("insufficient"))
            .collect();
    assert_single_finding(
        &findings,
        "段階 A の先頭の行に insufficient を付けた",
        &[BRIEFING_MD, &place],
    );
}

/// 腕 e: `singles` の行に鍵の違う id を混ぜると赤になる。
#[test]
fn mixing_a_different_key_into_a_singles_row_turns_the_order_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let carryover = Carryover::new();

    let (at, intruder) = a_singles_row_and_a_stranger(&documents.briefing, &documents.linkage);
    let mut broken = documents.briefing.clone();
    let RankTarget::Singles(ids) = &mut broken.ranks[at].target else {
        panic!("選んだ行が singles でない");
    };
    ids.push(
        EntryId::parse(&intruder)
            .unwrap_or_else(|err| panic!("{intruder} を id として読めない: {err}")),
    );
    let place = row_place(&broken.ranks[at]);

    let findings: Vec<String> =
        rank_order_findings(&documents.linkage, &broken, &repo.ledgers, &carryover)
            .into_iter()
            .filter(|line| line.contains("singles に並べた"))
            .collect();
    assert_single_finding(
        &findings,
        &format!("{place} に鍵の違う {intruder} を混ぜた"),
        &[BRIEFING_MD, &intruder],
    );
}

/// 腕 f: `[[barrier]]` の数を 1 ずらすと、そのページと鍵を名指して赤になる。
///
/// 6 行が数えるのは**ページの全項目**であって段階 A の項目ではない。ここで赤が
/// 1 件だけ出ることが、数え方が入れ替わっていない証拠になる。
#[test]
fn shifting_one_barrier_count_turns_the_distribution_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &distribution_findings(&documents.briefing, &facts),
    );

    let mut broken = documents.briefing.clone();
    let barrier = broken
        .barriers
        .first_mut()
        .expect("[[barrier]] が 1 行も無い");
    let page = barrier.page.as_str().to_owned();
    barrier.absent += 1;

    let findings = distribution_findings(&broken, &facts);
    assert_single_finding(
        &findings,
        &format!("[[barrier]] {page} の absent を 1 ずらした"),
        &[BRIEFING_MD, &page, "absent"],
    );
}

/// 腕 g: 「更新」を落とす／`system.` を落とすと、それぞれ赤になる。
///
/// 「いずれか」で判定していることも同時に見る——段階 B の `rank` 1 の 2 束のうち
/// 1 つから「更新」を落としただけでは緑のままで、両方から落として初めて赤になる。
#[test]
fn breaking_either_half_of_the_stage_rule_turns_the_g_arm_red() {
    let documents = Documents::load();

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &stage_rule_findings(&documents.linkage, &documents.briefing),
    );

    let head = row_targets(&documents.briefing, Stage::B, 1);
    assert!(
        head.len() >= 2,
        "段階 B の rank 1 が 1 行しかない——「いずれか」で判定する理由（同着）が消えた"
    );

    let mut one_left = documents.linkage.clone();
    drop_theme(&mut one_left, &head[..1]);
    assert_no_findings(
        "段階 B の rank 1 の束が 1 つでも「更新」を持つのに赤くなった",
        &stage_rule_findings(&one_left, &documents.briefing),
    );

    let mut none_left = one_left.clone();
    drop_theme(&mut none_left, &head[1..]);
    assert_single_finding(
        &stage_rule_findings(&none_left, &documents.briefing),
        "段階 B の rank 1 の束から「更新」を全部落とした",
        &[BRIEFING_MD, UPDATE_THEME],
    );

    let last_rank = documents
        .briefing
        .ranks
        .iter()
        .filter(|row| row.stage == Stage::C)
        .map(|row| row.rank)
        .max()
        .expect("段階 C に順位の行が無い");
    let mut no_system = documents.linkage.clone();
    for name in row_targets(&documents.briefing, Stage::C, last_rank) {
        no_system
            .bundles
            .get_mut(&name)
            .unwrap_or_else(|| panic!("束 {name} が写しに無い"))
            .members
            .retain(|member| !member.as_str().contains(SYSTEM_MARK));
    }
    assert_single_finding(
        &stage_rule_findings(&no_system, &documents.briefing),
        &format!("段階 C の最大 rank {last_rank} の束から `{SYSTEM_MARK}` の id を落とした"),
        &[BRIEFING_MD, SYSTEM_MARK],
    );
}

// ---------------------------------------------------------------------------
// 摂動の的（選び方も較正する）
// ---------------------------------------------------------------------------

/// その段階のその順位の行が指す束名。
fn row_targets(briefing: &Briefing, stage: Stage, rank: usize) -> Vec<String> {
    rows_at(briefing, stage, rank)
        .iter()
        .flat_map(|row| row_names(row))
        .map(str::to_owned)
        .collect()
}

/// 写しの束から「更新」のテーマを落とす。
fn drop_theme(linkage: &mut Linkage, names: &[String]) {
    for name in names {
        linkage
            .bundles
            .get_mut(name)
            .unwrap_or_else(|| panic!("束 {name} が写しに無い"))
            .themes
            .retain(|theme| theme != UPDATE_THEME);
    }
}

/// 順位表に載っている束を 1 つ選ぶ（`bundle` の行の相手）。
fn a_ranked_bundle(briefing: &Briefing) -> String {
    briefing
        .ranks
        .iter()
        .find_map(|row| match &row.target {
            RankTarget::Bundle(name) => Some(name.clone()),
            RankTarget::Singles(_) => None,
        })
        .expect("[[rank]] に bundle を指す行が 1 つも無い")
}

/// 書き換えてよい `priority` を 1 件選ぶ（ドメイン, id）。
///
/// 段階の 1 文字を保ったまま順位の数だけを変えられる行——つまり `priority` が空欄で
/// ない行——を採る。
fn a_rewritable_priority(ledgers: &[Ledger]) -> (Domain, String) {
    for ledger in ledgers {
        for (id, entry) in &ledger.entries {
            if !entry.priority.is_empty() {
                return (ledger.domain, id.as_str().to_owned());
            }
        }
    }
    panic!("priority が入っている項目が 1 つも無い");
}

/// `singles` の行と、その行の鍵と違う単独項目の id を選ぶ（行の位置, 混ぜる id）。
fn a_singles_row_and_a_stranger(briefing: &Briefing, linkage: &Linkage) -> (usize, String) {
    let singles: Vec<&str> = linkage
        .bundles
        .iter()
        .filter(|(_, bundle)| bundle.single)
        .map(|(name, _)| name.as_str())
        .collect();

    for (at, row) in briefing.ranks.iter().enumerate() {
        let RankTarget::Singles(ids) = &row.target else {
            continue;
        };
        let here: BTreeSet<&str> = ids.iter().map(EntryId::as_str).collect();
        let Some(first) = here.first().and_then(|id| linkage.bundles.get(*id)) else {
            continue;
        };
        for name in &singles {
            if here.contains(name) {
                continue;
            }
            // 鍵の第 1 根拠（壊れ方）か第 2 根拠（テーマ数）が違えば混ぜる相手になる。
            let other = &linkage.bundles[*name];
            if other.breakage != first.breakage || other.themes.len() != first.themes.len() {
                return (at, (*name).to_owned());
            }
        }
    }
    panic!("鍵の違う単独項目を混ぜられる singles の行が無い");
}
