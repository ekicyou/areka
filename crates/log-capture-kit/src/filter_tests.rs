//! `capture_under_filter` の自己テスト（feature `env-filter` のときだけコンパイルされる）。
//!
//! 本 API の契約は `wintf` の 96 呼出がそのまま回帰スイートになる形で決まっている
//! （設計 `## Components and Interfaces` → `#### C1 log-capture-kit`）。よってここで固定
//! するのは、その 96 呼出が依存している 3 点である。
//!
//! ⒜ **directive が実際に効く**（部分一致の模倣ではなく [`tracing_subscriber::EnvFilter`]
//!   による実濾過）。⒝ **番兵行が戻り値に現れない**（呼出側の主張が番兵の分だけ変わらない）。
//!   ⒞ **行の形**（ANSI 無し・レベル・宛先・本文・フィールド）。
//!
//! 「出ない」ことの主張は、同じ窓で必ず「出る」側の対照を隣に置く（不在だけの主張は
//! 捕捉が死んでいても緑になるため）。発行点の interest はプロセス大域に焼き付くので、
//! テストごとに専用の宛先を使う（`capture_tests.rs` と同じ規律）。

use super::*;

// ---- テスト専用の発行点（1 テスト 1 宛先） --------------------------------

const TARGET_PASS: &str = "log_capture_kit::selftest::filter_pass";
const TARGET_DROP: &str = "log_capture_kit::selftest::filter_drop";
const TARGET_EMPTY: &str = "log_capture_kit::selftest::filter_empty_directives";
const TARGET_COMMA: &str = "log_capture_kit::selftest::filter_trailing_comma";
const TARGET_SHAPE: &str = "log_capture_kit::selftest::filter_line_shape";
const TARGET_OFF: &str = "log_capture_kit::selftest::filter_everything_off";
const TARGET_BARE: &str = "log_capture_kit::selftest::filter_bare_level";

// ---- ⒜ directive が実際に効く --------------------------------------------

/// 要件 1.2: 宛先ごとの水準指定が効き、指定の無い宛先は既定水準で切られる。
///
/// 対照（`info` は通る）を同じ窓に置いてあるので、捕捉そのものが死んで全部空になった
/// 場合はこのテストが赤になる＝不在の主張が空振りしない。
#[test]
fn per_target_directives_decide_which_events_reach_the_output() {
    let directives = format!("info,{TARGET_PASS}=debug");
    let out = capture_under_filter(&directives, || {
        tracing::debug!(target: TARGET_PASS, "通る側の debug 本文");
        tracing::debug!(target: TARGET_DROP, "切られる側の debug 本文");
        tracing::info!(target: TARGET_DROP, "切られない側の info 本文");
    });

    assert!(
        out.contains("通る側の debug 本文"),
        "宛先指定 `{TARGET_PASS}=debug` を通るはずの行が出ていない: {out}"
    );
    assert!(
        out.contains("切られない側の info 本文"),
        "既定水準 `info` の行が出ていない（捕捉が死んでいる）: {out}"
    );
    assert!(
        !out.contains("切られる側の debug 本文"),
        "指定の無い宛先の debug が濾過を素通りしている: {out}"
    );
}

/// 要件 1.2: 宛先を持たない裸の水準指定は全宛先の既定として効く。
#[test]
fn a_bare_level_directive_applies_to_every_target() {
    let out = capture_under_filter("debug", || {
        tracing::debug!(target: TARGET_BARE, "裸の水準で通る本文");
        tracing::trace!(target: TARGET_BARE, "裸の水準では切られる本文");
    });

    assert!(
        out.contains("裸の水準で通る本文"),
        "裸の `debug` 指定が全宛先へ効いていない: {out}"
    );
    assert!(
        !out.contains("裸の水準では切られる本文"),
        "`debug` 指定なのに trace が素通りしている: {out}"
    );
}

/// 要件 1.2: **空の directive** は `EnvFilter` の既定（ERROR 水準）のまま振る舞う。
///
/// 番兵の directive を文字列連結で足すと、空文字列のときだけ「解釈できた指令が
/// 番兵 1 件」になって既定 ERROR が付かなくなり、呼出側の `error!` が黙って消える。
/// ここはその取り違えを捕まえるための檻である。
#[test]
fn empty_directives_still_carry_the_env_filter_default_of_error() {
    let out = capture_under_filter("", || {
        tracing::error!(target: TARGET_EMPTY, "空指定でも残る error 本文");
        tracing::warn!(target: TARGET_EMPTY, "空指定では切られる warn 本文");
    });

    assert!(
        out.contains("空指定でも残る error 本文"),
        "空 directive の既定（ERROR）が失われている: {out}"
    );
    assert!(
        !out.contains("空指定では切られる warn 本文"),
        "空 directive なのに WARN が通っている: {out}"
    );
}

