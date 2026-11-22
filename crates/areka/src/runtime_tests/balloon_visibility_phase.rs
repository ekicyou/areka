//! `areka_runtime::balloon_visibility::phase` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::balloon_visibility::BalloonVisibilityState;
use areka_runtime::balloon_visibility::phase::test_support::notify_timeout;
use bevy_ecs::world::World;

// 時間切れの知らせを置き場のゴーストの kanade へ送る配線（送った・送れなかったの記録と切替の直後）。
#[cfg(test)]
#[path = "balloon_visibility_phase_notice_tests.rs"]
mod notice_tests;
