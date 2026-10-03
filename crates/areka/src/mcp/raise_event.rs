//! `raise_event` のダミーの処理（spec: areka-P0-mcp-tool-entrances）。

use areka_mcp::tools::raise_event::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::ActiveGhost;

/// まだ中身が無い。World・ゴースト・引数は使わず（ゴーストに何もさせず）`NG:` で答える（要件 5.1・5.2）。
pub(super) fn handle(_world: &mut World, _ghost: &ActiveGhost, _args: Args, reply: ReplyTo) {
    reply.send(outcome::ng("not implemented yet"));
}

#[cfg(test)]
#[path = "raise_event_tests.rs"]
mod raise_event_tests;
