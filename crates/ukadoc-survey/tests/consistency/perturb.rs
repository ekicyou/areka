//! 実データの写しを 1 か所だけ壊す道具（設計 File Structure Plan）。
//!
//! テストの本体はここに 1 つも無い。壊す相手（[`Perturbed`]）と、壊した結果の見方
//! （[`expect_exactly`]）と、摂動で持ち込む綴りの定数だけを置く。使う側は
//! `consistency/checks.rs` の ⑶ の一群である。
//!
//! # repo のファイルには 1 バイトも触れない
//!
//! 手を入れるのは読み込んだ値のメモリ上の写しだけで、テストが終われば消える。書き出しも
//! 一時ディレクトリも使わない（設計 File Structure Plan）。

use std::collections::BTreeMap;

use ukadoc_survey::catalog::{Catalog, CatalogEntry};
use ukadoc_survey::check::{CheckInput, Finding, FindingKind, render, run};
use ukadoc_survey::evidence::EvidenceIndex;
use ukadoc_survey::evidence::extract::extract;
use ukadoc_survey::evidence::resolve::resolve;
use ukadoc_survey::ledger::{Ledger, LedgerEntry};
use ukadoc_survey::model::{Domain, EntryId, THEMES};

use super::RepoData;

/// 突き合わせの錨に使う項目 id（`doc/ukadoc-coverage/catalog.toml` に実在する）。
pub(super) const ANCHOR_ID: &str = "ukadoc:list_shiori_event:OnBoot:1";
/// カタログの場所（所見の「場所」に載る綴り）。
pub(super) const CATALOG_FILE: &str = "doc/ukadoc-coverage/catalog.toml";

/// shiori の台帳の場所。
pub(super) const SHIORI_LEDGER: &str = "doc/ukadoc-coverage/ledger/shiori.toml";

/// assets の台帳の場所。
pub(super) const ASSETS_LEDGER: &str = "doc/ukadoc-coverage/ledger/assets.toml";

/// カタログにも台帳にも無い id（摂動で持ち込む綴り）。
pub(super) const ABSENT_ID: &str = "ukadoc:list_shiori_event:OnNoSuchEventForTheTest:1";

/// 割り当て表に無いページを持つ id（摂動で持ち込む綴り）。
pub(super) const UNASSIGNED_PAGE_ID: &str = "ukadoc:no_such_page_for_the_test:Whatever:1";

/// 実データの写し。**repo のファイルには 1 バイトも触れない**。
///
/// 手を入れるのはこの写しの上だけで、テストが終われば消える。台帳のファイルそのものを
/// 書き換える確認は人手で 1 度だけ行った——常時テストがファイルを書き換えると、並行に
/// 走る他のテストがその途中の状態を読む。
///
/// 直に書き換える欄だけを兄弟（`checks.rs`）へ開ける。台帳は [`Perturbed::ledger_mut`]・
/// [`Perturbed::entry_mut`] を通す決まりなので閉じたままにする。
pub(super) struct Perturbed<'a> {
    data: &'a RepoData,
    pub(super) catalog: Catalog,
    ledgers: Vec<Ledger>,
    pub(super) evidence: EvidenceIndex,
    pub(super) domain_reports: BTreeMap<Domain, String>,
}

impl<'a> Perturbed<'a> {
    /// 手を入れていない写しを作る。
    pub(super) fn of(data: &'a RepoData) -> Self {
        Self {
            data,
            catalog: data.catalog.clone(),
            ledgers: data.ledgers.clone(),
            evidence: data.evidence.clone(),
            domain_reports: data.domain_reports.clone(),
        }
    }

    /// 写しから検査の入力を組む。割り当て表とテーマ名は本物をそのまま借りる。
    fn input(&self) -> CheckInput<'_> {
        CheckInput {
            catalog: &self.catalog,
            ledgers: &self.ledgers,
            assignment: &self.data.assignment,
            themes: &THEMES,
            evidence: &self.evidence,
            domain_reports: &self.domain_reports,
        }
    }

    /// 検査を走らせて所見を取る。
    pub(super) fn findings(&self) -> Vec<Finding> {
        run(&self.input()).findings
    }

    /// そのドメインの台帳（写し）。
    pub(super) fn ledger_mut(&mut self, domain: Domain) -> &mut Ledger {
        self.ledgers
            .iter_mut()
            .find(|ledger| ledger.domain == domain)
            .unwrap_or_else(|| panic!("{} の台帳が写しに無い", domain.as_key()))
    }

    /// その台帳の項目 1 つ（写し）。
    pub(super) fn entry_mut(&mut self, domain: Domain, id: &EntryId) -> &mut LedgerEntry {
        let key = id.clone();
        self.ledger_mut(domain)
            .entries
            .get_mut(&key)
            .unwrap_or_else(|| panic!("{} の台帳に {} が無い", domain.as_key(), key.as_str()))
    }
}

