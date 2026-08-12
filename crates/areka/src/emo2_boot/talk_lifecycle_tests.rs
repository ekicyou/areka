// =============================================================================
// BalloonLifecycleSink 決定論テスト（task 2.1・Requirements 4.1 / 4.5 / 4.8 / 7.2 / 7.8）
//
// design.md「Testing Strategy → Unit Tests ②」の 4 系統:
//   ⑴ 初回 emit の `TalkStarted` 先行（Ordering 契約）
//   ⑵ `max(at + duration)` の単調送出（Idempotency 契約＝max 集約）
//   ⑶ `Wait` cue の duration が占有終端へ算入されること（Requirement 4.1）
//   ⑷ clone 後の会話境界リセット（per-talk clone による talk 境界の自己検出）
//
// ＋縮退経路（いずれも無音でないことをログ捕捉で主張する・log-first）:
//   ⑸ 受信端切断時の送出失敗（WARN＋talk 非破壊・Requirement 4.8）
//   ⑹ 非有限な占有終端の記録付きスキップ
//   ⑺ 信号の源は `emit` のみ（構築・複製は無音）
//
// 実時間の待機・反復回数のみによる有界化は一切用いない（Requirement 9.2 / 9.3）。
// =============================================================================

use super::*;
use dola::cue::{ActorKey, CueCommand, CueSink, TalkCue};
use std::sync::mpsc::{TryRecvError, channel};
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing_subscriber::prelude::*;

/// 任意の cue を組む（`at`／`duration` を明示指定する最小ヘルパ）。
fn cue(at: f64, duration: f64, command: CueCommand) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from("0"),
        command,
        duration,
    }
}

/// 文字 cue（占有時間つき）。
fn text_cue(at: f64, duration: f64) -> TalkCue {
    cue(at, duration, CueCommand::Text("あ".into()))
}

/// 待機 cue（action を持たず duration のみを運ぶ第一級 cue・`compile.rs` が発行する形）。
fn wait_cue(at: f64, duration: f64) -> TalkCue {
    cue(at, duration, CueCommand::Wait)
}

/// 受信端から届いた信号を全件取り出す（非ブロック・実時間待機なし）。
fn drain(rx: &std::sync::mpsc::Receiver<TalkLifecycleSignal>) -> Vec<TalkLifecycleSignal> {
    rx.try_iter().collect()
}

/// 受信列から `DisplayEndAt` の値だけを順序どおり抜き出す。
fn ends(rx: &std::sync::mpsc::Receiver<TalkLifecycleSignal>) -> Vec<f64> {
    drain(rx)
        .into_iter()
        .filter_map(|s| match s {
            TalkLifecycleSignal::DisplayEndAt(h) => Some(h),
            TalkLifecycleSignal::TalkStarted => None,
        })
        .collect()
}

// -----------------------------------------------------------------------------
// ログ捕捉（無音破棄でないことを主張するため・log-first）
//
// 定石は `move_cue_move_severity_log_tests.rs` と同一——`tracing::subscriber::with_default`
// ＋スレッドローカル Capture Layer。`with_default` はスレッドローカルゆえ `cargo test` の
// 並行実行でも干渉しない。
// -----------------------------------------------------------------------------

/// イベントの `level` を 1 行文字列へ整形して共有 Vec へ push する最小 Layer。
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
    fn on_event(&self, ev: &tracing::Event<'_>, _: tracing_subscriber::layer::Context<'_, S>) {
        let meta = ev.metadata();
        let mut line = format!("level={} target={}", meta.level(), meta.target());
        struct V<'a>(&'a mut String);
        impl Visit for V<'_> {
            fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                let _ = write!(self.0, " {}={:?}", f.name(), v);
            }
        }
        ev.record(&mut V(&mut line));
        self.0.lock().unwrap().push(line);
    }
}

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let cap = Capture::default();
    let logs = cap.0.clone();
    let subscriber = tracing_subscriber::registry().with(cap);
    tracing::subscriber::with_default(subscriber, f);
    let guard = logs.lock().unwrap();
    guard.clone()
}

/// 捕捉行のうち指定 level（例 `"WARN"`）の件数を数える。
fn count_level(logs: &[String], level: &str) -> usize {
    let needle = format!("level={level}");
    logs.iter().filter(|l| l.contains(&needle)).count()
}

/// ⑴ Ordering 契約: per-talk clone の初回 `emit` で `TalkStarted` が 1 回だけ出て、
/// かつその talk の `DisplayEndAt` 群に**必ず先行**する（Requirement 4.5）。
#[test]
fn talk_started_is_emitted_once_and_precedes_display_end() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx);

    sink.emit(text_cue(0.0, 0.5));
    sink.emit(text_cue(0.5, 0.25));

    let signals = drain(&rx);
    assert_eq!(
        signals.first(),
        Some(&TalkLifecycleSignal::TalkStarted),
        "初回 emit の先頭は TalkStarted（Ordering 契約）: {signals:?}"
    );
    assert_eq!(
        signals
            .iter()
            .filter(|s| **s == TalkLifecycleSignal::TalkStarted)
            .count(),
        1,
        "TalkStarted は talk につき 1 回のみ: {signals:?}"
    );
    // TalkStarted は全 DisplayEndAt に先行する（添字比較で位置関係を直接主張する）。
    let started_at = signals
        .iter()
        .position(|s| *s == TalkLifecycleSignal::TalkStarted)
        .expect("TalkStarted が存在する");
    let first_end = signals
        .iter()
        .position(|s| matches!(s, TalkLifecycleSignal::DisplayEndAt(_)))
        .expect("DisplayEndAt が存在する");
    assert!(
        started_at < first_end,
        "TalkStarted は DisplayEndAt に先行する: {signals:?}"
    );
}

