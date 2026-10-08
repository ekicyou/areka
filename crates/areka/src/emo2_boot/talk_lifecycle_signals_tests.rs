// =============================================================================
// BalloonLifecycleSink の 3 つの知らせの檻（areka-P0-balloon-lifecycle-events task 4.1・
// 要件 5.2 / 5.6 / 7.5 / 9.1〜9.4 / 9.8）
//
//   ⑴ 配送から受け取った番号が開始の合図に載る（受け取っていなければ無し）
//   ⑵ 落ちるときの終わりの合図が、注入した時計の talk 相対秒で 1 回だけ出る
//      （合図を送っていない複製は出さない・時計に起点が無ければ時刻は無しで、記録が残る）
//   ⑶ `\![set,balloontimeout,時間]` の時間の欄の読み（正・0・負・省略・空・読めない値）と、
//      他の `\![set,…]` を拾わないこと・拾うたびの 1 行の記録
//
// 時刻はすべて注入した時計で進める（実時間の待機なし・要件 8.2）。
// =============================================================================

use super::*;
use crate::emo2_boot::talk_clock::TalkClock;
use areka_ghost::sink::BootCueSink;
use areka_sakura::TalkId;
use dola::cue::{ActorKey, CueCommand, TalkCue};
use log_capture_kit::{LineFormat, capture_lines};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};

/// 壁時刻を書き換えられる時計と、それを読む `TalkClock`。
fn controllable_clock() -> (Arc<Mutex<f64>>, TalkClock) {
    let now = Arc::new(Mutex::new(0.0f64));
    let now_for_clock = Arc::clone(&now);
    let clock = TalkClock::new(Arc::new(move || {
        *now_for_clock.lock().expect("test clock mutex poisoned")
    }));
    (now, clock)
}

fn set_now(now: &Arc<Mutex<f64>>, wall: f64) {
    *now.lock().expect("test clock mutex poisoned") = wall;
}

fn text_cue(at: f64, duration: f64) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from("0"),
        command: CueCommand::Text("あ".into()),
        duration,
    }
}

/// `\![name,tokens...]` の運び手の cue。
fn carrier_cue(name: &str, tokens: &[&str]) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from("0"),
        command: CueCommand::command_carrier(name, tokens.iter().map(|s| s.to_string()).collect()),
        duration: 0.0,
    }
}

fn drain(rx: &Receiver<TalkLifecycleSignal>) -> Vec<TalkLifecycleSignal> {
    rx.try_iter().collect()
}

/// 配送と同じ形で 1 トークぶんの受け口を作る（登録の原本を複製し、番号を渡す）。
fn talk_sink(registered: &BalloonLifecycleSink, talk_id: Option<TalkId>) -> Box<dyn BootCueSink> {
    let mut sink = registered.clone_box();
    if let Some(id) = talk_id {
        sink.begin_talk(id);
    }
    sink
}

fn timeouts(signals: &[TalkLifecycleSignal]) -> Vec<TalkTimeout> {
    signals
        .iter()
        .filter_map(|s| match s {
            TalkLifecycleSignal::BalloonTimeout(t) => Some(*t),
            _ => None,
        })
        .collect()
}

// -----------------------------------------------------------------------------
// ⑴ 番号
// -----------------------------------------------------------------------------

/// 配送が `begin_talk` で渡した番号が、そのトークの開始の合図に載る。複製ごとに番号は入れ替わる。
#[test]
fn begin_talk_number_rides_on_talk_started() {
    let (tx, rx) = channel();
    let (_now, clock) = controllable_clock();
    let registered = BalloonLifecycleSink::new(tx, clock);

    let mut first = talk_sink(&registered, Some(TalkId(7)));
    first.emit(text_cue(0.0, 0.5));
    assert_eq!(
        drain(&rx).first(),
        Some(&TalkLifecycleSignal::TalkStarted {
            talk_id: Some(TalkId(7))
        }),
        "番号 7 の複製の開始の合図に 7 が載る"
    );
    drop(first);
    drain(&rx);

    let mut second = talk_sink(&registered, Some(TalkId(8)));
    second.emit(text_cue(0.0, 0.5));
    assert_eq!(
        drain(&rx).first(),
        Some(&TalkLifecycleSignal::TalkStarted {
            talk_id: Some(TalkId(8))
        }),
        "次の複製は前の番号を持ち越さず、自分の番号を載せる"
    );
}

