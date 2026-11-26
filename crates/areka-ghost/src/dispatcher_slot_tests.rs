use super::choice_tests::{close_and_join, occupy, release, spawn_probe_talk, state_fixture};
use super::*;
use crate::test_support::{RecordingSink, test_system_vars};
use areka_sakura::contract::{CueCommand, TalkCue, TalkEndReason};
use dola::time::testing::{at_ms, delta_ms, ms};
use std::sync::mpsc;
use std::time::Duration;

/// cue をチャンネルで中継する `Clone` 可能な sink（`recv_timeout` で 1 件ずつ観測できる）。
#[derive(Clone)]
struct ChannelSink {
    tx: mpsc::Sender<TalkCue>,
}

impl CueSink for ChannelSink {
    fn emit(&mut self, cue: TalkCue) {
        let _ = self.tx.send(cue);
    }
}

/// `From<TalkCommand> for DispatcherMsg` が全 variant を無損失で対応アームへ写す。
///
/// kanade の送出口から start-relay を経て dispatcher inbox へ入る唯一の変換点なので、
/// variant が落ちるとその指示は届かない。本体の `From` は受け皿を持たないので、variant が増えると
/// コンパイラが本体の直しを迫る。このテストへの追加は手で行う。
#[test]
fn talk_command_converts_to_dispatcher_msg_for_every_variant() {
    use areka_sakura::contract::TalkCommand;

    let start = StartTalk::new(TalkId(101), r"\s[0]hi\e");
    match DispatcherMsg::from(TalkCommand::Start(start.clone())) {
        DispatcherMsg::Start(got) => {
            assert_eq!(got.talk_id, start.talk_id);
            assert_eq!(got.script, start.script);
            assert!(got.epilogue.is_empty());
        }
        _ => panic!("TalkCommand::Start は DispatcherMsg::Start へ変換されるべき"),
    }

    match DispatcherMsg::from(TalkCommand::ResolveChoice {
        talk_id: TalkId(102),
        id: "choice-1".to_string(),
    }) {
        DispatcherMsg::ResolveChoice { talk_id, id } => {
            assert_eq!(talk_id, TalkId(102));
            assert_eq!(id, "choice-1");
        }
        _ => panic!("TalkCommand::ResolveChoice は DispatcherMsg::ResolveChoice へ変換されるべき"),
    }

    match DispatcherMsg::from(TalkCommand::CancelChoice {
        talk_id: TalkId(103),
    }) {
        DispatcherMsg::CancelChoice { talk_id } => assert_eq!(talk_id, TalkId(103)),
        _ => panic!("TalkCommand::CancelChoice は DispatcherMsg::CancelChoice へ変換されるべき"),
    }
}

/// `From<ChoiceWaiting> for DispatcherMsg` が全フィールド（候補 id 列・表示完了時刻・
/// タイムアウト指令）を改変せず運ぶ。
#[test]
fn choice_waiting_converts_to_dispatcher_msg_without_loss() {
    use areka_sakura::contract::ChoiceWaiting;

    let waiting = ChoiceWaiting {
        talk_id: TalkId(201),
        choice_ids: vec!["a".to_string(), "b".to_string()],
        display_end: at_ms(1_250),
        timeout_directive: Some(delta_ms(12_000)),
    };
    match DispatcherMsg::from(waiting.clone()) {
        DispatcherMsg::ChoiceWaiting(got) => assert_eq!(got, waiting),
        _ => panic!("ChoiceWaiting は DispatcherMsg::ChoiceWaiting へ変換されるべき"),
    }

    // 未指定（None＝下流の既定値に任せる）も同じ。
    let unspecified = ChoiceWaiting {
        talk_id: TalkId(202),
        choice_ids: Vec::new(),
        display_end: at_ms(0),
        timeout_directive: None,
    };
    match DispatcherMsg::from(unspecified.clone()) {
        DispatcherMsg::ChoiceWaiting(got) => assert_eq!(got, unspecified),
        _ => panic!("ChoiceWaiting は DispatcherMsg::ChoiceWaiting へ変換されるべき"),
    }
}

