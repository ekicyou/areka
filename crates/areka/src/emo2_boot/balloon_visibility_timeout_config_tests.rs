// =============================================================================
// タイムアウト既定値の供給の決定論テスト（task 3.3）
//
// design.md「Testing Strategy → Unit Tests ③」（`parse_timeout_ms`: 未設定／正値／0・負・
// 非数の縮退表）と決定 D6（定数＋env）が所有する系統:
//   ⑴ 既定 30 秒が単一の定義箇所から供給されること — Requirement 4.2
//   ⑵ 環境変数名が `AREKA_` 名前空間規約に従うこと — steering「本番 env 変数は AREKA_ 冠」
//   ⑶ 指定の解釈の縮退表（未設定／空白／正値／0／負／非数／溢れ） — Requirement 4.2
//   ⑷ 採用値・供給源・既定値を 1 行で記録すること — Requirements 4.2 / 9.5
//   ⑸ 不正な指定は警告を出したうえで既定へ縮退すること — design「Error Strategy → 設定の不正」
//
// 本ファイルはプロセス大域の環境変数を一切書き換えない（同一プロセスで並行に走る他の
// テストを壊さないため）。env 文字列は `Option<&str>` として純関数へ直接与える。実時間の
// 待機・反復回数のみによる有界化も用いない（Requirement 9.2 / 9.3）。
// =============================================================================

use tracing::Level;

use super::*;
use crate::placement::test_support::{ExpectField, capture_logs, expect_one};

// ---------------------------------------------------------------------------
// ⑴⑵ 既定値の単一定義と環境変数名
// ---------------------------------------------------------------------------

/// 既定のタイムアウト時間は 30 秒である（Requirement 4.2）。
///
/// 値そのものを他所へ書き写さないことが要件の主眼であるため、ここは「唯一の定義箇所が
/// 30 秒を名乗っている」ことだけを固定する。
#[test]
fn default_timeout_is_thirty_seconds() {
    assert_eq!(
        DEFAULT_BALLOON_TIMEOUT_SECS, 30.0,
        "既定のタイムアウト時間は 30 秒（Requirement 4.2）"
    );
}

/// 短縮指定を受け付ける環境変数名（`AREKA_` 冠・design 決定 D6）。
#[test]
fn timeout_env_key_is_areka_namespaced() {
    assert_eq!(TIMEOUT_ENV_KEY, "AREKA_BALLOON_TIMEOUT_MS");
}

// ---------------------------------------------------------------------------
// ⑶ 指定の解釈の縮退表
// ---------------------------------------------------------------------------

/// 未設定・空・空白のみは「指定なし」であり、警告も出さない（本番既定の経路）。
///
/// 同一の捕捉窓の中で不正値をちょうど 1 つ通し、警告の出口が生きていることを併せて
/// 確かめる——「警告が無い」という主張が、捕捉ハーネスの取りこぼしによって恒真に
/// なっていないことを同じ窓で示すためである。
#[test]
fn parse_unset_or_blank_is_unspecified_and_silent() {
    let ((blank, calibration), events) = capture_logs(|| {
        let blank: Vec<Option<u64>> = [None, Some(""), Some("   "), Some("\t")]
            .into_iter()
            .map(parse_timeout_ms)
            .collect();
        // 較正: 不正値は必ず警告を出す（この 1 件が捕捉できなければ窓自体が壊れている）。
        let calibration = parse_timeout_ms(Some("abc"));
        (blank, calibration)
    });

    assert_eq!(
        blank,
        vec![None, None, None, None],
        "未設定・空・空白のみは指定なし"
    );
    assert_eq!(calibration, None, "較正に使う不正値も指定なしへ落ちる");

    let warned: Vec<&_> = events.iter().filter(|e| e.level == Level::WARN).collect();
    assert_eq!(
        warned.len(),
        1,
        "警告は較正に使った不正値の 1 件だけ（空白類は無音）: {events:?}"
    );
    assert!(
        warned[0].expect_field("value").contains("abc"),
        "捕捉された警告は較正値のもの: {:?}",
        warned[0]
    );
}

