//! 判定 ⑷（段階と順位・`briefing.md`）の腕 a〜g と、台帳から数え直す道具
//! （要件 11.1 ⑷・5.3・5.6・6.7・7.1〜7.3・設計「判定の一覧」・D-9）。
//!
//! # ⑷ が主張すること
//!
//! - **a 順位表の被覆**: 名前付き束（単独項目を含む）の全数が `[[rank]]` にちょうど
//!   1 度現れ、`[[rank]]` が指す束名がすべて `linkage.md` に在る。
//! - **b 段階ごとの 3 つの数**: `[stage.X]` の `bundles`・`singles`・`items` が
//!   数え直しと一致する（A〜E の 5 段階すべて。0 も比べる）。
//! - **c 台帳の優先度**: 台帳の全項目の `priority` が `derive::priorities` と一致する
//!   （`alias`・`not-applicable` は空文字）。
//! - **d 順位の行の導出値**: `[[rank]]` の `assets`・`shared` が `derive` と一致する。
//! - **e 並び**: 同じ段階の対象行（`override` も `insufficient` も持たない行）が
//!   比較鍵の降順に並び、鍵が等しければ同じ `rank`・異なれば異なる `rank` で、1 から
//!   密に増える。`singles` の行は並べた id の鍵がすべて等しい。`override` の
//!   `kind`・`ref` は受け付け形に収まる。`insufficient` の行は同じ段階の対象行より後。
//! - **f 書き戻し後の分布**: `[[barrier]]`・`[[after]]`・`[priority_blank]` の数が
//!   台帳の数え直しと一致する。
//! - **g 人の決め**: 段階 B の `rank` 1 の束の `themes` に「更新」が含まれ、段階 C の
//!   最大 `rank` の束の `members` に `system.` を含む id がある（要件 5.3 の釘付け）。
//!
//! 腕を関数に割ってあるのは摂動のためである（`linkage_checks.rs` と同じ流儀）。
//! 1 か所壊すと複数の腕が同時に赤くなるので、摂動は腕ごとに「ちょうど 1 件」を見る
//! （要件 11.3。その主張は `briefing_checks.rs` にある）。
//!
//! # 導出は二重に実装しない（設計 D-4）
//!
//! 腕 c は `derive::priorities` の戻り値と、腕 d・e は `derive::derive_bundle` と
//! `derive::axis_key` の戻り値と突き合わせる。ここで優先度を組み直したりテーマを
//! 数え直したりはしない。
//!
//! # `[[barrier]]` は「ページの全項目」を数える
//!
//! 6 行が並べるのは**ページ 1 つの状態分布**（要件 8.1 ⑷ の着手時の実測・設計 D-2）で
//! あって、段階 A に落ちた項目だけの分布ではない。2026-09-12 の実測で 6 行の合計は
//! **924** 件、段階 A の項目は **874** 件で、両者は別の数である。段階 A の側で数えると
//! 6 行とも赤になる。
//!
//! # 実データでは母数 0 の腕が 3 つある（だから較正を置く）
//!
//! 2026-09-12 時点で、`override` の `kind` は `stage-rule` が 2 行だけ（`second-stage`
//! は **0 行**）、`insufficient = true` の行は **0 行**、`singles` の行は 3 行のうち
//! 2 行が id 1 つ（鍵を比べる相手が無い）である。どれも腕がそのままでは恒真なので、
//! 写しに 1 行作って赤になることを確かめる。
//!
//! # 63 束すべての壊れ方が同値である（順位に同着が多い）
//!
//! `breakage` は 63 束とも「黙って壊れる」なので、比較鍵の第 1 根拠は全束で同じ値に
//! なる。実質の順序は テーマ数 → 資産 → 共有度 の 3 段で決まり、同順位が大量に出る。
//! 腕 e が「鍵が等しければ同じ `rank`」を主張するのはそのためで、順序だけを見て
//! 「同着を許さない」と読むと実データが赤くなる。
//!
//! # テストの本体はここに 1 つも置かない
//!
//! `documents.rs`・`perturb.rs` と同じ流儀で、ここにあるのは腕そのものと数え直しの
//! 道具だけである。実データで緑であることの主張・摂動 3 本・腕ごとの較正は兄弟の
//! `briefing_checks.rs` にある。2 つに割ってあるのは 1 ファイル 1,000 行の目安
//! （`structure.md:176`・設計「1,000 行の番人」）を守るためで、腕と摂動を同じ
//! ファイルに置くと 1,200 行を超える。

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};

