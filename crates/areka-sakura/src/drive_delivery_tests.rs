use super::*;
use crate::contract::{CueCommand, TalkCue, TalkId};
use crate::duration::text_playback_duration;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use super::test_support::*;

/// **broadcast**: 登録された全 sink が**同一の cue 列を同一順序で**受信する（中央振り分け廃止・
/// 演者側 relevance が action 選別・D4/R2.1）。`\s[10]hello\w[2]world\e` を 2 つの記録 sink で
/// 駆動し、両者が ClearAll/Emote/hello/Wait/world を過不足なく受けることを固定する。
#[test]
fn broadcast_delivers_identical_cue_stream_to_every_registered_sink() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[2]world\e".to_string(),
        talk_id: TalkId(200),
    };
    let surface = RecordingSink::new();
    let text = RecordingSink::new();
    let surface_records = surface.records();
    let text_records = text.records();

    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(surface, text),
        SystemVarSnapshot::default(),
    );
    // 初回 Tick(0.0) でアンカー刻印（0.0）、占有 horizon（world 再生完了＝0.35+0.25=0.60）を跨ぐ 1.0。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
    recv_done(&done_rx, Duration::from_secs(5))
        .expect("自然終端で TalkDone");
    handle.actor.join().expect("body は正常終了する");

    // 期待 broadcast 列（両 sink が同一）: ClearAll@0 / Emote{10}@0 / hello@0 / Wait@0.25 / world@0.35。
    let expected = vec![
        CueCommand::ClearAll,
        CueCommand::Emote { key: "10".into() },
        CueCommand::Text("hello".into()),
        CueCommand::Wait,
        CueCommand::Text("world".into()),
    ];
    assert_eq!(
        commands(&surface_records),
        expected,
        "surface sink が全 cue を broadcast 受信する（Emote だけでなく ClearAll/hello/Wait/world も）"
    );
    assert_eq!(
        commands(&text_records),
        expected,
        "text sink も同一の全 cue を broadcast 受信する（中央振り分けなし）"
    );
}

