//! `raise_event` の処理（spec: areka-P0-mcp-kanade-tools）。
//!
//! 引数を検査し、置き場のゴーストの kanade へ外から頼まれたイベント（`SstpNotify`）を送り、返事を
//! 後から答える口（`super::later`）で毎フレーム覗く。宛先と文言の部品は `super::sakurascript` のもの。

use areka_actor::{ReplyError, reply_channel};
use areka_kanade::KanadeMsg;
use areka_kanade::sstp::{NotifyArgError, SstpNotifyOutcome, validate_notify};
use areka_mcp::tools::raise_event::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::{ActiveGhost, listed_value};
use super::sakurascript::{NOT_NOW, ScriptTarget, UNAVAILABLE, script_target};

/// 台本が返ったときの 1 行目（SSP の実測の文言・要件 2.3）。
const SCRIPT_RETURNED: &str = "OK:the ghost returned a script and it is being played.";

/// 1 行目と `Script: ` の行の区切り（SSP 2.9.05 の生の本文で確かめた字・要件 2.3）。
const SCRIPT_LINE_BREAK: &str = "\r\n";

/// 台本が返らなかったときの付言（要件 2.4）。
const NO_SCRIPT: &str = "the ghost returned no script.";

/// イベントを 1 通送り、返事は待たずに `later` へ預ける（要件 2.1・2.3〜2.9・3.5・3.6・3.9・5.1）。
///
/// `ghost` は送り口が無いときの `warn!` の欄にだけ使う。成功の枝では記録を出さない（橋が `debug!` を残す）。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    // 検査を破ったものは SHIORI を呼ばない（kanade へ送らない・要件 2.6・2.7）。
    if let Err(e) = validate_notify(&args.event, &args.references) {
        reply.send(outcome::ng(match e {
            NotifyArgError::EmptyEvent => "event is empty",
            NotifyArgError::ControlCharacter => "event or reference contains a control character",
        }));
        return;
    }
    let ScriptTarget { kanade, gone } = script_target(world, args.ghost_name.as_deref());
    let Some(kanade) = kanade else {
        tracing::warn!(
            event = "mcp_raise_event_unavailable",
            ghost = listed_value(ghost).as_str(),
            "[mcp] raise_event: 宛先のゴーストの kanade の送り口が見つからない"
        );
        reply.send(outcome::ng(UNAVAILABLE));
        return;
    };
    let (tx, rx) = reply_channel::<SstpNotifyOutcome>();
    let msg = KanadeMsg::SstpNotify {
        event: args.event,
        references: args.references,
        strict: args.strict,
        reply: tx,
    };
    if kanade.send(msg).is_err() {
        tracing::debug!(
            event = "mcp_raise_event_kanade_gone",
            "[mcp] raise_event: kanade が止まっていてイベントを送れない"
        );
        reply.send(outcome::ng(gone));
        return;
    }
    super::later(world, reply, move |_| match rx.try_recv() {
        Ok(None) => None,
        Ok(Some(SstpNotifyOutcome::Script(script))) => Some(outcome::value(format!(
            "{SCRIPT_RETURNED}{SCRIPT_LINE_BREAK}Script: {script}"
        ))),
        Ok(Some(SstpNotifyOutcome::NoScript)) => Some(outcome::ok(NO_SCRIPT)),
        Ok(Some(SstpNotifyOutcome::NotAccepted)) => {
            tracing::debug!(
                event = "mcp_raise_event_not_accepted",
                "[mcp] raise_event: 手順の途中でイベントを受け付けなかった"
            );
            Some(outcome::ng(NOT_NOW))
        }
        Ok(Some(SstpNotifyOutcome::Failed)) => {
            tracing::debug!(
                event = "mcp_raise_event_failed",
                "[mcp] raise_event: イベントの往復が失敗したか kanade が止まった"
            );
            Some(outcome::ng(gone))
        }
        // `Timeout` は `try_recv` からは返らない。型の上の腕は「降りた」に倒す。
        Err(ReplyError::Dropped | ReplyError::Timeout) => {
            tracing::debug!(
                event = "mcp_raise_event_reply_dropped",
                "[mcp] raise_event: 返事の前に kanade の返信端が落ちた"
            );
            Some(outcome::ng(gone))
        }
    });
}

#[cfg(test)]
#[path = "raise_event_tests.rs"]
mod raise_event_tests;
