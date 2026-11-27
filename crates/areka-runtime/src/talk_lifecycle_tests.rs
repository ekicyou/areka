// =============================================================================
// BalloonLifecycleSink 決定論テスト
//
//   ⑴ 初回 emit の `TalkStarted` 先行
//   ⑵ `max(fire_time + duration)` の単調送出（max 集約）
//   ⑶ `Wait` cue の duration が占有終端へ算入されること
//   ⑷ clone 後の会話境界リセット（per-talk clone による talk 境界の自己検出）
//
// ＋縮退経路（いずれも無音でないことをログ捕捉で主張する）:
//   ⑸ 受信端切断時の送出失敗（WARN＋talk 非破壊）
//   ⑹ 信号の源は `emit` のみ（構築・複製は無音）
//
// 実時間の待機・反復回数のみによる有界化は用いない。
// =============================================================================

use super::*;
use crate::frame::test_support::{capture_logs, count_level, zero_clock};
use dola::cue::{ActorKey, CueCommand, CuePayload, CuePlayer, CueSink, TalkCue};
use dola::time::testing::{at_ms, ms};
use dola::{TimePoint, TimeSpan};
use std::sync::mpsc::{TryRecvError, channel};

// 台本レベルの会話終了観測で使う実経路の部品。
use areka_ghost::sink::BootCueSink;
use areka_sakura::CompiledTalk;
use areka_sakura::duration::CHAR_NOMINAL_MS;
use areka_sakura::sysvar::SystemVarSnapshot;

/// 任意の cue を組む（`at`／`duration` を明示指定する最小ヘルパ）。
fn cue(at: TimeSpan, duration: TimeSpan, command: CueCommand) -> TalkCue {
    TalkCue::scheduled(TimePoint::ZERO, at, ActorKey::from("0"), command, duration)
}

fn text_cue(at: TimeSpan, duration: TimeSpan) -> TalkCue {
    cue(at, duration, CueCommand::Text("あ".into()))
}

/// 待機 cue（action を持たず duration のみを運ぶ第一級 cue・`compile.rs` が発行する形）。
fn wait_cue(at: TimeSpan, duration: TimeSpan) -> TalkCue {
    cue(at, duration, CueCommand::Wait)
}

fn drain(rx: &std::sync::mpsc::Receiver<TalkLifecycleSignal>) -> Vec<TalkLifecycleSignal> {
    rx.try_iter().collect()
}

fn ends(rx: &std::sync::mpsc::Receiver<TalkLifecycleSignal>) -> Vec<TimePoint> {
    ends_of(&drain(rx))
}

/// 起点 0・台本の時刻 0 の最初の合図で出る開始の知らせ（番号なし）。
const STARTED: TalkLifecycleSignal = TalkLifecycleSignal::TalkStarted {
    at: TimePoint::ZERO,
    talk_id: None,
};

/// 発火時刻 0 の合図が上げた表示の終わりの知らせ。
fn end_from_zero(end: TimePoint) -> TalkLifecycleSignal {
    TalkLifecycleSignal::DisplayEndAt {
        at: TimePoint::ZERO,
        end,
    }
}

/// ⑴ per-talk clone の初回 `emit` で `TalkStarted` が 1 回だけ出て、
/// かつその talk の `DisplayEndAt` 群に**必ず先行**する。
#[test]
fn talk_started_is_emitted_once_and_precedes_display_end() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx, zero_clock());

    sink.emit(text_cue(ms(0), ms(500)));
    sink.emit(text_cue(ms(500), ms(250)));

    let signals = drain(&rx);
    assert_eq!(
        signals.first(),
        Some(&STARTED),
        "初回 emit の先頭は TalkStarted（Ordering 契約）: {signals:?}"
    );
    assert_eq!(
        signals.iter().filter(|s| **s == STARTED).count(),
        1,
        "TalkStarted は talk につき 1 回のみ: {signals:?}"
    );
    let started_at = signals
        .iter()
        .position(|s| *s == STARTED)
        .expect("TalkStarted が存在する");
    let first_end = signals
        .iter()
        .position(|s| matches!(s, TalkLifecycleSignal::DisplayEndAt { .. }))
        .expect("DisplayEndAt が存在する");
    assert!(
        started_at < first_end,
        "TalkStarted は DisplayEndAt に先行する: {signals:?}"
    );
}