/// **観測可能な完了条件（task 7.1）**: 同一台本を 2 回**異なる時刻で再生開始**すると、同一 cue が
/// **異なる絶対発火時刻**で配送される（絶対開始時刻が dispatch 刻印され honor される・R9.1/D6）。
///
/// `\s[0]hi\w[10]bye\e` の "bye" は相対 `at=0.6`（hi の D=0.1 ＋ `\w[10]`=0.5）。初回 Tick を
/// アンカー `A` として、"bye" の絶対発火時刻は `A + 0.6`。2 つの anchor（10.0 / 20.0）で
/// 再生開始すると "bye" の発火時刻は 10.6 / 20.6 と**異なる**。
///
/// 弁別（アンカー未刻印なら FAIL）: 初回 Tick(A) の時点では offset=0 ゆえ "bye"（at=0.6）は
/// **保留**される。もしアンカーを刻印せず 0.0 のままなら offset=A（=10 や 20）が既に 0.6 を
/// 超え、初回 Tick で "bye" が即発火してしまう＝下の「初回 Tick 直後は bye 未着」assert が FAIL する。
#[test]
fn same_sheet_started_at_different_times_delivers_cue_at_different_absolute_fire_times() {
    // 1 回の再生を anchor で駆動し、(初回Tick直後にbye未着か, A+0.5でbye未着か, A+0.6でbye着弾か) を返す。
    fn run_with_anchor(anchor: f64) -> (bool, bool, bool) {
        let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
        let start = StartTalk {
            epilogue: Vec::new(),
            script: r"\s[0]hi\w[10]bye\e".to_string(),
            talk_id: TalkId(1),
        };
        let (tx, rx) = mpsc::channel::<TalkCue>();
        let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

        // barrier 技法: 記録 sink を挟まず、bye の着弾のみをチャンネルで観測する。
        let bye_seen = |rx: &mpsc::Receiver<TalkCue>| -> bool {
            let mut seen = false;
            while let Ok(cue) = rx.try_recv() {
                if cue.command == CueCommand::Text("bye".into()) {
                    seen = true;
                }
            }
            seen
        };
        // 「この Tick 送出＋ドレインまでに bye が届いたか」を決定的に観測するため、Tick 投函後に
        // done も含めた barrier で drain を同期する。ここでは十分に決定的な probe cue で代替する:
        // 各 Tick 後に "hi"（初回群）や world を受けるので、それを recv barrier に使う。

        // 初回 Tick(A): offset 0 → ClearAll/Emote/hi が due。bye(0.6) は保留のはず。
        handle.inbox.send(SakuraMsg::Tick(anchor)).unwrap();
        // hi 着弾を barrier に、初回群の drain 完了を待つ（bye は同 tick で来ない）。
        let mut hi_seen = false;
        while !hi_seen {
            match rx.recv_timeout(Duration::from_secs(5)) {
                Ok(cue) if cue.command == CueCommand::Text("hi".into()) => hi_seen = true,
                Ok(_) => {}
                Err(_) => panic!("初回群の hi が届かない"),
            }
        }
        let bye_after_first = bye_seen(&rx);

        // Tick(A+0.5): offset 0.5 → Wait(0.25? いや at=0.1) は due だが bye(0.6) は保留。
        handle.inbox.send(SakuraMsg::Tick(anchor + 0.5)).unwrap();
        std::thread::yield_now();
        // Wait cue の着弾を barrier に使う（at=0.1 <= 0.5 ゆえこの Tick までに届く）。
        let mut wait_seen = false;
        for _ in 0..1000 {
            match rx.try_recv() {
                Ok(cue) if cue.command == CueCommand::Wait => {
                    wait_seen = true;
                    break;
                }
                Ok(_) => {}
                Err(TryRecvError::Empty) => std::thread::yield_now(),
                Err(TryRecvError::Disconnected) => break,
            }
        }
        assert!(wait_seen, "Wait(at=0.1) は A+0.5 までに届くはず（barrier）");
        let bye_after_half = bye_seen(&rx);

        // Tick(A+0.6): offset 0.6 → bye が due。着弾を待つ。
        handle.inbox.send(SakuraMsg::Tick(anchor + 0.6)).unwrap();
        let mut bye_after_full = false;
        // 自然終端まで進めてから observe すると drain が確定する。horizon=0.75 を跨ぐ A+1.0。
        handle.inbox.send(SakuraMsg::Tick(anchor + 1.0)).unwrap();
        recv_done(&done_rx, Duration::from_secs(5))
            .expect("horizon 到達で TalkDone");
        handle.actor.join().expect("body 正常終了");
        while let Ok(cue) = rx.try_recv() {
            if cue.command == CueCommand::Text("bye".into()) {
                bye_after_full = true;
            }
        }

        (bye_after_first, bye_after_half, bye_after_full)
    }

    // Run A（anchor 10.0）と Run B（anchor 20.0）。
    let (a_first, a_half, a_full) = run_with_anchor(10.0);
    let (b_first, b_half, b_full) = run_with_anchor(20.0);

    // 弁別の核心: 初回 Tick(A) 直後は bye 未着（アンカー刻印されているから offset=0）。
    // アンカー未刻印（0.0 固定）なら初回 Tick で offset=A>0.6 ゆえ bye が即着＝この assert が FAIL する。
    assert!(
        !a_first,
        "Run A: 初回 Tick(10.0) 直後は bye 未着（アンカー刻印の弁別）"
    );
    assert!(
        !b_first,
        "Run B: 初回 Tick(20.0) 直後は bye 未着（アンカー刻印の弁別）"
    );
    // A+0.5（=offset 0.5）でもまだ bye は保留（0.6 未達）。
    assert!(!a_half, "Run A: offset 0.5 では bye(0.6) 保留");
    assert!(!b_half, "Run B: offset 0.5 では bye(0.6) 保留");
    // A+0.6（=offset 0.6）で初めて bye が着弾する（＝絶対発火時刻 anchor+0.6）。
    assert!(a_full, "Run A: offset 0.6（絶対 10.6）で bye 着弾");
    assert!(b_full, "Run B: offset 0.6（絶対 20.6）で bye 着弾");
    // 同一 cue が 2 回の再生で異なる絶対発火時刻（10.6 vs 20.6）で配送された（構成的に相異）。
    assert_ne!(
        10.0 + 0.6,
        20.0 + 0.6,
        "同一台本を異なる時刻に再生開始すると bye の絶対発火時刻が異なる（10.6 != 20.6）"
    );
}

