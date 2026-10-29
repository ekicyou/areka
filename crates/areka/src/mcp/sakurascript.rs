//! `sakurascript` の処理（spec: areka-P0-mcp-kanade-tools）。
//!
//! 置き場のゴーストの kanade へ外からの台本（`SstpSend`）を送り、返事を後から答える口
//! （`super::later`）で毎フレーム覗く。`raise_event` と使う部品（`script_target`・文言の定数）も
//! ここに置く（振り分けのファイルに宣言を足さないため）。

use std::sync::mpsc::Sender;

use areka_actor::{ReplyError, reply_channel};
use areka_kanade::KanadeMsg;
use areka_kanade::sstp::SstpSendOutcome;
use areka_mcp::tools::sakurascript::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::{self, ActiveGhost, NOT_ACTIVE, Omitted, listed_value};
use crate::ghost_session::GhostSlot;

/// 起動・終了・切替の手順の途中で受け付けなかったときの本文（SSP の実測の文言・要件 3.6）。
pub(super) const NOT_NOW: &str = "the ghost cannot accept a script now";

/// 置き場に kanade の送り口が無いときの本文（areka 独自・本番では起きない内部の食い違い）。
pub(super) const UNAVAILABLE: &str = "Script is not available";

/// strict 真の答えに足す文の `<n>` の前まで（`sakurascript` と `raise_event` で共用・要件 1.1・3.10）。
const STRICT_NOTE_HEAD: &str = "strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=";

/// strict 真の答えに足す文（`OK:` は付けない）。`since_id` は送る前に読んだ最後の通し番号（要件 1.2）。
pub(super) fn strict_note(since_id: u64) -> String {
    format!("{STRICT_NOTE_HEAD}{since_id})")
}

/// strict 真のときだけ、送る前の最後の通し番号を読む（後に読むとその台本の記録より後になりうる）。
pub(super) fn since_id_if_strict(strict: Option<bool>) -> Option<u64> {
    (strict == Some(true)).then(crate::log_history::last_id)
}

/// 宛先の kanade の送り口と、答える前に降りたときの文言。
pub(super) struct ScriptTarget {
    pub kanade: Option<Sender<KanadeMsg>>,
    /// 起動中のゴースト無しで解決したときの失敗の理由（`get_status` と同じ求め方・要件 3.5）。
    pub gone: &'static str,
}

/// 置き場の 1 体から送り口を引き、`ghost_name` から「降りた」の文言を求める。
pub(super) fn script_target(world: &World, ghost_name: Option<&str>) -> ScriptTarget {
    let gone = resolve::resolve(None, ghost_name, Omitted::UseActive)
        .err()
        .unwrap_or(NOT_ACTIVE);
    let kanade = world
        .get_non_send::<GhostSlot>()
        .and_then(|slot| slot.0.as_ref())
        .and_then(|session| session.kanade())
        .cloned();
    ScriptTarget { kanade, gone }
}

/// 台本を 1 通送り、返事は待たずに `later` へ預ける（要件 1.1・1.3〜1.7・3.5・3.6・3.9・6.4・6.5）。
///
/// `ghost` は送り口が無いときの `warn!` の欄にだけ使う。成功の枝では記録を出さない（橋が `debug!` を残す）。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    // 空文字だけを空と数える（空白だけの台本は送る・要件 1.4）。
    if args.script.is_empty() {
        reply.send(outcome::ng("script is empty"));
        return;
    }
    let ScriptTarget { kanade, gone } = script_target(world, args.ghost_name.as_deref());
    let Some(kanade) = kanade else {
        tracing::warn!(
            event = "mcp_sakurascript_unavailable",
            ghost = listed_value(ghost).as_str(),
            "[mcp] sakurascript: 宛先のゴーストの kanade の送り口が見つからない"
        );
        reply.send(outcome::ng(UNAVAILABLE));
        return;
    };
    let (tx, rx) = reply_channel::<SstpSendOutcome>();
    let since_id = since_id_if_strict(args.strict);
    let msg = KanadeMsg::SstpSend {
        script: args.script,
        strict: args.strict,
        reply: tx,
    };
    if kanade.send(msg).is_err() {
        tracing::debug!(
            event = "mcp_sakurascript_kanade_gone",
            "[mcp] sakurascript: kanade が止まっていて台本を送れない"
        );
        reply.send(outcome::ng(gone));
        return;
    }
    super::later(world, reply, move |_| match rx.try_recv() {
        Ok(None) => None,
        Ok(Some(SstpSendOutcome::Accepted)) => Some(match since_id {
            Some(n) => outcome::ok(&strict_note(n)),
            None => outcome::ok(""),
        }),
        Ok(Some(SstpSendOutcome::NotAccepted)) => {
            tracing::debug!(
                event = "mcp_sakurascript_not_accepted",
                "[mcp] sakurascript: 手順の途中で台本を受け付けなかった"
            );
            Some(outcome::ng(NOT_NOW))
        }
        Ok(Some(SstpSendOutcome::Failed)) => {
            tracing::debug!(
                event = "mcp_sakurascript_failed",
                "[mcp] sakurascript: 台本の処理の中で kanade が止まった"
            );
            Some(outcome::ng(gone))
        }
        // `Timeout` は `try_recv` からは返らない。型の上の腕は「降りた」に倒す。
        Err(ReplyError::Dropped | ReplyError::Timeout) => {
            tracing::debug!(
                event = "mcp_sakurascript_reply_dropped",
                "[mcp] sakurascript: 返事の前に kanade の返信端が落ちた"
            );
            Some(outcome::ng(gone))
        }
    });
}

#[cfg(test)]
#[path = "sakurascript_tests.rs"]
mod sakurascript_tests;

#[cfg(test)]
#[path = "sakurascript_log_tests.rs"]
mod log_tests;
