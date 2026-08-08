use super::test_support::{RecordingSink, run_bounded, test_system_vars};
use super::*;
use crate::test_log_capture::{assert_logged, assert_logged_event, capture};
use areka_sakura::contract::TalkEndReason;
use std::sync::mpsc;
use std::time::Duration;
use tracing::Level;

// ── task 3.3: 選択系 3 アームの中継意味論と時刻換算（design C9・DD-9/DD-11・R1.3/5.5/7.2/7.5） ──
//
// 檻は 2 段構えで、それぞれ弁別できるものが違う:
//
// - **同期 state 檻**（[`state_fixture`]＋[`spawn_probe_talk`]）: [`DispatcherState::handle`] を
//   テストスレッド上で直接駆動する。talk スタンドインが受けた [`SakuraMsg`] を**そのまま**
//   突合できる（＝中継の送出内容の直接固定）ほか、棄却・防御アームのログを
//   [`capture`](crate::test_log_capture::capture) で観測できる（`with_default` は thread-local
//   ゆえ actor スレッドのログは載らない＝この段でしか語彙を固定できない）。
// - **actor e2e 檻**（[`spawn_dispatcher`]＋実 talk）: 実再生層を通した往復（選択待ち成立通知の
//   ms 換算・解決による再開・Close funnel の中断 ACK 帰還）を固定する。

/// 中継先 talk の**スタンドイン**（probe）。dispatcher が送った [`SakuraMsg`] を観測チャンネルへ
/// 素通しするだけの actor で、[`TalkHandle`] の形（inbox＋join ハンドル）を満たす。実再生を挟まない
/// ため、中継の**送出内容そのもの**を突合できる。`Close` を受けたら（実 talk 同様）停止する。
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

/// 既に消滅した talk のスタンドイン（中継の送出失敗経路を**決定的に**再現する）。
///
/// body は `rx` を drop してから合図を送るため、合図受領後に inbox へ送ると必ず `Err` になる
/// （`is_finished` のポーリングに頼らない）。
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

/// 同期駆動用の dispatcher 状態 fixture。
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
            self_sender: self_tx,
        },
        kanade_rx,
        _self_rx,
    }
}

impl StateFixture {
    /// 1 メッセージを同期処理する。選択系 3 アームは（`Tick`/`Done` と同じく）**決して
    /// dispatcher を停止させない**ため、`Continue` であることを毎回併せて固定する
    /// （停止経路は `Close` 単独・要件 4.5）。
    fn feed(&mut self, msg: DispatcherMsg) {
        assert_eq!(
            self.state.handle(msg),
            ControlFlow::Continue(()),
            "選択系・Tick・Done の各アームは dispatcher を停止させない（停止経路は Close のみ）"
        );
    }
}

/// slot へ talk スタンドインを据える（`base_now` は Tick 未着なら `None`）。
fn occupy(state: &mut DispatcherState, talk_id: TalkId, handle: TalkHandle) {
    state.active = Some(ActiveTalk {
        talk_id,
        handle,
        base_now: None,
    });
}

/// 後始末: slot に残ったスタンドインを畳む（スレッドを残さない）。
fn release(state: &mut DispatcherState) {
    if let Some(active) = state.active.take() {
        let _ = active.handle.inbox.send(SakuraMsg::Close);
        let _ = active.handle.actor.join();
    }
}

/// 実 talk の選択待ち台本（sakura drive.rs の MENU_SCRIPT と同一）。
///
/// compile 後（アンカー 0）: `hello`@0（D=0.25）／Wait@0.25（`\w[2]`=0.1）／Choice@0.35（id=targetA）
/// ／Barrier@0.35。占有 horizon＝0.35（barrier が最終 horizon 要素の menu ケース）。
const MENU_SCRIPT: &str = r"\s[10]hello\w[2]\q[選択A,targetA]\e";