/// ⑵ `DisplayEndAt` は `max(fire_time + duration)` が**既知最大を超えたときのみ**
/// 送出され、受信列は単調増加になる。
#[test]
fn display_end_is_monotonic_max_of_at_plus_duration() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx, zero_clock());

    sink.emit(text_cue(ms(0), ms(500))); // end = 0.5（増加）
    sink.emit(text_cue(ms(1_000), ms(1_000))); // end = 2.0（増加）
    sink.emit(text_cue(ms(500), ms(500))); // end = 1.0（後退＝送出しない）
    sink.emit(text_cue(ms(2_000), ms(0))); // end = 2.0（同値＝送出しない）
    sink.emit(text_cue(ms(2_000), ms(1_000))); // end = 3.0（増加）

    let ends = ends(&rx);

    assert_eq!(
        ends,
        vec![at_ms(500), at_ms(2_000), at_ms(3_000)],
        "既知最大の更新時のみ送出され、受信列は単調増加になる"
    );
    assert!(
        ends.windows(2).all(|w| w[0] < w[1]),
        "受信列は狭義単調増加（重複・後退値は送出されない）: {ends:?}"
    );
}

/// ⑶ 占有区間の終端は**待機を含む**。`Wait` cue の `duration` が
/// horizon へ算入され、最後の文字が現れた時刻ではなく台本の占有終端が届く。
///
/// 弁別: 実装が `duration` を落として `at` のみを horizon にすると最終値が 0.5 になり本 assert が落ちる。
#[test]
fn wait_cue_duration_is_included_in_occupancy_horizon() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx, zero_clock());

    // 文字 @0.0（D=0.5）→ 待機 @0.5（D=1.25）。占有終端 = 0.5 + 1.25 = 1.75。
    sink.emit(text_cue(ms(0), ms(500)));
    sink.emit(wait_cue(ms(500), ms(1_250)));

    let ends = ends(&rx);

    assert_eq!(
        ends.last().copied(),
        Some(at_ms(1_750)),
        "待機 cue の duration が占有終端へ算入される（最後の文字の時刻 0.5 ではない）: {ends:?}"
    );
}

/// ⑶-補: dola の占有 horizon 定義（`CueSheet::relative_end` の `max(start_time + duration)`）と
/// 同一値になることを、実 cue 列（Wait 含む）を流して確認する。期待は式で作らず値で書く。
#[test]
fn horizon_matches_dola_occupancy_definition() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx, zero_clock());

    let cues = vec![
        cue(ms(0), ms(0), CueCommand::ClearAll),
        text_cue(ms(0), ms(150)),
        wait_cue(ms(150), ms(800)),
        text_cue(ms(150), ms(250)),
    ];
    let expected = at_ms(950);

    for c in cues {
        sink.emit(c);
    }

    let ends = ends(&rx);

    assert_eq!(
        ends.last().copied(),
        Some(expected),
        "最終 horizon は dola の占有終端 0.95 と一致する: {ends:?}"
    );
}

