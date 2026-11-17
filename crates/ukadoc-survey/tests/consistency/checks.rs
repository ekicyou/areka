//! repo の実データに対する主張（要件 6.3〜6.8・6.10・6.11・7.4・7.5・6.1・6.2・6.12）。
//!
//! 読み込みは [`RepoData`] が引き受け、ここは**読んだ値に何を求めるか**だけを書く。
//!
//! # 赤になったときに読むもの
//!
//! 失敗の本文は [`ukadoc_survey::check::render`] が整えた食い違いの一覧**そのもの**で
//! ある（要件 6.12）。件数だけの要約に置き換えない——台帳を壊した人が、テストの出力
//! だけを見て直せるようにするためである。
//!
//! # 「食い違い 0 件」は、それだけでは何も言わない
//!
//! 実データに対する「所見 0 件」は、判定が受け持つ要件を**一括で**満たす。判定が 1 つ黙って
//! 消えても、対象が 0 件の判定が空回りしていても、同じ緑になる。だからこのファイルは
//! 3 段の主張を重ねる。
//!
//! ⑴ **一括の主張**（[`real_repo_data_produces_no_findings`]）——所見が 1 件も無い。
//! ⑵ **対象の数え上げ**（[`the_subject_census_says_which_requirements_are_vacuous`]）
//!    ——要件ごとに対象を数え、どれかが 0 件（＝空振り）へ落ちたら、その要件と対象を
//!    **名指しして**赤にする。
//! ⑶ **1 要件 1 摂動**（このファイルの後半）——実データの写しを 1 か所だけ壊し、
//!    該当の判定が該当の id と場所つきで赤くなることを要件ごとに確かめる。壊すのは
//!    メモリ上の写しだけで、repo のファイルには 1 バイトも触れない。壊す道具そのもの
//!    （写しの型・所見の見方・綴りの定数）は兄弟の `consistency/perturb.rs` にある。
//!
//! ⑶ が要る理由——実データの対象が非空でも、それは「判定を通る行がある」ことしか言わず、
//! **判定が食い違いを見つけられる**ことは言わない。今日の実データは食い違い 0 件なので、
//! 判定が丸ごと消えても ⑴ は緑になる。写しを壊す ⑶ だけが、その判定が**実データの上で**
//! 生きていることを示す。対象が 0 件の判定は、緑でも「壊れていない」ことを 1 つも言わない
//! ので、⑵ がその状態を名指しで赤にする。
//!
//! # ここに置かないもの
//!
//! - 件数そのものの釘付け（カタログの件数・台帳の本数など。要件 6.13）は
//!   `consistency/non_vacuity.rs` が持つ。ここで数えるのは**要件ごとの対象の有無**
//!   だけである。
//! - 語彙表経路（既存の語彙台帳 `SHIORI_RESOURCE_IDS` から証拠を引く経路）の較正は
//!   `consistency/values_md.rs` が持つ。ここが証拠を作るときは項目 URL を 1 行入れる
//!   だけで、語彙表経路は通さない。

use ukadoc_survey::check::{FindingKind, render, run};
use ukadoc_survey::evidence::EvidenceIndex;
use ukadoc_survey::evidence::extract::extract;
use ukadoc_survey::io::{files, paths};
use ukadoc_survey::ledger::read::read as read_ledger;
use ukadoc_survey::model::{Domain, Link, LinkKind, Status, THEMES};

use super::RepoData;
use super::perturb::{
    ABSENT_ID, ANCHOR_ID, ASSETS_LEDGER, CATALOG_FILE, Perturbed, SHIORI_LEDGER, SUMMARY_REPORT,
    UNASSIGNED_PAGE_ID, anchor_id, evidence_with_source_line, expect_exactly, fabricated_entry,
    id_of,
};

/// その項目の正典 URL（カタログの綴りを逐語で写したもの）。
///
/// [`ANCHOR_ID`] と**同じ項目**を指す。錨を選び直すときはここも一緒に差し替えること
/// ——綴りが割れると `the_check_survives_lines_moving` が別の項目の証拠を数える。
const ANCHOR_URL: &str =
    "https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFirstBoot:1";

