use super::test_support::{RecordingSink, run_bounded, test_system_vars};
use super::*;
use areka_sakura::contract::{CueCommand, TalkCue, TalkEndReason};
use std::sync::mpsc;
use std::time::Duration;

/// テスト専用の `Clone` 可能なチャンネル中継 sink（発火の到着を barrier として同期受信する
/// ため、`recv_timeout` で1件ずつ決定的に観測できる・sakura drive.rs のテスト流儀を踏襲）。
#[derive(Clone)]
struct ChannelSink {
    tx: mpsc::Sender<TalkCue>,
}

impl CueSink for ChannelSink {
    fn emit(&mut self, cue: TalkCue) {
        let _ = self.tx.send(cue);
    }
}

/// task 1.4（DD-5・C9）: `From<TalkCommand> for DispatcherMsg` の**全 variant** 網羅変換。
///
/// kanade の送出口（`Sender<TalkCommand>`）から start-relay を経て dispatcher inbox へ入る
/// 唯一の変換点であり、ここで variant が落ちると当該指示が物理的に到達不能になる。3 形すべてが
/// 情報無損失で対応アームへ写ることを固定する（新 variant 追加時は本檻がコンパイルエラーで気づく）。
#[test]
fn talk_command_converts_to_dispatcher_msg_for_every_variant() {
    use areka_sakura::contract::TalkCommand;

    // Start: StartTalk を情報無損失で包み直すだけ。
    let start = StartTalk {
        talk_id: TalkId(101),
        script: r"\s[0]hi\e".to_string(),
        epilogue: Vec::new(),
    };
    match DispatcherMsg::from(TalkCommand::Start(start.clone())) {
        DispatcherMsg::Start(got) => {
            assert_eq!(got.talk_id, start.talk_id);
            assert_eq!(got.script, start.script);
            assert!(got.epilogue.is_empty());
        }
        _ => panic!("TalkCommand::Start は DispatcherMsg::Start へ変換されるべき"),
    }

    // ResolveChoice: talk_id（stale ガード用）と選択肢 id をそのまま運ぶ。
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

    // CancelChoice: talk_id をそのまま運ぶ。
    match DispatcherMsg::from(TalkCommand::CancelChoice {
        talk_id: TalkId(103),
    }) {
        DispatcherMsg::CancelChoice { talk_id } => assert_eq!(talk_id, TalkId(103)),
        _ => panic!("TalkCommand::CancelChoice は DispatcherMsg::CancelChoice へ変換されるべき"),
    }
}

/// task 1.4（DD-5・C9）: `From<ChoiceWaiting> for DispatcherMsg` の全フィールド無損失変換。
///
/// `ChoiceWaiting` は talk → dispatcher の done ポート（`spawn_talk` の `D: From<..>` 境界）を
/// 流れるため、この `From` が無ければ通知経路が型として成立しない。搬送値（候補 id 列・
/// 表示完了時刻・タイムアウト指令）が一切改変されずに届くことを固定する。
#[test]
fn choice_waiting_converts_to_dispatcher_msg_without_loss() {
    use areka_sakura::contract::ChoiceWaiting;

    let waiting = ChoiceWaiting {
        talk_id: TalkId(201),
        choice_ids: vec!["a".to_string(), "b".to_string()],
        display_end_elapsed_secs: 1.25,
        timeout_directive_secs: Some(12.0),
    };
    match DispatcherMsg::from(waiting.clone()) {
        DispatcherMsg::ChoiceWaiting(got) => assert_eq!(got, waiting),
        _ => panic!("ChoiceWaiting は DispatcherMsg::ChoiceWaiting へ変換されるべき"),
    }

    // 未指定（None＝下流既定値へ委譲）も同様に無改変で運ばれる。
    let unspecified = ChoiceWaiting {
        talk_id: TalkId(202),
        choice_ids: Vec::new(),
        display_end_elapsed_secs: 0.0,
        timeout_directive_secs: None,
    };
    match DispatcherMsg::from(unspecified.clone()) {
        DispatcherMsg::ChoiceWaiting(got) => assert_eq!(got, unspecified),
        _ => panic!("ChoiceWaiting は DispatcherMsg::ChoiceWaiting へ変換されるべき"),
    }
}

