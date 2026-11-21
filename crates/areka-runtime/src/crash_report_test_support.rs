//! テストのプロセスの落ちの報告の入口（本体は `crash-report-kit`）。

pub use crash_report_kit::{hold, install, installed, release};

#[cfg(test)]
#[used]
#[unsafe(link_section = ".CRT$XCU")]
static INSTALL: extern "C" fn() = install;

/// ふだんの走行でも、受け手は libtest の main より前に据わっている（落とさずに確かめる）。
#[test]
fn crash_filter_is_installed_before_the_tests_start() {
    assert!(
        installed(),
        "`.CRT$XCU` の初期化子が走っていない（受け手が据わらない）"
    );
}

/// 較正: 許可を取り `GraphicsCore` を作った後（GPU の DLL を読み込んだ後も受け手が生きていることを
/// 見るため）、名前の無いスレッドで例外を起こす。出力に落ちたスレッド（名前なし）・持ち手の行のこのテストの名前・
/// 生きているスレッドの一覧が出ることを `load-flake.ps1` の summary で見る。
#[test]
#[ignore = "較正: わざとプロセスを落とす"]
fn crash_report_names_the_running_test() {
    let _gpu = crate::spine::GpuPermit::take();
    let _core = wintf::ecs::GraphicsCore::new().expect("GraphicsCore::new 失敗");
    crash_report_kit::crash_on_an_unnamed_thread();
}
