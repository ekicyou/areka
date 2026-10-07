//! 本物の読み手から合成まで、動く絵をシェルとバルーンで通す（spec: areka-P0-animated-image-playback
//! task 5.4・要件 1.1・1.2・2.3・6.1・6.7・9.1・design「E2E Tests」「出番の世代」）。
//!
//! 検体は `areka-emo-compose/tests/fixtures/animated-playback/`（中身は同じフォルダの README）。
//! 決め手の画素は写し元 `areka-emo-atlas/src/testdata/animated/README.md` の表が正本:
//! `rgb.apng`・`rgb.webp` は (7,7) が 0 番は赤・1 番は青（α が無いので左半分の白は使わない）、
//! `alpha.webp` は 0 番が (2,2) 赤・(7,7) 透明、2 番が (2,2) 青 (0,0,254)・(7,7) 緑 (0,254,0)。
//! 1 番（待ち時間 0）は出ない。
//!
//! seriko は本物のアクター（時計つきの起動）。合図の時刻は注入の時計、刻みの時刻は注入の値で、
//! 出た指令の列は FIFO なので決定論的に決まる（実時間は受け取りの待ちの上限にしか使わない）。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use areka_emo_atlas::{AtlasTable, UseSelfAlpha, WicDecoderArm};
use areka_emo_compose::{BindSet, ComposedSurface, Composer, EmoWorld, FilmSheet, PatternState};
use areka_emo_present::balloon::{build_balloon_target_from_faces, resolve_balloon_faces};
use areka_emo_present::presenter::VisibilityOwnership;
use areka_emo_present::{EmoPresenter, PresentCommand, load_shell_target};
use areka_sakura::ActorKey;
use areka_seriko::{
    AnimationTable, BindResolver, DisplayCommand, MockSurfaceOutput, SerikoClock, SerikoLoopConfig,
    SerikoSink, StageNote, SurfaceOutput, SurfaceResolver, seeded_rng, spawn_seriko_clocked,
};
use bevy_ecs::world::World;
use dola::cue::{CueCommand, CueSink, TalkCue};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use wintf::ecs::{DPI, GraphicsCore, WucGraphicsResource};

use crate::emo2_boot::adapter::PresentBridge;
use crate::emo2_boot::target_map::balloon_target;

/// 合図を処理する時計の始まり（ミリ秒）。
const START: u64 = 1_000;
/// 刻みの間隔（本番の既定と同じ 16 ms）。
const STEP: u64 = 16;

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const GRAY: [u8; 4] = [128, 128, 128, 255];
const CLEAR: [u8; 4] = [0, 0, 0, 0];
/// `alpha.webp` の重ねるコマの色（`image-webp` の丸めで 255 → 254・README）。
const BLUE_254: [u8; 4] = [0, 0, 254, 255];
const GREEN_254: [u8; 4] = [0, 254, 0, 255];

fn fixture(sub: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../areka-emo-compose/tests/fixtures/animated-playback")
        .join(sub)
}

/// WIC の読み手に要る COM の MTA 初期化（既初期化の S_FALSE／RPC_E_CHANGED_MODE は無視）。
fn com_init() {
    // SAFETY: テストスレッドごとの COM 初期化。結果は既初期化でも続けてよいので捨てる。
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn cue(scope: &str, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope),
        command,
        duration: 0.0,
    }
}

fn spawn<O: SurfaceOutput + Send + 'static>(
    config: SerikoLoopConfig,
    out: O,
    clock: SerikoClock,
) -> (SerikoSink, areka_actor::ActorHandle) {
    spawn_seriko_clocked(
        SurfaceResolver::new(BTreeMap::new()),
        BindSet::default(),
        BindResolver::empty(),
        config,
        out,
        Some(clock),
    )
}

/// 時計 [`START`] のアクターへ合図 `command` を 1 つ送り、`ticks` 回刻んで（`START + 16k`）閉じ、
/// 出た指令の全部を出た順に返す。
fn run(
    config: SerikoLoopConfig,
    scope: &str,
    command: CueCommand,
    ticks: u64,
) -> Vec<DisplayCommand> {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn(config, out, Arc::new(|| START));
    sink.emit(cue(scope, command));
    for k in 1..=ticks {
        sink.send_tick(START + STEP * k);
    }
    sink.close().expect("Close を送れる");
    handle.join().expect("Close で正常終了する");
    records.lock().unwrap().clone()
}

