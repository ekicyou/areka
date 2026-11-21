use super::*;
use crate::test_log_capture::{assert_logged, assert_logged_event, capture};
use crate::test_support::{RecordingSink, run_bounded, test_system_vars};
use areka_kanade::{AnchorInput, ChoiceInput};
use areka_sakura::contract::TalkEndReason;
use std::sync::mpsc;
use std::time::Duration;
use tracing::Level;

// ── 選択系 3 アームの中継意味論と時刻換算 ──
//
// - 同期 state の檻（[`state_fixture`]＋[`spawn_probe_talk`]）: [`DispatcherState::handle`] を
//   テストスレッド上で直接駆動し、talk スタンドインが受けた [`SakuraMsg`] をそのまま突合する。
//   棄却・防御アームのログは [`capture`](crate::test_log_capture::capture) で見る。捕捉窓は
//   呼出スレッド局所で actor スレッドのログは載らないので、ログの語彙はこの段でしか固定できない。
// - actor e2e の檻（[`spawn_dispatcher`]＋実 talk）: 実再生層を通した往復（選択待ち成立通知の
//   ms 換算・解決による再開・Close funnel の中断 ACK 帰還）を固定する。

/// 中継先 talk のスタンドイン（probe）。dispatcher が送った [`SakuraMsg`] を観測チャンネルへ
/// 素通しするだけの actor で、実再生を挟まないので中継の送出内容をそのまま突合できる。
/// `Close` を受けたら（実 talk 同様）停止する。
fn spawn_probe_talk() -> (TalkHandle, mpsc::Receiver<SakuraMsg>) {
    let (obs_tx, obs_rx) = mpsc::channel::<SakuraMsg>();
    let (inbox, actor) = spawn_actor::<SakuraMsg, _>("probe-talk", move |rx| {
        for msg in rx {
            let is_close = matches!(msg, SakuraMsg::Close);
            let _ = obs_tx.send(msg);
            if is_close {
                break;
            }
        }
    });
    (TalkHandle { inbox, actor }, obs_rx)
}

/// 既に消滅した talk のスタンドイン（中継の送出失敗を再現する）。
///
/// body は `rx` を drop してから合図を送るので、合図の後に inbox へ送ると必ず `Err` になる。
fn spawn_vanished_talk() -> TalkHandle {
    let (gone_tx, gone_rx) = mpsc::channel::<()>();
    let (inbox, actor) = spawn_actor::<SakuraMsg, _>("probe-talk-vanished", move |rx| {
        drop(rx);
        let _ = gone_tx.send(());
    });
    gone_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("probe talk は rx を drop した合図を送る");
    TalkHandle { inbox, actor }
}

/// 同期駆動用の dispatcher 状態の fixture。
struct StateFixture {
    state: DispatcherState,
    kanade_rx: mpsc::Receiver<KanadeMsg>,
    /// dispatcher 自身の inbox 受信端（`self_sender` の相方・保持しないと送出が切断される）。
    _self_rx: mpsc::Receiver<DispatcherMsg>,
}

fn state_fixture() -> StateFixture {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (self_tx, _self_rx) = mpsc::channel::<DispatcherMsg>();
    StateFixture {
        state: DispatcherState {
            active: None,
            kanade: kanade_tx,
            sinks: Vec::new(),
            system_vars: test_system_vars(),
            strict: None,
            self_sender: self_tx,
        },
        kanade_rx,
        _self_rx,
    }
}

impl StateFixture {
    /// 1 メッセージを同期処理する。選択系 3 アームは（`Tick`/`Done` と同じく）dispatcher を
    /// 停止させないので、`Continue` であることを毎回固定する（停止経路は `Close` だけ）。
    fn feed(&mut self, msg: DispatcherMsg) {
        assert_eq!(
            self.state.handle(msg),
            ControlFlow::Continue(()),
            "選択系・Tick・Done の各アームは dispatcher を停止させない（停止経路は Close のみ）"
        );
    }
}

/// slot へ talk スタンドインを据える（`base_now` は Tick が未着なら `None`）。
fn occupy(state: &mut DispatcherState, talk_id: TalkId, handle: TalkHandle) {
    state.active = Some(ActiveTalk {
        talk_id,
        handle,
        base_now: None,
    });
}

