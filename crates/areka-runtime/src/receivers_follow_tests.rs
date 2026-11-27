//! 受け手どうしは、置き直し・遅れて着いた知らせ・次の台本・中断の後も、同じ字に同じ絶対の時刻を持って追従する。
//!
//! 台本は本物の再生機で刻み（[`play`]）、受け口へ出た物を、文字の層（窓なしの [`TextOnly`] か、実 GPU の
//! [`Frontend`]）・バルーンの寿命（[`Frontend`]）・seriko（[`seriko_at`]）へ与える。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::sink::EmoTextSink;
use areka_emo_text::state::TextLayerConfig;
use areka_sakura::ActorKey;
use dola::cue::{CueCommand, CuePlayer, CueSink, TalkCue};
use dola::time::testing::{at_ms, ms};
use dola::{TimePoint, TimeSpan};
use log_capture_kit::{CapturedEvent, capture};

use crate::same_script_test_support::{
    Frontend, Item, Played, STEP, SerikoLooks, deliver_until, grid, merged, mouth_starts, play,
    seriko_at,
};

/// 台本の起点。
const ORIGIN: i64 = 1_000;
/// 1 つの塊の字だけの台本（字は 50 ms ごと）。
const PLAIN: &str = r"\0\s[0]あいうえおかきくけこさしすせそ\e";
/// 前の再生の後に続く台本。
const NEXT: &str = r"\0\s[0]たちつてと\e";
/// 置き直しの効く時刻（刻みの時刻・字の現れる時刻のどれとも重ならない）。
const E: i64 = 1_192;
/// 観測の終わり（どの台本も、ここまでに字が現れ切る）。
const UNTIL: i64 = 2_400;

fn actor() -> ActorKey {
    ActorKey::from("0")
}

/// 窓を持たない文字の層の受け手（UI の受け取りと、フレームの頭の進める相だけ）。
struct TextOnly {
    runtime: Rc<RefCell<TextLayerRuntime>>,
    sink: EmoTextSink,
    _drain: Box<dyn std::any::Any>,
}

impl TextOnly {
    fn new() -> Self {
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )));
        let (sink, drain) = spawn_emo_text(Rc::clone(&runtime));
        Self {
            runtime,
            sink,
            _drain: Box::new(drain),
        }
    }

    fn deliver(&mut self, item: &Item) {
        match item {
            Item::Cue(cue) => self.sink.emit(cue.clone()),
            Item::Origin(notice) => self.sink.origin_set(*notice),
            Item::End => {}
        }
    }

    /// フレームを 1 つ回し、現れた字の数を返す。
    fn frame(&mut self, now: TimePoint) -> usize {
        crate::spine::pump_until_idle();
        let mut runtime = self.runtime.borrow_mut();
        let _ = runtime.advance(now);
        runtime.state().visible_glyphs(&actor(), now)
    }

    fn times(&self) -> Vec<TimePoint> {
        self.runtime
            .borrow()
            .state()
            .actor_state(&actor())
            .map(|text| text.reveal().times().to_vec())
            .unwrap_or_default()
    }
}

/// 文字の層の受け手の、フレームごとの字の数と、最後の字の現れる時刻。
struct TextRun {
    glyphs: BTreeMap<TimePoint, usize>,
    times: Vec<TimePoint>,
}

/// `played` を、それぞれの時刻に、16 ms ごとのフレームで文字の層へ与える。
fn on_time_text(played: &Played) -> TextRun {
    let mut text = TextOnly::new();
    let mut next = 0;
    let mut glyphs = BTreeMap::new();
    for frame in grid(ORIGIN, UNTIL, STEP) {
        for (_, item) in deliver_until(played, &mut next, frame, false) {
            text.deliver(item);
        }
        glyphs.insert(frame, text.frame(frame));
    }
    TextRun {
        glyphs,
        times: text.times(),
    }
}

/// 16 ms ごとの刻みで、`played` をそれぞれの時刻に受けた seriko の口パクの再生の始まり。
fn on_time_mouths(played: &Played) -> Vec<TimePoint> {
    let ticks = grid(ORIGIN, UNTIL, STEP);
    let looks: SerikoLooks = grid(ORIGIN, UNTIL, 5)
        .into_iter()
        .map(|t| (t, seriko_at(played, &ticks, t)))
        .collect();
    mouth_starts(&looks)
}

