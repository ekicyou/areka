//! `windowposition` 語彙（`limit`／`x` キーワード）の**取得経路への結線**の檻
//! （areka-P0-windowposition-limit task 2.2・design C3「取得経路の拡張」）。
//!
//! 分類器そのものの檻は `windowposition_vocab_tests.rs`（task 2.1・純関数）が持つ。
//! 本ファイルが固定するのはその 1 段上——**2 層マージ済みのバルーン定義から生値を取り出し、
//! 分類結果を `ScopeConfig` の実値へ反映し、不正値に scope 付きの警告を出し、観測点 4 へ
//! 解決済み実値を載せる**という結線そのもの（要件 1.3/1.4/4.6/6.2/6.3）である。
//!
//! 観測面は 2 つあり、どちらも欠かせない:
//! - **構成の実値**（`cfg.scopes[scope].balloon_limit`／`balloon_x_mode`）
//!   ＝下流（resolver P5・起動時関門）が実際に読む値
//! - **ログ**（警告 2 種と観測点 4 の `limit`／`x_mode` フィールド）
//!   ＝実機サインオフが grep する契約面（要件 6.2/6.3）
//!
//! ログ捕捉ハーネス（`test_support::capture_logs`）は既に較正済みであり、同じ観測点 4 の
//! 行を `placement_windowposition_tests.rs` が捕捉して値を検査している。本ファイルの
//! 各テストも**必ず観測点 4 の行を 1 件捕捉すること**を先に主張してから「警告が無い」側の
//! 主張を行う（捕捉ゼロで恒真になる空虚な主張を作らない）。

use areka_emo_compose::ScaleRatio;

use super::config::{BalloonXMode, PlacementConfig, ScopeConfig};
use super::shared_test_support::{TempDir, synth_balloon_dir};
use super::test_support::{ExpectField, LogEvent, capture_logs, expect_one};
use super::*;

/// 観測点 4 の本文（実機サインオフの grep 対象・フィールド名ごと契約）。
const OBSERVATION: &str = "windowposition を初期既定位置の調整量へ";
/// `limit` 不正値の縮退警告の本文（要件 1.3/6.3）。
const LIMIT_WARN: &str = "windowposition.limit が 0/1 以外";
/// `x` 不正値の縮退警告の本文（要件 4.6/6.3）。
const X_WARN: &str = "windowposition.x が数値でもキーワード";

/// scope 0 のみを持つ最小の `PlacementConfig`（全欄は正典既定）。
fn cfg_scope0() -> PlacementConfig {
    let mut cfg = PlacementConfig {
        scopes: Default::default(),
        zorder_raw: None,
        sticky_window_raw: None,
        shell_dpi_raw: None,
    };
    cfg.scopes.insert(0, ScopeConfig::default());
    cfg
}

/// 合成バルーン定義 1 つに対し `apply_scope_windowpositions` を通し、
/// 解決後の構成と捕捉ログを返す（k は恒等＝語彙の観測に丸めを混ぜない）。
fn resolve(dir_name: &str, extra_lines: &str) -> (PlacementConfig, Vec<LogEvent>) {
    let root = TempDir::new();
    let balloon_dir = synth_balloon_dir(&root, dir_name, extra_lines);
    let mut cfg = cfg_scope0();
    let (_, events) = capture_logs(|| {
        apply_scope_windowpositions(&mut cfg, &balloon_dir, &[0], ScaleRatio::ONE);
    });
    (cfg, events)
}

/// 警告本文 `needle` を含むイベントが 1 件も無いことを主張する。
///
/// 空虚化しないための前提として、呼び手は**同じ `events` から観測点 4 を 1 件取り出した後**に
/// 呼ぶこと（捕捉が生きていることが確認済みの集合に対してのみ「無い」を主張する）。
fn assert_no_event(events: &[LogEvent], needle: &str) {
    let hits: Vec<&LogEvent> = events
        .iter()
        .filter(|e| e.message().contains(needle))
        .collect();
    assert!(
        hits.is_empty(),
        "`{needle}` の警告が出てはならない: {hits:?}"
    );
}

