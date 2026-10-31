//! `balloon.syncscale` の指定を World の資源として持つ（shell-scale-linkage 要件 6.5・6.6）。
//!
//! 配置の基準（[`DefaultPlacementBasis`]）が入れ直された巡でだけ、ゴーストとシェルの descript から
//! 読み直して丸ごと置き換える。起動・シェルの差し替え・ゴーストの切り替えはどれも基準を入れ直すので、
//! 1 つの印で覆える。値を使う下流はまだ無い（持つだけ・要件 6.6）。

use bevy_ecs::prelude::*;
use wintf::ecs::Update;

use super::descript_window_keys::{BalloonSyncScale, read_balloon_syncscale};
use super::reset_pos::DefaultPlacementBasis;

/// 走っているゴーストとシェルの `balloon.syncscale` の指定（保存しない）。
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BalloonSyncScaleState(pub(crate) BalloonSyncScale);

/// 基準が入れ直された巡でだけ読み直し、前の指定を残さず置き換える（要件 6.5）。
fn reload_balloon_syncscale(basis: Option<Res<DefaultPlacementBasis>>, mut commands: Commands) {
    let Some(basis) = basis.filter(|b| b.is_changed()) else {
        return;
    };
    let read = read_balloon_syncscale(&basis.source.ghost_kv, &basis.source.shell_kv);
    commands.insert_resource(BalloonSyncScaleState(read));
}

/// 系をプロセスに 1 回だけ載せる（`ghost_session::register_systems` から）。
pub(crate) fn register(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(Update, reload_balloon_syncscale);
}

#[cfg(test)]
#[path = "balloon_syncscale_tests.rs"]
mod tests;
