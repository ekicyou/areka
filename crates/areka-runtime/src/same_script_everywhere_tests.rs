//! 同じ台本は、届く順と届く時刻を変えた受け手のどれで再生しても、同じ観測の時刻で同じに見える。
//!
//! 台本を本物の再生機で刻み、受け口へ出た物（合図・起点の知らせ・トークの終わり）を出た順に控える。
//! その列を、それぞれの時刻（合図の発火時刻・知らせの効く時刻。前の物より前にはならない）に受ける組
//! （フレームは 16 ms ごと）と、粗いフレームで遅れてまとめて受ける組へ与える。文字の層とバルーンの寿命は、
//! 全部を最初のフレームの前に受ける組とも比べる。seriko は観測の時刻ごとに作り直し、その時刻までの物を
//! 流して刻んでから閉じ、最後の絵を読む。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::{Arc, mpsc};

use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::{ChoiceSpan, TextItem, TextLayerConfig};
use areka_ghost::sink::BootCueSink;
use areka_sakura::ActorKey;
use areka_sakura::sysvar::SystemVarSnapshot;
use areka_seriko::{
    AnimationTable, BindResolver, DefaultBinds, DisplayCommand, MockSurfaceOutput,
    SerikoLoopConfig, SurfaceResolver, seeded_rng, spawn_seriko,
};
use bevy_ecs::world::World;
use dola::TimePoint;
use dola::cue::{CueCommand, CuePlayer, CueSink, OriginNotice, TalkCue};
use dola::time::testing::{at_ms, delta_ms, ms};
use wintf::ecs::FrameTime;

use crate::balloon_hover::BalloonHover;
use crate::balloon_visibility::run_balloon_visibility_phase;
use areka_placement::spawn::GhostWindows;

use crate::assets::build_boot_assets;
use crate::frame::shared_test_support::wiring_with;
use crate::frame::test_support::gpu_attach_world;
use crate::frame::{RuntimeWiring, run_attach_phase, run_text_advance_phase};
use crate::sample_test_support::{emo2_balloon_root, emo2_root};
use crate::talk_clock::TalkClock;
use crate::talk_lifecycle::{BalloonLifecycleSink, TalkLifecycleSignal};

/// 台本の起点。
const ORIGIN: i64 = 1_000;
/// 再生機の刻みと、時刻どおりに受ける組のフレームの間隔。
const STEP: i64 = 16;
/// 遅れてまとめて受ける組のフレームの間隔。
const COARSE: i64 = 70;
/// seriko を観測する時刻の間隔。
const PROBE: i64 = 5;

/// 字・待ちの指定・消去と改行・面の切り替えを含む台本。
const SCRIPT: &str = r"\0\s[0]あいうえお\w9かきくけこ\c\nさしす\s[5]せそ\_w[300]たちつてと\e";

/// 受け口へ出た物。
#[derive(Clone, Debug)]
enum Item {
    Cue(TalkCue),
    Origin(OriginNotice),
    /// 再生が終わり、トークの受け口が落ちた。
    End,
}

/// 出た順の列（それぞれの時刻つき）と、再生が終わった刻みの時刻。
struct Played {
    items: Vec<(TimePoint, Item)>,
    end: TimePoint,
}

struct Recorder(Rc<RefCell<Vec<Item>>>);

impl CueSink for Recorder {
    fn emit(&mut self, cue: TalkCue) {
        self.0.borrow_mut().push(Item::Cue(cue));
    }

    fn origin_set(&mut self, notice: OriginNotice) {
        self.0.borrow_mut().push(Item::Origin(notice));
    }
}

/// [`SCRIPT`] を起点 [`ORIGIN`] から 16 ms ごとに刻み、`shifts` の（刻みの時刻, 動かす長さ）の刻みの
/// 直前に起点を動かす。
fn play(shifts: &[(i64, i64)]) -> Played {
    let instructions = areka_parsers::sakura::parse(SCRIPT);
    let compiled = areka_sakura::compile(&instructions, &SystemVarSnapshot::default());
    let sheet = compiled.sheet.with_absolute_start_time(at_ms(ORIGIN));
    let mut player = CuePlayer::from_sheet(&sheet).expect("起点を刻んだ台本");
    let log = Rc::new(RefCell::new(Vec::new()));
    player.register_sink(Box::new(Recorder(Rc::clone(&log))));
    let mut t = ORIGIN;
    loop {
        for &(_, delta) in shifts.iter().filter(|(at, _)| *at == t) {
            let moved = player.shift_origin(at_ms(t), delta_ms(delta));
            assert!(moved.is_some(), "前提: {t} ms の置き直しで起点が動く");
        }
        player.tick(at_ms(t));
        if player.is_completed() {
            break;
        }
        t += STEP;
        assert!(t < ORIGIN + 60_000, "前提: 台本が終わる");
    }
    let end = at_ms(t);
    let mut arrival = TimePoint::ZERO;
    let mut items: Vec<(TimePoint, Item)> = log
        .take()
        .into_iter()
        .map(|item| {
            let own = match &item {
                Item::Cue(cue) => cue.fire_time,
                Item::Origin(notice) => notice.effective_at,
                Item::End => end,
            };
            arrival = arrival.max(own);
            (arrival, item)
        })
        .collect();
    items.push((end, Item::End));
    Played { items, end }
}