/// 走査が届いていることの錨に使うソース（既存の語彙台帳・要件 9.2）。
///
/// **錨の項目の証拠を「ちょうど 1 件」で見てはいけない。** 調査 spec は実装済みと
/// 判定した項目の定義箇所に正典 URL を 1 行置く契約なので、同じ項目の証拠が複数の
/// ファイルから挙がるのは正常な状態である（[`EvidenceIndex::by_id`] の並びの契約も
/// 「重複を除いた名前順」であって件数を約束しない）。ここで確かめたいのは
/// 「書き足した行がその項目の証拠になる」ことだけである。
use super::documents::SHIORI_RESOURCE_RS as ANCHOR_SOURCE;

/// 書き足した 1 行が錨の項目の証拠として現れたこと（「含む」で見る）。
///
/// 恒真を避けるため、書き足す**前**に [`ANCHOR_SOURCE`] が錨の項目の証拠へ入って
/// いないことを同じ場所で確かめる——増えたのは書き足した行のせいである、と言える
/// ようにするためである。前から入っていたら錨を選び直す合図なので、そのときも赤に
/// する。
fn assert_the_added_line_became_evidence(data: &RepoData, after: &EvidenceIndex) {
    let anchor = anchor_id();
    let sources = |index: &EvidenceIndex| -> Vec<String> {
        index.by_id.get(&anchor).cloned().unwrap_or_default()
    };

    let before = sources(&data.evidence);
    assert!(
        !before.iter().any(|path| path == ANCHOR_SOURCE),
        "書き足す前から {ANCHOR_SOURCE} が錨の項目の証拠になっている。\
         これでは「足したから増えた」が言えないので、錨のソースを選び直すこと: {before:?}"
    );

    let after = sources(after);
    assert!(
        after.iter().any(|path| path == ANCHOR_SOURCE),
        "書き足した正典 URL の行が錨の項目の証拠に入っていない: {after:?}"
    );
}

// ---------------------------------------------------------------------------
// ⑴ 一括の主張
// ---------------------------------------------------------------------------

/// repo の実データに食い違いが 1 件も無いこと（要件 6.1・6.12）。
///
/// 失敗の本文は整形した一覧そのものである。
#[test]
fn real_repo_data_produces_no_findings() {
    let data = RepoData::load();
    let outcome = run(&data.input());

    assert!(outcome.findings.is_empty(), "{}", render(&outcome.findings));
}

/// 今の `catalog.toml` を読んで書き直すと、元の本文と 1 バイトも変わらないこと。
///
/// カタログの書き手と読み手が割れると、再生成のたびに差分が出る。
#[test]
fn the_catalog_round_trips_byte_for_byte() {
    let text = files::read_normalized(&paths::catalog_path()).unwrap_or_else(|err| panic!("{err}"));
    let catalog = ukadoc_survey::catalog::read::read(&text).expect("catalog.toml を読める");

    assert!(
        ukadoc_survey::catalog::write::write(&catalog) == text,
        "catalog.toml が読み直しで変わる（read → write の本文が元と違う）"
    );
}

/// 読み込みが repo の実データに届いていること。
///
/// 「食い違い 0 件」は、読み込みが空を返しても満たされる。上の事例だけでは見分けられ
/// ないので、ここで実データに届いていることを確かめる。
#[test]
fn the_load_reaches_the_real_repo_data() {
    let data = RepoData::load();
    let input = data.input();

    let anchor = anchor_id();
    let entry = data
        .catalog
        .entries
        .get(&anchor)
        .unwrap_or_else(|| panic!("カタログに {ANCHOR_ID} が無い"));
    assert_eq!(entry.page.as_str(), "list_shiori_event");
    assert_eq!(entry.category, "shiori_event");
    assert_eq!(entry.url, ANCHOR_URL);

    let shiori = &data.ledgers[0];
    assert!(
        shiori.entries.contains_key(&anchor),
        "shiori の台帳に {ANCHOR_ID} が無い"
    );

    assert!(
        data.values.contains("\n## 気配\n"),
        "テーマ定義を読んでいない"
    );

    assert!(
        data.sources.iter().any(|(path, _)| path == ANCHOR_SOURCE),
        "ソースの走査が {ANCHOR_SOURCE} へ届いていない"
    );

    assert_eq!(
        input.themes,
        THEMES.as_slice(),
        "検査へ渡すテーマ名が model::THEMES でない（報告を書き出した側と出どころが割れる）"
    );
}

// ---------------------------------------------------------------------------
// ⑵ 対象の数え上げ——どの要件も空振りしていないことを名指しで確かめる
// ---------------------------------------------------------------------------

