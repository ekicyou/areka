//! `areka-runtime` のテストのうち、本体の部品も使うものの置き場（spec: runtime-crate-split 設計 BodyRuntimeTests）。
//!
//! `areka-runtime` は本体を使えないので、本体の部品も使う runtime のテストはここに置く。木は runtime を写す:
//! 各ファイルは本物の親の代わりの親で、`use areka_runtime::<道>::*;`・本物の親が持っていて子が使う `use` の行・
//! 子のテストの宣言（本物の親と同じ印・同じ名前）だけを持つ。本番のコードは置かない。

use areka_ghost::ShioriWiring;
use areka_ghost::TickerMode;
use areka_kanade::BootOrigin;
use areka_placement::AuthorDpi;
use areka_placement::DefaultSurfaces;
use areka_runtime::*;
use bevy_ecs::world::World;
use std::path::PathBuf;

// 落ちの報告をこのテストの実行ファイルに据える（設計 D6）。中身は `areka-runtime` の 1 つだけを指す。
#[used]
#[unsafe(link_section = ".CRT$XCU")]
static INSTALL: extern "C" fn() = areka_runtime::crash_report_test_support::install;

/// 落ちの報告（とプロセスの寿命の MTA）がテストより先に据わっている（要件 3.8・設計 D6）。
#[test]
fn crash_filter_is_installed_before_the_tests_start() {
    assert!(areka_runtime::crash_report_test_support::installed());
}

mod assets;
mod balloon_visibility;
mod consumer_ledger;
mod frame;
mod shell_balloon_resolve;
mod shell_carry;
// 本体の `ghost_session_{reload,restart}_tests` も背骨の足場（`SpineHarness`）を使う（設計 BodyRuntimeTests）。
pub(crate) mod spine;
mod strict_judges;
mod switch_cue;

// タグ入口の結線（areka-P0-scope-zorder-pinning task 6.2）の檻。受け渡し口・入口の登録・
// 受け渡し構造・相の呼出という 4 点は、削っても判断のテストが 1 本も赤くならない性質を
// 持つので、到達性・相順・字面の 3 方向でここが受け持つ。
#[cfg(test)]
#[path = "zorder_wiring_tests.rs"]
mod zorder_wiring_tests;
// 毎フレームの相の登録先の檻（areka-P0-present-gpu-transform-scale task 4.2・T-N10）。
// 裁定 2026-09-12 で登録先が 1 巡の末尾の段から `Update` へ移った——絵の着地を文字と同じ巡に
// 揃えるための移動であり、実窓を持たない檻には挙動として映らないので、字面と `Schedules` の
// 構造の両方から押さえる。
#[cfg(test)]
#[path = "frame_schedule_tests.rs"]
mod frame_schedule_tests;
// 本物の読み手から合成まで、動く絵をシェルとバルーンで通す（areka-P0-animated-image-playback task 5.4）。
#[cfg(test)]
#[path = "film_playback_e2e_tests.rs"]
mod film_playback_e2e_tests;
#[cfg(test)]
#[path = "wire_tests.rs"]
mod wire_tests;
