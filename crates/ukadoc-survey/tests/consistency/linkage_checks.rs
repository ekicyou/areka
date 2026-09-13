//! 判定 ⑶（帰属の分割・`linkage.md`）
//! （要件 11.1 ⑶・11.3・11.4・4.2・設計「入口 / `tests/consistency`」→「判定の一覧」）。
//!
//! # ⑶ が主張すること
//!
//! `linkage.md` の束（名前付き束と単独項目）について、7 つの腕を並べる。
//!
//! - **a 互いに素**: 同じ構成 id が 2 つの束に属さない。
//! - **b 過不足なし**: 構成 id の和集合が、状態が `implemented`・`vocabulary-only`・
//!   `degraded`・`absent` である台帳の全項目とちょうど一致する。
//! - **c 由来**: `hand` が `members` の部分集合であり、`members ∖ hand`（機械由来と
//!   名乗る id）が、その束が引用した機械の束の構成 id の和集合に含まれる。
//! - **d 除外**: `alias`・`not-applicable` の id が構成に 1 つも現れない（要件 4.3）。
//! - **e 導出**: `themes`・`domains` が [`derive_bundle`] の値と一致し、
//!   `breakage = "該当なし"` の束は全構成 id が `implemented` である。
//! - **f 合計**: `[tally]` の数が数え直しと一致する（6 つの鍵とドメイン別の 4 欄）。
//! - **g 恒等式**: `[tally]` の数の間に 3 つの恒等式が成り立つ。
//!
//! 腕を関数に割ってあるのは摂動のためである。1 か所壊すと**複数の腕**が同時に赤くなる
//! ——構成 id を 1 つ抜けば b が「どの束にも属さない」と言い、f が「`from_machine` が
//! 1 多い」と言う。どちらも正しい所見なので、どちらかを黙らせるのではなく、腕ごとに
//! 「ちょうど 1 件」を確かめる。判定そのもの（実データが緑であること）は
//! [`the_attribution_of_every_target_item_splits_across_the_bundles`] が 7 つまとめて見る。
//!
//! # 導出は二重に実装しない（設計 D-4）
//!
//! 腕 e が突き合わせる相手は `documents::derive::derive_bundle` の戻り値そのもので、
//! ここでテーマの和集合や跨ぐドメインを数え直しはしない。2 か所に書くと、第二段の
//! 組み直しで片方だけが古びる。
//!
//! # 「該当なし」の腕は実データでは母数 0 である
//!
//! 2026-09-12 時点で 63 束すべての `breakage` が「黙って壊れる」なので、
//! 「`該当なし` の束は全構成 id が `implemented`」は**母数 0 の恒真**である。恒真の
//! まま置くと「判定を置いた」という記録だけが残るので、写しに「該当なし」の束を
//! 1 つ作って腕が働くことを較正する
//! （[`marking_a_bundle_not_applicable_turns_the_derived_arm_red`]）。同じ較正が
//! 「実データに『該当なし』の束は無い」ことも見張るので、束が 1 つでも増えれば
//! この段落が古びたことに気づける。
//!
//! # 数えたもの（2026-09-12・実データ）
//!
//! - 束: **67**（名前付き束 63・単独項目 4）。構成 id は延べ **1,552**。
//! - `[tally]`: `target = 1552`・`from_machine = 195`・`by_hand = 1353`・`singles = 4`・
//!   `alias_excluded = 27`・`not_applicable_excluded = 170`。
//!   `[tally.singles_by_domain]` は assets 4・property 0・sakura-script 0・shiori 0。
//! - `breakage` は 67 束すべてが「黙って壊れる」（「該当なし」は **0 束**）。
//!
//! # 読むだけ
//!
//! 摂動は本文の**写し**（`String`）か、読み終えた骨組みの**写し**（[`Linkage`]）の上で
//! だけ働かせる。repo のファイルには 1 バイトも触れない（`documents_non_vacuity.rs` の
//! 見張りが実ファイルの中身と更新時刻で確かめる）。

use std::collections::{BTreeMap, BTreeSet};

use ukadoc_survey::documents::derive::derive_bundle;
use ukadoc_survey::documents::parse::read_linkage;
use ukadoc_survey::documents::{Breakage, Briefing, Linkage};
use ukadoc_survey::ledger::Ledger;
use ukadoc_survey::model::{Domain, EntryId, Status};

