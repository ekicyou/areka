//! 字が普通のバルーンと箱を行き来する台詞・印を含む台詞でも、文字の層と seriko は同じ字に同じ時刻を持つ。
//!
//! シェルは面 0 が箱なし、面 10 に箱 `fuda`（element1）と `kami`（element2）。どちらの面も一番上に
//! `talk,1` の口を持つので、どの字も 1 回の口パクを始める（同じ時刻の字は 1 回）。箱の材料は本番と同じ
//! [`ShellBoxAssets`] から、文字の層へは [`ShellBoxAssets::hand_to`]、seriko へは
//! [`ShellBoxAssets::text_routing`] で渡す。合図は、受け手のフレームと刻みの前に、その時刻までに出た物だけを
//! 渡す。

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use areka_emo_compose::{EmoWorld, fold_boxes};
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::inline_marker::MARKER_GLYPH;
use areka_emo_text::sink::EmoTextSink;
use areka_emo_text::state::{TextItem, TextLayerConfig};
use areka_parsers::shell::{parse, parse_boxes};
use areka_sakura::ActorKey;
use areka_seriko::{AnimationTable, SerikoLoopConfig, seeded_rng};
use bevy_ecs::world::World;
use dola::TimePoint;
use dola::cue::{CueCommand, CueSink, TalkCue};
use dola::time::testing::{at_ms, ms};

use crate::same_script_test_support::{
    Item, MOUTH, MOUTH_MS, Played, SerikoLooks, deliver_until, grid, mouth_starts, play,
    seriko_with,
};
use crate::shell_box_assets::ShellBoxAssets;

/// 台本の起点。
const ORIGIN: i64 = 1_000;
/// 観測の終わり（どの台本も、ここまでに字が現れ切る）。
const UNTIL: i64 = 2_200;

const BOXES: &str = "\
balloon.fuda
{
size,100,50
}
balloon.kami
{
size,80,40
}
";

fn shell_text() -> String {
    let mut mouth =
        format!("animation0.interval,talk,1\nanimation0.pattern0,overlay,{MOUTH},0,0,0\n");
    for k in 1..=MOUTH_MS {
        mouth += &format!("animation0.pattern{k},overlay,{},1,0,0\n", MOUTH + k);
    }
    mouth += &format!("animation0.pattern{},overlay,-1,1,0,0\n", MOUTH_MS + 1);
    let mut text = format!(
        "{BOXES}surface0\n{{\n{mouth}}}\nsurface10\n{{\n\
         element1,balloon,fuda,0,0\nelement2,balloon,kami,0,60\n{mouth}}}\n"
    );
    for id in MOUTH..=MOUTH + MOUTH_MS {
        text += &format!("surface{id}\n{{\n}}\n");
    }
    text
}

/// 本番の起動と同じ材料の束（面の表・箱の表・面の表に在る番号）。
fn shell() -> (EmoWorld, ShellBoxAssets) {
    let text = shell_text();
    let world = EmoWorld::build(&parse(&text));
    let (layout, report) = fold_boxes(&parse_boxes(&text), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    assert_eq!(
        layout.placements(10).len(),
        2,
        "前提: 面 10 は箱を 2 つ持つ"
    );
    assert!(layout.placements(0).is_empty(), "前提: 面 0 は箱を持たない");
    let assets = ShellBoxAssets {
        layout,
        aliases: world.alias_snapshot(),
        surface_ids: world.surface_ids().collect(),
        ..ShellBoxAssets::default()
    };
    (world, assets)
}

fn seriko_config() -> SerikoLoopConfig {
    let (world, assets) = shell();
    SerikoLoopConfig {
        shell_table: AnimationTable::from_world(&world),
        balloon_tables: BTreeMap::new(),
        rng: seeded_rng(0),
        pick_rng: seeded_rng(0),
        text_routing: assets.text_routing(),
    }
}

fn actor() -> ActorKey {
    ActorKey::from("0")
}

/// 窓を持たない文字の層の受け手（箱の束を受け取り、UI の受け取りとフレームの頭の進める相だけを回す）。
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
        shell()
            .1
            .hand_to(&mut runtime.borrow_mut(), &mut World::new());
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

    /// フレームを 1 つ回し、スコープ 0 の全部の場所で、そのフレームまでに現れた字（印を除く）の時刻を
    /// `seen` へ足す。
    fn frame(&mut self, now: TimePoint, seen: &mut BTreeSet<TimePoint>, places: &mut usize) {
        crate::spine::pump_until_idle();
        let mut runtime = self.runtime.borrow_mut();
        let _ = runtime.advance(now);
        let mut here = 0;
        for (key, text) in runtime.state().places() {
            if key.actor != actor() {
                continue;
            }
            let glyphs = text.items().iter().filter_map(|item| match item {
                TextItem::Glyph { text } => Some(text),
                _ => None,
            });
            let times = text.reveal().times();
            assert_eq!(
                glyphs.clone().count(),
                times.len(),
                "前提: 字ごとに 1 つの時刻"
            );
            let shown = glyphs
                .zip(times)
                .filter(|&(glyph, &t)| &**glyph != MARKER_GLYPH && t <= now);
            seen.extend(shown.map(|(_, &t)| t));
            here += 1;
        }
        *places = (*places).max(here);
    }
}

