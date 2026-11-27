//! 部品の `talk` の檻（面の文字の窓を偽の文字の到着から刻みごとに組んで渡す）。

use areka_emo_compose::PatternState;
use dola::time::testing::at_ms;

use super::PartAnim;
use super::tests::{calls, count, overlay};
use super::trigger_tests::Rig;
use crate::test_support::{capture_logs, part_frames, part_playing as playing};

/// `talk` だけを持つ部品は構えるが（記録は構えたときの 1 回）、文字の窓が渡らない間は鳴らない。
#[test]
fn part_with_only_talk_is_armed_once_and_stays_silent() {
    let mut rig = Rig::new(
        "surface0\n{\nelement0,overlay,100,0,0\n}\n\
         surface100\n{\nanimation0.interval,talk,3\n\
         animation0.pattern0,overlay,101,0,0,0\n\
         animation0.pattern1,overlay,-1,50,0,0\n}\n",
        &[],
    );
    let lines = capture_logs(|| {
        for now in [at_ms(1000), at_ms(1016), at_ms(2000), at_ms(3000)] {
            assert!(rig.tick(0, now).is_empty(), "{now}");
            assert_eq!(rig.clock(100, 0), None, "{now}");
        }
    });
    assert_eq!(rig.armed(), vec![100]);
    assert_eq!(
        count(&lines, &["level=DEBUG", "引き金を構えた", "part=100"]),
        1,
        "{lines:#?}"
    );
    assert_eq!(count(&lines, &["を鳴らした"]), 0, "{lines:#?}");
}

/// 一番上 0 が子 100 を置く。100 の `talk,3`（animation 0）は頭のコマで別の部品 200 を見せ、80ms で
/// `-1`。200 の `runonce` は 201 → 500ms で `-1`。
const PART_MOUTH: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nanimation0.interval,talk,3\n\
    animation0.pattern0,overlay,200,0,0,0\n\
    animation0.pattern1,overlay,-1,80,0,0\n}\n\
    surface200\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,201,0,0,0\n\
    animation0.pattern1,overlay,-1,500,0,0\n}\n";

/// 見えている部品の `talk` は、面の文字の数えが区切り（3 文字ごと）を越えた刻みに鳴る。開始は刻みの
/// 時刻でなく区切りの文字が現れた時刻で、同じ刻みのうちに経過の分のコマと、そのコマが見せた子の
/// コマまで出る。乱数は引かない。
#[test]
fn visible_part_talk_fires_at_each_boundary_from_the_glyph_time() {
    let mut rig = Rig::new(PART_MOUTH, &[]);
    let top = PatternState::default();

    for now in [at_ms(1010), at_ms(1090)] {
        assert!(rig.talk_tick(0, now, &top).is_empty(), "{now}");
        assert_eq!(rig.clock(100, 0), None, "{now}: 3 文字目はまだ");
    }

    let p = rig.talk_tick(0, at_ms(1120), &top);
    assert_eq!(
        rig.clock(100, 0),
        playing(at_ms(1100)),
        "開始は 3 文字目が現れた 1100（刻みの 1120 ではない）"
    );
    assert_eq!(part_frames(&p, 100), vec![(0, 200)], "同じ刻みで口のコマ");
    assert_eq!(
        part_frames(&p, 200),
        vec![(0, 201)],
        "口のコマが見せた子も同じ刻みで出る"
    );

    let p = rig.talk_tick(0, at_ms(1190), &top);
    assert!(p.is_empty(), "`-1` で消して終える（経過 90）");
    assert_eq!(rig.clock(100, 0), None);

    rig.talk_tick(0, at_ms(1240), &top);
    assert_eq!(rig.clock(100, 0), None, "5 文字ではまだ");
    let p = rig.talk_tick(0, at_ms(1295), &top);
    assert_eq!(
        rig.clock(100, 0),
        playing(at_ms(1250)),
        "6 文字目が現れた時刻"
    );
    assert_eq!(
        part_frames(&p, 100),
        vec![(0, 200)],
        "遅れた 45ms の分だけ進んだコマを同じ刻みで出す"
    );
    assert_eq!(calls(&rig.probe), 0, "`talk` は部品でも乱数を引かない");
}

