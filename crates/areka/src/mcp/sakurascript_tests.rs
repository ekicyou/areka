//! `sakurascript` の決定論テスト。
//!
//! 置き場に偽の kanade の送り口（`mpsc` の送出端）を置き、受信端で外からの台本の知らせを受け取って
//! 返信端へ結果を送る（または落とす）。答えは後から答える置き場（`later`）を `drain` で覗いて取る。

use std::sync::mpsc;

use areka_actor::ReplySender;
use areka_mcp::ToolOutcome;
use areka_mcp::tools::{Pending, ToolCall, ToolRequest};
use log_capture_kit::{CapturedEvent, capture, count_levels};

use super::*;
use crate::mcp::kanade_fake_test_support::{
    STRICT_NOTE_BEFORE_ID, answer, body, body_and_error, ghost, ids, record_one, rig, text,
};
use crate::mcp::resolve::{CANNOT_FIND, NOT_ACTIVE};
use crate::mcp::{McpLater, drain};
use crate::session_tests::warn_or_worse;

const SCRIPT: &str = r"\0\s[0]hello\e";

/// `handle` を呼んで待つ側を返す（記録は呼んだスレッドで拾う）。
fn call(
    world: &mut World,
    script: &str,
    ghost_name: Option<&str>,
    strict: Option<bool>,
) -> (Pending, Vec<CapturedEvent>) {
    let args = Args {
        script: script.to_string(),
        ghost_name: ghost_name.map(str::to_string),
        strict,
    };
    let (req, pending) = ToolRequest::new(ToolCall::Sakurascript(args.clone()));
    let ((), events) = capture(|| handle(world, &ghost(), args, req.reply));
    (pending, events)
}

/// 偽の受信端に届いた外からの台本の知らせ（`SstpSend` 以外が届いたら落ちる）。
fn take_send(
    rx: &mpsc::Receiver<KanadeMsg>,
) -> (String, Option<bool>, ReplySender<SstpSendOutcome>) {
    match rx.try_recv() {
        Ok(KanadeMsg::SstpSend {
            script,
            strict,
            reply,
        }) => (script, strict, reply),
        other => panic!(
            "外からの台本の知らせ 1 通が届いているはず: {:?}",
            other.map(|_| "別の知らせ")
        ),
    }
}

/// 結果を送って 1 フレーム回した後の答え（その間の記録の warn 以上も返す）。
fn answer_with(ghost_name: Option<&str>, result: SstpSendOutcome) -> (ToolOutcome, usize) {
    let (mut world, rx) = rig();
    let (pending, called) = call(&mut world, SCRIPT, ghost_name, None);
    let (_, _, reply) = take_send(&rx);
    reply.send(result).expect("返信端は生きている");
    let ((), polled) = capture(|| drain(&mut world));
    (
        answer(&pending),
        warn_or_worse(&called).len() + warn_or_worse(&polled).len(),
    )
}

/// 受け付けた → `OK`・isError: false・warn 以上 0 件。
#[test]
fn accepted_is_ok() {
    let (outcome, warns) = answer_with(Some("Emily/Phase4.5"), SstpSendOutcome::Accepted);
    assert_eq!(outcome.content, text("OK"));
    assert!(!outcome.is_error);
    assert_eq!(warns, 0);
}

/// 受け付けなかった → `NG:the ghost cannot accept a script now`・warn 以上 0 件。
#[test]
fn not_accepted_is_cannot_accept_now() {
    let (outcome, warns) = answer_with(None, SstpSendOutcome::NotAccepted);
    assert_eq!(
        outcome.content,
        text("NG:the ghost cannot accept a script now")
    );
    assert!(outcome.is_error);
    assert_eq!(warns, 0);
}

/// 失敗 → `ghost_name` の有無で宛先が居ないときの 2 通りの `NG:`・warn 以上 0 件。
#[test]
fn failed_reads_as_the_resolve_failure() {
    for (ghost_name, expected) in [(Some("Emily/Phase4.5"), CANNOT_FIND), (None, NOT_ACTIVE)] {
        let (outcome, warns) = answer_with(ghost_name, SstpSendOutcome::Failed);
        assert_eq!(
            body_and_error(&outcome),
            (format!("NG:{expected}"), true),
            "{ghost_name:?}"
        );
        assert_eq!(warns, 0, "{ghost_name:?}");
    }
}

/// 台本と strict の真・偽・省略がそのまま知らせに載る。
#[test]
fn script_and_strict_are_carried_verbatim() {
    for strict in [Some(true), Some(false), None] {
        let (mut world, rx) = rig();
        let _ = call(&mut world, SCRIPT, None, strict);
        let (script, carried, _) = take_send(&rx);
        assert_eq!(script, SCRIPT);
        assert_eq!(carried, strict);
        assert!(rx.try_recv().is_err(), "知らせは 1 通だけ");
    }
}

/// 空文字の台本 → kanade へ何も送らず `NG:script is empty`。空白だけは空と数えない。
#[test]
fn empty_script_sends_nothing() {
    let (mut world, rx) = rig();
    let (pending, called) = call(&mut world, "", None, Some(true));
    let outcome = answer(&pending);
    assert_eq!(outcome.content, text("NG:script is empty"));
    assert!(outcome.is_error);
    assert!(rx.try_recv().is_err(), "kanade に何も届かない");
    assert!(world.non_send::<McpLater>().0.is_empty());
    assert_eq!(warn_or_worse(&called).len(), 0);

    let (mut world, rx) = rig();
    let _ = call(&mut world, " ", None, None);
    assert_eq!(take_send(&rx).0, " ", "空白だけは送る");
}

