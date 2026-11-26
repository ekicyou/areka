use super::choice_tests::{
    MENU_SCRIPT, StateFixture, close_and_join, occupy, release, spawn_probe_talk, state_fixture,
};
use super::*;
use crate::test_log_capture::{assert_logged, assert_logged_event, capture};
use crate::test_support::{RecordingSink, test_system_vars};
use areka_sakura::contract::TalkEndReason;
use dola::TimeDelta;
use dola::time::testing::{at_ms, delta_ms};
use std::sync::mpsc;
use std::time::Duration;
use tracing::Level;

/// `base` を初回 Tick で刻み（続けて `extra_ticks` を打ち）、`ChoiceWaiting` を投函して、
/// kanade へ転送された 1 通を返す。
fn relay_choice_waiting(
    base: TimePoint,
    extra_ticks: &[TimePoint],
    display_end: TimePoint,
    timeout_directive: Option<TimeDelta>,
) -> KanadeMsg {
    let mut fx = state_fixture();
    let (probe, _obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(906);
    occupy(&mut fx.state, talk_id, probe);

    fx.feed(DispatcherMsg::Tick { now: base });
    for now in extra_ticks {
        fx.feed(DispatcherMsg::Tick { now: *now });
    }

    fx.feed(DispatcherMsg::ChoiceWaiting(ChoiceWaiting {
        talk_id,
        choice_ids: vec!["a".to_string(), "b".to_string()],
        display_end,
        timeout_directive,
    }));

    let msg = fx
        .kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("一致する ChoiceWaiting は kanade へ転送されるべき");
    assert!(
        fx.kanade_rx.try_recv().is_err(),
        "転送はちょうど 1 通（dispatcher は増幅しない）"
    );
    release(&mut fx.state);
    msg
}

/// 表示の終わり＝`base_now` ＋ 通知の隔たり。`base_now` は Tick 中継の起点（初回 Tick の
/// `now`）で、後続 Tick でも通知時刻でもない。候補 id 列とタイムアウト指令は無改変で運ばれる。
///
/// 隔たり 356.7ms はミリ秒の格子に載らないので、ミリ秒へ丸める実装なら落ちる。
/// 後続 Tick(9_000) を起点に取る実装なら 9_356.7ms になって落ちる。
#[test]
fn choice_waiting_adds_the_tick_base_back_to_display_end_and_forwards_to_kanade() {
    const OFFSET_TICKS: i64 = 3_567_000;
    match relay_choice_waiting(
        at_ms(4_000),
        &[at_ms(9_000)],
        TimePoint::from_ticks(OFFSET_TICKS),
        Some(delta_ms(12_000)),
    ) {
        KanadeMsg::ChoiceWaiting {
            talk_id,
            choice_ids,
            display_end,
            timeout_directive,
        } => {
            assert_eq!(talk_id, TalkId(906), "talk_id はエコーされる");
            assert_eq!(
                choice_ids,
                vec!["a".to_string(), "b".to_string()],
                "候補 id 列は表示順のまま無改変で運ばれる"
            );
            assert_eq!(
                display_end,
                TimePoint::from_ticks(at_ms(4_000).as_ticks() + OFFSET_TICKS),
                "base_now(4_000ms) + 356.7ms を丸めずに足す（後続 Tick 9_000 起点ではない）"
            );
            assert_eq!(
                timeout_directive,
                Some(delta_ms(12_000)),
                "タイムアウト指令は写像せず素通し（deadline 写像は kanade の領分・DD-8）"
            );
        }
        _ => unreachable!("ChoiceWaiting を投函したので ChoiceWaiting が届く"),
    }

    match relay_choice_waiting(at_ms(4_000), &[], at_ms(1_250), None) {
        KanadeMsg::ChoiceWaiting {
            display_end,
            timeout_directive,
            ..
        } => {
            assert_eq!(
                display_end,
                at_ms(5_250),
                "base_now(4_000) + 隔たり 1_250 = 5_250"
            );
            assert_eq!(
                timeout_directive, None,
                "未指定（None＝下流既定値へ委譲）も無改変で運ばれる"
            );
        }
        _ => unreachable!("ChoiceWaiting を投函したので ChoiceWaiting が届く"),
    }
}

/// `talk_id` 不一致／slot 空の通知は kanade へ転送されず、`choice_waiting_stale`（info）で
/// 記録して棄却される。
#[test]
fn choice_waiting_with_mismatched_or_empty_slot_is_discarded_as_stale() {
    let mut fx = state_fixture();
    let (probe, _obs_rx) = spawn_probe_talk();
    occupy(&mut fx.state, TalkId(907), probe);
    fx.feed(DispatcherMsg::Tick { now: at_ms(1_000) });

    let events = capture(|| {
        fx.feed(DispatcherMsg::ChoiceWaiting(ChoiceWaiting {
            talk_id: TalkId(999),
            choice_ids: vec!["a".to_string()],
            display_end: at_ms(500),
            timeout_directive: None,
        }));
    });
    assert_logged_event(
        &events,
        Level::INFO,
        "areka_ghost::dispatcher",
        "choice_waiting_stale",
    );
    assert!(
        fx.kanade_rx.try_recv().is_err(),
        "不一致 ChoiceWaiting は kanade へ転送されない"
    );

    release(&mut fx.state);
    let events = capture(|| {
        fx.feed(DispatcherMsg::ChoiceWaiting(ChoiceWaiting {
            talk_id: TalkId(907),
            choice_ids: vec!["a".to_string()],
            display_end: at_ms(500),
            timeout_directive: None,
        }));
    });
    assert_logged_event(
        &events,
        Level::INFO,
        "areka_ghost::dispatcher",
        "choice_waiting_stale",
    );
    assert!(
        fx.kanade_rx.try_recv().is_err(),
        "slot 空でも kanade へ転送されない"
    );
}

/// Tick の前に通知が出ることは本来ない（通知は tick 駆動のバリア到達で出る）。それでも起点の欠如を
/// 黙って埋めず `warn` で記録し、kanade へは転送しない（作った起点で誤った deadline を配らない）。
#[test]
fn choice_waiting_before_any_tick_is_defended_with_warning_and_not_forwarded() {
    let mut fx = state_fixture();
    let (probe, _obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(908);
    occupy(&mut fx.state, talk_id, probe);

    let events = capture(|| {
        fx.feed(DispatcherMsg::ChoiceWaiting(ChoiceWaiting {
            talk_id,
            choice_ids: vec!["a".to_string()],
            display_end: at_ms(500),
            timeout_directive: None,
        }));
    });
    assert_logged_event(
        &events,
        Level::WARN,
        "areka_ghost::dispatcher",
        "choice_waiting_stale",
    );
    assert!(
        fx.kanade_rx.try_recv().is_err(),
        "起点未確定の通知は kanade へ転送しない"
    );

    release(&mut fx.state);
}

/// kanade が既に停止していると転送の `send` は失敗する。dispatcher は黙って捨てず `debug` で
/// 記録し、slot を乱さず続ける。
///
/// `kanade_rx` を投函前に drop するので送出は必ず `Err` になる。`feed` を使わず
/// [`DispatcherState::handle`] を直に呼ぶのは、fixture から受信端だけを取り出して drop するため。
#[test]
fn choice_waiting_forward_failure_after_kanade_stopped_is_recorded_at_debug_and_continues() {
    let StateFixture {
        mut state,
        kanade_rx,
        _self_rx,
    } = state_fixture();
    let (probe, _obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(910);
    occupy(&mut state, talk_id, probe);

    // base_now を刻み、換算アームまで届かせる（warn の防御枝にしない）。
    assert_eq!(
        state.handle(DispatcherMsg::Tick { now: at_ms(3_000) }),
        ControlFlow::Continue(())
    );
    drop(kanade_rx);

    let events = capture(|| {
        assert_eq!(
            state.handle(DispatcherMsg::ChoiceWaiting(ChoiceWaiting {
                talk_id,
                choice_ids: vec!["a".to_string()],
                display_end: at_ms(500),
                timeout_directive: None,
            })),
            ControlFlow::Continue(()),
            "転送失敗は dispatcher を停止させない（停止経路は Close のみ）"
        );
    });
    assert_logged(
        &events,
        Level::DEBUG,
        "areka_ghost::dispatcher",
        "dropping ChoiceWaiting forward",
    );
    assert_eq!(
        state.current_talk_id(),
        Some(talk_id),
        "転送失敗は slot を乱さない（talk は選択待ちのまま継続する）"
    );

    release(&mut state);
}

/// 実 talk が選択待ちバリアへ到達すると `ChoiceWaiting` が dispatcher 経由で kanade へ届き、
/// `display_end` は base_now ＋ 台本由来の占有 horizon（tick 時刻ではない）になる。続く
/// `ResolveChoice` は talk のバリアを解き、menu ケースなのでその場で完了して
/// `TalkDone{Ended}` が kanade へ届く。
#[test]
fn menu_talk_choice_waiting_reaches_kanade_and_resolve_resumes_it_to_completion() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![
            Box::new(RecordingSink::new()),
            Box::new(RecordingSink::new()),
        ],
        test_system_vars(),
        None,
    );

    let talk_id = TalkId(931);
    tx.send(DispatcherMsg::Start(StartTalk::new(talk_id, MENU_SCRIPT)))
        .expect("send Start(menu)");
    // base_now=1_000 を刻み、elapsed 0.5 で区切り 0.35 に届いて選択待ちになる。
    tx.send(DispatcherMsg::Tick { now: at_ms(1_000) })
        .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick { now: at_ms(1_500) })
        .expect("send Tick(base+500ms)");

    match kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("バリア成立で ChoiceWaiting が kanade へ届くべき")
    {
        KanadeMsg::ChoiceWaiting {
            talk_id: got,
            choice_ids,
            display_end,
            timeout_directive,
        } => {
            assert_eq!(got, talk_id);
            assert_eq!(choice_ids, vec!["targetA".to_string()]);
            assert_eq!(
                display_end,
                at_ms(1_350),
                "base_now(1_000) + 占有 horizon 350ms。**tick 時刻 1_500 ではない**"
            );
            assert_eq!(
                timeout_directive, None,
                "compile は未指定を書く（下流既定値へ委譲・DD-8）"
            );
        }
        _ => unreachable!("バリア成立で最初に届くのは ChoiceWaiting"),
    }

    tx.send(DispatcherMsg::ResolveChoice {
        talk_id,
        id: "targetA".to_string(),
    })
    .expect("send ResolveChoice");

    match kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("解決で再開した talk の TalkDone が kanade へ届くべき")
    {
        KanadeMsg::TalkDone(done) => {
            assert_eq!(done.talk_id, talk_id);
            assert_eq!(
                done.reason,
                TalkEndReason::Ended,
                "解決後は台本どおり自然終端（中断ではない）"
            );
        }
        _ => unreachable!("解決後に届くのは TalkDone"),
    }

    close_and_join(&tx, handle);
}