/// シナリオ1: 単一 slot 維持・置き換え。`Start(A)` 稼働中に `Start(B)` を送ると、A は
/// Close-then-join で終了してから B が spawn される。A の完了通知（`Interrupted`）は
/// 既に B へ差し替わった後に dispatcher inbox へ届く stale 通知となり、kanade へは決して
/// 転送されない（要件 4.1/4.2・stale 棄却は Close-then-spawn の直接帰結として自然発生する）。
#[test]
fn start_then_start_replaces_active_talk_and_discards_stale_done_from_replaced_talk() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let surface = RecordingSink::new();
    let text = RecordingSink::new();

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        test_system_vars(),
    );

    let talk_a = TalkId(1);
    let talk_b = TalkId(2);

    // A: 長い待ちを持つ script（差し替えまで一切 Tick を送らないので自然完了しない）。
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: talk_a,
        script: r"\s[1]A\w[50]A_END\e".to_string(),
    }))
    .expect("send Start(A)");

    // B: 短い script（差し替え後にこれを完走させる）。
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: talk_b,
        script: r"\s[2]B\w[2]B_END\e".to_string(),
    }))
    .expect("send Start(B)");

    // B を完走させる（D 焼き込み後 B/B_END の再生完了＋\w[2] を含む占有 horizon=0.40 を跨ぐ
    // elapsed 0.5・base_now は最初の Tick の now で確定）。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(1_000),
    })
    .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(1_500),
    })
    .expect("send Tick(base+500ms)");

    // kanade は B の TalkDone のみを受け取る（A の stale Interrupted は転送されない）。
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

    // A についての通知は決して kanade へ届かない（stale 棄却の直接観測）。
    assert!(
        kanade_rx.try_recv().is_err(),
        "no further KanadeMsg (in particular no stale TalkDone for A) should reach kanade"
    );

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

/// シナリオ2: 明示的な stale `Done` の棄却。A→B へ差し替え済みの状態で、A の
/// `talk_id` を持つ `Done` を手動投函しても kanade へは転送されず、B の slot は
/// 乱されない（要件 4.4 の直接固定）。
#[test]
fn explicit_stale_done_after_replacement_is_discarded_without_disturbing_current_slot() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let surface = RecordingSink::new();
    let text = RecordingSink::new();

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        test_system_vars(),
    );

    let talk_a = TalkId(11);
    let talk_b = TalkId(12);

    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: talk_a,
        script: r"\s[1]A\w[50]A_END\e".to_string(),
    }))
    .expect("send Start(A)");
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: talk_b,
        script: r"\s[2]B\w[2]B_END\e".to_string(),
    }))
    .expect("send Start(B)");

    // 手動で A の stale Done（自然発生分に加えた明示的な追験）を投函する。
    tx.send(DispatcherMsg::Done(TalkDone {
        talk_id: talk_a,
        reason: TalkEndReason::Interrupted,
    }))
    .expect("send manual stale Done(A)");

    // B の slot は乱されず、B を完走させれば正しく kanade へ転送される
    // （D 焼き込み後の占有 horizon=0.40 を跨ぐ elapsed 0.5）。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(2_000),
    })
    .expect("send Tick(base)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(2_500),
    })
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

/// シナリオ3: 停止時のクリーンアップ。稼働中の talk がある状態で `Close` を送ると、
/// dispatcher は active talk へ `SakuraMsg::Close` を送って join してから、自身も
/// 停止する。`close_active_if_any` は `Break` より前に talk actor の join を完了させる
/// ため、dispatcher 自身の join が有界時間内に成功すること自体が、内側の talk actor が
/// 先に正常終了していたことの直接証跡になる（要件 4.5）。
#[test]
fn close_while_active_closes_and_joins_active_talk_before_stopping_dispatcher() {
    let (kanade_tx, _kanade_rx) = mpsc::channel::<KanadeMsg>();
    let surface = RecordingSink::new();
    let text = RecordingSink::new();

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        test_system_vars(),
    );

    // 長い待ちを持つ script（Close 時点では自然完了していない）。
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: TalkId(21),
        script: r"\s[1]X\w[50]X_END\e".to_string(),
    }))
    .expect("send Start");

    tx.send(DispatcherMsg::Close).expect("send Close");

    run_bounded(
        "dispatcher join after Close with active talk",
        Duration::from_secs(5),
        move || {
            handle.join().expect(
                "dispatcher (and therefore its active talk, joined synchronously beforehand) \
                 terminates normally after Close",
            );
        },
    );
}

