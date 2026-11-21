use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc;

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use areka_emo_present::{EmoPresenter, PresentCommand};
use areka_emo_text::actor::HitRectPx;
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::actor::{ResolvedBalloonText, TextSlotBinding, present_frame};
use areka_emo_text::place::PlaceKey;
use areka_emo_text::scroll_marker::{ScrollChange, ScrollDir};
use areka_emo_text::state::{SpanKind, TextLayerConfig};
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_runtime::balloon_hover::BalloonHover;
use areka_sakura::TalkId;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};
use areka_seriko::{AnimationTable, BindResolver, SurfaceResolver};
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::world::World;
use log_capture_kit::{LineFormat, capture_lines};
use wintf::ecs::pointer::{DoubleClick, Phase, PointerLeave, PointerState};
use wintf::ecs::{Point, Visual, Window};

use areka_placement::spawn::BalloonWindowMarker;
use areka_runtime::assets::{BalloonScopeAssets, BootAssets, LoopTables, ScopeAssets};
use areka_runtime::balloon_visibility::phase::run_balloon_visibility_phase;
use areka_runtime::frame::RuntimeWiring;
use areka_runtime::frame::test_support::gpu_on;
use areka_runtime::move_cue::PositionCue;
use areka_runtime::spine::GpuPermit;
use areka_runtime::talk_clock::TalkClock;
use areka_runtime::talk_lifecycle::TalkLifecycleSignal;

/// 窓物理 px の行矩形を持つ `ChoiceHitRow` を組む（ordinal は入力順昇順を模す）。
/// 当たりの判定が見るのは種類（ここでは選択肢）と rect だけ（ほかは不透明転写の placeholder）。
pub(crate) fn row(ordinal: usize, left: f32, top: f32, right: f32, bottom: f32) -> ChoiceHitRow {
    ChoiceHitRow {
        kind: SpanKind::Choice,
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

/// アンカーの範囲の当たり行（[`row`] と種類だけが違う）。
pub(crate) fn anchor_row(
    ordinal: usize,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
) -> ChoiceHitRow {
    ChoiceHitRow {
        kind: SpanKind::Anchor,
        ..row(ordinal, left, top, right, bottom)
    }
}

/// 選択肢もアンカーも持たない文字の層。
pub(crate) fn empty_runtime() -> Rc<RefCell<TextLayerRuntime>> {
    Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )))
}

/// 空 `EmoWorld`（空 shell から build・COM/GPU 不要の寛容契約）。
fn empty_world() -> EmoWorld {
    EmoWorld::build(&areka_parsers::shell::parse(""))
}

/// 空アトラス（headless 構築）。
fn empty_atlas() -> AtlasTable {
    AtlasTable::new(Vec::new(), Vec::new(), Vec::new())
}

/// 合成 `BootAssets`（scope0 の最小形・COM/GPU/fixture 不要の純合成）。
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
            definitions: areka_runtime::assets::empty_definitions(0),
            // headless 純合成のバルーンは面画像を持たない＝背景色は既定の白。
            background_color: (255, 255, 255),
            name: String::new(),
        }],
        resolver: SurfaceResolver::new(BTreeMap::new()),
        static_binds: areka_seriko::DefaultBinds::uniform(BindSet::default()),
        bind_resolver: BindResolver::empty(),
        shell_author_dpi: 96,
        balloon_author_dpi: 96,
        boxes: areka_runtime::shell_box_assets::ShellBoxAssets::default(),
        cursors: areka_runtime::shell_cursor_assets::ShellCursorAssets::default(),
        loop_tables: LoopTables {
            shell: AnimationTable::empty(),
            balloon: BTreeMap::new(),
        },
    }
}

/// headless な `RuntimeWiring`（実 `EmoPresenter`／与えた runtime／合成 `BootAssets`・COM/GPU 不要）。
///
/// ハンドラが `RuntimeWiring::runtime()` から借りる runtime を、テスト側が事前に populate した実体で
/// 差し込むための最小結線。
pub(crate) fn headless_runtime_wiring(runtime: Rc<RefCell<TextLayerRuntime>>) -> RuntimeWiring {
    RuntimeWiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<PositionCue>().1,
        mpsc::channel::<TalkLifecycleSignal>().1,
        mpsc::channel::<areka_runtime::zorder_cue::ZOrderDirective>().1,
        runtime,
        TalkClock::new(Arc::new(|| 0.0)),
        synth_boot_assets(),
    )
}