/// 合成した絵の (x, y) の乗算していない RGBA（不透明か全透明の画素だけを読む）。
fn rgba(pic: &ComposedSurface, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * pic.width() + x) * 4) as usize;
    let [b, g, r, a] = pic.bytes()[at..at + 4] else {
        unreachable!("4 バイト")
    };
    assert!(a == 0 || a == 255, "({x},{y}) が半透明: {:?}", [r, g, b, a]);
    [r, g, b, a]
}

/// `rgb.apng`・`rgb.webp` のコマ（(7,7) の右半分の色）。
fn two_color_frame(p: [u8; 4]) -> u32 {
    match p {
        RED => 0,
        BLUE => 1,
        other => panic!("rgb の絵のどのコマの色でもない: {other:?}"),
    }
}

/// `alpha.webp` のコマ（(2,2) と (7,7)・透明な所は下地 `under` が見える）。
fn alpha_frame(p22: [u8; 4], p77: [u8; 4], under: [u8; 4]) -> u32 {
    match (p22, p77) {
        (RED, p) if p == under => 0,
        (BLUE_254, GREEN_254) => 2,
        other => panic!("alpha.webp の 0 番・2 番のどちらでもない（1 番は出ない）: {other:?}"),
    }
}

/// 続けて同じ値を 1 つにまとめた列（コマが替わった順）。
fn runs(frames: impl IntoIterator<Item = u32>) -> Vec<u32> {
    let mut v: Vec<u32> = frames.into_iter().collect();
    v.dedup();
    v
}

/// シェル: 本物の読み手で検体を読み、`\s[0]` を時計つきの seriko に流して刻み、出た指令の
/// `PatternState` を合成すると、決め手の画素が APNG（`rgb.apng`）と動く WebP（`alpha.webp`・
/// `rgb.webp`）のコマのとおりに進む。最初の指令の絵は経過 0（要件 1.1・1.2・9.1）。
///
/// 刻みは 16 ms を 49 回（経過 784 ms）: 終わりなしの 2 コマ（100・100）は 8 区間、合計 3 回の
/// `alpha.webp`（100・0・70）は 3 周（510 ms）で 2 番に止まって 6 区間。2 つの `rgb.apng` は同じ子。
#[test]
fn shell_films_play_from_the_real_reader_to_the_composer() {
    com_init();
    let arm = WicDecoderArm::new().expect("WIC の工場が作れる");
    let target = load_shell_target(&fixture("shell"), &arm).expect("検体のシェルは読める");
    let world = target.build_world();
    let atlas = target.atlas();
    let config = SerikoLoopConfig {
        shell_table: AnimationTable::from_world(&world),
        balloon_tables: BTreeMap::new(),
        rng: seeded_rng(0),
    };
    let cmds = run(config, "0", CueCommand::Emote { key: "0".into() }, 49);

    let mut composer = Composer::new();
    // 1 指令ごとの（rgb.apng 左, rgb.apng 右, alpha.webp, rgb.webp）のコマ。
    let frames: Vec<[u32; 4]> = cmds
        .iter()
        .map(|cmd| {
            let DisplayCommand::Show {
                surface_id: 0,
                binds,
                pattern,
                ..
            } = cmd
            else {
                panic!("面 0 の Show だけが出るはず: {cmd:?}");
            };
            let pic = composer
                .compose(&world, atlas, 0, binds, pattern)
                .expect("合成できる");
            assert_eq!((pic.width(), pic.height()), (16, 16), "外形は動かない");
            [
                two_color_frame(rgba(&pic, 7, 7)),
                two_color_frame(rgba(&pic, 15, 7)),
                alpha_frame(rgba(&pic, 2, 10), rgba(&pic, 7, 15), GRAY),
                two_color_frame(rgba(&pic, 15, 15)),
            ]
        })
        .collect();

    assert!(cmds.len() > 1, "前提: 刻みで指令が出ている: {cmds:?}");
    assert_eq!(
        frames[0],
        [0, 0, 0, 0],
        "最初の指令（`\\s[0]`）の絵は経過 0"
    );
    assert!(
        frames.iter().all(|f| f[0] == f[1]),
        "同じ絵を置く 2 か所は同じコマ"
    );
    let endless = vec![0, 1, 0, 1, 0, 1, 0, 1];
    assert_eq!(runs(frames.iter().map(|f| f[0])), endless, "rgb.apng");
    assert_eq!(runs(frames.iter().map(|f| f[3])), endless, "rgb.webp");
    assert_eq!(
        runs(frames.iter().map(|f| f[2])),
        vec![0, 2, 0, 2, 0, 2],
        "alpha.webp は 3 周で最後のコマに止まる"
    );
}

