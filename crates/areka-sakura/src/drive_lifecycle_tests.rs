use super::*;
use crate::contract::{CueCommand, TalkId};
use crate::duration::text_playback_duration;
use std::sync::mpsc;
use std::time::Duration;
use super::test_support::*;

/// 空発火列（空 script）の talk は時間軸駆動せず、Tick を一切送らなくても
/// コンパイル結果の終端理由（空 script＝`Ended`）を伴う `TalkDone` を**即座に**返す
/// （observable・R1.4）。`talk_id` は起動要求のものがエコーされる（R1.3）。
#[test]
fn empty_script_talk_returns_talkdone_immediately_without_tick() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(7);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: String::new(), // 空 script → 空 Instruction 列 → 空 sheet。
        talk_id,
    };

    let sink = RecordingSink::new();
    let records = sink.records();

    // Tick を一切送らずに spawn_talk を呼ぶ（時間軸駆動を要求しない）。
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(sink, NoopSink),
        SystemVarSnapshot::default(),
    );

    // TalkDone が即座に到達すること（Tick 不要・時間軸駆動なし）。
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("空 script の talk は即座に TalkDone を返すべき");

    assert_eq!(done.talk_id, talk_id, "talk_id がエコーされること");
    assert_eq!(done.reason, TalkEndReason::Ended, "空 script は Ended");

    assert!(
        records.lock().unwrap().is_empty(),
        "空 sheet では発火が無いこと"
    );
    handle.actor.join().expect("body は正常終了する");
}

/// **`Start` の二重受領が無視される**ことを検証する（プロトコルガード・`on_start`）。
/// 1 本目（script A）で spawn 後、別 script の 2 本目 `Start`(B) を送っても A のみ再生される。
#[test]
fn duplicate_start_is_ignored_and_first_talk_plays_unchanged() {
    let (done_a_tx, done_a_rx) = mpsc::channel::<TalkNotice>();
    let id_a = TalkId(11);
    let start_a = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[2]world\e".to_string(),
        talk_id: id_a,
    };
    let sink = RecordingSink::new();
    let records = sink.records();
    let handle = spawn_talk(
        start_a,
        done_a_tx,
        two_sinks(sink, NoopSink),
        SystemVarSnapshot::default(),
    );

    // 2 本目 Start(B)（別 script）を inbox へ。自己投函の Start(A) の後に処理され、無視される。
    let id_b = TalkId(99);
    let start_b = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[77]DIFFERENT\e".to_string(),
        talk_id: id_b,
    };
    handle
        .inbox
        .send(SakuraMsg::Start(start_b))
        .expect("2 本目 Start(B) 投函");

    // A を駆動して自然終端（world 再生完了 horizon=0.60 を跨ぐ Tick(1.0) まで）。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
    let done = recv_done(&done_a_rx, Duration::from_secs(5))
        .expect("A の TalkDone");
    assert_eq!(
        done.talk_id, id_a,
        "終端は A の talk_id（B に乗っ取られない）"
    );
    assert_eq!(done.reason, TalkEndReason::Ended);
    handle.actor.join().expect("body は正常終了する");

    // A の内容のみ（B の Emote{77}/DIFFERENT は現れない）: ClearAll/Emote{10}/hello/Wait/world。
    assert_eq!(
        commands(&records),
        vec![
            CueCommand::ClearAll,
            CueCommand::Emote { key: "10".into() },
            CueCommand::Text("hello".into()),
            CueCommand::Wait,
            CueCommand::Text("world".into()),
        ],
        "A の内容のみが broadcast される（B の DIFFERENT/Emote{{77}} は不在）"
    );
}

/// **終端時に done 受信端が drop 済みでも body が panic せず clean exit する**（R11.1/11.4）。
/// 駆動前に `done_rx` を drop → 自然終端で `done.send` が `Err` になるが `error!` の上で `Break`。
/// 発火自体は正常（done drop は終端信号にのみ影響し broadcast には影響しない）。
#[test]
fn dropped_done_receiver_at_terminal_exits_cleanly_without_panic() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[2]world\e".to_string(),
        talk_id: TalkId(4),
    };
    let sink = RecordingSink::new();
    let records = sink.records();
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(sink, NoopSink),
        SystemVarSnapshot::default(),
    );

    drop(done_rx); // 終端 TalkDone 送出前に受信端を drop（送出は Err になる）。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

    handle
        .actor
        .join()
        .expect("done 受信端 drop でも body は panic せず正常終了する");

    // broadcast は正常に行われた（ClearAll/Emote/hello/Wait/world の 5 件）。
    assert_eq!(
        records.lock().unwrap().len(),
        5,
        "done drop は broadcast に影響しない（5 cue 配送済み）"
    );
}