use super::RepoData;
use super::documents::{Documents, drop_member, shift_count};
use super::documents_checks::bundle_rows_in_report;

/// 帰属の正本のワークスペース根からの相対パス（失敗の本文はこの綴りで名指す）。
const LINKAGE_MD: &str = "doc/ukadoc-coverage/linkage.md";

/// 束の構成に入れる対象の 4 状態（要件 4.2）。
const TARGET_STATUSES: [Status; 4] = [
    Status::Implemented,
    Status::VocabularyOnly,
    Status::Degraded,
    Status::Absent,
];

/// 束の構成から除く 2 状態（要件 4.3）。
const EXCLUDED_STATUSES: [Status; 2] = [Status::Alias, Status::NotApplicable];

// ---------------------------------------------------------------------------
// 台帳から数え直す側
// ---------------------------------------------------------------------------

/// 台帳 4 本の項目（数え直しの出どころ）。
///
/// 台帳を 4 本とも 1 度だけ畳んで持つ。腕ごとに台帳を歩き直すと、同じ数え方が
/// 腕の数だけ散らばって、片方だけ直したときに気づけない。
struct LedgerFacts {
    /// 項目 id →（その項目を持つ台帳のドメイン, 状態）。
    entries: BTreeMap<String, (Domain, Status)>,
}

impl LedgerFacts {
    /// 台帳 4 本から畳む。
    fn collect(ledgers: &[Ledger]) -> Self {
        let mut entries = BTreeMap::new();
        for ledger in ledgers {
            for (id, entry) in &ledger.entries {
                entries.insert(id.as_str().to_owned(), (ledger.domain, entry.status));
            }
        }
        Self { entries }
    }

    /// 台帳 4 本の項目の総数（状態を問わない）。
    fn total(&self) -> usize {
        self.entries.len()
    }

    /// その id の状態（台帳に無ければ `None`）。
    fn status_of(&self, id: &str) -> Option<Status> {
        self.entries.get(id).map(|(_, status)| *status)
    }

    /// その id を持つ台帳のドメイン（台帳に無ければ `None`）。
    fn domain_of(&self, id: &str) -> Option<Domain> {
        self.entries.get(id).map(|(domain, _)| *domain)
    }

    /// 対象 4 状態の項目か。
    fn is_target(&self, id: &str) -> bool {
        self.status_of(id)
            .is_some_and(|status| TARGET_STATUSES.contains(&status))
    }

    /// 対象 4 状態の項目 id（文字順）。
    fn target_ids(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .filter(|(_, (_, status))| TARGET_STATUSES.contains(status))
            .map(|(id, _)| id.as_str())
    }

    /// その状態の項目数。
    fn count_of(&self, status: Status) -> usize {
        self.entries
            .values()
            .filter(|(_, actual)| *actual == status)
            .count()
    }

    /// 束の構成から除く 2 状態の項目 id を 1 つ（較正の的）。
    fn an_excluded_id(&self) -> &str {
        self.entries
            .iter()
            .find(|(_, (_, status))| EXCLUDED_STATUSES.contains(status))
            .map(|(id, _)| id.as_str())
            .expect("台帳に alias・not-applicable の項目が 1 つも無い")
    }
}

/// 報告 5 本の束の一覧から読んだ「機械の束 id → その構成 id」。
///
/// 同じ束 id が `report/summary.md` とドメイン別報告の両方に現れることがあるので、
/// 構成 id は和集合にする（片方だけを採ると腕 c が偽の赤になる）。
fn machine_bundle_members(
    repo: &RepoData,
    summary_text: &str,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut rows: Vec<(String, Vec<String>)> = bundle_rows_in_report(summary_text);
    for domain in Domain::ALL {
        let report = repo
            .domain_reports
            .get(&domain)
            .unwrap_or_else(|| panic!("{domain:?} のドメイン別報告が読み込まれていない"));
        rows.extend(bundle_rows_in_report(report));
    }

    let mut by_id: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (id, members) in rows {
        by_id.entry(id).or_default().extend(members);
    }
    by_id
}

