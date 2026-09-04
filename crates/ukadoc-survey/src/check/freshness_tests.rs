//! `freshness.rs` の在中テスト——**ドメイン別報告と台帳の突き合わせ**
//! （要件 6.13・7.4・7.5・7.6）。
//!
//! # 主張はすべて入口を通す
//!
//! どのテストも [`crate::check::run`] を呼び、`freshness::check` を直に呼ばない。部品を
//! 釘付けしても、入口がその部品を呼んでいるかは別に守る必要がある（タスク 4.1 からの
//! 必須の申し送り。判定を 1 つも呼ばない `run` は直に呼ぶテストでは永久に見つからない）。
//!
//! # 壊した対と壊さない対で置く
//!
//! 「所見が 0 件」だけの主張は、判定が何も返さない実装で無条件に真になる（タスク 1.6 の
//! 教訓）。復帰文字の側がとくにそうで、**復帰文字付きでも緑**という主張だけを置くと
//! 「常に一致と答える」実装が素通りする。だから復帰文字付きで**現に食い違う**本文が
//! 赤になることを対で置く。
//!
//! # 「全部を挙げる」は違反 2 件でしか守れない
//!
//! 報告は 4 本ある。1 本だけ壊した見本では「4 本ぜんぶ見る」と「最初の 1 本だけ見る」を
//! 区別できない（タスク 4.2 からの申し送り）。だから **先頭でも隣り合わせでもない
//! 2 本**（`assets` と `property`）を壊して件数まで主張する。
//!
//! # 台帳を渡す並びに左右されない
//!
//! 判定は [`Domain::ALL`] の順に見るので、台帳の並びは結果に出ない（要件 7.3）。
//! `World::normal()` は最初から [`Domain::ALL`] の順で台帳を作るため、同じ入力を 2 回
//! 流すテストではこの守りが見えない。**逆順の台帳**で 1 度通すテストがそこを釘付けする。

use std::collections::BTreeMap;

use super::super::{CheckInput, Finding, FindingKind, run};
use crate::ledger::Ledger;
use crate::lib_test_support::World;
use crate::model::{Domain, Status, THEMES};

// ---------------------------------------------------------------------------
// テスト専用の道具
// ---------------------------------------------------------------------------

/// 報告のファイルパス（所見の「場所」に載る綴り）。実装の関数は参照しない
/// ——表を表自身と比べるだけになると転記の誤りを 1 件も捕まえられない（タスク 1.5 の
/// 教訓）。姉妹の `content_test_support` は `content` の私有モジュールなので、ここからは
/// 見えない。写すのではなく、この判定に要る分だけを独立した文字列で書く。
fn report_place(domain: &str) -> String {
    format!("doc/ukadoc-coverage/report/{domain}.md")
}

/// 全体報告の場所。**ここに所見が出てはならない**（要件 7.6）。
const SUMMARY_PLACE: &str = "doc/ukadoc-coverage/report/summary.md";

/// 見本の報告に必ず 1 行だけ現れる注意書き（実装の定数は参照しない独立したリテラル）。
const HAND_EDIT_LINE: &str =
    "この本文は台帳から機械で作ります。手で書き換えず、食い違いは作り直して直します。";

/// 出た所見を種類ごとに数える（0 件の種類は落とす）。
///
/// これを**等式**で主張すると、意図しない種類が 1 件でも出れば赤になる。件数だけの
/// 主張は中身が全部誤っていても緑になるので、場所と詳細の逐語の主張と必ず対で置く
/// （タスク 1.5 の教訓）。
fn kinds(findings: &[Finding]) -> Vec<(FindingKind, usize)> {
    FindingKind::ALL
        .into_iter()
        .filter_map(|kind| {
            let count = findings.iter().filter(|f| f.kind == kind).count();
            (count > 0).then_some((kind, count))
        })
        .collect()
}

/// その種類の所見だけを出た順に取り出す。
fn of_kind(findings: &[Finding], kind: FindingKind) -> Vec<&Finding> {
    findings.iter().filter(|f| f.kind == kind).collect()
}

/// その種類の所見の場所を出た順に並べる。
fn places(findings: &[Finding], kind: FindingKind) -> Vec<&str> {
    of_kind(findings, kind)
        .into_iter()
        .map(|finding| finding.place.as_str())
        .collect()
}

