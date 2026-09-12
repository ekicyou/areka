//! 帰属（`linkage.md`）× 順位（`briefing.md`）× 台帳 → 機械で決まる値。
//!
//! ここは純粋層で、ファイルにもスナップショットにも触らない（設計「境界」）。
//! 導出の実体を 1 か所に置くのは、`priority` の書き戻し（要件 7.1）と「台帳が文書
//! どおりか」の判定 ⑷-c が**同じ計算**を見るためである（設計 D-4）。2 か所に書くと
//! 第二段の組み直しで片方だけが古びる。
//!
//! # 何が機械で決まるか
//!
//! 束の欄のうち 4 つは人が書かない——テーマの和集合・跨ぐドメイン・資産の広さ・
//! 基盤共有度（[`Derived`]）。文書にはその値が**書いてある**が、正しさの正本は
//! ここの計算で、判定 ⑷-d と ⑶-e が数え直して突き合わせる。
//!
//! # 4 つの根拠の序列は入れ替えない
//!
//! 順位は [`axis_key`] の降順で、序列は 壊れ方 ＞ テーマ数 ＞ 資産の広さ ＞ 基盤
//! 共有度に固定する（要件 6.1・設計 D-9）。テーマは集合なので比べられる値として
//! **個数**を使う。
//!
//! # 繕わない
//!
//! どの束にも属さない対象項目・順位表に無い束・帰属に無い束名は、黙って既定値を
//! 置かずに落とす（要件 11.5 の向き）。落とす本文には **id（または束名）と文書名**
//! を添える。

use std::collections::{BTreeMap, BTreeSet};

use super::parse::{BRIEFING_FILE, LINKAGE_FILE, document_file};
use super::{Breakage, Briefing, Linkage, NamedBundle, RankTarget};
use crate::error::SurveyError;
use crate::ledger::Ledger;
use crate::model::{Domain, EntryId, Status};

/// 束ごとの、機械で決まる値（設計 `documents::derive`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    /// 構成 id の `values` の和集合（要件 4.1 ⑻・6.4）。
    pub themes: BTreeSet<String>,
    /// 構成 id を持つ台帳のドメイン（要件 4.1 ⑷）。
    pub domains: BTreeSet<Domain>,
    /// 構成 id ∩ テンプレート辞書の語彙の件数（要件 6.3）。
    pub assets: usize,
    /// 同じ `foundation` を持つ束の数（自分を含む・要件 6.5）。
    pub shared: usize,
}

/// 束 1 つ分の、機械で決まる値を導く。
///
/// 束の欄に書かれた `domains`・`themes` は**読まない**。読めば「文書を文書自身と
/// 比べる」だけになって、写し違えを 1 件も捕まえられない。台帳とテンプレート辞書と
/// 帰属の 3 つだけから数える。
///
/// 台帳に無い構成 id は何も足さない（束と台帳の食い違いは判定 ⑶ の持ち場）。
pub fn derive_bundle(
    bundle: &NamedBundle,
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
) -> Derived {
    // 同じ id を 2 度書いても 1 件として数える。
    let members: BTreeSet<&EntryId> = bundle.members.iter().collect();

    let mut themes = BTreeSet::new();
    let mut domains = BTreeSet::new();
    for ledger in ledgers {
        for member in &members {
            if let Some(entry) = ledger.entries.get(*member) {
                domains.insert(ledger.domain);
                themes.extend(entry.values.iter().cloned());
            }
        }
    }

    let vocabulary: BTreeSet<&EntryId> = briefing
        .templates
        .iter()
        .flat_map(|template| template.ids.iter())
        .collect();

    Derived {
        themes,
        domains,
        assets: members
            .iter()
            .filter(|member| vocabulary.contains(*member))
            .count(),
        shared: shared_of(bundle, linkage),
    }
}

/// 基盤共有度（要件 6.5）。同じ `foundation` の束の数で、自分を含む。
///
/// `foundation` を持たない束——単独項目がそうで、読み手が欄そのものを禁じている——は
/// **0** とする。空文字どうしを数え合わせると「基盤を書いていない」が「同じ基盤を
/// 共有している」に化けて、単独項目の全数がそのまま共有度になってしまう。
fn shared_of(bundle: &NamedBundle, linkage: &Linkage) -> usize {
    if bundle.foundation.is_empty() {
        return 0;
    }
    linkage
        .bundles
        .values()
        .filter(|other| other.foundation == bundle.foundation)
        .count()
}

/// 4 つの根拠の比較鍵（大きいほど先）。壊れ方 ＞ テーマ数 ＞ 資産の広さ ＞ 基盤共有度。
///
/// 序列は要件 6.1 が凍結しているので、欄を足すときも**末尾**にしか置けない。
/// テーマは集合なので、比べられる値として個数を使う（設計 D-9）。
pub fn axis_key(
    breakage: Breakage,
    themes: usize,
    assets: usize,
    shared: usize,
) -> (u8, usize, usize, usize) {
    (breakage_weight(breakage), themes, assets, shared)
}

/// 壊れ方の重み（設計 D-9）。黙って壊れる ＞ 明示エラー ＞ 見た目の差 ＞ 該当なし。
fn breakage_weight(breakage: Breakage) -> u8 {
    // 既定の腕を置かない。語彙を増やしたらここが赤くなる。
    match breakage {
        Breakage::Silent => 3,
        Breakage::Explicit => 2,
        Breakage::Visual => 1,
        Breakage::NotApplicable => 0,
    }
}

