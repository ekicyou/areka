//! 部品の `runonce`・`periodic`・`talk` の檻（spec: areka-P0-seriko-trigger-intervals 要件 4.1・4.5・4.7・
//! 5.1・5.2・5.4・5.6・5.7・5.9・6.1・6.2・7.3・8.1・8.2・9.5・9.6・tasks.md 5.1・5.2）。
//!
//! 表は `surfaces.txt` の本文から実経路で組む。刻みは `advance` の `now_ms`、出来事の時刻は `refresh` の
//! `at_ms`、乱数は呼ばれた回数を数える注入列で、どれも決定論。刻みは全部「境界を跨いだ」扱いで回す
//! （抽選の animation が在れば必ず引く＝3 語が乱数を引かないことを回数で見分ける）。
//!
//! `talk` の檻は、面の文字の窓を偽の文字の到着（1000ms から 50ms ごとに 1 文字）から刻みごとに組んで
//! `advance` へ渡す。本物の窓（cue の到着から一番上の配線が組む窓）を通す檻は
//! `looper_talk_tests.rs` に在る。

use std::sync::{Arc, Mutex};

use areka_emo_compose::{PartKey, PatternState};

use super::tests::{calls, capture_logs, count, no_binds, overlay, part_frames, scope, table_of};
use super::{PartAnim, PartClocks};
use crate::looper::tests::{RngProbe, counting_rng};
use crate::state::Slot;
use crate::table::AnimationTable;
use crate::timeline::LoopRng;
use crate::trigger::TalkWindow;

/// 部品の時計・表・数える乱数の一式（シェルの面・着せ替えなし）。
struct Rig {
    clocks: PartClocks,
    table: AnimationTable,
    rng: LoopRng,
    probe: Arc<Mutex<RngProbe>>,
    /// 面の文字の数え（前の `talk_tick` までに数えた文字の数）。
    seen: u64,
}

/// 序数 `glyph` の文字が現れる時刻（偽の文字の到着: 1000ms から 50ms ごとに 1 文字）。
fn glyph_ms(glyph: u64) -> u64 {
    1000 + 50 * glyph
}

impl Rig {
    fn new(text: &str, rng_values: &[u32]) -> Rig {
        let (rng, probe) = counting_rng(rng_values);
        Rig {
            clocks: PartClocks::default(),
            table: table_of(text),
            rng,
            probe,
            seen: 0,
        }
    }

    /// 文字の窓つきの刻み 1 回（窓は開いている）。面の数えは 0 文字目から数え、窓は「前の
    /// `talk_tick` までに数えた数 〜 `now_ms` までに現れた数」。数えを進めるのは評価の後（一番上の
    /// 配線が刻みの最後に進めるのと同じ順）。
    fn talk_tick(&mut self, top: u32, now_ms: u64, top_pattern: &PatternState) -> PatternState {
        let now_seen = now_ms.checked_sub(1000).map_or(0, |since| since / 50 + 1);
        let window = TalkWindow {
            base: 0,
            prev_seen: self.seen,
            now_seen,
            wall_ms: &glyph_ms,
        };
        let mut pattern = top_pattern.clone();
        self.clocks.advance(
            &scope(),
            Slot::Shell,
            top,
            &no_binds(),
            &self.table,
            now_ms,
            true,
            true,
            Some(&window),
            &mut self.rng,
            &mut pattern,
        );
        self.seen = self.seen.max(now_seen);
        pattern
    }

    /// 刻み 1 回（窓は開いている・一番上の欄は空）。作り直した絵を返す。
    fn tick(&mut self, top: u32, now_ms: u64) -> PatternState {
        self.tick_with(top, now_ms, true, &PatternState::default())
    }

    /// 刻み 1 回（窓の開閉 `open`・一番上の進行を済ませた絵 `top_pattern`）。
    fn tick_with(
        &mut self,
        top: u32,
        now_ms: u64,
        open: bool,
        top_pattern: &PatternState,
    ) -> PatternState {
        let mut pattern = top_pattern.clone();
        self.clocks.advance(
            &scope(),
            Slot::Shell,
            top,
            &no_binds(),
            &self.table,
            now_ms,
            true,
            open,
            None,
            &mut self.rng,
            &mut pattern,
        );
        pattern
    }