// ------------------------------------------------------------------
// 反映（要件 1.1/1.4/4.1・design C3 (b)）
// ------------------------------------------------------------------

/// `limit` の受理値（0／1）が scope 別構成の実値へそのまま入る（要件 1.1/1.4）。
#[test]
fn apply_scope_windowpositions_reflects_accepted_limit_values() {
    for (raw, expected) in [("0", false), ("1", true)] {
        let (cfg, events) = resolve(
            &format!("balloon-limit-{raw}"),
            &format!("windowposition.limit,{raw}\n"),
        );
        assert_eq!(
            cfg.scopes[&0].balloon_limit, expected,
            "limit={raw} は {expected} として保持される"
        );
        let hit = expect_one(&events, OBSERVATION);
        assert_eq!(
            hit.expect_field("limit"),
            expected.to_string(),
            "観測点 4 の limit は解決済み実値（要件 6.2）"
        );
        assert_no_event(&events, LIMIT_WARN);
    }
}

/// `x` のキーワード 3 種が水平配置モードの実値へ落ちる（要件 4.1・`top` は `center` と同義）。
///
/// あわせて**キーワード時は数値 x が供給されない**ことを固定する（design C3 (d)）:
/// `windowposition.y` だけが調整量として残り、x 成分は 0 になる。
#[test]
fn apply_scope_windowpositions_reflects_keyword_x_modes() {
    for (raw, expected) in [
        ("center", BalloonXMode::CenterTop),
        ("top", BalloonXMode::CenterTop),
        ("bottom", BalloonXMode::CenterBottom),
    ] {
        let (cfg, events) = resolve(
            &format!("balloon-x-{raw}"),
            &format!("windowposition.x,{raw}\nwindowposition.y,-129\n"),
        );
        assert_eq!(
            cfg.scopes[&0].balloon_x_mode, expected,
            "x={raw} は {expected:?} として保持される"
        );
        assert_eq!(
            cfg.scopes[&0].balloon_offset,
            Some((0, -129)),
            "キーワード時の x 調整量は 0（数値 x は存在しない）・y は従来どおり（要件 4.4）"
        );
        let hit = expect_one(&events, OBSERVATION);
        assert_eq!(hit.expect_field("x_mode"), format!("{expected:?}"));
        assert_eq!(
            hit.expect_field("windowposition_x"),
            "None",
            "キーワードは数値 x を作らない（C1 の不変量）"
        );
        assert_no_event(&events, X_WARN);
    }
}

// ------------------------------------------------------------------
// 不正値の警告付き縮退（要件 1.3/4.6/6.3・design C3 (a)）
// ------------------------------------------------------------------

/// **task 2.2 の完了状態**: 不正な `limit` と不正な `x` を含む定義を通すと、
/// 警告が 2 種とも出て、構成にも観測点 4 にも**既定値**が現れる。
///
/// `Center`（大文字混在）が不正値になることは意図的な設計である——キーワードは小文字の
/// 完全一致のみ受理し、綴り違いは無言で数値扱いへ落とさず警告で作者に気づかせる（DD8・要件 4.6）。
#[test]
fn apply_scope_windowpositions_warns_and_degrades_on_invalid_limit_and_x() {
    let (cfg, events) = resolve(
        "balloon-invalid-both",
        "windowposition.limit,2\nwindowposition.x,Center\n",
    );

    // (a) 構成は正典既定へ縮退している。
    assert_eq!(
        cfg.scopes[&0].balloon_limit, true,
        "0/1 以外は正典既定 1（画面内へ維持）へ縮退（要件 1.3）"
    );
    assert_eq!(
        cfg.scopes[&0].balloon_x_mode,
        BalloonXMode::Side,
        "語彙外の x は未指定扱い＝現行の左右分岐のまま（要件 4.6）"
    );
    assert_eq!(
        cfg.scopes[&0].balloon_offset, None,
        "語彙外の x は調整量を作らない（未指定と同じ扱い）"
    );

    // (b) 警告が 2 種とも出ており、どちらも scope と生値を伴う（要件 6.3）。
    let limit_warn = expect_one(&events, LIMIT_WARN);
    assert_eq!(limit_warn.level, tracing::Level::WARN);
    assert_eq!(limit_warn.expect_field("scope"), "0");
    assert_eq!(limit_warn.expect_field("limit_raw"), "Some(\"2\")");

    let x_warn = expect_one(&events, X_WARN);
    assert_eq!(x_warn.level, tracing::Level::WARN);
    assert_eq!(x_warn.expect_field("scope"), "0");
    assert_eq!(x_warn.expect_field("x_raw"), "Some(\"Center\")");

    // (c) 観測点 4 の解決値は既定値（要件 6.2）。
    let hit = expect_one(&events, OBSERVATION);
    assert_eq!(hit.level, tracing::Level::INFO);
    assert_eq!(hit.expect_field("limit"), "true");
    assert_eq!(hit.expect_field("x_mode"), "Side");
}

