//! `freshness.rs` の在中テスト——**報告 5 本（ドメイン別 4 本と全体報告）と台帳の
//! 突き合わせ**。
//!
//! # 主張はすべて入口を通す
//!
//! どのテストも [`crate::check::run`] を呼び、`freshness::check` を直に呼ばない。部品を
//! 釘付けしても、入口がその部品を呼んでいるかは別に守る必要がある（判定を 1 つも
//! 呼ばない `run` は直に呼ぶテストでは永久に見つからない）。
//!
//! # 壊した対と壊さない対で置く
//!
//! 「所見が 0 件」だけの主張は、判定が何も返さない実装で無条件に真になる。壊していない
//! 見本で 0 件になる側は `structure_tests::the_untouched_sample_world_has_no_findings`
//! が `run` を通して全判定まとめて持ち、ここには壊した側を置く。とくに復帰文字の側では、
//! **復帰文字付きでも緑**という主張だけを置くと「常に一致と答える」実装が素通りする。
//! だから復帰文字付きで**現に食い違う**本文が赤になることを対で置く。
//!
//! # 「全部を挙げる」は違反 2 件でしか守れない
//!
//! ドメイン別報告は 4 本ある。1 本だけ壊した見本では「4 本ぜんぶ見る」と「最初の 1 本だけ
//! 見る」を区別できない。だから **先頭でも隣り合わせでもない 2 本**（`assets` と
//! `property`）を壊して件数まで主張する。
//!
//! # 台帳を渡す並びに左右されない
//!
//! 判定は [`Domain::ALL`] の順に見るので、台帳の並びは結果に出ない。
//! `World::normal()` は最初から [`Domain::ALL`] の順で台帳を作るため、同じ入力を 2 回
//! 流すテストではこの守りが見えない。**逆順の台帳**で 1 度通すテストがそこを釘付けする。

use std::collections::BTreeMap;

use super::super::test_support::{details, kinds, of_kind, places};
use super::super::{CheckInput, FindingKind, run};
use crate::ledger::Ledger;
use crate::lib_test_support::World;
use crate::model::{Domain, Status, THEMES};

// ---------------------------------------------------------------------------
// テスト専用の道具
// ---------------------------------------------------------------------------

/// 報告のファイルパス（所見の「場所」に載る綴り）。実装の関数は参照しない
/// ——表を表自身と比べるだけになると転記の誤りを 1 件も捕まえられない。共用の道具は
/// `check::test_support` にあるが、報告の場所の綴りはこの判定だけが使うので、独立した
/// 文字列でここに書く。
fn report_place(domain: &str) -> String {
    format!("doc/ukadoc-coverage/report/{domain}.md")
}

/// 全体報告の場所（実装の綴りは参照しない独立したリテラル）。
const SUMMARY_PLACE: &str = "doc/ukadoc-coverage/report/summary.md";

/// 全体報告が古いときの所見の本文（直し方の副手続きを名指す）。
const SUMMARY_STALE_DETAIL: &str = "全体報告がカタログと台帳から作り直した本文と一致しない。手で直さず `report-summary` で作り直すこと";

/// 見本の報告に必ず 1 行だけ現れる注意書き（実装の定数は参照しない独立したリテラル）。
const HAND_EDIT_LINE: &str =
    "この本文は台帳から機械で作ります。手で書き換えず、食い違いは作り直して直します。";

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

/// 報告の改行を復帰文字付きにする。
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
// 見るべき材料が見本にあること
// ---------------------------------------------------------------------------

/// 突き合わせる材料が見本にあることを肯定の側から数える。
///
/// 報告が 0 本、あるいは本文が空の入力なら「所見 0 件」は無条件に真になる。4 本ぜんぶ
/// 中身があり、しかも**その台帳の**報告であることをここで固定する。
#[test]
fn the_sample_world_holds_four_non_empty_domain_reports() {
    let world = World::normal();

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
// 1 行を書き換えるとそのドメインの所見が 1 件出る
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
/// 状態を 1 つ変えるだけで、そのドメインの報告と全体報告の分布表が変わる。
/// `refresh_reports()` を呼ばないので報告の 2 本だけが古いまま残り、他の判定は 1 件も
/// 出ない。
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
        vec![(FindingKind::DomainReportStale, 2)]
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::DomainReportStale),
        vec![report_place("property").as_str(), SUMMARY_PLACE]
    );
}

// ---------------------------------------------------------------------------
// 復帰文字付きの本文でも緑になる
// ---------------------------------------------------------------------------

/// 報告 4 本すべてが復帰文字付きでも所見は 1 件も出ない。
///
/// これは否定の主張なので、次の「復帰文字付きで現に食い違う」テストと対で読むこと
/// ——対が無いと「常に一致と答える」実装がここを素通りする。
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
// 「4 本ぜんぶを見る」
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

/// 台帳を渡す並びは結果に出ない。
///
/// 逆順で通しても、出る所見は場所も詳細も**順番まで**同じである。`World::normal()` は
/// [`Domain::ALL`] の順で台帳を作るので、この主張は逆順の入力でしか見えない。
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
// 作り直しに使う材料は入力から来る
// ---------------------------------------------------------------------------