/// 数え上げの 1 行（要件・何を数えたか・件数）。
struct CensusRow {
    requirement: &'static str,
    subject: &'static str,
    count: usize,
}

/// 要件ごとに、実データにある対象を数える。
///
/// **ここは判定を呼ばない。** 判定が見る母数を、判定とは別の道筋で数え直す——同じ
/// 関数を呼ぶと、判定が壊れたときに数え上げも一緒に壊れて気づけない。
fn census(data: &RepoData) -> Vec<CensusRow> {
    let ledger_rows: usize = data.ledgers.iter().map(|led| led.entries.len()).sum();
    let rows = || data.ledgers.iter().flat_map(|led| led.entries.values());

    let url_hits: usize = data
        .sources
        .iter()
        .map(|(path, text)| extract(path, text).len())
        .sum();
    let implemented = rows()
        .filter(|entry| entry.status == Status::Implemented)
        .count();
    let endpoints: usize = rows()
        .map(|entry| {
            usize::from(entry.alias_of.is_some()) + entry.supersedes.len() + entry.links.len()
        })
        .sum();
    let alias_rows = rows().filter(|entry| entry.status == Status::Alias).count();
    let introduced_rows = rows().filter(|entry| !entry.introduced.is_empty()).count();
    let theme_values: usize = rows().map(|entry| entry.values.len()).sum();
    let reports = data.domain_reports.len();
    let versioned = data
        .catalog
        .entries
        .values()
        .filter(|entry| !entry.versions.is_empty())
        .count();

    let row = |requirement, subject, count| CensusRow {
        requirement,
        subject,
        count,
    };
    vec![
        row("6.3", "台帳に現れる id", ledger_rows),
        row("6.4", "カタログの id", data.catalog.entries.len()),
        row("6.5", "ソースの正典 URL", url_hits),
        row("6.6", "状態が implemented の行", implemented),
        row("6.7", "関連・別名・後継の相手", endpoints),
        row("6.7", "状態が alias の行", alias_rows),
        row("6.7", "登場版の記入がある行", introduced_rows),
        row("6.7", "カタログに版番号のある項目", versioned),
        row("6.8", "台帳に書かれたテーマ名", theme_values),
        row("6.10", "状態の語彙を通る行", ledger_rows),
        row("6.11", "証拠の付いた項目", data.evidence.by_id.len()),
        row("7.4/7.5", "ドメイン別報告", reports),
    ]
}

/// 要件ごとに、実データに対象があることを名指しで確かめる（要件 6.13）。
///
/// **このテストは「緑だから守られている」を否定するために置く。** 対象が 0 件の判定は、
/// 判定が丸ごと消えても実データは緑のままである。だから対象が 0 件になったら赤にする。
///
/// このテストが言うのは「どの要件も空振りしていない」ことである。どれかが
/// 0 件へ落ちたら赤になる——台帳を読み込み損ねた・書き入れた欄が消えた・正典 URL の
/// 行が剥がれた、のいずれかが起きた合図である。
///
/// 件数そのものはここでは固定しない。それは `consistency/non_vacuity.rs` が持ち、
/// ここが言うのは**有無**だけである。
#[test]
fn the_subject_census_says_which_requirements_are_vacuous() {
    let data = RepoData::load();

    for row in census(data) {
        assert!(
            row.count > 0,
            "要件 {} の対象「{}」が 0 件になった。読み込みが空振りしていないか確かめること",
            row.requirement,
            row.subject
        );
    }
}

// ---------------------------------------------------------------------------
// ⑶ 1 要件 1 摂動——実データの写しを 1 か所だけ壊す
// ---------------------------------------------------------------------------

/// 手を入れていない写しが実データと同じく緑であること。
///
/// **摂動の較正である。** 写しの作り方が壊れていれば（台帳を 1 本落とす・報告を
/// 取り違えるなど）、以下の摂動が何を証明しているのか分からなくなる。ここで先に
/// 「壊す前は緑」を示してから壊す。
#[test]
fn the_untouched_copy_of_real_data_is_still_green() {
    let data = RepoData::load();
    let copy = Perturbed::of(data);

    expect_exactly(&copy.findings(), &[]);
}