/// `Start(A)` 稼働中に `Start(B)` を送ると、A は Close して join してから B が spawn される。
/// A の完了通知（`Interrupted`）は B へ差し替わった後に届く stale 通知で、kanade へは転送されない。
#[test]
fn start_then_start_replaces_active_talk_and_discards_stale_done_from_replaced_talk() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let surface = RecordingSink::new();
    let text = RecordingSink::new();

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        test_system_vars(),
        None,
    );

    let talk_a = TalkId(1);
    let talk_b = TalkId(2);

    // A は差し替えまで Tick を送らないので自然完了しない。
    tx.send(DispatcherMsg::Start(StartTalk::new(
        talk_a,
        r"\s[1]A\_w[2500]A_END\e",
    )))
    .expect("send Start(A)");

    tx.send(DispatcherMsg::Start(StartTalk::new(
        talk_b,
        r"\s[2]B\_w[100]B_END\e",
    )))
    .expect("send Start(B)");

    // B の占有 horizon=0.40 を跨ぐ elapsed 0.5 まで進めて完走させる（最初の Tick の now が台本の起点）。
    tx.send(DispatcherMsg::Tick { now: at_ms(1_000) })
        .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick { now: at_ms(1_500) })
        .expect("send Tick(base+500ms)");

    let done = kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("kanade should receive TalkDone for B");
    match done {
        KanadeMsg::TalkDone(done) => {
            assert_eq!(
                done.talk_id, talk_b,
                "forwarded TalkDone must be for B, not A"
            );
            assert_eq!(done.reason, TalkEndReason::Ended);
        }
        _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
    }

    assert!(
        kanade_rx.try_recv().is_err(),
        "no further KanadeMsg (in particular no stale TalkDone for A) should reach kanade"
    );

    close_and_join(&tx, handle);
}

/// A→B へ差し替え済みの状態で A の `talk_id` を持つ `Done` を手動で投函しても、kanade へは
/// 転送されず、B の slot も乱れない。
#[test]
fn explicit_stale_done_after_replacement_is_discarded_without_disturbing_current_slot() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let surface = RecordingSink::new();
    let text = RecordingSink::new();

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        test_system_vars(),
        None,
    );

    let talk_a = TalkId(11);
    let talk_b = TalkId(12);

    tx.send(DispatcherMsg::Start(StartTalk::new(
        talk_a,
        r"\s[1]A\_w[2500]A_END\e",
    )))
    .expect("send Start(A)");
    tx.send(DispatcherMsg::Start(StartTalk::new(
        talk_b,
        r"\s[2]B\_w[100]B_END\e",
    )))
    .expect("send Start(B)");

    tx.send(DispatcherMsg::Done(TalkDone {
        talk_id: talk_a,
        reason: TalkEndReason::Interrupted,
        quit_reserved: false,
    }))
    .expect("send manual stale Done(A)");

    // 占有 horizon=0.40 を跨ぐ elapsed 0.5 で B を完走させる。
    tx.send(DispatcherMsg::Tick { now: at_ms(2_000) })
        .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick { now: at_ms(2_500) })
        .expect("send Tick(base+500ms)");

    let done = kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("kanade should receive TalkDone for B despite the stale A notification");
    match done {
        KanadeMsg::TalkDone(done) => {
            assert_eq!(
                done.talk_id, talk_b,
                "slot must still be B after stale Done(A)"
            );
            assert_eq!(done.reason, TalkEndReason::Ended);
        }
        _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
    }
    assert!(
        kanade_rx.try_recv().is_err(),
        "the stale Done(A) must never surface to kanade as a KanadeMsg::TalkDone"
    );

    close_and_join(&tx, handle);
}

/// 稼働中の talk がある状態で `Close` を送ると、dispatcher は active talk へ `SakuraMsg::Close` を
/// 送って join してから自身も停止する。`close_active_if_any` は `Break` より前に talk actor の join を
/// 終えるので、dispatcher の join が有界時間内に成功すれば、内側の talk actor が先に正常終了している。
#[test]
fn close_while_active_closes_and_joins_active_talk_before_stopping_dispatcher() {
    let (kanade_tx, _kanade_rx) = mpsc::channel::<KanadeMsg>();
    let surface = RecordingSink::new();
    let text = RecordingSink::new();

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        test_system_vars(),
        None,
    );

    tx.send(DispatcherMsg::Start(StartTalk::new(
        TalkId(21),
        r"\s[1]X\_w[2500]X_END\e",
    )))
    .expect("send Start");

    close_and_join(&tx, handle);
}