/// 前の起点が在る（置き直しの）知らせの、列の中の番号。
fn re_origin_index(played: &Played) -> usize {
    played
        .items
        .iter()
        .position(|(_, item)| matches!(item, Item::Origin(n) if n.previous.is_some()))
        .expect("前提: 置き直しの知らせが在る")
}

/// 置き直しの知らせの後、効く時刻より後の字だけが、動いた長さと向きのとおりに動く。前へ動いて時刻の
/// 来た字は効く時刻に寄り、効く時刻のフレームで一度に現れる。効く時刻より前のフレームは変わらない。
#[test]
fn a_re_origin_moves_only_the_glyphs_after_its_effective_time() {
    let e = at_ms(E);
    let base = on_time_text(&play(PLAIN, ORIGIN, ORIGIN, &[], None));
    let back = on_time_text(&play(PLAIN, ORIGIN, ORIGIN, &[(E, 150)], None));
    let fwd = on_time_text(&play(PLAIN, ORIGIN, ORIGIN, &[(E, -80)], None));
    assert_eq!(base.times.len(), 15, "前提: 15 字");
    assert!(
        base.times.iter().any(|&t| t < e) && base.times.iter().any(|&t| t > e),
        "前提: 効く時刻の前と後に字が在る: {:?}",
        base.times
    );

    let moved = |d: i64| -> Vec<TimePoint> {
        base.times
            .iter()
            .map(|&t| {
                if t <= e {
                    t
                } else if d >= 0 {
                    t + ms(d as u32)
                } else {
                    (t - ms(d.unsigned_abs() as u32)).max(e)
                }
            })
            .collect()
    };
    assert_eq!(back.times, moved(150), "後ろへ 150 ms");
    assert_eq!(fwd.times, moved(-80), "前へ 80 ms");

    for (&frame, &count) in base.glyphs.range(..e) {
        assert_eq!(
            back.glyphs[&frame], count,
            "後ろへ: 効く時刻より前（{frame}）"
        );
        assert_eq!(fwd.glyphs[&frame], count, "前へ: 効く時刻より前（{frame}）");
    }
    let before = base.glyphs[&at_ms(E - STEP)];
    assert_eq!(
        back.glyphs[&e], before,
        "後ろへ: 効く時刻以前に現れた字は現れたまま"
    );
    let at_once = base.times.iter().filter(|&&t| t > e && t - ms(80) <= e);
    assert!(
        at_once.clone().count() >= 2,
        "前提: 2 字以上が効く時刻に寄る"
    );
    assert_eq!(
        fwd.glyphs[&e],
        before + at_once.count(),
        "前へ: 時刻の来た字が効く時刻のフレームで一度に現れる"
    );
}

/// 前へ 20 ms と後ろへ 150 ms の置き直しをはさんでも、文字の層の字の現れる時刻と、seriko の口パクの
/// 始まり（どの字も 1 回の再生を始める）が、字ごとに同じ絶対の時刻である。
#[test]
fn the_text_layer_and_seriko_hold_the_same_time_for_each_glyph() {
    const BACKWARD: i64 = 1_400;
    let played = play(PLAIN, ORIGIN, ORIGIN, &[(E, -20), (BACKWARD, 150)], None);
    let times = on_time_text(&played).times;
    assert!(
        times.contains(&at_ms(E)),
        "前提: 前へ動いて効く時刻に寄った字が在る: {times:?}"
    );
    assert!(
        times
            .windows(2)
            .any(|w| w[0] <= at_ms(BACKWARD) && w[1] > w[0] + ms(150)),
        "前提: 後ろへ動いた字が在る: {times:?}"
    );
    assert_eq!(on_time_mouths(&played), times);
}