/// slot に残ったスタンドインを畳む。
fn release(state: &mut DispatcherState) {
    if let Some(active) = state.active.take() {
        let _ = active.handle.inbox.send(SakuraMsg::Close);
        let _ = active.handle.actor.join();
    }
}

/// dispatcher へ `Close` を送り、有界の時間で join する（dispatcher のテストで共有）。
pub(super) fn close_and_join(tx: &mpsc::Sender<DispatcherMsg>, handle: ActorHandle) {
    tx.send(DispatcherMsg::Close).expect("send Close");
    run_bounded(
        "dispatcher join after Close",
        Duration::from_secs(5),
        move || {
            handle
                .join()
                .expect("dispatcher terminates normally after Close");
        },
    );
}

/// 実 talk の選択待ち台本（sakura drive.rs の MENU_SCRIPT と同じ）。
///
/// 占有 horizon は 0.35（Choice と Barrier が 0.35 にあり、barrier が最終 horizon 要素の menu ケース）。
const MENU_SCRIPT: &str = r"\s[10]hello\_w[100]\q[選択A,targetA]\e";

/// 現行 slot と `talk_id` が一致する `ResolveChoice` は `SakuraMsg::ResolveChoice{id}` として
/// talk へ無改変で転送され、slot と `base_now` は動かない。kanade へは何も出さない。
#[test]
fn resolve_choice_relays_id_to_matching_talk_without_touching_slot_or_base() {
    let mut fx = state_fixture();
    let (probe, obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(901);
    occupy(&mut fx.state, talk_id, probe);

    // Tick で base_now を刻み、中継が起点を動かさないことと、ミリ秒→秒の換算を見る。
    fx.feed(DispatcherMsg::Tick {
        now: MonotonicMs(7_000),
    });
    fx.feed(DispatcherMsg::Tick {
        now: MonotonicMs(7_500),
    });
    let relayed = || match obs_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(SakuraMsg::Tick(t)) => t,
        _ => panic!("talk へ SakuraMsg::Tick が中継されるべき"),
    };
    assert_eq!(
        (relayed(), relayed()),
        (0.0, 0.5),
        "初回 Tick は elapsed 0.0（base_now 刻印）・500ms 後は 0.5 秒として中継される"
    );

    fx.feed(DispatcherMsg::ResolveChoice {
        talk_id,
        id: "targetA".to_string(),
    });

    match obs_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(SakuraMsg::ResolveChoice { id }) => assert_eq!(
            id, "targetA",
            "選択肢 ID は無改変で talk の型付き入力へ転送される"
        ),
        _ => panic!("talk へ SakuraMsg::ResolveChoice が中継されるべき"),
    }

    assert_eq!(
        fx.state.current_talk_id(),
        Some(talk_id),
        "解決の中継で slot は解放されない（talk は継続する）"
    );
    assert_eq!(
        fx.state.active.as_ref().and_then(|a| a.base_now),
        Some(MonotonicMs(7_000)),
        "解決の中継で base_now（Tick 中継の起点）は動かない"
    );
    assert!(
        fx.kanade_rx.try_recv().is_err(),
        "解決の中継は kanade へ何も送らない（完了通知は talk 発の TalkDone が運ぶ）"
    );

    release(&mut fx.state);
}

