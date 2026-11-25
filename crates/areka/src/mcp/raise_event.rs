//! `raise_event` の処理。
//!
//! 引数を検査し、置き場のゴーストの kanade へ外から頼まれたイベント（`SstpNotify`）を送り、返事を
//! 後から答える口（`super::later`）で毎フレーム覗く。宛先は `super::resolve`、文言の部品は `super::sakurascript` のもの。

use areka_kanade::KanadeMsg;
use areka_kanade::sstp::{NotifyArgError, SstpNotifyOutcome, validate_notify};
use areka_mcp::tools::raise_event::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::{ActiveGhost, KanadeAsk, ask_kanade};
use super::sakurascript::{NOT_NOW, UNAVAILABLE, since_id_if_strict, strict_note};

/// 台本が返ったときの 1 行目（SSP の実測の文言）。
const SCRIPT_RETURNED: &str = "OK:the ghost returned a script and it is being played.";

/// 1 行目と `Script: ` の行の区切り（SSP 2.9.05 の生の本文で確かめた字）。
const SCRIPT_LINE_BREAK: &str = "\r\n";

/// 台本が返らなかったときの付言。
const NO_SCRIPT: &str = "the ghost returned no script.";

const ASK: KanadeAsk = KanadeAsk {
    tool: "raise_event",
    what: "イベント",
    unavailable: UNAVAILABLE,
    unavailable_event: "mcp_raise_event_unavailable",
    gone_event: "mcp_raise_event_kanade_gone",
    dropped_event: "mcp_raise_event_reply_dropped",
};

/// イベントを 1 通送り、返事は待たずに `later` へ預ける。
///
/// `ghost` は送り口が無いときの `warn!` の欄にだけ使う。成功の枝では記録を出さない（橋が `debug!` を残す）。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    // 検査を破ったものは SHIORI を呼ばない（kanade へ送らない）。
    if let Err(e) = validate_notify(&args.event, &args.references) {
        reply.send(outcome::ng(match e {
            NotifyArgError::EmptyEvent => "event is empty",
            NotifyArgError::ControlCharacter => "event or reference contains a control character",
        }));
        return;
    }
    let since_id = since_id_if_strict(args.strict);
    let Args {
        event,
        references,
        ghost_name,
        strict,
    } = args;
    ask_kanade(
        world,
        ghost,
        ghost_name.as_deref(),
        ASK,
        reply,
        |reply| KanadeMsg::SstpNotify {
            event,
            references,
            strict,
            reply,
        },
        move |notified, gone| match notified {
            SstpNotifyOutcome::Script(script) => {
                // strict 真のときだけ 1 行目の末尾に strict の文を足す（SSP の実測）。
                let note = since_id.map(|n| format!(" {}", strict_note(n)));
                outcome::value(format!(
                    "{SCRIPT_RETURNED}{}{SCRIPT_LINE_BREAK}Script: {script}",
                    note.unwrap_or_default()
                ))
            }
            SstpNotifyOutcome::NoScript => outcome::ok(NO_SCRIPT),
            SstpNotifyOutcome::NotAccepted => {
                tracing::debug!(
                    event = "mcp_raise_event_not_accepted",
                    "[mcp] raise_event: 手順の途中でイベントを受け付けなかった"
                );
                outcome::ng(NOT_NOW)
            }
            SstpNotifyOutcome::Failed => {
                tracing::debug!(
                    event = "mcp_raise_event_failed",
                    "[mcp] raise_event: イベントの往復が失敗したか kanade が止まった"
                );
                outcome::ng(gone)
            }
        },
    );
}

#[cfg(test)]
#[path = "raise_event_tests.rs"]
mod raise_event_tests;
