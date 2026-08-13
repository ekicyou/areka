//! `zorder_pair` の診断記録の語彙に対する決定論的テスト（実 HWND・実機不要）。
//!
//! 本ファイルが固定するのは 3 つである。
//!
//! - **行の組立**——各記録が design.md「診断ログ語彙（要件 6）」のレコード表どおりの
//!   名前と必須フィールドを持ち、値が復元できる形で 1 行に載ること。
//! - **水準と出力先**——失敗と検証不一致が error 水準であること（要件 6.2）、
//!   調整・見送り・沈降観測が診断手順の水準（debug）で点灯すること（要件 6.1／6.3）、
//!   そしてすべてが module path 既定の出力先（サインオフの grep 対象）へ出ること。
//! - **記録の入口の振る舞い**——判断結果を記録の入口（`record_decision`）へ渡すと、
//!   見送りは必ず理由つきの記録になったうえで適用対象としては返らず、是正の腕は
//!   記録を足さずにそのまま返ること（要件 6.3）。
//!
//! なお「記録の無い見送りが起きない」は**規約**であって構造保証ではない——
//! `decide_pair_fix` の結果は記録の入口を通さずにも握り潰せる（理由と、型で塞ぐ案を
//! 採らない裁定は `record_decision` の doc に書いてある）。本ファイルが固定できるのは
//! 「入口を通したときの振る舞い」までであり、入口を通すこと自体は維持系の実装
//! （タスク 2.2）とそのレビューが守る。
//!
//! # 記録捕捉のテストで「出ないこと」を主張するときの作法
//!
//! 「この水準では出ない」の主張は、捕捉そのものが死んでいても成立してしまう。
//! よって出ないことを見る各テストには、**同じ捕捉窓の中で確かに拾える記録**を
//! 併置してある（捕捉が生きている証拠を毎回同じ出力から取る）。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{E_INVALIDARG, HWND};

use super::{
    ExpectedOrder, InsertSpec, PairFixDecision, SkipReason, fix_line, log_owner_established,
    log_sink_observed, owner_establish_failed_line, owner_established_line, record_decision,
    record_verification, skip_line, verify_failed_line,
};
use crate::ecs::test_support::capture_under_filter;

/// 実機サインオフが用いる `RUST_LOG` 相当（design.md「Real-Machine Signoff」）。
/// areka 側の宣言レコードは別クレートゆえ、本ファイルでは wintf 側の指定だけを使う。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_pair=debug";

/// 既定水準（診断手順を有効化していない通常運転）。
const DEFAULT_DIRECTIVES: &str = "info";

/// 記録の出力先。design.md は「ログ target は module path 既定」と定めており、
/// サインオフの grep はこの名前で行う。
const LOG_TARGET: &str = "wintf::ecs::window::zorder_pair";

/// テスト用の偽 HWND（Win32 へは渡さない・値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
}

fn balloon() -> HWND {
    fake_hwnd(0x00B0)
}
fn character() -> HWND {
    fake_hwnd(0x00C0)
}
fn intruder_front() -> HWND {
    fake_hwnd(0x00F1)
}

/// 記録の結合キーに使う 2 つの entity（バルーン窓・キャラ窓の順）。
fn pair_entities() -> (Entity, Entity) {
    let mut world = World::new();
    let char_window = world.spawn_empty().id();
    let balloon_window = world.spawn_empty().id();
    (balloon_window, char_window)
}

/// 期待隣接（バルーンがキャラのすぐ手前）。
fn expected_order() -> ExpectedOrder {
    ExpectedOrder {
        above: balloon(),
        below: character(),
    }
}

/// 適用に失敗した Win32 呼出を模した誤り値。
fn sample_error() -> windows::core::Error {
    windows::core::Error::from(E_INVALIDARG)
}