/// 引用した機械の束の構成 id の和集合。
///
/// 報告に無い束 id は何も足さない——「引用した束 id が報告に在る」は判定 ⑵ の持ち場で、
/// ここで二重に赤くすると直す場所が 2 つあるように見える。
fn machine_members_of<'a>(
    cited: impl Iterator<Item = &'a str>,
    machine: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut known = BTreeSet::new();
    for id in cited {
        if let Some(members) = machine.get(id) {
            known.extend(members.iter().cloned());
        }
    }
    known
}

/// 構成 id → その id を持つ束の名前（束名の文字順で最初のもの）。
fn bundle_of_id(linkage: &Linkage) -> BTreeMap<&str, &str> {
    let mut by_id = BTreeMap::new();
    for (name, bundle) in &linkage.bundles {
        for member in &bundle.members {
            by_id.entry(member.as_str()).or_insert(name.as_str());
        }
    }
    by_id
}

// ---------------------------------------------------------------------------
// 判定そのもの（腕ごとに所見を返す。摂動の腕も同じ関数を通す）
// ---------------------------------------------------------------------------

/// 腕 a: 束の構成 id が互いに素であること。
fn disjoint_findings(linkage: &Linkage) -> Vec<String> {
    let mut owner: BTreeMap<&str, &str> = BTreeMap::new();
    let mut findings = Vec::new();
    for (name, bundle) in &linkage.bundles {
        for member in &bundle.members {
            let id = member.as_str();
            match owner.get(id) {
                Some(first) => findings.push(format!(
                    "{LINKAGE_MD}: 構成 id {id} が束 {first} と束 {name} の両方に属している（帰属はちょうど 1 つ・要件 4.2）"
                )),
                None => {
                    owner.insert(id, name.as_str());
                }
            }
        }
    }
    findings
}

/// 腕 b: 構成 id の和集合が対象 4 状態の全項目と過不足なく一致すること。
fn coverage_findings(linkage: &Linkage, facts: &LedgerFacts) -> Vec<String> {
    let by_id = bundle_of_id(linkage);
    let mut findings = Vec::new();
    for id in facts.target_ids() {
        if !by_id.contains_key(id) {
            findings.push(format!(
                "{LINKAGE_MD}: 対象 4 状態の {id} がどの束にも属さない（名前付き束か単独項目のどちらかに入れる・要件 4.2）"
            ));
        }
    }
    for (id, name) in &by_id {
        if !facts.is_target(id) {
            let status = facts
                .status_of(id)
                .map_or("台帳に無い".to_owned(), |status| {
                    format!("状態が {}", status.as_key())
                });
            findings.push(format!(
                "{LINKAGE_MD}: 束 {name} の構成 id {id} が対象 4 状態の項目でない（{status}）"
            ));
        }
    }
    findings
}

