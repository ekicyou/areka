//! `validate_notify` の決定論テスト（空・空白だけ・CR・LF・NUL・バイト値 1 と TAB は通す）。

use super::*;

fn refs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn empty_event_is_rejected() {
    assert_eq!(validate_notify("", &[]), Err(NotifyArgError::EmptyEvent));
}

#[test]
fn whitespace_only_event_is_rejected() {
    assert_eq!(
        validate_notify(" \t　", &[]),
        Err(NotifyArgError::EmptyEvent)
    );
}

#[test]
fn empty_event_wins_over_control_character_in_reference() {
    // 判定は EmptyEvent が先。
    assert_eq!(
        validate_notify("", &refs(&["a\nb"])),
        Err(NotifyArgError::EmptyEvent)
    );
    assert_eq!(
        validate_notify("\r\n", &[]),
        Err(NotifyArgError::EmptyEvent)
    );
}

#[test]
fn cr_lf_nul_in_event_are_rejected() {
    for name in ["On\rTest", "On\nTest", "On\0Test"] {
        assert_eq!(
            validate_notify(name, &[]),
            Err(NotifyArgError::ControlCharacter),
            "{name:?}"
        );
    }
}

#[test]
fn cr_lf_nul_in_reference_are_rejected() {
    for r in ["a\rb", "a\nb", "a\0b"] {
        assert_eq!(
            validate_notify("OnTest", &refs(&["ok", r])),
            Err(NotifyArgError::ControlCharacter),
            "{r:?}"
        );
    }
}

#[test]
fn byte_one_and_tab_pass() {
    assert_eq!(validate_notify("On\u{1}Test", &[]), Ok(()));
    assert_eq!(validate_notify("On\tTest", &[]), Ok(()));
    assert_eq!(
        validate_notify("OnGhostBoot", &refs(&["a\u{1}b\u{1}c", "x\ty", ""])),
        Ok(())
    );
}