/// トークの番号 `talk_id` を見せている `RuntimeWiring`（装着前・GPU 不要）。
/// 表示の線へ `TalkStarted` を積み、可視性の相を 1 回回して置く。
pub(crate) fn runtime_wiring_showing_talk(talk_id: TalkId) -> RuntimeWiring {
    let (lifecycle_tx, lifecycle_rx) = mpsc::channel::<TalkLifecycleSignal>();
    let mut wiring = RuntimeWiring::new(
        EmoPresenter::new(),
        mpsc::channel::<PresentCommand>().1,
        mpsc::channel::<PositionCue>().1,
        lifecycle_rx,
        mpsc::channel::<areka_runtime::zorder_cue::ZOrderDirective>().1,
        empty_runtime(),
        TalkClock::new(Arc::new(|| 0.0)),
        synth_boot_assets(),
    );
    lifecycle_tx
        .send(TalkLifecycleSignal::TalkStarted {
            talk_id: Some(talk_id),
        })
        .expect("受信端は結線が持つ");
    run_balloon_visibility_phase(&mut wiring, &mut World::new());
    assert_eq!(wiring.shown_talk_id(), Some(talk_id), "見せる番号が置けた");
    wiring
}

/// 選択肢スパンを 1 つ載せて `choice_active(actor)==true` にした実 runtime を組む（GPU 不要）。
/// `choice_hit_rows` は `present_frame`（GPU）未実行ゆえ空のまま（headless の既知制約）。
pub(super) fn runtime_with_active_choice(actor: &str) -> Rc<RefCell<TextLayerRuntime>> {
    let rt = empty_runtime();
    rt.borrow_mut().apply_cue(&choice_cue(actor));
    rt
}

/// 選択肢スパン `\q[はい,OnYes]` 1 つを載せる合図。
pub(crate) fn choice_cue(actor: &str) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command: CueCommand::Choice {
            id: "OnYes".into(),
            text: "はい".into(),
            references: Vec::new(),
        },
        duration: 0.0,
    }
}

/// アンカーの範囲 1 つ（`\_a[x]あい\_a`）を作る合図の列。
pub(crate) fn anchor_cues(actor: &str) -> [TalkCue; 3] {
    [
        CueCommand::AnchorBegin {
            id: "x".into(),
            references: Vec::new(),
        },
        CueCommand::Text("あい".into()),
        CueCommand::AnchorEnd,
    ]
    .map(|command| TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    })
}

/// アンカーの範囲だけを載せた実 runtime（選択肢は無い＝`choice_active` は偽・`hit_active` は真）。
/// 当たりの行は [`runtime_with_active_choice`] と同じく空のまま。
pub(super) fn runtime_with_active_anchor(actor: &str) -> Rc<RefCell<TextLayerRuntime>> {
    let rt = empty_runtime();
    for cue in anchor_cues(actor) {
        rt.borrow_mut().apply_cue(&cue);
    }
    rt
}

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
pub(crate) fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelFields, f);
    lines
}

/// `event` フィールドが `name` の記録だけを取り出す。
pub(crate) fn events_named<'a>(
    events: &'a [log_capture_kit::CapturedEvent],
    name: &str,
) -> Vec<&'a log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name))
        .collect()
}

/// シェル `shell` を畳んだ箱のうち、名前が `name` のもの（`BoxName` は畳み込みからしか作れない）。
pub(crate) fn box_name(shell: &str, name: &str) -> areka_emo_compose::BoxName {
    let world = EmoWorld::build(&areka_parsers::shell::parse(shell));
    let (layout, report) = areka_emo_compose::fold_boxes(
        &areka_parsers::shell::parse_boxes(shell),
        &BTreeMap::new(),
        &world,
    );
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    layout
        .placements(0)
        .iter()
        .find(|p| p.name.as_str() == name)
        .expect("検体に在る箱")
        .name
        .clone()
}