/// 未 due の発火は Tick を受けても**保留**され、`at` 到達（境界含む・`at <= offset`）の Tick で
/// 初めて配送されることを**中間観測で決定的に**検証する（実時計・sleep 非依存）。broadcast ゆえ
/// 単一の記録チャンネル sink が全 cue（surface/text の別なく）を受ける。
///
/// script `\s[10]hello\w[2]probeA\w[2]probeB\w[2]world\e` の発火予定（D 焼き込み後・アンカー 0）:
///   ClearAll@0・Emote{10}@0・hello@0 / Wait@0.25 / probeA@0.35 / Wait@0.65 / probeB@0.75 /
///   Wait@1.05 / world@1.15。probe 受信を barrier に、未 due cue が保留されることを try_recv Empty で固定する。
#[test]
fn undue_cues_are_withheld_until_their_at_is_reached() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(314);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[2]probeA\w[2]probeB\w[2]world\e".to_string(),
        talk_id,
    };

    let (tx, rx) = mpsc::channel::<TalkCue>();
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

    let d_hello = text_playback_duration("hello"); // 0.25
    let d_probe = text_playback_duration("probeA"); // 0.30
    let w = Duration::from_millis(100).as_secs_f64(); // \w[2] = 0.10
    let at_a = d_hello + w; // probeA: 0.35
    let at_b = at_a + d_probe + w; // probeB: 0.75
    let at_w = at_b + d_probe + w; // world:  1.15

    let recv = |rx: &mpsc::Receiver<TalkCue>| {
        rx.recv_timeout(Duration::from_secs(5))
            .expect("due な発火は届くこと")
    };
    // probe cue（Text）だけを追う barrier ヘルパ（Wait 等は読み飛ばす）。
    let recv_text = |rx: &mpsc::Receiver<TalkCue>, want: &str| {
        loop {
            let cue = recv(rx);
            if cue.command == CueCommand::Text(want.into()) {
                return cue;
            }
        }
    };

    // 初回 Tick(0.0) でアンカー刻印（0）。ClearAll/Emote/hello が due（probe は未 due）。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    let hello = recv_text(&rx, "hello");
    assert_eq!(hello.at, 0.0, "hello の発火時刻は 0.0");
    // 初回群 drain 後、probeA(0.35) は未着（保留の決定的証明）。
    assert_eq!(
        rx.try_recv().unwrap_err(),
        TryRecvError::Empty,
        "初回 Tick(0.0) では未 due の probeA(0.35) が保留されること"
    );

    // Tick(at_a=0.35): Wait@0.25 と probeA@0.35 が due。probeB/world は未 due。
    handle.inbox.send(SakuraMsg::Tick(at_a)).unwrap();
    let probe_a = recv_text(&rx, "probeA");
    assert_eq!(probe_a.at, at_a, "probeA の発火時刻は 0.35");
    assert_eq!(
        rx.try_recv().unwrap_err(),
        TryRecvError::Empty,
        "at=0.35 の Tick では未 due の probeB(0.75)/world(1.15) が保留されること"
    );

    // Tick(at_b=0.75): probeB@0.75 が新規 due。world は依然未 due。
    handle.inbox.send(SakuraMsg::Tick(at_b)).unwrap();
    let probe_b = recv_text(&rx, "probeB");
    assert_eq!(probe_b.at, at_b, "probeB の発火時刻は 0.75");
    assert_eq!(
        rx.try_recv().unwrap_err(),
        TryRecvError::Empty,
        "at=0.75 の Tick でも未 due の world(1.15) が保留されること"
    );

    // Tick(at_w=1.15): world@1.15 が due（境界含む `at <= offset`）→ ここで初めて発火。
    handle.inbox.send(SakuraMsg::Tick(at_w)).unwrap();
    let world = recv_text(&rx, "world");
    assert_eq!(world.at, at_w, "world の発火時刻は 1.15（境界包含で発火）");

    // 占有 horizon（world 再生完了＝1.15+0.25=1.40）を跨ぐ Tick で自然終端。
    handle.inbox.send(SakuraMsg::Tick(2.0)).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("末尾到達で TalkDone");
    assert_eq!(done.talk_id, talk_id, "talk_id エコー");
    assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
    handle.actor.join().expect("body は正常終了する");
}

/// **同一 `at` の発火順が記述順（FIFO）で保たれる**ことを broadcast の単一記録 sink で固定する
/// （canonical 変換 `to_talk_schedule` の per-cue insert の load-bearing 性質）。
///
/// script `\s[10]hello\nworld\e` → 発火（アンカー 0）:
///   ClearAll@0 / Emote{10}@0 / Text(hello)@0（at=0 群）→ NewLine@0.25 / Text(world)@0.25（at=0.25 群）。
#[test]
fn same_at_cues_preserve_script_order_fifo() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\nworld\e".to_string(),
        talk_id: TalkId(41),
    };
    let sink = RecordingSink::new();
    let records = sink.records();

    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(sink, NoopSink),
        SystemVarSnapshot::default(),
    );
    // 初回 Tick(0.0) 刻印＋単一 Tick(0.5) で全 due（world 再生完了 horizon=0.50 到達）→自然終端。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(0.5)).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("単一 Tick で自然終端");
    assert_eq!(done.reason, TalkEndReason::Ended);
    handle.actor.join().expect("body は正常終了する");

    let d_hello = text_playback_duration("hello");
    let recs = records.lock().unwrap();
    // 記述順（FIFO）: ClearAll/Emote/hello（at=0 群）→ NewLine/world（at=0.25 群）の 5 件。
    assert_eq!(
        recs.len(),
        5,
        "broadcast は ClearAll/Emote/hello/NewLine/world の 5 件"
    );
    assert_eq!(
        recs[0].command,
        CueCommand::ClearAll,
        "冒頭は全消去 ClearAll（at=0）"
    );
    assert_eq!(recs[0].at, 0.0);
    assert_eq!(recs[1].command, CueCommand::Emote { key: "10".into() });
    assert_eq!(recs[1].at, 0.0);
    assert_eq!(recs[2].command, CueCommand::Text("hello".into()));
    assert_eq!(recs[2].at, 0.0);
    assert!(
        matches!(recs[3].command, CueCommand::NewLine { .. }),
        "at=0.25 群先頭は NewLine（FIFO・extend なら逆順化する）"
    );
    assert_eq!(recs[3].at, d_hello);
    assert_eq!(recs[4].command, CueCommand::Text("world".into()));
    assert_eq!(recs[4].at, d_hello);
}

