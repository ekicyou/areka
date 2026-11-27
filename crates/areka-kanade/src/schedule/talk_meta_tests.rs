//! 再生の開始に押される実行するゴースト。
//!
//! どの確かめも最上位の [`step`] を通し、最後に出る再生の開始の付帯情報を見る。

use std::path::PathBuf;

use dola::time::testing::at_ms;

use super::super::translate::Translated;
use super::super::{Action, Input, step};
use crate::msg::{KanadeConfig, ShioriOutcome};
use crate::schedule::translate_test_support::{config as base_config, steady_none};
use crate::talk::{GhostPlace, TalkMeta, TalkRunner, TalkSender};
use areka_talk::SecurityLevel;

const SCRIPT: &str = r"\0台本\e";

fn place() -> GhostPlace {
    GhostPlace {
        master: PathBuf::from(r"C:\ghosts\a\ghost\master"),
        root: PathBuf::from(r"C:\ghosts\a"),
    }
}

fn config() -> KanadeConfig {
    KanadeConfig {
        ghost_name: "あ".to_string(),
        ghost_place: Some(place()),
        ..base_config()
    }
}

/// 入力を順に [`step`] へ通し、最後の一括の再生の開始の付帯情報を返す。
fn started_meta(config: &KanadeConfig, inputs: Vec<Input>) -> TalkMeta {
    let mut state = steady_none();
    let mut last = Vec::new();
    for input in inputs {
        let (s, actions) = step(state, input, config);
        state = s;
        last = actions;
    }
    match last.as_slice() {
        [Action::StartTalk(start)] => start.meta.clone(),
        other => panic!("再生の開始が 1 つ返るはず: {}", other.len()),
    }
}

fn ghost_reply_inputs() -> Vec<Input> {
    vec![
        Input::Tick { now: at_ms(2_000) },
        Input::ShioriReply {
            outcome: ShioriOutcome::Value(SCRIPT.to_string(), SecurityLevel::Local),
            origin: "OnSecondChange",
        },
        Input::TranslateDone(Ok(Translated::local(SCRIPT))),
    ]
}

#[test]
fn ghost_reply_talk_carries_config_runner_and_runner_sender() {
    let config = config();
    let meta = started_meta(&config, ghost_reply_inputs());
    assert_eq!(
        meta.runner,
        TalkRunner {
            name: "あ".to_string(),
            place: Some(place()),
        }
    );
    assert_eq!(meta.sender, TalkSender::Runner);
}
