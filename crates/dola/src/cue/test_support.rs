//! テストだけが使う合図の組み立て（`test-support` 機能の下だけで開く）。

use super::{ActorKey, CueCommand, TalkCue};

/// 正準のキャリアの合図 `\![name,tokens...]` を発火時刻 0・長さ 0 で組む。
pub fn carrier_cue(actor: &str, name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::command_carrier(name, tokens.iter().map(|t| t.to_string()).collect()),
        duration: 0.0,
    }
}

#[cfg(test)]
#[path = "test_support_tests.rs"]
mod tests;