/// 口パクの 1 回の長さ（ミリ秒）。字の間隔 50 ms より短いので、どの字も 1 回の再生を始める。
const MOUTH_MS: u32 = 30;
/// 口のコマの絵の番号の頭。再生を始めて k ms 目のコマは `MOUTH + k` なので、絵から始まりの時刻が分かる。
const MOUTH: u32 = 600;
/// 面 5 の `always` のコマの絵の番号の頭（1 ms ごとに次の絵）。
const ALWAYS: u32 = 700;

/// 面 0 と面 5 の一番上に `talk,1` の口（animation 0）、面 5 にだけ `always`（animation 1）。
fn shell_text() -> String {
    let frames = |id: u32, head: u32, count: u32, end: bool| {
        let mut text = format!("animation{id}.pattern0,overlay,{head},0,0,0\n");
        for k in 1..count {
            text += &format!("animation{id}.pattern{k},overlay,{},1,0,0\n", head + k);
        }
        if end {
            text += &format!("animation{id}.pattern{count},overlay,-1,1,0,0\n");
        }
        text
    };
    let mouth = format!(
        "animation0.interval,talk,1\n{}",
        frames(0, MOUTH, MOUTH_MS + 1, true)
    );
    let always = format!(
        "animation1.interval,always\n{}",
        frames(1, ALWAYS, 100, false)
    );
    let mut text = format!("surface0\n{{\n{mouth}}}\nsurface5\n{{\n{mouth}{always}}}\n");
    for id in (MOUTH..=MOUTH + MOUTH_MS).chain(ALWAYS..ALWAYS + 100) {
        text += &format!("surface{id}\n{{\n}}\n");
    }
    text
}

fn seriko_config() -> SerikoLoopConfig {
    let world = EmoWorld::build(&areka_parsers::shell::parse(&shell_text()));
    SerikoLoopConfig {
        shell_table: AnimationTable::from_world(&world),
        balloon_tables: BTreeMap::new(),
        rng: seeded_rng(0),
        pick_rng: seeded_rng(0),
    }
}

/// `from` から `to` まで `step` ms ごとの時刻。
fn grid(from: i64, to: i64, step: i64) -> Vec<TimePoint> {
    (from..=to).step_by(step as usize).map(at_ms).collect()
}

fn merged(mut a: Vec<TimePoint>, b: &[TimePoint]) -> Vec<TimePoint> {
    a.extend_from_slice(b);
    a.sort();
    a.dedup();
    a
}

/// 列のうち、まだ渡していない物で、`frame` までに出た物（`early` なら全部）を出た順に返す。
fn deliver_until<'a>(
    played: &'a Played,
    next: &mut usize,
    frame: TimePoint,
    early: bool,
) -> &'a [(TimePoint, Item)] {
    let from = *next;
    while *next < played.items.len() && (early || played.items[*next].0 <= frame) {
        *next += 1;
    }
    &played.items[from..*next]
}

/// 文字の層とバルーンの寿命の 1 組（UI の受け取りから、フレームの頭の進める相と可視性の相まで）。
///
/// 時間切れはバルーンが見えている時だけ測るので、実 GPU で emo2 を装着し、可視性の相が本当に出す。
struct Frontend {
    wiring: RuntimeWiring,
    world: World,
    _windows: GhostWindows,
    text: Box<dyn CueSink>,
    _text_actor: Box<dyn std::any::Any>,
    life: Option<Box<dyn BootCueSink>>,
    /// 登録の受け口の原本の代わり（本番と同じく、トークの受け口が落ちても線は切れない）。
    _registered: mpsc::Sender<TalkLifecycleSignal>,
    /// 欄は書いた順に捨てられるので、装置の許可は装置（`world`）より後の最後に置く。
    _gpu: crate::spine::GpuPermit,
}

impl Frontend {
    /// トークの受け口が落ちた時の時計の読みは `end`（どの組でも、落ちたのはその時刻）。
    fn new(end: TimePoint) -> Self {
        let runtime = Rc::new(RefCell::new(TextLayerRuntime::new(
            TextLayerConfig::default(),
        )));
        let (tx, rx) = mpsc::channel::<TalkLifecycleSignal>();
        let (gpu, mut world, windows) = gpu_attach_world();
        let assets = build_boot_assets(&emo2_root(), &emo2_balloon_root(), &[0, 1], 96, 96)
            .expect("emo2 の資産");
        let mut wiring = wiring_with(assets, Rc::clone(&runtime), EmoPresenter::new());
        wiring.lifecycle_rx = rx;
        run_attach_phase(&mut wiring, &mut world);
        assert_eq!(
            wiring
                .presenter
                .target_visible(crate::target_map::balloon_target(0)),
            Some(false),
            "前提: バルーンは装着され、見えないまま確立している"
        );
        world.insert_non_send(BalloonHover::default());
        let life = BalloonLifecycleSink::new(tx.clone(), TalkClock::new(Arc::new(move || end)))
            .clone_box();
        let (text, actor) = spawn_emo_text(runtime);
        Self {
            wiring,
            world,
            text: Box::new(text),
            _text_actor: Box::new(actor),
            life: Some(life),
            _registered: tx,
            _windows: windows,
            _gpu: gpu,
        }
    }

