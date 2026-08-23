//! `tick_gate_config` の決定論テスト（task 3.3・要件 3.2/6.8）。
//!
//! 見るのは純関数だけで、環境変数そのものは 1 度も読み書きしない（読み書きすると
//! 並列に走る他のテストへ漏れる）。読み解きの表を 1 件ずつ突き合わせる。

use super::*;

/// (要件 3.2) 「有効」と読む綴り——大文字小文字と前後の空白は無視する。
#[test]
fn truthy_spellings_turn_the_gate_on() {
    for value in ["1", "true", "TRUE", "True", "on", "ON", " 1 ", "\ttrue\n"] {
        assert_eq!(
            parse_tick_gate(Some(value)),
            TickGateSetting::Set(true),
            "`{value}` は門を有効にする綴りであるべき"
        );
        assert_eq!(tick_gate_from_env_value(Some(value)), Some(true));
    }
}

/// (要件 3.2) 「無効」と読む綴り。
#[test]
fn falsy_spellings_turn_the_gate_off() {
    for value in ["0", "false", "FALSE", "off", "OFF", " 0 "] {
        assert_eq!(
            parse_tick_gate(Some(value)),
            TickGateSetting::Set(false),
            "`{value}` は門を無効にする綴りであるべき"
        );
        assert_eq!(tick_gate_from_env_value(Some(value)), Some(false));
    }
}

/// (要件 3.2) 未設定・空・空白のみは「指定なし」——既定を動かさない。
#[test]
fn unset_or_blank_leaves_the_default_alone() {
    for value in [None, Some(""), Some("   "), Some("\t\n")] {
        assert_eq!(
            parse_tick_gate(value),
            TickGateSetting::Unset,
            "{value:?} は指定なしとして扱うべき"
        );
        assert_eq!(tick_gate_from_env_value(value), None);
    }
}

/// (要件 3.7) 表に無い綴りは既定のまま——黙って倒さず、呼び出し側が warn! を残す。
#[test]
fn unknown_spellings_are_reported_and_leave_the_default_alone() {
    for value in ["maybe", "2", "yes", "no", "-1", "onn"] {
        assert_eq!(
            parse_tick_gate(Some(value)),
            TickGateSetting::Unknown,
            "`{value}` は表に無い綴りとして報告されるべき"
        );
        assert_eq!(tick_gate_from_env_value(Some(value)), None);
    }
}

/// (要件 7.2) 環境変数の名前は `AREKA_` 冠の固定語（登記と実装が食い違わない）。
#[test]
fn env_name_is_the_registered_one() {
    assert_eq!(TICK_GATE_ENV, "AREKA_TICK_GATE");
}