/// 発火の時刻（broadcast なので text sink も全 cue を受ける）:
///   ClearAll@0.0・Emote{5}@0.0・FIRST@0.0 / Wait@0.25 / SECOND@0.45 / Wait@0.75 / THIRD@1.05。
///   占有 horizon=1.30（THIRD の再生完了）。
const THREE_TEXTS: &str = r"\s[5]FIRST\_w[200]SECOND\_w[300]THIRD\e";

/// `THREE_TEXTS` を起動して刻み `ticks` を送り、dispatcher を閉じて join してから届いた Text を順に返す。
/// Close は talk を join してから dispatcher を止めるので、join の後は刻みの発火が出揃っており、
/// 「まだ出ない」を `try_recv` の競りに頼らず見られる。
fn texts_after_ticks(ticks: &[i64]) -> Vec<String> {
    let (kanade_tx, _kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (text_tx, text_rx) = mpsc::channel::<TalkCue>();
    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![
            Box::new(RecordingSink::new()),
            Box::new(ChannelSink { tx: text_tx }),
        ],
        test_system_vars(),
        None,
    );
    tx.send(DispatcherMsg::Start(StartTalk::new(
        TalkId(32),
        THREE_TEXTS,
    )))
    .expect("send Start");
    for &now in ticks {
        tx.send(DispatcherMsg::Tick { now: at_ms(now) })
            .expect("send Tick");
    }
    close_and_join(&tx, handle);
    text_rx
        .try_iter()
        .filter_map(|cue| match cue.command {
            CueCommand::Text(text) => Some(text),
            _ => None,
        })
        .collect()
}

/// 各 Tick は対応する発火群だけを解放する（elapsed 0.0 で FIRST・0.5 で SECOND まで・1.1 で THIRD まで）。
#[test]
fn tick_relay_releases_only_the_cues_due_at_each_elapsed_time() {
    assert_eq!(texts_after_ticks(&[5_000]), ["FIRST"]);
    assert_eq!(texts_after_ticks(&[5_000, 5_500]), ["FIRST", "SECOND"]);
    assert_eq!(
        texts_after_ticks(&[5_000, 5_500, 6_100]),
        ["FIRST", "SECOND", "THIRD"]
    );
}

/// dispatcher は `Tick{now}` を変えずに active talk へ中継し、talk は最初の刻みを台本の起点に押す。
/// 合図は「起点＋台本の時刻」の絶対の発火時刻で届き、届く刻みと順は相対の刻みの時と同じ。
#[test]
fn tick_relay_forwards_absolute_now_and_first_tick_becomes_origin() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (text_tx, text_rx) = mpsc::channel::<TalkCue>();
    let surface = RecordingSink::new();
    let text = ChannelSink { tx: text_tx };

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        test_system_vars(),
        None,
    );

    tx.send(DispatcherMsg::Start(StartTalk::new(
        TalkId(31),
        THREE_TEXTS,
    )))
    .expect("send Start");

    // text sink には Emote/Wait 等の担当外 cue も届くので、目的の Text まで読み飛ばす。
    let recv_text = |want: &str| {
        loop {
            let cue = text_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("due な Text 発火は届くこと");
            if cue.command == CueCommand::Text(want.into()) {
                return cue.fire_time;
            }
        }
    };
    let origin = at_ms(5_000);

    // 初回 tick 5_000 が台本の起点で、FIRST までが due。
    tx.send(DispatcherMsg::Tick { now: origin })
        .expect("send first Tick (becomes the sheet origin)");
    assert_eq!(recv_text("FIRST"), origin, "FIRST は起点ちょうどに発火する");

    // elapsed=0.5 で SECOND まで due。
    tx.send(DispatcherMsg::Tick { now: at_ms(5_500) })
        .expect("send second Tick (elapsed 0.5)");
    assert_eq!(recv_text("SECOND"), origin + ms(450));

    // elapsed=1.1 で THIRD が due。
    tx.send(DispatcherMsg::Tick { now: at_ms(6_100) })
        .expect("send third Tick (elapsed 1.1)");
    assert_eq!(recv_text("THIRD"), origin + ms(1_050));

    // elapsed=1.4 は占有 horizon=1.30 を超え、自然終端する（末尾テキストの D を落とさない）。
    tx.send(DispatcherMsg::Tick { now: at_ms(6_400) })
        .expect("send fourth Tick (elapsed 1.4 ≥ horizon 1.30)");

    let done = kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("kanade should receive TalkDone after natural completion");
    match done {
        KanadeMsg::TalkDone(done) => {
            assert_eq!(done.talk_id, TalkId(31));
            assert_eq!(done.reason, TalkEndReason::Ended);
        }
        _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
    }

    close_and_join(&tx, handle);
}

