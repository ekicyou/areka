//! 調べる副手続きが持つ「判断」と「版面」を釘付けする（設計「入口 / cli」・
//! 「Data Models / 検査の出力」・要件 5.5・5.8・5.9・6.12・8.1）。
//!
//! 4 つの副手続き本体は repo の木（カタログ・台帳・報告）とスナップショットが要るので、
//! ここでは走らせない。このファイルが確かめるのは、入出力に触れずに決まる 6 つである。
//!
//! - 所見の件数から終了の仕方を決める判断（[`verdict`]）
//! - 証拠の並べ方（[`render_evidence_by_id`]・[`render_evidence`]）
//! - 候補の並べ方と、証拠へ混ざらないこと（[`render_candidates`]・要件 5.9）
//! - 2 つのカタログが比べられる形かどうかの見立て（[`comparability_notice`]）と差分の版面
//! - 検査の配線（[`examine`]）——判定を呼び、本文を組み、件数を数える 3 つが噛み合って
//!   いること。`check` から読む段だけを外した形なので、手で組んだ見本の世界で走らせられる
//! - 差分の配線（[`compare`]）——2 つのカタログを渡す向き・注意を捨てずに渡すこと・
//!   台帳を渡すことの 3 つ。`diff` から読む段（スナップショット）だけを外した形なので、
//!   手で組んだカタログの対で走らせられる
//!
//! 期待値の本文は実装の定数を引かず、独立した文字列として書く（実装と同じ値を
//! 参照すると、綴りが一斉に変わっても緑のままになるため）。
//!
//! このファイルはファイルを 1 つも作らず、一時ディレクトリも使わず、スナップショットも
//! 読まない（要件 6.2・設計 File Structure Plan）。

use std::collections::BTreeMap;

use super::{
    comparability_notice, compare, examine, render_candidates, render_check, render_diff,
    render_evidence, render_evidence_by_id, verdict,
};
use crate::catalog::{CATALOG_FORMAT, Catalog, SnapshotMeta};
use crate::check::{Finding, FindingKind};
use crate::diff::CatalogDiff;
use crate::error::SurveyError;
use crate::evidence::candidates::candidates;
use crate::evidence::{
    Candidate, CandidateKind, EvidenceIndex, NameMatchFailure, UnmatchedName, UnresolvedUrl,
};
use crate::lib_test_support;
use crate::model::{Domain, EntryId};

/// 項目 id を作る小道具（見本の綴りはいずれも 2 形のどちらか）。
fn id(raw: &str) -> EntryId {
    EntryId::parse(raw).unwrap_or_else(|_| panic!("見本の id が 2 形でない: {raw}"))
}

/// 証拠の索引を手で組む小道具（id → ファイルパスだけ）。
fn by_id(rows: &[(&str, &[&str])]) -> BTreeMap<EntryId, Vec<String>> {
    rows.iter()
        .map(|(raw, paths)| {
            (
                id(raw),
                paths.iter().map(|path| (*path).to_owned()).collect(),
            )
        })
        .collect()
}

/// カタログ冒頭の情報を手で組む小道具（比べるのは 2 欄だけ）。
fn meta(hash_algorithm: &str, catalog_format: u32) -> SnapshotMeta {
    SnapshotMeta {
        package: "ukagaka-doc-mcp".to_owned(),
        package_version: "0.2.7".to_owned(),
        snapshot_version: 1,
        generated_at: "2026-08-24T04:08:57.881Z".to_owned(),
        total_entries: 2983,
        ukadoc_entries: 1749,
        catalog_format,
        hash_algorithm: hash_algorithm.to_owned(),
    }
}

// --------------------------------------------------------------------------
// 終了の仕方（完了条件「所見が 0 件なら終了コード 0、所見があれば終了コード 1」）
// --------------------------------------------------------------------------

#[test]
fn no_finding_ends_without_an_error() {
    verdict(0).expect("所見が 0 件なのに失敗として返された");
}

#[test]
fn any_finding_ends_with_an_error_that_carries_the_count() {
    // 0 件の側だけを確かめると、何が来ても成功する形に壊れても緑になる。対で置く。
    for count in [1usize, 2, 37] {
        let err = verdict(count).expect_err("所見があるのに成功として返された");
        let body = err.to_string();
        assert!(
            body.contains(&count.to_string()),
            "失敗の本文に所見の件数 {count} が無い: {body}"
        );
    }
}

