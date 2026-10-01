// =============================================================================
// 配線の段（相関数）のテストで共有する道具
//
// design.md「File Structure Plan」の `balloon_visibility_phase_test_support.rs`。
// 親（`balloon_visibility`）の `test_support` は判断中核の観測表を組む道具であり、
// こちらは表示層への headless 装着だけを持つ（写しを各テストに残さない）。
// =============================================================================

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::EmoWorld;
use areka_emo_present::EmoPresenter;
use bevy_ecs::prelude::Entity;
use bevy_ecs::world::World;

use crate::emo2_boot::target_map::balloon_target;

/// 表示層へ scope の balloon target を headless 装着し、窓の entity を返す
/// （可視状態は `Some(false)` から始まる）。
///
/// `attach_target` は GPU 資源を要さないため、実 `EmoPresenter` のまま装着できる。
pub(super) fn attach_headless(
    presenter: &mut EmoPresenter,
    world: &mut World,
    scope: u32,
) -> Entity {
    let window = world.spawn_empty().id();
    presenter
        .attach_target(
            world,
            balloon_target(scope),
            window,
            EmoWorld::build(&areka_parsers::shell::parse("")),
            AtlasTable::new(Vec::new(), Vec::new(), Vec::new()),
            96,
        )
        .expect("headless 装着は成功する");
    window
}
