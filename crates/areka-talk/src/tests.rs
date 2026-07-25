//! [`TalkId`] / [`StartTalk`] / [`TalkDone`] / [`TalkEndReason`] の型検証テスト。
//!
//! kanade `talk.rs`（Copy／Hash／不透明 script）と sakura `contract.rs`
//! （PartialEq/Eq／3 値 reason）双方の既存テストを移設・統合し、3 値の網羅ケースを
//! 追加する（Requirements 1.1, 1.2, 1.7）。

use super::*;
use std::collections::HashSet;

// ── TalkId ──

#[test]
fn talk_id_equality_and_inequality() {
    assert_eq!(TalkId(1), TalkId(1));
    assert_ne!(TalkId(1), TalkId(2));
}

#[test]
fn talk_id_is_copy() {
    // Copy であること: 束縛後も元の値が move されず使える。
    let a = TalkId(7);
    let b = a;
    assert_eq!(a, b);
    assert_eq!(a.0, 7);
}

#[test]
fn talk_id_usable_in_hashset() {
    let mut set = HashSet::new();
    assert!(set.insert(TalkId(1)));
    assert!(set.insert(TalkId(2)));
    // 同一 id の再挿入は false（Hash + Eq が効いている）。
    assert!(!set.insert(TalkId(1)));
    assert!(set.contains(&TalkId(2)));
    assert_eq!(set.len(), 2);
}

// ── StartTalk ──

#[test]
fn start_talk_holds_opaque_script_and_id() {
    let msg = StartTalk {
        talk_id: TalkId(42),
        script: r"\0こんにちは\e".to_string(),
        epilogue: Vec::new(),
    };
    assert_eq!(msg.talk_id, TalkId(42));
    // script は素の String として保持されるのみ（解釈しない）。
    assert_eq!(msg.script, r"\0こんにちは\e");
}

#[test]
fn start_talk_is_cloneable() {
    let original = StartTalk {
        talk_id: TalkId(3),
        script: "hi".to_string(),
        epilogue: Vec::new(),
    };
    let cloned = original.clone();
    assert_eq!(cloned.talk_id, original.talk_id);
    assert_eq!(cloned.script, original.script);
}

// ── StartTalk::new / epilogue（Requirements 3.4・design C11） ──

/// 従来形コンストラクタ `StartTalk::new` は epilogue を空で構築する（従来挙動）。
#[test]
fn start_talk_new_has_empty_epilogue() {
    let msg = StartTalk::new(TalkId(1), r"\0hi\e");
    assert_eq!(msg.talk_id, TalkId(1));
    assert_eq!(msg.script, r"\0hi\e");
    assert!(msg.epilogue.is_empty());
}

/// `new` は `impl Into<String>` を受け、`String` でも `&str` でも構築できる。
#[test]
fn start_talk_new_accepts_string_and_str() {
    let from_str = StartTalk::new(TalkId(2), "abc");
    let from_string = StartTalk::new(TalkId(2), String::from("abc"));
    assert_eq!(from_str.script, "abc");
    assert_eq!(from_string.script, "abc");
    assert!(from_str.epilogue.is_empty());
    assert!(from_string.epilogue.is_empty());
}

/// `EpilogueCommand` は name + tokens を保持し、Debug/Clone/PartialEq/Eq を導出する。
#[test]
fn epilogue_command_constructs_and_derives() {
    let cmd = EpilogueCommand {
        name: "SET".to_string(),
        tokens: vec!["OnBoot".to_string(), "1".to_string()],
    };
    // Clone + PartialEq/Eq
    let cloned = cmd.clone();
    assert_eq!(cmd, cloned);
    assert_eq!(cmd.name, "SET");
    assert_eq!(cmd.tokens, vec!["OnBoot".to_string(), "1".to_string()]);
    // 差異のある値は不一致。
    let other = EpilogueCommand {
        name: "SET".to_string(),
        tokens: vec!["OnBoot".to_string()],
    };
    assert_ne!(cmd, other);
    // Debug が使えること。
    let _ = format!("{cmd:?}");
}

