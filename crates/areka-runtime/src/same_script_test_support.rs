//! 台本を本物の再生機で刻んで受け口へ出た物を控え、文字の層・バルーンの寿命・seriko の受け手へ与える足場。
//!
//! 受け口へ出た物（合図・起点の知らせ・トークの終わり）を出た順に、それぞれの時刻（合図の発火時刻・
//! 知らせの効く時刻。前の物より前にはならない）つきで控える。文字の層とバルーンの寿命は実 GPU で
//! emo2 を装着した [`Frontend`] で受け、seriko は観測の時刻ごとに作り直す [`seriko_at`] で受ける。

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, mpsc};

use areka_emo_compose::{BindSet, EmoWorld, PatternState};
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::{TextLayerRuntime, spawn_emo_text};
use areka_emo_text::state::{ChoiceSpan, TextItem, TextLayerConfig};
use areka_ghost::sink::BootCueSink;
use areka_placement::spawn::GhostWindows;
use areka_sakura::ActorKey;
use areka_sakura::sysvar::SystemVarSnapshot;
use areka_seriko::{
    AnimationTable, BindResolver, DefaultBinds, DisplayCommand, MockSurfaceOutput,
    SerikoLoopConfig, SurfaceResolver, seeded_rng, spawn_seriko,
};
use bevy_ecs::world::World;
use dola::TimePoint;
use dola::cue::{CuePlayer, CueSink, OriginNotice, TalkCue};
use dola::time::testing::{at_ms, delta_ms, ms};
use wintf::ecs::FrameTime;

use crate::assets::build_boot_assets;
use crate::balloon_hover::BalloonHover;
use crate::balloon_visibility::run_balloon_visibility_phase;
use crate::frame::shared_test_support::wiring_with;
use crate::frame::test_support::gpu_attach_world;
use crate::frame::{
    RuntimeWiring, run_attach_phase, run_text_advance_phase, run_text_phase, run_text_scale_phase,
};
use crate::sample_test_support::{emo2_balloon_root, emo2_root};
use crate::talk_clock::TalkClock;
use crate::talk_lifecycle::{BalloonLifecycleSink, TalkLifecycleSignal};

/// 再生機の刻みと、時刻どおりに受ける組のフレームの間隔。
pub(crate) const STEP: i64 = 16;

/// 受け口へ出た物。
#[derive(Clone, Debug)]
pub(crate) enum Item {
    Cue(TalkCue),
    Origin(OriginNotice),
    /// 再生が終わり（または中断され）、トークの受け口が落ちた。
    End,
}

