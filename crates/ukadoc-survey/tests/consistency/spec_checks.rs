//! 判定 ⑸（spec ディレクトリと宛先の整合・`roadmap-draft.md`）が実データで緑である
//! ことと、1 か所壊すと赤になることの主張（要件 11.1 ⑸・11.3・11.4・12.8・
//! 設計「判定の一覧」⑸ の a〜f）。
//!
//! # 6 つの腕
//!
//! - **a** `[briefs].count` ＝ `[[spec]]` の行数。
//! - **b** `[[spec]]` の各名前が `.kiro/specs/` の直下か `completed/` に実在する。
//! - **c** `[[spec]].owner_count` ＝ 台帳 4 本でその名前を宛先に持つ項目の数。
//! - **d** `[[spec]]`・`[[reserved]]` の `bundle` が `linkage.md` の束名に在る
//!   （`none = true` の行は束を持たないので相手にしない）。
//! - **e** `briefing.md` の `[[owner_completed]]` の spec 名が `completed/` に実在し、
//!   その名前を宛先に持つ項目の状態がすべて実装済みか縮退で、`items` が数え直しと合う。
//! - **f** 台帳の非空の宛先がすべて `[[spec]]` の名前か `[[owner_completed]]` の名前の
//!   いずれかである（要件 7.4 ⑶: brief の無い候補 spec 名を宛先に書かない）。
//!
//! # 他 spec の起票・完了で赤にならない（設計 D-6・「⑸ の数え方」）
//!
//! 腕 b が主張するのは**表の各名前の実在**だけで、生きたディレクトリの総数との一致は
//! 主張しない。総数と比べると他 spec が brief を 1 本足すたびにここが赤くなり、
//! `roadmap-draft.md` が全 spec の共有ファイルになる（W13「共有ファイル 0」）。
//! 赤になるのは表の spec が**改名・削除**されたときだけで、そのときは表を直す。
//! [`adding_another_spec_directory_stays_green_and_removing_a_listed_one_turns_red`]
//! がその両側を較正する。
//!
//! # 本 spec 自身の**パス**を書かない（要件 12.8）
//!
//! 一覧から自分を除くのに使うのはディレクトリ**名**（`documents::OWN_SPEC_DIR`）だけ
//! である。`/kiro-complete` が `completed/` へ移す際に書き換えるのは実ファイル読みの
//! パスなので、名前だけを持つこの一群は移動の前後で同じ結果を返す。
//!
//! # 数えたもの（2026-09-13 の実測）
//!
//! - `[[spec]]`: **27 行**（`[briefs].count` と一致）。うち束を持つ 13 行・
//!   `none = true` の 14 行。
//! - `[[reserved]]`: **18 行**（うち `none = true` が 7 行）。
//! - `[[owner_completed]]`: **14 行**・`items` の合計 **41 件**。
//! - 台帳 4 本の非空の宛先: **415 件**・**27 種**（`[[spec]]` の 13 種と
//!   `[[owner_completed]]` の 14 種でちょうど尽きる）。
//! - `.kiro/specs/completed/`: **174 本**（数え方: 直下の**ディレクトリ**を数えた。
//!   直下には単独の `.md` が 1 本混じっているので、`ls` の行数 175 とは 1 ずれる）。
//!
//! # 読むだけ
//!
//! 摂動は本文の**写し**（`String`）か、読み終えた骨組み・台帳の**写し**
//! （[`RoadmapDraft`]・[`Briefing`]・[`Ledger`]）の上でだけ働かせる。repo のファイルには
//! 1 バイトも触れない。

use std::collections::{BTreeMap, BTreeSet};

use ukadoc_survey::documents::parse::read_roadmap_draft;
use ukadoc_survey::documents::{Briefing, BundleRef, Linkage, RoadmapDraft};
use ukadoc_survey::ledger::Ledger;
use ukadoc_survey::model::Status;

use super::RepoData;
use super::briefing_arms::{BRIEFING_MD, ledger_file};
use super::documents::{Documents, shift_count};

