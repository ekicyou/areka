use super::test_support::*;
use super::*;
use crate::contract::TalkId;
use dola::TimeDelta;
use dola::time::testing::{at_ms, delta_ms};
use std::sync::mpsc;
use std::time::Duration;

// ── 時計を一度に進めても選択待ちに入る ──
//
// 台本 `\s[10]hello\_w[100]\q[選択A,targetA]\![set,choicetimeout,X]\e`（区切り＝占有 horizon 350ms）を
// `Tick` の次に `Tick` で駆動し、**区切りと指定の時間を一度に越える**。再生層の時刻表が
// 指定の値で区切りを飛ばすなら、選択待ちの知らせは届かず台本が終わってしまう（この檻が落ちる）。

/// 1 つの指定で台本を流し、時計を一度に進めて、選択待ちの知らせと終わらないことを確かめる。
fn assert_one_step_enters_choice_wait(
    directive: &str,
    expected: Option<TimeDelta>,
    talk_id: TalkId,
) {
    let script = format!(r"\s[10]hello\_w[100]\q[選択A,targetA]{directive}\e");
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let handle = spawn_talk(
        StartTalk::new(talk_id, script),
        done_tx,
        one_sink(NoopSink),
        SystemVarSnapshot::default(),
        None,
    );

    // アンカー 0.0 を刻印し、区切り＋指定の時間を遥かに越える 50.0 まで一度に進める。
    handle.inbox.send(SakuraMsg::Tick(at_ms(0))).unwrap();
    handle.inbox.send(SakuraMsg::Tick(at_ms(50000))).unwrap();

    // 1 通目は選択待ちの知らせ（値に依らず選択待ちに入る）。
    let notice = done_rx
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_else(|e| panic!("{directive:?}: 選択待ちの知らせが届くべき（{e:?}）"));
    assert_eq!(
        notice,
        TalkNotice::ChoiceWaiting(ChoiceWaiting {
            talk_id,
            choice_ids: vec!["targetA".to_string()],
            display_end: at_ms(0) + menu_horizon(),
            timeout_directive: expected,
        }),
        "{directive:?}: 表示の終わりは区切りの時刻（tick 時刻 50000ms ではない）・指令は指定どおり"
    );

    // さらに進めても台本は終わらず、知らせの 2 通目も来ない（区切りを自分で解かない）。
    handle.inbox.send(SakuraMsg::Tick(at_ms(100000))).unwrap();
    assert!(
        done_rx.recv_timeout(NEG_WINDOW).is_err(),
        "{directive:?}: 選択待ちの間は TalkDone も 2 通目の ChoiceWaiting も来ない"
    );
    assert!(
        !handle.actor.is_finished(),
        "{directive:?}: 選択待ちの間 talk は続く"
    );

    // 片付け: Close で中断 ACK を取り body を畳む。
    handle.inbox.send(SakuraMsg::Close).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5)).expect("Close で中断 ACK");
    assert_eq!(done.reason, TalkEndReason::Interrupted);
    handle.actor.join().expect("body は正常終了する");
}

/// 指定 `500`・`0`・`-1`・時間の省略のどれでも、時計を一度に進めて
/// 選択待ちに入り、知らせは 1 通だけ・指令はそれぞれ `Some`・`Some`・`Some`・`None`。
#[test]
fn one_step_clock_jump_enters_choice_wait_for_each_directive() {
    let cases: [(&str, Option<TimeDelta>, u64); 4] = [
        (r"\![set,choicetimeout,500]", Some(delta_ms(500)), 830),
        (r"\![set,choicetimeout,0]", Some(delta_ms(0)), 831),
        (r"\![set,choicetimeout,-1]", Some(delta_ms(-1)), 832),
        (r"\![set,choicetimeout]", None, 833),
    ];
    for (directive, expected, id) in cases {
        assert_one_step_enters_choice_wait(directive, expected, TalkId(id));
    }
}
