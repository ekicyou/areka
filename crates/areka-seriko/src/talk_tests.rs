//! 文字の時刻の列の決定論テスト。
//!
//! 期待の列は、文字の層の式から手で求めた固定の値（文字の層の crate は呼ばない）:
//! i 文字目＝「前の文字＋1 字の間隔」と「塊の頭」の大きい方・1 字の間隔＝塊の再生時間÷文字数・
//! 塊の頭は合図の発火時刻・列の先頭と消去の直後の文字は塊の頭・ある時刻に見えている数＝その時刻以前の
//! 文字の数。時刻は `TimePoint` のちょうどの値で比べる。列が単調であることは、単調な固定の列との一致で
//! 確かめる。

use super::*;
use dola::time::testing::{at_ms, ms};

/// 序数 `from` から末尾までの文字の、現れる時刻の列。
fn times_from(feed: &TalkFeed, from: u64) -> Vec<TimePoint> {
    (from..).map_while(|g| feed.time_of(g)).collect()
}

/// 字の現れる時刻は「発火時刻＋塊の頭からの隔たり」の絶対の時刻で、ミリ秒へ丸めない。発火時刻より前の
/// 「今」には、どの字も現れていない。
#[test]
fn glyph_times_are_the_fire_time_plus_the_offset_and_keep_sub_millisecond_precision() {
    let mut feed = TalkFeed::default();
    // 発火時刻 999.75 ms（9_997_500 ticks）・再生時間 1 ms・2 文字＝間隔 0.5 ms。
    let fire = TimePoint::from_ticks(9_997_500);
    feed.push_chunk(fire, ms(1), 2);
    assert_eq!(
        times_from(&feed, 0),
        [fire, TimePoint::from_ticks(10_002_500)],
        "999.75 ms と 1000.25 ms のまま（ミリ秒へ丸めない）"
    );
    assert_eq!(
        feed.revealed_until(TimePoint::from_ticks(9_997_499)),
        0,
        "発火時刻より前"
    );
    assert_eq!(feed.revealed_until(fire), 1);
    assert_eq!(feed.revealed_until(at_ms(1000)), 1);
    assert_eq!(feed.revealed_until(TimePoint::from_ticks(10_002_500)), 2);
}

/// 「前の字の時刻」は字の列と別の欄で、数え終えた字を全部刈り込んでも、次の塊は前の字から続く。
#[test]
fn previous_glyph_time_survives_pruning_the_whole_list() {
    let mut feed = TalkFeed::default();
    // 頭 100000ms・再生時間 1000ms・2 文字＝[100000, 100500] ms。
    feed.push_chunk(at_ms(100_000), ms(1000), 2);
    feed.prune_before(2);
    assert_eq!(feed.time_of(1), None, "列は空");
    // 頭 100200ms・500ms・2 文字＝間隔 250ms: 前の字 100500ms＋250ms から続く。
    feed.push_chunk(at_ms(100_200), ms(500), 2);
    assert_eq!(times_from(&feed, 2), [at_ms(100_750), at_ms(101_000)]);
}

