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

pub(super) fn answer(pending: &Pending) -> ToolOutcome {
    pending
        .try_answer()
        .ok()
        .flatten()
        .expect("答えが届いている")
        .outcome
}

/// 答えの本文 1 つ（本文が 1 つの文字列でなければ落ちる）。
pub(super) fn body(outcome: &ToolOutcome) -> &str {
    match outcome.content.as_slice() {
        [ToolContent::Text(text)] => text,
        other => panic!("本文は文字列 1 つのはず: {other:?}"),
    }
}

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
