//! repo の実データに対する主張（要件 6.1・6.2・6.12・設計 Testing Strategy 18・19）。
//!
//! 読み込みは [`RepoData`] が引き受け、ここは**読んだ値に何を求めるか**だけを書く。
//! 対になる 2 本を置く——「食い違いが 1 件も無いこと」と「その主張が空振りでないこと」
//! である。
//!
//! # 赤になったときに読むもの
//!
//! 失敗の本文は [`ukadoc_survey::check::render`] が整えた食い違いの一覧**そのもの**で
//! ある（要件 6.12）。件数だけの要約に置き換えない——台帳を壊した人が、テストの出力
//! だけを見て直せるようにするためである。
//!
//! # 「食い違い 0 件」は、それだけでは何も言わない
//!
//! 初期台帳は全行が未分類なので、要件 6.6〜6.8 の判定は対象 0 件でも緑になる。
//! 読み込みが空振りしていても同じく緑になるので、**読み込みが実データに届いている
//! ことを別の事例で主張する**（[`the_load_reaches_the_real_repo_data`]）。件数そのものを
//! 釘付けする一群はタスク 8.3 が `consistency/non_vacuity.rs` へ置く。

use ukadoc_survey::check::{render, run};
use ukadoc_survey::model::{Domain, EntryId, THEMES};

use super::RepoData;

/// 突き合わせの錨に使う項目 id（`doc/ukadoc-coverage/catalog.toml` に実在する）。
const ANCHOR_ID: &str = "ukadoc:list_shiori_event:OnBoot:1";

/// その項目の正典 URL（カタログの綴りを逐語で写したもの）。
const ANCHOR_URL: &str = "https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBoot:1";

/// 走査が届いていることの錨に使うソース（既存の語彙台帳・要件 9.2）。
const ANCHOR_SOURCE: &str = "crates/areka-sylphya/src/vocab/shiori_resource.rs";

/// repo の実データに食い違いが 1 件も無いこと（要件 6.1・6.12）。
///
/// 失敗の本文は整形した一覧そのものである。
#[test]
fn real_repo_data_produces_no_findings() {
    let data = RepoData::load();
    let outcome = run(&data.input());

    assert!(outcome.findings.is_empty(), "{}", render(&outcome.findings));
}

/// 読み込みが repo の実データに届いていること。
///
/// 「食い違い 0 件」を空振りで満たす道は 2 つある——読み込みが空を返す道と、判定の表を
/// たたみ込み損ねる道である。どちらも所見を 1 件も生まないので、上の事例だけでは
/// 見分けられない。ここで塞ぐ。
///
/// 証拠の索引（[`RepoData::evidence`]）は今どこも空である。ソースに正典 URL がまだ
/// 1 行も置かれていないからで、置かれた後に有効になる非空の主張はタスク 8.4 の持ち物で
/// ある。代わりに**走査そのものが実データへ届いたこと**を主張する。
#[test]
fn the_load_reaches_the_real_repo_data() {
    let data = RepoData::load();
    let input = data.input();
    let outcome = run(&input);

    assert_eq!(
        outcome.stats.judgements_run,
        vec!["structure", "content", "freshness"],
        "判定の表を 1 つでも通し損ねている"
    );

    let anchor = EntryId::parse(ANCHOR_ID).expect("錨の id の形が違う");
    let entry = data
        .catalog
        .entries
        .get(&anchor)
        .unwrap_or_else(|| panic!("カタログに {ANCHOR_ID} が無い"));
    assert_eq!(entry.page.as_str(), "list_shiori_event");
    assert_eq!(entry.category, "shiori_event");
    assert_eq!(entry.url, ANCHOR_URL);

    assert_eq!(data.ledgers.len(), Domain::ALL.len(), "台帳が 4 本でない");
    for (ledger, domain) in data.ledgers.iter().zip(Domain::ALL) {
        assert_eq!(ledger.domain, domain, "台帳の並びが Domain::ALL と違う");
        assert!(!ledger.entries.is_empty(), "{} の台帳が空", domain.as_key());
    }
    let shiori = &data.ledgers[0];
    assert!(
        shiori.entries.contains_key(&anchor),
        "shiori の台帳に {ANCHOR_ID} が無い"
    );

    for domain in Domain::ALL {
        let body = data
            .domain_reports
            .get(&domain)
            .unwrap_or_else(|| panic!("{} の報告を読んでいない", domain.as_key()));
        assert!(!body.is_empty(), "{} の報告が空", domain.as_key());
    }
    assert!(
        data.domain_reports[&Domain::Shiori].starts_with("# shiori の網羅状況\n"),
        "shiori の報告が別のファイルの本文になっている"
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