/// シナリオ4: 経過時間換算を伴う Tick 中継。複数の `Tick{now}` を送ると、dispatcher は
/// 最初の tick を経過秒 0.0 の起点として記録し、以降 `(now - base) / 1000.0` 秒を
/// `SakuraMsg::Tick(f64)` として active talk へ中継する（要件 5.2）。barrier 技法
/// （sakura drive.rs 流儀）で、各 Tick が対応する発火群のみを解放することを決定的に確認する。
#[test]
fn tick_relay_converts_absolute_now_to_elapsed_seconds_from_first_tick() {
    let (kanade_tx, kanade_rx) = mpsc::channel::<KanadeMsg>();
    let (text_tx, text_rx) = mpsc::channel::<TalkCue>();
    let surface = RecordingSink::new();
    let text = ChannelSink { tx: text_tx };

    let (tx, handle) = spawn_dispatcher(
        kanade_tx,
        vec![Box::new(surface), Box::new(text)],
        test_system_vars(),
    );

    // \w[4]=200ms・\w[6]=300ms。D 焼き込み後の発火（broadcast ゆえ text sink も全 cue を受ける）:
    //   ClearAll@0.0・Emote{5}@0.0・FIRST@0.0 / Wait@0.25 / SECOND@0.45（FIRST の D=0.25 + \w[4]=0.20）/
    //   Wait@0.75 / THIRD@1.05（SECOND の D=0.30 + \w[6]=0.30）。占有 horizon=1.30（THIRD 再生完了）。
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: TalkId(31),
        script: r"\s[5]FIRST\w[4]SECOND\w[6]THIRD\e".to_string(),
    }))
    .expect("send Start");

    // broadcast: text sink には Emote/Wait 等の担当外 cue も届く。本テストは Text 発火の
    // 順序・保留のみを観測するため、次の目的 Text 発火まで担当外 cue を読み飛ばす barrier ヘルパを使う。
    let recv_text = |want: &str| {
        loop {
            let cue = text_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("due な Text 発火は届くこと");
            if cue.command == CueCommand::Text(want.into()) {
                return cue;
            }
        }
    };

    // 初回 tick: elapsed=0.0 起点 → ClearAll@0.0・Emote@0.0・FIRST@0.0 のみ due（SECOND/THIRD は未 due）。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(5_000),
    })
    .expect("send first Tick (anchors base_now)");
    let first = recv_text("FIRST");
    assert_eq!(first.command, CueCommand::Text("FIRST".into()));
    // FIRST まで drain した時点で Wait@0.25/SECOND@0.45/THIRD は未 due（保留の決定的証明）。
    assert!(
        text_rx.try_recv().is_err(),
        "SECOND/THIRD must not fire before their elapsed time is reached"
    );

    // 2 回目 tick: now - base = 500ms → elapsed=0.5 → Wait@0.25・SECOND@0.45 due（THIRD@1.05 はまだ）。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(5_500),
    })
    .expect("send second Tick (elapsed 0.5)");
    let second = recv_text("SECOND");
    assert_eq!(second.command, CueCommand::Text("SECOND".into()));
    assert!(
        text_rx.try_recv().is_err(),
        "THIRD must not fire before elapsed 1.05 is reached"
    );

    // 3 回目 tick: now - base = 1100ms → elapsed=1.1 → Wait@0.75・THIRD@1.05 due。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(6_100),
    })
    .expect("send third Tick (elapsed 1.1)");
    let third = recv_text("THIRD");
    assert_eq!(third.command, CueCommand::Text("THIRD".into()));

    // 4 回目 tick: elapsed=1.4 → 占有 horizon=1.30 到達で自然終端（末尾テキストの D を落とさない）。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(6_400),
    })
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

