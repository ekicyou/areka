//! 口パクの決まり: 1 回の刻みで越えた区切りを古い順に全部、それぞれの字が現れた時刻で判定する。
//!
//! 細かい刻み（10 ms おき）で 1 つずつ越えた時と、粗い刻み 1 回でまとめて越えた時の、始まりの時刻の
//! 列（`trigger talk を鳴らした` の記録の `started_at`）と、最後の刻みの口の絵を比べる。文字は 1 字
//! 50 ms で、区切り（`talk,3`）の字は 1100・1250・1400 に現れる。

use dola::TimeSpan;
use dola::time::testing::{at_ms, ms};

use super::talk_tests::{MOUTH, NINE, Stage, nine_glyphs_from_1000};
use super::*;
use crate::test_support::{capture_logs, clear_all, count, cue, text};

/// 面 0 の一番上に `talk,3` の口で、長さ 200 ms（区切りの間隔 150 ms より長い）。
const LONG_MOUTH: &str = "surface0\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,602,100,0,0\n\
    animation0.pattern2,overlay,-1,100,0,0\n}\n\
    surface601\n{\n}\nsurface602\n{\n}\n";

fn talk_starts(logs: &[String]) -> Vec<String> {
    logs.iter()
        .filter(|l| l.contains("seriko: trigger talk を鳴らした"))
        .filter_map(|l| {
            l.split_whitespace()
                .find(|w| w.starts_with("started_at="))
                .map(str::to_owned)
        })
        .collect()
}

fn starts_at(ms: &[i64]) -> Vec<String> {
    ms.iter()
        .map(|&m| format!("started_at={}", at_ms(m)))
        .collect()
}

/// 9 文字を届けた舞台を `ticks` の刻みで進め、(始まりの列, 終わりの記録の数, 最後の開始, 最後の口の絵)。
fn run(
    mut st: Stage,
    ticks: impl IntoIterator<Item = i64>,
) -> (Vec<String>, usize, Option<TimePoint>, Option<u32>) {
    let logs = capture_logs(|| {
        for now in ticks {
            st.tick(now);
        }
    });
    (
        talk_starts(&logs),
        count(&logs, "seriko: loop 停止"),
        st.started(),
        st.mouth(),
    )
}

fn nine_long_from_1000() -> Stage {
    let mut st = Stage::shell(LONG_MOUTH);
    st.tick(1000);
    st.show(0, 1000);
    st.hear(1000, clear_all());
    st.hear(1000, text(ms(0), "0", NINE));
    st
}

/// 1 回の刻みで区切りを 3 つ越えると、1 つずつ越えた時と同じ時刻に 3 回始まる。前の再生は次の区切りの
/// 時刻には終えているので、終わりまで当ててから替える（終わりの記録の数も同じ）。最後の刻みの口の絵も
/// 同じ（1400 に始まった再生の 50 ms 目＝602）。
#[test]
fn boundaries_crossed_in_one_tick_start_as_if_crossed_one_by_one() {
    let fine = run(nine_glyphs_from_1000(), (1010..=1450).step_by(10));
    let coarse = run(nine_glyphs_from_1000(), [1450]);

    assert_eq!(fine.0, starts_at(&[1100, 1250, 1400]), "前提");
    assert_eq!(coarse, fine);
    assert_eq!((coarse.2, coarse.3), (Some(at_ms(1400)), Some(602)));
}

/// 同じ時刻に現れた字（再生時間 0 の塊）の区切りは、最初の区切りで 1 回だけ始まる。
#[test]
fn boundaries_of_glyphs_revealed_at_the_same_time_start_once() {
    let mut st = Stage::shell(MOUTH);
    st.tick(1000);
    st.show(0, 1000);
    st.hear(1000, clear_all());
    st.hear(
        1000,
        cue(
            ms(0),
            "0",
            areka_sakura::CueCommand::Text(NINE.into()),
            TimeSpan::ZERO,
        ),
    );

    let (starts, _, started, _) = run(st, [1020]);
    assert_eq!(starts, starts_at(&[1000]));
    assert_eq!(started, Some(at_ms(1000)));
}

/// 区切りの字が現れた時刻に同じ口が再生中なら、その区切りは見送る（1250 は 1100 の再生の途中）。
/// まとめて越えても、1 つずつ越えた時と同じ区切りだけが始まる。
#[test]
fn boundary_reached_while_the_mouth_plays_is_skipped_within_one_tick() {
    let fine = run(nine_long_from_1000(), (1010..=1450).step_by(10));
    let coarse = run(nine_long_from_1000(), [1450]);

    assert_eq!(fine.0, starts_at(&[1100, 1400]), "前提");
    assert_eq!(coarse, fine);
}
