//! 実データで検査の対象が 0 件でないことの主張（要件 6.13・設計 Testing Strategy 12〜15・17）。
//!
//! # なぜ件数を釘付けするのか
//!
//! 初期台帳は全行が「未分類」なので、整合検査の半分は**対象 0 件のまま緑になる**
//! （`checks.rs` の数え上げが、どの要件が今日空回りしているかを名指しで固定している）。
//! 対象が 0 件なら判定が丸ごと消えても緑である。だから「検査が何件を相手にしているか」
//! そのものを主張に変える——読み込みが空を返す・途中で打ち切る・同じファイルを 4 回
//! 読む、のいずれも、ここが赤で捕まえる。
//!
//! # 数は「写す」だけでなく「導く」
//!
//! 件数の literal（1,749・38・6・677・542・342・188・12・24・1・1）は要件 1 と要件 3.1
//! の実測だが、literal を並べるだけの主張は**その literal を書き換えれば黙って通る**。
//! だから同じ場所で関係も主張する:
//!
//! - カタログの前置き（`ukadoc_entries`）と実際に読めた行数が一致する。
//! - 台帳 4 本の項目数の**和**がカタログの項目数に等しく、4 本の id が互いに重ならない。
//! - ドメインごとの台帳の項目数を、カタログ側からページ→ドメインの割り当てで
//!   数え直した件数と突き合わせる。
//! - 割り当ての 12+24+1+1 が 38 に等しく、その 38 ページの集合が**カタログに実在する
//!   ページの集合と一致する**（割り当ては Rust の定数なので、ディスクのデータと結ばない
//!   限り「定数が定数を言い換える」だけになる）。
//! - ドメイン別報告の本文に、その台帳の件数が「合計」行として書かれている。
//!
//! # 否定の主張には肯定の対を置く
//!
//! 「走査が調査クレート由来のファイルを 1 つも含まない」は、走査が何も返さなければ
//! 無条件に成り立つ。だから同じ事例で「実在する名前のファイルへ届いている」ことを
//! 先に言う。
//!
//! # 長さはバイトでなく文字で数える
//!
//! カタログの最長行は 579 文字だが 619 バイトである（タスク 7.2 の申し送り）。
//! `str::len()` はバイト長を返すので、本文の長さを言うときは `chars().count()` を使う。
//!
//! # ここに置かないもの
//!
//! - 要件ごとの対象の**有無**（空回りしている要件の名指し）は `checks.rs` の持ち物。
//! - `values.md` の見出しと `model::THEMES` の一致、語彙表経路の較正はタスク 8.4 の
//!   持ち物である（設計 Testing Strategy 16・17a）。
//! - 全体報告 `summary.md` は読まない（要件 7.6・`mod.rs` の冒頭）。

use std::collections::{BTreeMap, BTreeSet};

use ukadoc_survey::io::paths;
use ukadoc_survey::model::{Domain, PageName};

use super::RepoData;

/// 正典（ukadoc）の項目数（要件 1 の実測）。
const CATALOG_ENTRIES: usize = 1_749;

/// 正典のページ数（要件 1・3.1）。カタログにも割り当てにも同じ数だけ現れる。
const CATALOG_PAGES: usize = 38;

/// カテゴリ名と件数（要件 1 の実測・6 種）。並びは名前順（`BTreeMap` の走査順）。
const CATEGORIES: [(&str, usize); 6] = [
    ("descript", 518),
    ("dev_guide", 7),
    ("file_structure", 8),
    ("protocol", 237),
    ("sakurascript", 342),
    ("shiori_event", 637),
];

/// ドメインごとの「台帳の項目数・担当ページ数」（要件 3.1 の表）。
///
/// 並びは [`Domain::ALL`]（＝[`RepoData::ledgers`] の並び）と同じでなければならない。
/// 食い違えば下の各事例が最初の突き合わせで赤くなる。
const DOMAIN_ROWS: [(Domain, usize, usize); 4] = [
    (Domain::Shiori, 677, 12),
    (Domain::Assets, 542, 24),
    (Domain::SakuraScript, 342, 1),
    (Domain::Property, 188, 1),
];