/// その種類の所見の詳細を出た順に並べる。
fn details(findings: &[Finding], kind: FindingKind) -> Vec<&str> {
    of_kind(findings, kind)
        .into_iter()
        .map(|finding| finding.detail.as_str())
        .collect()
}

/// 報告の 1 行を書き換える。
///
/// 書き換えが**現に効いた**ことを先に確かめる。綴りを写し間違えたテストは、何も
/// 壊さないまま「壊したつもり」で緑になる。
fn rewrite_one_line(world: &mut World, domain: Domain) {
    let body = world.report_mut(domain);
    assert!(
        body.contains(HAND_EDIT_LINE),
        "見本の報告に書き換える行が無い"
    );
    let after = body.replace(HAND_EDIT_LINE, "手で書き換えました。");
    assert_ne!(*body, after, "書き換えが効いていない");
    *body = after;
}

/// 報告の改行を復帰文字付きにする（新しく clone した作業ツリーの形・設計 D-6）。
fn add_carriage_returns(world: &mut World, domain: Domain) {
    let body = world.report_mut(domain);
    assert!(
        !body.contains('\r'),
        "見本の報告に復帰文字があってはならない"
    );
    let after = body.replace('\n', "\r\n");
    assert_ne!(*body, after, "復帰文字を入れられていない");
    *body = after;
}

// ---------------------------------------------------------------------------
// 壊していない見本（対の片方）と、見るべき材料があること（要件 6.13）
// ---------------------------------------------------------------------------

/// 正常な見本は所見を 1 件も出さない。
///
/// これは否定の主張なので、以下の壊した側の主張と必ず対で読むこと。
#[test]
fn the_untouched_sample_world_has_no_findings() {
    let world = World::normal();
    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
}

/// 突き合わせる材料が見本にあることを肯定の側から数える（要件 6.13）。
///
/// 報告が 0 本、あるいは本文が空の入力なら「所見 0 件」は無条件に真になる。4 本ぜんぶ
/// 中身があり、しかも**その台帳の**報告であることをここで固定する。
#[test]
fn the_sample_world_holds_four_non_empty_domain_reports() {
    let world = World::normal();
    let outcome = run(&world.input());

    assert_eq!(outcome.stats.domain_reports, 4, "報告の本数");
    assert_eq!(outcome.stats.non_empty_domain_reports, 4, "空でない本数");

    let headings: Vec<&str> = Domain::ALL
        .iter()
        .map(|domain| {
            world.domain_reports[domain]
                .lines()
                .next()
                .expect("報告の本文が空では突き合わせが空回りする")
        })
        .collect();
    assert_eq!(
        headings,
        vec![
            "# shiori の網羅状況",
            "# assets の網羅状況",
            "# sakura-script の網羅状況",
            "# property の網羅状況",
        ]
    );
    for domain in Domain::ALL {
        assert!(
            !world.domain_reports[&domain].contains('\r'),
            "見本の報告に復帰文字があってはならない"
        );
    }
}

// ---------------------------------------------------------------------------
// 完了条件その 1——1 行を書き換えるとそのドメインの所見が 1 件出る（要件 7.4・7.5）
// ---------------------------------------------------------------------------

/// 報告の 1 行を書き換えると `DomainReportStale` が 1 件出る。
#[test]
fn rewriting_one_line_of_one_report_is_one_finding_for_that_domain() {
    let mut world = World::normal();
    rewrite_one_line(&mut world, Domain::SakuraScript);

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 1)]
    );
    let finding = &of_kind(&outcome.findings, FindingKind::DomainReportStale)[0];
    assert_eq!(finding.id, None, "主語は項目でなくドメインである");
    assert_eq!(finding.place, report_place("sakura-script"));
    assert_eq!(
        finding.detail,
        "sakura-script の報告が台帳から作り直した本文と一致しない。手で直さず作り直すこと"
    );
}

/// 台帳を直して報告を作り直さないと古くなる（現場でいちばん起きる形）。
///
/// 状態を 1 つ変えるだけで報告の分布表が変わる。`refresh_reports()` を呼ばないので
/// 報告だけが古いまま残り、他の判定は 1 件も出ない。
#[test]
fn a_ledger_changed_without_regenerating_its_report_is_stale() {
    let mut world = World::normal();
    let entry = world
        .ledger_mut(Domain::Property)
        .entries
        .values_mut()
        .find(|entry| entry.status == Status::Unclassified)
        .expect("見本の property 台帳に未分類の項目があるはず");
    entry.status = Status::NotApplicable;

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 1)]
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::DomainReportStale),
        vec![report_place("property")]
    );
}

