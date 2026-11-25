//! `get_status` の処理。
//!
//! 置き場のゴーストの kanade へ今の実行の状態を問い、返事を後から答える口（`super::later`）で
//! 毎フレーム覗く。届いた値は `Status` の語のカンマ連結をそのまま本文にする（無ければ空の本文）。
//! 答える前にゴーストが降りたら、宛先の解決に失敗したときと同じ文言で答える。

use areka_kanade::{ExecutionStatus, KanadeMsg};
use areka_mcp::tools::get_status::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::{ActiveGhost, KanadeAsk, ask_kanade};

/// 置き場に問い合わせ先（kanade の送り口）が無いときの本文（areka 独自・本番では起きない）。
const UNAVAILABLE: &str = "Status is not available";

const ASK: KanadeAsk = KanadeAsk {
    tool: "get_status",
    what: "問い合わせ",
    unavailable: UNAVAILABLE,
    unavailable_event: "mcp_get_status_unavailable",
    gone_event: "mcp_get_status_kanade_gone",
    dropped_event: "mcp_get_status_reply_dropped",
};

/// 問い合わせを 1 通送り、返事は待たずに `later` へ預ける。
///
/// `ghost` は問い合わせ先が無いときの `warn!` の欄にだけ使う（問い合わせ先は置き場の 1 体から引く）。
/// 成功の枝では記録を出さない（橋が `debug!` を 1 行残す）。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    ask_kanade(
        world,
        ghost,
        args.ghost_name.as_deref(),
        ASK,
        reply,
        |reply| KanadeMsg::StatusQuery { reply },
        |status: ExecutionStatus, _| outcome::value(status.render().unwrap_or_default()),
    );
}

#[cfg(test)]
#[path = "get_status_tests.rs"]
mod get_status_tests;