use ukadoc_survey::documents::derive::{Derived, axis_key, derive_bundle, priorities};
use ukadoc_survey::documents::{
    Barrier, Briefing, Linkage, NamedBundle, OverrideKind, RankRow, RankTarget, Stage,
};
use ukadoc_survey::io::{files, paths};
use ukadoc_survey::ledger::Ledger;
use ukadoc_survey::model::{Domain, EntryId, PageName, Status};

/// 段階と順位の正本の相対パス（失敗の本文はこの綴りで名指す）。
pub(super) const BRIEFING_MD: &str = "doc/ukadoc-coverage/briefing.md";

/// 帰属の正本の相対パス（腕 a が「その束名が無い」と言うときの相手）。
const LINKAGE_MD: &str = "doc/ukadoc-coverage/linkage.md";

/// 要件 5.3 が段階 B の先頭に置くと決めたテーマ。
pub(super) const UPDATE_THEME: &str = "更新";

/// 要件 5.3 が段階 C の末尾に置くと決めた項目 id の目印。
pub(super) const SYSTEM_MARK: &str = "system.";

/// `override` の `kind = "stage-rule"` に許す `ref`（設計 D-2 の受け付け形）。
pub(super) const STAGE_RULE_REF: &str = "要件 5.3";

/// `second-stage` の `ref` に許す適合検証項目の番号の上限（要件 9.1 の 20 項目）。
const CONFORMANCE_ITEMS: usize = 20;

/// 持ち越し 8 行の出どころ（要件 9.1・9.2。`second-stage` の `ref` のもう一方の形）。
const CARRYOVER_DOC: &str =
    ".kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md";

/// 持ち越し行の数（要件 9.1）。取り出しがこの数を返さなければ読み方か出どころが
/// 変わっている——黙って短い一覧を返すと `ref` の受け付け形が緩む。
const CARRYOVER_ROWS: usize = 8;

// ---------------------------------------------------------------------------
// 台帳から数え直す側
// ---------------------------------------------------------------------------

/// 台帳の 1 項目のうち、この判定が見る 4 つの欄。
struct Fact {
    domain: Domain,
    status: Status,
    page: PageName,
    /// 台帳に**書いてある** `priority`（導出ではない。腕 c が両者を突き合わせる）。
    priority: String,
}

/// 台帳 4 本の項目（数え直しの出どころ）。
///
/// 台帳を 1 度だけ畳んで持ち、数え方は [`Self::count`] 1 つに集める。腕ごとに歩き
/// 直すと、同じ数え方が腕の数だけ散らばって、片方だけ直したときに気づけない。
pub(super) struct LedgerFacts {
    rows: Vec<Fact>,
}

impl LedgerFacts {
    /// 台帳 4 本から畳む。
    pub(super) fn collect(ledgers: &[Ledger]) -> Self {
        let rows = ledgers
            .iter()
            .flat_map(|ledger| {
                ledger.entries.iter().map(|(id, entry)| Fact {
                    domain: ledger.domain,
                    status: entry.status,
                    page: id.page(),
                    priority: entry.priority.clone(),
                })
            })
            .collect();
        Self { rows }
    }

    /// 条件に合う項目の数。
    fn count(&self, keep: impl Fn(&Fact) -> bool) -> usize {
        self.rows.iter().filter(|fact| keep(fact)).count()
    }

    /// `priority` がその段階の 1 文字で始まる項目数（腕 b の `items`）。
    fn items_of(&self, stage: Stage) -> usize {
        self.count(|fact| fact.priority.starts_with(stage.as_key()))
    }
}

/// 順位の行が指す束名（`bundle` は 1 つ・`singles` は並べた id の綴り）。
pub(super) fn row_names(row: &RankRow) -> Vec<&str> {
    match &row.target {
        RankTarget::Bundle(name) => vec![name.as_str()],
        RankTarget::Singles(ids) => ids.iter().map(EntryId::as_str).collect(),
    }
}

