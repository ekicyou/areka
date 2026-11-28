//! 場所（普通のバルーン／箱）ごとの前の字と、時刻の順の序数を持つ字の列の決定論テスト。
//!
//! 期待の値は、文字の層の式（場所ごとの列の最後の字を前の字にする・前の字＋1 字の間隔が塊の頭に
//! 届かなければ塊の頭から）を、場所ごとに手で当てて求めた固定の値。シェルは面 0 が箱なし、面 10 に
//! 箱 `fuda`（element1）と `kami`（element2）。

use std::collections::BTreeMap;

use areka_emo_compose::{BoxName, EmoWorld, SurfaceKeyOutcome, TextPlace, box_index, fold_boxes};
use areka_parsers::shell::{parse, parse_boxes};
use dola::time::testing::{at_ms, delta_ms, ms};

use super::*;

const SHELL: &str = "\
balloon.fuda
{
size,100,50
}
balloon.kami
{
size,80,40
}
surface10
{
element1,balloon,fuda,0,0
element2,balloon,kami,0,60
}
";

fn routing() -> TextRouting {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    TextRouting {
        boxes: box_index(&layout),
        surface_ids: [0, 10].into(),
    }
}

fn boxed(routing: &TextRouting, name: &str) -> TextPlace {
    let found: &BoxName = routing.boxes[&10]
        .iter()
        .find(|n| n.as_str() == name)
        .expect("面 10 の箱");
    TextPlace::Box(found.clone())
}

fn times_from(feed: &TalkFeed, from: u64) -> Vec<TimePoint> {
    (from..).map_while(|g| feed.time_of(g)).collect()
}

fn at_each(ms: &[i64]) -> Vec<TimePoint> {
    ms.iter().map(|&m| at_ms(m)).collect()
}

/// `\s[10]\q[はい,yes]\s[0]つづき`（全部の合図が 1000 ms・選択肢の長さ 0・「つづき」は 150 ms）:
/// 普通のバルーンには前の字が無いので、「つづき」は塊の頭から並ぶ（選択肢の字から続けない）。
#[test]
fn text_after_a_choice_in_a_box_starts_at_its_own_head_in_the_balloon() {
    let routing = routing();
    let mut feed = TalkFeed::default();
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(10));
    assert_eq!(
        feed.route.dest(),
        &boxed(&routing, "fuda"),
        "面 10 の既定は先頭の箱"
    );
    feed.push_text(at_ms(1000), ms(0), "はい");
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(0));
    assert_eq!(feed.route.dest(), &TextPlace::Balloon);
    feed.push_text(at_ms(1000), ms(150), "つづき");

    assert_eq!(
        times_from(&feed, 0),
        at_each(&[1000, 1000, 1000, 1050, 1100])
    );
}

/// 速い塊と遅い塊を別の場所へ交互に積む: どちらも自分の場所の前の字から続き、列は時刻の順に並ぶ
/// （同じ時刻は届いた順）。
#[test]
fn chunks_of_different_pace_continue_within_their_own_places() {
    let routing = routing();
    let mut feed = TalkFeed::default();
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(10));
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(0));
    // 普通のバルーン: 頭 0・100 ms・2 字＝[0, 50]。
    feed.push_chunk(at_ms(0), ms(100), 2);
    // 箱: 頭 0・400 ms・2 字＝[0, 200]（普通のバルーンの 50 から続けない）。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(10));
    feed.push_chunk(at_ms(0), ms(400), 2);
    // 普通のバルーン: 前の字 50 から 50 ずつ＝[100, 150]。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(0));
    feed.push_chunk(at_ms(0), ms(100), 2);
    // 箱: 前の字 200 から 200 ずつ＝[400, 600]（普通のバルーンの 150 からではない）。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(10));
    feed.push_chunk(at_ms(0), ms(400), 2);

    assert_eq!(
        times_from(&feed, 0),
        at_each(&[0, 0, 50, 100, 150, 200, 400, 600])
    );
    assert_eq!(feed.revealed_until(at_ms(150)), 5);
}