#[test]
fn the_verdict_error_is_the_one_that_means_a_failed_check() {
    // 件数だけを確かめると、別の誤りの型を返す形に壊れても緑になる。
    let err = verdict(3).expect_err("所見があるのに成功として返された");
    assert!(
        matches!(err, SurveyError::CheckFindings { count: 3 }),
        "整合検査の失敗でない誤りが返った: {err:?}"
    );
}

// --------------------------------------------------------------------------
// 証拠の並べ方（要件 5.5・設計 D-4）
// --------------------------------------------------------------------------

#[test]
fn the_evidence_listing_is_written_out_word_for_word() {
    let listing = by_id(&[
        (
            "ukadoc:list_shiori_event:OnBoot:1",
            &["crates/kanade/src/boot.rs", "crates/shiori/src/lib.rs"],
        ),
        ("ukadoc:manual_shell", &["crates/seriko/src/shell.rs"]),
    ]);
    let expected = "\
証拠のある項目 2 件
  ukadoc:list_shiori_event:OnBoot:1
    crates/kanade/src/boot.rs
    crates/shiori/src/lib.rs
  ukadoc:manual_shell
    crates/seriko/src/shell.rs
";
    assert_eq!(
        render_evidence_by_id(&listing),
        expected,
        "証拠の版面が変わっている"
    );
}

#[test]
fn an_empty_evidence_listing_still_says_how_many_there_were() {
    // 空のときに何も書かないと、読み手は「0 件」と「並べ忘れ」を見分けられない。
    assert_eq!(
        render_evidence_by_id(&BTreeMap::new()),
        "証拠のある項目 0 件\n",
        "証拠が 0 件のときの版面が変わっている"
    );
}

#[test]
fn the_evidence_listing_names_every_file_of_every_id() {
    // 件数だけの主張は「どれ」を 1 つも言わない（1.5・4.2 の教訓）。
    // id ごとに、その id のファイルが全部並ぶことを綴りで確かめる。
    let listing = by_id(&[
        (
            "ukadoc:manual_shell",
            &["a/one.rs", "a/two.rs", "a/three.rs"],
        ),
        ("ukadoc:spec_shiori3", &["b/four.rs"]),
    ]);
    let body = render_evidence_by_id(&listing);
    for needle in [
        "ukadoc:manual_shell",
        "a/one.rs",
        "a/two.rs",
        "a/three.rs",
        "ukadoc:spec_shiori3",
        "b/four.rs",
    ] {
        assert!(
            body.contains(needle),
            "証拠の本文に {needle} が無い: {body}"
        );
    }
}

