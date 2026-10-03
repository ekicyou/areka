//! `port` の決定論テスト（要件 2.6 の 9 値＋非 UTF-8）。

use std::env::VarError;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use log_capture_kit::capture;
use tracing::Level;

use super::{DEFAULT_PORT, PORT_ENV, port_from_env_value, port_from_var};

fn warn_count(events: &[log_capture_kit::CapturedEvent]) -> usize {
    events.iter().filter(|e| e.level == Level::WARN).count()
}

/// (要件 2.1, 2.7) 既定の番号と環境変数の名前を固定する（9801 は SSP の番号なので使わない）。
#[test]
fn default_port_and_env_name_are_fixed() {
    assert_eq!(DEFAULT_PORT, 9821);
    assert_eq!(PORT_ENV, "AREKA_MCP_PORT");
}

/// (要件 2.1〜2.3, 2.5, 2.6) 9 値の表: 値 → 待ち受ける番号／待ち受けない（None）と warn の件数。
#[test]
fn port_from_env_value_fixed_table() {
    let table: [(Option<&str>, Option<u16>, usize); 9] = [
        (None, Some(9821), 0),
        (Some("0"), None, 0),
        (Some("9821"), Some(9821), 0),
        (Some("65535"), Some(65535), 0),
        (Some("65536"), Some(9821), 1),
        (Some("-1"), Some(9821), 1),
        (Some("abc"), Some(9821), 1),
        (Some(""), Some(9821), 1),
        (Some(" 9000 "), Some(9000), 0),
    ];
    for (value, want, warns) in table {
        let (got, events) = capture(|| port_from_env_value(value));
        assert_eq!(got, want, "値 {value:?}");
        assert_eq!(
            warn_count(&events),
            warns,
            "値 {value:?} の warn: {events:?}"
        );
        assert_eq!(
            events.len(),
            warns,
            "値 {value:?} は warn 以外を出さない: {events:?}"
        );
    }
}

/// (要件 2.5) 空白だけ・溢れる値も読めない値として既定へ倒す。
#[test]
fn port_from_env_value_blank_and_overflow_fall_back() {
    for value in ["   ", "99999999999999999999", "+"] {
        let (got, events) = capture(|| port_from_env_value(Some(value)));
        assert_eq!(got, Some(DEFAULT_PORT), "値 {value:?}");
        assert_eq!(warn_count(&events), 1, "値 {value:?}: {events:?}");
    }
}

/// (要件 2.5) 読めない値の warn にはその値を載せる。
#[test]
fn port_from_env_value_warn_carries_the_value() {
    let (_, events) = capture(|| port_from_env_value(Some("abc")));
    let warn = events
        .iter()
        .find(|e| e.level == Level::WARN)
        .expect("warn");
    assert!(
        warn.fields
            .iter()
            .any(|(k, v)| k == "value" && v.debug.contains("abc")),
        "{warn:?}"
    );
}

/// (要件 2.5) UTF-8 でない値は warn 1 件で既定へ倒す（実際の環境変数は触らず、読み口の判断へ直接渡す）。
#[test]
fn read_port_env_non_unicode_falls_back_with_warn() {
    // 対になっていないサロゲート＝UTF-8 へ写せない値。
    let raw = OsString::from_wide(&[0xD800]);
    assert!(raw.to_str().is_none(), "較正: この値は UTF-8 でない");
    let (got, events) = capture(|| port_from_var(Err(VarError::NotUnicode(raw))));
    assert_eq!(got, Some(DEFAULT_PORT));
    assert_eq!(warn_count(&events), 1, "{events:?}");
    assert_eq!(events.len(), 1, "{events:?}");
}

/// (要件 2.2, 2.3) 読み口は未設定と読める値を純粋な判断へそのまま渡す。
#[test]
fn port_from_var_passes_through_to_the_decision() {
    let (got, events) = capture(|| port_from_var(Err(VarError::NotPresent)));
    assert_eq!(got, Some(DEFAULT_PORT));
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(port_from_var(Ok("0".to_owned())), None);
    assert_eq!(port_from_var(Ok("12345".to_owned())), Some(12345));
}