/// 失敗の本文でその行を名指す綴り（段階と順位。行番号は使わない・要件 12.6）。
pub(super) fn row_place(row: &RankRow) -> String {
    format!("[[rank]] {}{}", row.stage.as_key(), row.rank)
}

/// 束 1 つの導出（帰属にその名前が無ければ `None`。無いことは腕 a が名指す）。
fn derived_of<'a>(
    name: &str,
    linkage: &'a Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
) -> Option<(&'a NamedBundle, Derived)> {
    let bundle = linkage.bundles.get(name)?;
    Some((bundle, derive_bundle(bundle, linkage, briefing, ledgers)))
}

/// 数が数え直しと合わないことの本文。
fn count_mismatch(what: &str, declared: usize, actual: usize) -> String {
    format!("{BRIEFING_MD}: {what} は {declared} と書いてあるが、数え直しは {actual}")
}

/// その台帳の場所（失敗の本文に載る綴り）。
pub(super) fn ledger_file(domain: Domain) -> String {
    format!("doc/ukadoc-coverage/ledger/{}.toml", domain.as_key())
}

// ---------------------------------------------------------------------------
// 腕 a: 順位表の被覆
// ---------------------------------------------------------------------------

/// 腕 a: 束の全数が `[[rank]]` にちょうど 1 度現れ、行が指す束名が帰属に在ること。
pub(super) fn rank_coverage_findings(linkage: &Linkage, briefing: &Briefing) -> Vec<String> {
    let mut times: BTreeMap<&str, usize> = linkage
        .bundles
        .keys()
        .map(|name| (name.as_str(), 0))
        .collect();
    let mut findings = Vec::new();

    for row in &briefing.ranks {
        for name in row_names(row) {
            match times.get_mut(name) {
                Some(count) => *count += 1,
                None => findings.push(format!(
                    "{BRIEFING_MD}: {} が指す束 {name} が {LINKAGE_MD} に無い（順位を付ける相手は帰属の束だけ・要件 5.6）",
                    row_place(row)
                )),
            }
        }
    }

    for (name, count) in times {
        match count {
            1 => {}
            0 => findings.push(format!(
                "{BRIEFING_MD}: 束 {name} が [[rank]] に無い（束の全数が順位表にちょうど 1 度現れる・要件 5.6）"
            )),
            n => findings.push(format!(
                "{BRIEFING_MD}: 束 {name} が [[rank]] に {n} 度現れる（ちょうど 1 度・要件 5.6）"
            )),
        }
    }
    findings
}

// ---------------------------------------------------------------------------
// 腕 b: 段階ごとの 3 つの数
// ---------------------------------------------------------------------------

/// 腕 b: `[stage.X]` の 3 つの数が数え直しと一致すること（5 段階すべて・0 も比べる）。
pub(super) fn stage_count_findings(briefing: &Briefing, facts: &LedgerFacts) -> Vec<String> {
    let mut findings = Vec::new();
    for stage in Stage::ALL {
        let place = format!("[stage.{}]", stage.as_key());
        let Some(declared) = briefing.stages.get(&stage) else {
            findings.push(format!(
                "{BRIEFING_MD}: {place} が無い（A〜E の 5 つを省けない・要件 5.6）"
            ));
            continue;
        };

        let (mut bundles, mut singles) = (0, 0);
        for row in briefing.ranks.iter().filter(|row| row.stage == stage) {
            match &row.target {
                RankTarget::Bundle(_) => bundles += 1,
                RankTarget::Singles(ids) => singles += ids.len(),
            }
        }

        for (key, declared, actual) in [
            ("bundles", declared.bundles, bundles),
            ("singles", declared.singles, singles),
            ("items", declared.items, facts.items_of(stage)),
        ] {
            if declared != actual {
                findings.push(count_mismatch(
                    &format!("{place} の {key}"),
                    declared,
                    actual,
                ));
            }
        }
    }
    findings
}

// ---------------------------------------------------------------------------
// 腕 c: 台帳の優先度
// ---------------------------------------------------------------------------