#[test]
fn the_full_evidence_report_keeps_the_three_lists_apart() {
    // 証拠・解決できなかった URL・対応が付かなかった名前は役目が違う（`EvidenceIndex`
    // の 3 欄）。1 つの塊に混ぜると、読み手は証拠とそうでないものを見分けられない。
    let index = EvidenceIndex {
        by_id: by_id(&[("ukadoc:manual_shell", &["crates/seriko/src/shell.rs"])]),
        unresolved: vec![UnresolvedUrl {
            path: "crates/emo/src/atlas.rs".to_owned(),
            url: "https://ssp.shillest.net/ukadoc/manual/nosuchpage.html".to_owned(),
        }],
        unmatched_names: vec![UnmatchedName {
            path: "crates/sakura/src/table.rs".to_owned(),
            page_url: "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html".to_owned(),
            reason: NameMatchFailure::NoMatch("\\q[".to_owned()),
        }],
    };
    let expected = "\
証拠のある項目 1 件
  ukadoc:manual_shell
    crates/seriko/src/shell.rs

解決できなかった URL 1 件
  crates/emo/src/atlas.rs
    https://ssp.shillest.net/ukadoc/manual/nosuchpage.html

対応が付かなかった名前 1 件
  crates/sakura/src/table.rs
    https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html  同じ見出しが 1 つも無い: \\q[
";
    assert_eq!(
        render_evidence(&index),
        expected,
        "証拠の報告の版面が変わっている"
    );
}

#[test]
fn the_full_evidence_report_spells_out_each_reason_for_an_unmatched_name() {
    // 3 つの理由が同じ綴りになると、読み手は何を直せばよいか分からない。
    let mut reasons: Vec<String> = Vec::new();
    for reason in [
        NameMatchFailure::NoMatch("\\q[".to_owned()),
        NameMatchFailure::Ambiguous("\\q[".to_owned()),
        NameMatchFailure::TableMissing,
    ] {
        let index = EvidenceIndex {
            by_id: BTreeMap::new(),
            unresolved: Vec::new(),
            unmatched_names: vec![UnmatchedName {
                path: "crates/sakura/src/table.rs".to_owned(),
                page_url: "https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html"
                    .to_owned(),
                reason,
            }],
        };
        reasons.push(render_evidence(&index));
    }
    assert_ne!(
        reasons[0], reasons[1],
        "「無い」と「定まらない」が同じ本文になっている"
    );
    assert_ne!(
        reasons[1], reasons[2],
        "「定まらない」と「表が続かない」が同じ本文になっている"
    );
    assert_ne!(
        reasons[0], reasons[2],
        "「無い」と「表が続かない」が同じ本文になっている"
    );
}

// --------------------------------------------------------------------------
// 候補（要件 5.8・5.9）
// --------------------------------------------------------------------------

#[test]
fn the_candidate_listing_is_written_out_word_for_word() {
    let found = vec![
        Candidate {
            path: "crates/emo/src/present.rs".to_owned(),
            kind: CandidateKind::LogLine,
            text: "空 snapshot のため identity 縮退".to_owned(),
        },
        Candidate {
            path: "crates/kanade/src/allow.rs".to_owned(),
            kind: CandidateKind::AllowListElement,
            text: "OnBoot".to_owned(),
        },
        Candidate {
            path: "crates/kanade/src/allow.rs".to_owned(),
            kind: CandidateKind::AllowListElement,
            text: "OnClose".to_owned(),
        },
    ];
    let expected = "\
手掛かりの候補 3 件

[AllowListElement] 2 件
  crates/kanade/src/allow.rs
    OnBoot
    OnClose
[LogLine] 1 件
  crates/emo/src/present.rs
    空 snapshot のため identity 縮退
";
    assert_eq!(
        render_candidates(&found),
        expected,
        "候補の版面が変わっている"
    );
}

#[test]
fn an_empty_candidate_listing_still_says_how_many_there_were() {
    assert_eq!(
        render_candidates(&[]),
        "手掛かりの候補 0 件\n",
        "候補が 0 件のときの版面が変わっている"
    );
}

#[test]
fn every_candidate_kind_gets_its_own_spelling() {
    // 4 種が同じ綴りになると「種類つきで並ぶ」（要件 5.8）が成り立たない。
    let mut bodies: Vec<String> = Vec::new();
    for kind in [
        CandidateKind::AllowListElement,
        CandidateKind::BangCommandConsumer,
        CandidateKind::ConfigKey,
        CandidateKind::LogLine,
    ] {
        bodies.push(render_candidates(&[Candidate {
            path: "crates/a/src/b.rs".to_owned(),
            kind,
            text: "同じ文字列".to_owned(),
        }]));
    }
    for (left, right) in [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)] {
        assert_ne!(
            bodies[left], bodies[right],
            "手掛かりの種類 {left} と {right} が同じ本文になっている"
        );
    }
}

#[test]
fn no_candidate_ever_appears_in_the_evidence_listing() {
    // 要件 5.9。同じソース文から証拠と候補の両方を作り、混ざっていないことを確かめる。
    // 「ukadoc の URL の行」と「手掛かりの行」を別のファイルに置くのは、URL の行が
    // 直下の行を手掛かりから外すため（`candidates::has_marker_above`）。
    let catalog = lib_test_support::catalog();
    let url = catalog
        .entries
        .values()
        .map(|entry| entry.url.clone())
        .next()
        .expect("見本のカタログが空");
    let sources = vec![
        (
            "crates/kanade/src/boot.rs".to_owned(),
            format!("/// ukadoc: {url}\npub fn boot() {{}}\n"),
        ),
        (
            "crates/emo/src/present.rs".to_owned(),
            "fn present() {\n    debug!(\"空 snapshot のため identity 縮退\");\n}\n".to_owned(),
        ),
    ];

    let index = lib_test_support::evidence(&catalog, &sources);
    let found = candidates(&sources);

    // 見本が空回りしていないこと（どちらかが 0 件なら、混ざらないのは当たり前になる）。
    assert_eq!(
        index.by_id.len(),
        1,
        "見本のソース文から証拠が 1 件出ていない"
    );
    assert_eq!(
        found.len(),
        1,
        "見本のソース文から候補が 1 件出ていない: {found:?}"
    );

    let evidence_body = render_evidence(&index);
    let candidate_body = render_candidates(&found);
    assert!(
        !evidence_body.contains("縮退"),
        "候補が証拠の本文へ混ざっている: {evidence_body}"
    );
    assert!(
        !candidate_body.contains(&url),
        "証拠が候補の本文へ混ざっている: {candidate_body}"
    );
}