/// 走査が届いていることの錨（別々のクレートに実在するファイル・要件 9.2）。
const ANCHOR_SOURCES: [&str; 3] = [
    "crates/areka-sylphya/src/vocab/shiori_resource.rs",
    "crates/dola/src/lib.rs",
    "crates/log-capture-kit/src/lib.rs",
];

/// 走査から外す調査クレート自身（設計 D-3）。この接頭辞を持つ相対パスは 1 本も来ない。
///
/// 実装側の `io::sources::SELF_CRATE_DIR` は crate 内公開で、ここ（別クレートの統合
/// テスト）からは型検査が拒む。仮に借りられたとしても借りない——同じ定数を両側で
/// 使えば除外の主張は「実装が実装に同意する」だけになり、綴りを書き換えた瞬間に
/// 主張ごと付いてきてしまう。だから**わざと書き写す**。
///
/// 書き写した綴りには別の弱さがある。クレートの名前が変われば、この接頭辞はどこも
/// 指さない文字列になり、除外の主張（「この接頭辞で始まるパスは来ない」）は無条件に
/// 成り立って**黙って空回りする**。それを赤に変えるため、下の事例はこの接頭辞の下に
/// [`SELF_CRATE_MARKER`] が実在することを先に確かめる。
const SELF_CRATE_PREFIX: &str = "crates/ukadoc-survey/";

/// [`SELF_CRATE_PREFIX`] が実在の場所を指していることの目印（調査クレートの入口）。
const SELF_CRATE_MARKER: &str = "src/lib.rs";

/// 走査が返すファイル数の下限（実測 1,089 本）。
///
/// 実数を釘付けするとファイルを 1 本足すたびに赤くなるので下限にする。狙いは
/// 「0 件でない」の担保（要件 6.13・設計 Testing Strategy 15）で、走査が空を返す・
/// 起点を取り違える・途中の失敗を握り潰す、といった**ほとんど何も残らない**壊れ方を
/// 捕まえる。
///
/// 捕まえないものも書いておく。走査が 1 本おきにファイルを落とすような壊れ方
/// （実測 1,089 本の半分＝およそ 545 本）は、この下限も、下のクレート数の下限も、
/// 錨（[`ANCHOR_SOURCES`]）も、名前順の主張も**すべて素通りする**。ここが守るのは
/// 「ほぼ全滅していない」であって「取りこぼしが無い」ではない。後者を言うには
/// テストの側で走査をもう 1 度独立に組み直す必要があり、それはこの一群の役目を
/// 越える（要件 6.13 が問うのは「0 件でない」ことである）。
const MIN_SOURCE_FILES: usize = 500;

/// 走査に現れるクレートの数の下限（実測 24＝調査クレートを除く全部）。
///
/// 性格はファイル数の下限と同じで、クレートが 1 つ丸ごと走査から落ちる壊れ方までは
/// 届くが、どのクレートも中身が半分になる壊れ方には届かない。
const MIN_SOURCE_CRATES: usize = 20;

/// ドメイン別報告の本文の下限（文字数。実測の最小は property の 1,126 文字）。
const MIN_REPORT_CHARS: usize = 500;

