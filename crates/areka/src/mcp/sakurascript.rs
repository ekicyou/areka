//! `sakurascript` の処理。kanade へ台本（`SstpSend`）を送り、返事は `super::later` で後から答える。
//!
//! `raise_event` と使う部品（文言の定数と strict の文・番号）もここに置く。

use areka_kanade::KanadeMsg;
use areka_kanade::sstp::SstpSendOutcome;
use areka_mcp::tools::sakurascript::Args;
use areka_mcp::tools::{ReplyTo, outcome};
use bevy_ecs::world::World;

use super::resolve::{ActiveGhost, KanadeAsk, ask_kanade};

/// 起動・終了・切替の手順の途中で受け付けなかったときの本文（SSP の実測の文言）。
pub(super) const NOT_NOW: &str = "the ghost cannot accept a script now";

/// 置き場に kanade の送り口が無いときの本文（areka 独自・本番では起きない内部の食い違い）。
pub(super) const UNAVAILABLE: &str = "Script is not available";

/// strict 真の答えに足す文の `<n>` の前まで（`sakurascript` と `raise_event` で共用）。
const STRICT_NOTE_HEAD: &str = "strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=";

/// strict 真の答えに足す文（`OK:` は付けない）。`since_id` は送る前に読んだ最後の通し番号。
pub(super) fn strict_note(since_id: u64) -> String {
    format!("{STRICT_NOTE_HEAD}{since_id})")
}

/// strict 真のときだけ、送る前の最後の通し番号を読む（後に読むとその台本の記録より後になりうる）。
pub(super) fn since_id_if_strict(strict: Option<bool>) -> Option<u64> {
    (strict == Some(true)).then(areka_session::log_history::last_id)
}

const ASK: KanadeAsk = KanadeAsk {
    tool: "sakurascript",
    what: "台本",
    unavailable: UNAVAILABLE,
    unavailable_event: "mcp_sakurascript_unavailable",
    gone_event: "mcp_sakurascript_kanade_gone",
    dropped_event: "mcp_sakurascript_reply_dropped",
};

/// 台本を 1 通送り、返事は待たずに `later` へ預ける。
///
/// `ghost` は送り口が無いときの `warn!` の欄にだけ使う。成功の枝では記録を出さない（橋が `debug!` を残す）。
pub(super) fn handle(world: &mut World, ghost: &ActiveGhost, args: Args, reply: ReplyTo) {
    // 空文字だけを空と数える（空白だけの台本は送る）。
    if args.script.is_empty() {
        reply.send(outcome::ng("script is empty"));
        return;
    }
    let since_id = since_id_if_strict(args.strict);
    let Args {
        script,
        ghost_name,
        strict,
    } = args;
    ask_kanade(
        world,
        ghost,
        ghost_name.as_deref(),
        ASK,
        reply,
        |reply| KanadeMsg::SstpSend {
            script,
            strict,
            reply,
        },
        move |sent, gone| match sent {
            SstpSendOutcome::Accepted => match since_id {
                Some(n) => outcome::ok(&strict_note(n)),
                None => outcome::ok(""),
            },
            SstpSendOutcome::NotAccepted => {
                tracing::debug!(
                    event = "mcp_sakurascript_not_accepted",
                    "[mcp] sakurascript: 手順の途中で台本を受け付けなかった"
                );
                outcome::ng(NOT_NOW)
            }
            SstpSendOutcome::Failed => {
                tracing::debug!(
                    event = "mcp_sakurascript_failed",
                    "[mcp] sakurascript: 台本の処理の中で kanade が止まった"
                );
                outcome::ng(gone)
            }
        },
    );
}

#[cfg(test)]
#[path = "sakurascript_tests.rs"]
mod sakurascript_tests;