/// 要件 6.3——台帳の id がカタログから消えると、その id と台帳の場所つきで赤くなる。
///
/// 壊すのはカタログの側である（台帳の側の綴りを変えると、同じ摂動が 6.4 の側にも
/// 火を点けて何が起きたのか読みにくくなる）。台帳に残った id の相手がカタログから
/// 消えた状態は、要件 6.3 が禁じている形そのものである。
#[test]
fn a_ledger_id_that_left_the_catalog_turns_red() {
    let data = RepoData::load();
    let anchor = anchor_id();
    let mut copy = Perturbed::of(data);

    let removed = copy.catalog.entries.remove(&anchor);
    assert!(removed.is_some(), "カタログに {ANCHOR_ID} が無い");

    expect_exactly(
        &copy.findings(),
        &[(
            FindingKind::LedgerIdNotInCatalog,
            Some(ANCHOR_ID),
            SHIORI_LEDGER,
        )],
    );
}

/// 要件 6.4——カタログの id がどの台帳にも無いと赤くなる。
#[test]
fn a_catalog_id_in_no_ledger_turns_red() {
    let data = RepoData::load();
    let absent = id_of(ABSENT_ID);
    let mut copy = Perturbed::of(data);

    copy.catalog
        .entries
        .insert(absent.clone(), fabricated_entry(&absent));

    expect_exactly(
        &copy.findings(),
        &[(
            FindingKind::CatalogIdMissingFromLedgers,
            Some(ABSENT_ID),
            CATALOG_FILE,
        )],
    );
}

/// 要件 6.4・3.2——同じ id が 2 本の台帳に現れると赤くなる。
///
/// 巻き添えが 3 つ出るが、それは正しい——shiori のページの id を assets の台帳へ
/// 移した状態は、担当の食い違い（要件 3.1）でもあり、assets の報告と全体報告が台帳と
/// 食い違う状態（要件 7.4）でもある。4 つとも本文に出ることをここで固定する。
#[test]
fn an_id_in_two_ledgers_turns_red() {
    let data = RepoData::load();
    let anchor = anchor_id();
    let mut copy = Perturbed::of(data);

    let borrowed = copy.entry_mut(Domain::Shiori, &anchor).clone();
    copy.ledger_mut(Domain::Assets)
        .entries
        .insert(anchor.clone(), borrowed);

    expect_exactly(
        &copy.findings(),
        &[
            (
                FindingKind::CatalogIdInMultipleLedgers,
                Some(ANCHOR_ID),
                CATALOG_FILE,
            ),
            (
                FindingKind::LedgerIdPageMismatch,
                Some(ANCHOR_ID),
                ASSETS_LEDGER,
            ),
            (
                FindingKind::DomainReportStale,
                None,
                "doc/ukadoc-coverage/report/assets.md",
            ),
            (FindingKind::DomainReportStale, None, SUMMARY_REPORT),
        ],
    );
}

/// 要件 3.3a・付録 A——台帳の並びが崩れると、後ろに来た id つきで赤くなる。
///
/// 見るのは本文の順（`file_order`）で、`entries` は表なので作りからして昇順である。
/// 報告は `entries` から作るので、並びだけを崩しても報告は古くならない。
#[test]
fn a_ledger_out_of_order_turns_red() {
    let data = RepoData::load();
    let mut copy = Perturbed::of(data);

    let order = &mut copy.ledger_mut(Domain::Shiori).file_order;
    assert!(order.len() >= 2, "並びを崩せるだけの項目が無い");
    order.swap(0, 1);
    let demoted = order[1].as_str().to_owned();

    expect_exactly(
        &copy.findings(),
        &[(
            FindingKind::LedgerOutOfOrder,
            Some(demoted.as_str()),
            SHIORI_LEDGER,
        )],
    );
}

/// 要件 3.5——カタログに割り当ての無いページが現れると赤くなる。
#[test]
fn a_page_without_an_assignment_turns_red() {
    let data = RepoData::load();
    let stray = id_of(UNASSIGNED_PAGE_ID);
    let mut copy = Perturbed::of(data);

    copy.catalog
        .entries
        .insert(stray.clone(), fabricated_entry(&stray));

    expect_exactly(
        &copy.findings(),
        &[
            (
                FindingKind::CatalogIdMissingFromLedgers,
                Some(UNASSIGNED_PAGE_ID),
                CATALOG_FILE,
            ),
            (FindingKind::PageNotAssigned, None, CATALOG_FILE),
        ],
    );
}