/// 検体のバルーンの面（面 0 ＝ `balloons0.png`・面 1 ＝ `balloons1.png`）を本物の読み手で焼く。
fn load_balloon() -> (EmoWorld, AtlasTable) {
    com_init();
    let dir = fixture("balloon");
    let faces = resolve_balloon_faces(&dir, 0).expect("面 0 は解決できる");
    let arm = WicDecoderArm::new().expect("WIC の工場が作れる");
    let Ok(pair) = build_balloon_target_from_faces(&dir, &arm, &faces, UseSelfAlpha::On) else {
        panic!("バルーンの面は焼ける");
    };
    pair
}

/// バルーン: `resolve_balloon_faces` → `build_balloon_target_from_faces` の面で同じことをする。
/// 面 0 は APNG（`rgb.apng`・終わりなし）、面 1 は動く WebP（`alpha.webp`・合計 3 回）。どちらも
/// シェルと同じ seriko の表と同じ合成を通る（要件 6.1・6.7）。
#[test]
fn balloon_films_play_from_the_real_reader_to_the_composer() {
    let (world, atlas) = load_balloon();

    let mut composer = Composer::new();
    let mut play = |face: u32, ticks: u64| -> Vec<ComposedSurface> {
        let config = SerikoLoopConfig {
            shell_table: AnimationTable::empty(),
            balloon_tables: BTreeMap::from([(
                ActorKey::from("0"),
                AnimationTable::from_world(&world),
            )]),
            rng: seeded_rng(0),
        };
        let key = face.to_string();
        let cmds = run(config, "0", CueCommand::BalloonSurface { key }, ticks);
        assert!(cmds.len() > 1, "前提: 刻みで指令が出ている: {cmds:?}");
        cmds.iter()
            .map(|cmd| {
                let DisplayCommand::ShowBalloon {
                    surface_id,
                    pattern,
                    ..
                } = cmd
                else {
                    panic!("ShowBalloon だけが出るはず（知らせが無いので合図は 0 件）: {cmd:?}");
                };
                assert_eq!(*surface_id, face);
                composer
                    .compose(&world, &atlas, face, &BindSet::default(), pattern)
                    .expect("合成できる")
            })
            .collect()
    };

    // 面 0（APNG）: 16 ms を 24 回（384 ms）で 4 区間。
    let apng: Vec<u32> = play(0, 24)
        .iter()
        .map(|p| two_color_frame(rgba(p, 7, 7)))
        .collect();
    assert_eq!(apng[0], 0, "最初の指令（`\\b[0]`）の絵は経過 0");
    assert_eq!(runs(apng), vec![0, 1, 0, 1], "balloons0.png（rgb.apng）");

    // 面 1（動く WebP）: 16 ms を 49 回で 3 周して止まる。透明な所は下地が無いので全透明。
    let webp: Vec<u32> = play(1, 49)
        .iter()
        .map(|p| alpha_frame(rgba(p, 2, 2), rgba(p, 7, 7), CLEAR))
        .collect();
    assert_eq!(webp[0], 0, "最初の指令（`\\b[1]`）の絵は経過 0");
    assert_eq!(
        runs(webp),
        vec![0, 2, 0, 2, 0, 2],
        "balloons1.png（alpha.webp）は 3 周で最後のコマに止まる"
    );
}

// ---------------------------------------------------------------------------
// 閉じる → 出すの競り合い（seriko → 橋渡し `map_display_command` → 1 本の受け口 → 表示層）
// ---------------------------------------------------------------------------

/// 受け口から 1 件受ける（seriko は別スレッド。待ちの上限は失敗を早く知るためだけ）。
fn recv(rx: &Receiver<PresentCommand>) -> PresentCommand {
    rx.recv_timeout(Duration::from_secs(10))
        .expect("seriko から指令が届く")
}

/// 絵の見分け（経過 0・回数つきの 2 番）。
#[derive(Clone, Copy, Debug, PartialEq)]
enum Pic {
    Rest,
    Last,
}

