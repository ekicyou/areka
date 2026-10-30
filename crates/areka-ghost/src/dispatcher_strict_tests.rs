//! 配り口が印の真の台本だけ strict の束ねを渡すことの檻（spec: mcp-strict-errors 要件 3.5・task 6.1）。

use super::*;
use crate::test_log_capture::capture;
use areka_sakura::strict::{DetectionPoint, StrictJudges};
use std::sync::Arc;
use tracing::Level;

struct Judges;

impl StrictJudges for Judges {
    fn command_handled(&self, _name: &str, _first: Option<&str>) -> bool {
        true
    }
    fn font(&self, _args: &[String]) -> Option<DetectionPoint> {
        None
    }
}

fn hooks() -> StrictHooks {
    StrictHooks {
        ghost: "さくら".into(),
        judges: Arc::new(Judges),
    }
}

fn start(strict: bool) -> StartTalk {
    StartTalk {
        strict,
        ..StartTalk::new(TalkId(1), r"\0A\e")
    }
}

#[test]
fn hooks_are_handed_only_to_a_strict_talk() {
    let hooks = hooks();
    assert!(strict_hooks_for(&start(false), Some(&hooks)).is_none());
    let handed = strict_hooks_for(&start(true), Some(&hooks)).expect("strict talk gets the hooks");
    assert_eq!(&*handed.ghost, "さくら");
    assert!(Arc::ptr_eq(&handed.judges, &hooks.judges));
}

#[test]
fn strict_talk_without_judges_plays_without_records_and_says_so_once() {
    let mut handed = Some(hooks());
    let events = capture(|| handed = strict_hooks_for(&start(true), None));
    assert!(handed.is_none());
    let said: Vec<_> = events
        .iter()
        .filter(|e| {
            e.level == Level::DEBUG && e.message.contains("strict talk without strict judges")
        })
        .collect();
    assert_eq!(said.len(), 1, "{events:?}");

    // 印が偽の台本は差し込みが無くても何も言わない。
    let events = capture(|| handed = strict_hooks_for(&start(false), None));
    assert!(handed.is_none());
    assert!(
        !events
            .iter()
            .any(|e| e.message.contains("strict talk without strict judges")),
        "{events:?}"
    );
}
