//! `reload` の処理。読み直しの終わりは待たず、その場で答える。

use areka_mcp::ToolOutcome;
use areka_mcp::tools::reload::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::{self, ActiveGhost};
use crate::log_history::{self, TARGET_ERROR};
use crate::session::reload::{ReloadOrigin, ReloadRequest, ReloadVerdict, request_reload};
use areka_runtime::reload_args::{DescriptNotes, McpTarget, ReloadTarget};

/// 知らない `target` の理由（SSP の実測の文）。
const UNKNOWN_TARGET: &str =
    "Unknown target (ghost / shiori / shell / balloon / makoto / descript)";
const MAKOTO_UNSUPPORTED: &str = "Reloading makoto is not supported yet";
const CANNOT_RELOAD: &str = "Cannot reload now (another switch, reload or update is in progress)";

/// 入口。答えがあればその場で送り、終了の途中（`None`）は送らずに落とす（橋が既存の
/// "shutting down" で答える）。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    let request = |target| {
        request_reload(
            world,
            ReloadRequest {
                target,
                origin: ReloadOrigin::Mcp,
                notes: DescriptNotes::default(),
            },
        )
    };
    if let Some(answer) = answer(&args.target, ghost, request, log_history::last_id) {
        reply.send(answer);
    }
}

/// 答え。`request` は窓口へ渡す口（5 語のときだけ 1 度呼ぶ）、`last_id` は受理の直後に 1 度だけ読む
/// 記録の最後の番号（`since_id`）。
fn answer(
    target: &str,
    ghost: &ActiveGhost,
    request: impl FnOnce(ReloadTarget) -> ReloadVerdict,
    last_id: impl FnOnce() -> u64,
) -> Option<ToolOutcome> {
    match McpTarget::parse(target) {
        McpTarget::Unknown => Some(outcome::ng(UNKNOWN_TARGET)),
        McpTarget::Makoto => {
            tracing::warn!(
                target: TARGET_ERROR,
                ghost = %resolve::listed_value(ghost),
                event = "reload_makoto_unsupported",
                "makoto の読み直しはまだ無い"
            );
            Some(outcome::ng(MAKOTO_UNSUPPORTED))
        }
        McpTarget::Target(target) => match request(target) {
            ReloadVerdict::Accepted => Some(outcome::ok(&format!(
                "reload requested. It runs asynchronously; errors while loading (dictionary errors etc.) will be recorded to the error log. Wait until get_status answers again, then check get_log tool (log_type=error, since_id={})",
                last_id()
            ))),
            ReloadVerdict::Refused(_) => Some(outcome::ng(CANNOT_RELOAD)),
            ReloadVerdict::Closing => None,
        },
    }
}

#[cfg(test)]
#[path = "reload_tests.rs"]
mod reload_tests;
