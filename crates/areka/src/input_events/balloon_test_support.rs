use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::HitRectPx;
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::world::World;
use tracing::field::{Field, Visit};
use tracing_subscriber::prelude::*;
use wintf::ecs::Window;
use wintf::ecs::pointer::PointerLeave;

use crate::emo2_boot::assets::{BalloonScopeAssets, BootAssets, LoopTables, ScopeAssets};
use crate::emo2_boot::frame::Emo2Wiring;
use crate::emo2_boot::move_cue::MoveDirective;
use crate::emo2_boot::talk_clock::TalkClock;
use crate::emo2_boot::talk_lifecycle::TalkLifecycleSignal;
use crate::placement::spawn::BalloonWindowMarker;

/// 窓物理 px の行矩形を持つ `ChoiceHitRow` を組む（ordinal は入力順昇順を模す）。
/// rect 以外のフィールドは 3.1 の判定に無関係——不透明転写の placeholder。
pub(super) fn row(ordinal: usize, left: f32, top: f32, right: f32, bottom: f32) -> ChoiceHitRow {
    ChoiceHitRow {
        ordinal,
        id: format!("q{ordinal}"),
        label: format!("label{ordinal}"),
        references: Vec::new(),
        rect: HitRectPx {
            left,
            top,
            right,
            bottom,
        },
    }
}

/// 空 `EmoWorld`（空 shell から build・COM/GPU 不要の寛容契約・frame.rs 檻同型）。
fn empty_world() -> EmoWorld {
    EmoWorld::build(&areka_parsers::shell::parse(""))
}

/// 空アトラス（headless 構築・frame.rs 檻同型）。
fn empty_atlas() -> AtlasTable {
    AtlasTable::new(Vec::new(), Vec::new(), Vec::new())
}

/// 合成 `BootAssets`（scope0 の最小形・COM/GPU/fixture 不要の純合成・frame.rs synth_assets 同型）。
fn synth_boot_assets() -> BootAssets {
    BootAssets {
        shells: vec![ScopeAssets {
            scope: 0,
            emo_world: empty_world(),
            atlas: empty_atlas(),
            initial_surface_id: 0,
        }],
        balloons: vec![BalloonScopeAssets {
            scope: 0,
            emo_world: empty_world(),
            atlas: empty_atlas(),
            model: areka_parsers::balloon::parse_str("", None),
        }],
        resolver: SurfaceResolver::new(BTreeMap::new()),
        static_binds: BindSet::default(),
        bind_resolver: BindResolver::empty(),
        shell_author_dpi: 96,
        balloon_author_dpi: 96,
        loop_tables: LoopTables {
            shell: AnimationTable::empty(),
            balloon: BTreeMap::new(),
        },
    }
}

/// headless な `Emo2Wiring`（実 `EmoPresenter`／与えた runtime／合成 `BootAssets`・COM/GPU 不要）。
///
/// ハンドラが `Emo2Wiring::runtime()` から借りる runtime を、テスト側が事前に populate した実体で
/// 差し込むための最小結線（frame.rs の headless_wiring_with と同型）。
pub(super) fn headless_emo2_wiring(runtime: Rc<RefCell<TextLayerRuntime>>) -> Emo2Wiring {
    Emo2Wiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<MoveDirective>().1,
        mpsc::channel::<TalkLifecycleSignal>().1,
        runtime,
        TalkClock::new(Arc::new(|| 0.0)),
        synth_boot_assets(),
    )
}

/// 選択肢スパンを 1 つ載せて `choice_active(actor)==true` にした実 runtime を組む（GPU 不要）。
/// `choice_hit_rows` は `present_frame`（GPU）未実行ゆえ空のまま（headless の既知制約）。
pub(super) fn runtime_with_active_choice(actor: &str) -> Rc<RefCell<TextLayerRuntime>> {
    let rt = Rc::new(RefCell::new(TextLayerRuntime::new(TextLayerConfig::default())));
    rt.borrow_mut().apply_cue(&TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::Choice {
            id: "OnYes".into(),
            text: "はい".into(),
            references: Vec::new(),
        },
        duration: 0.0,
    });
    rt
}

// ── スレッドローカル tracing capture（frame.rs 檻の最小複製・ログレベル決定論観測用）─────────
#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<String>>>);

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
    fn on_event(
        &self,
        ev: &tracing::Event<'_>,
        _: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let meta = ev.metadata();
        let mut line = format!("level={}", meta.level());
        struct V<'a>(&'a mut String);
        impl Visit for V<'_> {
            fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                use std::fmt::Write;
                let _ = write!(self.0, " {}={:?}", f.name(), v);
            }
        }
        ev.record(&mut V(&mut line));
        self.0.lock().unwrap().push(line);
    }
}

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
pub(super) fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let cap = Capture::default();
    let logs = cap.0.clone();
    let subscriber = tracing_subscriber::registry().with(cap);
    tracing::subscriber::with_default(subscriber, f);
    let guard = logs.lock().unwrap();
    guard.clone()
}

/// バルーン窓（`BalloonWindowMarker`＋`Window`）を組み、その子 entity へ `PointerLeave` を載せる。
/// 子の親チェーンは window へ届く（`find_owner_window` 相当）。
pub(super) fn spawn_balloon_leave_child(world: &mut World, scope: usize) -> Entity {
    let win = world
        .spawn((BalloonWindowMarker { scope }, Window::default()))
        .id();
    world.spawn((PointerLeave, ChildOf(win))).id()
}