/// 値が空（`windowposition.x,`）は**未指定ではなく不正値**である（要件 7.2）。
/// キーはあるので生値 `Some("")` が届き、警告つきで未指定扱いへ縮退する。
#[test]
fn apply_scope_windowpositions_treats_empty_value_as_invalid_not_unspecified() {
    let (cfg, events) = resolve(
        "balloon-empty-x",
        "windowposition.x,\nwindowposition.limit,\n",
    );

    assert_eq!(cfg.scopes[&0].balloon_x_mode, BalloonXMode::Side);
    assert_eq!(cfg.scopes[&0].balloon_limit, true);

    assert_eq!(
        expect_one(&events, X_WARN).expect_field("x_raw"),
        "Some(\"\")"
    );
    assert_eq!(
        expect_one(&events, LIMIT_WARN).expect_field("limit_raw"),
        "Some(\"\")"
    );
}

// ------------------------------------------------------------------
// 未指定・数値経路（要件 1.2/5.1/7.2）
// ------------------------------------------------------------------

/// 未指定は**無警告**で正典既定（limit=1・`Side`）になる——不正値と区別される（要件 1.2/7.2）。
///
/// 「警告が無い」ことの主張が空虚にならないよう、先に観測点 4 が 1 件捕捉できていることを
/// 確かめる（捕捉ハーネスが生きていることの較正）。
#[test]
fn apply_scope_windowpositions_unspecified_vocab_is_silent_and_defaults() {
    let (cfg, events) = resolve("balloon-unspecified", "");

    let hit = expect_one(&events, OBSERVATION);
    assert_eq!(hit.expect_field("limit"), "true");
    assert_eq!(hit.expect_field("x_mode"), "Side");
    assert_eq!(
        hit.expect_field("adjusted"),
        "false",
        "どの軸も指定が無ければ調整量そのものが無い"
    );

    assert_eq!(cfg.scopes[&0].balloon_limit, true);
    assert_eq!(cfg.scopes[&0].balloon_x_mode, BalloonXMode::Side);
    assert_eq!(cfg.scopes[&0].balloon_offset, None);

    assert_no_event(&events, LIMIT_WARN);
    assert_no_event(&events, X_WARN);
}

/// 数値指定の経路は本仕様の前後で 1 ビットも変わらない（要件 5.1 の回帰境界）。
/// 数値が読めている限り生値は参照されず、モードは `Side` のまま・調整量は生値どおり。
#[test]
fn apply_scope_windowpositions_numeric_x_keeps_side_mode_and_existing_adjust() {
    let (cfg, events) = resolve(
        "balloon-numeric",
        "windowposition.x,266\nwindowposition.y,-129\nwindowposition.limit,1\n",
    );

    assert_eq!(cfg.scopes[&0].balloon_x_mode, BalloonXMode::Side);
    assert_eq!(cfg.scopes[&0].balloon_limit, true);
    assert_eq!(
        cfg.scopes[&0].balloon_offset,
        Some((266, -129)),
        "数値経路の調整量は従来どおり（生値そのまま・k=1）"
    );

    let hit = expect_one(&events, OBSERVATION);
    assert_eq!(hit.expect_field("windowposition_x"), "Some(266)");
    assert_eq!(hit.expect_field("adjust_dx"), "266");
    assert_eq!(hit.expect_field("x_mode"), "Side");
    assert_no_event(&events, X_WARN);
}