// --------------------------------------------------------------------------
// 検査の出力（所見の本文＋証拠）
// --------------------------------------------------------------------------

#[test]
fn a_clean_check_still_prints_a_body_and_the_evidence() {
    // 所見 0 件のときに何も書かないと、走ったのか走っていないのかが読めない。
    let listing = by_id(&[("ukadoc:manual_shell", &["crates/seriko/src/shell.rs"])]);
    let index = EvidenceIndex {
        by_id: listing,
        unresolved: Vec::new(),
        unmatched_names: Vec::new(),
    };
    let expected = "\
食い違い 0 件

証拠のある項目 1 件
  ukadoc:manual_shell
    crates/seriko/src/shell.rs
";
    assert_eq!(
        render_check(&[], &index),
        expected,
        "緑のときの版面が変わっている"
    );
}

#[test]
fn a_failed_check_prints_the_findings_and_the_evidence_together() {
    // 完了条件「所見の本文と、id ごとの証拠のファイルパスを並べて出す」。
    let findings = vec![Finding::new(
        FindingKind::ImplementedWithoutEvidence,
        Some(id("ukadoc:list_shiori_event:OnBoot:1")),
        "doc/ukadoc-coverage/ledger/shiori.toml",
        "正典 URL がソースに 1 件も無い",
    )];
    let index = EvidenceIndex {
        by_id: by_id(&[("ukadoc:manual_shell", &["crates/seriko/src/shell.rs"])]),
        unresolved: Vec::new(),
        unmatched_names: Vec::new(),
    };
    let expected = "\
食い違い 1 件

[ImplementedWithoutEvidence] 1 件
  doc/ukadoc-coverage/ledger/shiori.toml
    ukadoc:list_shiori_event:OnBoot:1  正典 URL がソースに 1 件も無い

証拠のある項目 1 件
  ukadoc:manual_shell
    crates/seriko/src/shell.rs
";
    assert_eq!(
        render_check(&findings, &index),
        expected,
        "赤のときの版面が変わっている"
    );
}

// --------------------------------------------------------------------------
// 検査の配線（判定を呼ぶ・本文を組む・件数を数える）
// --------------------------------------------------------------------------

#[test]
fn examining_a_clean_world_counts_nothing_and_still_shows_the_evidence() {
    // `check` の本体は repo の木が要るので `tests/cli_streams.rs` からは走らせられない。
    // 読む段を外した [`examine`] なら、手で組んだ入力で配線ごと確かめられる。
    let world = lib_test_support::World::normal();
    let (body, findings) = examine(&world.input());

    assert_eq!(findings, 0, "正常な見本で所見が出た:\n{body}");
    assert!(
        body.starts_with("食い違い 0 件\n"),
        "緑のときの本文が「食い違い 0 件」で始まっていない:\n{body}"
    );
    assert!(
        body.contains("証拠のある項目"),
        "所見の本文に証拠が並んでいない:\n{body}"
    );
}

#[test]
fn examining_a_broken_world_counts_exactly_what_it_prints() {
    // 件数と本文が別々の数え方をすると、終了コードと本文が食い違う。片方だけを
    // 確かめると、[`examine`] が数を 1 つずらす壊し方が誰にも気づかれずに通る。
    let mut world = lib_test_support::World::normal();
    world
        .report_mut(Domain::Shiori)
        .push_str("台帳から作り直した本文には無い行\n");
    let (body, findings) = examine(&world.input());

    assert_eq!(
        findings, 1,
        "報告を 1 行壊したのに所見が 1 件でない:\n{body}"
    );
    assert!(
        body.starts_with("食い違い 1 件\n"),
        "本文の件数が所見の件数と合っていない:\n{body}"
    );
    assert!(
        body.contains("DomainReportStale"),
        "壊した種類が本文に出ていない:\n{body}"
    );
}

// --------------------------------------------------------------------------
// 差分の配線（比べる向き・注意の受け渡し・台帳の受け渡し）
// --------------------------------------------------------------------------