/// バルーン窓（`BalloonWindowMarker`＋`Window`）を組み、その子 entity へ `PointerLeave` を載せる。
/// 子の親チェーンは window へ届く（`find_owner_window` 相当）。
pub(super) fn spawn_balloon_leave_child(world: &mut World, scope: usize) -> Entity {
    let win = world
        .spawn((BalloonWindowMarker { scope }, Window::default()))
        .id();
    world.spawn((PointerLeave, ChildOf(win))).id()
}

/// 窓の client 物理 px の移動（Bubble）。
pub(crate) fn bubble_move(x: i32, y: i32) -> Phase<PointerState> {
    crate::pointer_test_support::bubble_pointer(x, y, DoubleClick::None, false)
}

/// 左シングルクリック（窓の client 物理 px）。
pub(crate) fn left_press(x: i32, y: i32) -> Phase<PointerState> {
    Phase::Bubble(PointerState {
        client_point: Point { x, y },
        left_down: true,
        ..Default::default()
    })
}

/// `BalloonWiring` と `BalloonHover` を置き、選択の受け口を返す。
pub(crate) fn insert_wiring(world: &mut World) -> mpsc::Receiver<ChoiceSelection> {
    let (tx, rx) = mpsc::channel();
    world.insert_non_send(BalloonWiring::new(tx));
    world.insert_non_send(BalloonHover::default());
    rx
}

/// 幾何だけのバルーン定義（既定の書体・描画範囲は画像の全域・文字の原点は `origin`）。
pub(crate) fn plain_model(origin: (i32, i32)) -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(origin.0), Some(origin.1)),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    )
}

/// 実 GPU の World に普通のバルーンの窓（scope 0）と差し込み口を置き、文字の層を拡大率 `scale` で
/// 登録して 1 回提示する。返す `GpuPermit` は World より長く持つ（先頭に置いて最後に落とす）。
pub(crate) fn gpu_balloon_world(
    rt: &Rc<RefCell<TextLayerRuntime>>,
    scale: f32,
    image: (u32, u32),
    origin: (i32, i32),
) -> (GpuPermit, World, Entity) {
    // 途中で panic しても許可が装置より後に捨てられるよう、許可を先に束ねる。
    let (gpu, mut world) = {
        let mut world = World::new();
        let gpu = gpu_on(&mut world);
        (gpu, world)
    };
    let window = world.spawn(BalloonWindowMarker { scope: 0 }).id();
    let slot = world.spawn((Visual::default(), ChildOf(window))).id();
    world.flush();
    let surface = (
        (image.0 as f32 * scale).round() as u32,
        (image.1 as f32 * scale).round() as u32,
    );
    rt.borrow_mut().register_actor(
        ActorKey::from("0"),
        TextSlotBinding::new(slot, window, scale, surface, image),
        ResolvedBalloonText::resolve(&plain_model(origin), image),
    );
    present_frame(&mut rt.borrow_mut(), &mut world, 10.0).expect("提示");
    (gpu, world, window)
}

/// scope 0 の普通のバルーンを新しい側へ 1 行送ったときの変化。
pub(crate) fn newer_by_one(runtime: &RefCell<TextLayerRuntime>) -> ScrollChange {
    runtime.borrow_mut().scroll_manual_at(
        &PlaceKey::balloon(&ActorKey::from("0")),
        ScrollDir::Newer,
        1,
    )
}

/// 行の矩形の中心（窓のクライアント物理 px）。
pub(crate) fn row_center(row: &ChoiceHitRow) -> (i32, i32) {
    (
        ((row.rect.left + row.rect.right) / 2.0) as i32,
        ((row.rect.top + row.rect.bottom) / 2.0) as i32,
    )
}

/// 箱 a（0,0〜100,50）と箱 b（200,0〜300,50）を置いたシェル。
pub(crate) const TWO_BOX_SHELL: &str = "\
balloon.a
{
size,100,50
}
balloon.b
{
size,100,50
}
surface0
{
element1,balloon,a,0,0
element2,balloon,b,200,0
}
";