/// fixture 駆動の統合テスト（主 observable・R9.3）。`\s[10]hello\w[2]world\e` を注入 Tick 列で
/// 駆動し、broadcast の単一記録 sink が ClearAll/Emote/hello/Wait/world を **at 昇順・FIFO** で
/// 受け、最後に `TalkDone{Ended}`（talk_id エコー・R6.6）が返ることを確認する。
///
/// **task 9.5（再生時間搬送 e2e・R1.1/7.1）**: 併せて、各 delivered cue の **envelope
/// `duration`** が、コンパイル時に焼き込んだ再生時間と**同一算術**（テキストは
/// `text_playback_duration`・`\w[2]` は 2×50ms の `Duration` 算術）で一致することを固定する。
/// これは実際の `compile → drive → CuePlayer broadcast → sink` 経路上で観測した delivered
/// duration が無変形で届くことの唯一の檻であり（他 hop は個別 crate で既に檻済み）、演者側
/// reveal 完了時刻（区間 `[at, at+duration)` の終端）を導く素が正しく搬送されることを示す。
#[test]
fn fixture_script_drives_broadcast_and_returns_ended() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(42);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[2]world\e".to_string(),
        talk_id,
    };
    let sink = RecordingSink::new();
    let records = sink.records();

    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(sink, NoopSink),
        SystemVarSnapshot::default(),
    );

    let at_world = text_playback_duration("hello") + Duration::from_millis(100).as_secs_f64();

    // 初回 Tick(0.0) 刻印＋占有 horizon（world 再生完了＝at_world+0.25=0.60）を跨ぐ Tick(1.0)。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("自然終端で TalkDone が返るべき");
    assert_eq!(done.talk_id, talk_id, "talk_id エコー（R6.6）");
    assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
    handle.actor.join().expect("body は正常終了する");

    let recs = records.lock().unwrap();
    // ClearAll@0 / Emote{10}@0 / hello@0 / Wait@0.25 / world@0.35（at 昇順・FIFO）。
    assert_eq!(
        recs.len(),
        5,
        "broadcast は 5 件（ClearAll/Emote/hello/Wait/world）"
    );
    assert_eq!(recs[0].command, CueCommand::ClearAll);
    assert_eq!(recs[0].at, 0.0);
    assert_eq!(recs[1].command, CueCommand::Emote { key: "10".into() });
    assert_eq!(recs[1].at, 0.0);
    assert_eq!(recs[1].actor.as_str(), "0", "既定 scope=0 の転写");
    assert_eq!(recs[2].command, CueCommand::Text("hello".into()));
    assert_eq!(recs[2].at, 0.0);
    assert_eq!(
        recs[3].command,
        CueCommand::Wait,
        "Wait cue も broadcast される（旧中央振り分けは skip していた）"
    );
    assert_eq!(recs[3].at, text_playback_duration("hello"));
    assert_eq!(recs[4].command, CueCommand::Text("world".into()));
    assert_eq!(recs[4].at, at_world, "world は hello の D＋\\w[2] 後に発火");
    for pair in recs.windows(2) {
        assert!(pair[0].at <= pair[1].at, "broadcast は at 昇順");
    }

    // ── task 9.5: delivered envelope duration の無変形搬送檻（R1.1/7.1） ──
    // 期待値は production 経路と**同一算術**で導く（10 進リテラル直書きは IEEE-754 表現誤差ゆえ
    // 使わない）: テキストは compile が呼ぶのと同じ `text_playback_duration`、`\w[2]` は parser が
    // 生成するのと同じ `Duration::from_millis(2 × 50ms).as_secs_f64()`。この delivered duration が
    // 期待値とビット同一（`==`）なら、D 焼き込み → `to_talk_schedule` → CuePlayer broadcast の
    // どの hop でも duration が落とされ／ゼロ化され／再導出されていない（無変形搬送）ことの証拠。
    let d_hello = text_playback_duration("hello");
    let w2 = Duration::from_millis(100).as_secs_f64(); // \w[2] = 2 × 50ms（parser 算術と同一）
    let d_world = text_playback_duration("world");
    assert_eq!(recs[0].duration, 0.0, "ClearAll は瞬時（duration=0）");
    assert_eq!(recs[1].duration, 0.0, "Emote は瞬時（duration=0）");
    assert_eq!(
        recs[2].duration, d_hello,
        "hello の delivered duration はコンパイル焼き込み D（text_playback_duration）と無変形一致（R1.1/7.1）"
    );
    assert_eq!(
        recs[3].duration, w2,
        "Wait の delivered duration は \\w[2]=100ms（envelope duration が待ち時間を担う・無変形）"
    );
    assert_eq!(
        recs[4].duration, d_world,
        "world の delivered duration もコンパイル焼き込み D と無変形一致（演者側 reveal 完了時刻の素）"
    );

    // 演者側 reveal 完了時刻は delivered cue の区間 `[at, at+duration)` 終端で導かれる
    // （emo-text state.rs 檻）。その素になる hello の占有終端（at+duration）が後続 Wait の発火
    // 時刻（＝hello 再生完了後）と一致することを固定し、焼き込み duration が下流タイムラインの
    // 整列に無変形で効く e2e（コンパイル値 → reveal 完了時刻が同一算術）を drive 層で観測する。
    assert_eq!(
        recs[2].at + recs[2].duration,
        recs[3].at,
        "hello の reveal 完了時刻（at+duration）は後続 Wait の発火時刻と一致（焼き込み duration が整列の素）"
    );
}