/// `talk_id` が現行 slot と一致しない／slot が空の `ResolveChoice` は talk へ何も送らず、
/// `resolve_choice_stale`（info）で記録して棄却される。
#[test]
fn resolve_choice_with_mismatched_or_empty_slot_is_discarded_as_stale() {
    let mut fx = state_fixture();
    let (probe, obs_rx) = spawn_probe_talk();
    occupy(&mut fx.state, TalkId(902), probe);

    // 旧 talk 宛の遅れた指示。
    let events = capture(|| {
        fx.feed(DispatcherMsg::ResolveChoice {
            talk_id: TalkId(999),
            id: "targetA".to_string(),
        });
    });
    assert_logged_event(
        &events,
        Level::INFO,
        "areka_ghost::dispatcher",
        "resolve_choice_stale",
    );
    assert!(
        obs_rx.try_recv().is_err(),
        "不一致 ResolveChoice は talk へ一切転送されない"
    );
    assert_eq!(
        fx.state.current_talk_id(),
        Some(TalkId(902)),
        "stale 棄却は現行 slot を乱さない"
    );

    // talk 終了後の遅れた指示。
    release(&mut fx.state);
    let events = capture(|| {
        fx.feed(DispatcherMsg::ResolveChoice {
            talk_id: TalkId(902),
            id: "targetA".to_string(),
        });
    });
    assert_logged_event(
        &events,
        Level::INFO,
        "areka_ghost::dispatcher",
        "resolve_choice_stale",
    );
    assert!(
        fx.kanade_rx.try_recv().is_err(),
        "stale 棄却は kanade へ何も送らない"
    );
}

/// 一致していても talk が直前に消滅していると、中継の `send` は失敗する。dispatcher は
/// 黙って捨てず `debug` で記録し、処理を続ける。
#[test]
fn resolve_choice_relay_failure_after_talk_vanished_is_recorded_at_debug_and_continues() {
    let mut fx = state_fixture();
    let talk_id = TalkId(903);
    occupy(&mut fx.state, talk_id, spawn_vanished_talk());

    let events = capture(|| {
        fx.feed(DispatcherMsg::ResolveChoice {
            talk_id,
            id: "targetA".to_string(),
        });
    });
    assert_logged(
        &events,
        Level::DEBUG,
        "areka_ghost::dispatcher",
        "dropping ResolveChoice relay",
    );

    release(&mut fx.state);
}