/// 6 種の記録をすべて 1 回ずつ出す（水準・出力先の確認用）。
///
/// `declared` レコードは含まれない——scope を必須フィールドに持ち、scope を知る層は
/// areka だけであるため、出力点は areka 側の窓生成である（module doc 参照）。
fn emit_every_record() {
    let (balloon_entity, char_entity) = pair_entities();

    log_owner_established(
        balloon_entity,
        char_entity,
        balloon(),
        character(),
        Some(balloon()),
    );
    super::log_owner_establish_failed(balloon_entity, &sample_error());
    record_decision(
        balloon_entity,
        char_entity,
        PairFixDecision::Skip(SkipReason::PeerMissing),
    );
    record_verification(
        balloon_entity,
        char_entity,
        InsertSpec::After(intruder_front()),
        expected_order(),
        Some(character()),
    );
    record_verification(
        balloon_entity,
        char_entity,
        InsertSpec::After(intruder_front()),
        expected_order(),
        Some(intruder_front()),
    );
    log_sink_observed(balloon_entity, true, Some(intruder_front()), Some(true));
}

/// 捕捉した出力から、指定タグを含む行をすべて取り出す。
fn lines_with<'a>(out: &'a str, tag: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(tag)).collect()
}

/// 捕捉した出力から、指定タグを含む行をちょうど 1 本取り出す。
fn only_line_with<'a>(out: &'a str, tag: &str) -> &'a str {
    let found = lines_with(out, tag);
    assert_eq!(found.len(), 1, "`{tag}` の行がちょうど 1 本ではない: {out}");
    found[0]
}

// -------------------------------------------------------------------------
// 行の組立（純関数・捕捉を介さない）
// -------------------------------------------------------------------------

/// owner 確立の記録は、張った両ハンドルと確立直後の実測を同じ行へ載せる。
#[test]
fn owner_established_line_carries_both_handles_and_the_measurement() {
    let (balloon_entity, char_entity) = pair_entities();
    let line = owner_established_line(
        balloon_entity,
        char_entity,
        balloon(),
        character(),
        Some(balloon()),
    );

    assert!(line.contains("[zorder-pair] owner-established"), "{line}");
    assert!(
        line.contains(&format!("entity={balloon_entity:?}")),
        "{line}"
    );
    assert!(line.contains(&format!("peer={char_entity:?}")), "{line}");
    assert!(line.contains("owned_hwnd=0xB0"), "{line}");
    assert!(line.contains("owner_hwnd=0xC0"), "{line}");
    assert!(
        line.contains("measured_prev=0xB0"),
        "確立直後の実測が復元できない: {line}"
    );

    // 対照: 実測が取れなかった場合もフィールドは落ちず番兵になる
    // （フィールドごと消えると「出ていない」と「その経路には無い」が区別できなくなる）。
    let unmeasured =
        owner_established_line(balloon_entity, char_entity, balloon(), character(), None);
    assert!(unmeasured.contains("measured_prev=-"), "{unmeasured}");
}

/// 是正の記録は、出した指令とその後の実測を**同じ 1 行**に載せる（要件 6.1）。
///
/// 「指令行」と「実測行」に分けないのは design.md「Implementation Notes > Validation」の
/// 裁定である——2 行に分かれると「指令は出たが効かなかった」を突合で復元する羽目になる。
#[test]
fn fix_line_carries_the_command_and_the_measurement_on_one_line() {
    let (balloon_entity, char_entity) = pair_entities();
    let line = fix_line(
        balloon_entity,
        char_entity,
        InsertSpec::After(intruder_front()),
        Some(character()),
    );

    assert!(!line.contains('\n'), "記録は 1 行であること: {line}");
    assert!(line.contains("[zorder-pair] fix"), "{line}");
    assert!(
        line.contains(&format!("entity={balloon_entity:?}")),
        "{line}"
    );
    assert!(line.contains(&format!("peer={char_entity:?}")), "{line}");
    assert!(
        line.contains("insert_after=0xF1"),
        "出した指令（挿入位置）が復元できない: {line}"
    );
    assert!(
        line.contains("measured_next_after_fix=0xC0"),
        "適用後の実測が同じ行から復元できない: {line}"
    );

    // 対照: 最上位の縁は専用の語になり、ハンドル表現と混ざらない
    let top_edge = fix_line(balloon_entity, char_entity, InsertSpec::TopEdge, None);
    assert!(top_edge.contains("insert_after=top-edge"), "{top_edge}");
    assert!(top_edge.contains("measured_next_after_fix=-"), "{top_edge}");
    assert!(
        !top_edge.contains("insert_after=0x"),
        "縁がハンドル表現へ潰れている: {top_edge}"
    );

    // 対照: 「通常帯の最上」も専用の語で、縁とも混ざらない。指令としては同じ `HWND_TOP`
    // だが理由が違う——1 語へ潰れると、記録を読む者が是正の原因を辿れなくなる。
    let normal_band = fix_line(
        balloon_entity,
        char_entity,
        InsertSpec::TopOfNormalBand,
        Some(character()),
    );
    assert!(
        normal_band.contains("insert_after=top-of-normal-band"),
        "{normal_band}"
    );
    assert!(
        !normal_band.contains("insert_after=top-edge"),
        "「通常帯の最上」が縁の語へ潰れている: {normal_band}"
    );
    assert!(
        !normal_band.contains("insert_after=0x"),
        "「通常帯の最上」がハンドル表現へ潰れている: {normal_band}"
    );
}