/// ⑵ Idempotency 契約: `DisplayEndAt` は `max(at + duration)` が**既知最大を超えたときのみ**
/// 送出され、受信列は単調増加になる（重複・順不同の入力に対して max 集約が成り立つ）。
#[test]
fn display_end_is_monotonic_max_of_at_plus_duration() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx);

    // 終端が 0.5 → 2.0 → （後退 1.0）→ （同値 2.0）→ 3.0 と推移する入力列。
    sink.emit(text_cue(0.0, 0.5)); // end = 0.5（増加）
    sink.emit(text_cue(1.0, 1.0)); // end = 2.0（増加）
    sink.emit(text_cue(0.5, 0.5)); // end = 1.0（後退＝送出しない）
    sink.emit(text_cue(2.0, 0.0)); // end = 2.0（同値＝送出しない）
    sink.emit(text_cue(2.0, 1.0)); // end = 3.0（増加）

    let ends = ends(&rx);

    assert_eq!(
        ends,
        vec![0.5, 2.0, 3.0],
        "既知最大の更新時のみ送出され、受信列は単調増加になる"
    );
    assert!(
        ends.windows(2).all(|w| w[0] < w[1]),
        "受信列は狭義単調増加（重複・後退値は送出されない）: {ends:?}"
    );
}

/// ⑶ Requirement 4.1: 占有区間の終端は**待機を含む**。`Wait` cue の `duration` が
/// horizon へ算入され、最後の文字が現れた時刻ではなく台本の占有終端が届く。
///
/// 弁別: 実装が `duration` を落として `at` のみを horizon にすると最終値が 0.5 になり本 assert が落ちる。
#[test]
fn wait_cue_duration_is_included_in_occupancy_horizon() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx);

    // 文字 @0.0（D=0.5）→ 待機 @0.5（D=1.25）。占有終端 = 0.5 + 1.25 = 1.75。
    sink.emit(text_cue(0.0, 0.5));
    sink.emit(wait_cue(0.5, 1.25));

    let ends = ends(&rx);

    assert_eq!(
        ends.last().copied(),
        Some(1.75),
        "待機 cue の duration が占有終端へ算入される（最後の文字の時刻 0.5 ではない）: {ends:?}"
    );
}

/// ⑶-補: dola の占有 horizon 定義（`to_talk_schedule` の `max(start_time + clamp(duration))`）と
/// 同一値になることを、実 cue 列（Wait 含む）を流して確認する。
#[test]
fn horizon_matches_dola_occupancy_definition() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx);

    let cues = vec![
        cue(0.0, 0.0, CueCommand::ClearAll),
        text_cue(0.0, 0.25),
        wait_cue(0.25, 0.1),
        text_cue(0.35, 0.4),
    ];
    let expected = cues
        .iter()
        .map(|c| c.at + c.duration)
        .fold(0.0_f64, f64::max);

    for c in cues {
        sink.emit(c);
    }

    let ends = ends(&rx);

    assert_eq!(
        ends.last().copied(),
        Some(expected),
        "最終 horizon は dola の占有終端 max(at+duration) と一致する: {ends:?}"
    );
}

/// ⑷ 会話境界の自己検出: dispatcher は talk 起動ごとに登録 sink を clone する
/// （`areka-ghost/src/dispatcher.rs:290-294`）。clone は会話境界の状態を持ち越さないため、
/// 2 本目の talk でも `TalkStarted` が改めて出る（Requirement 4.5 の破棄契機）。
#[test]
fn clone_resets_talk_boundary_so_next_talk_re_emits_talk_started() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut registered = BalloonLifecycleSink::new(tx);

    // 1 本目の talk（登録 sink の clone）。
    let mut talk1 = registered.clone();
    talk1.emit(text_cue(0.0, 2.0));
    let talk1_signals = drain(&rx);
    assert_eq!(
        talk1_signals,
        vec![
            TalkLifecycleSignal::TalkStarted,
            TalkLifecycleSignal::DisplayEndAt(2.0),
        ],
        "1 本目の talk: TalkStarted 先行＋占有終端"
    );

    // 2 本目の talk（同じ登録 sink をもう一度 clone）。
    let mut talk2 = registered.clone();
    talk2.emit(text_cue(0.0, 0.5));
    let talk2_signals = drain(&rx);
    assert_eq!(
        talk2_signals,
        vec![
            TalkLifecycleSignal::TalkStarted,
            TalkLifecycleSignal::DisplayEndAt(0.5),
        ],
        "2 本目の talk でも TalkStarted が再送され、horizon は前 talk（2.0）を持ち越さない"
    );

    // 会話境界の状態は clone で必ずリセットされる——emit 済みインスタンスからの clone でも同じ
    // （登録 sink が emit されないという上流の暗黙不変条件に依存しない）。
    let mut talk3 = talk1.clone();
    talk3.emit(text_cue(0.0, 0.25));
    assert_eq!(
        drain(&rx),
        vec![
            TalkLifecycleSignal::TalkStarted,
            TalkLifecycleSignal::DisplayEndAt(0.25),
        ],
        "emit 済みインスタンスからの clone も会話境界をリセットする"
    );

    // 登録 sink 自身は emit していないので、ここまでで余分な信号は出ていない。
    registered.emit(text_cue(0.0, 0.1));
    assert_eq!(
        drain(&rx),
        vec![
            TalkLifecycleSignal::TalkStarted,
            TalkLifecycleSignal::DisplayEndAt(0.1),
        ],
        "登録 sink 自身も未 emit 状態から始まる"
    );
}

