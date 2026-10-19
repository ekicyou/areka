//! 起動オプションの中身の既定値（要件 1.13・7.1）。

use super::*;

#[test]
fn defaults_load_modules_listen_not_given_no_help() {
    let opts = StartupOptions::default();
    assert_eq!(opts.modules, ModulePolicy::Load);
    assert_eq!(opts.listen, ListenPlan::NotGiven);
    assert!(!opts.help);
    assert_eq!(opts.boot, BootRequest::default());
    assert_eq!(opts.boot.positional_ghost, None);
    assert_eq!(opts.boot.ghost, None);
    assert!(!opts.boot.random && !opts.boot.no_boot_check);
}

/// 本物の読み口: 置き換えの warn が先、まとめの info が最後。記録はすべて `[cli]` で始まる（1.12・1.14・7.1）。
#[cfg(windows)]
#[test]
fn read_from_logs_notes_and_returns_options() {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    let args = vec![
        OsString::from("areka"),
        OsString::from("--option"),
        OsString::from("dontloadmodule,bootunlock"),
        OsString::from_wide(&[0x61, 0xD800]),
    ];
    let (opts, events) = log_capture_kit::capture(|| read_from(&args));
    assert_eq!(opts.modules, ModulePolicy::Skip);
    assert_eq!(opts.boot.positional_ghost, Some(PathBuf::from("a\u{FFFD}")));
    let kinds: Vec<_> = events
        .iter()
        .map(|e| (e.level, e.field_str("event")))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (tracing::Level::WARN, Some("startup_arg_not_utf8")),
            (tracing::Level::INFO, Some("option_word_noop")),
            (tracing::Level::INFO, Some("startup_options")),
        ]
    );
    assert!(events.iter().all(|e| e.message().starts_with("[cli] ")));
}
