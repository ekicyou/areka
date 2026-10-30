//! `sakurascript` の決定論テスト（design Testing Strategy 要件 7.2）。
//!
//! 置き場に偽の kanade の送り口（`mpsc` の送出端）を置き、受信端で外からの台本の知らせを受け取って
//! 返信端へ結果を送る（または落とす）。答えは後から答える置き場（`later`）を `drain` で覗いて取る。

use std::path::PathBuf;
use std::sync::mpsc;

use areka_actor::ReplySender;
use areka_mcp::tools::{Pending, ToolCall, ToolRequest};
use areka_mcp::{ToolContent, ToolOutcome};
use log_capture_kit::{CapturedEvent, capture, count_levels};

use super::resolve::{CANNOT_FIND, NOT_ACTIVE};
use super::*;
use crate::ghost_session::GhostSession;
use crate::mcp::{McpLater, drain, install};

const SCRIPT: &str = r"\0\s[0]hello\e";

fn ghost() -> ActiveGhost {
    ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
        sakura_name: None,
        root: PathBuf::from(r"C:\ssp\ghost\emily4"),
    }
}

/// 受け口と後から答える置き場を据え、置き場に偽の kanade の送り口を置いた World と、その受信端。
fn rig() -> (World, mpsc::Receiver<KanadeMsg>) {
    let mut world = World::new();
    let (_inbox_tx, inbox_rx) = mpsc::channel();
    install(&mut world, inbox_rx);
    let (tx, rx) = mpsc::channel();
    world.insert_non_send(GhostSlot(Some(GhostSession::for_test(
        Some(tx),
        ghost().root,
    ))));
    (world, rx)
}

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

fn answer(pending: &Pending) -> ToolOutcome {
    pending
        .try_answer()
        .ok()
        .flatten()
        .expect("答えが届いている")
        .outcome
}

fn warn_or_worse(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|ev| ev.level <= tracing::Level::WARN)
        .count()
}

fn text(s: &str) -> Vec<ToolContent> {
    vec![ToolContent::Text(s.to_string())]
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
        warn_or_worse(&called) + warn_or_worse(&polled),
    )
}

/// 受け付けた → `OK`・isError: false・warn 以上 0 件（要件 1.1・6.5）。
#[test]
fn accepted_is_ok() {
    let (outcome, warns) = answer_with(Some("Emily/Phase4.5"), SstpSendOutcome::Accepted);
    assert_eq!(outcome.content, text("OK"));
    assert!(!outcome.is_error);
    assert_eq!(warns, 0);
}

/// 受け付けなかった → `NG:the ghost cannot accept a script now`・warn 以上 0 件（要件 3.6・3.9）。
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

