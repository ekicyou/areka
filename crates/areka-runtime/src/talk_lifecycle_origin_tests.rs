// =============================================================================
// BalloonLifecycleSink が知らせに付ける絶対の時刻と、置き直しの知らせの送り方の檻
//
//   ⑴ 合図から作る知らせは、その合図の発火時刻を運び、表示の終わりは発火時刻＋長さ
//   ⑵ 置き直しの知らせは、前の起点が在り、そのトークの最初の合図より後の物だけを送る
//   ⑶ 覚えている表示の終わりの最大も、同じ決まりで動く
// =============================================================================

use super::*;
use crate::frame::test_support::zero_clock;
use areka_ghost::sink::BootCueSink;
use dola::cue::{ActorKey, CueCommand, OriginNotice, TalkCue};
use dola::time::testing::{at_ms, delta_ms, ms};
use dola::{TimePoint, TimeSpan};
use std::sync::mpsc::{Receiver, channel};

fn text_at(origin: TimePoint, at: TimeSpan, duration: TimeSpan) -> TalkCue {
    TalkCue::scheduled(
        origin,
        at,
        ActorKey::from("0"),
        CueCommand::Text("あ".into()),
        duration,
    )
}

fn registered() -> (BalloonLifecycleSink, Receiver<TalkLifecycleSignal>) {
    let (tx, rx) = channel();
    (BalloonLifecycleSink::new(tx, zero_clock()), rx)
}

fn drain(rx: &Receiver<TalkLifecycleSignal>) -> Vec<TalkLifecycleSignal> {
    rx.try_iter().collect()
}

fn moved(effective: i64, origin: i64, previous: i64) -> OriginNotice {
    OriginNotice {
        effective_at: at_ms(effective),
        origin: at_ms(origin),
        previous: Some(at_ms(previous)),
    }
}

fn ends(signals: &[TalkLifecycleSignal]) -> Vec<TimePoint> {
    signals
        .iter()
        .filter_map(|s| match s {
            TalkLifecycleSignal::DisplayEndAt { end, .. } => Some(*end),
            _ => None,
        })
        .collect()
}

/// ⑴ 開始・表示の終わり・待ちの指定・選択肢の知らせが、合図の発火時刻（起点＋台本の時刻）を運ぶ。
#[test]
fn cue_notices_carry_the_fire_time_and_the_display_end_is_fire_time_plus_duration() {
    let (registered, rx) = registered();
    let mut sink = registered.clone_box();
    let origin = at_ms(10_000);

    sink.emit(text_at(origin, ms(500), ms(250)));
    sink.emit(TalkCue::scheduled(
        origin,
        ms(800),
        ActorKey::from("0"),
        CueCommand::Choice {
            id: "OnA".into(),
            text: "A".into(),
            references: vec![],
        },
        TimeSpan::ZERO,
    ));

    assert_eq!(
        drain(&rx),
        vec![
            TalkLifecycleSignal::TalkStarted {
                at: at_ms(10_500),
                talk_id: None
            },
            TalkLifecycleSignal::DisplayEndAt {
                at: at_ms(10_500),
                end: at_ms(10_750)
            },
            TalkLifecycleSignal::ChoiceShown { at: at_ms(10_800) },
            TalkLifecycleSignal::DisplayEndAt {
                at: at_ms(10_800),
                end: at_ms(10_800)
            },
        ]
    );
}

/// ⑴ 待ちの指定の知らせも、その合図の発火時刻を運ぶ。
#[test]
fn balloon_timeout_notice_carries_the_fire_time() {
    let (registered, rx) = registered();
    let mut sink = registered.clone_box();

    sink.emit(TalkCue::scheduled(
        at_ms(2_000),
        ms(300),
        ActorKey::from("0"),
        CueCommand::command_carrier("set", vec!["balloontimeout".into(), "3000".into()]),
        TimeSpan::ZERO,
    ));

    assert!(drain(&rx).contains(&TalkLifecycleSignal::BalloonTimeout {
        at: at_ms(2_300),
        timeout: TalkTimeout::After(ms(3_000)),
    }));
}

/// ⑵ 最初の起点の知らせ（前の起点が無い）は、合図の前でも後でも送らない。
#[test]
fn first_origin_notice_is_never_sent() {
    let (registered, rx) = registered();
    let mut sink = registered.clone_box();

    sink.origin_set(OriginNotice::first(at_ms(1_000)));
    sink.emit(text_at(at_ms(1_000), ms(0), ms(100)));
    sink.origin_set(OriginNotice::first(at_ms(5_000)));

    assert!(
        !drain(&rx)
            .iter()
            .any(|s| matches!(s, TalkLifecycleSignal::OriginMoved(_))),
        "最初の起点の知らせは受け手が何もしない物なので送らない"
    );
}

