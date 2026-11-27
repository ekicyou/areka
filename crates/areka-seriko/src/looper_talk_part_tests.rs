//! 部品の `talk`: 部品は文字の数えを持たず、面の文字の窓を借りることの決定論テスト。

use super::talk_tests::{MOUTH_ON_PART, NINE, Stage, nine_glyphs_from_1000, note};
use super::tests::{counting_rng, part_clock};
use super::*;
use crate::test_support::{
    capture_logs, clear_all, count, part_frames, part_playing, scope, table_of, text,
};
use dola::time::testing::{at_ms, ms};

// ── 部品の `talk`: 部品は文字の数えを持たず、面の文字の窓を借りる ──────────────

/// `\0` のシェルの面の部品 100 の欄（animation の番号, 絵）。
fn part_cells(st: &Stage) -> Vec<(u32, u32)> {
    part_frames(st.states.current_pattern(&scope(), Slot::Shell), 100)
}

/// 時計 1000 を台本の 0 秒にして 9 文字（1000・1050・…・1400 に現れる）を届ける。
fn hear_nine_from_1000(st: &mut Stage) {
    st.hear(1000, clear_all());
    st.hear(1000, text(ms(0), "0", NINE));
}

/// 一番上に 3 語が 1 つも無く部品にだけ `talk` が在る面でも、届いた文字で部品の口が動く（面の窓を
/// 借りる）。開始は区切りの文字が現れた時刻で、同じ刻みのうちに経過の分のコマを出す。一番上では
/// 何も始まらない。
#[test]
fn part_mouth_moves_on_a_surface_whose_top_has_none_of_the_three_words() {
    let mut st = Stage::shell(MOUTH_ON_PART);
    st.tick(1000);
    st.show(0, 1000);
    hear_nine_from_1000(&mut st);

    assert!(st.tick(1090).is_empty(), "2 文字（1000・1050）ではまだ");
    assert_eq!(part_clock(&st.rt, Slot::Shell, 100, 0), None);

    let cmds = st.tick(1120);
    assert_eq!(
        part_clock(&st.rt, Slot::Shell, 100, 0),
        part_playing(at_ms(1100)),
        "開始は 3 文字目が現れた 1100（刻みの 1120 ではない）"
    );
    assert_eq!(cmds.len(), 1, "同じ刻みで口のコマを出す");
    assert_eq!(part_cells(&st), [(0, 601)]);

    assert_eq!(st.tick(1145).len(), 1, "`-1` で口を消す（経過 45）");
    assert_eq!(part_clock(&st.rt, Slot::Shell, 100, 0), None);
    assert!(part_cells(&st).is_empty());

    st.tick(1240);
    assert_eq!(
        part_clock(&st.rt, Slot::Shell, 100, 0),
        None,
        "5 文字ではまだ"
    );
    st.tick(1270);
    assert_eq!(
        part_clock(&st.rt, Slot::Shell, 100, 0),
        part_playing(at_ms(1250)),
        "6 文字目"
    );
    assert_eq!(part_cells(&st), [(0, 601)]);
    assert!(st.rt.playback.is_empty(), "一番上では何も始まらない");
}

/// 面 0 の一番上に `talk,3` の口（601）が在り、面 0 に置かれた部品（面 100）にも `talk,3` の口（611）が
/// 在る。
const MOUTH_ON_TOP_AND_PART: &str = "surface0\n{\nelement0,overlay,100,0,0\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,-1,40,0,0\n}\n\
    surface100\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,611,0,0,0\n\
    animation0.pattern1,overlay,-1,40,0,0\n}\n\
    surface601\n{\n}\nsurface611\n{\n}\n";