/// 届いた指令の見分け。
#[derive(Debug, PartialEq)]
enum Arrival {
    Show(Pic),
    Ack(u64),
}

struct Race {
    world: EmoWorld,
    atlas: AtlasTable,
    sheet: FilmSheet,
    rest: Vec<u8>,
    last: Vec<u8>,
}

impl Race {
    fn new() -> Self {
        let (world, atlas) = load_balloon();
        let sheet = world
            .film_sheets()
            .find(|s| s.laps.is_some())
            .expect("面 1 の alpha.webp は回数つきの子")
            .clone();
        let mut last = PatternState::default();
        last.set_film(sheet.id, sheet.frames[2]);
        let mut race = Self {
            world,
            atlas,
            sheet,
            rest: Vec::new(),
            last: Vec::new(),
        };
        race.rest = race.compose(&PatternState::default());
        race.last = race.compose(&last);
        assert_ne!(race.rest, race.last, "前提: 経過 0 と 2 番の絵が違う");
        race
    }

    fn compose(&self, pattern: &PatternState) -> Vec<u8> {
        Composer::new()
            .compose(&self.world, &self.atlas, 1, &BindSet::default(), pattern)
            .expect("合成できる")
            .bytes()
            .to_vec()
    }

    fn classify(&self, cmd: &PresentCommand) -> Arrival {
        match cmd {
            PresentCommand::ShowSurface {
                target,
                surface_id: 1,
                pattern,
                ..
            } if *target == balloon_target(0) => {
                let pic = self.compose(pattern);
                Arrival::Show(if pic == self.rest {
                    Pic::Rest
                } else if pic == self.last {
                    Pic::Last
                } else {
                    panic!("経過 0 でも 2 番でもない指令: {pattern:?}")
                })
            }
            PresentCommand::StageAck { target, generation } if *target == balloon_target(0) => {
                Arrival::Ack(*generation)
            }
            _ => panic!("面 1 のバルーンの表示か合図だけが届くはず"),
        }
    }

    /// 表示層が最後に成立させた絵（MCP の `dump_balloon` が読む口 `last_shown`）。
    fn shown(&self, presenter: &EmoPresenter) -> Pic {
        let Some((1, Some(pic))) = presenter.last_shown(balloon_target(0)) else {
            panic!("隠れていても最後に成立した絵が引ける（dump_balloon が失敗しない）");
        };
        if pic.bytes() == self.rest.as_slice() {
            Pic::Rest
        } else if pic.bytes() == self.last.as_slice() {
            Pic::Last
        } else {
            panic!("表示層の絵が経過 0 でも 2 番でもない")
        }
    }
}

fn gpu_world() -> World {
    com_init();
    let core = GraphicsCore::new().expect("GraphicsCore::new 失敗");
    let d2d = core.d2d_device().expect("GraphicsCore::d2d_device が None");
    let wuc = WucGraphicsResource::new(d2d).expect("WucGraphicsResource::new 失敗");
    let mut world = World::new();
    world.insert_resource(core);
    world.insert_resource(wuc);
    world
}