/// 見送りの記録は必ず理由を名乗り、5 種の理由が互いに区別できる（要件 6.3）。
#[test]
fn skip_line_names_a_distinct_reason_for_every_skip() {
    let (balloon_entity, char_entity) = pair_entities();
    let all = [
        (SkipReason::AlreadyAdjacent, "AlreadyAdjacent"),
        (SkipReason::PeerMissing, "PeerMissing"),
        (SkipReason::HandleMissing, "HandleMissing"),
        (SkipReason::EchoOrIrrelevant, "EchoOrIrrelevant"),
        (SkipReason::StrategyDisabled, "StrategyDisabled"),
    ];

    let mut rendered: Vec<String> = Vec::new();
    for (reason, word) in all {
        let line = skip_line(balloon_entity, char_entity, reason);
        assert!(line.contains("[zorder-pair] skip"), "{line}");
        assert!(
            line.contains(&format!("entity={balloon_entity:?}")),
            "{line}"
        );
        assert!(
            line.contains(&format!("reason={word}")),
            "{reason:?} の理由語が載っていない: {line}"
        );
        rendered.push(line);
    }

    // 対照: 5 行がすべて相異なる（理由が 1 語へ潰れていれば重複が出る）
    let mut sorted = rendered.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        rendered.len(),
        "見送り理由の表示が重複している（区別が付かない）: {rendered:?}"
    );
}

/// 検証不一致の記録は、期待した隣接と実測を同じ行へ載せる（要件 6.2）。
#[test]
fn verify_failed_line_carries_expected_adjacency_and_measurement() {
    let (balloon_entity, char_entity) = pair_entities();
    let line = verify_failed_line(
        balloon_entity,
        char_entity,
        expected_order(),
        Some(intruder_front()),
    );

    assert!(line.contains("[zorder-pair] verify-failed"), "{line}");
    assert!(
        line.contains(&format!("entity={balloon_entity:?}")),
        "{line}"
    );
    assert!(
        line.contains("expected_above=0xB0") && line.contains("expected_below=0xC0"),
        "期待した隣接の 2 窓が復元できない: {line}"
    );
    assert!(line.contains("measured=0xF1"), "実測が復元できない: {line}");

    // 対照: 実測が「背後に誰も居ない」だった場合も番兵で残る
    let none_measured = verify_failed_line(balloon_entity, char_entity, expected_order(), None);
    assert!(none_measured.contains("measured=-"), "{none_measured}");
}

/// owner 確立失敗の記録は失敗の内容を載せる（要件 6.2・ゲート FAIL の材料）。
#[test]
fn owner_establish_failed_line_carries_the_error() {
    let (balloon_entity, _) = pair_entities();
    let line = owner_establish_failed_line(balloon_entity, &sample_error());

    assert!(
        line.contains("[zorder-pair] owner-establish-failed"),
        "{line}"
    );
    assert!(
        line.contains(&format!("entity={balloon_entity:?}")),
        "{line}"
    );
    assert!(line.contains("error="), "失敗の内容が載っていない: {line}");
    assert!(
        line.contains("0x80070057"),
        "失敗を特定できる符号が載っていない: {line}"
    );
}

