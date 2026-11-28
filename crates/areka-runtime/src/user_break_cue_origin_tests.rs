// =============================================================================
// 中断を禁じる区間の受け口が出どころで enter を止める決定論テスト
// =============================================================================
//
// 配送と同じく `clone_box` の直後に `begin_talk` で出どころを渡し、外部扱いのトークでは
// `Enter` を送らず `script_tag_refused` の warn が 1 件だけ残ること、ほかの合図（始まり・
// `Leave`・終わり）は今どおりであること、3 つの `local` の出どころでは今どおり `Enter` が
// 出ることを固定する。

use super::*;
use crate::consumer_ledger::test_support::carrier_cue;
use areka_ghost::sink::BootCueSink;
use areka_sakura::TalkId;
use areka_sakura::contract::{ScriptOrigin, SecurityLevel, SstpKind};
use log_capture_kit::{LineFormat, capture_lines};
use std::sync::mpsc::{Receiver, channel};

const TALK: TalkId = TalkId(42);

/// 配送と同じ手順（複製の直後に 1 回 `begin_talk`）でトークの受け口を作る。
fn talk_sink(origin: ScriptOrigin) -> (Box<dyn BootCueSink>, Receiver<NoUserBreakSignal>) {
    let (tx, rx) = channel::<NoUserBreakSignal>();
    let registered = NoUserBreakCueSink::new(tx);
    let mut talk = registered.clone_box();
    talk.begin_talk(TALK, origin, &Default::default());
    (talk, rx)
}

/// 1 つのトークで enter・leave を流して落とし、届いた合図と記録を返す。
fn run_talk(origin: ScriptOrigin) -> (Vec<NoUserBreakSignal>, Vec<String>) {
    let (mut talk, rx) = talk_sink(origin);
    let ((), logs) = capture_lines(LineFormat::LevelTargetFields, || {
        talk.emit(carrier_cue("enter", &["nouserbreakmode"]));
        talk.emit(carrier_cue("leave", &["nouserbreakmode"]));
        drop(talk);
    });
    (rx.try_iter().collect(), logs)
}

fn refused(logs: &[String]) -> Vec<&String> {
    logs.iter()
        .filter(|l| l.contains("level=WARN") && l.contains("script_tag_refused"))
        .collect()
}

/// 実効のレベルが `external` に下がったトーク。
const LOWERED: ScriptOrigin = ScriptOrigin {
    level: SecurityLevel::External,
    ..ScriptOrigin::GHOST
};

/// Owned でない SSTP（レベルは `local` でも外部扱い）。
const NOT_OWNED_SSTP: ScriptOrigin = ScriptOrigin {
    sstp: SstpKind::NotOwned,
    ..ScriptOrigin::MCP_NOTIFY
};

/// 外部扱いのトークでは `Enter` を送らず warn が 1 件。始まり・`Leave`・終わりは今どおり。
#[test]
fn external_treated_talk_refuses_enter_and_keeps_other_signals() {
    for origin in [LOWERED, NOT_OWNED_SSTP] {
        let (signals, logs) = run_talk(origin);
        assert_eq!(
            signals,
            vec![
                NoUserBreakSignal::TalkStarted,
                NoUserBreakSignal::Leave,
                NoUserBreakSignal::TalkEnded,
            ],
            "外部扱い {origin:?} では Enter だけが出ない"
        );
        let refused = refused(&logs);
        assert_eq!(
            refused.len(),
            1,
            "script_tag_refused の warn は 1 件: {logs:?}"
        );
        let line = refused[0];
        for needle in [
            "enter,nouserbreakmode",
            "talk_id=Some(TalkId(42))",
            &format!("sender_type={} ", origin.sender_type.render()),
            &format!("sstp={:?}", origin.sstp),
            &format!("level=\"{}\"", origin.level.wire()),
        ] {
            assert!(line.contains(needle), "記録に {needle} が無い: {line}");
        }
    }
}

/// ゴースト自身・MCP の SEND・MCP の NOTIFY（いずれも `local`）では今どおり `Enter` が出る。
#[test]
fn local_origins_still_send_enter() {
    for origin in [
        ScriptOrigin::GHOST,
        ScriptOrigin::MCP_SEND,
        ScriptOrigin::MCP_NOTIFY,
    ] {
        let (signals, logs) = run_talk(origin);
        assert_eq!(
            signals,
            vec![
                NoUserBreakSignal::TalkStarted,
                NoUserBreakSignal::Enter,
                NoUserBreakSignal::Leave,
                NoUserBreakSignal::TalkEnded,
            ],
            "{origin:?} では今どおり Enter が出る"
        );
        assert!(refused(&logs).is_empty(), "止めの記録は無い: {logs:?}");
    }
}