/// 一番上と部品の両方に `talk` が在る面では、同じ刻みで両方が同じ文字の窓を読み、同じ文字が現れた
/// 時刻に両方の口が動く。面の数えは両方が読み終えてから進むので、次の区切りでも両方が動く。
#[test]
fn top_mouth_and_part_mouth_on_one_surface_move_at_the_same_glyph() {
    let mut st = Stage::shell(MOUTH_ON_TOP_AND_PART);
    st.tick(1000);
    st.show(0, 1000);
    hear_nine_from_1000(&mut st);

    st.tick(1090);
    assert_eq!(
        (st.started(), part_clock(&st.rt, Slot::Shell, 100, 0)),
        (None, None),
        "2 文字（1000・1050）ではまだ"
    );

    st.tick(1120);
    assert_eq!(
        st.started(),
        Some(at_ms(1100)),
        "一番上: 3 文字目が現れた 1100"
    );
    assert_eq!(
        part_clock(&st.rt, Slot::Shell, 100, 0),
        part_playing(at_ms(1100)),
        "部品: 一番上と同じ刻みに、同じ 1100 から"
    );
    assert_eq!((st.mouth(), part_cells(&st)), (Some(601), vec![(0, 611)]));

    st.tick(1190);
    st.tick(1240);
    assert_eq!(
        (st.started(), part_clock(&st.rt, Slot::Shell, 100, 0)),
        (None, None),
        "5 文字ではまだ（どちらの口も `-1` で終えている）"
    );

    st.tick(1270);
    assert_eq!(st.started(), Some(at_ms(1250)), "一番上: 6 文字目");
    assert_eq!(
        part_clock(&st.rt, Slot::Shell, 100, 0),
        part_playing(at_ms(1250)),
        "部品: 6 文字目"
    );
}

/// 面 0 と面 1 がどちらも部品 100（`talk,3` の口）を置く。
const MOUTH_ON_SHARED_PART: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface1\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,-1,40,0,0\n}\n\
    surface601\n{\n}\n";

/// 面が切り替わると面の数えが数え直され、見え続けた部品の口もその数えに従う（部品は自分の数えを
/// 持たない）。
#[test]
fn part_mouth_follows_the_recount_when_the_surface_changes() {
    let mut st = Stage::shell(MOUTH_ON_SHARED_PART);
    st.tick(1000);
    st.show(0, 1000);
    hear_nine_from_1000(&mut st);
    st.tick(1060);
    st.show(1, 1060);
    assert_eq!(st.counted(Slot::Shell), Some((0, 2)), "2 文字が現れている");

    for now in [1110, 1160] {
        st.tick(now);
        assert_eq!(
            part_clock(&st.rt, Slot::Shell, 100, 0),
            None,
            "{now}: 古い面の数えなら 3 文字目（1100）で鳴っていた"
        );
    }
    st.tick(1210);
    assert_eq!(
        part_clock(&st.rt, Slot::Shell, 100, 0),
        part_playing(at_ms(1200)),
        "切り替えてから 3 文字目（通算 5 文字目）"
    );
}

/// 面が非表示のスコープでは、文字が届いても部品の時計も引き金の状態も記録も生まれない。面が出たら、
/// その時点で現れていた文字の次から数えた区切りで部品の口が動く。
#[test]
fn part_mouth_on_a_hidden_surface_stays_silent_and_counts_from_its_return() {
    let mut st = Stage::shell(MOUTH_ON_PART);
    let logs = capture_logs(|| {
        st.tick(1000);
        hear_nine_from_1000(&mut st);
        for now in [1060, 1120, 1190, 1240] {
            assert!(st.tick(now).is_empty(), "{now}");
            assert!(st.rt.parts.is_empty(), "{now}: 部品の入れ物は生まれない");
        }
    });
    assert_eq!(count(&logs, "seriko:"), 0, "{logs:?}");

    // 1260 に面を出す: 6 文字（1000〜1250）が現れている。そこから 3 文字目は 9 文字目（1400）。
    st.show(0, 1260);
    st.tick(1310);
    assert_eq!(
        part_clock(&st.rt, Slot::Shell, 100, 0),
        None,
        "戻る前の文字は数えない"
    );
    st.tick(1410);
    assert_eq!(
        part_clock(&st.rt, Slot::Shell, 100, 0),
        part_playing(at_ms(1400))
    );
}