/// 腕 c: 機械由来と名乗る id が、引用した機械の束の構成 id に含まれること。
///
/// 単独項目は `machine`・`hand` を持てない（読み手が欄そのものを禁じている）ので
/// 相手にしない。`[tally]` の `from_machine`・`by_hand` が単独でない束だけを合算する
/// のと同じ切り分けである（設計 D-2）。
fn provenance_findings(
    linkage: &Linkage,
    machine: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<String> {
    let mut findings = Vec::new();
    for (name, bundle) in &linkage.bundles {
        if bundle.single {
            continue;
        }
        let members: BTreeSet<&str> = bundle.members.iter().map(EntryId::as_str).collect();
        for hand in &bundle.hand {
            if !members.contains(hand.as_str()) {
                findings.push(format!(
                    "{LINKAGE_MD}: 束 {name} の hand の {} が members に無い（hand は members の部分集合・要件 4.1 ⑶）",
                    hand.as_str()
                ));
            }
        }

        let known = machine_members_of(bundle.machine.iter().map(EntryId::as_str), machine);
        let by_hand: BTreeSet<&str> = bundle.hand.iter().map(EntryId::as_str).collect();
        for member in &bundle.members {
            let id = member.as_str();
            if by_hand.contains(id) || known.contains(id) {
                continue;
            }
            findings.push(format!(
                "{LINKAGE_MD}: 束 {name} の構成 id {id} が、引用した機械の束の構成 id に無い（人手で足したなら hand に印を付ける・要件 4.1 ⑶）"
            ));
        }
    }
    findings
}

/// 腕 d: `alias`・`not-applicable` の id が構成に 1 つも現れないこと（要件 4.3）。
fn excluded_findings(linkage: &Linkage, facts: &LedgerFacts) -> Vec<String> {
    let mut findings = Vec::new();
    for (name, bundle) in &linkage.bundles {
        for member in &bundle.members {
            let id = member.as_str();
            let Some(status) = facts.status_of(id) else {
                continue;
            };
            if EXCLUDED_STATUSES.contains(&status) {
                findings.push(format!(
                    "{LINKAGE_MD}: 束 {name} の構成 id {id} は状態が {}（束の構成から除く・要件 4.3）",
                    status.as_key()
                ));
            }
        }
    }
    findings
}

/// 腕 e: `themes`・`domains` が導出と一致し、「該当なし」の束が全実装済みであること。
fn derived_findings(
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
    facts: &LedgerFacts,
) -> Vec<String> {
    let mut findings = Vec::new();
    for (name, bundle) in &linkage.bundles {
        let derived = derive_bundle(bundle, linkage, briefing, ledgers);

        let themes: BTreeSet<String> = bundle.themes.iter().cloned().collect();
        if themes != derived.themes {
            findings.push(format!(
                "{LINKAGE_MD}: 束 {name} の themes が導出と食い違う（書いてある: [{}]／数え直し: [{}]）",
                joined(themes.iter().map(String::as_str)),
                joined(derived.themes.iter().map(String::as_str))
            ));
        }

        let domains: BTreeSet<Domain> = bundle.domains.iter().copied().collect();
        if domains != derived.domains {
            findings.push(format!(
                "{LINKAGE_MD}: 束 {name} の domains が導出と食い違う（書いてある: [{}]／数え直し: [{}]）",
                joined(domains.iter().map(|domain| domain.as_key())),
                joined(derived.domains.iter().map(|domain| domain.as_key()))
            ));
        }

        if bundle.breakage != Breakage::NotApplicable {
            continue;
        }
        for member in &bundle.members {
            let id = member.as_str();
            if facts.status_of(id) == Some(Status::Implemented) {
                continue;
            }
            let status = facts
                .status_of(id)
                .map_or("台帳に無い".to_owned(), |status| {
                    status.as_key().to_owned()
                });
            findings.push(format!(
                "{LINKAGE_MD}: 束 {name} は breakage が「該当なし」だが、構成 id {id} の状態は {status}（該当なしは全構成 id が implemented の束にだけ・要件 4.1 ⑺）"
            ));
        }
    }
    findings
}

/// 並びを本文に書く形へ。
fn joined<'a>(values: impl Iterator<Item = &'a str>) -> String {
    values.collect::<Vec<&str>>().join(", ")
}

/// `[tally]` の数え直し（腕 f・g が突き合わせる相手）。
struct Recount {
    target: usize,
    from_machine: usize,
    by_hand: usize,
    singles: usize,
    alias_excluded: usize,
    not_applicable_excluded: usize,
    singles_by_domain: BTreeMap<Domain, usize>,
}

impl Recount {
    /// 帰属と台帳から数え直す。
    ///
    /// `from_machine`・`by_hand` は `single = true` でない束だけを合算し、`singles` は
    /// 単独項目の数とする（設計 D-2 の数え方）。単独項目の id が台帳のどのドメインにも
    /// 無ければ、ドメイン別の内訳には足さない——その食い違いは腕 b が名指す。
    fn of(linkage: &Linkage, facts: &LedgerFacts) -> Self {
        let mut singles_by_domain: BTreeMap<Domain, usize> =
            Domain::ALL.iter().map(|domain| (*domain, 0)).collect();
        let (mut from_machine, mut by_hand, mut singles) = (0, 0, 0);

        for bundle in linkage.bundles.values() {
            if bundle.single {
                singles += 1;
                for member in &bundle.members {
                    if let Some(domain) = facts.domain_of(member.as_str()) {
                        *singles_by_domain.entry(domain).or_insert(0) += 1;
                    }
                }
                continue;
            }
            let hand: BTreeSet<&str> = bundle.hand.iter().map(EntryId::as_str).collect();
            by_hand += bundle.hand.len();
            from_machine += bundle
                .members
                .iter()
                .filter(|member| !hand.contains(member.as_str()))
                .count();
        }

        Self {
            target: facts.target_ids().count(),
            from_machine,
            by_hand,
            singles,
            alias_excluded: facts.count_of(Status::Alias),
            not_applicable_excluded: facts.count_of(Status::NotApplicable),
            singles_by_domain,
        }
    }
}