/// 腕 c: 台帳の全項目の `priority` が導出と一致すること。
///
/// 導出そのものが落ちたら（帰属の無い対象項目・順位表に無い束など）、その本文を所見
/// 1 件として上げる。落ちた本文には既に id か束名と文書名が入っている。
pub(super) fn priority_findings(
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
) -> Vec<String> {
    let derived = match priorities(linkage, briefing, ledgers) {
        Ok(derived) => derived,
        Err(err) => return vec![format!("{err}")],
    };

    let mut findings = Vec::new();
    for ledger in ledgers {
        let file = ledger_file(ledger.domain);
        for (id, entry) in &ledger.entries {
            match derived.get(id) {
                None => findings.push(format!(
                    "{file}: {} の優先度を導出が返さない（台帳の全項目に値が要る・要件 7.1）",
                    id.as_str()
                )),
                Some(want) if entry.priority != *want => findings.push(format!(
                    "{file}: {} の priority は \"{}\" と書いてあるが、導出は \"{want}\"（要件 7.1・7.2）",
                    id.as_str(),
                    entry.priority
                )),
                Some(_) => {}
            }
        }
    }
    findings
}

// ---------------------------------------------------------------------------
// 腕 d: 順位の行の導出値
// ---------------------------------------------------------------------------

/// 腕 d: `[[rank]]` の `assets`・`shared` が導出と一致すること。
pub(super) fn rank_derived_findings(
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
) -> Vec<String> {
    let mut findings = Vec::new();
    for row in &briefing.ranks {
        for name in row_names(row) {
            let Some((_, derived)) = derived_of(name, linkage, briefing, ledgers) else {
                continue;
            };
            for (key, declared, actual) in [
                ("assets", row.assets, derived.assets),
                ("shared", row.shared, derived.shared),
            ] {
                if declared != actual {
                    let what = format!("{} の束 {name} の {key}", row_place(row));
                    findings.push(count_mismatch(&what, declared, actual));
                }
            }
        }
    }
    findings
}

// ---------------------------------------------------------------------------
// 腕 e: 並び
// ---------------------------------------------------------------------------

/// 4 つの根拠の比較鍵（大きいほど先）。
type AxisKey = (u8, usize, usize, usize);

/// その行の比較鍵。`singles` の行は並べた id の鍵がすべて等しくなければならない。
///
/// 帰属に無い束名（腕 a の持ち場）と、鍵の割れた `singles` の行は `None` を返す。
fn key_of(
    row: &RankRow,
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
    findings: &mut Vec<String>,
) -> Option<AxisKey> {
    let mut keys: Vec<(&str, AxisKey)> = Vec::new();
    for name in row_names(row) {
        let (bundle, derived) = derived_of(name, linkage, briefing, ledgers)?;
        let key = axis_key(
            bundle.breakage,
            derived.themes.len(),
            derived.assets,
            derived.shared,
        );
        keys.push((name, key));
    }

    let (first_name, first) = *keys.first()?;
    for (name, key) in &keys[1..] {
        if *key != first {
            findings.push(format!(
                "{BRIEFING_MD}: {} の singles に並べた {name} の鍵が {first_name} と違う（{key:?} ≠ {first:?}。鍵が違う単独項目は行を分ける・要件 6.7）",
                row_place(row)
            ));
            return None;
        }
    }
    Some(first)
}

/// 腕 e: 対象行が比較鍵の降順で密な順位に並び、例外の印が受け付け形に収まること。
pub(super) fn rank_order_findings(
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
    carryover: &Carryover,
) -> Vec<String> {
    let mut findings: Vec<String> = briefing
        .ranks
        .iter()
        .filter_map(|row| override_finding(row, carryover))
        .collect();

    for stage in Stage::ALL {
        let rows: Vec<(usize, &RankRow)> = briefing
            .ranks
            .iter()
            .enumerate()
            .filter(|(_, row)| row.stage == stage)
            .collect();
        let is_target = |row: &RankRow| row.exception.is_none() && !row.insufficient;
        let targets: Vec<&RankRow> = rows
            .iter()
            .filter(|(_, row)| is_target(row))
            .map(|(_, row)| *row)
            .collect();

        // 例外の印を持つ行は順序の主張から外れるが、`insufficient` の行だけは
        // 「同じ段階の対象行より後」に並ぶこと（設計 D-9 の退路）。
        let last_target = rows
            .iter()
            .filter(|(_, row)| is_target(row))
            .map(|(at, _)| *at)
            .next_back();
        for (at, row) in &rows {
            if row.insufficient && last_target.is_some_and(|last| *at < last) {
                findings.push(format!(
                    "{BRIEFING_MD}: {} は insufficient なのに同じ段階の対象行より前に並んでいる（対象行より後に置く・設計 D-9）",
                    row_place(row)
                ));
            }
        }

        findings.extend(dense_order_findings(
            &targets, stage, linkage, briefing, ledgers,
        ));
    }
    findings
}