/// 失敗・返信端の落ち・送れなかった → `ghost_name` の有無で宛先が居ないときの 2 通りの `NG:`・
/// warn 以上 0 件（要件 1.7・3.5・3.9・6.4）。
#[test]
fn failed_dropped_and_unsendable_read_as_the_resolve_failure() {
    for (ghost_name, expected) in [(Some("Emily/Phase4.5"), CANNOT_FIND), (None, NOT_ACTIVE)] {
        let want = text(&format!("NG:{expected}"));

        let (outcome, warns) = answer_with(ghost_name, SstpSendOutcome::Failed);
        assert_eq!(outcome.content, want, "Failed・{ghost_name:?}");
        assert!(outcome.is_error, "Failed・{ghost_name:?}");
        assert_eq!(warns, 0, "Failed・{ghost_name:?}");

        // 返信端を落とす。
        let (mut world, rx) = rig();
        let (pending, called) = call(&mut world, SCRIPT, ghost_name, None);
        drop(take_send(&rx));
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
        let (pending, called) = call(&mut world, SCRIPT, ghost_name, None);
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

/// 台本と strict の真・偽・省略がそのまま知らせに載る（要件 1.3・1.5）。
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

/// 空文字の台本 → kanade へ何も送らず `NG:script is empty`（要件 1.4）。空白だけは空と数えない。
#[test]
fn empty_script_sends_nothing() {
    let (mut world, rx) = rig();
    let (pending, called) = call(&mut world, "", None, Some(true));
    let outcome = answer(&pending);
    assert_eq!(outcome.content, text("NG:script is empty"));
    assert!(outcome.is_error);
    assert!(rx.try_recv().is_err(), "kanade に何も届かない");
    assert!(world.non_send::<McpLater>().0.is_empty());
    assert_eq!(warn_or_worse(&called), 0);

    let (mut world, rx) = rig();
    let _ = call(&mut world, " ", None, None);
    assert_eq!(take_send(&rx).0, " ", "空白だけは送る");
}

/// 返事が呼んだ直後（最初の覗きの前）に届いていても、何フレームか後に届いても答えになる（要件 6.4）。
#[test]
fn answer_may_be_ready_at_once_or_arrive_later() {
    // その場で返る（最初の覗きで答える）。
    let (mut world, rx) = rig();
    let (pending, _) = call(&mut world, SCRIPT, None, None);
    take_send(&rx)
        .2
        .send(SstpSendOutcome::Accepted)
        .expect("返信端は生きている");
    drain(&mut world);
    assert_eq!(answer(&pending).content, text("OK"));

    // 後から届く（返事の前の覗きでは答えず預けたまま）。
    let (mut world, rx) = rig();
    let (pending, _) = call(&mut world, SCRIPT, None, None);
    assert!(matches!(pending.try_answer(), Ok(None)), "呼んだ直後は未着");
    let (_, _, reply) = take_send(&rx);
    drain(&mut world);
    drain(&mut world);
    assert!(matches!(pending.try_answer(), Ok(None)), "返事の前は未着");
    assert_eq!(world.non_send::<McpLater>().0.len(), 1, "預けたまま");
    reply
        .send(SstpSendOutcome::Accepted)
        .expect("返信端は生きている");
    drain(&mut world);
    assert_eq!(answer(&pending).content, text("OK"));
    assert!(world.non_send::<McpLater>().0.is_empty());
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

/// 2 本で使う部品: 送り口と「降りた」の文言（`ghost_name` の有無で 2 通り）。
#[test]
fn script_target_reads_the_sender_and_the_gone_wording() {
    let (world, _rx) = rig();
    let target = script_target(&world, Some("Emily/Phase4.5"));
    assert!(target.kanade.is_some());
    assert_eq!(target.gone, CANNOT_FIND);

    let target = script_target(&World::new(), None);
    assert!(target.kanade.is_none());
    assert_eq!(target.gone, NOT_ACTIVE);
}

/// strict の答えの文の前後（`<n>` を挟む・要件 1.1）。
const STRICT_HEAD: &str = "OK:strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=";

/// 送った後・返事の前に記録を 1 件積み、その番号を返す（答えの `<n>` が送る前の値かを測る）。
fn record_one() -> u64 {
    crate::log_history::record(
        crate::log_history::Stamp {
            year: 2026,
            month: 10,
            day: 30,
            hour: 12,
            minute: 0,
        },
        crate::log_history::Draft {
            kind: crate::log_history::Kind::Error,
            label: "strict".to_string(),
            name: String::new(),
            body: "test".to_string(),
        },
    )
}

/// strict 真で受け付けた → strict の文・`<n>` は送る前に読んだ最後の番号（要件 1.1・1.2・7.4）。
///
/// 置き場はプロセスに 1 つで他のテストも積むので、`<n>` は「呼ぶ前の番号以上・送った後に積んだ番号未満」で測る。
#[test]
fn strict_true_answer_carries_the_id_read_before_sending() {
    let (mut world, rx) = rig();
    let before = crate::log_history::last_id();
    let (pending, called) = call(&mut world, SCRIPT, None, Some(true));
    let (_, _, reply) = take_send(&rx);
    let recorded = record_one();
    reply
        .send(SstpSendOutcome::Accepted)
        .expect("返信端は生きている");
    let ((), polled) = capture(|| drain(&mut world));
    let outcome = answer(&pending);
    assert!(!outcome.is_error);
    assert_eq!(warn_or_worse(&called) + warn_or_worse(&polled), 0);
    let [ToolContent::Text(body)] = outcome.content.as_slice() else {
        panic!("本文 1 つ: {:?}", outcome.content);
    };
    let n: u64 = body
        .strip_prefix(STRICT_HEAD)
        .and_then(|rest| rest.strip_suffix(')'))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("strict の文のはず: {body}"));
    assert!(
        before <= n && n < recorded,
        "送る前の番号: {before} <= {n} < {recorded}"
    );
}

/// strict 偽・省略で受け付けた → `OK` のまま（要件 1.4）。
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

/// strict 真でも NG の 4 つ（空・受け付けない・失敗・返信端の落ち）は今の答えのまま（要件 1.5）。
#[test]
fn strict_true_ng_answers_are_unchanged() {
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

/// 捕まえた出来事のうち、エラーログに積まれる strict の記録（宛先が `RECORD_TARGET` で `point` の欄を持つ）の数。
fn strict_records(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|ev| ev.target == areka_sakura::strict::RECORD_TARGET)
        .filter(|ev| ev.fields.iter().any(|(name, _)| name == "point"))
        .count()
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
    crate::mcp::get_log::handle(&mut World::new(), args, req.reply);
    let outcome = answer(&pending);
    assert!(!outcome.is_error);
    let [ToolContent::Text(body)] = outcome.content.as_slice() else {
        panic!("本文 1 つ: {:?}", outcome.content);
    };
    body.split("\r\n")
        .filter_map(|line| line.strip_prefix('#')?.split(' ').next()?.parse().ok())
        .collect()
}

/// 答えの `<n>` で `get_log(log_type=error, since_id=<n>)` を読むと、送った後に積んだ記録が全部読め、
/// 呼ぶ前に積んだ記録は読めない（要件 1.3・7.4）。
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
    let [ToolContent::Text(body)] = outcome.content.as_slice() else {
        panic!("本文 1 つ: {:?}", outcome.content);
    };
    let n: u64 = body
        .strip_prefix(STRICT_HEAD)
        .and_then(|rest| rest.strip_suffix(')'))
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("strict の文のはず: {body}"));
    let ids = error_log_ids_since(n);
    for id in recorded {
        assert!(ids.contains(&id), "送った後の {id} が読める: {ids:?}");
    }
    assert!(!ids.contains(&earlier), "呼ぶ前の {earlier} は読めない");
}