/// M-boot 外タグのみで発火列が空になる script は空 sheet へコンパイルされ、Tick を要さずに
/// 末尾到達の `Ended`（R1.4）を伴う `TalkDone` を即座に返す（リテラル空 script とは別経路）。
#[test]
fn ignored_tags_only_script_ends_immediately_with_ended_and_no_firing() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(55);
    // task 4.2 で SystemVar/GenericCommand は cue を発行するようになったため、無 cue フィラーには
    // `\0` を用いる。parser は `\0` を正典スコープタグ `SpeakerScope{n:0}` へ写像するが（task 12.1・
    // R1.5/R4.4）、compile は `SpeakerScope{n}` を「scope 状態更新のみ・cue 非発行」で扱う
    // （`compile.rs` の SpeakerScope アーム）。ゆえに内容 cue は皆無で empty-sheet 即時 TalkDone
    // 経路を保つ（`\0` の写像先が Raw から SpeakerScope へ変わっても本檻の観測は不変）。
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\0".to_string(),
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
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("無視タグのみの script も空 sheet 経路で即座に TalkDone を返すべき");

    assert_eq!(done.talk_id, talk_id, "talk_id エコー（R1.3）");
    assert_eq!(
        done.reason,
        TalkEndReason::Ended,
        "終端命令のない無視タグのみ script は末尾到達で Ended（R1.4）"
    );
    assert!(
        records.lock().unwrap().is_empty(),
        "無視タグは発火を生成しない"
    );
    handle.actor.join().expect("body は正常終了する");
}

/// 先行 cue のない `\-`（quit 相当のみ）の script は空 sheet＋`end=Quit` へコンパイルされ、Tick を
/// 要さずに **`Quit`（`Ended` ではない）** を伴う `TalkDone` を即座に返す（空 sheet 経路の弁別・R6.2）。
#[test]
fn quit_only_script_ends_immediately_with_quit_not_ended() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(56);
    // task 4.2 で SystemVar は cue を発行するようになったため、`\-` の先行フィラーには `\0` を用いる。
    // parser は `\0` を `SpeakerScope{n:0}` へ写像し（task 12.1・R1.5/R4.4）、compile はそれを
    // scope 状態更新のみ（cue 非発行）で扱う。先行内容 cue のない `\-` の empty-sheet＋Quit 経路を
    // 保つ（SpeakerScope は cue を生まず `\-` が Quit で切詰め＝空 sheet＋end=Quit）。
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\0\-".to_string(),
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
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("quit 相当のみの script も空 sheet 経路で即座に TalkDone を返すべき");

    assert_eq!(done.talk_id, talk_id, "talk_id エコー（R1.3）");
    assert_eq!(
        done.reason,
        TalkEndReason::Quit,
        "先行 cue のない `\\-` は空 sheet＋Quit（Ended を固定送出してはならない・R6.2）"
    );
    assert_ne!(
        done.reason,
        TalkEndReason::Ended,
        "空 sheet 経路で Ended を固定送出していない"
    );
    assert!(
        records.lock().unwrap().is_empty(),
        "quit 相当のみでは発火が無い"
    );
    handle.actor.join().expect("body は正常終了する");
}

/// 再生途中の中断（Close）で `TalkDone{Interrupted}` がちょうど 1 回返り、未発火分が sink に
/// 届かないこと（R7.1/7.2/7.3/7.4・R6.4）。`\s[10]hello\w[10]world\e`（world は \w[10] 後）を
/// 先頭群だけ発火させたところで Close。world（at=0.75）は未発火＝以降届いてはならない。
#[test]
fn mid_playback_close_returns_interrupted_once_and_drops_unfired_cues() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(101);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[10]world\e".to_string(),
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

    // 初回 Tick(0.0) 刻印＋at=0.0 群を発火（world は at=0.75・未達）。
    handle
        .inbox
        .send(SakuraMsg::Tick(0.0))
        .expect("Tick(0.0) 投函");
    // 中断（Close）を送る。進行中の再生を即時停止し Interrupted ACK を返すべき。
    handle.inbox.send(SakuraMsg::Close).expect("Close 投函");

    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("中断で TalkDone{Interrupted} が返るべき");
    assert_eq!(done.talk_id, talk_id, "talk_id エコー（R6.6）");
    assert_eq!(
        done.reason,
        TalkEndReason::Interrupted,
        "中断の終端理由は Interrupted（R7.4）"
    );
    handle.actor.join().expect("body は Break 後に正常終了する");

    // at=0.0 群のみ届き（ClearAll/Emote/hello）、未発火分（world@0.75）は届いていない（R7.2）。
    assert_eq!(
        commands(&records),
        vec![
            CueCommand::ClearAll,
            CueCommand::Emote { key: "10".into() },
            CueCommand::Text("hello".into()),
        ],
        "中断前に届いたのは ClearAll/Emote/hello のみ（world は未発火＝破棄・R7.2）"
    );
}