/// 対象行が比較鍵の降順で 1 から密な順位に並ぶこと（腕 e の中核）。
fn dense_order_findings(
    targets: &[&RankRow],
    stage: Stage,
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
) -> Vec<String> {
    let mut findings = Vec::new();
    let mut previous: Option<(AxisKey, usize)> = None;

    for row in targets {
        let Some(key) = key_of(row, linkage, briefing, ledgers, &mut findings) else {
            continue;
        };
        match previous {
            None if row.rank != 1 => findings.push(format!(
                "{BRIEFING_MD}: 段階 {} の先頭の {} の rank が 1 でない（順位は 1 から始める・要件 7.1）",
                stage.as_key(),
                row_place(row)
            )),
            None => {}
            Some((last_key, last_rank)) => {
                if key > last_key {
                    findings.push(format!(
                        "{BRIEFING_MD}: {} の鍵 {key:?} が 1 つ前の行の鍵 {last_key:?} より大きい（比較鍵の降順に並べる・要件 6.1・設計 D-9）",
                        row_place(row)
                    ));
                } else if key == last_key && row.rank != last_rank {
                    findings.push(format!(
                        "{BRIEFING_MD}: {} の鍵が 1 つ前の行と同じなのに rank が {last_rank} から {} へ動いている（同じ鍵は同順位・要件 6.7）",
                        row_place(row),
                        row.rank
                    ));
                } else if key < last_key && row.rank != last_rank + 1 {
                    findings.push(format!(
                        "{BRIEFING_MD}: {} の rank が {} でない（同順位の次は 1 だけ増やす密な順位・要件 7.1）",
                        row_place(row),
                        last_rank + 1
                    ));
                }
            }
        }
        previous = Some((key, row.rank));
    }
    findings
}

/// 例外の印の `kind` と `ref` が受け付け形に収まること（設計 D-2・D-9）。
pub(super) fn override_finding(row: &RankRow, carryover: &Carryover) -> Option<String> {
    let exception = row.exception.as_ref()?;
    let reference = exception.reference.as_str();
    let (accepted, allowed) = match exception.kind {
        OverrideKind::StageRule => (
            reference == STAGE_RULE_REF,
            format!("\"{STAGE_RULE_REF}\" だけ・要件 5.3"),
        ),
        OverrideKind::SecondStage => (
            conformance_item(reference) || carryover.headings().contains(reference),
            format!(
                "「項目 n」（n は 1〜{CONFORMANCE_ITEMS}）か {CARRYOVER_DOC} §6 の持ち越し行の見出し・要件 9.2"
            ),
        ),
    };
    (!accepted).then(|| {
        format!(
            "{BRIEFING_MD}: {} の override の ref \"{reference}\" は {} に許されない（{allowed}）",
            row_place(row),
            exception.kind.as_key()
        )
    })
}

/// 「項目 n」（n は 1〜20）か。
fn conformance_item(reference: &str) -> bool {
    reference
        .strip_prefix("項目 ")
        .and_then(|rest| rest.parse::<usize>().ok())
        .is_some_and(|number| (1..=CONFORMANCE_ITEMS).contains(&number))
}

/// 持ち越し 8 行の見出しの遅延読み（`second-stage` の行が現れたときだけ読む）。
///
/// 実データに `second-stage` の `override` は 1 行も無いので、素直に読むと**緑の走行が
/// 毎回よその spec の文書に寄りかかる**（読めなければ判定 ⑷ 全体が止まる）。ここで
/// 遅らせておくと、寄りかかるのは実際にその形の `ref` を書いた行があるときと、
/// 受け付け形を較正するテストのときだけになる。
pub(super) struct Carryover {
    cell: OnceCell<BTreeSet<String>>,
}