/// 失敗の本文に載る `roadmap-draft.md` の置き場。
const ROADMAP_MD: &str = "doc/ukadoc-coverage/roadmap-draft.md";

/// 失敗の本文に載る `linkage.md` の置き場。
const LINKAGE_MD: &str = "doc/ukadoc-coverage/linkage.md";

// ---------------------------------------------------------------------------
// 台帳の宛先の数え直し
// ---------------------------------------------------------------------------

/// 台帳 4 本の**非空の宛先**を 1 度だけ畳んだもの（腕 c・e・f の出どころ）。
///
/// 腕ごとに台帳を歩き直すと同じ数え方が 3 か所へ散らばり、片方だけ直したときに
/// 気づけない（`briefing_arms::LedgerFacts` と同じ理由）。
struct OwnerFacts {
    /// 宛先の名前 → その名前を持つ項目（台帳の置き場・id の綴り・状態）。
    by_owner: BTreeMap<String, Vec<OwnedItem>>,
}

/// 宛先を持つ項目 1 件。
struct OwnedItem {
    /// 台帳の置き場（失敗の本文に載る）。
    file: String,
    /// 項目 id の綴り（失敗の本文に載る）。
    id: String,
    /// 状態（腕 e が実装済みか縮退であることを見る）。
    status: Status,
}

impl OwnerFacts {
    /// 台帳 4 本から畳む。空文字列の宛先は持たない。
    fn collect(ledgers: &[Ledger]) -> Self {
        let mut by_owner: BTreeMap<String, Vec<OwnedItem>> = BTreeMap::new();
        for ledger in ledgers {
            let file = ledger_file(ledger.domain);
            for (id, entry) in &ledger.entries {
                if entry.owner.is_empty() {
                    continue;
                }
                by_owner
                    .entry(entry.owner.clone())
                    .or_default()
                    .push(OwnedItem {
                        file: file.clone(),
                        id: id.as_str().to_owned(),
                        status: entry.status,
                    });
            }
        }
        Self { by_owner }
    }

    /// その名前を宛先に持つ項目の数。
    fn count(&self, owner: &str) -> usize {
        self.by_owner.get(owner).map_or(0, Vec::len)
    }

    /// その名前を宛先に持つ項目。
    fn items(&self, owner: &str) -> &[OwnedItem] {
        self.by_owner.get(owner).map_or(&[], Vec::as_slice)
    }

    /// 現れた宛先の名前（文字順）。
    fn owners(&self) -> impl Iterator<Item = &str> {
        self.by_owner.keys().map(String::as_str)
    }
}

// ---------------------------------------------------------------------------
// 腕 a: 冒頭の数と表の行数
// ---------------------------------------------------------------------------

/// 腕 a: `[briefs].count` が `[[spec]]` の行数と一致すること。
fn brief_count_findings(roadmap: &RoadmapDraft) -> Vec<String> {
    let rows = roadmap.specs.len();
    if roadmap.briefs.count == rows {
        return Vec::new();
    }
    vec![format!(
        "{ROADMAP_MD}: [briefs].count は {} と書いてあるが、[[spec]] は {rows} 行",
        roadmap.briefs.count
    )]
}

// ---------------------------------------------------------------------------
// 腕 b: 表の各名前の実在
// ---------------------------------------------------------------------------