/// 腕 f: `[tally]` の数が数え直しと一致すること。
fn tally_findings(linkage: &Linkage, facts: &LedgerFacts) -> Vec<String> {
    let tally = &linkage.tally;
    let recount = Recount::of(linkage, facts);
    let mut findings = Vec::new();

    for (key, declared, actual) in [
        ("target", tally.target, recount.target),
        ("from_machine", tally.from_machine, recount.from_machine),
        ("by_hand", tally.by_hand, recount.by_hand),
        ("singles", tally.singles, recount.singles),
        (
            "alias_excluded",
            tally.alias_excluded,
            recount.alias_excluded,
        ),
        (
            "not_applicable_excluded",
            tally.not_applicable_excluded,
            recount.not_applicable_excluded,
        ),
    ] {
        if declared != actual {
            findings.push(mismatch(&format!("[tally] の {key}"), declared, actual));
        }
    }

    for domain in Domain::ALL {
        let declared = tally.singles_by_domain.get(&domain).copied();
        let actual = recount.singles_by_domain.get(&domain).copied().unwrap_or(0);
        match declared {
            None => findings.push(format!(
                "{LINKAGE_MD}: [tally.singles_by_domain] に {} の欄が無い（0 のドメインも省けない・要件 11.5）",
                domain.as_key()
            )),
            Some(declared) if declared != actual => findings.push(mismatch(
                &format!("[tally.singles_by_domain] の {}", domain.as_key()),
                declared,
                actual,
            )),
            Some(_) => {}
        }
    }
    findings
}

/// 数が数え直しと合わないことの本文。
fn mismatch(what: &str, declared: usize, actual: usize) -> String {
    format!("{LINKAGE_MD}: {what} は {declared} と書いてあるが、数え直しは {actual}")
}

/// 腕 g: `[tally]` の数の間に 3 つの恒等式が成り立つこと。
///
/// 恒等式 3 の右辺（台帳 4 本の項目数）だけは台帳から数え直す。3 つとも腕 f が緑なら
/// 自動的に成り立つわけではない——腕 f は 1 つずつ数を比べるだけで、`target` が
/// 「和に等しい」ことは分割（腕 a・b）と合わせて初めて言えるからである。
fn identity_findings(linkage: &Linkage, facts: &LedgerFacts) -> Vec<String> {
    let tally = &linkage.tally;
    let mut findings = Vec::new();

    let split = tally.from_machine + tally.by_hand + tally.singles;
    if tally.target != split {
        findings.push(format!(
            "{LINKAGE_MD}: 恒等式 1（target = from_machine + by_hand + singles）が成り立たない: {} ≠ {} + {} + {} = {split}",
            tally.target, tally.from_machine, tally.by_hand, tally.singles
        ));
    }

    let by_domain: usize = tally.singles_by_domain.values().sum();
    if by_domain != tally.singles {
        findings.push(format!(
            "{LINKAGE_MD}: 恒等式 2（singles_by_domain の 4 欄の和 = singles）が成り立たない: {by_domain} ≠ {}",
            tally.singles
        ));
    }

    let restored = tally.target + tally.alias_excluded + tally.not_applicable_excluded;
    if restored != facts.total() {
        findings.push(format!(
            "{LINKAGE_MD}: 恒等式 3（target + alias_excluded + not_applicable_excluded = 台帳 4 本の項目数）が成り立たない: {restored} ≠ {}",
            facts.total()
        ));
    }
    findings
}

/// 判定 ⑶ の所見（7 つの腕をまとめたもの）。空なら成り立っている。
fn attribution_findings(
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
    facts: &LedgerFacts,
    machine: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<String> {
    let mut findings = disjoint_findings(linkage);
    findings.extend(coverage_findings(linkage, facts));
    findings.extend(provenance_findings(linkage, machine));
    findings.extend(excluded_findings(linkage, facts));
    findings.extend(derived_findings(linkage, briefing, ledgers, facts));
    findings.extend(tally_findings(linkage, facts));
    findings.extend(identity_findings(linkage, facts));
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
// 実データで緑（要件 11.1 ⑶）
// ---------------------------------------------------------------------------

/// ⑶ 対象 4 状態の全項目がちょうど 1 つの束に属し、由来・除外・導出・合計が揃う。
#[test]
fn the_attribution_of_every_target_item_splits_across_the_bundles() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);
    let machine = machine_bundle_members(&repo, &documents.summary_text);

    let findings = attribution_findings(
        &documents.linkage,
        &documents.briefing,
        &repo.ledgers,
        &facts,
        &machine,
    );
    assert_no_findings("linkage.md の帰属の分割が台帳と食い違う", &findings);
}

