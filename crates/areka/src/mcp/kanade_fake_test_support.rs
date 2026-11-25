//! 偽の kanade を据えたテストの足場（MCP のツールの決定論テストが共有する。起動中の 1 体の値 [`ghost`] も置く）。

use std::path::PathBuf;
use std::sync::mpsc;

use areka_kanade::KanadeMsg;
use areka_mcp::tools::Pending;
use areka_mcp::{ToolContent, ToolOutcome};
use areka_session::ghost_session::{GhostSession, GhostSlot};
use bevy_ecs::world::World;
use log_capture_kit::CapturedEvent;

use super::install;
use super::resolve::ActiveGhost;

pub(super) fn ghost() -> ActiveGhost {
    ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    }
}

/// 受け口と後から答える置き場を据え、置き場に偽の kanade の送り口を置いた World と、その受信端。
pub(super) fn rig() -> (World, mpsc::Receiver<KanadeMsg>) {
    let mut world = World::new();
    let (_inbox_tx, inbox_rx) = mpsc::channel();
    install(&mut world, inbox_rx);
    let (tx, rx) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        ghost().root,
    ))));
    (world, rx)
}

/// 届いていれば答え（まだ・落ちたなら None）。
pub(super) fn try_outcome(pending: &Pending) -> Option<ToolOutcome> {
    pending.try_answer().ok().flatten().map(|a| a.outcome)
}

pub(super) fn answer(pending: &Pending) -> ToolOutcome {
    try_outcome(pending).expect("答えが届いている")
}

/// 答えの本文 1 つ（本文が 1 つの文字列でなければ落ちる）。
pub(super) fn body(outcome: &ToolOutcome) -> &str {
    match outcome.content.as_slice() {
        [ToolContent::Text(text)] => text,
        other => panic!("本文は文字列 1 つのはず: {other:?}"),
    }
}

/// 答えの（本文, isError）。
pub(super) fn body_and_error(outcome: &ToolOutcome) -> (String, bool) {
    (body(outcome).to_owned(), outcome.is_error)
}

/// `get_log` の本文のうち `#<番号> ` で始まる行の番号（古い順）。0 件の本文は空の列。
pub(super) fn ids(body: &str) -> Vec<u64> {
    body.split("\r\n")
        .filter_map(|line| line.strip_prefix('#')?.split(' ').next()?.parse().ok())
        .collect()
}

/// strict 真の答えに足す文の `<n>` の前まで（`OK:` や前の空白は付けない）。
pub(super) const STRICT_NOTE_BEFORE_ID: &str = "strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=";

pub(super) fn warn_or_worse(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|ev| ev.level <= tracing::Level::WARN)
        .count()
}

pub(super) fn text(s: &str) -> Vec<ToolContent> {
    vec![ToolContent::Text(s.to_string())]
}

/// 記録を 1 件積み、その番号を返す。
pub(super) fn record_one() -> u64 {
    areka_session::log_history::record(
        areka_session::log_history::Stamp {
            year: 2026,
            month: 10,
            day: 30,
            hour: 12,
            minute: 0,
        },
        areka_session::log_history::Draft {
            kind: areka_session::log_history::Kind::Error,
            label: "strict".to_string(),
            name: String::new(),
            body: "test".to_string(),
        },
    )
}