/// 出た順の列（それぞれの時刻つき）と、再生が終わった刻みの時刻。
pub(crate) struct Played {
    pub(crate) items: Vec<(TimePoint, Item)>,
    pub(crate) end: TimePoint,
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

/// `script` を起点 `origin` に刻印し、`first_tick` から 16 ms ごとに刻む。`shifts` の（刻みの時刻, 動かす
/// 長さ）の刻みの直前に起点を動かす。`stop` の刻みの後で中断する（再生機を捨てる）。
pub(crate) fn play(
    script: &str,
    origin: i64,
    first_tick: i64,
    shifts: &[(i64, i64)],
    stop: Option<i64>,
) -> Played {
    let instructions = areka_parsers::sakura::parse(script);
    let compiled = areka_sakura::compile(&instructions, &SystemVarSnapshot::default());
    let sheet = compiled.sheet.with_absolute_start_time(at_ms(origin));
    let mut player = CuePlayer::from_sheet(&sheet).expect("起点を刻んだ台本");
    let log = Rc::new(RefCell::new(Vec::new()));
    player.register_sink(Box::new(Recorder(Rc::clone(&log))));
    let mut t = first_tick;
    loop {
        for &(_, delta) in shifts.iter().filter(|(at, _)| *at == t) {
            let moved = player.shift_origin(at_ms(t), delta_ms(delta));
            assert!(moved.is_some(), "前提: {t} ms の置き直しで起点が動く");
        }
        player.tick(at_ms(t));
        if player.is_completed() || stop == Some(t) {
            break;
        }
        t += STEP;
        assert!(t < origin + 60_000, "前提: 台本が終わる");
    }
    let end = at_ms(t);
    let mut arrival = TimePoint::ZERO;
    let mut items: Vec<(TimePoint, Item)> = log
        .take()
        .into_iter()
        .map(|item| {
            let own = match &item {
                Item::Cue(cue) => cue.fire_time,
                // 最初の起点の知らせは受け手を何も動かさず、効く時刻が先の時刻でありうる。
                Item::Origin(notice) if notice.previous.is_none() => arrival,
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
pub(crate) const MOUTH_MS: u32 = 30;
/// 口のコマの絵の番号の頭。再生を始めて k ms 目のコマは `MOUTH + k` なので、絵から始まりの時刻が分かる。
pub(crate) const MOUTH: u32 = 600;
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
        text_routing: areka_seriko::TextRouting::default(),
    }
}

/// `from` から `to` まで `step` ms ごとの時刻。
pub(crate) fn grid(from: i64, to: i64, step: i64) -> Vec<TimePoint> {
    (from..=to).step_by(step as usize).map(at_ms).collect()
}

pub(crate) fn merged(mut a: Vec<TimePoint>, b: &[TimePoint]) -> Vec<TimePoint> {
    a.extend_from_slice(b);
    a.sort();
    a.dedup();
    a
}

/// 列のうち、まだ渡していない物で、`frame` までに出た物（`early` なら全部）を出た順に返す。
pub(crate) fn deliver_until<'a>(
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
pub(crate) struct Frontend {
    pub(crate) wiring: RuntimeWiring,
    world: World,
    _windows: GhostWindows,
    text: Box<dyn CueSink>,
    _text_actor: Box<dyn std::any::Any>,
    life: Option<Box<dyn BootCueSink>>,
    /// 登録の受け口の原本（本番と同じく、トークの受け口が落ちても線は切れない。トークごとに複製する）。
    registered: BalloonLifecycleSink,
    /// トークの受け口が落ちた時の時計の読み（ミリ秒）。
    end_ms: Arc<AtomicI64>,
    /// 欄は書いた順に捨てられるので、装置の許可は装置（`world`）より後の最後に置く。
    _gpu: crate::spine::GpuPermit,
}

impl Frontend {
    pub(crate) fn new() -> Self {
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
        let end_ms = Arc::new(AtomicI64::new(0));
        let clock_ms = Arc::clone(&end_ms);
        let registered = BalloonLifecycleSink::new(
            tx,
            TalkClock::new(Arc::new(move || at_ms(clock_ms.load(Ordering::SeqCst)))),
        );
        let (text, actor) = spawn_emo_text(runtime);
        Self {
            wiring,
            world,
            text: Box::new(text),
            _text_actor: Box::new(actor),
            life: Some(registered.clone_box()),
            registered,
            end_ms,
            _windows: windows,
            _gpu: gpu,
        }
    }

    /// `at` に出た物を文字の層とバルーンの寿命の両方の線へ渡す。
    pub(crate) fn deliver(&mut self, at: TimePoint, item: &Item) {
        self.deliver_text(item);
        self.deliver_life(at, item);
    }

    /// 文字の層の線へだけ渡す。
    pub(crate) fn deliver_text(&mut self, item: &Item) {
        match item {
            Item::Cue(cue) => self.text.emit(cue.clone()),
            Item::Origin(notice) => self.text.origin_set(*notice),
            Item::End => {}
        }
    }

    /// バルーンの寿命の線へだけ渡す。トークの終わりの時計の読みは `at`。終わりの後の合図は次のトーク。
    pub(crate) fn deliver_life(&mut self, at: TimePoint, item: &Item) {
        match item {
            Item::Cue(cue) => self
                .life
                .get_or_insert_with(|| self.registered.clone_box())
                .emit(cue.clone()),
            Item::Origin(notice) => {
                if let Some(life) = &mut self.life {
                    life.origin_set(*notice);
                }
            }
            Item::End => {
                self.end_ms
                    .store((at - TimePoint::ZERO).as_ticks() / 10_000, Ordering::SeqCst);
                self.life = None;
            }
        }
    }

    /// 文字の層の受け口へ、別の受け口を作って返す（同じ文字の層へ届く。先渡しを受ける口として使う）。
    pub(crate) fn another_text_sink(&self) -> (Box<dyn CueSink>, Box<dyn std::any::Any>) {
        let (sink, actor) = spawn_emo_text(Rc::clone(self.wiring.runtime()));
        (Box::new(sink), Box::new(actor))
    }

    pub(crate) fn frame(&mut self, now: TimePoint) -> FrontLook {
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

    /// [`frame`](Self::frame) の後に、文字の拡大率の相と文字の相で描き、バルーンの文字の面を読み戻す。
    pub(crate) fn drawn(&mut self) -> Vec<u8> {
        let _ = run_text_scale_phase(&mut self.wiring, &mut self.world);
        run_text_phase(&mut self.wiring, &mut self.world, None);
        self.wiring
            .runtime()
            .borrow()
            .surface(&ActorKey::from("0"))
            .expect("前提: バルーンの文字の面が在る")
            .read_back()
            .expect("文字の面の読み戻し")
    }
}

/// 1 つの観測の時刻の、文字の層とバルーンの寿命の姿。
#[derive(Debug, PartialEq)]
pub(crate) struct FrontLook {
    /// 現れた字の数。
    pub(crate) glyphs: usize,
    /// 当たった指令（文字の層の、並んだ物と字の現れる時刻と押せる範囲）。
    pub(crate) text: Option<(Vec<TextItem>, Vec<TimePoint>, Vec<ChoiceSpan>)>,
    /// 当たった消去の数。
    pub(crate) clears: u64,
    /// バルーンの寿命の判断が持つ物（表示の終わり・トークの終わり・期限・番号）。
    pub(crate) lifetime: String,
    /// バルーンが見えているか（時間切れで隠れたか）。
    pub(crate) balloon_shown: Option<bool>,
}

impl FrontLook {
    /// 字の現れる時刻（文字の層がまだ何も持たなければ空）。
    pub(crate) fn reveal_times(&self) -> &[TimePoint] {
        self.text.as_ref().map_or(&[][..], |(_, times, _)| times)
    }
}

/// `frames` の時刻ごとにフレームを回し、`observe` の時刻の姿を返す。
pub(crate) fn run_frontend(
    played: &Played,
    frames: &[TimePoint],
    early: bool,
    observe: &[TimePoint],
) -> BTreeMap<TimePoint, FrontLook> {
    let mut front = Frontend::new();
    let mut next = 0;
    let mut looks = BTreeMap::new();
    for &frame in frames {
        for (at, item) in deliver_until(played, &mut next, frame, early) {
            front.deliver(*at, item);
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
pub(crate) fn seriko_at(
    played: &Played,
    ticks: &[TimePoint],
    at: TimePoint,
) -> Option<(u32, PatternState)> {
    seriko_with(seriko_config(), played, ticks, at)
}

/// [`seriko_at`] を、構成 `config` で作った seriko で行う。
pub(crate) fn seriko_with(
    config: SerikoLoopConfig,
    played: &Played,
    ticks: &[TimePoint],
    at: TimePoint,
) -> Option<(u32, PatternState)> {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, actor) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        DefaultBinds::uniform(BindSet::default()),
        BindResolver::empty(),
        config,
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

pub(crate) type SerikoLooks = BTreeMap<TimePoint, Option<(u32, PatternState)>>;

/// 口パクの再生の始まり（観測した口のコマの絵から逆算する）。
pub(crate) fn mouth_starts(looks: &SerikoLooks) -> Vec<TimePoint> {
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