/// バルーンの面の部品の `talk`: 窓が閉じている間は文字の窓が無く、部品の口は動かない。開いたら
/// その時点で現れていた文字の次から数えた区切りで動き、閉じたら再生中の口の時計を捨てる。
#[test]
fn part_mouth_is_silent_while_the_balloon_window_is_closed() {
    let (rng, _probe) = counting_rng(&[]);
    let mut st = Stage::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(scope(), table_of(MOUTH_ON_PART))]),
        rng,
        pick_rng: crate::seeded_rng(0),
    });
    note(&mut st, false, 1000);
    st.tick(1000);
    hear_nine_from_1000(&mut st);
    for now in [1060, 1120, 1160] {
        st.tick(now);
        assert_eq!(
            part_clock(&st.rt, Slot::Balloon, 100, 0),
            None,
            "{now}: 閉じている間は動かない"
        );
    }

    // 1160 に開く: 4 文字（1000〜1150）が現れている。そこから 3 文字目は 7 文字目（1300）。
    note(&mut st, true, 1160);
    st.tick(1260);
    assert_eq!(part_clock(&st.rt, Slot::Balloon, 100, 0), None);
    st.tick(1310);
    assert_eq!(
        part_clock(&st.rt, Slot::Balloon, 100, 0),
        part_playing(at_ms(1300))
    );

    note(&mut st, false, 1320);
    st.tick(1330);
    assert_eq!(
        part_clock(&st.rt, Slot::Balloon, 100, 0),
        None,
        "閉じたら再生中の口の時計を捨てる"
    );
}

/// 面 0 に置かれた部品（面 100）に `talk,3` が 2 本: animation 0 は `-1` で止まり、animation 1 は
/// 末尾のコマを残して終える。一番上には何も無い。
const TWO_MOUTHS_ON_PART: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface100\n{\n\
    animation0.interval,talk,3\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,-1,40,0,0\n\
    animation1.interval,talk,3\n\
    animation1.pattern0,overlay,611,0,0,0\n\
    animation1.pattern1,overlay,612,40,0,0\n}\n\
    surface601\n{\n}\nsurface611\n{\n}\nsurface612\n{\n}\n";

/// 部品の `talk` の開始・停止・末尾の記録は `debug!` で、区切りごとに 1 件ずつ。文字ごと・刻みごとの
/// 記録は無い（30 文字・約 100 刻みで、構えた 2 件と区切り 10 回分の記録だけ）。
#[test]
fn part_talk_records_are_debug_and_bounded_by_boundaries() {
    let mut st = Stage::shell(TWO_MOUTHS_ON_PART);
    let thirty = NINE.repeat(3) + "こさし";
    let logs = capture_logs(|| {
        st.tick(1000);
        st.show(0, 1000);
        st.hear(1000, clear_all());
        st.hear(1000, text(ms(0), "0", &thirty));
        let mut now = 1016;
        while now <= 2600 {
            st.tick(now);
            now += 16;
        }
    });

    let ours: Vec<&String> = logs
        .iter()
        .filter(|l| {
            ["seriko: trigger", "seriko: loop", "seriko: part"]
                .iter()
                .any(|needle| l.contains(needle))
        })
        .collect();
    assert_eq!(count(&logs, "seriko: trigger 面に入った"), 1, "{ours:?}");
    assert_eq!(count(&logs, "seriko: part trigger 部品が見えた"), 1);
    assert_eq!(
        count(&logs, "seriko: part trigger talk を鳴らした"),
        20,
        "{ours:?}"
    );
    assert_eq!(count(&logs, "seriko: part 停止"), 10, "{ours:?}");
    assert_eq!(count(&logs, "seriko: part 末尾残留"), 10, "{ours:?}");
    assert_eq!(ours.len(), 42, "ほかの記録は無い: {ours:?}");
    assert!(
        ours.iter().all(|l| l.contains("level=DEBUG")),
        "どれも debug!: {ours:?}"
    );
}

/// 合図の更新（着せ替えなど・1120）も `talk` を判定する: 3 文字目（1100）の区切りでその場で口が動き、
/// 数えを進めるので次の刻みは同じ区切りで鳴り直さない。
#[test]
fn refresh_judges_talk_and_advances_the_count() {
    let mut st = nine_glyphs_from_1000();
    st.tick(1090);
    let logs = capture_logs(|| {
        st.rt
            .refresh(&scope(), Slot::Shell, at_ms(1120), &mut st.states);
        assert_eq!(
            st.started(),
            Some(at_ms(1100)),
            "合図の時刻で 1100 に始まる"
        );
        st.tick(1130);
    });
    assert_eq!(st.started(), Some(at_ms(1100)));
    assert_eq!(count(&logs, "trigger talk を鳴らした"), 1, "{logs:?}");
}
