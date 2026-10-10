//! 文字の写し（起点の見積もり・文字の時刻の列）の決定論テスト
//! （spec: areka-P0-seriko-trigger-intervals 要件 4.2・4.3・4.10・6.3・6.4・9.4）。
//!
//! 期待の列は、文字の層の式から手で求めた固定の値（文字の層の crate は呼ばない）:
//! i 文字目＝「前の文字＋1 字の間隔」と「塊の頭」の大きい方・1 字の間隔＝塊の再生時間÷文字数・
//! 列の先頭と消去の直後の文字は塊の頭・ある時刻に見えている数＝その時刻以前の文字の数。
//! 台本の秒は 2 進で割り切れる値だけを使い、列を誤差なしの一致で比べる。列が単調であることは、
//! 単調な固定の列との一致で確かめる。

use super::*;

/// 序数 `from` から末尾までの文字の、台本の秒の列。
fn times_from(feed: &TalkFeed, from: u64) -> Vec<f64> {
    (from..).map_while(|g| feed.time_of(g)).collect()
}

/// 起点は届いた cue ごとの「今 − cue の時刻」の最大（新しいトークで前へ飛び、小さい値には負けない）。
#[test]
fn epoch_is_the_largest_now_minus_cue_time() {
    let mut epoch = TalkEpoch::default();
    assert_eq!(epoch.talk_time(100_000), None, "cue が届く前は起点が無い");
    assert_eq!(epoch.wall_ms(0.5), None);

    // 100 秒目に台本の 2 秒の cue: 起点は 98 秒目。
    epoch.observe(2.0, 100_000);
    assert_eq!(epoch.talk_time(100_000), Some(2.0));
    assert_eq!(epoch.wall_ms(0.5), Some(98_500));

    // 早く届いた cue（98.5 秒目に台本の 1 秒＝候補は 97.5 秒目）は起点を下げない。
    epoch.observe(1.0, 98_500);
    assert_eq!(epoch.talk_time(100_000), Some(2.0));

    // 新しいトークの頭（200 秒目に台本の 0 秒）で、起点は前へ飛ぶ。
    epoch.observe(0.0, 200_000);
    assert_eq!(epoch.talk_time(200_000), Some(0.0));
    assert_eq!(epoch.talk_time(200_250), Some(0.25));
    assert_eq!(epoch.wall_ms(0.5), Some(200_500));
    // 起点より前の時刻は台本の 0 秒（負にしない）。
    assert_eq!(epoch.talk_time(199_000), Some(0.0));
}

/// 壁時刻へは最も近い 1 ms へ丸めて写す（切り捨てない）。
#[test]
fn wall_time_rounds_to_the_nearest_millisecond() {
    let mut epoch = TalkEpoch::default();
    // 1000 ms 目に台本の 0.25 ms の cue: 起点は 999.75 ms。
    epoch.observe(0.000_25, 1000);
    assert_eq!(
        epoch.wall_ms(0.0),
        Some(1000),
        "999.75 は 1000（切り捨てなら 999）"
    );
    assert_eq!(epoch.wall_ms(0.000_5), Some(1000), "1000.25 は 1000");
    assert_eq!(epoch.wall_ms(0.001), Some(1001), "1000.75 は 1001");
}