/// 塊ごとに「前の文字＋1 字の間隔」と「塊の頭」の大きい方で 1 文字ずつ積み、序数は塊をまたいで続く。
#[test]
fn chunk_times_follow_the_text_layer_formula_across_chunks() {
    let mut feed = TalkFeed::default();

    // 1 つ目の塊（頭 1.0・再生時間 0.5・4 文字＝間隔 0.125）: 先頭は塊の頭、後は間隔ずつ。
    feed.push_chunk(at_ms(1000), ms(500), 4);
    // 待ちの後の塊（頭 2.0・0.5・2 文字＝間隔 0.25）: 前の文字＋間隔は 1.625 で、塊の頭 2.0 の方が大きい。
    feed.push_chunk(at_ms(2000), ms(500), 2);
    // 頭が前の文字に追い付かれている塊（頭 2.0・1.0・2 文字＝間隔 0.5）: 前の文字 2.25＋0.5 の方が大きい。
    feed.push_chunk(at_ms(2000), ms(1000), 2);
    // 再生時間 0 の塊（頭 4.0・3 文字＝間隔 0）: 全部が塊の頭で同時に現れる。
    feed.push_chunk(at_ms(4000), ms(0), 3);
    // 文字の無い塊は何も足さず、次の塊の計算にも影響しない。
    feed.push_chunk(at_ms(9000), ms(1000), 0);
    // 最後の塊（頭 4.0・0.25・1 文字）: 前の文字 4.0＋0.25。
    feed.push_chunk(at_ms(4000), ms(250), 1);

    assert_eq!(
        times_from(&feed, 0),
        [
            at_ms(1000),
            at_ms(1125),
            at_ms(1250),
            at_ms(1375), // 1 つ目
            at_ms(2000),
            at_ms(2250), // 待ちの後
            at_ms(2750),
            at_ms(3250), // 追い付かれた塊
            at_ms(4000),
            at_ms(4000),
            at_ms(4000), // 同時
            at_ms(4250), // 最後
        ]
    );
    assert_eq!(feed.time_of(4), Some(at_ms(2000)), "序数は塊をまたいで連続");
    assert_eq!(feed.time_of(12), None, "まだ無い序数");

    // ある時刻までに現れた文字の数（ちょうどの時刻は現れている）。
    assert_eq!(feed.revealed_until(at_ms(500)), 0);
    assert_eq!(feed.revealed_until(at_ms(1000)), 1);
    assert_eq!(feed.revealed_until(at_ms(1300)), 3);
    assert_eq!(feed.revealed_until(at_ms(2000)), 5);
    assert_eq!(feed.revealed_until(at_ms(4000)), 11);
    assert_eq!(feed.revealed_until(at_ms(99000)), 12);
}

/// 文字は書記素クラスタで数える（結合文字つき・絵文字の列・国旗はそれぞれ 1 文字）。
#[test]
fn grapheme_clusters_count_as_one_character_each() {
    let mut feed = TalkFeed::default();
    // か゚＋家族の絵文字（ZWJ 列）＋国旗＋a＝4 文字（スカラー値では 10 個）。再生時間 0.5＝間隔 0.125。
    feed.push_text(
        at_ms(0),
        ms(500),
        "\u{304B}\u{309A}👨\u{200D}👩\u{200D}👧🇯🇵a",
    );
    assert_eq!(
        times_from(&feed, 0),
        [at_ms(0), at_ms(125), at_ms(250), at_ms(375)]
    );

    // 空の文字列は何も足さない。
    feed.push_text(at_ms(5000), ms(1000), "");
    assert_eq!(feed.revealed_until(at_ms(99000)), 4);
}

/// 消去の後の塊: 消去の時刻までに現れていない文字は捨てられ、次の塊は塊の頭から始まる。
#[test]
fn clear_drops_unrevealed_characters_and_restarts_at_the_chunk_head() {
    let mut feed = TalkFeed::default();
    // 頭 0.0・再生時間 1.0・4 文字＝[0.0, 0.25, 0.5, 0.75]。
    feed.push_chunk(at_ms(0), ms(1000), 4);

    // 0.5 で消去: 0.75 の文字は現れないまま消える（ちょうど 0.5 の文字は現れている）。
    feed.restart_chain(at_ms(500));
    assert_eq!(times_from(&feed, 0), [at_ms(0), at_ms(250), at_ms(500)]);
    assert_eq!(
        feed.revealed_until(at_ms(99000)),
        3,
        "現れなかった文字は数に入らない"
    );

    // 消去の後の塊（頭 0.5・0.5・2 文字＝間隔 0.25）: 文字の層は空の列から始めるので [0.5, 0.75]
    // （前の文字 0.5＋0.25 から続けた [0.75, 1.0] ではない）。
    feed.push_chunk(at_ms(500), ms(500), 2);
    assert_eq!(
        times_from(&feed, 0),
        [at_ms(0), at_ms(250), at_ms(500), at_ms(500), at_ms(750)]
    );

    // 見えている数＝消去の前に現れた 3 文字＋文字の層が消去の後に見せる数（0.5 で 1・0.75 で 2）。
    assert_eq!(feed.revealed_until(at_ms(500)), 3 + 1);
    assert_eq!(feed.revealed_until(at_ms(625)), 3 + 1);
    assert_eq!(feed.revealed_until(at_ms(750)), 3 + 2);
}

