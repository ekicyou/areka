//! `areka_runtime::balloon_visibility` の代わりの親（子のテストの宣言だけ）。

use areka_runtime::balloon_visibility::phase::run_balloon_visibility_phase;
use areka_runtime::balloon_visibility::*;

#[path = "balloon_visibility_phase.rs"]
mod phase;

// 会話終了観測から判断中核までの端から端。実台本の再生から、受け口が落ちてトークの
// 終わりが届き、占有終端（止まった時刻と同じ）が計測起点になるところまでを 1 本で通す。判断中核の私有状態を読むため親の内側に置く。
#[cfg(test)]
#[path = "balloon_visibility_lifecycle_e2e_tests.rs"]
mod lifecycle_e2e_tests;