/// 寿命の線と文字の線の片方にだけ先に届いたフレームでも、可視性の相は、同じフレームの文字の層が見せる
/// 字の数で判断する（字が現れたフレームで出て、それより前は出ない）。
#[test]
fn a_frame_where_only_one_line_has_received_reads_the_same_glyph_count() {
    const LAG: i64 = 3 * STEP;
    let played = play(PLAIN, ORIGIN, ORIGIN, &[], None);
    for text_lags in [true, false] {
        let mut front = Frontend::new();
        let (mut text_next, mut life_next) = (0, 0);
        let mut first_glyph = None;
        for frame in grid(ORIGIN, ORIGIN + 400, STEP) {
            let (text_until, life_until) = if text_lags {
                (frame - ms(LAG as u32), frame)
            } else {
                (frame, frame - ms(LAG as u32))
            };
            for (_, item) in deliver_until(&played, &mut text_next, text_until, false) {
                front.deliver_text(item);
            }
            for (at, item) in deliver_until(&played, &mut life_next, life_until, false) {
                front.deliver_life(*at, item);
            }
            let look = front.frame(frame);
            if look.glyphs > 0 {
                first_glyph.get_or_insert(frame);
            }
            assert_eq!(
                look.balloon_shown,
                Some(look.glyphs > 0),
                "文字の線が遅れる={text_lags}（{frame}）: 字 {}",
                look.glyphs
            );
        }
        let expected = if text_lags {
            at_ms(ORIGIN + LAG)
        } else {
            at_ms(ORIGIN)
        };
        assert_eq!(
            first_glyph,
            Some(expected),
            "前提: 文字の線が遅れる={text_lags} の最初の字のフレーム"
        );
    }
}

/// 次の台本の合図を先にまとめて受けても、その発火時刻より前のフレームは、前の再生の姿（文字の層・
/// バルーンの寿命）のままである。発火時刻のフレームで次の台本が効く。
#[test]
fn frames_before_the_next_scripts_fire_time_keep_the_previous_look() {
    const NEXT_ORIGIN: i64 = 2_500;
    let previous = play(PLAIN, ORIGIN, ORIGIN, &[], None);
    let next = play(NEXT, NEXT_ORIGIN, NEXT_ORIGIN, &[], None);
    let frames = merged(
        grid(ORIGIN, NEXT_ORIGIN - 4, STEP),
        &[at_ms(NEXT_ORIGIN - 1)],
    );
    let run = |with_next: bool| {
        let mut front = Frontend::new();
        let mut cursor = 0;
        let mut next_given = !with_next;
        let mut looks = BTreeMap::new();
        for &frame in &frames {
            for (at, item) in deliver_until(&previous, &mut cursor, frame, false) {
                front.deliver(*at, item);
            }
            if !next_given && frame > previous.end {
                for (at, item) in &next.items {
                    front.deliver(*at, item);
                }
                next_given = true;
            }
            looks.insert(frame, front.frame(frame));
        }
        let at_fire = with_next.then(|| front.frame(at_ms(NEXT_ORIGIN)));
        (looks, at_fire)
    };
    let (alone, _) = run(false);
    let (with_next, at_fire) = run(true);
    for (frame, look) in &alone {
        assert_eq!(
            &with_next[frame], look,
            "次の台本の発火時刻より前（{frame}）"
        );
    }
    let last = &alone[&at_ms(NEXT_ORIGIN - 1)];
    assert_eq!(last.glyphs, 15, "前提: 前の再生の字が出ている");
    let at_fire = at_fire.expect("次の台本を受けた組");
    assert_eq!(
        (at_fire.glyphs, at_fire.clears > last.clears),
        (1, true),
        "発火時刻のフレームで、頭の消去と最初の字が当たる"
    );
}

