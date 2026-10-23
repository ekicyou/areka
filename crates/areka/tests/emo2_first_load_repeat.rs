//! くり返しの試験（areka-P0-emo2-real-run-stability 要件 4.1〜4.4・4.6〜4.8・5.1〜5.3・6.1）。
//!
//! 新しい写しからの emo2 の最初の起動を N 回回し、成功・LOAD の失敗・ほかの失敗を数える。1 回の走りは
//! 件 1 の試験と同じ共通の手続き（`emo2_real_run/common.rs` の `spawn_emo2`＋`boot_and_close`）。
//!
//! ## 門（要件 4.2）
//! `AREKA_EMO2_REPEAT` が無い・空・空白だけなら子を起こさず素通り。数でない・0・UTF-8 でない値と、
//! `AREKA_EMO2_REPEAT_DROP` の知らない値は、子を起こさずに理由を書いて赤。
//!
//! ## 比べの条件（`AREKA_EMO2_REPEAT_DROP`・要件 4.4・6.1）
//! - 無い・空: 基本。写した直後に起こし、前の回の直後に次を起こし、待つ間に MCP で面とバルーンを読む。
//! - `gap`: 前の回の終わりから次の起動まで 10 秒あける。
//! - `copy-wait`: 写しを取った後 10 秒待ってから起こす。
//! - `no-mcp`: 待つ間の MCP の呼び出しを止める。
//!
//! ## 出力（ASCII・標準エラー・要件 4.6・5.2）
//!
//!     emo2_first_load_repeat run=1/20 outcome=ok: wire_ms=812 attach_ms=1540 ...
//!     emo2_first_load_repeat run=2/20 outcome=load_failed: kept=C:\...\target\test-roots\areka-rr-1234-2
//!     emo2_first_load_repeat: condition=basic runs=20 ok=19 load_failed=1 other_failed=0
//!
//! 落ちた回の置き場（写しと出力）は窓口が残し、成功の回は消える（要件 5.1・5.3）。落ちた回が 1 回でも
//! あれば、全部の回の後にまとめと落ちた回の一覧で赤（要件 4.8）。
//!
//! ## 回し方
//! 排他テストの机を借りてから（`doc/impl-watch.md`）:
//!
//!     $env:AREKA_EMO2_REPEAT="20"; cargo test -p areka --test emo2_first_load_repeat -- --nocapture
//!     $env:AREKA_EMO2_REPEAT="20"; $env:AREKA_EMO2_REPEAT_DROP="gap"; cargo test -p areka --test emo2_first_load_repeat -- --nocapture

// 件 1 の試験だけが使う残りの警告を抑える。
#[allow(dead_code)]
#[path = "emo2_real_run/common.rs"]
mod common;

use common::{Condition, Outcome};
use std::panic::{AssertUnwindSafe, catch_unwind};

const REPEAT_ENV: &str = "AREKA_EMO2_REPEAT";
const DROP_ENV: &str = "AREKA_EMO2_REPEAT_DROP";
const WHO: &str = "emo2_first_load_repeat";

/// 回数の値の読み。無い・空・空白だけ＝`Ok(None)`（素通り）。数でない・0 は Err（理由の文）。
fn parse_count(value: Option<&str>) -> Result<Option<u32>, String> {
    let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    match v.parse::<u32>() {
        Ok(0) | Err(_) => Err(format!(
            "{REPEAT_ENV}={v:?} は回数として読めません（1 以上の整数を置く）"
        )),
        Ok(n) => Ok(Some(n)),
    }
}

fn condition_name(c: Condition) -> &'static str {
    match c {
        Condition::Basic => "basic",
        Condition::Gap => "gap",
        Condition::CopyWait => "copy-wait",
        Condition::NoMcp => "no-mcp",
    }
}

fn outcome_name(o: Outcome) -> &'static str {
    match o {
        Outcome::Ok => "ok",
        Outcome::LoadFailed => "load_failed",
        Outcome::OtherFailed => "other_failed",
    }
}

/// 各回の行の頭（`Times::line` の `who` にも使う）。
fn run_head(i: u32, n: u32, o: Outcome) -> String {
    format!("{WHO} run={i}/{n} outcome={}", outcome_name(o))
}

/// 落ちた回の分け（写しておいた出力の全文から）。
fn classify_failed(output: &str) -> Outcome {
    if common::is_load_returned_false(output) {
        Outcome::LoadFailed
    } else {
        Outcome::OtherFailed
    }
}

fn summary_line(c: Condition, n: u32, outcomes: &[Outcome]) -> String {
    let count = |o| outcomes.iter().filter(|&&x| x == o).count();
    format!(
        "{WHO}: condition={} runs={n} ok={} load_failed={} other_failed={}",
        condition_name(c),
        count(Outcome::Ok),
        count(Outcome::LoadFailed),
        count(Outcome::OtherFailed),
    )
}

