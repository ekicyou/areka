//! `get_status` の決定論テスト（design Testing Strategy M1〜M5）。
//!
//! 置き場に偽の kanade の送り口（`mpsc` の送出端）を置き、受信端で問い合わせを受け取って
//! 返信端へ値を送る（または落とす）。答えは後から答える置き場（`later`）を `drain` で覗いて取る。

use std::path::PathBuf;
use std::sync::mpsc;

use areka_kanade::{BalloonBinding, ExecutionSnapshot};
use areka_mcp::tools::{Pending, ToolCall, ToolRequest};
use areka_mcp::{ToolContent, ToolOutcome};
use log_capture_kit::{CapturedEvent, capture, count_levels};

use super::resolve::CANNOT_FIND;
use super::*;
use crate::ghost_session::GhostSession;
use crate::mcp::{McpLater, drain, install};

fn ghost() -> ActiveGhost {
    ActiveGhost {
        name: Some("Emily/Phase4.5".to_string()),
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
fn call(world: &mut World, ghost_name: Option<&str>) -> (Pending, Vec<CapturedEvent>) {
    let args = Args {
        ghost_name: ghost_name.map(str::to_string),
    };
    let (req, pending) = ToolRequest::new(ToolCall::GetStatus(args.clone()));
    let ((), events) = capture(|| handle(world, &ghost(), args, req.reply));
    (pending, events)
}

/// 偽の受信端に届いた問い合わせの返信端（`StatusQuery` 以外が届いたら落ちる）。
fn take_query(rx: &mpsc::Receiver<KanadeMsg>) -> areka_actor::ReplySender<ExecutionStatus> {
    match rx.try_recv() {
        Ok(KanadeMsg::StatusQuery { reply }) => reply,
        other => panic!(
            "問い合わせ 1 通が届いているはず: {:?}",
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

/// 値を送って 1 フレーム回した後の答え（その間の記録の warn 以上も返す）。
fn answer_with(status: ExecutionStatus) -> (ToolOutcome, usize) {
    let (mut world, rx) = rig();
    let (pending, called) = call(&mut world, Some("Emily/Phase4.5"));
    take_query(&rx)
        .send(status)
        .ok()
        .expect("返信端は生きている");
    let ((), polled) = capture(|| drain(&mut world));
    (
        answer(&pending),
        warn_or_worse(&called) + warn_or_worse(&polled),
    )
}

/// M1: 値なし（どの状態も立っていない）→ 空の本文・isError: false（要件 1.3）。
#[test]
fn m1_no_state_is_an_empty_body() {
    let (outcome, warns) = answer_with(ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE));
    assert_eq!(outcome.content, vec![ToolContent::Text(String::new())]);
    assert!(!outcome.is_error);
    assert_eq!(warns, 0);
}

/// M2: 再生中＋バルーン 2 つ → `render()` の値と一字違わず同じ本文 1 つ（要件 1.1・1.6・5.3）。
#[test]
fn m2_states_are_the_rendered_value_verbatim() {
    let status = ExecutionStatus::derive(&ExecutionSnapshot {
        talk_active: true,
        balloons: vec![
            BalloonBinding {
                character_id: 1,
                balloon_id: 0,
            },
            BalloonBinding {
                character_id: 0,
                balloon_id: 0,
            },
        ],
        ..ExecutionSnapshot::INACTIVE
    });
    let rendered = status.render().expect("状態が立っている");
    assert_eq!(rendered, "talking,balloon(0=0/1=0)");

    let (outcome, warns) = answer_with(status);
    assert_eq!(outcome.content, vec![ToolContent::Text(rendered)]);
    assert!(!outcome.is_error);
    assert_eq!(warns, 0);
}

/// M3: 答える前に降りた（`ghost_name` の有無 × 返信端を落とす／送れない）→ 宛先の解決の失敗と
/// 同じ文言・isError: true・warn 以上は 0 件（要件 3.2・3.5）。
#[test]
fn m3_gone_before_answering_reads_as_the_resolve_failure() {
    for (ghost_name, expected) in [(Some("Emily/Phase4.5"), CANNOT_FIND), (None, NOT_ACTIVE)] {
        // 返信端を落とす。
        let (mut world, rx) = rig();
        let (pending, called) = call(&mut world, ghost_name);
        drop(take_query(&rx));
        let ((), polled) = capture(|| drain(&mut world));
        let outcome = answer(&pending);
        assert_eq!(
            outcome.content,
            vec![ToolContent::Text(format!("NG:{expected}"))],
            "返信端を落とす・{ghost_name:?}"
        );
        assert!(outcome.is_error, "返信端を落とす・{ghost_name:?}");
        assert_eq!(
            warn_or_worse(&called) + warn_or_worse(&polled),
            0,
            "返信端を落とす・{ghost_name:?}: {called:?} {polled:?}"
        );

        // 受信端を先に落として送れなくする（その場で答える）。
        let (mut world, rx) = rig();
        drop(rx);
        let (pending, called) = call(&mut world, ghost_name);
        let outcome = answer(&pending);
        assert_eq!(
            outcome.content,
            vec![ToolContent::Text(format!("NG:{expected}"))],
            "送れない・{ghost_name:?}"
        );
        assert!(outcome.is_error, "送れない・{ghost_name:?}");
        assert_eq!(
            warn_or_worse(&called),
            0,
            "送れない・{ghost_name:?}: {called:?}"
        );
        assert!(
            world.non_send::<McpLater>().0.is_empty(),
            "送れない・預けない"
        );
    }
}

/// M4: 呼んだ直後は答えが無く、預けた組が 1 つ・届いたのは問い合わせ 1 通だけ。返信端へ送る前に
/// 回しても答えは出ず預けたまま。成功の枝で warn 以上は 0 件（要件 4.1・4.3・4.4）。
#[test]
fn m4_does_not_wait_and_sends_exactly_one_query() {
    let (mut world, rx) = rig();
    let (pending, called) = call(&mut world, None);

    assert!(matches!(pending.try_answer(), Ok(None)), "呼んだ直後は未着");
    assert_eq!(world.non_send::<McpLater>().0.len(), 1);
    let reply = take_query(&rx);
    assert!(rx.try_recv().is_err(), "問い合わせは 1 通だけ");

    let ((), before) = capture(|| drain(&mut world));
    assert!(matches!(pending.try_answer(), Ok(None)), "返事の前は未着");
    assert_eq!(world.non_send::<McpLater>().0.len(), 1, "預けたまま");

    reply
        .send(ExecutionStatus::derive(&ExecutionSnapshot::INACTIVE))
        .ok()
        .expect("返信端は生きている");
    let ((), after) = capture(|| drain(&mut world));
    assert!(!answer(&pending).is_error);
    assert!(world.non_send::<McpLater>().0.is_empty());
    assert_eq!(
        warn_or_worse(&called) + warn_or_worse(&before) + warn_or_worse(&after),
        0
    );
}

/// M5: 置き場に問い合わせ先が無い（空の World）→ `NG:Status is not available`・isError: true・
/// warn がちょうど 1 件・error は 0 件（要件 3.6）。
#[test]
fn m5_answers_not_available_without_a_kanade_in_the_slot() {
    let args = Args {
        ghost_name: Some("Emily/Phase4.5".to_string()),
    };
    let (req, pending) = ToolRequest::new(ToolCall::GetStatus(args.clone()));

    let ((), levels) = count_levels(|| handle(&mut World::new(), &ghost(), args, req.reply));

    let outcome = answer(&pending);
    assert_eq!(
        outcome.content,
        vec![ToolContent::Text("NG:Status is not available".to_string())]
    );
    assert!(outcome.is_error);
    assert_eq!((levels.warn, levels.error), (1, 0));
}