/// カタログが 1,749 項目・38 ページ・6 カテゴリを抱えていること
/// （設計 Testing Strategy 12）。
#[test]
fn the_catalog_holds_1749_entries_over_38_pages_and_6_categories() {
    let data = RepoData::load();
    let catalog = &data.catalog;

    assert_eq!(
        catalog.entries.len(),
        CATALOG_ENTRIES,
        "カタログの項目数が {CATALOG_ENTRIES} でない。\
         スナップショットを入れ替えたなら要件 1 の実測から書き換えること"
    );
    // 前置きの申告と実際に読めた行数を突き合わせる。読み込みが途中で打ち切っても、
    // 行を黙って畳んでも、前置きの数は変わらないのでここが食い違う。
    assert_eq!(
        catalog.snapshot.ukadoc_entries,
        catalog.entries.len(),
        "カタログの前置きが申告する件数と、読めた項目の数が違う（読み落としている）"
    );
    for (key, entry) in &catalog.entries {
        assert_eq!(
            &entry.id, key,
            "鍵と項目の id が食い違う（読み込みが別の項目を同じ鍵へ入れている）"
        );
    }

    let mut by_page: BTreeMap<&PageName, usize> = BTreeMap::new();
    let mut by_category: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in catalog.entries.values() {
        *by_page.entry(&entry.page).or_default() += 1;
        *by_category.entry(entry.category.as_str()).or_default() += 1;
    }

    assert_eq!(
        by_page.len(),
        CATALOG_PAGES,
        "カタログに現れるページが {CATALOG_PAGES} 種でない"
    );
    assert_eq!(
        by_page.values().sum::<usize>(),
        CATALOG_ENTRIES,
        "ページ別の件数の和が項目数に合わない"
    );

    let found: Vec<(&str, usize)> = by_category.iter().map(|(name, n)| (*name, *n)).collect();
    assert_eq!(
        found.as_slice(),
        CATEGORIES.as_slice(),
        "カテゴリの顔ぶれか件数が要件 1 の実測（6 種）と違う"
    );
    assert_eq!(
        CATEGORIES.iter().map(|(_, n)| n).sum::<usize>(),
        CATALOG_ENTRIES,
        "カテゴリ別の件数の和が {CATALOG_ENTRIES} にならない（表の側が壊れている）"
    );
}

/// 台帳 4 本が 677・542・342・188 を抱え、重なりなくカタログを覆っていること
/// （設計 Testing Strategy 13）。
#[test]
fn the_four_ledgers_hold_677_542_342_188_rows_that_add_up_to_the_catalog() {
    let data = RepoData::load();

    assert_eq!(
        data.ledgers.len(),
        DOMAIN_ROWS.len(),
        "台帳が 4 本でない（読み込みが 1 本落としている）"
    );

    let mut total = 0;
    let mut union = BTreeSet::new();
    for (ledger, (domain, rows, _)) in data.ledgers.iter().zip(DOMAIN_ROWS) {
        let key = domain.as_key();
        assert_eq!(ledger.domain, domain, "台帳の並びが Domain::ALL と違う");
        assert_eq!(
            ledger.entries.len(),
            rows,
            "{key} の台帳の項目数が {rows} でない（要件 3.1 の表）"
        );
        // 本文に現れた順の列と鍵の表は同じ長さでなければならない。塊の切り分けが
        // 行を畳むと、鍵の表だけが縮んで件数の主張が別の理由で赤くなる。
        assert_eq!(
            ledger.file_order.len(),
            ledger.entries.len(),
            "{key} の台帳で同じ id が 2 度書かれている（鍵の表が畳んだ）"
        );
        // 同じ件数をカタログ側から数え直す。台帳を書き換えれば片方だけが動く。
        let from_catalog = data
            .catalog
            .entries
            .values()
            .filter(|entry| data.assignment.domain_of(&entry.page) == Some(domain))
            .count();
        assert_eq!(
            from_catalog, rows,
            "{key} の担当ページに属するカタログの項目数が {rows} でない"
        );

        total += ledger.entries.len();
        union.extend(ledger.entries.keys());
    }

    assert_eq!(
        total, CATALOG_ENTRIES,
        "台帳 4 本の項目数の和がカタログの項目数と違う"
    );
    assert_eq!(
        union.len(),
        total,
        "台帳どうしが同じ id を持っている（和は合うのに覆いが重なっている）"
    );
    assert_eq!(
        data.catalog.entries.len(),
        total,
        "カタログの項目数と台帳の総数が違う"
    );
}