/// 沈降観測の記録は、隣接の維持と前面窓との相対を載せる（要件 4.4／7.5）。
#[test]
fn sink_observed_line_carries_adjacency_and_foreground_relation() {
    let (balloon_entity, _) = pair_entities();
    let line = super::sink_observed_line(balloon_entity, true, Some(intruder_front()), Some(true));

    assert!(line.contains("[zorder-pair] sink-observed"), "{line}");
    assert!(
        line.contains(&format!("entity={balloon_entity:?}")),
        "{line}"
    );
    assert!(line.contains("adjacency_ok=true"), "{line}");
    assert!(line.contains("foreground=0xF1"), "{line}");
    assert!(line.contains("behind_foreground=true"), "{line}");

    // 対照: 判定が偽・前面窓が取れなかった場合も同じフィールドで出る
    // （成立時だけ出る記録では「沈まなかった」を観測できない）
    let negative = super::sink_observed_line(balloon_entity, false, None, Some(false));
    assert!(negative.contains("adjacency_ok=false"), "{negative}");
    assert!(negative.contains("foreground=-"), "{negative}");
    assert!(negative.contains("behind_foreground=false"), "{negative}");

    // 前面窓との相対そのものが取れなかった場合は番兵（偽と混同しない）
    let unknown = super::sink_observed_line(balloon_entity, true, None, None);
    assert!(unknown.contains("behind_foreground=-"), "{unknown}");
}

// -------------------------------------------------------------------------
// 水準と出力先（実際の濾過を通した捕捉）
// -------------------------------------------------------------------------

/// サインオフの `RUST_LOG` では 6 種すべてが点灯し、grep 対象の出力先へ出る。
#[test]
fn every_record_is_visible_under_the_signoff_directive() {
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, emit_every_record);

    for tag in [
        "[zorder-pair] owner-established",
        "[zorder-pair] owner-establish-failed",
        "[zorder-pair] fix",
        "[zorder-pair] skip",
        "[zorder-pair] verify-failed",
        "[zorder-pair] sink-observed",
    ] {
        assert!(
            out.contains(tag),
            "サインオフの水準で `{tag}` が観測できない: {out}"
        );
    }
    assert!(
        out.contains(LOG_TARGET),
        "grep 対象の出力先が module path 既定でない: {out}"
    );
}

/// 既定水準では調整・見送り・沈降観測は無音で、失敗と検証不一致だけが残る。
///
/// これは同時に**捕捉が生きている証拠**でもある——同じ捕捉窓から error／info の記録が
/// 実際に取れているため、「出ない」の主張が捕捉の死によって恒真になっていない。
#[test]
fn diagnostic_records_are_silent_at_default_level_while_failures_still_speak() {
    let out = capture_under_filter(DEFAULT_DIRECTIVES, emit_every_record);

    for tag in [
        "[zorder-pair] fix",
        "[zorder-pair] skip",
        "[zorder-pair] sink-observed",
    ] {
        assert!(
            !out.contains(tag),
            "診断専用の `{tag}` が既定水準へ漏れている: {out}"
        );
    }
    for tag in [
        "[zorder-pair] owner-established",
        "[zorder-pair] owner-establish-failed",
        "[zorder-pair] verify-failed",
    ] {
        assert!(
            out.contains(tag),
            "既定水準で残るべき `{tag}` が出ていない（捕捉が死んでいる疑い）: {out}"
        );
    }
}