/// 自然終端後に中断（Close）を受けても追加の `TalkDone` が発生しないこと（R6.4/R7.5）。
/// 自然終端後はアクタースレッドが消えており `inbox.send(Close)` が `Err`＝二重終端不能の構造的証。
#[test]
fn close_after_natural_end_produces_no_extra_talkdone() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(102);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[2]world\e".to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(NoopSink, NoopSink),
        SystemVarSnapshot::default(),
    );

    // 自然終端まで駆動する（0.0 刻印 → 占有 horizon=0.60 を跨ぐ 1.0）。
    handle
        .inbox
        .send(SakuraMsg::Tick(0.0))
        .expect("Tick(0.0) 投函");
    handle
        .inbox
        .send(SakuraMsg::Tick(1.0))
        .expect("Tick(1.0) 投函");

    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("自然終端で TalkDone{Ended} が返るべき");
    assert_eq!(done.talk_id, talk_id, "talk_id エコー");
    assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
    handle
        .actor
        .join()
        .expect("body は自然終端後に正常終了する");

    // 自然終端後の Close: アクターは既に消えており inbox.send は Err（二重終端不能の証）。
    let send_result = handle.inbox.send(SakuraMsg::Close);
    assert!(
        send_result.is_err(),
        "自然終端後はアクターが消えており Close 送出は失敗する（二重終端不能の証）"
    );
}

/// 複数 talk を異なる相関 ID・独立 sink で同時駆動し、各 `TalkDone` に起動時と同一の `talk_id`
/// が対応付けられ、出力が talk 間で混線しないことを確認する（R1.3/R6.6）。
#[test]
fn multiple_talks_echo_own_talk_id_without_cross_talk_mixing() {
    // talk A: TalkId(7)・Ended 経路。
    let (done_a_tx, done_a_rx) = mpsc::channel::<TalkNotice>();
    let id_a = TalkId(7);
    let start_a = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[2]world\e".to_string(),
        talk_id: id_a,
    };
    let sink_a = RecordingSink::new();
    let records_a = sink_a.records();

    // talk B: TalkId(42)・Quit 経路（末尾 `\-`）。
    let (done_b_tx, done_b_rx) = mpsc::channel::<TalkNotice>();
    let id_b = TalkId(42);
    let start_b = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[20]bye\w[2]done\-".to_string(),
        talk_id: id_b,
    };
    let sink_b = RecordingSink::new();
    let records_b = sink_b.records();

    let handle_a = spawn_talk(
        start_a,
        done_a_tx,
        two_sinks(sink_a, NoopSink),
        SystemVarSnapshot::default(),
    );
    let handle_b = spawn_talk(
        start_b,
        done_b_tx,
        two_sinks(sink_b, NoopSink),
        SystemVarSnapshot::default(),
    );

    handle_a
        .inbox
        .send(SakuraMsg::Tick(0.0))
        .expect("A Tick(0.0)");
    handle_a
        .inbox
        .send(SakuraMsg::Tick(1.0))
        .expect("A Tick(1.0)");
    handle_b
        .inbox
        .send(SakuraMsg::Tick(0.0))
        .expect("B Tick(0.0)");
    handle_b
        .inbox
        .send(SakuraMsg::Tick(1.0))
        .expect("B Tick(1.0)");

    let done_a = recv_done(&done_a_rx, Duration::from_secs(5))
        .expect("talk A は TalkDone を返すべき");
    let done_b = recv_done(&done_b_rx, Duration::from_secs(5))
        .expect("talk B は TalkDone を返すべき");

    assert_eq!(done_a.talk_id, id_a, "talk A の TalkDone は id_a をエコー");
    assert_eq!(done_b.talk_id, id_b, "talk B の TalkDone は id_b をエコー");
    assert_ne!(done_a.talk_id, done_b.talk_id, "2 talk の id は相異なる");
    assert_eq!(done_a.reason, TalkEndReason::Ended, "A の `\\e` は Ended");
    assert_eq!(done_b.reason, TalkEndReason::Quit, "B の `\\-` は Quit");

    handle_a.actor.join().expect("A body 正常終了");
    handle_b.actor.join().expect("B body 正常終了");

    // 各 talk の cue は自分の sink にのみ届く（混線しない）。
    assert_eq!(
        commands(&records_a),
        vec![
            CueCommand::ClearAll,
            CueCommand::Emote { key: "10".into() },
            CueCommand::Text("hello".into()),
            CueCommand::Wait,
            CueCommand::Text("world".into()),
        ],
        "A sink は A の cue 列のみ"
    );
    assert_eq!(
        commands(&records_b),
        vec![
            CueCommand::ClearAll,
            CueCommand::Emote { key: "20".into() },
            CueCommand::Text("bye".into()),
            CueCommand::Wait,
            CueCommand::Text("done".into()),
        ],
        "B sink は B の cue 列のみ"
    );
}

