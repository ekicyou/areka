//! `areka_runtime::frame::switch` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::frame::switch::*;
use std::time::Instant;
use areka_actor::reply_channel;
use areka_emo_present::PresentOutcome;
use areka_emo_present::balloon_definitions::ScopeBalloonDefinitions;
use areka_kanade::GapLeft;
use areka_kanade::KanadeMsg;
use areka_kanade::MarkedEnd;
use areka_kanade::ShioriMethod;
use areka_kanade::TalkGap;
use areka_sakura::ActorKey;
use areka_seriko::SerikoReplace;
use areka_sylphya::PersistKey;
use bevy_ecs::world::World;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;
use areka_runtime::frame::RuntimeWiring;
use areka_runtime::shell_balloon_resolve::SkinCandidate;
use areka_runtime::shell_box_assets::ShellBoxAssets;
use areka_runtime::shell_cursor_assets::ShellCursorAssets;
use areka_runtime::skin_switch::SkinKind;
use areka_runtime::skin_switch::SkinSwitchInFlight;
use areka_runtime::skin_switch::SkinSwitchStage;
use areka_runtime::skin_switch::SwapFinish;
use areka_runtime::switch_assets::SwapBuilt;
use areka_runtime::switch_assets::SwapPayload;
use areka_placement::reseed::BalloonPlacementInputs;
use areka_placement::reset_pos::DefaultPlacementBasis;
use areka_placement::source::DescriptSource;
use areka_placement::spawn::GhostWindows;

#[cfg(test)]
#[path = "switch_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "switch_finish_tests.rs"]
mod finish_tests;