/// 失敗と検証不一致は error 水準（要件 6.2）。対照として他の記録の水準も同時に固定する。
#[test]
fn failures_and_verification_mismatch_are_recorded_at_error_level() {
    let out = capture_under_filter(SIGNOFF_DIRECTIVES, emit_every_record);

    for tag in [
        "[zorder-pair] owner-establish-failed",
        "[zorder-pair] verify-failed",
    ] {
        let line = only_line_with(&out, tag);
        assert!(
            line.contains("ERROR"),
            "`{tag}` が error 水準でない: {line}"
        );
    }

    // 対照: 確立成功は info、是正・見送り・沈降観測は debug
    // （すべてが error になっていれば上の主張は無意味になる）
    let established = only_line_with(&out, "[zorder-pair] owner-established");
    assert!(
        established.contains("INFO"),
        "確立の記録は info 水準: {established}"
    );
    for tag in [
        "[zorder-pair] fix",
        "[zorder-pair] skip",
        "[zorder-pair] sink-observed",
    ] {
        let line = only_line_with(&out, tag);
        assert!(
            line.contains("DEBUG"),
            "`{tag}` は診断水準（debug）で出す: {line}"
        );
    }
}

// -------------------------------------------------------------------------
// 記録の入口（`record_decision`）の振る舞い（要件 6.3）
// -------------------------------------------------------------------------

/// 判断の記録の入口は、見送りを必ず理由つきの記録にしたうえで**適用対象を返さない**。
#[test]
fn record_decision_reports_every_skip_reason_and_yields_no_fix() {
    let (balloon_entity, char_entity) = pair_entities();

    for reason in [
        SkipReason::AlreadyAdjacent,
        SkipReason::PeerMissing,
        SkipReason::HandleMissing,
        SkipReason::EchoOrIrrelevant,
        SkipReason::StrategyDisabled,
    ] {
        let mut taken = Some(PairFixDecision::Skip(reason));
        let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
            taken = record_decision(balloon_entity, char_entity, PairFixDecision::Skip(reason));
        });

        assert_eq!(taken, None, "{reason:?}: 見送りは適用対象として返らない");
        let line = only_line_with(&out, "[zorder-pair] skip");
        assert!(
            line.contains(&format!("reason={reason:?}")),
            "{reason:?}: 理由の無い見送りになっている: {line}"
        );
    }
}

/// 是正の腕は記録の入口をそのまま通り抜け、見送りの記録は出ない（対照）。
///
/// 適用の記録（`fix`）はこの時点では出ない——出した指令の実測がまだ無く、
/// 指令と実測を同じ行へ載せられないためである（次巡の検証で 1 行にまとめて出す）。
///
/// # 「出ない」を空虚にしないための目印
///
/// 是正の腕だけを捕捉窓へ通すと、その窓に記録は 1 件も現れない——つまり
/// 「見送りが記録されていない」という主張は、記録の出力そのものが壊れていても
/// 成立してしまう。そこで**同じ捕捉窓の先頭で確実に出る見送りを 1 件通し**、
/// その 1 行の実在を確かめたうえで、是正の腕を通しても記録が増えないことを見る。
/// 目印が消えていれば捕捉か出力が死んでいるのであり、テストはそこで落ちる。
#[test]
fn record_decision_passes_fix_arms_through_without_a_skip_record() {
    let (balloon_entity, char_entity) = pair_entities();

    for decision in [
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::After(intruder_front()),
        },
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::TopEdge,
        },
        PairFixDecision::PlaceBelowUnderAbove {
            insert_after: balloon(),
        },
    ] {
        let mut marker = None;
        let mut taken = None;
        let out = capture_under_filter(SIGNOFF_DIRECTIVES, || {
            // ① 目印——確実に記録される見送りを 1 件通す（捕捉が生きている証拠）
            marker = record_decision(
                balloon_entity,
                char_entity,
                PairFixDecision::Skip(SkipReason::PeerMissing),
            );
            // ② 本題——是正の腕を同じ捕捉窓で通す
            taken = record_decision(balloon_entity, char_entity, decision);
        });

        assert_eq!(marker, None, "目印の見送りは適用対象として返らない");
        assert_eq!(taken, Some(decision), "{decision:?}: 是正はそのまま返る");

        // 目印は確かに拾えている——この 1 行が無ければ以下の主張は空虚になる
        let skips = lines_with(&out, "[zorder-pair] skip");
        assert_eq!(
            skips.len(),
            1,
            "{decision:?}: 見送りの記録が目印の 1 件のままでない（捕捉が死んでいるか、\
             是正が見送りとして記録されている）: {out}"
        );
        assert!(
            skips[0].contains("reason=PeerMissing"),
            "{decision:?}: 残った 1 行が目印ではない＝是正が見送りとして記録された: {}",
            skips[0]
        );
        // 是正の腕はこの時点で何の記録も足さない（記録は次巡の検証で 1 行にまとめて出る）
        assert_eq!(
            lines_with(&out, "[zorder-pair]").len(),
            1,
            "{decision:?}: 是正の腕が判断時点で記録を出している: {out}"
        );
    }
}