// ---------------------------------------------------------------------------
// 完了条件その 2——復帰文字付きの本文でも緑になる（設計 D-6）
// ---------------------------------------------------------------------------

/// 報告 4 本すべてが復帰文字付きでも所見は 1 件も出ない。
///
/// これは否定の主張なので、次の「復帰文字付きで現に食い違う」テストと対で読むこと
/// ——対が無いと「常に一致と答える」実装がここを素通りする（タスク 1.6 の教訓）。
#[test]
fn report_bodies_with_carriage_returns_still_match() {
    let mut world = World::normal();
    for domain in Domain::ALL {
        add_carriage_returns(&mut world, domain);
    }
    assert!(
        world.domain_reports[&Domain::Shiori].contains("\r\n"),
        "復帰文字が入っていなければ試験にならない"
    );

    let outcome = run(&world.input());
    assert_eq!(kinds(&outcome.findings), vec![]);
}

/// 復帰文字付きでも、本文が現に違えば所見は出る（対のもう片方）。
#[test]
fn a_report_with_carriage_returns_that_really_differs_is_a_finding() {
    let mut world = World::normal();
    rewrite_one_line(&mut world, Domain::Shiori);
    add_carriage_returns(&mut world, Domain::Shiori);
    assert!(
        world.domain_reports[&Domain::Shiori].contains("\r\n"),
        "復帰文字が入っていなければ試験にならない"
    );

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 1)]
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::DomainReportStale),
        vec![report_place("shiori")]
    );
}

// ---------------------------------------------------------------------------
// 「4 本ぜんぶを見る」（タスク 4.2 からの申し送り）
// ---------------------------------------------------------------------------

/// 先頭でも隣り合わせでもない 2 本を壊すと、2 件とも出る。
///
/// `assets`（2 本目）と `property`（4 本目）を選ぶのは、「最初の 1 本だけ見る」と
/// 「最初に壊れた 1 本で止まる」と「隣を巻き込む」の 3 つをまとめて捕まえるためである。
/// 間の `sakura-script` は壊さないので、正しい 1 本を挟んでも走査が止まらないことも
/// 同時に固定される。
#[test]
fn two_stale_reports_are_both_reported() {
    let mut world = World::normal();
    rewrite_one_line(&mut world, Domain::Assets);
    rewrite_one_line(&mut world, Domain::Property);

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 2)]
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::DomainReportStale),
        vec![report_place("assets"), report_place("property")]
    );
    assert_eq!(
        details(&outcome.findings, FindingKind::DomainReportStale),
        vec![
            "assets の報告が台帳から作り直した本文と一致しない。手で直さず作り直すこと",
            "property の報告が台帳から作り直した本文と一致しない。手で直さず作り直すこと",
        ]
    );
}

/// 台帳を渡す並びは結果に出ない（要件 7.3）。
///
/// 逆順で通しても、出る所見は場所も詳細も**順番まで**同じである。`World::normal()` は
/// [`Domain::ALL`] の順で台帳を作るので、この主張は逆順の入力でしか見えない
/// （タスク 4.2 からの申し送り）。
#[test]
fn the_result_does_not_depend_on_the_order_of_the_ledgers() {
    let mut world = World::normal();
    rewrite_one_line(&mut world, Domain::Assets);
    rewrite_one_line(&mut world, Domain::Property);

    let reversed: Vec<Ledger> = world.ledgers.iter().rev().cloned().collect();
    assert_ne!(
        reversed[0].domain, world.ledgers[0].domain,
        "並びを変えていなければ試験にならない"
    );

    let forward = run(&world.input()).findings;
    let backward = run(&CheckInput {
        ledgers: &reversed,
        ..world.input()
    })
    .findings;

    let expected = vec![report_place("assets"), report_place("property")];
    assert_eq!(places(&forward, FindingKind::DomainReportStale), expected);
    assert_eq!(
        places(&backward, FindingKind::DomainReportStale),
        expected,
        "台帳を渡す並びが判定に漏れている"
    );
    assert_eq!(
        details(&forward, FindingKind::DomainReportStale),
        details(&backward, FindingKind::DomainReportStale)
    );
    assert_eq!(kinds(&forward), kinds(&backward));
}

// ---------------------------------------------------------------------------
// 作り直しに使う材料は入力から来る（設計 D-11）
// ---------------------------------------------------------------------------