/// ⑷ dispatcher は talk 起動ごとに登録 sink を clone する（`areka-ghost` の dispatcher）。
/// clone は会話境界の状態を持ち越さないため、2 本目の talk でも `TalkStarted` が改めて出る。
#[test]
fn clone_resets_talk_boundary_so_next_talk_re_emits_talk_started() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut registered = BalloonLifecycleSink::new(tx, zero_clock());

    let mut talk1 = registered.clone_box();
    talk1.emit(text_cue(ms(0), ms(2_000)));
    let talk1_signals = drain(&rx);
    assert_eq!(
        talk1_signals,
        vec![STARTED, end_from_zero(at_ms(2_000)),],
        "1 本目の talk: TalkStarted 先行＋占有終端"
    );

    let mut talk2 = registered.clone_box();
    talk2.emit(text_cue(ms(0), ms(500)));
    let talk2_signals = drain(&rx);
    assert_eq!(
        talk2_signals,
        vec![STARTED, end_from_zero(at_ms(500)),],
        "2 本目の talk でも TalkStarted が再送され、horizon は前 talk（2.0）を持ち越さない"
    );

    // 会話境界の状態は clone で必ずリセットされる——emit 済みインスタンスからの clone でも同じ
    // （登録 sink が emit されないという上流の暗黙不変条件に依存しない）。
    let mut talk3 = talk1.clone_box();
    talk3.emit(text_cue(ms(0), ms(250)));
    assert_eq!(
        drain(&rx),
        vec![STARTED, end_from_zero(at_ms(250)),],
        "emit 済みインスタンスからの clone も会話境界をリセットする"
    );

    registered.emit(text_cue(ms(0), ms(100)));
    assert_eq!(
        drain(&rx),
        vec![STARTED, end_from_zero(at_ms(100)),],
        "登録 sink 自身も未 emit 状態から始まる"
    );
}

/// 送信失敗（受信端切断）は talk を殺さない——非 panic・無音破棄でもない（log-first・
/// `MoveCueSink` と同一規律）。
///
/// 弁別: 送出結果を `let _ =` で捨てる無音実装は WARN=0 になって落ちる。さらに WARN の件数が
/// 「送出を試みた回数」そのものなので、切断後も horizon 集約が前進し続けている（0.5→1.0 の
/// 2 回目の更新でも送出を試みている）ことを、受信端が無い状況で唯一観測できる形で主張する。
#[test]
fn send_failure_after_receiver_drop_is_logged_and_non_fatal() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx, zero_clock());
    drop(rx);

    let logs = capture_logs(|| {
        sink.emit(text_cue(ms(0), ms(500))); // TalkStarted ＋ DisplayEndAt(0.5) の 2 回
        sink.emit(wait_cue(ms(500), ms(500))); // DisplayEndAt(1.0)（切断後も horizon は前進する）
    });

    assert_eq!(
        count_level(&logs, "WARN"),
        3,
        "切断後の送出失敗は毎回 WARN で記録される（無音破棄でない）。件数 3 は TalkStarted＋\
         DisplayEndAt(0.5)＋DisplayEndAt(1.0)＝切断後も horizon 集約が前進した証: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "受信端切断は talk を殺す致命ではない（WARN 一択・MoveCueSink と同一規律）: {logs:?}"
    );
}

/// 占有終端が 0.0 の talk（瞬時 cue のみ）でも `DisplayEndAt(0.0)` が届く——
/// 「信号が一度も届かない」状態と区別できる形にする。
#[test]
fn instantaneous_talk_still_reports_zero_horizon() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx, zero_clock());

    sink.emit(cue(ms(0), ms(0), CueCommand::ClearAll));

    assert_eq!(
        drain(&rx),
        vec![STARTED, end_from_zero(TimePoint::ZERO),],
        "占有終端 0.0 も信号として届く（信号欠落と区別できる）"
    );
}

/// 信号の唯一の源は `emit` である——構築（`new`）も複製（`clone`）も何も送出しない。
///
/// 弁別: 会話境界の自己検出を「複製した瞬間に `TalkStarted` を送る」形で実装すると、cue が
/// 1 つも来ない talk（バリア待ちのみで終わる等）で `TalkStarted` だけが先走って届き、受信側の
/// 計測が破棄されたまま `DisplayEndAt` が来ない状態を作る。それを本テストが落とす。
#[test]
fn construction_and_clone_emit_nothing() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let sink = BalloonLifecycleSink::new(tx, zero_clock());

    assert_eq!(
        rx.try_recv(),
        Err(TryRecvError::Empty),
        "構築だけでは信号は出ない"
    );

    // dispatcher が talk 起動ごとに行う複製（clone_box 相当）を 2 回踏んでも、emit が無い限り無音。
    let clone1 = sink.clone_box();
    let _clone2 = clone1.clone_box();
    assert_eq!(
        rx.try_recv(),
        Err(TryRecvError::Empty),
        "複製だけでは信号は出ない（信号の源は emit のみ）"
    );
}

