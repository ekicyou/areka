// =============================================================================
// BalloonLifecycleSink の知らせの檻
//
//   ⑴ 配送から受け取った番号が開始の合図に載る（受け取っていなければ無し）
//   ⑵ 落ちるときの終わりの合図が、注入した時計の読みで 1 回だけ出る
//      （合図を送っていない複製は出さない）
//   ⑶ `\![set,balloontimeout,時間]` の時間の欄の読み（正・0・負・省略・空・読めない値）と、
//      他の `\![set,…]` を拾わないこと・拾うたびの 1 行の記録
//
// 時刻はすべて注入した時計で進める（実時間の待機なし）。
// =============================================================================

use super::*;
use crate::consumer_ledger::test_support::carrier_cue;
use crate::talk_clock::{controllable_clock, set_now};
use areka_ghost::sink::BootCueSink;
use areka_sakura::TalkId;
use dola::cue::{ActorKey, CueCommand, TalkCue};
use dola::time::testing::{at_ms, ms};
use dola::{TimePoint, TimeSpan};
use log_capture_kit::{LineFormat, capture_lines};
use std::sync::mpsc::{Receiver, channel};

fn text_cue(at: TimeSpan, duration: TimeSpan) -> TalkCue {
    TalkCue::scheduled(
        TimePoint::ZERO,
        at,
        ActorKey::from("0"),
        CueCommand::Text("あ".into()),
        duration,
    )
}

fn drain(rx: &Receiver<TalkLifecycleSignal>) -> Vec<TalkLifecycleSignal> {
    rx.try_iter().collect()
}

/// 配送と同じ形で 1 トークぶんの受け口を作る（登録の原本を複製し、番号を渡す）。
fn talk_sink(registered: &BalloonLifecycleSink, talk_id: Option<TalkId>) -> Box<dyn BootCueSink> {
    let mut sink = registered.clone_box();
    if let Some(id) = talk_id {
        sink.begin_talk(id, areka_sakura::contract::ScriptOrigin::GHOST);
    }
    sink
}

fn timeouts(signals: &[TalkLifecycleSignal]) -> Vec<TalkTimeout> {
    signals
        .iter()
        .filter_map(|s| match s {
            TalkLifecycleSignal::BalloonTimeout { timeout, .. } => Some(*timeout),
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
    first.emit(text_cue(ms(0), ms(500)));
    assert_eq!(
        drain(&rx).first(),
        Some(&TalkLifecycleSignal::TalkStarted {
            at: TimePoint::ZERO,
            talk_id: Some(TalkId(7))
        }),
        "番号 7 の複製の開始の合図に 7 が載る"
    );
    drop(first);
    drain(&rx);

    let mut second = talk_sink(&registered, Some(TalkId(8)));
    second.emit(text_cue(ms(0), ms(500)));
    assert_eq!(
        drain(&rx).first(),
        Some(&TalkLifecycleSignal::TalkStarted {
            at: TimePoint::ZERO,
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
    sink.emit(text_cue(ms(0), ms(500)));
    assert_eq!(
        drain(&rx).first(),
        Some(&TalkLifecycleSignal::TalkStarted {
            at: TimePoint::ZERO,
            talk_id: None
        })
    );
}

// -----------------------------------------------------------------------------
// ⑵ 終わりの合図
// -----------------------------------------------------------------------------

/// 落ちた瞬間の時計の読み（注入した時計の値そのもの・丸めない）で、終わりの合図が 1 回だけ出て、
/// そのトークの合図の最後に並ぶ。
#[test]
fn talk_ended_carries_the_injected_clock_once_when_dropped() {
    let (tx, rx) = channel();
    let (now, clock) = controllable_clock();
    let registered = BalloonLifecycleSink::new(tx, clock);

    set_now(&now, at_ms(100_000));
    let mut sink = talk_sink(&registered, Some(TalkId(3)));
    sink.emit(text_cue(ms(0), ms(2_000)));
    set_now(&now, at_ms(101_234));
    drop(sink);

    let signals = drain(&rx);
    let ended: Vec<&TalkLifecycleSignal> = signals
        .iter()
        .filter(|s| matches!(s, TalkLifecycleSignal::TalkEnded { .. }))
        .collect();
    assert_eq!(ended.len(), 1, "終わりの合図は 1 回だけ: {signals:?}");
    assert_eq!(
        signals.last(),
        Some(&TalkLifecycleSignal::TalkEnded { at: at_ms(101_234) }),
        "落ちた瞬間の時計の読み 101.234 s を運ぶ（フレームに丸めない）"
    );
}

/// 合図を 1 つも送っていない複製（cue が来なかったトーク）と、登録の原本は、落ちても何も送らない。
#[test]
fn sink_that_sent_nothing_sends_no_talk_ended() {
    let (tx, rx) = channel();
    let (now, clock) = controllable_clock();
    set_now(&now, at_ms(50_000));
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

// -----------------------------------------------------------------------------
// ⑶ 待ち時間の指定
// -----------------------------------------------------------------------------

/// 時間の欄の読み。
#[test]
fn parse_balloon_timeout_reads_every_shape_of_the_value() {
    let cases: &[(Option<&str>, TalkTimeout, bool)] = &[
        (Some("3000"), TalkTimeout::After(ms(3_000)), false),
        (Some("1"), TalkTimeout::After(ms(1)), false),
        (Some(" 250 "), TalkTimeout::After(ms(250)), false),
        (Some("0"), TalkTimeout::Never, false),
        (Some("-1"), TalkTimeout::Never, false),
        (Some("-5000"), TalkTimeout::Never, false),
        (None, TalkTimeout::Default, false),
        (Some(""), TalkTimeout::Default, false),
        (Some("abc"), TalkTimeout::Default, true),
        (Some("1.5"), TalkTimeout::Default, true),
        (Some("99999999999999999999999"), TalkTimeout::Default, true),
        // i64 には収まるが長さの型（100ns の数が i64）に収まらないミリ秒。
        (Some("1000000000000000"), TalkTimeout::Default, true),
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
            TalkTimeout::After(ms(3_000)),
            TalkTimeout::Never,
            TalkTimeout::Default
        ],
        "balloontimeout の 3 つだけが台本の順に届く"
    );
}

/// 拾うたびに 1 行記録する。読めた値は INFO、読めなかった値は WARN でその値を載せる。
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