/// 番号を受け取らなかった複製（配送を通らない再生）の開始の合図は番号が無し。
#[test]
fn talk_started_without_begin_talk_has_no_number() {
    let (tx, rx) = channel();
    let (_now, clock) = controllable_clock();
    let registered = BalloonLifecycleSink::new(tx, clock);

    let mut sink = talk_sink(&registered, None);
    sink.emit(text_cue(0.0, 0.5));
    assert_eq!(
        drain(&rx).first(),
        Some(&TalkLifecycleSignal::TalkStarted { talk_id: None })
    );
}

// -----------------------------------------------------------------------------
// ⑵ 終わりの合図
// -----------------------------------------------------------------------------

/// 落ちた瞬間の talk 相対秒（注入した時計の値そのもの・丸めない）で、終わりの合図が 1 回だけ出て、
/// そのトークの合図の最後に並ぶ（要件 5.2）。
#[test]
fn talk_ended_carries_the_injected_clock_once_when_dropped() {
    let (tx, rx) = channel();
    let (now, clock) = controllable_clock();
    let registered = BalloonLifecycleSink::new(tx, clock.clone());

    // 文字の受け口（ClockedTextSink）が起点を決めた形: 壁 100.0 に at 0.0 が届いた → 起点 100.0。
    set_now(&now, 100.0);
    clock.observe_cue(0.0);

    let mut sink = talk_sink(&registered, Some(TalkId(3)));
    sink.emit(text_cue(0.0, 2.0));
    // 台本の途中で止まった: 壁 101.234 で受け口が落ちる。
    set_now(&now, 101.234);
    drop(sink);

    let signals = drain(&rx);
    let ended: Vec<&TalkLifecycleSignal> = signals
        .iter()
        .filter(|s| matches!(s, TalkLifecycleSignal::TalkEnded { .. }))
        .collect();
    assert_eq!(ended.len(), 1, "終わりの合図は 1 回だけ: {signals:?}");
    let Some(TalkLifecycleSignal::TalkEnded { at: Some(at) }) = signals.last() else {
        panic!("終わりの合図が最後に時刻つきで並ぶ: {signals:?}");
    };
    assert!(
        (at - 1.234).abs() < 1e-9,
        "落ちた瞬間の talk 相対秒 1.234 を運ぶ（フレームに丸めない）: {at}"
    );
}

/// 合図を 1 つも送っていない複製（cue が来なかったトーク）と、登録の原本は、落ちても何も送らない。
#[test]
fn sink_that_sent_nothing_sends_no_talk_ended() {
    let (tx, rx) = channel();
    let (now, clock) = controllable_clock();
    set_now(&now, 50.0);
    clock.observe_cue(0.0);
    let registered = BalloonLifecycleSink::new(tx, clock);

    let silent = talk_sink(&registered, Some(TalkId(1)));
    drop(silent);
    drop(registered);
    assert_eq!(
        drain(&rx),
        vec![],
        "合図を送っていない受け口は終わりの合図も送らない"
    );
}

/// 時計に起点が無ければ、終わりの合図は時刻無しで出て、時刻が取れなかったことが記録される
/// （要件 5.6）。
#[test]
fn talk_ended_without_epoch_has_no_time_and_is_logged() {
    let (tx, rx) = channel();
    let (_now, clock) = controllable_clock();
    let registered = BalloonLifecycleSink::new(tx, clock);
    let mut sink = talk_sink(&registered, Some(TalkId(2)));
    sink.emit(text_cue(0.0, 0.5));

    let ((), logs) = capture_lines(LineFormat::LevelTargetFields, || drop(sink));

    assert_eq!(
        drain(&rx).last(),
        Some(&TalkLifecycleSignal::TalkEnded { at: None }),
        "起点の無い時計では時刻無し"
    );
    assert_eq!(
        logs.iter().filter(|l| l.contains("level=WARN")).count(),
        1,
        "止まった時刻が取れなかったことを 1 行 WARN で残す: {logs:?}"
    );
}

