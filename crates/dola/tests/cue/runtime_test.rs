//! `CuePlayer`（受動的注入時刻ランタイム）の状態機械骨格の檻。
//!
//! Task 4.2: cue 再生の状態管理（再生中・入力待ち・選択肢待ち・完了）を dola の受動的
//! ランタイムへ一本化し、外部解決待ちの停止点（バリア seam）と選択肢の先積みを、動的な
//! 一時停止/再開の状態（Non-Goals）を持ち込まずに移植したことを公開 API 越しに固定する
//! （R11.2・D6/D7）。
//!
//! 主 observable: **バリアに到達すると停止し、外部からの解決通知で再開する**（バリア手前の
//! cue は配送されるが、バリア以降の cue は解決通知が来るまで配送されない）。

use dola::cue::{
    ActorKey, BarrierKind, Cue, CueCommand, CuePayload, CuePlayer, CuePlayerState, CueSheet,
    CueSink, TalkCue, TimedSchedule, to_talk_schedule,
};
use std::cell::RefCell;
use std::rc::Rc;

/// バリア cue を作る補助（Barrier ペイロードは presentation でなく duration 0）。
fn barrier(start_time: f64, kind: BarrierKind) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CuePayload::Barrier(kind),
        duration: 0.0,
    }
}

/// テキスト cue を作る補助。
fn text(start_time: f64, s: &str, duration: f64) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CueCommand::Text(s.into()).into(),
        duration,
    }
}

/// 選択肢 cue を作る補助（WaitForChoice の前に先積みされる）。
fn choice(start_time: f64, id: &str, s: &str) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CueCommand::Choice {
            id: id.into(),
            text: s.into(),
            references: vec![],
        }
        .into(),
        duration: 0.0,
    }
}

/// ready() の中に指定テキストが含まれるか。
fn ready_has_text(player: &CuePlayer, s: &str) -> bool {
    player
        .ready()
        .iter()
        .any(|c| c.command == CueCommand::Text(s.into()))
}

// ============================================================================
// 主 observable: WaitForInput バリアで停止し、外部解決（resolve_click）で再開する
// ============================================================================