/// 見本のカタログを 1 つ借りて、id を 1 つ選ぶ。
///
/// 選ぶのは byte 順の先頭。どの id でも成り立つが、固定しておかないと見本を
/// 差し替えたときに何を見ていたか分からなくなる。
fn base_catalog_and_first_id() -> (Catalog, EntryId) {
    let base = lib_test_support::catalog();
    let first = base
        .entries
        .keys()
        .next()
        .expect("見本のカタログが空")
        .clone();
    (base, first)
}

#[test]
fn comparing_two_catalogs_that_differ_by_one_body_names_that_id() {
    // 同じカタログを 2 度渡す壊し方（自分と自分を比べる）は、差分が永久に空になる
    // ——きれいに見えたまま間違う。片方の本文だけを変えた対を渡して、その id が
    // 本文に出ることを版面ごと確かめる。
    let (current, id) = base_catalog_and_first_id();
    let mut next = current.clone();
    next.entries
        .get_mut(&id)
        .expect("選んだ id が見本のカタログに無い")
        .hash = "ffffffffffffffff".to_owned();

    let expected = format!(
        "増えた項目 0 件\n消えた項目 0 件\n本文が変わった項目 1 件\n  {}\n台帳の見直しが要る項目 0 件\n",
        id.as_str()
    );
    assert_eq!(
        compare(&current, &next, &lib_test_support::ledgers()),
        expected,
        "本文が変わった 1 件が差分に出ていない"
    );
}

#[test]
fn an_id_only_in_the_new_catalog_is_an_addition_not_a_removal() {
    // 2 つのカタログを取り違えると増減が裏返る。件数だけを確かめると裏返っても
    // 緑になるので、どちらの見出しの下に出るかを版面ごと固定する。
    let (next, id) = base_catalog_and_first_id();
    let mut current = next.clone();
    current
        .entries
        .remove(&id)
        .expect("選んだ id が見本のカタログに無い");

    let expected = format!(
        "増えた項目 1 件\n  {}\n消えた項目 0 件\n本文が変わった項目 0 件\n台帳の見直しが要る項目 0 件\n",
        id.as_str()
    );
    assert_eq!(
        compare(&current, &next, &lib_test_support::ledgers()),
        expected,
        "新しい方にだけある id が「増えた項目」に出ていない"
    );
}

#[test]
fn a_removed_id_that_a_ledger_still_carries_is_called_out_separately() {
    // 台帳を渡し忘れる壊し方（空の一覧を渡す）は、この主張だけが捕まえる。
    let (current, id) = base_catalog_and_first_id();
    let ledgers = lib_test_support::ledgers();
    assert!(
        ledgers
            .iter()
            .any(|ledger| ledger.entries.contains_key(&id)),
        "見本の台帳に {} が無いので、この主張は空回りする",
        id.as_str()
    );

    let mut next = current.clone();
    next.entries
        .remove(&id)
        .expect("選んだ id が見本のカタログに無い");

    let expected = format!(
        "増えた項目 0 件\n消えた項目 1 件\n  {id}\n本文が変わった項目 0 件\n台帳の見直しが要る項目 1 件\n  {id}\n",
        id = id.as_str()
    );
    assert_eq!(
        compare(&current, &next, &ledgers),
        expected,
        "台帳に残る id が「台帳の見直しが要る項目」に出ていない"
    );
}

#[test]
fn a_catalog_built_with_another_hash_algorithm_gets_its_warning_before_the_lists() {
    // 注意を渡し忘れる壊し方（`None` を渡す）は、差分の一覧が 1 行も変わらないので
    // 版面のテストでは捕まらない。注意が本文に在ること、しかも一覧より先に在ることを
    // ここで確かめる。
    let current = lib_test_support::catalog();
    let mut next = current.clone();
    next.snapshot.hash_algorithm = "xxh3".to_owned();

    let body = compare(&current, &next, &lib_test_support::ledgers());
    for needle in [current.snapshot.hash_algorithm.as_str(), "xxh3"] {
        assert!(
            body.contains(needle),
            "算法 {needle} が差分の本文に無い:\n{body}"
        );
    }
    let notice_at = body.find("注意").unwrap_or_else(|| {
        panic!("注意が本文に無い:\n{body}");
    });
    let first_list_at = body.find("増えた項目").expect("増えた項目の見出しが無い");
    assert!(
        notice_at < first_list_at,
        "注意が一覧より後ろに出ている:\n{body}"
    );
}

