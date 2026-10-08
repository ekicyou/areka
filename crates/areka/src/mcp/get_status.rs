//! `get_status` の処理（spec: areka-P0-mcp-get-status）。
//!
//! 置き場のゴーストの kanade へ今の実行の状態を問い、返事を後から答える口（`super::later`）で
//! 毎フレーム覗く。届いた値は `Status` の語のカンマ連結をそのまま本文にする（無ければ空の本文）。
//! 答える前にゴーストが降りたら、宛先の解決に失敗したときと同じ文言で答える。

use areka_actor::{ReplyError, reply_channel};
use areka_kanade::{ExecutionStatus, KanadeMsg};
use areka_mcp::tools::get_status::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::{self, ActiveGhost, NOT_ACTIVE, Omitted, listed_value};
use crate::ghost_session::GhostSlot;

/// 置き場に問い合わせ先（kanade の送り口）が無いときの本文（areka 独自・本番では起きない）。
const UNAVAILABLE: &str = "Status is not available";

/// 問い合わせを 1 通送り、返事は待たずに `later` へ預ける（要件 1.1・1.3・1.6・3.2〜3.6・4.3・4.4）。
///
/// `ghost` は問い合わせ先が無いときの `warn!` の欄にだけ使う（問い合わせ先は置き場の 1 体から引く）。
/// 成功の枝では記録を出さない（橋が `debug!` を 1 行残す）。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    // 降りたときの文言＝起動中のゴースト無しで解決したときの失敗の理由（`Ok` にはならない）。
    let gone = resolve::resolve(None, args.ghost_name.as_deref(), Omitted::UseActive)
        .err()
        .unwrap_or(NOT_ACTIVE);
    let kanade = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.kanade())
        .cloned();
    let Some(kanade) = kanade else {
        tracing::warn!(
            event = "mcp_get_status_unavailable",
            ghost = listed_value(ghost).as_str(),
            "[mcp] get_status: 宛先のゴーストの kanade の送り口が見つからない"
        );
        reply.send(outcome::ng(UNAVAILABLE));
        return;
    };
    let (tx, rx) = reply_channel::<ExecutionStatus>();
    if kanade.send(KanadeMsg::StatusQuery { reply: tx }).is_err() {
        tracing::debug!(
            event = "mcp_get_status_kanade_gone",
            "[mcp] get_status: kanade が止まっていて問い合わせを送れない"
        );
        reply.send(outcome::ng(gone));
        return;
    }
    super::later(world, reply, move |_| match rx.try_recv() {
        Ok(None) => None,
        Ok(Some(status)) => Some(outcome::value(status.render().unwrap_or_default())),
        // `Timeout` は `try_recv` からは返らない。型の上の腕は「降りた」に倒す。
        Err(ReplyError::Dropped | ReplyError::Timeout) => {
            tracing::debug!(
                event = "mcp_get_status_reply_dropped",
                "[mcp] get_status: 返事の前に kanade の返信端が落ちた"
            );
            Some(outcome::ng(gone))
        }
    });
}

#[cfg(test)]
#[path = "get_status_tests.rs"]
mod get_status_tests;