/// 起点が先の時刻の台本を、最初の刻みの前に前へ置き直して刻むと、文字の層・寿命の時計・seriko のどれも、
/// 元の起点を待たずに、置き直した後の発火時刻で当たる。
#[test]
fn a_future_origin_moved_before_the_first_tick_applies_at_the_moved_times() {
    const FIRST_TICK: i64 = 1_000;
    const FUTURE: i64 = 3_000;
    const MOVED: i64 = 1_200;
    let played = play(
        PLAIN,
        FUTURE,
        FIRST_TICK,
        &[(FIRST_TICK, MOVED - FUTURE)],
        None,
    );
    let notice = &played.items[re_origin_index(&played)];
    assert!(
        matches!(notice, (_, Item::Origin(n)) if n.effective_at == at_ms(FIRST_TICK) && n.origin == at_ms(MOVED)),
        "前提: 最初の刻みの時刻に起点が {MOVED} へ動く: {notice:?}"
    );

    let mut front = Frontend::new();
    let mut next = 0;
    let frames = merged(
        grid(FIRST_TICK, MOVED + 200, STEP),
        &[at_ms(MOVED - 1), at_ms(MOVED)],
    );
    let mut shown = None;
    for &frame in &frames {
        for (at, item) in deliver_until(&played, &mut next, frame, false) {
            front.deliver(*at, item);
        }
        let look = front.frame(frame);
        if frame == at_ms(MOVED - 1) {
            assert_eq!(
                (look.glyphs, look.balloon_shown),
                (0, Some(false)),
                "置き直した後の発火時刻の前"
            );
        }
        if frame == at_ms(MOVED) {
            assert_eq!(look.reveal_times().first(), Some(&at_ms(MOVED)), "文字の層");
            shown = look.balloon_shown;
        }
    }
    assert_eq!(
        shown,
        Some(true),
        "寿命の時計: 置き直した後の発火時刻に出る"
    );

    let ticks = grid(FIRST_TICK, MOVED + 200, STEP);
    let looks: SerikoLooks = grid(FIRST_TICK, MOVED + 200, 5)
        .into_iter()
        .map(|t| (t, seriko_at(&played, &ticks, t)))
        .collect();
    assert_eq!(
        mouth_starts(&looks).first(),
        Some(&at_ms(MOVED)),
        "seriko: 最初の口パクは置き直した後の発火時刻"
    );
}

/// 中断された台詞のまだ現れていない字は、後始末の合図だけの台本（頭の消去が無い）が続いても、文字の層と
/// seriko で、中断しなかった時と同じ絶対の時刻に現れる。
#[test]
fn the_rest_of_an_interrupted_line_appears_at_its_original_times() {
    const CLEANUP: i64 = 1_300;
    let whole = on_time_text(&play(PLAIN, ORIGIN, ORIGIN, &[], None));
    let mut cut = play(PLAIN, ORIGIN, ORIGIN, &[], Some(E));
    cut.items.push((
        at_ms(CLEANUP),
        Item::Cue(TalkCue::scheduled(
            at_ms(CLEANUP),
            TimeSpan::ZERO,
            actor(),
            CueCommand::Emote { key: "0".into() },
            TimeSpan::ZERO,
        )),
    ));
    let text = on_time_text(&cut);
    assert!(
        text.times.iter().any(|&t| t > at_ms(CLEANUP)),
        "前提: 後始末の合図より後に現れる字が在る: {:?}",
        text.times
    );
    assert_eq!(text.times, whole.times, "文字の層の字の現れる時刻");
    assert_eq!(text.glyphs, whole.glyphs, "文字の層のフレームごとの字の数");
    assert_eq!(on_time_mouths(&cut), text.times, "seriko の口パクの始まり");
}

/// 後ろへ動かす知らせが、効く時刻より後のフレームを 3 つ回してから着いても、着いた後のフレームは、時刻
/// どおりに受けた受け手と同じ字を見せる。違うのは、効く時刻から着くまでの間のフレームだけである。
#[test]
fn a_late_backward_notice_matches_the_on_time_receiver_once_it_arrives() {
    const ARRIVES: i64 = E + 3 * STEP;
    let played = play(PLAIN, ORIGIN, ORIGIN, &[(E, 150)], None);
    let notice = re_origin_index(&played);
    let on_time = on_time_text(&played);

    let mut late = TextOnly::new();
    let mut next = 0;
    let mut ahead = 0;
    for frame in grid(ORIGIN, UNTIL, STEP) {
        let until = if frame < at_ms(ARRIVES) {
            notice
        } else {
            played.items.len()
        };
        while next < until && played.items[next].0 <= frame {
            late.deliver(&played.items[next].1);
            next += 1;
        }
        let glyphs = late.frame(frame);
        let expected = on_time.glyphs[&frame];
        if frame <= at_ms(E) || frame >= at_ms(ARRIVES) {
            assert_eq!(glyphs, expected, "{frame}");
        } else if glyphs != expected {
            assert!(glyphs > expected, "着く前は前の時刻のまま見せる（{frame}）");
            ahead += 1;
        }
    }
    assert!(
        ahead > 0,
        "前提: 着く前に、後で隠れる字を見せたフレームが在る"
    );
    assert_eq!(late.times(), on_time.times, "着いた後の字の現れる時刻");
}

