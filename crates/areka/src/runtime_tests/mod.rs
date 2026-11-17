//! `areka-runtime` のテストのうち、本体の部品も使うものの置き場（spec: runtime-crate-split 設計 BodyRuntimeTests）。
//!
//! `areka-runtime` は本体を使えないので、本体の部品も使う runtime のテストはここに置く。木は runtime を写す:
//! 各ファイルは本物の親の代わりの親で、`use areka_runtime::<道>::*;`・本物の親が持っていて子が使う `use` の行・
//! 子のテストの宣言（本物の親と同じ印・同じ名前）だけを持つ。本番のコードは置かない。

use areka_runtime::*;

// 落ちの報告をこのテストの実行ファイルに据える（runtime-crate-split 設計 D6）。中身は `areka-runtime` の 1 つだけを指す。
#[used]
#[unsafe(link_section = ".CRT$XCU")]
static INSTALL: extern "C" fn() = areka_runtime::crash_report_test_support::install;

/// 落ちの報告（とプロセスの寿命の MTA）がテストより先に据わっている（runtime-crate-split 要件 3.8・設計 D6）。
#[test]
fn crash_filter_is_installed_before_the_tests_start() {
    assert!(areka_runtime::crash_report_test_support::installed());
}

mod assets;
mod balloon_visibility;
mod consumer_ledger;
mod frame;
mod shell_carry;
// 本体の `ghost_session_{reload,restart}_tests` も背骨の足場（`SpineHarness`）を使う（runtime-crate-split 設計 BodyRuntimeTests）。
pub(crate) mod spine;
mod strict_judges;

// タグ入口の結線と shell 設定の起動時結線のうち、本体の側の字面（`runtime_boot.rs`・`main.rs`）を押さえる 2 本。
#[path = "zorder_wiring_tests.rs"]
mod zorder_wiring_tests;
// 終了相（`ghost_quit_system`）の登録の形と、毎フレームの相が無い World での終了の 1 本。
#[path = "frame_schedule_tests.rs"]
mod frame_schedule_tests;