/// **末尾に明示的な待ちを持つ talk**: 完了通知は cue 配送完了（entry 枯渇）でなく、末尾 Wait の
/// 再生時間を含む占有 horizon 到達で初めて発火する（R2.5/D6・#3 の実機構）。
///
/// `\s[10]hello\_w[800]\e` の台本（アンカー 0）:
///   ClearAll@0 / Emote{10}@0 / hello@0(dur=D) / Wait@D(dur=0.8)。
/// 全 cue の配送は Tick(D) で完了する（entry 枯渇）が、占有 horizon＝`D + 0.8` であり、そこへ達する
/// まで `TalkDone` は発火しない。末尾待ちの 0.8 秒が talk 終端で切り捨てられない（早期終了しない）。
#[test]
fn trailing_wait_talkdone_fires_at_horizon_not_at_cue_exhaustion() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(720);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\_w[800]\e".to_string(),
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

    // 期待値は本番と同一の算術で導出（10 進直書きの表現誤差を排除・注入時刻決定論）。
    let d_hello = text_playback_duration("hello"); // 0.25
    let w = Duration::from_millis(800).as_secs_f64(); // 0.8（\_w[800]）
    let t_wait = d_hello; // Wait cue の相対発火時刻
    let horizon = d_hello + w; // 占有 horizon＝末尾 Wait の再生完了時刻（1.05）
    let near_horizon = d_hello + w * 0.5; // horizon 手前（entry 枯渇後・horizon 未満）

    // 初回 Tick(0.0) 刻印。Tick(D) で Wait を配送し **entry を枯渇**、さらに horizon 手前まで前進する
    // （いずれも horizon 未満・単調増加 0.0 < 0.25 < 0.65 < 1.05）。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(t_wait)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(near_horizon)).unwrap();

    // 負の窓: entry 枯渇かつ horizon 手前では完了通知が **発火しない**（配送 ≠ 再生完了・早期終了しない）。
    assert!(
        recv_done(&done_rx, NEG_WINDOW).is_err(),
        "全 cue 配送済み（entry 枯渇）かつ horizon 未満では TalkDone は発火してはならない（配送 ≠ 完了・R2.5）"
    );
    // 窓明けの race なし観測: 全 cue は既に broadcast 配送済み（配送完了）だが完了はしていない。
    assert_eq!(
        commands(&records),
        vec![
            CueCommand::ClearAll,
            CueCommand::Emote { key: "10".into() },
            CueCommand::Text("hello".into()),
            CueCommand::Wait,
        ],
        "末尾 Wait まで含め全 cue が配送済み（占有 horizon 未達でも配送は完了している）"
    );
    assert!(
        !handle.actor.is_finished(),
        "配送完了後も horizon 未達ゆえ talk は駆動継続（早期終了せず TalkDone 未送出）"
    );

    // horizon 到達で初めて完了する（末尾 Wait の 0.8 秒が終端で切り捨てられない）。
    handle.inbox.send(SakuraMsg::Tick(horizon)).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("占有 horizon 到達で TalkDone が発火するべき");
    assert_eq!(done.talk_id, talk_id, "talk_id エコー");
    assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
    handle.actor.join().expect("body は正常終了する");
}