/// 時計の口: 起点があれば今の壁時刻から talk 相対秒を読み、無ければ無し。
#[test]
fn talk_clock_now_talk_time_reads_the_injected_clock() {
    let (now, clock) = controllable_clock();
    assert_eq!(clock.now_talk_time(), None, "起点が無ければ無し");
    set_now(&now, 10.0);
    clock.observe_cue(0.5);
    set_now(&now, 12.0);
    let t = clock.now_talk_time().expect("起点がある");
    assert!((t - 2.5).abs() < 1e-9, "12.0 − (10.0 − 0.5) = 2.5: {t}");
}

// -----------------------------------------------------------------------------
// ⑶ 待ち時間の指定
// -----------------------------------------------------------------------------

/// 時間の欄の読み（要件 9.1〜9.4）。
#[test]
fn parse_balloon_timeout_reads_every_shape_of_the_value() {
    let cases: &[(Option<&str>, TalkTimeout, bool)] = &[
        (Some("3000"), TalkTimeout::Millis(3000), false),
        (Some("1"), TalkTimeout::Millis(1), false),
        (Some(" 250 "), TalkTimeout::Millis(250), false),
        (Some("0"), TalkTimeout::Never, false),
        (Some("-1"), TalkTimeout::Never, false),
        (Some("-5000"), TalkTimeout::Never, false),
        (None, TalkTimeout::Default, false),
        (Some(""), TalkTimeout::Default, false),
        (Some("abc"), TalkTimeout::Default, true),
        (Some("1.5"), TalkTimeout::Default, true),
        (Some("99999999999999999999999"), TalkTimeout::Default, true),
    ];
    for (value, timeout, unreadable) in cases {
        assert_eq!(
            parse_balloon_timeout(*value),
            ParsedTimeout {
                timeout: *timeout,
                unreadable: *unreadable
            },
            "時間の欄 {value:?}"
        );
    }
}

/// `\![set,balloontimeout,…]` の cue だけを拾い、到着順に待ち時間の指定の合図を送る。
/// 他の `\![set,…]`・第 1 引数の無い `\![set]`・別の名前は拾わない。
#[test]
fn sink_picks_only_set_balloontimeout() {
    let (tx, rx) = channel();
    let (_now, clock) = controllable_clock();
    let registered = BalloonLifecycleSink::new(tx, clock);
    let mut sink = talk_sink(&registered, Some(TalkId(4)));

    sink.emit(carrier_cue("set", &["zorder", "1", "0"]));
    sink.emit(carrier_cue("set", &["windowstate", "stayontop"]));
    sink.emit(carrier_cue("set", &[]));
    sink.emit(carrier_cue("reset", &["balloontimeout"]));
    sink.emit(carrier_cue("set", &["balloontimeout", "3000"]));
    sink.emit(carrier_cue("set", &["balloontimeout", "0"]));
    sink.emit(carrier_cue("set", &["balloontimeout"]));

    assert_eq!(
        timeouts(&drain(&rx)),
        vec![
            TalkTimeout::Millis(3000),
            TalkTimeout::Never,
            TalkTimeout::Default
        ],
        "balloontimeout の 3 つだけが台本の順に届く"
    );
}

/// 拾うたびに 1 行記録する。読めた値は INFO、読めなかった値は WARN でその値を載せる（要件 7.5）。
#[test]
fn each_pick_is_logged_in_one_line_with_the_unreadable_value() {
    let (tx, _rx) = channel();
    let (_now, clock) = controllable_clock();
    let registered = BalloonLifecycleSink::new(tx, clock);
    let mut sink = talk_sink(&registered, Some(TalkId(5)));

    let ((), logs) = capture_lines(LineFormat::LevelTargetFields, || {
        sink.emit(carrier_cue("set", &["balloontimeout", "3000"]));
        sink.emit(carrier_cue("set", &["balloontimeout", "abc"]));
        sink.emit(carrier_cue("set", &["zorder", "1", "0"]));
    });

    let picks: Vec<&String> = logs
        .iter()
        .filter(|l| l.contains("balloon_timeout_set"))
        .collect();
    assert_eq!(picks.len(), 2, "拾った 2 回だけ 1 行ずつ: {logs:?}");
    assert!(
        picks[0].contains("level=INFO") && picks[0].contains("3000"),
        "読めた値は INFO で採った値つき: {picks:?}"
    );
    assert!(
        picks[1].contains("level=WARN") && picks[1].contains("abc"),
        "読めなかった値は WARN でその値つき: {picks:?}"
    );
}