/// 置き場に送り口が無い（空の World）→ `NG:Script is not available`・warn ちょうど 1 件・error 0 件。
#[test]
fn answers_not_available_without_a_kanade_in_the_slot() {
    let args = Args {
        script: SCRIPT.to_string(),
        ghost_name: Some("Emily/Phase4.5".to_string()),
        strict: None,
    };
    let (req, pending) = ToolRequest::new(ToolCall::Sakurascript(args.clone()));

    let ((), levels) = count_levels(|| handle(&mut World::new(), &ghost(), args, req.reply));

    let outcome = answer(&pending);
    assert_eq!(outcome.content, text("NG:Script is not available"));
    assert!(outcome.is_error);
    assert_eq!((levels.warn, levels.error), (1, 0));
}

/// strict の文の答えの `<n>`（strict の文でなければ落ちる）。
fn strict_id(outcome: &ToolOutcome) -> u64 {
    let body = body(outcome);
    body.strip_prefix("OK:")
        .and_then(|rest| rest.strip_prefix(STRICT_NOTE_BEFORE_ID))
        .and_then(|rest| rest.strip_suffix(')'))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("strict の文のはず: {body}"))
}

/// strict 真で受け付けた → strict の文・`<n>` は送る前に読んだ最後の番号。
///
/// 置き場はプロセスに 1 つで他のテストも積むので、`<n>` は「呼ぶ前の番号以上・送った後に積んだ番号未満」で測る。
#[test]
fn strict_true_answer_carries_the_id_read_before_sending() {
    let (mut world, rx) = rig();
    let before = areka_session::log_history::last_id();
    let (pending, called) = call(&mut world, SCRIPT, None, Some(true));
    let (_, _, reply) = take_send(&rx);
    let recorded = record_one();
    reply
        .send(SstpSendOutcome::Accepted)
        .expect("返信端は生きている");
    let ((), polled) = capture(|| drain(&mut world));
    let outcome = answer(&pending);
    assert!(!outcome.is_error);
    assert_eq!(
        warn_or_worse(&called).len() + warn_or_worse(&polled).len(),
        0
    );
    let n = strict_id(&outcome);
    assert!(
        before <= n && n < recorded,
        "送る前の番号: {before} <= {n} < {recorded}"
    );
}

/// strict 偽・省略で受け付けた → `OK` のまま。
#[test]
fn strict_false_or_omitted_answer_is_plain_ok() {
    for strict in [Some(false), None] {
        let (mut world, rx) = rig();
        let (pending, _) = call(&mut world, SCRIPT, None, strict);
        take_send(&rx)
            .2
            .send(SstpSendOutcome::Accepted)
            .expect("返信端は生きている");
        drain(&mut world);
        assert_eq!(answer(&pending).content, text("OK"), "{strict:?}");
    }
}

/// strict 真でも NG の 4 つ（空・受け付けない・失敗・返信端の落ち）に strict の文は付かない。
#[test]
fn strict_true_ng_answers_carry_no_strict_note() {
    let (mut world, _rx) = rig();
    let (pending, _) = call(&mut world, "", None, Some(true));
    assert_eq!(answer(&pending).content, text("NG:script is empty"));

    for (result, want) in [
        (
            Some(SstpSendOutcome::NotAccepted),
            "NG:the ghost cannot accept a script now".to_string(),
        ),
        (Some(SstpSendOutcome::Failed), format!("NG:{NOT_ACTIVE}")),
        (None, format!("NG:{NOT_ACTIVE}")),
    ] {
        let (mut world, rx) = rig();
        let (pending, _) = call(&mut world, SCRIPT, None, Some(true));
        let (_, _, reply) = take_send(&rx);
        match result {
            Some(r) => reply.send(r).expect("返信端は生きている"),
            None => drop(reply),
        }
        drain(&mut world);
        let outcome = answer(&pending);
        assert_eq!(outcome.content, text(&want));
        assert!(outcome.is_error);
    }
}

/// `get_log(log_type=error, since_id)` の答えの行の番号（`#<id> ` で始まる行だけ）。
fn error_log_ids_since(since_id: u64) -> Vec<u64> {
    let args = areka_mcp::tools::get_log::Args {
        log_type: Some("error".to_string()),
        ghost_name: None,
        since_id: Some(since_id as i64),
        max_count: None,
    };
    let (req, pending) = ToolRequest::new(ToolCall::GetLog(args.clone()));
    crate::mcp::get_log::handle(None, args, req.reply);
    let outcome = answer(&pending);
    assert!(!outcome.is_error);
    ids(body(&outcome))
}

/// 答えの `<n>` で `get_log(log_type=error, since_id=<n>)` を読むと、送った後に積んだ記録が全部読め、
/// 呼ぶ前に積んだ記録は読めない。
///
/// 置き場はプロセスに 1 つで他のテストも積むので、読めた番号の集まりに含まれるか・含まれないかで測る。
#[test]
fn since_id_of_the_answer_reads_every_record_after_sending() {
    let (mut world, rx) = rig();
    let earlier = record_one();
    let (pending, _) = call(&mut world, SCRIPT, None, Some(true));
    let (_, _, reply) = take_send(&rx);
    let recorded = [record_one(), record_one()];
    reply
        .send(SstpSendOutcome::Accepted)
        .expect("返信端は生きている");
    drain(&mut world);
    let outcome = answer(&pending);
    let n = strict_id(&outcome);
    let ids = error_log_ids_since(n);
    for id in recorded {
        assert!(ids.contains(&id), "送った後の {id} が読める: {ids:?}");
    }
    assert!(!ids.contains(&earlier), "呼ぶ前の {earlier} は読めない");
}