/// 環境変数を読む。無い＝None、UTF-8 でない＝子を起こさずに赤。
fn read_env(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) => Some(v),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(v)) => {
            panic!("{name}={v:?} は UTF-8 でないので読めません")
        }
    }
}

#[test]
fn emo2_first_load_repeat() {
    // 門と値の読みは、子を起こす前に全部済ませる。
    let n = match parse_count(read_env(REPEAT_ENV).as_deref()) {
        Ok(Some(n)) => n,
        Ok(None) => {
            eprintln!("{WHO}: skipped (opt-in; set {REPEAT_ENV}=<count> to run)");
            return;
        }
        Err(why) => panic!("{why}"),
    };
    let condition = Condition::parse(&read_env(DROP_ENV).unwrap_or_default())
        .unwrap_or_else(|why| panic!("{why}"));
    let wait_after_copy = (condition == Condition::CopyWait).then_some(common::DROP_WAIT);
    let mcp = condition != Condition::NoMcp;

    let mut outcomes = Vec::new();
    let mut failed = Vec::new();
    for i in 1..=n {
        if i > 1 && condition == Condition::Gap {
            std::thread::sleep(common::DROP_WAIT);
        }
        // 起こすのは受け止めの外（写しが取れない・仲介が揃わないのはくり返しごと赤）。
        let (run, started) = common::spawn_emo2(wait_after_copy);
        let files = [run.stdout_path().to_owned(), run.stderr_path().to_owned()];
        let kept = run.run_dir().to_owned();
        // 取っ手は受け止めの内側で捨てる（panic の途中なら窓口が置き場を残す）。
        let line = match catch_unwind(AssertUnwindSafe(move || {
            common::boot_and_close(run, started, mcp)
        })) {
            Ok(times) => {
                outcomes.push(Outcome::Ok);
                times.line(&run_head(i, n, Outcome::Ok))
            }
            Err(_) => {
                let output: String = files
                    .iter()
                    .map(|p| match std::fs::read(p) {
                        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                        // 読めない回は other_failed に数える（黙って落とさない）。
                        Err(e) => {
                            eprintln!("{WHO}: cannot read output {} ({:?})", p.display(), e.kind());
                            String::new()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let o = classify_failed(&output);
                outcomes.push(o);
                let line = format!("{}: kept={}", run_head(i, n, o), kept.display());
                failed.push(line.clone());
                line
            }
        };
        eprintln!("{line}");
    }
    let summary = summary_line(condition, n, &outcomes);
    eprintln!("{summary}");
    assert!(failed.is_empty(), "{summary}\n{}", failed.join("\n"));
}

#[test]
fn parse_count_passes_through_when_missing_or_blank() {
    assert_eq!(parse_count(None), Ok(None));
    assert_eq!(parse_count(Some("")), Ok(None));
    assert_eq!(parse_count(Some("  ")), Ok(None));
}

#[test]
fn parse_count_reads_a_positive_count() {
    assert_eq!(parse_count(Some(" 20 ")), Ok(Some(20)));
}

#[test]
fn parse_count_rejects_zero_and_non_numbers_with_a_reason() {
    for v in ["0", "abc", "-3", "1.5"] {
        let err = parse_count(Some(v)).unwrap_err();
        assert!(err.contains(REPEAT_ENV), "{err}");
    }
}

#[test]
fn run_and_summary_lines_are_ascii() {
    assert_eq!(
        run_head(3, 20, Outcome::LoadFailed),
        "emo2_first_load_repeat run=3/20 outcome=load_failed"
    );
    let outcomes = [
        Outcome::Ok,
        Outcome::LoadFailed,
        Outcome::Ok,
        Outcome::OtherFailed,
    ];
    let line = summary_line(Condition::CopyWait, 4, &outcomes);
    assert_eq!(
        line,
        "emo2_first_load_repeat: condition=copy-wait runs=4 ok=2 load_failed=1 other_failed=1"
    );
    assert!(line.is_ascii());
    for c in [Condition::Basic, Condition::Gap, Condition::NoMcp] {
        assert!(summary_line(c, 1, &[]).contains(&format!("condition={}", condition_name(c))));
    }
}

#[test]
fn classify_failed_splits_load_failure_from_others() {
    let load = format!(
        "x\n2026-10-23T00:00:00.000000Z ERROR {}\n",
        common::LOAD_RETURNED_FALSE
    );
    assert_eq!(classify_failed(&load), Outcome::LoadFailed);
    assert_eq!(
        classify_failed("event=\"connect_failed\""),
        Outcome::OtherFailed
    );
}