/// 回数つきの `alpha.webp`（面 1）が 2 番に居る状態から「隠す → 出す」を行い、その間に seriko が
/// 出した指令を本番の橋渡し（`PresentBridge` → `map_display_command`）と 1 本の受け口を通して
/// 表示層へ流す。受け口に着く順は seriko が出した順のまま（①古い 2 番の指令 → ②閉じた知らせの
/// 戻し → ③合図 → ④新しい指令）で、④ が着くまでの全部の表示が経過 0、④ の後は 2 番になる
/// （要件 2.3・design「出番の世代」の道が順を保つことの固定）。隠れている間に届いた指令は預かる
/// だけで、`dump_balloon` の読む口は成立済みの絵を返す。
#[test]
fn close_then_show_race_keeps_order_through_the_bridge() {
    let race = Race::new();
    let (emo_world, atlas) = load_balloon();
    let mut world = gpu_world();
    let t = balloon_target(0);
    let mut presenter = EmoPresenter::new();
    let window = world.spawn(DPI::from_dpi(96, 96)).id();
    presenter
        .attach_target(&mut world, t, window, emo_world, atlas, 96)
        .expect("attach_target");
    presenter
        .set_visibility_ownership(t, VisibilityOwnership::External)
        .expect("set_visibility_ownership");

    let now = Arc::new(AtomicU64::new(START));
    let source = Arc::clone(&now);
    let clock: SerikoClock = Arc::new(move || source.load(Ordering::SeqCst));
    let config = SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(
            ActorKey::from("0"),
            AnimationTable::from_world(&race.world),
        )]),
        rng: seeded_rng(0),
    };
    let (tx, rx) = mpsc::channel();
    let (mut sink, handle) = spawn(config, PresentBridge::new(tx), clock);
    let note = |open: bool, generation: u64| StageNote::Balloon {
        scope: ActorKey::from("0"),
        open,
        face: 1,
        generation,
    };

    // 面 1 を経過 0 で確立して出す（`\b[1]` の指令が届いた＝合図の時計は START で読まれた）。
    sink.emit(cue("0", CueCommand::BalloonSurface { key: "1".into() }));
    let first = recv(&rx);
    assert_eq!(race.classify(&first), Arrival::Show(Pic::Rest));
    presenter.apply(&mut world, first);
    presenter.show_target(&mut world, t).expect("show_target");
    assert_eq!(presenter.stage_generation(t), Some(1));

    // 開いた知らせ（世代 1）→ 刻み 1120 で 2 番（経過 120）。届いた順に流す。
    sink.send_stage(note(true, 1));
    sink.send_tick(START + 120);
    let ack = recv(&rx);
    assert_eq!(race.classify(&ack), Arrival::Ack(1), "合図が先");
    presenter.apply(&mut world, ack);
    loop {
        let cmd = recv(&rx);
        let arrival = race.classify(&cmd);
        presenter.apply(&mut world, cmd);
        if arrival == Arrival::Show(Pic::Last) {
            break;
        }
        assert_eq!(arrival, Arrival::Show(Pic::Rest), "2 番の前は経過 0");
    }
    assert_eq!(
        race.shown(&presenter),
        Pic::Last,
        "前提: 見えている間は 2 番"
    );

    // seriko はまだ出番を知らない: 刻み 1180 で 2 周目の経過 0（Y）、1290 で 2 番（①）。
    sink.send_tick(START + 180);
    sink.send_tick(START + 290);

    // 隠す。閉じた知らせを送り、隠れている間に Y が着く（預かるだけ・読む口は成立済みの 2 番）。
    presenter.apply(
        &mut world,
        PresentCommand::Hide {
            target: t,
            reply: None,
        },
    );
    now.store(START + 300, Ordering::SeqCst);
    sink.send_stage(note(false, 1));
    let held = recv(&rx);
    assert_eq!(race.classify(&held), Arrival::Show(Pic::Rest), "Y");
    presenter.apply(&mut world, held);
    assert_eq!(presenter.target_visible(t), Some(false));
    assert_eq!(
        race.shown(&presenter),
        Pic::Last,
        "隠れている間の dump_balloon"
    );

    // 出す（世代 2）。開いた知らせ（START + 310）の後、刻み 1350（経過 40）・1420（経過 110 で 2 番）。
    presenter.show_target(&mut world, t).expect("show_target");
    assert_eq!(presenter.stage_generation(t), Some(2));
    assert_eq!(race.shown(&presenter), Pic::Rest, "出し直しは経過 0");
    now.store(START + 310, Ordering::SeqCst);
    sink.send_stage(note(true, 2));
    sink.send_tick(START + 350);
    sink.send_tick(START + 420);
    sink.close().expect("Close を送れる");
    handle.join().expect("Close で正常終了する");

    // 残りを着いた順に流し、1 件ごとに表示層の絵を読む。
    let mut seen = Vec::new();
    for cmd in rx.try_iter() {
        seen.push((race.classify(&cmd), {
            presenter.apply(&mut world, cmd);
            assert_eq!(presenter.target_visible(t), Some(true));
            race.shown(&presenter)
        }));
    }
    assert_eq!(
        seen,
        vec![
            (Arrival::Show(Pic::Last), Pic::Rest), // ① 古い 2 番は回数つきの欄を外される
            (Arrival::Show(Pic::Rest), Pic::Rest), // ② 閉じた知らせの戻し
            (Arrival::Ack(2), Pic::Rest),          // ③ 合図
            (Arrival::Show(Pic::Last), Pic::Last), // ④ 新しい出番の 2 番
        ],
        "受け口に着く順と、そのたびの表示（{:?}）",
        race.sheet.path
    );
}