/// 塊ごとに「前の文字＋1 字の間隔」と「塊の頭」の大きい方で 1 文字ずつ積み、序数は塊をまたいで続く。
#[test]
fn chunk_times_follow_the_text_layer_formula_across_chunks() {
    let mut feed = TalkFeed::default();

    // 1 つ目の塊（頭 1.0・再生時間 0.5・4 文字＝間隔 0.125）: 先頭は塊の頭、後は間隔ずつ。
    feed.push_chunk(1.0, 0.5, 4);
    // 待ちの後の塊（頭 2.0・0.5・2 文字＝間隔 0.25）: 前の文字＋間隔は 1.625 で、塊の頭 2.0 の方が大きい。
    feed.push_chunk(2.0, 0.5, 2);
    // 頭が前の文字に追い付かれている塊（頭 2.0・1.0・2 文字＝間隔 0.5）: 前の文字 2.25＋0.5 の方が大きい。
    feed.push_chunk(2.0, 1.0, 2);
    // 再生時間 0 の塊（頭 4.0・3 文字＝間隔 0）: 全部が塊の頭で同時に現れる。
    feed.push_chunk(4.0, 0.0, 3);
    // 文字の無い塊は何も足さず、次の塊の計算にも影響しない。
    feed.push_chunk(9.0, 1.0, 0);
    // 最後の塊（頭 4.0・0.25・1 文字）: 前の文字 4.0＋0.25。
    feed.push_chunk(4.0, 0.25, 1);

    assert_eq!(
        times_from(&feed, 0),
        [
            1.0, 1.125, 1.25, 1.375, // 1 つ目
            2.0, 2.25, // 待ちの後
            2.75, 3.25, // 追い付かれた塊
            4.0, 4.0, 4.0,  // 同時
            4.25, // 最後
        ]
    );
    assert_eq!(feed.time_of(4), Some(2.0), "序数は塊をまたいで連続");
    assert_eq!(feed.time_of(12), None, "まだ無い序数");

    // ある台本の秒までに現れた文字の数（ちょうどの時刻は現れている）。
    assert_eq!(feed.revealed_until(0.5), 0);
    assert_eq!(feed.revealed_until(1.0), 1);
    assert_eq!(feed.revealed_until(1.3), 3);
    assert_eq!(feed.revealed_until(2.0), 5);
    assert_eq!(feed.revealed_until(4.0), 11);
    assert_eq!(feed.revealed_until(99.0), 12);
}

/// 文字は書記素クラスタで数える（結合文字つき・絵文字の列・国旗はそれぞれ 1 文字）。
#[test]
fn grapheme_clusters_count_as_one_character_each() {
    let mut feed = TalkFeed::default();
    // か゚＋家族の絵文字（ZWJ 列）＋国旗＋a＝4 文字（スカラー値では 10 個）。再生時間 0.5＝間隔 0.125。
    feed.push_text(0.0, 0.5, "\u{304B}\u{309A}👨\u{200D}👩\u{200D}👧🇯🇵a");
    assert_eq!(times_from(&feed, 0), [0.0, 0.125, 0.25, 0.375]);

    // 空の文字列は何も足さない。
    feed.push_text(5.0, 1.0, "");
    assert_eq!(feed.revealed_until(99.0), 4);
}

/// 消去の後の塊: 消去の時刻までに現れていない文字は捨てられ、次の塊は塊の頭から始まる。
#[test]
fn clear_drops_unrevealed_characters_and_restarts_at_the_chunk_head() {
    let mut feed = TalkFeed::default();
    // 頭 0.0・再生時間 1.0・4 文字＝[0.0, 0.25, 0.5, 0.75]。
    feed.push_chunk(0.0, 1.0, 4);

    // 0.5 で消去: 0.75 の文字は現れないまま消える（ちょうど 0.5 の文字は現れている）。
    feed.restart_chain(0.5);
    assert_eq!(times_from(&feed, 0), [0.0, 0.25, 0.5]);
    assert_eq!(
        feed.revealed_until(99.0),
        3,
        "現れなかった文字は数に入らない"
    );

    // 消去の後の塊（頭 0.5・0.5・2 文字＝間隔 0.25）: 文字の層は空の列から始めるので [0.5, 0.75]
    // （前の文字 0.5＋0.25 から続けた [0.75, 1.0] ではない）。
    feed.push_chunk(0.5, 0.5, 2);
    assert_eq!(times_from(&feed, 0), [0.0, 0.25, 0.5, 0.5, 0.75]);

    // 見えている数＝消去の前に現れた 3 文字＋文字の層が消去の後に見せる数（0.5 で 1・0.75 で 2）。
    assert_eq!(feed.revealed_until(0.5), 3 + 1);
    assert_eq!(feed.revealed_until(0.625), 3 + 1);
    assert_eq!(feed.revealed_until(0.75), 3 + 2);
}