/// 突き合わせ相手はその台帳 1 本と**入力のテーマ名**から作る（設計 D-11）。
///
/// 見知らぬテーマ名を 1 つ足すと、報告のテーマ別の表に 0 件の行が 1 本増える。テーマ名を
/// 別の値（実装の定数など）から取る実装はここで赤になる。8 つは全部残すので
/// `UnknownTheme` は出ない——出る所見は 4 本の古さだけである。
///
/// あわせて、所見の場所が 4 本のドメイン別報告に限られること（要件 7.6。
/// `summary.md` は常時検査に含めない）もここで固定する。
#[test]
fn the_themes_for_the_comparison_come_from_the_input() {
    let world = World::normal();
    let mut themes: Vec<&str> = THEMES.to_vec();
    themes.push("架空のテーマ");

    let outcome = run(&CheckInput {
        themes: &themes,
        ..world.input()
    });
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 4)]
    );
    let seen = places(&outcome.findings, FindingKind::DomainReportStale);
    assert_eq!(
        seen,
        vec![
            report_place("shiori"),
            report_place("assets"),
            report_place("sakura-script"),
            report_place("property"),
        ]
    );
    assert!(
        !seen.contains(&SUMMARY_PLACE),
        "全体報告は常時検査に含めない（要件 7.6）"
    );
}

// ---------------------------------------------------------------------------
// 揃っていない入力——報告が足りない側と、台帳が足りない側
// ---------------------------------------------------------------------------

/// 報告の本文が渡されていないドメインは所見になる。
///
/// 黙って飛ばすと、報告 1 本が丸ごと無い入力が緑になる。要件 7.5 は「どのドメインの
/// 再生成が要るか」を求めるので、無い側もその答えを言える形にしておく。
#[test]
fn a_domain_report_missing_from_the_input_is_a_finding() {
    let mut world = World::normal();
    let removed = world.domain_reports.remove(&Domain::Assets);
    assert!(removed.is_some(), "取り除けていなければ試験にならない");

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 1)]
    );
    let finding = &of_kind(&outcome.findings, FindingKind::DomainReportStale)[0];
    assert_eq!(finding.id, None);
    assert_eq!(finding.place, report_place("assets"));
    assert_eq!(
        finding.detail,
        "assets の報告の本文が渡されていない。作り直すこと"
    );
}

/// 台帳の無いドメインの報告は、この判定の持ち物ではない。
///
/// 作り直す元が無いので「古い」とは言えない。台帳が 4 本揃っていないことは構造の検査が
/// `CatalogIdMissingFromLedgers` として拾う（要件 6.4）ので、黙って消えるわけではない。
#[test]
fn a_report_without_a_ledger_is_not_this_judgements_business() {
    let world = World::normal();
    let three: Vec<Ledger> = world
        .ledgers
        .iter()
        .filter(|ledger| ledger.domain != Domain::SakuraScript)
        .cloned()
        .collect();
    assert_eq!(
        three.len(),
        3,
        "台帳を 1 本落としていなければ試験にならない"
    );
    let reports: &BTreeMap<Domain, String> = &world.domain_reports;
    assert_eq!(reports.len(), 4, "報告は 4 本のまま残す");

    let outcome = run(&CheckInput {
        ledgers: &three,
        ..world.input()
    });
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::CatalogIdMissingFromLedgers, 3)],
        "古さの所見は出ず、台帳の欠けは構造の検査が拾う"
    );
}

/// 同じドメインの台帳が 2 本来たら 2 本とも突き合わせる（片方を黙って落とさない）。
///
/// 見本は 1 ドメイン 1 本なので、2 本目を見るか捨てるかを見分けられない。ここだけ台帳を
/// 複製した入力を作って、`check` の doc が主張する防御そのものを釘付けする。台帳が
/// 重なっていること自体は構造の検査が `CatalogIdInMultipleLedgers` として拾う（要件 6.4）。
#[test]
fn two_ledgers_of_the_same_domain_are_both_compared() {
    let mut world = World::normal();
    rewrite_one_line(&mut world, Domain::Assets);
    let doubled: Vec<Ledger> = world
        .ledgers
        .iter()
        .flat_map(|ledger| {
            if ledger.domain == Domain::Assets {
                vec![ledger.clone(), ledger.clone()]
            } else {
                vec![ledger.clone()]
            }
        })
        .collect();
    assert_eq!(doubled.len(), 5, "複製できていなければ試験にならない");

    let outcome = run(&CheckInput {
        ledgers: &doubled,
        ..world.input()
    });
    assert_eq!(
        kinds(&outcome.findings),
        vec![
            (FindingKind::CatalogIdInMultipleLedgers, 3),
            (FindingKind::DomainReportStale, 2),
        ]
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::DomainReportStale),
        vec![report_place("assets"), report_place("assets")]
    );
}