/// 正の整数ミリ秒はそのまま採用する（周辺の空白は落とす）。
#[test]
fn parse_accepts_positive_milliseconds() {
    let (adopted, events) = capture_logs(|| {
        ["1", "5000", " 5000 ", "\t30000\n", "18446744073709551615"]
            .into_iter()
            .map(|value| parse_timeout_ms(Some(value)))
            .collect::<Vec<_>>()
    });

    assert_eq!(
        adopted,
        vec![
            Some(1),
            Some(5000),
            Some(5000),
            Some(30_000),
            Some(u64::MAX)
        ],
        "正の整数ミリ秒（周辺空白トリム込み）は採用する"
    );
    assert!(events.is_empty(), "正常な指定はログを出さない: {events:?}");
}

/// 0・負値・非数・溢れはいずれも不正であり、警告を出して指定なしへ落とす。
///
/// 0 を不正側に置くのは design 決定 D6（`AREKA_BALLOON_TIMEOUT_MS` は**正の**整数 ms）と
/// Testing Strategy ③（「0・負・非数（warn＋既定）」）の指定による。0 を受理すると
/// 会話終了と同時にバルーンが消え、実機サインオフの ⑶（既定時間の経過を目視で確かめる）
/// が観測できなくなる。
#[test]
fn parse_rejects_zero_negative_non_numeric_and_overflow() {
    // (指定, 不正と判ずる理由)
    let cases: [(&str, &str); 9] = [
        ("0", "0 は正ではない"),
        (" 0 ", "空白を落としても 0 は正ではない"),
        ("-1", "負値"),
        ("-5000", "負値"),
        ("abc", "非数"),
        ("5000ms", "単位付きは非数"),
        ("1.5", "小数は非数"),
        ("5 000", "内部の空白は非数"),
        ("18446744073709551616", "u64 の範囲を超える"),
    ];

    for (value, why) in cases {
        let (parsed, events) = capture_logs(|| parse_timeout_ms(Some(value)));
        assert_eq!(parsed, None, "{value:?} は不正（{why}）");

        let warned: Vec<&_> = events.iter().filter(|e| e.level == Level::WARN).collect();
        assert_eq!(
            warned.len(),
            1,
            "{value:?}: 警告はちょうど 1 件: {events:?}"
        );
        assert!(
            warned[0].message().contains("[balloon-visibility]"),
            "{value:?}: 警告に接頭辞がある: {:?}",
            warned[0]
        );
        assert!(
            warned[0].expect_field("env").contains(TIMEOUT_ENV_KEY),
            "{value:?}: 警告が環境変数名を名指す: {:?}",
            warned[0]
        );
        assert!(
            warned[0].expect_field("value").contains(value.trim()),
            "{value:?}: 警告が受け取った指定を写す: {:?}",
            warned[0]
        );
    }
}

// ---------------------------------------------------------------------------
// ⑷⑸ 採用値・供給源の決定と記録
// ---------------------------------------------------------------------------

/// 環境変数なしでは既定 30 秒を採り、供給源を `default` として 1 行記録する。
#[test]
fn resolve_without_env_adopts_default_and_records_source() {
    let ((secs, source), events) = capture_logs(|| resolve_timeout_secs(None));

    assert_eq!(secs, DEFAULT_BALLOON_TIMEOUT_SECS, "既定 30 秒を採る");
    assert_eq!(source, TimeoutSource::Default, "供給源は既定");

    let recorded = expect_one(&events, "[balloon-visibility]");
    assert_eq!(recorded.level, Level::INFO, "採用値の記録は info 水準");
    assert!(
        recorded.expect_field("source").contains("default"),
        "供給源を記録する: {recorded:?}"
    );
    assert_eq!(
        recorded.expect_field("timeout_secs"),
        "30.0",
        "採用値を記録する"
    );
    assert_eq!(
        recorded.expect_field("default_secs"),
        "30.0",
        "短縮の有無に依らず既定値を併記する（Requirement 9.5）"
    );
    assert_eq!(
        events.len(),
        1,
        "記録は 1 行だけ（警告は出ない）: {events:?}"
    );
}