/// epilogue を持つ StartTalk は Clone で epilogue ごと複製される。
#[test]
fn start_talk_carries_epilogue_and_clones() {
    let epilogue = vec![EpilogueCommand {
        name: "SET".to_string(),
        tokens: vec!["k".to_string(), "v".to_string()],
    }];
    let msg = StartTalk {
        talk_id: TalkId(4),
        script: "hi".to_string(),
        epilogue: epilogue.clone(),
    };
    let cloned = msg.clone();
    assert_eq!(cloned.epilogue, epilogue);
}

// ── TalkEndReason（3 値の網羅） ──

#[test]
fn talk_end_reason_has_three_distinct_variants() {
    let ended = TalkEndReason::Ended;
    let quit = TalkEndReason::Quit;
    let interrupted = TalkEndReason::Interrupted;

    assert_eq!(ended, TalkEndReason::Ended);
    assert_eq!(quit, TalkEndReason::Quit);
    assert_eq!(interrupted, TalkEndReason::Interrupted);

    assert_ne!(ended, quit);
    assert_ne!(ended, interrupted);
    assert_ne!(quit, interrupted);
}

#[test]
fn talk_end_reason_is_copy() {
    let reason = TalkEndReason::Quit;
    let copied = reason;
    // Copy ゆえ reason も引き続き有効。
    assert_eq!(reason, copied);
}

/// 全 3 値を網羅的に match し、コンパイラが catch-all を使わず全 variant を
/// 強制することを検証する（新規 variant 追加時に再検討を強制・要件 1.2）。
#[test]
fn talk_end_reason_exhaustive_match_covers_all_variants() {
    fn label(reason: TalkEndReason) -> &'static str {
        match reason {
            TalkEndReason::Ended => "ended",
            TalkEndReason::Quit => "quit",
            TalkEndReason::Interrupted => "interrupted",
        }
    }

    assert_eq!(label(TalkEndReason::Ended), "ended");
    assert_eq!(label(TalkEndReason::Quit), "quit");
    assert_eq!(label(TalkEndReason::Interrupted), "interrupted");
}

// ── TalkDone ──

#[test]
fn talk_done_carries_id_and_reason_ended() {
    let done = TalkDone {
        talk_id: TalkId(9),
        reason: TalkEndReason::Ended,
    };
    assert_eq!(done.talk_id, TalkId(9));
    assert_eq!(done.reason, TalkEndReason::Ended);
}

#[test]
fn talk_done_carries_id_and_reason_quit() {
    let done = TalkDone {
        talk_id: TalkId(10),
        reason: TalkEndReason::Quit,
    };
    assert_eq!(done.talk_id, TalkId(10));
    assert_eq!(done.reason, TalkEndReason::Quit);
}

#[test]
fn talk_done_carries_id_and_reason_interrupted() {
    let done = TalkDone {
        talk_id: TalkId(11),
        reason: TalkEndReason::Interrupted,
    };
    assert_eq!(done.talk_id, TalkId(11));
    assert_eq!(done.reason, TalkEndReason::Interrupted);
}

#[test]
fn talk_done_is_copy() {
    let done = TalkDone {
        talk_id: TalkId(1),
        reason: TalkEndReason::Ended,
    };
    let copied = done;
    // Copy ゆえ done も引き続き有効。
    assert_eq!(done.talk_id, copied.talk_id);
    assert_eq!(done.reason, copied.reason);
}

#[test]
fn talk_done_supports_equality() {
    let a = TalkDone {
        talk_id: TalkId(5),
        reason: TalkEndReason::Quit,
    };
    let b = TalkDone {
        talk_id: TalkId(5),
        reason: TalkEndReason::Quit,
    };
    let c = TalkDone {
        talk_id: TalkId(5),
        reason: TalkEndReason::Interrupted,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}
