use super::*;
use crate::test_log_capture::{assert_logged, assert_logged_event, capture};
use crate::test_support::{run_bounded, test_system_vars};
use areka_sakura::contract::TalkEndReason;
use dola::time::testing::at_ms;
use std::sync::mpsc;
use std::time::Duration;
use tracing::Level;

// ── 選択系 3 アームの中継意味論 ──
//
// - 同期 state の檻（[`state_fixture`]＋[`spawn_probe_talk`]）: [`DispatcherState::handle`] を
//   テストスレッド上で直接駆動し、talk スタンドインが受けた [`SakuraMsg`] をそのまま突合する。
//   棄却・防御アームのログは [`capture`](crate::test_log_capture::capture) で見る。捕捉窓は
//   呼出スレッド局所で actor スレッドのログは載らないので、ログの語彙はこの段でしか固定できない。
// - actor e2e の檻（[`spawn_dispatcher`]＋実 talk）: 実再生層を通した往復（選択待ち成立通知の
//   表示の終わり・解決による再開・Close funnel の中断 ACK 帰還）を固定する。

/// 中継先 talk のスタンドイン（probe）。dispatcher が送った [`SakuraMsg`] を観測チャンネルへ
/// 素通しするだけの actor で、実再生を挟まないので中継の送出内容をそのまま突合できる。
/// `Close` を受けたら（実 talk 同様）停止する。
pub(super) fn spawn_probe_talk() -> (TalkHandle, mpsc::Receiver<SakuraMsg>) {
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
pub(super) struct StateFixture {
    pub(super) state: DispatcherState,
    pub(super) kanade_rx: mpsc::Receiver<KanadeMsg>,
    /// dispatcher 自身の inbox 受信端（`self_sender` の相方・保持しないと送出が切断される）。
    pub(super) _self_rx: mpsc::Receiver<DispatcherMsg>,
}

pub(super) fn state_fixture() -> StateFixture {
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
    pub(super) fn feed(&mut self, msg: DispatcherMsg) {
        assert_eq!(
            self.state.handle(msg),
            ControlFlow::Continue(()),
            "選択系・Tick・Done の各アームは dispatcher を停止させない（停止経路は Close のみ）"
        );
    }
}

/// slot へ talk スタンドインを据える。
pub(super) fn occupy(state: &mut DispatcherState, talk_id: TalkId, handle: TalkHandle) {
    state.active = Some(ActiveTalk { talk_id, handle });
}

/// slot に残ったスタンドインを畳む。
pub(super) fn release(state: &mut DispatcherState) {
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
pub(super) const MENU_SCRIPT: &str = r"\s[10]hello\_w[100]\q[選択A,targetA]\e";

/// 現行 slot と `talk_id` が一致する `ResolveChoice` は `SakuraMsg::ResolveChoice{id}` として
/// talk へ無改変で転送され、slot は動かない。kanade へは何も出さない。
#[test]
fn resolve_choice_relays_id_to_matching_talk_without_touching_slot() {
    let mut fx = state_fixture();
    let (probe, obs_rx) = spawn_probe_talk();
    let talk_id = TalkId(901);
    occupy(&mut fx.state, talk_id, probe);

    // 刻みは最初の刻みからの隔たりへ直さず、運ぶ時刻のまま中継される。
    fx.feed(DispatcherMsg::Tick { now: at_ms(7_000) });
    fx.feed(DispatcherMsg::Tick { now: at_ms(7_500) });
    let relayed = || match obs_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(SakuraMsg::Tick(t)) => t,
        _ => panic!("talk へ SakuraMsg::Tick が中継されるべき"),
    };
    assert_eq!(
        (relayed(), relayed()),
        (at_ms(7_000), at_ms(7_500)),
        "刻みは運ぶ絶対の時刻のまま中継される（初回を 0 に直さない）"
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