#[test]
fn two_catalogs_of_the_same_shape_get_no_warning_in_the_diff_body() {
    // 注意が出る側だけを確かめると、何にでも注意を出す形に壊れても緑になる。
    let current = lib_test_support::catalog();
    let body = compare(&current, &current.clone(), &lib_test_support::ledgers());
    assert!(
        !body.contains("注意"),
        "同じ形なのに注意が出ている:\n{body}"
    );
}

// --------------------------------------------------------------------------
// 差分（要件 8.1）と、比べられる形かどうかの見立て
// --------------------------------------------------------------------------

#[test]
fn two_catalogs_of_the_same_shape_draw_no_warning() {
    // 警告が出る側だけを確かめると、何にでも警告を出す形に壊れても緑になる。
    let same = comparability_notice(
        &meta("fnv1a64", CATALOG_FORMAT),
        &meta("fnv1a64", CATALOG_FORMAT),
    );
    assert_eq!(same, None, "同じ形なのに注意が出た: {same:?}");
}

#[test]
fn a_different_hash_algorithm_draws_a_warning_that_names_both_spellings() {
    let notice = comparability_notice(&meta("fnv1a64", 1), &meta("xxh3", 1))
        .expect("算法が違うのに注意が出ない");
    for needle in ["fnv1a64", "xxh3"] {
        assert!(
            notice.contains(needle),
            "注意の本文に算法 {needle} が無い: {notice}"
        );
    }
}

#[test]
fn a_different_catalog_format_draws_a_warning_that_names_both_numbers() {
    let notice = comparability_notice(&meta("fnv1a64", 1), &meta("fnv1a64", 2))
        .expect("形の版が違うのに注意が出ない");
    for needle in ["1", "2"] {
        assert!(
            notice.contains(needle),
            "注意の本文に形の版 {needle} が無い: {notice}"
        );
    }
}

#[test]
fn the_warning_says_which_of_the_two_disagrees() {
    // 2 つの食い違いが同じ本文になると、どちらを直せばよいか分からない。
    let hash = comparability_notice(&meta("fnv1a64", 1), &meta("xxh3", 1)).expect("算法の注意");
    let format =
        comparability_notice(&meta("fnv1a64", 1), &meta("fnv1a64", 2)).expect("形の版の注意");
    assert_ne!(
        hash, format,
        "算法の食い違いと形の版の食い違いが同じ本文になっている"
    );
}

#[test]
fn the_diff_listing_is_written_out_word_for_word() {
    let delta = CatalogDiff {
        added: vec![id("ukadoc:list_shiori_event:OnNewEvent:1")],
        removed: vec![id("ukadoc:manual_shell"), id("ukadoc:spec_shiori3")],
        changed: vec![id("ukadoc:descript_ghost:charset:1")],
        removed_in_ledger: vec![id("ukadoc:spec_shiori3")],
    };
    let expected = "\
増えた項目 1 件
  ukadoc:list_shiori_event:OnNewEvent:1
消えた項目 2 件
  ukadoc:manual_shell
  ukadoc:spec_shiori3
本文が変わった項目 1 件
  ukadoc:descript_ghost:charset:1
台帳の見直しが要る項目 1 件
  ukadoc:spec_shiori3
";
    assert_eq!(
        render_diff(&delta, None),
        expected,
        "差分の版面が変わっている"
    );
}

#[test]
fn an_empty_diff_still_names_all_four_lists() {
    // 0 件の塊を落とすと、読み手は「無かった」と「見ていない」を見分けられない。
    let expected = "\
増えた項目 0 件
消えた項目 0 件
本文が変わった項目 0 件
台帳の見直しが要る項目 0 件
";
    assert_eq!(
        render_diff(&CatalogDiff::default(), None),
        expected,
        "差分が空のときの版面が変わっている"
    );
}

#[test]
fn the_warning_comes_before_the_four_lists() {
    // 注意を後ろに置くと、読み手は「本文が変わった項目」を全部読んでから、それが
    // 読む価値の無い一覧だったと知ることになる。
    let body = render_diff(&CatalogDiff::default(), Some("注意: 比べられない"));
    let notice_at = body.find("注意: 比べられない").expect("注意が本文に無い");
    let first_list_at = body.find("増えた項目").expect("増えた項目の見出しが無い");
    assert!(
        notice_at < first_list_at,
        "注意が一覧より後ろに出ている: {body}"
    );
}
