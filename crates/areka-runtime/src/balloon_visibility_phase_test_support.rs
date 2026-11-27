// =============================================================================
// 配線の段（相関数）のテストで共有する道具
// 親（`balloon_visibility`）の `test_support` は判断中核の観測表を組む道具であり、
// こちらは表示層への headless 装着と、配線の段のテストが使う World と文字の層の組み立てを持つ
// （写しを各テストに残さない）。
// =============================================================================

use bevy_ecs::world::World;
#[cfg(test)]
use {
    crate::target_map::balloon_target, areka_emo_atlas::AtlasTable, areka_emo_compose::EmoWorld,
    areka_emo_present::EmoPresenter, areka_emo_text::actor::TextLayerRuntime,
    areka_emo_text::state::TextLayerConfig, bevy_ecs::prelude::Entity, std::cell::RefCell,
    std::rc::Rc, wintf::ecs::FrameTime,
};

/// 表示層へ scope の balloon target を headless 装着し、窓の entity を返す
/// （可視状態は `Some(false)` から始まる）。
///
/// `attach_target` は GPU 資源を要さないため、実 `EmoPresenter` のまま装着できる。
#[cfg(test)]
pub(in crate::balloon_visibility) fn attach_headless(
    presenter: &mut EmoPresenter,
    world: &mut World,
    scope: u32,
) -> Entity {
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
#[cfg(test)]
pub(super) fn world_at(frame_now: dola::TimePoint) -> World {
    let mut world = World::new();
    world.insert_resource(FrameTime(frame_now));
    world
}

/// 既定の設定の文字の層。
#[cfg(test)]
pub(in crate::balloon_visibility) fn runtime() -> Rc<RefCell<TextLayerRuntime>> {
    Rc::new(RefCell::new(TextLayerRuntime::new(
        TextLayerConfig::default(),
    )))
}

/// `[balloon-visibility]` を名乗る行だけを取り出す（他所のログは配線の関心事ではない）。
#[cfg(test)]
pub(super) fn visibility_lines(
    events: &[log_capture_kit::CapturedEvent],
) -> Vec<&log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.message().contains("[balloon-visibility]"))
        .collect()
}

/// [`super::notify_timeout`] へそのまま渡す（本体のテストが使う）。
#[cfg(feature = "test-support")]
pub fn notify_timeout(world: &World, notice: Option<areka_sakura::TalkId>) {
    super::notify_timeout(world, notice)
}
