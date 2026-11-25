//! `raise_event` の決定論テスト。
//!
//! 置き場に偽の kanade の送り口（`mpsc` の送出端）を置き、受信端で外から頼まれたイベントの知らせを
//! 受け取って返信端へ結果を送る（または落とす）。答えは後から答える置き場（`later`）を `drain` で覗いて取る。

use std::sync::mpsc;

use areka_actor::ReplySender;
use areka_mcp::tools::{Pending, ToolCall, ToolRequest};
use areka_mcp::{ToolContent, ToolOutcome};
use log_capture_kit::{CapturedEvent, capture, count_levels};

use super::*;
use crate::mcp::kanade_fake_test_support::{answer, ghost, record_one, rig, text, warn_or_worse};
use crate::mcp::resolve::{CANNOT_FIND, NOT_ACTIVE};
use crate::mcp::{McpLater, drain};

const EVENT: &str = "OnAiTalk";
const SCRIPT: &str = r"\0\s[0]hello\e";

fn args(event: &str, references: &[&str], ghost_name: Option<&str>, strict: Option<bool>) -> Args {
    Args {
        event: event.to_string(),
        references: references.iter().map(|r| r.to_string()).collect(),
        ghost_name: ghost_name.map(str::to_string),
        strict,
    }
}

/// `handle` を呼んで待つ側を返す（記録は呼んだスレッドで拾う）。
fn call(world: &mut World, args: Args) -> (Pending, Vec<CapturedEvent>) {
    let (req, pending) = ToolRequest::new(ToolCall::RaiseEvent(args.clone()));
    let ((), events) = capture(|| handle(world, &ghost(), args, req.reply));
    (pending, events)
}

type Notify = (
    String,
    Vec<String>,
    Option<bool>,
    ReplySender<SstpNotifyOutcome>,
);

/// 偽の受信端に届いた外から頼まれたイベントの知らせ（`SstpNotify` 以外が届いたら落ちる）。
fn take_notify(rx: &mpsc::Receiver<KanadeMsg>) -> Notify {
    match rx.try_recv() {
        Ok(KanadeMsg::SstpNotify {
            event,
            references,
            strict,
            reply,
        }) => (event, references, strict, reply),
        other => panic!(
            "外から頼まれたイベントの知らせ 1 通が届いているはず: {:?}",
            other.map(|_| "別の知らせ")
        ),
    }
}

/// 結果を送って 1 フレーム回した後の答え（その間の記録の warn 以上も返す）。
fn answer_with(ghost_name: Option<&str>, result: SstpNotifyOutcome) -> (ToolOutcome, usize) {
    let (mut world, rx) = rig();
    let (pending, called) = call(&mut world, args(EVENT, &[], ghost_name, None));
    let (_, _, _, reply) = take_notify(&rx);
    reply.send(result).expect("返信端は生きている");
    let ((), polled) = capture(|| drain(&mut world));
    (
        answer(&pending),
        warn_or_worse(&called) + warn_or_worse(&polled),
    )
}

/// 台本が返った → 2 行（区切りは `\r\n`）・isError: false・warn 以上 0 件。
#[test]
fn script_is_two_lines_with_crlf() {
    let (outcome, warns) = answer_with(
        Some("Emily/Phase4.5"),
        SstpNotifyOutcome::Script(SCRIPT.to_string()),
    );
    assert_eq!(
        outcome.content,
        text(&format!(
            "OK:the ghost returned a script and it is being played.\r\nScript: {SCRIPT}"
        ))
    );
    assert!(!outcome.is_error);
    assert_eq!(warns, 0);
}

/// 台本が返らなかった → `OK:the ghost returned no script.`・isError: false。
#[test]
fn no_script_is_ok_no_script() {
    let (outcome, warns) = answer_with(None, SstpNotifyOutcome::NoScript);
    assert_eq!(outcome.content, text("OK:the ghost returned no script."));
    assert!(!outcome.is_error);
    assert_eq!(warns, 0);
}

/// 受け付けなかった → `NG:the ghost cannot accept a script now`・warn 以上 0 件。
#[test]
fn not_accepted_is_cannot_accept_now() {
    let (outcome, warns) = answer_with(None, SstpNotifyOutcome::NotAccepted);
    assert_eq!(
        outcome.content,
        text("NG:the ghost cannot accept a script now")
    );
    assert!(outcome.is_error);
    assert_eq!(warns, 0);
}