// =============================================================================
// 台本レベルの会話終了観測
//
// 上の単体の檻は手組みの `TalkCue` を直接 `emit` する。ここから下は
// **実台本** を parse → compile → `CuePlayer` と通し、本番と同じ経路で sink が受け取る
// cue から占有終端を組み立てる。単体檻との違いは 2 点:
//
//   ⒜ 待機の duration が cue へ載る経路（compile）まで込みで検証する。手組み cue は
//      「Wait cue が duration を運ぶ」という compile の性質を仮定してしまっている。
//   ⒝ 期待値を実行結果から読まず、per-char ノミナルウェイトと `\w` 単位から導出する。
//
// 待機を**効かせる**ために、台本は末尾を待機で終える形にしてある——待機の後ろに文字が
// 続くと、その文字 cue の発火時刻に待機時間が畳み込まれてしまい、待機の duration を
// 落としても占有終端が変わらない（=待機の検証にならない）。この落とし穴は
// `frame_attach_tests.rs::talk_playback_reaches_runtime_wiring_lifecycle_receiver` の台本
// `\0あい\w9\1うえ\e` に実在する（末尾の文字 cue の終端 0.65 が待機の終端 0.55 を上回る）。
//
// 時刻は台本から導いた発火時刻の注入のみで進める（実時間の待機なし）。
// =============================================================================

/// `\wN` の 1 ウェイト単位（ミリ秒）。
///
/// 定義は parser 側の非公開定数（`areka-parsers` の sakura decode の
/// `WAIT_UNIT_MS`・ukadoc 確定の 50ms）。公開されていないため本檻が同値を持つ——
/// 期待値を実行結果から読まずに導出するための写しであり、値が食い違えば下の
/// 「台本の占有終端との一致」assert が落ちる。
const WAIT_UNIT_MS: u64 = 50;

/// ミリ秒の数を長さにする（期待値の導出用）。
fn millis(n: u64) -> TimeSpan {
    TimeSpan::try_from_millis(n).expect("長さの型に収まる")
}

/// 台本を parse → compile し、cue の発火時刻を昇順に辿って sink へ配送する。
///
/// 注入する時刻は台本自身から導いた発火時刻と、末尾の観測点（占有終端）だけ——実時間の
/// 待機もスピンも使わず、観測すべき時点を追い越さない。
fn play_script(script: &str, sink: Box<dyn CueSink>) -> CompiledTalk {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = areka_sakura::compile(&instructions, &SystemVarSnapshot::default());

    let sheet = compiled
        .sheet
        .clone()
        .with_absolute_start_time(TimePoint::ZERO);
    let mut player = CuePlayer::from_sheet(&sheet).expect("起点を刻んだ台本");
    player.register_sink(sink);

    let mut tick_points: Vec<TimePoint> = sheet
        .cues()
        .iter()
        .map(|cue| sheet.absolute_fire_time(cue).expect("起点を刻んだ台本"))
        .collect();
    tick_points.push(sheet.absolute_end_time().expect("起点を刻んだ台本"));
    for at in tick_points {
        player.tick(at);
    }

    compiled
}

/// 受信列から `DisplayEndAt` の値だけを順序どおり抜き出す（受信済みスライス版）。
fn ends_of(signals: &[TalkLifecycleSignal]) -> Vec<TimePoint> {
    signals
        .iter()
        .filter_map(|s| match s {
            TalkLifecycleSignal::DisplayEndAt { end, .. } => Some(*end),
            _ => None,
        })
        .collect()
}