    /// 面の切り替えの直後の評価（出来事の時刻 `at_ms`・窓は開いている）。
    fn refresh(&mut self, top: u32, at_ms: Option<u64>) -> PatternState {
        let mut pattern = PatternState::default();
        self.clocks.refresh(
            &scope(),
            Slot::Shell,
            top,
            &no_binds(),
            &self.table,
            at_ms,
            true,
            &mut pattern,
        );
        pattern
    }

    fn clock(&self, part: u32, animation_id: u32) -> Option<PartAnim> {
        self.clocks.clock(&scope(), part, animation_id)
    }

    /// 引き金の状態を持つ部品の番号（昇順）。
    fn armed(&self) -> Vec<u32> {
        self.clocks
            .armed_parts(&scope(), Slot::Shell)
            .into_iter()
            .map(|part| match part {
                PartKey::Surface(id) => id,
                PartKey::Film(_) => unreachable!("動く絵の子は 3 語を持たない"),
            })
            .collect()
    }
}

fn playing(at: u64) -> Option<PartAnim> {
    Some(PartAnim::Playing { started_at_ms: at })
}

/// 一番上 0 が子 100 を置き、一番上 1 は何も置かない。100 の `runonce`（animation 0）は頭のコマで
/// 別の部品 200 を見せ、300ms で `-1`。100 の `periodic,1`（animation 1）は 101 → 400ms で `-1`。
/// 200 の `runonce` は 201 → 100ms で 202 → 100ms で `-1`。
const NESTED: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\nsurface1\n{\n}\n\
    surface100\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,200,0,0,0\n\
    animation0.pattern1,overlay,-1,300,0,0\n\
    animation1.interval,periodic,1\n\
    animation1.pattern0,overlay,101,0,0,0\n\
    animation1.pattern1,overlay,-1,400,0,0\n}\n\
    surface200\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,201,0,0,0\n\
    animation0.pattern1,overlay,202,100,0,0\n\
    animation0.pattern2,overlay,-1,100,0,0\n}\n";

/// 見えている部品の `runonce` は見えた刻みに 1 回鳴り、そのコマが見せた子の `runonce` も同じ刻みで
/// 鳴って子のコマまで出る（1 刻み遅れない）。遅れた刻みは過ぎた分だけ進み、`-1` で終えた後は鳴らない。
/// `periodic` は見えた瞬間には鳴らず、見えた時刻から数えた周の境目を開始にする。乱数は引かない
/// （要件 5.1・5.2・5.4・5.7・5.9・6.2）。
#[test]
fn visible_part_fires_runonce_and_its_revealed_child_in_the_same_tick() {
    let mut rig = Rig::new(NESTED, &[]);

    let p = rig.tick(0, 1016);
    assert_eq!(part_frames(&p, 100), vec![(0, 200)], "見えた刻みに頭のコマ");
    assert_eq!(
        part_frames(&p, 200),
        vec![(0, 201)],
        "コマが見せた子も同じ刻みで頭のコマを出す"
    );
    assert_eq!(rig.clock(100, 0), playing(1016), "開始は見えた刻みの時刻");
    assert_eq!(rig.clock(200, 0), playing(1016));
    assert_eq!(rig.clock(100, 1), None, "periodic は見えた瞬間には鳴らない");
    assert_eq!(rig.armed(), vec![100, 200]);

    let p = rig.tick(0, 1130);
    assert_eq!(
        part_frames(&p, 200),
        vec![(0, 202)],
        "経過 114 で 2 枚目（遅れた分だけ進む）"
    );
    let p = rig.tick(0, 1216);
    assert!(part_frames(&p, 200).is_empty(), "子は `-1` で消して終える");
    assert_eq!(rig.clock(200, 0), None);
    assert_eq!(part_frames(&p, 100), vec![(0, 200)]);

    let p = rig.tick(0, 1316);
    assert!(p.is_empty(), "親も `-1` で消して終える");
    assert_eq!(rig.clock(100, 0), None);
    assert_eq!(
        rig.armed(),
        vec![100],
        "見えなくなった子の引き金の状態は捨てる"
    );

    let mut now = 1400;
    while now < 2016 {
        let p = rig.tick(0, now);
        assert_eq!(
            rig.clock(100, 0),
            None,
            "{now}: runonce は 2 回目を鳴らさない"
        );
        assert!(p.is_empty(), "{now}: 周の境目の前");
        now += 50;
    }

    let p = rig.tick(0, 2050);
    assert_eq!(
        rig.clock(100, 1),
        playing(2016),
        "開始は見えた時刻から 1 秒の境目そのもの（刻みの時刻ではない）"
    );
    assert_eq!(part_frames(&p, 100), vec![(1, 101)]);
    assert_eq!(rig.clock(100, 0), None);
    assert_eq!(calls(&rig.probe), 0, "3 語は部品でも乱数を引かない");
}