impl Carryover {
    /// まだ何も読んでいない器。
    pub(super) fn new() -> Self {
        Self {
            cell: OnceCell::new(),
        }
    }

    /// 持ち越し行の見出し（初回だけ読む）。
    pub(super) fn headings(&self) -> &BTreeSet<String> {
        self.cell.get_or_init(carryover_headings)
    }
}

/// 持ち越し 8 行の見出し（要件 9.1 が名指す表の 1 列目・`——` の手前まで）。
///
/// 出どころは完了 spec の適合検証の記録である。読めない・8 行にならないときは黙って
/// 短い一覧を返さずに止まる——短い一覧を返すと `ref` の受け付け形が緩む。
fn carryover_headings() -> BTreeSet<String> {
    let path = paths::workspace_root().join(CARRYOVER_DOC);
    let text = files::read_normalized(&path).unwrap_or_else(|err| panic!("{err}"));

    let mut headings = BTreeSet::new();
    let mut inside = false;
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            inside = heading.starts_with("6.");
            continue;
        }
        if !inside || !line.starts_with('|') {
            continue;
        }
        let cell = line
            .split('|')
            .nth(1)
            .unwrap_or_default()
            .split("——")
            .next()
            .unwrap_or_default()
            .trim();
        if cell.is_empty() || cell.starts_with("---") || cell == "持ち越した事項" {
            continue;
        }
        headings.insert(cell.to_owned());
    }

    assert_eq!(
        headings.len(),
        CARRYOVER_ROWS,
        "{CARRYOVER_DOC} §6 の持ち越し行を {CARRYOVER_ROWS} 行として読めない（{} 行）",
        headings.len()
    );
    headings
}

// ---------------------------------------------------------------------------
// 腕 f: 書き戻し後の分布
// ---------------------------------------------------------------------------

