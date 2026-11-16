//! `areka_runtime::frame::status_report` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::frame::status_report::*;
use std::collections::BTreeMap;
use areka_emo_present::EmoPresenter;
use areka_emo_text::actor::TextLayerRuntime;
use areka_kanade::BalloonBinding;
use areka_kanade::ExecutionStateUpdate;
use areka_kanade::KanadeMsg;
use areka_sakura::ActorKey;
use areka_seriko::StageNote;
use bevy_ecs::world::World;
use tracing::error;
use areka_runtime::target_map::balloon_target;

#[cfg(test)]
#[path = "status_report_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "status_report_stage_tests.rs"]
mod stage_tests;