/// 選択待ち中の `CancelChoice` は Close funnel を通って talk を終了させ、talk 発の
/// `TalkDone{Interrupted}` が kanade へ届く。
///
/// dispatcher が `close_active_if_any`（slot の先行解放）を使う実装なら、返ってきた `TalkDone` は
/// stale として棄却され、この recv が timeout で落ちる。
#[test]
fn menu_talk_cancel_choice_ends_talk_and_interrupted_talkdone_reaches_kanade() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![
            Box::new(RecordingSink::new()),
            Box::new(RecordingSink::new()),
        ],
        test_system_vars(),
        None,
    );

    let talk_id = TalkId(932);
    tx.send(DispatcherMsg::Start(StartTalk::new(talk_id, MENU_SCRIPT)))
        .expect("send Start(menu)");
    tx.send(DispatcherMsg::Tick { now: at_ms(2_000) })
        .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick { now: at_ms(2_500) })
        .expect("send Tick(base+500ms)");

    // Cancel の前提の選択待ちが成立するのを、通知の到着で待つ。
    assert!(
        matches!(
            kanade_rx.recv_timeout(Duration::from_secs(5)),
            Ok(KanadeMsg::ChoiceWaiting { .. })
        ),
        "Cancel の前提として選択待ちが成立していること"
    );

    tx.send(DispatcherMsg::CancelChoice { talk_id })
        .expect("send CancelChoice");

    match kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("解除で終了した talk の TalkDone{Interrupted} が kanade へ届くべき（DD-11）")
    {
        KanadeMsg::TalkDone(done) => {
            assert_eq!(done.talk_id, talk_id);
            assert_eq!(
                done.reason,
                TalkEndReason::Interrupted,
                "解除は Close funnel 経由の中断（talk が理由を確定する）"
            );
        }
        _ => unreachable!("解除後に届くのは TalkDone"),
    }

    close_and_join(&tx, handle);
}
