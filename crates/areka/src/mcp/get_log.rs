//! `get_log` のダミーの処理（spec: areka-P0-mcp-tool-entrances）。

use areka_mcp::tools::get_log::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

/// まだ中身が無い。World・引数は使わず（ゴーストに何もさせず）`NG:` で答える（要件 5.1・5.2）。
pub(super) fn handle(_world: &mut World, _args: Args, reply: ReplyTo) {
    reply.send(outcome::ng("not implemented yet"));
}

#[cfg(test)]
#[path = "get_log_tests.rs"]
mod get_log_tests;