// ---------------------------------------------------------------------------
// 摂動（1 か所壊すと赤・要件 11.3）
// ---------------------------------------------------------------------------

/// 摂動 1: 構成 id を 1 つ抜くと、腕 b が**ちょうどその 1 件**を名指して赤になる。
#[test]
fn dropping_one_member_turns_the_coverage_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);

    // 壊す前は緑であること（赤の原因が摂動だと言い切るために要る）。
    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &coverage_findings(&documents.linkage, &facts),
    );

    let (name, target) = a_droppable_member(&documents.linkage, &documents.linkage_text);
    let broken = read_linkage(&drop_member(&documents.linkage_text, &target))
        .unwrap_or_else(|err| panic!("id を 1 つ抜いた写しを linkage.md として読めない: {err}"));

    let findings = coverage_findings(&broken, &facts);
    assert_single_finding(
        &findings,
        &format!("束 {name} から構成 id {target} を 1 つ抜いた"),
        &[LINKAGE_MD, &target],
    );
}

/// 摂動 2: 同じ id を 2 つの束に入れると、腕 a が**両方の束名と id** を名指して赤になる。
#[test]
fn putting_one_id_in_two_bundles_turns_the_disjoint_arm_red() {
    let documents = Documents::load();

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &disjoint_findings(&documents.linkage),
    );

    let (source, host, id) = a_second_home(&documents.linkage);
    let mut broken = documents.linkage.clone();
    broken
        .bundles
        .get_mut(&host)
        .unwrap_or_else(|| panic!("束 {host} が写しに無い"))
        .members
        .push(EntryId::parse(&id).unwrap_or_else(|err| panic!("{id} を id として読めない: {err}")));

    let findings = disjoint_findings(&broken);
    assert_single_finding(
        &findings,
        &format!("{id} を束 {source} と束 {host} の両方に入れた"),
        &[LINKAGE_MD, &id, &source, &host],
    );
}

/// 摂動 3: `[tally]` の数を 1 ずらすと、腕 f が**その鍵**を名指して赤になる。
#[test]
fn shifting_the_tally_by_one_turns_the_tally_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &tally_findings(&documents.linkage, &facts),
    );

    let shifted = shift_count(&documents.linkage_text, "[tally]", "singles");
    let broken = read_linkage(&shifted)
        .unwrap_or_else(|err| panic!("合計を 1 ずらした写しを linkage.md として読めない: {err}"));

    let findings = tally_findings(&broken, &facts);
    assert_single_finding(
        &findings,
        "[tally] の singles を 1 ずらした",
        &[LINKAGE_MD, "[tally] の singles"],
    );
}

// ---------------------------------------------------------------------------
// 残る腕の較正（1 件混ぜると赤・要件 11.3）
// ---------------------------------------------------------------------------

/// 腕 c: `hand` の印を 1 つ外すと、その id が「機械由来と名乗るのに機械の束に無い」で
/// 赤になる。
#[test]
fn removing_one_hand_mark_turns_the_provenance_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let machine = machine_bundle_members(&repo, &documents.summary_text);

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &provenance_findings(&documents.linkage, &machine),
    );

    let (name, id) = a_hand_mark(&documents.linkage, &machine);
    let mut broken = documents.linkage.clone();
    broken
        .bundles
        .get_mut(&name)
        .unwrap_or_else(|| panic!("束 {name} が写しに無い"))
        .hand
        .retain(|member| member.as_str() != id);

    let findings = provenance_findings(&broken, &machine);
    assert_single_finding(
        &findings,
        &format!("束 {name} の hand から {id} を外した"),
        &[LINKAGE_MD, &name, &id],
    );
}