/// 一番上 0 が子 200 を置く。200 の `always` は 300（経過 0）→ 100ms で 201 → 100ms で 300（周期 200）。
/// 300 の `runonce` は 301 → 150ms で 302 → 400ms で `-1`。経過 100〜200 の間、300 は絵に出ない
/// （それでも評価の 1 回目では見える扱いになり、1 段目の評価は受ける）。
const HIDDEN_BY_FRAME: &str = "surface0\n{\nelement0,overlay,200,0,0\n}\n\
    surface200\n{\nanimation0.interval,always\n\
    animation0.pattern0,overlay,300,0,0,0\n\
    animation0.pattern1,overlay,201,100,0,0\n\
    animation0.pattern2,overlay,300,100,0,0\n}\n\
    surface300\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,301,0,0,0\n\
    animation0.pattern1,overlay,302,150,0,0\n\
    animation0.pattern2,overlay,-1,400,0,0\n}\n";

/// 外側のコマで見えなくなった部品は、再生中の 3 語の時計と引き金の状態を捨てる。見えない間は 1 段目の
/// 評価を受けても構えず、再生も始まらない。再び見えた刻みが新しい起点で、頭のコマから始まる
/// （途中のコマから始まらない・要件 5.6・5.9）。
#[test]
fn part_hidden_by_the_outer_frame_is_dropped_and_restarts_from_the_top_when_seen_again() {
    let mut rig = Rig::new(HIDDEN_BY_FRAME, &[]);

    let lines = capture_logs(|| {
        let p = rig.tick(0, 1000);
        assert_eq!(
            part_frames(&p, 300),
            vec![(0, 301)],
            "前提: 見えていれば鳴る"
        );
        assert_eq!(rig.clock(300, 0), playing(1000));
        assert_eq!(rig.armed(), vec![300]);

        // 200 のコマが 201 になり、300 は絵に出ない。
        let p = rig.tick(0, 1100);
        assert_eq!(part_frames(&p, 200), vec![(0, 201)]);
        assert!(part_frames(&p, 300).is_empty());
        assert_eq!(rig.clock(300, 0), None, "再生中の時計を捨てる");
        assert!(rig.armed().is_empty(), "引き金の状態も捨てる");

        let p = rig.tick(0, 1150);
        assert!(part_frames(&p, 300).is_empty());
        assert_eq!(rig.clock(300, 0), None, "見えない部品では始まらない");
        assert!(
            rig.armed().is_empty(),
            "1 段目の評価を受けても、見えない部品は構えない"
        );

        // 周の頭へ戻って 300 が再び見える。
        let p = rig.tick(0, 1210);
        assert_eq!(
            rig.clock(300, 0),
            playing(1210),
            "再び見えた刻みが新しい起点"
        );
        assert_eq!(
            part_frames(&p, 300),
            vec![(0, 301)],
            "頭のコマから（前の再生の続きの 302 ではない）"
        );
    });
    assert_eq!(
        count(
            &lines,
            &[
                "level=INFO",
                "runonce を鳴らした",
                "part=300",
                "animation_id=0"
            ]
        ),
        2,
        "見えるたびに 1 回: {lines:#?}"
    );
    assert_eq!(
        count(&lines, &["level=DEBUG", "引き金を構えた", "part=300"]),
        2,
        "構えた記録は構えたときだけ: {lines:#?}"
    );
    assert_eq!(count(&lines, &["level=WARN"]), 0, "{lines:#?}");
}

