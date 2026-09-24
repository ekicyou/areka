use super::*;

// -- 自動 close ゲート `smoke_exit_ms_from`（純粋・env 非依存・task 2.3・R4.1） --

/// env 未設定（`None`）ではゲート発火なし（`None`）＝タスク不投入。
#[test]
fn smoke_exit_ms_unset_yields_none() {
    assert_eq!(smoke_exit_ms_from(None), None);
}

/// 空文字・空白のみは発火なし（`None`）。
#[test]
fn smoke_exit_ms_empty_or_whitespace_yields_none() {
    assert_eq!(smoke_exit_ms_from(Some("")), None);
    assert_eq!(smoke_exit_ms_from(Some("   ")), None);
    assert_eq!(smoke_exit_ms_from(Some("\t")), None);
}

/// 非数値は発火なし（`None`）。
#[test]
fn smoke_exit_ms_non_numeric_yields_none() {
    assert_eq!(smoke_exit_ms_from(Some("abc")), None);
    assert_eq!(smoke_exit_ms_from(Some("12ms")), None);
    assert_eq!(smoke_exit_ms_from(Some("1.5")), None);
}

/// `"0"` は即時発火（0ms）として `Some(0)`。周辺空白はトリムして受理する。
#[test]
fn smoke_exit_ms_zero_yields_some_zero() {
    assert_eq!(smoke_exit_ms_from(Some("0")), Some(0));
    assert_eq!(smoke_exit_ms_from(Some("  0  ")), Some(0));
}

/// 正の整数はその値をミリ秒として受理する。
#[test]
fn smoke_exit_ms_positive_yields_some() {
    assert_eq!(smoke_exit_ms_from(Some("500")), Some(500));
    assert_eq!(smoke_exit_ms_from(Some(" 1500 ")), Some(1500));
}

/// 負値・`u64` 溢れは発火なし（`None`）＝不正入力はゲート OFF。
#[test]
fn smoke_exit_ms_negative_or_overflow_yields_none() {
    assert_eq!(smoke_exit_ms_from(Some("-1")), None);
    // u64::MAX + 1（20 桁）は溢れて None。
    assert_eq!(smoke_exit_ms_from(Some("18446744073709551616")), None);
}
