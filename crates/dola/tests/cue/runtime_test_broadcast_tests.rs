//! broadcast 配送・占有終了検知（`is_completed`）・中断（`stop`）・sink 登録の檻。

use super::{ActorKey, BarrierKind, Cue, CueCommand, CuePlayer, CuePlayerState, CueSheet, TalkCue};
use crate::test_support::{barrier, logged_commands, recording_sink, text, wait};
use std::rc::Rc;

/// **観測可能な完了条件（Task 4.3・前段）**: 1 つの台本を複数の登録済み sink へ流すと、
/// 全 sink が**同一の cue 列を同一絶対時刻で**受信する（broadcast fan-out・R2.1/R1.4）。
///
/// `TalkCue.at` は相対 offset（canonical 変換が `cue.start_time` を無変形複写）、絶対発火時刻は
/// アンカー `absolute_start_time` ＋ `at`。全 sink が同一 `at` を受け、アンカーが共有ゆえ、
/// 各 sink が独立に算出する絶対時刻も同一になる（配送時に別時刻を導出しない・R1.3）。
#[test]
fn broadcast_delivers_identical_stream_to_every_sink_at_same_absolute_time() {
    let actor = ActorKey::from("0");
    // 絶対開始時刻 100.0 を刻印した自己完結台本（アンカーが配送駆動に効くことを具体化する）。
    let sheet = CueSheet::new(vec![
        Cue {
            actor: actor.clone(),
            start_time: 0.0,
            payload: CueCommand::Emote {
                key: "smile".into(),
            }
            .into(),
            duration: 0.5,
        },
        text(0.5, "hello", 1.0),
        wait(1.5, 0.25),
    ])
    .with_absolute_start_time(100.0);

    let mut player = CuePlayer::from_sheet(&sheet);
    let (log_a, sink_a) = recording_sink();
    let (log_b, sink_b) = recording_sink();
    let (log_c, sink_c) = recording_sink();
    player.register_sink(sink_a);
    player.register_sink(sink_b);
    player.register_sink(sink_c);

    // 絶対時刻（アンカー 100.0 ＋ 相対 offset）で駆動する。
    player.tick(100.0); // Emote@rel0.0
    player.tick(100.5); // Text@rel0.5
    player.tick(101.5); // Wait@rel1.5

    let a = log_a.borrow().clone();
    let b = log_b.borrow().clone();
    let c = log_c.borrow().clone();

    // 全 sink は同一の cue 列を受信する（broadcast＝全員が全 cue を受ける）。
    assert_eq!(a, b, "sink A と B は同一 cue 列を受信する");
    assert_eq!(b, c, "sink B と C は同一 cue 列を受信する");

    // 受信列は台本記述順・相対 `at`・`duration` ともに無変形。担当 action を持たない Wait も、
    // Balloon 担当の Text も、Shell 担当の Emote も、選別なく全 sink へ broadcast される。
    let expected = vec![
        TalkCue {
            at: 0.0,
            actor: actor.clone(),
            command: CueCommand::Emote {
                key: "smile".into(),
            },
            duration: 0.5,
        },
        TalkCue {
            at: 0.5,
            actor: actor.clone(),
            command: CueCommand::Text("hello".into()),
            duration: 1.0,
        },
        TalkCue {
            at: 1.5,
            actor: actor.clone(),
            command: CueCommand::Wait,
            duration: 0.25,
        },
    ];
    assert_eq!(
        a, expected,
        "各 sink は at/duration を無変形で受け取る（Wait/Emote も broadcast＝duration honor の対象）"
    );
}

/// broadcast はバリア手前の cue のみを sink へ配り、バリア以降は外部解決まで配送しない
/// （broadcast をバリア seam と結線）。解決後は残り cue が配送され、各 cue は生涯 1 回のみ
/// （待機中・解決跨ぎで before を二重配送しない）。
#[test]
fn broadcast_stops_at_barrier_and_resumes_delivery_after_resolve() {
    let sheet = CueSheet::new(vec![
        text(0.0, "before", 0.0),
        barrier(0.1, BarrierKind::WaitForInput),
        text(0.2, "after", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(10.0);
    assert_eq!(player.state(), &CuePlayerState::WaitingForInput);
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("before".into())],
        "バリア手前の before のみ sink へ配送される"
    );

    // 待機中の tick では after は配送されない（バリアが保持される）。
    player.tick(20.0);
    assert_eq!(log.borrow().len(), 1, "待機中は after を配送しない");

    // 外部解決で再開し、残り cue が配送される。
    player.resolve_click();
    player.tick(20.0);
    assert_eq!(
        logged_commands(&log),
        vec![
            CueCommand::Text("before".into()),
            CueCommand::Text("after".into()),
        ],
        "解決後に after が配送される（before の二重配送はない・各 cue は 1 回のみ）"
    );
}

/// broadcast は各 cue を**生涯 1 回のみ**配送する。同一時刻の再 tick（冪等）でも再配送しない
/// （配送は schedule 前進時のみ）。
#[test]
fn broadcast_delivers_each_cue_once_even_when_ticked_repeatedly() {
    let sheet = CueSheet::new(vec![text(0.0, "a", 0.0), text(0.5, "b", 0.0)]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(0.0);
    player.tick(0.0); // 同一時刻の再 tick は再配送しない（冪等）。
    assert_eq!(
        log.borrow().len(),
        1,
        "同一時刻の再 tick で a を二重配送しない"
    );

    player.tick(0.5);
    player.tick(0.5);
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("a".into()), CueCommand::Text("b".into())],
        "各 cue は生涯 1 回のみ配送される（冪等再 tick で二重配送しない）"
    );
}