/// `[[barrier]]` の 1 行が宣言する 6 つの数（鍵・宣言・数え直す状態）。
fn barrier_counts(barrier: &Barrier) -> [(&'static str, usize, Status); 6] {
    [
        ("implemented", barrier.implemented, Status::Implemented),
        (
            "vocabulary_only",
            barrier.vocabulary_only,
            Status::VocabularyOnly,
        ),
        ("degraded", barrier.degraded, Status::Degraded),
        ("absent", barrier.absent, Status::Absent),
        ("alias", barrier.alias, Status::Alias),
        (
            "not_applicable",
            barrier.not_applicable,
            Status::NotApplicable,
        ),
    ]
}

/// 腕 f: `[[barrier]]`・`[[after]]`・`[priority_blank]` が台帳の数え直しと一致すること。
pub(super) fn distribution_findings(briefing: &Briefing, facts: &LedgerFacts) -> Vec<String> {
    let mut findings = Vec::new();

    for barrier in &briefing.barriers {
        let page = &barrier.page;
        for (key, declared, status) in barrier_counts(barrier) {
            // ページの**全項目**を数える（段階 A に落ちた項目だけではない）。
            let actual = facts.count(|fact| fact.page == *page && fact.status == status);
            if declared != actual {
                let what = format!("[[barrier]] {} の {key}", page.as_str());
                findings.push(count_mismatch(&what, declared, actual));
            }
        }
    }

    for after in &briefing.afters {
        let domain = after.domain;
        for stage in Stage::ALL {
            let declared = after.stages.get(&stage).copied().unwrap_or_default();
            let actual = facts
                .count(|fact| fact.domain == domain && fact.priority.starts_with(stage.as_key()));
            if declared != actual {
                let what = format!("[[after]] {} の {}", domain.as_key(), stage.as_key());
                findings.push(count_mismatch(&what, declared, actual));
            }
        }
        let blank = facts.count(|fact| fact.domain == domain && fact.priority.is_empty());
        if after.empty != blank {
            let what = format!("[[after]] {} の empty", domain.as_key());
            findings.push(count_mismatch(&what, after.empty, blank));
        }
    }

    for (key, declared, status) in [
        ("alias", briefing.priority_blank.alias, Status::Alias),
        (
            "not_applicable",
            briefing.priority_blank.not_applicable,
            Status::NotApplicable,
        ),
    ] {
        let actual = facts.count(|fact| fact.status == status);
        if declared != actual {
            let what = format!("[priority_blank] の {key}");
            findings.push(count_mismatch(&what, declared, actual));
        }
    }
    findings
}

// ---------------------------------------------------------------------------
// 腕 g: 人の決め（要件 5.3）
// ---------------------------------------------------------------------------

/// その段階のその順位の行。
pub(super) fn rows_at(briefing: &Briefing, stage: Stage, rank: usize) -> Vec<&RankRow> {
    briefing
        .ranks
        .iter()
        .filter(|row| row.stage == stage && row.rank == rank)
        .collect()
}

/// その並びのいずれかの行の束が、条件を満たすこと。
fn any_bundle(rows: &[&RankRow], linkage: &Linkage, keep: impl Fn(&NamedBundle) -> bool) -> bool {
    rows.iter().any(|row| {
        row_names(row)
            .iter()
            .any(|name| linkage.bundles.get(*name).is_some_and(&keep))
    })
}

/// 腕 g: 「更新」が段階 B の先頭・`system.*` が段階 C の末尾に居ること。
///
/// **どちらも「いずれか」で判定する。** 同じ段階の同じ `rank` に行が複数あるからで、
/// 2026-09-12 の実データでは段階 B の `rank` 1 が 2 行ある（対象行の「インストール」と
/// `override` の「更新」）。「すべての行が満たす」に読み替えると、要件 5.3 と関係の
/// 無い束が同着で先頭に並んだだけで赤くなる。
pub(super) fn stage_rule_findings(linkage: &Linkage, briefing: &Briefing) -> Vec<String> {
    let mut findings = Vec::new();

    let head = rows_at(briefing, Stage::B, 1);
    if head.is_empty() {
        findings.push(format!(
            "{BRIEFING_MD}: 段階 B に rank 1 の行が無い（順位は 1 から始める・要件 7.1）"
        ));
    } else if !any_bundle(&head, linkage, |bundle| {
        bundle.themes.iter().any(|theme| theme == UPDATE_THEME)
    }) {
        findings.push(format!(
            "{BRIEFING_MD}: 段階 B の rank 1 の {} 行のいずれの束も themes に「{UPDATE_THEME}」を持たない（「{UPDATE_THEME}」は段階 B の先頭・要件 5.3）",
            head.len()
        ));
    }

    let last_rank = briefing
        .ranks
        .iter()
        .filter(|row| row.stage == Stage::C)
        .map(|row| row.rank)
        .max();
    match last_rank {
        None => findings.push(format!(
            "{BRIEFING_MD}: 段階 C に順位の行が無い（`{SYSTEM_MARK}` を末尾に置けない・要件 5.3）"
        )),
        Some(last_rank) => {
            let tail = rows_at(briefing, Stage::C, last_rank);
            if !any_bundle(&tail, linkage, |bundle| {
                bundle
                    .members
                    .iter()
                    .any(|member| member.as_str().contains(SYSTEM_MARK))
            }) {
                findings.push(format!(
                    "{BRIEFING_MD}: 段階 C の最大 rank {last_rank} の {} 行のいずれの束も members に `{SYSTEM_MARK}` を含む id を持たない（`{SYSTEM_MARK}` は段階 C の末尾・要件 5.3）",
                    tail.len()
                ));
            }
        }
    }
    findings
}

// ---------------------------------------------------------------------------
// 7 つの腕をまとめる
// ---------------------------------------------------------------------------

/// 判定 ⑷ の所見（a〜g をまとめたもの）。空なら成り立っている。
pub(super) fn stage_and_rank_findings(
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
    facts: &LedgerFacts,
    carryover: &Carryover,
) -> Vec<String> {
    let mut findings = rank_coverage_findings(linkage, briefing);
    findings.extend(stage_count_findings(briefing, facts));
    findings.extend(priority_findings(linkage, briefing, ledgers));
    findings.extend(rank_derived_findings(linkage, briefing, ledgers));
    findings.extend(rank_order_findings(linkage, briefing, ledgers, carryover));
    findings.extend(distribution_findings(briefing, facts));
    findings.extend(stage_rule_findings(linkage, briefing));
    findings
}