/// **観測可能な完了条件（Task 4.2）**: WaitForInput バリアに到達すると再生が停止し、
/// バリア以降の cue は配送されない。外部からの解決通知（`resolve_click`）で再開し、以降の
/// cue が配送される。占有 horizon 到達で `Completed` へ遷移する。
#[test]
fn wait_for_input_barrier_stops_and_resolve_click_resumes() {
    // before@0.0 → Barrier(WaitForInput)@0.1 → after@0.2。
    let sheet = CueSheet::new(vec![
        text(0.0, "before", 0.0),
        barrier(0.1, BarrierKind::WaitForInput { timeout: None }),
        text(0.2, "after", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "初期状態は Playing"
    );

    // 全時刻を跨ぐ tick を送るが、バリアで停止するため before までしか配送されない。
    player.tick(10.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForInput,
        "バリア到達で WaitingForInput へ停止する"
    );
    assert!(
        ready_has_text(&player, "before"),
        "バリア手前の before は配送される"
    );
    assert!(
        !ready_has_text(&player, "after"),
        "バリア以降の after は解決前に配送されない（停止の証）"
    );
    assert!(
        player.current_barrier().is_some(),
        "現在バリアで停止中である"
    );
    assert_eq!(player.remaining(), 1, "after はスケジュールに残置される");

    // 待機中に更に tick を送っても再生は進まない（after は依然配送されない）。
    player.tick(20.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForInput,
        "待機中の tick は進行しない（バリアが保持される）"
    );
    assert!(
        !ready_has_text(&player, "after"),
        "待機中の tick でも after は配送されない"
    );
    assert_eq!(player.remaining(), 1, "after は依然残置");

    // 外部解決（クリック）で再開する。
    player.resolve_click();
    // 解決直後、次の tick で after が配送され、占有 horizon 到達で完了する。
    player.tick(20.0);
    assert!(
        ready_has_text(&player, "after"),
        "解決通知の後、after が配送される（再開の証）"
    );
    assert_eq!(
        player.state(),
        &CuePlayerState::Completed,
        "全 cue 配送かつ horizon 到達で Completed へ遷移する"
    );
}

// ============================================================================
// WaitForChoice バリア: 選択肢先積み ＋ resolve_choice で再開
// ============================================================================

/// **選択肢先積み ＋ 再開（Task 4.2）**: WaitForChoice バリアの手前で連続投入された Choice cue は
/// `ready()` に action cue として現れず `pending_choices()` へ先積みされる。バリア到達で
/// WaitingForChoice へ停止し、`resolve_choice` で再開する。
#[test]
fn wait_for_choice_barrier_preloads_choices_and_resolve_choice_resumes() {
    // Choice(a)@0.0, Choice(b)@0.0 → Barrier(WaitForChoice)@0.1 → picked@0.2。
    let sheet = CueSheet::new(vec![
        choice(0.0, "a", "選択A"),
        choice(0.0, "b", "選択B"),
        barrier(0.1, BarrierKind::WaitForChoice { timeout: None }),
        text(0.2, "picked", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);

    player.tick(10.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForChoice,
        "WaitForChoice バリアで WaitingForChoice へ停止する"
    );

    // Choice は ready action cue として現れず、pending_choices へ先積みされる。
    assert!(
        !player
            .ready()
            .iter()
            .any(|c| matches!(c.command, CueCommand::Choice { .. })),
        "Choice は ready の action cue として surface されない（先積みプロトコル）"
    );
    let ids: Vec<&str> = player
        .pending_choices()
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(ids, vec!["a", "b"], "選択肢が記述順で先積みされる");
    let texts: Vec<&str> = player
        .pending_choices()
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert_eq!(texts, vec!["選択A", "選択B"], "選択肢テキストも保持される");
    assert!(
        !ready_has_text(&player, "picked"),
        "バリア以降の picked は未配送"
    );

    // 未知 id は再開しない（該当 id のみ解決可能）。
    assert_eq!(
        player.resolve_choice("zzz"),
        None,
        "未知 id では解決されない"
    );
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForChoice,
        "未知 id 解決後も WaitingForChoice のまま"
    );

    // 該当 id で再開。
    assert_eq!(
        player.resolve_choice("b"),
        Some("b".to_string()),
        "該当 id は解決され id を返す"
    );
    assert!(
        player.pending_choices().is_empty(),
        "解決で先積み選択肢はクリアされる"
    );
    player.tick(20.0);
    assert!(
        ready_has_text(&player, "picked"),
        "選択肢解決の後、picked が配送される（再開の証）"
    );
    assert_eq!(
        player.state(),
        &CuePlayerState::Completed,
        "horizon 到達で完了"
    );
}

/// 選択肢は WaitForChoice バリアの手前で**複数 tick に跨って**累積し、
/// `pending_choices()` から取得できる（先積みの累積性）。
#[test]
fn choices_accumulate_across_ticks_before_choice_barrier() {
    // Choice(a)@0.0 → Choice(b)@0.1 → Barrier(WaitForChoice)@0.2。
    let sheet = CueSheet::new(vec![
        choice(0.0, "a", "A"),
        choice(0.1, "b", "B"),
        barrier(0.2, BarrierKind::WaitForChoice { timeout: None }),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);

    player.tick(0.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "0.0 ではまだ Playing"
    );
    assert_eq!(
        player
            .pending_choices()
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a"],
        "0.0 で選択肢 a が先積みされる"
    );

    player.tick(0.1);
    assert_eq!(
        player
            .pending_choices()
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"],
        "0.1 で選択肢 b が追加累積される（跨 tick 累積）"
    );
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "0.1 でもまだ Playing"
    );

    player.tick(0.2);
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForChoice,
        "バリア到達で停止"
    );
    assert_eq!(
        player
            .pending_choices()
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"],
        "バリア時点で累積した全選択肢が先積みされている"
    );
}

// ============================================================================
// バリア seam: skip_barrier による強制再開
// ============================================================================

/// `skip_barrier` は待機状態（入力/選択いずれも）を強制的に Playing へ戻す。
#[test]
fn skip_barrier_force_resumes_from_waiting() {
    let sheet = CueSheet::new(vec![
        text(0.0, "before", 0.0),
        barrier(0.1, BarrierKind::WaitForInput { timeout: None }),
        text(0.2, "after", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);

    player.tick(10.0);
    assert_eq!(player.state(), &CuePlayerState::WaitingForInput);

    player.skip_barrier();
    player.tick(20.0);
    assert!(
        ready_has_text(&player, "after"),
        "skip_barrier で強制再開し after が配送される"
    );
    assert_eq!(player.state(), &CuePlayerState::Completed);
}

/// 非待機状態での解決通知は no-op（Playing のまま何も壊さない）。
#[test]
fn resolve_on_non_waiting_state_is_noop() {
    let sheet = CueSheet::new(vec![text(0.0, "only", 0.0)]);
    let mut player = CuePlayer::from_sheet(&sheet);

    // まだ tick 前＝Playing。resolve_click / skip_barrier は no-op。
    player.resolve_click();
    player.skip_barrier();
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "非待機での解決は no-op"
    );
    assert_eq!(
        player.resolve_choice("x"),
        None,
        "非待機の resolve_choice は None"
    );
}

// ============================================================================
// 完了は占有 horizon 到達で（entry 枯渇だけでは完了しない）
// ============================================================================

/// 末尾に待ちを持つ台本では、全 cue の**配送完了**時点ではまだ Playing のままで、
/// 占有 horizon（最終 Wait の duration 端）到達で初めて `Completed` へ遷移する
/// （早期終了しない・D6/R2.5 の CuePlayer レベル固定）。
#[test]
fn player_reaches_completed_only_at_occupancy_horizon() {
    let d = 0.15_f64;
    let wait_dur = 0.8_f64;
    let sheet = CueSheet::new(vec![
        text(0.0, "bye", d),
        Cue {
            actor: ActorKey::from("0"),
            start_time: d,
            payload: CueCommand::Wait.into(),
            duration: wait_dur,
        },
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);

    // 全 entry を配り終える時刻（末尾 Wait の発火時刻＝d）まで進める。
    player.tick(d);
    assert_eq!(player.remaining(), 0, "全 entry を配り終えた");
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "配送完了時点では占有 horizon 未到達ゆえまだ Playing（早期終了しない）"
    );

    // 占有 horizon 直前も未完了。
    player.tick(d + wait_dur - 0.01);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "horizon 直前は未完了"
    );

    // 占有 horizon 到達で Completed。
    player.tick(d + wait_dur);
    assert_eq!(
        player.state(),
        &CuePlayerState::Completed,
        "占有 horizon 到達で初めて Completed"
    );
}

/// バリアの無い台本は占有 horizon（全 cue が瞬時なら最終 at）到達で完了する。
#[test]
fn barrier_free_sheet_completes_at_last_cue() {
    let sheet = CueSheet::new(vec![text(0.0, "a", 0.0), text(0.5, "b", 0.0)]);
    let mut player = CuePlayer::from_sheet(&sheet);

    player.tick(0.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "b 未到達ゆえ Playing"
    );
    player.tick(0.5);
    assert_eq!(
        player.state(),
        &CuePlayerState::Completed,
        "最終 cue（horizon）到達で完了"
    );
}

// ============================================================================
// Timeout バリア: Playing を維持しつつ継続 tick で自動解除される
// ============================================================================

/// Timeout バリアは待機状態にせず Playing を維持し（schedule が自動管理）、継続 tick で
/// duration 経過後に自動解除されて後続 cue が配送される。
#[test]
fn timeout_barrier_keeps_playing_and_auto_resolves_by_continued_ticking() {
    // before@0.0 → Barrier(Timeout{1.0})@0.1 → after@0.2。
    let sheet = CueSheet::new(vec![
        text(0.0, "before", 0.0),
        barrier(0.1, BarrierKind::Timeout { duration: 1.0 }),
        text(0.2, "after", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);

    // Timeout バリアに到達しても Playing を維持する（WaitingFor* へは遷移しない）。
    player.tick(0.1);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "Timeout バリアは Playing を維持する（自動管理）"
    );
    assert!(
        !ready_has_text(&player, "after"),
        "duration 未経過ゆえ after は未配送"
    );

    // Timeout duration（0.1 + 1.0 = 1.1）経過後の tick で自動解除され after が配送される。
    player.tick(1.1);
    assert!(
        ready_has_text(&player, "after"),
        "Timeout duration 経過で自動解除され after が配送される"
    );
}

// ============================================================================
// 構築経路: from_schedule も from_sheet と等価（4.3/7.1 との合成用）
// ============================================================================

/// `from_schedule` は canonical 変換で得た `TimedSchedule<TalkCue>` を直接包む代替構築口。
/// `from_sheet`（内部で `to_talk_schedule` を呼ぶ）と同一挙動になる。
#[test]
fn from_schedule_is_equivalent_to_from_sheet() {
    let sheet = CueSheet::new(vec![
        text(0.0, "x", 0.0),
        barrier(0.1, BarrierKind::WaitForInput { timeout: None }),
        text(0.2, "y", 0.0),
    ]);

    // from_sheet 経路。
    let mut a = CuePlayer::from_sheet(&sheet);
    // from_schedule 経路（同一 canonical 変換を明示的に通す）。
    let schedule: TimedSchedule<_> = to_talk_schedule(&sheet);
    let mut b = CuePlayer::from_schedule(schedule);

    a.tick(10.0);
    b.tick(10.0);
    assert_eq!(a.state(), b.state(), "両構築経路の状態遷移は一致する");
    assert_eq!(a.state(), &CuePlayerState::WaitingForInput);
    assert_eq!(
        a.ready()
            .iter()
            .map(|c| c.command.clone())
            .collect::<Vec<_>>(),
        b.ready()
            .iter()
            .map(|c| c.command.clone())
            .collect::<Vec<_>>(),
        "両構築経路の ready 列は一致する"
    );
}

// ============================================================================
// Task 4.3: broadcast 配送・占有終了検知（is_completed）・中断（stop）・sink 登録
// ============================================================================

/// broadcast の記録用テストダブル sink。受け取った全 `TalkCue` を共有ログへ push する。
///
/// 演者側 relevance（担当 action の選別）は 4.3 の範囲外（演者タスク 6.x）ゆえ、ここでは
/// 「登録された全 sink が全 cue を無変形で受け取る」ことのみを観測する（broadcast＝全員が
/// 全 cue を受ける・R2.1）。
struct RecordingSink {
    log: Rc<RefCell<Vec<TalkCue>>>,
}

impl CueSink for RecordingSink {
    fn emit(&mut self, cue: TalkCue) {
        self.log.borrow_mut().push(cue);
    }
}

/// 記録用 sink を作り、（外から中身を覗くための共有ログ, `CuePlayer` へ渡す trait object）を返す。
fn recording_sink() -> (Rc<RefCell<Vec<TalkCue>>>, Box<dyn CueSink>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let sink = Box::new(RecordingSink { log: log.clone() });
    (log, sink)
}

/// ログに記録された cue の command 列だけを取り出す補助。
fn logged_commands(log: &Rc<RefCell<Vec<TalkCue>>>) -> Vec<CueCommand> {
    log.borrow().iter().map(|c| c.command.clone()).collect()
}

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
        Cue {
            actor: actor.clone(),
            start_time: 1.5,
            payload: CueCommand::Wait.into(),
            duration: 0.25,
        },
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

    // 各 cue の絶対発火時刻はアンカー 100.0 ＋ 相対 `at`。全 sink が同一 `at`・同一アンカーゆえ
    // 独立に同一絶対時刻を算出できる（協調不要で同期成立・R1.4）。
    assert_eq!(
        a.iter()
            .map(|c| sheet.absolute_start_time() + c.at)
            .collect::<Vec<_>>(),
        vec![100.0, 100.5, 101.5],
        "絶対発火時刻はアンカー＋相対 at で台本から復元でき、全 sink で一致する"
    );
}

/// broadcast はバリア手前の cue のみを sink へ配り、バリア以降は外部解決まで配送しない
/// （broadcast をバリア seam と結線）。解決後は残り cue が配送され、各 cue は生涯 1 回のみ
/// （待機中・解決跨ぎで before を二重配送しない）。
#[test]
fn broadcast_stops_at_barrier_and_resumes_delivery_after_resolve() {
    let sheet = CueSheet::new(vec![
        text(0.0, "before", 0.0),
        barrier(0.1, BarrierKind::WaitForInput { timeout: None }),
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
/// （4.2 の `ready()` は冪等再読取で同一バッファを返すが、配送は schedule 前進時のみ）。
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

/// Timeout バリア継続中の（時刻前進する）tick でも、バリア手前の cue を二重配送しない
/// （schedule は Timeout 継続で early-return し ready バッファを据え置くが、配送は前進時のみ）。
#[test]
fn broadcast_does_not_redeliver_across_timeout_barrier_continuation() {
    let sheet = CueSheet::new(vec![
        text(0.0, "before", 0.0),
        barrier(0.1, BarrierKind::Timeout { duration: 1.0 }),
        text(0.2, "after", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    // before 配送 ＋ Timeout バリア到達（Playing 維持）。
    player.tick(0.1);
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("before".into())],
        "before のみ配送される"
    );

    // Timeout 継続中（duration 未経過）の前進 tick でも before を再配送しない。
    player.tick(0.5);
    assert_eq!(
        log.borrow().len(),
        1,
        "Timeout 継続中の前進 tick で before を二重配送しない"
    );

    // Timeout duration 経過（0.1 + 1.0 = 1.1）で自動解除され after が配送される。
    player.tick(1.1);
    assert_eq!(
        logged_commands(&log),
        vec![
            CueCommand::Text("before".into()),
            CueCommand::Text("after".into()),
        ],
        "Timeout 解除後に after が配送される（各 cue は 1 回のみ）"
    );
}

/// **観測可能な完了条件（Task 4.3・後段）**: 末尾に待ちを持つ台本では、全 cue の**配送完了**
/// （entry 枯渇）時点では `is_completed()` が false で、占有 horizon（最終 Wait の duration 端）
/// 到達で初めて true になる（配送 ≠ 再生完了・早期終了しない・R2.5/D6 の caller-facing 固定）。
#[test]
fn is_completed_is_gated_by_occupancy_horizon_for_trailing_wait() {
    let d = 0.15_f64;
    let wait_dur = 0.8_f64;
    let sheet = CueSheet::new(vec![
        text(0.0, "bye", d),
        Cue {
            actor: ActorKey::from("0"),
            start_time: d,
            payload: CueCommand::Wait.into(),
            duration: wait_dur,
        },
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    // 全 entry を配り終える時刻（末尾 Wait の発火時刻＝d）まで進める。
    player.tick(d);
    assert_eq!(player.remaining(), 0, "全 entry を配り終えた");
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
    player.tick(d + wait_dur - 0.01);
    assert!(!player.is_completed(), "horizon 直前は未完了");

    // 占有 horizon 到達で初めて完了通知が出る。
    player.tick(d + wait_dur);
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

    // 最終 Text の配送時刻（0.0）到達 — 配り終えたが再生（duration）は未完。
    player.tick(0.0);
    assert_eq!(player.remaining(), 0, "全 entry 配送済み");
    assert!(
        !player.is_completed(),
        "最終 Text の配送時点では duration 端未到達ゆえ未完了"
    );

    // duration 端（占有 horizon）到達で完了。
    player.tick(d);
    assert!(
        player.is_completed(),
        "最終 Text の duration 端到達で is_completed() が true"
    );
}

/// `stop()` は残 entry を破棄して以降の配送を止める（Close/中断の discard プリミティブ）。
/// 破棄後の tick では cue が届かず、プレイヤーは終端（完了）状態になる。
#[test]
fn stop_discards_remaining_and_halts_further_delivery() {
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

    // 中断: 残 entry（b）を破棄する。
    player.stop();
    assert_eq!(player.remaining(), 0, "stop で残 entry を破棄する");
    assert!(
        player.is_completed(),
        "stop 後はプレイヤーが終端（以降の配送はない）"
    );

    // 破棄後の tick では b は配送されない。
    player.tick(0.5);
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("a".into())],
        "stop 後の tick では b は配送されない"
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
    player.tick(0.5);
    assert!(
        player.is_completed(),
        "sink 未登録でも占有 horizon 到達で完了する（配送先 0 個でも破綻しない）"
    );
}