/// 割り当てが 38 ページを 12・24・1・1 に分け、カタログのページを漏れなく覆うこと
/// （設計 Testing Strategy 14）。
#[test]
fn the_assignment_splits_38_pages_into_12_24_1_1_and_covers_the_catalog() {
    let data = RepoData::load();

    let mut assigned_pages = BTreeSet::new();
    let mut total = 0;
    for (ledger, (domain, _, pages)) in data.ledgers.iter().zip(DOMAIN_ROWS) {
        let key = domain.as_key();
        assert_eq!(ledger.domain, domain, "台帳の並びが Domain::ALL と違う");

        let mine = data.assignment.pages_of(domain);
        assert_eq!(
            mine.len(),
            pages,
            "{key} の担当ページが {pages} 種でない（要件 3.1 の表）"
        );
        // 割り当ては Rust の定数なので、ディスクの台帳が申告する担当ページ数と結ぶ。
        // これを欠くと「定数が定数を言い換える」だけの主張になる。
        assert_eq!(
            ledger.pages.len(),
            pages,
            "{key} の台帳の前置きが申告する担当ページ数が {pages} でない"
        );

        for page in &mine {
            assert!(
                assigned_pages.insert(page.clone()),
                "{} が 2 つのドメインに割り当てられている",
                page.as_str()
            );
            assert_eq!(
                data.assignment.domain_of(page),
                Some(domain),
                "{} の引き当てが担当ページの一覧と食い違う",
                page.as_str()
            );
        }
        total += mine.len();
    }

    assert_eq!(
        total, CATALOG_PAGES,
        "内訳の和が {CATALOG_PAGES} ページにならない"
    );
    assert_eq!(
        assigned_pages.len(),
        CATALOG_PAGES,
        "割り当てが 38 ページでない"
    );

    let catalog_pages: BTreeSet<PageName> = data
        .catalog
        .entries
        .values()
        .map(|entry| entry.page.clone())
        .collect();
    assert_eq!(
        assigned_pages, catalog_pages,
        "割り当てのページ集合が、カタログに実在するページ集合と違う"
    );
    assert!(
        data.assignment.unassigned(catalog_pages.iter()).is_empty(),
        "カタログのページに担当の無いものがある"
    );
}