/// **待ちを持たない末尾テキストのみの talk**: 完了通知は最終テキストの **配送時刻**（発火 start）でなく、
/// その再生時間 D を含む絶対終了時刻（start + D）到達で発火する（R2.5/D6）。
///
/// `\s[10]hello\_w[500]world\e` の台本（アンカー 0）:
///   ClearAll@0 / Emote{10}@0 / hello@0(dur=D_h) / Wait@D_h(dur=0.5) / world@(D_h+0.5)(dur=D_w)。
/// 末尾 cue は Text(world)。world は Tick(D_h+0.5) で配送されるが（entry 枯渇）、占有 horizon＝
/// `(D_h+0.5) + D_w` であり、world の **再生時間 D_w** が終端で落とされずそこまで完了は遅れる。
#[test]
fn trailing_final_text_talkdone_fires_after_text_duration_not_at_delivery() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(721);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\_w[500]world\e".to_string(),
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

    let d_hello = text_playback_duration("hello"); // 0.25
    let w = Duration::from_millis(500).as_secs_f64(); // 0.5（\_w[500]）
    let d_world = text_playback_duration("world"); // 0.25
    let t_world = d_hello + w; // 末尾テキスト world の配送時刻（0.75）
    let horizon = t_world + d_world; // world の再生完了時刻＝占有 horizon（1.0）

    // 初回 Tick(0.0) 刻印 → Tick(D_h) で Wait 配送 → Tick(t_world) で末尾 world を配送し entry 枯渇。
    // t_world は末尾テキストの **発火時刻** であって完了時刻ではない（単調 0.0 < 0.25 < 0.75）。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(d_hello)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(t_world)).unwrap();

    // 負の窓: 末尾テキストは配送済み（発火 start 到達）だが、その再生時間 D_w ぶん完了は遅れる。
    assert!(
        recv_done(&done_rx, NEG_WINDOW).is_err(),
        "末尾テキストの配送時刻（発火 start）では TalkDone は発火してはならない（再生時間を終端で落とさない・R2.5）"
    );
    assert_eq!(
        commands(&records),
        vec![
            CueCommand::ClearAll,
            CueCommand::Emote { key: "10".into() },
            CueCommand::Text("hello".into()),
            CueCommand::Wait,
            CueCommand::Text("world".into()),
        ],
        "末尾テキスト world まで全 cue 配送済み（発火はしたが再生は未完了）"
    );
    assert!(
        !handle.actor.is_finished(),
        "末尾テキスト配送後も start+D 未達ゆえ駆動継続（配送 ≠ 再生完了）"
    );

    // start + D（世界の再生完了＝占有 horizon）到達で初めて完了する。
    handle.inbox.send(SakuraMsg::Tick(horizon)).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("末尾テキストの再生完了（start+D）で TalkDone が発火するべき");
    assert_eq!(done.talk_id, talk_id, "talk_id エコー");
    assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
    handle.actor.join().expect("body は正常終了する");
}

/// **tick 源の liveness 契約**: horizon 未満で tick が止まると `TalkDone` は発火せず、horizon まで
/// tick を送り続けると発火する（task 7.2 申し送り＝「tick 源は entries 枯渇後も horizon 到達まで
/// tick を送り続ける」）。本 spec は本番 tick 源を変えず、drive は `is_completed()` 成立で発火する。
///
/// `\s[10]ab\_w[600]\e`（ab=2char→D=0.1・Wait 0.6・horizon=0.7）で、entry 枯渇（0.1）でも、その先の
/// horizon 手前（0.5）でも、tick を止めれば完了通知は保留され、horizon（0.7）到達で初めて発火する。
#[test]
fn talkdone_withheld_while_ticks_stop_below_horizon_then_fires_on_resume() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(722);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]ab\_w[600]\e".to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(NoopSink, NoopSink),
        SystemVarSnapshot::default(),
    );

    let d_ab = text_playback_duration("ab"); // 0.1
    let w = Duration::from_millis(600).as_secs_f64(); // 0.6
    let horizon = d_ab + w; // 0.7

    // 初回 Tick(0.0) 刻印 → Tick(D) で Wait 配送＝entry 枯渇。ここで tick を **止める**。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(d_ab)).unwrap();
    // tick 停止中（entry 枯渇・horizon 未満）は完了通知が発火しない。
    assert!(
        recv_done(&done_rx, NEG_WINDOW).is_err(),
        "tick が horizon 未満で止まると TalkDone は発火しない（entry 枯渇 ≠ 完了・R2.5）"
    );
    assert!(!handle.actor.is_finished(), "駆動継続（未完了）");

    // tick を再開するが依然 horizon 手前（0.5 < 0.7）。まだ発火しない。
    handle.inbox.send(SakuraMsg::Tick(0.5)).unwrap();
    assert!(
        recv_done(&done_rx, NEG_WINDOW).is_err(),
        "horizon 手前まで進めても未達なら TalkDone は発火しない"
    );
    assert!(!handle.actor.is_finished(), "horizon 手前ゆえ依然駆動継続");

    // horizon まで tick を送り切ると初めて発火する（liveness 契約の正の側）。
    handle.inbox.send(SakuraMsg::Tick(horizon)).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("horizon まで tick を送り続けると TalkDone が発火するべき");
    assert_eq!(done.talk_id, talk_id, "talk_id エコー");
    assert_eq!(done.reason, TalkEndReason::Ended, "`\\e` は Ended");
    handle.actor.join().expect("body は正常終了する");
}