/// 腕 d: 除外した状態の id を 1 つ混ぜると赤になる（要件 4.3）。
#[test]
fn smuggling_one_excluded_id_turns_the_excluded_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &excluded_findings(&documents.linkage, &facts),
    );

    let id = facts.an_excluded_id().to_owned();
    let name = a_plain_bundle(&documents.linkage);
    let mut broken = documents.linkage.clone();
    broken
        .bundles
        .get_mut(&name)
        .unwrap_or_else(|| panic!("束 {name} が写しに無い"))
        .members
        .push(EntryId::parse(&id).unwrap_or_else(|err| panic!("{id} を id として読めない: {err}")));

    let findings = excluded_findings(&broken, &facts);
    assert_single_finding(
        &findings,
        &format!("束 {name} に除外した id {id} を混ぜた"),
        &[LINKAGE_MD, &name, &id],
    );
}

/// 腕 e: `themes` を 1 つ落とすと、導出との食い違いで赤になる。
#[test]
fn dropping_one_theme_turns_the_derived_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &derived_findings(
            &documents.linkage,
            &documents.briefing,
            &repo.ledgers,
            &facts,
        ),
    );

    let name = a_themed_bundle(&documents.linkage);
    let mut broken = documents.linkage.clone();
    let dropped = broken
        .bundles
        .get_mut(&name)
        .unwrap_or_else(|| panic!("束 {name} が写しに無い"))
        .themes
        .pop()
        .expect("テーマを持つ束を選んだのに themes が空");

    let findings = derived_findings(&broken, &documents.briefing, &repo.ledgers, &facts);
    assert_single_finding(
        &findings,
        &format!("束 {name} から themes の {dropped} を落とした"),
        &[LINKAGE_MD, &name, &dropped],
    );
}

/// 腕 e: 「該当なし」の腕が母数 0 の恒真でないこと。
///
/// 実データの 67 束はすべて「黙って壊れる」なので、この腕は**実データでは 1 件も
/// 相手にしていない**。写しに「該当なし」の束を 1 つ作り、その構成 id に実装済みで
/// ないものが混じっていれば赤になることを見る。これを置かないと、腕が壊れていても
/// 実データは緑のままである。
#[test]
fn marking_a_bundle_not_applicable_turns_the_derived_arm_red() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);

    assert!(
        documents
            .linkage
            .bundles
            .values()
            .all(|bundle| bundle.breakage != Breakage::NotApplicable),
        "実データに「該当なし」の束がある——この較正の前提（母数 0）が変わった"
    );

    let (name, unfinished) = a_bundle_with_an_unfinished_member(&documents.linkage, &facts);
    let mut broken = documents.linkage.clone();
    broken
        .bundles
        .get_mut(&name)
        .unwrap_or_else(|| panic!("束 {name} が写しに無い"))
        .breakage = Breakage::NotApplicable;

    let findings: Vec<String> =
        derived_findings(&broken, &documents.briefing, &repo.ledgers, &facts)
            .into_iter()
            .filter(|line| line.contains("該当なし"))
            .collect();
    assert!(
        !findings.is_empty(),
        "束 {name} を「該当なし」にしたのに赤にならない（腕が母数 0 の恒真）"
    );
    assert!(
        findings.iter().all(|line| line.contains(&name)),
        "失敗の本文が束名を名指していない:\n{}",
        findings.join("\n")
    );
    assert!(
        findings.iter().any(|line| line.contains(&unfinished)),
        "実装済みでない構成 id {unfinished} を名指していない:\n{}",
        findings.join("\n")
    );
}

/// 腕 g: 恒等式の腕が働くこと（`singles_by_domain` を 1 ずらす）。
#[test]
fn shifting_one_domain_of_the_singles_breaks_the_second_identity() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let facts = LedgerFacts::collect(&repo.ledgers);

    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &identity_findings(&documents.linkage, &facts),
    );

    let mut broken = documents.linkage.clone();
    *broken
        .tally
        .singles_by_domain
        .get_mut(&Domain::Property)
        .expect("[tally.singles_by_domain] に property の欄が無い") += 1;

    let findings = identity_findings(&broken, &facts);
    assert_single_finding(
        &findings,
        "[tally.singles_by_domain] の property を 1 ずらした",
        &[LINKAGE_MD, "singles_by_domain"],
    );
}

// ---------------------------------------------------------------------------
// 摂動の的（選び方も較正する）
// ---------------------------------------------------------------------------

