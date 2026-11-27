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

/// `ticks` を順に打ってから `ChoiceWaiting` を投函して、kanade へ転送された 1 通を返す。
fn relay_choice_waiting(
    ticks: &[TimePoint],
    display_end: TimePoint,
    timeout_directive: Option<TimeDelta>,
) -> KanadeMsg {
    let mut fx = state_fixture();
    let (probe, _obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(906);
    occupy(&mut fx.state, talk_id, probe);

    for now in ticks {
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

/// 表示の終わりは、通知の値（絶対の時刻）を換算せずに kanade へ渡す。先に受けた刻みの時刻に
/// 依らない。候補 id 列とタイムアウト指令も無改変で運ばれる。
///
/// 4_356.7ms はミリ秒の格子に載らないので、ミリ秒へ丸める実装なら落ちる。
/// 最初の刻み(4_000) を足す実装なら 8_356.7ms になって落ちる。
#[test]
fn choice_waiting_forwards_display_end_unchanged_to_kanade() {
    let display_end_in = TimePoint::from_ticks(at_ms(4_000).as_ticks() + 3_567_000);
    match relay_choice_waiting(
        &[at_ms(4_000), at_ms(9_000)],
        display_end_in,
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
                display_end, display_end_in,
                "通知の表示の終わりを丸めず換算せずに渡す"
            );
            assert_eq!(
                timeout_directive,
                Some(delta_ms(12_000)),
                "タイムアウト指令は写像せず素通し（deadline 写像は kanade の領分・DD-8）"
            );
        }
        _ => unreachable!("ChoiceWaiting を投函したので ChoiceWaiting が届く"),
    }

    match relay_choice_waiting(&[at_ms(4_000)], at_ms(5_250), None) {
        KanadeMsg::ChoiceWaiting {
            display_end,
            timeout_directive,
            ..
        } => {
            assert_eq!(display_end, at_ms(5_250), "通知の 5_250 がそのまま着く");
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

/// 転送は dispatcher が受けた刻みに依らないので、刻みを 1 つも受けていなくても通知はそのまま着く。
#[test]
fn choice_waiting_before_any_tick_is_forwarded_unchanged() {
    match relay_choice_waiting(&[], at_ms(500), None) {
        KanadeMsg::ChoiceWaiting { display_end, .. } => assert_eq!(
            display_end,
            at_ms(500),
            "刻みを受ける前の通知も、表示の終わりを換算せずに渡す"
        ),
        _ => unreachable!("ChoiceWaiting を投函したので ChoiceWaiting が届く"),
    }
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
/// `display_end` は最初の刻み（台本の起点）＋ 台本由来の占有 horizon（tick 時刻ではない）になる。続く
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
    // 最初の刻み 1_000 が台本の起点になり、1_500 で区切り 0.35 に届いて選択待ちになる。
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
                "起点 1_000 + 占有 horizon 350ms。**tick 時刻 1_500 ではない**"
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