/// 全項目の `priority`（要件 7.1・7.2）。
///
/// 鍵は渡された台帳の全 id。値は対象 4 状態なら「段階 1 文字＋段階内の順位」、
/// `alias`・`not-applicable` なら空文字。順位の数は `[[rank]]` に書かれたものを
/// **そのまま**写す——密な順位（1,2,2,3）を主張するのは判定 ⑷-e の持ち場で、
/// ここで振り直すと文書と台帳が二重の正本になる。
///
/// 次のいずれかがあれば落ちる（id か束名と文書名を添える）。
///
/// - 対象 4 状態の項目がどの束にも属さない
/// - 帰属にある束が `[[rank]]` に無い／2 度ある
/// - `[[rank]]` が帰属に無い束名を指す
/// - `bundle` が単独項目を指す／`singles` に名前付き束が混ざる
/// - 同じ id が 2 つの束に属する
/// - 状態が `unclassified`（要件 7.1 も 7.2 も行き先を決めていない）
pub fn priorities(
    linkage: &Linkage,
    briefing: &Briefing,
    ledgers: &[Ledger],
) -> Result<BTreeMap<EntryId, String>, SurveyError> {
    let by_id = priority_by_id(linkage, briefing)?;
    let linkage_file = document_file(LINKAGE_FILE);

    let mut priorities = BTreeMap::new();
    for ledger in ledgers {
        for (id, entry) in &ledger.entries {
            let value = match entry.status {
                // 束の構成から除いてある（要件 4.3）ので、帰属が無くても落とさない。
                Status::Alias | Status::NotApplicable => String::new(),
                Status::Implemented
                | Status::VocabularyOnly
                | Status::Degraded
                | Status::Absent => by_id.get(id).cloned().ok_or_else(|| {
                    mismatch(
                        &linkage_file,
                        format!(
                            "{} がどの束にも属さない（名前付き束か単独項目のどちらかに入れる）",
                            id.as_str()
                        ),
                    )
                })?,
                Status::Unclassified => {
                    return Err(mismatch(
                        &ledger_file(ledger.domain),
                        format!(
                            "{} の状態が unclassified で、段階も空欄も決められない",
                            id.as_str()
                        ),
                    ));
                }
            };
            priorities.insert(id.clone(), value);
        }
    }
    Ok(priorities)
}

/// 帰属 × 順位 → 項目ごとの `priority` の綴り。
///
/// 順位の行を辿りながら束を引き、その構成 id へ同じ値を配る。行と束が 1 対 1 で
/// 対応することもここで確かめる——対応が崩れたまま先へ進むと、台帳の側で
/// 「どの束にも属さない」という**別の**理由で落ちて、直す場所を取り違える。
fn priority_by_id<'a>(
    linkage: &'a Linkage,
    briefing: &Briefing,
) -> Result<BTreeMap<&'a EntryId, String>, SurveyError> {
    let briefing_file = document_file(BRIEFING_FILE);
    let linkage_file = document_file(LINKAGE_FILE);

    let mut ranked: BTreeSet<&str> = BTreeSet::new();
    let mut by_id: BTreeMap<&EntryId, String> = BTreeMap::new();

    for row in &briefing.ranks {
        let value = format!("{}{}", row.stage.as_key(), row.rank);
        let (want_single, names): (bool, Vec<&str>) = match &row.target {
            RankTarget::Bundle(name) => (false, vec![name.as_str()]),
            RankTarget::Singles(ids) => (true, ids.iter().map(EntryId::as_str).collect()),
        };

        for name in names {
            let bundle = linkage.bundles.get(name).ok_or_else(|| {
                mismatch(
                    &briefing_file,
                    format!("順位の行が指す束 {name} が linkage.md に無い"),
                )
            })?;
            if bundle.single != want_single {
                return Err(mismatch(&briefing_file, wrong_slot(name, want_single)));
            }
            if !ranked.insert(name) {
                return Err(mismatch(
                    &briefing_file,
                    format!("束 {name} が [[rank]] に 2 度現れる"),
                ));
            }
            for member in &bundle.members {
                if by_id.insert(member, value.clone()).is_some() {
                    return Err(mismatch(
                        &linkage_file,
                        format!(
                            "{} が 2 つの束に属している（後の束は {name}）",
                            member.as_str()
                        ),
                    ));
                }
            }
        }
    }

    for name in linkage.bundles.keys() {
        if !ranked.contains(name.as_str()) {
            return Err(mismatch(
                &briefing_file,
                format!("束 {name} が [[rank]] に無い（順位を付けない束は置けない）"),
            ));
        }
    }
    Ok(by_id)
}

/// 単独項目と名前付き束を置き違えたときの本文。直し方まで書く。
fn wrong_slot(name: &str, want_single: bool) -> String {
    if want_single {
        format!("名前付き束 {name} が singles に混ざっている（bundle で指す）")
    } else {
        format!("単独項目 {name} を bundle で指している（singles に並べる）")
    }
}

/// その台帳の場所（失敗の本文に載る綴り）。
fn ledger_file(domain: Domain) -> String {
    format!("doc/ukadoc-coverage/ledger/{}.toml", domain.as_key())
}

/// 導けないことを告げる失敗。文書名を必ず添える。
fn mismatch(file: &str, reason: impl Into<String>) -> SurveyError {
    SurveyError::DeriveMismatch {
        file: file.to_owned(),
        reason: reason.into(),
    }
}

#[cfg(test)]
#[path = "derive_tests.rs"]
mod tests;