/// 一番上 0 は子を置かない（一番上のコマが 100 を指したときだけ 100 が見える）。100 の `talk,3` は
/// 101 → 40ms で `-1`。
const LATE_MOUTH: &str = "surface0\n{\n}\n\
    surface100\n{\nanimation0.interval,talk,3\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,-1,40,0,0\n}\n";

/// 見えていない部品の `talk` は、区切りを越えても始まらない。後から見えた部品は数え直さずに面の
/// 数えを借りる: 見えなかった間に越えた区切りは後から鳴らず、面の数えの次の区切りで鳴る（見えてから
/// 3 文字目ではない）。
#[test]
fn hidden_part_talk_stays_silent_and_joins_the_slot_count_when_it_appears() {
    let mut rig = Rig::new(LATE_MOUTH, &[]);
    let none = PatternState::default();
    let mut shows_100 = PatternState::default();
    shows_100.set(7, overlay(100));

    rig.talk_tick(0, at_ms(1010), &none);
    rig.talk_tick(0, at_ms(1120), &none);
    assert_eq!(
        rig.clock(100, 0),
        None,
        "見えていない部品では始まらない（3 文字目の区切り）"
    );
    assert!(rig.armed().is_empty());

    // 4 文字（1000〜1150）が現れたところで見える。
    rig.talk_tick(0, at_ms(1160), &shows_100);
    assert_eq!(rig.armed(), vec![100]);
    assert_eq!(
        rig.clock(100, 0),
        None,
        "見えなかった間に越えた区切りは後から鳴らない"
    );
    rig.talk_tick(0, at_ms(1210), &shows_100);
    assert_eq!(rig.clock(100, 0), None, "面の数えで 5 文字");

    let p = rig.talk_tick(0, at_ms(1260), &shows_100);
    assert_eq!(
        rig.clock(100, 0),
        playing(at_ms(1250)),
        "面の数えの 6 文字目（見えてから数え直すなら 7 文字目の 1300）"
    );
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);

    let p = rig.talk_tick(0, at_ms(1410), &none);
    assert!(p.is_empty());
    assert_eq!(
        rig.clock(100, 0),
        None,
        "再び見えなくなったら 9 文字目の区切りでも始まらない"
    );
    assert!(rig.armed().is_empty());
}

/// 区切りの文字（3 文字目・1100）が、部品の見えた刻み（1160）の窓の中で、見え始めより前に現れたら、
/// 開始は見え始めの時刻（部品は 1100 にはまだ見えていない・時刻は正確に扱う）。
#[test]
fn part_talk_crossed_in_the_revealing_tick_starts_at_the_reveal() {
    let mut rig = Rig::new(LATE_MOUTH, &[]);
    let mut shows_100 = PatternState::default();
    shows_100.set(7, overlay(100));

    rig.talk_tick(0, at_ms(1010), &PatternState::default());
    let p = rig.talk_tick(0, at_ms(1160), &shows_100);
    assert_eq!(rig.clock(100, 0), playing(at_ms(1160)), "1100 からではない");
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
}

/// 子 100 の `talk,3` が 2 本: animation 0 は長さが区切りの間隔（3 文字＝150ms）ちょうど（101 →
/// 150ms で `-1`）、animation 1 は間隔より長い（111 → 200ms で `-1`）。
const MOUTH_GAPS: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nanimation0.interval,talk,3\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,-1,150,0,0\n\
    animation1.interval,talk,3\n\
    animation1.pattern0,overlay,111,0,0,0\n\
    animation1.pattern1,overlay,-1,200,0,0\n}\n";