/// 中継する時刻は受けた刻みだけで決まり、talk ごとの値（その talk の最初の刻み）に依らない。
/// slot を別の talk に差し替えた後の最初の刻みも、0 に直さずに運ぶ時刻のまま中継される。
#[test]
fn tick_relay_does_not_depend_on_per_talk_first_tick() {
    let mut fx = state_fixture();
    let relayed = |rx: &mpsc::Receiver<SakuraMsg>| match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(SakuraMsg::Tick(t)) => t,
        _ => panic!("talk へ SakuraMsg::Tick が中継されるべき"),
    };

    let (first, first_rx) = spawn_probe_talk();
    occupy(&mut fx.state, TalkId(41), first);
    fx.feed(DispatcherMsg::Tick { now: at_ms(3_000) });
    fx.feed(DispatcherMsg::Tick { now: at_ms(3_050) });
    assert_eq!(
        (relayed(&first_rx), relayed(&first_rx)),
        (at_ms(3_000), at_ms(3_050))
    );
    release(&mut fx.state);

    let (second, second_rx) = spawn_probe_talk();
    occupy(&mut fx.state, TalkId(42), second);
    fx.feed(DispatcherMsg::Tick { now: at_ms(3_100) });
    assert_eq!(
        relayed(&second_rx),
        at_ms(3_100),
        "次の talk の最初の刻みも運ぶ時刻のまま"
    );
    release(&mut fx.state);
}

/// talk が自然完了すると `KanadeMsg::TalkDone` が正しい `talk_id`/`reason` で kanade へ届き、
/// slot が解放される。解放は、後続の `Start` が明示 Close なしで再生され、2 件目の `TalkDone` が
/// 過不足なく届くことで見る。
#[test]
fn natural_completion_forwards_talkdone_and_clears_slot_for_next_start() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let surface = RecordingSink::new();
    let text = RecordingSink::new();
    let surface_records = surface.records();

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        test_system_vars(),
        None,
    );

    let talk_c = TalkId(41);
    tx.send(DispatcherMsg::Start(StartTalk::new(
        talk_c,
        r"\s[9]hello\_w[100]world\e",
    )))
    .expect("send Start(C)");
    tx.send(DispatcherMsg::Tick { now: at_ms(9_000) })
        .expect("send Tick(base)");
    // C の占有 horizon=0.60 を跨ぐ elapsed 0.7。
    tx.send(DispatcherMsg::Tick { now: at_ms(9_700) })
        .expect("send Tick(base+700ms)");

    let done_c = kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("kanade should receive TalkDone for C");
    match done_c {
        KanadeMsg::TalkDone(done) => {
            assert_eq!(done.talk_id, talk_c);
            assert_eq!(done.reason, TalkEndReason::Ended);
        }
        _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
    }

    let talk_d = TalkId(42);
    tx.send(DispatcherMsg::Start(StartTalk::new(
        talk_d,
        r"\s[8]again\e",
    )))
    .expect("send Start(D)");
    // D の占有 horizon=0.25 を跨ぐ base(10_000)＋elapsed 0.3 の 2 tick。
    tx.send(DispatcherMsg::Tick { now: at_ms(10_000) })
        .expect("send Tick(base) for D");
    tx.send(DispatcherMsg::Tick { now: at_ms(10_300) })
        .expect("send Tick(base+300ms) for D");

    let done_d = kanade_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("kanade should receive TalkDone for D");
    match done_d {
        KanadeMsg::TalkDone(done) => {
            assert_eq!(done.talk_id, talk_d, "second slot occupant must be D");
            assert_eq!(done.reason, TalkEndReason::Ended);
        }
        _ => unreachable!("dispatcher only forwards KanadeMsg::TalkDone"),
    }
    assert!(
        kanade_rx.try_recv().is_err(),
        "exactly two TalkDone (C then D) — no stray duplicates or stale entries"
    );

    // surface sink には両 talk の全 cue が届くので、Emote だけを抜いて C=scope9・D=scope8 が
    // 1 件ずつなことを見る。
    let surface = surface_records.lock().expect("records mutex poisoned");
    let emotes: Vec<&CueCommand> = surface
        .iter()
        .map(|c| &c.command)
        .filter(|c| matches!(c, CueCommand::Emote { .. }))
        .collect();
    assert_eq!(
        emotes,
        vec![
            &CueCommand::Emote { key: "9".into() },
            &CueCommand::Emote { key: "8".into() },
        ],
        "broadcast 経由でも Emote 発火は C(scope9)→D(scope8) の 1 件ずつ"
    );

    close_and_join(&tx, handle);
}

