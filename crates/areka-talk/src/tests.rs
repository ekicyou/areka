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

// ── TalkCommand（Requirements 5.1, 5.6・design C7・DD-4/DD-5/DD-11） ──

/// `Start` は既存の `StartTalk` を情報無損失で包む（talk_id・script・epilogue の全点）。
#[test]
fn talk_command_start_wraps_start_talk_without_loss() {
    let start = StartTalk {
        talk_id: TalkId(21),
        script: r"\0えらんで\q[はい,yes]\e".to_string(),
        epilogue: vec![EpilogueCommand {
            name: "SET".to_string(),
            tokens: vec!["k".to_string(), "v".to_string()],
        }],
    };
    let cmd = TalkCommand::Start(start.clone());

    let TalkCommand::Start(inner) = &cmd else {
        panic!("Start でなければならない");
    };
    assert_eq!(inner.talk_id, start.talk_id);
    assert_eq!(inner.script, start.script);
    assert_eq!(inner.epilogue, start.epilogue);
}

/// `ResolveChoice` は stale ガード用 talk_id と選択肢 ID を保持する（Req5.1）。
#[test]
fn talk_command_resolve_choice_carries_talk_id_and_choice_id() {
    let cmd = TalkCommand::ResolveChoice {
        talk_id: TalkId(30),
        id: "yes".to_string(),
    };
    let TalkCommand::ResolveChoice { talk_id, id } = &cmd else {
        panic!("ResolveChoice でなければならない");
    };
    assert_eq!(*talk_id, TalkId(30));
    assert_eq!(id, "yes");
}

/// `CancelChoice` は talk_id のみを保持する（DD-11・Close funnel へ写像される）。
#[test]
fn talk_command_cancel_choice_carries_talk_id() {
    let cmd = TalkCommand::CancelChoice {
        talk_id: TalkId(31),
    };
    let TalkCommand::CancelChoice { talk_id } = &cmd else {
        panic!("CancelChoice でなければならない");
    };
    assert_eq!(*talk_id, TalkId(31));
}

/// 3 形を catch-all なしで網羅 match でき、新 variant 追加時に再検討が強制される。
#[test]
fn talk_command_exhaustive_match_covers_three_forms() {
    fn label(cmd: &TalkCommand) -> &'static str {
        match cmd {
            TalkCommand::Start(_) => "start",
            TalkCommand::ResolveChoice { .. } => "resolve",
            TalkCommand::CancelChoice { .. } => "cancel",
        }
    }

    assert_eq!(
        label(&TalkCommand::Start(StartTalk::new(TalkId(1), "x"))),
        "start"
    );
    assert_eq!(
        label(&TalkCommand::ResolveChoice {
            talk_id: TalkId(1),
            id: "a".to_string(),
        }),
        "resolve"
    );
    assert_eq!(
        label(&TalkCommand::CancelChoice { talk_id: TalkId(1) }),
        "cancel"
    );
}

/// 3 形が**単一型**であることで単一チャンネルを流れ、DD-4 の
/// `[ResolveChoice, Start]` バッチ順序がそのまま保存される（Req5.6・順序保存契約）。
#[test]
fn talk_command_single_stream_preserves_batch_order() {
    fn labels(stream: &[TalkCommand]) -> Vec<&'static str> {
        stream
            .iter()
            .map(|cmd| match cmd {
                TalkCommand::Start(_) => "start",
                TalkCommand::ResolveChoice { .. } => "resolve",
                TalkCommand::CancelChoice { .. } => "cancel",
            })
            .collect()
    }

    // DD-4: 最終段が Value のとき、同一バッチで解決 → 起動の順に流れる。
    let batch = vec![
        TalkCommand::ResolveChoice {
            talk_id: TalkId(40),
            id: "menu".to_string(),
        },
        TalkCommand::Start(StartTalk::new(TalkId(41), r"\0つづき\e")),
    ];
    assert_eq!(labels(&batch), vec!["resolve", "start"]);
}