/// 一致する `CancelChoice` は `SakuraMsg::Close` を talk へ転送するだけで、slot も join も保つ。
/// そのため talk 発の `TalkDone{Interrupted}` は `on_done` の一致判定を通り kanade へ届く。
///
/// `close_active_if_any`（即 join・slot の先行解放）を使う実装なら、Close 転送の直後に slot が
/// `None` になって後続の assert が落ち、続く `Done` も stale として棄却される。
#[test]
fn cancel_choice_forwards_close_and_keeps_slot_so_talkdone_reaches_kanade() {
    let mut fx = state_fixture();
    let (probe, obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(904);
    occupy(&mut fx.state, talk_id, probe);

    fx.feed(DispatcherMsg::CancelChoice { talk_id });

    assert!(
        matches!(
            obs_rx.recv_timeout(Duration::from_secs(5)),
            Ok(SakuraMsg::Close)
        ),
        "解除は単一 Close funnel へ写像される（skip_barrier の外部到達口を新設しない）"
    );
    assert_eq!(
        fx.state.current_talk_id(),
        Some(talk_id),
        "Close は転送のみ——slot は維持される（close_active_if_any を使っていない直接証跡）"
    );

    // talk 発の中断 ACK。slot が保たれているので一致判定を通る。
    fx.feed(DispatcherMsg::Done(TalkDone {
        talk_id,
        reason: TalkEndReason::Interrupted,
        quit_reserved: false,
    }));
    match fx
        .kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("解除後の TalkDone{Interrupted} は kanade へ届くべき（DD-11）")
    {
        KanadeMsg::TalkDone(done) => {
            assert_eq!(done.talk_id, talk_id);
            assert_eq!(done.reason, TalkEndReason::Interrupted);
        }
        _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
    }
    assert_eq!(
        fx.state.current_talk_id(),
        None,
        "完了通知の転送で slot が解放される（既存 on_done の規律）"
    );

    release(&mut fx.state);
}

/// 一致していても talk が直前に消滅していると、Close 転送の `send` は失敗する。dispatcher は
/// 黙って捨てず `debug` で記録し、slot を保ったまま続ける。その talk は既に自力で終端していて、
/// 別に届く `TalkDone` が slot を解放する。
#[test]
fn cancel_choice_relay_failure_after_talk_vanished_is_recorded_at_debug_and_continues() {
    let mut fx = state_fixture();
    let talk_id = TalkId(909);
    occupy(&mut fx.state, talk_id, spawn_vanished_talk());

    let events = capture(|| {
        fx.feed(DispatcherMsg::CancelChoice { talk_id });
    });
    assert_logged(
        &events,
        Level::DEBUG,
        "areka_ghost::dispatcher",
        "dropping CancelChoice Close relay",
    );
    assert_eq!(
        fx.state.current_talk_id(),
        Some(talk_id),
        "転送失敗でも slot は先行解放しない（解放は talk 発の TalkDone の役目・DD-11）"
    );

    release(&mut fx.state);
}

/// `talk_id` 不一致／slot 空の `CancelChoice` は Close を転送せず、`cancel_choice_stale`（info）で
/// 記録して棄却される（現行 talk を巻き添えにしない）。
#[test]
fn cancel_choice_with_mismatched_or_empty_slot_is_discarded_as_stale() {
    let mut fx = state_fixture();
    let (probe, obs_rx) = spawn_probe_talk();
    occupy(&mut fx.state, TalkId(905), probe);

    let events = capture(|| {
        fx.feed(DispatcherMsg::CancelChoice {
            talk_id: TalkId(999),
        });
    });
    assert_logged_event(
        &events,
        Level::INFO,
        "areka_ghost::dispatcher",
        "cancel_choice_stale",
    );
    assert!(
        obs_rx.try_recv().is_err(),
        "不一致 CancelChoice は現行 talk へ Close を送らない（巻き添え終了させない）"
    );
    assert_eq!(
        fx.state.current_talk_id(),
        Some(TalkId(905)),
        "stale 棄却は現行 slot を乱さない"
    );

    release(&mut fx.state);
    let events = capture(|| {
        fx.feed(DispatcherMsg::CancelChoice {
            talk_id: TalkId(905),
        });
    });
    assert_logged_event(
        &events,
        Level::INFO,
        "areka_ghost::dispatcher",
        "cancel_choice_stale",
    );
    assert!(
        fx.kanade_rx.try_recv().is_err(),
        "stale 棄却は kanade へ何も送らない"
    );
}

/// `base` を初回 Tick で刻み（続けて `extra_ticks` を打ち）、`ChoiceWaiting` を投函して、
/// kanade へ転送された 1 通を返す。
fn relay_choice_waiting(
    base: MonotonicMs,
    extra_ticks: &[MonotonicMs],
    elapsed_secs: f64,
    timeout_directive_secs: Option<f64>,
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
        display_end_elapsed_secs: elapsed_secs,
        timeout_directive_secs,
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

/// `display_end_ms = base_now + round(秒 × 1000)`。`base_now` は Tick 中継の起点（初回 Tick の
/// `now`）で、後続 Tick でも通知時刻でもない。候補 id 列とタイムアウト指令は無改変で運ばれる。
///
/// 0.3567s は 356.7ms なので、切り捨て実装なら 356（→ 4_356）で落ちる。1.2504s は 1250.4ms
/// なので、切り上げ実装なら 1251（→ 5_251）で落ちる。2 つで四捨五入を挟み撃つ。
/// 後続 Tick(9_000) を起点に取る実装なら 9_357 になって落ちる。
#[test]
fn choice_waiting_converts_elapsed_secs_to_ms_from_the_tick_base_and_forwards_to_kanade() {
    match relay_choice_waiting(
        MonotonicMs(4_000),
        &[MonotonicMs(9_000)],
        0.3567,
        Some(12.0),
    ) {
        KanadeMsg::ChoiceWaiting {
            talk_id,
            choice_ids,
            display_end,
            timeout_directive_secs,
        } => {
            assert_eq!(talk_id, TalkId(906), "talk_id はエコーされる");
            assert_eq!(
                choice_ids,
                vec!["a".to_string(), "b".to_string()],
                "候補 id 列は表示順のまま無改変で運ばれる"
            );
            assert_eq!(
                display_end,
                MonotonicMs(4_357),
                "base_now(4_000) + round(0.3567 × 1000 = 356.7) = 4_357（後続 Tick 9_000 起点ではない）"
            );
            assert_eq!(
                timeout_directive_secs,
                Some(12.0),
                "タイムアウト指令は写像せず素通し（deadline 写像は kanade の領分・DD-8）"
            );
        }
        _ => unreachable!("ChoiceWaiting を投函したので ChoiceWaiting が届く"),
    }

    match relay_choice_waiting(MonotonicMs(4_000), &[], 1.2504, None) {
        KanadeMsg::ChoiceWaiting {
            display_end,
            timeout_directive_secs,
            ..
        } => {
            assert_eq!(
                display_end,
                MonotonicMs(5_250),
                "base_now(4_000) + round(1.2504 × 1000 = 1250.4) = 5_250（切り上げでない）"
            );
            assert_eq!(
                timeout_directive_secs, None,
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
    fx.feed(DispatcherMsg::Tick {
        now: MonotonicMs(1_000),
    });

    let events = capture(|| {
        fx.feed(DispatcherMsg::ChoiceWaiting(ChoiceWaiting {
            talk_id: TalkId(999),
            choice_ids: vec!["a".to_string()],
            display_end_elapsed_secs: 0.5,
            timeout_directive_secs: None,
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
            display_end_elapsed_secs: 0.5,
            timeout_directive_secs: None,
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
            display_end_elapsed_secs: 0.5,
            timeout_directive_secs: None,
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
        state.handle(DispatcherMsg::Tick {
            now: MonotonicMs(3_000)
        }),
        ControlFlow::Continue(())
    );
    drop(kanade_rx);

    let events = capture(|| {
        assert_eq!(
            state.handle(DispatcherMsg::ChoiceWaiting(ChoiceWaiting {
                talk_id,
                choice_ids: vec!["a".to_string()],
                display_end_elapsed_secs: 0.5,
                timeout_directive_secs: None,
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
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(1_000),
    })
    .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(1_500),
    })
    .expect("send Tick(base+500ms)");

    match kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("バリア成立で ChoiceWaiting が kanade へ届くべき")
    {
        KanadeMsg::ChoiceWaiting {
            talk_id: got,
            choice_ids,
            display_end,
            timeout_directive_secs,
        } => {
            assert_eq!(got, talk_id);
            assert_eq!(choice_ids, vec!["targetA".to_string()]);
            assert_eq!(
                display_end,
                MonotonicMs(1_350),
                "base_now(1_000) + 占有 horizon 0.35s→350ms。**tick 時刻 1_500 ではない**（R7.2）"
            );
            assert_eq!(
                timeout_directive_secs, None,
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
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(2_000),
    })
    .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(2_500),
    })
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

// ── 押下の選択の取り次ぎ ──
//
// UI の押下の選択は `ChoiceWaiting` と同じ inbox に並び、dispatcher は判断せずに inbox の順のまま
// kanade へ送る（slot と突き合わせない・中身を変えない）。同期 state の檻で、inbox へ積む順を
// テストが決めて、kanade の口に着く順を見る。

/// 中身の改変を見分けられる選択肢の入力（前後空白・空文字の引数・さくらスクリプト風）。
fn dirty_choice_input() -> ChoiceInput {
    ChoiceInput {
        id: "  Onおしゃべり頻度  ".to_string(),
        label: r" \q[もどる,back] ＞＞ ".to_string(),
        scope: 1,
        references: vec!["  前後  ".to_string(), String::new(), r"\e".to_string()],
    }
}

/// 同じくアンカーの入力。
fn dirty_anchor_input() -> AnchorInput {
    AnchorInput {
        id: " https://example.invalid/ ".to_string(),
        text: "  あい ".to_string(),
        scope: 0,
        references: vec![String::new(), "  ".to_string()],
    }
}

/// 稼働中のトークの `ChoiceWaiting` の後に inbox へ積んだ選択は、間に時刻の前進とトークの起動を
/// 挟んでも、kanade の口へ `ChoiceWaiting` → `Choice` → `Anchor` の順で着き、中身は積んだまま
/// （`Start` が slot を差し替えても棄却しない）。
#[test]
fn pressed_selection_queued_after_choice_waiting_reaches_kanade_in_inbox_order() {
    let mut fx = state_fixture();
    let (probe, _obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(941);
    occupy(&mut fx.state, talk_id, probe);

    fx.feed(DispatcherMsg::Tick {
        now: MonotonicMs(1_000),
    });
    fx.feed(DispatcherMsg::ChoiceWaiting(ChoiceWaiting {
        talk_id,
        choice_ids: vec!["targetA".to_string()],
        display_end_elapsed_secs: 0.35,
        timeout_directive_secs: None,
    }));
    fx.feed(DispatcherMsg::Tick {
        now: MonotonicMs(1_200),
    });
    fx.feed(DispatcherMsg::Start(StartTalk::new(
        TalkId(942),
        MENU_SCRIPT,
    )));
    fx.feed(DispatcherMsg::Choice(dirty_choice_input()));
    fx.feed(DispatcherMsg::Anchor(dirty_anchor_input()));

    let got: Vec<KanadeMsg> = fx.kanade_rx.try_iter().collect();
    assert!(
        matches!(
            &got[..],
            [
                KanadeMsg::ChoiceWaiting { talk_id: waited, .. },
                KanadeMsg::Choice(choice),
                KanadeMsg::Anchor(anchor),
            ] if *waited == talk_id
                && *choice == dirty_choice_input()
                && *anchor == dirty_anchor_input()
        ),
        "inbox の順のまま ChoiceWaiting → Choice → Anchor・中身は積んだまま（届いた数 {}）",
        got.len()
    );

    release(&mut fx.state);
}

/// slot が空でも選択肢とアンカーは stale として捨てず、そのまま kanade へ送る（受けるかどうかは
/// kanade が決める）。stale の記録も残さない。
#[test]
fn pressed_selection_with_empty_slot_is_forwarded_without_rejection() {
    let mut fx = state_fixture();
    assert_eq!(fx.state.current_talk_id(), None, "前提: slot は空");

    let events = capture(|| {
        fx.feed(DispatcherMsg::Choice(dirty_choice_input()));
        fx.feed(DispatcherMsg::Anchor(dirty_anchor_input()));
    });

    let got: Vec<KanadeMsg> = fx.kanade_rx.try_iter().collect();
    assert!(
        matches!(
            &got[..],
            [KanadeMsg::Choice(choice), KanadeMsg::Anchor(anchor)]
                if *choice == dirty_choice_input() && *anchor == dirty_anchor_input()
        ),
        "slot が空でも 2 件ともそのまま送る（届いた数 {}）",
        got.len()
    );
    assert!(
        events.iter().all(|e| e.level > Level::INFO),
        "棄却の記録（info 以上）を残さない: {:?}",
        events
            .iter()
            .map(|e| (e.level, e.event.clone(), e.message.clone()))
            .collect::<Vec<_>>()
    );
}

/// kanade の停止後は、選択肢は `choice_forward_failed`・アンカーは `anchor_forward_failed` の
/// `warn!` を 1 行ずつ残し、dispatcher は止まらずに続く（後の Tick も受ける）。
#[test]
fn pressed_selection_forward_failure_after_kanade_stopped_warns_once_and_continues() {
    let StateFixture {
        mut state,
        kanade_rx,
        _self_rx,
    } = state_fixture();
    drop(kanade_rx);

    let events = capture(|| {
        for msg in [
            DispatcherMsg::Choice(dirty_choice_input()),
            DispatcherMsg::Anchor(dirty_anchor_input()),
            DispatcherMsg::Tick {
                now: MonotonicMs(5_000),
            },
        ] {
            assert_eq!(
                state.handle(msg),
                ControlFlow::Continue(()),
                "送れなくても dispatcher は止まらない（停止経路は Close のみ）"
            );
        }
    });

    let count = |name: &str| {
        events
            .iter()
            .filter(|e| {
                e.target == "areka_ghost::dispatcher"
                    && e.level == Level::WARN
                    && e.event.as_deref() == Some(name)
            })
            .count()
    };
    assert_eq!(
        (
            count("choice_forward_failed"),
            count("anchor_forward_failed")
        ),
        (1, 1),
        "種類ごとの名前で warn が 1 行ずつ: {:?}",
        events
            .iter()
            .map(|e| (e.level, e.event.clone(), e.message.clone()))
            .collect::<Vec<_>>()
    );
}