// ---------------------------------------------------------------------------
// 突き合わせはバイト単位——空白 1 個も、空も見逃さない
// ---------------------------------------------------------------------------

/// 末尾に空行が 1 つ増えただけでも所見になる。
///
/// 突き合わせを**緩める**摂動の檻である（タスク 2.3 の教訓。等値の判定は必ず緩める
/// 向きを走らせる）。両側に `trim_end()` を挟む実装——末尾の空白を無かったことにする
/// ——を当てると、このテストだけが赤くなる（他の 475 本は緑のまま通る）。末尾に空行が
/// 増えるのは報告を手で開いて保存したときに現に起きる形で、要件 7.7 が禁じ
/// 要件 7.4・7.5 が捕まえることになっているものそのものである。
///
/// 復帰文字を落とすのは保存側の 1 回だけ（設計 D-6）で、それ以外の正規化は 1 つも
/// 挟まない——本文そのものが契約だから（タスク 3.5 の申し送り）。
#[test]
fn a_report_with_one_extra_trailing_newline_is_stale() {
    let mut world = World::normal();
    world.report_mut(Domain::Assets).push('\n');

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 1)]
    );
    let finding = &of_kind(&outcome.findings, FindingKind::DomainReportStale)[0];
    assert_eq!(finding.place, report_place("assets"));
    assert_eq!(
        finding.detail,
        "assets の報告が台帳から作り直した本文と一致しない。手で直さず作り直すこと"
    );
}

/// 先頭に空行が 1 つ増えただけでも所見になる。
///
/// 直前の末尾の檻と**対**である（タスク 4.3 の教訓「対になる判定は対で檻を置く」）。
/// 末尾だけを守っていたときは、両側に `trim_start()` を挟む実装が全 476 本を素通り
/// した。先頭の空白は末尾ほど起きやすくはないが、対の片側だけを守ると「どちらを
/// 守ったのか」が読む人に伝わらない。
#[test]
fn a_report_with_one_extra_leading_newline_is_stale() {
    let mut world = World::normal();
    world.report_mut(Domain::Assets).insert(0, '\n');

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 1)]
    );
    let finding = &of_kind(&outcome.findings, FindingKind::DomainReportStale)[0];
    assert_eq!(finding.place, report_place("assets"));
    assert_eq!(
        finding.detail,
        "assets の報告が台帳から作り直した本文と一致しない。手で直さず作り直すこと"
    );
}

/// 中身を空にされた報告も所見になる。
///
/// 報告が**渡されていない**ときに黙って飛ばさないことは
/// `a_domain_report_missing_from_the_input_is_a_finding` が守っているが、**空の本文**で
/// 飛ばす形はそれとは別の穴である。突き合わせの前に `if stored.is_empty() { return; }`
/// を挟む実装を当てると、このテストだけが赤くなる（他の 475 本は緑のまま通る）。
///
/// `ScanStats::non_empty_domain_reports`（`check/mod.rs`）はまさにこの空回りを心配して
/// 置かれた欄なので、件数が 3 に減っていることも同じ実行から確かめる——見本が空に
/// なっていなければ、この檻は何も守らないまま緑になる。
#[test]
fn a_report_emptied_to_zero_bytes_is_stale() {
    let mut world = World::normal();
    world.report_mut(Domain::Assets).clear();
    assert!(
        world.domain_reports[&Domain::Assets].is_empty(),
        "空にできていなければ試験にならない"
    );

    let outcome = run(&world.input());
    assert_eq!(outcome.stats.domain_reports, 4, "報告は 4 本のまま渡す");
    assert_eq!(
        outcome.stats.non_empty_domain_reports, 3,
        "空でない本数が減っていなければ試験にならない"
    );
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 1)]
    );
    let finding = &of_kind(&outcome.findings, FindingKind::DomainReportStale)[0];
    assert_eq!(finding.place, report_place("assets"));
    assert_eq!(
        finding.detail,
        "assets の報告が台帳から作り直した本文と一致しない。手で直さず作り直すこと"
    );
}