/// 文節の折り返しで、全文なら「天気ですね。」が 2 行目へ回り、届いた「天気」までなら 1 行目に入る台本
/// （最初の塊の後の `\w9` で分かれる）。
const SPLIT: &str = r"\0\s[0]今日はとても良い天気\w9ですね。散歩に行きましょう。\e";

/// `script` を起点 `origin` に刻印した再生機へ、`front` の文字の層へ届く別の受け口を登録する
/// （先渡しが 1 度届く）。再生機と受け口の持ち主を返す。
fn register_preview(
    front: &Frontend,
    script: &str,
    origin: i64,
) -> (CuePlayer, Box<dyn std::any::Any>) {
    let compiled = areka_sakura::compile(
        &areka_parsers::sakura::parse(script),
        &areka_sakura::sysvar::SystemVarSnapshot::default(),
    );
    let mut player = CuePlayer::from_sheet(&compiled.sheet.with_absolute_start_time(at_ms(origin)))
        .expect("起点を刻んだ台本");
    let (sink, actor) = front.another_text_sink();
    player.register_sink(sink);
    (player, actor)
}

/// 先渡しを受けた記録の数。
fn previews_received(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.message() == "先渡しを受け取った——空回しで区間の全文を求めた")
        .count()
}

/// 前の再生の字が途中まで出ているところで、起点が先の時刻の台本の先渡しが当たっても、画面に出ている
/// 字とその後に現れる字の配置（描いた文字の面）が、先渡しを受けなかった時と同じフレームで同じである。
/// 前の再生は最初の塊の後で中断され、残りの塊は届かないので、配置は前の再生の区間の全文で決まっている。
#[test]
fn a_preview_of_a_future_script_keeps_the_shown_glyphs_in_place() {
    const REGISTER: i64 = ORIGIN + 13 * STEP;
    const LAST: i64 = REGISTER + 29 * STEP;
    let previous = play(SPLIT, ORIGIN, ORIGIN, &[], Some(ORIGIN));
    let observe = grid(REGISTER + STEP, LAST, 4 * STEP);
    let run = |own_preview: bool, future: bool| {
        let (drawn, events) = capture(|| {
            let mut front = Frontend::new();
            let _own = own_preview.then(|| register_preview(&front, SPLIT, ORIGIN));
            crate::spine::pump_until_idle();
            let mut next = 0;
            let mut _future = None;
            let mut drawn = BTreeMap::new();
            for frame in grid(ORIGIN, LAST, STEP) {
                for (at, item) in deliver_until(&previous, &mut next, frame, false) {
                    front.deliver(*at, item);
                }
                if future && frame == at_ms(REGISTER) {
                    let look = front.frame(frame);
                    assert!(
                        (1..10).contains(&look.glyphs),
                        "前提: 先渡しの時、字は途中まで出ている: {}",
                        look.glyphs
                    );
                    _future = Some(register_preview(&front, NEXT, 9_000));
                }
                let look = front.frame(frame);
                if observe.contains(&frame) {
                    drawn.insert(frame, (look.glyphs, front.drawn()));
                }
            }
            drawn
        });
        (drawn, previews_received(&events))
    };
    let (alone, alone_previews) = run(true, false);
    let (with_future, future_previews) = run(true, true);
    let (arrived_only, _) = run(false, false);
    assert_eq!(
        alone[&at_ms(LAST)].0,
        10,
        "前提: 最初の塊の字だけが現れ切る"
    );
    assert_ne!(
        alone, arrived_only,
        "前提: 前の再生の区間の全文で配置した面は、届いた字だけで配置した面と違う"
    );
    assert_eq!(
        (alone_previews, future_previews),
        (1, 2),
        "先の時刻の台本の先渡しが届いた"
    );
    for (frame, look) in &alone {
        assert_eq!(
            &with_future[frame], look,
            "先渡しの後も、字の数と描いた面が変わらない（{frame}）"
        );
    }
}