/// ⑵ そのトークの最初の合図より前の置き直しの知らせは送らない（受け手はまだそのトークの物を
/// 何も持っていない）。開始の知らせもまだ出ない。
#[test]
fn re_origin_before_the_first_cue_is_not_sent() {
    let (registered, rx) = registered();
    let mut sink = registered.clone_box();

    sink.origin_set(moved(500, 3_000, 2_000));

    assert_eq!(drain(&rx), vec![]);
}

/// ⑵ 最初の合図より後で前の起点の在る置き直しの知らせは、そのまま `OriginMoved` として送る。
#[test]
fn re_origin_after_the_first_cue_is_sent_as_origin_moved() {
    let (registered, rx) = registered();
    let mut sink = registered.clone_box();
    sink.emit(text_at(TimePoint::ZERO, ms(0), ms(1_000)));
    drain(&rx);

    let notice = moved(400, 500, 0);
    sink.origin_set(notice);

    assert_eq!(drain(&rx), vec![TalkLifecycleSignal::OriginMoved(notice)]);
}

/// ⑶ 前へ動かす置き直しの後: 覚えている最大（1000）は効く時刻（200）より後なので 500 ずつ前へ
/// 動いて 500 になり、置き直しの後の合図の終わり（600）が送られる。動かさなければ 600 は 1000 を
/// 越えないので送られない。
#[test]
fn remembered_maximum_moves_earlier_so_a_later_cue_end_is_sent() {
    let (registered, rx) = registered();
    let mut sink = registered.clone_box();
    sink.emit(text_at(TimePoint::ZERO, ms(0), ms(1_000)));
    sink.origin_set(moved(200, -500, 0));
    drain(&rx);

    sink.emit(text_at(at_ms(-500), ms(800), ms(300)));

    assert_eq!(ends(&drain(&rx)), vec![at_ms(600)]);
}

/// ⑶ 後ろへ動かす置き直しの後: 覚えている最大（1000）は 1500 へ動くので、動いた後の合図の
/// 終わり 1200 は最大を越えず、送られない。1600 は送られる。
#[test]
fn remembered_maximum_moves_later_so_a_smaller_end_is_not_sent() {
    let (registered, rx) = registered();
    let mut sink = registered.clone_box();
    sink.emit(text_at(TimePoint::ZERO, ms(0), ms(1_000)));
    sink.origin_set(moved(300, 500, 0));
    drain(&rx);

    sink.emit(text_at(at_ms(500), ms(400), ms(300)));
    sink.emit(text_at(at_ms(500), ms(700), ms(400)));

    assert_eq!(ends(&drain(&rx)), vec![at_ms(1_600)]);
}

/// ⑶ 前へ大きく動いても、覚えている最大は効く時刻より前にはならない（効く時刻に寄る）。
/// 効く時刻以前の最大は動かない。
#[test]
fn remembered_maximum_never_moves_before_the_effective_time() {
    let (registered, rx) = registered();
    let mut sink = registered.clone_box();
    sink.emit(text_at(TimePoint::ZERO, ms(0), ms(1_000)));
    sink.origin_set(moved(700, -5_000, 0));
    drain(&rx);

    // 最大は 700 に寄っている: 終わり 700 は越えず、701 ミリ秒の終わりは越える。
    sink.emit(text_at(at_ms(-5_000), ms(5_600), ms(100)));
    sink.emit(text_at(at_ms(-5_000), ms(5_600), ms(101)));
    assert_eq!(ends(&drain(&rx)), vec![at_ms(701)]);

    // 効く時刻（2000）が最大（701）より後の置き直しでは、最大は動かない。
    sink.origin_set(moved(2_000, -4_000, -5_000));
    drain(&rx);
    sink.emit(text_at(at_ms(-4_000), ms(4_702), ms(0)));
    assert_eq!(ends(&drain(&rx)), vec![at_ms(702)]);
}

/// 受け手と受け口が共有する動かし方: 効く時刻以前は動かさず、後なら動いた長さだけ動かし、
/// 効く時刻より前になるなら効く時刻にする。
#[test]
fn moved_time_follows_the_origin_notice_rule() {
    let notice = moved(1_000, 1_500, 2_000);
    assert_eq!(notice.delta(), Some(delta_ms(-500)));
    assert_eq!(
        moved_time(&notice, at_ms(1_000)),
        at_ms(1_000),
        "効く時刻ちょうどは動かない"
    );
    assert_eq!(moved_time(&notice, at_ms(400)), at_ms(400));
    assert_eq!(
        moved_time(&notice, at_ms(1_300)),
        at_ms(1_000),
        "前へ動いて効く時刻に寄る"
    );
    assert_eq!(moved_time(&notice, at_ms(3_000)), at_ms(2_500));
    let later = moved(1_000, 2_500, 2_000);
    assert_eq!(moved_time(&later, at_ms(1_001)), at_ms(1_501));
    assert_eq!(
        moved_time(&OriginNotice::first(at_ms(0)), at_ms(5_000)),
        at_ms(5_000),
        "最初の起点の知らせでは動かさない"
    );
}
