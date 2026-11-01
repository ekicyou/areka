//! 出どころの型（[`SecurityLevel`]・[`SenderType`]・[`ScriptOrigin`]）の検証テスト
//! （Requirements 1.8, 2.7, 3.2, 3.3）。

use super::*;

// ── SenderType::render（2.7） ──

#[test]
fn render_head_only() {
    let t = SenderType {
        head: SenderHead::Internal,
        rest: &[],
    };
    assert_eq!(t.render(), "internal");
}

#[test]
fn render_external_sstp() {
    let t = SenderType {
        head: SenderHead::External,
        rest: &[SenderWord::Sstp],
    };
    assert_eq!(t.render(), "external,sstp");
}

#[test]
fn render_internal_raise() {
    let t = SenderType {
        head: SenderHead::Internal,
        rest: &[SenderWord::Raise],
    };
    assert_eq!(t.render(), "internal,raise");
}

#[test]
fn render_all_rest_words_in_order() {
    // 残りの 7 語の綴りを ukadoc の語で固定する（1.8）。
    let t = SenderType {
        head: SenderHead::External,
        rest: &[
            SenderWord::SakuraApi,
            SenderWord::Embed,
            SenderWord::Raise,
            SenderWord::Property,
            SenderWord::Plugin,
            SenderWord::Sstp,
            SenderWord::Communicate,
        ],
    };
    assert_eq!(
        t.render(),
        "external,sakuraapi,embed,raise,property,plugin,sstp,communicate"
    );
}

// ── SecurityLevel::wire／parse（3.3） ──

#[test]
fn wire_spelling() {
    assert_eq!(SecurityLevel::Local.wire(), "local");
    assert_eq!(SecurityLevel::External.wire(), "external");
}

#[test]
fn parse_ignores_ascii_case() {
    assert_eq!(SecurityLevel::parse("LOCAL"), Some(SecurityLevel::Local));
    assert_eq!(
        SecurityLevel::parse("External"),
        Some(SecurityLevel::External)
    );
    assert_eq!(SecurityLevel::parse("local"), Some(SecurityLevel::Local));
}

#[test]
fn parse_trims_surrounding_whitespace() {
    assert_eq!(SecurityLevel::parse("Local "), Some(SecurityLevel::Local));
    assert_eq!(
        SecurityLevel::parse(" external\t"),
        Some(SecurityLevel::External)
    );
}

#[test]
fn parse_unreadable_is_none() {
    assert_eq!(SecurityLevel::parse("remote"), None);
    assert_eq!(SecurityLevel::parse(""), None);
    assert_eq!(SecurityLevel::parse("local,external"), None);
}

// ── lowered_by の 4 通り（3.2・上がらない） ──

#[test]
fn lowered_by_four_cases() {
    use SecurityLevel::{External, Local};
    assert_eq!(Local.lowered_by(Local), Local);
    assert_eq!(Local.lowered_by(External), External);
    assert_eq!(External.lowered_by(Local), External);
    assert_eq!(External.lowered_by(External), External);
}

#[test]
fn origin_lowered_by_never_raises() {
    let lowered = ScriptOrigin::GHOST.lowered_by(SecurityLevel::External);
    assert_eq!(lowered.level, SecurityLevel::External);
    // 語の組と Owned はそのまま。
    assert_eq!(lowered.sender_type, ScriptOrigin::GHOST.sender_type);
    assert_eq!(lowered.sstp, ScriptOrigin::GHOST.sstp);
    assert_eq!(lowered.lowered_by(SecurityLevel::Local), lowered);
}

// ── 3 定数（1.8） ──

#[test]
fn constants_render_and_level() {
    assert_eq!(ScriptOrigin::GHOST.sender_type.render(), "internal");
    assert_eq!(ScriptOrigin::GHOST.sstp, SstpKind::NotSstp);
    assert_eq!(ScriptOrigin::GHOST.level, SecurityLevel::Local);

    assert_eq!(ScriptOrigin::MCP_SEND.sender_type.render(), "internal");
    assert_eq!(ScriptOrigin::MCP_SEND.sstp, SstpKind::Owned);
    assert_eq!(ScriptOrigin::MCP_SEND.level, SecurityLevel::Local);

    assert_eq!(
        ScriptOrigin::MCP_NOTIFY.sender_type.render(),
        "external,sstp"
    );
    assert_eq!(ScriptOrigin::MCP_NOTIFY.sstp, SstpKind::Owned);
    assert_eq!(ScriptOrigin::MCP_NOTIFY.level, SecurityLevel::Local);
}

// ── 返しのイベントの出どころ（shell-scale-linkage 1.3） ──

#[test]
fn property_constant_renders_internal_property() {
    assert_eq!(
        ScriptOrigin::PROPERTY.sender_type.render(),
        "internal,property"
    );
    assert_eq!(ScriptOrigin::PROPERTY.sstp, SstpKind::NotSstp);
    assert_eq!(ScriptOrigin::PROPERTY.level, SecurityLevel::Local);
}

// ── is_external_treated／is_sstp の真理表 ──

fn not_owned_sstp() -> ScriptOrigin {
    ScriptOrigin {
        sender_type: SenderType {
            head: SenderHead::External,
            rest: &[SenderWord::Sstp],
        },
        sstp: SstpKind::NotOwned,
        level: SecurityLevel::Local,
    }
}

#[test]
fn truth_table_local() {
    // (出どころ, 外部扱い, SSTP 系)
    let rows = [
        (ScriptOrigin::GHOST, false, false),
        (ScriptOrigin::MCP_SEND, false, true),
        (ScriptOrigin::MCP_NOTIFY, false, true),
        (not_owned_sstp(), true, true),
    ];
    for (o, ext, sstp) in rows {
        assert_eq!(o.is_external_treated(), ext, "{o:?}");
        assert_eq!(o.is_sstp(), sstp, "{o:?}");
    }
}

#[test]
fn truth_table_lowered_to_external() {
    // 実効のレベルが External なら、どの出どころも外部扱い。SSTP 系かは変わらない。
    let rows = [
        (ScriptOrigin::GHOST, false),
        (ScriptOrigin::MCP_SEND, true),
        (ScriptOrigin::MCP_NOTIFY, true),
        (not_owned_sstp(), true),
    ];
    for (o, sstp) in rows {
        let o = o.lowered_by(SecurityLevel::External);
        assert!(o.is_external_treated(), "{o:?}");
        assert_eq!(o.is_sstp(), sstp, "{o:?}");
    }
}