/// 一番上 0 は子を置かない（一番上のコマが 100 を指したときだけ 100 が見える）。100 の `periodic,1` は
/// 101 → 300ms で `-1`。
const LATE: &str = "surface0\n{\n}\n\
    surface100\n{\nanimation0.interval,periodic,1\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,-1,300,0,0\n}\n";

/// 外側のコマの変化で後から見えた部品は、見えた刻みを起点に周を数える（外側の面に入った時刻からでは
/// ない）。見えなくなったら止まり、再び見えた刻みが新しい起点になる（要件 5.9・3.2）。
#[test]
fn part_revealed_later_counts_its_period_from_the_tick_it_appeared() {
    let mut rig = Rig::new(LATE, &[]);
    let none = PatternState::default();
    let mut shows_100 = PatternState::default();
    shows_100.set(7, overlay(100));

    rig.tick_with(0, 1000, true, &none);
    assert!(rig.armed().is_empty(), "見えていない間は構えない");

    rig.tick_with(0, 1500, true, &shows_100);
    assert_eq!(rig.armed(), vec![100]);
    for now in [2000u64, 2460] {
        rig.tick_with(0, now, true, &shows_100);
        assert_eq!(
            rig.clock(100, 0),
            None,
            "{now}: 見えてから 1 秒たっていない"
        );
    }
    let p = rig.tick_with(0, 2530, true, &shows_100);
    assert_eq!(
        rig.clock(100, 0),
        playing(2500),
        "見えた刻み 1500 から 1 秒"
    );
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);

    let p = rig.tick_with(0, 2600, true, &none);
    assert!(part_frames(&p, 100).is_empty());
    assert_eq!(rig.clock(100, 0), None, "見えなくなったら再生中でも捨てる");
    assert!(rig.armed().is_empty());

    rig.tick_with(0, 2700, true, &shows_100);
    rig.tick_with(0, 3600, true, &shows_100);
    assert_eq!(rig.clock(100, 0), None, "前の起点（1500）の周では鳴らない");
    rig.tick_with(0, 3710, true, &shows_100);
    assert_eq!(
        rig.clock(100, 0),
        playing(3700),
        "新しい起点 2700 から 1 秒"
    );
}

/// 子 100 の `periodic,1` が 2 本: animation 0 は長さが周期ちょうど（101 → 1000ms で `-1`）、
/// animation 1 は周期より長い（111 → 1500ms で `-1`）。
const LAPS: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nanimation0.interval,periodic,1\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,-1,1000,0,0\n\
    animation1.interval,periodic,1\n\
    animation1.pattern0,overlay,111,0,0,0\n\
    animation1.pattern1,overlay,-1,1500,0,0\n}\n";

