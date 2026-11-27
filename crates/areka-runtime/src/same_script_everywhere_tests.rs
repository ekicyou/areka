//! 同じ台本は、届く順と届く時刻を変えた受け手のどれで再生しても、同じ観測の時刻で同じに見える。
//!
//! 台本を本物の再生機で刻み、受け口へ出た物（合図・起点の知らせ・トークの終わり）を出た順に控える。
//! その列を、それぞれの時刻（合図の発火時刻・知らせの効く時刻。前の物より前にはならない）に受ける組
//! （フレームは 16 ms ごと）と、粗いフレームで遅れてまとめて受ける組へ与える。文字の層とバルーンの寿命は、
//! 全部を最初のフレームの前に受ける組とも比べる。seriko は観測の時刻ごとに作り直し、その時刻までの物を
//! 流して刻んでから閉じ、最後の絵を読む。

use std::collections::BTreeMap;

use areka_emo_text::state::TextItem;
use dola::TimePoint;
use dola::cue::{CueCommand, TalkCue};
use dola::time::testing::{at_ms, ms};

use crate::same_script_test_support::{
    FrontLook, Item, Played, STEP, SerikoLooks, grid, merged, mouth_starts, play, run_frontend,
    seriko_at,
};

/// 台本の起点。
const ORIGIN: i64 = 1_000;
/// 遅れてまとめて受ける組のフレームの間隔。
const COARSE: i64 = 70;
/// seriko を観測する時刻の間隔。
const PROBE: i64 = 5;

/// 字・待ちの指定・消去と改行・面の切り替えを含む台本。
const SCRIPT: &str = r"\0\s[0]あいうえお\w9かきくけこ\c\nさしす\s[5]せそ\_w[300]たちつてと\e";

/// 時刻どおりに受けた組と、全部を先に受けた組の、文字の層とバルーンの寿命の観測。
struct Looks {
    on_time: BTreeMap<TimePoint, FrontLook>,
    early: BTreeMap<TimePoint, FrontLook>,
}

/// 時刻どおりに受ける組と、遅れてまとめて受ける組（と、全部を先に受ける組）の観測が、どの観測の時刻
/// でも等しい事を確かめ、観測を返す。`extra` は観測に足す時刻（ミリ秒）。
fn assert_same_everywhere(played: &Played, extra: &[i64]) -> Looks {
    let deadline = played.end + crate::balloon_visibility::configured_timeout();
    let script_end = (played.end - TimePoint::ZERO).as_ticks() / 10_000 + 200;
    let around_deadline = [deadline - ms(50), deadline];
    let extra: Vec<TimePoint> = extra.iter().map(|&t| at_ms(t)).collect();
    let coarse = merged(
        merged(grid(ORIGIN, script_end, COARSE), &around_deadline),
        &extra,
    );
    let fine = merged(grid(ORIGIN, script_end, STEP), &coarse);

    let on_time = run_frontend(played, &fine, false, &coarse);
    let batched = run_frontend(played, &coarse, false, &coarse);
    let early = run_frontend(played, &coarse, true, &coarse);
    for &t in &coarse {
        assert_eq!(batched[&t], on_time[&t], "遅れてまとめて受けた組（{t}）");
        assert_eq!(early[&t], on_time[&t], "全部を先に受けた組（{t}）");
    }

    let probes = grid(ORIGIN, script_end, PROBE);
    let fine_ticks = grid(ORIGIN, script_end, STEP);
    let seriko = |ticks: &[TimePoint]| -> SerikoLooks {
        probes
            .iter()
            .map(|&t| (t, seriko_at(played, ticks, t)))
            .collect()
    };
    let seriko_on_time = seriko(&fine_ticks);
    let seriko_batched = seriko(&coarse);
    for &t in &probes {
        assert_eq!(
            seriko_batched[&t], seriko_on_time[&t],
            "seriko の面と絵（{t}）"
        );
    }
    let mouths = mouth_starts(&seriko_on_time);
    assert!(mouths.len() > 10, "前提: 口が何度も動く: {mouths:?}");
    assert_eq!(
        mouth_starts(&seriko_batched),
        mouths,
        "口パクの再生の始まり"
    );
    assert!(
        seriko_on_time
            .values()
            .flatten()
            .any(|(surface, pattern)| *surface == 5 && pattern.get(1).is_some()),
        "前提: 面 5 への切り替えで `always` が動いている"
    );

    let [before, after] = around_deadline;
    assert_eq!(on_time[&before].balloon_shown, Some(true), "前提: 出ている");
    assert_eq!(
        on_time[&after].balloon_shown,
        Some(false),
        "前提: 寿命が尽きて隠れる"
    );
    Looks { on_time, early }
}

/// 全部を先に受けた組の文字の層は、観測の時刻が字の合図の発火時刻より前なら、その字を並べていない。
fn assert_text_waits_for_fire_time(played: &Played, looks: &Looks) {
    let mut checked = 0;
    for (_, item) in &played.items {
        let Item::Cue(TalkCue {
            fire_time,
            command: CueCommand::Text(text),
            ..
        }) = item
        else {
            continue;
        };
        let head: &str = &text[..text.chars().next().expect("字が在る").len_utf8()];
        for (t, look) in looks.early.range(..*fire_time) {
            let items = look.text.as_ref().map_or(&[][..], |(items, _, _)| items);
            assert!(
                !items
                    .iter()
                    .any(|i| matches!(i, TextItem::Glyph { text } if &**text == head)),
                "{t} は「{text}」の発火時刻 {fire_time} より前なのに並んでいる"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 10,
        "前提: 発火時刻より前の観測が在る（{checked}）"
    );
}

/// 字・待ちの指定・消去と改行・面の切り替えを含む台本を、それぞれの時刻に受けても、遅れてまとめて
/// 受けても、全部を先に受けても、同じ観測の時刻で同じ字・指令・口・アニメのコマ・寿命の判断になる。
#[test]
fn the_same_script_looks_the_same_however_it_arrives() {
    let played = play(SCRIPT, ORIGIN, ORIGIN, &[], None);
    let looks = assert_same_everywhere(&played, &[]);
    assert_text_waits_for_fire_time(&played, &looks);
}

/// 途中で前へ 80 ms・後ろへ 150 ms 置き直しても、どの観測の時刻（効く時刻の前と後も）でも等しい。
/// 前へ動くと、効く時刻を越えた字が効く時刻に一度に現れ、後ろへ動くと、現れた字は現れたまま、
/// 残りが遅れる。
#[test]
fn re_origins_forward_and_backward_look_the_same_however_they_arrive() {
    const FORWARD: i64 = 1_784;
    const BACKWARD: i64 = 2_504;
    let played = play(
        SCRIPT,
        ORIGIN,
        ORIGIN,
        &[(FORWARD, -80), (BACKWARD, 150)],
        None,
    );
    let looks = assert_same_everywhere(
        &played,
        &[FORWARD - 1, FORWARD, BACKWARD - 1, BACKWARD, BACKWARD + 20],
    );
    assert_text_waits_for_fire_time(&played, &looks);

    let glyphs = |t: i64| looks.on_time[&at_ms(t)].glyphs;
    assert_eq!(
        (glyphs(FORWARD - 1), glyphs(FORWARD)),
        (7, 9),
        "前へ: 「く」「け」が効く時刻に一度に現れる"
    );
    assert_eq!(
        (glyphs(BACKWARD - 1), glyphs(BACKWARD + 20)),
        (7, 7),
        "後ろへ: 「たち」は現れたまま、「つ」（元は 2520 ms）は遅れる"
    );
}