/// 送信失敗（受信端切断）は talk を殺さない——非 panic・無音破棄でもない（log-first・
/// `MoveCueSink` と同一規律・Requirement 4.8 の観測可能な縮退）。
///
/// 弁別: 送出結果を `let _ =` で捨てる無音実装は WARN=0 になって落ちる。さらに WARN の件数が
/// 「送出を試みた回数」そのものなので、切断後も horizon 集約が前進し続けている（0.5→1.0 の
/// 2 回目の更新でも送出を試みている）ことを、受信端が無い状況で唯一観測できる形で主張する。
#[test]
fn send_failure_after_receiver_drop_is_logged_and_non_fatal() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx);
    drop(rx);

    let logs = capture_logs(|| {
        sink.emit(text_cue(0.0, 0.5)); // TalkStarted ＋ DisplayEndAt(0.5) の 2 回
        sink.emit(wait_cue(0.5, 0.5)); // DisplayEndAt(1.0)（切断後も horizon は前進する）
    });

    // 到達＝panic しない。
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

/// 非有限な時刻（NaN/±inf）を持つ壊れた cue は horizon を汚さず、記録付きでスキップされる。
///
/// `+inf` を horizon に採ると受信側の `deadline = display_end + timeout` が到達不能になり、
/// タイムアウトが永久に成立しなくなる（Requirement 4.8 と同じ「表示を保持する」側の縮退だが、
/// 無音で起こすと原因が追えない）。NaN が `>` 比較で偶然落ちるだけの実装と、`is_finite` で
/// 明示的に弾く実装とを、`+inf` の有無と WARN 件数の双方で弁別する。
#[test]
fn non_finite_cue_times_are_skipped_with_a_record() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx);

    let logs = capture_logs(|| {
        sink.emit(text_cue(0.0, 1.0)); // end = 1.0
        sink.emit(text_cue(f64::NAN, 0.0)); // end = NaN（スキップ）
        sink.emit(text_cue(f64::INFINITY, 0.0)); // end = +inf（スキップ＝`>` 比較では落ちない）
        sink.emit(text_cue(1.0, 0.5)); // end = 1.5（増加）
    });

    let got = ends(&rx);
    assert_eq!(
        got,
        vec![1.0, 1.5],
        "非有限な終端は送出されず、有限な最大のみが単調に届く: {got:?}"
    );
    assert_eq!(
        count_level(&logs, "WARN"),
        2,
        "非有限 2 件（NaN・+inf）はいずれも WARN で記録される（無音スキップでない・log-first）: {logs:?}"
    );
}

/// 占有終端が 0.0 の talk（瞬時 cue のみ）でも `DisplayEndAt(0.0)` が届く——
/// 「信号が一度も届かない」（Requirement 4.8 の保持側縮退）と区別できる形にする。
#[test]
fn instantaneous_talk_still_reports_zero_horizon() {
    let (tx, rx) = channel::<TalkLifecycleSignal>();
    let mut sink = BalloonLifecycleSink::new(tx);

    sink.emit(cue(0.0, 0.0, CueCommand::ClearAll));

    assert_eq!(
        drain(&rx),
        vec![
            TalkLifecycleSignal::TalkStarted,
            TalkLifecycleSignal::DisplayEndAt(0.0),
        ],
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
    let sink = BalloonLifecycleSink::new(tx);

    assert_eq!(
        rx.try_recv(),
        Err(TryRecvError::Empty),
        "構築だけでは信号は出ない"
    );

    // dispatcher が talk 起動ごとに行う複製（clone_box 相当）を 2 回踏んでも、emit が無い限り無音。
    let clone1 = sink.clone();
    let _clone2 = clone1.clone();
    assert_eq!(
        rx.try_recv(),
        Err(TryRecvError::Empty),
        "複製だけでは信号は出ない（信号の源は emit のみ）"
    );
}