/// 中断→新しい台詞の頭の全消去（その発火時刻 600 ms）→新しい塊: 前の台詞の現れなかった文字が残らず、
/// 列は単調のまま。
#[test]
fn interrupt_then_clear_all_at_the_new_fire_time_keeps_the_list_monotone() {
    let mut feed = TalkFeed::default();
    // 前の台詞（頭 0.0・再生時間 2.0・8 文字＝間隔 0.25）が途中で中断された。
    feed.push_chunk(at_ms(0), ms(2000), 8);

    // 新しい台詞の頭の全消去（発火時刻 0.6）: それより後の文字（0.75〜1.75）を捨てる。
    feed.restart_chain(at_ms(600));
    assert_eq!(times_from(&feed, 0), [at_ms(0), at_ms(250), at_ms(500)]);

    // 新しい台詞の塊（頭 0.6・0.5・4 文字＝間隔 0.125）: 文字の層の列は [0.6, 0.725, 0.85, 0.975]。
    feed.push_chunk(at_ms(600), ms(500), 4);
    assert_eq!(
        times_from(&feed, 0),
        [
            at_ms(0),
            at_ms(250),
            at_ms(500),
            at_ms(600),
            at_ms(725),
            at_ms(850),
            at_ms(975)
        ]
    );

    // 見えている数＝残った 3 文字＋文字の層が見せる数（0.6 で 1・0.8 で 2・0.975 で 4）。
    // 前の台詞の時刻（1.0・1.25…）が来ても、現れなかった文字で数は増えない。
    assert_eq!(feed.revealed_until(at_ms(600)), 3 + 1);
    assert_eq!(feed.revealed_until(at_ms(800)), 3 + 2);
    assert_eq!(feed.revealed_until(at_ms(975)), 3 + 4);
    assert_eq!(feed.revealed_until(at_ms(99000)), 3 + 4);
}

/// 刈り込みは序数を変えない（捨てた文字の後ろの文字・後から届く文字の序数がずれない）。
#[test]
fn pruning_keeps_ordinals() {
    let mut feed = TalkFeed::default();
    // [0.0, 0.25, 0.5, 0.75]
    feed.push_chunk(at_ms(0), ms(1000), 4);

    feed.prune_before(2);
    assert_eq!(feed.time_of(1), None, "刈り込んだ序数");
    assert_eq!(feed.time_of(2), Some(at_ms(500)));
    assert_eq!(feed.time_of(3), Some(at_ms(750)));
    assert_eq!(feed.revealed_until(at_ms(500)), 3);
    // 手前の序数を指しても戻らない。
    feed.prune_before(0);
    assert_eq!(feed.time_of(2), Some(at_ms(500)));

    // 全部刈り込んだ後の塊（頭 0.5・0.5・2 文字＝間隔 0.25）: 前の文字 0.75＋0.25 から続き、序数は 4 から。
    feed.prune_before(4);
    feed.push_chunk(at_ms(500), ms(500), 2);
    assert_eq!(times_from(&feed, 4), [at_ms(1000), at_ms(1250)]);
    assert_eq!(feed.revealed_until(at_ms(1000)), 5);

    // 末尾より先を指しても、序数は在る文字の数までしか進まない。
    feed.prune_before(100);
    assert_eq!(feed.revealed_until(at_ms(99000)), 6);
    feed.push_chunk(at_ms(5000), ms(0), 1);
    assert_eq!(feed.time_of(6), Some(at_ms(5000)));

    // 中断された台詞の続き: 数え終えた文字を刈り込んだ後の全消去でも、序数は続きから。
    // 頭 5.0・再生時間 3.0・3 文字＝間隔 1.0 を足す（前の文字 5.0 から続いて [6.0, 7.0, 8.0]）。
    // 6.0 まで数え終え（序数 8 の手前まで）、新しい台詞の頭（発火時刻 6.5）で残りの 2 文字を捨てる。
    feed.push_chunk(at_ms(5000), ms(3000), 3);
    feed.prune_before(feed.revealed_until(at_ms(6000)));
    feed.restart_chain(at_ms(6500));
    assert_eq!(feed.revealed_until(at_ms(99000)), 8);
    feed.push_chunk(at_ms(6500), ms(0), 1);
    assert_eq!(feed.time_of(8), Some(at_ms(6500)));
}