/// 冪等/逆行 `Tick` で二重発火しない（設計クリティカルな二重発火ガードの固定・R11.x）。
#[test]
fn duplicate_and_backward_tick_do_not_double_fire() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[10]world\e".to_string(),
        talk_id: TalkId(1),
    };
    let sink = RecordingSink::new();
    let records = sink.records();

    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(sink, NoopSink),
        SystemVarSnapshot::default(),
    );

    // 初回 Tick(0.0) 刻印。同値・逆行 Tick を織り交ぜて at=0.0 群を発火させる。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap(); // 同値 → no-op
    handle.inbox.send(SakuraMsg::Tick(-1.0)).unwrap(); // 逆行 → no-op
    handle.inbox.send(SakuraMsg::Tick(0.1)).unwrap(); // 前進だが world(at=0.75) 未達

    // 終端まで進める（hello D=0.25＋\w[10]=0.5 後 world@0.75・horizon=1.0 を跨ぐ）。
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("終端で TalkDone");
    assert_eq!(done.reason, TalkEndReason::Ended);
    handle.actor.join().expect("body は正常終了する");

    // 二重発火なし: ClearAll/Emote/hello/Wait/world 各 1 回＝5 件。
    assert_eq!(
        commands(&records),
        vec![
            CueCommand::ClearAll,
            CueCommand::Emote { key: "10".into() },
            CueCommand::Text("hello".into()),
            CueCommand::Wait,
            CueCommand::Text("world".into()),
        ],
        "dupe/逆行 Tick でも二重発火しない（各 cue 1 回）"
    );
}

/// 非有限 `Tick`（`NaN`/`inf`）は無視され再生が破綻しない（R11.1/11.2）。刻印前（`Armed`）でも
/// 非有限 Tick でアンカーを刻印せず（NaN アンカー防止）、その後の正常 Tick で通常どおり終端する。
#[test]
fn non_finite_tick_is_ignored_and_playback_survives() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[2]world\e".to_string(),
        talk_id: TalkId(9),
    };
    let sink = RecordingSink::new();
    let records = sink.records();

    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(sink, NoopSink),
        SystemVarSnapshot::default(),
    );

    // 刻印前に非有限 Tick を送る: 無視され（error ログ＋SakuraError 記録）刻印もされない。
    handle.inbox.send(SakuraMsg::Tick(f64::NAN)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(f64::INFINITY)).unwrap();

    // 正常 Tick 列で通常どおり駆動・終端する（ガードがループを殺していないことの証）。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("非有限 Tick 後も正常 Tick で終端するべき");
    assert_eq!(done.reason, TalkEndReason::Ended, "再生は破綻せず Ended");
    handle.actor.join().expect("body は正常終了する");

    // 非有限 Tick で早期全量配信されず、正常 Tick 分だけ届く（5 件）。
    assert_eq!(
        records.lock().unwrap().len(),
        5,
        "非有限 Tick で早期全量配信されていない（ClearAll/Emote/hello/Wait/world）"
    );
}

/// 同一 fixture script＋同一注入 Tick 列を N 回実行し、毎回**同一の観測結果**（cue 列・at・
/// actor・順序・終端理由）が得られることを確認する（R9.4・決定的再現）。
#[test]
fn same_fixture_and_tick_sequence_produces_identical_observation_each_run() {
    type CueKey = (u64, String, CueCommand);
    fn project(records: &Arc<Mutex<Vec<TalkCue>>>) -> Vec<CueKey> {
        records
            .lock()
            .unwrap()
            .iter()
            .map(|c| {
                (
                    c.at.to_bits(),
                    c.actor.as_str().to_string(),
                    c.command.clone(),
                )
            })
            .collect()
    }

    fn run_once() -> (Vec<CueKey>, TalkEndReason) {
        let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
        let start = StartTalk {
            epilogue: Vec::new(),
            script: r"\s[10]hello\w[2]world\e".to_string(),
            talk_id: TalkId(7),
        };
        let sink = RecordingSink::new();
        let records = sink.records();

        let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(sink, NoopSink),
        SystemVarSnapshot::default(),
    );
        handle.inbox.send(SakuraMsg::Tick(0.0)).expect("Tick(0.0)");
        handle.inbox.send(SakuraMsg::Tick(1.0)).expect("Tick(1.0)");

        let done = recv_done(&done_rx, Duration::from_secs(5))
            .expect("自然終端で TalkDone");
        handle.actor.join().expect("body 正常終了");
        (project(&records), done.reason)
    }

    const RUNS: usize = 3;
    let baseline = run_once();
    assert_eq!(
        baseline.0.len(),
        5,
        "baseline は 5 cue（ClearAll/Emote/hello/Wait/world）"
    );
    assert_eq!(baseline.1, TalkEndReason::Ended, "baseline は Ended");
    for run in 1..RUNS {
        let observed = run_once();
        assert_eq!(
            observed, baseline,
            "run {run} の観測が baseline と完全一致すること（cue 列・at・actor・順序・終端理由・R9.4）"
        );
    }
}

