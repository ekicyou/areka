use super::*;
use crate::config::{BalloonSide, BalloonXMode};

fn off(args: &[&str]) -> Result<ScriptOffset, OffsetArgError> {
    parse_balloon_offset(args)
}

fn reason(args: &[&str]) -> OffsetArgReason {
    off(args).expect_err("読めない値").reason
}

#[test]
fn absolute_both_axes() {
    assert_eq!(
        off(&["100", "0"]),
        Ok(ScriptOffset {
            x: OffsetAxis::Absolute(100),
            y: OffsetAxis::Absolute(0)
        })
    );
}

#[test]
fn mixed_relative_and_absolute_per_axis() {
    assert_eq!(
        off(&["@-10", "200"]),
        Ok(ScriptOffset {
            x: OffsetAxis::Relative(-10),
            y: OffsetAxis::Absolute(200)
        })
    );
}

#[test]
fn relative_both_axes() {
    assert_eq!(
        off(&["@0", "@0"]),
        Ok(ScriptOffset {
            x: OffsetAxis::Relative(0),
            y: OffsetAxis::Relative(0)
        })
    );
}

#[test]
fn plus_sign_and_surrounding_spaces() {
    assert_eq!(
        off(&[" +5 ", " -5"]),
        Ok(ScriptOffset {
            x: OffsetAxis::Absolute(5),
            y: OffsetAxis::Absolute(-5)
        })
    );
}

#[test]
fn at_without_number_rejects_both_axes() {
    let e = off(&["@", "abc"]).unwrap_err();
    assert_eq!(e.reason, OffsetArgReason::AtWithoutNumber);
    assert_eq!(e.raw, vec!["@".to_string(), "abc".to_string()]);
    assert_eq!(reason(&["@0", "@"]), OffsetArgReason::AtWithoutNumber);
}

#[test]
fn not_a_number_rejects_both_axes() {
    assert_eq!(reason(&["abc", "0"]), OffsetArgReason::NotANumber);
    assert_eq!(reason(&["0", "1.5"]), OffsetArgReason::NotANumber);
    assert_eq!(reason(&["@x", "0"]), OffsetArgReason::NotANumber);
    assert_eq!(reason(&["", "0"]), OffsetArgReason::NotANumber);
}

#[test]
fn missing_axis() {
    assert_eq!(reason(&["10"]), OffsetArgReason::MissingAxis);
    assert_eq!(reason(&[]), OffsetArgReason::MissingAxis);
}

#[test]
fn beyond_i32_is_not_a_number() {
    assert_eq!(reason(&["2147483648", "0"]), OffsetArgReason::NotANumber);
    assert_eq!(reason(&["0", "@-2147483649"]), OffsetArgReason::NotANumber);
    assert!(off(&["2147483647", "-2147483648"]).is_ok());
}

#[test]
fn extra_args_are_ignored() {
    assert_eq!(off(&["1", "2", "3", "zzz"]), off(&["1", "2"]));
}

#[test]
fn align_words_case_insensitive() {
    assert_eq!(parse_balloon_align(&["RIGHT"]), Some(ScriptAlign::Right));
    assert_eq!(parse_balloon_align(&["Center"]), Some(ScriptAlign::Center));
    assert_eq!(parse_balloon_align(&[" left "]), Some(ScriptAlign::Left));
    assert_eq!(parse_balloon_align(&["Bottom"]), Some(ScriptAlign::Bottom));
    assert_eq!(parse_balloon_align(&["NONE"]), Some(ScriptAlign::None));
    assert_eq!(parse_balloon_align(&["top"]), Some(ScriptAlign::Top));
}

#[test]
fn align_unknown_carries_received_text() {
    assert_eq!(
        parse_balloon_align(&["foo"]),
        Some(ScriptAlign::Unknown("foo".to_string()))
    );
    assert_eq!(
        parse_balloon_align(&["Foo", "left"]),
        Some(ScriptAlign::Unknown("Foo".to_string()))
    );
}

#[test]
fn align_without_id_is_none() {
    assert_eq!(parse_balloon_align(&[]), None);
    assert_eq!(parse_balloon_align(&[""]), None);
    assert_eq!(parse_balloon_align(&["  "]), None);
}

#[test]
fn align_maps_to_side_and_mode() {
    use BalloonSide::{Auto, Left, Right};
    use BalloonXMode::{CenterBottom, CenterTop, Side};
    assert_eq!(ScriptAlign::Left.side_and_mode(), (Some(Left), Side));
    assert_eq!(ScriptAlign::Right.side_and_mode(), (Some(Right), Side));
    assert_eq!(ScriptAlign::Center.side_and_mode(), (None, CenterTop));
    assert_eq!(ScriptAlign::Top.side_and_mode(), (None, CenterTop));
    assert_eq!(ScriptAlign::Bottom.side_and_mode(), (None, CenterBottom));
    assert_eq!(ScriptAlign::None.side_and_mode(), (Some(Auto), Side));
    assert_eq!(
        ScriptAlign::Unknown("foo".into()).side_and_mode(),
        (Some(Auto), Side)
    );
}