/// 正の指定は短縮として採用し、供給源を `env` として記録する（Requirement 9.5）。
#[test]
fn resolve_with_positive_env_shortens_and_records_source() {
    let ((secs, source), events) = capture_logs(|| resolve_timeout_secs(Some("5000")));

    assert_eq!(secs, 5.0, "5000ms は 5 秒");
    assert_eq!(source, TimeoutSource::Env, "供給源は環境変数");

    let recorded = expect_one(&events, "[balloon-visibility]");
    assert_eq!(recorded.level, Level::INFO);
    assert!(
        recorded.expect_field("source").contains("env"),
        "供給源を記録する: {recorded:?}"
    );
    assert_eq!(
        recorded.expect_field("timeout_secs"),
        "5.0",
        "短縮後の採用値"
    );
    assert_eq!(
        recorded.expect_field("default_secs"),
        "30.0",
        "短縮していない既定値が 30 秒であることを同じ 1 行で確かめられる（Requirement 9.5）"
    );
}

/// 不正な指定は警告を出したうえで既定 30 秒へ縮退し、供給源も `default` になる。
#[test]
fn resolve_with_invalid_env_warns_and_degrades_to_default() {
    for value in ["0", "-1", "abc", "1.5", "18446744073709551616"] {
        let ((secs, source), events) = capture_logs(|| resolve_timeout_secs(Some(value)));

        assert_eq!(
            secs, DEFAULT_BALLOON_TIMEOUT_SECS,
            "{value:?}: 既定 30 秒へ縮退する"
        );
        assert_eq!(source, TimeoutSource::Default, "{value:?}: 供給源は既定");

        let levels: Vec<Level> = events.iter().map(|e| e.level).collect();
        assert_eq!(
            levels,
            vec![Level::WARN, Level::INFO],
            "{value:?}: 警告のあとに採用値の記録が 1 行ずつ出る: {events:?}"
        );
        let recorded = &events[1];
        assert!(
            recorded.expect_field("source").contains("default"),
            "{value:?}: 縮退後の供給源は既定: {recorded:?}"
        );
        assert_eq!(recorded.expect_field("timeout_secs"), "30.0");
    }
}

/// 空白のみの指定は「未設定」と同じ扱いであり、警告なしで既定を採る。
#[test]
fn resolve_with_blank_env_is_treated_as_unset() {
    let ((secs, source), events) = capture_logs(|| resolve_timeout_secs(Some("   ")));

    assert_eq!(secs, DEFAULT_BALLOON_TIMEOUT_SECS);
    assert_eq!(source, TimeoutSource::Default);
    assert_eq!(events.len(), 1, "警告は出さない: {events:?}");
    assert_eq!(events[0].level, Level::INFO);
}

/// 供給源の表記は実機サインオフの文字列検索に使う語である（design 決定 D6）。
#[test]
fn timeout_source_labels_are_grep_vocabulary() {
    assert_eq!(TimeoutSource::Default.as_str(), "default");
    assert_eq!(TimeoutSource::Env.as_str(), "env");
}

// ---------------------------------------------------------------------------
// 読み取り口（env は 1 回だけ読む）
// ---------------------------------------------------------------------------

/// [`configured_timeout_secs`] は解決結果をプロセス寿命で 1 度だけ確定し、以後同じ値を返す。
///
/// プロセス大域の環境変数は書き換えない（並行して走る他テストを壊さないため）。読み取り
/// だけを行い、同じ入力に対する [`resolve_timeout_secs`] と一致することで結線を確かめる。
#[test]
fn configured_timeout_secs_reads_env_once_and_is_stable() {
    let expected = resolve_timeout_secs(std::env::var(TIMEOUT_ENV_KEY).ok().as_deref()).0;

    let first = configured_timeout_secs();
    let second = configured_timeout_secs();

    assert_eq!(first, expected, "環境変数の解決結果を採る");
    assert_eq!(first, second, "2 度目以降も同じ値（read-once）");
}