/// 普通のバルーンと箱 `fuda` に 4 字ずつ（頭 0・400 ms＝[0, 100, 200, 300]）。列は
/// 0:風0 1:箱0 2:風100 3:箱100 4:風200 5:箱200 6:風300 7:箱300。行き先は箱 `fuda`。
fn two_places_of_four(routing: &TextRouting) -> TalkFeed {
    let mut feed = TalkFeed::default();
    feed.push_chunk(at_ms(0), ms(400), 4);
    feed.follow_surface(routing, SurfaceKeyOutcome::Show(10));
    feed.push_chunk(at_ms(0), ms(400), 4);
    feed
}

/// `\c` は行き先の場所の、発火時刻より後に現れる字だけを捨て、消える前の序数を昇順で返す。
/// 捨てた場所は塊の頭から始め直し、ほかの場所は前の字から続く。
#[test]
fn clear_drops_only_the_destination_place() {
    let routing = routing();
    let mut feed = two_places_of_four(&routing);
    feed.prune_before(2);

    assert_eq!(feed.restart_chain(at_ms(150)), [5, 7]);
    assert_eq!(times_from(&feed, 2), at_each(&[100, 100, 200, 300]));

    // 箱: 頭 150・100 ms・2 字＝[150, 200]（前の字は無しに戻った）。同じ 200 は届いた順で後ろ。
    feed.push_chunk(at_ms(150), ms(100), 2);
    // 普通のバルーン: 前の字 300 から＝[400]。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(0));
    feed.push_chunk(at_ms(0), ms(100), 1);
    assert_eq!(
        times_from(&feed, 2),
        at_each(&[100, 100, 150, 200, 200, 300, 400])
    );
}

/// 全消去は全部の場所の発火時刻より後の字を捨て、全部の場所の前の字を無しにし、行き先を既定へ戻す。
#[test]
fn clear_all_drops_every_place_and_resets_the_destination() {
    let routing = routing();
    let mut feed = two_places_of_four(&routing);
    feed.select_box(&routing, "kami");
    assert_eq!(feed.route.dest(), &boxed(&routing, "kami"));

    assert_eq!(feed.restart_all(at_ms(150), &routing), [4, 5, 6, 7]);
    assert_eq!(feed.route.dest(), &boxed(&routing, "fuda"), "面 10 の既定");
    assert_eq!(times_from(&feed, 0), at_each(&[0, 0, 100, 100]));

    // どの場所も塊の頭から: 箱 `fuda` に頭 150・100 ms・2 字、普通のバルーンに頭 150・100 ms・1 字。
    feed.push_chunk(at_ms(150), ms(100), 2);
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(0));
    feed.push_chunk(at_ms(150), ms(100), 1);
    assert_eq!(
        times_from(&feed, 0),
        at_each(&[0, 0, 100, 100, 150, 150, 200])
    );
}

/// `あ\![*]いう`（「あ」は 0 から 50 ms・印は 50 で長さ 0・「いう」は 50 から 100 ms）: 印は前の字を
/// 50 へ進めるだけで序数を作らないので、「いう」は 100・150。
#[test]
fn a_marker_advances_the_previous_glyph_without_an_ordinal() {
    let mut feed = TalkFeed::default();
    feed.push_text(at_ms(0), ms(50), "あ");
    feed.push_mark(at_ms(50), ms(0));
    feed.push_text(at_ms(50), ms(100), "いう");

    assert_eq!(times_from(&feed, 0), at_each(&[0, 100, 150]));
    assert_eq!(feed.revealed_until(at_ms(99_000)), 3, "印は数えない");
}

