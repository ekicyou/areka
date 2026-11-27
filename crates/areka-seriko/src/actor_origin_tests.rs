//! 置き直しの知らせ: 受け口が送る物と、受け口 `handle_message` に届いた知らせが口パクを動かす事。
//!
//! 面 0 の一番上に `talk,3` の口（601 → 40ms で 602 → 40ms で `-1`）。文字は 1 字 50 ms で、9 文字が
//! 1000・1050・…・1400 に現れる（区切りの字は 1100・1250・1400）。刻みと合図と知らせには時刻を書いて
//! 渡す。

use super::test_support::*;
use super::*;
use crate::looper::tests::{always_fire, cfg};
use crate::test_support::table_of;
use dola::cue::{CueSink, OriginNotice};
use dola::time::testing::{at_ms, ms};

const MOUTH_FACE: &str = "surface0\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,602,40,0,0\n\
    animation0.pattern2,overlay,-1,40,0,0\n}\n\
    surface601\n{\n}\nsurface602\n{\n}\n";

const MOUTH: u32 = 0;

fn rig() -> ActorRig {
    ActorRig::with_config(cfg(table_of(MOUTH_FACE), always_fire()))
}

/// 台本の起点が `previous_ms` から `origin_ms` へ、時刻 `effective_ms` に置き直された知らせ。
fn moved(effective_ms: i64, previous_ms: i64, origin_ms: i64) -> OriginNotice {
    OriginNotice {
        effective_at: at_ms(effective_ms),
        origin: at_ms(origin_ms),
        previous: Some(at_ms(previous_ms)),
    }
}

/// 面 0 を出し、起点 1000 の台本の 9 文字を届ける。
fn hear_nine(rig: &mut ActorRig) {
    rig.hear_fired(at_ms(1000), emote_cue(ms(0), "0", "0"));
    rig.hear_fired(at_ms(1000), clear_all());
    rig.hear_fired(at_ms(1000), text(ms(0), "0", "あいうえおかきくけ"));
}

/// 受け口は、前の起点の在る知らせだけを、合図と同じ口へ届いた順に積む。最初の起点の知らせは送らない。
#[test]
fn sink_queues_only_notices_with_a_previous_origin_in_order_with_cues() {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut sink = SerikoSink::new(tx);
    CueSink::origin_set(&mut sink, OriginNotice::first(at_ms(1000)));
    assert!(rx.try_recv().is_err(), "最初の起点の知らせは送らない");

    let notice = moved(1130, 1000, 1200);
    CueSink::emit(&mut sink, fired(at_ms(1000), clear_all()));
    CueSink::origin_set(&mut sink, notice);
    CueSink::emit(&mut sink, fired(at_ms(1300), clear_all()));
    assert!(matches!(rx.try_recv(), Ok(SerikoMsg::Cue(_))));
    assert!(matches!(rx.try_recv(), Ok(SerikoMsg::OriginSet(n)) if n == notice));
    assert!(matches!(rx.try_recv(), Ok(SerikoMsg::Cue(_))));
    assert!(rx.try_recv().is_err());
}

/// 後ろへ 200 ms の置き直し（効く時刻 1130）が届くと、効く時刻より後の区切りの字（1250・1400）は
/// 1450・1600 に動き、口はそこで動く。効く時刻より前に始まった口（1100）はそのまま。
#[test]
fn notice_moves_the_mouth_boundaries_after_the_effective_time() {
    let mut rig = rig();
    rig.tick(at_ms(1000));
    hear_nine(&mut rig);
    assert_eq!(frames(&rig.tick(at_ms(1120)), MOUTH), [("0", Some(601))]);

    assert!(
        rig.send(SerikoMsg::OriginSet(moved(1130, 1000, 1200)))
            .is_empty()
    );
    assert_eq!(
        frames(&rig.tick(at_ms(1200)), MOUTH),
        [("0", None)],
        "1100 の口の終わり"
    );
    assert!(rig.tick(at_ms(1260)).is_empty(), "元の 1250 では動かない");
    assert!(rig.tick(at_ms(1440)).is_empty());
    assert_eq!(frames(&rig.tick(at_ms(1460)), MOUTH), [("0", Some(601))]);
    assert_eq!(frames(&rig.tick(at_ms(1540)), MOUTH), [("0", None)]);
    assert_eq!(frames(&rig.tick(at_ms(1610)), MOUTH), [("0", Some(601))]);
}

/// 同じ合図と知らせを、刻みとの順を変えて与えても、刻みを済ませる前に届いていれば同じ指令が出る。
/// 前へ 100 ms の置き直し（効く時刻 1130）で、区切りの字は 1150（1100 の口の再生中なので見送る）と
/// 1300 に動く。
#[test]
fn same_cues_and_notice_give_the_same_commands_however_they_arrive() {
    let ticks = [1120, 1200, 1260, 1310, 1400, 1500];
    let notice = moved(1130, 1000, 900);

    let mut on_time = rig();
    on_time.tick(at_ms(1000));
    hear_nine(&mut on_time);
    on_time.tick(at_ms(ticks[0]));
    on_time.send(SerikoMsg::OriginSet(notice));
    for now in &ticks[1..] {
        on_time.tick(at_ms(*now));
    }

    let mut early = rig();
    early.tick(at_ms(1000));
    hear_nine(&mut early);
    early.send(SerikoMsg::OriginSet(notice));
    for now in ticks {
        early.tick(at_ms(now));
    }

    assert_eq!(early.all(), on_time.all());
    let all = on_time.all();
    let starts = frames(&all, MOUTH)
        .iter()
        .filter(|&&(_, cell)| cell == Some(601))
        .count();
    assert_eq!(starts, 2, "1100 と 1300 の 2 回");
}
