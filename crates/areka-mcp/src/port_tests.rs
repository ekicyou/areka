//! `port` の決定論テスト（要件 2.6 の 9 値＋20 候補の並び＋非 UTF-8）。ソケットは開かない。

use std::env::VarError;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

use log_capture_kit::capture;
use tracing::Level;

use super::{
    DEFAULT_PORTS, FALLBACK_STEPS, PORT_ENV, candidates_from_env_value, candidates_from_var,
};

fn warn_count(events: &[log_capture_kit::CapturedEvent]) -> usize {
    events.iter().filter(|e| e.level == Level::WARN).count()
}

/// 既定の 20 候補（未設定のときの列）。
fn defaults() -> Vec<u16> {
    candidates_from_env_value(None)
}

/// (要件 2.1, 2.8) 既定の候補は 9801 → 9821 から始まり、隣（+1〜+9）を交互に足した 20 個。
#[test]
fn default_candidates_are_twenty_in_alternating_order() {
    assert_eq!(DEFAULT_PORTS, [9801, 9821]);
    assert_eq!(FALLBACK_STEPS, 9);
    assert_eq!(PORT_ENV, "AREKA_MCP_PORT");
    let want: [u16; 20] = [
        9801, 9821, 9802, 9822, 9803, 9823, 9804, 9824, 9805, 9825, 9806, 9826, 9807, 9827, 9808,
        9828, 9809, 9829, 9810, 9830,
    ];
    let got = defaults();
    assert_eq!(got, want);
    let mut unique = got.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), 20, "重複なし: {got:?}");
}

/// (要件 2.1〜2.3, 2.5, 2.6) 9 値の表: 値 → 試す候補の列（空＝待ち受けない）と warn の件数。
#[test]
fn candidates_from_env_value_fixed_table() {
    let defaults = defaults();
    let table: [(Option<&str>, Vec<u16>, usize); 9] = [
        (None, defaults.clone(), 0),
        (Some("0"), vec![], 0),
        (Some("9821"), vec![9821], 0),
        (Some("65535"), vec![65535], 0),
        (Some("65536"), defaults.clone(), 1),
        (Some("-1"), defaults.clone(), 1),
        (Some("abc"), defaults.clone(), 1),
        (Some(""), defaults.clone(), 1),
        (Some(" 9000 "), vec![9000], 0),
    ];
    for (value, want, warns) in table {
        let (got, events) = capture(|| candidates_from_env_value(value));
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

/// (要件 2.5) 空白だけ・溢れる値も読めない値として既定の候補へ倒す。
#[test]
fn candidates_from_env_value_blank_and_overflow_fall_back() {
    for value in ["   ", "99999999999999999999", "+"] {
        let (got, events) = capture(|| candidates_from_env_value(Some(value)));
        assert_eq!(got, defaults(), "値 {value:?}");
        assert_eq!(warn_count(&events), 1, "値 {value:?}: {events:?}");
    }
}

/// (要件 2.5) 読めない値の warn にはその値を載せ、文面は「既定の候補で待ち受ける」。
#[test]
fn candidates_from_env_value_warn_carries_the_value() {
    let (_, events) = capture(|| candidates_from_env_value(Some("abc")));
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
    assert!(
        warn.message().contains("既定の候補で待ち受ける"),
        "{warn:?}"
    );
}

/// (要件 2.5) UTF-8 でない値は warn 1 件で既定の候補へ倒す（実際の環境変数は触らず、読み口の判断へ直接渡す）。
#[test]
fn read_port_candidates_non_unicode_falls_back_with_warn() {
    // 対になっていないサロゲート＝UTF-8 へ写せない値。
    let raw = OsString::from_wide(&[0xD800]);
    assert!(raw.to_str().is_none(), "較正: この値は UTF-8 でない");
    let (got, events) = capture(|| candidates_from_var(Err(VarError::NotUnicode(raw))));
    assert_eq!(got, defaults());
    assert_eq!(warn_count(&events), 1, "{events:?}");
    assert_eq!(events.len(), 1, "{events:?}");
    assert!(
        events[0].message().contains("既定の候補で待ち受ける"),
        "{events:?}"
    );
}

/// (要件 2.2, 2.3) 読み口は未設定と読める値を純粋な判断へそのまま渡す（指定の番号に隣は足さない）。
#[test]
fn candidates_from_var_passes_through_to_the_decision() {
    let (got, events) = capture(|| candidates_from_var(Err(VarError::NotPresent)));
    assert_eq!(got, defaults());
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(candidates_from_var(Ok("0".to_owned())), Vec::<u16>::new());
    assert_eq!(candidates_from_var(Ok("12345".to_owned())), vec![12345]);
}