/// 腕 b: `[[spec]]` の各名前が `spec_dirs ∪ completed_specs` に在ること。
///
/// **総数とは比べない**（設計 D-6）。比べるのは表に書いた名前 1 つずつの実在だけで、
/// 一覧の側に知らない名前が増えても何も言わない。
fn spec_existence_findings(
    roadmap: &RoadmapDraft,
    spec_dirs: &BTreeSet<String>,
    completed_specs: &BTreeSet<String>,
) -> Vec<String> {
    roadmap
        .specs
        .iter()
        .filter(|row| !spec_dirs.contains(&row.name) && !completed_specs.contains(&row.name))
        .map(|row| {
            format!(
                "{ROADMAP_MD}: [[spec]] {} のディレクトリが .kiro/specs/ の直下にも \
                 completed/ にも無い（改名か削除。表を直す）",
                row.name
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 腕 c: 宛先の数
// ---------------------------------------------------------------------------

/// 腕 c: `owner_count` が台帳の数え直しと一致すること。
fn owner_count_findings(roadmap: &RoadmapDraft, owners: &OwnerFacts) -> Vec<String> {
    roadmap
        .specs
        .iter()
        .filter_map(|row| {
            let actual = owners.count(&row.name);
            (row.owner_count != actual).then(|| {
                format!(
                    "{ROADMAP_MD}: [[spec]] {} の owner_count は {} と書いてあるが、\
                     台帳 4 本の数え直しは {actual}",
                    row.name, row.owner_count
                )
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 腕 d: 束名の実在
// ---------------------------------------------------------------------------

/// 腕 d: `[[spec]]`・`[[reserved]]` の `bundle` が `linkage.md` に在ること。
///
/// `none = true` の行は束を持たない（要件 11.5 により理由付きで明示されている）ので
/// 相手にしない。
fn bundle_reference_findings(roadmap: &RoadmapDraft, linkage: &Linkage) -> Vec<String> {
    let named = roadmap
        .specs
        .iter()
        .map(|row| (format!("[[spec]] {}", row.name), &row.bundle))
        .chain(
            roadmap
                .reserved
                .iter()
                .map(|row| (format!("[[reserved]] {}", row.name), &row.bundle)),
        );

    let mut findings = Vec::new();
    for (place, bundle) in named {
        let BundleRef::Named(name) = bundle else {
            continue;
        };
        if !linkage.bundles.contains_key(name) {
            findings.push(format!(
                "{ROADMAP_MD}: {place} が指す束 {name} が {LINKAGE_MD} に無い"
            ));
        }
    }
    findings
}

// ---------------------------------------------------------------------------
// 腕 e: 完了済み spec を宛先に残した行
// ---------------------------------------------------------------------------

/// 腕 e: `[[owner_completed]]` の spec が `completed/` に実在し、その名前を宛先に持つ
/// 項目の状態がすべて実装済みか縮退で、`items` が数え直しと合うこと（要件 7.4 ⑵）。
///
/// **生きた `completed/` の全走査はしない**——列挙した名前だけを引く。全走査にすると
/// 他 spec が完了するたびにここが赤くなる。
fn owner_completed_findings(
    briefing: &Briefing,
    completed_specs: &BTreeSet<String>,
    owners: &OwnerFacts,
) -> Vec<String> {
    let mut findings = Vec::new();
    for row in &briefing.owner_completed {
        let place = format!("[[owner_completed]] {}", row.spec);
        if !completed_specs.contains(&row.spec) {
            findings.push(format!(
                "{BRIEFING_MD}: {place} のディレクトリが .kiro/specs/completed/ に無い"
            ));
        }
        let items = owners.items(&row.spec);
        if row.items != items.len() {
            findings.push(format!(
                "{BRIEFING_MD}: {place} の items は {} と書いてあるが、\
                 台帳 4 本の数え直しは {}",
                row.items,
                items.len()
            ));
        }
        for item in items {
            if !matches!(item.status, Status::Implemented | Status::Degraded) {
                findings.push(format!(
                    "{BRIEFING_MD}: {place} を宛先に持つ {} の {} が {} である\
                     （完了済み spec に残す宛先は実装済みか縮退だけ・要件 7.4 ⑵）",
                    item.file,
                    item.id,
                    item.status.as_key()
                ));
            }
        }
    }
    findings
}

// ---------------------------------------------------------------------------
// 腕 f: 宛先の行き先
// ---------------------------------------------------------------------------

/// 腕 f: 台帳の非空の宛先がすべて `[[spec]]` の名前か `[[owner_completed]]` の名前で
/// あること（要件 7.4 ⑶）。
fn owner_destination_findings(
    roadmap: &RoadmapDraft,
    briefing: &Briefing,
    owners: &OwnerFacts,
) -> Vec<String> {
    let listed: BTreeSet<&str> = roadmap
        .specs
        .iter()
        .map(|row| row.name.as_str())
        .chain(briefing.owner_completed.iter().map(|row| row.spec.as_str()))
        .collect();

    owners
        .owners()
        .filter(|owner| !listed.contains(owner))
        .map(|owner| {
            let item = &owners.items(owner)[0];
            format!(
                "{}: {} の宛先 {owner} が {ROADMAP_MD} の [[spec]] にも \
                 {BRIEFING_MD} の [[owner_completed]] にも無い（要件 7.4 ⑶）",
                item.file, item.id
            )
        })
        .collect()
}

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

/// 実データ一式（6 つの腕がどれも同じものを見る）。
struct Fixture {
    repo: RepoData,
    documents: Documents,
    owners: OwnerFacts,
}

impl Fixture {
    fn load() -> Self {
        let repo = RepoData::load();
        let documents = Documents::load();
        let owners = OwnerFacts::collect(&repo.ledgers);
        Self {
            repo,
            documents,
            owners,
        }
    }

    /// 6 つの腕の所見をまとめて集める。
    fn findings(&self) -> Vec<String> {
        let Self {
            documents, owners, ..
        } = self;
        [
            brief_count_findings(&documents.roadmap),
            spec_existence_findings(
                &documents.roadmap,
                &documents.spec_dirs,
                &documents.completed_specs,
            ),
            owner_count_findings(&documents.roadmap, owners),
            bundle_reference_findings(&documents.roadmap, &documents.linkage),
            owner_completed_findings(&documents.briefing, &documents.completed_specs, owners),
            owner_destination_findings(&documents.roadmap, &documents.briefing, owners),
        ]
        .concat()
    }
}

// ---------------------------------------------------------------------------
// 実データで緑（要件 11.1 ⑸）
// ---------------------------------------------------------------------------

/// ⑸ spec 表・予約群・完了済みの宛先が、ディレクトリと台帳と帰属に揃う（腕 a〜f）。
#[test]
fn the_spec_table_agrees_with_the_directories_the_ledgers_and_the_bundles() {
    let fixture = Fixture::load();
    assert_no_findings("判定 ⑸ が赤い", &fixture.findings());
}

// ---------------------------------------------------------------------------
// 摂動 3 本（1 か所壊すと赤・要件 11.3）
// ---------------------------------------------------------------------------

/// 摂動 ⑴ 冒頭の数を 1 ずらすと、腕 a が両方の数を挙げて赤になる。
///
/// 壊すのは**本文の写し**である（骨組みの写しではなく）——`[briefs].count` は
/// 読み手が最初に目にする数なので、綴りの上で 1 ずれたら赤くなることを見る。
#[test]
fn shifting_the_brief_count_turns_the_count_arm_red() {
    let documents = Documents::load();
    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &brief_count_findings(&documents.roadmap),
    );

    let shifted = shift_count(&documents.roadmap_text, "[briefs]", "count");
    let broken = read_roadmap_draft(&shifted)
        .unwrap_or_else(|err| panic!("1 ずらした写しを読めない: {err}"));

    let rows = documents.roadmap.specs.len();
    assert_single_finding(
        &brief_count_findings(&broken),
        "[briefs].count を 1 ずらした",
        &[
            ROADMAP_MD,
            "[briefs].count",
            &(rows + 1).to_string(),
            &rows.to_string(),
        ],
    );
}

/// 摂動 ⑵ spec 名を 1 文字変えると、腕 b がその名前を挙げて赤になる。
#[test]
fn twisting_one_spec_name_turns_the_existence_arm_red() {
    let documents = Documents::load();
    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &spec_existence_findings(
            &documents.roadmap,
            &documents.spec_dirs,
            &documents.completed_specs,
        ),
    );

    let mut broken = documents.roadmap.clone();
    let row = broken.specs.first_mut().expect("[[spec]] が 1 行も無い");
    let twisted = format!("{}X", row.name);
    row.name.clone_from(&twisted);

    assert_single_finding(
        &spec_existence_findings(&broken, &documents.spec_dirs, &documents.completed_specs),
        "spec 名を 1 文字変えた",
        &[ROADMAP_MD, &twisted],
    );
}

/// 摂動 ⑶ 宛先を 1 件書き換えると、腕 c と腕 f がそれぞれ赤になる。
///
/// 書き換え先はどちらの一覧にも無い名前にする。こうすると ⑴ もとの spec の
/// `owner_count` が 1 件足りなくなり（腕 c）、⑵ 行き先の無い宛先が 1 件できる（腕 f）。
/// 2 つの腕が別々の壊れ方を見ていることは、それぞれちょうど 1 件で赤くなることで分かる。
#[test]
fn rewriting_one_owner_turns_the_count_and_destination_arms_red() {
    let fixture = Fixture::load();
    assert_no_findings("摂動の前から赤い", &fixture.findings());

    let mut ledgers = fixture.repo.ledgers.clone();
    let stranger = "areka-P0-not-a-spec";
    let (file, id, was) = {
        let ledger = ledgers
            .iter_mut()
            .find(|ledger| ledger.entries.values().any(|entry| !entry.owner.is_empty()))
            .expect("非空の宛先を持つ台帳が無い");
        let file = ledger_file(ledger.domain);
        let (id, entry) = ledger
            .entries
            .iter_mut()
            .find(|(_, entry)| !entry.owner.is_empty())
            .expect("非空の宛先を持つ項目が無い");
        let was = std::mem::replace(&mut entry.owner, stranger.to_owned());
        (file, id.as_str().to_owned(), was)
    };
    let broken = OwnerFacts::collect(&ledgers);

    assert_single_finding(
        &owner_count_findings(&fixture.documents.roadmap, &broken),
        &format!("{id} の宛先を {was} から書き換えた"),
        &[ROADMAP_MD, &was],
    );
    assert_single_finding(
        &owner_destination_findings(
            &fixture.documents.roadmap,
            &fixture.documents.briefing,
            &broken,
        ),
        &format!("{id} の宛先を {stranger} へ書き換えた"),
        &[&file, &id, stranger],
    );
}

// ---------------------------------------------------------------------------
// 残る腕の較正（1 か所壊すと赤・要件 11.3）
// ---------------------------------------------------------------------------

/// 腕 b: 他 spec のディレクトリが増えても緑のまま・表の名前が消えると赤（設計 D-6）。
///
/// 一覧の写しに知らない名前を足すのが「他 spec の起票・完了」で、表に在る名前を
/// 一覧から抜くのが「改名・削除」である。前者で赤くなる書き方（総数との一致）は
/// この一群を全 spec の共有ファイルにしてしまう。
#[test]
fn adding_another_spec_directory_stays_green_and_removing_a_listed_one_turns_red() {
    let documents = Documents::load();

    let mut grown = documents.spec_dirs.clone();
    assert!(
        grown.insert("areka-P0-a-brand-new-spec".to_owned()),
        "足そうとした名前が既に一覧にある（較正が空振りする）"
    );
    assert_no_findings(
        "他 spec のディレクトリが 1 本増えただけで赤くなった",
        &spec_existence_findings(&documents.roadmap, &grown, &documents.completed_specs),
    );

    let gone = documents
        .roadmap
        .specs
        .iter()
        .map(|row| row.name.as_str())
        .find(|name| documents.spec_dirs.contains(*name))
        .expect("表の名前が 1 つも直下に無い")
        .to_owned();
    let mut shrunk = documents.spec_dirs.clone();
    assert!(shrunk.remove(&gone), "抜こうとした名前が一覧に無い");
    assert_single_finding(
        &spec_existence_findings(&documents.roadmap, &shrunk, &documents.completed_specs),
        &format!("表に在る {gone} を一覧から抜いた"),
        &[ROADMAP_MD, &gone],
    );
}

/// 腕 d: 束名を 1 文字変えると、その名前を挙げて赤になる。
#[test]
fn an_unknown_bundle_name_turns_the_bundle_arm_red() {
    let documents = Documents::load();
    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &bundle_reference_findings(&documents.roadmap, &documents.linkage),
    );

    let mut broken = documents.roadmap.clone();
    let row = broken
        .reserved
        .iter_mut()
        .find(|row| matches!(row.bundle, BundleRef::Named(_)))
        .expect("束を持つ [[reserved]] が 1 行も無い");
    let BundleRef::Named(name) = &mut row.bundle else {
        unreachable!("束を持つ行だけを選んだ")
    };
    name.push('X');
    let twisted = name.clone();
    let place = row.name.clone();

    assert_single_finding(
        &bundle_reference_findings(&broken, &documents.linkage),
        &format!("[[reserved]] {place} の束名を 1 文字変えた"),
        &[ROADMAP_MD, LINKAGE_MD, &twisted],
    );
}

/// 腕 e: 完了済みの宛先の名前を 1 文字変えると、`completed/` に無いことを名指して
/// 赤になる。
#[test]
fn a_completed_owner_outside_the_completed_directory_turns_the_completed_arm_red() {
    let fixture = Fixture::load();
    assert_no_findings(
        "摂動の前から赤い（摂動が赤の原因だと言えない）",
        &owner_completed_findings(
            &fixture.documents.briefing,
            &fixture.documents.completed_specs,
            &fixture.owners,
        ),
    );

    let mut broken = fixture.documents.briefing.clone();
    let row = broken
        .owner_completed
        .first_mut()
        .expect("[[owner_completed]] が 1 行も無い");
    let was = row.spec.clone();
    let twisted = format!("{was}X");
    row.spec.clone_from(&twisted);
    // 宛先の数え直しは元の名前のままなので、名前を変えた行は 0 件になる。
    row.items = 0;

    assert_single_finding(
        &owner_completed_findings(&broken, &fixture.documents.completed_specs, &fixture.owners),
        &format!("{was} を 1 文字変えた"),
        &[BRIEFING_MD, &twisted, "completed/"],
    );
}

/// 腕 e: 完了済みの宛先に実装済みでも縮退でもない項目が 1 件混じると赤になる。
///
/// 実データでは 41 件すべてが実装済みか縮退なので、この主張は**母数はあるが違反が
/// 0 件**である。1 件混ぜて赤にできなければ、何も見ていないのと同じ（要件 11.3）。
#[test]
fn a_completed_owner_with_an_unfinished_item_turns_the_completed_arm_red() {
    let fixture = Fixture::load();

    let spec = fixture
        .documents
        .briefing
        .owner_completed
        .first()
        .expect("[[owner_completed]] が 1 行も無い")
        .spec
        .clone();
    let mut ledgers = fixture.repo.ledgers.clone();
    let (file, id) = {
        let ledger = ledgers
            .iter_mut()
            .find(|ledger| ledger.entries.values().any(|entry| entry.owner == spec))
            .expect("その名前を宛先に持つ台帳が無い");
        let file = ledger_file(ledger.domain);
        let (id, entry) = ledger
            .entries
            .iter_mut()
            .find(|(_, entry)| entry.owner == spec)
            .expect("その名前を宛先に持つ項目が無い");
        entry.status = Status::Absent;
        (file, id.as_str().to_owned())
    };
    let broken = OwnerFacts::collect(&ledgers);

    assert_single_finding(
        &owner_completed_findings(
            &fixture.documents.briefing,
            &fixture.documents.completed_specs,
            &broken,
        ),
        &format!("{id} の状態を未対応にした"),
        &[BRIEFING_MD, &spec, &file, &id, "absent"],
    );
}

/// 腕 e: `items` を 1 ずらすと、両方の数を挙げて赤になる。
#[test]
fn shifting_one_completed_owner_items_turns_the_completed_arm_red() {
    let fixture = Fixture::load();

    let mut broken = fixture.documents.briefing.clone();
    let row = broken
        .owner_completed
        .first_mut()
        .expect("[[owner_completed]] が 1 行も無い");
    row.items += 1;
    let spec = row.spec.clone();
    let declared = row.items;

    assert_single_finding(
        &owner_completed_findings(&broken, &fixture.documents.completed_specs, &fixture.owners),
        &format!("{spec} の items を 1 ずらした"),
        &[
            BRIEFING_MD,
            &spec,
            &declared.to_string(),
            &(declared - 1).to_string(),
        ],
    );
}
