//! 写した記録から CSV の中身・終了コード・CSV のファイルを作る（要件 7.1・7.2・7.4〜7.8・8.1〜8.3）。

use std::path::Path;

use tracing::Level;

use crate::log_history::CapturedRecord;

/// 写しが空なら 0・warn だけなら 1・error があれば 2（8.1）。CSV の有無に関わらない（8.2）。
pub(crate) fn exit_code(records: &[CapturedRecord]) -> i32 {
    match worst(records) {
        None => 0,
        Some("Warning") => 1,
        Some(_) => 2,
    }
}

/// 写しの最も重いレベルの語（`Warning`／`Error`）。空なら `None`。
pub(crate) fn worst(records: &[CapturedRecord]) -> Option<&'static str> {
    // tracing の Level は重いほど小さい（ERROR < WARN）。
    records.iter().map(|r| r.level).min().map(level_word)
}

/// 写しは warn と error だけ（info は窓に入らない＝7.4・8.3）。
fn level_word(level: Level) -> &'static str {
    if level == Level::ERROR {
        "Error"
    } else {
        "Warning"
    }
}

/// BOM 付き UTF-8・CRLF・`"時刻","ゴースト名","レベル","内容"` の 4 列（7.1・7.2・7.5）。
pub(crate) fn csv_bytes(ghost_name: &str, records: &[CapturedRecord]) -> Vec<u8> {
    let quote = |s: &str| s.replace('"', "\"\"");
    let ghost = quote(ghost_name);
    let mut text = String::from("\u{FEFF}");
    for r in records {
        text.push_str(&format!(
            "\"{:02}:{:02}:{:02}\",\"{ghost}\",\"{}\",\"{}\"\r\n",
            r.at.hour,
            r.at.minute,
            r.at.second,
            level_word(r.level),
            quote(&r.body)
        ));
    }
    text.into_bytes()
}

/// records が空なら何もしない（作らず、在るファイルに触れない＝7.6）。書けなければ error! を出して Err（7.7）。
pub(crate) fn write_csv(
    path: &Path,
    ghost_name: &str,
    records: &[CapturedRecord],
) -> std::io::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    std::fs::write(path, csv_bytes(ghost_name, records)).inspect_err(|e| {
        tracing::error!(
            event = "offline_dump_error_log_write_failed",
            path = %path.display(),
            error = %e,
            "[offline_dump] --dump-error-log の CSV を書けませんでした"
        );
    })
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod report_tests;
