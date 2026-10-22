//! 共通の手続きの純粋な関数の試験（子を起こさない・常時のテストで走る）。

use super::*;

const MONITORS: &str = "\
2026-10-22T13:18:15.900000Z DEBUG wintf::ecs::layout::systems::monitor_systems: [initialize_layout_root] Creating Monitor entity handle=111 bounds_left=-1920 bounds_top=0 bounds_right=0 bounds_bottom=1080 work_area=-1920,0,0,1040 dpi=96 is_primary=false
2026-10-22T13:18:15.900100Z DEBUG wintf::ecs::layout::systems::monitor_systems: [initialize_layout_root] Creating Monitor entity handle=222 bounds_left=0 bounds_top=0 bounds_right=3840 bounds_bottom=2160 work_area=0,0,3840,2100 dpi=192 is_primary=true
";

#[test]
fn primary_dpi_picks_the_primary_monitor_line() {
    assert_eq!(primary_dpi(MONITORS), Some(192));
}

#[test]
fn primary_dpi_is_none_without_the_line() {
    assert_eq!(
        primary_dpi("2026-10-22T13:18:15.900000Z  INFO areka: started\n"),
        None
    );
    // 主でない行だけなら取らない。
    let only_other = MONITORS.lines().next().unwrap();
    assert_eq!(primary_dpi(only_other), None);
}

const TALK_THEN_WRAP: &str = "\
2026-10-22T13:18:16.000100Z  INFO kanade: talk event=\"boot_talk\"
2026-10-22T13:18:16.310100Z  INFO areka::emo2_boot: text surface attached wrap=BudouxWordWrap
";

#[test]
fn log_gap_ms_is_signed_difference_of_line_times() {
    assert_eq!(log_gap_ms(TALK_THEN_WRAP, BOOT_TALK, WRAP), Some(310));
    assert_eq!(log_gap_ms(TALK_THEN_WRAP, WRAP, BOOT_TALK), Some(-310));
}

#[test]
fn log_gap_ms_is_none_when_a_line_is_missing_or_time_unreadable() {
    assert_eq!(log_gap_ms(TALK_THEN_WRAP, ATTACH, WRAP), None);
    let no_time = "no time here event=\"boot_talk\"\n2026-10-22T13:18:16.310100Z  INFO x: wrap=BudouxWordWrap\n";
    assert_eq!(log_gap_ms(no_time, BOOT_TALK, WRAP), None);
}

#[test]
fn is_load_returned_false_only_for_the_load_returned_false_line() {
    assert!(is_load_returned_false(
        "[helper] LOAD 失敗（観測・ack[0]）: LoadReturnedFalse { last_error: 5 }\n"
    ));
    assert!(!is_load_returned_false(
        "[helper] LOAD 失敗（観測・ack[0]）: LoadLibraryFailed(Error { code: HRESULT(0x8007007E) })\n"
    ));
    assert!(!is_load_returned_false(
        "2026-10-22T13:18:16.000100Z ERROR areka: shiori event=\"connect_failed\"\n"
    ));
}

#[test]
fn condition_parse_reads_the_four_conditions() {
    assert_eq!(Condition::parse(""), Ok(Condition::Basic));
    assert_eq!(Condition::parse("  "), Ok(Condition::Basic));
    assert_eq!(Condition::parse("gap"), Ok(Condition::Gap));
    assert_eq!(Condition::parse("copy-wait"), Ok(Condition::CopyWait));
    assert_eq!(Condition::parse("no-mcp"), Ok(Condition::NoMcp));
}

#[test]
fn condition_parse_rejects_unknown_values_with_a_reason() {
    let err = Condition::parse("nap").unwrap_err();
    assert!(err.contains("nap"), "理由の文に値が無い: {err}");
    assert!(
        err.contains("copy-wait"),
        "理由の文に受ける値の一覧が無い: {err}"
    );
}

#[test]
fn times_line_is_ascii_millis_and_none() {
    let times = Times {
        wire: Duration::from_micros(812_900),
        attach: Duration::from_millis(1540),
        wrap: Duration::from_millis(6260),
        close: Duration::from_millis(22104),
        talk_to_wrap: Some(Duration::from_millis(310)),
        attach_to_talk_ms: Some(-4410),
        primary_dpi: None,
    };
    let line = times.line("emo2");
    assert_eq!(
        line,
        "emo2: wire_ms=812 attach_ms=1540 wrap_ms=6260 close_ms=22104 talk_to_wrap_ms=310 attach_to_talk_ms=-4410 primary_dpi=none"
    );
    assert!(line.is_ascii());
}

#[test]
fn times_line_writes_none_for_every_missing_value() {
    let times = Times {
        wire: Duration::ZERO,
        attach: Duration::ZERO,
        wrap: Duration::ZERO,
        close: Duration::ZERO,
        talk_to_wrap: None,
        attach_to_talk_ms: None,
        primary_dpi: Some(96),
    };
    assert!(
        times
            .line("x")
            .ends_with("talk_to_wrap_ms=none attach_to_talk_ms=none primary_dpi=96")
    );
}

fn status(code: u32) -> std::process::ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(code)
}

#[test]
fn keep_calling_continues_on_any_reply_including_ng() {
    let ng = ToolReply {
        text: "NG:not ready".into(),
        is_error: true,
    };
    assert!(keep_calling(Ok(ng)).unwrap());
}

#[test]
fn keep_calling_stops_when_the_child_exited() {
    let exited = McpError::ChildExited(real_run_kit::ChildExited { status: status(1) });
    assert!(!keep_calling(Err(exited)).unwrap());
}

#[test]
fn keep_calling_fails_only_when_unreachable_or_not_listening() {
    let unreachable = McpError::Unreachable {
        target: "127.0.0.1:9821".into(),
        reason: "refused".into(),
    };
    assert!(keep_calling(Err(unreachable)).is_err());
    let listen = McpError::ListenFailed {
        port: 9821,
        line: "MCP: どの候補のポートも束ねられなかった（待ち受けない）".into(),
    };
    assert!(keep_calling(Err(listen)).is_err());
}

#[test]
fn ending_is_ok_only_for_user_close_with_exit_zero() {
    let ended = |code, by| Ended {
        status: status(code),
        by,
    };
    assert_eq!(ending_failure(&ended(0, EndedBy::UserClose)), None);
    assert!(
        ending_failure(&ended(1, EndedBy::UserClose))
            .unwrap()
            .contains("終了コード Some(1)")
    );
    // 保険の自動終了は終了コード 0 でも赤で、その旨を書く。
    assert!(
        ending_failure(&ended(0, EndedBy::SafetyExit))
            .unwrap()
            .contains("保険の自動終了（45 秒）")
    );
    assert!(ending_failure(&ended(0, EndedBy::Unlogged)).is_some());
    assert!(ending_failure(&ended(0, EndedBy::Other("Smoke2".into()))).is_some());
}
