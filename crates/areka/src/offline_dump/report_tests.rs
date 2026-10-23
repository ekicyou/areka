//! `report.rs` の試験（終了コードの段・BOM・CRLF・列・空なら作らない）。

use log_capture_kit::capture;
use temp_path_kit::TempPath;
use tracing::Level;

use super::{csv_bytes, exit_code, worst, write_csv};
use crate::log_history::{CapturedRecord, Hms};

fn rec(hour: u8, minute: u8, second: u8, level: Level, body: &str) -> CapturedRecord {
    CapturedRecord {
        at: Hms {
            hour,
            minute,
            second,
        },
        level,
        body: body.to_owned(),
    }
}

/// 空は 0・warn だけは 1・error があれば 2（8.1）。最も重いレベルの語も同じ段で決まる。
#[test]
fn exit_code_and_worst_follow_the_heaviest_level() {
    let warn = rec(1, 2, 3, Level::WARN, "w");
    let error = rec(1, 2, 3, Level::ERROR, "e");
    let warns = [warn.clone(), warn.clone()];
    let errors = [error.clone()];
    assert_eq!((exit_code(&[]), worst(&[])), (0, None));
    assert_eq!((exit_code(&warns), worst(&warns)), (1, Some("Warning")));
    assert_eq!((exit_code(&errors), worst(&errors)), (2, Some("Error")));
    let mixed = [warn.clone(), error, warn];
    assert_eq!((exit_code(&mixed), worst(&mixed)), (2, Some("Error")));
}

/// BOM・CRLF・4 列・時刻の 0 埋め・レベルの語・`"` の重ね（7.1・7.2・7.4）。
#[test]
fn csv_bytes_has_bom_crlf_quoted_four_columns() {
    let records = [
        rec(9, 5, 7, Level::WARN, "面 \"3\" が無い"),
        rec(23, 59, 0, Level::ERROR, "読めない"),
    ];
    let bytes = csv_bytes("ぱすた \"さん\"", &records);
    assert_eq!(&bytes[..3], b"\xEF\xBB\xBF");
    let text = std::str::from_utf8(&bytes[3..]).expect("UTF-8");
    assert_eq!(
        text,
        "\"09:05:07\",\"ぱすた \"\"さん\"\"\",\"Warning\",\"面 \"\"3\"\" が無い\"\r\n\
         \"23:59:00\",\"ぱすた \"\"さん\"\"\",\"Error\",\"読めない\"\r\n"
    );
}

/// ゴースト名が無ければ空の `""`（7.2）。
#[test]
fn empty_ghost_name_is_an_empty_quoted_column() {
    let bytes = csv_bytes("", &[rec(0, 0, 1, Level::WARN, "x")]);
    assert_eq!(&bytes[3..], b"\"00:00:01\",\"\",\"Warning\",\"x\"\r\n");
}

/// 10,001 件を間引かずに 1 件ずつ書く（7.5）。
#[test]
fn ten_thousand_and_one_records_are_all_written() {
    let records: Vec<_> = (0..10_001)
        .map(|i| rec(0, 0, 0, Level::WARN, &format!("r{i}")))
        .collect();
    let bytes = csv_bytes("g", &records);
    let text = std::str::from_utf8(&bytes[3..]).expect("UTF-8");
    assert_eq!(text.matches("\r\n").count(), 10_001);
    assert!(text.ends_with("\"r10000\"\r\n"));
}

/// 写しが空なら作らず、在るファイルにも触れない（7.6）。
#[test]
fn empty_records_do_not_create_or_touch_the_file() {
    let root = TempPath::under_target("offline_dump_report_empty");
    std::fs::create_dir_all(root.path()).expect("根の作成");
    let absent = root.path().join("absent.csv");
    let present = root.path().join("present.csv");
    std::fs::write(&present, b"old").expect("前のファイル");
    let ((a, b), events) =
        capture(|| (write_csv(&absent, "g", &[]), write_csv(&present, "g", &[])));
    assert!(a.is_ok() && b.is_ok());
    assert!(!absent.exists(), "作らない");
    assert_eq!(
        std::fs::read(&present).expect("読み"),
        b"old",
        "在るものに触れない"
    );
    assert!(events.is_empty(), "記録なし: {events:?}");
}

/// 記録があれば csv_bytes の中身をそのまま書く（前のファイルは上書き）。
#[test]
fn write_csv_writes_the_csv_bytes() {
    let root = TempPath::under_target("offline_dump_report_write");
    std::fs::create_dir_all(root.path()).expect("根の作成");
    let path = root.path().join("log.csv");
    std::fs::write(&path, b"old old old old old old old old old old old").expect("前のファイル");
    let records = [rec(12, 34, 56, Level::ERROR, "e")];
    write_csv(&path, "g", &records).expect("書ける");
    assert_eq!(
        std::fs::read(&path).expect("読み"),
        csv_bytes("g", &records)
    );
}

/// 書けなければ error を道筋と OS の誤りつきで出して Err（7.7）。
#[test]
fn unwritable_path_logs_error_and_returns_err() {
    let root = TempPath::under_target("offline_dump_report_unwritable");
    std::fs::create_dir_all(root.path()).expect("根の作成");
    // フォルダは CSV のファイルとして開けない。
    let (result, events) =
        capture(|| write_csv(root.path(), "g", &[rec(0, 0, 0, Level::WARN, "w")]));
    assert!(result.is_err());
    let errors: Vec<_> = events.iter().filter(|e| e.level == Level::ERROR).collect();
    assert_eq!(errors.len(), 1, "{events:?}");
    let e = errors[0];
    assert_eq!(
        e.field_str("event"),
        Some("offline_dump_error_log_write_failed")
    );
    assert!(
        e.field("path")
            .is_some_and(|p| p.contains("offline_dump_report_unwritable"))
    );
    assert!(e.field("error").is_some());
}