// ── task 10.1: 配送列統合檻（R9.7/R1.8/R8.6・責務二分） ──

/// **配送列統合檻（R9.7/R1.8/R8.6）**: 実 `parse`→`compile` の menu 台本を `CuePlayer` ＋記録
/// sink×**複数**で駆動し、broadcast 観測順が **compile 順**に一致すること（Choice が NewLine/
/// Cursor と**交互のまま**配送列に現れること・R1.8）と、同一 Choice が同時に**バッグ**
/// （`pending_choices`）へも積まれること（責務二分＝配送列は表示の単一真実源／バッグは解決照合の
/// 単一真実源・R8.6/R9.7）を、実 `compile → CuePlayer broadcast → sink` 経路上で固定する。
///
/// **task 2.2 との差**: 2.2 は dola `runtime_test.rs` で**手組みの CueSheet** を使う runtime 檻。
/// 本檻は **areka-sakura の実 `parse`＋実 `compile`** から得た CueSheet を出発点とし、
/// **複数**記録 sink へ broadcast させて配送列とバッグの並存を end-to-end で固定する統合檻である
/// （案C の「Choice 除外」廃止・配送列合流を実 compile 出力に対して立証する）。
///
/// 弁別: もし Choice が配送列から隠される（旧・先積み一択）なら配送列の等価 assert が FAIL し、
/// もしバッグへ積まれなければ `pending_choices` の assert が FAIL する（配送列とバッグの乖離を捕捉）。
#[test]
fn compile_broadcast_stream_preserves_order_and_choices_also_land_in_bag() {
    use dola::cue::{CuePlayer, PendingChoice};

    // menu.pasta:15 の raw さくらスクリプト断片（`\q \n \q \_l[5em,2lh] \q`・task 4.4 cage と同一）。
    // 実 parse → 実 compile を通す（手組み Instruction でなく、パーサ〜コンパイラの実経路を出発点にする）。
    let script = concat!(
        r"\q[おしゃべり頻度,Onおしゃべり頻度メニュー]",
        r"\n",
        r"\q[エモの位置調整,Onエモの位置調整メニュー]",
        r"\_l[5em,2lh]",
        r"\q[閉じる,Onメニュー閉じる]",
    );
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = compile(&instructions, &SystemVarSnapshot::default());

    // 実 compile 出力から CuePlayer を構築し、記録 sink を **2 本** broadcast 登録する
    // （どの sink も全 cue を受ける・登録順は配送内容に影響しない）。
    let mut player = CuePlayer::from_sheet(&compiled.sheet);
    let sink_a = RecordingSink::new();
    let sink_b = RecordingSink::new();
    let records_a = sink_a.records();
    let records_b = sink_b.records();
    player.register_sink(Box::new(sink_a));
    player.register_sink(Box::new(sink_b));

    // 全内容は瞬時（at=0）＋末尾に選択待ち barrier@0。単一 tick(0.0) で offset 0 群（内容 6 cue）を
    // 配送し barrier@0 到達 → WaitingForChoice（barrier 手前の cue は配送済み）。
    player.tick(0.0);
    assert_eq!(
        player.state(),
        &dola::cue::CuePlayerState::WaitingForChoice,
        "menu 台本は末尾 barrier で WaitingForChoice へ停止する（barrier 手前は配送済み）"
    );

    // 期待配送列（compile 順）: 冒頭 ClearAll 前置＋内容 5 件（Choice が NewLine/Cursor と交互）。
    // barrier は配送列に現れない（Barrier は presentation でなく sink へ配られない）。
    let expected_stream = vec![
        CueCommand::ClearAll,
        CueCommand::Choice {
            id: "Onおしゃべり頻度メニュー".into(),
            text: "おしゃべり頻度".into(),
            references: vec![],
        },
        CueCommand::NewLine { ratio: 1.0 },
        CueCommand::Choice {
            id: "Onエモの位置調整メニュー".into(),
            text: "エモの位置調整".into(),
            references: vec![],
        },
        CueCommand::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        CueCommand::Choice {
            id: "Onメニュー閉じる".into(),
            text: "閉じる".into(),
            references: vec![],
        },
    ];
    // 複数 sink が **同一の配送列を同一順序**で受ける（broadcast・Choice を隠さず交互のまま合流・R1.8）。
    assert_eq!(
        commands(&records_a),
        expected_stream,
        "sink A: 配送列が compile 順（Choice が NewLine/Cursor と交互のまま現れる・R1.8/R9.7）"
    );
    assert_eq!(
        commands(&records_b),
        expected_stream,
        "sink B: broadcast ゆえ両 sink が同一の配送列を受ける（中央振り分けなし）"
    );

    // 交互配置の直接固定（index 1/3/5 が Choice・2 が NewLine・4 が Cursor）。full-vector 等価に
    // 加えて「交互のまま」の意図を legible に残す（Choice が改行/カーソルに埋もれず順序保持）。
    let stream_a = commands(&records_a);
    assert!(
        matches!(stream_a[1], CueCommand::Choice { .. })
            && matches!(stream_a[2], CueCommand::NewLine { .. })
            && matches!(stream_a[3], CueCommand::Choice { .. })
            && matches!(stream_a[4], CueCommand::Cursor { .. })
            && matches!(stream_a[5], CueCommand::Choice { .. }),
        "Choice/NewLine/Choice/Cursor/Choice が交互のまま配送列に並ぶ（R1.8）"
    );

    // 責務二分（R8.6/R9.7）: **同一 3 Choice** がバッグ（解決照合の単一真実源）へも**同時に**積まれる。
    // バッグ内容は id/text で配送列の 3 Choice と一致する（配送列とバッグが乖離しない）。
    let expected_bag = vec![
        PendingChoice {
            id: "Onおしゃべり頻度メニュー".into(),
            text: "おしゃべり頻度".into(),
        },
        PendingChoice {
            id: "Onエモの位置調整メニュー".into(),
            text: "エモの位置調整".into(),
        },
        PendingChoice {
            id: "Onメニュー閉じる".into(),
            text: "閉じる".into(),
        },
    ];
    assert_eq!(
        player.pending_choices(),
        expected_bag.as_slice(),
        "同一 3 Choice がバッグへも同時に積まれる（責務二分＝配送列とバッグが並存・R8.6）"
    );

    // 配送列側の Choice を抽出し、バッグと (id, text) で完全一致することを固定する
    // （同一 Choice が配送列とバッグの**両路**に現れる＝責務二分の相互整合）。
    let stream_choices: Vec<(String, String)> = stream_a
        .iter()
        .filter_map(|cmd| match cmd {
            CueCommand::Choice { id, text, .. } => Some((id.clone(), text.clone())),
            _ => None,
        })
        .collect();
    let bag_choices: Vec<(String, String)> = player
        .pending_choices()
        .iter()
        .map(|c| (c.id.clone(), c.text.clone()))
        .collect();
    assert_eq!(
        stream_choices, bag_choices,
        "配送列に現れる 3 Choice とバッグの 3 Choice が同一（id/text・順序とも一致）"
    );
}