/// 1 ms ごとのフレームで `played` を文字の層へ与え、現れた字の時刻（印を除き、同じ時刻は 1 つ・時刻の順）
/// と、同時に字を持った場所の数の最大を返す。
fn text_times(played: &Played) -> (Vec<TimePoint>, usize) {
    let mut text = TextOnly::new();
    let mut next = 0;
    let mut seen = BTreeSet::new();
    let mut places = 0;
    for frame in grid(ORIGIN, UNTIL, 1) {
        for (_, item) in deliver_until(played, &mut next, frame, false) {
            text.deliver(item);
        }
        text.frame(frame, &mut seen, &mut places);
    }
    (seen.into_iter().collect(), places)
}

/// 16 ms ごとの刻みで `played` を受けた、箱の材料を入れた seriko の口パクの再生の始まり。
fn mouths(played: &Played) -> Vec<TimePoint> {
    let ticks = grid(ORIGIN, UNTIL, 16);
    let looks: SerikoLooks = grid(ORIGIN, UNTIL, 5)
        .into_iter()
        .map(|t| (t, seriko_with(seriko_config(), played, &ticks, t)))
        .collect();
    mouth_starts(&looks)
}

fn at(offset: i64) -> TimePoint {
    at_ms(ORIGIN + offset)
}

fn assert_same_times(played: &Played, label: &str, min_places: usize) {
    let (times, places) = text_times(played);
    assert!(
        places >= min_places,
        "前提: {label}: 字を持つ場所が {min_places} つ以上: {places}"
    );
    assert_eq!(mouths(played), times, "{label}");
}

/// 選択肢の字が箱に、その直後の `\s[0]` の後の字が普通のバルーンに現れる。普通のバルーンには前の字が
/// 無いので「つづき」は塊の頭から現れ、口もそこで始まる。選択を待つ台本なので、字が現れ切った後で止める。
#[test]
fn a_line_that_leaves_a_box_right_after_a_choice() {
    let played = play(
        r"\0\s[10]\q[はい,yes]\s[0]つづき\e",
        ORIGIN,
        ORIGIN,
        &[],
        Some(ORIGIN + 400),
    );
    let (times, _) = text_times(&played);
    assert_eq!(
        times,
        vec![at(0), at(50), at(100)],
        "前提: 選択肢と「つ」が同じ時刻"
    );
    assert_same_times(&played, "選択肢の後の場所の切り替え", 2);
}

/// 印は時刻の並びに 1 つの目印として入るので、印の後の字は 1 字分後ろに現れる。印は口を動かさない。
#[test]
fn a_line_with_an_inline_marker() {
    let played = play(r"\0\s[0]あ\![*]いう\e", ORIGIN, ORIGIN, &[], None);
    let (times, _) = text_times(&played);
    assert_eq!(
        times,
        vec![at(0), at(100), at(150)],
        "前提: 印の後の字は 1 字分後ろ"
    );
    assert_same_times(&played, "印", 1);
}

/// `\b[名前]` で面の既定の箱から別の箱へ移り、その箱だけを `\c` で消してから字を足し、普通のバルーンへ
/// 戻る。
#[test]
fn a_line_that_selects_a_box_by_name_and_clears_it() {
    let played = play(
        r"\0\s[10]あい\b[kami]う\cえお\s[0]か\e",
        ORIGIN,
        ORIGIN,
        &[],
        None,
    );
    assert_same_times(&played, "名前の箱と場所だけの消去", 3);
}

/// 手で組んだ合図の列（塊ごとに字の速さが違う）。場所の切り替え・選択肢の直後の箱の切り替え・場所だけの
/// 消去を含み、最後の消去は、普通のバルーンにまだ現れていない字を残したまま箱だけを消す。
#[test]
fn hand_built_cues_with_per_chunk_speeds() {
    let cue = |offset: i64, command: CueCommand, duration: i64| {
        (
            at(offset),
            Item::Cue(TalkCue::scheduled(
                at_ms(ORIGIN),
                ms(offset as u32),
                actor(),
                command,
                ms(duration as u32),
            )),
        )
    };
    let text = |s: &str| CueCommand::Text(s.into());
    let emote = |key: &str| CueCommand::Emote { key: key.into() };
    let mut items = vec![
        cue(0, emote("10"), 0),
        cue(0, text("あいう"), 120),
        cue(120, emote("0"), 0),
        cue(120, text("えお"), 200),
        cue(320, emote("10"), 0),
        cue(320, text("かき"), 90),
        cue(
            410,
            CueCommand::Choice {
                id: "yes".into(),
                text: "はい".into(),
                references: vec![],
            },
            0,
        ),
        cue(410, CueCommand::BalloonSurface { key: "kami".into() }, 0),
        cue(410, text("く"), 60),
        cue(470, CueCommand::Clear, 0),
        cue(470, text("けこ"), 140),
        cue(610, emote("0"), 0),
        cue(610, text("さしす"), 300),
        cue(650, emote("10"), 0),
        cue(650, CueCommand::Clear, 0),
        cue(650, text("せ"), 0),
    ];
    let end = at(1_000);
    items.push((end, Item::End));
    let played = Played { items, end };
    let (times, _) = text_times(&played);
    assert_eq!(
        times,
        [
            0, 40, 80, 120, 220, 320, 365, 410, 470, 540, 610, 650, 710, 810
        ]
        .map(at),
        "前提: 場所ごとの前の字から求めた字の時刻"
    );
    assert_same_times(&played, "手で組んだ列", 3);
}