/// 要件 6.5・6.10——ソースの正典 URL がカタログに無いと、そのファイルつきで赤くなる。
///
/// 実データのソースにはカタログにある URL だけがあり（[`census`] の 6.5 の行は空でない）、
/// 食い違いは 0 件である。だからソースの写しに 1 行だけ書き足して食い違いを作る。
/// 取り出し（`extract`）と解決（`resolve`）を実データのソース全域に対して回すので、
/// 走査と突き合わせの経路が丸ごと通る。
#[test]
fn an_unknown_canon_url_in_a_source_turns_red() {
    let data = RepoData::load();
    assert!(
        data.evidence.unresolved.is_empty(),
        "実データに解決できない URL が既にある。この摂動の前提が崩れている"
    );

    let bogus = "https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnNoSuchThing:1";
    let mut copy = Perturbed::of(data);
    copy.evidence =
        evidence_with_source_line(data, ANCHOR_SOURCE, &format!("// ukadoc: {bogus}"), 0);

    assert_eq!(
        copy.evidence.unresolved.len(),
        1,
        "書き足した 1 行が取り出せていない"
    );
    expect_exactly(
        &copy.findings(),
        &[(FindingKind::SourceUrlNotInCatalog, None, ANCHOR_SOURCE)],
    );
    assert!(
        render(&copy.findings()).contains(bogus),
        "本文が綴りの違う URL そのものを載せていない"
    );
}

/// 要件 6.6——`implemented` の行に証拠が無いと赤くなり、証拠が付くと消える。
///
/// **証拠の無い側は摂動で作る。** 錨の項目には今や実ソースの正典 URL が付いている
/// （調査 spec が `areka-kanade/src/schedule/events.rs` へ 1 行置いた）
/// ので、状態を `implemented` にするだけでは所見が生まれない。だから写しの索引から
/// **錨の id の証拠を取り除いて**対象を作る。
///
/// 錨を「今日まだ証拠の無い項目」へ選び直す道は採らない。調査 spec の仕事はまさに
/// 実装済みの項目へ正典 URL を置くことなので、証拠の無い項目を選んでも同じ理由で
/// また腐るからである。
///
/// **報告は古くならない。** 錨は台帳の側で既に `implemented` なので、状態を動かす
/// 摂動がそもそも無いからである。この摂動が触るのは証拠の索引だけで、そこは報告の
/// 材料ではない（報告が数えるのは状態と世代とテーマの分布）。
///
/// 後半はソースへ正典 URL を 1 行置いて所見が消えることまで見る。片方だけだと
/// 「常に赤い判定」と見分けが付かない。
#[test]
fn implemented_without_evidence_turns_red_and_evidence_clears_it() {
    let data = RepoData::load();
    let anchor = anchor_id();

    let mut copy = Perturbed::of(data);
    assert_eq!(
        copy.entry_mut(Domain::Shiori, &anchor).status,
        Status::Implemented,
        "錨の項目が台帳で `implemented` でなくなっている。この判定は「錨は既に実装済み\
         なので報告は動かない」ことに寄りかかって所見を数えているので、状態が変わったなら\
         期待も錨の選び方も見直すこと"
    );
    let stripped = copy.evidence.by_id.remove(&anchor);
    assert!(
        stripped.is_some(),
        "錨の項目に証拠が 1 件も無い。この摂動は証拠を剥がして対象を作るので、\
         剥がす相手が要る——錨に正典 URL が置かれているか確かめること"
    );

    // 証拠が無い側。報告は動かないので、出るのはこの 1 件だけ。
    expect_exactly(
        &copy.findings(),
        &[(
            FindingKind::ImplementedWithoutEvidence,
            Some(ANCHOR_ID),
            SHIORI_LEDGER,
        )],
    );

    // 証拠が付いた側。`ImplementedWithoutEvidence` が消えて所見は 1 件も残らない。
    copy.evidence =
        evidence_with_source_line(data, ANCHOR_SOURCE, &format!("// ukadoc: {ANCHOR_URL}"), 0);
    assert_the_added_line_became_evidence(data, &copy.evidence);
    expect_exactly(&copy.findings(), &[]);
}

