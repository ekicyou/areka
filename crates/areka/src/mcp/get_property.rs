//! `get_property` の処理（spec: areka-P0-mcp-get-property）。
//!
//! 置き場の実行系の記憶の読み手に、そのゴースト自身の問い手として名前を聞き、
//! 値なら素のまま、無ければ `NG:Cannot find such property name.` で答える。
//! 実行系が無いときは `warn!` を 1 件残して `NG:Property system is not available` で答える。

use areka_ghost::sylphya_wiring::ghost_asker_id;
use areka_mcp::tools::get_property::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use areka_sylphya::{AskerContext, DottedResolution};
use bevy_ecs::world::World;

use super::resolve::{ActiveGhost, listed_value};
use crate::ghost_session::GhostSlot;

/// 値が無い・名前の書式が読めないときの本文（SSP と同じ）。
const NOT_FOUND: &str = "Cannot find such property name.";
/// 置き場に実行系が無いときの本文（areka 独自）。
const UNAVAILABLE: &str = "Property system is not available";

/// 置き場の実行系から読み、返る前にちょうど 1 回答える（要件 1.1〜1.6・2.1〜2.3・3.1〜3.3）。
///
/// 名前は手直しせず読み手へ渡す。`ghost` は実行系が無いときの `warn!` の欄にだけ使う
/// （実行系は置き場の 1 つから引く）。待たず、普通の答えでは記録を出さない。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    let runtime = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.runtime());
    let Some(runtime) = runtime else {
        tracing::warn!(
            event = "mcp_get_property_unavailable",
            ghost = listed_value(ghost).as_str(),
            property_name = args.property_name.as_str(),
            "[mcp] get_property: 宛先のゴーストの実行系が見つからない"
        );
        reply.send(outcome::ng(UNAVAILABLE));
        return;
    };
    let asker = AskerContext {
        asker: ghost_asker_id(&runtime.mount().shiori.dir),
    };
    let outcome = match runtime
        .sylphya_reader()
        .resolve_dotted_str(&asker, &args.property_name)
    {
        DottedResolution::Value(value) => outcome::value(value),
        DottedResolution::NotFound => outcome::ng(NOT_FOUND),
    };
    reply.send(outcome);
}

#[cfg(test)]
#[path = "get_property_tests.rs"]
mod get_property_tests;