/// 錨の id。
pub(super) fn anchor_id() -> EntryId {
    EntryId::parse(ANCHOR_ID).expect("錨の id の形が違う")
}

/// 綴りから id を作る。
pub(super) fn id_of(raw: &str) -> EntryId {
    EntryId::parse(raw).unwrap_or_else(|err| panic!("{raw} の形が違う: {err}"))
}

/// カタログに足す作り物の項目。
///
/// 版番号は空にする——空なら登場版の判定が見ないので（要件 6.7 の番人）、摂動の
/// 巻き添えが 1 つ減る。
pub(super) fn fabricated_entry(id: &EntryId) -> CatalogEntry {
    CatalogEntry {
        id: id.clone(),
        page: id.page(),
        title: "テストのために作った見出し".to_owned(),
        category: "shiori_event".to_owned(),
        versions: Vec::new(),
        hash: "0000000000000000".to_owned(),
        url: format!("https://example.invalid/{}", id.as_str()),
    }
}

/// 所見を (種類, id, 場所) の並べ替え済みの一覧にする。
fn seen(findings: &[Finding]) -> Vec<(String, String, String)> {
    let mut rows: Vec<(String, String, String)> = findings
        .iter()
        .map(|finding| {
            (
                finding.kind.as_key().to_owned(),
                finding
                    .id
                    .as_ref()
                    .map(|id| id.as_str().to_owned())
                    .unwrap_or_default(),
                finding.place.clone(),
            )
        })
        .collect();
    rows.sort();
    rows
}

/// 所見の顔ぶれが**ちょうど**これであることと、本文が id と場所を名指すこと
/// （要件 6.10・6.12）。
///
/// 「含む」ではなく「ちょうど」で見る——巻き添えの所見が増えたことに気づけないと、
/// 摂動が何を起こしたのかを読み違える。id を持たない所見は空文字で書く。
pub(super) fn expect_exactly(findings: &[Finding], expected: &[(FindingKind, Option<&str>, &str)]) {
    let mut want: Vec<(String, String, String)> = expected
        .iter()
        .map(|(kind, id, place)| {
            (
                kind.as_key().to_owned(),
                id.unwrap_or_default().to_owned(),
                (*place).to_owned(),
            )
        })
        .collect();
    want.sort();

    let body = render(findings);
    assert_eq!(seen(findings), want, "所見の顔ぶれが違う:\n{body}");

    for (kind, id, place) in &want {
        if !id.is_empty() {
            assert!(
                body.contains(id),
                "本文が id {id} を名指していない:\n{body}"
            );
        }
        assert!(
            body.contains(place),
            "本文が場所 {place} を名指していない:\n{body}"
        );
        assert!(
            body.contains(kind),
            "本文が種類 {kind} を名指していない:\n{body}"
        );
    }
}

/// ソースの写しに 1 行だけ入れて、証拠の索引を作り直す。
///
/// `at` は入れる位置（行番号）で、`checks.rs` の `the_check_survives_lines_moving` が
/// ここを動かして要件 6.11 を確かめる。repo のファイルには触れない。
pub(super) fn evidence_with_source_line(
    data: &RepoData,
    path: &str,
    line: &str,
    at: usize,
) -> EvidenceIndex {
    let mut sources: Vec<(String, String)> = data.sources.clone();
    let target = sources
        .iter_mut()
        .find(|(each, _)| each == path)
        .unwrap_or_else(|| panic!("走査に {path} が無い"));
    let mut body: Vec<String> = target.1.lines().map(str::to_owned).collect();
    let at = at.min(body.len());
    body.insert(at, line.to_owned());
    target.1 = body.join("\n");

    let hits: Vec<_> = sources
        .iter()
        .flat_map(|(each, text)| extract(each, text))
        .collect();
    resolve(&hits, &sources, &data.catalog)
}