/// ソースの走査が実在するファイルへ届き、調査クレート由来を 1 本も含まないこと
/// （設計 Testing Strategy 15）。
#[test]
fn the_source_walk_reaches_other_crates_and_never_the_survey_crate() {
    let data = RepoData::load();

    // 否定の主張（調査クレート由来を含まない）は走査が空でも成り立つ。先に肯定を言う。
    assert!(
        !data.sources.is_empty(),
        "ソースの走査が 1 本も返していない"
    );
    assert!(
        data.sources.len() >= MIN_SOURCE_FILES,
        "走査が返したのは {} 本で、下限 {MIN_SOURCE_FILES} を割っている",
        data.sources.len()
    );
    for anchor in ANCHOR_SOURCES {
        let (_, body) = data
            .sources
            .iter()
            .find(|(path, _)| path == anchor)
            .unwrap_or_else(|| panic!("走査が {anchor} へ届いていない"));
        // 本文まで読めていること。パスだけ集めて中身を捨てると証拠が 1 つも作れない。
        assert!(
            !body.trim().is_empty(),
            "{anchor} を空の本文として読んでいる"
        );
        if anchor == ANCHOR_SOURCES[0] {
            assert!(
                body.contains("SHIORI_RESOURCE_IDS"),
                "{anchor} の本文が別のファイルの中身になっている"
            );
        }
    }

    // 除外の主張（下の `!path.starts_with(...)`）も否定なので、接頭辞がどこも指さない
    // 文字列に成り下がれば無条件に成り立つ。クレート名が変わったときに黙って空回り
    // させないため、写した綴りがディスクの実在の場所であることを先に言う。
    let self_crate_marker = paths::workspace_root()
        .join(SELF_CRATE_PREFIX)
        .join(SELF_CRATE_MARKER);
    assert!(
        self_crate_marker.is_file(),
        "{SELF_CRATE_PREFIX} が実在しない（調査クレートの名前が変わったなら \
         SELF_CRATE_PREFIX を書き換えること）: {}",
        self_crate_marker.display()
    );

    let mut crates = BTreeSet::new();
    for (path, _) in &data.sources {
        assert!(
            path.starts_with("crates/") && path.ends_with(".rs"),
            "走査が crates 配下の Rust ファイル以外を返した: {path}"
        );
        assert!(
            !path.starts_with(SELF_CRATE_PREFIX),
            "調査クレート由来のファイルを拾っている: {path}"
        );
        let rest = path.strip_prefix("crates/").unwrap_or_default();
        crates.insert(rest.split('/').next().unwrap_or_default());
    }
    assert!(
        !crates.contains("ukadoc-survey"),
        "調査クレートが走査に現れている"
    );
    assert!(
        crates.len() >= MIN_SOURCE_CRATES,
        "走査に現れたクレートは {} 個で、下限 {MIN_SOURCE_CRATES} を割っている",
        crates.len()
    );

    // 名前順・重複なしで返す約束（`io::sources::walk`）。同じ本文を 2 度数えていない。
    assert!(
        data.sources.windows(2).all(|pair| pair[0].0 < pair[1].0),
        "走査の結果が名前順でないか、同じパスが 2 度現れている"
    );
}

/// ドメイン別報告 4 本が実在し、いずれも空でなく、互いに別の本文であること
/// （設計 Testing Strategy 17・要件 7.4 の検査が空回りしていないこと）。
#[test]
fn the_four_domain_reports_exist_on_disk_and_are_not_empty() {
    let data = RepoData::load();

    // 本数（`domain_reports.len()`）は読み込みが `Domain::ALL` を回して作るので構造的に
    // 4 であり、それ自体は何も言わない。だからディスクの実在と本文を 1 本ずつ見る。
    let mut bodies = BTreeSet::new();
    for (domain, rows, _) in DOMAIN_ROWS {
        let key = domain.as_key();
        let path = paths::domain_report_path(domain);
        assert!(path.is_file(), "報告が実在しない: {}", path.display());

        let body = data
            .domain_reports
            .get(&domain)
            .unwrap_or_else(|| panic!("{key} の報告を読んでいない"));
        assert!(!body.is_empty(), "{key} の報告が空");
        // 長さは文字数で数える（バイト長ではない・このファイル冒頭の理由）。
        assert!(
            body.chars().count() >= MIN_REPORT_CHARS,
            "{key} の報告が {} 文字しかない（下限 {MIN_REPORT_CHARS}）",
            body.chars().count()
        );

        // 4 本が同じファイルの読み直しでないこと。見出しが自分のドメインを名乗り、
        // 本文どうしも相異なる。
        let heading = format!("# {key} の網羅状況");
        assert_eq!(
            body.lines().next(),
            Some(heading.as_str()),
            "{key} の報告が別のファイルの本文になっている"
        );
        assert!(
            bodies.insert(body.as_str()),
            "{key} の報告が他のドメインと同じ本文である"
        );

        // 本文が台帳の件数と繋がっていること。台帳が動けば報告も動くので、
        // 「空でない」だけの主張より 1 段強い。
        let tally = format!("\n| 合計 | {rows} |\n");
        assert!(
            body.contains(&tally),
            "{key} の報告に「合計 {rows}」の行が無い（台帳の件数と繋がっていない）"
        );
    }
    assert_eq!(
        bodies.len(),
        DOMAIN_ROWS.len(),
        "相異なる報告が 4 本そろっていない"
    );
}