/// 要件 1.2: 末尾のコンマ（空の指令片）があっても結果が変わらない。
#[test]
fn a_trailing_comma_in_the_directives_changes_nothing() {
    let plain = format!("info,{TARGET_COMMA}=debug");
    let trailing = format!("{plain},");

    let emit = || {
        tracing::debug!(target: TARGET_COMMA, "コンマ検査の debug 本文");
        tracing::trace!(target: TARGET_COMMA, "コンマ検査の trace 本文");
    };

    let without = capture_under_filter(&plain, emit);
    let with = capture_under_filter(&trailing, emit);

    for (label, out) in [("末尾コンマ無し", &without), ("末尾コンマ有り", &with)] {
        assert!(
            out.contains("コンマ検査の debug 本文"),
            "{label}の側で debug 行が出ていない: {out}"
        );
        assert!(
            !out.contains("コンマ検査の trace 本文"),
            "{label}の側で trace が素通りしている: {out}"
        );
    }
}

// ---- ⒝ 番兵は内部で通し、戻り値からは消える ------------------------------

/// 要件 1.2: 呼出側の directive が**何も通さない**場合でも番兵検査は成立し、
/// 戻り値は 1 バイトも無い（番兵行が漏れない）。
///
/// 番兵の directive を内部で足していなければ番兵が濾過で消えて panic するので、
/// このテストは「番兵を通す」と「番兵を消す」の両方を同時に押さえている。
#[test]
fn a_directive_that_passes_nothing_yields_an_empty_string_not_a_sentinel_line() {
    let out = capture_under_filter("off", || {
        tracing::error!(target: TARGET_OFF, "全遮断の窓では出ない本文");
    });

    assert_eq!(
        out, "",
        "全遮断の窓の戻り値に何かが残っている（番兵行の漏れ）: {out}"
    );
}

/// 要件 1.2: 通常の窓でも戻り値に番兵の宛先が現れない（対照は同じ窓の実データ）。
#[test]
fn the_returned_string_never_mentions_the_sentinel_target() {
    let out = capture_under_filter("info", || {
        tracing::info!(target: TARGET_SHAPE, "番兵検査の対照本文");
    });

    assert!(
        out.contains("番兵検査の対照本文"),
        "対照の行が出ていない（不在の主張が空振りする）: {out}"
    );
    assert!(
        !out.contains(SENTINEL_TARGET),
        "戻り値に番兵行が残っている: {out}"
    );
}

// ---- 番兵行の除去そのもの（純関数・較正つき） ----------------------------

/// 番兵行だけを、前後の行を 1 バイトも変えずに取り除く。
#[test]
fn stripping_removes_only_the_sentinel_line_and_keeps_the_rest_verbatim() {
    let input =
        format!("先頭の行\n2026-08-23T00:00:00.000000Z TRACE {SENTINEL_TARGET}: live\n末尾の行\n");

    assert_eq!(strip_sentinel_lines(&input), "先頭の行\n末尾の行\n");
}

/// 末尾に改行が無い行も落とさない（`split_inclusive` の空要素で末尾が消えないこと）。
#[test]
fn stripping_keeps_a_last_line_without_a_trailing_newline() {
    let input = format!("TRACE {SENTINEL_TARGET}: live\n改行で終わらない行");

    assert_eq!(strip_sentinel_lines(&input), "改行で終わらない行");
}

/// 較正: 番兵行が無い入力は**赤にできる**。
///
/// この検査が赤にできないなら、番兵は何も証明していない（`capture_tests.rs` の
/// 番兵較正と同じ役割）。
#[test]
#[should_panic(expected = "対照イベント")]
fn stripping_panics_when_no_sentinel_line_is_present() {
    let _ = strip_sentinel_lines("先頭の行\n末尾の行\n");
}

// ---- ⒞ 行の形（96 呼出が読んでいる文字列そのもの） -----------------------

/// 要件 1.2: 1 イベント 1 行・ANSI 無し・レベル／宛先／本文／フィールドが載る。
///
/// `wintf` の 96 呼出は `contains("stage=begin")` のようにこの形の内側を読む。
/// 整形器の設定を変えるとここが赤になる。
#[test]
fn one_event_becomes_one_plain_line_carrying_level_target_message_and_fields() {
    let out = capture_under_filter("info", || {
        tracing::info!(target: TARGET_SHAPE, alpha = 7, beta = "b", "行の形の本文");
    });

    assert_eq!(
        out.lines().count(),
        1,
        "1 イベントが 1 行になっていない: {out:?}"
    );
    assert!(out.ends_with('\n'), "行が改行で終わっていない: {out:?}");
    assert!(
        !out.contains('\u{1b}'),
        "ANSI 制御列が混ざっている（`with_ansi(false)` が外れた）: {out:?}"
    );
    assert!(out.contains("INFO"), "レベルが載っていない: {out}");
    assert!(out.contains(TARGET_SHAPE), "宛先が載っていない: {out}");
    assert!(out.contains("行の形の本文"), "本文が載っていない: {out}");
    assert!(
        out.contains("alpha=7"),
        "数値フィールドが載っていない: {out}"
    );
    assert!(
        out.contains("beta=\"b\""),
        "文字列フィールドが載っていない: {out}"
    );
}