/// 完了条件: **待機を含む台本**で占有終端が台本の占有区間の終端と一致する。
/// あわせて会話開始の通知が占有終端の通知に先行する。
///
/// 台本 `\0あい\w9\e` の占有終端の導出（実行結果からは読まない）:
///
/// | cue | 発火 | duration | 終端 |
/// |---|---|---|---|
/// | `ClearAll`（compile が先頭へ前置） | 0.0 | 0.0 | 0.0 |
/// | `Text("あい")` | 0.0 | 2 文字 × 50ms = 0.1 | 0.1 |
/// | `Wait`（`\w9`） | 0.1 | 9 × 50ms = 0.45 | **0.55** |
///
/// 待機が末尾ゆえ占有終端 0.55 は待機だけが決める——待機の duration が落ちれば 0.1 になる。
#[test]
fn script_ending_with_a_wait_reports_occupancy_end_that_only_the_wait_determines() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();

    // 期待値の導出: 文字は per-char ノミナルウェイト、待機は `\w` 単位から。
    let text_span = millis(2 * CHAR_NOMINAL_MS);
    let wait_span = millis(9 * WAIT_UNIT_MS);
    let expected_end = text_span + wait_span;

    let compiled = play_script(
        r"\0あい\w9\e",
        Box::new(BalloonLifecycleSink::new(tx, zero_clock())),
    );

    // 前提①: 待機が効いている台本であること——待機以外の cue の終端は占有終端に届かない。
    // （待機の後ろに文字が続く台本ではここが等しくなり、待機の検証にならない。）
    let non_wait_end = compiled
        .sheet
        .cues()
        .iter()
        .filter(|cue| !matches!(&cue.payload, CuePayload::Command(CueCommand::Wait)))
        .map(|cue| cue.start_time + cue.duration)
        .max()
        .unwrap_or(TimeSpan::ZERO);
    assert!(
        non_wait_end < expected_end,
        "前提: 占有終端 {expected_end} は待機だけが決める（待機以外の終端は {non_wait_end}）"
    );

    // 前提②: 導出した期待値が台本の占有区間の終端（dola の定義）と一致すること。
    assert_eq!(
        compiled.sheet.relative_end(),
        expected_end,
        "前提: 導出値は台本の占有区間の終端と一致する（文字 {text_span} ＋ 待機 {wait_span}）"
    );

    let signals = drain(&rx);

    let started_positions: Vec<usize> = signals
        .iter()
        .enumerate()
        .filter(|(_, s)| **s == STARTED)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        started_positions,
        vec![0],
        "会話開始の通知は台本の先頭で 1 回だけ出る: {signals:?}"
    );
    let end_positions: Vec<usize> = signals
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s, TalkLifecycleSignal::DisplayEndAt { .. }))
        .map(|(i, _)| i)
        .collect();
    assert!(
        !end_positions.is_empty() && end_positions.iter().all(|i| *i > 0),
        "占有終端の通知はすべて会話開始の通知より後に届く: {signals:?}"
    );

    let ends = ends_of(&signals);
    assert_eq!(
        ends.last().copied(),
        Some(TimePoint::ZERO + expected_end),
        "待機を含む台本の占有終端が届く（待機の duration を落とすと {non_wait_end} になる）: {ends:?}"
    );
}

