//! 台本の付帯情報（[`TalkMeta`]）の既定値と送り手の名の検証テスト。

use super::*;
use crate::{StartTalk, TalkId};

#[test]
fn default_has_no_place_empty_name_and_runner_sender() {
    let meta = TalkMeta::default();
    assert_eq!(meta.runner.place, None);
    assert_eq!(meta.runner.name, "");
    assert_eq!(meta.sender, TalkSender::Runner);
}

#[test]
fn start_talk_new_carries_default_meta() {
    assert_eq!(StartTalk::new(TalkId(1), "x").meta, TalkMeta::default());
}

#[test]
fn sender_name_per_sender_kind() {
    let mut meta = TalkMeta {
        runner: TalkRunner {
            name: "runner".to_owned(),
            place: None,
        },
        sender: TalkSender::Runner,
    };
    assert_eq!(meta.sender_name(), "runner");
    meta.sender = TalkSender::Ghost("other".to_owned());
    assert_eq!(meta.sender_name(), "other");
    meta.sender = TalkSender::Tool("raise_event".to_owned());
    assert_eq!(meta.sender_name(), "raise_event");
}
