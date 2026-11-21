//! `areka_runtime::frame::status_report` の代わりの親（子のテストの宣言だけ）。

use areka_emo_present::EmoPresenter;
use areka_kanade::BalloonBinding;
use areka_kanade::ExecutionStateUpdate;
use areka_kanade::KanadeMsg;
use areka_runtime::frame::status_report::*;
use areka_runtime::target_map::balloon_target;
use bevy_ecs::world::World;
// 中で記録を出す私有の関数は渡し口から同じ名前で引く。
use areka_runtime::frame::status_report::test_support::report_observed;

#[cfg(test)]
#[path = "status_report_stage_tests.rs"]
mod stage_tests;
#[cfg(test)]
#[path = "status_report_tests.rs"]
mod tests;