/// 会話境界のリセットを**本番の複製経路**で確かめる。
///
/// dispatcher は登録 sink を talk 起動ごとに `clone_box` で複製する
/// （`areka-ghost` の dispatcher）。本檻はその `BootCueSink::clone_box`
/// をそのまま踏み、2 本の実台本を続けて再生する。
///
/// 2 本目の占有終端（0.05）は 1 本目（0.7）より**小さい**——会話境界が持ち越されると
/// 2 本目は既知最大を超えられず、占有終端の通知が 1 件も出ない状態になる。
///
/// 3 本目の脚は、1 本目の cue を実際に浴びたインスタンスからの複製を試す。今日の dispatcher は
/// 登録 sink を emit しないため複製元は常に未使用だが、本 sink はその上流の暗黙不変条件に
/// 依存せず境界を戻す設計であり（`talk_lifecycle.rs` の `impl BootCueSink`（`clone_box`）の注記）、実台本の
/// 占有終端でもその保証が成り立つことをここで押さえる。
#[test]
fn per_talk_clone_box_resets_the_conversation_boundary_between_two_scripts() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    // `GhostBootOptions.sinks` へ登録される形（複製元）。
    let registered: Box<dyn BootCueSink> =
        Box::new(BalloonLifecycleSink::new(tx.clone(), zero_clock()));

    // 1 本目 `\0あいうえお\w9\e`: 文字 5 × 50ms = 0.25 ＋ 待機 9 × 50ms = 0.45 → 終端 0.7。
    // （`\wN` の N は 1 桁のみ——`\w20` は `\w2` ＋ 文字 "0" になる。長い待ちは `\_w[ms]` 形。）
    let first_end = millis(5 * CHAR_NOMINAL_MS) + millis(9 * WAIT_UNIT_MS);
    // 2 本目 `\0あ\e`: 文字 1 × 50ms → 終端 0.05。
    let second_end = millis(CHAR_NOMINAL_MS);
    assert!(
        second_end < first_end,
        "前提: 2 本目の占有終端 {second_end} は 1 本目 {first_end} より小さい（境界が持ち越されると通知が消える形）"
    );

    let first = play_script(r"\0あいうえお\w9\e", registered.clone_box());
    let first_signals = drain(&rx);
    assert_eq!(
        first.sheet.relative_end(),
        first_end,
        "前提: 1 本目の導出値は台本の占有区間の終端と一致する"
    );
    assert_eq!(
        first_signals.first(),
        Some(&STARTED),
        "1 本目: 会話開始の通知が先行する: {first_signals:?}"
    );
    assert_eq!(
        ends_of(&first_signals).last().copied(),
        Some(TimePoint::ZERO + first_end),
        "1 本目: 待機を含む占有終端が届く: {first_signals:?}"
    );

    let second = play_script(r"\0あ\e", registered.clone_box());
    let second_signals = drain(&rx);
    assert_eq!(
        second.sheet.relative_end(),
        second_end,
        "前提: 2 本目の導出値は台本の占有区間の終端と一致する"
    );
    assert_eq!(
        second_signals.first(),
        Some(&STARTED),
        "2 本目: 複製ごとに会話開始の通知が改めて出る: {second_signals:?}"
    );
    assert_eq!(
        ends_of(&second_signals).last().copied(),
        Some(TimePoint::ZERO + second_end),
        "2 本目: 前の会話の占有終端（{first_end}）を持ち越さず、自分の終端が届く: {second_signals:?}"
    );

    // 3 本目: 1 本目の cue を実際に浴びたインスタンスから複製する。複製元の既知最大は 0.7 まで
    // 上がっているため、境界を引き継ぐ複製では 2 本目と同じ台本（終端 0.05）で通知が 1 件も出ない。
    let mut used = BalloonLifecycleSink::new(tx, zero_clock());
    for cue in first.sheet.cues() {
        if let CuePayload::Command(command) = &cue.payload {
            used.emit(TalkCue::scheduled(
                TimePoint::ZERO,
                cue.start_time,
                cue.actor.clone(),
                command.clone(),
                cue.duration,
            ));
        }
    }
    let used_ends = ends(&rx);
    assert_eq!(
        used_ends.last().copied(),
        Some(TimePoint::ZERO + first_end),
        "前提: 複製元は 1 本目の占有終端まで既知最大を上げてある: {used_ends:?}"
    );

    let used_registered: Box<dyn BootCueSink> = Box::new(used);
    play_script(r"\0あ\e", used_registered.clone_box());
    let third_signals = drain(&rx);
    assert_eq!(
        third_signals.first(),
        Some(&STARTED),
        "3 本目: 使用済みインスタンスからの複製でも会話開始の通知が出る: {third_signals:?}"
    );
    assert_eq!(
        ends_of(&third_signals).last().copied(),
        Some(TimePoint::ZERO + second_end),
        "3 本目: 使用済みインスタンスからの複製でも前の会話の占有終端（{first_end}）を持ち越さない: {third_signals:?}"
    );
}