/// **観測可能な完了条件（Task 4.3・後段）**: 末尾に待ちを持つ台本では、全 cue の**配送完了**
/// （entry 枯渇）時点では `is_completed()` が false で、占有 horizon（最終 Wait の duration 端）
/// 到達で初めて true になる（配送 ≠ 再生完了・早期終了しない・R2.5/D6 の caller-facing 固定）。
#[test]
fn is_completed_is_gated_by_occupancy_horizon_for_trailing_wait() {
    let d = 0.15_f64;
    let wait_dur = 0.8_f64;
    let sheet = CueSheet::new(vec![text(0.0, "bye", d), wait(d, wait_dur)]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    // 全 entry を配り終える時刻（末尾 Wait の発火時刻＝d）まで進める。
    player.tick(d);
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("bye".into()), CueCommand::Wait],
        "bye と末尾 Wait が sink へ配送済み"
    );
    assert!(
        !player.is_completed(),
        "配送完了（entry 枯渇）時点では占有 horizon 未到達ゆえ未完了（早期終了しない・R2.5）"
    );

    // 占有 horizon 直前も未完了。
    player.tick(sheet.absolute_end_time() - 0.01);
    assert!(!player.is_completed(), "horizon 直前は未完了");

    // 占有 horizon（150 ms＋800 ms）到達で初めて完了通知が出る。Tick の式 `n as f64 / 1000.0` の値で見る。
    player.tick(950.0 / 1000.0);
    assert!(
        player.is_completed(),
        "占有 horizon（末尾 Wait の duration 端）到達で初めて is_completed() が true"
    );
}

/// 待ちを持たない末尾テキストのみの台本でも、テキストの再生時間（duration）端＝占有 horizon
/// 到達まで `is_completed()` は false（最終 Text の duration を終端で落とさない・R2.5）。
#[test]
fn is_completed_is_gated_by_final_text_duration() {
    let d = 0.5_f64;
    let sheet = CueSheet::new(vec![text(0.0, "bye", d)]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    // 最終 Text の配送時刻（0.0）到達 — 配り終えたが再生（duration）は未完。
    player.tick(0.0);
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("bye".into())],
        "全 entry 配送済み"
    );
    assert!(
        !player.is_completed(),
        "最終 Text の配送時点では duration 端未到達ゆえ未完了"
    );

    // duration 端（占有 horizon）到達で完了。
    player.tick(sheet.absolute_end_time());
    assert!(
        player.is_completed(),
        "最終 Text の duration 端到達で is_completed() が true"
    );
}

/// 中断は上流が player を捨てる形（本番の `drop(player)` と同じ）。捨てた後は残りの cue
/// （b）が受け手へ届かない。
#[test]
fn dropping_player_halts_further_delivery() {
    let sheet = CueSheet::new(vec![text(0.0, "a", 0.0), text(0.5, "b", 0.0)]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(0.0);
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("a".into())],
        "a は配送される"
    );

    // 中断: player を捨てる（受け手も一緒に捨てられ、記録の側だけが残る）。
    drop(player);
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("a".into())],
        "捨てた後に b は配送されない"
    );
    assert_eq!(
        Rc::strong_count(&log),
        1,
        "受け手は player と一緒に捨てられた"
    );
}

/// sink 未登録でも broadcast は破綻しない（配送先 0 個は 0 回 emit・完了判定は独立に成立）。
#[test]
fn tick_with_no_registered_sinks_still_advances_and_completes() {
    let sheet = CueSheet::new(vec![text(0.0, "a", 0.0), text(0.5, "b", 0.0)]);
    let mut player = CuePlayer::from_sheet(&sheet);

    player.tick(0.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "b 未到達ゆえ Playing"
    );
    player.tick(sheet.absolute_end_time());
    assert!(
        player.is_completed(),
        "sink 未登録でも占有 horizon 到達で完了する（配送先 0 個でも破綻しない）"
    );
}