/// 中断→全消去（時刻 0）→新しい塊: 前の台詞の現れなかった文字が残らず、列は単調のまま。
#[test]
fn interrupt_then_clear_all_at_zero_keeps_the_list_monotone() {
    let mut feed = TalkFeed::default();
    // 前の台詞（頭 0.0・再生時間 2.0・8 文字＝間隔 0.25）が途中で中断された。
    feed.push_chunk(0.0, 2.0, 8);

    // 新しい台詞の頭の全消去は台本の 0 秒: それより後の文字（0.25〜1.75）を捨てる。
    feed.restart_chain(0.0);
    assert_eq!(times_from(&feed, 0), [0.0]);

    // 新しい台詞の塊（頭 0.0・0.5・4 文字＝間隔 0.125）: 文字の層の列は [0.0, 0.125, 0.25, 0.375]。
    feed.push_chunk(0.0, 0.5, 4);
    assert_eq!(times_from(&feed, 0), [0.0, 0.0, 0.125, 0.25, 0.375]);

    // 見えている数＝残った 1 文字＋文字の層が見せる数（0.0 で 1・0.2 で 2・0.375 で 4）。
    // 前の台詞の時刻（0.5・1.0…）が来ても、現れなかった文字で数は増えない。
    assert_eq!(feed.revealed_until(0.0), 1 + 1);
    assert_eq!(feed.revealed_until(0.2), 1 + 2);
    assert_eq!(feed.revealed_until(0.375), 1 + 4);
    assert_eq!(feed.revealed_until(99.0), 1 + 4);
}

/// 刈り込みは序数を変えない（捨てた文字の後ろの文字・後から届く文字の序数がずれない）。
#[test]
fn pruning_keeps_ordinals() {
    let mut feed = TalkFeed::default();
    // [0.0, 0.25, 0.5, 0.75]
    feed.push_chunk(0.0, 1.0, 4);

    feed.prune_before(2);
    assert_eq!(feed.time_of(1), None, "刈り込んだ序数");
    assert_eq!(feed.time_of(2), Some(0.5));
    assert_eq!(feed.time_of(3), Some(0.75));
    assert_eq!(feed.revealed_until(0.5), 3);
    // 手前の序数を指しても戻らない。
    feed.prune_before(0);
    assert_eq!(feed.time_of(2), Some(0.5));

    // 全部刈り込んだ後の塊（頭 0.5・0.5・2 文字＝間隔 0.25）: 前の文字 0.75＋0.25 から続き、序数は 4 から。
    feed.prune_before(4);
    feed.push_chunk(0.5, 0.5, 2);
    assert_eq!(times_from(&feed, 4), [1.0, 1.25]);
    assert_eq!(feed.revealed_until(1.0), 5);

    // 末尾より先を指しても、序数は在る文字の数までしか進まない。
    feed.prune_before(100);
    assert_eq!(feed.revealed_until(99.0), 6);
    feed.push_chunk(5.0, 0.0, 1);
    assert_eq!(feed.time_of(6), Some(5.0));

    // 中断された台詞の続き: 数え終えた文字を刈り込んだ後の全消去でも、序数は続きから。
    // 頭 5.0・再生時間 3.0・3 文字＝間隔 1.0 を足す（前の文字 5.0 から続いて [6.0, 7.0, 8.0]）。
    // 6.0 まで数え終え（序数 8 の手前まで）、新しい台詞の頭（0 秒）で残りの 2 文字を捨てる。
    feed.push_chunk(5.0, 3.0, 3);
    feed.prune_before(feed.revealed_until(6.0));
    feed.restart_chain(0.0);
    assert_eq!(feed.revealed_until(99.0), 8);
    feed.push_chunk(0.0, 0.0, 1);
    assert_eq!(feed.time_of(8), Some(0.0));
}