    fn deliver(&mut self, item: &Item) {
        match item {
            Item::Cue(cue) => {
                self.text.emit(cue.clone());
                if let Some(life) = &mut self.life {
                    life.emit(cue.clone());
                }
            }
            Item::Origin(notice) => {
                self.text.origin_set(*notice);
                if let Some(life) = &mut self.life {
                    life.origin_set(*notice);
                }
            }
            Item::End => self.life = None,
        }
    }

    fn frame(&mut self, now: TimePoint) -> FrontLook {
        crate::spine::pump_until_idle();
        self.world.insert_resource(FrameTime(now));
        run_text_advance_phase(&mut self.wiring, &mut self.world, None);
        run_balloon_visibility_phase(&mut self.wiring, &mut self.world);
        let runtime = self.wiring.runtime().borrow();
        let actor = ActorKey::from("0");
        let state = runtime.state();
        let text = state.actor_state(&actor).map(|text| {
            (
                text.items().to_vec(),
                text.reveal().times().to_vec(),
                text.choices().to_vec(),
            )
        });
        FrontLook {
            glyphs: state.visible_glyphs(&actor, now),
            text,
            clears: state.clear_count(&actor),
            lifetime: format!("{:?}", self.wiring.balloon_visibility),
            balloon_shown: self
                .wiring
                .presenter
                .target_visible(crate::target_map::balloon_target(0)),
        }
    }
}

/// 1 つの観測の時刻の、文字の層とバルーンの寿命の姿。
#[derive(Debug, PartialEq)]
struct FrontLook {
    /// 現れた字の数。
    glyphs: usize,
    /// 当たった指令（文字の層の、並んだ物と字の現れる時刻と押せる範囲）。
    text: Option<(Vec<TextItem>, Vec<TimePoint>, Vec<ChoiceSpan>)>,
    /// 当たった消去の数。
    clears: u64,
    /// バルーンの寿命の判断が持つ物（表示の終わり・トークの終わり・期限・番号）。
    lifetime: String,
    /// バルーンが見えているか（時間切れで隠れたか）。
    balloon_shown: Option<bool>,
}

/// `frames` の時刻ごとにフレームを回し、`observe` の時刻の姿を返す。
fn run_frontend(
    played: &Played,
    frames: &[TimePoint],
    early: bool,
    observe: &[TimePoint],
) -> BTreeMap<TimePoint, FrontLook> {
    let mut front = Frontend::new(played.end);
    let mut next = 0;
    let mut looks = BTreeMap::new();
    for &frame in frames {
        for (_, item) in deliver_until(played, &mut next, frame, early) {
            front.deliver(item);
        }
        let look = front.frame(frame);
        if observe.contains(&frame) {
            looks.insert(frame, look);
        }
    }
    looks
}

/// seriko を作り、`ticks` のうち `at` より前の刻みと `at` の刻みを、それぞれの刻みの前にその時刻までに
/// 出た物を渡して回し、閉じて、面と絵の最後の姿を返す。
fn seriko_at(played: &Played, ticks: &[TimePoint], at: TimePoint) -> Option<(u32, PatternState)> {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, actor) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        DefaultBinds::uniform(BindSet::default()),
        BindResolver::empty(),
        seriko_config(),
        out,
    );
    let mut next = 0;
    for &tick in ticks.iter().filter(|&&t| t < at).chain([&at]) {
        for (_, item) in deliver_until(played, &mut next, tick, false) {
            match item {
                Item::Cue(cue) => sink.emit(cue.clone()),
                Item::Origin(notice) => sink.origin_set(*notice),
                Item::End => {}
            }
        }
        sink.send_tick(tick);
    }
    sink.close().expect("seriko の受け口が在る");
    actor.join().expect("seriko が止まる");
    let records = records.lock().expect("記録の鍵");
    records.iter().rev().find_map(|cmd| match cmd {
        DisplayCommand::Show {
            scope,
            surface_id,
            pattern,
            ..
        } if *scope == ActorKey::from("0") => Some((*surface_id, pattern.clone())),
        _ => None,
    })
}

type SerikoLooks = BTreeMap<TimePoint, Option<(u32, PatternState)>>;

/// 口パクの再生の始まり（観測した口のコマの絵から逆算する）。
fn mouth_starts(looks: &SerikoLooks) -> Vec<TimePoint> {
    let mut starts: Vec<TimePoint> = looks
        .iter()
        .filter_map(|(&t, look)| {
            let frame = look.as_ref()?.1.get(0)?.surface_id;
            (MOUTH..=MOUTH + MOUTH_MS)
                .contains(&frame)
                .then(|| t - ms(frame - MOUTH))
        })
        .collect();
    starts.dedup();
    starts
}

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
    let played = play(&[]);
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
    let played = play(&[(FORWARD, -80), (BACKWARD, 150)]);
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