/// 失敗・返信端の落ち・送れなかった → `ghost_name` の有無で宛先が居ないときの 2 通りの `NG:`・
/// warn 以上 0 件。
#[test]
fn failed_dropped_and_unsendable_read_as_the_resolve_failure() {
    for (ghost_name, expected) in [(Some("Emily/Phase4.5"), CANNOT_FIND), (None, NOT_ACTIVE)] {
        let want = text(&format!("NG:{expected}"));

        let (outcome, warns) = answer_with(ghost_name, SstpNotifyOutcome::Failed);
        assert_eq!(outcome.content, want, "Failed・{ghost_name:?}");
        assert!(outcome.is_error, "Failed・{ghost_name:?}");
        assert_eq!(warns, 0, "Failed・{ghost_name:?}");

        let (mut world, rx) = rig();
        let (pending, called) = call(&mut world, args(EVENT, &[], ghost_name, None));
        drop(take_notify(&rx));
        let ((), polled) = capture(|| drain(&mut world));
        let outcome = answer(&pending);
        assert_eq!(outcome.content, want, "返信端を落とす・{ghost_name:?}");
        assert!(outcome.is_error);
        assert_eq!(
            warn_or_worse(&called) + warn_or_worse(&polled),
            0,
            "返信端を落とす・{ghost_name:?}: {called:?} {polled:?}"
        );

        // 受信端を先に落として送れなくする（その場で答え、預けない）。
        let (mut world, rx) = rig();
        drop(rx);
        let (pending, called) = call(&mut world, args(EVENT, &[], ghost_name, None));
        let outcome = answer(&pending);
        assert_eq!(outcome.content, want, "送れない・{ghost_name:?}");
        assert!(outcome.is_error);
        assert_eq!(warn_or_worse(&called), 0, "送れない・{ghost_name:?}");
        assert!(
            world.non_send::<McpLater>().0.is_empty(),
            "送れない・預けない"
        );
    }
}

/// イベント名・Reference・strict の真・偽・省略がそのまま知らせに載る。
#[test]
fn event_references_and_strict_are_carried_verbatim() {
    for strict in [Some(true), Some(false), None] {
        let (mut world, rx) = rig();
        let _ = call(&mut world, args(EVENT, &["a", "", "b c"], None, strict));
        let (event, references, carried, _) = take_notify(&rx);
        assert_eq!(event, EVENT);
        assert_eq!(references, vec!["a", "", "b c"]);
        assert_eq!(carried, strict);
        assert!(rx.try_recv().is_err(), "知らせは 1 通だけ");
    }
}

/// 検査の誤り → kanade へ何も送らず、その場で `NG:`・warn 以上 0 件。
#[test]
fn invalid_arguments_send_nothing() {
    let cases = [
        (args("", &[], None, None), "NG:event is empty"),
        (args("  ", &["x"], None, None), "NG:event is empty"),
        (
            args("On\rTest", &[], None, None),
            "NG:event or reference contains a control character",
        ),
        (
            args(EVENT, &["ok", "a\nb"], None, Some(true)),
            "NG:event or reference contains a control character",
        ),
        (
            args(EVENT, &["a\0"], None, Some(false)),
            "NG:event or reference contains a control character",
        ),
    ];
    for (a, want) in cases {
        let label = format!("{:?} {:?}", a.event, a.references);
        let (mut world, rx) = rig();
        let (pending, called) = call(&mut world, a);
        let outcome = answer(&pending);
        assert_eq!(outcome.content, text(want), "{label}");
        assert!(outcome.is_error, "{label}");
        assert!(rx.try_recv().is_err(), "kanade に何も届かない・{label}");
        assert!(world.non_send::<McpLater>().0.is_empty(), "{label}");
        assert_eq!(warn_or_worse(&called), 0, "{label}");
    }
}