/// 部品の `talk` の「再生中か」は刻みの時刻でなく区切りの文字が現れた時刻で測る: 長さが区切りの間隔
/// ちょうどの口は区切りごとに毎回動き、区切りの時刻に再生中だった口は、遅れた刻みが来た時点でもう
/// 終えていてもその区切りを見送る。`talk` の開始と終わりの記録は `debug!` で、再生 1 本につき 1 件ずつ。
#[test]
fn part_talk_is_judged_playing_at_the_boundary_glyph_time() {
    let mut rig = Rig::new(MOUTH_GAPS, &[]);
    let top = PatternState::default();

    let lines = capture_logs(|| {
        rig.talk_tick(0, at_ms(1010), &top);
        rig.talk_tick(0, at_ms(1120), &top);
        assert_eq!(rig.clock(100, 0), playing(at_ms(1100)));
        assert_eq!(rig.clock(100, 1), playing(at_ms(1100)));

        // 遅れた刻み: 6 文字目の 1250 に animation 0 は終えたところ（鳴る）、animation 1 は再生中（見送る）。
        let p = rig.talk_tick(0, at_ms(1310), &top);
        assert_eq!(
            rig.clock(100, 0),
            playing(at_ms(1250)),
            "長さが区切りの間隔ちょうどでも毎回動く・開始は区切りの文字の時刻"
        );
        assert_eq!(
            rig.clock(100, 1),
            None,
            "区切りの時刻に再生中だった口は、刻みの時刻に終えていても始め直さない"
        );
        assert_eq!(part_frames(&p, 100), vec![(0, 101)]);

        rig.talk_tick(0, at_ms(1410), &top);
        assert_eq!(rig.clock(100, 0), playing(at_ms(1400)));
        assert_eq!(
            rig.clock(100, 1),
            playing(at_ms(1400)),
            "見送った次の区切りは鳴る"
        );
    });
    let fire = ["level=DEBUG", "part trigger talk を鳴らした", "part=100"];
    let stop = ["level=DEBUG", "seriko: part 停止", "part=100"];
    for (id, fired, stopped) in [("animation_id=0", 3, 2), ("animation_id=1", 2, 1)] {
        assert_eq!(
            count(&lines, &[fire[0], fire[1], fire[2], id]),
            fired,
            "{id}: {lines:#?}"
        );
        assert_eq!(
            count(&lines, &[stop[0], stop[1], stop[2], id]),
            stopped,
            "{id}: 終わりの記録は再生 1 本につき 1 回: {lines:#?}"
        );
    }
    assert_eq!(
        count(&lines, &["level=DEBUG"]),
        lines.len(),
        "`talk` の記録はどれも debug!: {lines:#?}"
    );
}

/// 子 100 の `talk,3` は待ち 0 のコマ 1 枚だけ（開始の時刻にもう終えている）。面 200 は `runonce`
/// （201 → 300ms で `-1`）を持ち、一番上のコマが指したときだけ見える。
const INSTANT_MOUTH: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nanimation0.interval,talk,3\n\
    animation0.pattern0,overlay,101,0,0,0\n}\n\
    surface200\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,201,0,0,0\n\
    animation0.pattern1,overlay,-1,300,0,0\n}\n";

/// 開始の時刻にもう終えている口（待ち 0 のコマ 1 枚）は、区切りを越えた刻みにちょうど 1 回鳴る。同じ
/// 刻みに別の部品の引き金が鳴って評価が 2 段目をもう 1 回通っても、判定済みの部品は判定し直さない
/// （判定し直すと「再生中でない」ので同じ区切りでもう 1 回鳴る）。
#[test]
fn part_talk_that_ends_at_its_start_fires_once_when_the_evaluation_loops() {
    let mut rig = Rig::new(INSTANT_MOUTH, &[]);
    let none = PatternState::default();
    let mut shows_200 = PatternState::default();
    shows_200.set(7, overlay(200));

    rig.talk_tick(0, at_ms(1010), &none);
    let lines = capture_logs(|| {
        let p = rig.talk_tick(0, at_ms(1120), &shows_200);
        assert_eq!(part_frames(&p, 100), vec![(0, 101)], "末尾のコマを保つ");
        assert_eq!(
            rig.clock(100, 0),
            Some(PartAnim::Residual { frame_index: 0 })
        );
    });
    assert_eq!(
        count(&lines, &["runonce を鳴らした", "part=200"]),
        1,
        "前提: 同じ刻みに別の部品の引き金が鳴った（2 段目をもう 1 回通る）: {lines:#?}"
    );
    assert_eq!(
        count(&lines, &["talk を鳴らした", "part=100"]),
        1,
        "{lines:#?}"
    );
    assert_eq!(
        count(&lines, &["seriko: part 末尾残留", "part=100"]),
        1,
        "{lines:#?}"
    );
}