/// 印は行き先の場所の前の字だけを進める（ほかの場所の続きは変えない）。
#[test]
fn a_marker_advances_only_the_destination_place() {
    let routing = routing();
    let mut feed = TalkFeed::default();
    // 普通のバルーン: [0]。
    feed.push_chunk(at_ms(0), ms(100), 1);
    // 箱: 印を 500 に置く（前の字が無いので塊の頭 500）。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(10));
    feed.push_mark(at_ms(500), ms(0));
    // 普通のバルーン: 前の字 0 から 100 ずつ＝[100]（箱の印の 500 からではない）。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(0));
    feed.push_chunk(at_ms(0), ms(100), 1);
    // 箱: 印の 500 から 100 ずつ＝[600]。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(10));
    feed.push_chunk(at_ms(0), ms(100), 1);

    assert_eq!(times_from(&feed, 0), at_each(&[0, 100, 600]));
}

/// 後から届いた別の場所の早い字は、時刻の順の位置へ差し込まれる（列は時刻について単調のまま）。
/// 序数は差し込みの後の順で、消去が返す序数もその順で数える。
#[test]
fn a_late_earlier_glyph_is_inserted_in_time_order() {
    let routing = routing();
    let mut feed = TalkFeed::default();
    // 箱: 頭 0・1000 ms・2 字＝[0, 500]。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(10));
    feed.push_chunk(at_ms(0), ms(1000), 2);
    // 普通のバルーン: 頭 0・400 ms・2 字＝[0, 200]。0 は箱の 0 の後ろ、200 は箱の 500 の前。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(0));
    feed.push_chunk(at_ms(0), ms(400), 2);

    assert_eq!(times_from(&feed, 0), at_each(&[0, 0, 200, 500]));
    assert_eq!(feed.revealed_until(at_ms(200)), 3);
    // 普通のバルーンの `\c`: 200 の字は序数 2。
    assert_eq!(feed.restart_chain(at_ms(100)), [2]);
    assert_eq!(times_from(&feed, 0), at_each(&[0, 0, 500]));
}

/// 場所が 1 つ（箱だけ）なら、時刻と消去の序数は普通のバルーンだけの今の列と同じ。
#[test]
fn a_single_place_matches_the_balloon_only_list() {
    let routing = routing();
    let drive = |feed: &mut TalkFeed| -> Vec<u64> {
        feed.push_chunk(at_ms(1000), ms(500), 4);
        feed.push_chunk(at_ms(2000), ms(500), 2);
        feed.push_mark(at_ms(2000), ms(100));
        feed.push_chunk(at_ms(2000), ms(1000), 2);
        let dropped = feed.restart_chain(at_ms(2600));
        feed.push_chunk(at_ms(2600), ms(500), 2);
        feed.shift_after(at_ms(2700), delta_ms(300));
        dropped
    };
    let mut balloon = TalkFeed::default();
    let mut boxed = TalkFeed::default();
    boxed.follow_surface(&routing, SurfaceKeyOutcome::Show(10));

    assert_eq!(drive(&mut balloon), drive(&mut boxed));
    assert_eq!(times_from(&balloon, 0), times_from(&boxed, 0));
    // 1 つ目 [1000, 1125, 1250, 1375]・2 つ目 [2000, 2250]・印は 2350・3 つ目 [2850, 3350] を
    // 2600 で切って捨て、[2600, 2850] を積み、2700 より後の 2850 を 300 動かす。
    assert_eq!(
        times_from(&balloon, 0),
        at_each(&[1000, 1125, 1250, 1375, 2000, 2250, 2600, 3150])
    );
}

/// 置き直しの知らせは全部の場所の字と、全部の場所の前の字へ当たる。
#[test]
fn a_shift_notice_moves_every_place() {
    let routing = routing();
    let mut feed = two_places_of_four(&routing);
    feed.shift_after(at_ms(150), delta_ms(1000));
    assert_eq!(
        times_from(&feed, 0),
        at_each(&[0, 0, 100, 100, 1200, 1200, 1300, 1300])
    );
    // 箱の前の字は 1300 へ動いた: 頭 0・400 ms・1 字は 1700。
    feed.push_chunk(at_ms(0), ms(400), 1);
    // 普通のバルーンの前の字も 1300 へ動いた: 頭 0・400 ms・1 字は 1700。
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(0));
    feed.push_chunk(at_ms(0), ms(400), 1);
    assert_eq!(times_from(&feed, 8), at_each(&[1700, 1700]));
}