/// `system_vars` provider が talk 起動ごとに一度呼ばれ、その時点で凍結されたスナップショットが
/// sakura 側のコンパイルへ流れる。
///
/// 呼び出しのたびに `username` を `user1`→`user2`… と変える provider を注入し、`%username` を含む
/// talk を 2 回起動する。各 talk の `%username` はその起動時点の値（1 本目=`user1`／2 本目=`user2`）の
/// Text cue へ展開される。値が talk 間で異なることが talk ごとの凍結の証拠で、provider の呼出回数が
/// talk 起動数と一致することは、boot 時 1 回きりの固定像でないことの証拠になる。
#[test]
fn system_vars_provider_is_invoked_and_frozen_per_talk_start() {
    use crate::runtime::SystemVarSource;
    use areka_sakura::contract::SystemVarSnapshot;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let (kanade_tx, _kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (text_tx, text_rx) = mpsc::channel::<TalkCue>();
    let surface = RecordingSink::new();
    let text = ChannelSink { tx: text_tx };

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_provider = Arc::clone(&calls);
    let provider: SystemVarSource = Box::new(move || {
        let n = calls_for_provider.fetch_add(1, Ordering::SeqCst) + 1;
        let mut snapshot = SystemVarSnapshot::default();
        snapshot.insert("username", format!("user{n}"));
        snapshot
    });

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        provider,
        None,
    );

    // text sink には ClearAll/Emote 等の担当外 cue も届くので、次の Text まで読み飛ばす。
    let recv_text = |want: &str| -> TalkCue {
        loop {
            let cue = text_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("due な Text 発火は届くこと");
            if cue.command == CueCommand::Text(want.into()) {
                return cue;
            }
        }
    };

    tx.send(DispatcherMsg::Start(StartTalk::new(
        TalkId(61),
        r"\s[0]%username\e",
    )))
    .expect("send Start(1)");
    tx.send(DispatcherMsg::Tick { now: at_ms(1_000) })
        .expect("send Tick for talk 1");
    recv_text("user1");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "provider は talk 1 の起動で 1 回だけ呼ばれる（刻印点）"
    );

    tx.send(DispatcherMsg::Start(StartTalk::new(
        TalkId(62),
        r"\s[0]%username\e",
    )))
    .expect("send Start(2)");
    tx.send(DispatcherMsg::Tick { now: at_ms(2_000) })
        .expect("send Tick for talk 2");
    recv_text("user2");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "provider の呼出回数が talk 起動数と一致（per-talk 刻印・boot 時 1 回固定でない）"
    );

    close_and_join(&tx, handle);
}