/// 要件 6.7——関連の相手がカタログに無いと、書いた側の id つきで赤くなる。
///
/// **ドメイン別報告は古くならず、全体報告だけが古くなる。** ドメイン別報告に載る束は
/// 「構成 id が全部この台帳にある」ものだけで（`report::domain` の `closed_bundles`）、
/// 相手が台帳に無い辺の束は丸ごと落ちる。全体報告は台帳に無い構成 id を持つ束を
/// 「（台帳に無い）」を跨ぐ束として載せるので、本文が変わる。ただし全体報告の所見は
/// 「作り直せ」としか言わない——宙に浮いた関連を名指すのはこの判定だけである。
#[test]
fn a_link_to_a_missing_id_turns_red() {
    let data = RepoData::load();
    let anchor = anchor_id();
    let mut copy = Perturbed::of(data);

    copy.entry_mut(Domain::Shiori, &anchor).links = vec![Link {
        kind: LinkKind::SameFeature,
        to: id_of(ABSENT_ID),
    }];

    let findings = copy.findings();
    expect_exactly(
        &findings,
        &[
            (
                FindingKind::LinkEndpointMissing,
                Some(ANCHOR_ID),
                SHIORI_LEDGER,
            ),
            (FindingKind::DomainReportStale, None, SUMMARY_REPORT),
        ],
    );
    assert!(
        render(&findings).contains(ABSENT_ID),
        "本文が相手の id を載せていない"
    );
}

/// 要件 6.7・2.4——`alias_of` の指す先も別名だと赤くなる（別名の連鎖の禁止）。
///
/// 指す先は `alias_of` を持たない別名の行になるが、その形を拒むのは読み取りの段
/// （`ledger::read` の付録 A.2 の突き合わせ）であって検査の段ではない。ここは読み取りを
/// 通さずに写しを組むので、その拒みには掛からない。
#[test]
fn an_alias_chain_turns_red() {
    let data = RepoData::load();
    let mut copy = Perturbed::of(data);

    // shiori の台帳から実在する 2 つを取る（並びは id の byte 昇順で決まる）。
    let shiori = data
        .ledgers
        .iter()
        .find(|ledger| ledger.domain == Domain::Shiori)
        .expect("shiori の台帳が無い");
    let mut keys = shiori.entries.keys();
    let first = keys.next().expect("台帳が空").clone();
    let second = keys.next().expect("台帳の項目が 1 つしかない").clone();

    copy.entry_mut(Domain::Shiori, &first).status = Status::Alias;
    copy.entry_mut(Domain::Shiori, &first).alias_of = Some(second.clone());
    copy.entry_mut(Domain::Shiori, &second).status = Status::Alias;

    let findings = copy.findings();
    expect_exactly(
        &findings,
        &[
            (FindingKind::AliasChain, Some(first.as_str()), SHIORI_LEDGER),
            (
                FindingKind::DomainReportStale,
                None,
                "doc/ukadoc-coverage/report/shiori.md",
            ),
            (FindingKind::DomainReportStale, None, SUMMARY_REPORT),
        ],
    );
    assert!(
        render(&findings).contains(second.as_str()),
        "本文が指す先の id を載せていない"
    );
}