/// 「再生中か」は刻みの時刻でなく周の境目の時刻で測る: 長さが周期ちょうどの animation は毎周鳴り、
/// 境目の時刻に再生中だった animation は、遅れた刻みが来た時点でもう終えていてもその周を飛ばす。
/// 終えた再生の終わりの記録は 1 回ずつ（要件 3.1・3.3・3.5・5.3・5.4）。
#[test]
fn periodic_part_is_judged_playing_at_the_lap_boundary() {
    let mut rig = Rig::new(LAPS, &[]);

    let lines = capture_logs(|| {
        rig.tick(0, 10_000);
        rig.tick(0, 11_000);
        assert_eq!(rig.clock(100, 0), playing(11_000));
        assert_eq!(rig.clock(100, 1), playing(11_000));

        // 遅れた刻み: 境目 12000 に animation 0 は終えたところ（鳴る）、animation 1 は再生中（飛ばす）。
        let p = rig.tick(0, 12_600);
        assert_eq!(
            rig.clock(100, 0),
            playing(12_000),
            "長さが周期ちょうどでも毎周鳴る・開始は境目"
        );
        assert_eq!(
            rig.clock(100, 1),
            None,
            "境目の時刻に再生中だった周は、刻みの時刻に終えていても飛ばす"
        );
        assert_eq!(part_frames(&p, 100), vec![(0, 101)]);

        rig.tick(0, 13_000);
        assert_eq!(rig.clock(100, 0), playing(13_000));
        assert_eq!(rig.clock(100, 1), playing(13_000), "飛ばした次の周は鳴る");
    });
    let stop = ["level=INFO", "seriko: part 停止", "part=100"];
    assert_eq!(
        count(&lines, &[stop[0], stop[1], stop[2], "animation_id=0"]),
        2,
        "終わりの記録は再生 1 本につき 1 回: {lines:#?}"
    );
    assert_eq!(
        count(&lines, &[stop[0], stop[1], stop[2], "animation_id=1"]),
        1,
        "{lines:#?}"
    );
    let fire = ["level=INFO", "periodic を鳴らした", "part=100"];
    assert_eq!(
        count(&lines, &[fire[0], fire[1], fire[2], "animation_id=0"]),
        3,
        "{lines:#?}"
    );
    assert_eq!(
        count(&lines, &[fire[0], fire[1], fire[2], "animation_id=1"]),
        2,
        "{lines:#?}"
    );
}

/// 末尾のコマを保っている `periodic` が鳴り直した刻みに、頭のコマの待ちの前なら保っていたコマを消す
/// （鳴り直した後は開始からの経過だけで決まる・一番上と同じ・要件 5.1・5.4）。
#[test]
fn refired_part_clears_its_held_frame_while_the_first_frame_is_pending() {
    let mut rig = Rig::new(
        "surface0\n{\nelement0,overlay,100,0,0\n}\n\
         surface100\n{\nanimation0.interval,periodic,1\n\
         animation0.pattern0,overlay,101,50,0,0\n\
         animation0.pattern1,overlay,102,50,0,0\n}\n",
        &[],
    );
    rig.tick(0, 10_000);
    assert!(rig.tick(0, 11_000).is_empty(), "頭のコマの待ちの前");
    assert_eq!(part_frames(&rig.tick(0, 11_060), 100), vec![(0, 101)]);
    assert_eq!(part_frames(&rig.tick(0, 11_100), 100), vec![(0, 102)]);
    assert_eq!(
        part_frames(&rig.tick(0, 11_500), 100),
        vec![(0, 102)],
        "前提: 末尾のコマを保つ"
    );

    let p = rig.tick(0, 12_010);
    assert_eq!(rig.clock(100, 0), playing(12_000));
    assert!(p.is_empty(), "保っていたコマを消す（待ちの前）");
    assert_eq!(part_frames(&rig.tick(0, 12_060), 100), vec![(0, 101)]);
}

/// 子 100 の animation 0 は 2 で割る抽選（101 → 50ms で `-1`）。3 語は無い。
const NO_WORDS: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nanimation0.interval,random,2\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,-1,50,0,0\n}\n";

/// 表に 3 語が無ければ、動く部品が見えて再生していても引き金の状態は生まれず、記録も増えない
/// （要件 8.1）。同じ形で 3 語を書いた表では生まれる（陽性対照）。
#[test]
fn table_without_the_three_words_never_arms_a_part() {
    let run = |text: &str| {
        let mut rig = Rig::new(text, &[0]);
        let lines = capture_logs(|| {
            for now in [1000u64, 1016, 1100, 2000] {
                rig.tick(0, now);
            }
        });
        (rig.armed(), count(&lines, &["seriko: part trigger"]), lines)
    };

    let (armed, trigger_lines, lines) = run(NO_WORDS);
    assert!(
        count(&lines, &["seriko: part 抽選発火", "part=100"]) >= 1,
        "前提: 動く部品が見えて再生している: {lines:#?}"
    );
    assert!(armed.is_empty());
    assert_eq!(trigger_lines, 0, "{lines:#?}");

    let (armed, trigger_lines, lines) = run(&NO_WORDS.replace("random,2", "runonce"));
    assert_eq!(armed, vec![100], "陽性対照");
    assert!(trigger_lines >= 1, "陽性対照: {lines:#?}");
}