/// 突き合わせ相手はその台帳 1 本と**入力のテーマ名**から作る。
///
/// 見知らぬテーマ名を 1 つ足すと、報告のテーマ別の表に 0 件の行が 1 本増える。テーマ名を
/// 別の値（実装の定数など）から取る実装はここで赤になる。出る所見は報告 5 本の古さ
/// だけで、全体報告が最後に来る。
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
        vec![(FindingKind::DomainReportStale, 5)]
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::DomainReportStale),
        vec![
            report_place("shiori").as_str(),
            &report_place("assets"),
            &report_place("sakura-script"),
            &report_place("property"),
            SUMMARY_PLACE,
        ]
    );
}

// ---------------------------------------------------------------------------
// 揃っていない入力——報告が足りない側と、台帳が足りない側
// ---------------------------------------------------------------------------

/// 報告の本文が渡されていないドメインは所見になる。
///
/// 黙って飛ばすと、報告 1 本が丸ごと無い入力が緑になる。どのドメインの再生成が要るかを
/// 言えるよう、無い側も所見にする。
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
/// `CatalogIdMissingFromLedgers` として拾うので、黙って消えるわけではない。
/// 全体報告は 3 本の台帳から作り直した本文と比べるので、4 本で作った本文は古い。
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
        vec![
            (FindingKind::CatalogIdMissingFromLedgers, 3),
            (FindingKind::DomainReportStale, 1),
        ],
        "ドメイン別報告の古さの所見は出ず、台帳の欠けは構造の検査が拾う"
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::DomainReportStale),
        vec![SUMMARY_PLACE]
    );
}

/// 同じドメインの台帳が 2 本来たら 2 本とも突き合わせる（片方を黙って落とさない）。
///
/// 見本は 1 ドメイン 1 本なので、2 本目を見るか捨てるかを見分けられない。ここだけ台帳を
/// 複製した入力を作って、`check` の doc が主張する防御そのものを釘付けする。台帳が
/// 重なっていること自体は構造の検査が `CatalogIdInMultipleLedgers` として拾う。
/// 全体報告は 2 本目の行も並べるので、4 本で作った本文とは食い違う。
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
            (FindingKind::DomainReportStale, 3),
        ]
    );
    assert_eq!(
        places(&outcome.findings, FindingKind::DomainReportStale),
        vec![
            report_place("assets").as_str(),
            &report_place("assets"),
            SUMMARY_PLACE
        ]
    );
}

// ---------------------------------------------------------------------------
// 突き合わせはバイト単位——空白 1 個も、空も見逃さない
// ---------------------------------------------------------------------------

/// 末尾に空行が 1 つ増えただけでも所見になる。
///
/// 突き合わせを**緩める**摂動の檻である（等値の判定は必ず緩める向きを走らせる）。
/// 両側に `trim_end()` を挟む実装——末尾の空白を無かったことにする——を当てると、
/// このテストだけが赤くなる。末尾に空行が増えるのは報告を手で開いて保存したときに現に
/// 起きる形で、この検査が捕まえるべきものそのものである。
///
/// 復帰文字を落とすのは保存側の 1 回だけで、それ以外の正規化は 1 つも
/// 挟まない——本文そのものが契約だから。
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
/// 直前の末尾の檻と**対**である。末尾だけを守っていたときは、両側に `trim_start()` を
/// 挟む実装が全テストを素通りした。先頭の空白は末尾ほど起きやすくはないが、対の片側だけを
/// 守ると「どちらを守ったのか」が読む人に伝わらない。
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
/// を挟む実装を当てると、このテストだけが赤くなる。
///
/// 見本の報告が空になっていなければ、この檻は何も守らないまま緑になる。だから空に
/// できたことを先に確かめる。
#[test]
fn a_report_emptied_to_zero_bytes_is_stale() {
    let mut world = World::normal();
    world.report_mut(Domain::Assets).clear();
    assert!(
        world.domain_reports[&Domain::Assets].is_empty(),
        "空にできていなければ試験にならない"
    );

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

// ---------------------------------------------------------------------------
// 全体報告も同じ突き合わせに入る
// ---------------------------------------------------------------------------

/// 全体報告を 1 文字変えると、全体報告の場所の所見が 1 件だけ出る。
///
/// 本文は直し方の副手続き `report-summary` を名指す（ドメイン別報告の `report` では
/// 全体報告は直らない）。
#[test]
fn changing_one_character_of_the_summary_is_one_finding_at_the_summary() {
    let mut world = World::normal();
    let after = world.summary_report.replacen("# ukadoc", "# Ukadoc", 1);
    assert_ne!(world.summary_report, after, "書き換えが効いていない");
    world.summary_report = after;

    let outcome = run(&world.input());
    assert_eq!(
        kinds(&outcome.findings),
        vec![(FindingKind::DomainReportStale, 1)]
    );
    let finding = &of_kind(&outcome.findings, FindingKind::DomainReportStale)[0];
    assert_eq!(finding.id, None);
    assert_eq!(finding.place, SUMMARY_PLACE);
    assert_eq!(finding.detail, SUMMARY_STALE_DETAIL);
}

/// 全体報告が復帰文字付きでも所見は出ず、復帰文字付きで現に違えば出る（対で置く）。
#[test]
fn the_summary_with_carriage_returns_matches_unless_it_really_differs() {
    let mut world = World::normal();
    world.summary_report = world.summary_report.replace('\n', "\r\n");
    assert!(
        world.summary_report.contains("\r\n"),
        "復帰文字が入っていなければ試験にならない"
    );
    assert_eq!(kinds(&run(&world.input()).findings), vec![]);

    world.summary_report.push('x');
    let outcome = run(&world.input());
    assert_eq!(
        places(&outcome.findings, FindingKind::DomainReportStale),
        vec![SUMMARY_PLACE]
    );
}