/// 要件 6.7——登場版がカタログの版番号の外にあると赤くなり、中にあれば赤くならない。
///
/// カタログ側に版番号のある項目を実データから 1 つ選ぶ（[`census`] がその母数が
/// 0 件でないことを守っている）。**両方向を見る**——外へ動かすと赤、中へ戻すと緑。
/// 片方だけだと「常に赤い判定」「常に緑の判定」と見分けが付かない。
///
/// **戻した側で報告は古くならない。** 選ばれる項目の `introduced` は台帳の側で既に
/// カタログの先頭の版番号と同値なので、戻す操作は台帳を実データそのものへ返すだけ
/// だからである。だから戻した側の期待は所見 0 件になる。下の `assert_eq!` がその
/// 同値をその場で確かめるので、台帳が変わったら黙って通り抜けずに赤くなる。
///
/// 「台帳の `introduced` がカタログの版番号と**異なる**項目を選ぶ」道は測ったうえで
/// 採らなかった。そういう項目は少なく、しかもどれも違いは末尾の節だけである（台帳
/// `2.7.26`／カタログ `2.7.25` の形）。`introduced` がドメイン別報告へ届く唯一の道は
/// 世代（先頭 2 節）の分布なので、戻しても世代は `2.7` のまま動かず、結局戻した側の
/// 所見は同じく 0 件になる。得るものが無いのに、その行が直された日に選び方ごと
/// 立ち行かなくなる脆さだけが増える。
///
/// 全体報告は登場版を載せないので、外へ動かした側でも古くならない（期待に全体報告の
/// 行が無いのはこのため）。
#[test]
fn an_introduced_version_outside_the_catalog_turns_red() {
    let data = RepoData::load();
    let report = "doc/ukadoc-coverage/report/shiori.md";

    // 版番号のある項目のうち shiori の台帳にあるものを 1 つ（並びは id の byte 昇順）。
    let shiori = data
        .ledgers
        .iter()
        .find(|ledger| ledger.domain == Domain::Shiori)
        .expect("shiori の台帳が無い");
    let (id, known) = shiori
        .entries
        .keys()
        .find_map(|id| {
            let entry = data.catalog.entries.get(id)?;
            let version = entry.versions.first()?;
            Some((id.clone(), version.clone()))
        })
        .expect("版番号のある項目が shiori の台帳に 1 つも無い");

    let mut copy = Perturbed::of(data);
    assert_eq!(
        copy.entry_mut(Domain::Shiori, &id).introduced,
        known,
        "選ばれた項目の台帳の `introduced` がカタログの先頭の版番号と食い違っている。\
         この判定は戻す操作が台帳を実データそのものへ返すことに寄りかかって、戻した側の\
         期待を所見 0 件にしているので、食い違うならその期待を見直すこと"
    );
    copy.entry_mut(Domain::Shiori, &id).introduced = "0.0.0-none".to_owned();

    let findings = copy.findings();
    expect_exactly(
        &findings,
        &[
            (
                FindingKind::IntroducedNotInCatalogVersions,
                Some(id.as_str()),
                SHIORI_LEDGER,
            ),
            (FindingKind::DomainReportStale, None, report),
        ],
    );
    assert!(
        render(&findings).contains(&known),
        "本文がカタログの版番号を載せていない"
    );

    // カタログにある版番号へ戻せば、台帳は実データそのものに返るので所見は 1 件も残らない。
    copy.entry_mut(Domain::Shiori, &id).introduced = known;
    expect_exactly(&copy.findings(), &[]);
}