/// 子 100 の `runonce`（101 → 300ms で `-1`）と `periodic,1`（111 → 200ms で `-1`）。
const WINDOW: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,-1,300,0,0\n\
    animation1.interval,periodic,1\n\
    animation1.pattern0,overlay,111,0,0,0\n\
    animation1.pattern1,overlay,-1,200,0,0\n}\n";

/// 窓が閉じている間は部品の 3 語は鳴らず、再生中の 3 語の時計は捨てる。開いた時刻が `periodic` の
/// 新しい起点になり、`runonce` は最初に開いたときに 1 回だけ鳴って、開き直しでは鳴らない
/// （一番上と同じ・要件 5.4・5.5）。
#[test]
fn closed_window_silences_part_triggers_and_reopening_rebases_the_period() {
    let mut rig = Rig::new(WINDOW, &[]);
    let top = PatternState::default();

    assert!(rig.tick_with(0, 1000, false, &top).is_empty());
    assert_eq!(rig.clock(100, 0), None, "閉じている間は鳴らない");
    assert_eq!(rig.armed(), vec![100], "隠れた状態で構える");

    let p = rig.tick_with(0, 1500, true, &top);
    assert_eq!(rig.clock(100, 0), playing(1500), "最初に開いた時刻で 1 回");
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);
    let p = rig.tick_with(0, 2550, true, &top);
    assert_eq!(rig.clock(100, 1), playing(2500), "開いた時刻から 1 秒");
    assert_eq!(part_frames(&p, 100), vec![(1, 111)]);

    assert!(rig.tick_with(0, 2600, false, &top).is_empty());
    assert_eq!(rig.clock(100, 1), None, "閉じたら再生中の時計を捨てる");
    rig.tick_with(0, 3600, false, &top);
    assert_eq!(rig.clock(100, 1), None, "閉じている間は周が来ても鳴らない");

    rig.tick_with(0, 4000, true, &top);
    assert_eq!(rig.clock(100, 0), None, "開き直しでは runonce を鳴らさない");
    rig.tick_with(0, 4990, true, &top);
    assert_eq!(rig.clock(100, 1), None, "前の起点（1500）の周では鳴らない");
    rig.tick_with(0, 5010, true, &top);
    assert_eq!(rig.clock(100, 1), playing(5000), "開き直した時刻から 1 秒");
}

/// 面が隠れた（`\s[-1]`）ら 3 語の時計と引き金の状態を捨て、戻った刻みでもう 1 回 `runonce` が鳴る。
/// 新しい出番（窓の知らせ）は時計を捨てて隠すだけで、`runonce` の 1 回は使い済みのまま。表の差し替えは
/// 全部捨てる（要件 5.9・2.6）。
#[test]
fn hidden_surface_forgets_part_triggers_and_a_new_turn_only_hides_them() {
    let mut rig = Rig::new(WINDOW, &[]);

    rig.tick(0, 1000);
    assert_eq!(rig.clock(100, 0), playing(1000));
    rig.clocks.drop_hidden(&scope(), Slot::Shell, &rig.table);
    assert_eq!(rig.clock(100, 0), None);
    assert!(rig.armed().is_empty(), "面が隠れたら引き金の状態を捨てる");
    rig.tick(0, 2000);
    assert_eq!(rig.clock(100, 0), playing(2000), "戻った刻みでもう 1 回");

    rig.clocks.drop_finite(&scope(), Slot::Shell, &rig.table);
    assert_eq!(rig.clock(100, 0), None, "新しい出番は再生中の時計を捨てる");
    assert_eq!(rig.armed(), vec![100]);
    rig.tick(0, 2500);
    assert_eq!(rig.clock(100, 0), None, "runonce の 1 回は使い済み");
    rig.tick(0, 3510);
    assert_eq!(
        rig.clock(100, 1),
        playing(3500),
        "periodic は現れた時刻から"
    );

    rig.clocks.clear(Slot::Shell);
    assert!(rig.armed().is_empty(), "表の差し替えで捨てる");
    assert_eq!(rig.clock(100, 1), None);
}

