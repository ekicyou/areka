//! 判定の表の固定（要件 4.8）と許すかの真理表（要件 4.1）。

use super::*;
use areka_sakura::contract::{SecurityLevel, SstpKind};

/// 要件 4.1 の 5 つの綴り（並びも表と同じ）。
const EXPECTED: [&str; 5] = [
    r"\![execute,filewatch,…]",
    r"\![execute,schedule-add,…]",
    r"\![execute,schedule-delete,…]",
    r"\![enter,nouserbreakmode]",
    r"\m[umsg,wparam,lparam]",
];

#[test]
fn table_has_exactly_the_five_tags_of_requirement_4_1() {
    let actual: Vec<&str> = RESTRICTIONS.iter().map(|row| row.display).collect();
    assert_eq!(
        actual, EXPECTED,
        "判定の表が要件 4.1 の 5 つのタグと食い違う。ukadoc（list_sakura_script）を引き直して表を直す"
    );
}

#[test]
fn only_enter_nouserbreakmode_is_enforced_and_leave_is_not_listed() {
    let enforced: Vec<&str> = RESTRICTIONS
        .iter()
        .filter(|row| row.enforced)
        .map(|row| row.display)
        .collect();
    assert_eq!(enforced, [r"\![enter,nouserbreakmode]"]);
    assert!(command_row("leave", "nouserbreakmode").is_none());
    for row in &RESTRICTIONS {
        assert!(
            row.source
                .starts_with("https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#"),
            "{}",
            row.display
        );
        assert!(!row.note.is_empty(), "{}", row.display);
    }
}

#[test]
fn command_row_finds_each_bang_row_by_name_and_selector() {
    for (name, selector, display) in [
        ("execute", "filewatch", EXPECTED[0]),
        ("execute", "schedule-add", EXPECTED[1]),
        ("execute", "schedule-delete", EXPECTED[2]),
        ("enter", "nouserbreakmode", EXPECTED[3]),
    ] {
        assert_eq!(
            command_row(name, selector).map(|r| r.display),
            Some(display)
        );
    }
    assert!(command_row("execute", "http-get").is_none());
    assert!(command_row("umsg", "").is_none());
}

fn origin(sstp: SstpKind, level: SecurityLevel) -> ScriptOrigin {
    ScriptOrigin {
        sstp,
        level,
        ..ScriptOrigin::GHOST
    }
}

#[test]
fn permits_truth_table() {
    let external = &RESTRICTIONS[3];
    let sstp_only = &RESTRICTIONS[4];
    // (出どころ, 外部扱いで止める行を許す, SSTP 専用の行を許す)
    let cases = [
        (ScriptOrigin::GHOST, true, false),
        (ScriptOrigin::MCP_SEND, true, true),
        (ScriptOrigin::MCP_NOTIFY, true, true),
        (
            origin(SstpKind::NotSstp, SecurityLevel::External),
            false,
            false,
        ),
        (
            origin(SstpKind::Owned, SecurityLevel::External),
            false,
            true,
        ),
        (
            origin(SstpKind::NotOwned, SecurityLevel::Local),
            false,
            true,
        ),
        (
            origin(SstpKind::NotOwned, SecurityLevel::External),
            false,
            true,
        ),
    ];
    for (o, want_external, want_sstp) in cases {
        assert_eq!(permits(external, &o), want_external, "{o:?}");
        assert_eq!(permits(sstp_only, &o), want_sstp, "{o:?}");
    }
}
