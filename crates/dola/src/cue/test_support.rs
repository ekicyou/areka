//! テストだけが使う合図の組み立て（`test-support` 機能の下だけで開く）。

use super::{ActorKey, CueCommand, TalkCue};
use crate::time::{TimePoint, TimeSpan};

/// 正準のキャリアの合図 `\![name,tokens...]` を起点 0・台本の時刻 0・長さ 0 で組む。
pub fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue::scheduled(
        TimePoint::ZERO,
        TimeSpan::ZERO,
        ActorKey::from(actor),
        CueCommand::command_carrier(name, tokens.iter().map(|t| t.to_string()).collect()),
        TimeSpan::ZERO,
    )
}

#[cfg(test)]
#[path = "test_support_tests.rs"]
mod tests;