/// NG の 4 つ（空・受け付けない・居ない・降りた）を strict 真で、受け付けたのを strict 偽・省略で
/// → エラーログに strict の記録が増えない（要件 1.5・3.5・7.5）。記録は呼んだ・回したスレッドで拾う。
#[test]
fn no_strict_records_for_ng_answers_or_strict_false() {
    let (mut world, _rx) = rig();
    let (pending, called) = call(&mut world, "", None, Some(true));
    let _ = answer(&pending);
    assert_eq!(strict_records(&called), 0, "空");

    let (mut world, rx) = rig();
    drop(rx);
    let (pending, called) = call(&mut world, SCRIPT, None, Some(true));
    let _ = answer(&pending);
    assert_eq!(strict_records(&called), 0, "送れない");

    let cases = [
        (
            Some(true),
            Some(SstpSendOutcome::NotAccepted),
            "受け付けない",
        ),
        (Some(true), Some(SstpSendOutcome::Failed), "居ない"),
        (Some(true), None, "降りた"),
        (Some(false), Some(SstpSendOutcome::Accepted), "strict 偽"),
        (None, Some(SstpSendOutcome::Accepted), "strict 省略"),
    ];
    for (strict, result, label) in cases {
        let (mut world, rx) = rig();
        let (pending, called) = call(&mut world, SCRIPT, None, strict);
        let (_, _, reply) = take_send(&rx);
        match result {
            Some(r) => reply.send(r).expect("返信端は生きている"),
            None => drop(reply),
        }
        let ((), polled) = capture(|| drain(&mut world));
        let _ = answer(&pending);
        assert_eq!(
            strict_records(&called) + strict_records(&polled),
            0,
            "{label}"
        );
    }
}