/// **終端理由と絶対終了時刻の型的別概念（D6・R2.5）**: `TalkDone.reason` は compile 時に確定する
/// 終端理由 `TalkEndReason`（`Ended`/`Quit`＝時間量でない）に等しく、一方 **発火の時刻** は台本由来の
/// 占有 horizon（`absolute_end_time`）で決まる——この 2 つは互いに独立した事実である。
///
/// `\s[10]hi\_w[700]\-`（末尾 `\-`→Quit・末尾に Wait 0.7）で、(1) `done.reason` が compile の
/// `TalkEndReason::Quit` に一致し（`Ended` の反例で時間由来でないことを示す）、(2) その発火は
/// `compiled.sheet.absolute_end_time()` 由来の horizon 到達まで遅れる（entry 枯渇では発火しない）ことを固定する。
#[test]
fn talkdone_reason_is_compiled_end_while_firing_time_is_horizon_derived() {
    let script = r"\s[10]hi\_w[700]\-";

    // FACT 1（終端理由）: reason は compile 時に確定する TalkEndReason（時間量でない enum）。
    let compiled = compile(
        &areka_parsers::sakura::parse(script),
        &crate::sysvar::SystemVarSnapshot::default(),
    );
    assert_eq!(
        compiled.end,
        TalkEndReason::Quit,
        "末尾 `\\-` の終端理由は Quit（時刻でなく理由）"
    );
    // FACT 2（終了時刻）: 発火時刻の権威は台本由来の占有 horizon（アンカー未刻印＝0 起点で導出）。
    let horizon = compiled.sheet.absolute_end_time(); // 0.1(hi) + 0.7(\_w[700]) = 0.8

    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(723);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: script.to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(NoopSink, NoopSink),
        SystemVarSnapshot::default(),
    );

    let d_hi = text_playback_duration("hi"); // 0.1（末尾 Wait の発火時刻＝entry 枯渇点）

    // 初回 Tick(0.0) 刻印 → Tick(D) で末尾 Wait 配送＝entry 枯渇（horizon=0.8 の遥か手前）。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(d_hi)).unwrap();
    // 発火時刻が horizon 由来である証: entry 枯渇では発火しない（reason が確定していても時刻は別権威）。
    assert!(
        recv_done(&done_rx, NEG_WINDOW).is_err(),
        "終端理由が確定していても発火は entry 枯渇でなく horizon 到達に従う（時刻は別権威・D6）"
    );
    assert!(!handle.actor.is_finished(), "horizon 未達ゆえ駆動継続");

    // 台本由来の horizon 到達で発火。reason は compile 由来（Quit）で、firing time は horizon 由来。
    handle.inbox.send(SakuraMsg::Tick(horizon)).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("台本由来の horizon 到達で TalkDone が発火するべき");
    assert_eq!(done.talk_id, talk_id, "talk_id エコー");
    assert_eq!(
        done.reason, compiled.end,
        "reason は compile が確定した TalkEndReason（Quit）に等しい（時間量でない）"
    );
    assert_eq!(
        done.reason,
        TalkEndReason::Quit,
        "末尾 `\\-` は Quit（Ended でない＝reason は時刻でなく理由由来）"
    );
    handle.actor.join().expect("body は正常終了する");
}