/// 子 100 の `runonce`（101 → 100ms で 102 → 100ms で `-1`）と `periodic,1`（111 → 100ms で `-1`）。
const EVENT: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation0.pattern2,overlay,-1,100,0,0\n\
    animation1.interval,periodic,1\n\
    animation1.pattern0,overlay,111,0,0,0\n\
    animation1.pattern1,overlay,-1,100,0,0\n}\n";

/// 外側の面の切り替えで最初から見えた部品は、出来事の時刻（刻みの境目に丸めない）が起点: その評価が
/// `runonce` を鳴らして頭のコマを載せ、次の刻みは出来事の時刻からの経過で進む。出来事の直後の評価は
/// `runonce` だけを鳴らし（`periodic` は刻みが鳴らす）、構えた後の評価（着せ替えの変化）は鳴らさない。
/// 時刻が分からない評価は構えない（要件 5.9・6.1・6.2・2.4）。
#[test]
fn refresh_arms_at_the_event_time_and_fires_only_runonce() {
    let mut rig = Rig::new(EVENT, &[]);

    assert!(rig.refresh(0, None).is_empty());
    assert!(rig.armed().is_empty(), "時刻が分からなければ構えない");

    let p = rig.refresh(0, Some(1234));
    assert_eq!(rig.clock(100, 0), playing(1234), "開始は出来事の時刻");
    assert_eq!(
        part_frames(&p, 100),
        vec![(0, 101)],
        "切り替えの評価が頭のコマを載せる"
    );

    rig.refresh(0, Some(1300));
    assert_eq!(
        rig.clock(100, 0),
        playing(1234),
        "構えた後の評価は鳴らさない"
    );
    let p = rig.tick(0, 1340);
    assert_eq!(
        part_frames(&p, 100),
        vec![(0, 102)],
        "経過は出来事の時刻から（106ms で 2 枚目）"
    );

    rig.tick(0, 1500);
    rig.refresh(0, Some(2300));
    assert_eq!(
        rig.clock(100, 1),
        None,
        "出来事の直後の評価は periodic を鳴らさない"
    );
    rig.tick(0, 2310);
    assert_eq!(
        rig.clock(100, 1),
        playing(2234),
        "周は出来事の時刻 1234 から数える"
    );
    assert_eq!(calls(&rig.probe), 0);
}

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
        for now in [1000u64, 1016, 2000, 3000] {
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
/// コマまで出る。乱数は引かない（要件 4.1・4.5・5.4・5.7・6.2）。
#[test]
fn visible_part_talk_fires_at_each_boundary_from_the_glyph_time() {
    let mut rig = Rig::new(PART_MOUTH, &[]);
    let top = PatternState::default();

    for now in [1010u64, 1090] {
        assert!(rig.talk_tick(0, now, &top).is_empty(), "{now}");
        assert_eq!(rig.clock(100, 0), None, "{now}: 3 文字目はまだ");
    }

    let p = rig.talk_tick(0, 1120, &top);
    assert_eq!(
        rig.clock(100, 0),
        playing(1100),
        "開始は 3 文字目が現れた 1100（刻みの 1120 ではない）"
    );
    assert_eq!(part_frames(&p, 100), vec![(0, 200)], "同じ刻みで口のコマ");
    assert_eq!(
        part_frames(&p, 200),
        vec![(0, 201)],
        "口のコマが見せた子も同じ刻みで出る"
    );

    let p = rig.talk_tick(0, 1190, &top);
    assert!(p.is_empty(), "`-1` で消して終える（経過 90）");
    assert_eq!(rig.clock(100, 0), None);

    rig.talk_tick(0, 1240, &top);
    assert_eq!(rig.clock(100, 0), None, "5 文字ではまだ");
    let p = rig.talk_tick(0, 1295, &top);
    assert_eq!(rig.clock(100, 0), playing(1250), "6 文字目が現れた時刻");
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
/// 3 文字目ではない・要件 5.6・9.5）。
#[test]
fn hidden_part_talk_stays_silent_and_joins_the_slot_count_when_it_appears() {
    let mut rig = Rig::new(LATE_MOUTH, &[]);
    let none = PatternState::default();
    let mut shows_100 = PatternState::default();
    shows_100.set(7, overlay(100));

    rig.talk_tick(0, 1010, &none);
    rig.talk_tick(0, 1120, &none);
    assert_eq!(
        rig.clock(100, 0),
        None,
        "見えていない部品では始まらない（3 文字目の区切り）"
    );
    assert!(rig.armed().is_empty());

    // 4 文字（1000〜1150）が現れたところで見える。
    rig.talk_tick(0, 1160, &shows_100);
    assert_eq!(rig.armed(), vec![100]);
    assert_eq!(
        rig.clock(100, 0),
        None,
        "見えなかった間に越えた区切りは後から鳴らない"
    );
    rig.talk_tick(0, 1210, &shows_100);
    assert_eq!(rig.clock(100, 0), None, "面の数えで 5 文字");

    let p = rig.talk_tick(0, 1260, &shows_100);
    assert_eq!(
        rig.clock(100, 0),
        playing(1250),
        "面の数えの 6 文字目（見えてから数え直すなら 7 文字目の 1300）"
    );
    assert_eq!(part_frames(&p, 100), vec![(0, 101)]);

    let p = rig.talk_tick(0, 1410, &none);
    assert!(p.is_empty());
    assert_eq!(
        rig.clock(100, 0),
        None,
        "再び見えなくなったら 9 文字目の区切りでも始まらない"
    );
    assert!(rig.armed().is_empty());
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
/// 終えていてもその区切りを見送る。`talk` の開始と終わりの記録は `debug!` で、再生 1 本につき 1 件ずつ
/// （要件 4.1・4.7・5.3・7.3）。
#[test]
fn part_talk_is_judged_playing_at_the_boundary_glyph_time() {
    let mut rig = Rig::new(MOUTH_GAPS, &[]);
    let top = PatternState::default();

    let lines = capture_logs(|| {
        rig.talk_tick(0, 1010, &top);
        rig.talk_tick(0, 1120, &top);
        assert_eq!(rig.clock(100, 0), playing(1100));
        assert_eq!(rig.clock(100, 1), playing(1100));

        // 遅れた刻み: 6 文字目の 1250 に animation 0 は終えたところ（鳴る）、animation 1 は再生中（見送る）。
        let p = rig.talk_tick(0, 1310, &top);
        assert_eq!(
            rig.clock(100, 0),
            playing(1250),
            "長さが区切りの間隔ちょうどでも毎回動く・開始は区切りの文字の時刻"
        );
        assert_eq!(
            rig.clock(100, 1),
            None,
            "区切りの時刻に再生中だった口は、刻みの時刻に終えていても始め直さない"
        );
        assert_eq!(part_frames(&p, 100), vec![(0, 101)]);

        rig.talk_tick(0, 1410, &top);
        assert_eq!(rig.clock(100, 0), playing(1400));
        assert_eq!(rig.clock(100, 1), playing(1400), "見送った次の区切りは鳴る");
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
/// （判定し直すと「再生中でない」ので同じ区切りでもう 1 回鳴る・要件 4.6・5.4）。
#[test]
fn part_talk_that_ends_at_its_start_fires_once_when_the_evaluation_loops() {
    let mut rig = Rig::new(INSTANT_MOUTH, &[]);
    let none = PatternState::default();
    let mut shows_200 = PatternState::default();
    shows_200.set(7, overlay(200));

    rig.talk_tick(0, 1010, &none);
    let lines = capture_logs(|| {
        let p = rig.talk_tick(0, 1120, &shows_200);
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
