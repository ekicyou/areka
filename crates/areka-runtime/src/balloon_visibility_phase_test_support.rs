// =============================================================================
// 配線の段（相関数）のテストで共有する道具
// 親（`balloon_visibility`）の `test_support` は判断中核の観測表を組む道具であり、
// こちらは表示層への headless 装着と、配線の段のテストが使う World と文字の層の組み立てを持つ
// （写しを各テストに残さない）。
// =============================================================================

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::EmoWorld;
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::TextLayerRuntime;
use areka_emo_text::state::TextLayerConfig;
use bevy_ecs::prelude::Entity;
use bevy_ecs::world::World;
use std::cell::RefCell;
use std::rc::Rc;
use wintf::ecs::FrameTime;

use crate::target_map::balloon_target;

/// 表示層へ scope の balloon target を headless 装着し、窓の entity を返す
/// （可視状態は `Some(false)` から始まる）。
///
/// `attach_target` は GPU 資源を要さないため、実 `EmoPresenter` のまま装着できる。
pub fn attach_headless(presenter: &mut EmoPresenter, world: &mut World, scope: u32) -> Entity {
    let window = world.spawn_empty().id();
    presenter.attach_target(
        balloon_target(scope),
        window,
        EmoWorld::build(&areka_parsers::shell::parse("")),
        AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
        96,
    );
    window
}

/// 描画の時刻だけを持つ World。
pub fn world_at(frame_now: f64) -> World {
    let mut world = World::new();
    world.insert_resource(FrameTime(frame_now));
    world
}

/// 既定の設定の文字の層。
pub fn runtime() -> Rc<RefCell<TextLayerRuntime>> {
    Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )))
}

// 本体のテスト（`crates/areka/src/runtime_tests/`）が使う私有の関数（中で記録を出す相の部品）へそのまま渡す口（`test-support` のときだけ・
// 本番の可視性は変えない）。
#[cfg(feature = "test-support")]
use crate::balloon_visibility::{
    BalloonVisibilityState, VisibilityLogEvent, VisibilityObservations,
};
#[cfg(feature = "test-support")]
use crate::shell_box_hover::ShellBoxHover;
#[cfg(feature = "test-support")]
use crate::talk_lifecycle::TalkLifecycleSignal;
#[cfg(feature = "test-support")]
use areka_emo_text::actor::ShownBox;
#[cfg(feature = "test-support")]
use areka_sakura::TalkId;

/// [`super::collect_observations`] へそのまま渡す。
#[cfg(feature = "test-support")]
pub fn collect_observations(
    presenter: &EmoPresenter,
    runtime: &Rc<RefCell<TextLayerRuntime>>,
    world: &mut World,
    scopes: &[u32],
    now_talk_time: Option<f64>,
    lifecycle: Vec<TalkLifecycleSignal>,
    state: &mut BalloonVisibilityState,
) -> VisibilityObservations {
    super::collect_observations(
        presenter,
        runtime,
        world,
        scopes,
        now_talk_time,
        lifecycle,
        state,
    )
}

/// [`super::observe_scope_boxes`] へそのまま渡す。
#[cfg(feature = "test-support")]
pub fn observe_scope_boxes(
    shown: &[ShownBox],
    hover: Option<&mut ShellBoxHover>,
    scope: u32,
    missing_logged: &mut bool,
) -> (bool, bool) {
    super::observe_scope_boxes(shown, hover, scope, missing_logged)
}

/// [`super::notify_timeout`] へそのまま渡す。
#[cfg(feature = "test-support")]
pub fn notify_timeout(world: &World, notice: Option<TalkId>) {
    super::notify_timeout(world, notice)
}

/// [`super::emit_visibility_logs`] へそのまま渡す。
#[cfg(feature = "test-support")]
pub fn emit_visibility_logs(logs: &[VisibilityLogEvent], not_shown: &[u32]) {
    super::emit_visibility_logs(logs, not_shown)
}