/// 解決できない `\s` は行き先を変えない。名前の形の `\b` は今の面の箱へ切り替え、無い名前では変えない。
#[test]
fn routing_follows_surfaces_and_box_names() {
    let routing = routing();
    let mut feed = TalkFeed::default();
    assert_eq!(feed.route.dest(), &TextPlace::Balloon, "まだ `\\s` が無い");
    feed.follow_surface(&routing, SurfaceKeyOutcome::Show(10));
    feed.select_box(&routing, "kami");
    assert_eq!(feed.route.dest(), &boxed(&routing, "kami"));
    feed.select_box(&routing, "nai");
    assert_eq!(feed.route.dest(), &boxed(&routing, "kami"), "無い名前");
    feed.follow_surface(&routing, SurfaceKeyOutcome::Unresolved);
    assert_eq!(feed.route.dest(), &boxed(&routing, "kami"), "解決できない");
    feed.follow_surface(&routing, SurfaceKeyOutcome::Hide);
    assert_eq!(feed.route.dest(), &TextPlace::Balloon);
}

/// 番号からの種まき: 番号があればその面の既定、無ければ普通のバルーン。
#[test]
fn seeding_from_a_surface_id_sets_its_default_destination() {
    let routing = routing();
    let mut feed = TalkFeed::default();
    feed.seed_surface(&routing, Some(10));
    assert_eq!(feed.route.dest(), &boxed(&routing, "fuda"));
    assert_eq!(feed.route.surface(), Some(10));
    feed.seed_surface(&routing, None);
    assert_eq!(feed.route.dest(), &TextPlace::Balloon);
    assert_eq!(feed.route.surface(), None);
}

/// シェルの切替: 箱の場所の字のうち指定の時刻より後の物だけを捨て、箱の前の字を無しにし、新しい表で
/// 行き先を決め直す（今の番号が新しいシェルに無ければ、候補の最初に在る番号へ）。
#[test]
fn rebasing_boxes_drops_late_box_glyphs_and_reroutes() {
    let routing = routing();
    let mut feed = two_places_of_four(&routing);
    let next = TextRouting {
        boxes: routing.boxes.clone(),
        surface_ids: [0].into(),
    };

    assert_eq!(feed.rebase_boxes(at_ms(150), &next, &[0]), [5, 7]);
    assert_eq!(feed.route.surface(), Some(0), "候補の最初に在る番号");
    assert_eq!(feed.route.dest(), &TextPlace::Balloon);
    assert_eq!(times_from(&feed, 0), at_each(&[0, 0, 100, 100, 200, 300]));

    // 普通のバルーンは前の字 300 から続く。
    feed.push_chunk(at_ms(0), ms(100), 1);
    // 箱は前の字が無しに戻った: 頭 150・100 ms・1 字＝150。
    feed.follow_surface(&next, SurfaceKeyOutcome::Show(10));
    feed.push_chunk(at_ms(150), ms(100), 1);
    assert_eq!(
        times_from(&feed, 0),
        at_each(&[0, 0, 100, 100, 150, 200, 300, 400])
    );
}

/// 今の番号が新しいシェルに在れば、行き先は新しい表の既定へ戻るだけ。
#[test]
fn rebasing_boxes_keeps_a_surface_that_still_exists() {
    let routing = routing();
    let mut feed = two_places_of_four(&routing);
    feed.select_box(&routing, "kami");

    assert_eq!(
        feed.rebase_boxes(at_ms(400), &routing, &[0]),
        Vec::<u64>::new()
    );
    assert_eq!(feed.route.surface(), Some(10));
    assert_eq!(feed.route.dest(), &boxed(&routing, "fuda"));
}