/// **一致中継（Resolve）・R5.5**: 現行 slot と `talk_id` が一致する `ResolveChoice` は
/// `SakuraMsg::ResolveChoice{id}` として talk へ無改変で転送され、slot と `base_now` は動かない
/// （解決はバリアを解くだけで talk の同一性も時刻起点も変えない）。kanade へは何も出さない。
#[test]
fn resolve_choice_relays_id_to_matching_talk_without_touching_slot_or_base() {
    let mut fx = state_fixture();
    let (probe, obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(901);
    occupy(&mut fx.state, talk_id, probe);

    // base_now を Tick で刻印しておく（中継が起点を動かさないことを見るため）。
    fx.feed(DispatcherMsg::Tick {
        now: MonotonicMs(7_000),
    });
    assert!(
        matches!(obs_rx.recv_timeout(Duration::from_secs(5)), Ok(SakuraMsg::Tick(t)) if t == 0.0),
        "初回 Tick は elapsed 0.0 として中継される（base_now 刻印）"
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

/// **不一致棄却（Resolve）・R1.3/5.5**: `talk_id` が現行 slot と一致しない／slot が空の
/// `ResolveChoice` は talk へ何も送らず、`resolve_choice_stale`（info）で記録して棄却される。
#[test]
fn resolve_choice_with_mismatched_or_empty_slot_is_discarded_as_stale() {
    let mut fx = state_fixture();
    let (probe, obs_rx) = spawn_probe_talk();
    occupy(&mut fx.state, TalkId(902), probe);

    // (1) 不一致（旧 talk 宛の遅延指示）。
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

    // (2) slot 空（talk 終了後の遅延指示）。
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

/// **送出失敗でも運行継続（Resolve）**: 一致していても talk が直前に消滅していた場合、中継の
/// `send` は失敗する。dispatcher は黙って捨てず `debug` で記録し、処理を継続する
/// （steering: areka-log-first-no-silent-failure）。
#[test]
fn resolve_choice_relay_failure_after_talk_vanished_is_recorded_at_debug_and_continues() {
    let mut fx = state_fixture();
    let talk_id = TalkId(903);
    occupy(&mut fx.state, talk_id, spawn_vanished_talk());

    // `feed` は `Continue`（＝送出失敗でも dispatcher を停止させない）を併せて固定する。
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

/// **一致中継（Cancel）＋slot 維持・R7.5/DD-11**: 一致する `CancelChoice` は `SakuraMsg::Close`
/// を talk へ**転送**するだけで、slot も join も保持する。その結果、talk 発の
/// `TalkDone{Interrupted}` は `on_done` の一致判定を通り kanade へ届く。
///
/// **弁別**: `close_active_if_any`（即 join・slot 先行解放）を使う実装なら、Close 転送直後に
/// slot が `None` になるため後続 assert が落ち、続く `Done` も stale 棄却されて kanade へ届かない。
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

    // talk 発の中断 ACK（正規経路）。slot が維持されているので一致判定を通る。
    fx.feed(DispatcherMsg::Done(TalkDone {
        talk_id,
        reason: TalkEndReason::Interrupted,
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

/// **送出失敗でも運行継続（Cancel）**: 一致していても talk が直前に消滅していた場合、Close 転送の
/// `send` は失敗する。dispatcher は黙って捨てず `debug` で記録し、slot を維持したまま継続する
/// （steering: areka-log-first-no-silent-failure）——その talk は既に自力で終端しており、
/// 別途届く `TalkDone` が slot を解放する（Resolve 側の同型檻と対をなす失敗経路）。
#[test]
fn cancel_choice_relay_failure_after_talk_vanished_is_recorded_at_debug_and_continues() {
    let mut fx = state_fixture();
    let talk_id = TalkId(909);
    occupy(&mut fx.state, talk_id, spawn_vanished_talk());

    // `feed` は `Continue`（＝送出失敗でも dispatcher を停止させない）を併せて固定する。
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

/// **不一致棄却（Cancel）・R1.3**: `talk_id` 不一致／slot 空の `CancelChoice` は Close を
/// 一切転送せず、`cancel_choice_stale`（info）で記録して棄却される（現行 talk を巻き添えにしない）。
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

/// 換算の実測ヘルパ: `base` を初回 Tick で刻印し（続けて `extra_ticks` を打ち）、
/// `ChoiceWaiting` を投函して kanade へ転送された 1 通を返す。
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

/// **換算値（ChoiceWaiting）・R7.2/DD-9**: `display_end_ms = base_now + round(秒 × 1000)`。
/// `base_now` は **Tick 中継の既存起点**（初回 Tick の `now`）であり、後続 Tick でも通知時刻でも
/// ない（新しい時間基準を作らない）。候補 id 列とタイムアウト指令は無改変で運ばれる。
///
/// **弁別**: 0.3567s は 356.7ms ゆえ切り捨て実装なら 356（→ 4_356）で落ちる。1.2504s は
/// 1250.4ms ゆえ切り上げ実装なら 1251（→ 5_251）で落ちる。両者の同時固定で「四捨五入」を挟み撃つ。
/// 起点は base(4_000) 固定ゆえ、後続 Tick(9_000) を起点に取る実装なら 9_357 になって落ちる。
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

/// **不一致棄却（ChoiceWaiting）・R1.3**: `talk_id` 不一致／slot 空の通知は kanade へ転送されず、
/// `choice_waiting_stale`（info）で記録して棄却される。
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

/// **`base_now` 未確定の防御・DD-9**: Tick 前に通知が出るのは構造上あり得ない（通知は tick 駆動の
/// バリア到達で出る）。それでも起点欠如を黙って埋めず `warn` で記録し、kanade へは転送しない
/// （でっち上げた起点で誤った deadline を配らない）。
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

/// **転送失敗でも運行継続（ChoiceWaiting）**: kanade が既に停止していると転送の `send` は失敗する。
/// dispatcher は黙って捨てず `debug` で記録し、slot を乱さず継続する（停止経路は `Close` のみ・
/// steering: areka-log-first-no-silent-failure）。
///
/// **決定性**: `kanade_rx` を投函**前**に drop するため送出は必ず `Err` になる（`is_finished`
/// 等のポーリングに依存しない）。`feed` を使わず [`DispatcherState::handle`] を直呼びするのは、
/// fixture から受信端だけを取り出して drop するためである（`Continue` の固定は同等に行う）。
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

    // base_now を刻印（換算アームまで到達させる＝warn 防御枝ではないことを担保）。
    assert_eq!(
        state.handle(DispatcherMsg::Tick {
            now: MonotonicMs(3_000)
        }),
        ControlFlow::Continue(())
    );
    // 投函前に kanade 側を停止（以降の転送は必ず Err）。
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

/// **e2e（実 talk・R7.2/5.1）**: 実再生層が選択待ちバリアへ到達すると `ChoiceWaiting` が
/// dispatcher 経由で kanade へ届き、`display_end` は **base_now ＋ 台本由来の占有 horizon**
/// （tick 時刻ではない）になる。続く `ResolveChoice` は talk のバリアを解き、menu ケースゆえ
/// その場で完了して `TalkDone{Ended}` が kanade へ届く。
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
    );

    let talk_id = TalkId(931);
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id,
        script: MENU_SCRIPT.to_string(),
    }))
    .expect("send Start(menu)");
    // base_now=1_000 刻印 → elapsed 0.5 で Choice@0.35・Barrier@0.35 到達（WaitingForChoice）。
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

/// **e2e（実 talk・R7.5/DD-11）**: 選択待ち中の `CancelChoice` は Close funnel を通って talk を
/// 終了させ、talk 発の `TalkDone{Interrupted}` が**正規経路で** kanade へ届く。
///
/// **弁別**: dispatcher が `close_active_if_any`（slot 先行解放）を使う実装なら、返ってきた
/// `TalkDone` は stale 判定で棄却され kanade へ届かず、この recv が timeout で落ちる。
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
    );

    let talk_id = TalkId(932);
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id,
        script: MENU_SCRIPT.to_string(),
    }))
    .expect("send Start(menu)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(2_000),
    })
    .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(2_500),
    })
    .expect("send Tick(base+500ms)");

    // バリア成立（＝Cancel を送る前提条件）を通知の到着で決定的に待つ。
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
