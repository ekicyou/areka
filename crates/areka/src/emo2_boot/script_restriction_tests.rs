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

// ---------------------------------------------------------------- 実装との照合（要件 4.6・4.7）
//
// 名簿（`consumer_ledger.rs`）は読むだけ（先例 `consumer_ledger_agreement_tests.rs`）。

use super::super::consumer_ledger::{CommandConsumer, ConsumerLedger};
use super::super::user_break_cue::{NoUserBreakCueSink, NoUserBreakSignal};
use areka_ghost::sink::BootCueSink;
use areka_parsers::sakura::{Instruction, parse};
use areka_sakura::TalkId;
use dola::cue::{ActorKey, CueCommand, TalkCue};
use std::sync::mpsc::channel;

/// 外部扱いの出どころ（実効のレベルが `external`）。
const EXTERNAL: ScriptOrigin = ScriptOrigin {
    level: SecurityLevel::External,
    ..ScriptOrigin::GHOST
};

/// 台本の読みが `\m[umsg,…]` を解釈するか（未解釈の `Raw` 1 つなら未実装）。
fn umsg_is_interpreted() -> bool {
    !matches!(parse(r"\m[umsg,0,0]").as_slice(), [Instruction::Raw(_)])
}

/// 表の旗が実装の有無と一致する。`\!` の行は名簿の担当の有無、`\m[…]` の行は台本の読み（4.6・4.7）。
#[test]
fn enforced_flag_agrees_with_ledger_and_parser() {
    let ledger = ConsumerLedger::canonical();
    for row in &RESTRICTIONS {
        let implemented = match row.tag {
            RestrictedTag::Command { name, selector } => {
                ledger.consumer_of(name, Some(selector)).is_some()
            }
            RestrictedTag::Umsg => umsg_is_interpreted(),
        };
        assert_eq!(
            implemented, row.enforced,
            "{}: 表の実装済みの旗が実装と食い違う。表の旗か受け口の止めを直す",
            row.display
        );
    }
}

/// 担当の受け口（配送と同じく複製したもの）と、そのタグの合図が出たかを見る手。
type Probe = (Box<dyn BootCueSink>, Box<dyn Fn() -> bool>);

/// 名簿の担当から受け口を作る。既定の腕は置かない——担当の種類が増えればコンパイルが止まり、
/// 表の行を実装済みにした担当に受け口の照合が無ければここで赤になる。
fn probe_for(consumer: CommandConsumer) -> Probe {
    match consumer {
        CommandConsumer::UserBreakSink => {
            let (tx, rx) = channel();
            let talk = NoUserBreakCueSink::new(tx).clone_box();
            let came_out = move || {
                rx.try_iter()
                    .any(|s| matches!(s, NoUserBreakSignal::Enter | NoUserBreakSignal::Leave))
            };
            (talk, Box::new(came_out))
        }
        CommandConsumer::MoveSink
        | CommandConsumer::Seriko
        | CommandConsumer::ZOrderSink
        | CommandConsumer::ReadmeSink
        | CommandConsumer::TextLayer
        | CommandConsumer::ChangeSink
        | CommandConsumer::SwitchSink
        | CommandConsumer::InstallSink
        | CommandConsumer::UpdateSink
        | CommandConsumer::ScriptCompile
        | CommandConsumer::PropertyWrite
        | CommandConsumer::LifecycleSink
        | CommandConsumer::ReloadSink
        | CommandConsumer::WindowStateSink => {
            panic!("{consumer:?} が担当する行が実装済みになった。ここに受け口と合図の見方を足す")
        }
    }
}

/// 1 つのトークでタグを流し、合図が出たかを返す（配送と同じく複製の直後に `begin_talk`）。
fn signal_came_out(
    consumer: CommandConsumer,
    name: &str,
    selector: &str,
    origin: ScriptOrigin,
) -> bool {
    let (mut talk, came_out) = probe_for(consumer);
    talk.begin_talk(TalkId(7), origin);
    talk.emit(TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(name, vec![selector.to_string()]),
        duration: 0.0,
    });
    drop(talk);
    came_out()
}

/// 旗だけで済ませない：実装済みの行ごとに担当の受け口へ外部扱いの出どころでタグを流し、
/// 合図が出ないことを確かめる。ゴースト自身では出ることも見て、照合が空振りでないことを保つ（4.7）。
#[test]
fn enforced_rows_are_refused_by_their_sink_for_external_origin() {
    let ledger = ConsumerLedger::canonical();
    let mut checked = 0;
    for row in RESTRICTIONS.iter().filter(|row| row.enforced) {
        let RestrictedTag::Command { name, selector } = row.tag else {
            panic!(r"{}: `\m[…]` の行の受け口の照合が無い", row.display);
        };
        let consumer = ledger
            .consumer_of(name, Some(selector))
            .unwrap_or_else(|| panic!("{}: 実装済みなのに名簿に担当が無い", row.display));
        assert!(
            !signal_came_out(consumer, name, selector, EXTERNAL),
            "{}: 外部扱いの出どころなのに合図が出た（受け口の止めが無い）",
            row.display
        );
        assert!(
            signal_came_out(consumer, name, selector, ScriptOrigin::GHOST),
            "{}: ゴースト自身でも合図が出ない（照合が空振り）",
            row.display
        );
        checked += 1;
    }
    assert!(checked > 0, "実装済みの行が 1 つも無い");
}