/// `TalkCommand` は Clone 可能（送出前の記録・複製が壊れないこと）。
#[test]
fn talk_command_is_cloneable() {
    let cmd = TalkCommand::ResolveChoice {
        talk_id: TalkId(50),
        id: "id".to_string(),
    };
    let cloned = cmd.clone();
    let TalkCommand::ResolveChoice { talk_id, id } = cloned else {
        panic!("ResolveChoice でなければならない");
    };
    assert_eq!(talk_id, TalkId(50));
    assert_eq!(id, "id");
    // Debug が使えること。
    let _ = format!("{cmd:?}");
}

// ── ChoiceWaiting（Requirements 7.1・design C7・DD-6/DD-7/DD-8） ──

/// 3 情報（候補 id 列・表示完了時刻・タイムアウト指令）＋ talk_id を保持する。
#[test]
fn choice_waiting_carries_ids_display_end_and_timeout_directive() {
    let waiting = ChoiceWaiting {
        talk_id: TalkId(60),
        choice_ids: vec!["a".to_string(), "b".to_string()],
        display_end_elapsed_secs: 1.5,
        timeout_directive_secs: Some(10.0),
    };
    assert_eq!(waiting.talk_id, TalkId(60));
    assert_eq!(waiting.choice_ids, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(waiting.display_end_elapsed_secs, 1.5);
    assert_eq!(waiting.timeout_directive_secs, Some(10.0));
}

/// 候補 id 列は表示順のまま保存される（DD-7 の照合が順序を壊さない前提）。
#[test]
fn choice_waiting_preserves_choice_id_order() {
    let ids = vec![
        "first".to_string(),
        "second".to_string(),
        "third".to_string(),
    ];
    let waiting = ChoiceWaiting {
        talk_id: TalkId(61),
        choice_ids: ids.clone(),
        display_end_elapsed_secs: 0.0,
        timeout_directive_secs: None,
    };
    assert_eq!(waiting.choice_ids, ids);
}

/// DD-8 のタイムアウト指令語彙 3 値（未指定／無効化／明示秒）が
/// `Option<f64>` の規約として表現できる（下流の写像はここでは行わない）。
#[test]
fn choice_waiting_timeout_directive_vocabulary_is_representable() {
    /// doc に明記した語彙をテスト側で写して固定する（DD-8）。
    fn vocabulary(directive: Option<f64>) -> &'static str {
        match directive {
            None => "unspecified",
            Some(v) if v <= 0.0 => "disabled",
            Some(_) => "explicit",
        }
    }

    let make = |directive: Option<f64>| ChoiceWaiting {
        talk_id: TalkId(62),
        choice_ids: vec!["x".to_string()],
        display_end_elapsed_secs: 2.0,
        timeout_directive_secs: directive,
    };

    assert_eq!(vocabulary(make(None).timeout_directive_secs), "unspecified");
    assert_eq!(
        vocabulary(make(Some(0.0)).timeout_directive_secs),
        "disabled"
    );
    assert_eq!(
        vocabulary(make(Some(-1.0)).timeout_directive_secs),
        "disabled"
    );
    assert_eq!(
        vocabulary(make(Some(30.0)).timeout_directive_secs),
        "explicit"
    );
}

/// `ChoiceWaiting` は Clone / PartialEq / Debug を備える（檻での到着記録・突合用）。
#[test]
fn choice_waiting_is_cloneable_and_comparable() {
    let waiting = ChoiceWaiting {
        talk_id: TalkId(63),
        choice_ids: vec!["a".to_string()],
        display_end_elapsed_secs: 3.25,
        timeout_directive_secs: None,
    };
    let cloned = waiting.clone();
    assert_eq!(waiting, cloned);

    let other = ChoiceWaiting {
        talk_id: TalkId(63),
        choice_ids: vec!["a".to_string()],
        display_end_elapsed_secs: 3.5,
        timeout_directive_secs: None,
    };
    assert_ne!(waiting, other);
    let _ = format!("{waiting:?}");
}
