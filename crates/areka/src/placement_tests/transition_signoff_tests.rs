//! 実機ログのサインオフランナー（task 3.2・要件 8.3・design C7 の Integration）。
//!
//! 決定論テストと**同一の純関数**（[`super::judge_transition_log`]）を実機ログへ当てる入口で
//! ある。判定の実装をここに 1 行も持たないのが要点で、持った瞬間に「決定論テストが緑でも
//! サインオフだけ別の判定を通る」形が生まれる。Python 版は作らない（C7）。
//!
//! # 走らせ方
//!
//! ```text
//! AREKA_TRANSITION_LOG=<絶対パス> cargo test -p areka transition_signoff -- --ignored --nocapture
//! ```
//!
//! 既定の `cargo test` では `#[ignore]` により 1 度も走らない——実機ログが無い環境で
//! 「違反 0 件」を出すと、それが充足の根拠に化けるためである（要件 8.5）。
//!
//! # 静かに成功しない
//!
//! 環境変数が無い・パスが読めない・観測行が 1 行も無い・遷移が 1 本も無い——いずれも
//! **失敗**として落とす。無視指定のテストが不備なパスで緑になるのは、テストが無いより悪い。
//! 判定の入口は [`signoff_log`]（純関数・環境変数を引数で受ける）に分けてあり、既定で走る
//! テストがその失敗経路を固定する。

use std::fs;
use std::path::Path;

use temp_path_kit::TempPath;

use super::{Report, TRANSITION_LOG_ENV, judge_transition_log};

/// 実機ログを読み、判定まで通す。
///
/// `raw_path` は環境変数の値（未設定なら `None`）。I/O の失敗も「観測行 0 行」も `Err` にする
/// ——どれも「違反 0 件」を作れてしまう入力だからである。
fn signoff_log(raw_path: Option<&str>) -> Result<(Report, String), String> {
    let Some(raw_path) = raw_path.map(str::trim).filter(|path| !path.is_empty()) else {
        return Err(format!(
            "{TRANSITION_LOG_ENV} が未設定である。実機ログの絶対パスを与えて \
             `{TRANSITION_LOG_ENV}=<絶対パス> cargo test -p areka transition_signoff -- --ignored --nocapture` \
             で実行する"
        ));
    };
    let path = Path::new(raw_path);
    let log = fs::read_to_string(path).map_err(|error| {
        format!("{TRANSITION_LOG_ENV}={raw_path} を読めない: {error}（絶対パスで与えること）")
    })?;
    let report = judge_transition_log(&log);
    if report.records == 0 {
        return Err(format!(
            "{TRANSITION_LOG_ENV}={raw_path} に観測行が 1 行も無い（{} 文字）。\
             観測 target と水準が有効になっていない採取を「発生 0 回」の根拠にしない（要件 8.5）",
            log.len()
        ));
    }
    Ok((report, raw_path.to_owned()))
}

/// 実機ログの判定（既定では走らない）。
#[test]
#[ignore = "実機ログの判定（AREKA_TRANSITION_LOG に絶対パスを与えて明示実行する）"]
fn judges_a_real_machine_transition_log() {
    let raw_path = std::env::var(TRANSITION_LOG_ENV).ok();
    let (report, path) = match signoff_log(raw_path.as_deref()) {
        Ok(judged) => judged,
        Err(message) => panic!("{message}"),
    };

    // 合否によらず Report 全文を出す（`--nocapture` で手順書の記録へ貼る）。
    println!("== transition signoff: {path} ==");
    print!("{report}");

    assert!(
        !report.failed(),
        "実機ログが上限を満たさない（上の列挙が違反の全件・決定論／実機専用の 2 系統ぶん）"
    );
}

// ---------------------------------------------------------------------------
// 入口そのものの檻（既定で走る）
// ---------------------------------------------------------------------------

#[test]
fn a_missing_environment_variable_is_an_error_not_an_empty_pass() {
    for raw_path in [None, Some(""), Some("   ")] {
        let error = signoff_log(raw_path).expect_err("未設定は失敗でなければならない");
        assert!(
            error.contains(TRANSITION_LOG_ENV),
            "何を与えればよいか読めない: {error}"
        );
    }
}

#[test]
fn an_unreadable_path_is_an_error_not_an_empty_pass() {
    let temp = TempPath::new("transition-signoff-missing");
    let missing = temp.child("does-not-exist.log");
    assert!(!missing.exists(), "この檻は存在しないパスを前提にする");
    let error = signoff_log(missing.to_str()).expect_err("読めないパスは失敗");
    assert!(error.contains(TRANSITION_LOG_ENV), "{error}");
}

#[test]
fn a_log_without_any_observation_line_is_an_error_not_an_empty_pass() {
    // 空のファイルと、観測行を 1 行も含まないファイルの両方。どちらも「違反 0 件」を
    // 作れてしまう入力なので、合格ではなく失敗として落ちなければならない（要件 8.5）。
    let temp = TempPath::new("transition-signoff-empty");
    let path = temp.child("empty.log");
    for body in ["", "何も観測していないログ\n"] {
        fs::write(&path, body).expect("一時ファイルを書けるはず");
        let error = signoff_log(path.to_str()).expect_err("観測行 0 行は失敗");
        assert!(error.contains(TRANSITION_LOG_ENV), "{error}");
    }
    fs::remove_file(&path).expect("一時ファイルを消せるはず");
}

#[test]
fn a_directory_given_instead_of_a_log_is_an_error_not_an_empty_pass() {
    // 採取先のフォルダを渡す取り違えは実際に起こる。読めない理由が何であれ失敗にする。
    let temp = TempPath::new("transition-signoff-directory");
    let directory = temp.path();
    assert!(directory.is_dir());
    let error = signoff_log(directory.to_str()).expect_err("フォルダは読めない");
    assert!(error.contains(TRANSITION_LOG_ENV), "{error}");
}

#[test]
fn the_runner_reads_the_same_pure_functions_as_the_deterministic_tests() {
    // 埋め込みログを一時ファイルへ落として入口を一巡させ、判定が決定論テストと同じ結論
    // （現行ログは決定論の上限を満たさない）を返すことを固定する。
    let temp = TempPath::new("transition-signoff-fixture");
    let path = temp.child("fixture.log");
    fs::write(
        &path,
        super::transition_judge_reobservation_tests::REOBSERVATION_TRANSITION_1,
    )
    .expect("一時ファイルを書けるはず");

    let (report, echoed) = signoff_log(path.to_str()).expect("埋め込みログは判定できる");
    assert_eq!(echoed, path.to_str().expect("UTF-8 のはず"));
    assert_eq!(report.transitions.len(), 1);
    assert!(report.failed(), "是正前のログは合格しない");
    assert!(report.transitions[0].deterministic.is_err());
    assert!(report.transitions[0].signoff.is_err());

    fs::remove_file(&path).expect("一時ファイルを消せるはず");
}