/// 要件 6.8——テーマ名の綴りが違うと、id と場所つきで台帳の読み取りが止まり、定義にある
/// 綴りなら読める。
///
/// テーマ名の綴りは検査の段には届かない（`ledger::read` が `model::parse_theme` で、
/// 検査へ渡すのと同じ 8 つの綴りと完全一致で読む）。だから状態の綴りと同じく読み取りの
/// 段で確かめる。摂動は「気配り」の末尾に空白 1 つを足したものである。テーマ定義には
/// 「気配」と「気配り」の 2 つがあり、片方が他方の接頭辞なので、部分一致や前後の空白
/// 落としで読む実装はこの摂動を素通りさせる。
#[test]
fn a_misspelled_theme_stops_the_ledger_read() {
    let path = paths::ledger_path(Domain::Shiori);
    let text = files::read_normalized(&path).unwrap_or_else(|err| panic!("{err}"));

    let (id, broken) = replace_first_line(&text, "values = ", r#"values = ["気配り "]"#);
    let err =
        read_ledger(&broken, Domain::Shiori).expect_err("綴りの違うテーマ名が読み取りを素通りした");
    let body = err.to_string();

    assert!(
        body.contains(&id),
        "失敗の本文が id を名指していない: {body}"
    );
    assert!(
        body.contains(SHIORI_LEDGER),
        "失敗の本文が場所を名指していない: {body}"
    );
    assert!(
        body.contains("気配り "),
        "失敗の本文が綴りの違うテーマ名そのものを載せていない: {body}"
    );

    // 定義にある綴りなら読める。
    let (_, fixed) = replace_first_line(&text, "values = ", r#"values = ["気配り"]"#);
    read_ledger(&fixed, Domain::Shiori).expect("定義にある綴りのテーマ名で読み取りが止まった");
}

/// 要件 6.10——状態の綴りが 7 つのいずれでもないと、id と場所つきで読み取りが止まる。
///
/// 状態の語彙は検査の段には届かない（`Ledger` は語彙を通った値しか持てない）。
/// だから読み取りの段で確かめる。壊すのは**読み込んだ本文の写し**で、repo の
/// ファイルには触れない。
#[test]
fn a_status_word_outside_the_seven_stops_the_ledger_read() {
    let path = paths::ledger_path(Domain::Shiori);
    let text = files::read_normalized(&path).unwrap_or_else(|err| panic!("{err}"));

    // 壊す前は読める。
    read_ledger(&text, Domain::Shiori).expect("実データの台帳が読めない");

    let (id, broken) = replace_first_line(&text, "status = ", r#"status = "jissou""#);
    let err = read_ledger(&broken, Domain::Shiori)
        .expect_err("語彙に無い状態の綴りが読み取りを素通りした");
    let body = err.to_string();

    assert!(
        body.contains(&id),
        "失敗の本文が id を名指していない: {body}"
    );
    assert!(
        body.contains(SHIORI_LEDGER),
        "失敗の本文が場所を名指していない: {body}"
    );
    assert!(
        body.contains("jissou"),
        "失敗の本文が綴りそのものを載せていない: {body}"
    );
}

/// 本文で `prefix` から始まる最初の行を `replacement` へ替え、その項目の id と替えた本文を
/// 返す。
fn replace_first_line(text: &str, prefix: &str, replacement: &str) -> (String, String) {
    let mut id: Option<String> = None;
    let mut broken = false;
    let mut out: Vec<String> = Vec::new();

    for line in text.lines() {
        if !broken {
            if let Some(rest) = line.strip_prefix("[entry.\"") {
                id = rest.strip_suffix("\"]").map(str::to_owned);
            }
            if line.starts_with(prefix) {
                out.push(replacement.to_owned());
                broken = true;
                continue;
            }
        }
        out.push(line.to_owned());
    }

    assert!(broken, "台帳の本文に {prefix} の行が無い");
    (
        id.unwrap_or_else(|| panic!("{prefix} の行の前に項目の見出しが無い")),
        out.join("\n"),
    )
}

/// 要件 7.4・7.5——ドメイン別報告が台帳と食い違うと、そのドメインを名指して赤くなる。
///
/// 壊すのは 1 本だけである。残りの 3 本が巻き添えで赤くならないこと（＝どのドメインの
/// 再生成が要るかが読めること）まで [`expect_exactly`] が固定する。
#[test]
fn a_stale_domain_report_turns_red_and_names_its_domain() {
    let data = RepoData::load();
    let mut copy = Perturbed::of(data);

    let body = copy
        .domain_reports
        .get_mut(&Domain::Property)
        .expect("property の報告を読んでいない");
    let mut lines: Vec<&str> = body.lines().collect();
    assert!(!lines.is_empty(), "property の報告が空");
    let rewritten = format!("{}（手で書き換えた）", lines[0]);
    lines[0] = rewritten.as_str();
    let edited = lines.join("\n");
    *body = edited;

    let findings = copy.findings();
    expect_exactly(
        &findings,
        &[(
            FindingKind::DomainReportStale,
            None,
            "doc/ukadoc-coverage/report/property.md",
        )],
    );
    assert!(
        findings[0].detail.contains("property"),
        "所見がどのドメインの報告かを言っていない: {}",
        findings[0].detail
    );
}

/// 要件 6.11——証拠の行が動いても検査は壊れない。
///
/// 同じ 1 行を先頭に置いた場合と、ずっと後ろに置いた場合とで、証拠の索引が**値として
/// 同じ**になることを見る。証拠は行番号を持たない（要件 5.1）ので、実装が入れ替わって
/// 正典 URL の行が上下しても同じ値になる。
#[test]
fn the_check_survives_lines_moving() {
    let data = RepoData::load();
    let line = format!("// ukadoc: {ANCHOR_URL}");

    let at_top = evidence_with_source_line(data, ANCHOR_SOURCE, &line, 0);
    let far_down = evidence_with_source_line(data, ANCHOR_SOURCE, &line, 40);
    assert_eq!(
        at_top, far_down,
        "証拠の索引が行の位置で変わった（要件 6.11・5.1）"
    );

    assert_the_added_line_became_evidence(data, &at_top);

    // 証拠が付いた状態でも、実データに食い違いは 1 件も生まれない（行が増えただけで
    // 赤にならないこと）。
    let mut copy = Perturbed::of(data);
    copy.evidence = at_top;
    expect_exactly(&copy.findings(), &[]);
}