/// 返事が呼んだ直後（最初の覗きの前）に届いていても、何フレームか後に届いても答えになる。
#[test]
fn answer_may_be_ready_at_once_or_arrive_later() {
    // その場で返る（最初の覗きで答える）。
    let (mut world, rx) = rig();
    let (pending, _) = call(&mut world, args(EVENT, &[], None, None));
    take_notify(&rx)
        .3
        .send(SstpNotifyOutcome::NoScript)
        .expect("返信端は生きている");
    drain(&mut world);
    assert_eq!(
        answer(&pending).content,
        text("OK:the ghost returned no script.")
    );

    // 後から届く（返事の前の覗きでは答えず預けたまま）。
    let (mut world, rx) = rig();
    let (pending, _) = call(&mut world, args(EVENT, &[], None, None));
    assert!(matches!(pending.try_answer(), Ok(None)), "呼んだ直後は未着");
    let (_, _, _, reply) = take_notify(&rx);
    drain(&mut world);
    drain(&mut world);
    assert!(matches!(pending.try_answer(), Ok(None)), "返事の前は未着");
    assert_eq!(world.non_send::<McpLater>().0.len(), 1, "預けたまま");
    reply
        .send(SstpNotifyOutcome::NoScript)
        .expect("返信端は生きている");
    drain(&mut world);
    assert_eq!(
        answer(&pending).content,
        text("OK:the ghost returned no script.")
    );
    assert!(world.non_send::<McpLater>().0.is_empty());
}

/// 置き場に送り口が無い（空の World）→ `NG:Script is not available`・warn ちょうど 1 件・error 0 件。
#[test]
fn answers_not_available_without_a_kanade_in_the_slot() {
    let a = args(EVENT, &["a"], Some("Emily/Phase4.5"), None);
    let (req, pending) = ToolRequest::new(ToolCall::RaiseEvent(a.clone()));

    let ((), levels) = count_levels(|| handle(&mut World::new(), &ghost(), a, req.reply));

    let outcome = answer(&pending);
    assert_eq!(outcome.content, text("NG:Script is not available"));
    assert!(outcome.is_error);
    assert_eq!((levels.warn, levels.error), (1, 0));
}

/// strict の文（`<n>` の前まで）。
const STRICT_HEAD: &str = " strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=";

/// strict 真で台本が返った → 1 行目の末尾に strict の文・区切りと `Script: ` の行は strict の無いときと同じ・
/// `<n>` は送る前に読んだ最後の番号。
///
/// 置き場はプロセスに 1 つで他のテストも積むので、`<n>` は「呼ぶ前の番号以上・送った後に積んだ番号未満」で測る。
#[test]
fn strict_true_script_answer_carries_the_id_read_before_sending() {
    let (mut world, rx) = rig();
    let before = areka_session::log_history::last_id();
    let (pending, _) = call(&mut world, args(EVENT, &[], None, Some(true)));
    let (_, _, _, reply) = take_notify(&rx);
    let recorded = record_one();
    reply
        .send(SstpNotifyOutcome::Script(SCRIPT.to_string()))
        .expect("返信端は生きている");
    drain(&mut world);
    let outcome = answer(&pending);
    assert!(!outcome.is_error);
    let [ToolContent::Text(body)] = outcome.content.as_slice() else {
        panic!("本文 1 つ: {:?}", outcome.content);
    };
    let n: u64 = body
        .strip_prefix("OK:the ghost returned a script and it is being played.")
        .and_then(|rest| rest.strip_prefix(STRICT_HEAD))
        .and_then(|rest| rest.strip_suffix(&format!(")\r\nScript: {SCRIPT}")))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("strict の文の付いた 2 行のはず: {body:?}"));
    assert!(
        before <= n && n < recorded,
        "送る前の番号: {before} <= {n} < {recorded}"
    );
}

/// strict 偽・省略で台本が返った・strict 真でも台本が返らない／受け付けない → strict の文は付かない。
#[test]
fn strict_sentence_only_when_strict_true_and_a_script_returned() {
    let plain =
        format!("OK:the ghost returned a script and it is being played.\r\nScript: {SCRIPT}");
    let cases = [
        (
            Some(false),
            SstpNotifyOutcome::Script(SCRIPT.to_string()),
            plain.as_str(),
        ),
        (
            None,
            SstpNotifyOutcome::Script(SCRIPT.to_string()),
            plain.as_str(),
        ),
        (
            Some(true),
            SstpNotifyOutcome::NoScript,
            "OK:the ghost returned no script.",
        ),
        (
            Some(true),
            SstpNotifyOutcome::NotAccepted,
            "NG:the ghost cannot accept a script now",
        ),
    ];
    for (strict, result, want) in cases {
        let (mut world, rx) = rig();
        let (pending, _) = call(&mut world, args(EVENT, &[], None, strict));
        take_notify(&rx).3.send(result).expect("返信端は生きている");
        drain(&mut world);
        assert_eq!(answer(&pending).content, text(want), "{strict:?}");
    }
}