// ── task 10.3: 未知コマンド名の第一級縮退（統合檻・R8.2/R8.5/R9.3b） ──

/// **未知コマンド名の第一級縮退（統合檻・R8.2/R8.5/R9.3b）**: `\!` 名前空間の**未知・M1 未対応
/// コマンド名**（`\![raise,OnBoot]`／単独形 `\![vanish]`）を含む生 script を `spawn_talk` の actor
/// 境界（内部で parse→compile→CuePlayer broadcast を通す）へ投入し、次の 3 点を end-to-end で固定する:
///
///  - **R8.2（compile 卒業）**: 未知名 `\!` は compile の無音落ちでなく汎用コマンド cue（`Custom`
///    キャリア）として**台本に第一級で載る**。ゆえに broadcast された各記録 sink の配送列に
///    キャリア cue が現れる（2 名とも `raise`／`vanish` を受ける＝配送で消えない）。
///  - **R8.5/R5（良性スキップ）**: どの消費者も未知名キャリアに action しない——消費者は
///    自らのコマンド名リテラルで**名前自己選別**するため、未知名はどの消費者の名前にも一致しない。
///    記録 sink は全 cue を**記録**する（無音破棄でも異常終了でもない・honor は不変）。**複数** sink が
///    同一列を受けて talk が完走することがその証跡。
///  - **R9.3b/R2.6（第一級縮退＋語彙フリー）**: dola はコマンド名の語彙も名前写像 API も持たず、
///    キャリアの型レベル分類は `cue_target_of(Custom)=None` に一様に落ちる＝
///    「型レベルでは担当未定＝消費側が名前で自己選別する」。「1 名前=高々 1 消費者」の一意性は
///    結線層（areka）の消費者台帳が保証する（dola の名前権威表ではない）。本檻は統合経路上で
///    その帰結（未知名キャリアが第一級で配送され、型レベルでは担当未定＝自己選別へ委譲される）を確認する。
///
/// 弁別: もし compile が未知名を無音落ちさせるなら配送列にキャリアが現れず等価 assert が FAIL する。
#[test]
fn unknown_command_names_broadcast_and_benign_skip_then_talk_completes() {
    use dola::cue::cue_target_of;

    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(103);
    // 未知名 2 種（引数付き `raise` と単独形 `vanish`）＋テキストを挟み `\e` で終端。
    // parse: `\![raise,OnBoot]`→GenericCommand{"raise",["OnBoot"]}／`\![vanish]`→GenericCommand{"vanish",[]}。
    // compile: いずれも command_carrier(name, args)（Custom キャリア）へ卒業・無音落ちしない（R8.2）。
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\![raise,OnBoot]hello\![vanish]world\e".to_string(),
        talk_id,
    };
    // broadcast の第一級性を立証するため**複数**記録 sink を登録（両者が同一配送列を受ける）。
    let sink_a = RecordingSink::new();
    let sink_b = RecordingSink::new();
    let records_a = sink_a.records();
    let records_b = sink_b.records();

    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(sink_a, sink_b),
        SystemVarSnapshot::default(),
    );

    // 初回 Tick(0.0) でアンカー刻印。全内容は at=0 群（raise/hello）と at=0.25 群（vanish/world）。
    // 占有 horizon（world 再生完了＝0.25+0.25=0.50）を跨ぐ Tick(1.0) で自然終端する。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

    // R8.2 の帰結: 未知名キャリアが無音落ちせず talk が完走し TalkDone{Ended} を返す。
    let done = recv_done(&done_rx, Duration::from_secs(5)).expect(
        "未知名キャリアを含む talk も良性スキップして完了すべき（無音落ち／panic しない）",
    );
    assert_eq!(done.talk_id, talk_id, "talk_id エコー（R1.3）");
    assert_eq!(
        done.reason,
        TalkEndReason::Ended,
        "`\\e` 終端＝Ended（未知名は終端理由に影響しない）"
    );
    handle
        .actor
        .join()
        .expect("未知名キャリアでも body は panic せず正常終了する（良性スキップ）");

    // 期待 broadcast 列（compile 順・冒頭 ClearAll 前置）: ClearAll / raise / hello / vanish / world。
    // 未知名キャリアが**配送列に第一級で現れる**（compile が卒業させた証・R8.2）。
    let expected = vec![
        CueCommand::ClearAll,
        CueCommand::command_carrier("raise", vec!["OnBoot".into()]),
        CueCommand::Text("hello".into()),
        CueCommand::command_carrier("vanish", vec![]),
        CueCommand::Text("world".into()),
    ];
    assert_eq!(
        commands(&records_a),
        expected,
        "sink A: 未知名キャリア（raise/vanish）が配送列に第一級で現れる（無音落ちしない・R8.2）"
    );
    // broadcast の第一級性: 2 つ目の sink も同一列を受ける（未知名も両者へ届く＝配送で消えない・R5）。
    assert_eq!(
        commands(&records_b),
        expected,
        "sink B: broadcast ゆえ両 sink が同一配送列を受ける（未知名キャリアも欠落しない）"
    );

    // R8.5/R9.3b（良性スキップ＋自己選別への委譲）: 配送列中の各未知名キャリアについて、
    // 型レベル分類 `cue_target_of(Custom)=None`＝「型レベルでは担当未定＝消費側が名前で自己選別
    // する」へ一様に落ちる（dola はコマンド名の語彙を持たない・R2.6）。未知名はどの消費者の
    // 名前リテラルにも一致しないため、どの消費者も action しない良性スキップとなる。
    // キャリア variant からのコマンド名抽出（`as_command_carrier`）を通し、抽出できた名前で確認する。
    let carrier_names: Vec<String> = commands(&records_a)
        .iter()
        .filter_map(|cmd| cmd.as_command_carrier().map(|(name, _)| name.to_string()))
        .collect();
    assert_eq!(
        carrier_names,
        vec!["raise".to_string(), "vanish".to_string()],
        "配送列から未知名キャリア 2 件（raise/vanish）が抽出される"
    );
    for cmd in commands(&records_a).iter() {
        if cmd.as_command_carrier().is_some() {
            assert_eq!(
                cue_target_of(cmd),
                None,
                "キャリア cue の型レベル分類は一様に None＝消費側の名前自己選別へ委譲（R8.5/R9.3b・dola は名前語彙を持たない）"
            );
        }
    }

    // 自己選別モデル（R2.6）の統合経路上の焦点確認: dola はコマンド名で分岐しない——`move`
    // キャリアであっても型レベル分類は None（dola は名前写像 API を持たず、名前→担当の
    // 中央権威表は無い）。「1 名前=高々 1 消費者」の一意性は結線層（areka）の消費者台帳が保証する。
    assert_eq!(
        cue_target_of(&CueCommand::command_carrier("move", vec![])),
        None,
        "move キャリアでも型レベル分類は None（dola は名前語彙を持たず消費側が自己選別する）"
    );
}