/// 適用後の照合は、一致なら是正の記録・不一致なら error 記録を出す（どちらか一方だけ）。
#[test]
fn record_verification_emits_fix_on_match_and_error_on_mismatch() {
    let (balloon_entity, char_entity) = pair_entities();
    let insert_after = InsertSpec::After(intruder_front());

    // 一致——バルーンの最も近い可視の背後が期待どおりキャラだった
    let mut matched = false;
    let ok = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        matched = record_verification(
            balloon_entity,
            char_entity,
            insert_after,
            expected_order(),
            Some(character()),
        );
    });
    assert!(matched, "実測が期待と一致した場合は真を返す");
    let fix = only_line_with(&ok, "[zorder-pair] fix");
    assert!(
        fix.contains("insert_after=0xF1") && fix.contains("measured_next_after_fix=0xC0"),
        "指令と実測が同じ行に載っていない: {fix}"
    );
    assert!(
        !ok.contains("[zorder-pair] verify-failed"),
        "一致なのに不一致の記録が出ている: {ok}"
    );

    // 不一致——指令は出したが、実測では別の窓が割り込んだままだった
    let mut mismatched = true;
    let ng = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        mismatched = record_verification(
            balloon_entity,
            char_entity,
            insert_after,
            expected_order(),
            Some(intruder_front()),
        );
    });
    assert!(!mismatched, "実測が期待と食い違えば偽を返す");
    let failed = only_line_with(&ng, "[zorder-pair] verify-failed");
    assert!(
        failed.contains("expected_below=0xC0") && failed.contains("measured=0xF1"),
        "期待と実測の差が同じ行から読めない: {failed}"
    );
    assert!(
        !ng.contains("[zorder-pair] fix"),
        "不一致なのに是正の記録が出ている: {ng}"
    );

    // 実測そのものが取れなかった場合も不一致として error 記録に落ちる（黙って成功にしない）
    let mut unmeasured = true;
    let none = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        unmeasured = record_verification(
            balloon_entity,
            char_entity,
            insert_after,
            expected_order(),
            None,
        );
    });
    assert!(!unmeasured, "実測が取れなければ一致とは言えない");
    assert!(none.contains("[zorder-pair] verify-failed"), "{none}");
}

/// 判断結果から挿入位置だけを取り出す射影は、見送りに対しては何も返さない。
///
/// 検証の記録（`fix`）は「出した指令」を必要とするため、見送りが挿入位置を持ち得ない
/// ことがそのまま「見送りは是正として記録されない」を担保する。
#[test]
fn insert_spec_projection_is_absent_for_skips_and_present_for_fixes() {
    assert_eq!(
        PairFixDecision::Skip(SkipReason::AlreadyAdjacent).insert_spec(),
        None
    );
    assert_eq!(
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::TopEdge
        }
        .insert_spec(),
        Some(InsertSpec::TopEdge)
    );
    assert_eq!(
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::After(intruder_front())
        }
        .insert_spec(),
        Some(InsertSpec::After(intruder_front()))
    );
    assert_eq!(
        PairFixDecision::PlaceBelowUnderAbove {
            insert_after: balloon()
        }
        .insert_spec(),
        Some(InsertSpec::After(balloon())),
        "キャラを動かす腕の挿入位置はバルーン自身（縁ではない）"
    );
}