/// 抜いてよい構成 id を 1 つ選ぶ（束名, id）。
///
/// 条件は 3 つ。⑴ 単独項目でない束の id（単独項目は `members` が空になって読み手が
/// 落とす）、⑵ その束が 2 つ以上の id を持つ（同じ理由）、⑶ `hand` に印が無い
/// （`hand` だけ残ると読み手が「hand の id が members に無い」で落ちる）。さらに
/// `drop_member` が要求する「本文に引用符付きでちょうど 1 度」も満たすものを採る。
fn a_droppable_member(linkage: &Linkage, text: &str) -> (String, String) {
    for (name, bundle) in &linkage.bundles {
        if bundle.single || bundle.members.len() < 2 {
            continue;
        }
        for member in &bundle.members {
            let id = member.as_str();
            if bundle.hand.iter().any(|hand| hand.as_str() == id) {
                continue;
            }
            if text.matches(&format!("\"{id}\"")).count() == 1 {
                return (name.clone(), id.to_owned());
            }
        }
    }
    panic!("抜いてよい構成 id が 1 つも無い");
}

/// 「2 つ目の住処」を選ぶ——ある束の id を、別の束へ入れる組み合わせ。
///
/// 返すのは（元の束名, 入れる先の束名, id）。腕 a は束名の文字順で最初に見つけた束を
/// 「元」と呼ぶので、入れる先は元より後の束から採る——順を決めておかないと、失敗の
/// 本文の 2 つの束名が入れ替わって読み手が直す場所を取り違える。
fn a_second_home(linkage: &Linkage) -> (String, String, String) {
    let names: Vec<&String> = linkage.bundles.keys().collect();
    let source = names.first().expect("束が 1 つも無い");
    let id = linkage.bundles[*source]
        .members
        .first()
        .expect("束の members が空")
        .as_str()
        .to_owned();
    let host = names
        .iter()
        .skip(1)
        .find(|name| !linkage.bundles[**name].single)
        .expect("名前付き束が 2 つ以上無い");
    ((*source).clone(), (*host).clone(), id)
}

/// 人手の印が付いていて、機械の束の構成 id には現れない id を 1 つ選ぶ（束名, id）。
fn a_hand_mark(
    linkage: &Linkage,
    machine: &BTreeMap<String, BTreeSet<String>>,
) -> (String, String) {
    for (name, bundle) in &linkage.bundles {
        let known = machine_members_of(bundle.machine.iter().map(EntryId::as_str), machine);
        for hand in &bundle.hand {
            if !known.contains(hand.as_str()) {
                return (name.clone(), hand.as_str().to_owned());
            }
        }
    }
    panic!("人手の印が 1 つも無い（腕 c を較正できない）");
}

/// 単独項目でない束を 1 つ選ぶ。
fn a_plain_bundle(linkage: &Linkage) -> String {
    linkage
        .bundles
        .iter()
        .find(|(_, bundle)| !bundle.single)
        .map(|(name, _)| name.clone())
        .expect("名前付き束が 1 つも無い")
}

/// `themes` を 1 つ以上持つ束を選ぶ。
fn a_themed_bundle(linkage: &Linkage) -> String {
    linkage
        .bundles
        .iter()
        .find(|(_, bundle)| !bundle.themes.is_empty())
        .map(|(name, _)| name.clone())
        .expect("themes を持つ束が 1 つも無い")
}

/// 実装済みでない構成 id を持つ束を 1 つ選ぶ（束名, その id）。
fn a_bundle_with_an_unfinished_member(linkage: &Linkage, facts: &LedgerFacts) -> (String, String) {
    for (name, bundle) in &linkage.bundles {
        for member in &bundle.members {
            if facts.status_of(member.as_str()) != Some(Status::Implemented) {
                return (name.clone(), member.as_str().to_owned());
            }
        }
    }
    panic!("実装済みでない構成 id を持つ束が 1 つも無い");
}

/// 報告から読んだ「束 id → 構成 id」が、実データで空でないこと。
#[test]
fn the_machine_bundle_members_are_read_from_the_reports() {
    let repo = RepoData::load();
    let documents = Documents::load();
    let machine = machine_bundle_members(&repo, &documents.summary_text);

    assert!(
        !machine.is_empty(),
        "報告 5 本から機械の束を 1 つも読めていない"
    );
    assert!(
        machine.values().all(|members| !members.is_empty()),
        "構成 id が 1 つも無い機械の束がある（表の最終列を読めていない）"
    );
}