/// シナリオ5: 完了通知の転送（happy path）。talk が自然完了すると `KanadeMsg::TalkDone`
/// が正しい `talk_id`/`reason` で kanade へ届き、slot は解放される。解放されたことは、
/// 後続の `Start` が（明示 Close を要さず）新しい talk を正しく再生し、2 件目の
/// `TalkDone` も過不足なく届くことで確認する（要件 4.3）。
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
    );

    let talk_c = TalkId(41);
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: talk_c,
        script: r"\s[9]hello\w[2]world\e".to_string(),
    }))
    .expect("send Start(C)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(9_000),
    })
    .expect("send Tick(base)");
    // D 焼き込み後 C の占有 horizon=0.60（world 再生完了）を跨ぐ elapsed 0.7。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(9_700),
    })
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

    // slot は解放済み: 後続 Start は Close を要さず新規 talk をそのまま再生できる。
    let talk_d = TalkId(42);
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: talk_d,
        script: r"\s[8]again\e".to_string(),
    }))
    .expect("send Start(D)");
    // D（`again`＝5 char・D=0.25）の占有 horizon=0.25 を跨ぐため base(10_000)＋elapsed 0.3 の 2 tick。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(10_000),
    })
    .expect("send Tick(base) for D");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(10_300),
    })
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

    // broadcast: surface sink には両 talk の全 cue（ClearAll/Text/Wait 含む）が届くため、
    // Emote 発火だけを抽出して「C=scope9・D=scope8 が 1 件ずつ」を確認する（partition は演者側 relevance の責務）。
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

/// シナリオ6（task 6.2・凍結像の刻印点）: `system_vars` provider が talk 起動ごとに
/// 一度呼び出され、その時点で凍結されたスナップショットが sakura 側のコンパイルへ流れる
/// ことを end-to-end に固定する（R7.3/7.4）。
///
/// 呼び出しのたびに `username` を `user1`→`user2`… と変える counter provider を注入し、
/// `%username` を含む talk を 2 回起動する。各 talk の `%username` は**その talk の起動
/// 時点で凍結された**値（1 本目=`user1`／2 本目=`user2`）の Text cue へ展開され、broadcast
/// で観測できる。値が talk 間で異なること自体が「talk ごとに独立して凍結される」意味論
/// （sylphya の per-talk 凍結と同形）の直接証跡になる。provider の呼出回数が talk 起動数と
/// 一致することも固定する（＝ per-talk 刻印であって boot 時 1 回きりの固定像ではない）。
///
/// task 6.1 の暫定既定橋渡し（`SystemVarSnapshot::default()`）のままでは provider は
/// 一度も呼ばれず、`%username` は既定値 `DEFAULT_USERNAME` へ展開されるため、本檻は
/// `user1`/`user2` を観測できず（かつ呼出回数 0）RED になる。
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

    // 呼び出しごとに username を `user{n}` と変える provider（凍結＝各 talk が自分の
    // 起動時点の値を見ることの証明用）。呼出回数も観測する。
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_for_provider = Arc::clone(&calls);
    let provider: SystemVarSource = Box::new(move || {
        let n = calls_for_provider.fetch_add(1, Ordering::SeqCst) + 1;
        let mut snapshot = SystemVarSnapshot::default();
        snapshot.insert("username", format!("user{n}"));
        snapshot
    });

    let (tx, handle) =
        spawn_dispatcher(kanade_tx, vec![Box::new(surface), Box::new(text)], provider);

    // broadcast: text sink には ClearAll/Emote 等の担当外 cue も届く。次の Text 発火まで読み飛ばす。
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

    // talk 1: `%username`（→ 起動時点で凍結された provider 値 `user1` へ展開）。
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: TalkId(61),
        script: r"\s[0]%username\e".to_string(),
    }))
    .expect("send Start(1)");
    // 初回 Tick で base_now 刻印＋elapsed=0.0 群（ClearAll/Emote/Text@0.0）を発火。
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(1_000),
    })
    .expect("send Tick for talk 1");
    let first = recv_text("user1");
    assert_eq!(
        first.command,
        CueCommand::Text("user1".into()),
        "talk 1 の %username は起動時点で凍結された provider 値 user1 へ展開される（既定値でない）"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "provider は talk 1 の起動で 1 回だけ呼ばれる（刻印点）"
    );

    // talk 2: 差し替え起動（talk 1 は Close funnel で終了）。provider の次の呼出＝`user2`。
    tx.send(DispatcherMsg::Start(StartTalk {
        epilogue: Vec::new(),
        talk_id: TalkId(62),
        script: r"\s[0]%username\e".to_string(),
    }))
    .expect("send Start(2)");
    tx.send(DispatcherMsg::Tick {
        now: MonotonicMs(2_000),
    })
    .expect("send Tick for talk 2");
    let second = recv_text("user2");
    assert_eq!(
        second.command,
        CueCommand::Text("user2".into()),
        "talk 2 は自分の起動時点で凍結された provider 値 user2 を見る（talk ごと独立凍結）"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        2,
        "provider の呼出回数が talk 起動数と一致（per-talk 刻印・boot 時 1 回固定でない）"
    );

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
